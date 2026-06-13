use super::*;
use oasis7_proto::distributed_dht::{
    PeerDeploymentMode, PeerDiscoverySource, PeerNodeRole, PeerReachabilityClass,
};
use std::time::{Duration, Instant};

fn wait_until(what: &str, deadline: Instant, mut condition: impl FnMut() -> bool) {
    while Instant::now() < deadline {
        if condition() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("timed out waiting for condition: {what}");
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

fn listening_addr_with_peer_id(network: &Libp2pReplicationNetwork) -> Multiaddr {
    network
        .listening_addrs()
        .into_iter()
        .find(|addr| addr.to_string().contains("127.0.0.1"))
        .expect("listener visible addr")
        .with(libp2p::multiaddr::Protocol::P2p(network.peer_id().into()))
}

#[test]
fn bootstrap_peer_addrs_by_peer_id_indexes_p2p_bootstrap_addrs() {
    let listener = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener addr")],
        peer_record: Some(test_peer_record("listener-bootstrap-index")),
        ..Libp2pReplicationNetworkConfig::default()
    });
    wait_until(
        "listener bind",
        Instant::now() + Duration::from_secs(10),
        || !listener.listening_addrs().is_empty(),
    );
    let addr = listening_addr_with_peer_id(&listener);

    let indexed = bootstrap_peer_addrs_by_peer_id(std::slice::from_ref(&addr));

    assert_eq!(indexed.get(&listener.peer_id()), Some(&addr));
}

#[test]
fn libp2p_replication_network_known_peers_includes_bootstrap_providers_before_connect() {
    let storage = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("storage addr")],
        peer_record: Some(test_peer_record("storage-bootstrap-known")),
        ..Libp2pReplicationNetworkConfig::default()
    });
    wait_until(
        "storage bind",
        Instant::now() + Duration::from_secs(10),
        || !storage.listening_addrs().is_empty(),
    );

    let observer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("observer addr")],
        bootstrap_peers: vec![listening_addr_with_peer_id(&storage)],
        ..Libp2pReplicationNetworkConfig::default()
    });

    assert!(
        observer
            .known_peer_ids()
            .iter()
            .any(|peer_id| peer_id == &storage.peer_id().to_string()),
        "bootstrap storage provider should be visible to gap-sync peer sweeps before it is connected"
    );
}
