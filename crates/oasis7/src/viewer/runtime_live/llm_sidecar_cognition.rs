use super::*;
use crate::simulator::{
    BudgetContractV1, CONTINUOUS_AGENT_CONTEXT_DISCRIMINATOR, CONTINUOUS_AGENT_CONTEXT_VERSION,
    ContinuousAgentRequestContextV1, ContinuousAgentTurnContextV1,
    DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION, DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION,
    DecisionRequest, Digest32, Observation, ObservationEnvelope, ProviderInteractionTarget,
    ProviderMissionContext, ProviderNavigationNode, ProviderNearbyEntity, ProviderObservation,
    ProviderRecentEvent, ProviderSelfState, RuntimeBindingV1, h_v1,
};
use oasis7_wasm_abi::{CapabilityCatalogSnapshot, CapabilityPresenter, CapabilitySubject};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[path = "llm_sidecar_cognition_wait.rs"]
mod wait_admission;

const PROVIDER_ADAPTER_PROTOCOL_VERSION: &str = "world-simulator-provider-loopback-http-v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(in crate::viewer::runtime_live) struct ProviderContextState {
    pub(in crate::viewer::runtime_live) turn_context: ContinuousAgentTurnContextV1,
    pub(in crate::viewer::runtime_live) request_context: ContinuousAgentRequestContextV1,
}

#[derive(Clone, Debug)]
struct ProviderCapabilityContext {
    catalog: CapabilityCatalogSnapshot,
    invocation: crate::capability_invocation_context::CapabilityInvocationContext,
    session_id: String,
}

/// Viewer-side seam for the Runtime-owned cognition binding. The viewer does
/// not inspect or synthesize persisted authority fields; Runtime is the sole
/// source of the canonical world/manifest roots and finality lineage.
trait RuntimeBindingSource {
    fn current_runtime_binding(&self, world_id: &str) -> Result<RuntimeBindingV1, String>;
}

impl RuntimeBindingSource for RuntimeWorld {
    fn current_runtime_binding(&self, world_id: &str) -> Result<RuntimeBindingV1, String> {
        let binding = self
            .current_cognition_runtime_binding()
            .map_err(|error| format!("Runtime cognition binding unavailable: {error:?}"))?;
        if binding.world_id != world_id {
            return Err(format!(
                "Runtime cognition binding world_id mismatch: expected {world_id}, got {}",
                binding.world_id
            ));
        }
        binding
            .validate()
            .map_err(|error| format!("Runtime cognition binding invalid: {error}"))?;
        Ok(binding)
    }
}

impl RuntimeLlmSidecar {
    /// Prepare complete Runtime-bound outer requests before an async actor is
    /// started. The actor receives both the reduced behavior context and this
    /// outer V1 request atomically through `AsyncAgentRunner`.
    pub(super) fn prepare_provider_request_contexts(
        &mut self,
        world: &mut RuntimeWorld,
        kernel: &mut WorldKernel,
        world_id: &str,
    ) -> Result<(), String> {
        // The wasm runner uses this path instead of the native async poll;
        // keep durable provider notifications retryable on both lanes.
        self.flush_pending_provider_world_events();
        self.hydrate_provider_lineage(world);
        let provider_settings = provider_settings_from_env()?;
        let runtime_binding = world.current_runtime_binding(world_id)?;
        self.provider_lineage_binding = Some(runtime_binding.clone());
        let recent_event_summary = recent_runtime_event_summaries(world);
        self.release_due_provider_waits(world)?;
        if self.runner.is_none() {
            return Ok(());
        }
        let agent_ids = self
            .provider_agent_ids
            .iter()
            .filter(|agent_id| {
                !self.provider_recovery_pending.contains_key(*agent_id)
                    && !self.provider_active_turns.contains_key(*agent_id)
                    && (self.has_pending_runtime_wake_for_agent(agent_id.as_str())
                        || !self.provider_contexts.contains_key(*agent_id)
                        || self.provider_retry_contexts.contains_key(*agent_id))
            })
            .cloned()
            .collect::<Vec<_>>();

        for agent_id in agent_ids {
            let settings = match provider_settings.as_ref() {
                Some(settings) => settings.clone(),
                None => builtin_cognition_settings(agent_id.as_str())?,
            };
            if !provider_actor_exists(self, agent_id.as_str()) {
                self.quarantine_missing_provider_agent(world, agent_id.as_str());
                continue;
            }
            let observation = kernel.observe(agent_id.as_str());
            let observation = match observation {
                Ok(observation) => observation,
                Err(crate::simulator::RejectReason::AgentNotFound { .. }) => {
                    self.quarantine_missing_provider_agent(world, agent_id.as_str());
                    continue;
                }
                Err(error) => {
                    return Err(format!("provider context observation failed: {error:?}"));
                }
            };
            let replan_cause = self.provider_stale_replan_cause(agent_id.as_str());
            let runtime_wake = self
                .pending_runtime_wakes
                .values()
                .filter(|wake| wake.agent_id == agent_id)
                .min_by_key(|wake| (wake.wake_seq, wake.wake_id.as_str()))
                .cloned();
            if let Some(wake) = runtime_wake.as_ref() {
                let continuation = active_runtime_continuation_for_wake(world, wake)?;
                if continuation.remaining_budget.value == 1 {
                    // A positive simulator proposal cannot represent a final
                    // zero-budget resume. Let Runtime consume that last unit
                    // atomically instead of rejecting the wake after it has
                    // been selected (which would leave a ghost continuation).
                    let runtime_context = runtime_context_digests_for_continuation(
                        world,
                        wake.continuation_id.as_str(),
                    )?;
                    let consumption = world
                        .consume_cognition_continuation_budget_with_context(
                            continuation.continuation_id.as_str(),
                            1,
                            runtime_context.clone(),
                        )
                        .map_err(|error| {
                            format!(
                                "Runtime final continuation budget consumption rejected {}: {error:?}",
                                wake.wake_id
                            )
                        })?;
                    #[cfg(not(target_arch = "wasm32"))]
                    if let Some(runner) = self
                        .runner
                        .as_mut()
                        .and_then(RuntimeDecisionRunner::async_runner_mut)
                    {
                        // Runtime owns the terminal status, budget and status
                        // digest. Carry that exact transition into the
                        // Harness before clearing the Viewer mirrors; a
                        // generic local invalidation would misclassify the
                        // completed continuation as expired.
                        if consumption.status != crate::runtime::ContinuationStatusV1::Completed
                            || consumption.remaining_budget.value != 0
                        {
                            return Err(format!(
                                "Runtime final continuation projection is not terminal for {}",
                                wake.wake_id
                            ));
                        }
                        let mut terminal_projection = continuation.clone();
                        terminal_projection.remaining_budget = consumption.remaining_budget.clone();
                        terminal_projection.status = consumption.status;
                        terminal_projection.logical_tick = world.state().time;
                        terminal_projection.continuation_status_digest =
                            Some(consumption.continuation_status_digest.clone());
                        terminal_projection.terminal_disposition =
                            Some("budget_exhausted".to_string());
                        terminal_projection
                            .validate_authoritative()
                            .map_err(|error| {
                                format!(
                                    "Runtime final continuation projection invalid for {}: {error}",
                                    wake.wake_id
                                )
                            })?;
                        let authority = crate::simulator::ContinuationAuthorityContextV1 {
                            baseline_observation_digest: runtime_context
                                .baseline_observation_digest,
                            goal_digest: runtime_context.goal_digest,
                            policy_digest: runtime_context.policy_digest,
                            precondition_digest: runtime_context.precondition_digest,
                        };
                        runner
                            .apply_runtime_terminal_continuation_projection(
                                agent_id.as_str(),
                                terminal_projection,
                                &authority,
                            )
                            .map_err(|error| {
                                format!(
                                    "Harness final continuation projection failed {}: {error}",
                                    wake.wake_id
                                )
                            })?;
                    }
                    self.pending_runtime_wakes.remove(&wake.wake_id);
                    self.provider_contexts.remove(agent_id.as_str());
                    self.provider_active_turns.remove(agent_id.as_str());
                    self.provider_retry_contexts.remove(agent_id.as_str());
                    self.provider_wait_until.remove(agent_id.as_str());
                    self.persist_provider_lineage_best_effort();
                    continue;
                }
            }
            let context = if runtime_wake.is_none()
                && let Some(mut retry) = self.provider_retry_contexts.remove(&agent_id)
            {
                if retry.request_context.transport_attempt >= MAX_PROVIDER_TRANSPORT_ATTEMPTS {
                    self.provider_transport_exhausted.insert(agent_id.clone());
                    self.persist_provider_lineage_best_effort();
                    continue;
                }
                retry.request_context.transport_attempt =
                    retry.request_context.transport_attempt.saturating_add(1);
                retry
            } else {
                let sequence = self
                    .provider_context_seq
                    .entry(agent_id.clone())
                    .or_insert(1);
                let current_sequence = runtime_wake
                    .as_ref()
                    .map(|wake| {
                        let next = (*sequence).max(wake.retry_seq.saturating_add(1)).max(1);
                        *sequence = next.saturating_add(1);
                        next
                    })
                    .unwrap_or((*sequence).max(1));
                if runtime_wake.is_none() {
                    *sequence = current_sequence.saturating_add(1);
                }
                let capability_context = provider_capability_context(
                    world,
                    &runtime_binding,
                    agent_id.as_str(),
                    current_sequence,
                )?;
                let goal_snapshot =
                    trusted_provider_goal_snapshot(self.prompt_profiles.get(agent_id.as_str()))?;
                let session_id = runtime_wake
                    .as_ref()
                    .map(|_| {
                        format!(
                            "{}-resume-{}",
                            capability_context.session_id, current_sequence
                        )
                    })
                    .unwrap_or_else(|| {
                        self.provider_session_ids
                            .entry(agent_id.clone())
                            .or_insert_with(|| capability_context.session_id.clone())
                            .clone()
                    });
                let (runtime_continuation, runtime_resume_proposal) = runtime_wake
                    .as_ref()
                    .map(|wake| {
                        runtime_continuation_for_wake_with_identity(
                            world,
                            wake,
                            session_id.as_str(),
                            current_sequence,
                        )
                    })
                    .transpose()?
                    .map_or((None, None), |(simulator, runtime)| {
                        (Some(simulator), Some(runtime))
                    });
                let observation_for_context = observation.clone();
                let (turn_context, request_context) = build_provider_context(
                    session_id.as_str(),
                    current_sequence,
                    agent_id.as_str(),
                    observation.clone(),
                    &settings,
                    runtime_binding.clone(),
                    recent_event_summary.as_slice(),
                    capability_context,
                    replan_cause.as_ref(),
                    runtime_continuation.as_ref(),
                    &self.provider_memory_store,
                    goal_snapshot,
                )?;
                if let (Some(wake), Some(proposal)) =
                    (runtime_wake.as_ref(), runtime_resume_proposal)
                {
                    let resume = crate::runtime::CognitionContinuationResumeRequestV1 {
                        agent_session_id: request_context.agent_session_id.clone(),
                        agent_turn_id: request_context.agent_turn_id.clone(),
                        decision_request_id: request_context.decision_request_id.clone(),
                        request_digest: request_context.request_digest.to_string(),
                        context_digest: async_support::runtime_provider_context_digest(
                            &request_context,
                        ),
                    };
                    let current_context =
                        crate::simulator::ContinuationCurrentContextV1::from_observation(
                            observation_for_context,
                            &turn_context.goal_snapshot,
                            provider_policy_context_digest(&request_context),
                            provider_wait_precondition_digest(&observation),
                        );
                    let resumed = match world.resume_cognition_wake_with_context(
                        &wake.wake_id,
                        proposal,
                        1,
                        resume,
                        crate::runtime::CognitionContextDigestsV1 {
                            baseline_observation_digest: current_context
                                .authority
                                .baseline_observation_digest
                                .clone(),
                            goal_digest: current_context.authority.goal_digest.clone(),
                            policy_digest: current_context.authority.policy_digest.clone(),
                            precondition_digest: current_context
                                .authority
                                .precondition_digest
                                .clone(),
                        },
                    ) {
                        Ok(result) => result,
                        Err(error) => {
                            // A stale wake must not remain leased just
                            // because the current-context gate rejected it.
                            // Runtime's terminal handoff is scoped to this
                            // exact wake; local mirrors are then removed for
                            // this Agent only.
                            let _ = world.handoff_cognition_wake_with_context(
                                &wake.wake_id,
                                crate::runtime::CognitionWakeDispositionV1::Terminal {
                                    status: crate::runtime::ContinuationStatusV1::Rejected,
                                    reason: "provider_wake_resume_failed".to_string(),
                                },
                                crate::runtime::CognitionContextDigestsV1 {
                                    baseline_observation_digest: current_context
                                        .authority
                                        .baseline_observation_digest
                                        .clone(),
                                    goal_digest: current_context.authority.goal_digest.clone(),
                                    policy_digest: current_context.authority.policy_digest.clone(),
                                    precondition_digest: current_context
                                        .authority
                                        .precondition_digest
                                        .clone(),
                                },
                            );
                            self.pending_runtime_wakes.remove(&wake.wake_id);
                            self.provider_contexts.remove(agent_id.as_str());
                            self.provider_active_turns.remove(agent_id.as_str());
                            self.provider_retry_contexts.remove(agent_id.as_str());
                            self.provider_wait_until.remove(agent_id.as_str());
                            self.persist_provider_lineage_best_effort();
                            return Err(format!(
                                "Runtime cognition wake resume rejected {}: {error:?}",
                                wake.wake_id
                            ));
                        }
                    };
                    #[cfg(not(target_arch = "wasm32"))]
                    if let Some(runner) = self
                        .runner
                        .as_mut()
                        .and_then(RuntimeDecisionRunner::async_runner_mut)
                    {
                        runner
                            .reconcile_runtime_wake_with_current_context(
                                agent_id.as_str(),
                                &current_context,
                                &resumed.continuation,
                                resumed
                                    .replanned_continuation
                                    .as_ref()
                                    .map(|_| runtime_continuation.clone())
                                    .flatten(),
                            )
                            .map_err(|error| {
                                format!(
                                    "Harness wake reconciliation failed {}: {error}",
                                    wake.wake_id
                                )
                            })?;
                    }
                    // The selected wake has been consumed. Its re-planned
                    // continuation is now Runtime-active and will be selected
                    // by the normal scheduler on a later tick; never retain
                    // the consumed lease in the adapter's mirror.
                    self.pending_runtime_wakes.remove(&wake.wake_id);
                }
                ProviderContextState {
                    turn_context,
                    request_context,
                }
            };
            self.provider_contexts.insert(agent_id.clone(), context);
            if replan_cause.is_some() {
                self.mark_provider_stale_replan_dispatched(agent_id.as_str());
            }
            self.persist_provider_lineage_best_effort();
        }
        Ok(())
    }
}

fn provider_wait_precondition_digest(observation: &Observation) -> String {
    crate::simulator::h_v1("oasis7.cognition.provider-wait-precondition.v1", &{
        let mut stable = observation.clone();
        stable.time = 0;
        stable
    })
    .to_string()
}

fn provider_policy_context_digest(
    request: &crate::simulator::ContinuousAgentRequestContextV1,
) -> String {
    let policy_hash = request
        .base_decision_request
        .capability_catalog
        .as_ref()
        .map(|catalog| catalog.policy_hash.as_str())
        .unwrap_or("missing-provider-policy");
    crate::simulator::h_v1("oasis7.cognition.provider-policy.v1", &policy_hash).to_string()
}

#[cfg(not(target_arch = "wasm32"))]
fn builtin_cognition_settings(agent_id: &str) -> Result<ProviderDecisionSettings, String> {
    let config = LlmAgentConfig::from_default_sources_for_agent(agent_id)
        .map_err(|error| format!("builtin cognition config failed for {agent_id}: {error}"))?;
    Ok(ProviderDecisionSettings {
        requested_provider_mode: "builtin_llm".to_string(),
        provider_transport: "builtin_openai".to_string(),
        base_url: config.base_url,
        auth_token: None,
        connect_timeout_ms: config.timeout_ms.max(1),
        decision_timeout_ms: config.timeout_ms.max(1),
        agent_profile: config.prompt_profile.as_str().to_string(),
        execution_mode: ProviderExecutionMode::HeadlessAgent,
        fallback_reason: None,
    })
}

fn provider_actor_exists(sidecar: &RuntimeLlmSidecar, agent_id: &str) -> bool {
    match sidecar.runner.as_ref() {
        #[cfg(not(target_arch = "wasm32"))]
        Some(RuntimeDecisionRunner::Builtin(runner))
        | Some(RuntimeDecisionRunner::ProviderBacked(runner)) => runner.has_agent(agent_id),
        #[cfg(target_arch = "wasm32")]
        Some(RuntimeDecisionRunner::ProviderBacked(runner)) => runner.get(agent_id).is_some(),
        _ => false,
    }
}

impl RuntimeLlmSidecar {
    fn quarantine_missing_provider_agent(&mut self, world: &mut RuntimeWorld, agent_id: &str) {
        self.provider_agent_ids.remove(agent_id);
        self.provider_session_ids.remove(agent_id);
        self.provider_context_seq.remove(agent_id);
        self.provider_contexts.remove(agent_id);
        self.provider_retry_contexts.remove(agent_id);
        self.provider_active_turns.remove(agent_id);
        self.provider_recovery_pending.remove(agent_id);
        self.provider_wait_until.remove(agent_id);
        self.provider_stale_replans.remove(agent_id);
        self.provider_transport_exhausted.remove(agent_id);
        self.provider_held_decisions.remove(agent_id);

        let wake_ids = self
            .pending_runtime_wakes
            .values()
            .filter(|wake| wake.agent_id == agent_id)
            .map(|wake| wake.wake_id.clone())
            .collect::<Vec<_>>();
        for wake_id in wake_ids {
            // Close only this missing Agent's lease.  A sibling Agent's wake
            // must remain schedulable and must not be consumed as cleanup.
            if let Err(error) = world.handoff_cognition_wake(
                wake_id.as_str(),
                crate::runtime::CognitionWakeDispositionV1::Terminal {
                    status: crate::runtime::ContinuationStatusV1::Rejected,
                    reason: "provider_actor_missing".to_string(),
                },
            ) {
                tracing::warn!(
                    agent_id,
                    wake_id,
                    error = ?error,
                    "missing provider actor wake cleanup failed"
                );
            }
            self.pending_runtime_wakes.remove(wake_id.as_str());
        }
        let event_keys = self
            .pending_provider_world_events
            .iter()
            .filter(|(_, pending)| pending.agent_id == agent_id)
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        for key in event_keys {
            self.pending_provider_world_events.remove(key.as_str());
            self.provider_world_event_quarantine
                .insert(key, format!("provider_actor_missing:{agent_id}"));
        }
        self.persist_provider_lineage_best_effort();
    }
}

fn build_provider_context(
    session_id: &str,
    sequence: u64,
    agent_id: &str,
    observation: Observation,
    settings: &ProviderDecisionSettings,
    runtime_binding: RuntimeBindingV1,
    recent_event_summary: &[String],
    capability_context: ProviderCapabilityContext,
    replan_cause: Option<&ProviderStaleReplanCause>,
    continuation: Option<&SimulatorContinuationProposalV1>,
    memory_store: &MemoryWriteStore,
    goal_snapshot: crate::simulator::GoalSnapshotV1,
) -> Result<
    (
        ContinuousAgentTurnContextV1,
        ContinuousAgentRequestContextV1,
    ),
    String,
> {
    let action_catalog = provider_phase1_action_catalog();
    let memory_snapshot = memory_store.context_snapshot(agent_id, session_id, "session_private", 8);
    let memory_summary =
        provider_memory_summary_with_snapshot(provider_phase1_memory_summary(), &memory_snapshot);
    let mut recent_event_summary = recent_event_summary.to_vec();
    if let Some(cause) = replan_cause {
        recent_event_summary.push(format!(
            "stale_base_replan parent_agent_turn_id={} parent_decision_request_id={} replan_count={}",
            cause.parent_agent_turn_id, cause.parent_decision_request_id, cause.count
        ));
        let keep_from = recent_event_summary.len().saturating_sub(8);
        recent_event_summary.drain(..keep_from);
    }
    let provider_observation = provider_observation_from_runtime_observation(
        &observation,
        settings.execution_mode,
        action_catalog.as_slice(),
        recent_event_summary.as_slice(),
    );
    let capability_catalog_digest = h_v1(
        "oasis7.cognition.capability-catalog.v1",
        &capability_context.catalog,
    );
    let capability_invocation_context_digest = h_v1(
        "oasis7.cognition.capability-invocation-context.v1",
        &capability_context.invocation,
    );
    let base_decision_request = DecisionRequest {
        observation: ObservationEnvelope {
            agent_id: agent_id.to_string(),
            world_time: observation.time,
            mode: settings.execution_mode,
            observation_schema_version: DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION.to_string(),
            action_schema_version: DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION.to_string(),
            environment_class: Some("runtime_live".to_string()),
            fallback_reason: settings.fallback_reason.clone(),
            recent_event_summary: provider_observation
                .recent_events
                .iter()
                .map(|event| event.summary.clone())
                .collect(),
            observation: provider_observation,
            memory_summary: Some(memory_summary),
            action_catalog,
            module_command_catalog: Vec::new(),
            timeout_budget_ms: settings.decision_timeout_ms,
        },
        provider_config_ref: Some(format!(
            "provider://{}/runtime-live/{}",
            settings.provider_transport, agent_id
        )),
        agent_profile: Some(settings.agent_profile.clone()),
        fixture_id: None,
        replay_id: None,
        capability_catalog: Some(capability_context.catalog),
        capability_invocation_context: Some(capability_context.invocation),
        timeout_budget_ms: settings.decision_timeout_ms,
    };
    let agent_turn_id = continuation
        .map(|value| value.agent_turn_id.clone())
        .unwrap_or_else(|| format!("{session_id}-turn-{sequence}"));
    let decision_request_id = continuation
        .map(|value| value.decision_request_id.clone())
        .unwrap_or_else(|| format!("{session_id}-request-{sequence}"));
    let mut request_context = ContinuousAgentRequestContextV1 {
        base_decision_request,
        context_discriminator: CONTINUOUS_AGENT_CONTEXT_DISCRIMINATOR.to_string(),
        context_version: CONTINUOUS_AGENT_CONTEXT_VERSION,
        protocol_version: "oasis7.continuous-agent-request.v1".to_string(),
        agent_session_id: session_id.to_string(),
        agent_turn_id: agent_turn_id.clone(),
        decision_request_id: decision_request_id.clone(),
        retry_seq: sequence,
        transport_attempt: 1,
        agent_subject: agent_id.to_string(),
        runtime_binding,
        observation_digest: h_v1("oasis7.cognition.observation.v1", &Value::Null),
        capability_catalog_digest,
        capability_invocation_context_digest,
        memory_snapshot_digest: Digest32::from(memory_snapshot.digest.clone()),
        goal_snapshot_digest: Digest32::from(goal_snapshot.digest.clone()),
        continuation_digest: Digest32::from(
            continuation
                .map(|value| h_v1("oasis7.cognition.continuation.v1", value))
                .unwrap_or_else(|| h_v1("oasis7.cognition.continuation.v1", &Value::Null)),
        ),
        adapter_protocol_version: PROVIDER_ADAPTER_PROTOCOL_VERSION.to_string(),
        budget_contract: BudgetContractV1 {
            max_latency_ms: settings.decision_timeout_ms,
            max_repair_attempts: 0,
        },
        request_digest: Digest32::default(),
    };
    request_context.observation_digest = h_v1(
        "oasis7.cognition.observation.v1",
        &request_context.base_decision_request.observation,
    );
    request_context.request_digest = request_context.request_digest();
    request_context
        .validate_production_lane()
        .map_err(|error| format!("provider request context invalid: {error}"))?;
    let turn_context = ContinuousAgentTurnContextV1 {
        agent_id: agent_id.to_string(),
        agent_session_id: session_id.to_string(),
        agent_turn_id,
        decision_request_id,
        request_digest: request_context.request_digest.clone(),
        memory_snapshot,
        goal_snapshot,
        continuation: continuation.cloned(),
    };
    turn_context
        .validate_for_agent(agent_id)
        .map_err(|error| format!("provider turn context invalid: {error}"))?;
    Ok((turn_context, request_context))
}

fn trusted_provider_goal_snapshot(
    profile: Option<&AgentPromptProfile>,
) -> Result<crate::simulator::GoalSnapshotV1, String> {
    let revision = profile.map(|profile| profile.version).unwrap_or(0).max(1);
    let short_term_summary = profile
        .and_then(|profile| profile.short_term_goal_override.clone())
        .unwrap_or_else(runtime_live_phase1_short_term_goal);
    let long_term_summary = profile
        .and_then(|profile| profile.long_term_goal_override.clone())
        .unwrap_or_default();
    crate::simulator::GoalSnapshotProjector::project(
        Some(crate::simulator::GoalSnapshotInputV1 {
            revision,
            short_term_summary,
            long_term_summary,
            blocked_reason: None,
            // The Viewer only supplies the host-owned projection. Provider
            // output is never accepted as a goal source.
            provenance: "harness_projection".to_string(),
        }),
        None,
    )
    .map_err(|error| format!("trusted provider goal snapshot invalid: {error}"))
}

fn provider_memory_summary_with_snapshot(
    mut base: String,
    snapshot: &crate::simulator::MemoryContextSnapshotV1,
) -> String {
    if snapshot.entries.is_empty() {
        return base;
    }
    base.push_str("\ncommitted_memory_snapshot:");
    for entry in &snapshot.entries {
        base.push_str("\n- ");
        base.push_str(entry.summary.as_str());
        if !entry.tags.is_empty() {
            base.push_str(" [");
            base.push_str(entry.tags.join(",").as_str());
            base.push(']');
        }
    }
    // Keep the legacy provider request bounded even when all individual
    // memory intents satisfy their per-entry limits.
    const MAX_SUMMARY_BYTES: usize = 4096;
    if base.len() > MAX_SUMMARY_BYTES {
        let mut end = MAX_SUMMARY_BYTES;
        while !base.is_char_boundary(end) {
            end -= 1;
        }
        base.truncate(end);
    }
    base
}

pub(super) fn active_runtime_continuation_for_wake(
    world: &RuntimeWorld,
    wake: &crate::runtime::SchedulerWakeV1,
) -> Result<crate::runtime::AgentContinuation, String> {
    // Runtime owns continuation identity and lifecycle. Keep the provider
    // context overlay below for the simulator-only proposal fields, but never
    // select a continuation from its raw persistence projection.
    world
        .active_cognition_continuations()
        .map_err(|error| format!("Runtime continuation readback failed: {error:?}"))?
        .into_iter()
        .find(|continuation| {
            continuation.continuation_id == wake.continuation_id
                && continuation.wake_id == wake.wake_id
                && continuation.agent_id == wake.agent_id
        })
        .ok_or_else(|| {
            format!(
                "Runtime continuation missing for scheduler wake {}",
                wake.wake_id
            )
        })
}

fn runtime_context_digests_for_continuation(
    world: &RuntimeWorld,
    continuation_id: &str,
) -> Result<crate::runtime::CognitionContextDigestsV1, String> {
    let entry = world
        .cognition()
        .get("continuation_contexts")
        .and_then(Value::as_object)
        .and_then(|contexts| contexts.get(continuation_id))
        .cloned()
        .ok_or_else(|| {
            format!(
                "Runtime continuation context missing for final budget consumption {continuation_id}"
            )
        })?;
    serde_json::from_value(entry).map_err(|error| {
        format!(
            "Runtime continuation context invalid for final budget consumption {continuation_id}: {error}"
        )
    })
}

fn runtime_continuation_for_wake_with_identity(
    world: &RuntimeWorld,
    wake: &crate::runtime::SchedulerWakeV1,
    session_id: &str,
    sequence: u64,
) -> Result<
    (
        SimulatorContinuationProposalV1,
        crate::runtime::CognitionContinuationProposalV1,
    ),
    String,
> {
    let continuation = active_runtime_continuation_for_wake(world, wake)?;
    if continuation.remaining_budget.value <= 1 {
        return Err(format!(
            "Runtime continuation {} has no budget for a resumed request",
            continuation.continuation_id
        ));
    }
    let mut proposal = serde_json::to_value(continuation).map_err(|error| {
        format!(
            "Runtime continuation {} cannot cross provider boundary: {error}",
            wake.continuation_id
        )
    })?;
    if let Some(context) = world
        .cognition()
        .get("continuation_contexts")
        .and_then(Value::as_object)
        .and_then(|contexts| contexts.get(&wake.continuation_id))
        .and_then(Value::as_object)
    {
        for field in [
            "baseline_observation_digest",
            "goal_digest",
            "policy_digest",
            "policy_revision",
            "precondition_summary",
            "precondition_digest",
        ] {
            if let Some(value) = context.get(field) {
                proposal[field] = value.clone();
            }
        }
    }
    proposal["action_or_plan_kind"] = serde_json::json!("continuation_resume");
    // AgentContinuation is the Runtime-owned durable projection and does not
    // retain the adapter source label. Reintroduce the bounded paired-schema
    // field before decoding the resume proposal; Runtime still owns and
    // verifies every identity/digest below.
    proposal["source"] = serde_json::json!("runtime-resume");
    let continuation_proposal_id = format!(
        "{}:resume:{}",
        proposal["continuation_proposal_id"]
            .as_str()
            .unwrap_or("continuation"),
        sequence
    );
    proposal["continuation_proposal_id"] = serde_json::json!(continuation_proposal_id);
    proposal["agent_session_id"] = serde_json::json!(session_id);
    proposal["agent_turn_id"] = serde_json::json!(format!("{session_id}-turn-{sequence}"));
    proposal["decision_request_id"] = serde_json::json!(format!("{session_id}-request-{sequence}"));
    let remaining = proposal
        .get("remaining_budget")
        .and_then(Value::as_object)
        .and_then(|budget| budget.get("value"))
        .and_then(Value::as_u64)
        .ok_or_else(|| "Runtime continuation budget projection is invalid".to_string())?;
    let remaining = remaining
        .checked_sub(1)
        .ok_or_else(|| "Runtime continuation budget is exhausted".to_string())?;
    proposal["remaining_budget"]["value"] = serde_json::json!(remaining);
    proposal["schema_version"] = serde_json::json!(1);
    let mut simulator = serde_json::from_value::<SimulatorContinuationProposalV1>(proposal.clone())
        .map_err(|error| {
            format!(
                "Runtime continuation {} cannot cross provider boundary: {error}",
                wake.continuation_id
            )
        })?;
    simulator.proposal_digest = simulator
        .proposal_digest()
        .map_err(|error| format!("simulator continuation digest failed: {error}"))?
        .to_string();
    let mut runtime =
        serde_json::from_value::<crate::runtime::CognitionContinuationProposalV1>(proposal)
            .map_err(|error| {
                format!(
                    "Runtime continuation {} cannot produce admission proposal: {error}",
                    wake.continuation_id
                )
            })?;
    runtime.proposal_digest = runtime.proposal_digest();
    Ok((simulator, runtime))
}

fn provider_capability_context(
    world: &RuntimeWorld,
    runtime_binding: &RuntimeBindingV1,
    agent_id: &str,
    sequence: u64,
) -> Result<ProviderCapabilityContext, String> {
    // The persisted invocation identifies the governed presenter/session.  A
    // fresh catalog/context pair is then projected by Runtime for this turn;
    // Viewer never reconstructs grants, policy roots, or catalog entries.
    let invocation = world
        .capability_invocation_contexts()
        .values()
        .find(|context| {
            matches!(
                &context.subject,
                CapabilitySubject::Agent { agent_id: subject_id, .. } if subject_id == agent_id
            ) && context.presenter.presenter_kind == "provider"
                && context.audience.world_id == runtime_binding.world_id
                && context.audience.branch_id == runtime_binding.branch_id
                && context.audience.finality_epoch == runtime_binding.finality_epoch
        })
        .cloned()
        .ok_or_else(|| {
            format!(
                "Runtime capability invocation context is unavailable for provider agent {agent_id}"
            )
        })?;
    let presenter: CapabilityPresenter = invocation.presenter.clone();
    let response_nonce = format!(
        "runtime-live:{}:{}:{}",
        agent_id,
        world.state().time,
        sequence
    );
    let (catalog, invocation) = world
        .capability_context_for_agent(agent_id, presenter, response_nonce)
        .map_err(|error| format!("Runtime capability context unavailable: {error:?}"))?;
    if catalog.world_id != runtime_binding.world_id
        || catalog.branch_id != runtime_binding.branch_id
        || catalog.finality_epoch != runtime_binding.finality_epoch
        || catalog.logical_tick != runtime_binding.base_tick
    {
        return Err(
            "Runtime capability context is not bound to the current cognition binding".to_string(),
        );
    }
    let session_id = invocation
        .presenter
        .session_id
        .as_deref()
        .filter(|session_id| !session_id.trim().is_empty())
        .ok_or_else(|| {
            format!(
                "Runtime provider presenter session identity is unavailable for agent {agent_id}"
            )
        })?
        .to_string();
    Ok(ProviderCapabilityContext {
        catalog,
        invocation,
        session_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_continuation_hydration_reads_typed_active_projection() {
        let mut world = RuntimeWorld::new();
        let mut continuation: crate::runtime::AgentContinuation =
            serde_json::from_value(serde_json::json!({
                "schema_version": "agent-continuation.v1",
                "continuation_id": "continuation-typed-readback",
                "wake_id": "wake-typed-readback",
                "world_id": "world-typed-readback",
                "branch_id": "main",
                "finality_epoch": 0,
                "finality_block_hash": null,
                "finality_status": "pending",
                "reorg_epoch": 0,
                "runtime_manifest_hash": "manifest-typed-readback",
                "agent_id": "agent-typed-readback",
                "agent_session_id": "session-typed-readback",
                "agent_turn_id": "turn-typed-readback",
                "decision_request_id": "request-typed-readback",
                "origin_turn_id": "turn-typed-readback",
                "origin_request_digest": "origin-typed-readback",
                "continuation_proposal_id": "proposal-typed-readback",
                "proposal_digest": "proposal-digest-typed-readback",
                "action_or_envelope_digest": null,
                "wake_conditions": [{
                    "schema_version": "wake-condition.v1",
                    "kind": "at_or_after_tick",
                    "logical_tick": 0
                }],
                "next_wake_tick": 0,
                "remaining_budget": {"unit": "steps", "value": 1},
                "valid_until_tick": 10,
                "precondition_digest": "precondition-typed-readback",
                "wake_seq": 1,
                "logical_tick": 0,
                "status": "scheduled",
                "terminal_disposition": null
            }))
            .expect("typed continuation fixture");
        continuation.refresh_status_digest();
        continuation
            .validate_authoritative()
            .expect("typed continuation fixture is authoritative");
        world
            .install_cognition_continuation_for_test(continuation.clone())
            .expect("install typed continuation fixture");
        let wake: crate::runtime::SchedulerWakeV1 = serde_json::from_value(serde_json::json!({
            "schema_version": "scheduler-wake.v1",
            "wake_id": "wake-typed-readback",
            "continuation_id": "continuation-typed-readback",
            "world_id": "world-typed-readback",
            "branch_id": "main",
            "finality_epoch": 0,
            "finality_block_hash": "genesis",
            "finality_status": "pending",
            "reorg_epoch": 0,
            "runtime_manifest_hash": "manifest-typed-readback",
            "agent_id": "agent-typed-readback",
            "agent_session_id": "session-typed-readback",
            "agent_turn_id": "turn-typed-readback",
            "decision_request_id": "request-typed-readback",
            "next_wake_tick": 0,
            "eligible_since_tick": 0,
            "starvation_deadline_tick": 1,
            "initial_priority": 0,
            "wake_seq": 1,
            "retry_seq": 0,
            "status": "pending",
            "pending_reason": "capacity_available"
        }))
        .expect("typed wake fixture");

        assert_eq!(
            active_runtime_continuation_for_wake(&world, &wake)
                .expect("typed active continuation")
                .continuation_id,
            continuation.continuation_id
        );
    }
}

fn provider_observation_from_runtime_observation(
    observation: &Observation,
    mode: ProviderExecutionMode,
    action_catalog: &[ActionCatalogEntry],
    recent_event_summary: &[String],
) -> ProviderObservation {
    let mut locations = observation.visible_locations.clone();
    locations.sort_by(|left, right| {
        left.distance_cm
            .cmp(&right.distance_cm)
            .then_with(|| left.location_id.cmp(&right.location_id))
    });
    let mut agents = observation.visible_agents.clone();
    agents.sort_by(|left, right| {
        left.distance_cm
            .cmp(&right.distance_cm)
            .then_with(|| left.agent_id.cmp(&right.agent_id))
    });
    let current_location = locations
        .iter()
        .find(|location| location.distance_cm == 0)
        .or_else(|| locations.first())
        .map(|location| location.location_id.clone())
        .unwrap_or_else(|| format!("agent:{}:position", observation.agent_id));
    let move_available = action_catalog
        .iter()
        .any(|entry| entry.action_ref == "move_agent");
    let inspect_available = action_catalog
        .iter()
        .any(|entry| entry.action_ref == "inspect_target");
    let speak_available = action_catalog
        .iter()
        .any(|entry| entry.action_ref == "speak_to_nearby");
    let mut nearby_entities = locations
        .iter()
        .map(|location| ProviderNearbyEntity {
            entity_ref: location.location_id.clone(),
            kind: "location".to_string(),
            relation: if location.distance_cm == 0 {
                "current_location".to_string()
            } else {
                "reachable_location".to_string()
            },
            relative_hint: format!(
                "distance_cm={} visible_name={}",
                location.distance_cm.max(0),
                location.name
            ),
            interaction_hint: (location.distance_cm > 0 && move_available)
                .then(|| "move_agent".to_string()),
        })
        .collect::<Vec<_>>();
    nearby_entities.extend(agents.iter().map(|agent| ProviderNearbyEntity {
        entity_ref: agent.agent_id.clone(),
        kind: "agent".to_string(),
        relation: "nearby_agent".to_string(),
        relative_hint: format!("distance_cm={}", agent.distance_cm.max(0)),
        interaction_hint: if speak_available {
            Some("speak_to_nearby".to_string())
        } else if inspect_available {
            Some("inspect_target".to_string())
        } else {
            None
        },
    }));
    let local_navigation_graph = if matches!(mode, ProviderExecutionMode::HeadlessAgent) {
        locations
            .iter()
            .map(|location| ProviderNavigationNode {
                node_ref: location.location_id.clone(),
                relation: if location.distance_cm == 0 {
                    "current_location".to_string()
                } else {
                    "reachable_location".to_string()
                },
                relative_hint: format!(
                    "distance_cm={} visible_name={}",
                    location.distance_cm.max(0),
                    location.name
                ),
                traversable: location.distance_cm >= 0,
            })
            .collect()
    } else {
        Vec::new()
    };
    let interaction_targets = if matches!(mode, ProviderExecutionMode::HeadlessAgent) {
        let mut targets = Vec::new();
        if move_available {
            targets.extend(
                locations
                    .iter()
                    .filter(|location| location.distance_cm > 0)
                    .map(|location| ProviderInteractionTarget {
                        target_ref: location.location_id.clone(),
                        target_kind: "location".to_string(),
                        interaction_hint: "move_agent".to_string(),
                    }),
            );
        }
        if inspect_available {
            targets.extend(agents.iter().map(|agent| ProviderInteractionTarget {
                target_ref: agent.agent_id.clone(),
                target_kind: "agent".to_string(),
                interaction_hint: "inspect_target".to_string(),
            }));
        }
        targets
    } else {
        Vec::new()
    };
    ProviderObservation {
        self_state: ProviderSelfState {
            location_ref: current_location.clone(),
            pose_hint: match mode {
                ProviderExecutionMode::PlayerParity => {
                    format!("player_visible_pose@{current_location}")
                }
                ProviderExecutionMode::HeadlessAgent => format!(
                    "grid_pose=({}, {}, {}) visibility_range_cm={}",
                    observation.pos.x_cm,
                    observation.pos.y_cm,
                    observation.pos.z_cm,
                    observation.visibility_range_cm
                ),
            },
            status_flags: Vec::new(),
            resource_summary: observation
                .self_resources
                .amounts
                .iter()
                .map(|(kind, amount)| (format!("{kind:?}"), *amount))
                .collect(),
        },
        mission_context: ProviderMissionContext {
            goal_summary: match mode {
                ProviderExecutionMode::PlayerParity => {
                    "preserve player-visible forward progress".to_string()
                }
                ProviderExecutionMode::HeadlessAgent => {
                    "preserve deterministic local progress with structured hints".to_string()
                }
            },
            blocked_reason: None,
        },
        nearby_entities,
        recent_events: recent_event_summary
            .iter()
            .rev()
            .enumerate()
            .map(|(index, summary)| ProviderRecentEvent {
                event_ref: format!("recent_event_{index}"),
                kind: "runtime_event_summary".to_string(),
                summary: summary.clone(),
                age_ticks: index as u64,
            })
            .collect(),
        local_navigation_graph,
        hazard_summary: Vec::new(),
        interaction_targets,
    }
}

fn recent_runtime_event_summaries(world: &RuntimeWorld) -> Vec<String> {
    let mut recent = world
        .journal()
        .events
        .iter()
        .rev()
        .take(8)
        .collect::<Vec<_>>();
    recent.reverse();
    recent
        .into_iter()
        .map(|event| {
            let body = serde_json::to_string(&event.body)
                .unwrap_or_else(|_| "<runtime event body unavailable>".to_string());
            format!(
                "runtime_event_id={} time={} body={body}",
                event.id, event.time
            )
        })
        .collect()
}
