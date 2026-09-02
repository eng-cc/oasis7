//! LIVE-4 RED tests for World-owned cognition orchestration.
//!
//! The scheduler, wake, continuation and retention modules each have focused
//! in-memory fixtures.  Those fixtures do not prove that a real `World` owns
//! one durable lifecycle, restores its cursor and pins, or derives wake
//! decisions from the committed head.  This file is intentionally the next
//! integration seam: until the World APIs below exist, the focused filter must
//! fail for the missing production integration methods rather than silently
//! passing against a fixture-only implementation.

use super::super::*;
use crate::runtime::{
    AgentContinuation, ContinuationStatusV1, RetentionRecordV1, SchedulerPolicyV1, SchedulerWakeV1,
    WakeConditionV1,
};
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const WORLD_ID: &str = "world-live-scheduler";
const BRANCH_ID: &str = "main";
const AGENT_A: &str = "agent-live-a";
const AGENT_B: &str = "agent-live-b";

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("oasis7-live-scheduler-{label}-{nonce}"))
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
    .expect("decode live scheduler policy")
}

fn wake(
    agent_id: &str,
    wake_id: &str,
    continuation_id: &str,
    next_wake_tick: u64,
) -> SchedulerWakeV1 {
    serde_json::from_value(json!({
        "schema_version": "scheduler-wake.v1",
        "wake_id": wake_id,
        "continuation_id": continuation_id,
        "world_id": WORLD_ID,
        "branch_id": BRANCH_ID,
        "finality_epoch": 7,
        "finality_block_hash": "blake3:finality-live-7",
        "finality_status": "verified",
        "reorg_epoch": 3,
        "runtime_manifest_hash": "blake3:runtime-manifest-live-7",
        "agent_id": agent_id,
        "agent_session_id": format!("session.{agent_id}"),
        "agent_turn_id": format!("turn.{agent_id}"),
        "decision_request_id": format!("request.{agent_id}"),
        "next_wake_tick": next_wake_tick,
        "eligible_since_tick": 1,
        "starvation_deadline_tick": 5,
        "initial_priority": 0,
        "wake_seq": 1,
        "retry_seq": 0,
        "status": "pending",
        "pending_reason": "capacity_available"
    }))
    .expect("decode live scheduler wake")
}

fn continuation(status: ContinuationStatusV1) -> AgentContinuation {
    let mut continuation: AgentContinuation = serde_json::from_value(json!({
        "schema_version": "agent-continuation.v1",
        "continuation_id": "continuation-live-1",
        "wake_id": "wake-live-1",
        "world_id": WORLD_ID,
        "branch_id": BRANCH_ID,
        "finality_epoch": 7,
        "finality_block_hash": "blake3:finality-live-7",
        "finality_status": "verified",
        "reorg_epoch": 3,
        "runtime_manifest_hash": "blake3:runtime-manifest-live-7",
        "agent_id": AGENT_A,
        "agent_session_id": "session.agent-live-a",
        "agent_turn_id": "turn.agent-live-a",
        "decision_request_id": "request.agent-live-a",
        "origin_turn_id": "turn.agent-live-a",
        "origin_request_digest": "blake3:origin-request-live-1",
        "continuation_proposal_id": "proposal-live-1",
        "proposal_digest": "blake3:proposal-live-1",
        "action_or_envelope_digest": null,
        "wake_conditions": [{
            "schema_version": "wake-condition.v1",
            "kind": "receipt_linked",
            "receipt_id": "receipt-live-1"
        }],
        "next_wake_tick": null,
        "remaining_budget": {"unit": "steps", "value": 2},
        "valid_until_tick": 100,
        "precondition_digest": "blake3:precondition-live-1",
        "action_or_plan_kind": "wait",
        "baseline_observation_digest": "blake3:baseline-live-1",
        "goal_digest": "blake3:goal-live-1",
        "policy_digest": "blake3:policy-live-1",
        "policy_revision": 1,
        "precondition_summary": "ready",
        "source": "runtime-test",
        "wake_seq": 1,
        "status": status,
        "terminal_disposition": null
    }))
    .expect("decode live continuation");
    continuation.refresh_status_digest();
    continuation
}

fn world_with_scheduler() -> World {
    World::new().with_cognition_scheduler(policy(), 1)
}

#[test]
fn world_scheduler_is_nonblocking_fair_and_restores_cursor_and_backpressure() {
    let mut world = world_with_scheduler();
    let accepted = world
        .enqueue_cognition_wake(wake(AGENT_A, "wake-a", "continuation-a", 1))
        .expect("first wake should fit the bounded World scheduler");
    assert_eq!(accepted.disposition, "accepted");

    let pending = world
        .enqueue_cognition_wake(wake(AGENT_B, "wake-b", "continuation-b", 1))
        .expect("queue-full is a durable pending disposition, not a blocking error");
    assert_eq!(pending.disposition, "pending");
    assert_eq!(pending.reason, "scheduler_backpressure");
    assert_eq!(pending.provider_invocation_count, 0);
    assert_eq!(pending.effect_count, 0);
    assert_eq!(pending.debit_count, 0);
    assert_eq!(pending.receipt_count, 0);
    assert_eq!(pending.world_receipt_linked_count, 0);

    let selected = world
        .select_ready_cognition_wakes(1)
        .expect("World should select ready wakes without provider I/O");
    assert_eq!(
        selected
            .iter()
            .map(|wake| wake.wake_id.as_str())
            .collect::<Vec<_>>(),
        ["wake-a"]
    );

    let before = world.cognition_scheduler_snapshot();
    let dir = temp_dir("scheduler-round-trip");
    world.save_to_dir(&dir).expect("save live scheduler state");
    let mut restored = World::load_from_dir(&dir).expect("restore live scheduler state");
    assert_eq!(restored.cognition_scheduler_snapshot(), before);

    let recovered = restored
        .recover_cognition_scheduler(2)
        .expect("capacity recovery should preserve the original wake identity and age");
    assert_eq!(
        recovered
            .iter()
            .map(|wake| wake.wake_id.as_str())
            .collect::<Vec<_>>(),
        ["wake-b"]
    );
    let after = restored.cognition_scheduler_snapshot();
    assert_eq!(after["policy"], before["policy"]);
    assert_eq!(after["cursor"], before["cursor"]);
    assert_eq!(after["backpressure_count"], 0);
    assert_eq!(after["active"][0]["wake_id"], "wake-b");
    assert_eq!(after["active"][0]["eligible_since_tick"], 1);
    assert_eq!(after["active"][0]["retry_seq"], 0);
    assert_eq!(
        restored.cognition_execution_metrics()["provider_invocation_count"],
        0
    );
    assert_eq!(restored.cognition_execution_metrics()["effect_count"], 0);
    assert_eq!(restored.cognition_execution_metrics()["debit_count"], 0);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_wake_evaluation_reads_one_committed_head_for_event_receipt_and_state() {
    let world = world_with_scheduler();
    let conditions: Vec<WakeConditionV1> = serde_json::from_value(json!([
        {
            "schema_version": "wake-condition.v1",
            "kind": "world_event_committed",
            "event_digest": "blake3:committed-event-live-1"
        },
        {
            "schema_version": "wake-condition.v1",
            "kind": "receipt_linked",
            "receipt_id": "receipt-live-1"
        },
        {
            "schema_version": "wake-condition.v1",
            "kind": "state_predicate",
            "subject": {"kind": "world", "id": WORLD_ID},
            "path_or_rule": "world.logical_tick",
            "operator": "gte",
            "expected_value_bytes": [0]
        }
    ]))
    .expect("decode live wake conditions");

    let evaluation = world
        .evaluate_cognition_wake(&conditions)
        .expect("World should evaluate wake conditions from its committed head");
    assert_eq!(evaluation.status, "pending");
    assert_eq!(evaluation.reason, "condition_not_met");

    let committed = world
        .evaluate_cognition_wake_from_committed_projection(
            &conditions,
            "blake3:committed-event-live-1",
            "receipt-live-1",
        )
        .expect("committed event/receipt projection should produce a deterministic wake");
    assert_eq!(committed.status, "ready");
    assert_eq!(committed.reason, "condition_met");
}

#[test]
fn world_continuation_reorg_invalidates_durable_schedule_without_reexecution() {
    let mut world = world_with_scheduler();
    world
        .schedule_cognition_continuation(continuation(ContinuationStatusV1::Scheduled))
        .expect("schedule continuation through World ownership");

    let report = world
        .invalidate_cognition_for_reorg(4)
        .expect("reorg should invalidate the pending continuation");
    assert_eq!(report["terminal_disposition"], "reorg_invalidated");
    assert_eq!(report["provider_invocation_count"], 0);
    assert_eq!(report["effect_count"], 0);
    assert_eq!(report["receipt_count"], 0);

    let state = world.cognition_continuations();
    assert_eq!(state[0]["status"], "invalidated");
    assert_eq!(state[0]["terminal_disposition"], "reorg_invalidated");
}

#[test]
fn world_retention_gc_keeps_terminal_pins_and_replay_is_read_only_after_restore() {
    let mut world = world_with_scheduler();
    world
        .record_cognition_terminal(terminal_record())
        .expect("record terminal cognition outcome through World ownership");
    world.pin_cognition_reference("key:live-1", "pending_wake");

    let before = world.cognition_execution_metrics();
    let gc = world
        .gc_cognition(1_000, 1_000)
        .expect("GC should honor the active continuation pin");
    assert_eq!(gc.deleted_count, 0);
    assert!(gc.pinned_reference_count >= 1);

    let dir = temp_dir("retention-replay");
    world.save_to_dir(&dir).expect("save pinned terminal state");
    let mut restored = World::load_from_dir(&dir).expect("restore pinned terminal state");
    let replay = restored
        .replay_cognition_terminal("key:live-1", "blake3:envelope-live-1")
        .expect("terminal replay should read its canonical record");
    assert_eq!(replay["provider_invocation_count"], 0);
    assert_eq!(replay["effect_delta"], 0);
    assert_eq!(replay["world_receipt_linked_delta"], 0);
    assert_eq!(restored.cognition_execution_metrics(), before);

    let _ = fs::remove_dir_all(dir);
}

fn terminal_record() -> RetentionRecordV1 {
    serde_json::from_value(json!({
        "schema_version": "cognition-retention-record.v1",
        "world_id": WORLD_ID,
        "envelope_idempotency_key": "key:live-1",
        "envelope_digest": "blake3:envelope-live-1",
        "status": "committed",
        "base_tick": 0,
        "issued_at_tick": 0,
        "terminal_disposition": null,
        "receipt_id": "receipt-live-1",
        "receipt_digest": "blake3:receipt-live-1",
        "response_artifact_id": "artifact:live-1",
        "continuation_id": "continuation-live-1",
        "commit_record_id": "commit:live-1"
    }))
    .expect("decode terminal cognition record")
}

#[test]
fn world_scheduler_state_is_durable_json_and_contains_no_provider_effect_or_debit_replay() {
    let mut world = world_with_scheduler();
    world
        .enqueue_cognition_wake(wake(AGENT_A, "wake-json", "continuation-json", 1))
        .expect("enqueue live wake");
    let state: Value = world.cognition_scheduler_snapshot();
    assert_eq!(state["policy"]["schema_version"], "scheduler-policy.v1");
    assert_eq!(state["cursor"]["schema_version"], "scheduler-cursor.v1");
    assert_eq!(state["metrics"]["provider_invocation_count"], 0);
    assert_eq!(state["metrics"]["effect_count"], 0);
    assert_eq!(state["metrics"]["debit_count"], 0);
}
