//! RED fixtures for the World-owned cognition authority/remediation seam.

use super::super::*;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const WORLD_ID: &str = "world-runtime-hardening";
const AGENT_ID: &str = "agent-runtime-hardening";

fn digest(seed: u8) -> String {
    format!("blake3:{}", char::from(b'a' + seed).to_string().repeat(64))
}

fn envelope(world: &World) -> AgentDecisionEnvelopeV1 {
    let request_digest = digest(0);
    let mut envelope = AgentDecisionEnvelopeV1 {
        schema_version: AGENT_DECISION_ENVELOPE_V1_SCHEMA.to_string(),
        world_id: WORLD_ID.to_string(),
        agent_id: AGENT_ID.to_string(),
        branch_id: "main".to_string(),
        finality_epoch: 0,
        finality_block_hash: None,
        finality_status: "pending".to_string(),
        agent_session_id: "session.runtime-hardening".to_string(),
        agent_turn_id: "turn.runtime-hardening".to_string(),
        decision_request_id: "request.runtime-hardening".to_string(),
        retry_seq: 1,
        base_tick: world.state().time,
        base_world_hash: world.current_state_root_hash().expect("world root"),
        reorg_epoch: 0,
        runtime_manifest_hash: world.current_manifest_hash().expect("manifest hash"),
        capability_snapshot_hash: digest(1),
        authority_context_hash: digest(2),
        observation_digest: digest(3),
        context_digest: digest(4),
        issued_at_tick: world.state().time,
        valid_until_tick: world.state().time + 10,
        preconditions: Vec::new(),
        decision_kind: "wait".to_string(),
        action: json!({"type": "wait"}),
        request_digest,
        decision_digest: String::new(),
        envelope_digest: String::new(),
        provider_invocation_key: String::new(),
        envelope_idempotency_key: String::new(),
        origin_intent_ref: None,
        source: "runtime-hardening-test".to_string(),
    };
    envelope.decision_digest = envelope.derive_decision_digest();
    envelope.envelope_digest = envelope.derive_envelope_digest();
    envelope.provider_invocation_key = envelope.derive_provider_invocation_key();
    envelope.envelope_idempotency_key = envelope.derive_envelope_idempotency_key();
    envelope
}

fn policy() -> SchedulerPolicyV1 {
    serde_json::from_value(json!({
        "schema_version": "scheduler-policy.v1",
        "max_total_wakes_per_tick": 8,
        "max_wakes_per_agent_per_tick": 1,
        "aging_after_ticks": 2,
        "max_starvation_ticks": 4,
        "initial_priority": 0,
        "comparator": "deadline_due_desc,next_wake_tick_asc,effective_priority_desc,starvation_deadline_tick_asc,cursor_distance_asc,agent_id_asc,continuation_id_asc,wake_seq_asc",
        "service_order": "stable_round_robin"
    }))
    .expect("policy")
}

fn proposal(world: &World) -> CognitionContinuationProposalV1 {
    let mut proposal = CognitionContinuationProposalV1 {
        schema_version: 1,
        continuation_proposal_id: "proposal.runtime-hardening".to_string(),
        world_id: WORLD_ID.to_string(),
        branch_id: "main".to_string(),
        finality_epoch: 0,
        finality_block_hash: None,
        finality_status: "pending".to_string(),
        reorg_epoch: 0,
        runtime_manifest_hash: world.current_manifest_hash().expect("manifest hash"),
        agent_id: AGENT_ID.to_string(),
        agent_session_id: "session.runtime-hardening".to_string(),
        agent_turn_id: "turn.runtime-hardening".to_string(),
        decision_request_id: "request.runtime-hardening".to_string(),
        origin_turn_id: "turn.runtime-hardening".to_string(),
        origin_request_digest: digest(5),
        action_or_plan_kind: "wait".to_string(),
        action_or_envelope_digest: None,
        remaining_budget: ContinuationBudgetV1 {
            unit: "steps".to_string(),
            value: 2,
        },
        baseline_observation_digest: digest(6),
        goal_digest: digest(7),
        policy_digest: digest(8),
        policy_revision: 1,
        precondition_summary: "ready".to_string(),
        precondition_digest: digest(9),
        wake_conditions: vec![WakeConditionV1 {
            schema_version: "wake-condition.v1".to_string(),
            kind: "at_or_after_tick".to_string(),
            logical_tick: Some(world.state().time + 1),
            event_digest: None,
            receipt_id: None,
            subject: None,
            path_or_rule: None,
            operator: None,
            expected_value_bytes: None,
        }],
        next_wake_tick: Some(world.state().time + 1),
        valid_until_tick: Some(world.state().time + 10),
        source: "runtime-hardening-test".to_string(),
        proposal_digest: String::new(),
    };
    proposal.proposal_digest = proposal.proposal_digest();
    proposal
}

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("oasis7-runtime-hardening-{label}-{nonce}"))
}

fn read_snapshot(dir: &Path) -> Value {
    serde_json::from_slice(&fs::read(dir.join("snapshot.json")).expect("snapshot"))
        .expect("decode snapshot")
}

fn write_snapshot(dir: &Path, snapshot: &Value) {
    fs::write(
        dir.join("snapshot.json"),
        serde_json::to_vec_pretty(snapshot).expect("encode snapshot"),
    )
    .expect("write snapshot");
}

#[test]
fn world_commit_prepare_finalize_is_bound_and_receipt_is_world_derived() {
    let mut world = World::new();
    let envelope = envelope(&world);
    let prepared = world
        .prepare_cognition_envelope(envelope.clone(), Some(json!({"decision": "wait"})))
        .expect("prepare through World");
    assert_eq!(prepared.status, "prepared");
    assert_eq!(prepared.parent_world_hash, envelope.base_world_hash);
    let committed = world
        .finalize_cognition_commit(&prepared.commit_id)
        .expect("finalize through World");
    assert_eq!(committed.status, "committed");
    let lineage = world
        .read_runtime_receipt_lineage(&committed.receipt_id)
        .expect("read World-derived receipt lineage");
    assert_eq!(lineage.agent_id, envelope.agent_id);
    assert_eq!(lineage.request_digest, envelope.request_digest);
    assert!(world.verify_runtime_receipt_lineage(&lineage).is_ok());
}

#[test]
fn restore_rejects_tampered_canonical_response_artifact() {
    let mut world = World::new();
    let envelope = envelope(&world);
    let prepared = world
        .prepare_cognition_envelope(envelope, Some(json!({"decision": "wait"})))
        .expect("prepare");
    world
        .finalize_cognition_commit(&prepared.commit_id)
        .expect("finalize");
    let dir = temp_dir("tampered-response");
    world.save_to_dir(&dir).expect("save");
    let mut snapshot = read_snapshot(&dir);
    snapshot["cognition"]["responses"][0]["response_artifact"]["decision"] = json!("tampered");
    write_snapshot(&dir, &snapshot);
    fs::remove_dir_all(dir.join(".distfs-state")).expect("force JSON restore");
    assert!(
        World::load_from_dir(&dir).is_err(),
        "tampered response must fail closed"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn continuation_admission_recomputes_digest_binds_authority_and_enqueues_wake() {
    let mut world = World::new()
        .try_with_cognition_scheduler(policy(), 2)
        .expect("validated scheduler");
    let proposal = proposal(&world);
    let admitted = world
        .admit_cognition_continuation(proposal.clone())
        .expect("admit proposal");
    assert_eq!(
        admitted.continuation_proposal_id,
        proposal.continuation_proposal_id
    );
    assert!(
        world.cognition()["scheduler_state"]["active"]
            .as_array()
            .expect("active wakes")
            .iter()
            .any(|wake| wake["wake_id"] == admitted.wake_id)
    );

    let mut forged = proposal;
    forged.proposal_digest = digest(10);
    assert!(world.admit_cognition_continuation(forged).is_err());
}

#[test]
fn scheduler_constructor_is_fallible_and_foreign_wake_is_rejected() {
    let invalid = SchedulerPolicyV1 {
        max_total_wakes_per_tick: 0,
        ..policy()
    };
    assert!(
        World::new()
            .try_with_cognition_scheduler(invalid, 1)
            .is_err()
    );

    let mut world = World::new()
        .try_with_cognition_scheduler(policy(), 1)
        .expect("scheduler");
    world
        .bind_cognition_runtime(WORLD_ID, "main", 0, None, "pending", 0)
        .expect("bind scheduler identity");
    let wake: SchedulerWakeV1 = serde_json::from_value(json!({
        "schema_version": "scheduler-wake.v1",
        "wake_id": "wake-foreign",
        "continuation_id": "continuation-foreign",
        "world_id": "other-world",
        "branch_id": "main",
        "finality_epoch": 0,
        "finality_block_hash": "",
        "finality_status": "pending",
        "reorg_epoch": 0,
        "runtime_manifest_hash": digest(11),
        "agent_id": AGENT_ID,
        "agent_session_id": "session.runtime-hardening",
        "agent_turn_id": "turn.runtime-hardening",
        "decision_request_id": "request.runtime-hardening",
        "next_wake_tick": 1,
        "eligible_since_tick": 0,
        "starvation_deadline_tick": 4,
        "initial_priority": 0,
        "wake_seq": 1,
        "retry_seq": 0,
        "status": "pending",
        "pending_reason": "capacity_available"
    }))
    .expect("wake");
    assert!(world.enqueue_cognition_wake(wake).is_err());
}

#[test]
fn response_replay_rejects_missing_or_mismatched_journal_head() {
    let mut world = World::new();
    let envelope = envelope(&world);
    let prepared = world
        .prepare_cognition_envelope(envelope, Some(json!({"decision": "wait"})))
        .expect("prepare");
    world
        .finalize_cognition_commit(&prepared.commit_id)
        .expect("finalize");
    let mut projection = world.cognition().clone();
    projection["responses"][0]["journal_head"] = json!("blake3:bad");
    let dir = temp_dir("tampered-journal-head");
    world.save_to_dir(&dir).expect("save");
    let mut snapshot = read_snapshot(&dir);
    snapshot["cognition"] = projection;
    write_snapshot(&dir, &snapshot);
    fs::remove_dir_all(dir.join(".distfs-state")).expect("force JSON restore");
    assert!(World::load_from_dir(&dir).is_err());
    let _ = fs::remove_dir_all(dir);
}
