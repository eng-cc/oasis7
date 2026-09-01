//! Harness-policy RED fixtures for ContinuationProposalV1.
//!
//! Harness owns the bounded proposal and policy fields; Runtime owns the
//! durable schedule/status projection and finality truth.  These fixtures
//! intentionally do not cover GoalGraph, belief/preference state or billing.

use crate::runtime::{AgentContinuation, ContinuationBudgetV1, ContinuationStatusV1};
use crate::simulator::{
    AsyncAgentRunner, ContinuationHarness, ContinuationInvalidationReason, ContinuationProposalV1,
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

fn runtime_projection(
    proposal: &ContinuationProposalV1,
    status: ContinuationStatusV1,
    terminal_disposition: Option<&str>,
) -> AgentContinuation {
    let wake_conditions = serde_json::from_value(
        serde_json::to_value(&proposal.wake_conditions).expect("encode proposal wake conditions"),
    )
    .expect("decode Runtime wake conditions");
    let mut runtime = AgentContinuation {
        schema_version: "agent-continuation.v1".to_string(),
        continuation_id: "continuation-runtime-1".to_string(),
        wake_id: "wake-runtime-1".to_string(),
        world_id: proposal.world_id.clone(),
        branch_id: "branch-continuation-fixture".to_string(),
        finality_epoch: 7,
        finality_block_hash: Some(
            "blake3:7777777777777777777777777777777777777777777777777777777777777777".to_string(),
        ),
        finality_status: "verified".to_string(),
        reorg_epoch: 1,
        runtime_manifest_hash:
            "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
        agent_id: proposal.agent_id.clone(),
        agent_session_id: proposal.agent_session_id.clone(),
        agent_turn_id: proposal.agent_turn_id.clone(),
        decision_request_id: proposal.decision_request_id.clone(),
        origin_turn_id: proposal.origin_turn_id.clone(),
        origin_request_digest: proposal.origin_request_digest.clone(),
        continuation_proposal_id: proposal.continuation_proposal_id.clone(),
        proposal_digest: proposal.proposal_digest.clone(),
        action_or_envelope_digest: proposal.action_or_envelope_digest.clone(),
        wake_conditions,
        next_wake_tick: Some(10),
        remaining_budget: ContinuationBudgetV1 {
            unit: proposal.remaining_budget.unit.clone(),
            value: proposal.remaining_budget.value,
        },
        valid_until_tick: proposal.valid_until_tick,
        precondition_digest: proposal.precondition_digest.clone(),
        wake_seq: 1,
        logical_tick: 10,
        status,
        continuation_status_digest: None,
        terminal_disposition: terminal_disposition.map(str::to_string),
    };
    runtime.continuation_status_digest = Some(runtime.status_digest());
    runtime
        .validate_authoritative()
        .expect("Runtime continuation projection must be authoritative");
    runtime
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
        .consume_runtime_projection(
            pending,
            &runtime_projection(&proposal(), ContinuationStatusV1::Pending, None),
        )
        .expect("consume Runtime pending projection");
    let projection = serde_json::to_value(projection).expect("encode pending projection");
    assert_eq!(projection["status"], "pending");
    assert_eq!(projection["terminal_disposition"], Value::Null);
    assert_eq!(projection["world_effect"], false);

    let mut committed = harness.submit(proposal()).expect("submit second proposal");
    let receipt = runtime_projection(
        &committed.proposal,
        ContinuationStatusV1::Completed,
        Some("completed"),
    );
    committed = harness
        .consume_runtime_projection(committed, &receipt)
        .expect("consume committed Runtime projection");
    let committed = serde_json::to_value(committed).expect("encode committed projection");
    assert_eq!(committed["status"], "completed");
    assert_eq!(committed["provenance"], "runtime_authoritative");
    assert_eq!(committed["terminal_disposition"], "completed");
    assert_eq!(committed["world_effect"], false);
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

#[test]
fn continuation_submission_is_proposal_only_and_rejects_uncorrelated_runtime_status() {
    let submitted_proposal = proposal();
    let mut runner = AsyncAgentRunner::builtin_fixture(AGENT_ID);
    let handle = runner
        .submit_continuation_proposal(AGENT_ID, submitted_proposal)
        .expect("submit Harness continuation proposal");

    let proposal_only = handle.continuation_id.is_empty()
        && handle.wake_id.is_empty()
        && handle.wake_seq == 0
        && handle.continuation_digest.is_empty()
        && handle.continuation_status_digest.is_empty()
        && handle.status == "scheduled"
        && handle.provenance == "harness_policy"
        && !handle.world_effect;

    let mut harness = ContinuationHarness::default();
    let submitted = harness
        .submit(proposal())
        .expect("submit proposal to Harness");
    let unrelated_status = RuntimeContinuationStatusV1 {
        status: "pending".to_string(),
        terminal_disposition: None,
        continuation_id: "runtime-unrelated-continuation".to_string(),
        wake_id: "runtime-unrelated-wake".to_string(),
        wake_seq: 99,
        continuation_digest:
            "blake3:9999999999999999999999999999999999999999999999999999999999999999".to_string(),
    };
    let uncorrelated_status_rejected = harness
        .consume_runtime_status(submitted, unrelated_status)
        .is_err();
    assert!(
        proposal_only && uncorrelated_status_rejected,
        "Harness must remain proposal-only and reject uncorrelated Runtime status: proposal_only={proposal_only}, uncorrelated_status_rejected={uncorrelated_status_rejected}, handle={handle:?}"
    );
}
