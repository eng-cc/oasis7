use oasis7_wasm_abi::{
    ModuleCallCaller, ModuleCallInput, ModuleCallOrigin, ModuleSandbox, ModuleSubscriptionStage,
    ModuleTickLifecycleDirective,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use super::super::util::{hash_json, to_canonical_cbor};
use super::super::{ModuleKind, ModuleManifest, ModuleRegistry, WorldError};
use super::World;
use super::module_runtime_labels::{
    module_kind_label, module_role_label, subscription_stage_label,
};
use crate::simulator::ModuleInstallTarget;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleTickRoutingDurationBuckets {
    pub lt_1ms: u64,
    pub ms_1_to_5: u64,
    pub ms_5_to_25: u64,
    pub ms_25_to_100: u64,
    pub ge_100ms: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleTickRoutingMetricsSnapshot {
    pub schedule_len: usize,
    pub last_due_count: usize,
    pub last_invoked_count: usize,
    pub missing_invocation_count: u64,
    pub last_missing_invocation_count: usize,
    pub oldest_overdue_ticks: Option<u64>,
    pub routing_count: u64,
    pub last_route_duration_ms: u64,
    pub max_route_duration_ms: u64,
    pub cumulative_route_duration_ms: u64,
    pub duration_buckets: ModuleTickRoutingDurationBuckets,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleTickRoutingDeterministicSnapshot {
    pub last_due_count: usize,
    pub last_invoked_count: usize,
    pub missing_invocation_count: u64,
    pub last_missing_invocation_count: usize,
    pub oldest_overdue_ticks: Option<u64>,
    pub routing_count: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ModuleTickRoutingMetrics {
    pub(super) missing_invocation_count: u64,
    pub(super) routing_count: u64,
    pub(super) last_due_count: usize,
    pub(super) last_invoked_count: usize,
    pub(super) last_missing_invocation_count: usize,
    pub(super) oldest_overdue_ticks: Option<u64>,
    pub(super) last_route_duration_ms: u64,
    pub(super) max_route_duration_ms: u64,
    pub(super) cumulative_route_duration_ms: u64,
    pub(super) duration_buckets: ModuleTickRoutingDurationBuckets,
}

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
        let routing_started_at = Instant::now();
        let now = self.state.time;
        let schedule_len = self.module_tick_schedule.len();
        let invocation_ids: Vec<String> = self
            .module_tick_schedule
            .iter()
            .filter_map(|(instance_id, wake_at)| (*wake_at <= now).then_some(instance_id.clone()))
            .collect();
        if invocation_ids.is_empty() {
            self.record_module_tick_routing_metrics(
                schedule_len,
                0,
                0,
                0,
                None,
                routing_started_at.elapsed(),
            );
            return Ok(0);
        }
        let oldest_overdue_ticks = self
            .module_tick_schedule
            .iter()
            .filter_map(|(_, wake_at)| (*wake_at <= now).then_some(now.saturating_sub(*wake_at)))
            .max();

        let world_config_hash = self.current_manifest_hash()?;
        let mut due_invocations = Vec::with_capacity(invocation_ids.len());
        let mut missing_invocation_count = 0usize;
        for invocation_id in invocation_ids {
            let invocation = match self.active_module_invocation_for_id(&invocation_id) {
                Ok(invocation) => invocation,
                Err(err) => {
                    self.record_module_tick_routing_metrics(
                        schedule_len,
                        due_invocations.len().saturating_add(1),
                        0,
                        missing_invocation_count.saturating_add(1),
                        oldest_overdue_ticks,
                        routing_started_at.elapsed(),
                    );
                    return Err(err);
                }
            };
            if invocation.is_none() {
                missing_invocation_count = missing_invocation_count.saturating_add(1);
            }
            due_invocations.push((invocation_id, invocation));
        }
        let due_count = due_invocations.len();

        let mut invoked = 0;
        for (invocation_id, invocation) in due_invocations {
            // Always remove the previous schedule first. The module output decides whether to
            // reschedule itself (wake) or stay suspended.
            self.module_tick_schedule.remove(invocation_id.as_str());

            let Some(invocation) = invocation else {
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
                    caller: ModuleCallCaller::System {
                        system_id: "module_tick".to_string(),
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
        self.record_module_tick_routing_metrics(
            schedule_len,
            due_count,
            invoked,
            missing_invocation_count,
            oldest_overdue_ticks,
            routing_started_at.elapsed(),
        );
        Ok(invoked)
    }

    pub fn module_tick_routing_metrics_snapshot(&self) -> ModuleTickRoutingMetricsSnapshot {
        self.module_tick_routing_metrics
            .snapshot(self.module_tick_schedule.len())
    }

    fn record_module_tick_routing_metrics(
        &mut self,
        schedule_len: usize,
        due_count: usize,
        invoked_count: usize,
        missing_invocation_count: usize,
        oldest_overdue_ticks: Option<u64>,
        duration: std::time::Duration,
    ) {
        self.module_tick_routing_metrics.record(
            schedule_len,
            due_count,
            invoked_count,
            missing_invocation_count,
            oldest_overdue_ticks,
            duration,
        );
    }
}

impl ModuleTickRoutingMetrics {
    pub(super) fn from_deterministic_snapshot(
        snapshot: ModuleTickRoutingDeterministicSnapshot,
    ) -> Self {
        Self {
            missing_invocation_count: snapshot.missing_invocation_count,
            routing_count: snapshot.routing_count,
            last_due_count: snapshot.last_due_count,
            last_invoked_count: snapshot.last_invoked_count,
            last_missing_invocation_count: snapshot.last_missing_invocation_count,
            oldest_overdue_ticks: snapshot.oldest_overdue_ticks,
            last_route_duration_ms: 0,
            max_route_duration_ms: 0,
            cumulative_route_duration_ms: 0,
            duration_buckets: ModuleTickRoutingDurationBuckets::default(),
        }
    }

    pub(super) fn deterministic_snapshot(&self) -> ModuleTickRoutingDeterministicSnapshot {
        ModuleTickRoutingDeterministicSnapshot {
            last_due_count: self.last_due_count,
            last_invoked_count: self.last_invoked_count,
            missing_invocation_count: self.missing_invocation_count,
            last_missing_invocation_count: self.last_missing_invocation_count,
            oldest_overdue_ticks: self.oldest_overdue_ticks,
            routing_count: self.routing_count,
        }
    }

    fn record(
        &mut self,
        _schedule_len: usize,
        due_count: usize,
        invoked_count: usize,
        missing_invocation_count: usize,
        oldest_overdue_ticks: Option<u64>,
        duration: std::time::Duration,
    ) {
        let duration_ms = duration.as_millis().min(u128::from(u64::MAX)) as u64;
        self.routing_count = self.routing_count.saturating_add(1);
        self.last_due_count = due_count;
        self.last_invoked_count = invoked_count;
        self.last_missing_invocation_count = missing_invocation_count;
        self.missing_invocation_count = self
            .missing_invocation_count
            .saturating_add(missing_invocation_count as u64);
        self.oldest_overdue_ticks = oldest_overdue_ticks;
        self.last_route_duration_ms = duration_ms;
        self.max_route_duration_ms = self.max_route_duration_ms.max(duration_ms);
        self.cumulative_route_duration_ms = self
            .cumulative_route_duration_ms
            .saturating_add(duration_ms);
        if duration_ms == 0 {
            self.duration_buckets.lt_1ms = self.duration_buckets.lt_1ms.saturating_add(1);
        } else if duration_ms < 5 {
            self.duration_buckets.ms_1_to_5 = self.duration_buckets.ms_1_to_5.saturating_add(1);
        } else if duration_ms < 25 {
            self.duration_buckets.ms_5_to_25 = self.duration_buckets.ms_5_to_25.saturating_add(1);
        } else if duration_ms < 100 {
            self.duration_buckets.ms_25_to_100 =
                self.duration_buckets.ms_25_to_100.saturating_add(1);
        } else {
            self.duration_buckets.ge_100ms = self.duration_buckets.ge_100ms.saturating_add(1);
        }
    }

    fn snapshot(&self, schedule_len: usize) -> ModuleTickRoutingMetricsSnapshot {
        ModuleTickRoutingMetricsSnapshot {
            schedule_len,
            last_due_count: self.last_due_count,
            last_invoked_count: self.last_invoked_count,
            missing_invocation_count: self.missing_invocation_count,
            last_missing_invocation_count: self.last_missing_invocation_count,
            oldest_overdue_ticks: self.oldest_overdue_ticks,
            routing_count: self.routing_count,
            last_route_duration_ms: self.last_route_duration_ms,
            max_route_duration_ms: self.max_route_duration_ms,
            cumulative_route_duration_ms: self.cumulative_route_duration_ms,
            duration_buckets: self.duration_buckets.clone(),
        }
    }
}

fn module_has_tick_subscription(manifest: &ModuleManifest) -> bool {
    manifest
        .subscriptions
        .iter()
        .any(|subscription| subscription.resolved_stage() == ModuleSubscriptionStage::Tick)
}
