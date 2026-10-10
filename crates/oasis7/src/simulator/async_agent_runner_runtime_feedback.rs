use super::feedback::RuntimeReceiptReadbackVerifier;
use super::*;
use crate::runtime::RuntimeFeedbackOutboxRecordV1;

impl AsyncAgentRunner {
    pub(crate) fn needs_runtime_feedback_recovery(&self) -> bool {
        !self.feedback_store.feedback_recovery_initialized()
    }

    pub(crate) fn feedback_recovery_blocked(&self, agent: &str, session: &str) -> bool {
        self.feedback_store
            .feedback_recovery_blocked(agent, session)
    }

    #[cfg(test)]
    pub(crate) fn agent_feedback_recovery_blocked(&self, agent: &str) -> bool {
        self.feedback_store.agent_feedback_recovery_blocked(agent)
    }

    /// Restore only the bounded replay/collision verifier from Runtime's
    /// durable outbox. Feedback is not consumed again and memory is untouched.
    pub(crate) fn restore_runtime_feedback_outbox(
        &mut self,
        records: &[RuntimeFeedbackOutboxRecordV1],
    ) -> Result<(), AsyncAgentRunnerError> {
        if !self.needs_runtime_feedback_recovery() {
            return Ok(());
        }
        self.feedback_store
            .restore_runtime_feedback_outbox(records)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))
    }

    /// Caller must have authenticated the Agent-scoped Runtime View.
    pub(crate) fn restore_preverified_scoped_feedback_history(
        &mut self,
        records: &[(FeedbackEnvelopeV1, String, String)],
    ) -> Result<(), AsyncAgentRunnerError> {
        self.feedback_store
            .restore_preverified_scoped_feedback_history(records)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))
    }

    /// Roll back native verifier/turn ledgers and private memory if the sole
    /// durable consumption checkpoint fails. No actor I/O occurs here.
    pub(crate) fn with_committed_feedback_memory_transaction<F>(
        &mut self,
        agent: &str,
        feedback: FeedbackEnvelopeV1,
        receipt: &RuntimeReceiptLineageV1,
        memory: &mut MemoryWriteStore,
        persist: F,
    ) -> Result<(), AsyncAgentRunnerError>
    where
        F: FnOnce(&mut MemoryWriteStore) -> Result<(), String>,
    {
        let before = (
            self.feedback_store.clone(),
            self.awaiting_runtime.clone(),
            self.awaiting_outcomes.clone(),
            self.continuations.clone(),
            self.continuation_harness.clone(),
            memory.clone(),
        );
        #[cfg(any(test, feature = "test_tier_required"))]
        let native_before = self.rejected_wait_test_ledger_digests();
        let result = self
            .consume_runtime_feedback_with_lineage(agent, feedback, Some(receipt), memory)
            .and_then(|()| {
                #[cfg(any(test, feature = "test_tier_required"))]
                if std::env::var("PRE2_APP_ADMISSION").ok().as_deref()
                    == Some("memory-ack-write-failure")
                {
                    let root = std::env::var_os("PRE2_METADATA_DIR")
                        .map(std::path::PathBuf::from)
                        .ok_or_else(|| {
                            AsyncAgentRunnerError::Cognition("native ACK probe root missing".into())
                        })?;
                    if !root.join("feedback-ack-before-persist").exists() {
                        for (name, value) in [
                            ("feedback-ack-native-before.json", native_before),
                            (
                                "feedback-ack-native-staged.json",
                                self.rejected_wait_test_ledger_digests(),
                            ),
                        ] {
                            let path = root.join(name);
                            std::fs::write(&path, serde_json::to_vec(&value).unwrap())
                                .map_err(|e| AsyncAgentRunnerError::Cognition(e.to_string()))?;
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt;
                                std::fs::set_permissions(
                                    path,
                                    std::fs::Permissions::from_mode(0o600),
                                )
                                .map_err(|e| AsyncAgentRunnerError::Cognition(e.to_string()))?;
                            }
                        }
                    }
                }
                persist(memory).map_err(AsyncAgentRunnerError::Cognition)
            });
        if result.is_err() {
            (
                self.feedback_store,
                self.awaiting_runtime,
                self.awaiting_outcomes,
                self.continuations,
                self.continuation_harness,
                *memory,
            ) = before;
        }
        result
    }

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
        request_digest: &str,
    ) -> Result<(), AsyncAgentRunnerError> {
        let Some((&turn_id, _outcome)) = self.awaiting_outcomes.iter().find(|(_, outcome)| {
            outcome.agent_id == agent_id
                && outcome.prepared_context.as_ref().is_some_and(|context| {
                    context.agent_session_id == agent_session_id
                        && context.agent_turn_id == agent_turn_id
                        && context.decision_request_id == decision_request_id
                        && context.request_digest.to_string() == request_digest
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

    /// Release the actor's completed outcome after a provider Wait has been
    /// admitted as a continuation. Unlike `expire_runtime_turn`, this keeps
    /// the Harness continuation chain active: the Runtime-owned wake is now
    /// the durable owner of the next invocation, so expiring here would
    /// incorrectly invalidate a successfully admitted wait.
    pub fn release_runtime_turn_for_continuation(
        &mut self,
        agent_id: &str,
        agent_session_id: &str,
        agent_turn_id: &str,
        decision_request_id: &str,
        request_digest: &str,
    ) -> Result<(), AsyncAgentRunnerError> {
        let Some((&turn_id, _outcome)) = self.awaiting_outcomes.iter().find(|(_, outcome)| {
            outcome.agent_id == agent_id
                && outcome.prepared_context.as_ref().is_some_and(|context| {
                    context.agent_session_id == agent_session_id
                        && context.agent_turn_id == agent_turn_id
                        && context.decision_request_id == decision_request_id
                        && context.request_digest.to_string() == request_digest
                })
        }) else {
            return Err(AsyncAgentRunnerError::Cognition(
                "unknown pending Runtime turn".to_string(),
            ));
        };
        self.release_runtime_turn(agent_id, turn_id);
        // A provider Wait is terminal for this logical request once Runtime
        // has admitted its continuation. Clear the single-flight marker so
        // the fresh wake/resume request can enter the actor; request replay
        // protection remains in digest_by_request_id.
        self.feedback_store.clear_agent(agent_id);
        Ok(())
    }
}
