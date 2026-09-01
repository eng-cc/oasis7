//! Harness-policy RED fixtures for ContinuationProposalV1.
//!
//! Harness owns the bounded proposal and policy fields; Runtime owns the
//! durable schedule/status projection and finality truth.  These fixtures
//! intentionally do not cover GoalGraph, belief/preference state or billing.

use crate::simulator::{
    ContinuationHarness, ContinuationInvalidationReason, ContinuationProposalV1,
    RuntimeContinuationStatusV1, h_v1,
};
use serde_json::{Value, json};

const WORLD_ID: &str = "world-continuation-fixture";
const AGENT_ID: &str = "agent-continuation-1";
const REQUEST_DIGEST: &str =
    "blake3:2222222222222222222222222222222222222222222222222222222222222222";

fn proposal_value() -> Value {
    json!({
        "schema_version": 1,
        "continuation_proposal_id": "proposal-1",
        "world_id": WORLD_ID,
        "agent_id": AGENT_ID,
        "agent_session_id": "session-continuation-1",
        "agent_turn_id": "turn-continuation-1",
        "decision_request_id": "request-continuation-1",
        "origin_turn_id": "turn-continuation-1",
        "origin_request_digest": REQUEST_DIGEST,
        "action_or_plan_kind": "wait_for_receipt",
        "action_or_envelope_digest": null,
        "remaining_budget": {"unit": "steps", "value": 2},
        "baseline_observation_digest": "blake3:3333333333333333333333333333333333333333333333333333333333333333",
        "goal_digest": "blake3:4444444444444444444444444444444444444444444444444444444444444444",
        "policy_digest": "blake3:5555555555555555555555555555555555555555555555555555555555555555",
        "policy_revision": 3,
        "precondition_summary": "receipt pending",
        "precondition_digest": "blake3:6666666666666666666666666666666666666666666666666666666666666666",
        "wake_conditions": [{
            "schema_version": "wake-condition.v1",
            "kind": "receipt_linked",
            "receipt_id": "receipt-1"
        }],
        "valid_until_tick": 100,
        "source": "harness",
        "proposal_digest": null
    })
}

fn proposal() -> ContinuationProposalV1 {
    let mut value = proposal_value();
    let mut digest_input = value.clone();
    digest_input
        .as_object_mut()
        .expect("proposal object")
        .remove("proposal_digest");
    let digest = h_v1("oasis7.cognition.continuation-proposal.v1", &digest_input);
    value["proposal_digest"] = json!(digest.as_str());
    serde_json::from_value(value).expect("decode ContinuationProposalV1")
}

fn runtime_status(status: &str, reason: Option<&str>) -> RuntimeContinuationStatusV1 {
    serde_json::from_value(json!({
        "status": status,
        "terminal_disposition": reason,
        "continuation_id": "continuation-runtime-1",
        "wake_id": "wake-runtime-1",
        "wake_seq": 1,
        "continuation_digest": "blake3:8888888888888888888888888888888888888888888888888888888888888888"
    }))
    .expect("decode Runtime continuation status projection")
}

#[test]
fn proposal_canonical_digest_covers_full_identity_binding_policy_and_budget() {
    let first = proposal();
    first.validate().expect("valid continuation proposal");
    let first_digest = first.proposal_digest().expect("proposal digest");
    assert!(first_digest.as_str().starts_with("blake3:"));
    assert_eq!(
        first_digest,
        first.proposal_digest().expect("stable digest")
    );

    for (field, changed) in [
        ("agent_session_id", json!("session-other")),
        ("origin_request_digest", json!("blake3:other-request")),
        (
            "baseline_observation_digest",
            json!("blake3:other-observation"),
        ),
        ("goal_digest", json!("blake3:other-goal")),
        ("policy_digest", json!("blake3:other-policy")),
    ] {
        let mut value = serde_json::to_value(&first).expect("encode proposal");
        value[field] = changed;
        let candidate: ContinuationProposalV1 =
            serde_json::from_value(value).expect("decode changed proposal");
        assert_ne!(
            first_digest,
            candidate
                .proposal_digest()
                .expect("changed proposal digest"),
            "{field} must remain in proposal identity"
        );
    }

    for unit in ["steps", "ticks"] {
        let mut value = serde_json::to_value(&first).expect("encode proposal");
        value["remaining_budget"] = json!({"unit": unit, "value": 2});
        let mut candidate: ContinuationProposalV1 =
            serde_json::from_value(value).expect("decode budget variant");
        candidate.proposal_digest = candidate
            .proposal_digest()
            .expect("recompute bounded budget digest")
            .to_string();
        candidate.validate().expect("bounded budget variant");
        assert!(candidate.remaining_budget.value > 0);
    }
}

#[test]
fn invalid_proposal_binding_budget_or_digest_fails_closed_without_projection() {
    let mut invalid = serde_json::to_value(proposal()).expect("encode proposal");
    invalid["agent_session_id"] = Value::Null;
    serde_json::from_value::<ContinuationProposalV1>(invalid).expect_err("missing binding");

    let mut invalid_budget = serde_json::to_value(proposal()).expect("encode proposal");
    invalid_budget["remaining_budget"] = json!({"unit": "minutes", "value": 2});
    let error = serde_json::from_value::<ContinuationProposalV1>(invalid_budget)
        .expect("decode bounded input for validation")
        .validate()
        .expect_err("unknown budget unit");
    assert_eq!(error.code(), "continuation_budget_invalid");

    let mut invalid_digest = serde_json::to_value(proposal()).expect("encode proposal");
    invalid_digest["proposal_digest"] = json!("blake3:tampered");
    let candidate: ContinuationProposalV1 =
        serde_json::from_value(invalid_digest).expect("decode tampered digest");
    assert_eq!(
        candidate
            .validate()
            .expect_err("tampered proposal digest")
            .code(),
        "continuation_digest_mismatch"
    );
}

#[test]
fn runtime_status_projection_is_consumed_and_harness_never_invents_terminal_truth() {
    let mut harness = ContinuationHarness::default();
    let submitted_proposal = proposal();
    let pending = harness
        .submit(submitted_proposal)
        .expect("submit proposal to Harness");
    let projection = harness
        .consume_runtime_status(
            pending,
            runtime_status("pending", Some("scheduler_backpressure")),
        )
        .expect("consume Runtime pending projection");
    let projection = serde_json::to_value(projection).expect("encode pending projection");
    assert_eq!(projection["status"], "pending");
    assert_eq!(projection["terminal_disposition"], "scheduler_backpressure");
    assert_eq!(projection["world_effect"], false);

    let mut committed = harness.submit(proposal()).expect("submit second proposal");
    let receipt = runtime_status("committed", None);
    committed = harness
        .consume_runtime_status(committed, receipt)
        .expect("consume committed Runtime projection");
    let committed = serde_json::to_value(committed).expect("encode committed projection");
    assert_eq!(committed["status"], "completed");
    assert_eq!(committed["provenance"], "runtime_authoritative");
    assert_eq!(committed["world_effect"], true);
}

#[test]
fn observation_goal_policy_and_terminal_runtime_changes_clear_continuation_deterministically() {
    for (reason, expected_status) in [
        (
            ContinuationInvalidationReason::ObservationChanged,
            "invalidated",
        ),
        (ContinuationInvalidationReason::GoalChanged, "invalidated"),
        (ContinuationInvalidationReason::PolicyChanged, "invalidated"),
        (ContinuationInvalidationReason::Rejected, "rejected"),
        (ContinuationInvalidationReason::Stale, "invalidated"),
        (ContinuationInvalidationReason::Timeout, "expired"),
        (ContinuationInvalidationReason::Expired, "expired"),
        (ContinuationInvalidationReason::Reorg, "invalidated"),
        (ContinuationInvalidationReason::Cancelled, "cancelled"),
    ] {
        let mut harness = ContinuationHarness::default();
        let active = harness.submit(proposal()).expect("submit continuation");
        let result = harness
            .invalidate(active, reason)
            .expect("invalidate continuation");
        let result = serde_json::to_value(result).expect("encode invalidation result");
        assert_eq!(result["status"], expected_status);
        assert_eq!(result["active"], false);
        assert_eq!(result["provider_invocation_count"], 0);
        assert_eq!(result["world_effect"], false);
    }
}
