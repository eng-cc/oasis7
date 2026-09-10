use super::*;
use crate::simulator::FeedbackEnvelopeV1;
use serde_json::Value;

impl RuntimeLlmSidecar {
    /// Rebuild the viewer's allocation cursors from Runtime-owned cognition
    /// records.  The sidecar remains an in-memory transport adapter, but it
    /// must never restart at sequence one after a process restart.
    pub(in crate::viewer::runtime_live) fn hydrate_provider_lineage(
        &mut self,
        world: &RuntimeWorld,
    ) {
        if self.provider_lineage_hydrated {
            return;
        }
        if self.provider_lineage_store.is_some() && !self.provider_lineage_restored {
            if let Err(error) = self.restore_provider_lineage(world) {
                self.provider_lineage_recovery_pending = Some(error.clone());
                tracing::warn!(error, "provider lineage checkpoint restore failed");
            }
        }
        if self.provider_lineage_recovery_pending.is_some() {
            // Do not rebuild fresh provider contexts from Runtime projections
            // while the durable sidecar checkpoint is undecodable. That would
            // erase active identity/recovery fences and permit a duplicate
            // provider invocation after restart.
            self.provider_lineage_hydrated = true;
            return;
        }
        if let Err(error) = self.sync_runtime_wakes(world) {
            self.provider_lineage_recovery_pending = Some(error.clone());
            // Runtime wake state is authoritative. Preserve the recovery
            // fence when a sidecar checkpoint is configured so a restart
            // cannot reinterpret an unreadable projection as no work.
            self.persist_provider_lineage_best_effort();
            tracing::warn!(error, "Runtime cognition wake projection unavailable");
            // Keep hydration incomplete. The caller observes the recovery
            // fence below, and a fresh sidecar can retry once Runtime has
            // repaired the authoritative projection.
            return;
        }
        self.provider_lineage_hydrated = true;
        let projection = world.cognition();
        let journal_head_seq = projection
            .get("cognition_journal")
            .and_then(|journal| journal.get("head_seq"))
            .and_then(Value::as_u64)
            .unwrap_or_default();
        let mut identities = Vec::new();
        let mut feedbacks = Vec::new();

        if let Some(records) = projection.get("commit_records").and_then(Value::as_array) {
            identities.extend(records.iter().filter_map(|record| {
                Some((
                    record.get("agent_id")?.as_str()?.to_string(),
                    record
                        .get("agent_session_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    record
                        .get("agent_turn_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    record
                        .get("decision_request_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    record
                        .get("response_retry_seq")
                        .and_then(Value::as_u64)
                        .unwrap_or_default(),
                ))
            }));
        }

        if let Some(records) = projection.get("feedback_outbox").and_then(Value::as_object) {
            for record in records.values() {
                let Some(payload) = record.get("payload") else {
                    continue;
                };
                if let Ok(feedback) = serde_json::from_value::<FeedbackEnvelopeV1>(payload.clone())
                {
                    identities.push((
                        feedback.agent_subject.clone(),
                        feedback.agent_session_id.clone(),
                        feedback.agent_turn_id.clone(),
                        feedback.decision_request_id.clone(),
                        0,
                    ));
                    feedbacks.push(feedback);
                }
            }
        }

        for (agent_id, session_id, _turn_id, _request_id, retry_seq) in identities {
            if agent_id.is_empty() {
                continue;
            }
            if !session_id.is_empty() {
                self.provider_session_ids
                    .entry(agent_id.clone())
                    .or_insert(session_id);
            }
            let inferred_seq = retry_seq.max(journal_head_seq);
            if inferred_seq > 0 {
                let next = inferred_seq.saturating_add(1);
                self.provider_context_seq
                    .entry(agent_id)
                    .and_modify(|current| *current = (*current).max(next))
                    .or_insert(next);
            }
        }

        for feedback in feedbacks {
            let next = feedback.feedback_seq.max(1).saturating_add(1);
            let session_key = provider_feedback_session_key(
                feedback.agent_subject.as_str(),
                feedback.agent_session_id.as_str(),
            );
            self.provider_feedback_seq
                .entry(feedback.agent_subject.clone())
                .and_modify(|current| *current = (*current).max(next))
                .or_insert(next);
            self.provider_feedback_seq_by_session
                .entry(session_key)
                .and_modify(|current| *current = (*current).max(next))
                .or_insert(next);
        }

        // A historical stale rejection only proves that the old turn closed.
        // It does not prove that a replan is still pending: the corresponding
        // provider response may already have been retried, committed, or
        // terminally rejected. Pending replans are restored only from the
        // sidecar checkpoint, where their exact causal identity is retained.
    }

    pub(in crate::viewer::runtime_live) fn mark_provider_transport_exhausted(
        &mut self,
        agent_id: String,
    ) {
        self.provider_transport_exhausted.insert(agent_id);
        self.persist_provider_lineage_best_effort();
    }

    pub(in crate::viewer::runtime_live) fn provider_transport_exhausted_agent(
        &self,
    ) -> Option<String> {
        self.provider_transport_exhausted.iter().next().cloned()
    }

    pub(in crate::viewer::runtime_live) fn take_provider_transport_exhausted_agent(
        &mut self,
    ) -> Option<String> {
        self.provider_transport_exhausted_agent()
    }

    pub(in crate::viewer::runtime_live) fn provider_transport_exhausted_agent_excluding(
        &self,
        excluded_agent: Option<&str>,
    ) -> Option<String> {
        self.provider_transport_exhausted
            .iter()
            .find(|agent_id| Some(agent_id.as_str()) != excluded_agent)
            .cloned()
    }

    pub(in crate::viewer::runtime_live) fn provider_recovery_context(
        &self,
        agent_id: &str,
    ) -> Option<cognition_context::ProviderContextState> {
        self.provider_recovery_pending
            .get(agent_id)
            .map(|pending| pending.active.clone())
            .or_else(|| {
                self.provider_wake_recovery_pending
                    .get(agent_id)
                    .map(|pending| pending.active.clone())
            })
            .or_else(|| self.provider_contexts.get(agent_id).cloned())
            .or_else(|| self.provider_active_turns.get(agent_id).cloned())
    }

    pub(in crate::viewer::runtime_live) fn provider_wake_recovery_pending_agent(
        &self,
    ) -> Option<String> {
        self.provider_wake_recovery_pending.keys().next().cloned()
    }

    pub(in crate::viewer::runtime_live) fn provider_wake_recovery_pending(
        &self,
        agent_id: &str,
    ) -> Option<lineage_persistence::ProviderWakeRecoveryPending> {
        self.provider_wake_recovery_pending.get(agent_id).cloned()
    }

    /// Retain the exact request identity while a Runtime wake handoff is
    /// unresolved. This is intentionally separate from generic transport
    /// recovery: the provider turn is already terminal, but Runtime still
    /// owns the wake acknowledgement.
    pub(in crate::viewer::runtime_live) fn retain_provider_wake_recovery_pending(
        &mut self,
        agent_id: &str,
        context: &cognition_context::ProviderContextState,
        status: crate::runtime::ContinuationStatusV1,
        reason: impl Into<String>,
    ) {
        self.provider_wake_recovery_pending.insert(
            agent_id.to_string(),
            lineage_persistence::ProviderWakeRecoveryPending {
                active: context.clone(),
                status,
                reason: reason.into(),
            },
        );
        self.provider_transport_exhausted
            .insert(agent_id.to_string());
        self.provider_active_turns
            .entry(agent_id.to_string())
            .or_insert_with(|| context.clone());
        self.provider_contexts
            .entry(agent_id.to_string())
            .or_insert_with(|| context.clone());
        self.persist_provider_lineage_best_effort();
    }

    /// Remove wake recovery state only after Runtime has accepted the wake.
    /// A checkpoint failure restores every identity-bearing mirror so the
    /// next pass can retry cleanup without redispatching the provider turn.
    pub(in crate::viewer::runtime_live) fn complete_provider_wake_recovery(
        &mut self,
        agent_id: &str,
    ) -> Result<(), String> {
        let wake_backup = self.provider_wake_recovery_pending.get(agent_id).cloned();
        let context_backup = self.provider_contexts.get(agent_id).cloned();
        let active_backup = self.provider_active_turns.get(agent_id).cloned();
        let retry_backup = self.provider_retry_contexts.get(agent_id).cloned();
        let wait_backup = self.provider_wait_until.get(agent_id).copied();
        let held_backup = self.provider_held_decisions.get(agent_id).cloned();
        let exhausted_backup = self.provider_transport_exhausted.contains(agent_id);
        self.provider_wake_recovery_pending.remove(agent_id);
        self.provider_contexts.remove(agent_id);
        self.provider_active_turns.remove(agent_id);
        self.provider_retry_contexts.remove(agent_id);
        self.provider_wait_until.remove(agent_id);
        self.provider_held_decisions.remove(agent_id);
        self.provider_transport_exhausted.remove(agent_id);
        if let Err(error) = self.persist_provider_lineage() {
            if let Some(wake) = wake_backup {
                self.provider_wake_recovery_pending
                    .insert(agent_id.to_string(), wake);
            }
            if let Some(context) = context_backup {
                self.provider_contexts.insert(agent_id.to_string(), context);
            }
            if let Some(active) = active_backup {
                self.provider_active_turns
                    .insert(agent_id.to_string(), active);
            }
            if let Some(retry) = retry_backup {
                self.provider_retry_contexts
                    .insert(agent_id.to_string(), retry);
            }
            if let Some(wait_until) = wait_backup {
                self.provider_wait_until
                    .insert(agent_id.to_string(), wait_until);
            }
            if let Some(held) = held_backup {
                self.provider_held_decisions
                    .insert(agent_id.to_string(), held);
            }
            if exhausted_backup {
                self.provider_transport_exhausted
                    .insert(agent_id.to_string());
            }
            return Err(format!(
                "provider wake recovery cleanup persistence failed: {error}"
            ));
        }
        Ok(())
    }

    pub(in crate::viewer::runtime_live) fn update_provider_wake_recovery(
        &mut self,
        agent_id: &str,
        status: crate::runtime::ContinuationStatusV1,
        reason: &str,
    ) -> Result<(), String> {
        let Some(recovery) = self.provider_wake_recovery_pending.get_mut(agent_id) else {
            return Ok(());
        };
        recovery.status = status;
        recovery.reason = reason.to_string();
        self.persist_provider_lineage()
    }

    pub(super) fn handle_provider_wake_resume_failure(
        &mut self,
        world: &mut RuntimeWorld,
        wake: &crate::runtime::SchedulerWakeV1,
        current_context: &crate::simulator::ContinuationCurrentContextV1,
        turn_context: &crate::simulator::ContinuousAgentTurnContextV1,
        request_context: &crate::simulator::ContinuousAgentRequestContextV1,
        predecessor_proposal_id: &str,
        next_proposal_id: &str,
        agent_id: &str,
        resume_error: &crate::runtime::WorldError,
    ) -> String {
        let recovery_context = cognition_context::ProviderContextState {
            turn_context: turn_context.clone(),
            request_context: request_context.clone(),
        };
        let handoff_error = world
            .handoff_cognition_wake_with_context(
                &wake.wake_id,
                crate::runtime::CognitionWakeDispositionV1::Terminal {
                    status: crate::runtime::ContinuationStatusV1::Rejected,
                    reason: "provider_wake_resume_failed".to_string(),
                },
                crate::runtime::CognitionContextDigestsV1 {
                    baseline_observation_digest: current_context
                        .authority
                        .baseline_observation_digest
                        .clone(),
                    goal_digest: current_context.authority.goal_digest.clone(),
                    policy_digest: current_context.authority.policy_digest.clone(),
                    precondition_digest: current_context.authority.precondition_digest.clone(),
                },
            )
            .err();
        self.provider_continuation_proposals
            .remove(predecessor_proposal_id);
        self.provider_continuation_proposals
            .remove(next_proposal_id);
        if let Some(handoff_error) = handoff_error {
            self.retain_provider_wake_recovery_pending(
                agent_id,
                &recovery_context,
                crate::runtime::ContinuationStatusV1::Rejected,
                "provider_wake_resume_failed",
            );
            return format!(
                "Runtime cognition wake resume rejected {}: {resume_error:?}; Runtime wake handoff failed: {handoff_error:?}",
                wake.wake_id
            );
        }
        self.pending_runtime_wakes.remove(&wake.wake_id);
        self.provider_contexts.remove(agent_id);
        self.provider_active_turns.remove(agent_id);
        self.provider_retry_contexts.remove(agent_id);
        self.provider_wait_until.remove(agent_id);
        self.persist_provider_lineage_best_effort();
        format!(
            "Runtime cognition wake resume rejected {}: {resume_error:?}",
            wake.wake_id
        )
    }

    /// Keep an exact interrupted request fenced until every Runtime/Harness
    /// recovery step has completed. This record is intentionally additive to
    /// the transport exhaustion marker so a failed checkpoint write cannot
    /// turn a known identity into an eligible fresh request.
    pub(in crate::viewer::runtime_live) fn retain_provider_recovery_pending(
        &mut self,
        agent_id: &str,
        context: &cognition_context::ProviderContextState,
        reason: impl Into<String>,
    ) {
        self.provider_recovery_pending.insert(
            agent_id.to_string(),
            lineage_persistence::ProviderRecoveryPending {
                active: context.clone(),
                reason: reason.into(),
            },
        );
        self.provider_transport_exhausted
            .insert(agent_id.to_string());
        self.provider_active_turns
            .entry(agent_id.to_string())
            .or_insert_with(|| context.clone());
        self.provider_contexts
            .entry(agent_id.to_string())
            .or_insert_with(|| context.clone());
        self.persist_provider_lineage_best_effort();
    }
}

pub(super) fn provider_feedback_session_key(agent_subject: &str, session_id: &str) -> String {
    format!("{agent_subject}\u{1f}{session_id}")
}
