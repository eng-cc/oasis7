use oasis7_wasm_abi::{
    ModuleCallInput, ModuleCallOrigin, ModuleSandbox, ModuleSubscriptionStage,
    ModuleTickLifecycleDirective,
};
use std::collections::BTreeMap;

use super::super::util::{hash_json, to_canonical_cbor};
use super::super::{ModuleKind, ModuleManifest, ModuleRegistry, WorldError};
use super::module_runtime_labels::{
    module_kind_label, module_role_label, subscription_stage_label,
};
use super::World;
use crate::simulator::ModuleInstallTarget;

impl World {
    pub(super) fn sync_tick_schedule_for_activation(
        &mut self,
        module_id: &str,
        version: &str,
        time: u64,
    ) -> Result<(), WorldError> {
        self.sync_tick_schedule_for_instance(module_id, module_id, version, time)
    }

    pub(super) fn sync_tick_schedule_for_instance(
        &mut self,
        instance_id: &str,
        module_id: &str,
        version: &str,
        time: u64,
    ) -> Result<(), WorldError> {
        let key = ModuleRegistry::record_key(module_id, version);
        let record = self.module_registry.records.get(&key).ok_or_else(|| {
            WorldError::ModuleChangeInvalid {
                reason: format!("module record missing {key}"),
            }
        })?;
        if module_has_tick_subscription(&record.manifest) {
            self.module_tick_schedule
                .insert(instance_id.to_string(), time);
        } else {
            self.module_tick_schedule.remove(instance_id);
        }
        Ok(())
    }

    pub(super) fn remove_tick_schedule(&mut self, module_id: &str) {
        self.module_tick_schedule.remove(module_id);
    }

    pub fn route_tick_to_modules(
        &mut self,
        sandbox: &mut dyn ModuleSandbox,
    ) -> Result<usize, WorldError> {
        let now = self.state.time;
        let mut invocation_ids: Vec<String> = self
            .module_tick_schedule
            .iter()
            .filter_map(|(instance_id, wake_at)| (*wake_at <= now).then_some(instance_id.clone()))
            .collect();
        invocation_ids.sort();
        if invocation_ids.is_empty() {
            return Ok(0);
        }

        let mut active_invocations = BTreeMap::new();
        for invocation in self.collect_active_module_invocations()? {
            active_invocations.insert(invocation.instance_id.clone(), invocation);
        }
        let world_config_hash = self.current_manifest_hash()?;
        let mut invoked = 0;
        for invocation_id in invocation_ids {
            // Always remove the previous schedule first. The module output decides whether to
            // reschedule itself (wake) or stay suspended.
            self.module_tick_schedule.remove(invocation_id.as_str());

            let Some(invocation) = active_invocations.get(&invocation_id).cloned() else {
                continue;
            };
            let manifest = invocation.manifest;
            let module_id = invocation.module_id;
            let instance_id = invocation.instance_id;
            if !module_has_tick_subscription(&manifest) {
                continue;
            }
            let module_manifest_hash = hash_json(&manifest)?;

            let (origin_kind, origin_id, trace_id) = match invocation.install_target {
                ModuleInstallTarget::SelfAgent => (
                    "tick".to_string(),
                    now.to_string(),
                    format!("tick-{}-{}", now, instance_id),
                ),
                ModuleInstallTarget::LocationInfrastructure { location_id } => {
                    let location_id = location_id.trim().to_string();
                    if location_id.is_empty() {
                        (
                            "tick".to_string(),
                            now.to_string(),
                            format!("tick-{}-{}", now, instance_id),
                        )
                    } else {
                        (
                            "infrastructure_tick".to_string(),
                            format!("{}:{}", location_id, now),
                            format!("infra-tick-{}-{}-{}", now, location_id, instance_id),
                        )
                    }
                }
            };
            let state = match manifest.kind {
                ModuleKind::Reducer => Some(
                    self.state
                        .module_states
                        .get(&instance_id)
                        .cloned()
                        .unwrap_or_default(),
                ),
                ModuleKind::Pure => None,
            };
            let input = ModuleCallInput {
                ctx: oasis7_wasm_abi::ModuleContext {
                    v: "wasm-1".to_string(),
                    module_id: module_id.clone(),
                    trace_id: trace_id.clone(),
                    time: now,
                    origin: ModuleCallOrigin {
                        kind: origin_kind,
                        id: origin_id,
                    },
                    limits: manifest.limits.clone(),
                    stage: Some(
                        subscription_stage_label(ModuleSubscriptionStage::Tick).to_string(),
                    ),
                    world_config_hash: Some(world_config_hash.clone()),
                    manifest_hash: Some(module_manifest_hash),
                    journal_height: Some(self.journal.events.len() as u64),
                    module_version: Some(manifest.version.clone()),
                    module_kind: Some(module_kind_label(&manifest.kind).to_string()),
                    module_role: Some(module_role_label(&manifest.role).to_string()),
                },
                event: None,
                action: None,
                state,
            };
            let input_bytes = to_canonical_cbor(&input)?;
            let output = self.execute_module_call_with_manifest_and_state_key(
                module_id.as_str(),
                instance_id.as_str(),
                &manifest,
                trace_id,
                input_bytes,
                sandbox,
            )?;
            invoked += 1;

            match output.tick_lifecycle {
                Some(ModuleTickLifecycleDirective::WakeAfterTicks { ticks }) => {
                    let wake_after = ticks.max(1);
                    self.module_tick_schedule
                        .insert(instance_id, now.saturating_add(wake_after));
                }
                Some(ModuleTickLifecycleDirective::Suspend) | None => {}
            }
        }
        Ok(invoked)
    }
}

fn module_has_tick_subscription(manifest: &ModuleManifest) -> bool {
    manifest
        .subscriptions
        .iter()
        .any(|subscription| subscription.resolved_stage() == ModuleSubscriptionStage::Tick)
}
