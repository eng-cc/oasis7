use super::*;

#[test]
fn viewer_response_round_trip_authoritative_recovery_ack() {
    let response = ViewerResponse::<
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        u64,
    >::AuthoritativeRecoveryAck {
        ack: AuthoritativeRecoveryAck {
            status: AuthoritativeRecoveryStatus::SessionRotated,
            reorg_epoch: 2,
            snapshot_height: 88,
            snapshot_hash: "snapshot-hash-1".to_string(),
            log_cursor: 123,
            stable_batch_id: Some("batch-9".to_string()),
            player_id: Some("player-1".to_string()),
            agent_id: Some("agent-7".to_string()),
            session_pubkey: Some("old-key".to_string()),
            replaced_by_pubkey: Some("new-key".to_string()),
            session_epoch: Some(5),
            message: Some("session rotated".to_string()),
            revoke_reason: Some("compromised".to_string()),
            revoked_by: Some("ops".to_string()),
            rollback_receipt: None,
            acknowledged_at_tick: 89,
        },
    };
    let json = serde_json::to_string(&response).expect("serialize response");
    let parsed: ViewerResponse<
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        u64,
    > = serde_json::from_str(&json).expect("deserialize response");
    assert_eq!(parsed, response);
}

#[test]
fn viewer_hello_ack_defaults_to_playback_profile_for_default_payload() {
    let json = r#"{
        "type":"hello_ack",
        "server":"oasis7",
        "version":1,
        "world_id":"w1"
    }"#;
    let parsed: ViewerResponse<
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        u64,
    > = serde_json::from_str(json).expect("deserialize hello ack");
    let ViewerResponse::HelloAck {
        control_profile, ..
    } = parsed
    else {
        panic!("expected hello ack");
    };
    assert_eq!(control_profile, ViewerControlProfile::Playback);
}

#[test]
fn viewer_v2_hello_ack_advertises_signed_rollback_capability() {
    let response = ViewerResponse::<
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        u64,
    >::HelloAck {
        server: "oasis7".to_string(),
        version: VIEWER_PROTOCOL_VERSION,
        min_version: 1,
        max_version: VIEWER_PROTOCOL_VERSION,
        capabilities: vec![SIGNED_AUTHORITATIVE_ROLLBACK_CAPABILITY.to_string()],
        world_id: "world-v2".to_string(),
        control_profile: ViewerControlProfile::Live,
    };
    let value = serde_json::to_value(response).expect("serialize hello ack");

    assert_eq!(
        VIEWER_PROTOCOL_VERSION, 2,
        "signed rollback requires protocol v2"
    );
    assert_eq!(value["min_version"], serde_json::json!(1));
    assert_eq!(value["max_version"], serde_json::json!(2));
    assert!(
        value["capabilities"].as_array().is_some_and(|items| items
            .iter()
            .any(|item| { item == &serde_json::json!("signed_authoritative_rollback_v1") })),
        "v2 hello must explicitly negotiate signed rollback"
    );
}

#[test]
fn rollback_ack_round_trip_carries_replay_target_and_idempotent_receipt() {
    let json = r#"{
        "type":"authoritative_recovery_ack",
        "ack":{
            "status":"rolled_back",
            "reorg_epoch":4,
            "snapshot_height":80,
            "snapshot_hash":"snapshot-80",
            "log_cursor":400,
            "stable_batch_id":"batch-2",
            "acknowledged_at_tick":80,
            "rollback_receipt":{
                "receipt_id":"rollback-receipt-1",
                "authorization_nonce":"nonce-1",
                "rollback_ticket":"ROLLBACK-2313",
                "target_batch_id":"batch-2",
                "invalidated_batch_ids":["batch-3","batch-4"],
                "prior_reorg_epoch":3,
                "committed_reorg_epoch":4,
                "replay_from_snapshot_height":80,
                "replay_from_log_cursor":400,
                "snapshot_hash":"snapshot-80",
                "snapshot_reload_required":true
            }
        }
    }"#;
    let parsed: ViewerResponse<
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        serde_json::Value,
        u64,
    > = serde_json::from_str(json).expect("decode v2 rollback receipt");
    let encoded = serde_json::to_value(parsed).expect("re-encode v2 rollback receipt");

    assert_eq!(
        encoded["ack"]["rollback_receipt"]["invalidated_batch_ids"],
        serde_json::json!(["batch-3", "batch-4"])
    );
    assert_eq!(
        encoded["ack"]["rollback_receipt"]["snapshot_reload_required"],
        serde_json::json!(true)
    );
}
