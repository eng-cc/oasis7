use libp2p::Multiaddr;
use oasis7_proto::distributed_dht::PeerRecord;

use super::{Libp2pNetwork, Libp2pNetworkConfig};

pub(super) fn loopback_test_peer(
    bootstrap_peers: Vec<Multiaddr>,
    peer_record: PeerRecord,
) -> Libp2pNetwork {
    Libp2pNetwork::new(Libp2pNetworkConfig {
        allow_loopback_external_addrs_for_testing: true,
        listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listen")],
        bootstrap_peers,
        peer_record: Some(peer_record),
        discovery_query_interval_ms: 100,
        ..Libp2pNetworkConfig::default()
    })
}
