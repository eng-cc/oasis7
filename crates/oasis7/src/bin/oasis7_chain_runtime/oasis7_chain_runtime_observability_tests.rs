use super::*;
use oasis7::runtime::ReleaseSecurityPolicy;
use oasis7_node::{
    Libp2pReachabilitySnapshot, LiveAutoNatStatus, LivePublicPortReachability, NodeAutoNatStatus,
    NodeConsensusSnapshot, NodeHolePunchViability, NodeNetworkPolicy, NodePublicPortReachability,
    NodeReachabilityAutoDetection, NodeRole, NodeSnapshot, NodeUserMode,
};
use oasis7_proto::distributed_dht::{PeerDeploymentMode, PeerNodeRole, PeerReachabilityClass};
use oasis7_proto::storage_profile::{StorageProfile, StorageProfileConfig};
use std::collections::BTreeMap;
use std::path::Path;

#[test]
fn build_chain_status_payload_includes_storage_metrics() {
    let mut consensus = NodeConsensusSnapshot::default();
    consensus.committed_height = 5;
    consensus.network_committed_height = 7;
    consensus.known_peer_heads = 1;
    let snapshot = NodeSnapshot {
        node_id: "node-a".to_string(),
        player_id: "player-a".to_string(),
        world_id: "live-a".to_string(),
        role: NodeRole::Storage,
        running: true,
        tick_count: 42,
        last_tick_unix_ms: Some(1_700_000_000_000),
        consensus,
        last_error: None,
    };
    let reward_runtime = super::reward_runtime_worker::RewardRuntimeMetricsSnapshot {
        enabled: true,
        metrics_available: true,
        report_dir: "/tmp/reports".to_string(),
        report_count: 2,
        latest_epoch_index: 1,
        latest_report_observed_at_unix_ms: 1_700_000_000_000,
        latest_total_distributed_points: 10,
        latest_minted_record_count: 1,
        cumulative_minted_record_count: 1,
        distfs_total_checks: 0,
        distfs_failed_checks: 0,
        distfs_failure_ratio: 0.0,
        settlement_apply_attempts_total: 0,
        settlement_apply_failures_total: 0,
        settlement_apply_failure_ratio: 0.0,
        invariant_ok: true,
        last_error: None,
    };
    let storage = super::storage_metrics::StorageMetricsSnapshot {
        storage_profile: "dev_local".to_string(),
        effective_budget: StorageProfileConfig::from(StorageProfile::DevLocal),
        bytes_by_dir: BTreeMap::from([("runtime_root".to_string(), 128)]),
        blob_counts: BTreeMap::from([("execution_store_blobs".to_string(), 2)]),
        ref_count: 5,
        pin_count: 3,
        retained_heights: vec![1, 2],
        checkpoint_count: 1,
        replay_summary: super::storage_metrics::StorageReplaySummary {
            retained_height_count: 2,
            earliest_retained_height: Some(1),
            latest_retained_height: Some(2),
            earliest_checkpoint_height: Some(2),
            latest_checkpoint_height: Some(2),
            mode: "checkpoint_plus_log".to_string(),
        },
        orphan_blob_count: 0,
        last_gc_at_ms: Some(1_700_000_000_000),
        last_gc_result: "failed".to_string(),
        last_gc_error: Some("gc failed".to_string()),
        degraded_reason: Some("storage degraded".to_string()),
    };
    let replication = super::ChainReplicationDebugStatus {
        local_peer_id: "peer-local".to_string(),
        connected_peers: vec!["peer-a".to_string()],
        peer_healths: vec![
            super::ChainPeerHealthStatus {
                peer_id: "peer-a".to_string(),
                status: "active".to_string(),
                issues: Vec::new(),
                discovery_sources: vec!["bootstrap".to_string()],
                active_path_kind: Some("direct".to_string()),
                source_operator: None,
                source_asn: None,
            },
            super::ChainPeerHealthStatus {
                peer_id: "peer-b".to_string(),
                status: "suspect".to_string(),
                issues: vec!["missing_peer_record".to_string()],
                discovery_sources: vec!["dht".to_string()],
                active_path_kind: None,
                source_operator: None,
                source_asn: None,
            },
        ],
        registered_protocols: vec!["/oasis7/fetch-commit/1".to_string()],
        protocol_retry_cooldown_peers: BTreeMap::new(),
        recent_errors: vec!["request failed: Timeout".to_string()],
    };

    let payload = build_chain_status_payload(
        snapshot,
        Path::new("/tmp/execution-world"),
        &NodeNetworkPolicy::recommend_for_user_mode(
            NodeRole::Storage,
            NodeUserMode::AutoJoin,
            NodeReachabilityAutoDetection {
                observed_reachability: Some(PeerReachabilityClass::Public),
                hole_punch_viability: NodeHolePunchViability::Viable,
                relay_available: true,
                probe_stable: true,
                autonat_status: NodeAutoNatStatus::Public,
                public_port_reachability: NodePublicPortReachability::Reachable,
            },
            false,
        )
        .expect("recommendation"),
        Some("private_safe".to_string()),
        NodeNetworkPolicy {
            deployment_mode: PeerDeploymentMode::Private,
            node_role_claim: PeerNodeRole::FullStorage,
        },
        &Libp2pReachabilitySnapshot {
            autonat_status: LiveAutoNatStatus::Public,
            public_port_reachability: LivePublicPortReachability::Reachable,
            observed_public_addr: Some("/dns4/public.example/tcp/4001".to_string()),
            confirmed_external_direct_addrs: vec!["/dns4/public.example/tcp/4001".to_string()],
            ..Libp2pReachabilitySnapshot::default()
        },
        NodeReachabilityAutoDetection {
            observed_reachability: Some(PeerReachabilityClass::Public),
            hole_punch_viability: NodeHolePunchViability::Viable,
            relay_available: true,
            probe_stable: true,
            autonat_status: NodeAutoNatStatus::Public,
            public_port_reachability: NodePublicPortReachability::Reachable,
        },
        ReleaseSecurityPolicy::default(),
        reward_runtime,
        storage.clone(),
        super::ChainTrafficStatus {
            udp_gossip: None,
            libp2p_replication: oasis7_node::Libp2pTrafficMetricsSnapshot::default(),
        },
        replication,
    );

    assert_eq!(payload.storage.storage_profile, "dev_local");
    assert_eq!(payload.storage.ref_count, 5);
    assert_eq!(payload.storage.pin_count, 3);
    assert_eq!(payload.storage.checkpoint_count, 1);
    assert_eq!(
        payload.storage.effective_budget.profile,
        StorageProfile::DevLocal
    );
    assert_eq!(payload.storage.replay_summary.mode, "checkpoint_plus_log");
    assert_eq!(
        payload.storage.replay_summary.latest_retained_height,
        Some(2)
    );
    assert_eq!(payload.storage.last_gc_result, "failed");
    assert_eq!(payload.storage.last_gc_error.as_deref(), Some("gc failed"));
    assert_eq!(
        payload.storage.degraded_reason.as_deref(),
        Some("storage degraded")
    );
    assert_eq!(payload.p2p.requested_user_mode, "auto_join");
    assert_eq!(payload.p2p.recommended_user_mode, "public_entry");
    assert_eq!(payload.p2p.effective_user_mode, "private_safe");
    assert_eq!(
        payload.p2p.applied_effective_user_mode.as_deref(),
        Some("private_safe")
    );
    assert!(payload.p2p.requires_explicit_public_entry_confirmation);
    assert_eq!(payload.p2p.detected_reachability.as_deref(), Some("public"));
    assert_eq!(payload.p2p.autonat_status, "public");
    assert_eq!(payload.p2p.public_port_reachability, "reachable");
    assert_eq!(
        payload.p2p.observed_public_addr.as_deref(),
        Some("/dns4/public.example/tcp/4001")
    );
    assert_eq!(
        payload.p2p.confirmed_external_direct_addrs,
        vec!["/dns4/public.example/tcp/4001".to_string()]
    );
    assert_eq!(
        payload.release_security_policy,
        ReleaseSecurityPolicy::default()
    );
    assert_eq!(
        payload.traffic.libp2p_replication.scope,
        "application_payload_only"
    );
    assert!(payload.traffic.udp_gossip.is_none());
    assert_eq!(payload.observability.status, "warn");
    assert_eq!(payload.observability.connected_peer_count, 1);
    assert_eq!(payload.observability.active_peer_count, 1);
    assert_eq!(payload.observability.suspect_peer_count, 1);
    assert_eq!(payload.observability.peer_with_issues_count, 1);
    assert_eq!(payload.observability.known_peer_heads, 1);
    assert_eq!(payload.observability.network_height_lag, 2);
    assert_eq!(payload.observability.recent_replication_error_count, 1);
    assert!(payload.observability.storage_degraded);
    assert!(!payload.observability.reward_runtime_degraded);
    assert!(payload
        .observability
        .alerts
        .iter()
        .any(|alert| alert.code == "consensus_network_lag"));
    assert!(payload
        .observability
        .alerts
        .iter()
        .any(|alert| alert.code == "replication_peer_health_degraded"));
    assert!(payload
        .observability
        .alerts
        .iter()
        .any(|alert| alert.code == "replication_recent_errors"));
    assert!(payload
        .observability
        .alerts
        .iter()
        .any(|alert| alert.code == "storage_degraded"));
}
