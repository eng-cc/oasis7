use std::io::Cursor;

use base64::Engine;
use oasis7_local_signer::protocol::{
    IpcRequest, MAX_FRAME_BYTES, MAX_METADATA_BYTES, RollbackProtocolContext, SignContext,
    decode_payload, encode_request_frame, read_request_frame,
};

fn rollback_request(payload: &[u8]) -> IpcRequest {
    IpcRequest::Sign {
        schema_version: "oasis7.local_signer_ipc.v1".to_owned(),
        installation_id: "install-01".to_owned(),
        request_id: "request-01".to_owned(),
        purpose: "rollback_strict_audit_v1".to_owned(),
        provider_id: None.into(),
        signer_id: "rollback-audit-r1".to_owned(),
        grant_id: Some("grant-01".to_owned()).into(),
        context: SignContext {
            deployment_id: "testnet-a".to_owned(),
            network_id: "network-01".to_owned(),
            task_uid: "task_0123456789abcdef0123456789abcdef".to_owned(),
            source_head_oid: "0123456789abcdef0123456789abcdef01234567".to_owned(),
            protocol_context: RollbackProtocolContext {
                authority_id: "authority-01".to_owned(),
                rollback_ticket: "ticket-01".to_owned(),
                receipt_id: "receipt-01".to_owned(),
                nonce: "nonce-01".to_owned(),
            },
        },
        payload_base64: base64::engine::general_purpose::STANDARD.encode(payload),
    }
}

fn frame_json(json: &[u8]) -> Vec<u8> {
    let mut frame = (json.len() as u32).to_be_bytes().to_vec();
    frame.extend_from_slice(json);
    frame
}

#[test]
fn rollback_sign_request_roundtrips_and_preserves_payload_bytes() {
    let payload = b"oasis7:rollback-strict-audit-evidence:v1\0\x7b\x7d";
    let request = rollback_request(payload);
    let frame = encode_request_frame(&request).expect("encode request");
    assert!(frame.len() <= MAX_FRAME_BYTES + 4);

    let decoded = read_request_frame(&mut Cursor::new(frame)).expect("decode request");
    let IpcRequest::Sign { payload_base64, .. } = decoded else {
        panic!("expected sign request");
    };
    assert_eq!(decode_payload(&payload_base64).expect("payload"), payload);
}

#[test]
fn request_frame_rejects_unknown_and_duplicate_fields() {
    let unknown = br#"{"schema_version":"oasis7.local_signer_ipc.v1","command":"doctor","installation_id":"install-01","extra":true}"#;
    assert!(read_request_frame(&mut Cursor::new(frame_json(unknown))).is_err());

    let duplicate = br#"{"schema_version":"oasis7.local_signer_ipc.v1","schema_version":"oasis7.local_signer_ipc.v1","command":"doctor","installation_id":"install-01"}"#;
    assert!(read_request_frame(&mut Cursor::new(frame_json(duplicate))).is_err());
}

#[test]
fn request_frame_rejects_truncated_trailing_and_oversized_input() {
    let mut truncated_header = Cursor::new([0_u8, 0, 0]);
    assert!(read_request_frame(&mut truncated_header).is_err());

    let mut truncated_body = Cursor::new([0_u8, 0, 0, 4, b'{', b'}']);
    assert!(read_request_frame(&mut truncated_body).is_err());

    let doctor = br#"{"schema_version":"oasis7.local_signer_ipc.v1","command":"doctor","installation_id":"install-01"}"#;
    let mut with_trailing = frame_json(doctor);
    with_trailing.push(0);
    assert!(read_request_frame(&mut Cursor::new(with_trailing)).is_err());

    let oversized = (u32::try_from(MAX_FRAME_BYTES + 1).expect("frame cap fits u32")).to_be_bytes();
    assert!(read_request_frame(&mut Cursor::new(oversized)).is_err());
}

#[test]
fn request_frame_enforces_metadata_limit_separately_from_payload_limit() {
    let huge_metadata = format!(
        "{{\"schema_version\":\"oasis7.local_signer_ipc.v1\",\"command\":\"doctor\",\"installation_id\":\"{}\"}}",
        "x".repeat(MAX_METADATA_BYTES)
    );
    assert!(read_request_frame(&mut Cursor::new(frame_json(huge_metadata.as_bytes()))).is_err());

    let too_large_payload = vec![b'x'; 16 * 1024 * 1024 + 1];
    let request = rollback_request(&too_large_payload);
    assert!(encode_request_frame(&request).is_err());
}
