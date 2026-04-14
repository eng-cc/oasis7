use std::time::{Duration, Instant};

use oasis7_proto::distributed::DistributedErrorCode;
use oasis7_proto::distributed_dht::{PeerDeploymentMode, PeerNodeRole, PeerReachabilityClass};

use super::*;

fn wait_until(what: &str, deadline: Instant, mut condition: impl FnMut() -> bool) {
    while Instant::now() < deadline {
        if condition() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("timed out waiting for condition: {what}");
}

fn listening_addr_with_peer_id(network: &Libp2pReplicationNetwork) -> Multiaddr {
    network
        .listening_addrs()
        .into_iter()
        .find(|addr| addr.to_string().contains("127.0.0.1"))
        .expect("listener visible addr")
        .with(libp2p::multiaddr::Protocol::P2p(network.peer_id().into()))
}

fn test_peer_record(node_id: &str) -> PeerRecord {
    PeerRecord {
        peer_id: String::new(),
        node_id: node_id.to_string(),
        world_id: "world-a".to_string(),
        network_id: "world-a".to_string(),
        node_role: PeerNodeRole::FullStorage.as_str().to_string(),
        deployment_mode: PeerDeploymentMode::Private,
        reachability_class: PeerReachabilityClass::Private,
        direct_addrs: Vec::new(),
        hole_punch_addrs: Vec::new(),
        relay_addrs: Vec::new(),
        discovery_sources: vec![
            PeerDiscoverySource::StaticBootstrap,
            PeerDiscoverySource::Dht,
        ],
        capability_lanes: PeerNodeRole::FullStorage.default_capability_lanes(),
        source_operator: None,
        source_asn: None,
        published_at_ms: 0,
        ttl_ms: 60_000,
    }
}

#[test]
fn connected_or_active_transport_peers_prefers_connected_snapshot() {
    let peer_a = PeerId::random();
    let peer_b = PeerId::random();
    let active_only = vec![ReplicationPeerHealthDebug {
        peer_id: PeerId::random().to_string(),
        status: "active".to_string(),
        issues: Vec::new(),
        discovery_sources: Vec::new(),
        active_path_kind: Some("direct".to_string()),
        source_operator: None,
        source_asn: None,
    }];

    let resolved =
        connected_or_active_transport_peers(vec![peer_b, peer_a], active_only.as_slice());

    let mut expected = vec![peer_a, peer_b];
    expected.sort_by_key(|peer| peer.to_string());
    assert_eq!(resolved, expected);
}

#[test]
fn connected_or_active_transport_peers_falls_back_to_active_health_peers() {
    let active_peer = PeerId::random();
    let candidate_peer = PeerId::random();
    let healths = vec![
        ReplicationPeerHealthDebug {
            peer_id: candidate_peer.to_string(),
            status: "candidate".to_string(),
            issues: Vec::new(),
            discovery_sources: vec!["dht".to_string()],
            active_path_kind: None,
            source_operator: None,
            source_asn: None,
        },
        ReplicationPeerHealthDebug {
            peer_id: active_peer.to_string(),
            status: "active".to_string(),
            issues: Vec::new(),
            discovery_sources: vec!["dht".to_string()],
            active_path_kind: Some("direct".to_string()),
            source_operator: None,
            source_asn: None,
        },
    ];

    let resolved = connected_or_active_transport_peers(Vec::new(), healths.as_slice());

    assert_eq!(resolved, vec![active_peer]);
}

#[test]
fn connected_or_active_transport_peers_excludes_blocked_peer() {
    let blocked_peer = PeerId::random();
    let active_peer = PeerId::random();
    let healths = vec![
        ReplicationPeerHealthDebug {
            peer_id: blocked_peer.to_string(),
            status: "blocked".to_string(),
            issues: vec!["relay_budget_exceeded relayed_active_peers=2 active_peer_count=2 limit_per_mille=500".to_string()],
            discovery_sources: vec!["dht".to_string()],
            active_path_kind: Some("direct".to_string()),
            source_operator: None,
            source_asn: None,
        },
        ReplicationPeerHealthDebug {
            peer_id: active_peer.to_string(),
            status: "active".to_string(),
            issues: Vec::new(),
            discovery_sources: vec!["dht".to_string()],
            active_path_kind: Some("direct".to_string()),
            source_operator: None,
            source_asn: None,
        },
    ];

    let resolved =
        connected_or_active_transport_peers(vec![blocked_peer, active_peer], healths.as_slice());

    assert_eq!(resolved, vec![active_peer]);
}

#[test]
fn connected_or_active_transport_peers_rejects_fully_blocked_connected_set() {
    let blocked_peer = PeerId::random();
    let healths = vec![ReplicationPeerHealthDebug {
        peer_id: blocked_peer.to_string(),
        status: "blocked".to_string(),
        issues: vec!["operator_concentration source_operator=op-a peers_in_bucket=2 active_peer_count=2 limit_per_mille=500".to_string()],
        discovery_sources: vec!["dht".to_string()],
        active_path_kind: Some("direct".to_string()),
        source_operator: None,
        source_asn: None,
    }];

    let resolved = connected_or_active_transport_peers(vec![blocked_peer], healths.as_slice());

    assert!(resolved.is_empty());
}

#[test]
fn connected_or_active_transport_peers_prefers_known_record_peer_over_missing_record_peer() {
    let blocked_peer = PeerId::random();
    let active_peer = PeerId::random();
    let healths = vec![
        ReplicationPeerHealthDebug {
            peer_id: blocked_peer.to_string(),
            status: "blocked".to_string(),
            issues: vec![
                "missing_peer_record".to_string(),
                "insufficient_active_discovery_sources observed=1 required=2".to_string(),
            ],
            discovery_sources: vec!["static_bootstrap".to_string()],
            active_path_kind: Some("direct".to_string()),
            source_operator: None,
            source_asn: None,
        },
        ReplicationPeerHealthDebug {
            peer_id: active_peer.to_string(),
            status: "active".to_string(),
            issues: Vec::new(),
            discovery_sources: vec!["dht".to_string()],
            active_path_kind: Some("direct".to_string()),
            source_operator: None,
            source_asn: None,
        },
    ];

    let resolved =
        connected_or_active_transport_peers(vec![blocked_peer, active_peer], healths.as_slice());

    assert_eq!(resolved, vec![active_peer]);
}

#[test]
fn connected_or_active_transport_peers_falls_back_to_missing_record_peer_when_needed() {
    let blocked_peer = PeerId::random();
    let healths = vec![ReplicationPeerHealthDebug {
        peer_id: blocked_peer.to_string(),
        status: "blocked".to_string(),
        issues: vec!["missing_peer_record".to_string()],
        discovery_sources: vec!["static_bootstrap".to_string()],
        active_path_kind: Some("direct".to_string()),
        source_operator: None,
        source_asn: None,
    }];

    let resolved = connected_or_active_transport_peers(vec![blocked_peer], healths.as_slice());

    assert_eq!(resolved, vec![blocked_peer]);
}

#[test]
fn connected_or_active_transport_peers_falls_back_to_bootstrap_peer_while_record_exchange_is_pending(
) {
    let bootstrap_peer = PeerId::random();
    let healths = vec![ReplicationPeerHealthDebug {
        peer_id: bootstrap_peer.to_string(),
        status: "blocked".to_string(),
        issues: vec![
            "missing_peer_record".to_string(),
            "insufficient_active_discovery_sources observed=1 required=2".to_string(),
        ],
        discovery_sources: vec!["static_bootstrap".to_string()],
        active_path_kind: Some("direct".to_string()),
        source_operator: None,
        source_asn: None,
    }];

    let resolved = connected_or_active_transport_peers(vec![bootstrap_peer], healths.as_slice());

    assert_eq!(resolved, vec![bootstrap_peer]);
}

#[test]
fn libp2p_replication_network_request_with_providers_honors_provider_subset() {
    let listener_fail = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener fail addr")],
        peer_record: Some(test_peer_record("listener-fail")),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let listener_ok = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener ok addr")],
        peer_record: Some(test_peer_record("listener-ok")),
        ..Libp2pReplicationNetworkConfig::default()
    });
    let listen_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("listener fail bind", listen_deadline, || {
        !listener_fail.listening_addrs().is_empty()
    });
    wait_until("listener ok bind", listen_deadline, || {
        !listener_ok.listening_addrs().is_empty()
    });

    listener_fail
        .register_handler(
            "/aw/node/replication/ping",
            Box::new(|_payload| {
                Err(WorldError::NetworkRequestFailed {
                    code: DistributedErrorCode::ErrUnsupported,
                    message: "forced failure".to_string(),
                    retryable: false,
                })
            }),
        )
        .expect("register listener fail handler");
    listener_ok
        .register_handler(
            "/aw/node/replication/ping",
            Box::new(|payload| {
                let mut out = payload.to_vec();
                out.extend_from_slice(b"-ok");
                Ok(out)
            }),
        )
        .expect("register listener ok handler");

    let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
        bootstrap_peers: vec![
            listening_addr_with_peer_id(&listener_fail),
            listening_addr_with_peer_id(&listener_ok),
        ],
        ..Libp2pReplicationNetworkConfig::default()
    });
    let connect_deadline = Instant::now() + Duration::from_secs(10);
    wait_until("dialer connects to two peers", connect_deadline, || {
        dialer.connected_peers().len() >= 2
    });

    let fail_only = dialer.request_with_providers(
        "/aw/node/replication/ping",
        b"node",
        &[listener_fail.peer_id().to_string()],
    );
    assert!(
        matches!(
            fail_only,
            Err(WorldError::NetworkRequestFailed { .. })
                | Err(WorldError::NetworkProtocolUnavailable { .. })
        ),
        "expected provider-restricted request to stay on failing peer, got {fail_only:?}"
    );

    let ok_only = dialer
        .request_with_providers(
            "/aw/node/replication/ping",
            b"node",
            &[listener_ok.peer_id().to_string()],
        )
        .expect("provider-restricted request should reach ok peer");
    assert_eq!(ok_only, b"node-ok".to_vec());
}
