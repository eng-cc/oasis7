//! P0.1 RED fixtures for the continuous-agent identity and wire contract.
//!
//! These tests intentionally target the neutral cognition API described by
//! issue #3602.  The API is not present in the legacy provider adapter yet;
//! this file is the immutable behavior contract for the implementation slice.

use super::*;
use crate::simulator::{
    AgentCognitionStore, ContinuousAgentRequestContextV1, ContinuousAgentResponseContextV1,
    Digest32, FeedbackEnvelopeV1, FinalityBindingV1, MemoryWriteIntentV1, RuntimeBindingV1, h_v1,
};
use oasis7_wasm_abi::AgentCommandResponse;
use serde_json::{Value, json};

const REQUEST_DOMAIN: &str = "oasis7.cognition.request.v1";
const PROVIDER_INVOCATION_DOMAIN: &str = "oasis7.cognition.provider-invocation.v1";
const CONTEXT_DISCRIMINATOR: &str = "oasis7.continuous-agent-context";

fn inner_request() -> Value {
    serde_json::to_value(
        &golden_decision_provider_fixtures()
            .into_iter()
            .next()
            .expect("golden provider fixture")
            .request,
    )
    .expect("serialize inner decision request")
}

fn request_fixture(transport_attempt: u64, timeout_budget_ms: u64) -> Value {
    let mut base_decision_request = inner_request();
    base_decision_request["timeout_budget_ms"] = json!(timeout_budget_ms);
    json!({
        "base_decision_request": base_decision_request,
        "context_discriminator": CONTEXT_DISCRIMINATOR,
        "context_version": 1,
        "protocol_version": "continuous-agent-v1",
        "agent_session_id": "session.agent-1.v1",
        "agent_turn_id": "turn.agent-1.7",
        "decision_request_id": "request.agent-1.7",
        "retry_seq": 0,
        "transport_attempt": transport_attempt,
        "agent_subject": "agent-1",
        "runtime_binding": {
            "world_id": "world-1",
            "branch_id": "main",
            "finality_epoch": 7,
            "finality_block_hash": "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "finality_status": "verified",
            "base_tick": 42,
            "base_world_hash": "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "reorg_epoch": 3,
            "runtime_manifest_hash": "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
        },
        "observation_digest": "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
        "capability_catalog_digest": "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        "capability_invocation_context_digest": "blake3:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        "memory_snapshot_digest": "blake3:1111111111111111111111111111111111111111111111111111111111111111",
        "goal_snapshot_digest": "blake3:2222222222222222222222222222222222222222222222222222222222222222",
        "continuation_digest": "blake3:3333333333333333333333333333333333333333333333333333333333333333",
        "adapter_protocol_version": "loopback-http-v1",
        "budget_contract": {"max_latency_ms": 60_000, "max_repair_attempts": 2},
        "request_digest": "blake3:4444444444444444444444444444444444444444444444444444444444444444"
    })
}

fn request(transport_attempt: u64, timeout_budget_ms: u64) -> ContinuousAgentRequestContextV1 {
    serde_json::from_value(request_fixture(transport_attempt, timeout_budget_ms))
        .expect("decode ContinuousAgentRequestContextV1 fixture")
}

fn response_fixture(decision: Value) -> Value {
    json!({
        "base_decision_response": {
            "decision": decision,
            "provider_error": null,
            "diagnostics": {},
            "trace_payload": {},
            "memory_write_intents": []
        },
        "context_discriminator": CONTEXT_DISCRIMINATOR,
        "context_version": 1,
        "agent_session_id": "session.agent-1.v1",
        "agent_turn_id": "turn.agent-1.7",
        "decision_request_id": "request.agent-1.7",
        "retry_seq": 0,
        "transport_attempt": 0,
        "request_digest": "blake3:4444444444444444444444444444444444444444444444444444444444444444"
    })
}

#[test]
fn request_digest_golden_excludes_request_digest_and_transport_attempt() {
    let first = request(0, 60_000);
    let retried = request(1, 60_000);

    assert_eq!(
        first
            .canonical_request_bytes()
            .expect("canonical request bytes"),
        retried
            .canonical_request_bytes()
            .expect("canonical retry bytes")
    );
    assert_eq!(
        first.request_digest().expect("request digest"),
        retried.request_digest()
    );
    assert_eq!(
        first
            .provider_invocation_key()
            .expect("provider invocation key"),
        retried.provider_invocation_key()
    );
    assert_eq!(
        first
            .provider_invocation_key()
            .expect("provider invocation key"),
        h_v1(
            PROVIDER_INVOCATION_DOMAIN,
            &first.request_digest().expect("request digest")
        )
    );
    assert_eq!(
        h_v1(
            REQUEST_DOMAIN,
            &first.canonical_request_bytes().expect("canonical bytes")
        ),
        first.request_digest().expect("request digest")
    );
}

#[test]
fn provider_key_is_invariant_to_timeout_and_transport_attempt() {
    let baseline = request(0, 100);
    let timeout_changed = request(0, 5_000);
    let transport_retry = request(9, 100);

    assert_eq!(
        baseline.provider_invocation_key().expect("provider key"),
        timeout_changed
            .provider_invocation_key()
            .expect("provider key")
    );
    assert_eq!(
        baseline.provider_invocation_key().expect("provider key"),
        transport_retry
            .provider_invocation_key()
            .expect("provider key")
    );
}

#[test]
fn outer_request_roundtrips_and_unknown_version_fails_closed() {
    let fixture = request_fixture(0, 60_000);
    let context: ContinuousAgentRequestContextV1 =
        serde_json::from_value(fixture.clone()).expect("decode outer request");
    let encoded = serde_json::to_value(&context).expect("encode outer request");
    let decoded: ContinuousAgentRequestContextV1 =
        serde_json::from_value(encoded).expect("round-trip outer request");
    assert_eq!(context, decoded);

    let mut unknown_version = fixture;
    unknown_version["context_version"] = json!(99);
    let error = ContinuousAgentRequestContextV1::validate_value(&unknown_version)
        .expect_err("unknown context version must fail closed");
    assert_eq!(error.code(), "unsupported_context_version");
}

#[test]
fn outer_response_preserves_all_six_tagged_provider_decisions() {
    let fixture = golden_decision_provider_fixtures()
        .into_iter()
        .next()
        .expect("golden provider fixture");
    let query = ProviderDecision::Query {
        query_ref: "query-1".to_string(),
        query: AgentQuery::QuoteMicroDepotInstall(Action::MoveAgent {
            agent_id: "agent-1".to_string(),
            to: "loc-2".to_string(),
        }),
    };
    let module_command = ProviderDecision::ModuleCommand {
        module_command: ProviderModuleCommand {
            module_id: "m.weather".to_string(),
            module_version: "0.1.0".to_string(),
            namespace: "weather".to_string(),
            name: "observe".to_string(),
            schema_version: 1,
            schema_hash: "blake3:module".to_string(),
            payload: vec![1, 2, 3],
        },
    };
    let command_response: AgentCommandResponse = serde_json::from_value(json!({
        "response_nonce": "nonce-1",
        "subject": {"kind": "agent", "agent_id": "agent-1", "owner_binding": "owner-1", "generation": 1},
        "presenter": {"presenter_id": "provider-1", "presenter_kind": "provider"},
        "audience": {
            "world_id": "world-1", "branch_id": "main", "finality_epoch": 1,
            "target_kind": "module_instance", "target_id": "instance-1"
        },
        "catalog_snapshot_id": "catalog-1",
        "selected_entry": {
            "module_id": "m.weather", "module_version": "0.1.0", "namespace": "weather",
            "command": "observe", "schema_version": 1, "schema_hash": "0000000000000000000000000000000000000000000000000000000000000000",
            "max_payload_bytes": 128
        },
        "envelope": {
            "namespace": "weather", "name": "observe", "schema_version": 1,
            "schema_hash": "0000000000000000000000000000000000000000000000000000000000000000",
            "payload": [1, 2, 3]
        },
        "provider_id": "provider-1", "trace_id": "trace-1"
    }))
    .expect("decode typed module response");
    let decisions: Vec<Value> = vec![
        serde_json::to_value(ProviderDecision::Wait).expect("encode wait decision"),
        serde_json::to_value(ProviderDecision::WaitTicks { ticks: 3 })
            .expect("encode wait-ticks decision"),
        serde_json::to_value(fixture.expected_decision).expect("encode act decision"),
        serde_json::to_value(query).expect("encode query decision"),
        serde_json::to_value(module_command).expect("encode module command decision"),
        serde_json::to_value(ProviderDecision::ModuleCommandResponse {
            response: command_response,
        })
        .expect("encode module response decision"),
    ];

    for decision in decisions {
        let response: ContinuousAgentResponseContextV1 =
            serde_json::from_value(response_fixture(decision.clone()))
                .expect("decode outer response");
        let encoded = serde_json::to_value(response).expect("encode outer response");
        assert_eq!(encoded["base_decision_response"]["decision"], decision);
    }
}

#[test]
fn same_key_with_different_digest_is_a_fail_closed_collision() {
    let mut store = AgentCognitionStore::default();
    let first = request(0, 60_000);
    let mut collision = request_fixture(0, 60_000);
    collision["observation_digest"] =
        json!("blake3:9999999999999999999999999999999999999999999999999999999999999999");
    let second: ContinuousAgentRequestContextV1 =
        serde_json::from_value(collision).expect("decode collision fixture");

    store.begin_request(first).expect("first request accepted");
    let error = store
        .begin_request(second)
        .expect_err("same key/different digest must fail closed");
    assert_eq!(error.code(), "request_identity_collision");
}

#[test]
fn one_agent_cannot_hold_two_sessions_but_two_agents_are_partitioned() {
    let mut store = AgentCognitionStore::default();
    store
        .begin_request(request(0, 60_000))
        .expect("first session accepted");

    let mut second_session = request_fixture(0, 60_000);
    second_session["agent_session_id"] = json!("session.agent-1.v2");
    second_session["agent_turn_id"] = json!("turn.agent-1.8");
    second_session["decision_request_id"] = json!("request.agent-1.8");
    let second_session: ContinuousAgentRequestContextV1 =
        serde_json::from_value(second_session).expect("decode second session");
    assert_eq!(
        store
            .begin_request(second_session)
            .expect_err("global per-agent single-flight")
            .code(),
        "agent_busy"
    );

    let mut other_agent = request_fixture(0, 60_000);
    other_agent["agent_subject"] = json!("agent-2");
    other_agent["agent_session_id"] = json!("session.agent-2.v1");
    other_agent["agent_turn_id"] = json!("turn.agent-2.1");
    other_agent["decision_request_id"] = json!("request.agent-2.1");
    store
        .begin_request(serde_json::from_value(other_agent).expect("decode other-agent request"))
        .expect("different agents remain independent");
}

#[test]
fn feedback_partition_rejects_cross_agent_and_preserves_runtime_lineage() {
    let mut store = AgentCognitionStore::default();
    store
        .begin_request(request(0, 60_000))
        .expect("request accepted");
    let feedback: FeedbackEnvelopeV1 = serde_json::from_value(json!({
        "feedback_id": "feedback-1", "feedback_seq": 1,
        "agent_subject": "agent-2", "agent_session_id": "session.agent-2.v1",
        "agent_turn_id": "turn.agent-2.1", "decision_request_id": "request.agent-2.1",
        "candidate_action_id": null, "runtime_receipt_id": null,
        "status": "rejected", "request_digest": "blake3:cross-agent",
        "reject_reason": "no_effect", "provenance": "runtime_authoritative"
    }))
    .expect("decode feedback fixture");

    let diagnostic = store
        .accept_feedback(feedback)
        .expect_err("cross-agent feedback");
    assert_eq!(diagnostic.code(), "cross_agent_feedback");
}

#[test]
fn legacy_feedback_without_committed_receipt_is_ambiguous() {
    let legacy = json!({
        "action_id": 7,
        "success": true,
        "reject_reason": null,
        "emitted_events": [],
        "world_delta_summary": "legacy response"
    });
    let error = FeedbackEnvelopeV1::from_legacy_value(legacy)
        .expect_err("legacy success without committed receipt is ambiguous");
    assert_eq!(error.code(), "legacy_feedback_ambiguous");
}

#[test]
fn cognition_wire_exports_the_shared_v1_types() {
    fn assert_wire_types<
        D: serde::Serialize + serde::de::DeserializeOwned,
        F: serde::Serialize + serde::de::DeserializeOwned,
        M: serde::Serialize + serde::de::DeserializeOwned,
        R: serde::Serialize + serde::de::DeserializeOwned,
    >() {
    }

    assert_wire_types::<Digest32, FinalityBindingV1, MemoryWriteIntentV1, RuntimeBindingV1>();
}
