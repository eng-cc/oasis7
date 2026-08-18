use oasis7_node::{NodeConsensusSnapshot, NodeRole, NodeSnapshot, ReplicationNetworkDebugSnapshot};

use super::rebuild_status::{RebuildStatusResponse, build_rebuild_status};

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
    let response = build_rebuild_status(snapshot, network, None, 1_000);
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
}

#[test]
fn rebuild_status_marks_stopped_runtime_not_ready_without_full_status_builder() {
    let response = build_rebuild_status(
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
        Some((2, "checkpoint-1".to_string(), 1, "manifest".to_string())),
        1_000,
    );
    assert!(!response.ok);
    assert!(!response.liveness.running);
    assert_eq!(response.readiness.status, "not_ready");
    assert_eq!(response.checkpoint.unwrap().height, 1);
}

#[allow(dead_code)]
fn _type_check(_: RebuildStatusResponse) {}

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
