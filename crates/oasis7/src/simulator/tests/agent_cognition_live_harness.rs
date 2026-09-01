//! LIVE-5 RED tests for the production Harness/Runtime handoff.
//!
//! The policy fixtures prove bounded normalization in isolation.  These tests
//! deliberately enter through the target actor runner and therefore remain
//! RED until Builtin and ProviderBacked actors share the host projection,
//! Runtime receipt gate, and Runtime-owned continuation status.  They do not
//! grant any authority to provider output and do not cover deferred
//! GoalGraph, shared memory, beliefs, preference correction, or billing.

use crate::runtime::{
    AgentContinuation, ContinuationBudgetV1, ContinuationStatusV1, RuntimeReceiptLineageV1,
    WakeConditionV1,
};
use crate::simulator::{
    ActionCatalogEntry, AsyncAgentRunner, AsyncTurnFeedback, AsyncTurnLifecycle, AsyncWorldEffect,
    ContinuationProposalV1, ContinuousAgentTurnContextV1, DecisionProviderError, DecisionResponse,
    FeedbackEnvelopeV1, GoalSnapshotInputV1, GoalSnapshotProjector, MemoryContextEntryV1,
    MemoryContextSnapshotV1, MemoryWriteIntent, MemoryWriteStore, MockDecisionProvider,
    ProviderBackedAgentBehavior, ProviderDecision, ProviderDiagnostics, ProviderExecutionMode,
    ProviderTraceEnvelope, ProviderTranscriptEntry, h_v1,
};
use serde_json::{Value, json};

const AGENT_ID: &str = "agent-live-harness";
const SESSION_ID: &str = "session-live-harness";
const TURN_ID: &str = "turn-live-harness";
const REQUEST_ID: &str = "request-live-harness";
const REQUEST_DIGEST: &str =
    "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const MAX_PROVIDER_TRANSCRIPT_ENTRIES: usize = 64;
const MAX_PROVIDER_TOOL_TRACE_ENTRIES: usize = 64;
const MAX_PROVIDER_TRANSCRIPT_ENTRY_BYTES: usize = 2 * 1024;
const MAX_PROVIDER_TOOL_TRACE_ENTRY_BYTES: usize = 1024;
const MAX_PROVIDER_SUMMARY_BYTES: usize = 1024;
const MAX_PROVIDER_TRACE_BYTES: usize = 32 * 1024;

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
    provider_backed_runner_with_state().0
}

fn provider_backed_runner_with_response(response: DecisionResponse) -> AsyncAgentRunner {
    let provider =
        MockDecisionProvider::with_scripted_responses("live-harness-provider", vec![Ok(response)]);
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

fn provider_backed_runner_with_state() -> (
    AsyncAgentRunner,
    std::sync::Arc<std::sync::Mutex<crate::simulator::MockDecisionProviderState>>,
) {
    let provider = MockDecisionProvider::with_scripted_responses(
        "live-harness-provider",
        vec![Ok(wait_response_with_memory_intent())],
    );
    let shared_state = provider.shared_state();
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
    (runner, shared_state)
}

fn provider_failure_runner() -> AsyncAgentRunner {
    let provider = MockDecisionProvider::with_scripted_responses(
        "live-harness-provider",
        vec![Err(DecisionProviderError::new(
            "provider_timeout",
            "provider exceeded the turn budget",
            true,
        ))],
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

fn assert_provider_record_has<T: serde::Serialize>(recorded_wire: &str, field: &str, expected: &T) {
    let recorded: Value = serde_json::from_str(recorded_wire).expect("decode provider recording");
    let expected = serde_json::to_value(expected).expect("serialize expected projection");
    fn contains_field(value: &Value, field: &str, expected: &Value) -> bool {
        match value {
            Value::Object(object) => {
                object.get(field).is_some_and(|value| value == expected)
                    || object
                        .values()
                        .any(|value| contains_field(value, field, expected))
            }
            Value::Array(values) => values
                .iter()
                .any(|value| contains_field(value, field, expected)),
            _ => false,
        }
    }
    assert!(
        contains_field(&recorded, field, &expected),
        "provider recording must contain frozen {field} field; expected={expected}, recorded={recorded_wire}"
    );
}

#[test]
fn provider_boundary_preserves_frozen_context_and_response_lineage() {
    let (mut runner, shared_state) = provider_backed_runner_with_state();
    let mut context = host_context();
    context.continuation = Some(continuation_proposal());

    runner
        .start_turn_with_context(AGENT_ID, context.clone())
        .expect("open provider-backed target turn");
    let outcome = completed_turn(&mut runner);

    let state = shared_state.lock().expect("mock state lock").clone();
    assert_eq!(state.recorded_requests.len(), 1);
    let recorded_wire =
        serde_json::to_string(&state).expect("serialize MockDecisionProvider recording");
    for (field, expected) in [
        (
            "memory_snapshot",
            serde_json::to_value(&context.memory_snapshot).expect("serialize memory snapshot"),
        ),
        (
            "continuation",
            serde_json::to_value(&context.continuation).expect("serialize continuation"),
        ),
        (
            "goal_snapshot",
            serde_json::to_value(&context.goal_snapshot).expect("serialize goal snapshot"),
        ),
    ] {
        assert_provider_record_has(&recorded_wire, field, &expected);
    }
    for (field, expected) in [
        ("agent_session_id", SESSION_ID),
        ("agent_turn_id", TURN_ID),
        ("decision_request_id", REQUEST_ID),
        ("request_digest", REQUEST_DIGEST),
    ] {
        assert_provider_record_has(&recorded_wire, field, &expected);
    }

    let response_wire = outcome
        .decision_trace
        .and_then(|trace| trace.llm_output)
        .expect("provider response trace");
    let response: Value = serde_json::from_str(&response_wire).expect("decode provider response");
    assert_eq!(
        response.get("context_discriminator"),
        Some(&json!("oasis7.continuous-agent-context")),
        "provider response must carry the versioned cognition envelope"
    );
    assert_eq!(response.get("context_version"), Some(&json!(1)));
    for (field, expected) in [
        ("agent_session_id", SESSION_ID),
        ("agent_turn_id", TURN_ID),
        ("decision_request_id", REQUEST_ID),
        ("request_digest", REQUEST_DIGEST),
    ] {
        assert_eq!(
            response.get(field),
            Some(&json!(expected)),
            "provider response lineage field {field} must echo the request"
        );
    }
    assert!(
        response.get("base_decision_response").is_some(),
        "provider response must retain the inner DecisionResponse under the outer lineage"
    );
}

#[test]
fn provider_trace_is_bounded_redacted_and_non_authoritative() {
    let sensitive_summary = json!({
        "credential": "credential-secret",
        "token": "token-secret",
        "authorization": "authorization-secret",
        "private_key": "private-key-secret",
        "path": "/private/path/secret",
        "cookie": "cookie-secret",
        "access_key": "access-key-secret",
        "refreshKey": "refresh-key-secret",
        "session_key": "session-key-secret"
    })
    .to_string();
    let mut redaction_response = wait_response_with_memory_intent();
    redaction_response.decision = ProviderDecision::WaitTicks { ticks: 3 };
    redaction_response.trace_payload = ProviderTraceEnvelope {
        provider_id: Some("live-harness-provider".to_string()),
        input_summary: Some(sensitive_summary.clone()),
        output_summary: Some(sensitive_summary.clone()),
        transcript: vec![ProviderTranscriptEntry {
            role: "tool".to_string(),
            content: sensitive_summary.clone(),
        }],
        tool_trace: vec![sensitive_summary.clone()],
        upstream_trace: Some(json!({
            "credential": "credential-secret",
            "token": "token-secret",
            "authorization": "authorization-secret",
            "private_key": "private-key-secret",
            "path": "/private/path/secret",
            "cookie": "cookie-secret",
            "access_key": "access-key-secret",
            "refreshKey": "refresh-key-secret",
            "session_key": "session-key-secret"
        })),
        ..ProviderTraceEnvelope::default()
    };

    let mut redaction_runner = provider_backed_runner_with_response(redaction_response);
    redaction_runner
        .start_turn_with_context(AGENT_ID, host_context())
        .expect("open redaction fixture turn");
    let redaction_outcome = completed_turn(&mut redaction_runner);
    assert_eq!(
        redaction_outcome.decision,
        Some(crate::simulator::AgentDecision::WaitTicks(3)),
        "non-authority redaction diagnostics must not change the candidate decision"
    );
    let redacted_trace = redaction_outcome
        .decision_trace
        .expect("redaction fixture retains a decision trace");
    let redacted_wire = serde_json::to_string(&redacted_trace).expect("serialize redacted trace");
    assert!(
        redacted_wire.contains("<redacted>"),
        "sensitive provider trace fields must use the fixed redaction marker"
    );
    for secret in [
        "credential-secret",
        "token-secret",
        "authorization-secret",
        "private-key-secret",
        "/private/path/secret",
        "cookie-secret",
        "access-key-secret",
        "refresh-key-secret",
        "session-key-secret",
    ] {
        assert!(
            !redacted_wire.contains(secret),
            "provider trace must not retain sensitive value {secret:?}"
        );
    }

    let mut overflow_response = wait_response_with_memory_intent();
    overflow_response.decision = ProviderDecision::WaitTicks { ticks: 7 };
    overflow_response.trace_payload.input_summary =
        Some("i".repeat(MAX_PROVIDER_SUMMARY_BYTES + 1));
    overflow_response.trace_payload.output_summary =
        Some("o".repeat(MAX_PROVIDER_SUMMARY_BYTES + 1));
    overflow_response.trace_payload.transcript = (0..=MAX_PROVIDER_TRANSCRIPT_ENTRIES)
        .map(|index| ProviderTranscriptEntry {
            role: "agent".to_string(),
            content: format!(
                "transcript-{index}-{}",
                "t".repeat(MAX_PROVIDER_TRANSCRIPT_ENTRY_BYTES)
            ),
        })
        .collect();
    overflow_response.trace_payload.tool_trace = (0..=MAX_PROVIDER_TOOL_TRACE_ENTRIES)
        .map(|index| {
            format!(
                "tool-{index}-{}",
                "u".repeat(MAX_PROVIDER_TOOL_TRACE_ENTRY_BYTES)
            )
        })
        .collect();
    overflow_response.trace_payload.upstream_trace = Some(json!({
        "trace": "x".repeat(MAX_PROVIDER_TRACE_BYTES)
    }));

    let mut overflow_runner = provider_backed_runner_with_response(overflow_response);
    overflow_runner
        .start_turn_with_context(AGENT_ID, host_context())
        .expect("open overflow fixture turn");
    let overflow_outcome = completed_turn(&mut overflow_runner);
    assert_eq!(
        overflow_outcome.decision,
        Some(crate::simulator::AgentDecision::WaitTicks(7)),
        "non-authority trace overflow must not change the candidate decision"
    );
    let overflow_trace = overflow_outcome
        .decision_trace
        .expect("overflow fixture retains a bounded decision trace");
    assert!(overflow_trace.llm_chat_messages.len() <= MAX_PROVIDER_TRANSCRIPT_ENTRIES);
    assert!(overflow_trace.llm_step_trace.len() <= MAX_PROVIDER_TOOL_TRACE_ENTRIES);
    assert!(
        overflow_trace
            .llm_chat_messages
            .iter()
            .all(|entry| entry.content.len() <= MAX_PROVIDER_TRANSCRIPT_ENTRY_BYTES)
    );
    assert!(overflow_trace.llm_step_trace.iter().all(|entry| {
        entry.input_summary.len() <= MAX_PROVIDER_TOOL_TRACE_ENTRY_BYTES
            && entry.output_summary.len() <= MAX_PROVIDER_TOOL_TRACE_ENTRY_BYTES
    }));
    if let Some(input) = overflow_trace.llm_input.as_ref() {
        assert!(input.len() <= MAX_PROVIDER_SUMMARY_BYTES);
    }
    if let Some(output) = overflow_trace.llm_output.as_ref() {
        assert!(output.len() <= MAX_PROVIDER_SUMMARY_BYTES);
    }
    assert!(
        serde_json::to_vec(&overflow_trace)
            .expect("serialize bounded trace")
            .len()
            <= MAX_PROVIDER_TRACE_BYTES,
        "retained provider trace must stay within the canonical aggregate bound"
    );
    assert!(
        overflow_trace
            .llm_error
            .as_deref()
            .unwrap_or_default()
            .contains("trace_payload_too_large"),
        "overflow must retain the stable trace_payload_too_large diagnostic"
    );
}

#[test]
fn upstream_trace_over_four_kib_retains_overflow_diagnostic() {
    let mut response = wait_response_with_memory_intent();
    response.trace_payload.upstream_trace = Some(json!({
        "provider_payload": "x".repeat(8 * 1024)
    }));
    let mut runner = provider_backed_runner_with_response(response);
    runner
        .start_turn_with_context(AGENT_ID, host_context())
        .expect("open upstream overflow fixture turn");
    let outcome = completed_turn(&mut runner);
    let trace = outcome
        .decision_trace
        .expect("overflow fixture retains trace");
    assert!(
        trace
            .llm_error
            .as_deref()
            .is_some_and(|error| error.contains("trace_payload_too_large"))
    );
}

#[test]
fn provider_failure_is_not_reported_as_a_successful_wait() {
    let mut runner = provider_failure_runner();
    runner
        .start_turn_with_context(AGENT_ID, host_context())
        .expect("open provider-backed target turn");

    let outcome = completed_turn(&mut runner);

    assert_eq!(outcome.lifecycle, AsyncTurnLifecycle::Failed);
    assert_eq!(
        outcome.feedback,
        AsyncTurnFeedback::ProviderError {
            code: "provider_timeout".to_string()
        }
    );
    assert_eq!(outcome.world_effect, AsyncWorldEffect::NoEffect);
    assert!(outcome.decision.is_none());
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

    let mut committed = outcome
        .feedback_for_runtime_status("committed", Some("receipt-live-harness-1"))
        .expect("build correlated committed feedback");
    committed.candidate_action_id = Some(7);
    committed.provenance = "runtime_authoritative".to_string();
    let receipt = runtime_receipt_for_feedback(&committed);
    runner
        .consume_runtime_feedback_with_lineage(
            AGENT_ID,
            committed.clone(),
            Some(&receipt),
            &mut store,
        )
        .expect("matching Runtime committed receipt writes memory");
    runner
        .consume_runtime_feedback_with_lineage(AGENT_ID, committed, Some(&receipt), &mut store)
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
    let mut feedback = outcome
        .feedback_for_runtime_status(status, None)
        .expect("build correlated non-committed feedback");
    feedback.provenance = "runtime_authoritative".to_string();
    let context = outcome
        .prepared_context
        .as_ref()
        .expect("outcome retains host context");
    assert_eq!(feedback.agent_turn_id, context.agent_turn_id);
    assert_eq!(feedback.decision_request_id, context.decision_request_id);
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
    assert_eq!(accepted.status, "scheduled");
    assert!(accepted.continuation_id.is_empty());
    assert!(accepted.wake_id.is_empty());
    assert_eq!(accepted.wake_seq, 0);
    assert!(accepted.continuation_digest.is_empty());
    assert!(accepted.continuation_status_digest.is_empty());
    assert_eq!(accepted.provenance, "harness_policy");
    assert!(!accepted.world_effect);
    assert_ne!(
        accepted.continuation_status_digest, proposal.proposal_digest,
        "Runtime must own status digest instead of echoing Harness proposal digest"
    );

    let rejected = runner
        .apply_runtime_continuation_projection(
            AGENT_ID,
            rejected_runtime_projection(&accepted.proposal),
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

fn runtime_receipt_for_feedback(feedback: &FeedbackEnvelopeV1) -> RuntimeReceiptLineageV1 {
    RuntimeReceiptLineageV1 {
        schema_version: RuntimeReceiptLineageV1::SCHEMA_VERSION.to_string(),
        status: "committed".to_string(),
        receipt_id: feedback
            .runtime_receipt_id
            .clone()
            .expect("committed feedback carries Runtime receipt ID"),
        receipt_digest: "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"
            .to_string(),
        envelope_digest: "blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
            .to_string(),
        action_id: feedback
            .candidate_action_id
            .expect("committed feedback carries Runtime action ID")
            .to_string(),
        agent_id: feedback.agent_subject.clone(),
        agent_session_id: feedback.agent_session_id.clone(),
        agent_turn_id: feedback.agent_turn_id.clone(),
        decision_request_id: feedback.decision_request_id.clone(),
        request_digest: feedback.request_digest.to_string(),
        feedback_id: feedback.feedback_id.clone(),
    }
}

fn rejected_runtime_projection(proposal: &ContinuationProposalV1) -> AgentContinuation {
    let wake_conditions: Vec<WakeConditionV1> = serde_json::from_value(
        serde_json::to_value(&proposal.wake_conditions).expect("encode proposal wake conditions"),
    )
    .expect("decode Runtime wake conditions");
    let mut runtime = AgentContinuation {
        schema_version: "agent-continuation.v1".to_string(),
        continuation_id: "continuation-runtime-live-harness-1".to_string(),
        wake_id: "wake-runtime-live-harness-1".to_string(),
        world_id: proposal.world_id.clone(),
        branch_id: "branch-live-harness".to_string(),
        finality_epoch: 1,
        finality_block_hash: Some(
            "blake3:1111111111111111111111111111111111111111111111111111111111111111".to_string(),
        ),
        finality_status: "verified".to_string(),
        reorg_epoch: 0,
        runtime_manifest_hash:
            "blake3:2222222222222222222222222222222222222222222222222222222222222222".to_string(),
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
        status: ContinuationStatusV1::Rejected,
        continuation_status_digest: None,
        terminal_disposition: Some("runtime_denied".to_string()),
    };
    runtime.continuation_status_digest = Some(runtime.status_digest());
    runtime
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
