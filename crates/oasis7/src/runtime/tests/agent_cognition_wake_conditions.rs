//! P1 RED fixtures for WakeConditionV1 and continuation lifecycle recovery.
//!
//! Wake conditions are a bounded all-of registry, not an implicit timer or a
//! provider callback.  Continuation status and status digests are durable
//! Runtime projections and must invalidate on a trusted reorg.

use super::super::*;
use crate::runtime::{
    AgentContinuation, ContinuationStatusV1, ContinuationTransition, WakeConditionV1,
    WakeConditionValidator, WakeEvaluationContext,
};
use serde_json::{Value, json};

const WORLD_ID: &str = "world-wake-fixture";
const AGENT_ID: &str = "agent-wake-1";

fn condition(value: Value) -> WakeConditionV1 {
    serde_json::from_value(value).expect("decode WakeConditionV1 fixture")
}

fn valid(kind: &str) -> Value {
    match kind {
        "at_or_after_tick" => json!({
            "schema_version": "wake-condition.v1",
            "kind": kind,
            "logical_tick": 42
        }),
        "world_event_committed" => json!({
            "schema_version": "wake-condition.v1",
            "kind": kind,
            "event_digest": "digest:event-42"
        }),
        "receipt_linked" => json!({
            "schema_version": "wake-condition.v1",
            "kind": kind,
            "receipt_id": "receipt-42"
        }),
        "state_predicate" => json!({
            "schema_version": "wake-condition.v1",
            "kind": kind,
            "subject": {"kind": "world", "id": WORLD_ID},
            "path_or_rule": "world.logical_tick",
            "operator": "gte",
            "expected_value_bytes": [42]
        }),
        other => panic!("unknown test kind: {other}"),
    }
}

fn continuation(status: &str) -> AgentContinuation {
    serde_json::from_value(json!({
        "schema_version": "agent-continuation.v1",
        "continuation_id": "continuation-42",
        "wake_id": "wake-42",
        "world_id": WORLD_ID,
        "branch_id": "main",
        "finality_epoch": 7,
        "finality_block_hash": "hash:finality-7",
        "finality_status": "verified",
        "reorg_epoch": 3,
        "runtime_manifest_hash": "hash:runtime-manifest-7",
        "agent_id": AGENT_ID,
        "agent_session_id": "session.agent-wake-1",
        "agent_turn_id": "turn.agent-wake-1",
        "decision_request_id": "request.agent-wake-1",
        "origin_turn_id": "turn.agent-wake-1",
        "origin_request_digest": "digest:request-42",
        "continuation_proposal_id": "proposal-42",
        "proposal_digest": "digest:proposal-42",
        "action_or_envelope_digest": null,
        "wake_conditions": [valid("at_or_after_tick")],
        "next_wake_tick": 42,
        "remaining_budget": {"unit": "steps", "value": 2},
        "valid_until_tick": 100,
        "precondition_digest": "digest:precondition-42",
        "wake_seq": 1,
        "status": status,
        "terminal_disposition": null
    }))
    .expect("decode AgentContinuation fixture")
}

#[test]
fn every_wake_condition_kind_accepts_only_its_one_of_registry() {
    let conditions = [
        condition(valid("at_or_after_tick")),
        condition(valid("world_event_committed")),
        condition(valid("receipt_linked")),
        condition(valid("state_predicate")),
    ];
    for item in conditions {
        WakeConditionValidator::validate(std::slice::from_ref(&item))
            .expect("valid WakeConditionV1 item");
    }
}

#[test]
fn wake_conditions_are_nonempty_all_of_sorted_and_bounded() {
    let empty = WakeConditionValidator::validate(&[]).expect_err("empty wake list must fail");
    assert_eq!(empty.code(), "wake_conditions_empty");

    let first = condition(valid("receipt_linked"));
    let second = condition(valid("at_or_after_tick"));
    let canonical = WakeConditionValidator::canonicalize(vec![first.clone(), second.clone()])
        .expect("canonicalize valid all-of list");
    assert_eq!(canonical.len(), 2);
    let first_bytes = WakeConditionValidator::canonical_bytes(&canonical[0]);
    let second_bytes = WakeConditionValidator::canonical_bytes(&canonical[1]);
    assert!(
        first_bytes <= second_bytes,
        "conditions must be sorted by canonical CBOR bytes"
    );
    assert!(!WakeConditionValidator::conditions_digest(&canonical).is_empty());

    let duplicate = WakeConditionValidator::validate(&[first.clone(), first]);
    assert_eq!(
        duplicate.expect_err("duplicate condition").code(),
        "wake_condition_invalid"
    );

    let mut forbidden = valid("at_or_after_tick");
    forbidden["receipt_id"] = json!("receipt-forbidden");
    assert_eq!(
        WakeConditionValidator::validate(&[condition(forbidden)])
            .expect_err("forbidden field")
            .code(),
        "wake_condition_invalid"
    );

    let mut unknown = valid("state_predicate");
    unknown["path_or_rule"] = json!("world.unknown");
    assert_eq!(
        WakeConditionValidator::validate(&[condition(unknown)])
            .expect_err("unknown predicate")
            .code(),
        "wake_condition_invalid"
    );
}

#[test]
fn missing_reference_false_predicate_and_expired_reference_are_deterministic_pending_or_terminal() {
    let event = condition(valid("world_event_committed"));
    let receipt = condition(valid("receipt_linked"));
    let predicate = condition(valid("state_predicate"));

    let missing = WakeEvaluationContext::at(42);
    for item in [&event, &receipt, &predicate] {
        let result = serde_json::to_value(
            WakeConditionValidator::evaluate(std::slice::from_ref(item), &missing)
                .expect("missing/false condition evaluation"),
        )
        .expect("encode evaluation");
        assert_eq!(result["status"], "pending");
        assert_eq!(result["reason"], "condition_not_met");
    }

    let expired = WakeEvaluationContext::at(200).with_gc_references(&["digest:event-42"]);
    let result = serde_json::to_value(
        WakeConditionValidator::evaluate(&[event, receipt], &expired)
            .expect("expired condition evaluation"),
    )
    .expect("encode expiry evaluation");
    assert_eq!(result["status"], "expired");
    assert_eq!(result["reason"], "wake_condition_expired");
}

#[test]
fn continuation_status_transitions_are_closed_and_status_digest_is_stateful() {
    let allowed = [
        (
            ContinuationStatusV1::Scheduled,
            ContinuationStatusV1::Pending,
        ),
        (
            ContinuationStatusV1::Scheduled,
            ContinuationStatusV1::Waking,
        ),
        (ContinuationStatusV1::Pending, ContinuationStatusV1::Waking),
        (ContinuationStatusV1::Waking, ContinuationStatusV1::Consumed),
        (
            ContinuationStatusV1::Consumed,
            ContinuationStatusV1::Completed,
        ),
    ];
    for (from, to) in allowed {
        ContinuationTransition::validate(from, to).expect("documented continuation transition");
    }
    let invalid = ContinuationTransition::validate(
        ContinuationStatusV1::Completed,
        ContinuationStatusV1::Waking,
    )
    .expect_err("terminal continuation cannot reopen");
    assert_eq!(invalid.code(), "recovery_pending");

    let mut pending = continuation("pending");
    let pending_digest = pending.status_digest();
    ContinuationTransition::apply(&mut pending, ContinuationStatusV1::Waking)
        .expect("pending wakes");
    let waking_digest = pending.status_digest();
    assert_ne!(pending_digest, waking_digest);
    assert!(!pending_digest.is_empty());
}

#[test]
fn trusted_reorg_invalidates_uncommitted_continuation_without_provider_or_effect() {
    let mut pending = continuation("pending");
    let report = serde_json::to_value(
        ContinuationTransition::invalidate_for_reorg(&mut pending, 4).expect("reorg invalidation"),
    )
    .expect("encode reorg report");
    assert_eq!(pending.status, ContinuationStatusV1::Invalidated);
    assert_eq!(report["terminal_disposition"], "reorg_invalidated");
    assert_eq!(report["provider_invocation_count"], 0);
    assert_eq!(report["effect_count"], 0);
    assert_eq!(report["receipt_count"], 0);
}
