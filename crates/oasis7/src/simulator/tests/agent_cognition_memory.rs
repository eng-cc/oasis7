//! Harness-policy RED fixtures for memory retrieval and write intents.
//!
//! This file deliberately stays inside the P0 Agent-private policy: shared
//! world/player-visible memory, belief/preference correction and cognition
//! billing are not part of this contract.

use crate::simulator::{
    AgentBehavior, AgentDecision, AsyncAgentRunner, AsyncAgentTurnOutcome,
    ContinuousAgentTurnContextV1, MemoryContextSnapshotV1, MemoryWriteIntent,
    MemoryWriteIntentPolicyV1, MemoryWriteIntentV1, MemoryWritePolicyContextV1,
    MemoryWritePolicyOutcome, MemoryWriteStore, Observation,
};
use serde_json::json;

const AGENT_ID: &str = "agent-memory-1";
const SESSION_ID: &str = "session-memory-1";
const TURN_ID: &str = "turn-memory-1";
const REQUEST_DIGEST: &str =
    "blake3:1111111111111111111111111111111111111111111111111111111111111111";

fn context() -> MemoryWritePolicyContextV1 {
    serde_json::from_value(json!({
        "agent_id": AGENT_ID,
        "agent_session_id": SESSION_ID,
        "agent_turn_id": TURN_ID,
        "request_digest": REQUEST_DIGEST,
        "source": "provider",
        "provenance": "provider_unverified"
    }))
    .expect("decode memory policy context")
}

fn intent(scope: &str, summary: Option<&str>, tags: &[&str]) -> MemoryWriteIntentV1 {
    serde_json::from_value(json!({
        "schema_version": 1,
        "scope": scope,
        "summary": summary,
        "tags": tags
    }))
    .expect("decode MemoryWriteIntentV1")
}

#[test]
fn empty_memory_snapshot_is_explicit_and_digest_stable() {
    let first = MemoryContextSnapshotV1::empty("turn_private");
    let second = MemoryContextSnapshotV1::empty("turn_private");
    assert_eq!(first.revision, 0);
    assert!(first.entries.is_empty());
    assert_eq!(first.scope, "turn_private");
    assert!(!first.digest.is_empty());
    assert_eq!(first.digest, second.digest);
    assert_eq!(
        serde_json::to_value(&first).expect("encode empty memory snapshot")["digest"],
        serde_json::to_value(&second).expect("encode empty memory snapshot")["digest"]
    );

    let encoded = serde_json::to_value(&first).expect("encode explicit empty snapshot");
    assert!(encoded.get("entries").is_some());
    assert!(encoded.get("scope").is_some());
    assert!(encoded.get("digest").is_some());
    assert!(
        MemoryContextSnapshotV1::from_value(json!({
            "revision": 0,
            "entries": null,
            "scope": "turn_private",
            "digest": first.digest
        }))
        .is_err(),
        "null must not stand in for the canonical empty entries list"
    );
}

#[test]
fn memory_intent_normalizes_nfc_trim_and_only_dedupes_sorted_tags() {
    let policy = MemoryWriteIntentPolicyV1::default();
    let normalized = policy
        .normalize(
            intent("session_private", Some(" e\u{301} "), &[" z ", "a", "a"]),
            &context(),
        )
        .expect("valid private memory intent");
    assert_eq!(normalized.summary.as_deref(), Some("é"));
    assert_eq!(normalized.tags, ["a", "z"]);
    let digest = policy
        .intent_digest(&normalized, &context())
        .expect("memory intent digest");
    assert!(digest.as_str().starts_with("blake3:"));

    let reordered = policy
        .normalize(
            intent("session_private", Some("é"), &["a", "z"]),
            &context(),
        )
        .expect("equivalent normalized intent");
    assert_eq!(
        policy
            .intent_digest(&reordered, &context())
            .expect("reordered digest"),
        digest
    );
}

#[test]
fn memory_policy_rejects_scope_aliases_in_target_lane_and_all_bounds_fail_closed() {
    let policy = MemoryWriteIntentPolicyV1::default();
    assert_eq!(
        policy
            .normalize(intent("short_term", Some("legacy"), &[]), &context())
            .expect_err("target lane must not guess legacy scope")
            .code(),
        "memory_scope_denied"
    );
    let legacy = policy
        .normalize_legacy(intent("short_term", Some(" legacy "), &[]), &context())
        .expect("explicit legacy_v1 scope alias");
    let legacy = serde_json::to_value(legacy).expect("encode legacy projection");
    assert_eq!(legacy["scope"], "turn_private");
    assert_eq!(legacy["compatibility_reason"], "memory_scope_alias_used");

    assert_eq!(
        policy
            .normalize(intent("turn_private", Some("   "), &[]), &context())
            .expect_err("empty summary")
            .code(),
        "memory_summary_invalid"
    );
    assert_eq!(
        policy
            .normalize(
                intent("turn_private", Some(&"x".repeat(513)), &[]),
                &context()
            )
            .expect_err("summary bound")
            .code(),
        "memory_summary_too_large"
    );
    assert_eq!(
        policy
            .normalize(
                intent(
                    "turn_private",
                    Some("ok"),
                    &["1", "2", "3", "4", "5", "6", "7", "8", "9"]
                ),
                &context(),
            )
            .expect_err("tag count bound")
            .code(),
        "memory_tag_count_exceeded"
    );
    assert_eq!(
        policy
            .normalize(intent("turn_private", Some("ok"), &["\0"]), &context())
            .expect_err("control tag")
            .code(),
        "memory_tag_invalid"
    );
}

#[test]
fn memory_write_requires_runtime_authority_and_committed_receipt_for_every_scope() {
    let policy = MemoryWriteIntentPolicyV1::default();
    let normalized = policy
        .normalize(intent("turn_private", Some("fact"), &[]), &context())
        .expect("valid intent");
    let digest = policy
        .intent_digest(&normalized, &context())
        .expect("intent digest");
    let mut store = MemoryWriteStore::default();

    for outcome in [
        MemoryWritePolicyOutcome::Rejected,
        MemoryWritePolicyOutcome::Failed,
        MemoryWritePolicyOutcome::Pending,
    ] {
        let result = store
            .apply(normalized.clone(), digest.clone(), outcome)
            .expect_err("noncommitted outcome cannot write authoritative memory");
        assert_eq!(result.code(), "memory_no_committed_outcome");
        assert!(store.entries().is_empty());
    }

    let committed = MemoryWritePolicyOutcome::Committed {
        receipt_id: "receipt-memory-1".to_string(),
        provenance: "runtime_authoritative".to_string(),
    };
    store
        .apply(normalized.clone(), digest.clone(), committed.clone())
        .expect("committed receipt gates one memory write");
    assert_eq!(store.entries().len(), 1);
    store
        .apply(normalized, digest, committed)
        .expect("same intent digest is idempotent");
    assert_eq!(store.entries().len(), 1);
    assert_eq!(store.entries()[0]["provenance"], "runtime_authoritative");
    assert_eq!(store.entries()[0]["receipt_id"], "receipt-memory-1");
}

#[test]
fn memory_digest_context_is_tied_to_agent_turn_and_request_not_provider_provenance() {
    let policy = MemoryWriteIntentPolicyV1::default();
    let normalized = policy
        .normalize(intent("turn_private", Some("fact"), &[]), &context())
        .expect("valid intent");
    let first = policy
        .intent_digest(&normalized, &context())
        .expect("first digest");
    let mut other = context();
    other.agent_id = "agent-memory-2".to_string();
    other.provenance = "runtime_fact".to_string();
    let second = policy
        .intent_digest(&normalized, &other)
        .expect("second digest");
    assert_ne!(first, second);
    assert_ne!(other.provenance, "runtime_authoritative");
}

struct MemoryCandidateBehavior;

impl AgentBehavior for MemoryCandidateBehavior {
    fn agent_id(&self) -> &str {
        AGENT_ID
    }

    fn decide(&mut self, _observation: &Observation) -> AgentDecision {
        AgentDecision::Wait
    }

    fn take_memory_write_intents(&mut self) -> Vec<MemoryWriteIntent> {
        vec![MemoryWriteIntent {
            scope: "turn_private".to_string(),
            summary: "candidate awaiting a Runtime action receipt".to_string(),
            tags: vec!["candidate".to_string()],
        }]
    }
}

fn turn_context() -> ContinuousAgentTurnContextV1 {
    ContinuousAgentTurnContextV1 {
        agent_id: AGENT_ID.to_string(),
        agent_session_id: SESSION_ID.to_string(),
        agent_turn_id: TURN_ID.to_string(),
        decision_request_id: "request-memory-1".to_string(),
        request_digest: serde_json::from_value(json!(REQUEST_DIGEST))
            .expect("decode request digest"),
        memory_snapshot: MemoryContextSnapshotV1::empty("turn_private"),
        goal_snapshot: crate::simulator::GoalSnapshotV1::empty(),
        continuation: None,
    }
}

fn completed_turn(runner: &mut AsyncAgentRunner) -> AsyncAgentTurnOutcome {
    for _ in 0..1024 {
        if let Some(outcome) = runner
            .poll_completed()
            .expect("poll memory candidate actor")
            .into_iter()
            .next()
        {
            return outcome;
        }
        std::thread::yield_now();
    }
    panic!("memory candidate actor did not complete within bounded polling budget");
}

#[test]
fn memory_write_rejects_caller_signed_receipt_without_runtime_action_feedback_lineage() {
    let mut runner = AsyncAgentRunner::new(1).expect("create memory candidate runner");
    runner
        .register(MemoryCandidateBehavior)
        .expect("register memory candidate actor");
    runner
        .start_turn_with_context(AGENT_ID, turn_context())
        .expect("open memory candidate turn");
    let outcome = completed_turn(&mut runner);
    assert_eq!(outcome.memory_write_intents.len(), 1);

    // This envelope has the right local request/turn identity, but the
    // receipt and candidate action are caller-selected rather than issued and
    // linked by Runtime.  The write gate must reject it before persistence.
    let mut forged = outcome
        .feedback_for_runtime_status("committed", Some("caller-forged-receipt"))
        .expect("construct the forged feedback envelope");
    forged.candidate_action_id = Some(7_777);

    let mut store = MemoryWriteStore::default();
    runner
        .consume_runtime_feedback(AGENT_ID, forged, &mut store)
        .expect_err("caller-signed receipt without Runtime lineage must be rejected");
    assert!(store.entries().is_empty());
}
