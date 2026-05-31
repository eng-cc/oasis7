use std::collections::BTreeMap;

#[test]
fn classify_transport_stability_marks_repeated_transport_errors_unstable() {
    let stability =
        super::status_payload::classify_transport_stability(&super::ChainReplicationDebugStatus {
            local_peer_id: "peer-local".to_string(),
            connected_peers: vec!["peer-a".to_string()],
            peer_healths: Vec::new(),
            registered_protocols: Vec::new(),
            protocol_retry_cooldown_peers: BTreeMap::new(),
            transport_retry_cooldown_peers: Vec::new(),
            request_peer_scores: BTreeMap::new(),
            recent_errors: vec![
                "libp2p connection closed peer=peer-a".to_string(),
                "libp2p connection closed peer=peer-a".to_string(),
                "libp2p publish failed topic=x: InsufficientPeers".to_string(),
                "request failed: Timeout".to_string(),
                "request failed: Timeout".to_string(),
            ],
        });

    assert!(!stability.stable);
    assert!(stability.score < 70);
    assert_eq!(stability.connection_closed_count, 2);
    assert_eq!(stability.insufficient_peers_count, 1);
    assert_eq!(stability.timeout_count, 2);
}
