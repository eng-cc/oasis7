//! Pure memory gate probes use a genuinely persisted native checkpoint and actual signed Lookup receipt.
use super::*;
use oasis7::viewer::ViewerRuntimeLiveServer;

#[test]
fn real_tcp_captured_native_memory_gate_rejects_checkpoint_and_receipt_tampering() {
    application_harness::run_isolated_application_mode(
        false,
        false,
        false,
        true,
        false,
        false,
        "memory-crash-gate",
    );
}

pub(super) fn verify_actual_checkpoint_gate(fixture: &Fixture, bytes: &[u8]) {
    let checkpoint: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    let pending_map = checkpoint["provider_service_pending"].as_object().unwrap();
    assert_eq!(
        pending_map.len(),
        1,
        "exact original pending Act checkpoint required"
    );
    let pending = pending_map.values().next().unwrap().clone();
    let request: SubmitIntentRequest<WorldServicePayloadV1> = serde_json::from_value(serde_json::json!({
        "contract_version":1,"correlation":pending["correlation"],"signed_payload":pending["payload"],"deadline_unix_ms":null
    })).unwrap();
    fixture.committed(&request);
    let actual = fixture
        .client
        .lookup(
            LookupIntentRequest {
                contract_version: 1,
                key: request.correlation.key.clone(),
            },
            request.signed_payload,
        )
        .unwrap();
    let IntentOutcome::Committed { receipt, .. } = actual.outcome else {
        panic!("genuine signed Lookup must supply canonical cognition receipt");
    };
    let positive = ViewerRuntimeLiveServer::test_project_original_receipt_memory(
        pending.clone(),
        receipt.clone(),
    );
    assert_eq!(
        positive["accepted"], true,
        "unaltered actual checkpoint and receipt must pass before tamper probes"
    );
    assert!(
        positive["memory_store"]
            .to_string()
            .contains("native canonical memory survives application process loss")
    );
    let empty = serde_json::to_value(oasis7::simulator::MemoryWriteStore::default()).unwrap();
    for kind in [
        "original_context",
        "original_response",
        "captured_intents",
        "receipt_request_digest",
        "receipt_feedback_id",
    ] {
        let mut changed_pending = pending.clone();
        let mut changed_receipt = receipt.clone();
        match kind {
            "original_context" => {
                changed_pending["cognition"]["request"]["request_context"]["goal_snapshot_digest"] =
                    serde_json::json!(format!("blake3:{}", "f".repeat(64)))
            }
            "original_response" => {
                changed_pending["cognition"]["response"]["base_decision_response"]["memory_write_intents"]
                    [0]["summary"] = serde_json::json!("tampered original response memory")
            }
            "captured_intents" => {
                changed_pending["cognition"]["memory_write_intents"][0]["summary"] =
                    serde_json::json!("tampered captured memory intent")
            }
            "receipt_request_digest" => {
                changed_receipt["commit_record"]["request_digest"] =
                    serde_json::json!(format!("blake3:{}", "e".repeat(64)))
            }
            "receipt_feedback_id" => {
                changed_receipt["feedback"]["feedback_id"] =
                    serde_json::json!("tampered-canonical-feedback-id")
            }
            _ => unreachable!(),
        }
        let rejected = ViewerRuntimeLiveServer::test_project_original_receipt_memory(
            changed_pending,
            changed_receipt,
        );
        assert_eq!(rejected["accepted"], false, "{kind} tamper accepted");
        assert_eq!(rejected["error_category"], "receipt_memory_gate_rejected");
        assert_eq!(
            rejected["memory_store"], empty,
            "{kind} mutated fresh memory on rejection"
        );
        println!(
            "native_memory_real_capture_gate_negative kind={kind} rejected=true fresh_memory_unchanged=true"
        );
    }
    application_fairness::validate_canonical_identity(fixture);
    println!("PRE2_ACTUAL_CAPTURE_MEMORY_GATE_FIVE_NEGATIVES_PASSED pure_gate_only=true");
}
