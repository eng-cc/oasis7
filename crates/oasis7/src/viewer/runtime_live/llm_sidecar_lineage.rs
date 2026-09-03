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
                tracing::warn!(error, "provider lineage checkpoint restore failed");
            }
        }
        if let Err(error) = self.sync_runtime_wakes(world) {
            tracing::warn!(error, "Runtime cognition wake projection unavailable");
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

    pub(in crate::viewer::runtime_live) fn clear_provider_transport_exhausted(
        &mut self,
        agent_id: &str,
    ) {
        if self.provider_transport_exhausted.remove(agent_id) {
            self.persist_provider_lineage_best_effort();
        }
    }
}

pub(super) fn provider_feedback_session_key(agent_subject: &str, session_id: &str) -> String {
    format!("{agent_subject}\u{1f}{session_id}")
}
