use super::peer_record::{sign_peer_record, verify_signed_peer_record};
use super::transport_paths::{
    active_transport_path_from_endpoint, peer_record_transport_paths,
    select_preferred_transport_path, sync_known_transport_paths, TransportMuxer, TransportPathKind,
    TransportSecurity, TransportSessionFlavor,
};
use super::utils::push_bounded_vec;
use super::*;

#[test]
fn libp2p_network_generates_peer_id() {
    let network = Libp2pNetwork::new(Libp2pNetworkConfig::default());
    assert!(!network.peer_id().to_string().is_empty());
}

#[test]
fn dht_get_providers_collects_results() {
    let (sender, receiver) = oneshot::channel();
    let mut pending = PendingDhtQuery::GetProviders {
        response: Some(sender),
        providers: HashSet::new(),
        error: None,
    };
    let key_label = "providers".to_string();
    let key = RecordKey::new(&key_label);
    let mut providers = HashSet::new();
    providers.insert(PeerId::random());
    providers.insert(PeerId::random());
    let expected: HashSet<String> = providers.iter().map(|peer| peer.to_string()).collect();
    let result =
        kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders { key, providers }));
    handle_dht_progress(&mut pending, result, true);
    let records = futures::executor::block_on(receiver)
        .expect("oneshot")
        .expect("get providers");
    let actual: HashSet<String> = records
        .into_iter()
        .map(|record| record.provider_id)
        .collect();
    assert_eq!(actual, expected);
}

#[test]
fn dht_get_world_head_decodes_record() {
    let head = WorldHeadAnnounce {
        world_id: "w1".to_string(),
        height: 9,
        block_hash: "b1".to_string(),
        state_root: "s1".to_string(),
        timestamp_ms: 42,
        signature: "sig".to_string(),
    };
    let payload = to_canonical_cbor(&head).expect("encode head");
    let key_label = "head".to_string();
    let record = kad::Record {
        key: RecordKey::new(&key_label),
        value: payload,
        publisher: None,
        expires: None,
    };
    let peer_record = kad::PeerRecord { peer: None, record };
    let (sender, receiver) = oneshot::channel();
    let mut pending = PendingDhtQuery::GetWorldHead {
        response: Some(sender),
        head: None,
        error: None,
    };
    let result = kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(peer_record)));
    handle_dht_progress(&mut pending, result, true);
    let loaded = futures::executor::block_on(receiver)
        .expect("oneshot")
        .expect("get head");
    assert_eq!(loaded, Some(head));
}

#[test]
fn dht_get_membership_directory_decodes_record() {
    let snapshot = MembershipDirectorySnapshot {
        world_id: "w1".to_string(),
        requester_id: "seq-1".to_string(),
        requested_at_ms: 99,
        reason: Some("sync".to_string()),
        validators: vec!["seq-1".to_string(), "seq-2".to_string()],
        quorum_threshold: 2,
        signature_key_id: None,
        signature: None,
    };
    let payload = to_canonical_cbor(&snapshot).expect("encode snapshot");
    let key_label = "membership".to_string();
    let record = kad::Record {
        key: RecordKey::new(&key_label),
        value: payload,
        publisher: None,
        expires: None,
    };
    let peer_record = kad::PeerRecord { peer: None, record };
    let (sender, receiver) = oneshot::channel();
    let mut pending = PendingDhtQuery::GetMembershipDirectory {
        response: Some(sender),
        snapshot: None,
        error: None,
    };
    let result = kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(peer_record)));
    handle_dht_progress(&mut pending, result, true);

    let loaded = futures::executor::block_on(receiver)
        .expect("oneshot")
        .expect("get membership");
    assert_eq!(loaded, Some(snapshot));
}

#[test]
fn sign_and_verify_peer_record_round_trip() {
    let keypair = Keypair::generate_ed25519();
    let record = PeerRecord {
        peer_id: PeerId::from(keypair.public()).to_string(),
        node_id: "node-a".to_string(),
        world_id: "world-a".to_string(),
        network_id: "network-a".to_string(),
        node_role: "sequencer".to_string(),
        reachability_class: crate::dht::PeerReachabilityClass::Private,
        direct_addrs: vec!["/ip4/127.0.0.1/tcp/4101".to_string()],
        relay_addrs: Vec::new(),
        discovery_sources: vec![
            crate::dht::PeerDiscoverySource::StaticBootstrap,
            crate::dht::PeerDiscoverySource::Dht,
        ],
        published_at_ms: 42,
        ttl_ms: 60_000,
    };

    let signed = sign_peer_record(&record, &keypair).expect("sign peer record");
    verify_signed_peer_record(&signed).expect("verify peer record");
}

#[test]
fn dht_get_peer_record_decodes_and_verifies_record() {
    let keypair = Keypair::generate_ed25519();
    let signed = sign_peer_record(
        &PeerRecord {
            peer_id: PeerId::from(keypair.public()).to_string(),
            node_id: "node-a".to_string(),
            world_id: "world-a".to_string(),
            network_id: "network-a".to_string(),
            node_role: "storage".to_string(),
            reachability_class: crate::dht::PeerReachabilityClass::Hybrid,
            direct_addrs: vec!["/ip4/127.0.0.1/tcp/4102".to_string()],
            relay_addrs: vec!["/dns4/relay.example/tcp/443".to_string()],
            discovery_sources: vec![crate::dht::PeerDiscoverySource::Dht],
            published_at_ms: 77,
            ttl_ms: 60_000,
        },
        &keypair,
    )
    .expect("sign peer record");
    let payload = to_canonical_cbor(&signed).expect("encode peer record");
    let key_label = "peer-record".to_string();
    let record = kad::Record {
        key: RecordKey::new(&key_label),
        value: payload,
        publisher: None,
        expires: None,
    };
    let peer_record = kad::PeerRecord { peer: None, record };
    let (sender, receiver) = oneshot::channel();
    let mut pending = PendingDhtQuery::GetPeerRecord {
        response: Some(sender),
        record: None,
        error: None,
    };
    let result = kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(peer_record)));
    handle_dht_progress(&mut pending, result, true);

    let loaded = futures::executor::block_on(receiver)
        .expect("oneshot")
        .expect("get peer record");
    assert_eq!(loaded, Some(signed));
}

#[test]
fn republish_interval_gate() {
    assert!(!should_republish(100, 150, 100));
    assert!(should_republish(100, 200, 100));
    assert!(should_republish(100, 201, 100));
    assert!(!should_republish(100, 200, 0));
}

#[test]
fn push_bounded_vec_keeps_recent_window() {
    let mut values = vec![1_u64, 2, 3];
    push_bounded_vec(&mut values, 4, 3);
    assert_eq!(values, vec![2, 3, 4]);

    push_bounded_vec(&mut values, 5, 1);
    assert_eq!(values, vec![5]);
}

#[test]
fn try_send_command_reports_queue_disconnect() {
    let (sender, receiver) = mpsc::channel(1);
    drop(receiver);
    let err = try_send_command(
        &sender,
        Command::Subscribe {
            topic: "topic-a".to_string(),
        },
    )
    .expect_err("send must fail once receiver is dropped");
    assert!(matches!(
        err,
        WorldError::NetworkProtocolUnavailable { ref protocol }
            if protocol.contains("disconnected")
    ));
}

#[test]
fn peer_record_transport_paths_rank_quic_direct_before_tcp_and_relay() {
    let keypair = Keypair::generate_ed25519();
    let signed = sign_peer_record(
        &PeerRecord {
            peer_id: PeerId::from(keypair.public()).to_string(),
            node_id: "node-a".to_string(),
            world_id: "world-a".to_string(),
            network_id: "network-a".to_string(),
            node_role: "storage".to_string(),
            reachability_class: crate::dht::PeerReachabilityClass::Hybrid,
            direct_addrs: vec![
                "/ip4/127.0.0.1/tcp/4102".to_string(),
                "/ip4/127.0.0.1/udp/4103/quic-v1".to_string(),
                "/ip4/127.0.0.1/tcp/4102".to_string(),
            ],
            relay_addrs: vec!["/dns4/relay.example/tcp/443/p2p-circuit".to_string()],
            discovery_sources: vec![crate::dht::PeerDiscoverySource::Dht],
            published_at_ms: 77,
            ttl_ms: 60_000,
        },
        &keypair,
    )
    .expect("sign peer record");

    let paths = peer_record_transport_paths(&signed).expect("transport paths");
    assert_eq!(paths.len(), 3);
    assert_eq!(paths[0].kind, TransportPathKind::Direct);
    assert_eq!(paths[0].flavor, TransportSessionFlavor::Quic);
    assert_eq!(paths[0].security, TransportSecurity::QuicTls);
    assert_eq!(paths[0].muxer, TransportMuxer::Quic);
    assert_eq!(paths[1].kind, TransportPathKind::Direct);
    assert_eq!(paths[1].flavor, TransportSessionFlavor::TcpNoiseYamux);
    assert_eq!(paths[1].security, TransportSecurity::Noise);
    assert_eq!(paths[1].muxer, TransportMuxer::Yamux);
    assert_eq!(paths[2].kind, TransportPathKind::Relay);
    assert!(paths[0].addr.to_string().contains("/p2p/"));
}

#[test]
fn preferred_transport_path_skips_failed_quic_and_falls_back_to_tcp_before_relay() {
    let keypair = Keypair::generate_ed25519();
    let signed = sign_peer_record(
        &PeerRecord {
            peer_id: PeerId::from(keypair.public()).to_string(),
            node_id: "node-a".to_string(),
            world_id: "world-a".to_string(),
            network_id: "network-a".to_string(),
            node_role: "storage".to_string(),
            reachability_class: crate::dht::PeerReachabilityClass::Hybrid,
            direct_addrs: vec![
                "/ip4/127.0.0.1/udp/4103/quic-v1".to_string(),
                "/ip4/127.0.0.1/tcp/4102".to_string(),
            ],
            relay_addrs: vec!["/dns4/relay.example/tcp/443/p2p-circuit".to_string()],
            discovery_sources: vec![crate::dht::PeerDiscoverySource::Dht],
            published_at_ms: 77,
            ttl_ms: 60_000,
        },
        &keypair,
    )
    .expect("sign peer record");

    let paths = peer_record_transport_paths(&signed).expect("transport paths");
    let failed: HashSet<String> = [paths[0].label()].into_iter().collect();
    let selected =
        select_preferred_transport_path(paths.as_slice(), &failed).expect("fallback path");
    assert_eq!(selected.kind, TransportPathKind::Direct);
    assert_eq!(selected.flavor, TransportSessionFlavor::TcpNoiseYamux);
}

#[test]
fn sync_known_transport_paths_removes_stale_failed_labels() {
    let keypair = Keypair::generate_ed25519();
    let peer_id = PeerId::from(keypair.public());
    let signed = sign_peer_record(
        &PeerRecord {
            peer_id: peer_id.to_string(),
            node_id: "node-a".to_string(),
            world_id: "world-a".to_string(),
            network_id: "network-a".to_string(),
            node_role: "storage".to_string(),
            reachability_class: crate::dht::PeerReachabilityClass::Hybrid,
            direct_addrs: vec![
                "/ip4/127.0.0.1/udp/4103/quic-v1".to_string(),
                "/ip4/127.0.0.1/tcp/4102".to_string(),
            ],
            relay_addrs: vec!["/dns4/relay.example/tcp/443/p2p-circuit".to_string()],
            discovery_sources: vec![crate::dht::PeerDiscoverySource::Dht],
            published_at_ms: 77,
            ttl_ms: 60_000,
        },
        &keypair,
    )
    .expect("sign peer record");
    let initial_paths = peer_record_transport_paths(&signed).expect("transport paths");

    let mut known = HashMap::new();
    let mut failed: HashSet<String> = [initial_paths[2].label()].into_iter().collect();
    sync_known_transport_paths(&mut known, &mut failed, peer_id, initial_paths.clone());
    sync_known_transport_paths(
        &mut known,
        &mut failed,
        peer_id,
        initial_paths[..2].to_vec(),
    );
    assert!(failed.is_empty());
}

#[test]
fn active_transport_path_from_endpoint_infers_quic_and_relay_semantics() {
    let peer_id = PeerId::random();

    let quic_path = active_transport_path_from_endpoint(
        &HashMap::new(),
        peer_id,
        &"/ip4/127.0.0.1/udp/4103/quic-v1"
            .parse()
            .expect("quic endpoint"),
    );
    assert_eq!(quic_path.kind, TransportPathKind::Direct);
    assert_eq!(quic_path.flavor, TransportSessionFlavor::Quic);
    assert_eq!(quic_path.security, TransportSecurity::QuicTls);
    assert_eq!(quic_path.muxer, TransportMuxer::Quic);

    let relay_path = active_transport_path_from_endpoint(
        &HashMap::new(),
        peer_id,
        &format!(
            "/dns4/relay.example/tcp/443/p2p/{}/p2p-circuit",
            PeerId::random()
        )
        .parse()
        .expect("relay endpoint"),
    );
    assert_eq!(relay_path.kind, TransportPathKind::Relay);
    assert_eq!(relay_path.flavor, TransportSessionFlavor::TcpNoiseYamux);
    assert_eq!(relay_path.security, TransportSecurity::Noise);
    assert_eq!(relay_path.muxer, TransportMuxer::Yamux);
}
