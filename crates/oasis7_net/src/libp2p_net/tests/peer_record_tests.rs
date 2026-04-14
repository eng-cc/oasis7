use super::super::discovery::peer_record_enables_rendezvous;
use super::super::peer_record::{build_configured_peer_record, verify_signed_peer_record};
use super::*;
use crate::util::to_canonical_cbor;
use libp2p::kad::RecordKey;
use oasis7_proto::distributed::dht_peer_record_key;

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
        source_operator: None,
        source_asn: None,
        published_at_ms: 42,
        ttl_ms: 60_000,
    };

    let signed = sign_peer_record(&record, &keypair).expect("sign peer record");
    verify_signed_peer_record(&signed).expect("verify peer record");
}

#[test]
fn build_configured_peer_record_excludes_loopback_direct_addrs_by_default() {
    let keypair = Keypair::generate_ed25519();
    let reachability = Arc::new(Mutex::new(Libp2pReachabilitySnapshot::default()));
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
            source_operator: None,
            source_asn: None,
            published_at_ms: 0,
            ttl_ms: 60_000,
        },
        &listening_addrs,
        &reachability,
        false,
    )
    .expect("build peer record");
    assert!(signed.record.direct_addrs.is_empty());
    assert_eq!(signed.record.hole_punch_addrs, Vec::<String>::new());
    assert_eq!(signed.record.relay_addrs.len(), 1);
    assert!(signed.record.relay_addrs[0].contains("/p2p-circuit"));
}

#[test]
fn build_configured_peer_record_keeps_private_validator_without_direct_addrs() {
    let keypair = Keypair::generate_ed25519();
    let reachability = Arc::new(Mutex::new(Libp2pReachabilitySnapshot::default()));
    let listening_addrs = Arc::new(Mutex::new(vec![
        "/ip4/127.0.0.1/tcp/4202"
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
            node_id: "node-private-validator".to_string(),
            world_id: "world-a".to_string(),
            network_id: "network-a".to_string(),
            node_role: PeerNodeRole::ValidatorCore.as_str().to_string(),
            deployment_mode: PeerDeploymentMode::Private,
            reachability_class: crate::dht::PeerReachabilityClass::Private,
            direct_addrs: Vec::new(),
            hole_punch_addrs: Vec::new(),
            relay_addrs: Vec::new(),
            discovery_sources: vec![
                crate::dht::PeerDiscoverySource::StaticBootstrap,
                crate::dht::PeerDiscoverySource::Dht,
            ],
            capability_lanes: PeerNodeRole::ValidatorCore.default_capability_lanes(),
            source_operator: None,
            source_asn: None,
            published_at_ms: 0,
            ttl_ms: 60_000,
        },
        &listening_addrs,
        &reachability,
        false,
    )
    .expect("build private validator peer record");

    assert!(signed.record.direct_addrs.is_empty());
    assert_eq!(signed.record.relay_addrs.len(), 1);
}

#[test]
fn build_configured_peer_record_allows_loopback_direct_addrs_for_testing() {
    let keypair = Keypair::generate_ed25519();
    let reachability = Arc::new(Mutex::new(Libp2pReachabilitySnapshot::default()));
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
            node_id: "node-loopback-test".to_string(),
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
            source_operator: None,
            source_asn: None,
            published_at_ms: 0,
            ttl_ms: 60_000,
        },
        &listening_addrs,
        &reachability,
        true,
    )
    .expect("build loopback testing peer record");

    assert_eq!(
        signed.record.direct_addrs,
        vec!["/ip4/127.0.0.1/udp/4103/quic-v1".to_string()]
    );
    assert_eq!(signed.record.relay_addrs.len(), 1);
}

#[test]
fn build_configured_peer_record_prefers_confirmed_external_direct_addrs() {
    let keypair = Keypair::generate_ed25519();
    let reachability = Arc::new(Mutex::new(Libp2pReachabilitySnapshot {
        confirmed_external_direct_addrs: vec!["/dns4/public.example/tcp/4103".to_string()],
        ..Libp2pReachabilitySnapshot::default()
    }));
    let listening_addrs = Arc::new(Mutex::new(vec![
        "/ip4/10.0.0.2/udp/4103/quic-v1"
            .parse()
            .expect("private listen addr"),
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
            node_id: "node-public".to_string(),
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
            source_operator: None,
            source_asn: None,
            published_at_ms: 0,
            ttl_ms: 60_000,
        },
        &listening_addrs,
        &reachability,
        false,
    )
    .expect("build public peer record");

    assert_eq!(
        signed.record.direct_addrs,
        vec!["/dns4/public.example/tcp/4103".to_string()]
    );
    assert_eq!(signed.record.relay_addrs.len(), 1);
}

#[test]
fn peer_record_enables_rendezvous_only_when_source_is_declared() {
    let without_rendezvous = PeerRecord {
        peer_id: String::new(),
        node_id: "node-no-rendezvous".to_string(),
        world_id: "world-a".to_string(),
        network_id: "network-a".to_string(),
        node_role: PeerNodeRole::FullStorage.as_str().to_string(),
        deployment_mode: PeerDeploymentMode::Private,
        reachability_class: crate::dht::PeerReachabilityClass::Private,
        direct_addrs: Vec::new(),
        hole_punch_addrs: Vec::new(),
        relay_addrs: Vec::new(),
        discovery_sources: vec![
            crate::dht::PeerDiscoverySource::StaticBootstrap,
            crate::dht::PeerDiscoverySource::Dht,
        ],
        capability_lanes: PeerNodeRole::FullStorage.default_capability_lanes(),
        source_operator: None,
        source_asn: None,
        published_at_ms: 0,
        ttl_ms: 60_000,
    };
    assert!(!peer_record_enables_rendezvous(&without_rendezvous));

    let with_rendezvous = PeerRecord {
        discovery_sources: vec![
            crate::dht::PeerDiscoverySource::StaticBootstrap,
            crate::dht::PeerDiscoverySource::Rendezvous,
        ],
        ..without_rendezvous
    };
    assert!(peer_record_enables_rendezvous(&with_rendezvous));
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
            source_operator: None,
            source_asn: None,
            published_at_ms: 77,
            ttl_ms: 60_000,
        },
        &keypair,
    )
    .expect("sign peer record");
    let payload = to_canonical_cbor(&signed).expect("encode peer record");
    let key = dht_peer_record_key("world-a", signed.record.peer_id.as_str());
    let mut pending = PendingDhtQuery::GetPeerRecord {
        response: None,
        record: None,
        error: None,
    };
    let result = kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(kad::PeerRecord {
        peer: None,
        record: kad::Record {
            key: RecordKey::new(&key),
            value: payload,
            publisher: None,
            expires: None,
        },
    })));
    handle_dht_progress(&mut pending, result, true);
    let PendingDhtQuery::GetPeerRecord { record, .. } = pending else {
        panic!("unexpected query state");
    };
    assert_eq!(record, Some(signed));
}
