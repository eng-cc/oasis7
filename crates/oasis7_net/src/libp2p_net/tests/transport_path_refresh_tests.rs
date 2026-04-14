use super::*;
use libp2p::swarm::ConnectionId;

use crate::libp2p_net::transport_paths::recompute_active_transport_path_for_peer;

#[test]
fn libp2p_network_generates_peer_id() {
    let network = Libp2pNetwork::new(Libp2pNetworkConfig::default());
    assert!(!network.peer_id().to_string().is_empty());
}

#[test]
fn refresh_active_transport_path_after_partial_close_promotes_remaining_known_path() {
    let peer_id = PeerId::random();
    let direct_addr = format!("/ip4/39.104.205.67/tcp/5612/p2p/{peer_id}")
        .parse()
        .expect("direct addr");
    let relay_addr = format!(
        "/dns4/relay.example/tcp/443/p2p/{}/p2p-circuit/p2p/{peer_id}",
        PeerId::random()
    )
    .parse()
    .expect("relay addr");
    let mut known = HashMap::new();
    known.insert(
        peer_id,
        vec![
            active_transport_path_from_endpoint(&HashMap::new(), peer_id, &direct_addr),
            active_transport_path_from_endpoint(&HashMap::new(), peer_id, &relay_addr),
        ],
    );
    let direct_connection = ConnectionId::new_unchecked(1);
    let relay_connection = ConnectionId::new_unchecked(2);
    let mut active_transport_paths = HashMap::new();
    let mut established_transport_paths = HashMap::from([
        (
            direct_connection,
            active_transport_path_from_endpoint(&known, peer_id, &direct_addr),
        ),
        (
            relay_connection,
            active_transport_path_from_endpoint(&known, peer_id, &relay_addr),
        ),
    ]);
    let initial = recompute_active_transport_path_for_peer(
        &mut active_transport_paths,
        &established_transport_paths,
        &HashMap::from([(
            peer_id,
            HashSet::from([direct_connection, relay_connection]),
        )]),
        peer_id,
    )
    .expect("initial active path");
    assert_eq!(initial.addr, direct_addr);

    established_transport_paths.remove(&direct_connection);
    let refreshed = recompute_active_transport_path_for_peer(
        &mut active_transport_paths,
        &established_transport_paths,
        &HashMap::from([(peer_id, HashSet::from([relay_connection]))]),
        peer_id,
    )
    .expect("replacement path");

    assert_eq!(refreshed.addr, relay_addr);
    assert_eq!(
        active_transport_paths
            .get(&peer_id)
            .expect("active path after refresh")
            .addr,
        relay_addr
    );
}
