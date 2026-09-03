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
        self.hydrate_provider_lineage(world);
        let settings = provider_settings_from_env()?.ok_or_else(|| {
            "provider settings disappeared before context preparation".to_string()
        })?;
        let runtime_binding = world.current_runtime_binding(world_id)?;
        self.provider_lineage_binding = Some(runtime_binding.clone());
        let recent_event_summary = recent_runtime_event_summaries(world);
        self.release_due_provider_waits(world)?;
        if !matches!(self.runner, Some(RuntimeDecisionRunner::ProviderBacked(_))) {
            return Ok(());
        }
        let agent_ids = self
            .provider_agent_ids
            .iter()
            .filter(|agent_id| {
                !self.provider_active_turns.contains_key(*agent_id)
                    && (self.has_pending_runtime_wake_for_agent(agent_id.as_str())
                        || !self.provider_contexts.contains_key(*agent_id)
                        || self.provider_retry_contexts.contains_key(*agent_id))
            })
            .cloned()
            .collect::<Vec<_>>();

        for agent_id in agent_ids {
            let observation = kernel
                .observe(agent_id.as_str())
                .map_err(|error| format!("provider context observation failed: {error:?}"))?;
            let replan_cause = self.provider_stale_replan_cause(agent_id.as_str());
            let runtime_wake = self
                .pending_runtime_wakes
                .values()
                .filter(|wake| wake.agent_id == agent_id)
                .min_by_key(|wake| (wake.wake_seq, wake.wake_id.as_str()))
                .cloned();
            let runtime_continuation = runtime_wake
                .as_ref()
                .map(|wake| runtime_continuation_for_wake(world, wake))
                .transpose()?;
            let context = if runtime_wake.is_none()
                && let Some(mut retry) = self.provider_retry_contexts.remove(&agent_id)
            {
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
                    .map(|wake| wake.retry_seq.max(1))
                    .unwrap_or((*sequence).max(1));
                if runtime_wake.is_none() {
                    *sequence = current_sequence.saturating_add(1);
                } else {
                    *sequence = (*sequence).max(current_sequence.saturating_add(1));
                }
                let capability_context = provider_capability_context(
                    world,
                    &runtime_binding,
                    agent_id.as_str(),
                    current_sequence,
                )?;
                let session_id = runtime_continuation
                    .as_ref()
                    .map(|continuation| continuation.agent_session_id.clone())
                    .unwrap_or_else(|| capability_context.session_id.clone());
                let session_id = self
                    .provider_session_ids
                    .entry(agent_id.clone())
                    .or_insert_with(|| session_id.clone());
                if runtime_continuation.is_some() {
                    *session_id = runtime_continuation
                        .as_ref()
                        .map(|continuation| continuation.agent_session_id.clone())
                        .unwrap_or_else(|| session_id.clone());
                }
                let (turn_context, request_context) = build_provider_context(
                    session_id.as_str(),
                    current_sequence,
                    agent_id.as_str(),
                    observation,
                    &settings,
                    runtime_binding.clone(),
                    recent_event_summary.as_slice(),
                    capability_context,
                    replan_cause.as_ref(),
                    runtime_continuation.as_ref(),
                )?;
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
) -> Result<
    (
        ContinuousAgentTurnContextV1,
        ContinuousAgentRequestContextV1,
    ),
    String,
> {
    let action_catalog = provider_phase1_action_catalog();
    let memory_summary = provider_phase1_memory_summary();
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
    let memory_snapshot =
        crate::simulator::MemoryContextSnapshotV1::empty(format!("agent:{agent_id}"));
    let goal_snapshot = crate::simulator::GoalSnapshotV1::empty();
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

fn runtime_continuation_for_wake(
    world: &RuntimeWorld,
    wake: &crate::runtime::SchedulerWakeV1,
) -> Result<SimulatorContinuationProposalV1, String> {
    let continuation = active_runtime_continuation_for_wake(world, wake)?;
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
    proposal["schema_version"] = serde_json::json!(1);
    serde_json::from_value(proposal).map_err(|error| {
        format!(
            "Runtime continuation {} cannot cross provider boundary: {error}",
            wake.continuation_id
        )
    })
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
