use std::collections::{HashMap, HashSet};

use libp2p::multiaddr::Protocol;
use libp2p::swarm::Swarm;
use libp2p::{Multiaddr, PeerId};

use crate::error::WorldError;
use oasis7_proto::distributed_dht::SignedPeerRecord;

use super::swarm_behaviour::{
    dial_addr_with_optional_peer_id, ensure_peer_id, split_peer_id, Behaviour,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum TransportPathKind {
    Direct,
    HolePunched,
    RelayReserved,
}

impl TransportPathKind {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::HolePunched => "hole_punched",
            Self::RelayReserved => "relay_reserved",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum TransportSessionFlavor {
    Quic,
    TcpNoiseYamux,
    RelayTunnel,
}

impl TransportSessionFlavor {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Quic => "quic",
            Self::TcpNoiseYamux => "tcp+noise+yamux",
            Self::RelayTunnel => "relay+tunnel+noise+yamux",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransportSecurity {
    QuicTls,
    Noise,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TransportMuxer {
    Quic,
    Yamux,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct TransportPath {
    pub(super) peer_id: PeerId,
    pub(super) addr: Multiaddr,
    pub(super) kind: TransportPathKind,
    pub(super) flavor: TransportSessionFlavor,
    pub(super) security: TransportSecurity,
    pub(super) muxer: TransportMuxer,
}

impl TransportPath {
    pub(super) fn label(&self) -> String {
        self.addr.to_string()
    }

    pub(super) fn kind_label(&self) -> &'static str {
        self.kind.label()
    }

    pub(super) fn flavor_label(&self) -> &'static str {
        self.flavor.label()
    }

    pub(super) fn preference_rank(&self) -> (TransportPathKind, TransportSessionFlavor) {
        (self.kind, self.flavor)
    }
}

pub(super) fn peer_record_transport_paths(
    record: &SignedPeerRecord,
) -> Result<Vec<TransportPath>, WorldError> {
    let peer_id = record.record.peer_id.parse::<PeerId>().map_err(|_| {
        WorldError::NetworkProtocolUnavailable {
            protocol: "peer record peer_id must be valid".to_string(),
        }
    })?;
    let mut paths = Vec::new();
    let mut seen = HashSet::new();

    extend_paths(
        &mut paths,
        &mut seen,
        peer_id,
        record.record.direct_addrs.iter(),
        TransportPathKind::Direct,
    );
    extend_paths(
        &mut paths,
        &mut seen,
        peer_id,
        record.record.hole_punch_addrs.iter(),
        TransportPathKind::HolePunched,
    );
    extend_paths(
        &mut paths,
        &mut seen,
        peer_id,
        record.record.relay_addrs.iter(),
        TransportPathKind::RelayReserved,
    );
    paths.sort_unstable_by_key(TransportPath::preference_rank);

    Ok(paths)
}

pub(super) fn sync_known_transport_paths(
    known_transport_paths: &mut HashMap<PeerId, Vec<TransportPath>>,
    failed_transport_path_labels: &mut HashSet<String>,
    peer_id: PeerId,
    paths: Vec<TransportPath>,
) {
    let fresh_labels: HashSet<String> = paths.iter().map(TransportPath::label).collect();
    if let Some(previous) = known_transport_paths.get(&peer_id) {
        for stale in previous
            .iter()
            .map(TransportPath::label)
            .filter(|label| !fresh_labels.contains(label))
        {
            failed_transport_path_labels.remove(&stale);
        }
    }
    known_transport_paths.insert(peer_id, paths);
}

pub(super) fn select_preferred_transport_path(
    paths: &[TransportPath],
    failed_transport_path_labels: &HashSet<String>,
) -> Option<TransportPath> {
    paths
        .iter()
        .find(|path| !failed_transport_path_labels.contains(&path.label()))
        .cloned()
        .or_else(|| paths.first().cloned())
}

pub(super) fn active_transport_path_from_endpoint(
    known_transport_paths: &HashMap<PeerId, Vec<TransportPath>>,
    peer_id: PeerId,
    endpoint_addr: &Multiaddr,
) -> TransportPath {
    let normalized = ensure_peer_id(endpoint_addr.clone(), peer_id);
    let normalized_without_peer_id = split_peer_id(normalized.clone()).1;

    if let Some(found) = known_transport_paths
        .get(&peer_id)
        .and_then(|paths| transport_path_for_endpoint(paths.as_slice(), &normalized))
        .cloned()
    {
        return found;
    }

    TransportPath {
        peer_id,
        addr: normalized,
        kind: infer_transport_path_kind(&normalized_without_peer_id),
        flavor: infer_transport_session_flavor(
            infer_transport_path_kind(&normalized_without_peer_id),
            &normalized_without_peer_id,
        ),
        security: infer_transport_security(
            infer_transport_path_kind(&normalized_without_peer_id),
            &normalized_without_peer_id,
        ),
        muxer: infer_transport_muxer(
            infer_transport_path_kind(&normalized_without_peer_id),
            &normalized_without_peer_id,
        ),
    }
}

fn transport_path_for_endpoint<'a>(
    paths: &'a [TransportPath],
    endpoint_addr: &Multiaddr,
) -> Option<&'a TransportPath> {
    let endpoint_without_peer_id = split_peer_id(endpoint_addr.clone()).1;
    paths
        .iter()
        .find(|path| {
            path.addr == *endpoint_addr
                || split_peer_id(path.addr.clone()).1 == endpoint_without_peer_id
        })
        .or_else(|| {
            let endpoint_family = transport_endpoint_family(endpoint_without_peer_id.clone());
            paths.iter().find(|path| {
                transport_endpoint_family(split_peer_id(path.addr.clone()).1) == endpoint_family
            })
        })
}

fn transport_endpoint_family(addr: Multiaddr) -> Vec<String> {
    addr.iter()
        .map(|protocol| match protocol {
            Protocol::Ip4(value) => format!("ip4:{value}"),
            Protocol::Ip6(value) => format!("ip6:{value}"),
            Protocol::Dns(value) => format!("dns:{value}"),
            Protocol::Dns4(value) => format!("dns4:{value}"),
            Protocol::Dns6(value) => format!("dns6:{value}"),
            Protocol::Dnsaddr(value) => format!("dnsaddr:{value}"),
            Protocol::Tcp(_) => "tcp:*".to_string(),
            Protocol::Udp(_) => "udp:*".to_string(),
            Protocol::Quic => "quic".to_string(),
            Protocol::QuicV1 => "quic-v1".to_string(),
            Protocol::P2pCircuit => "p2p-circuit".to_string(),
            other => format!("{other}"),
        })
        .collect()
}

pub(super) fn note_established_transport_path(
    known_transport_paths: &HashMap<PeerId, Vec<TransportPath>>,
    active_transport_paths: &mut HashMap<PeerId, TransportPath>,
    last_dialed_transport_paths: &mut HashMap<PeerId, TransportPath>,
    failed_transport_path_labels: &mut HashSet<String>,
    peer_id: PeerId,
    endpoint_addr: &Multiaddr,
) -> TransportPath {
    let active_path =
        active_transport_path_from_endpoint(known_transport_paths, peer_id, endpoint_addr);
    failed_transport_path_labels.remove(&active_path.label());
    last_dialed_transport_paths.remove(&peer_id);
    active_transport_paths.insert(peer_id, active_path.clone());
    active_path
}

pub(super) fn dial_transport_path(
    swarm: &mut Swarm<Behaviour>,
    last_dialed_transport_paths: &mut HashMap<PeerId, TransportPath>,
    path: TransportPath,
) -> Result<(), WorldError> {
    dial_addr_with_optional_peer_id(swarm, path.addr.clone()).map_err(|err| {
        WorldError::NetworkProtocolUnavailable {
            protocol: format!("transport dial failed: {err}"),
        }
    })?;
    last_dialed_transport_paths.insert(path.peer_id, path);
    Ok(())
}

pub(super) fn failover_transport_path(
    swarm: &mut Swarm<Behaviour>,
    known_transport_paths: &HashMap<PeerId, Vec<TransportPath>>,
    active_transport_paths: &mut HashMap<PeerId, TransportPath>,
    last_dialed_transport_paths: &mut HashMap<PeerId, TransportPath>,
    failed_transport_path_labels: &mut HashSet<String>,
    peer_id: PeerId,
) -> Result<Option<(TransportPath, TransportPath)>, WorldError> {
    let Some(active_path) = active_transport_paths.remove(&peer_id) else {
        return Ok(None);
    };
    failed_transport_path_labels.insert(active_path.label());
    let Some(next_path) = known_transport_paths.get(&peer_id).and_then(|paths| {
        select_preferred_transport_path(paths.as_slice(), failed_transport_path_labels)
    }) else {
        return Ok(None);
    };
    if next_path.label() == active_path.label() {
        return Ok(None);
    }
    dial_transport_path(swarm, last_dialed_transport_paths, next_path.clone())?;
    Ok(Some((active_path, next_path)))
}

pub(super) fn retry_transport_path_after_error(
    swarm: &mut Swarm<Behaviour>,
    known_transport_paths: &HashMap<PeerId, Vec<TransportPath>>,
    last_dialed_transport_paths: &mut HashMap<PeerId, TransportPath>,
    failed_transport_path_labels: &mut HashSet<String>,
    peer_id: PeerId,
) -> Result<Option<(TransportPath, TransportPath)>, WorldError> {
    let Some(last_path) = last_dialed_transport_paths.remove(&peer_id) else {
        return Ok(None);
    };
    failed_transport_path_labels.insert(last_path.label());
    let Some(next_path) = known_transport_paths.get(&peer_id).and_then(|paths| {
        select_preferred_transport_path(paths.as_slice(), failed_transport_path_labels)
    }) else {
        return Ok(None);
    };
    if next_path.label() == last_path.label() {
        return Ok(None);
    }
    dial_transport_path(swarm, last_dialed_transport_paths, next_path.clone())?;
    Ok(Some((last_path, next_path)))
}

fn extend_paths<'a>(
    paths: &mut Vec<TransportPath>,
    seen: &mut HashSet<String>,
    peer_id: PeerId,
    raw_addrs: impl Iterator<Item = &'a String>,
    kind: TransportPathKind,
) {
    for addr in raw_addrs.filter_map(|raw| raw.parse::<Multiaddr>().ok()) {
        let addr = ensure_peer_id(addr, peer_id);
        let label = addr.to_string();
        if !seen.insert(label) {
            continue;
        }
        let addr_without_peer_id = split_peer_id(addr.clone()).1;
        paths.push(TransportPath {
            peer_id,
            addr,
            kind,
            flavor: infer_transport_session_flavor(kind, &addr_without_peer_id),
            security: infer_transport_security(kind, &addr_without_peer_id),
            muxer: infer_transport_muxer(kind, &addr_without_peer_id),
        });
    }
}

fn infer_transport_path_kind(addr: &Multiaddr) -> TransportPathKind {
    if addr
        .iter()
        .any(|protocol| matches!(protocol, Protocol::P2pCircuit))
    {
        TransportPathKind::RelayReserved
    } else {
        TransportPathKind::Direct
    }
}

fn infer_transport_session_flavor(
    kind: TransportPathKind,
    addr: &Multiaddr,
) -> TransportSessionFlavor {
    if matches!(kind, TransportPathKind::RelayReserved) {
        return TransportSessionFlavor::RelayTunnel;
    }
    if addr
        .iter()
        .any(|protocol| matches!(protocol, Protocol::QuicV1))
    {
        TransportSessionFlavor::Quic
    } else {
        TransportSessionFlavor::TcpNoiseYamux
    }
}

fn infer_transport_security(kind: TransportPathKind, addr: &Multiaddr) -> TransportSecurity {
    match infer_transport_session_flavor(kind, addr) {
        TransportSessionFlavor::Quic => TransportSecurity::QuicTls,
        TransportSessionFlavor::TcpNoiseYamux | TransportSessionFlavor::RelayTunnel => {
            TransportSecurity::Noise
        }
    }
}

fn infer_transport_muxer(kind: TransportPathKind, addr: &Multiaddr) -> TransportMuxer {
    match infer_transport_session_flavor(kind, addr) {
        TransportSessionFlavor::Quic => TransportMuxer::Quic,
        TransportSessionFlavor::TcpNoiseYamux | TransportSessionFlavor::RelayTunnel => {
            TransportMuxer::Yamux
        }
    }
}
