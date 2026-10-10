use std::collections::BTreeMap;

use oasis7_agent_api::{
    CognitionLeaseConsumptionViewV1, CognitionLeaseStatusV1, ContinuationBudgetV1,
    ContinuationProposalV1, ContinuousAgentRequestContextV1, DecisionResponse, Digest32,
    FeedbackEnvelopeV1, ObservationEnvelope, ProviderDecision, ProviderExecutionMode,
    ResponseArtifactIdentityV1, RuntimeReceiptLineageV1, WakeConditionV1,
};
use serde_json::{Value, json};

const HASH_A: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const HASH_B: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const HASH_C: &str = "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const HASH_D: &str = "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

fn legacy_request_fixture(transport_attempt: u64, timeout_budget_ms: u64) -> Value {
    json!({
        "base_decision_request": {
            "observation": {
                "agent_id":"agent-1",
                "world_time":42,
                "mode":"headless_agent",
                "observation_schema_version":"oc_dual_obs_v1",
                "action_schema_version":"oc_dual_act_v1",
                "observation": {
                    "self_state":{"location_ref":"", "pose_hint":"", "status_flags":[]},
                    "mission_context":{"goal_summary":""},
                    "nearby_entities":[],
                    "recent_events":[]
                },
                "recent_event_summary":[],
                "action_catalog":[],
                "timeout_budget_ms":timeout_budget_ms
            },
            "timeout_budget_ms":timeout_budget_ms
        },
        "context_discriminator":"oasis7.continuous-agent-context",
        "context_version":1,
        "protocol_version":"continuous-agent-v1",
        "agent_session_id":"session.agent-1.v1",
        "agent_turn_id":"turn.agent-1.7",
        "decision_request_id":"request.agent-1.7",
        "retry_seq":0,
        "transport_attempt":transport_attempt,
        "agent_subject":"agent-1",
        "runtime_binding":{
            "world_id":"world-1",
            "branch_id":"main",
            "finality_epoch":7,
            "finality_status":"pending",
            "base_tick":42,
            "base_world_hash":HASH_B,
            "reorg_epoch":3,
            "runtime_manifest_hash":HASH_C
        },
        "observation_digest":HASH_D,
        "capability_catalog_digest":HASH_A,
        "capability_invocation_context_digest":HASH_B,
        "memory_snapshot_digest":HASH_C,
        "goal_snapshot_digest":HASH_D,
        "continuation_digest":HASH_A,
        "adapter_protocol_version":"loopback-http-v1",
        "budget_contract":{
            "max_latency_ms":60000,
            "max_repair_attempts":2,
            "max_model_calls":4,
            "max_tool_calls":3
        },
        "request_digest":"blake3:4444444444444444444444444444444444444444444444444444444444444444"
    })
}

fn request_context(
    transport_attempt: u64,
    timeout_budget_ms: u64,
) -> (Value, ContinuousAgentRequestContextV1) {
    let fixture = legacy_request_fixture(transport_attempt, timeout_budget_ms);
    let context = serde_json::from_value(fixture.clone()).expect("legacy request fixture");
    (fixture, context)
}

fn legacy_h_v1<T: serde::Serialize>(domain: &str, payload: &T) -> Digest32 {
    let bytes = oasis7_wasm_abi::encode_canonical_cbor(&(domain, payload))
        .expect("legacy golden payload must have canonical CBOR");
    Digest32(format!("blake3:{}", blake3::hash(bytes.as_slice())))
}

fn continuation_fixture() -> Value {
    json!({
        "schema_version": 1,
        "continuation_proposal_id": "proposal-1",
        "world_id": "world-continuation-fixture",
        "agent_id": "agent-continuation-1",
        "agent_session_id": "session-continuation-1",
        "agent_turn_id": "turn-continuation-1",
        "decision_request_id": "request-continuation-1",
        "origin_turn_id": "turn-continuation-1",
        "origin_request_digest": "blake3:2222222222222222222222222222222222222222222222222222222222222222",
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
            "receipt_id": "blake3:9999999999999999999999999999999999999999999999999999999999999999"
        }],
        "valid_until_tick": 100,
        "source": "harness",
        "proposal_digest": ""
    })
}

#[test]
fn typed_provider_decision_keeps_the_legacy_tagged_shape() {
    let action = ProviderDecision::<BTreeMap<String, String>, Value>::Act {
        action_ref: "move_agent".to_string(),
        action: BTreeMap::from([("target".to_string(), "loc-2".to_string())]),
    };
    assert_eq!(
        serde_json::to_value(action).expect("serialize typed decision"),
        json!({"decision":"act", "action_ref":"move_agent", "action":{"target":"loc-2"}})
    );
    let response = DecisionResponse::<BTreeMap<String, String>, Value>::wait("provider-1");
    assert_eq!(
        serde_json::to_value(response).expect("serialize wait response"),
        json!({
            "decision": {"decision":"wait"},
            "diagnostics": {"provider_id":"provider-1", "retry_count":0},
            "trace_payload": {
                "provider_id":"provider-1",
                "output_summary":"decision=wait",
                "transcript":[],
                "tool_trace":[],
                "schema_repair_count":0
            },
            "memory_write_intents":[]
        })
    );
}

#[test]
fn observation_defaults_match_the_legacy_provider_wire_shape() {
    let fixture = json!({
        "agent_id":"agent-1",
        "world_time":42,
        "observation":{"self_state":{"location_ref":"", "pose_hint":"", "status_flags":[]}, "mission_context":{"goal_summary":""}},
        "timeout_budget_ms":3000
    });
    let decoded: ObservationEnvelope =
        serde_json::from_value(fixture.clone()).expect("legacy observation fixture");
    assert_eq!(decoded.mode, ProviderExecutionMode::HeadlessAgent);
    assert_eq!(decoded.observation_schema_version, "oc_dual_obs_v1");
    assert_eq!(decoded.action_schema_version, "oc_dual_act_v1");
    let mut expected = fixture;
    expected["mode"] = json!("headless_agent");
    expected["observation_schema_version"] = json!("oc_dual_obs_v1");
    expected["action_schema_version"] = json!("oc_dual_act_v1");
    expected["recent_event_summary"] = json!([]);
    expected["action_catalog"] = json!([]);
    expected["observation"]["nearby_entities"] = json!([]);
    expected["observation"]["recent_events"] = json!([]);
    assert_eq!(serde_json::to_value(decoded).expect("round trip"), expected);
}

#[test]
fn request_identity_ignores_only_legacy_transport_fields_and_preserves_null_binding() {
    let (first_wire, first) = request_context(1, 3_000);
    let (retry_wire, retried) = request_context(4, 9_000);
    assert_eq!(
        serde_json::to_value(&first).expect("new request serialization"),
        first_wire
    );
    assert_eq!(
        serde_json::to_value(&retried).expect("new retry serialization"),
        retry_wire
    );
    assert_eq!(
        first.request_digest(),
        retried.request_digest(),
        "transport attempt and both legacy timeout fields are excluded from request identity"
    );

    let mut canonical = first_wire;
    let outer = canonical.as_object_mut().expect("outer object");
    outer.remove("request_digest");
    outer.remove("transport_attempt");
    let base = outer
        .get_mut("base_decision_request")
        .and_then(Value::as_object_mut)
        .expect("base request object");
    base.remove("timeout_budget_ms");
    base.get_mut("observation")
        .and_then(Value::as_object_mut)
        .expect("observation object")
        .remove("timeout_budget_ms");
    let runtime = outer
        .get_mut("runtime_binding")
        .and_then(Value::as_object_mut)
        .expect("runtime binding object");
    runtime.insert("finality_block_hash".to_string(), Value::Null);
    let legacy_canonical_bytes = oasis7_wasm_abi::encode_canonical_cbor(&canonical)
        .expect("legacy canonical request fixture");
    let expected = legacy_h_v1("oasis7.cognition.request.v1", &legacy_canonical_bytes);
    assert_eq!(
        first
            .canonical_request_bytes()
            .expect("new canonical bytes"),
        legacy_canonical_bytes
    );
    assert_eq!(first.request_digest(), expected);
    assert_eq!(
        first.request_digest().as_str(),
        "blake3:6275bb3e9a13260675a6783ab498103f2ea695b5b31f55493fdde74ee4573c70"
    );
}

#[test]
fn continuation_fixture_keeps_null_optionals_in_digest_domain() {
    let fixture = continuation_fixture();
    let mut proposal: ContinuationProposalV1 =
        serde_json::from_value(fixture.clone()).expect("legacy continuation fixture");
    proposal.proposal_digest = proposal
        .proposal_digest()
        .expect("proposal digest")
        .to_string();
    let digest = proposal.proposal_digest().expect("stable proposal digest");
    assert_eq!(
        digest.as_str(),
        "blake3:a2390a5908bd4568a1ebd77d3e82c876556d8e93697ea60890ba3137d9adb8ea"
    );
    assert_eq!(digest, legacy_continuation_digest(&fixture));
    assert_eq!(proposal.proposal_digest, digest.as_str());
    assert!(proposal.validate().is_ok());

    let serialized = serde_json::to_value(&proposal).expect("serialized proposal");
    let mut legacy_wire = fixture;
    legacy_wire["proposal_digest"] = json!(digest.as_str());
    assert_eq!(serialized, legacy_wire);
    assert_eq!(serialized["action_or_envelope_digest"], Value::Null);
    assert_eq!(
        serialized["wake_conditions"][0]["logical_tick"],
        Value::Null
    );
    let expected_bytes =
        oasis7_wasm_abi::encode_canonical_cbor(&serialized).expect("proposal admission bytes");
    assert_eq!(
        proposal.runtime_admission_bytes().expect("admission bytes"),
        expected_bytes
    );
}

fn legacy_continuation_digest(fixture: &Value) -> Digest32 {
    let mut value = fixture.clone();
    let object = value.as_object_mut().expect("legacy proposal object");
    object.remove("proposal_digest");
    if let Some(wakes) = object
        .get_mut("wake_conditions")
        .and_then(Value::as_array_mut)
    {
        for wake in wakes {
            if let Some(wake) = wake.as_object_mut() {
                for field in [
                    "logical_tick",
                    "event_digest",
                    "receipt_id",
                    "subject",
                    "path_or_rule",
                    "operator",
                    "expected_value_bytes",
                ] {
                    wake.entry(field).or_insert(Value::Null);
                }
            }
        }
    }
    legacy_h_v1("oasis7.cognition.continuation-proposal.v1", &value)
}

#[test]
fn receipt_and_feedback_are_correlations_not_readback_proofs() {
    let lineage = RuntimeReceiptLineageV1 {
        schema_version: RuntimeReceiptLineageV1::SCHEMA_VERSION.to_string(),
        status: "committed".to_string(),
        receipt_id: "receipt-1".to_string(),
        receipt_digest: HASH_A.to_string(),
        envelope_digest: HASH_B.to_string(),
        action_id: "action:7".to_string(),
        agent_id: "agent-1".to_string(),
        agent_session_id: "session-1".to_string(),
        agent_turn_id: "turn-1".to_string(),
        decision_request_id: "request-1".to_string(),
        request_digest: HASH_C.to_string(),
        feedback_id: "feedback-1".to_string(),
    };
    assert!(lineage.validate().is_ok());
    assert_eq!(
        serde_json::to_value(&lineage).expect("lineage serialization")["status"],
        "committed"
    );

    let feedback = FeedbackEnvelopeV1 {
        feedback_id: "feedback-1".to_string(),
        feedback_seq: 1,
        agent_subject: "agent-1".to_string(),
        agent_session_id: "session-1".to_string(),
        agent_turn_id: "turn-1".to_string(),
        decision_request_id: "request-1".to_string(),
        candidate_action_id: Some(7),
        runtime_receipt_id: Some("receipt-1".to_string()),
        status: "committed".to_string(),
        request_digest: HASH_C.into(),
        reject_reason: None,
        provenance: "runtime_authoritative".to_string(),
    };
    assert!(feedback.validate().is_ok());
    assert!(FeedbackEnvelopeV1::validate_value(&serde_json::to_value(feedback).unwrap()).is_ok());
}

#[test]
fn feedback_status_registry_does_not_invent_cancellation_disposition() {
    for status in ["pending", "committed", "rejected", "failed"] {
        let feedback = FeedbackEnvelopeV1 {
            feedback_id: "feedback-1".to_string(),
            feedback_seq: 1,
            agent_subject: "agent-1".to_string(),
            agent_session_id: "session-1".to_string(),
            agent_turn_id: "turn-1".to_string(),
            decision_request_id: "request-1".to_string(),
            candidate_action_id: (status == "committed").then_some(7),
            runtime_receipt_id: (status == "committed").then(|| "receipt-1".to_string()),
            status: status.to_string(),
            request_digest: HASH_C.into(),
            reject_reason: None,
            provenance: "runtime_authoritative".to_string(),
        };
        assert!(feedback.validate().is_ok(), "existing status `{status}`");
    }
    let cancelled = FeedbackEnvelopeV1 {
        feedback_id: "feedback-1".to_string(),
        feedback_seq: 1,
        agent_subject: "agent-1".to_string(),
        agent_session_id: "session-1".to_string(),
        agent_turn_id: "turn-1".to_string(),
        decision_request_id: "request-1".to_string(),
        candidate_action_id: None,
        runtime_receipt_id: None,
        status: "cancelled".to_string(),
        request_digest: HASH_C.into(),
        reject_reason: None,
        provenance: "runtime_authoritative".to_string(),
    };
    assert_eq!(
        cancelled.validate().unwrap_err().code(),
        "feedback_contract_invalid"
    );
}

#[test]
fn response_artifact_identity_digest_keeps_the_existing_domain_and_fields() {
    let mut identity = ResponseArtifactIdentityV1 {
        schema_version: 1,
        context_discriminator: "oasis7.continuous-agent-context".to_string(),
        context_version: 1,
        agent_session_id: "session-1".to_string(),
        agent_turn_id: "turn-1".to_string(),
        decision_request_id: "request-1".to_string(),
        retry_seq: 2,
        transport_attempt: 3,
        request_digest: HASH_A.into(),
        response_digest: HASH_B.into(),
        artifact_digest: Digest32::default(),
    };
    let mut legacy_payload = serde_json::to_value(&identity).expect("legacy identity payload");
    legacy_payload
        .as_object_mut()
        .expect("identity object")
        .remove("artifact_digest");
    identity.artifact_digest = legacy_h_v1(
        "oasis7.cognition.response-artifact-identity.v1",
        &legacy_payload,
    );
    assert_eq!(
        identity.artifact_digest.as_str(),
        "blake3:f448bedfaf3ad779e1cf475b37e068cef152812ac80744763830c4bcafeb81c4"
    );
    assert!(identity.validate().is_ok());
}

#[test]
fn lease_view_exposes_consumption_identity_without_durable_accounting() {
    let view = CognitionLeaseConsumptionViewV1 {
        schema_version: CognitionLeaseConsumptionViewV1::SCHEMA_VERSION.to_string(),
        lease_id: "lease-1".to_string(),
        idempotency_key: "invocation-1".to_string(),
        agent_id: "agent-1".to_string(),
        agent_session_id: "session-1".to_string(),
        agent_turn_id: "turn-1".to_string(),
        decision_request_id: "request-1".to_string(),
        request_digest: HASH_D.to_string(),
        status: CognitionLeaseStatusV1::Reserved,
        reserved_amount: 1,
        reserved_at_tick: 42,
    };
    let value = serde_json::to_value(view).expect("lease view");
    assert_eq!(value["status"], "reserved");
    for forbidden in [
        "quote",
        "settled_amount",
        "released_amount",
        "refunded_amount",
        "receipt_id",
    ] {
        assert!(
            value.get(forbidden).is_none(),
            "lease projection must omit {forbidden}"
        );
    }
}

#[test]
fn continuation_budget_wire_shape_is_unchanged() {
    let budget = ContinuationBudgetV1 {
        unit: "steps".to_string(),
        value: 2,
    };
    assert_eq!(
        serde_json::to_value(budget).unwrap(),
        json!({"unit":"steps", "value":2})
    );
    let wake: WakeConditionV1 = serde_json::from_value(json!({
        "schema_version":"wake-condition.v1", "kind":"at_or_after_tick", "logical_tick":12
    }))
    .expect("wake condition fixture");
    assert_eq!(
        serde_json::to_value(wake).unwrap(),
        json!({
            "schema_version":"wake-condition.v1", "kind":"at_or_after_tick", "logical_tick":12
        })
    );
}
