use super::*;

fn test_peer_health(
    peer_id: PeerId,
    status: PeerManagerHealthStatus,
    issues: Vec<PeerManagerHealthIssue>,
    active_path_kind: Option<&str>,
) -> PeerManagerPeerHealth {
    PeerManagerPeerHealth {
        peer_id: peer_id.to_string(),
        status,
        issues,
        discovery_sources: Vec::new(),
        active_path_kind: active_path_kind.map(str::to_string),
        source_operator: None,
        source_asn: None,
    }
}

#[test]
fn admissible_request_peers_prefers_non_blocked_connected_peer_without_health_snapshot_clone() {
    let network = Libp2pNetwork::new(Libp2pNetworkConfig::default());
    let blocked_peer = PeerId::random();
    let active_peer = PeerId::random();

    {
        let mut connected_peers = network
            .connected_peers
            .lock()
            .expect("lock connected peers");
        connected_peers.insert(blocked_peer);
        connected_peers.insert(active_peer);
    }
    {
        let mut peer_healths = network.peer_healths.lock().expect("lock peer healths");
        peer_healths.insert(
            blocked_peer.to_string(),
            test_peer_health(
                blocked_peer,
                PeerManagerHealthStatus::Blocked,
                vec![PeerManagerHealthIssue::RelayBudgetExceeded {
                    relayed_active_peers: 2,
                    active_peer_count: 2,
                    limit_per_mille: 500,
                }],
                Some("direct"),
            ),
        );
        peer_healths.insert(
            active_peer.to_string(),
            test_peer_health(
                active_peer,
                PeerManagerHealthStatus::Active,
                Vec::new(),
                Some("direct"),
            ),
        );
    }

    assert_eq!(network.admissible_request_peers(), vec![active_peer]);
}

#[test]
fn admissible_request_peers_falls_back_to_active_transport_peer_when_connected_snapshot_is_empty() {
    let network = Libp2pNetwork::new(Libp2pNetworkConfig::default());
    let bootstrap_peer = PeerId::random();

    {
        let mut peer_healths = network.peer_healths.lock().expect("lock peer healths");
        peer_healths.insert(
            bootstrap_peer.to_string(),
            test_peer_health(
                bootstrap_peer,
                PeerManagerHealthStatus::Blocked,
                vec![
                    PeerManagerHealthIssue::MissingPeerRecord,
                    PeerManagerHealthIssue::InsufficientActiveDiscoverySources {
                        observed_sources: 1,
                        required_sources: 2,
                    },
                ],
                Some("direct"),
            ),
        );
    }

    assert_eq!(network.admissible_request_peers(), vec![bootstrap_peer]);
}

#[test]
fn admissible_request_peers_returns_empty_when_connected_peers_are_hard_blocked() {
    let network = Libp2pNetwork::new(Libp2pNetworkConfig::default());
    let blocked_connected_peer = PeerId::random();
    let healthy_active_peer = PeerId::random();

    {
        let mut connected_peers = network
            .connected_peers
            .lock()
            .expect("lock connected peers");
        connected_peers.insert(blocked_connected_peer);
    }
    {
        let mut peer_healths = network.peer_healths.lock().expect("lock peer healths");
        peer_healths.insert(
            blocked_connected_peer.to_string(),
            test_peer_health(
                blocked_connected_peer,
                PeerManagerHealthStatus::Blocked,
                vec![PeerManagerHealthIssue::RelayBudgetExceeded {
                    relayed_active_peers: 2,
                    active_peer_count: 2,
                    limit_per_mille: 500,
                }],
                Some("direct"),
            ),
        );
        peer_healths.insert(
            healthy_active_peer.to_string(),
            test_peer_health(
                healthy_active_peer,
                PeerManagerHealthStatus::Active,
                Vec::new(),
                Some("direct"),
            ),
        );
    }

    assert!(network.admissible_request_peers().is_empty());
}

#[test]
fn admissible_request_peers_prefers_soft_deprioritized_connected_peer_over_active_fallback() {
    let network = Libp2pNetwork::new(Libp2pNetworkConfig::default());
    let connected_soft_peer = PeerId::random();
    let active_fallback_peer = PeerId::random();

    {
        let mut connected_peers = network
            .connected_peers
            .lock()
            .expect("lock connected peers");
        connected_peers.insert(connected_soft_peer);
    }
    {
        let mut peer_healths = network.peer_healths.lock().expect("lock peer healths");
        peer_healths.insert(
            connected_soft_peer.to_string(),
            test_peer_health(
                connected_soft_peer,
                PeerManagerHealthStatus::Blocked,
                vec![
                    PeerManagerHealthIssue::MissingPeerRecord,
                    PeerManagerHealthIssue::InsufficientActiveDiscoverySources {
                        observed_sources: 1,
                        required_sources: 2,
                    },
                ],
                Some("direct"),
            ),
        );
        peer_healths.insert(
            active_fallback_peer.to_string(),
            test_peer_health(
                active_fallback_peer,
                PeerManagerHealthStatus::Active,
                Vec::new(),
                Some("direct"),
            ),
        );
    }

    assert_eq!(
        network.admissible_request_peers(),
        vec![connected_soft_peer]
    );
}
