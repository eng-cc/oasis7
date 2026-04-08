use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use futures::channel::oneshot;
use libp2p::gossipsub::{IdentTopic, TopicHash};
use libp2p::kad::{self, Quorum, RecordKey};
use libp2p::request_response;
use libp2p::swarm::Swarm;
use libp2p::{Multiaddr, PeerId};

use super::{
    classify_network_protocol, classify_network_topic, maybe_discover_rendezvous_namespace,
    maybe_register_rendezvous_namespace, maybe_request_cached_discovery_peers, now_ms,
    publish_configured_peer_record, publish_discovery_provider, push_bounded_clone,
    put_record_query, recompute_peer_manager_healths, should_republish, start_peer_discovery_query,
    Behaviour, Command, Handler, Keypair, NetworkMessage, NetworkRequest, PeerManagerBlockArtifact,
    PeerManagerHealthIssue, PeerManagerHealthStatus, PeerManagerPeerHealth, PeerManagerPolicy,
    PeerRecord, PendingDhtQuery, PendingPeerRecordRequest, SignedPeerRecord, TransportPath,
    WorldError, DEFAULT_SUBSCRIPTION_INBOX_MAX_MESSAGES,
};

pub(super) enum CommandOutcome {
    Continue,
    Break,
}

pub(super) struct CommandContext<'a> {
    pub event_published: &'a Arc<Mutex<Vec<NetworkMessage>>>,
    pub event_errors: &'a Arc<Mutex<Vec<String>>>,
    pub event_listening_addrs: &'a Arc<Mutex<Vec<Multiaddr>>>,
    pub keypair: &'a Keypair,
    pub peer_record_template: Option<&'a PeerRecord>,
    pub local_peer_id: PeerId,
    pub max_published_messages: usize,
    pub max_error_messages: usize,
    pub republish_interval_ms: i64,
}

pub(super) struct CommandStateRefs<'a> {
    pub subscriptions: &'a mut HashSet<String>,
    pub topic_map: &'a mut HashMap<TopicHash, String>,
    pub topic_inbox_limits: &'a mut HashMap<String, usize>,
    pub handlers: &'a mut HashMap<String, Handler>,
    pub pending: &'a mut HashMap<
        request_response::OutboundRequestId,
        oneshot::Sender<Result<Vec<u8>, WorldError>>,
    >,
    pub pending_peer_record_requests:
        &'a mut HashMap<request_response::OutboundRequestId, PendingPeerRecordRequest>,
    pub pending_dht: &'a mut HashMap<kad::QueryId, PendingDhtQuery>,
    pub peers: &'a mut Vec<PeerId>,
    pub provider_keys: &'a mut HashMap<String, i64>,
    pub discovered_peer_records: &'a HashMap<PeerId, SignedPeerRecord>,
    pub peer_healths_by_id: &'a HashMap<PeerId, PeerManagerPeerHealth>,
    pub pending_cached_discovery_peers: &'a mut HashSet<PeerId>,
    pub pending_rendezvous_registers: &'a mut HashSet<PeerId>,
    pub pending_rendezvous_discovers: &'a mut HashSet<PeerId>,
    pub registered_rendezvous_nodes: &'a HashSet<PeerId>,
    pub rendezvous_cookies: &'a HashMap<PeerId, libp2p::rendezvous::Cookie>,
    pub peer_record_last_published_at_ms: &'a mut Option<i64>,
}

pub(super) fn filter_request_peers_by_lane(
    peers: Vec<PeerId>,
    protocol: &str,
    discovered_peer_records: &HashMap<PeerId, SignedPeerRecord>,
) -> Vec<PeerId> {
    let Some(lane) = classify_network_protocol(protocol) else {
        return peers;
    };
    let mut capable_record_peers = Vec::new();
    let mut unknown_record_peers = Vec::new();
    for peer_id in peers.iter().copied() {
        match discovered_peer_records.get(&peer_id) {
            Some(record) if record.record.supports_lane(lane) => capable_record_peers.push(peer_id),
            Some(_) => {}
            None => unknown_record_peers.push(peer_id),
        }
    }
    if !capable_record_peers.is_empty() {
        capable_record_peers
    } else if !unknown_record_peers.is_empty() {
        unknown_record_peers
    } else {
        Vec::new()
    }
}

pub(super) fn filter_request_peers_by_health(
    peers: Vec<PeerId>,
    peer_healths: &HashMap<PeerId, PeerManagerPeerHealth>,
) -> Vec<PeerId> {
    let mut ranked: Vec<(usize, PeerId)> = peers
        .iter()
        .copied()
        .enumerate()
        .map(|(idx, peer_id)| {
            let rank: usize = match peer_healths.get(&peer_id).map(|health| health.status) {
                Some(PeerManagerHealthStatus::Active) => 0,
                Some(PeerManagerHealthStatus::Candidate) | None => 1,
                Some(PeerManagerHealthStatus::Suspect) => 2,
                Some(PeerManagerHealthStatus::Blocked) => 3,
            };
            (idx, (rank, peer_id))
        })
        .map(|(idx, (rank, peer_id))| (rank.saturating_mul(10_000).saturating_add(idx), peer_id))
        .collect();
    ranked.sort_by_key(|(rank, _)| *rank);
    ranked
        .iter()
        .filter(|(_, peer_id)| {
            !matches!(
                peer_healths.get(peer_id).map(|health| health.status),
                Some(PeerManagerHealthStatus::Blocked)
            )
        })
        .map(|(_, peer_id)| *peer_id)
        .collect()
}

pub(super) fn peer_requires_active_quarantine(
    peer_id: PeerId,
    peer_healths: &HashMap<PeerId, PeerManagerPeerHealth>,
) -> bool {
    let Some(health) = peer_healths.get(&peer_id) else {
        return false;
    };
    match health.status {
        PeerManagerHealthStatus::Suspect => true,
        PeerManagerHealthStatus::Blocked => !health
            .issues
            .iter()
            .all(|issue| matches!(issue, PeerManagerHealthIssue::MissingPeerRecord)),
        PeerManagerHealthStatus::Active | PeerManagerHealthStatus::Candidate => false,
    }
}

#[cfg(test)]
pub(super) fn collect_quarantined_active_peers(
    active_transport_paths: &HashMap<PeerId, TransportPath>,
    peer_healths: &HashMap<PeerId, PeerManagerPeerHealth>,
) -> Vec<PeerId> {
    active_transport_paths
        .keys()
        .copied()
        .filter(|peer_id| peer_requires_active_quarantine(*peer_id, peer_healths))
        .collect()
}

#[cfg(test)]
pub(super) fn admitted_active_transport_paths(
    active_transport_paths: &HashMap<PeerId, TransportPath>,
    peer_healths: &HashMap<PeerId, PeerManagerPeerHealth>,
) -> HashMap<PeerId, TransportPath> {
    active_transport_paths
        .iter()
        .filter(|(peer_id, _)| {
            matches!(
                peer_healths.get(*peer_id).map(|health| health.status),
                Some(PeerManagerHealthStatus::Active)
            )
        })
        .map(|(peer_id, path)| (*peer_id, path.clone()))
        .collect()
}

fn active_transport_paths_for_peers(
    active_transport_paths: &HashMap<PeerId, TransportPath>,
    peer_ids: &HashSet<PeerId>,
) -> HashMap<PeerId, TransportPath> {
    active_transport_paths
        .iter()
        .filter(|(peer_id, _)| peer_ids.contains(peer_id))
        .map(|(peer_id, path)| (*peer_id, path.clone()))
        .collect()
}

pub(super) fn refresh_peer_manager_healths(
    discovered_peer_records: &HashMap<PeerId, SignedPeerRecord>,
    active_transport_paths: &HashMap<PeerId, TransportPath>,
    previously_admitted_active_peers: &HashSet<PeerId>,
    peer_manager_policy: &PeerManagerPolicy,
    event_peer_healths: &Arc<Mutex<HashMap<String, PeerManagerPeerHealth>>>,
    event_block_artifacts: &Arc<Mutex<HashMap<String, PeerManagerBlockArtifact>>>,
    event_errors: &Arc<Mutex<Vec<String>>>,
    max_error_messages: usize,
) -> (
    HashMap<PeerId, PeerManagerPeerHealth>,
    HashSet<PeerId>,
    HashSet<PeerId>,
) {
    let raw_healths = recompute_peer_manager_healths(
        discovered_peer_records,
        active_transport_paths,
        peer_manager_policy,
    );
    let mut quarantined_active_peers = HashSet::new();
    let mut admitted_active_peers: HashSet<PeerId> = previously_admitted_active_peers
        .iter()
        .copied()
        .filter(|peer_id| active_transport_paths.contains_key(peer_id))
        .collect();
    let admitted_baseline_paths =
        active_transport_paths_for_peers(active_transport_paths, &admitted_active_peers);
    let admitted_baseline_healths = recompute_peer_manager_healths(
        discovered_peer_records,
        &admitted_baseline_paths,
        peer_manager_policy,
    );
    for peer_id in admitted_active_peers.clone() {
        if peer_requires_active_quarantine(peer_id, &admitted_baseline_healths) {
            admitted_active_peers.remove(&peer_id);
            quarantined_active_peers.insert(peer_id);
        }
    }
    let mut admitted_paths =
        active_transport_paths_for_peers(active_transport_paths, &admitted_active_peers);
    let mut pending_active_peers: Vec<PeerId> = active_transport_paths
        .keys()
        .copied()
        .filter(|peer_id| !admitted_active_peers.contains(peer_id))
        .collect();
    pending_active_peers.sort_unstable_by_key(|peer_id| peer_id.to_string());
    for peer_id in pending_active_peers {
        if quarantined_active_peers.contains(&peer_id)
            || !discovered_peer_records.contains_key(&peer_id)
        {
            continue;
        }
        let Some(path) = active_transport_paths.get(&peer_id).cloned() else {
            continue;
        };
        let mut trial_paths = admitted_paths.clone();
        trial_paths.insert(peer_id, path.clone());
        let trial_healths = recompute_peer_manager_healths(
            discovered_peer_records,
            &trial_paths,
            peer_manager_policy,
        );
        let peer_is_active = matches!(
            trial_healths.get(&peer_id).map(|health| health.status),
            Some(PeerManagerHealthStatus::Active)
        );
        let degrades_admitted_peer = admitted_active_peers.iter().any(|admitted_peer_id| {
            !matches!(
                trial_healths
                    .get(admitted_peer_id)
                    .map(|health| health.status),
                Some(PeerManagerHealthStatus::Active)
            )
        });
        if peer_is_active && !degrades_admitted_peer {
            admitted_paths.insert(peer_id, path);
            admitted_active_peers.insert(peer_id);
        } else if peer_requires_active_quarantine(peer_id, &trial_healths) {
            quarantined_active_peers.insert(peer_id);
        }
    }
    let mut healths = if admitted_paths.len() == active_transport_paths.len() {
        raw_healths.clone()
    } else {
        recompute_peer_manager_healths(
            discovered_peer_records,
            &admitted_paths,
            peer_manager_policy,
        )
    };
    for peer_id in active_transport_paths.keys().copied() {
        if admitted_paths.contains_key(&peer_id) {
            continue;
        }
        if let Some(raw_health) = raw_healths.get(&peer_id) {
            healths.insert(peer_id, raw_health.clone());
        }
    }
    {
        let mut guard = event_peer_healths.lock().expect("lock peer healths");
        let previous = guard.clone();
        let latest: HashMap<String, PeerManagerPeerHealth> = healths
            .values()
            .cloned()
            .map(|health| (health.peer_id.clone(), health))
            .collect();
        for (peer_id, health) in &latest {
            let old = previous.get(peer_id);
            let transitioned_to_suspect = matches!(
                old.map(|entry| entry.status),
                None | Some(PeerManagerHealthStatus::Active | PeerManagerHealthStatus::Candidate)
            ) && matches!(
                health.status,
                PeerManagerHealthStatus::Suspect | PeerManagerHealthStatus::Blocked
            );
            if transitioned_to_suspect {
                push_bounded_clone(
                    event_errors,
                    format!(
                        "libp2p peer manager suspect peer={} issues={:?}",
                        peer_id, health.issues
                    ),
                    max_error_messages,
                    "lock errors",
                );
            }
        }
        *guard = latest;
    }
    {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or(0);
        let mut guard = event_block_artifacts
            .lock()
            .expect("lock peer block artifacts");
        for health in healths.values() {
            let existing = guard.get_mut(&health.peer_id);
            match (existing, health.status) {
                (Some(artifact), PeerManagerHealthStatus::Blocked) => {
                    artifact.status = health.status;
                    artifact.issues = health.issues.clone();
                    artifact.active_path_kind = health.active_path_kind.clone();
                    artifact.source_operator = health.source_operator.clone();
                    artifact.source_asn = health.source_asn.clone();
                    artifact.last_blocked_at_ms = now_ms;
                    artifact.last_cleared_at_ms = None;
                }
                (Some(artifact), _) => {
                    artifact.status = health.status;
                    artifact.issues = health.issues.clone();
                    artifact.active_path_kind = health.active_path_kind.clone();
                    artifact.source_operator = health.source_operator.clone();
                    artifact.source_asn = health.source_asn.clone();
                    if artifact.last_cleared_at_ms.is_none() {
                        artifact.last_cleared_at_ms = Some(now_ms);
                    }
                }
                (None, PeerManagerHealthStatus::Blocked) => {
                    guard.insert(
                        health.peer_id.clone(),
                        PeerManagerBlockArtifact {
                            peer_id: health.peer_id.clone(),
                            status: health.status,
                            issues: health.issues.clone(),
                            active_path_kind: health.active_path_kind.clone(),
                            source_operator: health.source_operator.clone(),
                            source_asn: health.source_asn.clone(),
                            first_blocked_at_ms: now_ms,
                            last_blocked_at_ms: now_ms,
                            last_cleared_at_ms: None,
                        },
                    );
                }
                (None, _) => {}
            }
        }
    }
    (healths, quarantined_active_peers, admitted_active_peers)
}

pub(super) fn enforce_peer_manager_quarantine(
    swarm: &mut Swarm<Behaviour>,
    quarantined_active_peers: &HashSet<PeerId>,
    pending_quarantine_disconnects: &mut HashSet<PeerId>,
    event_errors: &Arc<Mutex<Vec<String>>>,
    max_error_messages: usize,
) {
    pending_quarantine_disconnects.retain(|peer_id| quarantined_active_peers.contains(peer_id));
    for peer_id in quarantined_active_peers.iter().copied() {
        if !pending_quarantine_disconnects.insert(peer_id) {
            continue;
        }
        if swarm.disconnect_peer_id(peer_id).is_ok() {
            push_bounded_clone(
                event_errors,
                format!("libp2p peer manager quarantine disconnect peer={peer_id}"),
                max_error_messages,
                "lock errors",
            );
        } else {
            pending_quarantine_disconnects.remove(&peer_id);
        }
    }
}

pub(super) fn handle_command(
    swarm: &mut Swarm<Behaviour>,
    command: Option<Command>,
    state: CommandStateRefs<'_>,
    ctx: &CommandContext<'_>,
) -> CommandOutcome {
    let CommandStateRefs {
        subscriptions,
        topic_map,
        topic_inbox_limits,
        handlers,
        pending,
        pending_peer_record_requests,
        pending_dht,
        peers,
        provider_keys,
        discovered_peer_records,
        peer_healths_by_id,
        pending_cached_discovery_peers,
        pending_rendezvous_registers,
        pending_rendezvous_discovers,
        registered_rendezvous_nodes,
        rendezvous_cookies,
        peer_record_last_published_at_ms,
    } = state;

    match command {
        Some(Command::Publish { topic, payload }) => {
            let message = NetworkMessage {
                topic: topic.clone(),
                payload: payload.clone(),
            };
            push_bounded_clone(
                ctx.event_published,
                message,
                ctx.max_published_messages,
                "lock published",
            );
            let topic_handle = IdentTopic::new(topic);
            let _ = swarm
                .behaviour_mut()
                .gossipsub
                .publish(topic_handle, payload);
            CommandOutcome::Continue
        }
        Some(Command::Subscribe { topic }) => {
            if subscriptions.insert(topic.clone()) {
                let topic_handle = IdentTopic::new(topic.clone());
                if swarm
                    .behaviour_mut()
                    .gossipsub
                    .subscribe(&topic_handle)
                    .is_ok()
                {
                    let inbox_limit = classify_network_topic(topic.as_str())
                        .map(|lane| lane.default_subscription_inbox_messages())
                        .unwrap_or(DEFAULT_SUBSCRIPTION_INBOX_MAX_MESSAGES);
                    topic_inbox_limits.insert(topic.clone(), inbox_limit);
                    topic_map.insert(topic_handle.hash(), topic);
                }
            }
            CommandOutcome::Continue
        }
        Some(Command::Dial { addr }) => {
            if let Err(err) = super::dial_addr_with_optional_peer_id(swarm, addr) {
                push_bounded_clone(
                    ctx.event_errors,
                    format!("libp2p dial failed: {err}"),
                    ctx.max_error_messages,
                    "lock errors",
                );
            }
            CommandOutcome::Continue
        }
        Some(Command::Request {
            protocol,
            payload,
            providers,
            response,
        }) => {
            if peers.is_empty() {
                if let Some(handler) = handlers.get(&protocol) {
                    let _ = response.send(handler(&payload));
                } else {
                    let _ = response.send(Err(WorldError::NetworkProtocolUnavailable { protocol }));
                }
                return CommandOutcome::Continue;
            }
            let mut candidate_peers: Vec<PeerId> = Vec::new();
            if !providers.is_empty() {
                for provider in providers {
                    if let Ok(peer_id) = provider.parse::<PeerId>() {
                        if peers.contains(&peer_id) {
                            candidate_peers.push(peer_id);
                        }
                    }
                }
            }
            if candidate_peers.is_empty() {
                candidate_peers = peers.clone();
            }
            candidate_peers = filter_request_peers_by_lane(
                candidate_peers,
                protocol.as_str(),
                discovered_peer_records,
            );
            candidate_peers = filter_request_peers_by_health(candidate_peers, peer_healths_by_id);
            if candidate_peers.is_empty() {
                let _ = response.send(Err(WorldError::NetworkProtocolUnavailable {
                    protocol: format!("no healthy provider for protocol {protocol}"),
                }));
                return CommandOutcome::Continue;
            }
            let peer = candidate_peers[0];
            let request = NetworkRequest { protocol, payload };
            let request_id = swarm
                .behaviour_mut()
                .request_response
                .send_request(&peer, request);
            pending.insert(request_id, response);
            CommandOutcome::Continue
        }
        Some(Command::RequestToPeer {
            protocol,
            payload,
            peer,
            response,
        }) => {
            if !peers.contains(&peer) {
                let _ = response.send(Err(WorldError::NetworkProtocolUnavailable {
                    protocol: format!("peer {peer} is not connected for protocol {protocol}"),
                }));
                return CommandOutcome::Continue;
            }
            let request_id = swarm
                .behaviour_mut()
                .request_response
                .send_request(&peer, NetworkRequest { protocol, payload });
            pending.insert(request_id, response);
            CommandOutcome::Continue
        }
        Some(Command::RegisterHandler {
            protocol,
            handler,
            response,
        }) => {
            handlers.insert(protocol, handler);
            let _ = response.send(Ok(()));
            CommandOutcome::Continue
        }
        Some(Command::PublishProvider { key, response }) => {
            let dht_key = RecordKey::new(&key);
            match swarm.behaviour_mut().kademlia.start_providing(dht_key) {
                Ok(query_id) => {
                    provider_keys.insert(key, now_ms());
                    pending_dht.insert(
                        query_id,
                        PendingDhtQuery::PublishProvider {
                            response: Some(response),
                        },
                    );
                }
                Err(err) => {
                    let _ = response.send(Err(WorldError::NetworkProtocolUnavailable {
                        protocol: format!("kad start_providing failed: {err}"),
                    }));
                }
            }
            CommandOutcome::Continue
        }
        Some(Command::GetProviders { key, response }) => {
            let dht_key = RecordKey::new(&key);
            let query_id = swarm.behaviour_mut().kademlia.get_providers(dht_key);
            pending_dht.insert(
                query_id,
                PendingDhtQuery::GetProviders {
                    response: Some(response),
                    providers: HashSet::new(),
                    error: None,
                },
            );
            CommandOutcome::Continue
        }
        Some(Command::PutWorldHead {
            key,
            payload,
            response,
        }) => {
            let dht_key = RecordKey::new(&key);
            let record = kad::Record {
                key: dht_key,
                value: payload,
                publisher: None,
                expires: None,
            };
            match swarm
                .behaviour_mut()
                .kademlia
                .put_record(record, Quorum::One)
            {
                Ok(query_id) => {
                    pending_dht.insert(
                        query_id,
                        PendingDhtQuery::PutWorldHead {
                            response: Some(response),
                        },
                    );
                }
                Err(err) => {
                    let _ = response.send(Err(WorldError::NetworkProtocolUnavailable {
                        protocol: format!("kad put_record failed: {err}"),
                    }));
                }
            }
            CommandOutcome::Continue
        }
        Some(Command::GetWorldHead { key, response }) => {
            let dht_key = RecordKey::new(&key);
            let query_id = swarm.behaviour_mut().kademlia.get_record(dht_key);
            pending_dht.insert(
                query_id,
                PendingDhtQuery::GetWorldHead {
                    response: Some(response),
                    head: None,
                    error: None,
                },
            );
            CommandOutcome::Continue
        }
        Some(Command::PutMembershipDirectory {
            key,
            payload,
            response,
        }) => {
            let dht_key = RecordKey::new(&key);
            let record = kad::Record {
                key: dht_key,
                value: payload,
                publisher: None,
                expires: None,
            };
            match swarm
                .behaviour_mut()
                .kademlia
                .put_record(record, Quorum::One)
            {
                Ok(query_id) => {
                    pending_dht.insert(
                        query_id,
                        PendingDhtQuery::PutMembershipDirectory {
                            response: Some(response),
                        },
                    );
                }
                Err(err) => {
                    let _ = response.send(Err(WorldError::NetworkProtocolUnavailable {
                        protocol: format!("kad put_record failed: {err}"),
                    }));
                }
            }
            CommandOutcome::Continue
        }
        Some(Command::GetMembershipDirectory { key, response }) => {
            let dht_key = RecordKey::new(&key);
            let query_id = swarm.behaviour_mut().kademlia.get_record(dht_key);
            pending_dht.insert(
                query_id,
                PendingDhtQuery::GetMembershipDirectory {
                    response: Some(response),
                    snapshot: None,
                    error: None,
                },
            );
            CommandOutcome::Continue
        }
        Some(Command::PutPeerRecord {
            key,
            payload,
            response,
        }) => {
            match put_record_query(swarm, key, payload) {
                Ok(query_id) => {
                    pending_dht.insert(
                        query_id,
                        PendingDhtQuery::PutPeerRecord {
                            response: Some(response),
                        },
                    );
                }
                Err(err) => {
                    let _ = response.send(Err(err));
                }
            }
            CommandOutcome::Continue
        }
        Some(Command::GetPeerRecord { key, response }) => {
            let dht_key = RecordKey::new(&key);
            let query_id = swarm.behaviour_mut().kademlia.get_record(dht_key);
            pending_dht.insert(
                query_id,
                PendingDhtQuery::GetPeerRecord {
                    response: Some(response),
                    record: None,
                    error: None,
                },
            );
            CommandOutcome::Continue
        }
        Some(Command::RefreshPeerDiscovery) => {
            if let Some(template) = ctx.peer_record_template {
                let _ = publish_configured_peer_record(
                    swarm,
                    pending_dht,
                    ctx.keypair,
                    template,
                    ctx.event_listening_addrs,
                    None,
                );
                publish_discovery_provider(swarm, provider_keys, template.world_id.as_str());
                start_peer_discovery_query(swarm, pending_dht, template);
                let connected_peers = peers.clone();
                for peer_id in connected_peers {
                    maybe_request_cached_discovery_peers(
                        swarm,
                        pending_peer_record_requests,
                        pending_cached_discovery_peers,
                        peer_id,
                        ctx.local_peer_id,
                    );
                    if let Err(err) = maybe_register_rendezvous_namespace(
                        swarm,
                        pending_rendezvous_registers,
                        registered_rendezvous_nodes,
                        peer_id,
                        ctx.local_peer_id,
                        template,
                    ) {
                        push_bounded_clone(
                            ctx.event_errors,
                            format!("libp2p rendezvous register failed peer={peer_id}: {err:?}"),
                            ctx.max_error_messages,
                            "lock errors",
                        );
                    }
                    if let Err(err) = maybe_discover_rendezvous_namespace(
                        swarm,
                        pending_rendezvous_discovers,
                        rendezvous_cookies,
                        peer_id,
                        ctx.local_peer_id,
                        template,
                    ) {
                        push_bounded_clone(
                            ctx.event_errors,
                            format!("libp2p rendezvous discover failed peer={peer_id}: {err:?}"),
                            ctx.max_error_messages,
                            "lock errors",
                        );
                    }
                }
            }
            CommandOutcome::Continue
        }
        Some(Command::RepublishProviders) => {
            if ctx.republish_interval_ms > 0 {
                let now = now_ms();
                let keys: Vec<String> = provider_keys
                    .iter()
                    .filter_map(|(key, last_publish)| {
                        if should_republish(*last_publish, now, ctx.republish_interval_ms) {
                            Some(key.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                for key in keys {
                    let dht_key = RecordKey::new(&key);
                    if swarm
                        .behaviour_mut()
                        .kademlia
                        .start_providing(dht_key)
                        .is_ok()
                    {
                        provider_keys.insert(key, now);
                    }
                }
                if let Some(template) = ctx.peer_record_template {
                    if peer_record_last_published_at_ms
                        .map(|last_ms| should_republish(last_ms, now, ctx.republish_interval_ms))
                        .unwrap_or(true)
                        && publish_configured_peer_record(
                            swarm,
                            pending_dht,
                            ctx.keypair,
                            template,
                            ctx.event_listening_addrs,
                            None,
                        )
                        .is_ok()
                    {
                        publish_discovery_provider(
                            swarm,
                            provider_keys,
                            template.world_id.as_str(),
                        );
                        *peer_record_last_published_at_ms = Some(now);
                    }
                }
            }
            CommandOutcome::Continue
        }
        Some(Command::Shutdown) | None => CommandOutcome::Break,
    }
}
