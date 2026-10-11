use oasis7_agent_api::game::{
    ActionSubmissionV1, DecimalU64, MAX_JSON_DEPTH, MAX_REQUEST_BYTES, OperationStatusV1,
    RetryAdviceV1, parse_request,
};
use serde_json::{Value, json};

fn action() -> Value {
    json!({
        "api_revision": 1, "client_operation_id": "operation-1",
        "task_id": "task-1", "executor_epoch": "18446744073709551615",
        "context_id": "context-1", "action_ref": "schedule_recipe",
        "arguments": {"recipe_id": "smelter"},
        "actor_proof": {"challenge_id": "challenge-1", "subject_public_key": "device", "signature_hex": "proof"}
    })
}

#[test]
fn recovery_keeps_original_operation_and_unknown_statuses_fail_closed() {
    let pending: OperationStatusV1 = parse_request(br#""pending""#).unwrap();
    let recovery: OperationStatusV1 = parse_request(br#""recovery_required""#).unwrap();
    assert_eq!(pending.retry_advice(), RetryAdviceV1::Wait);
    assert_eq!(recovery.retry_advice(), RetryAdviceV1::LookupOriginal);
    for terminal in [
        OperationStatusV1::Committed,
        OperationStatusV1::Rejected,
        OperationStatusV1::Failed,
    ] {
        assert_eq!(terminal.retry_advice(), RetryAdviceV1::None);
    }
    for unknown in [
        br#""cancelled""#.as_slice(),
        br#""success""#,
        br#""run_unknown""#,
    ] {
        assert!(parse_request::<OperationStatusV1>(unknown).is_err());
    }
}

#[test]
fn action_roundtrip_preserves_full_u64_and_rejects_unknown_request_fields() {
    let value = action();
    let request: ActionSubmissionV1 = parse_request(&serde_json::to_vec(&value).unwrap()).unwrap();
    request.validate().unwrap();
    assert_eq!(request.executor_epoch, DecimalU64(u64::MAX));
    assert_eq!(serde_json::to_value(&request).unwrap(), value);
    for path in ["root", "proof"] {
        let mut value = action();
        if path == "root" {
            value["authority"] = json!("owner");
        } else {
            value["actor_proof"]["authority"] = json!("owner");
        }
        assert!(parse_request::<ActionSubmissionV1>(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}

#[test]
fn decimal_codec_rejects_numeric_and_noncanonical_epochs() {
    for invalid in [
        json!(1),
        json!("01"),
        json!("+1"),
        json!("-1"),
        json!(""),
        json!("18446744073709551616"),
    ] {
        let mut value = action();
        value["executor_epoch"] = invalid;
        assert!(parse_request::<ActionSubmissionV1>(&serde_json::to_vec(&value).unwrap()).is_err());
    }
}

#[test]
fn duplicate_keys_are_rejected_even_inside_dynamic_arguments() {
    for raw in [
        r#"{"api_revision":1,"api_revision":2}"#,
        r#"{"arguments":{"recipe_id":"a","recipe_id":"b"}}"#,
        r#"{"arguments":[{"key":1,"k\u0065y":2}]}"#,
    ] {
        let error = parse_request::<Value>(raw.as_bytes()).unwrap_err();
        assert!(error.to_string().contains("duplicate JSON key"));
    }
}

#[test]
fn parser_enforces_exact_depth_byte_and_single_document_boundaries() {
    let nested = |depth: usize| format!("{}0{}", "[".repeat(depth), "]".repeat(depth));
    assert!(parse_request::<Value>(nested(MAX_JSON_DEPTH).as_bytes()).is_ok());
    assert!(parse_request::<Value>(nested(MAX_JSON_DEPTH + 1).as_bytes()).is_err());
    let mut bytes = b"null".to_vec();
    bytes.resize(MAX_REQUEST_BYTES, b' ');
    assert!(parse_request::<Value>(&bytes).is_ok());
    bytes.push(b' ');
    assert!(parse_request::<Value>(&bytes).is_err());
    for invalid in [b"null null".as_slice(), b"{", b"[NaN]", b"\xff"] {
        assert!(parse_request::<Value>(invalid).is_err());
    }
}

#[test]
fn action_structure_cannot_claim_a_future_revision_or_scalar_arguments() {
    let mut request: ActionSubmissionV1 =
        parse_request(&serde_json::to_vec(&action()).unwrap()).unwrap();
    request.api_revision = 2;
    assert!(request.validate().is_err());
    request.api_revision = 1;
    request.arguments = json!("freeform command");
    assert!(request.validate().is_err());
    request.arguments = json!({});
    request.client_operation_id.clear();
    assert!(request.validate().is_err());
}
