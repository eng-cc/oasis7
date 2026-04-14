use std::collections::BTreeSet;
use std::sync::Arc;

use oasis7_wasm_abi::{
    ModuleCallErrorCode, ModuleCallFailure, ModuleCallInput, ModuleCallOrigin, ModuleCallRequest,
    ModuleContext, ModuleEmitEvent, ModuleOutput, ModuleSandbox, ModuleStateUpdate,
};
use oasis7_wasm_router::{
    prepare_subscriptions, prepared_module_subscribes_to_action,
    prepared_module_subscribes_to_event, PreparedSubscription,
};

use super::super::util::{hash_json, to_canonical_cbor};
use super::super::{
    ActionEnvelope, EffectOrigin, ModuleArtifact, ModuleEvent, ModuleEventKind, ModuleKind,
    ModuleLimits, ModuleManifest, ModuleRegistry, ModuleSubscriptionStage, WorldError, WorldEvent,
    WorldEventBody,
};
use super::module_runtime_labels::{
    action_kind_label, event_kind_label, module_kind_label, module_role_label,
    subscription_stage_label,
};
use super::World;
use crate::simulator::ModuleInstallTarget;

fn count_exceeds_limit(count: usize, limit: u32) -> bool {
    match u32::try_from(count) {
        Ok(value) => value > limit,
        Err(_) => true,
    }
}

#[derive(Debug, Clone)]
pub(super) struct ActiveModuleInvocation {
    pub(super) instance_id: String,
    pub(super) module_id: String,
    pub(super) install_target: ModuleInstallTarget,
    pub(super) manifest: ModuleManifest,
}

impl World {
    // ---------------------------------------------------------------------
    // Module artifact and limits
    // ---------------------------------------------------------------------

    pub fn register_module_artifact(
        &mut self,
        wasm_hash: impl Into<String>,
        bytes: &[u8],
    ) -> Result<(), WorldError> {
        let wasm_hash = wasm_hash.into();
        let computed = super::super::util::sha256_hex(bytes);
        if computed != wasm_hash {
            return Err(WorldError::ModuleChangeInvalid {
                reason: format!("artifact hash mismatch expected {wasm_hash} found {computed}"),
            });
        }
        self.module_artifacts.insert(wasm_hash);
        self.module_artifact_bytes
            .insert(computed, Arc::<[u8]>::from(bytes));
        Ok(())
    }

    pub fn set_module_limits_max(&mut self, limits: ModuleLimits) {
        self.module_limits_max = limits;
    }

    pub fn set_module_cache_max(&mut self, max_cached_modules: usize) {
        self.module_cache.set_max_cached_modules(max_cached_modules);
    }

    pub fn load_module(&mut self, wasm_hash: &str) -> Result<ModuleArtifact, WorldError> {
        if let Some(artifact) = self.module_cache.get(wasm_hash) {
            return Ok(artifact);
        }
        let bytes = self
            .module_artifact_bytes
            .get(wasm_hash)
            .ok_or_else(|| WorldError::ModuleChangeInvalid {
                reason: format!("module artifact bytes missing {wasm_hash}"),
            })?
            .clone();
        let artifact = ModuleArtifact {
            wasm_hash: wasm_hash.to_string(),
            bytes,
        };
        self.module_cache.insert(artifact.clone());
        Ok(artifact)
    }

    pub fn validate_module_output_limits(
        &self,
        module_id: &str,
        limits: &ModuleLimits,
        effect_count: usize,
        emit_count: usize,
        output_bytes: u64,
    ) -> Result<(), WorldError> {
        if count_exceeds_limit(effect_count, limits.max_effects) {
            return Err(WorldError::ModuleChangeInvalid {
                reason: format!("module output effects exceeded {module_id}"),
            });
        }
        if count_exceeds_limit(emit_count, limits.max_emits) {
            return Err(WorldError::ModuleChangeInvalid {
                reason: format!("module output emits exceeded {module_id}"),
            });
        }
        if output_bytes > limits.max_output_bytes {
            return Err(WorldError::ModuleChangeInvalid {
                reason: format!("module output bytes exceeded {module_id}"),
            });
        }
        Ok(())
    }

    fn prepared_subscriptions_for_manifest(
        &mut self,
        manifest: &ModuleManifest,
    ) -> Result<Arc<[PreparedSubscription]>, WorldError> {
        let key = ModuleRegistry::record_key(&manifest.module_id, &manifest.version);
        if let Some(prepared) = self.prepared_subscription_cache.get(&key) {
            return Ok(prepared.clone());
        }
        let prepared = prepare_subscriptions(&manifest.subscriptions, &manifest.module_id)
            .map_err(|reason| WorldError::ModuleChangeInvalid { reason })?;
        self.prepared_subscription_cache
            .insert(key, prepared.clone());
        Ok(prepared)
    }

    pub fn execute_module_call(
        &mut self,
        module_id: &str,
        trace_id: impl Into<String>,
        input: Vec<u8>,
        sandbox: &mut dyn ModuleSandbox,
    ) -> Result<ModuleOutput, WorldError> {
        let manifest = self.active_module_manifest(module_id)?.clone();
        self.execute_module_call_with_manifest_and_state_key(
            module_id,
            module_id,
            &manifest,
            trace_id.into(),
            input,
            sandbox,
        )
    }

    pub(super) fn execute_module_call_with_manifest_and_state_key(
        &mut self,
        module_id: &str,
        state_key: &str,
        manifest: &ModuleManifest,
        trace_id: String,
        input: Vec<u8>,
        sandbox: &mut dyn ModuleSandbox,
    ) -> Result<ModuleOutput, WorldError> {
        let input_bytes = input.len() as u64;
        let output = match self.call_module_raw(module_id, &trace_id, input, manifest, sandbox) {
            Ok(output) => output,
            Err(failure) => {
                self.module_call_failed(failure)?;
                unreachable!("module_call_failed always returns Err")
            }
        };

        self.process_module_output(
            module_id,
            state_key,
            &trace_id,
            manifest,
            input_bytes,
            &output,
            sandbox,
        )?;
        Ok(output)
    }

    pub(super) fn collect_active_module_invocations(
        &self,
    ) -> Result<Vec<ActiveModuleInvocation>, WorldError> {
        let mut invocations = Vec::new();
        let mut module_ids_with_instances = BTreeSet::new();

        for instance in self.state.module_instances.values() {
            module_ids_with_instances.insert(instance.module_id.clone());
        }

        let mut instance_ids: Vec<String> = self.state.module_instances.keys().cloned().collect();
        instance_ids.sort();
        for instance_id in instance_ids {
            let Some(instance) = self.state.module_instances.get(&instance_id) else {
                continue;
            };
            if !instance.active {
                continue;
            }
            let key = ModuleRegistry::record_key(&instance.module_id, &instance.module_version);
            let record = self.module_registry.records.get(&key).ok_or_else(|| {
                WorldError::ModuleChangeInvalid {
                    reason: format!("module record missing {key}"),
                }
            })?;
            invocations.push(ActiveModuleInvocation {
                instance_id: instance.instance_id.clone(),
                module_id: instance.module_id.clone(),
                install_target: instance.install_target.clone(),
                manifest: record.manifest.clone(),
            });
        }

        let mut module_ids_without_instances: Vec<String> =
            self.module_registry.active.keys().cloned().collect();
        module_ids_without_instances.sort();
        for module_id in module_ids_without_instances {
            if module_ids_with_instances.contains(&module_id) {
                continue;
            }
            let version = self.module_registry.active.get(&module_id).ok_or_else(|| {
                WorldError::ModuleChangeInvalid {
                    reason: format!("module not active {module_id}"),
                }
            })?;
            let key = ModuleRegistry::record_key(&module_id, version);
            let record = self.module_registry.records.get(&key).ok_or_else(|| {
                WorldError::ModuleChangeInvalid {
                    reason: format!("module record missing {key}"),
                }
            })?;
            invocations.push(ActiveModuleInvocation {
                instance_id: module_id.clone(),
                module_id: module_id.clone(),
                install_target: self
                    .state
                    .installed_module_targets
                    .get(&module_id)
                    .cloned()
                    .unwrap_or(ModuleInstallTarget::SelfAgent),
                manifest: record.manifest.clone(),
            });
        }

        invocations.sort_by(|left, right| left.instance_id.cmp(&right.instance_id));
        Ok(invocations)
    }

    pub fn route_event_to_modules(
        &mut self,
        event: &WorldEvent,
        sandbox: &mut dyn ModuleSandbox,
    ) -> Result<usize, WorldError> {
        let event_kind = event_kind_label(&event.body);
        let event_value = serde_json::to_value(event)?;
        let invocations = self.collect_active_module_invocations()?;
        let event_bytes = to_canonical_cbor(event)?;
        let world_config_hash = self.current_manifest_hash()?;
        let mut invoked = 0;
        for invocation in invocations {
            let manifest = invocation.manifest;
            let module_id = invocation.module_id;
            let instance_id = invocation.instance_id;
            let prepared = self.prepared_subscriptions_for_manifest(&manifest)?;
            let subscribed =
                prepared_module_subscribes_to_event(prepared.as_ref(), event_kind, &event_value);
            if !subscribed {
                continue;
            }

            let trace_id = format!("event-{}-{}", event.id, instance_id);
            let module_manifest_hash = hash_json(&manifest)?;
            let ctx = ModuleContext {
                v: "wasm-1".to_string(),
                module_id: module_id.clone(),
                trace_id: trace_id.clone(),
                time: event.time,
                origin: ModuleCallOrigin {
                    kind: "event".to_string(),
                    id: event.id.to_string(),
                },
                limits: manifest.limits.clone(),
                stage: Some("post_event".to_string()),
                world_config_hash: Some(world_config_hash.clone()),
                manifest_hash: Some(module_manifest_hash),
                journal_height: Some(event.id),
                module_version: Some(manifest.version.clone()),
                module_kind: Some(module_kind_label(&manifest.kind).to_string()),
                module_role: Some(module_role_label(&manifest.role).to_string()),
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
                ctx,
                event: Some(event_bytes.clone()),
                action: None,
                state,
            };
            let input_bytes = to_canonical_cbor(&input)?;
            self.execute_module_call_with_manifest_and_state_key(
                module_id.as_str(),
                instance_id.as_str(),
                &manifest,
                trace_id,
                input_bytes,
                sandbox,
            )?;
            invoked += 1;
        }
        Ok(invoked)
    }

    pub fn route_action_to_modules(
        &mut self,
        envelope: &ActionEnvelope,
        sandbox: &mut dyn ModuleSandbox,
    ) -> Result<usize, WorldError> {
        self.route_action_to_modules_with_stage(
            envelope,
            ModuleSubscriptionStage::PreAction,
            sandbox,
        )
    }

    pub fn route_action_to_modules_with_stage(
        &mut self,
        envelope: &ActionEnvelope,
        stage: ModuleSubscriptionStage,
        sandbox: &mut dyn ModuleSandbox,
    ) -> Result<usize, WorldError> {
        let action_kind = action_kind_label(&envelope.action);
        let action_value = serde_json::to_value(envelope)?;
        let invocations = self.collect_active_module_invocations()?;
        let action_bytes = to_canonical_cbor(envelope)?;
        let world_config_hash = self.current_manifest_hash()?;
        let mut invoked = 0;

        for invocation in invocations {
            let manifest = invocation.manifest;
            let module_id = invocation.module_id;
            let instance_id = invocation.instance_id;
            let prepared = self.prepared_subscriptions_for_manifest(&manifest)?;
            let subscribed = prepared_module_subscribes_to_action(
                prepared.as_ref(),
                stage,
                action_kind,
                &action_value,
            );
            if !subscribed {
                continue;
            }

            let trace_id = format!("action-{}-{}", envelope.id, instance_id);
            let module_manifest_hash = hash_json(&manifest)?;
            let ctx = ModuleContext {
                v: "wasm-1".to_string(),
                module_id: module_id.clone(),
                trace_id: trace_id.clone(),
                time: self.state.time,
                origin: ModuleCallOrigin {
                    kind: "action".to_string(),
                    id: envelope.id.to_string(),
                },
                limits: manifest.limits.clone(),
                stage: Some(subscription_stage_label(stage).to_string()),
                world_config_hash: Some(world_config_hash.clone()),
                manifest_hash: Some(module_manifest_hash),
                journal_height: Some(self.journal.events.len() as u64),
                module_version: Some(manifest.version.clone()),
                module_kind: Some(module_kind_label(&manifest.kind).to_string()),
                module_role: Some(module_role_label(&manifest.role).to_string()),
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
                ctx,
                event: None,
                action: Some(action_bytes.clone()),
                state,
            };
            let input_bytes = to_canonical_cbor(&input)?;
            self.execute_module_call_with_manifest_and_state_key(
                module_id.as_str(),
                instance_id.as_str(),
                &manifest,
                trace_id,
                input_bytes,
                sandbox,
            )?;
            invoked += 1;
        }

        Ok(invoked)
    }

    fn module_call_failed(&mut self, failure: ModuleCallFailure) -> Result<(), WorldError> {
        self.append_event(WorldEventBody::ModuleCallFailed(failure.clone()), None)?;
        Err(WorldError::ModuleCallFailed {
            module_id: failure.module_id,
            trace_id: failure.trace_id,
            code: failure.code,
            detail: failure.detail,
        })
    }

    pub(super) fn apply_module_changes(
        &mut self,
        proposal_id: super::super::ProposalId,
        changes: &super::super::ModuleChangeSet,
        actor: &str,
    ) -> Result<(), WorldError> {
        let mut registers = changes.register.clone();
        registers.sort_by(|left, right| left.module_id.cmp(&right.module_id));
        for module in registers {
            let event = ModuleEvent {
                proposal_id,
                kind: ModuleEventKind::RegisterModule {
                    module,
                    registered_by: actor.to_string(),
                },
            };
            self.append_event(WorldEventBody::ModuleEvent(event), None)?;
        }

        let mut upgrades = changes.upgrade.clone();
        upgrades.sort_by(|left, right| left.module_id.cmp(&right.module_id));
        for upgrade in upgrades {
            let event = ModuleEvent {
                proposal_id,
                kind: ModuleEventKind::UpgradeModule {
                    module_id: upgrade.module_id,
                    from_version: upgrade.from_version,
                    to_version: upgrade.to_version,
                    wasm_hash: upgrade.manifest.wasm_hash.clone(),
                    manifest: upgrade.manifest,
                    upgraded_by: actor.to_string(),
                },
            };
            self.append_event(WorldEventBody::ModuleEvent(event), None)?;
        }

        let mut activations = changes.activate.clone();
        activations.sort_by(|left, right| left.module_id.cmp(&right.module_id));
        for activation in activations {
            let event = ModuleEvent {
                proposal_id,
                kind: ModuleEventKind::ActivateModule {
                    module_id: activation.module_id,
                    version: activation.version,
                    activated_by: actor.to_string(),
                },
            };
            self.append_event(WorldEventBody::ModuleEvent(event), None)?;
        }

        let mut deactivations = changes.deactivate.clone();
        deactivations.sort_by(|left, right| left.module_id.cmp(&right.module_id));
        for deactivation in deactivations {
            let event = ModuleEvent {
                proposal_id,
                kind: ModuleEventKind::DeactivateModule {
                    module_id: deactivation.module_id,
                    reason: deactivation.reason,
                    deactivated_by: actor.to_string(),
                },
            };
            self.append_event(WorldEventBody::ModuleEvent(event), None)?;
        }

        Ok(())
    }

    pub(super) fn apply_module_event(
        &mut self,
        event: &ModuleEvent,
        time: super::super::WorldTime,
    ) -> Result<(), WorldError> {
        match &event.kind {
            ModuleEventKind::RegisterModule {
                module,
                registered_by,
            } => {
                let key = ModuleRegistry::record_key(&module.module_id, &module.version);
                self.prepared_subscription_cache.remove(&key);
                self.module_registry.records.insert(
                    key,
                    super::super::ModuleRecord {
                        manifest: module.clone(),
                        registered_at: time,
                        registered_by: registered_by.clone(),
                        audit_event_id: None,
                    },
                );
                self.module_artifacts.insert(module.wasm_hash.clone());
            }
            ModuleEventKind::UpgradeModule {
                module_id,
                to_version,
                manifest,
                upgraded_by,
                ..
            } => {
                let key = ModuleRegistry::record_key(module_id, to_version);
                self.prepared_subscription_cache.remove(&key);
                self.module_registry.records.insert(
                    key,
                    super::super::ModuleRecord {
                        manifest: manifest.clone(),
                        registered_at: time,
                        registered_by: upgraded_by.clone(),
                        audit_event_id: None,
                    },
                );
                self.module_artifacts.insert(manifest.wasm_hash.clone());
            }
            ModuleEventKind::ActivateModule {
                module_id, version, ..
            } => {
                self.module_registry
                    .active
                    .insert(module_id.clone(), version.clone());
                self.sync_tick_schedule_for_activation(module_id, version, time)?;
            }
            ModuleEventKind::DeactivateModule { module_id, .. } => {
                self.module_registry.active.remove(module_id);
                self.remove_tick_schedule(module_id);
            }
        }
        Ok(())
    }

    fn process_module_output(
        &mut self,
        module_id: &str,
        state_key: &str,
        trace_id: &str,
        manifest: &ModuleManifest,
        input_bytes: u64,
        output: &ModuleOutput,
        sandbox: &mut dyn ModuleSandbox,
    ) -> Result<(), WorldError> {
        if manifest.kind == ModuleKind::Pure && output.new_state.is_some() {
            return self.module_call_failed(ModuleCallFailure {
                module_id: module_id.to_string(),
                trace_id: trace_id.to_string(),
                code: ModuleCallErrorCode::InvalidOutput,
                detail: "pure module returned new_state".to_string(),
            });
        }
        if count_exceeds_limit(output.effects.len(), manifest.limits.max_effects) {
            return self.module_call_failed(ModuleCallFailure {
                module_id: module_id.to_string(),
                trace_id: trace_id.to_string(),
                code: ModuleCallErrorCode::EffectLimitExceeded,
                detail: "effects exceeded".to_string(),
            });
        }
        if count_exceeds_limit(output.emits.len(), manifest.limits.max_emits) {
            return self.module_call_failed(ModuleCallFailure {
                module_id: module_id.to_string(),
                trace_id: trace_id.to_string(),
                code: ModuleCallErrorCode::EmitLimitExceeded,
                detail: "emits exceeded".to_string(),
            });
        }
        if output.output_bytes > manifest.limits.max_output_bytes {
            return self.module_call_failed(ModuleCallFailure {
                module_id: module_id.to_string(),
                trace_id: trace_id.to_string(),
                code: ModuleCallErrorCode::OutputTooLarge,
                detail: "output bytes exceeded".to_string(),
            });
        }

        let mut resolved_caps = Vec::with_capacity(output.effects.len());
        for effect in &output.effects {
            let resolved_cap_ref = if let Some(slot) = effect.cap_slot.as_deref() {
                let Some(bound_cap_ref) = manifest.abi_contract.cap_slots.get(slot) else {
                    return self.module_call_failed(ModuleCallFailure {
                        module_id: module_id.to_string(),
                        trace_id: trace_id.to_string(),
                        code: ModuleCallErrorCode::CapsDenied,
                        detail: format!("cap_slot not bound {}", slot),
                    });
                };
                if !effect.cap_ref.trim().is_empty() && effect.cap_ref != *bound_cap_ref {
                    return self.module_call_failed(ModuleCallFailure {
                        module_id: module_id.to_string(),
                        trace_id: trace_id.to_string(),
                        code: ModuleCallErrorCode::CapsDenied,
                        detail: format!(
                            "cap_slot {} conflicts with cap_ref {}",
                            slot, effect.cap_ref
                        ),
                    });
                }
                bound_cap_ref.clone()
            } else {
                if effect.cap_ref.trim().is_empty() {
                    return self.module_call_failed(ModuleCallFailure {
                        module_id: module_id.to_string(),
                        trace_id: trace_id.to_string(),
                        code: ModuleCallErrorCode::CapsDenied,
                        detail: "cap_ref is empty".to_string(),
                    });
                }
                effect.cap_ref.clone()
            };

            if !manifest
                .required_caps
                .iter()
                .any(|cap| cap == &resolved_cap_ref)
            {
                return self.module_call_failed(ModuleCallFailure {
                    module_id: module_id.to_string(),
                    trace_id: trace_id.to_string(),
                    code: ModuleCallErrorCode::CapsDenied,
                    detail: format!("cap_ref not allowed {}", resolved_cap_ref),
                });
            }

            if let Err(failure) = self.enforce_pure_policy_hooks(
                module_id,
                trace_id,
                manifest,
                effect,
                &resolved_cap_ref,
                sandbox,
            ) {
                return self.module_call_failed(failure);
            }
            resolved_caps.push(resolved_cap_ref);
        }

        if let Err(failure) =
            self.try_charge_module_runtime(module_id, trace_id, manifest, input_bytes, output)
        {
            return self.module_call_failed(failure);
        }

        let mut intents = Vec::new();
        for (effect, resolved_cap_ref) in output.effects.iter().zip(resolved_caps.into_iter()) {
            let intent = match self.build_effect_intent(
                effect.kind.clone(),
                effect.params.clone(),
                resolved_cap_ref,
                EffectOrigin::Module {
                    module_id: module_id.to_string(),
                },
            ) {
                Ok(intent) => intent,
                Err(err) => {
                    let (code, detail) = match err {
                        WorldError::CapabilityMissing { cap_ref } => (
                            ModuleCallErrorCode::CapsDenied,
                            format!("cap missing {cap_ref}"),
                        ),
                        WorldError::CapabilityExpired { cap_ref } => (
                            ModuleCallErrorCode::CapsDenied,
                            format!("cap expired {cap_ref}"),
                        ),
                        WorldError::CapabilityNotAllowed { cap_ref, kind } => (
                            ModuleCallErrorCode::CapsDenied,
                            format!("cap not allowed {cap_ref} {kind}"),
                        ),
                        WorldError::PolicyDenied { reason, .. } => {
                            (ModuleCallErrorCode::PolicyDenied, reason)
                        }
                        other => (ModuleCallErrorCode::InvalidOutput, format!("{other:?}")),
                    };
                    return self.module_call_failed(ModuleCallFailure {
                        module_id: module_id.to_string(),
                        trace_id: trace_id.to_string(),
                        code,
                        detail,
                    });
                }
            };
            intents.push(intent);
        }

        if let Some(state) = &output.new_state {
            let update = ModuleStateUpdate {
                module_id: state_key.to_string(),
                trace_id: trace_id.to_string(),
                state: state.clone(),
            };
            self.append_event(WorldEventBody::ModuleStateUpdated(update), None)?;
        }

        for intent in intents {
            self.append_event(WorldEventBody::EffectQueued(intent), None)?;
        }

        for emit in &output.emits {
            let event = ModuleEmitEvent {
                module_id: module_id.to_string(),
                trace_id: trace_id.to_string(),
                kind: emit.kind.clone(),
                payload: emit.payload.clone(),
            };
            self.append_event(WorldEventBody::ModuleEmitted(event), None)?;
        }

        Ok(())
    }

    fn call_module_raw(
        &mut self,
        module_id: &str,
        trace_id: &str,
        input: Vec<u8>,
        manifest: &ModuleManifest,
        sandbox: &mut dyn ModuleSandbox,
    ) -> Result<ModuleOutput, ModuleCallFailure> {
        let wasm_hash = manifest.wasm_hash.clone();
        let artifact = self
            .load_module(&wasm_hash)
            .map_err(|err| ModuleCallFailure {
                module_id: module_id.to_string(),
                trace_id: trace_id.to_string(),
                code: ModuleCallErrorCode::SandboxUnavailable,
                detail: format!("load module failed: {err:?}"),
            })?;

        let request = ModuleCallRequest {
            module_id: module_id.to_string(),
            wasm_hash,
            trace_id: trace_id.to_string(),
            entrypoint: manifest.kind.entrypoint().to_string(),
            input,
            limits: manifest.limits.clone(),
            wasm_bytes: artifact.bytes.clone(),
        };
        sandbox.call(&request)
    }

    fn enforce_pure_policy_hooks(
        &mut self,
        module_id: &str,
        trace_id: &str,
        manifest: &ModuleManifest,
        effect: &oasis7_wasm_abi::ModuleEffectIntent,
        resolved_cap_ref: &str,
        sandbox: &mut dyn ModuleSandbox,
    ) -> Result<(), ModuleCallFailure> {
        for policy_module_id in &manifest.abi_contract.policy_hooks {
            let policy_manifest = self
                .active_module_manifest(policy_module_id)
                .map_err(|err| ModuleCallFailure {
                    module_id: module_id.to_string(),
                    trace_id: trace_id.to_string(),
                    code: ModuleCallErrorCode::PolicyDenied,
                    detail: format!(
                        "pure policy hook {} not available: {err:?}",
                        policy_module_id
                    ),
                })?
                .clone();
            if policy_manifest.kind != ModuleKind::Pure {
                return Err(ModuleCallFailure {
                    module_id: module_id.to_string(),
                    trace_id: trace_id.to_string(),
                    code: ModuleCallErrorCode::PolicyDenied,
                    detail: format!("pure policy hook {} is not pure", policy_module_id),
                });
            }
            let hook_trace_id = format!("policy-{trace_id}-{policy_module_id}");
            let world_config_hash =
                self.current_manifest_hash()
                    .map_err(|err| ModuleCallFailure {
                        module_id: module_id.to_string(),
                        trace_id: trace_id.to_string(),
                        code: ModuleCallErrorCode::PolicyDenied,
                        detail: format!(
                            "pure policy hook {} cannot read world config hash: {err:?}",
                            policy_module_id
                        ),
                    })?;
            let policy_manifest_hash =
                hash_json(&policy_manifest).map_err(|err| ModuleCallFailure {
                    module_id: module_id.to_string(),
                    trace_id: trace_id.to_string(),
                    code: ModuleCallErrorCode::PolicyDenied,
                    detail: format!(
                        "pure policy hook {} cannot hash module manifest: {err:?}",
                        policy_module_id
                    ),
                })?;
            let ctx = ModuleContext {
                v: "wasm-1".to_string(),
                module_id: policy_module_id.clone(),
                trace_id: hook_trace_id.clone(),
                time: self.state.time,
                origin: ModuleCallOrigin {
                    kind: "module_policy".to_string(),
                    id: trace_id.to_string(),
                },
                limits: policy_manifest.limits.clone(),
                stage: Some("module_policy".to_string()),
                world_config_hash: Some(world_config_hash.clone()),
                manifest_hash: Some(policy_manifest_hash),
                journal_height: Some(self.journal.events.len() as u64),
                module_version: Some(policy_manifest.version.clone()),
                module_kind: Some(module_kind_label(&policy_manifest.kind).to_string()),
                module_role: Some(module_role_label(&policy_manifest.role).to_string()),
            };
            let policy_payload = serde_json::json!({
                "source_module_id": module_id,
                "trace_id": trace_id,
                "effect_kind": effect.kind,
                "effect_params": effect.params,
                "cap_ref": resolved_cap_ref,
            });
            let input = ModuleCallInput {
                ctx,
                event: None,
                action: Some(to_canonical_cbor(&policy_payload).map_err(|err| {
                    ModuleCallFailure {
                        module_id: module_id.to_string(),
                        trace_id: trace_id.to_string(),
                        code: ModuleCallErrorCode::PolicyDenied,
                        detail: format!(
                            "pure policy hook {} input encode failed: {err:?}",
                            policy_module_id
                        ),
                    }
                })?),
                state: None,
            };
            let input_bytes = to_canonical_cbor(&input).map_err(|err| ModuleCallFailure {
                module_id: module_id.to_string(),
                trace_id: trace_id.to_string(),
                code: ModuleCallErrorCode::PolicyDenied,
                detail: format!(
                    "pure policy hook {} input envelope encode failed: {err:?}",
                    policy_module_id
                ),
            })?;
            let hook_output = self
                .call_module_raw(
                    policy_module_id,
                    &hook_trace_id,
                    input_bytes,
                    &policy_manifest,
                    sandbox,
                )
                .map_err(|failure| ModuleCallFailure {
                    module_id: module_id.to_string(),
                    trace_id: trace_id.to_string(),
                    code: ModuleCallErrorCode::PolicyDenied,
                    detail: format!(
                        "pure policy hook {} call failed: {}",
                        policy_module_id, failure.detail
                    ),
                })?;
            if hook_output.new_state.is_some() || !hook_output.effects.is_empty() {
                return Err(ModuleCallFailure {
                    module_id: module_id.to_string(),
                    trace_id: trace_id.to_string(),
                    code: ModuleCallErrorCode::PolicyDenied,
                    detail: format!(
                        "pure policy hook {} returned state/effects",
                        policy_module_id
                    ),
                });
            }
            if hook_output.emits.len() > 1 {
                return Err(ModuleCallFailure {
                    module_id: module_id.to_string(),
                    trace_id: trace_id.to_string(),
                    code: ModuleCallErrorCode::PolicyDenied,
                    detail: format!(
                        "pure policy hook {} returned multiple emits",
                        policy_module_id
                    ),
                });
            }
            if let Some(emit) = hook_output.emits.first() {
                match emit.kind.as_str() {
                    "policy.allow" => {}
                    "policy.deny" => {
                        let reason = emit
                            .payload
                            .get("reason")
                            .and_then(|value| value.as_str())
                            .unwrap_or("pure_policy_denied");
                        return Err(ModuleCallFailure {
                            module_id: module_id.to_string(),
                            trace_id: trace_id.to_string(),
                            code: ModuleCallErrorCode::PolicyDenied,
                            detail: format!(
                                "pure policy hook {} denied effect: {}",
                                policy_module_id, reason
                            ),
                        });
                    }
                    other => {
                        return Err(ModuleCallFailure {
                            module_id: module_id.to_string(),
                            trace_id: trace_id.to_string(),
                            code: ModuleCallErrorCode::PolicyDenied,
                            detail: format!(
                                "pure policy hook {} returned unknown emit {}",
                                policy_module_id, other
                            ),
                        });
                    }
                }
            }
        }
        Ok(())
    }
}
