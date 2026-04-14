use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use libp2p::core::connection::ConnectedPoint;
use libp2p::rendezvous;
use libp2p::swarm::ConnectionId;
use libp2p::{Multiaddr, PeerId};
use oasis7_proto::distributed_dht::SignedPeerRecord;

use super::reachability::refresh_active_transport_snapshot;
use super::runtime_loop::{enforce_peer_manager_quarantine, refresh_peer_manager_healths};
use super::swarm_behaviour::Behaviour;
use super::transport_paths::{
    failover_transport_path, note_established_transport_path,
    recompute_active_transport_path_for_peer, TransportPath,
};
use super::utils::push_bounded_clone;
use super::{
    Libp2pReachabilitySnapshot, PeerManagerBlockArtifact, PeerManagerPeerHealth, PeerManagerPolicy,
};

pub(super) fn record_established_connection(
    known_transport_paths: &HashMap<PeerId, Vec<TransportPath>>,
    active_transport_paths: &mut HashMap<PeerId, TransportPath>,
    last_dialed_transport_paths: &mut HashMap<PeerId, TransportPath>,
    failed_transport_path_labels: &mut HashSet<String>,
    established_transport_paths_by_connection: &mut HashMap<ConnectionId, TransportPath>,
    established_connections_by_peer: &mut HashMap<PeerId, HashSet<ConnectionId>>,
    peer_id: PeerId,
    connection_id: ConnectionId,
    endpoint: &ConnectedPoint,
) -> (Option<TransportPath>, Option<Multiaddr>) {
    let (established_path, dialed_addr) = match endpoint {
        ConnectedPoint::Dialer { address, .. } => (
            note_established_transport_path(
                known_transport_paths,
                active_transport_paths,
                last_dialed_transport_paths,
                failed_transport_path_labels,
                peer_id,
                address,
            ),
            Some(address.clone()),
        ),
        ConnectedPoint::Listener { send_back_addr, .. } => (
            note_established_transport_path(
                known_transport_paths,
                active_transport_paths,
                last_dialed_transport_paths,
                failed_transport_path_labels,
                peer_id,
                send_back_addr,
            ),
            None,
        ),
    };
    established_transport_paths_by_connection.insert(connection_id, established_path);
    established_connections_by_peer
        .entry(peer_id)
        .or_default()
        .insert(connection_id);
    (
        recompute_active_transport_path_for_peer(
            active_transport_paths,
            established_transport_paths_by_connection,
            established_connections_by_peer,
            peer_id,
        ),
        dialed_addr,
    )
}

pub(super) fn refresh_active_path_after_connection_close(
    active_transport_paths: &mut HashMap<PeerId, TransportPath>,
    established_transport_paths_by_connection: &mut HashMap<ConnectionId, TransportPath>,
    established_connections_by_peer: &mut HashMap<PeerId, HashSet<ConnectionId>>,
    peer_id: PeerId,
    connection_id: ConnectionId,
) -> Option<TransportPath> {
    established_transport_paths_by_connection.remove(&connection_id);
    if let Some(connection_ids) = established_connections_by_peer.get_mut(&peer_id) {
        connection_ids.remove(&connection_id);
        if connection_ids.is_empty() {
            established_connections_by_peer.remove(&peer_id);
        }
    }
    recompute_active_transport_path_for_peer(
        active_transport_paths,
        established_transport_paths_by_connection,
        established_connections_by_peer,
        peer_id,
    )
}

pub(super) fn clear_disconnected_peer_state(
    peers: &mut Vec<PeerId>,
    admitted_active_peers: &mut HashSet<PeerId>,
    quarantined_active_peers: &mut HashSet<PeerId>,
    pending_quarantine_disconnects: &mut HashSet<PeerId>,
    active_transport_paths: &mut HashMap<PeerId, TransportPath>,
    last_dialed_transport_paths: &mut HashMap<PeerId, TransportPath>,
    pending_rendezvous_registers: &mut HashSet<PeerId>,
    pending_rendezvous_discovers: &mut HashSet<PeerId>,
    registered_rendezvous_nodes: &mut HashSet<PeerId>,
    rendezvous_cookies: &mut HashMap<PeerId, rendezvous::Cookie>,
    event_connected_peers: &Arc<Mutex<HashSet<PeerId>>>,
    peer_id: PeerId,
) -> bool {
    peers.retain(|peer| peer != &peer_id);
    admitted_active_peers.remove(&peer_id);
    let quarantined = quarantined_active_peers.remove(&peer_id)
        || pending_quarantine_disconnects.contains(&peer_id);
    pending_quarantine_disconnects.remove(&peer_id);
    if quarantined {
        active_transport_paths.remove(&peer_id);
        last_dialed_transport_paths.remove(&peer_id);
    }
    pending_rendezvous_registers.remove(&peer_id);
    pending_rendezvous_discovers.remove(&peer_id);
    registered_rendezvous_nodes.remove(&peer_id);
    rendezvous_cookies.remove(&peer_id);
    event_connected_peers
        .lock()
        .expect("lock connected peers")
        .remove(&peer_id);
    quarantined
}

pub(super) fn refresh_peer_manager_views(
    swarm: &mut libp2p::Swarm<Behaviour>,
    discovered_peer_records: &HashMap<PeerId, SignedPeerRecord>,
    active_transport_paths: &HashMap<PeerId, TransportPath>,
    admitted_active_peers: &HashSet<PeerId>,
    peer_manager_policy: &PeerManagerPolicy,
    event_peer_healths: &Arc<Mutex<HashMap<String, PeerManagerPeerHealth>>>,
    event_block_artifacts: &Arc<Mutex<HashMap<String, PeerManagerBlockArtifact>>>,
    pending_quarantine_disconnects: &mut HashSet<PeerId>,
    event_errors: &Arc<Mutex<Vec<String>>>,
    max_error_messages: usize,
    event_reachability: &Arc<Mutex<Libp2pReachabilitySnapshot>>,
) -> (
    HashMap<PeerId, PeerManagerPeerHealth>,
    HashSet<PeerId>,
    HashSet<PeerId>,
) {
    let (peer_healths_by_id, quarantined_active_peers, admitted_active_peers) =
        refresh_peer_manager_healths(
            discovered_peer_records,
            active_transport_paths,
            admitted_active_peers,
            peer_manager_policy,
            event_peer_healths,
            event_block_artifacts,
            event_errors,
            max_error_messages,
        );
    enforce_peer_manager_quarantine(
        swarm,
        &quarantined_active_peers,
        pending_quarantine_disconnects,
        event_errors,
        max_error_messages,
    );
    refresh_active_transport_snapshot(event_reachability, active_transport_paths);
    (
        peer_healths_by_id,
        quarantined_active_peers,
        admitted_active_peers,
    )
}

pub(super) fn failover_after_disconnect(
    swarm: &mut libp2p::Swarm<Behaviour>,
    known_transport_paths: &HashMap<PeerId, Vec<TransportPath>>,
    active_transport_paths: &mut HashMap<PeerId, TransportPath>,
    last_dialed_transport_paths: &mut HashMap<PeerId, TransportPath>,
    failed_transport_path_labels: &mut HashSet<String>,
    event_errors: &Arc<Mutex<Vec<String>>>,
    max_error_messages: usize,
    peer_id: PeerId,
) {
    match failover_transport_path(
        swarm,
        known_transport_paths,
        active_transport_paths,
        last_dialed_transport_paths,
        failed_transport_path_labels,
        peer_id,
    ) {
        Ok(Some((active_path, next_path))) => {
            push_bounded_clone(
                event_errors,
                format!(
                    "libp2p transport failover peer={peer_id} from={} to={}",
                    active_path.addr, next_path.addr,
                ),
                max_error_messages,
                "lock errors",
            );
        }
        Err(err) => {
            push_bounded_clone(
                event_errors,
                format!("libp2p transport failover dial failed peer={peer_id}: {err:?}"),
                max_error_messages,
                "lock errors",
            );
        }
        Ok(None) => {}
    }
}
