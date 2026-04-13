use std::sync::{Arc, Mutex};

use libp2p::autonat;
use libp2p::swarm::Swarm;
use libp2p::Multiaddr;

use super::reachability::{
    is_public_direct_addr, note_autonat_status, note_external_addr_confirmed,
    note_external_addr_expired, sync_relay_reservation_from_listening_addrs,
    Libp2pReachabilitySnapshot, LiveAutoNatStatus,
};
use super::swarm_behaviour::Behaviour;
use super::utils::push_bounded_clone;

pub(super) fn handle_autonat_event(
    reachability: &Arc<Mutex<Libp2pReachabilitySnapshot>>,
    event: &autonat::Event,
) -> String {
    match event {
        autonat::Event::StatusChanged { old, new } => {
            let (status, observed_addr) = match new {
                autonat::NatStatus::Public(address) => (LiveAutoNatStatus::Public, Some(address)),
                autonat::NatStatus::Private => (LiveAutoNatStatus::Private, None),
                autonat::NatStatus::Unknown => (LiveAutoNatStatus::Unknown, None),
            };
            note_autonat_status(reachability, status, observed_addr);
            format!("libp2p autonat status changed old={old:?} new={new:?}")
        }
        other => format!("libp2p autonat event {other:?}"),
    }
}

pub(super) fn handle_external_addr_candidate(address: &Multiaddr) -> String {
    format!("libp2p external address candidate address={address}")
}

pub(super) fn handle_external_addr_confirmed(
    reachability: &Arc<Mutex<Libp2pReachabilitySnapshot>>,
    address: &Multiaddr,
) -> String {
    note_external_addr_confirmed(reachability, address);
    format!("libp2p external address confirmed address={address}")
}

pub(super) fn handle_external_addr_expired(
    reachability: &Arc<Mutex<Libp2pReachabilitySnapshot>>,
    address: &Multiaddr,
) -> String {
    note_external_addr_expired(reachability, address);
    format!("libp2p external address expired address={address}")
}

pub(super) fn handle_new_listen_addr(
    swarm: &mut Swarm<Behaviour>,
    listening_addrs_shared: &Arc<Mutex<Vec<Multiaddr>>>,
    reachability: &Arc<Mutex<Libp2pReachabilitySnapshot>>,
    address: &Multiaddr,
    max_listening_addrs: usize,
) {
    if !is_public_direct_addr(address) {
        swarm.add_external_address(address.clone());
    }
    push_bounded_clone(
        listening_addrs_shared,
        address.clone(),
        max_listening_addrs,
        "lock listening addrs",
    );
    let listening_addrs = listening_addrs_shared
        .lock()
        .expect("lock listening addrs")
        .clone();
    sync_relay_reservation_from_listening_addrs(reachability, listening_addrs.as_slice());
}

pub(super) fn handle_expired_listen_addr(
    swarm: &mut Swarm<Behaviour>,
    listening_addrs_shared: &Arc<Mutex<Vec<Multiaddr>>>,
    reachability: &Arc<Mutex<Libp2pReachabilitySnapshot>>,
    address: &Multiaddr,
) {
    if !is_public_direct_addr(address) {
        swarm.remove_external_address(address);
    }
    let mut listening_addrs = listening_addrs_shared.lock().expect("lock listening addrs");
    listening_addrs.retain(|candidate| candidate != address);
    sync_relay_reservation_from_listening_addrs(reachability, listening_addrs.as_slice());
}

pub(super) fn handle_listener_closed(
    swarm: &mut Swarm<Behaviour>,
    listening_addrs_shared: &Arc<Mutex<Vec<Multiaddr>>>,
    reachability: &Arc<Mutex<Libp2pReachabilitySnapshot>>,
    addresses: &[Multiaddr],
) {
    for address in addresses {
        if !is_public_direct_addr(address) {
            swarm.remove_external_address(address);
        }
    }
    let mut listening_addrs = listening_addrs_shared.lock().expect("lock listening addrs");
    listening_addrs.retain(|candidate| !addresses.iter().any(|addr| addr == candidate));
    sync_relay_reservation_from_listening_addrs(reachability, listening_addrs.as_slice());
}
