//! Scoped replay metadata, not a durable outbox or feedback delivery payload.
use crate::runtime::{RuntimeReceiptLineageV1, World};
#[cfg(not(target_arch = "wasm32"))]
use crate::simulator::AsyncAgentRunner;
use crate::simulator::FeedbackEnvelopeV1;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldServiceFeedbackReplayRecord {
    pub feedback: FeedbackEnvelopeV1,
    /// Digest of the ORIGINAL validated payload including private projection.
    /// It must not be recomputed from this sanitized envelope.
    pub original_envelope_digest: String,
    pub delivery_state: String,
    pub receipt_lineage: Option<RuntimeReceiptLineageV1>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorldServiceFeedbackHistory {
    pub agent_id: String,
    pub records: Vec<WorldServiceFeedbackReplayRecord>,
}
impl WorldServiceFeedbackHistory {
    pub fn from_world(world: &World, agent: &str) -> Result<Self, String> {
        let mut records = Vec::new();
        for record in world
            .runtime_feedback_outbox()
            .map_err(|e| format!("{e:?}"))?
        {
            if record.agent_subject != agent {
                continue;
            }
            record.validate().map_err(|e| e.to_string())?;
            let feedback: FeedbackEnvelopeV1 =
                serde_json::from_value(record.payload).map_err(|e| e.to_string())?;
            let receipt_lineage = if feedback.status == "committed" {
                Some(
                    world
                        .read_runtime_receipt_lineage(
                            feedback
                                .runtime_receipt_id
                                .as_deref()
                                .ok_or("feedback receipt missing")?,
                        )
                        .map_err(|e| format!("{e:?}"))?,
                )
            } else {
                None
            };
            records.push(WorldServiceFeedbackReplayRecord {
                feedback,
                original_envelope_digest: record.envelope_digest,
                delivery_state: record.state,
                receipt_lineage,
            });
        }
        let history = Self {
            agent_id: agent.into(),
            records,
        };
        history.metadata(agent)?;
        Ok(history)
    }
    pub(crate) fn metadata(
        &self,
        agent: &str,
    ) -> Result<Vec<(FeedbackEnvelopeV1, String, String)>, String> {
        if self.agent_id != agent || self.records.len() > 16384 {
            return Err("scoped feedback history audience or bound invalid".into());
        }
        self.records
            .iter()
            .map(|record| {
                let f = &record.feedback;
                if f.agent_subject != agent {
                    return Err("scoped feedback history contains another Agent".into());
                }
                if f.status == "committed" {
                    let r = record
                        .receipt_lineage
                        .as_ref()
                        .ok_or("committed feedback receipt missing")?;
                    r.validate().map_err(|e| e.to_string())?;
                    if Some(r.receipt_id.as_str()) != f.runtime_receipt_id.as_deref()
                        || r.agent_id != f.agent_subject
                        || r.agent_session_id != f.agent_session_id
                        || r.agent_turn_id != f.agent_turn_id
                        || r.decision_request_id != f.decision_request_id
                        || r.request_digest != f.request_digest.to_string()
                        || r.feedback_id != f.feedback_id
                        || f.candidate_action_id
                            .is_none_or(|id| r.action_id != format!("action:{id}"))
                    {
                        return Err("scoped feedback receipt lineage mismatch".into());
                    }
                } else if record.receipt_lineage.is_some() {
                    return Err("unexpected feedback receipt lineage".into());
                }
                Ok((
                    f.clone(),
                    record.original_envelope_digest.clone(),
                    record.delivery_state.clone(),
                ))
            })
            .collect()
    }
    /// Only call after the enclosing View has passed authentication and CAS validation.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn restore_preverified(
        &self,
        agent: &str,
        runner: &mut AsyncAgentRunner,
    ) -> Result<(), String> {
        runner
            .restore_preverified_scoped_feedback_history(&self.metadata(agent)?)
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn check_fresh_session(&self, agent: &str, session: &str) -> Result<(), String> {
        let mut checker = AsyncAgentRunner::new(1).map_err(|e| e.to_string())?;
        self.restore_preverified(agent, &mut checker)?;
        if checker.feedback_recovery_blocked(agent, session) {
            return Err("Runtime feedback history fences fresh admission".into());
        }
        Ok(())
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::super::WorldServiceProjection;
    use super::*;
    use crate::simulator::Digest32;
    fn feedback(agent: &str, session: &str, seq: u64) -> FeedbackEnvelopeV1 {
        FeedbackEnvelopeV1 {
            feedback_id: format!("{agent}-{session}-{seq}"),
            feedback_seq: seq,
            agent_subject: agent.into(),
            agent_session_id: session.into(),
            agent_turn_id: format!("turn-{seq}"),
            decision_request_id: format!("request-{seq}"),
            candidate_action_id: None,
            runtime_receipt_id: None,
            status: "rejected".into(),
            request_digest: Digest32(format!("blake3:{}", "a".repeat(64))),
            reject_reason: Some("stale_base".into()),
            provenance: "runtime_authoritative".into(),
        }
    }
    fn enqueue(world: &mut World, agent: &str, session: &str, seq: u64) {
        let f = feedback(agent, session, seq);
        world.enqueue_runtime_feedback(f.clone()).unwrap();
        world.claim_runtime_feedback(&f.feedback_id).unwrap();
        world
            .retry_runtime_feedback(&f.feedback_id, "private delivery diagnostic")
            .unwrap();
        world.claim_runtime_feedback(&f.feedback_id).unwrap();
        world.ack_runtime_feedback(&f.feedback_id).unwrap();
    }
    #[test]
    fn scoped_feedback_history_uses_actual_world_and_omits_private_other_agent_metadata() {
        let mut world = World::new();
        enqueue(&mut world, "agent-a", "session-a", 1);
        enqueue(&mut world, "agent-a", "session-a", 2);
        enqueue(&mut world, "agent-b", "session-b", 1);
        let projection = WorldServiceProjection::from_world(&world, Some("agent-a")).unwrap();
        let history = projection.feedback_history.unwrap();
        assert_eq!(history.records.len(), 2);
        history.check_fresh_session("agent-a", "session-a").unwrap();
        let encoded = serde_json::to_string(&history).unwrap();
        assert!(!encoded.contains("agent-b"));
        assert!(!encoded.contains("private delivery diagnostic"));
        assert!(!encoded.contains("last_error"));
        assert!(!encoded.contains("emitted_events"));
        let actual = world.runtime_feedback_outbox().unwrap();
        assert_eq!(
            history.records[0].original_envelope_digest,
            actual[0].envelope_digest
        );
        let public = WorldServiceProjection::from_world(&world, None).unwrap();
        assert!(public.feedback_history.is_none());
        assert!(
            serde_json::to_value(public)
                .unwrap()
                .get("feedback_history")
                .is_none()
        );
    }
    #[test]
    fn actual_canonical_feedback_gap_fences_only_matching_session() {
        let mut world = World::new();
        enqueue(&mut world, "agent-a", "bad-session", 1);
        enqueue(&mut world, "agent-a", "bad-session", 3);
        let history = WorldServiceFeedbackHistory::from_world(&world, "agent-a").unwrap();
        assert!(
            history
                .check_fresh_session("agent-a", "bad-session")
                .is_err()
        );
        history
            .check_fresh_session("agent-a", "new-session")
            .unwrap();
        assert!(
            history
                .check_fresh_session("agent-b", "bad-session")
                .is_err()
        );
    }
    #[test]
    fn preverified_metadata_collision_fences_both_sessions_without_outbox_fabrication() {
        let mut world = World::new();
        enqueue(&mut world, "agent-a", "session-a", 1);
        enqueue(&mut world, "agent-a", "session-b", 1);
        let mut history = WorldServiceFeedbackHistory::from_world(&world, "agent-a").unwrap();
        history.records[1].feedback.feedback_id = history.records[0].feedback.feedback_id.clone();
        assert!(history.check_fresh_session("agent-a", "session-a").is_err());
        assert!(history.check_fresh_session("agent-a", "session-b").is_err());
    }
}
