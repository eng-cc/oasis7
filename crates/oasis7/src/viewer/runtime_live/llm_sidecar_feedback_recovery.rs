//! Read-only bridge from Runtime's validated feedback outbox to the native
//! runner's bounded replay/collision verifier.

#[cfg(test)]
use std::collections::{BTreeMap, BTreeSet};

use crate::runtime::World as RuntimeWorld;
use crate::simulator::{AsyncAgentRunner, FeedbackEnvelopeV1};

pub(in crate::viewer::runtime_live) fn restore_agent_feedback_history(
    world: &RuntimeWorld,
    runner: &mut AsyncAgentRunner,
) -> Result<(), String> {
    let records = world
        .runtime_feedback_outbox()
        .map_err(|error| format!("Runtime feedback recovery outbox read failed: {error:?}"))?;
    for record in &records {
        record
            .validate()
            .map_err(|error| format!("Runtime feedback recovery record invalid: {error}"))?;
        let feedback = serde_json::from_value::<FeedbackEnvelopeV1>(record.payload.clone())
            .map_err(|error| format!("Runtime feedback recovery payload invalid: {error}"))?;
        if feedback.status != "committed" {
            continue;
        }
        let receipt_id = feedback
            .runtime_receipt_id
            .as_deref()
            .ok_or_else(|| "committed Runtime feedback lacks a receipt identity".to_string())?;
        let receipt = world
            .read_runtime_receipt_lineage(receipt_id)
            .map_err(|error| format!("Runtime feedback receipt readback failed: {error:?}"))?;
        receipt
            .validate()
            .map_err(|error| format!("Runtime feedback receipt lineage invalid: {error}"))?;
        let action_matches = feedback
            .candidate_action_id
            .is_some_and(|action_id| receipt.action_id == format!("action:{action_id}"));
        if receipt.receipt_id != receipt_id
            || receipt.agent_id != feedback.agent_subject
            || receipt.agent_session_id != feedback.agent_session_id
            || receipt.agent_turn_id != feedback.agent_turn_id
            || receipt.decision_request_id != feedback.decision_request_id
            || receipt.request_digest != feedback.request_digest.to_string()
            || receipt.feedback_id != feedback.feedback_id
            || !action_matches
        {
            return Err(format!(
                "Runtime feedback {} does not match its committed receipt lineage",
                feedback.feedback_id
            ));
        }
    }
    runner
        .restore_runtime_feedback_outbox(&records)
        .map_err(|error| format!("Runtime feedback verifier recovery failed: {error}"))
}

#[cfg(test)]
pub(in crate::viewer::runtime_live) fn blocked_provider_agents(
    runner: &AsyncAgentRunner,
    candidates: &BTreeSet<String>,
    session_by_agent: &BTreeMap<String, Option<String>>,
) -> BTreeSet<String> {
    candidates
        .iter()
        .filter(|agent| {
            match session_by_agent
                .get(agent.as_str())
                .and_then(Option::as_deref)
            {
                Some(session) => runner.feedback_recovery_blocked(agent, session),
                None => runner.agent_feedback_recovery_blocked(agent),
            }
        })
        .cloned()
        .collect()
}

pub(in crate::viewer::runtime_live) fn feedback_history_blocks_dispatch(
    runner: &AsyncAgentRunner,
    agent: &str,
    session: &str,
) -> bool {
    runner.feedback_recovery_blocked(agent, session)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{RuntimeFeedbackOutboxRecordV1, RuntimeFeedbackProjectionV1};
    use crate::simulator::{Digest32, FeedbackEnvelopeV1};

    fn feedback(agent: &str, session: &str, seq: u64, id: &str) -> FeedbackEnvelopeV1 {
        FeedbackEnvelopeV1 {
            feedback_id: id.to_string(),
            feedback_seq: seq,
            agent_subject: agent.to_string(),
            agent_session_id: session.to_string(),
            agent_turn_id: format!("turn-{seq}"),
            decision_request_id: format!("request-{seq}"),
            candidate_action_id: None,
            runtime_receipt_id: None,
            status: "rejected".to_string(),
            request_digest: Digest32(format!("blake3:{}", "a".repeat(64))),
            reject_reason: Some("stale_base".to_string()),
            provenance: "runtime_authoritative".to_string(),
        }
    }

    fn outbox_record(feedback: &FeedbackEnvelopeV1, state: &str) -> RuntimeFeedbackOutboxRecordV1 {
        let mut record = RuntimeFeedbackOutboxRecordV1::from_feedback_with_projection(
            feedback,
            RuntimeFeedbackProjectionV1::default(),
        )
        .expect("feedback creates a valid outbox record");
        record.state = state.to_string();
        record.validate().expect("outbox record remains valid");
        record
    }

    #[test]
    fn recovery_fence_filters_only_the_agent_session_with_bad_history() {
        let records = vec![
            outbox_record(&feedback("agent-a", "session-a", 1, "a-1"), "acked"),
            outbox_record(&feedback("agent-b", "session-b", 1, "b-1"), "acked"),
            outbox_record(&feedback("agent-b", "session-b", 3, "b-3"), "acked"),
        ];
        let mut runner = AsyncAgentRunner::new(4).expect("native Runner capacity is valid");
        runner
            .restore_runtime_feedback_outbox(&records)
            .expect("identity-bearing gap is retained as a partition fence");

        let candidates = BTreeSet::from(["agent-a".to_string(), "agent-b".to_string()]);
        let session_by_agent = BTreeMap::from([
            ("agent-a".to_string(), Some("session-a".to_string())),
            ("agent-b".to_string(), Some("session-b".to_string())),
        ]);
        assert_eq!(
            blocked_provider_agents(&runner, &candidates, &session_by_agent),
            BTreeSet::from(["agent-b".to_string()])
        );

        let newer_session = BTreeMap::from([
            ("agent-a".to_string(), Some("session-a".to_string())),
            ("agent-b".to_string(), Some("session-b-new".to_string())),
        ]);
        assert!(blocked_provider_agents(&runner, &candidates, &newer_session).is_empty());

        let unknown_session = BTreeMap::from([
            ("agent-a".to_string(), Some("session-a".to_string())),
            ("agent-b".to_string(), None),
        ]);
        assert_eq!(
            blocked_provider_agents(&runner, &candidates, &unknown_session),
            BTreeSet::from(["agent-b".to_string()])
        );
    }
}
