use super::*;

#[test]
fn filter_request_peers_by_health_prefers_non_suspect_peers() {
    let peer_a = PeerId::random();
    let peer_b = PeerId::random();
    let peer_c = PeerId::random();
    let peers = vec![peer_a, peer_b, peer_c];
    let healths = HashMap::from([
        (
            peer_a,
            PeerManagerPeerHealth {
                peer_id: peer_a.to_string(),
                status: PeerManagerHealthStatus::Suspect,
                issues: Vec::new(),
                discovery_sources: Vec::new(),
                active_path_kind: Some("relay_reserved".to_string()),
                source_operator: None,
                source_asn: None,
            },
        ),
        (
            peer_b,
            PeerManagerPeerHealth {
                peer_id: peer_b.to_string(),
                status: PeerManagerHealthStatus::Active,
                issues: Vec::new(),
                discovery_sources: Vec::new(),
                active_path_kind: Some("direct".to_string()),
                source_operator: None,
                source_asn: None,
            },
        ),
        (
            peer_c,
            PeerManagerPeerHealth {
                peer_id: peer_c.to_string(),
                status: PeerManagerHealthStatus::Candidate,
                issues: Vec::new(),
                discovery_sources: Vec::new(),
                active_path_kind: None,
                source_operator: None,
                source_asn: None,
            },
        ),
    ]);

    let filtered = filter_request_peers_by_health(peers, &healths);
    assert_eq!(filtered, vec![peer_b, peer_c, peer_a]);
}

#[test]
fn filter_request_peers_by_health_excludes_all_blocked_peers() {
    let peer_a = PeerId::random();
    let peer_b = PeerId::random();
    let peers = vec![peer_a, peer_b];
    let healths = HashMap::from([
        (
            peer_a,
            PeerManagerPeerHealth {
                peer_id: peer_a.to_string(),
                status: PeerManagerHealthStatus::Blocked,
                issues: Vec::new(),
                discovery_sources: Vec::new(),
                active_path_kind: None,
                source_operator: None,
                source_asn: None,
            },
        ),
        (
            peer_b,
            PeerManagerPeerHealth {
                peer_id: peer_b.to_string(),
                status: PeerManagerHealthStatus::Blocked,
                issues: Vec::new(),
                discovery_sources: Vec::new(),
                active_path_kind: None,
                source_operator: None,
                source_asn: None,
            },
        ),
    ]);

    let filtered = filter_request_peers_by_health(peers, &healths);
    assert!(filtered.is_empty());
}

#[test]
fn filter_request_peers_by_health_keeps_record_exchange_pending_blocked_peer_as_fallback() {
    let active_peer = PeerId::random();
    let soft_blocked_peer = PeerId::random();
    let hard_blocked_peer = PeerId::random();
    let peers = vec![soft_blocked_peer, hard_blocked_peer, active_peer];
    let healths = HashMap::from([
        (
            active_peer,
            PeerManagerPeerHealth {
                peer_id: active_peer.to_string(),
                status: PeerManagerHealthStatus::Active,
                issues: Vec::new(),
                discovery_sources: Vec::new(),
                active_path_kind: Some("direct".to_string()),
                source_operator: None,
                source_asn: None,
            },
        ),
        (
            soft_blocked_peer,
            PeerManagerPeerHealth {
                peer_id: soft_blocked_peer.to_string(),
                status: PeerManagerHealthStatus::Blocked,
                issues: vec![
                    PeerManagerHealthIssue::MissingPeerRecord,
                    PeerManagerHealthIssue::InsufficientActiveDiscoverySources {
                        observed_sources: 1,
                        required_sources: 2,
                    },
                ],
                discovery_sources: Vec::new(),
                active_path_kind: Some("direct".to_string()),
                source_operator: None,
                source_asn: None,
            },
        ),
        (
            hard_blocked_peer,
            PeerManagerPeerHealth {
                peer_id: hard_blocked_peer.to_string(),
                status: PeerManagerHealthStatus::Blocked,
                issues: vec![PeerManagerHealthIssue::RelayBudgetExceeded {
                    relayed_active_peers: 2,
                    active_peer_count: 2,
                    limit_per_mille: 500,
                }],
                discovery_sources: Vec::new(),
                active_path_kind: Some("relay_reserved".to_string()),
                source_operator: None,
                source_asn: None,
            },
        ),
    ]);

    let filtered = filter_request_peers_by_health(peers, &healths);
    assert_eq!(filtered, vec![active_peer, soft_blocked_peer]);
}
