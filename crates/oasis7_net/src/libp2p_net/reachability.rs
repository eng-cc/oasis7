use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use libp2p::multiaddr::Protocol;
use libp2p::{Multiaddr, PeerId};

use super::transport_paths::{TransportPath, TransportPathKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveHolePunchState {
    Unknown,
    Viable,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveTransportKind {
    Direct,
    HolePunched,
    RelayReserved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Libp2pReachabilitySnapshot {
    pub active_transport_kind: Option<LiveTransportKind>,
    pub active_direct_path_count: usize,
    pub active_hole_punch_path_count: usize,
    pub active_relay_path_count: usize,
    pub relay_reservation_active: bool,
    pub hole_punch_state: LiveHolePunchState,
}

impl Default for Libp2pReachabilitySnapshot {
    fn default() -> Self {
        Self {
            active_transport_kind: None,
            active_direct_path_count: 0,
            active_hole_punch_path_count: 0,
            active_relay_path_count: 0,
            relay_reservation_active: false,
            hole_punch_state: LiveHolePunchState::Unknown,
        }
    }
}

impl Libp2pReachabilitySnapshot {
    pub fn has_live_signal(&self) -> bool {
        self.relay_reservation_active
            || !matches!(self.hole_punch_state, LiveHolePunchState::Unknown)
            || self.active_transport_kind.is_some()
    }

    pub fn has_stable_signal(&self) -> bool {
        self.relay_reservation_active
            || matches!(self.hole_punch_state, LiveHolePunchState::Viable)
            || self.active_transport_kind.is_some()
    }
}

pub(super) fn snapshot_clone(
    shared: &Arc<Mutex<Libp2pReachabilitySnapshot>>,
) -> Libp2pReachabilitySnapshot {
    shared.lock().expect("lock reachability snapshot").clone()
}

pub(super) fn note_relay_reservation_accepted(shared: &Arc<Mutex<Libp2pReachabilitySnapshot>>) {
    let mut snapshot = shared.lock().expect("lock reachability snapshot");
    snapshot.relay_reservation_active = true;
    if snapshot.active_transport_kind.is_none() {
        snapshot.active_transport_kind = Some(LiveTransportKind::RelayReserved);
    }
}

pub(super) fn sync_relay_reservation_from_listening_addrs(
    shared: &Arc<Mutex<Libp2pReachabilitySnapshot>>,
    listening_addrs: &[Multiaddr],
) {
    let mut snapshot = shared.lock().expect("lock reachability snapshot");
    snapshot.relay_reservation_active = listening_addrs.iter().any(is_relay_addr);
    snapshot.active_transport_kind = preferred_transport_kind(&snapshot);
}

pub(super) fn note_hole_punch_result(
    shared: &Arc<Mutex<Libp2pReachabilitySnapshot>>,
    success: bool,
) {
    let mut snapshot = shared.lock().expect("lock reachability snapshot");
    if success {
        snapshot.hole_punch_state = LiveHolePunchState::Viable;
        snapshot.active_transport_kind = Some(LiveTransportKind::HolePunched);
    }
}

pub(super) fn refresh_active_transport_snapshot(
    shared: &Arc<Mutex<Libp2pReachabilitySnapshot>>,
    active_transport_paths: &HashMap<PeerId, TransportPath>,
) {
    let mut snapshot = shared.lock().expect("lock reachability snapshot");
    snapshot.active_direct_path_count = 0;
    snapshot.active_hole_punch_path_count = 0;
    snapshot.active_relay_path_count = 0;
    for path in active_transport_paths.values() {
        match path.kind {
            TransportPathKind::Direct => {
                snapshot.active_direct_path_count =
                    snapshot.active_direct_path_count.saturating_add(1);
            }
            TransportPathKind::HolePunched => {
                snapshot.active_hole_punch_path_count =
                    snapshot.active_hole_punch_path_count.saturating_add(1);
            }
            TransportPathKind::RelayReserved => {
                snapshot.active_relay_path_count =
                    snapshot.active_relay_path_count.saturating_add(1);
            }
        }
    }
    snapshot.active_transport_kind = preferred_transport_kind(&snapshot);
}

fn is_relay_addr(addr: &Multiaddr) -> bool {
    addr.iter()
        .any(|protocol| matches!(protocol, Protocol::P2pCircuit))
}

fn preferred_transport_kind(snapshot: &Libp2pReachabilitySnapshot) -> Option<LiveTransportKind> {
    if snapshot.active_hole_punch_path_count > 0 {
        Some(LiveTransportKind::HolePunched)
    } else if snapshot.active_relay_path_count > 0 || snapshot.relay_reservation_active {
        Some(LiveTransportKind::RelayReserved)
    } else if snapshot.active_direct_path_count > 0 {
        Some(LiveTransportKind::Direct)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::PeerId;

    fn path(kind: TransportPathKind) -> TransportPath {
        TransportPath {
            peer_id: PeerId::random(),
            addr: "/ip4/127.0.0.1/tcp/4001".parse().expect("transport addr"),
            kind,
            flavor: super::super::transport_paths::TransportSessionFlavor::TcpNoiseYamux,
            security: super::super::transport_paths::TransportSecurity::Noise,
            muxer: super::super::transport_paths::TransportMuxer::Yamux,
        }
    }

    #[test]
    fn refresh_active_transport_snapshot_prefers_hole_punch_then_relay_then_direct() {
        let shared = Arc::new(Mutex::new(Libp2pReachabilitySnapshot::default()));
        let mut active = HashMap::new();
        active.insert(PeerId::random(), path(TransportPathKind::Direct));
        active.insert(PeerId::random(), path(TransportPathKind::HolePunched));
        active.insert(PeerId::random(), path(TransportPathKind::RelayReserved));

        refresh_active_transport_snapshot(&shared, &active);
        let snapshot = snapshot_clone(&shared);

        assert_eq!(
            snapshot.active_transport_kind,
            Some(LiveTransportKind::HolePunched)
        );
        assert_eq!(snapshot.active_direct_path_count, 1);
        assert_eq!(snapshot.active_hole_punch_path_count, 1);
        assert_eq!(snapshot.active_relay_path_count, 1);
    }

    #[test]
    fn note_hole_punch_result_tracks_viable_and_blocked_states() {
        let shared = Arc::new(Mutex::new(Libp2pReachabilitySnapshot::default()));

        note_hole_punch_result(&shared, true);
        assert_eq!(
            snapshot_clone(&shared).hole_punch_state,
            LiveHolePunchState::Viable
        );

        note_hole_punch_result(&shared, false);
        assert_eq!(
            snapshot_clone(&shared).hole_punch_state,
            LiveHolePunchState::Viable
        );
    }

    #[test]
    fn failed_hole_punch_does_not_create_blocked_inference_from_unknown() {
        let shared = Arc::new(Mutex::new(Libp2pReachabilitySnapshot::default()));

        note_hole_punch_result(&shared, false);
        assert_eq!(
            snapshot_clone(&shared).hole_punch_state,
            LiveHolePunchState::Unknown
        );
    }

    #[test]
    fn sync_relay_reservation_follows_relay_listen_addrs() {
        let shared = Arc::new(Mutex::new(Libp2pReachabilitySnapshot::default()));
        let relay_addr: Multiaddr = format!(
            "/dns4/relay.example/tcp/443/p2p/{}/p2p-circuit",
            PeerId::random()
        )
        .parse()
        .expect("relay addr");
        let direct_addr: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().expect("direct addr");

        sync_relay_reservation_from_listening_addrs(&shared, &[direct_addr.clone(), relay_addr]);
        assert!(snapshot_clone(&shared).relay_reservation_active);
        assert_eq!(
            snapshot_clone(&shared).active_transport_kind,
            Some(LiveTransportKind::RelayReserved)
        );

        sync_relay_reservation_from_listening_addrs(&shared, &[direct_addr]);
        assert!(!snapshot_clone(&shared).relay_reservation_active);
        assert_eq!(snapshot_clone(&shared).active_transport_kind, None);
    }
}
