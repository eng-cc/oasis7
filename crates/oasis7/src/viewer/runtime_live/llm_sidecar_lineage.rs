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
            self.provider_feedback_seq
                .entry(feedback.agent_subject)
                .and_modify(|current| *current = (*current).max(next))
                .or_insert(next);
        }

        // A stale Runtime rejection is durable evidence that the old turn is
        // closed and one bounded semantic replan is still required. Rebuild
        // that pending causal edge so a viewer restart cannot strand it.
        if let Some(events) = projection
            .get("cognition_journal")
            .and_then(|journal| journal.get("events"))
            .and_then(Value::as_array)
        {
            for event in events {
                if event.get("kind").and_then(Value::as_str) != Some("DecisionRejected")
                    || event.get("reject_reason").and_then(Value::as_str) != Some("stale_base")
                {
                    continue;
                }
                let Some(agent_id) = event.get("agent_id").and_then(Value::as_str) else {
                    continue;
                };
                let state = self
                    .provider_stale_replans
                    .entry(agent_id.to_string())
                    .or_insert_with(|| ProviderStaleReplanState {
                        count: 0,
                        pending_cause: None,
                    });
                state.count = state
                    .count
                    .saturating_add(1)
                    .min(MAX_PROVIDER_STALE_REPLANS);
                if state.count < MAX_PROVIDER_STALE_REPLANS {
                    state.pending_cause = Some(ProviderStaleReplanCause {
                        parent_agent_turn_id: event
                            .get("agent_turn_id")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        parent_decision_request_id: event
                            .get("decision_request_id")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string(),
                        count: state.count,
                    });
                }
            }
        }
    }

    pub(in crate::viewer::runtime_live) fn mark_provider_transport_exhausted(
        &mut self,
        agent_id: String,
    ) {
        self.provider_transport_exhausted.insert(agent_id);
    }

    pub(in crate::viewer::runtime_live) fn provider_transport_exhausted_agent(
        &self,
    ) -> Option<String> {
        self.provider_transport_exhausted.iter().next().cloned()
    }

    pub(in crate::viewer::runtime_live) fn take_provider_transport_exhausted_agent(
        &mut self,
    ) -> Option<String> {
        let agent_id = self.provider_transport_exhausted_agent()?;
        self.provider_transport_exhausted.remove(&agent_id);
        Some(agent_id)
    }
}
