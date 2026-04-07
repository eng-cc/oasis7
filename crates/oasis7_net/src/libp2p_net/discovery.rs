use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use libp2p::identity::Keypair;
use libp2p::kad::{self, RecordKey};
use libp2p::rendezvous;
use libp2p::request_response;
use libp2p::swarm::Swarm;
use libp2p::{Multiaddr, PeerId};

use crate::error::WorldError;
use crate::util::to_canonical_cbor;
use oasis7_proto::distributed::{
    dht_peer_discovery_key, dht_peer_record_key,
    rendezvous_namespace as distributed_rendezvous_namespace, DistributedErrorCode, ErrorResponse,
};
use oasis7_proto::distributed_dht::{PeerDiscoverySource, PeerRecord, SignedPeerRecord};
use oasis7_proto::distributed_net::NetworkRequest;

use super::kad_queries::PendingDhtQuery;
use super::peer_manager::{
    recompute_peer_manager_healths, PeerManagerHealthStatus, PeerManagerPolicy,
};
use super::peer_record::{
    build_configured_peer_record, put_record_query, validate_discovered_peer_record,
};
use super::swarm_behaviour::{split_peer_id, Behaviour};
use super::transport_paths::{
    dial_transport_path, peer_record_transport_paths, select_preferred_transport_path,
    sync_known_transport_paths, TransportPath,
};
use super::{push_bounded_clone, Handler};

pub(super) enum PendingPeerRecordRequest {
    ConnectedPeerRecord { peer_id: PeerId },
    CachedPeerRecord { peer_id: PeerId },
    CachedDiscoveryPeers { peer_id: PeerId },
}

pub(super) fn start_peer_discovery_query(
    swarm: &mut Swarm<Behaviour>,
    pending_dht: &mut HashMap<kad::QueryId, PendingDhtQuery>,
    template: &PeerRecord,
) {
    let key = dht_peer_discovery_key(template.world_id.as_str());
    let query_id = swarm
        .behaviour_mut()
        .kademlia
        .get_providers(RecordKey::new(&key));
    pending_dht.insert(
        query_id,
        PendingDhtQuery::DiscoverPeers {
            peers: HashSet::new(),
            error: None,
        },
    );
}

pub(super) fn publish_discovery_provider(
    swarm: &mut Swarm<Behaviour>,
    provider_keys: &mut HashMap<String, i64>,
    world_id: &str,
) {
    let key = dht_peer_discovery_key(world_id);
    if swarm
        .behaviour_mut()
        .kademlia
        .start_providing(RecordKey::new(&key))
        .is_ok()
    {
        provider_keys.insert(key, super::now_ms());
    }
}

pub(super) fn maybe_register_rendezvous_namespace(
    swarm: &mut Swarm<Behaviour>,
    pending_rendezvous_registers: &mut HashSet<PeerId>,
    registered_rendezvous_nodes: &HashSet<PeerId>,
    peer_id: PeerId,
    local_peer_id: PeerId,
    template: &PeerRecord,
) -> Result<bool, WorldError> {
    if !peer_record_enables_rendezvous(template) {
        return Ok(false);
    }
    if peer_id == local_peer_id
        || pending_rendezvous_registers.contains(&peer_id)
        || registered_rendezvous_nodes.contains(&peer_id)
    {
        return Ok(false);
    }
    let namespace = rendezvous_namespace(template)?;
    swarm
        .behaviour_mut()
        .rendezvous_client
        .as_mut()
        .expect("rendezvous client enabled when peer record opts in")
        .register(namespace, peer_id, None)
        .map_err(|err| WorldError::NetworkProtocolUnavailable {
            protocol: format!("rendezvous register failed: {err}"),
        })?;
    pending_rendezvous_registers.insert(peer_id);
    Ok(true)
}

pub(super) fn maybe_discover_rendezvous_namespace(
    swarm: &mut Swarm<Behaviour>,
    pending_rendezvous_discovers: &mut HashSet<PeerId>,
    rendezvous_cookies: &HashMap<PeerId, rendezvous::Cookie>,
    peer_id: PeerId,
    local_peer_id: PeerId,
    template: &PeerRecord,
) -> Result<bool, WorldError> {
    if !peer_record_enables_rendezvous(template) {
        return Ok(false);
    }
    if peer_id == local_peer_id || pending_rendezvous_discovers.contains(&peer_id) {
        return Ok(false);
    }
    let namespace = rendezvous_namespace(template)?;
    let cookie = rendezvous_cookies.get(&peer_id).cloned();
    swarm
        .behaviour_mut()
        .rendezvous_client
        .as_mut()
        .expect("rendezvous client enabled when peer record opts in")
        .discover(Some(namespace), cookie, None, peer_id);
    pending_rendezvous_discovers.insert(peer_id);
    Ok(true)
}

pub(super) fn peer_record_enables_rendezvous(template: &PeerRecord) -> bool {
    template
        .discovery_sources
        .iter()
        .any(|source| matches!(source, PeerDiscoverySource::Rendezvous))
}

pub(super) fn maybe_queue_discovery_peer_record(
    swarm: &mut Swarm<Behaviour>,
    pending_dht: &mut HashMap<kad::QueryId, PendingDhtQuery>,
    pending_discovery_peer_records: &mut HashSet<PeerId>,
    peer_id: PeerId,
    local_peer_id: PeerId,
    world_id: &str,
) {
    if world_id.trim().is_empty()
        || peer_id == local_peer_id
        || pending_discovery_peer_records.contains(&peer_id)
    {
        return;
    }
    let key = dht_peer_record_key(world_id, peer_id.to_string().as_str());
    let query_id = swarm
        .behaviour_mut()
        .kademlia
        .get_record(RecordKey::new(&key));
    pending_discovery_peer_records.insert(peer_id);
    pending_dht.insert(
        query_id,
        PendingDhtQuery::DiscoverPeerRecord {
            peer_id,
            record: None,
            error: None,
        },
    );
}

pub(super) fn maybe_request_connected_peer_record(
    swarm: &mut Swarm<Behaviour>,
    pending_peer_record_requests: &mut HashMap<
        request_response::OutboundRequestId,
        PendingPeerRecordRequest,
    >,
    pending_connected_peer_records: &mut HashSet<PeerId>,
    peer_id: PeerId,
    local_peer_id: PeerId,
) {
    if peer_id == local_peer_id || pending_connected_peer_records.contains(&peer_id) {
        return;
    }
    let request_id = swarm.behaviour_mut().request_response.send_request(
        &peer_id,
        NetworkRequest {
            protocol: super::RR_GET_LOCAL_PEER_RECORD.to_string(),
            payload: Vec::new(),
        },
    );
    pending_connected_peer_records.insert(peer_id);
    pending_peer_record_requests.insert(
        request_id,
        PendingPeerRecordRequest::ConnectedPeerRecord { peer_id },
    );
}

fn request_cached_peer_record_via(
    swarm: &mut Swarm<Behaviour>,
    pending_peer_record_requests: &mut HashMap<
        request_response::OutboundRequestId,
        PendingPeerRecordRequest,
    >,
    pending_cached_peer_records: &mut HashSet<PeerId>,
    ask_peer: PeerId,
    peer_id: PeerId,
    local_peer_id: PeerId,
) -> bool {
    if peer_id == local_peer_id
        || ask_peer == local_peer_id
        || pending_cached_peer_records.contains(&peer_id)
    {
        return false;
    }
    let request_id = swarm.behaviour_mut().request_response.send_request(
        &ask_peer,
        NetworkRequest {
            protocol: super::RR_GET_CACHED_PEER_RECORD.to_string(),
            payload: peer_id.to_string().into_bytes(),
        },
    );
    pending_cached_peer_records.insert(peer_id);
    pending_peer_record_requests.insert(
        request_id,
        PendingPeerRecordRequest::CachedPeerRecord { peer_id },
    );
    true
}

pub(super) fn maybe_request_cached_peer_record(
    swarm: &mut Swarm<Behaviour>,
    pending_peer_record_requests: &mut HashMap<
        request_response::OutboundRequestId,
        PendingPeerRecordRequest,
    >,
    pending_cached_peer_records: &mut HashSet<PeerId>,
    connected_peers: &[PeerId],
    peer_id: PeerId,
    local_peer_id: PeerId,
) -> bool {
    if peer_id == local_peer_id || pending_cached_peer_records.contains(&peer_id) {
        return false;
    }
    let Some(ask_peer) = connected_peers
        .iter()
        .copied()
        .find(|candidate| *candidate != peer_id && *candidate != local_peer_id)
        .or_else(|| connected_peers.first().copied())
    else {
        return false;
    };
    request_cached_peer_record_via(
        swarm,
        pending_peer_record_requests,
        pending_cached_peer_records,
        ask_peer,
        peer_id,
        local_peer_id,
    )
}

pub(super) fn maybe_request_cached_discovery_peers(
    swarm: &mut Swarm<Behaviour>,
    pending_peer_record_requests: &mut HashMap<
        request_response::OutboundRequestId,
        PendingPeerRecordRequest,
    >,
    pending_cached_discovery_peers: &mut HashSet<PeerId>,
    peer_id: PeerId,
    local_peer_id: PeerId,
) {
    if peer_id == local_peer_id || pending_cached_discovery_peers.contains(&peer_id) {
        return;
    }
    let request_id = swarm.behaviour_mut().request_response.send_request(
        &peer_id,
        NetworkRequest {
            protocol: super::RR_GET_CACHED_DISCOVERY_PEERS.to_string(),
            payload: Vec::new(),
        },
    );
    pending_cached_discovery_peers.insert(peer_id);
    pending_peer_record_requests.insert(
        request_id,
        PendingPeerRecordRequest::CachedDiscoveryPeers { peer_id },
    );
}

pub(super) fn handle_rendezvous_discovered(
    swarm: &mut Swarm<Behaviour>,
    rendezvous_node: PeerId,
    registrations: Vec<rendezvous::Registration>,
    pending_dht: &mut HashMap<kad::QueryId, PendingDhtQuery>,
    pending_peer_record_requests: &mut HashMap<
        request_response::OutboundRequestId,
        PendingPeerRecordRequest,
    >,
    pending_discovery_peer_records: &mut HashSet<PeerId>,
    pending_cached_peer_records: &mut HashSet<PeerId>,
    connected_peers: &[PeerId],
    local_peer_id: PeerId,
    template: Option<&PeerRecord>,
    max_error_messages: usize,
    event_errors: &Arc<Mutex<Vec<String>>>,
) {
    if registrations.is_empty() {
        return;
    }
    push_bounded_clone(
        event_errors,
        format!(
            "libp2p rendezvous discovered registrations via={rendezvous_node} count={}",
            registrations.len()
        ),
        max_error_messages,
        "lock errors",
    );
    let world_id = template
        .map(|record| record.world_id.as_str())
        .unwrap_or_default();
    for registration in registrations {
        let peer_id = registration.record.peer_id();
        if peer_id == local_peer_id {
            continue;
        }
        maybe_queue_discovery_peer_record(
            swarm,
            pending_dht,
            pending_discovery_peer_records,
            peer_id,
            local_peer_id,
            world_id,
        );
        maybe_request_cached_peer_record(
            swarm,
            pending_peer_record_requests,
            pending_cached_peer_records,
            connected_peers,
            peer_id,
            local_peer_id,
        );
    }
}

pub(super) fn process_discovered_peer_record(
    swarm: &mut Swarm<Behaviour>,
    discovered_peer_records: &mut HashMap<PeerId, SignedPeerRecord>,
    known_transport_paths: &mut HashMap<PeerId, Vec<TransportPath>>,
    last_dialed_transport_paths: &mut HashMap<PeerId, TransportPath>,
    active_transport_paths: &HashMap<PeerId, TransportPath>,
    failed_transport_path_labels: &mut HashSet<String>,
    dialed_discovery_addrs: &mut HashSet<String>,
    template: Option<&PeerRecord>,
    peer_manager_policy: &PeerManagerPolicy,
    record: SignedPeerRecord,
) -> Result<(), WorldError> {
    validate_discovered_peer_record(&record, template)?;
    let peer_id = record.record.peer_id.parse::<PeerId>().map_err(|_| {
        WorldError::NetworkProtocolUnavailable {
            protocol: "peer record peer_id must be valid".to_string(),
        }
    })?;
    let transport_paths = peer_record_transport_paths(&record)?;
    for path in &transport_paths {
        let (_, kad_addr) = split_peer_id(path.addr.clone());
        swarm
            .behaviour_mut()
            .kademlia
            .add_address(&peer_id, kad_addr);
    }
    sync_known_transport_paths(
        known_transport_paths,
        failed_transport_path_labels,
        peer_id,
        transport_paths.clone(),
    );
    discovered_peer_records.insert(peer_id, record);
    let peer_healths = recompute_peer_manager_healths(
        discovered_peer_records,
        active_transport_paths,
        peer_manager_policy,
    );
    let peer_status = peer_healths
        .get(&peer_id)
        .map(|health| health.status)
        .unwrap_or(PeerManagerHealthStatus::Candidate);
    if let Some(preferred_path) =
        select_preferred_transport_path(transport_paths.as_slice(), failed_transport_path_labels)
    {
        let should_dial = !matches!(
            peer_status,
            PeerManagerHealthStatus::Suspect | PeerManagerHealthStatus::Blocked
        ) && active_transport_paths
            .get(&peer_id)
            .map(|active| preferred_path.preference_rank() < active.preference_rank())
            .unwrap_or(true);
        let addr_label = preferred_path.label();
        if should_dial && dialed_discovery_addrs.insert(addr_label) {
            let _ = dial_transport_path(swarm, last_dialed_transport_paths, preferred_path.clone());
        }
    }
    Ok(())
}

pub(super) fn handle_request_response_request(
    request: &NetworkRequest,
    handlers: &HashMap<String, Handler>,
    peer_record_template: Option<&PeerRecord>,
    keypair: &Keypair,
    listening_addrs: &Arc<Mutex<Vec<Multiaddr>>>,
    discovered_peer_records: &HashMap<PeerId, SignedPeerRecord>,
) -> Result<Vec<u8>, WorldError> {
    match request.protocol.as_str() {
        super::RR_GET_LOCAL_PEER_RECORD => {
            let Some(template) = peer_record_template else {
                return Err(WorldError::NetworkProtocolUnavailable {
                    protocol: super::RR_GET_LOCAL_PEER_RECORD.to_string(),
                });
            };
            let record = build_configured_peer_record(keypair, template, listening_addrs)?;
            to_canonical_cbor(&record)
        }
        super::RR_GET_CACHED_PEER_RECORD => {
            let peer_id = String::from_utf8(request.payload.clone())
                .map_err(|_| WorldError::NetworkProtocolUnavailable {
                    protocol: "cached peer record payload must be utf-8".to_string(),
                })?
                .parse::<PeerId>()
                .map_err(|_| WorldError::NetworkProtocolUnavailable {
                    protocol: "cached peer record peer_id must be valid".to_string(),
                })?;
            let record = discovered_peer_records.get(&peer_id).ok_or_else(|| {
                WorldError::NetworkProtocolUnavailable {
                    protocol: super::RR_GET_CACHED_PEER_RECORD.to_string(),
                }
            })?;
            to_canonical_cbor(record)
        }
        super::RR_GET_CACHED_DISCOVERY_PEERS => {
            let peers: Vec<String> = discovered_peer_records
                .values()
                .filter(|record| {
                    peer_record_template
                        .map(|template| {
                            record.record.world_id == template.world_id
                                && record.record.network_id == template.network_id
                        })
                        .unwrap_or(true)
                })
                .map(|record| record.record.peer_id.clone())
                .collect();
            to_canonical_cbor(&peers)
        }
        _ => {
            if let Some(handler) = handlers.get(&request.protocol) {
                handler(&request.payload)
            } else {
                Err(WorldError::NetworkProtocolUnavailable {
                    protocol: request.protocol.clone(),
                })
            }
        }
    }
}

pub(super) fn handle_peer_record_response(
    swarm: &mut Swarm<Behaviour>,
    kind: PendingPeerRecordRequest,
    payload: &[u8],
    pending_peer_record_requests: &mut HashMap<
        request_response::OutboundRequestId,
        PendingPeerRecordRequest,
    >,
    pending_dht: &mut HashMap<kad::QueryId, PendingDhtQuery>,
    discovered_peer_records: &mut HashMap<PeerId, SignedPeerRecord>,
    known_transport_paths: &mut HashMap<PeerId, Vec<TransportPath>>,
    last_dialed_transport_paths: &mut HashMap<PeerId, TransportPath>,
    active_transport_paths: &HashMap<PeerId, TransportPath>,
    failed_transport_path_labels: &mut HashSet<String>,
    pending_discovery_peer_records: &mut HashSet<PeerId>,
    dialed_discovery_addrs: &mut HashSet<String>,
    peer_record_template: Option<&PeerRecord>,
    local_peer_id: PeerId,
    pending_connected_peer_records: &mut HashSet<PeerId>,
    pending_cached_peer_records: &mut HashSet<PeerId>,
    pending_cached_discovery_peers: &mut HashSet<PeerId>,
    max_error_messages: usize,
    event_errors: &Arc<Mutex<Vec<String>>>,
    peer_manager_policy: &PeerManagerPolicy,
) {
    clear_pending_peer_record_request(
        &kind,
        pending_connected_peer_records,
        pending_cached_peer_records,
        pending_cached_discovery_peers,
    );
    let requested_peer_id = match &kind {
        PendingPeerRecordRequest::ConnectedPeerRecord { peer_id }
        | PendingPeerRecordRequest::CachedPeerRecord { peer_id }
        | PendingPeerRecordRequest::CachedDiscoveryPeers { peer_id } => *peer_id,
    };
    if let PendingPeerRecordRequest::CachedDiscoveryPeers { peer_id } = &kind {
        match decode_cached_discovery_peers_response(payload) {
            Ok(peer_ids) => {
                for discovered_peer_id in peer_ids {
                    maybe_queue_discovery_peer_record(
                        swarm,
                        pending_dht,
                        pending_discovery_peer_records,
                        discovered_peer_id,
                        local_peer_id,
                        peer_record_template
                            .map(|record| record.world_id.as_str())
                            .unwrap_or_default(),
                    );
                    let _ = request_cached_peer_record_via(
                        swarm,
                        pending_peer_record_requests,
                        pending_cached_peer_records,
                        *peer_id,
                        discovered_peer_id,
                        local_peer_id,
                    );
                }
            }
            Err(err) => {
                push_bounded_clone(
                    event_errors,
                    format!(
                        "libp2p cached discovery peers decode failed peer={requested_peer_id}: {err:?}"
                    ),
                    max_error_messages,
                    "lock errors",
                );
            }
        }
        return;
    }
    match decode_optional_peer_record_response(payload) {
        Ok(Some(record)) => {
            if let Err(err) = process_discovered_peer_record(
                swarm,
                discovered_peer_records,
                known_transport_paths,
                last_dialed_transport_paths,
                active_transport_paths,
                failed_transport_path_labels,
                dialed_discovery_addrs,
                peer_record_template,
                peer_manager_policy,
                record.clone(),
            ) {
                push_bounded_clone(
                    event_errors,
                    format!(
                        "libp2p peer record response rejected peer={requested_peer_id}: {err:?}"
                    ),
                    max_error_messages,
                    "lock errors",
                );
            } else {
                let _ = republish_cached_peer_record(swarm, pending_dht, &record);
            }
        }
        Ok(None) => {}
        Err(err) => {
            push_bounded_clone(
                event_errors,
                format!(
                    "libp2p peer record response decode failed peer={requested_peer_id}: {err:?}"
                ),
                max_error_messages,
                "lock errors",
            );
        }
    }
}

pub(super) fn clear_pending_peer_record_request(
    kind: &PendingPeerRecordRequest,
    pending_connected_peer_records: &mut HashSet<PeerId>,
    pending_cached_peer_records: &mut HashSet<PeerId>,
    pending_cached_discovery_peers: &mut HashSet<PeerId>,
) {
    match kind {
        PendingPeerRecordRequest::ConnectedPeerRecord { peer_id } => {
            pending_connected_peer_records.remove(peer_id);
        }
        PendingPeerRecordRequest::CachedPeerRecord { peer_id } => {
            pending_cached_peer_records.remove(peer_id);
        }
        PendingPeerRecordRequest::CachedDiscoveryPeers { peer_id } => {
            pending_cached_discovery_peers.remove(peer_id);
        }
    }
}

pub(super) fn republish_cached_peer_record(
    swarm: &mut Swarm<Behaviour>,
    pending_dht: &mut HashMap<kad::QueryId, PendingDhtQuery>,
    record: &SignedPeerRecord,
) -> Result<(), WorldError> {
    let key = dht_peer_record_key(
        record.record.world_id.as_str(),
        record.record.peer_id.as_str(),
    );
    let payload = to_canonical_cbor(record)?;
    let query_id = put_record_query(swarm, key, payload)?;
    pending_dht.insert(query_id, PendingDhtQuery::PutPeerRecord { response: None });
    Ok(())
}

pub(super) fn decode_optional_peer_record_response(
    payload: &[u8],
) -> Result<Option<SignedPeerRecord>, WorldError> {
    if let Ok(error) = serde_cbor::from_slice::<ErrorResponse>(payload) {
        if error.code == DistributedErrorCode::ErrNotFound {
            return Ok(None);
        }
        return Err(WorldError::NetworkRequestFailed {
            code: error.code,
            message: error.message,
            retryable: error.retryable,
        });
    }
    Ok(Some(serde_cbor::from_slice(payload)?))
}

pub(super) fn decode_cached_discovery_peers_response(
    payload: &[u8],
) -> Result<Vec<PeerId>, WorldError> {
    let peer_ids: Vec<String> = serde_cbor::from_slice(payload)?;
    let mut decoded = Vec::with_capacity(peer_ids.len());
    for peer_id in peer_ids {
        let peer_id =
            peer_id
                .parse::<PeerId>()
                .map_err(|_| WorldError::NetworkProtocolUnavailable {
                    protocol: "cached discovery peer_id must be valid".to_string(),
                })?;
        decoded.push(peer_id);
    }
    Ok(decoded)
}

pub(super) fn rendezvous_namespace(
    template: &PeerRecord,
) -> Result<rendezvous::Namespace, WorldError> {
    rendezvous::Namespace::new(distributed_rendezvous_namespace(
        template.world_id.as_str(),
        template.network_id.as_str(),
    ))
    .map_err(|_| WorldError::NetworkProtocolUnavailable {
        protocol: "rendezvous namespace must be <= 255 bytes".to_string(),
    })
}
