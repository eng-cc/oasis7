//! LIVE-5 RED tests for the production Harness/Runtime handoff.
//!
//! The policy fixtures prove bounded normalization in isolation.  These tests
//! deliberately enter through the target actor runner and therefore remain
//! RED until Builtin and ProviderBacked actors share the host projection,
//! Runtime receipt gate, and Runtime-owned continuation status.  They do not
//! grant any authority to provider output and do not cover deferred
//! GoalGraph, shared memory, beliefs, preference correction, or billing.

use crate::simulator::{
    ActionCatalogEntry, AsyncAgentRunner, ContinuationProposalV1, ContinuousAgentTurnContextV1,
    DecisionResponse, GoalSnapshotInputV1, GoalSnapshotProjector, MemoryContextEntryV1,
    MemoryContextSnapshotV1, MemoryWriteIntent, MemoryWriteStore, MockDecisionProvider,
    ProviderBackedAgentBehavior, ProviderDecision, ProviderDiagnostics, ProviderExecutionMode,
    ProviderTraceEnvelope, RuntimeContinuationStatusV1, h_v1,
};
use serde_json::{Value, json};

const AGENT_ID: &str = "agent-live-harness";
const SESSION_ID: &str = "session-live-harness";
const TURN_ID: &str = "turn-live-harness";
const REQUEST_ID: &str = "request-live-harness";
const REQUEST_DIGEST: &str =
    "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn host_memory() -> MemoryContextSnapshotV1 {
    let mut snapshot = MemoryContextSnapshotV1 {
        revision: 7,
        entries: vec![MemoryContextEntryV1 {
            id: "memory-1".to_string(),
            summary: "the last committed receipt was receipt-previous".to_string(),
            tags: vec!["receipt".to_string()],
        }],
        scope: "session_private".to_string(),
        digest: String::new(),
    };
    snapshot.digest = snapshot.computed_digest();
    snapshot
}

fn host_goal() -> crate::simulator::GoalSnapshotV1 {
    GoalSnapshotProjector::project(
        Some(GoalSnapshotInputV1 {
            revision: 11,
            short_term_summary: "inspect the newly committed site".to_string(),
            long_term_summary: "establish a durable industrial route".to_string(),
            blocked_reason: None,
            provenance: "harness_projection".to_string(),
        }),
        None,
    )
    .expect("valid host goal projection")
}

fn host_context() -> ContinuousAgentTurnContextV1 {
    serde_json::from_value(json!({
        "agent_id": AGENT_ID,
        "agent_session_id": SESSION_ID,
        "agent_turn_id": TURN_ID,
        "decision_request_id": REQUEST_ID,
        "request_digest": REQUEST_DIGEST,
        "memory_snapshot": host_memory(),
        "goal_snapshot": host_goal(),
        "continuation": null
    }))
    .expect("decode host cognition context")
}

fn wait_response_with_memory_intent() -> DecisionResponse {
    DecisionResponse {
        decision: ProviderDecision::Wait,
        module_command: None,
        provider_error: None,
        diagnostics: ProviderDiagnostics {
            provider_id: Some("live-harness-provider".to_string()),
            ..ProviderDiagnostics::default()
        },
        trace_payload: ProviderTraceEnvelope {
            provider_id: Some("live-harness-provider".to_string()),
            output_summary: Some("decision=wait".to_string()),
            ..ProviderTraceEnvelope::default()
        },
        memory_write_intents: vec![MemoryWriteIntent {
            scope: "session_private".to_string(),
            summary: "candidate observation awaiting Runtime receipt".to_string(),
            tags: vec!["candidate".to_string()],
        }],
    }
}

fn provider_backed_runner() -> AsyncAgentRunner {
    let provider = MockDecisionProvider::with_scripted_responses(
        "live-harness-provider",
        vec![Ok(wait_response_with_memory_intent())],
    );
    let behavior = ProviderBackedAgentBehavior::new(
        AGENT_ID,
        provider,
        vec![ActionCatalogEntry::new("wait", "wait without world effect")],
    )
    .with_execution_mode(ProviderExecutionMode::HeadlessAgent);
    let mut runner = AsyncAgentRunner::new(16).expect("create target actor runner");
    runner
        .register(behavior)
        .expect("register provider-backed actor");
    runner
}

fn completed_turn(runner: &mut AsyncAgentRunner) -> crate::simulator::AsyncAgentTurnOutcome {
    for _ in 0..1024 {
        if let Some(outcome) = runner
            .poll_completed()
            .expect("poll target actor")
            .into_iter()
            .next()
        {
            return outcome;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    panic!("target actor did not complete within bounded polling budget");
}

#[test]
fn builtin_and_provider_backed_target_actors_consume_host_goal_not_memory_as_goal() {
    let context = host_context();
    let expected_goal = context.goal_snapshot.clone();
    let expected_memory = context.memory_snapshot.clone();

    for mut runner in [
        AsyncAgentRunner::builtin_fixture(AGENT_ID),
        provider_backed_runner(),
    ] {
        let _turn_id = runner
            .start_turn_with_context(AGENT_ID, context.clone())
            .expect("open target cognition turn");
        let outcome = completed_turn(&mut runner);
        let prepared = outcome
            .prepared_context
            .expect("target actor must retain prepared host context");

        assert_eq!(prepared.goal_snapshot, expected_goal);
        assert_eq!(prepared.memory_snapshot, expected_memory);
        assert_ne!(
            prepared.goal_snapshot.short_term_summary, prepared.memory_snapshot.entries[0].summary,
            "memory retrieval must never be projected as mission goal"
        );
        assert_eq!(
            outcome.world_effect,
            crate::simulator::AsyncWorldEffect::NoEffect
        );
    }
}

#[test]
fn target_actor_memory_intents_require_matching_committed_runtime_receipt_exactly_once() {
    let mut store = MemoryWriteStore::default();
    for status in ["pending", "rejected", "failed"] {
        assert_non_committed_status_does_not_write(status);
    }

    let mut runner = provider_backed_runner();
    let turn_id = runner
        .start_turn_with_context(AGENT_ID, host_context())
        .expect("open provider-backed target turn");
    let outcome = completed_turn(&mut runner);
    assert_eq!(outcome.turn_id, turn_id);
    assert_eq!(outcome.memory_write_intents.len(), 1);

    let committed = outcome
        .feedback_for_runtime_status("committed", Some("receipt-live-harness-1"))
        .expect("build correlated committed feedback");
    runner
        .consume_runtime_feedback(AGENT_ID, committed.clone(), &mut store)
        .expect("matching Runtime committed receipt writes memory");
    runner
        .consume_runtime_feedback(AGENT_ID, committed, &mut store)
        .expect("same receipt replay is exactly-once");
    assert_eq!(store.entries().len(), 1);
    assert_eq!(
        store.entries()[0]["provenance"],
        "runtime_authoritative",
        "provider must not self-assign authoritative provenance"
    );
}

fn assert_non_committed_status_does_not_write(status: &str) {
    let mut runner = provider_backed_runner();
    let _turn_id = runner
        .start_turn_with_context(AGENT_ID, host_context())
        .expect("open provider-backed target turn");
    let outcome = completed_turn(&mut runner);
    assert_eq!(outcome.memory_write_intents.len(), 1);

    let mut store = MemoryWriteStore::default();
    let feedback = outcome
        .feedback_for_runtime_status(status, None)
        .expect("build correlated non-committed feedback");
    runner
        .consume_runtime_feedback(AGENT_ID, feedback, &mut store)
        .expect("non-committed feedback is a no-write outcome");
    assert!(
        store.entries().is_empty(),
        "status={status} must not write memory"
    );
}

#[test]
fn target_actor_continuation_is_runtime_owned_and_rejected_continuation_cannot_reenter() {
    let proposal = continuation_proposal();
    let mut runner = AsyncAgentRunner::builtin_fixture(AGENT_ID);

    let accepted = runner
        .submit_continuation_proposal(AGENT_ID, proposal.clone())
        .expect("submit Harness continuation proposal");
    assert_eq!(accepted.status, "pending");
    assert_ne!(
        accepted.continuation_status_digest, proposal.proposal_digest,
        "Runtime must own status digest instead of echoing Harness proposal digest"
    );

    let rejected = runner
        .apply_runtime_continuation_status(
            AGENT_ID,
            RuntimeContinuationStatusV1 {
                status: "rejected".to_string(),
                terminal_disposition: Some("runtime_denied".to_string()),
                continuation_id: accepted.continuation_id,
                wake_id: accepted.wake_id,
                wake_seq: accepted.wake_seq,
                continuation_digest: accepted.continuation_digest,
            },
        )
        .expect("Runtime rejection invalidates continuation");
    assert!(!rejected.active);
    assert_eq!(rejected.status, "rejected");
    assert_eq!(rejected.provenance, "runtime_authoritative");
    assert_eq!(rejected.provider_invocation_count, 0);
    assert!(
        runner
            .start_turn_with_context(AGENT_ID, host_context())
            .is_ok(),
        "a rejected continuation must release the actor single-flight"
    );
}

fn continuation_proposal() -> ContinuationProposalV1 {
    let mut value: Value = json!({
        "schema_version": 1,
        "continuation_proposal_id": "proposal-live-harness-1",
        "world_id": "world-live-harness",
        "agent_id": AGENT_ID,
        "agent_session_id": SESSION_ID,
        "agent_turn_id": TURN_ID,
        "decision_request_id": REQUEST_ID,
        "origin_turn_id": TURN_ID,
        "origin_request_digest": REQUEST_DIGEST,
        "action_or_plan_kind": "wait_for_receipt",
        "action_or_envelope_digest": null,
        "remaining_budget": {"unit": "steps", "value": 2},
        "baseline_observation_digest": "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "goal_digest": host_goal().digest,
        "policy_digest": "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        "policy_revision": 1,
        "precondition_summary": "receipt pending",
        "precondition_digest": "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
        "wake_conditions": [{
            "schema_version": "wake-condition.v1",
            "kind": "receipt_linked",
            "receipt_id": "receipt-live-harness-1"
        }],
        "valid_until_tick": 100,
        "source": "harness",
        "proposal_digest": null
    });
    let mut digest_input = value.clone();
    digest_input
        .as_object_mut()
        .expect("continuation proposal object")
        .remove("proposal_digest");
    value["proposal_digest"] = json!(h_v1(
        "oasis7.cognition.continuation-proposal.v1",
        &digest_input
    ));
    serde_json::from_value(value).expect("decode continuation proposal")
}
