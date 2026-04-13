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
pub enum LiveAutoNatStatus {
    Unknown,
    Public,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LivePublicPortReachability {
    Unknown,
    Reachable,
    Unreachable,
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
    pub autonat_status: LiveAutoNatStatus,
    pub public_port_reachability: LivePublicPortReachability,
    pub observed_public_addr: Option<String>,
    pub confirmed_external_direct_addrs: Vec<String>,
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
            autonat_status: LiveAutoNatStatus::Unknown,
            public_port_reachability: LivePublicPortReachability::Unknown,
            observed_public_addr: None,
            confirmed_external_direct_addrs: Vec::new(),
        }
    }
}

impl Libp2pReachabilitySnapshot {
    pub fn has_live_signal(&self) -> bool {
        self.relay_reservation_active
            || !matches!(self.hole_punch_state, LiveHolePunchState::Unknown)
            || !matches!(self.autonat_status, LiveAutoNatStatus::Unknown)
            || !matches!(
                self.public_port_reachability,
                LivePublicPortReachability::Unknown
            )
            || self.active_transport_kind.is_some()
    }

    pub fn has_stable_signal(&self) -> bool {
        self.relay_reservation_active
            || matches!(self.hole_punch_state, LiveHolePunchState::Viable)
            || !matches!(self.autonat_status, LiveAutoNatStatus::Unknown)
            || !matches!(
                self.public_port_reachability,
                LivePublicPortReachability::Unknown
            )
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

pub(super) fn note_autonat_status(
    shared: &Arc<Mutex<Libp2pReachabilitySnapshot>>,
    status: LiveAutoNatStatus,
    observed_public_addr: Option<&Multiaddr>,
) {
    let mut snapshot = shared.lock().expect("lock reachability snapshot");
    snapshot.autonat_status = status;
    snapshot.observed_public_addr = observed_public_addr.map(ToString::to_string);
    recompute_public_port_reachability(&mut snapshot);
}

pub(super) fn note_external_addr_confirmed(
    shared: &Arc<Mutex<Libp2pReachabilitySnapshot>>,
    address: &Multiaddr,
) {
    if !is_public_direct_addr(address) {
        return;
    }
    let mut snapshot = shared.lock().expect("lock reachability snapshot");
    let label = address.to_string();
    if !snapshot.confirmed_external_direct_addrs.contains(&label) {
        snapshot.confirmed_external_direct_addrs.push(label);
        snapshot.confirmed_external_direct_addrs.sort();
    }
    recompute_public_port_reachability(&mut snapshot);
}

pub(super) fn note_external_addr_expired(
    shared: &Arc<Mutex<Libp2pReachabilitySnapshot>>,
    address: &Multiaddr,
) {
    let mut snapshot = shared.lock().expect("lock reachability snapshot");
    let label = address.to_string();
    snapshot
        .confirmed_external_direct_addrs
        .retain(|candidate| candidate != &label);
    recompute_public_port_reachability(&mut snapshot);
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

pub(super) fn is_relay_addr(addr: &Multiaddr) -> bool {
    addr.iter()
        .any(|protocol| matches!(protocol, Protocol::P2pCircuit))
}

pub(super) fn is_public_direct_addr(addr: &Multiaddr) -> bool {
    if is_relay_addr(addr) {
        return false;
    }
    addr.iter().any(|protocol| match protocol {
        Protocol::Ip4(ip) => {
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_broadcast()
                && !ip.is_documentation()
                && !ip.is_unspecified()
        }
        Protocol::Ip6(ip) => {
            !ip.is_loopback()
                && !ip.is_unspecified()
                && !ip.is_unicast_link_local()
                && !ip.is_unique_local()
        }
        Protocol::Dns(_) | Protocol::Dns4(_) | Protocol::Dns6(_) | Protocol::Dnsaddr(_) => true,
        _ => false,
    })
}

fn is_loopback_direct_addr(addr: &Multiaddr) -> bool {
    if is_relay_addr(addr) {
        return false;
    }
    addr.iter().any(|protocol| match protocol {
        Protocol::Ip4(ip) => ip.is_loopback(),
        Protocol::Ip6(ip) => ip.is_loopback(),
        _ => false,
    })
}

pub(super) fn should_register_external_listen_addr(
    addr: &Multiaddr,
    allow_loopback_external_addrs_for_testing: bool,
) -> bool {
    is_relay_addr(addr)
        || (allow_loopback_external_addrs_for_testing && is_loopback_direct_addr(addr))
}

fn recompute_public_port_reachability(snapshot: &mut Libp2pReachabilitySnapshot) {
    snapshot.public_port_reachability = if !snapshot.confirmed_external_direct_addrs.is_empty() {
        LivePublicPortReachability::Reachable
    } else {
        match snapshot.autonat_status {
            LiveAutoNatStatus::Unknown => LivePublicPortReachability::Unknown,
            LiveAutoNatStatus::Public => LivePublicPortReachability::Reachable,
            LiveAutoNatStatus::Private => LivePublicPortReachability::Unreachable,
        }
    };
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

    #[test]
    fn autonat_public_status_marks_public_port_reachable() {
        let shared = Arc::new(Mutex::new(Libp2pReachabilitySnapshot::default()));
        let public_addr: Multiaddr = "/dns4/public.example/tcp/4001"
            .parse()
            .expect("public addr");

        note_autonat_status(&shared, LiveAutoNatStatus::Public, Some(&public_addr));

        let snapshot = snapshot_clone(&shared);
        assert_eq!(snapshot.autonat_status, LiveAutoNatStatus::Public);
        assert_eq!(
            snapshot.public_port_reachability,
            LivePublicPortReachability::Reachable
        );
        assert_eq!(
            snapshot.observed_public_addr.as_deref(),
            Some("/dns4/public.example/tcp/4001")
        );
    }

    #[test]
    fn external_addr_confirmed_tracks_direct_public_port_reachability() {
        let shared = Arc::new(Mutex::new(Libp2pReachabilitySnapshot::default()));
        let public_addr: Multiaddr = "/dns4/public.example/tcp/443".parse().expect("public addr");

        note_external_addr_confirmed(&shared, &public_addr);
        assert_eq!(
            snapshot_clone(&shared).public_port_reachability,
            LivePublicPortReachability::Reachable
        );

        note_external_addr_expired(&shared, &public_addr);
        assert_eq!(
            snapshot_clone(&shared).public_port_reachability,
            LivePublicPortReachability::Unknown
        );
    }

    #[test]
    fn external_listen_addr_registration_only_keeps_relay_addrs_by_default() {
        let relay_addr: Multiaddr = format!(
            "/dns4/relay.example/tcp/443/p2p/{}/p2p-circuit",
            PeerId::random()
        )
        .parse()
        .expect("relay addr");
        let loopback_addr: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().expect("loopback addr");
        let unspecified_addr: Multiaddr =
            "/ip4/0.0.0.0/tcp/4001".parse().expect("unspecified addr");
        let public_direct_addr: Multiaddr = "/dns4/node.example/tcp/4001"
            .parse()
            .expect("public direct addr");

        assert!(should_register_external_listen_addr(&relay_addr, false));
        assert!(!should_register_external_listen_addr(&loopback_addr, false));
        assert!(!should_register_external_listen_addr(
            &unspecified_addr,
            false
        ));
        assert!(!should_register_external_listen_addr(
            &public_direct_addr,
            false
        ));
    }

    #[test]
    fn external_listen_addr_registration_allows_loopback_only_in_test_mode() {
        let loopback_addr: Multiaddr = "/ip4/127.0.0.1/tcp/4001".parse().expect("loopback addr");
        let unspecified_addr: Multiaddr =
            "/ip4/0.0.0.0/tcp/4001".parse().expect("unspecified addr");

        assert!(should_register_external_listen_addr(&loopback_addr, true));
        assert!(!should_register_external_listen_addr(
            &unspecified_addr,
            true
        ));
    }
}
