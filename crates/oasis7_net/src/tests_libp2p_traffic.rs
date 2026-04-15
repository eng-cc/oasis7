use std::time::{Duration, Instant};

use libp2p::Multiaddr;
use oasis7_proto::distributed_dht::{
    PeerDeploymentMode, PeerDiscoverySource, PeerNodeRole, PeerReachabilityClass, PeerRecord,
};
use oasis7_proto::distributed_net::DistributedNetwork as _;

use super::{Libp2pNetwork, Libp2pNetworkConfig, PeerManagerPolicy, WorldError};

fn wait_until(what: &str, deadline: Instant, mut condition: impl FnMut() -> bool) {
    while Instant::now() < deadline {
        if condition() {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("timed out waiting for condition: {what}");
}

#[test]
fn libp2p_traffic_metrics_track_requests_and_gossip_payloads() {
    let peer_manager_policy = PeerManagerPolicy {
        min_active_discovery_sources: 0,
        min_peer_discovery_sources: 0,
        max_ipv4_subnet_share_per_mille: 1_000,
        block_ipv4_subnet_share_per_mille: 1_000,
        max_relay_domain_share_per_mille: 1_000,
        block_relay_domain_share_per_mille: 1_000,
        max_operator_share_per_mille: 1_000,
        block_operator_share_per_mille: 1_000,
        max_asn_share_per_mille: 1_000,
        block_asn_share_per_mille: 1_000,
        max_relayed_active_peer_share_per_mille: 1_000,
    };
    let peer_record = PeerRecord {
        peer_id: String::new(),
        node_id: "traffic-node".to_string(),
        world_id: "traffic-world".to_string(),
        network_id: "traffic-network".to_string(),
        node_role: PeerNodeRole::FullStorage.as_str().to_string(),
        deployment_mode: PeerDeploymentMode::Private,
        reachability_class: PeerReachabilityClass::Private,
        direct_addrs: Vec::new(),
        hole_punch_addrs: Vec::new(),
        relay_addrs: Vec::new(),
        discovery_sources: vec![PeerDiscoverySource::Dht],
        capability_lanes: PeerNodeRole::FullStorage.default_capability_lanes(),
        source_operator: None,
        source_asn: None,
        published_at_ms: 1,
        ttl_ms: 60_000,
    };

    let listen_addr: Multiaddr = "/ip4/127.0.0.1/tcp/0".parse().expect("multiaddr");
    let net1 = Libp2pNetwork::new(Libp2pNetworkConfig {
        listen_addrs: vec![listen_addr],
        peer_record: Some(peer_record.clone()),
        peer_manager_policy: peer_manager_policy.clone(),
        ..Libp2pNetworkConfig::default()
    });

    let deadline = Instant::now() + Duration::from_secs(10);
    wait_until("net1 listening", deadline, || {
        !net1.listening_addrs().is_empty()
    });
    let dial_addr = net1
        .listening_addrs()
        .into_iter()
        .find(|addr| addr.to_string().contains("127.0.0.1"))
        .expect("net1 addr")
        .with(libp2p::multiaddr::Protocol::P2p(net1.peer_id().into()));

    net1.register_handler(
        "/aw/rr/1.0.0/ping",
        Box::new(|payload| {
            let mut out = payload.to_vec();
            out.extend_from_slice(b"-ok");
            Ok(out)
        }),
    )
    .expect("register");

    let net2 = Libp2pNetwork::new(Libp2pNetworkConfig {
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listen")],
        bootstrap_peers: vec![dial_addr],
        peer_record: Some(peer_record),
        peer_manager_policy,
        ..Libp2pNetworkConfig::default()
    });

    wait_until("net2 connected", deadline, || {
        !net2.connected_peers().is_empty()
    });
    wait_until("request converges", deadline, || {
        match net2.request("/aw/rr/1.0.0/ping", b"ping") {
            Ok(reply) => reply == b"ping-ok".to_vec(),
            Err(WorldError::NetworkProtocolUnavailable { .. }) => false,
            Err(err) => panic!("unexpected request error: {err:?}"),
        }
    });

    let sub2 = net2.subscribe("aw.traffic").expect("sub2");
    let _sub1 = net1.subscribe("aw.traffic").expect("sub1");
    std::thread::sleep(Duration::from_millis(200));
    net1.publish("aw.traffic", b"hello").expect("publish");
    wait_until("gossipsub", deadline, || {
        sub2.drain().iter().any(|payload| payload == b"hello")
    });

    let net1_traffic = net1.traffic_metrics_snapshot();
    assert_eq!(net1_traffic.gossip.outbound.payload_bytes, 5);
    assert_eq!(
        net1_traffic
            .by_protocol
            .get("/aw/rr/1.0.0/ping")
            .map(|lane| (lane.inbound.payload_bytes, lane.outbound.payload_bytes)),
        Some((4, 7))
    );
    assert_eq!(
        net1_traffic
            .by_protocol
            .get("/aw/rr/1.0.0/ping")
            .map(|lane| (lane.inbound.messages, lane.outbound.messages)),
        Some((1, 1))
    );

    let net2_traffic = net2.traffic_metrics_snapshot();
    assert_eq!(
        net2_traffic
            .by_protocol
            .get("/aw/rr/1.0.0/ping")
            .map(|lane| (lane.outbound.payload_bytes, lane.inbound.payload_bytes)),
        Some((4, 7))
    );
    assert_eq!(
        net2_traffic
            .by_topic
            .get("aw.traffic")
            .map(|lane| lane.inbound.payload_bytes),
        Some(5)
    );
}
