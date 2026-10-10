//! In-memory correlation, single-flight, and feedback replay algorithms.
//!
//! Runtime remains the durable source of truth. This store only fences
//! concurrent cognition contamination and validates feedback sequencing in
//! the Harness process.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use oasis7_agent_api::{
    CognitionError, ContinuousAgentRequestContextV1, ContinuousAgentTurnContextV1, Digest32,
    FeedbackEnvelopeV1, feedback_digest,
};

use crate::RuntimeAuthority;

const MAX_FEEDBACK_REPLAY_ENTRIES: usize = 8;

#[derive(Debug, Clone)]
struct ActiveCognitionRequest {
    session_id: String,
    turn_id: String,
    request_id: String,
    request_digest: Digest32,
}

#[derive(Debug, Clone, Default)]
struct FeedbackPartition {
    next_seq: u64,
    digest_by_id: BTreeMap<String, Digest32>,
    digest_by_seq: BTreeMap<u64, (String, Digest32)>,
    replay_order: VecDeque<String>,
    held: BTreeMap<u64, FeedbackEnvelopeV1>,
}

/// Non-serialized Runtime outbox projection. The host must first validate the
/// durable record and payload digest; this data is used only to rebuild local
/// replay/collision metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedbackHistoryProjection {
    pub feedback: FeedbackEnvelopeV1,
    pub acknowledged: bool,
}

/// In-memory single-flight and bounded replay guard. This does not persist
/// Runtime feedback or replay receipt/memory effects.
#[derive(Debug, Clone, Default)]
pub struct AgentCognitionStore {
    active_by_agent: BTreeMap<String, ActiveCognitionRequest>,
    digest_by_request_id: BTreeMap<String, Digest32>,
    subject_by_request_id: BTreeMap<String, String>,
    feedback_partitions: BTreeMap<(String, String), FeedbackPartition>,
    feedback_recovery_blocked: BTreeMap<(String, String), String>,
    feedback_recovery_initialized: bool,
}

impl AgentCognitionStore {
    pub fn restore_from_runtime<A: RuntimeAuthority>(
        &mut self,
        authority: &A,
    ) -> Result<(), CognitionError> {
        let history = authority.validated_feedback_history()?;
        self.restore_validated_feedback_history(&history)
    }

    /// Hydrate local replay/collision metadata from a host-validated complete
    /// Runtime history. No feedback effects are replayed here.
    pub(crate) fn restore_validated_feedback_history(
        &mut self,
        records: &[FeedbackHistoryProjection],
    ) -> Result<(), CognitionError> {
        let mut groups = BTreeMap::<(String, String), Vec<&FeedbackHistoryProjection>>::new();
        let mut id_partitions = BTreeMap::<String, BTreeSet<(String, String)>>::new();
        for record in records {
            let feedback = &record.feedback;
            feedback.validate()?;
            let key = (
                feedback.agent_subject.clone(),
                feedback.agent_session_id.clone(),
            );
            id_partitions
                .entry(feedback.feedback_id.clone())
                .or_default()
                .insert(key.clone());
            groups.entry(key).or_default().push(record);
        }
        let colliding = id_partitions
            .values()
            .filter(|partitions| partitions.len() > 1)
            .flat_map(|partitions| partitions.iter().cloned())
            .collect::<BTreeSet<_>>();

        for (key, entries) in &mut groups {
            entries.sort_by_key(|record| record.feedback.feedback_seq);
            let mut reason = colliding.contains(key).then(|| {
                "Runtime feedback identity is reused by another Agent/session partition".to_string()
            });
            let mut sequences = BTreeSet::new();
            let mut identities = BTreeSet::new();
            for (index, record) in entries.iter().enumerate() {
                let feedback = &record.feedback;
                if !sequences.insert(feedback.feedback_seq)
                    || !identities.insert(feedback.feedback_id.as_str())
                {
                    reason = Some(
                        "Runtime feedback history contains a duplicate sequence or identity"
                            .to_string(),
                    );
                    break;
                }
                let expected = index as u64 + 1;
                if feedback.feedback_seq != expected {
                    reason = Some(format!(
                        "Runtime feedback history has a sequence gap: expected {expected}"
                    ));
                    break;
                }
                if !record.acknowledged {
                    reason = Some(format!(
                        "Runtime feedback {} is not acknowledged yet",
                        feedback.feedback_id
                    ));
                    break;
                }
            }
            if reason.is_none()
                && entries
                    .last()
                    .is_some_and(|record| !is_terminal_status(record.feedback.status.as_str()))
            {
                reason = Some(
                    "Runtime feedback history ends with a nonterminal disposition".to_string(),
                );
            }
            let maximum_seq = entries
                .last()
                .map(|record| record.feedback.feedback_seq)
                .unwrap_or_default();
            let authoritative = entries
                .iter()
                .map(|record| {
                    (
                        record.feedback.feedback_seq,
                        (
                            record.feedback.feedback_id.as_str(),
                            feedback_digest(&record.feedback),
                        ),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            if let Some(existing) = self.feedback_partitions.get(key) {
                if existing.next_seq > maximum_seq || !existing.held.is_empty() {
                    reason.get_or_insert_with(|| {
                        "live feedback verifier state is ahead of or held beyond Runtime history"
                            .to_string()
                    });
                }
                if existing.digest_by_seq.iter().any(|(seq, (id, digest))| {
                    authoritative
                        .get(seq)
                        .is_none_or(|(source_id, source_digest)| {
                            *source_id != id.as_str() || source_digest != digest
                        })
                }) {
                    reason.get_or_insert_with(|| {
                        "live feedback verifier identity conflicts with Runtime history".to_string()
                    });
                }
            }
            if let Some(reason) = reason {
                self.feedback_recovery_blocked.insert(key.clone(), reason);
                continue;
            }
            let partition = self.feedback_partitions.entry(key.clone()).or_default();
            partition.next_seq = maximum_seq;
            partition.digest_by_id.clear();
            partition.digest_by_seq.clear();
            partition.replay_order.clear();
            for record in entries.iter().rev().take(MAX_FEEDBACK_REPLAY_ENTRIES).rev() {
                let feedback = &record.feedback;
                let digest = feedback_digest(feedback);
                partition.digest_by_seq.insert(
                    feedback.feedback_seq,
                    (feedback.feedback_id.clone(), digest.clone()),
                );
                partition
                    .digest_by_id
                    .insert(feedback.feedback_id.clone(), digest);
                partition
                    .replay_order
                    .push_back(feedback.feedback_id.clone());
            }
            self.feedback_recovery_blocked.remove(key);
        }
        for (key, existing) in &self.feedback_partitions {
            if existing.next_seq > 0 && !groups.contains_key(key) {
                self.feedback_recovery_blocked.insert(
                    key.clone(),
                    "Runtime outbox does not contain the live feedback verifier history"
                        .to_string(),
                );
            }
        }
        self.feedback_recovery_initialized = self.feedback_recovery_blocked.is_empty();
        Ok(())
    }

    pub fn feedback_recovery_initialized(&self) -> bool {
        self.feedback_recovery_initialized
    }

    pub fn feedback_recovery_blocked(&self, agent: &str, session: &str) -> bool {
        self.feedback_recovery_blocked
            .contains_key(&(agent.to_string(), session.to_string()))
    }

    pub fn begin_turn(
        &mut self,
        turn: &ContinuousAgentTurnContextV1,
    ) -> Result<(), CognitionError> {
        turn.validate_for_agent(turn.agent_id.as_str())?;
        if let Some(reason) = self
            .feedback_recovery_blocked
            .get(&(turn.agent_id.clone(), turn.agent_session_id.clone()))
        {
            return Err(CognitionError::new(
                "feedback_recovery_blocked",
                reason.clone(),
            ));
        }
        if let Some(active) = self.active_by_agent.get(&turn.agent_id) {
            if active.session_id == turn.agent_session_id
                && active.turn_id == turn.agent_turn_id
                && active.request_id == turn.decision_request_id
                && active.request_digest == turn.request_digest
            {
                return Ok(());
            }
            return Err(CognitionError::new(
                "agent_busy",
                "an Agent already has an in-flight cognition request",
            ));
        }
        if self
            .digest_by_request_id
            .contains_key(&turn.decision_request_id)
        {
            if self
                .subject_by_request_id
                .get(&turn.decision_request_id)
                .is_some_and(|subject| subject != &turn.agent_id)
            {
                return Err(CognitionError::new(
                    "request_identity_collision",
                    "decision_request_id was reused by another Agent subject",
                ));
            }
            return Err(CognitionError::new(
                "request_replay",
                "decision_request_id was already admitted and is no longer active",
            ));
        }
        self.digest_by_request_id.insert(
            turn.decision_request_id.clone(),
            turn.request_digest.clone(),
        );
        self.subject_by_request_id
            .insert(turn.decision_request_id.clone(), turn.agent_id.clone());
        self.active_by_agent.insert(
            turn.agent_id.clone(),
            ActiveCognitionRequest {
                session_id: turn.agent_session_id.clone(),
                turn_id: turn.agent_turn_id.clone(),
                request_id: turn.decision_request_id.clone(),
                request_digest: turn.request_digest.clone(),
            },
        );
        Ok(())
    }

    pub fn begin_request(
        &mut self,
        request: ContinuousAgentRequestContextV1,
    ) -> Result<(), CognitionError> {
        request.validate()?;
        let key = (
            request.agent_subject.clone(),
            request.agent_session_id.clone(),
        );
        if let Some(reason) = self.feedback_recovery_blocked.get(&key) {
            return Err(CognitionError::new(
                "feedback_recovery_blocked",
                reason.clone(),
            ));
        }
        let digest = request.request_digest();
        if let Some(previous) = self.digest_by_request_id.get(&request.decision_request_id) {
            if previous != &digest
                || self
                    .subject_by_request_id
                    .get(&request.decision_request_id)
                    .is_some_and(|subject| subject != &request.agent_subject)
            {
                return Err(CognitionError::new(
                    "request_identity_collision",
                    "decision_request_id was reused with different canonical identity",
                ));
            }
            if self.active_by_agent.values().any(|active| {
                active.request_id == request.decision_request_id && active.request_digest == digest
            }) {
                return Ok(());
            }
            return Err(CognitionError::new(
                "request_replay",
                "decision_request_id was already admitted and is no longer active",
            ));
        }
        if self.active_by_agent.contains_key(&request.agent_subject) {
            return Err(CognitionError::new(
                "agent_busy",
                "an Agent already has an in-flight cognition request",
            ));
        }
        self.digest_by_request_id
            .insert(request.decision_request_id.clone(), digest.clone());
        self.subject_by_request_id.insert(
            request.decision_request_id.clone(),
            request.agent_subject.clone(),
        );
        self.active_by_agent.insert(
            request.agent_subject,
            ActiveCognitionRequest {
                session_id: request.agent_session_id,
                turn_id: request.agent_turn_id,
                request_id: request.decision_request_id,
                request_digest: digest,
            },
        );
        Ok(())
    }

    pub fn accept_feedback(&mut self, feedback: FeedbackEnvelopeV1) -> Result<(), CognitionError> {
        if !self.active_by_agent.contains_key(&feedback.agent_subject)
            && self
                .active_by_agent
                .keys()
                .any(|agent| agent != &feedback.agent_subject)
        {
            return Err(CognitionError::new(
                "cross_agent_feedback",
                "feedback does not belong to the active Agent subject",
            ));
        }
        feedback.validate()?;
        let partition_key = (
            feedback.agent_subject.clone(),
            feedback.agent_session_id.clone(),
        );
        let current_digest = feedback_digest(&feedback);
        let Some(active) = self.active_by_agent.get(&feedback.agent_subject) else {
            if let Some(partition) = self.feedback_partitions.get(&partition_key) {
                if let Some(previous) = partition.digest_by_id.get(&feedback.feedback_id) {
                    return Self::replay_or_reject_feedback(previous, current_digest);
                }
                if let Some((previous_id, previous_digest)) =
                    partition.digest_by_seq.get(&feedback.feedback_seq)
                    && (previous_id != &feedback.feedback_id || previous_digest != &current_digest)
                {
                    return Err(CognitionError::new(
                        "feedback_identity_collision",
                        "feedback_seq was reused with a different envelope",
                    ));
                }
            }
            return Err(CognitionError::new(
                "unknown_feedback",
                "feedback does not match an active cognition request",
            ));
        };
        let partition = self.feedback_partitions.entry(partition_key).or_default();
        if let Some(previous) = partition.digest_by_id.get(&feedback.feedback_id) {
            return Self::replay_or_reject_feedback(previous, current_digest);
        }
        if active.session_id != feedback.agent_session_id
            || active.turn_id != feedback.agent_turn_id
            || active.request_id != feedback.decision_request_id
        {
            return Err(CognitionError::new(
                "feedback_correlation_mismatch",
                "feedback cognition lineage does not match the active request",
            ));
        }
        if active.request_digest != feedback.request_digest {
            return Err(CognitionError::new(
                "feedback_digest_mismatch",
                "feedback request digest does not match the active request",
            ));
        }
        let expected = partition.next_seq.saturating_add(1);
        if feedback.feedback_seq > expected {
            if let Some(previous) = partition.held.get(&feedback.feedback_seq) {
                if feedback_digest(previous) != current_digest {
                    return Err(CognitionError::new(
                        "feedback_identity_collision",
                        "feedback_seq was reused with a different held envelope",
                    ));
                }
                return Err(CognitionError::new(
                    "feedback_sequence_gap",
                    format!("expected feedback sequence {expected}"),
                ));
            }
            if let Some(previous) = partition
                .held
                .values()
                .find(|previous| previous.feedback_id == feedback.feedback_id)
            {
                if feedback_digest(previous) != current_digest {
                    return Err(CognitionError::new(
                        "feedback_id_conflict",
                        "feedback_id was reused with a different held envelope",
                    ));
                }
                return Err(CognitionError::new(
                    "feedback_sequence_gap",
                    format!("expected feedback sequence {expected}"),
                ));
            }
            if partition.held.len() >= MAX_FEEDBACK_REPLAY_ENTRIES {
                return Err(CognitionError::new(
                    "feedback_sequence_overflow",
                    "feedback sequence gap exceeds the bounded replay window",
                ));
            }
            partition.held.insert(feedback.feedback_seq, feedback);
            return Err(CognitionError::new(
                "feedback_sequence_gap",
                format!("expected feedback sequence {expected}"),
            ));
        }
        if feedback.feedback_seq < expected {
            if let Some((previous_id, previous)) =
                partition.digest_by_seq.get(&feedback.feedback_seq)
            {
                if previous_id == &feedback.feedback_id {
                    return Self::replay_or_reject_feedback(previous, current_digest);
                }
                return Err(CognitionError::new(
                    "feedback_identity_collision",
                    "feedback_seq was reused with a different envelope",
                ));
            }
            return Err(CognitionError::new(
                "feedback_sequence_gap",
                format!(
                    "feedback sequence {} is behind {expected}",
                    feedback.feedback_seq
                ),
            ));
        }
        Self::remember_feedback(partition, feedback.feedback_id.clone(), current_digest);
        partition.next_seq = feedback.feedback_seq;
        if is_terminal_status(feedback.status.as_str()) {
            self.active_by_agent.remove(&feedback.agent_subject);
        }
        while let Some(next) = partition.held.remove(&partition.next_seq.saturating_add(1)) {
            let digest = feedback_digest(&next);
            Self::remember_feedback(partition, next.feedback_id.clone(), digest);
            partition.next_seq = next.feedback_seq;
            if is_terminal_status(next.status.as_str()) {
                self.active_by_agent.remove(&next.agent_subject);
            }
        }
        Ok(())
    }

    fn replay_or_reject_feedback(
        previous: &Digest32,
        current: Digest32,
    ) -> Result<(), CognitionError> {
        if previous == &current {
            return Ok(());
        }
        Err(CognitionError::new(
            "feedback_id_conflict",
            "feedback_id was reused with a different envelope",
        ))
    }

    fn remember_feedback(partition: &mut FeedbackPartition, feedback_id: String, digest: Digest32) {
        let sequence = partition.next_seq.saturating_add(1);
        partition
            .digest_by_seq
            .insert(sequence, (feedback_id.clone(), digest.clone()));
        partition.digest_by_id.insert(feedback_id.clone(), digest);
        partition.replay_order.push_back(feedback_id);
        while partition.replay_order.len() > MAX_FEEDBACK_REPLAY_ENTRIES {
            let Some(expired_id) = partition.replay_order.pop_front() else {
                break;
            };
            partition.digest_by_id.remove(&expired_id);
            if let Some(sequence) = partition
                .digest_by_seq
                .iter()
                .find_map(|(sequence, (id, _))| (id == &expired_id).then_some(*sequence))
            {
                partition.digest_by_seq.remove(&sequence);
            }
        }
    }

    pub fn block_feedback_recovery(
        &mut self,
        agent_subject: impl Into<String>,
        session_id: impl Into<String>,
        reason: impl Into<String>,
    ) {
        self.feedback_recovery_blocked
            .insert((agent_subject.into(), session_id.into()), reason.into());
    }

    pub fn clear_agent(&mut self, agent_subject: &str) {
        self.active_by_agent.remove(agent_subject);
    }

    pub fn contains_feedback(
        &self,
        agent_subject: &str,
        agent_session_id: &str,
        feedback_id: &str,
    ) -> bool {
        self.feedback_partitions
            .get(&(agent_subject.to_string(), agent_session_id.to_string()))
            .is_some_and(|partition| partition.digest_by_id.contains_key(feedback_id))
    }
}

fn is_terminal_status(status: &str) -> bool {
    matches!(status, "committed" | "rejected" | "failed")
}

#[cfg(test)]
mod tests {
    use super::*;
    use oasis7_agent_api::{GoalSnapshotV1, MemoryContextSnapshotV1};

    const DIGEST: &str = "blake3:1111111111111111111111111111111111111111111111111111111111111111";

    fn turn(agent: &str, request: &str) -> ContinuousAgentTurnContextV1 {
        ContinuousAgentTurnContextV1 {
            agent_id: agent.into(),
            agent_session_id: "session-1".into(),
            agent_turn_id: "turn-1".into(),
            decision_request_id: request.into(),
            request_digest: Digest32::from(DIGEST),
            memory_snapshot: MemoryContextSnapshotV1::empty("session_private"),
            goal_snapshot: GoalSnapshotV1::empty(),
            continuation: None,
        }
    }

    fn feedback(agent: &str, seq: u64, id: &str, status: &str) -> FeedbackEnvelopeV1 {
        FeedbackEnvelopeV1 {
            feedback_id: id.into(),
            feedback_seq: seq,
            agent_subject: agent.into(),
            agent_session_id: "session-1".into(),
            agent_turn_id: "turn-1".into(),
            decision_request_id: "request-1".into(),
            candidate_action_id: (status == "committed").then_some(7),
            runtime_receipt_id: (status == "committed").then(|| "receipt-1".into()),
            status: status.into(),
            request_digest: Digest32::from(DIGEST),
            reject_reason: None,
            provenance: "runtime_authoritative".into(),
        }
    }

    #[test]
    fn single_flight_replay_and_terminal_feedback_keep_the_existing_contract() {
        let mut store = AgentCognitionStore::default();
        store.begin_turn(&turn("agent-1", "request-1")).unwrap();
        store.begin_turn(&turn("agent-1", "request-1")).unwrap();
        assert_eq!(
            store
                .begin_turn(&turn("agent-1", "request-2"))
                .unwrap_err()
                .code(),
            "agent_busy"
        );
        store
            .accept_feedback(feedback("agent-1", 1, "feedback-1", "committed"))
            .unwrap();
        assert!(store.contains_feedback("agent-1", "session-1", "feedback-1"));
        assert_eq!(
            store
                .begin_turn(&turn("agent-1", "request-1"))
                .unwrap_err()
                .code(),
            "request_replay"
        );
    }

    #[test]
    fn feedback_sequence_gap_is_held_but_not_applied_until_contiguous() {
        let mut store = AgentCognitionStore::default();
        store.begin_turn(&turn("agent-1", "request-1")).unwrap();
        assert_eq!(
            store
                .accept_feedback(feedback("agent-1", 2, "feedback-2", "pending"))
                .unwrap_err()
                .code(),
            "feedback_sequence_gap"
        );
        store
            .accept_feedback(feedback("agent-1", 1, "feedback-1", "pending"))
            .unwrap();
        assert!(store.contains_feedback("agent-1", "session-1", "feedback-2"));
    }

    #[test]
    fn feedback_from_another_subject_cannot_close_active_turn() {
        let mut store = AgentCognitionStore::default();
        store.begin_turn(&turn("agent-1", "request-1")).unwrap();
        assert_eq!(
            store
                .accept_feedback(feedback("agent-2", 1, "forged", "failed"))
                .unwrap_err()
                .code(),
            "cross_agent_feedback"
        );
        assert_eq!(store.active_by_agent.len(), 1);
    }

    #[test]
    fn recovery_imports_acknowledged_replay_metadata_without_replaying_effects() {
        let history = vec![FeedbackHistoryProjection {
            feedback: feedback("agent-1", 1, "feedback-1", "committed"),
            acknowledged: true,
        }];
        let mut store = AgentCognitionStore::default();
        store.restore_validated_feedback_history(&history).unwrap();

        assert!(store.feedback_recovery_initialized());
        assert!(store.contains_feedback("agent-1", "session-1", "feedback-1"));
        assert!(store.active_by_agent.is_empty());
        assert!(store.feedback_recovery_blocked.is_empty());
    }

    #[test]
    fn incomplete_feedback_history_fences_only_its_agent_session() {
        let history = vec![FeedbackHistoryProjection {
            feedback: feedback("agent-1", 2, "feedback-gap", "committed"),
            acknowledged: false,
        }];
        let mut store = AgentCognitionStore::default();
        store.restore_validated_feedback_history(&history).unwrap();

        assert!(!store.feedback_recovery_initialized());
        assert!(store.feedback_recovery_blocked("agent-1", "session-1"));
        assert!(!store.contains_feedback("agent-1", "session-1", "feedback-gap"));
        assert_eq!(
            store
                .begin_turn(&turn("agent-1", "request-1"))
                .unwrap_err()
                .code(),
            "feedback_recovery_blocked"
        );
        store.begin_turn(&turn("agent-2", "request-2")).unwrap();
    }
}
