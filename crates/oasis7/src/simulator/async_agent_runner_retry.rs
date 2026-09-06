use std::sync::atomic::Ordering;

use super::*;

impl AsyncAgentRunner {
    /// Retry a provider transport for a turn whose actor outcome is awaiting
    /// Runtime disposition. The semantic request and reduced turn context
    /// must be the exact awaiting values; this method creates the next wire
    /// attempt by changing only `transport_attempt`, keeps the original
    /// [`AsyncTurnId`], and does not re-admit the request to the feedback
    /// single-flight store.
    ///
    /// Callers pass the previously accepted request context (with its current
    /// transport attempt). A successful retry increments that value by one
    /// before dispatch. A fresh observation is accepted for the actor, but it
    /// cannot alter the outer request identity or its Runtime binding.
    pub fn retry_awaiting_turn_with_request_context_and_observation(
        &mut self,
        agent_id: &str,
        observation: Observation,
        context: ContinuousAgentTurnContextV1,
        request_context: ContinuousAgentRequestContextV1,
    ) -> Result<AsyncTurnId, AsyncAgentRunnerError> {
        context
            .validate_for_agent(agent_id)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        request_context
            .validate_production_lane()
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        if request_context.agent_subject != agent_id {
            return Err(AsyncAgentRunnerError::Cognition(
                "retry request subject does not match the actor".to_string(),
            ));
        }
        if observation.agent_id != agent_id {
            return Err(AsyncAgentRunnerError::AgentIdentityMismatch {
                expected: agent_id.to_string(),
                observed: observation.agent_id,
            });
        }

        let Some(&turn_id) = self.awaiting_runtime.get(agent_id) else {
            return Err(AsyncAgentRunnerError::Cognition(
                "retry requires an active awaiting Runtime turn".to_string(),
            ));
        };
        let Some(outcome) = self.awaiting_outcomes.get(&turn_id) else {
            return Err(AsyncAgentRunnerError::Cognition(
                "awaiting Runtime turn identity is unavailable".to_string(),
            ));
        };
        let expected_context = outcome.prepared_context.clone().ok_or_else(|| {
            AsyncAgentRunnerError::Cognition(
                "retry requires the awaiting turn's cognition context".to_string(),
            )
        })?;
        let expected_request_context =
            outcome.prepared_request_context.clone().ok_or_else(|| {
                AsyncAgentRunnerError::Cognition(
                    "retry requires the awaiting turn's outer request context".to_string(),
                )
            })?;
        if context != expected_context {
            return Err(AsyncAgentRunnerError::Cognition(
                "retry cognition identity does not match the awaiting turn".to_string(),
            ));
        }
        if request_context != expected_request_context {
            return Err(AsyncAgentRunnerError::Cognition(
                "retry request identity does not match the awaiting turn".to_string(),
            ));
        }
        let next_transport_attempt = request_context
            .transport_attempt
            .checked_add(1)
            .ok_or_else(|| {
                AsyncAgentRunnerError::Cognition(
                    "retry transport_attempt cannot be incremented".to_string(),
                )
            })?;
        let mut retry_request_context = request_context;
        retry_request_context.transport_attempt = next_transport_attempt;

        let actor = self
            .actors
            .get_mut(agent_id)
            .ok_or_else(|| AsyncAgentRunnerError::AgentNotRegistered(agent_id.to_string()))?;
        if actor.active_turn.load(Ordering::Acquire) {
            return Err(AsyncAgentRunnerError::AgentBusy(agent_id.to_string()));
        }
        if !actor.accepting_commands.load(Ordering::Acquire) {
            return Err(AsyncAgentRunnerError::ActorUnavailable(
                agent_id.to_string(),
            ));
        }
        actor
            .try_send(ActorCommand::Decide {
                turn_id,
                observation,
                context: Some(context),
                request_context: Some(retry_request_context),
            })
            .map_err(|error| error.with_agent(agent_id))?;
        actor.active_turn.store(true, Ordering::Release);
        self.active_turns = self.active_turns.saturating_add(1);
        // The caller has already observed this failed/completed outcome and
        // is explicitly retrying it. Remove the stale world-facing queue
        // entry while retaining the awaiting Runtime correlation record.
        self.completed.retain(|outcome| outcome.turn_id != turn_id);
        Ok(turn_id)
    }
}
