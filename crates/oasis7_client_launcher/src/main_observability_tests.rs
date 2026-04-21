use super::*;

#[test]
fn apply_web_snapshot_tracks_chain_p2p_status_payload() {
    let mut app = ClientLauncherApp::default();
    let snapshot = WebStateSnapshot {
        status: "idle".to_string(),
        detail: None,
        chain_status: "ready".to_string(),
        chain_detail: None,
        chain_p2p_status: Some(super::WebChainP2pStatus {
            requested_user_mode: "auto_join".to_string(),
            recommended_user_mode: "public_entry".to_string(),
            effective_user_mode: "private_safe".to_string(),
            applied_effective_user_mode: Some("private_safe".to_string()),
            requires_explicit_public_entry_confirmation: true,
            detected_reachability: Some("public".to_string()),
            hole_punch_viability: "viable".to_string(),
            relay_available: false,
            probe_stable: true,
            deployment_mode: "private".to_string(),
            node_role_claim: "validator_core".to_string(),
            rationale: vec![
                "observed_reachability=public".to_string(),
                "public entry confirmation pending".to_string(),
            ],
        }),
        chain_observability_status: Some(super::WebChainNodeObservabilityStatus {
            status: "warn".to_string(),
            summary: "network committed height is ahead by 2".to_string(),
            connected_peer_count: 1,
            active_peer_count: 1,
            candidate_peer_count: 0,
            suspect_peer_count: 0,
            blocked_peer_count: 0,
            peer_with_issues_count: 0,
            known_peer_heads: 1,
            network_height_lag: 2,
            recent_replication_error_count: 0,
            storage_degraded: false,
            reward_runtime_degraded: false,
            alerts: vec![super::WebChainNodeObservabilityAlert {
                severity: "warn".to_string(),
                code: "consensus_network_lag".to_string(),
                summary: "network committed height is ahead by 2".to_string(),
            }],
        }),
        chain_replication_status: Some(super::WebChainReplicationStatus {
            local_peer_id: "peer-local".to_string(),
            connected_peers: vec!["peer-a".to_string()],
            peer_healths: vec![super::WebChainReplicationPeerHealth {
                peer_id: "peer-a".to_string(),
                status: "active".to_string(),
                issues: Vec::new(),
                discovery_sources: vec!["bootstrap".to_string()],
                active_path_kind: Some("direct".to_string()),
                source_operator: None,
                source_asn: None,
            }],
        }),
        chain_recovery: None,
        game_url: "http://127.0.0.1:4173/".to_string(),
        config: app.config.clone(),
        logs: vec![],
    };

    app.apply_web_snapshot(snapshot);
    let status = app
        .chain_p2p_status
        .clone()
        .expect("p2p status should exist");
    assert_eq!(status.recommended_user_mode, "public_entry");
    assert!(status.requires_explicit_public_entry_confirmation);
    let observability = app
        .chain_observability_status
        .clone()
        .expect("observability status should exist");
    assert_eq!(observability.status, "warn");
    assert_eq!(observability.connected_peer_count, 1);
    assert_eq!(observability.network_height_lag, 2);
    let replication = app
        .chain_replication_status
        .clone()
        .expect("replication status should exist");
    assert_eq!(replication.local_peer_id, "peer-local");
    assert_eq!(replication.connected_peers, vec!["peer-a".to_string()]);
}
