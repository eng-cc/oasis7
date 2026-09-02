use super::*;
use crate::simulator::{
    AgentBehavior, BudgetContractV1, CONTINUOUS_AGENT_CONTEXT_DISCRIMINATOR,
    CONTINUOUS_AGENT_CONTEXT_VERSION, ContinuousAgentRequestContextV1,
    ContinuousAgentTurnContextV1, DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION,
    DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION, DecisionRequest, Digest32, Observation,
    ObservationEnvelope, ProviderObservation, RuntimeBindingV1, h_v1,
};
use serde_json::Value;

const PROVIDER_ADAPTER_PROTOCOL_VERSION: &str = "world-simulator-provider-loopback-http-v1";

impl RuntimeLlmSidecar {
    /// Install a complete, Runtime-bound outer request on every provider
    /// behavior before the synchronous compatibility scheduler invokes it.
    /// The scheduler API currently exposes only the legacy tick entrypoint;
    /// this viewer adapter therefore uses the public AgentBehavior context
    /// hooks while retaining the provider's full request/response path.
    pub(super) fn prepare_provider_request_contexts(
        &mut self,
        world: &RuntimeWorld,
        kernel: &mut WorldKernel,
        world_id: &str,
    ) -> Result<(), String> {
        let settings = provider_settings_from_env()?.ok_or_else(|| {
            "provider settings disappeared before context preparation".to_string()
        })?;
        let state_hash = world
            .current_state_root_hash()
            .map_err(|error| format!("runtime state binding unavailable: {error:?}"))?;
        let manifest_hash = world
            .current_manifest_hash()
            .map_err(|error| format!("runtime manifest binding unavailable: {error:?}"))?;
        let base_world_hash = h_v1("oasis7.runtime.world-state.v1", &state_hash);
        let runtime_manifest_hash = h_v1("oasis7.runtime.manifest.v1", &manifest_hash);
        let agent_ids = match self.runner.as_ref() {
            Some(RuntimeDecisionRunner::ProviderBacked(runner)) => runner.agent_ids(),
            _ => return Ok(()),
        };

        for agent_id in agent_ids {
            let observation = kernel
                .observe(agent_id.as_str())
                .map_err(|error| format!("provider context observation failed: {error:?}"))?;
            let sequence = self.provider_context_seq.max(1);
            self.provider_context_seq = sequence.saturating_add(1);
            let (turn_context, request_context) = build_provider_context(
                self.provider_session_id.as_str(),
                sequence,
                world,
                world_id,
                agent_id.as_str(),
                observation,
                &settings,
                base_world_hash.clone(),
                runtime_manifest_hash.clone(),
            )?;
            let Some(RuntimeDecisionRunner::ProviderBacked(runner)) = self.runner.as_mut() else {
                return Err("provider runner disappeared during context preparation".to_string());
            };
            let Some(agent) = runner.get_mut(agent_id.as_str()) else {
                return Err(format!("provider agent disappeared: {agent_id}"));
            };
            agent
                .behavior
                .set_continuous_turn_context(Some(&turn_context));
            agent
                .behavior
                .set_continuous_request_context(Some(&request_context));
        }
        Ok(())
    }
}

fn build_provider_context(
    session_id: &str,
    sequence: u64,
    world: &RuntimeWorld,
    world_id: &str,
    agent_id: &str,
    observation: Observation,
    settings: &ProviderDecisionSettings,
    base_world_hash: Digest32,
    runtime_manifest_hash: Digest32,
) -> Result<
    (
        ContinuousAgentTurnContextV1,
        ContinuousAgentRequestContextV1,
    ),
    String,
> {
    let action_catalog = provider_phase1_action_catalog();
    let memory_summary = provider_phase1_memory_summary();
    let base_decision_request = DecisionRequest {
        observation: ObservationEnvelope {
            agent_id: agent_id.to_string(),
            world_time: observation.time,
            mode: settings.execution_mode,
            observation_schema_version: DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION.to_string(),
            action_schema_version: DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION.to_string(),
            environment_class: Some("runtime_live".to_string()),
            fallback_reason: settings.fallback_reason.clone(),
            observation: ProviderObservation::default(),
            recent_event_summary: Vec::new(),
            memory_summary: Some(memory_summary),
            action_catalog,
            module_command_catalog: Vec::new(),
            timeout_budget_ms: settings.decision_timeout_ms,
        },
        provider_config_ref: Some(format!(
            "provider://{}/runtime-live/pid-{}/{}",
            settings.provider_transport,
            std::process::id(),
            agent_id
        )),
        agent_profile: Some(settings.agent_profile.clone()),
        fixture_id: None,
        replay_id: None,
        capability_catalog: None,
        capability_invocation_context: None,
        timeout_budget_ms: settings.decision_timeout_ms,
    };
    let memory_snapshot =
        crate::simulator::MemoryContextSnapshotV1::empty(format!("agent:{agent_id}"));
    let goal_snapshot = crate::simulator::GoalSnapshotV1::empty();
    let agent_turn_id = format!("{session_id}-turn-{sequence}");
    let decision_request_id = format!("{session_id}-request-{sequence}");
    let runtime_binding = RuntimeBindingV1 {
        world_id: world_id.to_string(),
        branch_id: "main".to_string(),
        finality_epoch: 0,
        finality_block_hash: None,
        finality_status: "pending".to_string(),
        base_tick: world.state().time,
        base_world_hash,
        reorg_epoch: 0,
        runtime_manifest_hash,
    };
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
        capability_catalog_digest: h_v1("oasis7.cognition.capability-catalog.v1", &Value::Null),
        capability_invocation_context_digest: h_v1(
            "oasis7.cognition.capability-invocation-context.v1",
            &Value::Null,
        ),
        memory_snapshot_digest: Digest32::from(memory_snapshot.digest.clone()),
        goal_snapshot_digest: Digest32::from(goal_snapshot.digest.clone()),
        continuation_digest: h_v1("oasis7.cognition.continuation.v1", &Value::Null),
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
        continuation: None,
    };
    turn_context
        .validate_for_agent(agent_id)
        .map_err(|error| format!("provider turn context invalid: {error}"))?;
    Ok((turn_context, request_context))
}
