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

#[test]
fn governed_rollback_v2_requires_distinct_checkpoint_and_later_replay_target_shape() {
    let v2 = r#"{
        "target_batch_id":"batch-target-12",
        "reason":"recover-and-catch-up",
        "approval":{
            "intent":{
                "schema_version":2,
                "rollback_ticket":"ROLLBACK-2313",
                "rollback_checkpoint":{
                    "batch_id":"batch-checkpoint-8",
                    "snapshot_hash":"snapshot-8",
                    "journal_len":80
                },
                "replay_target":{
                    "batch_id":"batch-target-12",
                    "journal_len":120,
                    "state_root":"state-root-12",
                    "journal_commitment":"journal-commitment-12"
                },
                "expected_reorg_epoch":3,
                "max_replay_events":40,
                "max_replay_bytes":65536,
                "reason":"recover-and-catch-up",
                "issued_at_ms":1720000000000,
                "expires_at_ms":1720000060000,
                "nonce":"nonce-12"
            },
            "signatures":[]
        }
    }"#;
    let decoded: AuthoritativeRollbackRequest =
        serde_json::from_str(v2).expect("canonical v2 rollback wire shape must decode");
    let encoded = serde_json::to_value(decoded).expect("re-encode canonical v2 rollback");

    assert_eq!(
        encoded["approval"]["intent"]["rollback_checkpoint"]["batch_id"],
        serde_json::json!("batch-checkpoint-8")
    );
    assert_eq!(
        encoded["approval"]["intent"]["replay_target"]["batch_id"],
        serde_json::json!("batch-target-12")
    );
    assert_ne!(
        encoded["approval"]["intent"]["rollback_checkpoint"]["batch_id"],
        encoded["approval"]["intent"]["replay_target"]["batch_id"],
        "rollback checkpoint C and replay target T must remain distinct on the wire"
    );
}

fn sample_v2_intent() -> RollbackIntentV2 {
    RollbackIntentV2 {
        schema_version: 2,
        rollback_ticket: "ROLLBACK-2313".to_string(),
        rollback_checkpoint: RollbackCheckpointRef {
            batch_id: "batch-c".to_string(),
            snapshot_hash: "snapshot-c".to_string(),
            snapshot_journal_len: 8,
        },
        replay_target: RollbackReplayTarget {
            batch_id: "batch-t".to_string(),
            target_journal_len: 12,
            expected_target_state_root: "state-t".to_string(),
            journal_commitment: "journal-t".to_string(),
        },
        expected_reorg_epoch: 3,
        max_replay_events: 4,
        max_replay_bytes: 4096,
        reason: "recover".to_string(),
        issued_at_ms: 1_720_000_000_000,
        expires_at_ms: 1_720_000_060_000,
        nonce: "nonce-v2".to_string(),
    }
}

#[test]
fn v2_signature_binds_checkpoint_batch_identity() {
    let original = sample_v2_intent();
    let mut changed = original.clone();
    changed.rollback_checkpoint.batch_id = "batch-other-c".to_string();
    assert_ne!(
        original
            .canonical_signing_payload()
            .expect("original payload"),
        changed
            .canonical_signing_payload()
            .expect("changed payload"),
        "changing only checkpoint batch identity must invalidate the signature and nonce digest"
    );
}

#[test]
fn v2_signature_binds_expected_reorg_epoch() {
    let original = sample_v2_intent();
    let mut changed = original.clone();
    changed.expected_reorg_epoch += 1;
    assert_ne!(
        original
            .canonical_signing_payload()
            .expect("original payload"),
        changed
            .canonical_signing_payload()
            .expect("changed payload")
    );
}

#[test]
fn v2_signature_binds_replay_event_and_byte_bounds() {
    let original = sample_v2_intent();
    for changed in [
        RollbackIntentV2 {
            max_replay_events: original.max_replay_events + 1,
            ..original.clone()
        },
        RollbackIntentV2 {
            max_replay_bytes: original.max_replay_bytes + 1,
            ..original.clone()
        },
    ] {
        assert_ne!(
            original
                .canonical_signing_payload()
                .expect("original payload"),
            changed
                .canonical_signing_payload()
                .expect("changed payload")
        );
    }
}
