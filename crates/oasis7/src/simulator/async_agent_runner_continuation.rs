//! Async runner integration for strict continuation admission and wake handoff.

use crate::runtime::AgentContinuation as RuntimeAgentContinuation;
use crate::simulator::Observation;
use crate::simulator::cognition_policy::{
    ContinuationAuthorityContextV1, ContinuationBudgetV1, ContinuationHandle,
    ContinuationProposalV1, RuntimeContinuationStatusV1,
};
use crate::simulator::continuous_agent_harness::{
    ContinuousAgentRequestContextV1, ContinuousAgentTurnContextV1,
};

use super::{AsyncAgentRunner, AsyncAgentRunnerError, AsyncTurnId};

impl AsyncAgentRunner {
    pub fn submit_continuation_proposal(
        &mut self,
        agent_id: &str,
        proposal: ContinuationProposalV1,
    ) -> Result<ContinuationHandle, AsyncAgentRunnerError> {
        if !self.actors.contains_key(agent_id) {
            return Err(AsyncAgentRunnerError::AgentNotRegistered(
                agent_id.to_string(),
            ));
        }
        if proposal.agent_id != agent_id {
            return Err(AsyncAgentRunnerError::Cognition(
                "continuation proposal agent identity mismatch".to_string(),
            ));
        }
        proposal
            .validate()
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        if self
            .continuations
            .get(agent_id)
            .is_some_and(|continuation| continuation.active)
        {
            return Err(AsyncAgentRunnerError::AgentBusy(agent_id.to_string()));
        }
        let remaining_budget = proposal.remaining_budget.clone();
        let handle = ContinuationHandle {
            proposal,
            chain_id: String::new(),
            continuation_id: String::new(),
            wake_id: String::new(),
            wake_seq: 0,
            continuation_digest: String::new(),
            continuation_status_digest: String::new(),
            status: "scheduled".to_string(),
            terminal_disposition: None,
            active: true,
            provenance: "harness_policy".to_string(),
            world_effect: false,
            provider_invocation_count: 0,
            remaining_budget,
            consumed_budget: 0,
        };
        self.continuations
            .insert(agent_id.to_string(), handle.clone());
        Ok(handle)
    }

    /// Production continuation admission. The proposal is checked against
    /// the current Runtime-derived observation, goal, policy and precondition
    /// digests before it can occupy the agent's continuation slot.
    pub fn submit_continuation_proposal_with_context(
        &mut self,
        agent_id: &str,
        proposal: ContinuationProposalV1,
        context: &ContinuationAuthorityContextV1,
    ) -> Result<ContinuationHandle, AsyncAgentRunnerError> {
        if !self.actors.contains_key(agent_id) {
            return Err(AsyncAgentRunnerError::AgentNotRegistered(
                agent_id.to_string(),
            ));
        }
        if proposal.agent_id != agent_id {
            return Err(AsyncAgentRunnerError::Cognition(
                "continuation proposal agent identity mismatch".to_string(),
            ));
        }
        if self
            .continuations
            .get(agent_id)
            .is_some_and(|continuation| continuation.active)
        {
            return Err(AsyncAgentRunnerError::AgentBusy(agent_id.to_string()));
        }
        let handle = self
            .continuation_harness
            .submit_with_context(proposal, context)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        self.continuations
            .insert(agent_id.to_string(), handle.clone());
        Ok(handle)
    }

    pub fn apply_runtime_continuation_status(
        &mut self,
        _agent_id: &str,
        runtime: RuntimeContinuationStatusV1,
    ) -> Result<ContinuationHandle, AsyncAgentRunnerError> {
        let _ = runtime;
        Err(AsyncAgentRunnerError::Cognition(
            "legacy Runtime continuation status lacks an authoritative projection".to_string(),
        ))
    }

    /// Apply a complete Runtime-owned continuation projection.  Runtime, not
    /// this runner, allocates schedule IDs, sequence and status digests.
    pub fn apply_runtime_continuation_projection(
        &mut self,
        agent_id: &str,
        runtime: RuntimeAgentContinuation,
    ) -> Result<ContinuationHandle, AsyncAgentRunnerError> {
        let existing =
            self.continuations.get(agent_id).cloned().ok_or_else(|| {
                AsyncAgentRunnerError::Cognition("unknown continuation".to_string())
            })?;
        if !existing.chain_id.is_empty() {
            let handle = self
                .continuation_harness
                .consume_runtime_projection(existing, &runtime)
                .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
            if handle.active {
                self.continuations
                    .insert(agent_id.to_string(), handle.clone());
            } else {
                self.continuations.remove(agent_id);
            }
            return Ok(handle);
        }
        runtime
            .validate_authoritative()
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        if runtime.continuation_proposal_id != existing.proposal.continuation_proposal_id
            || runtime.proposal_digest != existing.proposal.proposal_digest
            || runtime.agent_id != existing.proposal.agent_id
            || runtime.agent_session_id != existing.proposal.agent_session_id
            || runtime.agent_turn_id != existing.proposal.agent_turn_id
            || runtime.decision_request_id != existing.proposal.decision_request_id
            || runtime.origin_turn_id != existing.proposal.origin_turn_id
            || runtime.origin_request_digest != existing.proposal.origin_request_digest
        {
            return Err(AsyncAgentRunnerError::Cognition(
                "Runtime continuation projection correlation mismatch".to_string(),
            ));
        }
        let (status, active) = match runtime.status {
            crate::runtime::ContinuationStatusV1::Scheduled => ("scheduled", true),
            crate::runtime::ContinuationStatusV1::Pending => ("pending", true),
            crate::runtime::ContinuationStatusV1::Waking => ("waking", true),
            crate::runtime::ContinuationStatusV1::Consumed => ("consumed", true),
            crate::runtime::ContinuationStatusV1::Completed => ("completed", false),
            crate::runtime::ContinuationStatusV1::Cancelled => ("cancelled", false),
            crate::runtime::ContinuationStatusV1::Invalidated => ("invalidated", false),
            crate::runtime::ContinuationStatusV1::Expired => ("expired", false),
            crate::runtime::ContinuationStatusV1::Rejected => ("rejected", false),
        };
        let handle = ContinuationHandle {
            proposal: existing.proposal,
            chain_id: existing.chain_id,
            continuation_id: runtime.continuation_id.clone(),
            wake_id: runtime.wake_id.clone(),
            wake_seq: runtime.wake_seq,
            continuation_digest: runtime.continuation_digest(),
            continuation_status_digest: runtime
                .continuation_status_digest
                .clone()
                .expect("validated Runtime continuation has a status digest"),
            status: status.to_string(),
            terminal_disposition: runtime.terminal_disposition.clone(),
            active,
            provenance: "runtime_authoritative".to_string(),
            world_effect: false,
            provider_invocation_count: 0,
            remaining_budget: ContinuationBudgetV1 {
                unit: runtime.remaining_budget.unit.clone(),
                value: runtime.remaining_budget.value,
            },
            consumed_budget: existing.consumed_budget,
        };
        if active {
            self.continuations
                .insert(agent_id.to_string(), handle.clone());
        } else {
            self.continuations.remove(agent_id);
        }
        Ok(handle)
    }

    /// Production Runtime projection path with current cognition-context
    /// revalidation. The reduced compatibility method above remains available
    /// for legacy fixtures, but cannot admit an unverified context update.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn apply_runtime_continuation_projection_with_context(
        &mut self,
        agent_id: &str,
        runtime: RuntimeAgentContinuation,
        authority_context: &ContinuationAuthorityContextV1,
    ) -> Result<ContinuationHandle, AsyncAgentRunnerError> {
        let existing =
            self.continuations.get(agent_id).cloned().ok_or_else(|| {
                AsyncAgentRunnerError::Cognition("unknown continuation".to_string())
            })?;
        if existing.chain_id.is_empty() {
            return Err(AsyncAgentRunnerError::Cognition(
                "Runtime projection requires strict continuation admission".to_string(),
            ));
        }
        let handle = self
            .continuation_harness
            .consume_runtime_projection_with_context(existing, &runtime, authority_context)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        if handle.active {
            self.continuations
                .insert(agent_id.to_string(), handle.clone());
        } else {
            self.continuations.remove(agent_id);
        }
        Ok(handle)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn retire_ready_continuation(
        &mut self,
        agent_id: &str,
        turn_context: &ContinuousAgentTurnContextV1,
        authority_context: &ContinuationAuthorityContextV1,
        runtime: &RuntimeAgentContinuation,
    ) -> Result<(bool, ContinuationHandle), AsyncAgentRunnerError> {
        if !self.actors.contains_key(agent_id) {
            return Err(AsyncAgentRunnerError::AgentNotRegistered(
                agent_id.to_string(),
            ));
        }
        turn_context
            .validate_for_agent(agent_id)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        if turn_context.goal_snapshot.digest != authority_context.goal_digest {
            return Err(AsyncAgentRunnerError::Cognition(
                "continuation next-turn goal does not match the authoritative context".to_string(),
            ));
        }
        let existing =
            self.continuations.get(agent_id).cloned().ok_or_else(|| {
                AsyncAgentRunnerError::Cognition("unknown continuation".to_string())
            })?;
        let projection = self
            .continuation_harness
            .advance_ready_wake(existing, runtime, authority_context)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        self.continuations.remove(agent_id);
        Ok((
            runtime.status == crate::runtime::ContinuationStatusV1::Consumed,
            projection,
        ))
    }

    /// Revalidate a consumed Runtime wake and dispatch exactly one new actor
    /// turn for replan/next action. A terminal Runtime projection returns its
    /// terminal handle and does not invoke the actor. A lease acknowledgement
    /// or pending wake cannot enter this path.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn resume_consumed_continuation(
        &mut self,
        agent_id: &str,
        observation: Observation,
        turn_context: ContinuousAgentTurnContextV1,
        authority_context: &ContinuationAuthorityContextV1,
        runtime: RuntimeAgentContinuation,
    ) -> Result<Result<AsyncTurnId, ContinuationHandle>, AsyncAgentRunnerError> {
        let (consumed, projection) =
            self.retire_ready_continuation(agent_id, &turn_context, authority_context, &runtime)?;
        if !consumed {
            return Ok(Err(projection));
        }
        let next_turn = self.start_turn_with_context_and_observation_and_request(
            agent_id,
            observation,
            Some(turn_context),
            None,
        )?;
        Ok(Ok(next_turn))
    }

    /// Production variant retaining the complete outer request lineage for
    /// the resumed turn. It shares the same strict wake retirement path, then
    /// enters the normal request-context correlation checks before dispatch.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn resume_consumed_continuation_with_request_context(
        &mut self,
        agent_id: &str,
        observation: Observation,
        turn_context: ContinuousAgentTurnContextV1,
        request_context: ContinuousAgentRequestContextV1,
        authority_context: &ContinuationAuthorityContextV1,
        runtime: RuntimeAgentContinuation,
    ) -> Result<Result<AsyncTurnId, ContinuationHandle>, AsyncAgentRunnerError> {
        turn_context
            .validate_for_agent(agent_id)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        request_context
            .validate_production_lane()
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        if request_context.agent_subject != agent_id
            || request_context.agent_session_id != turn_context.agent_session_id
            || request_context.agent_turn_id != turn_context.agent_turn_id
            || request_context.decision_request_id != turn_context.decision_request_id
            || request_context.request_digest != turn_context.request_digest
        {
            return Err(AsyncAgentRunnerError::Cognition(
                "outer and reduced cognition contexts do not correlate".to_string(),
            ));
        }
        let (consumed, projection) =
            self.retire_ready_continuation(agent_id, &turn_context, authority_context, &runtime)?;
        if !consumed {
            return Ok(Err(projection));
        }
        let next_turn = self.start_turn_with_context_and_observation_and_request(
            agent_id,
            observation,
            Some(turn_context),
            Some(request_context),
        )?;
        Ok(Ok(next_turn))
    }
}
