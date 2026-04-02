use super::peer_record::{
    build_configured_peer_record, sign_peer_record, verify_signed_peer_record,
};
use super::transport_paths::{
    active_transport_path_from_endpoint, peer_record_transport_paths,
    select_preferred_transport_path, sync_known_transport_paths, TransportMuxer, TransportPathKind,
    TransportSecurity, TransportSessionFlavor,
};
use super::utils::push_bounded_vec;
use super::*;
use oasis7_proto::distributed_dht::{PeerDeploymentMode, PeerNodeRole};
use oasis7_proto::distributed_net::NetworkLane;

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
        node_role: PeerNodeRole::ValidatorCore.as_str().to_string(),
        deployment_mode: PeerDeploymentMode::ValidatorHidden,
        reachability_class: crate::dht::PeerReachabilityClass::Private,
        direct_addrs: Vec::new(),
        hole_punch_addrs: Vec::new(),
        relay_addrs: Vec::new(),
        discovery_sources: vec![
            crate::dht::PeerDiscoverySource::StaticBootstrap,
            crate::dht::PeerDiscoverySource::Dht,
        ],
        capability_lanes: PeerNodeRole::ValidatorCore.default_capability_lanes(),
        published_at_ms: 42,
        ttl_ms: 60_000,
    };

    let signed = sign_peer_record(&record, &keypair).expect("sign peer record");
    verify_signed_peer_record(&signed).expect("verify peer record");
}

#[test]
fn build_configured_peer_record_splits_direct_and_relay_listener_addrs() {
    let keypair = Keypair::generate_ed25519();
    let listening_addrs = Arc::new(Mutex::new(vec![
        "/ip4/127.0.0.1/udp/4103/quic-v1"
            .parse()
            .expect("direct listen addr"),
        format!(
            "/dns4/relay.example/tcp/443/p2p/{}/p2p-circuit",
            PeerId::random()
        )
        .parse()
        .expect("relay listen addr"),
    ]));
    let signed = build_configured_peer_record(
        &keypair,
        &PeerRecord {
            peer_id: String::new(),
            node_id: "node-a".to_string(),
            world_id: "world-a".to_string(),
            network_id: "network-a".to_string(),
            node_role: PeerNodeRole::FullStorage.as_str().to_string(),
            deployment_mode: PeerDeploymentMode::Hybrid,
            reachability_class: crate::dht::PeerReachabilityClass::Hybrid,
            direct_addrs: Vec::new(),
            hole_punch_addrs: Vec::new(),
            relay_addrs: Vec::new(),
            discovery_sources: vec![crate::dht::PeerDiscoverySource::Dht],
            capability_lanes: PeerNodeRole::FullStorage.default_capability_lanes(),
            published_at_ms: 0,
            ttl_ms: 60_000,
        },
        &listening_addrs,
    )
    .expect("build peer record");
    assert_eq!(signed.record.direct_addrs.len(), 1);
    assert_eq!(signed.record.hole_punch_addrs, Vec::<String>::new());
    assert_eq!(signed.record.relay_addrs.len(), 1);
    assert!(signed.record.direct_addrs[0].contains("/quic-v1"));
    assert!(signed.record.relay_addrs[0].contains("/p2p-circuit"));
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
            node_role: PeerNodeRole::FullStorage.as_str().to_string(),
            deployment_mode: PeerDeploymentMode::Hybrid,
            reachability_class: crate::dht::PeerReachabilityClass::Hybrid,
            direct_addrs: vec!["/ip4/127.0.0.1/tcp/4102".to_string()],
            hole_punch_addrs: Vec::new(),
            relay_addrs: vec!["/dns4/relay.example/tcp/443".to_string()],
            discovery_sources: vec![crate::dht::PeerDiscoverySource::Dht],
            capability_lanes: PeerNodeRole::FullStorage.default_capability_lanes(),
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
fn filter_request_peers_by_lane_prefers_capable_peer_records() {
    let blob_peer_key = Keypair::generate_ed25519();
    let sync_only_peer_key = Keypair::generate_ed25519();
    let blob_peer_id = PeerId::from(blob_peer_key.public());
    let sync_only_peer_id = PeerId::from(sync_only_peer_key.public());
    let mut discovered = HashMap::new();
    discovered.insert(
        blob_peer_id,
        sign_peer_record(
            &PeerRecord {
                peer_id: blob_peer_id.to_string(),
                node_id: "blob-peer".to_string(),
                world_id: "world-a".to_string(),
                network_id: "network-a".to_string(),
                node_role: PeerNodeRole::FullStorage.as_str().to_string(),
                deployment_mode: PeerDeploymentMode::Hybrid,
                reachability_class: crate::dht::PeerReachabilityClass::Hybrid,
                direct_addrs: Vec::new(),
                hole_punch_addrs: Vec::new(),
                relay_addrs: Vec::new(),
                discovery_sources: vec![crate::dht::PeerDiscoverySource::Dht],
                capability_lanes: vec![NetworkLane::BlobState, NetworkLane::Control],
                published_at_ms: 1,
                ttl_ms: 60_000,
            },
            &blob_peer_key,
        )
        .expect("blob peer record"),
    );
    discovered.insert(
        sync_only_peer_id,
        sign_peer_record(
            &PeerRecord {
                peer_id: sync_only_peer_id.to_string(),
                node_id: "sync-peer".to_string(),
                world_id: "world-a".to_string(),
                network_id: "network-a".to_string(),
                node_role: PeerNodeRole::ValidatorCore.as_str().to_string(),
                deployment_mode: PeerDeploymentMode::Private,
                reachability_class: crate::dht::PeerReachabilityClass::Private,
                direct_addrs: Vec::new(),
                hole_punch_addrs: Vec::new(),
                relay_addrs: Vec::new(),
                discovery_sources: vec![crate::dht::PeerDiscoverySource::Dht],
                capability_lanes: vec![NetworkLane::Sync, NetworkLane::Control],
                published_at_ms: 1,
                ttl_ms: 60_000,
            },
            &sync_only_peer_key,
        )
        .expect("sync peer record"),
    );

    let filtered = filter_request_peers_by_lane(
        vec![sync_only_peer_id, blob_peer_id],
        "/aw/node/replication/fetch-blob/1.0.0",
        &discovered,
    );

    assert_eq!(filtered, vec![blob_peer_id]);
}

#[test]
fn peer_record_transport_paths_rank_direct_before_hole_punch_before_relay() {
    let keypair = Keypair::generate_ed25519();
    let signed = sign_peer_record(
        &PeerRecord {
            peer_id: PeerId::from(keypair.public()).to_string(),
            node_id: "node-a".to_string(),
            world_id: "world-a".to_string(),
            network_id: "network-a".to_string(),
            node_role: PeerNodeRole::FullStorage.as_str().to_string(),
            deployment_mode: PeerDeploymentMode::Hybrid,
            reachability_class: crate::dht::PeerReachabilityClass::Hybrid,
            direct_addrs: vec![
                "/ip4/127.0.0.1/tcp/4102".to_string(),
                "/ip4/127.0.0.1/udp/4103/quic-v1".to_string(),
                "/ip4/127.0.0.1/tcp/4102".to_string(),
            ],
            hole_punch_addrs: vec!["/ip4/127.0.0.1/udp/5103/quic-v1".to_string()],
            relay_addrs: vec!["/dns4/relay.example/tcp/443/p2p-circuit".to_string()],
            discovery_sources: vec![crate::dht::PeerDiscoverySource::Dht],
            capability_lanes: PeerNodeRole::FullStorage.default_capability_lanes(),
            published_at_ms: 77,
            ttl_ms: 60_000,
        },
        &keypair,
    )
    .expect("sign peer record");

    let paths = peer_record_transport_paths(&signed).expect("transport paths");
    assert_eq!(paths.len(), 4);
    assert_eq!(paths[0].kind, TransportPathKind::Direct);
    assert_eq!(paths[0].flavor, TransportSessionFlavor::Quic);
    assert_eq!(paths[0].security, TransportSecurity::QuicTls);
    assert_eq!(paths[0].muxer, TransportMuxer::Quic);
    assert_eq!(paths[1].kind, TransportPathKind::Direct);
    assert_eq!(paths[1].flavor, TransportSessionFlavor::TcpNoiseYamux);
    assert_eq!(paths[1].security, TransportSecurity::Noise);
    assert_eq!(paths[1].muxer, TransportMuxer::Yamux);
    assert_eq!(paths[2].kind, TransportPathKind::HolePunched);
    assert_eq!(paths[2].flavor, TransportSessionFlavor::Quic);
    assert_eq!(paths[3].kind, TransportPathKind::RelayReserved);
    assert_eq!(paths[3].flavor, TransportSessionFlavor::RelayTunnel);
    assert!(paths[0].addr.to_string().contains("/p2p/"));
}

#[test]
fn preferred_transport_path_skips_direct_and_falls_back_to_hole_punch_before_relay() {
    let keypair = Keypair::generate_ed25519();
    let signed = sign_peer_record(
        &PeerRecord {
            peer_id: PeerId::from(keypair.public()).to_string(),
            node_id: "node-a".to_string(),
            world_id: "world-a".to_string(),
            network_id: "network-a".to_string(),
            node_role: PeerNodeRole::FullStorage.as_str().to_string(),
            deployment_mode: PeerDeploymentMode::Hybrid,
            reachability_class: crate::dht::PeerReachabilityClass::Hybrid,
            direct_addrs: vec![
                "/ip4/127.0.0.1/udp/4103/quic-v1".to_string(),
                "/ip4/127.0.0.1/tcp/4102".to_string(),
            ],
            hole_punch_addrs: vec!["/ip4/127.0.0.1/udp/5103/quic-v1".to_string()],
            relay_addrs: vec!["/dns4/relay.example/tcp/443/p2p-circuit".to_string()],
            discovery_sources: vec![crate::dht::PeerDiscoverySource::Dht],
            capability_lanes: PeerNodeRole::FullStorage.default_capability_lanes(),
            published_at_ms: 77,
            ttl_ms: 60_000,
        },
        &keypair,
    )
    .expect("sign peer record");

    let paths = peer_record_transport_paths(&signed).expect("transport paths");
    let failed: HashSet<String> = [paths[0].label(), paths[1].label()].into_iter().collect();
    let selected =
        select_preferred_transport_path(paths.as_slice(), &failed).expect("fallback path");
    assert_eq!(selected.kind, TransportPathKind::HolePunched);
    assert_eq!(selected.flavor, TransportSessionFlavor::Quic);
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
            node_role: PeerNodeRole::FullStorage.as_str().to_string(),
            deployment_mode: PeerDeploymentMode::Hybrid,
            reachability_class: crate::dht::PeerReachabilityClass::Hybrid,
            direct_addrs: vec![
                "/ip4/127.0.0.1/udp/4103/quic-v1".to_string(),
                "/ip4/127.0.0.1/tcp/4102".to_string(),
            ],
            hole_punch_addrs: vec!["/ip4/127.0.0.1/udp/5103/quic-v1".to_string()],
            relay_addrs: vec!["/dns4/relay.example/tcp/443/p2p-circuit".to_string()],
            discovery_sources: vec![crate::dht::PeerDiscoverySource::Dht],
            capability_lanes: PeerNodeRole::FullStorage.default_capability_lanes(),
            published_at_ms: 77,
            ttl_ms: 60_000,
        },
        &keypair,
    )
    .expect("sign peer record");
    let initial_paths = peer_record_transport_paths(&signed).expect("transport paths");

    let mut known = HashMap::new();
    let mut failed: HashSet<String> = [initial_paths[3].label()].into_iter().collect();
    sync_known_transport_paths(&mut known, &mut failed, peer_id, initial_paths.clone());
    sync_known_transport_paths(
        &mut known,
        &mut failed,
        peer_id,
        initial_paths[..3].to_vec(),
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
    assert_eq!(relay_path.kind, TransportPathKind::RelayReserved);
    assert_eq!(relay_path.flavor, TransportSessionFlavor::RelayTunnel);
    assert_eq!(relay_path.security, TransportSecurity::Noise);
    assert_eq!(relay_path.muxer, TransportMuxer::Yamux);
}

#[test]
fn active_transport_path_from_endpoint_keeps_hole_punch_kind_when_known() {
    let peer_id = PeerId::random();
    let signed = sign_peer_record(
        &PeerRecord {
            peer_id: peer_id.to_string(),
            node_id: "node-a".to_string(),
            world_id: "world-a".to_string(),
            network_id: "network-a".to_string(),
            node_role: PeerNodeRole::FullStorage.as_str().to_string(),
            deployment_mode: PeerDeploymentMode::Hybrid,
            reachability_class: crate::dht::PeerReachabilityClass::Hybrid,
            direct_addrs: Vec::new(),
            hole_punch_addrs: vec!["/ip4/127.0.0.1/udp/5103/quic-v1".to_string()],
            relay_addrs: Vec::new(),
            discovery_sources: vec![crate::dht::PeerDiscoverySource::Dht],
            capability_lanes: PeerNodeRole::FullStorage.default_capability_lanes(),
            published_at_ms: 77,
            ttl_ms: 60_000,
        },
        &Keypair::generate_ed25519(),
    )
    .expect("sign peer record");
    let mut known = HashMap::new();
    known.insert(
        peer_id,
        peer_record_transport_paths(&signed).expect("transport paths"),
    );
    let hole_punched_path = active_transport_path_from_endpoint(
        &known,
        peer_id,
        &"/ip4/127.0.0.1/udp/5103/quic-v1"
            .parse()
            .expect("hole-punch endpoint"),
    );
    assert_eq!(hole_punched_path.kind, TransportPathKind::HolePunched);
    assert_eq!(hole_punched_path.flavor, TransportSessionFlavor::Quic);
}
