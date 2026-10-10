use super::feedback::RuntimeReceiptReadbackVerifier;
use super::*;
use crate::runtime::RuntimeFeedbackOutboxRecordV1;

impl AsyncAgentRunner {
    /// Restore only the bounded replay/collision verifier from an Oasis-owned
    /// Runtime authority projection. The Harness receives validated history,
    /// never the Runtime World or authority itself.
    pub(crate) fn restore_host_validated_feedback_history<H: RuntimeAuthority>(
        &mut self,
        authority: &H,
    ) -> Result<(), AsyncAgentRunnerError> {
        if !self.needs_runtime_feedback_recovery() {
            return Ok(());
        }
        self.harness
            .restore_feedback_history(authority)
            .map_err(map_harness_error)
    }

    /// Release exactly the completed production turn around a host durability
    /// boundary. The Harness keeps an identity-bound handoff capability until
    /// persistence succeeds; on failure both the actor replay fence and the
    /// local Runtime-awaiting maps are restored before the caller can retry.
    ///
    /// The callback receives only an immutable runner view, so it cannot
    /// enqueue a successor provider turn while the release is provisional.
    pub(crate) fn with_runtime_turn_release_for_continuation<F>(
        &mut self,
        agent_id: &str,
        agent_session_id: &str,
        agent_turn_id: &str,
        decision_request_id: &str,
        request_digest: &str,
        persist: F,
    ) -> Result<(), AsyncAgentRunnerError>
    where
        F: FnOnce(&Self) -> Result<(), String>,
    {
        let mut matches = self.awaiting_outcomes.iter().filter(|(_, outcome)| {
            outcome.agent_id == agent_id
                && outcome.prepared_context.as_ref().is_some_and(|context| {
                    context.agent_session_id == agent_session_id
                        && context.agent_turn_id == agent_turn_id
                        && context.decision_request_id == decision_request_id
                        && context.request_digest.to_string() == request_digest
                })
                && outcome
                    .prepared_request_context
                    .as_ref()
                    .is_some_and(|request| {
                        request.agent_subject == agent_id
                            && request.agent_session_id == agent_session_id
                            && request.agent_turn_id == agent_turn_id
                            && request.decision_request_id == decision_request_id
                            && request.request_digest.to_string() == request_digest
                    })
        });
        let Some((&turn_id, outcome)) = matches.next() else {
            return Err(AsyncAgentRunnerError::Cognition(
                "unknown pending Runtime turn".into(),
            ));
        };
        if matches.next().is_some()
            || self.awaiting_runtime.get(agent_id) != Some(&turn_id)
            || self
                .awaiting_outcomes
                .values()
                .filter(|outcome| outcome.agent_id == agent_id)
                .count()
                != 1
        {
            return Err(AsyncAgentRunnerError::Cognition(
                "pending Runtime turn identity or actor lane conflicts".into(),
            ));
        }
        let outcome = outcome.clone();
        let harness_turn = *self.harness_turns.get(&turn_id).ok_or_else(|| {
            AsyncAgentRunnerError::Cognition("Harness turn identity is unavailable".into())
        })?;
        let handoff = self
            .harness
            .prepare_turn_release(harness_turn)
            .map_err(map_harness_error)?;

        // Staging removes only this Runtime turn. The authenticated replay
        // fence and turn record remain owned by the Harness capability until
        // commit or rollback below.
        self.awaiting_runtime.remove(agent_id);
        self.awaiting_outcomes.remove(&turn_id);
        match persist(self) {
            Ok(()) => self
                .harness
                .commit_turn_release(handoff)
                .map_err(map_harness_error),
            Err(persist_error) => {
                self.awaiting_runtime.insert(agent_id.to_string(), turn_id);
                self.awaiting_outcomes.insert(turn_id, outcome);
                self.harness
                    .rollback_turn_release(handoff)
                    .map_err(|rollback_error| {
                        AsyncAgentRunnerError::Cognition(format!(
                            "Runtime turn release persistence failed ({persist_error}); exact-turn rollback failed: {rollback_error}"
                        ))
                    })?;
                Err(AsyncAgentRunnerError::Cognition(persist_error))
            }
        }
    }

    pub(crate) fn needs_runtime_feedback_recovery(&self) -> bool {
        self.harness.needs_feedback_history_refresh()
    }

    /// True after the Harness has received a complete host-validated history
    /// projection. A loaded projection may still fence individual damaged
    /// Agent/session partitions; that must not be confused with having no
    /// history at all when deciding whether unrelated Agents may dispatch.
    pub(crate) fn runtime_feedback_history_loaded(&self) -> bool {
        self.harness.feedback_recovery_loaded()
    }

    pub(crate) fn feedback_recovery_blocked(&self, agent: &str, session: &str) -> bool {
        self.harness.feedback_recovery_blocked(agent, session)
    }

    #[cfg(test)]
    pub(crate) fn agent_feedback_recovery_blocked(&self, agent: &str) -> bool {
        self.harness.agent_feedback_recovery_blocked(agent)
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
        let projections = records
            .iter()
            .map(|record| {
                record.validate().map_err(|error| {
                    AsyncAgentRunnerError::Cognition(format!("invalid feedback record: {error}"))
                })?;
                let feedback = serde_json::from_value::<FeedbackEnvelopeV1>(record.payload.clone())
                    .map_err(|error| {
                        AsyncAgentRunnerError::Cognition(format!(
                            "invalid feedback payload: {error}"
                        ))
                    })?;
                if feedback.status == "committed" {
                    return Err(AsyncAgentRunnerError::Cognition(
                        "committed feedback history requires Runtime receipt authority".into(),
                    ));
                }
                Ok(oasis7_agent_harness::FeedbackHistoryProjection {
                    feedback: super::super::agent_api_compat::feedback_to_api(&feedback),
                    acknowledged: record.state == "acked",
                    lineage: Some(oasis7_agent_harness::FeedbackHistoryLineage {
                        original_envelope_digest: record.envelope_digest.clone(),
                        delivery_state: record.state.clone(),
                        receipt_lineage: None,
                    }),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let authority =
            super::agent_harness_adapter::ScopedFeedbackHistoryAuthority::new(projections);
        self.restore_host_validated_feedback_history(&authority)
    }

    /// Caller must have authenticated the Agent-scoped Runtime View.
    pub(crate) fn restore_preverified_scoped_feedback_history(
        &mut self,
        records: &[oasis7_agent_harness::FeedbackHistoryProjection],
    ) -> Result<(), AsyncAgentRunnerError> {
        let authority =
            super::agent_harness_adapter::ScopedFeedbackHistoryAuthority::new(records.to_vec());
        self.restore_host_validated_feedback_history(&authority)
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
        let outcome = self
            .find_feedback_outcome(agent, &feedback)
            .ok_or_else(|| AsyncAgentRunnerError::Cognition("unknown actor outcome".into()))?;
        let context = outcome.prepared_context.as_ref().ok_or_else(|| {
            AsyncAgentRunnerError::Cognition("committed feedback requires a host context".into())
        })?;
        if outcome.prepared_request_context.is_none() {
            return Err(AsyncAgentRunnerError::Cognition(
                "committed feedback requires a production Runtime request".into(),
            ));
        }
        validate_feedback(context, agent, &feedback)?;
        validate_runtime_receipt_lineage(context, &feedback, receipt)?;
        let mut staged_memory = memory.clone();
        project_receipt_memory(
            context,
            &feedback,
            receipt,
            &outcome.memory_write_intents,
            &mut staged_memory,
        )?;
        let original_memory = memory.clone();
        let harness_turn = *self.harness_turns.get(&outcome.turn_id).ok_or_else(|| {
            AsyncAgentRunnerError::Cognition("Harness turn identity is unavailable".into())
        })?;
        #[cfg(any(test, feature = "test_tier_required"))]
        let native_before = self.rejected_wait_test_ledger_digests();
        let result = self.harness.accept_feedback_with_persistence_observed(
            harness_turn,
            super::super::agent_api_compat::feedback_to_api(&feedback),
            |harness_cognition_digest| {
                *memory = staged_memory.clone();
                #[cfg(any(test, feature = "test_tier_required"))]
                if std::env::var("PRE2_APP_ADMISSION").ok().as_deref()
                    == Some("memory-ack-write-failure")
                {
                    let root = std::env::var_os("PRE2_METADATA_DIR")
                        .map(std::path::PathBuf::from)
                        .ok_or_else(|| "native ACK probe root missing".to_string())?;
                    if !root.join("feedback-ack-before-persist").exists() {
                        let mut staged = native_before.clone();
                        let mut hash = blake3::Hasher::new();
                        hash.update(b"oasis7.rejected-wait.local-test-ledger.v1\0");
                        hash.update(b"feedback-store\0");
                        hash.update(harness_cognition_digest.as_bytes());
                        staged[4] = hash.finalize().to_hex().to_string();
                        for (name, value) in [
                            ("feedback-ack-native-before.json", native_before),
                            ("feedback-ack-native-staged.json", staged),
                        ] {
                            let path = root.join(name);
                            std::fs::write(&path, serde_json::to_vec(&value).unwrap())
                                .map_err(|error| error.to_string())?;
                            #[cfg(unix)]
                            {
                                use std::os::unix::fs::PermissionsExt;
                                std::fs::set_permissions(
                                    path,
                                    std::fs::Permissions::from_mode(0o600),
                                )
                                .map_err(|error| error.to_string())?;
                            }
                        }
                    }
                }
                persist(memory).map_err(|error| {
                    *memory = original_memory.clone();
                    error
                })
            },
        );
        if result.is_err() {
            *memory = original_memory;
            return result.map_err(map_harness_error);
        }
        self.release_runtime_turn(agent, outcome.turn_id);
        Ok(())
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
        Ok(())
    }
}
