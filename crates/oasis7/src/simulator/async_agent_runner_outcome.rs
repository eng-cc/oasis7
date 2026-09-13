use crate::runtime::{CognitionLeaseStatusV1, CognitionLeaseV1};

use super::super::agent::{AgentDecision, AgentDecisionTrace};
use super::super::continuous_agent_harness::ContinuousAgentRequestContextV1;
use super::super::types::WorldTime;
use super::budget;
use super::{
    ActorCompletion, AsyncAgentRunnerError, AsyncAgentTurnOutcome, AsyncTurnFeedback,
    AsyncTurnLifecycle, AsyncWorldEffect, Observation,
};

/// Recover the structured provider error emitted by the provider-backed
/// behavior. `AgentBehavior::decide` is intentionally an infallible interface,
/// so provider failures cross the actor boundary in the decision trace.
fn provider_error_code(trace: &AgentDecisionTrace) -> Option<String> {
    let Some(error) = trace.llm_error.as_deref() else {
        return None;
    };

    let structured_code = trace
        .llm_output
        .as_deref()
        .and_then(|payload| serde_json::from_str::<serde_json::Value>(payload).ok())
        .and_then(|payload| payload.get("provider_error").cloned())
        .and_then(|provider_error| provider_error.get("code").cloned())
        .and_then(|code| code.as_str().map(str::to_owned));
    if structured_code.is_some() {
        return structured_code;
    }

    error
        .split_once(':')
        .map(|(code, _)| code.trim())
        .filter(|code| !code.is_empty())
        .map(str::to_owned)
}

pub(super) fn outcome_from_completion(completion: ActorCompletion) -> AsyncAgentTurnOutcome {
    if completion.panicked {
        return AsyncAgentTurnOutcome {
            turn_id: completion.turn_id,
            agent_id: completion.agent_id,
            lifecycle: AsyncTurnLifecycle::Failed,
            feedback: AsyncTurnFeedback::ActorPanicked,
            world_effect: AsyncWorldEffect::NoEffect,
            decision: None,
            decision_trace: None,
            prepared_context: completion.prepared_context,
            prepared_request_context: completion.prepared_request_context,
            prepared_response_context: completion.prepared_response_context,
            cognition_lease: None,
            memory_write_intents: completion.memory_write_intents,
        };
    }
    let completion = match budget::normalize_completion(completion) {
        Ok(completion) => completion,
        Err(outcome) => return outcome,
    };
    if let Some(code) = completion
        .decision_trace
        .as_ref()
        .and_then(provider_error_code)
    {
        return AsyncAgentTurnOutcome {
            turn_id: completion.turn_id,
            agent_id: completion.agent_id,
            lifecycle: AsyncTurnLifecycle::Failed,
            feedback: AsyncTurnFeedback::ProviderError { code },
            world_effect: AsyncWorldEffect::NoEffect,
            decision: None,
            decision_trace: completion.decision_trace,
            prepared_context: completion.prepared_context,
            prepared_request_context: completion.prepared_request_context,
            prepared_response_context: completion.prepared_response_context,
            cognition_lease: None,
            memory_write_intents: completion.memory_write_intents,
        };
    }
    let decision = completion.decision;
    let (feedback, world_effect) = match decision.as_ref() {
        Some(AgentDecision::Wait) => (AsyncTurnFeedback::Wait, AsyncWorldEffect::NoEffect),
        Some(AgentDecision::WaitTicks(ticks)) => (
            AsyncTurnFeedback::WaitTicks(*ticks),
            AsyncWorldEffect::NoEffect,
        ),
        Some(AgentDecision::Act(_)) => (
            AsyncTurnFeedback::ActionProposed,
            AsyncWorldEffect::ActionProposal,
        ),
        Some(AgentDecision::Query(_)) => (
            AsyncTurnFeedback::QueryProposed,
            AsyncWorldEffect::QueryProposal,
        ),
        Some(AgentDecision::ModuleCommand { .. }) => (
            AsyncTurnFeedback::ModuleCommandProposed,
            AsyncWorldEffect::ModuleCommandProposal,
        ),
        None => (
            AsyncTurnFeedback::ProviderError {
                code: "decision_missing".to_string(),
            },
            AsyncWorldEffect::NoEffect,
        ),
    };
    AsyncAgentTurnOutcome {
        turn_id: completion.turn_id,
        agent_id: completion.agent_id,
        lifecycle: AsyncTurnLifecycle::Completed,
        feedback,
        world_effect,
        decision,
        decision_trace: completion.decision_trace,
        prepared_context: completion.prepared_context,
        prepared_request_context: completion.prepared_request_context,
        prepared_response_context: completion.prepared_response_context,
        cognition_lease: None,
        memory_write_intents: completion.memory_write_intents,
    }
}

pub(super) fn default_observation(agent_id: &str, time: WorldTime) -> Observation {
    Observation {
        time,
        agent_id: agent_id.to_string(),
        pos: crate::geometry::GeoPos::new(0, 0, 0),
        self_resources: Default::default(),
        visibility_range_cm: 0,
        visible_agents: Vec::new(),
        visible_locations: Vec::new(),
        module_lifecycle: Default::default(),
        module_market: Default::default(),
        power_market: Default::default(),
        social_state: Default::default(),
    }
}

pub(super) fn validate_cognition_lease_for_request(
    agent_id: &str,
    request_context: &ContinuousAgentRequestContextV1,
    lease: &CognitionLeaseV1,
    logical_tick: WorldTime,
) -> Result<(), AsyncAgentRunnerError> {
    request_context.validate().map_err(|error| {
        AsyncAgentRunnerError::Cognition(format!("cognition_request_invalid: {error}"))
    })?;
    lease.validate().map_err(|error| {
        AsyncAgentRunnerError::Cognition(format!("cognition_lease_invalid: {error}"))
    })?;
    if lease.status != CognitionLeaseStatusV1::Reserved {
        return Err(AsyncAgentRunnerError::Cognition(
            "cognition_lease_not_reserved".to_string(),
        ));
    }
    if lease.agent_id != agent_id
        || request_context.agent_subject != agent_id
        || lease.idempotency_key != request_context.provider_invocation_key().to_string()
        || lease.agent_session_id != request_context.agent_session_id
        || lease.agent_turn_id != request_context.agent_turn_id
        || lease.decision_request_id != request_context.decision_request_id
        || lease.request_digest != request_context.request_digest.to_string()
        || lease.account_id != lease.quote.payer_id
        || lease.quote.resource != "cognition_units"
        || lease.reserved_amount != 1
        || lease.quote.resource_version != crate::runtime::COGNITION_RESOURCE_VERSION_V1
        || lease.quote.purpose != "provider_cognition"
        || lease.quote.scope != "agent_turn"
        || lease.quote.policy_revision
            != crate::runtime::COGNITION_FIXED_UNIT_EXPERIMENTAL_POLICY_REVISION
        || lease.quote.authority_context
            != request_context
                .capability_invocation_context_digest
                .to_string()
        || lease.quote.world_binding != request_context.runtime_binding.base_world_hash.to_string()
    {
        return Err(AsyncAgentRunnerError::Cognition(
            "cognition_lease_identity_mismatch".to_string(),
        ));
    }
    if lease.reserved_at_tick > logical_tick {
        return Err(AsyncAgentRunnerError::Cognition(
            "cognition_lease_reserved_in_future".to_string(),
        ));
    }
    if lease
        .quote
        .valid_until_tick
        .is_some_and(|expires| logical_tick > expires)
    {
        return Err(AsyncAgentRunnerError::Cognition(
            "cognition_lease_expired".to_string(),
        ));
    }
    Ok(())
}
