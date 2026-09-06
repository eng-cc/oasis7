//! Async runner integration for strict continuation admission and wake handoff.

use crate::runtime::AgentContinuation as RuntimeAgentContinuation;
use crate::simulator::Observation;
use crate::simulator::cognition_policy::{
    ContinuationAuthorityContextV1, ContinuationCurrentContextV1, ContinuationHandle,
    ContinuationInvalidationReason, ContinuationProposalV1, RuntimeContinuationStatusV1,
};
use crate::simulator::continuous_agent_harness::{
    ContinuousAgentRequestContextV1, ContinuousAgentTurnContextV1,
};

use super::{AsyncAgentRunner, AsyncAgentRunnerError, AsyncTurnId};

impl AsyncAgentRunner {
    /// Return the proposal identity currently occupying an Agent's
    /// continuation slot.  Runtime-owned continuation identity is checked by
    /// the Viewer before a restarted wake is allowed to proceed.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn active_continuation_proposal_id(&self, agent_id: &str) -> Option<&str> {
        self.continuations
            .get(agent_id)
            .filter(|continuation| continuation.active)
            .map(|continuation| continuation.proposal.continuation_proposal_id.as_str())
    }

    /// Validate an already-local Harness handle against a fresh Runtime
    /// projection without advancing either local ledger.  This closes the
    /// same-process path's pre-mutation identity check just as hydration does
    /// for a restarted runner.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn validate_active_continuation_with_authority(
        &self,
        agent_id: &str,
        authority: &ContinuationAuthorityContextV1,
        runtime: &RuntimeAgentContinuation,
    ) -> Result<(), AsyncAgentRunnerError> {
        let existing = self
            .continuations
            .get(agent_id)
            .filter(|continuation| continuation.active)
            .cloned()
            .ok_or_else(|| AsyncAgentRunnerError::Cognition("unknown continuation".to_string()))?;
        runtime.validate_authoritative().map_err(|error| {
            AsyncAgentRunnerError::Cognition(format!(
                "Runtime continuation projection invalid: {error}"
            ))
        })?;
        let mut harness = self.continuation_harness.clone();
        harness
            .consume_runtime_projection_with_context(existing, runtime, authority)
            .map(|_| ())
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))
    }

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

    /// Reconcile the exact Runtime wake transition with the Harness chain.
    /// Runtime returns the consumed predecessor and (when budget remains) a
    /// newly admitted continuation proposal. Keeping this transition here
    /// prevents the Viewer from treating a durable wake as resumed before the
    /// Harness has retired the predecessor and admitted the next bounded turn.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reconcile_runtime_wake_with_current_context(
        &mut self,
        agent_id: &str,
        current: &ContinuationCurrentContextV1,
        consumed_runtime: &RuntimeAgentContinuation,
        next_proposal: Option<ContinuationProposalV1>,
    ) -> Result<(), AsyncAgentRunnerError> {
        current
            .validate_for_agent(agent_id)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        let previous_harness = self.continuation_harness.clone();
        let previous_continuations = self.continuations.clone();
        let result = (|| {
            let existing = self.continuations.get(agent_id).cloned().ok_or_else(|| {
                AsyncAgentRunnerError::Cognition("unknown continuation".to_string())
            })?;
            let retired = self
                .continuation_harness
                .advance_ready_wake(existing, consumed_runtime, &current.authority)
                .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
            self.continuations.remove(agent_id);
            if let Some(next_proposal) = next_proposal {
                self.submit_continuation_proposal_with_current_context(
                    agent_id,
                    next_proposal,
                    current,
                )?;
            }
            debug_assert!(!retired.active);
            Ok(())
        })();
        if result.is_err() {
            self.continuation_harness = previous_harness;
            self.continuations = previous_continuations;
        }
        result
    }

    /// Close a locally admitted proposal when Runtime rejects the paired
    /// durable admission. This is an explicit rollback of the Harness
    /// projection; it never claims that Runtime itself was admitted.
    pub fn invalidate_continuation_for_agent(
        &mut self,
        agent_id: &str,
        reason: ContinuationInvalidationReason,
    ) -> Result<(), AsyncAgentRunnerError> {
        let Some(handle) = self.continuations.remove(agent_id) else {
            return Ok(());
        };
        self.continuation_harness
            .invalidate(handle, reason)
            .map(|_| ())
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))
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

    /// Reconcile a terminal Runtime projection after Runtime has already
    /// validated the authoritative context for the exact wake. Terminal
    /// cleanup does not dispatch a new provider turn, so it must not require
    /// a fresh observation whose precondition may have changed after the
    /// preceding committed action.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn apply_runtime_terminal_continuation_projection(
        &mut self,
        agent_id: &str,
        runtime: RuntimeAgentContinuation,
        authority_context: &ContinuationAuthorityContextV1,
    ) -> Result<ContinuationHandle, AsyncAgentRunnerError> {
        if !matches!(
            runtime.status,
            crate::runtime::ContinuationStatusV1::Completed
                | crate::runtime::ContinuationStatusV1::Cancelled
                | crate::runtime::ContinuationStatusV1::Invalidated
                | crate::runtime::ContinuationStatusV1::Expired
                | crate::runtime::ContinuationStatusV1::Rejected
        ) {
            return Err(AsyncAgentRunnerError::Cognition(
                "Runtime terminal continuation projection has a non-terminal status".to_string(),
            ));
        }
        self.apply_runtime_continuation_projection_using_authority(
            agent_id,
            runtime,
            authority_context,
        )
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

    /// Apply the Runtime wake transition and the next actor dispatch as one
    /// local state transaction. The actor send path is fallible (for example
    /// when its mailbox is full or the actor is unavailable), so a failed
    /// dispatch must restore the Harness, continuation, feedback and turn
    /// counters exactly as they were before the wake was retired.
    #[cfg(not(target_arch = "wasm32"))]
    fn retire_and_dispatch_continuation<F>(
        &mut self,
        agent_id: &str,
        turn_context: &ContinuousAgentTurnContextV1,
        authority_context: &ContinuationAuthorityContextV1,
        runtime: &RuntimeAgentContinuation,
        dispatch: F,
    ) -> Result<Result<AsyncTurnId, ContinuationHandle>, AsyncAgentRunnerError>
    where
        F: FnOnce(&mut Self) -> Result<AsyncTurnId, AsyncAgentRunnerError>,
    {
        let previous_harness = self.continuation_harness.clone();
        let previous_continuations = self.continuations.clone();
        let previous_feedback_store = self.feedback_store.clone();
        let previous_next_turn_id = self.next_turn_id;
        let previous_active_turns = self.active_turns;
        let result = (|| {
            let (consumed, projection) =
                self.retire_ready_continuation(agent_id, turn_context, authority_context, runtime)?;
            if !consumed {
                return Ok(Err(projection));
            }
            dispatch(self).map(Ok)
        })();
        match result {
            Ok(result) => Ok(result),
            Err(error) => {
                self.continuation_harness = previous_harness;
                self.continuations = previous_continuations;
                self.feedback_store = previous_feedback_store;
                self.next_turn_id = previous_next_turn_id;
                self.active_turns = previous_active_turns;
                Err(error)
            }
        }
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
        let dispatch_turn_context = turn_context.clone();
        self.retire_and_dispatch_continuation(
            agent_id,
            &turn_context,
            authority_context,
            &runtime,
            |runner| {
                runner.start_turn_with_context_and_observation_and_request(
                    agent_id,
                    observation,
                    Some(dispatch_turn_context),
                    None,
                )
            },
        )
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
        let dispatch_turn_context = turn_context.clone();
        self.retire_and_dispatch_continuation(
            agent_id,
            &turn_context,
            authority_context,
            &runtime,
            |runner| {
                runner.start_turn_with_context_and_observation_and_request(
                    agent_id,
                    observation,
                    Some(dispatch_turn_context),
                    Some(request_context),
                )
            },
        )
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

    /// Rebuild a fresh runner's exact Harness continuation from the
    /// Runtime-owned projection.  The Harness is cloned and all validation is
    /// performed against the clone before replacing local state, so a stale or
    /// mismatched restart checkpoint cannot partially mutate the new runner.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn hydrate_runtime_continuation_with_authority(
        &mut self,
        agent_id: &str,
        proposal: ContinuationProposalV1,
        authority: &ContinuationAuthorityContextV1,
        runtime: RuntimeAgentContinuation,
    ) -> Result<ContinuationHandle, AsyncAgentRunnerError> {
        if !self.actors.contains_key(agent_id) {
            return Err(AsyncAgentRunnerError::AgentNotRegistered(
                agent_id.to_string(),
            ));
        }
        if proposal.agent_id != agent_id || runtime.agent_id != agent_id {
            return Err(AsyncAgentRunnerError::Cognition(
                "hydrated continuation agent identity mismatch".to_string(),
            ));
        }
        if self.active_continuation_proposal_id(agent_id).is_some() {
            return Err(AsyncAgentRunnerError::AgentBusy(agent_id.to_string()));
        }
        runtime.validate_authoritative().map_err(|error| {
            AsyncAgentRunnerError::Cognition(format!(
                "Runtime continuation projection invalid: {error}"
            ))
        })?;
        authority
            .validate_proposal(&proposal)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;

        let mut harness = self.continuation_harness.clone();
        let admitted = harness
            .submit_with_context(proposal, authority)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        let handle = harness
            .consume_runtime_projection_with_context(admitted, &runtime, authority)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;

        self.continuation_harness = harness;
        if handle.active {
            self.continuations
                .insert(agent_id.to_string(), handle.clone());
        } else {
            self.continuations.remove(agent_id);
        }
        Ok(handle)
    }
}
