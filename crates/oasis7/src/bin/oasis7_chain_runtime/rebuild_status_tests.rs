use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use oasis7_node::{NodeConsensusSnapshot, NodeRole, NodeSnapshot, ReplicationNetworkDebugSnapshot};
use sha2::{Digest, Sha256};

use super::execution_bridge::ExecutionCheckpointStatusEvidence;
use super::feedback_submit_api::FeedbackSubmitSigner;
use super::rebuild_status::{
    RebuildStatusResponse, build_rebuild_status_with_signer, verify_rebuild_proof,
    verify_rebuild_proof_file,
};

#[test]
fn rebuild_status_is_bounded_and_contains_rebuild_proof_fields_only() {
    let snapshot = snapshot();
    let network = ReplicationNetworkDebugSnapshot {
        local_peer_id: "12D3KooWLocal".to_string(),
        connected_peers: vec!["12D3KooWPeer".to_string()],
        peer_healths: Vec::new(),
        registered_protocols: Vec::new(),
        protocol_retry_cooldown_peers: Default::default(),
        transport_retry_cooldown_peers: Vec::new(),
        request_peer_scores: Default::default(),
        recent_errors: Vec::new(),
    };
    let response =
        build_rebuild_status_with_signer(snapshot, network, None, 1_000, None, &signer())
            .expect("bounded proof response");
    let json = serde_json::to_value(&response).expect("serialize bounded response");
    for key in [
        "schema_version",
        "ok",
        "liveness",
        "readiness",
        "heights",
        "network_head",
        "checkpoint",
        "local_peer_id",
        "connected_peers",
    ] {
        assert!(json.get(key).is_some(), "missing bounded field {key}");
    }
    assert!(json.get("consensus").is_none());
    assert!(json.get("observability").is_none());
    assert_eq!(response.local_peer_id, "12D3KooWLocal");
    assert_eq!(response.connected_peers, vec!["12D3KooWPeer"]);
    assert!(json.get("proof").is_some());
    assert!(json.get("identity").is_none());
    assert!(json.get("signature").is_none());
    verify_rebuild_proof(&response).expect("proof envelope verifies");
}

#[test]
fn rebuild_status_marks_stopped_runtime_not_ready_without_full_status_builder() {
    let response = build_rebuild_status_with_signer(
        snapshot(),
        ReplicationNetworkDebugSnapshot {
            local_peer_id: "local".to_string(),
            connected_peers: Vec::new(),
            peer_healths: Vec::new(),
            registered_protocols: Vec::new(),
            protocol_retry_cooldown_peers: Default::default(),
            transport_retry_cooldown_peers: Vec::new(),
            request_peer_scores: Default::default(),
            recent_errors: Vec::new(),
        },
        Some(ExecutionCheckpointStatusEvidence {
            schema_version: 2,
            checkpoint_id: "checkpoint-1".to_string(),
            world_id: "world".to_string(),
            height: 1,
            execution_block_hash: "exec-1".to_string(),
            execution_state_root: "state-1".to_string(),
            manifest_hash: "manifest".to_string(),
        }),
        1_000,
        None,
        &signer(),
    )
    .expect("bounded proof response");
    assert!(!response.ok);
    assert!(!response.liveness.running);
    assert_eq!(response.readiness.status, "not_ready");
    assert_eq!(response.checkpoint.unwrap().height, 1);
}

#[allow(dead_code)]
fn _type_check(_: RebuildStatusResponse) {}

#[test]
fn rebuild_status_rejects_checkpoint_head_mismatch_and_signs_not_ready_proof() {
    let mut snapshot = snapshot();
    snapshot.running = true;
    snapshot.world_id = "world-a".to_string();
    snapshot.consensus.committed_height = 42;
    snapshot.consensus.network_committed_height = 42;
    snapshot.consensus.last_execution_height = 42;
    snapshot.consensus.last_execution_block_hash = Some("exec-42".to_string());
    snapshot.consensus.last_execution_state_root = Some("state-42".to_string());
    let checkpoint = ExecutionCheckpointStatusEvidence {
        schema_version: 1,
        checkpoint_id: "checkpoint-42".to_string(),
        world_id: "world-a".to_string(),
        height: 42,
        execution_block_hash: "tampered-exec-42".to_string(),
        execution_state_root: "state-42".to_string(),
        manifest_hash: "manifest-42".to_string(),
    };
    let response = build_rebuild_status_with_signer(
        snapshot,
        network(),
        Some(checkpoint),
        1_000,
        None,
        &signer(),
    )
    .expect("proof response");
    assert!(!response.ok);
    assert_eq!(response.readiness.status, "not_ready");
    assert!(
        response
            .readiness
            .failed_gates
            .iter()
            .any(|gate| gate == "checkpoint_head_mismatch")
    );
    verify_rebuild_proof(&response).expect("not-ready proof still verifies");
}

#[test]
fn rebuild_status_accepts_retained_checkpoint_below_current_execution_head() {
    let mut snapshot = snapshot();
    snapshot.running = true;
    snapshot.world_id = "world-a".to_string();
    snapshot.consensus.committed_height = 64;
    snapshot.consensus.network_committed_height = 64;
    snapshot.consensus.last_execution_height = 64;
    snapshot.consensus.last_execution_block_hash = Some("exec-64".to_string());
    snapshot.consensus.last_execution_state_root = Some("state-64".to_string());
    let checkpoint = ExecutionCheckpointStatusEvidence {
        schema_version: 1,
        checkpoint_id: "checkpoint-42".to_string(),
        world_id: "world-a".to_string(),
        height: 42,
        execution_block_hash: "exec-42".to_string(),
        execution_state_root: "state-42".to_string(),
        manifest_hash: "manifest-42".to_string(),
    };
    let response = build_rebuild_status_with_signer(
        snapshot,
        network(),
        Some(checkpoint),
        1_000,
        None,
        &signer(),
    )
    .expect("proof response");
    assert!(
        response.ok,
        "retained checkpoint should be valid below head"
    );
    assert_eq!(
        response.checkpoint.as_ref().map(|item| item.height),
        Some(42)
    );
    verify_rebuild_proof(&response).expect("retained-boundary proof verifies");
}

#[test]
fn rebuild_proof_file_verifier_binds_expected_signer_and_public_key() {
    let response =
        build_rebuild_status_with_signer(snapshot(), network(), None, 1_000, None, &signer())
            .expect("bounded proof response");
    let path = temp_proof_path();
    fs::write(
        &path,
        serde_json::to_vec(&response).expect("serialize proof response"),
    )
    .expect("write proof response");
    verify_rebuild_proof_file(
        &path,
        response.proof.signer_id.as_str(),
        response.proof.signer_public_key_hex.as_str(),
    )
    .expect("trusted proof file verifies");
    let err = verify_rebuild_proof_file(
        &path,
        "unexpected-signer",
        response.proof.signer_public_key_hex.as_str(),
    )
    .expect_err("unexpected signer must fail closed");
    assert!(err.contains("trusted signer id mismatch"), "{err}");
    let mut tampered = serde_json::to_value(&response).expect("serialize tampered proof");
    tampered["proof"]["signature_hex"] = serde_json::Value::String("00".repeat(64));
    fs::write(
        &path,
        serde_json::to_vec(&tampered).expect("serialize tampered proof value"),
    )
    .expect("write tampered proof");
    let err = verify_rebuild_proof_file(
        &path,
        response.proof.signer_id.as_str(),
        response.proof.signer_public_key_hex.as_str(),
    )
    .expect_err("tampered signature must fail closed");
    assert!(err.contains("signature verification failed"), "{err}");
    let _ = fs::remove_file(path);
}

#[test]
fn rebuild_proof_verification_receipt_binds_peer_and_exact_raw_proof_digest() {
    let mut snapshot = snapshot();
    snapshot.node_id = "sequencer-node".to_string();
    let response =
        build_rebuild_status_with_signer(snapshot, network(), None, 1_000, None, &signer())
            .expect("bounded proof response");
    let path = temp_proof_path();
    let bytes = serde_json::to_vec(&response).expect("serialize proof response");
    fs::write(&path, bytes.as_slice()).expect("write proof response");
    let receipt = verify_rebuild_proof_file(
        &path,
        response.proof.signer_id.as_str(),
        response.proof.signer_public_key_hex.as_str(),
    )
    .expect("trusted proof file verifies");
    assert_eq!(receipt.local_peer_id, response.local_peer_id);
    assert_eq!(
        receipt.proof_sha256,
        hex::encode(Sha256::digest(bytes.as_slice()))
    );
    assert_ne!(
        receipt.proof_sha256,
        hex::encode(Sha256::digest(b"different-proof"))
    );
    let _ = fs::remove_file(path);
}

#[test]
fn rebuild_proof_verification_receipt_rejects_tampered_or_wrong_raw_proof_binding() {
    let response_a =
        build_rebuild_status_with_signer(snapshot(), network(), None, 1_000, None, &signer())
            .expect("first bounded proof response");
    let response_b =
        build_rebuild_status_with_signer(snapshot(), network(), None, 2_000, None, &signer())
            .expect("second bounded proof response");
    let path_a = temp_proof_path();
    let path_b = temp_proof_path();
    let bytes_a = serde_json::to_vec(&response_a).expect("serialize first proof");
    let bytes_b = serde_json::to_vec(&response_b).expect("serialize second proof");
    fs::write(&path_a, bytes_a.as_slice()).expect("write first proof");
    fs::write(&path_b, bytes_b.as_slice()).expect("write second proof");
    let receipt_a = verify_rebuild_proof_file(
        &path_a,
        response_a.proof.signer_id.as_str(),
        response_a.proof.signer_public_key_hex.as_str(),
    )
    .expect("first proof verifies");
    let receipt_b = verify_rebuild_proof_file(
        &path_b,
        response_b.proof.signer_id.as_str(),
        response_b.proof.signer_public_key_hex.as_str(),
    )
    .expect("second proof verifies");
    assert_ne!(receipt_a.proof_sha256, receipt_b.proof_sha256);
    let tampered = String::from_utf8(bytes_b)
        .expect("proof JSON is UTF-8")
        .replace(
            "\"observed_at_unix_ms\":2000",
            "\"observed_at_unix_ms\":2001",
        )
        .into_bytes();
    fs::write(&path_b, tampered).expect("write tampered second proof");
    assert!(
        verify_rebuild_proof_file(
            &path_b,
            response_b.proof.signer_id.as_str(),
            response_b.proof.signer_public_key_hex.as_str(),
        )
        .is_err()
    );
    let _ = fs::remove_file(path_a);
    let _ = fs::remove_file(path_b);
}

fn temp_proof_path() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("oasis7-rebuild-proof-{nonce}.json"))
}

fn snapshot() -> NodeSnapshot {
    NodeSnapshot {
        node_id: "node".to_string(),
        player_id: "player".to_string(),
        world_id: "world".to_string(),
        role: NodeRole::Sequencer,
        replication_enabled: false,
        running: false,
        tick_count: 0,
        last_tick_unix_ms: None,
        consensus: NodeConsensusSnapshot::default(),
        consensus_progress_observer_error: None,
        last_error: None,
    }
}

fn network() -> ReplicationNetworkDebugSnapshot {
    ReplicationNetworkDebugSnapshot {
        local_peer_id: "local".to_string(),
        connected_peers: Vec::new(),
        peer_healths: Vec::new(),
        registered_protocols: Vec::new(),
        protocol_retry_cooldown_peers: Default::default(),
        transport_retry_cooldown_peers: Vec::new(),
        request_peer_scores: Default::default(),
        recent_errors: Vec::new(),
    }
}

fn signer() -> FeedbackSubmitSigner {
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&[7_u8; 32]);
    FeedbackSubmitSigner {
        private_key_hex: hex::encode(signing_key.to_bytes()),
        public_key_hex: hex::encode(signing_key.verifying_key().to_bytes()),
    }
}
