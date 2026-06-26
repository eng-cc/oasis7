use oasis7_node::{NodeConsensusSnapshot, NodePeerCommittedHead, NodeRole, NodeSnapshot};
use std::collections::BTreeMap;

#[test]
fn build_chain_status_payload_allows_genesis_self_head_cold_start() {
    let mut consensus = NodeConsensusSnapshot::default();
    consensus.committed_height = 0;
    consensus.network_committed_height = 0;
    consensus.replication_persisted_height = 0;
    consensus.known_peer_heads = 0;
    consensus.peer_heads = Vec::new();
    let snapshot = NodeSnapshot {
        node_id: "validator-genesis".to_string(),
        player_id: "player-genesis".to_string(),
        world_id: "world-public-testnet-genesis".to_string(),
        role: NodeRole::Sequencer,
        replication_enabled: true,
        running: true,
        tick_count: 1,
        last_tick_unix_ms: Some(1_700_000_000_000),
        consensus,
        last_error: None,
    };
    let observed_at_ms = 1_700_000_000_000;
    let network_head =
        super::status_payload::build_network_head_status(&snapshot, observed_at_ms, None);
    let policy = super::status_payload::readiness_policy(&snapshot, None);
    let observability = super::status_payload::build_chain_node_observability_status(
        &snapshot,
        &super::observability_tests::sample_observability_storage_metrics(),
        &super::observability_tests::sample_observability_reward_runtime_metrics(),
        &super::ChainReplicationDebugStatus {
            local_peer_id: "validator-genesis".to_string(),
            connected_peers: vec!["validator-peer".to_string()],
            peer_healths: vec![super::ChainPeerHealthStatus {
                peer_id: "validator-peer".to_string(),
                status: "active".to_string(),
                issues: Vec::new(),
                discovery_sources: vec!["static_bootstrap".to_string()],
                active_path_kind: Some("direct".to_string()),
                source_operator: Some("validator".to_string()),
                source_asn: None,
            }],
            registered_protocols: vec![
                "/aw/node/replication/fetch-blob/1.0.0".to_string(),
                "/aw/node/replication/fetch-commit/1.0.0".to_string(),
            ],
            protocol_retry_cooldown_peers: BTreeMap::new(),
            transport_retry_cooldown_peers: Vec::new(),
            request_peer_scores: BTreeMap::from([("validator-peer".to_string(), 100)]),
            connection_events: Vec::new(),
            recent_errors: vec![
                "libp2p incoming connection error: LocalPeerId { endpoint: Listener { local_addr: /ip4/172.26.53.91/tcp/6831, send_back_addr: /ip4/39.104.204.172/tcp/34306 } }".to_string(),
            ],
        },
        &network_head,
        &super::observability_tests::sample_observability_p2p_status(),
        &policy,
        observed_at_ms,
    );
    let readiness = super::status_payload::build_readiness_status(&observability, policy);

    assert_eq!(network_head.source, "self_only");
    assert_eq!(network_head.required_peer_count, 0);
    assert!(observability.network_head_available);
    assert!(observability.ready);
    assert_eq!(readiness.status, "ready");
    assert!(
        readiness.failed_gates.is_empty(),
        "{:?}",
        readiness.failed_gates
    );
    assert!(
        observability
            .alerts
            .iter()
            .all(|alert| alert.code != "consensus_peer_head_unavailable")
    );
    assert!(
        observability
            .alerts
            .iter()
            .all(|alert| alert.code != "replication_recent_errors")
    );
}

#[test]
fn build_chain_status_payload_allows_clean_genesis_cold_start() {
    let mut consensus = NodeConsensusSnapshot::default();
    consensus.committed_height = 0;
    consensus.network_committed_height = 0;
    consensus.replication_persisted_height = 0;
    consensus.known_peer_heads = 0;
    consensus.peer_heads = Vec::new();
    let snapshot = NodeSnapshot {
        node_id: "validator-a".to_string(),
        player_id: "player-validator-a".to_string(),
        world_id: "world-public-testnet".to_string(),
        role: NodeRole::Sequencer,
        replication_enabled: true,
        running: true,
        tick_count: 1,
        last_tick_unix_ms: None,
        consensus,
        last_error: None,
    };
    let observed_at_ms = 10_100;
    let network_head =
        super::status_payload::build_network_head_status(&snapshot, observed_at_ms, None);
    let policy = super::status_payload::readiness_policy(&snapshot, None);
    let observability = super::status_payload::build_chain_node_observability_status(
        &snapshot,
        &super::observability_tests::sample_observability_storage_metrics(),
        &super::observability_tests::sample_observability_reward_runtime_metrics(),
        &super::ChainReplicationDebugStatus {
            local_peer_id: "validator-a".to_string(),
            connected_peers: vec!["validator-b".to_string()],
            peer_healths: vec![super::ChainPeerHealthStatus {
                peer_id: "validator-b".to_string(),
                status: "active".to_string(),
                issues: Vec::new(),
                discovery_sources: vec!["static_bootstrap".to_string()],
                active_path_kind: Some("direct".to_string()),
                source_operator: Some("validator".to_string()),
                source_asn: None,
            }],
            registered_protocols: vec![
                "/aw/node/replication/fetch-blob/1.0.0".to_string(),
                "/aw/node/replication/fetch-commit/1.0.0".to_string(),
                "/aw/node/replication/fetch-commit/head/1.0.0".to_string(),
            ],
            protocol_retry_cooldown_peers: BTreeMap::new(),
            transport_retry_cooldown_peers: Vec::new(),
            request_peer_scores: BTreeMap::from([("validator-b".to_string(), 100)]),
            connection_events: Vec::new(),
            recent_errors: vec![
                "libp2p incoming connection error: LocalPeerId { endpoint: Listener { local_addr: /ip4/172.26.53.91/tcp/6831, send_back_addr: /ip4/39.104.204.172/tcp/34306 } }".to_string(),
                "libp2p connection established peer=validator-b direction=outbound addr=/ip4/39.104.205.67/tcp/6832/p2p/validator-b".to_string(),
            ],
        },
        &network_head,
        &super::observability_tests::sample_observability_p2p_status(),
        &policy,
        observed_at_ms,
    );
    let readiness = super::status_payload::build_readiness_status(&observability, policy);

    assert_eq!(network_head.required_peer_count, 0);
    assert_eq!(network_head.source, "self_only");
    assert_eq!(network_head.decision, "ready");
    assert!(observability.network_head_available);
    assert!(observability.ready);
    assert!(
        observability
            .alerts
            .iter()
            .all(|alert| alert.code != "consensus_peer_head_unavailable")
    );
    assert!(
        observability
            .alerts
            .iter()
            .all(|alert| alert.code != "replication_recent_errors")
    );
    assert_eq!(readiness.status, "ready");
}

#[test]
fn build_chain_status_payload_blocks_isolated_genesis_validator() {
    let mut consensus = NodeConsensusSnapshot::default();
    consensus.committed_height = 0;
    consensus.network_committed_height = 0;
    consensus.replication_persisted_height = 0;
    consensus.known_peer_heads = 0;
    consensus.peer_heads = Vec::new();
    let snapshot = NodeSnapshot {
        node_id: "validator-a".to_string(),
        player_id: "player-validator-a".to_string(),
        world_id: "world-public-testnet".to_string(),
        role: NodeRole::Sequencer,
        replication_enabled: true,
        running: true,
        tick_count: 1,
        last_tick_unix_ms: None,
        consensus,
        last_error: None,
    };
    let observed_at_ms = 10_100;
    let network_head =
        super::status_payload::build_network_head_status(&snapshot, observed_at_ms, None);
    let policy = super::status_payload::readiness_policy(&snapshot, None);
    let observability = super::status_payload::build_chain_node_observability_status(
        &snapshot,
        &super::observability_tests::sample_observability_storage_metrics(),
        &super::observability_tests::sample_observability_reward_runtime_metrics(),
        &super::ChainReplicationDebugStatus {
            local_peer_id: "validator-a".to_string(),
            connected_peers: Vec::new(),
            peer_healths: Vec::new(),
            registered_protocols: vec![
                "/aw/node/replication/fetch-blob/1.0.0".to_string(),
                "/aw/node/replication/fetch-commit/1.0.0".to_string(),
                "/aw/node/replication/fetch-commit/head/1.0.0".to_string(),
            ],
            protocol_retry_cooldown_peers: BTreeMap::new(),
            transport_retry_cooldown_peers: Vec::new(),
            request_peer_scores: BTreeMap::new(),
            connection_events: Vec::new(),
            recent_errors: Vec::new(),
        },
        &network_head,
        &super::observability_tests::sample_observability_p2p_status(),
        &policy,
        observed_at_ms,
    );
    let readiness = super::status_payload::build_readiness_status(&observability, policy);

    assert_eq!(network_head.source, "self_only");
    assert_eq!(network_head.decision, "ready");
    assert!(!observability.network_head_available);
    assert!(!observability.ready);
    assert_eq!(readiness.status, "not_ready");
    assert!(
        readiness
            .failed_gates
            .iter()
            .any(|gate| gate == "replication_no_connected_peers"),
        "{:?}",
        readiness.failed_gates
    );
}

#[test]
fn build_chain_status_payload_tolerates_noisy_external_peer_with_healthy_validator_path() {
    let mut consensus = NodeConsensusSnapshot::default();
    consensus.committed_height = 10;
    consensus.network_committed_height = 10;
    consensus.replication_persisted_height = 10;
    consensus.known_peer_heads = 1;
    consensus.last_committed_at_ms = Some(10_000);
    consensus.peer_heads = vec![NodePeerCommittedHead {
        node_id: "validator-b".to_string(),
        validator_id: None,
        height: 10,
        block_hash: "block-a".to_string(),
        committed_at_ms: 10_000,
        observed_at_ms: 10_050,
        execution_block_hash: Some("execution-a".to_string()),
        execution_state_root: Some("state-a".to_string()),
    }];
    let snapshot = NodeSnapshot {
        node_id: "validator-a".to_string(),
        player_id: "player-validator-a".to_string(),
        world_id: "world-public-testnet".to_string(),
        role: NodeRole::Sequencer,
        replication_enabled: true,
        running: true,
        tick_count: 1,
        last_tick_unix_ms: None,
        consensus,
        last_error: None,
    };
    let observed_at_ms = 10_100;
    let network_head =
        super::status_payload::build_network_head_status(&snapshot, observed_at_ms, None);
    let policy = super::status_payload::readiness_policy(&snapshot, None);
    let observability = super::status_payload::build_chain_node_observability_status(
        &snapshot,
        &super::observability_tests::sample_observability_storage_metrics(),
        &super::observability_tests::sample_observability_reward_runtime_metrics(),
        &super::ChainReplicationDebugStatus {
            local_peer_id: "validator-a".to_string(),
            connected_peers: vec!["validator-b".to_string(), "observer-noisy".to_string()],
            peer_healths: vec![
                super::ChainPeerHealthStatus {
                    peer_id: "validator-b".to_string(),
                    status: "active".to_string(),
                    issues: Vec::new(),
                    discovery_sources: vec!["static_bootstrap".to_string()],
                    active_path_kind: Some("direct".to_string()),
                    source_operator: Some("validator".to_string()),
                    source_asn: None,
                },
                super::ChainPeerHealthStatus {
                    peer_id: "observer-noisy".to_string(),
                    status: "active".to_string(),
                    issues: Vec::new(),
                    discovery_sources: vec!["dht".to_string()],
                    active_path_kind: Some("direct".to_string()),
                    source_operator: Some("observer".to_string()),
                    source_asn: None,
                },
            ],
            registered_protocols: vec![
                "/aw/node/replication/fetch-blob/1.0.0".to_string(),
                "/aw/node/replication/fetch-commit/1.0.0".to_string(),
            ],
            protocol_retry_cooldown_peers: BTreeMap::new(),
            transport_retry_cooldown_peers: Vec::new(),
            request_peer_scores: BTreeMap::from([
                ("validator-b".to_string(), 100),
                ("observer-noisy".to_string(), 0),
            ]),
            connection_events: Vec::new(),
            recent_errors: vec![
                "libp2p inbound failure from PeerId(\"observer-noisy\"): Timeout".to_string(),
                "libp2p inbound failure from PeerId(\"observer-noisy\"): Timeout".to_string(),
                "libp2p inbound failure from PeerId(\"observer-noisy\"): Timeout".to_string(),
                "libp2p inbound failure from PeerId(\"observer-noisy\"): Timeout".to_string(),
                "libp2p incoming connection error: Transport(Other(Custom { kind: Other, error: ProtocolError(InvalidMessage) }))".to_string(),
            ],
        },
        &network_head,
        &super::observability_tests::sample_observability_p2p_status(),
        &policy,
        observed_at_ms,
    );
    let readiness = super::status_payload::build_readiness_status(&observability, policy);

    assert!(observability.ready);
    assert!(observability.transport_stable);
    assert_eq!(observability.transport_stability_score, 100);
    assert_eq!(observability.recent_replication_error_count, 5);
    assert!(
        observability
            .alerts
            .iter()
            .all(|alert| alert.code != "replication_recent_errors")
    );
    assert!(
        observability
            .alerts
            .iter()
            .all(|alert| alert.code != "replication_transport_unstable")
    );
    assert_eq!(readiness.status, "ready");
    assert!(readiness.ready);
    assert!(!readiness.failed_gates.iter().any(
        |gate| gate == "replication_recent_errors" || gate == "replication_transport_unstable"
    ));
}
