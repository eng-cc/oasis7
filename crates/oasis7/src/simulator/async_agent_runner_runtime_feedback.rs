use super::feedback::RuntimeReceiptReadbackVerifier;
use super::*;

impl AsyncAgentRunner {
    pub fn consume_runtime_feedback(
        &mut self,
        agent_id: &str,
        feedback: FeedbackEnvelopeV1,
        store: &mut MemoryWriteStore,
    ) -> Result<(), AsyncAgentRunnerError> {
        self.consume_runtime_feedback_with_lineage(agent_id, feedback, None, store)
    }

    /// Strict production seam: Runtime must prove the durable World readback
    /// before a committed response/artifact can authorize memory projection.
    /// The compatibility lineage method remains available for older adapters,
    /// but it does not claim this readback proof.
    pub fn consume_runtime_feedback_with_world_readback(
        &mut self,
        agent_id: &str,
        feedback: FeedbackEnvelopeV1,
        runtime_receipt: &RuntimeReceiptLineageV1,
        response_identity: &super::super::continuous_agent_harness::ResponseArtifactIdentityV1,
        verifier: &dyn RuntimeReceiptReadbackVerifier,
        store: &mut MemoryWriteStore,
    ) -> Result<(), AsyncAgentRunnerError> {
        let outcome = self
            .awaiting_outcomes
            .values()
            .find(|outcome| {
                outcome.agent_id == agent_id
                    && outcome.prepared_context.as_ref().is_some_and(|context| {
                        context.agent_session_id == feedback.agent_session_id
                            && context.agent_turn_id == feedback.agent_turn_id
                            && context.decision_request_id == feedback.decision_request_id
                    })
            })
            .cloned()
            .ok_or_else(|| {
                AsyncAgentRunnerError::Cognition(
                    "Runtime readback requires an awaiting outcome".to_string(),
                )
            })?;
        if feedback.status == "committed" {
            let context = outcome
                .prepared_context
                .as_ref()
                .expect("outcome response has a prepared context");
            validate_feedback(context, agent_id, &feedback)?;
            validate_runtime_receipt_lineage(context, &feedback, runtime_receipt)?;
            let response = outcome.prepared_response_context.as_ref().ok_or_else(|| {
                AsyncAgentRunnerError::Cognition(
                    "Runtime response artifact identity is unavailable".to_string(),
                )
            })?;
            response
                .validate_response_artifact_identity(response_identity)
                .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
            let handle = verifier.verify_world_readback(
                context,
                &feedback,
                runtime_receipt,
                response_identity,
            )?;
            if handle.receipt_id() != runtime_receipt.receipt_id
                || handle.receipt_digest().as_str() != runtime_receipt.receipt_digest
                || handle.response_artifact_digest() != &response_identity.artifact_digest
            {
                return Err(AsyncAgentRunnerError::Cognition(
                    "Runtime readback verifier returned mismatched identity".to_string(),
                ));
            }
        }
        self.consume_runtime_feedback_with_lineage(agent_id, feedback, Some(runtime_receipt), store)
    }

    /// Explicitly expire a Runtime-owned pending lease. This is the only
    /// non-terminal path that releases a prepared outcome; it does not erase
    /// the request identity, so the same request cannot be replayed as a new
    /// provider invocation after expiry.
    pub fn expire_runtime_turn(
        &mut self,
        agent_id: &str,
        agent_session_id: &str,
        agent_turn_id: &str,
        decision_request_id: &str,
    ) -> Result<(), AsyncAgentRunnerError> {
        let Some((&turn_id, _outcome)) = self.awaiting_outcomes.iter().find(|(_, outcome)| {
            outcome.agent_id == agent_id
                && outcome.prepared_context.as_ref().is_some_and(|context| {
                    context.agent_session_id == agent_session_id
                        && context.agent_turn_id == agent_turn_id
                        && context.decision_request_id == decision_request_id
                })
        }) else {
            return Err(AsyncAgentRunnerError::Cognition(
                "unknown pending Runtime turn".to_string(),
            ));
        };
        self.invalidate_continuation_for_turn(
            agent_id,
            agent_session_id,
            agent_turn_id,
            decision_request_id,
            super::ContinuationInvalidationReason::Timeout,
        );
        self.release_runtime_turn(agent_id, turn_id);
        self.feedback_store.clear_agent(agent_id);
        Ok(())
    }
}
