//! Async runner integration for strict continuation admission and wake handoff.

use crate::runtime::AgentContinuation as RuntimeAgentContinuation;
use crate::simulator::Observation;
use crate::simulator::cognition_policy::{
    ContinuationAuthorityContextV1, ContinuationCurrentContextV1, ContinuationHandle,
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
        // Even the explicit compatibility lane must register the proposal in
        // the Harness chain ledger.  Keeping an untracked local handle would
        // let terminal feedback be followed by a fresh admission of the same
        // proposal, bypassing the chain's terminal anti-revival fence.
        let handle = self
            .continuation_harness
            .submit(proposal)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        self.continuations
            .insert(agent_id.to_string(), handle.clone());
        Ok(handle)
    }

    /// Production continuation admission. The proposal is checked against
    /// the current Runtime-derived observation, goal, policy and precondition
    /// digests before it can occupy the agent's continuation slot.
    fn submit_continuation_proposal_using_authority(
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

    /// Compatibility fence: target admission must include the actual current
    /// observation, not just historical digest strings.
    pub fn submit_continuation_proposal_with_context(
        &mut self,
        agent_id: &str,
        proposal: ContinuationProposalV1,
        context: &ContinuationAuthorityContextV1,
    ) -> Result<ContinuationHandle, AsyncAgentRunnerError> {
        let _ = (agent_id, proposal, context);
        Err(AsyncAgentRunnerError::Cognition(
            "current cognition observation is required for target continuation admission"
                .to_string(),
        ))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn submit_continuation_proposal_with_current_context(
        &mut self,
        agent_id: &str,
        proposal: ContinuationProposalV1,
        current: &ContinuationCurrentContextV1,
    ) -> Result<ContinuationHandle, AsyncAgentRunnerError> {
        current
            .validate_for_agent(agent_id)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        self.submit_continuation_proposal_using_authority(agent_id, proposal, &current.authority)
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
        let _ = (agent_id, runtime);
        Err(AsyncAgentRunnerError::Cognition(
            "Runtime projection requires strict continuation admission and current context"
                .to_string(),
        ))
    }

    /// Production Runtime projection path with current cognition-context
    /// revalidation. The reduced compatibility method above remains available
    /// for legacy fixtures, but cannot admit an unverified context update.
    #[cfg(not(target_arch = "wasm32"))]
    fn apply_runtime_continuation_projection_using_authority(
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
    pub fn apply_runtime_continuation_projection_with_context(
        &mut self,
        agent_id: &str,
        runtime: RuntimeAgentContinuation,
        authority_context: &ContinuationAuthorityContextV1,
    ) -> Result<ContinuationHandle, AsyncAgentRunnerError> {
        let _ = (agent_id, runtime, authority_context);
        Err(AsyncAgentRunnerError::Cognition(
            "current cognition observation is required for Runtime projection".to_string(),
        ))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn apply_runtime_continuation_projection_with_current_context(
        &mut self,
        agent_id: &str,
        runtime: RuntimeAgentContinuation,
        current: &ContinuationCurrentContextV1,
    ) -> Result<ContinuationHandle, AsyncAgentRunnerError> {
        current
            .validate_for_agent(agent_id)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        self.apply_runtime_continuation_projection_using_authority(
            agent_id,
            runtime,
            &current.authority,
        )
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
        Self::validate_current_observation(&observation, authority_context, agent_id)?;
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

    fn validate_current_observation(
        observation: &Observation,
        authority_context: &ContinuationAuthorityContextV1,
        agent_id: &str,
    ) -> Result<(), AsyncAgentRunnerError> {
        ContinuationCurrentContextV1 {
            observation: observation.clone(),
            authority: authority_context.clone(),
        }
        .validate_for_agent(agent_id)
        .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))
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
        if request_context.observation_digest.as_str()
            != authority_context.baseline_observation_digest
            || request_context.goal_snapshot_digest.as_str() != authority_context.goal_digest
        {
            return Err(AsyncAgentRunnerError::Cognition(
                "outer request cognition digests do not match the current wake context".to_string(),
            ));
        }
        Self::validate_current_observation(&observation, authority_context, agent_id)?;
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

    /// Production wake consumer: one consumed Runtime wake yields one next
    /// actor turn, while a terminal wake returns without invoking the actor.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn dispatch_ready_cognition_wake(
        &mut self,
        agent_id: &str,
        current: ContinuationCurrentContextV1,
        turn_context: ContinuousAgentTurnContextV1,
        request_context: ContinuousAgentRequestContextV1,
        runtime: RuntimeAgentContinuation,
    ) -> Result<Result<AsyncTurnId, ContinuationHandle>, AsyncAgentRunnerError> {
        current
            .validate_for_agent(agent_id)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        if turn_context.goal_snapshot.digest != current.authority.goal_digest
            || request_context.observation_digest.as_str()
                != current.authority.baseline_observation_digest
            || request_context.goal_snapshot_digest.as_str() != current.authority.goal_digest
        {
            return Err(AsyncAgentRunnerError::Cognition(
                "next cognition turn does not match the current wake context".to_string(),
            ));
        }
        self.resume_consumed_continuation_with_request_context(
            agent_id,
            current.observation,
            turn_context,
            request_context,
            &current.authority,
            runtime,
        )
    }

    /// Rebuild local continuation state in a fresh Agent runner from the
    /// Runtime-owned proposal and status projection.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn hydrate_runtime_continuation(
        &mut self,
        agent_id: &str,
        proposal: ContinuationProposalV1,
        current: &ContinuationCurrentContextV1,
        runtime: RuntimeAgentContinuation,
    ) -> Result<ContinuationHandle, AsyncAgentRunnerError> {
        current
            .validate_for_agent(agent_id)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        if proposal.agent_id != agent_id {
            return Err(AsyncAgentRunnerError::Cognition(
                "hydrated continuation agent identity mismatch".to_string(),
            ));
        }
        if self
            .continuations
            .get(agent_id)
            .is_some_and(|continuation| continuation.active)
        {
            return Err(AsyncAgentRunnerError::AgentBusy(agent_id.to_string()));
        }
        let admitted = self
            .continuation_harness
            .submit_with_context(proposal, &current.authority)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        let handle = self
            .continuation_harness
            .consume_runtime_projection_with_context(admitted, &runtime, &current.authority)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        if handle.active {
            self.continuations
                .insert(agent_id.to_string(), handle.clone());
        }
        Ok(handle)
    }
}
