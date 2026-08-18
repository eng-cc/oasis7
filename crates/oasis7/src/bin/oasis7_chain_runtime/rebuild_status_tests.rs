use oasis7_node::{NodeConsensusSnapshot, NodeRole, NodeSnapshot, ReplicationNetworkDebugSnapshot};

use super::execution_bridge::ExecutionCheckpointStatusEvidence;
use super::feedback_submit_api::FeedbackSubmitSigner;
use super::rebuild_status::{
    RebuildStatusResponse, build_rebuild_status_with_signer, verify_rebuild_proof,
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
