use super::libp2p_test_helpers::loopback_test_peer;
use super::*;

#[test]
fn libp2p_discovery_acquires_peer_from_dht_peer_record() {
    use std::time::{Duration, Instant};

    use libp2p::Multiaddr;
    use oasis7_proto::distributed_dht::{
        PeerDeploymentMode, PeerDiscoverySource, PeerNodeRole, PeerReachabilityClass, PeerRecord,
    };

    fn wait_until(what: &str, deadline: Instant, mut condition: impl FnMut() -> bool) {
        while Instant::now() < deadline {
            if condition() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("timed out waiting for condition: {what}");
    }

    fn default_peer_record(node_id: &str) -> PeerRecord {
        PeerRecord {
            peer_id: String::new(),
            node_id: node_id.to_string(),
            world_id: "world-discovery".to_string(),
            network_id: "world-discovery".to_string(),
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

    let bootstrap = Libp2pNetwork::new(Libp2pNetworkConfig {
        allow_loopback_external_addrs_for_testing: true,
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listen")],
        peer_record: Some(default_peer_record("bootstrap")),
        discovery_query_interval_ms: 100,
        ..Libp2pNetworkConfig::default()
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    wait_until("bootstrap listening", deadline, || {
        !bootstrap.listening_addrs().is_empty()
    });
    let bootstrap_addr: Multiaddr = bootstrap
        .listening_addrs()
        .into_iter()
        .find(|addr| addr.to_string().contains("127.0.0.1"))
        .expect("bootstrap addr")
        .with(libp2p::multiaddr::Protocol::P2p(bootstrap.peer_id().into()));

    let publisher = loopback_test_peer(
        vec![bootstrap_addr.clone()],
        default_peer_record("publisher"),
    );
    wait_until("publisher connected bootstrap", deadline, || {
        publisher.connected_peers().contains(&bootstrap.peer_id())
    });

    let seeker = loopback_test_peer(vec![bootstrap_addr], default_peer_record("seeker"));
    wait_until("seeker connected bootstrap", deadline, || {
        seeker.connected_peers().contains(&bootstrap.peer_id())
    });

    let publisher_peer_id = publisher.peer_id();
    let discovery_deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < discovery_deadline {
        if let Ok(Some(record)) =
            seeker.get_peer_record("world-discovery", publisher_peer_id.to_string().as_str())
        {
            assert_eq!(record.record.peer_id, publisher_peer_id.to_string());
            assert!(record
                .record
                .discovery_sources
                .iter()
                .any(|source| matches!(source, PeerDiscoverySource::Dht)));
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!(
        "timed out waiting for seeker reads publisher peer record from dht; seeker_peers={:?}; seeker_errors={:?}; seeker_healths={:?}; publisher_peers={:?}; publisher_errors={:?}; bootstrap_errors={:?}",
        seeker.connected_peers(),
        seeker.debug_errors(),
        seeker.debug_peer_healths(),
        publisher.connected_peers(),
        publisher.debug_errors(),
        bootstrap.debug_errors(),
    );
}

#[test]
fn libp2p_rendezvous_discovery_acquires_peer_from_bootstrap_registration() {
    use std::time::{Duration, Instant};

    use libp2p::Multiaddr;
    use oasis7_proto::distributed_dht::{
        PeerDeploymentMode, PeerDiscoverySource, PeerNodeRole, PeerReachabilityClass, PeerRecord,
    };

    fn wait_until(what: &str, deadline: Instant, mut condition: impl FnMut() -> bool) {
        while Instant::now() < deadline {
            if condition() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("timed out waiting for condition: {what}");
    }

    fn rendezvous_peer_record(node_id: &str) -> PeerRecord {
        PeerRecord {
            peer_id: String::new(),
            node_id: node_id.to_string(),
            world_id: "world-rendezvous".to_string(),
            network_id: "world-rendezvous".to_string(),
            node_role: PeerNodeRole::FullStorage.as_str().to_string(),
            deployment_mode: PeerDeploymentMode::Private,
            reachability_class: PeerReachabilityClass::Private,
            direct_addrs: Vec::new(),
            hole_punch_addrs: Vec::new(),
            relay_addrs: Vec::new(),
            discovery_sources: vec![
                PeerDiscoverySource::StaticBootstrap,
                PeerDiscoverySource::Rendezvous,
            ],
            capability_lanes: PeerNodeRole::FullStorage.default_capability_lanes(),
            source_operator: None,
            source_asn: None,
            published_at_ms: 0,
            ttl_ms: 60_000,
        }
    }

    let bootstrap = Libp2pNetwork::new(Libp2pNetworkConfig {
        enable_rendezvous: true,
        allow_loopback_external_addrs_for_testing: true,
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listen")],
        peer_record: Some(rendezvous_peer_record("bootstrap")),
        discovery_query_interval_ms: 100,
        ..Libp2pNetworkConfig::default()
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    wait_until("bootstrap listening", deadline, || {
        !bootstrap.listening_addrs().is_empty()
    });
    let bootstrap_addr: Multiaddr = bootstrap
        .listening_addrs()
        .into_iter()
        .find(|addr| addr.to_string().contains("127.0.0.1"))
        .expect("bootstrap addr")
        .with(libp2p::multiaddr::Protocol::P2p(bootstrap.peer_id().into()));

    let seeker = loopback_test_peer(
        vec![bootstrap_addr.clone()],
        rendezvous_peer_record("seeker"),
    );
    wait_until("seeker connected bootstrap", deadline, || {
        seeker.connected_peers().contains(&bootstrap.peer_id())
    });

    let publisher = loopback_test_peer(vec![bootstrap_addr], rendezvous_peer_record("publisher"));
    wait_until("publisher connected bootstrap", deadline, || {
        publisher.connected_peers().contains(&bootstrap.peer_id())
    });

    let discovery_deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < discovery_deadline {
        let seeker_errors = seeker.debug_errors();
        if seeker_errors
            .iter()
            .any(|line| line.contains("libp2p rendezvous discovered registrations"))
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!(
        "timed out waiting for seeker rendezvous discovery event; seeker_peers={:?}; seeker_errors={:?}; seeker_healths={:?}; publisher_peers={:?}; publisher_errors={:?}; bootstrap_errors={:?}",
        seeker.connected_peers(),
        seeker.debug_errors(),
        seeker.debug_peer_healths(),
        publisher.connected_peers(),
        publisher.debug_errors(),
        bootstrap.debug_errors(),
    );
}
