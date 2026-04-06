//! Libp2p-based network adapter skeleton (gossipsub + request/response).

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

mod api;
mod discovery;
mod kad_queries;
mod peer_manager;
mod peer_record;
mod reachability;
mod runtime_loop;
mod swarm_behaviour;
mod transport_paths;
mod utils;

use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, StreamExt};
use libp2p::gossipsub::{self, TopicHash};
use libp2p::identity::Keypair;
use libp2p::kad::{self};
use libp2p::relay;
use libp2p::rendezvous;
use libp2p::request_response::{self};
use libp2p::swarm::SwarmEvent;
use libp2p::{Multiaddr, PeerId};

use crate::error::WorldError;
use oasis7_proto::distributed::{DistributedErrorCode, ErrorResponse, WorldHeadAnnounce};
use oasis7_proto::distributed_dht::{
    MembershipDirectorySnapshot, PeerRecord, ProviderRecord, SignedPeerRecord,
};
use oasis7_proto::distributed_net::{
    classify_network_protocol, classify_network_topic, push_bounded_inbox_message, NetworkMessage,
    NetworkRequest, NetworkResponse, DEFAULT_SUBSCRIPTION_INBOX_MAX_MESSAGES,
};

use crate::util::to_canonical_cbor;
use discovery::{
    clear_pending_peer_record_request, handle_peer_record_response, handle_rendezvous_discovered,
    handle_request_response_request, maybe_discover_rendezvous_namespace,
    maybe_queue_discovery_peer_record, maybe_register_rendezvous_namespace,
    maybe_request_cached_discovery_peers, maybe_request_cached_peer_record,
    maybe_request_connected_peer_record, process_discovered_peer_record,
    publish_discovery_provider, start_peer_discovery_query, PendingPeerRecordRequest,
};
use kad_queries::{handle_dht_progress, DhtProgressAction, PendingDhtQuery};
use peer_manager::recompute_peer_manager_healths;
pub use peer_manager::{
    PeerManagerHealthIssue, PeerManagerHealthStatus, PeerManagerPeerHealth, PeerManagerPolicy,
};
use peer_record::{publish_configured_peer_record, put_record_query};
pub use reachability::{Libp2pReachabilitySnapshot, LiveHolePunchState, LiveTransportKind};
use reachability::{
    note_hole_punch_result, note_relay_reservation_accepted, refresh_active_transport_snapshot,
    snapshot_clone, sync_relay_reservation_from_listening_addrs,
};
use runtime_loop::{
    enforce_peer_manager_quarantine, handle_command, refresh_peer_manager_healths, CommandContext,
    CommandOutcome, CommandStateRefs,
};
use swarm_behaviour::{build_swarm, dial_addr_with_optional_peer_id, Behaviour, BehaviourEvent};
use transport_paths::{
    failover_transport_path, note_established_transport_path, retry_transport_path_after_error,
    TransportPath,
};
use utils::{
    decode_membership_directory, decode_world_head, now_ms, push_bounded_clone, should_republish,
    try_send_command,
};

#[cfg(test)]
use runtime_loop::{
    admitted_active_transport_paths, collect_quarantined_active_peers,
    filter_request_peers_by_health, filter_request_peers_by_lane,
};

const DEFAULT_COMMAND_BUFFER_CAPACITY: usize = 2048;
const DEFAULT_MAX_PUBLISHED_MESSAGES: usize = 4096;
const DEFAULT_MAX_ERROR_MESSAGES: usize = 4096;
const DEFAULT_MAX_LISTENING_ADDRS: usize = 128;
const DEFAULT_DISCOVERY_QUERY_INTERVAL_MS: i64 = 15_000;
const RR_GET_LOCAL_PEER_RECORD: &str = "/aw/rr/1.0.0/get_local_peer_record";
const RR_GET_CACHED_PEER_RECORD: &str = "/aw/rr/1.0.0/get_cached_peer_record";
const RR_GET_CACHED_DISCOVERY_PEERS: &str = "/aw/rr/1.0.0/get_cached_discovery_peers";

#[derive(Debug, Clone)]
pub struct Libp2pNetworkConfig {
    pub keypair: Option<Keypair>,
    pub peer_record: Option<PeerRecord>,
    pub listen_addrs: Vec<Multiaddr>,
    pub bootstrap_peers: Vec<Multiaddr>,
    pub republish_interval_ms: i64,
    pub discovery_query_interval_ms: i64,
    pub command_buffer_capacity: usize,
    pub max_published_messages: usize,
    pub max_error_messages: usize,
    pub max_listening_addrs: usize,
    pub peer_manager_policy: PeerManagerPolicy,
}

impl Default for Libp2pNetworkConfig {
    fn default() -> Self {
        Self {
            keypair: None,
            peer_record: None,
            listen_addrs: Vec::new(),
            bootstrap_peers: Vec::new(),
            republish_interval_ms: 5 * 60 * 1000,
            discovery_query_interval_ms: DEFAULT_DISCOVERY_QUERY_INTERVAL_MS,
            command_buffer_capacity: DEFAULT_COMMAND_BUFFER_CAPACITY,
            max_published_messages: DEFAULT_MAX_PUBLISHED_MESSAGES,
            max_error_messages: DEFAULT_MAX_ERROR_MESSAGES,
            max_listening_addrs: DEFAULT_MAX_LISTENING_ADDRS,
            peer_manager_policy: PeerManagerPolicy::default(),
        }
    }
}

#[derive(Clone)]
pub struct Libp2pNetwork {
    peer_id: PeerId,
    keypair: Keypair,
    command_tx: mpsc::Sender<Command>,
    inbox: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
    published: Arc<Mutex<Vec<NetworkMessage>>>,
    listening_addrs: Arc<Mutex<Vec<Multiaddr>>>,
    connected_peers: Arc<Mutex<HashSet<PeerId>>>,
    errors: Arc<Mutex<Vec<String>>>,
    peer_healths: Arc<Mutex<HashMap<String, PeerManagerPeerHealth>>>,
    reachability: Arc<Mutex<Libp2pReachabilitySnapshot>>,
}

type Handler = Arc<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>;

enum Command {
    Publish {
        topic: String,
        payload: Vec<u8>,
    },
    Subscribe {
        topic: String,
    },
    Dial {
        addr: Multiaddr,
    },
    Request {
        protocol: String,
        payload: Vec<u8>,
        providers: Vec<String>,
        response: oneshot::Sender<Result<Vec<u8>, WorldError>>,
    },
    RegisterHandler {
        protocol: String,
        handler: Handler,
    },
    PublishProvider {
        key: String,
        response: oneshot::Sender<Result<(), WorldError>>,
    },
    GetProviders {
        key: String,
        response: oneshot::Sender<Result<Vec<ProviderRecord>, WorldError>>,
    },
    PutWorldHead {
        key: String,
        payload: Vec<u8>,
        response: oneshot::Sender<Result<(), WorldError>>,
    },
    GetWorldHead {
        key: String,
        response: oneshot::Sender<Result<Option<WorldHeadAnnounce>, WorldError>>,
    },
    PutMembershipDirectory {
        key: String,
        payload: Vec<u8>,
        response: oneshot::Sender<Result<(), WorldError>>,
    },
    GetMembershipDirectory {
        key: String,
        response: oneshot::Sender<Result<Option<MembershipDirectorySnapshot>, WorldError>>,
    },
    PutPeerRecord {
        key: String,
        payload: Vec<u8>,
        response: oneshot::Sender<Result<(), WorldError>>,
    },
    GetPeerRecord {
        key: String,
        response: oneshot::Sender<Result<Option<SignedPeerRecord>, WorldError>>,
    },
    RefreshPeerDiscovery,
    RepublishProviders,
    Shutdown,
}

impl Libp2pNetwork {
    pub fn new(config: Libp2pNetworkConfig) -> Self {
        let keypair = config
            .keypair
            .clone()
            .unwrap_or_else(Keypair::generate_ed25519);
        let peer_id = PeerId::from(keypair.public());
        let inbox = Arc::new(Mutex::new(HashMap::<String, Vec<Vec<u8>>>::new()));
        let published = Arc::new(Mutex::new(Vec::new()));
        let listening_addrs = Arc::new(Mutex::new(Vec::new()));
        let connected_peers = Arc::new(Mutex::new(HashSet::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));
        let peer_healths = Arc::new(Mutex::new(HashMap::<String, PeerManagerPeerHealth>::new()));
        let reachability = Arc::new(Mutex::new(Libp2pReachabilitySnapshot::default()));
        let command_buffer_capacity = config.command_buffer_capacity.max(1);
        let (command_tx, command_rx) = mpsc::channel(command_buffer_capacity);
        let max_published_messages = config.max_published_messages.max(1);
        let max_error_messages = config.max_error_messages.max(1);
        let max_listening_addrs = config.max_listening_addrs.max(1);

        let event_inbox = Arc::clone(&inbox);
        let event_published = Arc::clone(&published);
        let event_listening_addrs = Arc::clone(&listening_addrs);
        let event_connected_peers = Arc::clone(&connected_peers);
        let event_errors = Arc::clone(&errors);
        let event_peer_healths = Arc::clone(&peer_healths);
        let event_reachability = Arc::clone(&reachability);
        let config_clone = config.clone();
        let keypair_clone = keypair.clone();
        let local_peer_id = peer_id;
        let republish_tx = command_tx.clone();
        let discovery_tx = command_tx.clone();
        let bootstrap_peers = config.bootstrap_peers.clone();
        let peer_record_template = config.peer_record.clone();

        std::thread::spawn(move || {
            let mut swarm = build_swarm(&keypair_clone);
            let mut subscriptions = HashSet::new();
            let mut topic_map: HashMap<TopicHash, String> = HashMap::new();
            let mut topic_inbox_limits: HashMap<String, usize> = HashMap::new();
            let mut handlers: HashMap<String, Handler> = HashMap::new();
            let mut pending: HashMap<
                request_response::OutboundRequestId,
                oneshot::Sender<Result<Vec<u8>, WorldError>>,
            > = HashMap::new();
            let mut pending_peer_record_requests: HashMap<
                request_response::OutboundRequestId,
                PendingPeerRecordRequest,
            > = HashMap::new();
            let mut pending_dht: HashMap<kad::QueryId, PendingDhtQuery> = HashMap::new();
            let mut peers: Vec<PeerId> = Vec::new();
            let mut provider_keys: HashMap<String, i64> = HashMap::new();
            let mut discovered_peer_records: HashMap<PeerId, SignedPeerRecord> = HashMap::new();
            let mut known_transport_paths: HashMap<PeerId, Vec<TransportPath>> = HashMap::new();
            let mut last_dialed_transport_paths: HashMap<PeerId, TransportPath> = HashMap::new();
            let mut active_transport_paths: HashMap<PeerId, TransportPath> = HashMap::new();
            let mut peer_healths_by_id: HashMap<PeerId, PeerManagerPeerHealth> = HashMap::new();
            let mut quarantined_active_peers: HashSet<PeerId> = HashSet::new();
            let mut admitted_active_peers: HashSet<PeerId> = HashSet::new();
            let mut failed_transport_path_labels: HashSet<String> = HashSet::new();
            let mut pending_quarantine_disconnects: HashSet<PeerId> = HashSet::new();
            let mut pending_discovery_peer_records: HashSet<PeerId> = HashSet::new();
            let mut pending_connected_peer_records: HashSet<PeerId> = HashSet::new();
            let mut pending_cached_peer_records: HashSet<PeerId> = HashSet::new();
            let mut pending_cached_discovery_peers: HashSet<PeerId> = HashSet::new();
            let mut pending_rendezvous_registers: HashSet<PeerId> = HashSet::new();
            let mut pending_rendezvous_discovers: HashSet<PeerId> = HashSet::new();
            let mut registered_rendezvous_nodes: HashSet<PeerId> = HashSet::new();
            let mut rendezvous_cookies: HashMap<PeerId, rendezvous::Cookie> = HashMap::new();
            let mut dialed_discovery_addrs: HashSet<String> = HashSet::new();
            let mut peer_record_last_published_at_ms = None;
            let republish_interval_ms = config_clone.republish_interval_ms;
            let discovery_query_interval_ms = config_clone.discovery_query_interval_ms;

            for addr in config_clone.listen_addrs {
                if let Err(err) = swarm.listen_on(addr) {
                    let msg = format!("libp2p listen failed: {err}");
                    push_bounded_clone(&event_errors, msg, max_error_messages, "lock errors");
                }
            }

            if republish_interval_ms > 0 {
                std::thread::spawn(move || {
                    let mut republish_tx = republish_tx;
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(
                            republish_interval_ms as u64,
                        ));
                        match republish_tx.try_send(Command::RepublishProviders) {
                            Ok(()) => {}
                            Err(err) if err.is_full() => {
                                // Best effort: skip this republish tick if the command queue is saturated.
                            }
                            Err(_) => break,
                        }
                    }
                });
            }

            if discovery_query_interval_ms > 0 && peer_record_template.is_some() {
                std::thread::spawn(move || {
                    let mut discovery_tx = discovery_tx;
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(
                            discovery_query_interval_ms as u64,
                        ));
                        match discovery_tx.try_send(Command::RefreshPeerDiscovery) {
                            Ok(()) => {}
                            Err(err) if err.is_full() => {}
                            Err(_) => break,
                        }
                    }
                });
            }

            async_std::task::block_on(async move {
                let mut command_rx = command_rx;
                let command_ctx = CommandContext {
                    event_published: &event_published,
                    event_errors: &event_errors,
                    event_listening_addrs: &event_listening_addrs,
                    keypair: &keypair_clone,
                    peer_record_template: peer_record_template.as_ref(),
                    local_peer_id,
                    max_published_messages,
                    max_error_messages,
                    republish_interval_ms,
                };
                loop {
                    futures::select! {
                        command = command_rx.next().fuse() => {
                            match handle_command(
                                &mut swarm,
                                command,
                                CommandStateRefs {
                                    subscriptions: &mut subscriptions,
                                    topic_map: &mut topic_map,
                                    topic_inbox_limits: &mut topic_inbox_limits,
                                    handlers: &mut handlers,
                                    pending: &mut pending,
                                    pending_peer_record_requests: &mut pending_peer_record_requests,
                                    pending_dht: &mut pending_dht,
                                    peers: &mut peers,
                                    provider_keys: &mut provider_keys,
                                    discovered_peer_records: &discovered_peer_records,
                                    peer_healths_by_id: &peer_healths_by_id,
                                    pending_cached_discovery_peers: &mut pending_cached_discovery_peers,
                                    pending_rendezvous_registers: &mut pending_rendezvous_registers,
                                    pending_rendezvous_discovers: &mut pending_rendezvous_discovers,
                                    registered_rendezvous_nodes: &registered_rendezvous_nodes,
                                    rendezvous_cookies: &rendezvous_cookies,
                                    peer_record_last_published_at_ms: &mut peer_record_last_published_at_ms,
                                },
                                &command_ctx,
                            ) {
                                CommandOutcome::Continue => {}
                                CommandOutcome::Break => {
                                    break;
                                }
                            }
                        }
                        event = swarm.select_next_some().fuse() => {
                            match event {
                                SwarmEvent::Behaviour(BehaviourEvent::Gossipsub(gossipsub::Event::Message { message, .. })) => {
                                    let topic = topic_map
                                        .get(&message.topic)
                                        .cloned()
                                        .unwrap_or_else(|| message.topic.as_str().to_string());
                                    let inbox_limit = topic_inbox_limits
                                        .get(topic.as_str())
                                        .copied()
                                        .or_else(|| {
                                            classify_network_topic(topic.as_str())
                                                .map(|lane| lane.default_subscription_inbox_messages())
                                        })
                                        .unwrap_or(DEFAULT_SUBSCRIPTION_INBOX_MAX_MESSAGES);
                                    push_bounded_inbox_message(
                                        &event_inbox,
                                        topic.as_str(),
                                        message.data,
                                        inbox_limit,
                                    );
                                }
                                SwarmEvent::Behaviour(BehaviourEvent::RequestResponse(event)) => {
                                    match event {
                                        request_response::Event::Message { message, peer: _ } => {
                                            match message {
                                                request_response::Message::Request { request, channel, .. } => {
                                                    let reply = handle_request_response_request(
                                                        &request,
                                                        &handlers,
                                                        peer_record_template.as_ref(),
                                                        &keypair_clone,
                                                        &event_listening_addrs,
                                                        &discovered_peer_records,
                                                    );
                                                    let response_bytes = match reply {
                                                        Ok(bytes) => bytes,
                                                        Err(err) => {
                                                            let error = ErrorResponse::from_code(
                                                                DistributedErrorCode::ErrNotFound,
                                                                format!("{err:?}"),
                                                            );
                                                            to_canonical_cbor(&error).unwrap_or_default()
                                                        }
                                                    };
                                                    let response = NetworkResponse { payload: response_bytes };
                                                    swarm.behaviour_mut().request_response.send_response(channel, response).ok();
                                                }
                                                request_response::Message::Response { request_id, response } => {
                                                    if let Some(kind) = pending_peer_record_requests.remove(&request_id) {
                                                        handle_peer_record_response(
                                                            &mut swarm,
                                                            kind,
                                                            response.payload.as_slice(),
                                                            &mut pending_peer_record_requests,
                                                            &mut pending_dht,
                                                        &mut discovered_peer_records,
                                                            &mut known_transport_paths,
                                                            &mut last_dialed_transport_paths,
                                                            &active_transport_paths,
                                                            &mut failed_transport_path_labels,
                                                            &mut pending_discovery_peer_records,
                                                            &mut dialed_discovery_addrs,
                                                            peer_record_template.as_ref(),
                                                            local_peer_id,
                                                            &mut pending_connected_peer_records,
                                                            &mut pending_cached_peer_records,
                                                            &mut pending_cached_discovery_peers,
                                                            max_error_messages,
                                                            &event_errors,
                                                            &config_clone.peer_manager_policy,
                                                        );
                                                        (
                                                            peer_healths_by_id,
                                                            quarantined_active_peers,
                                                            admitted_active_peers,
                                                        ) = refresh_peer_manager_healths(
                                                            &discovered_peer_records,
                                                            &active_transport_paths,
                                                            &admitted_active_peers,
                                                            &config_clone.peer_manager_policy,
                                                            &event_peer_healths,
                                                            &event_errors,
                                                            max_error_messages,
                                                        );
                                                        enforce_peer_manager_quarantine(
                                                            &mut swarm,
                                                            &quarantined_active_peers,
                                                            &mut pending_quarantine_disconnects,
                                                            &event_errors,
                                                            max_error_messages,
                                                        );
                                                    } else if let Some(sender) = pending.remove(&request_id) {
                                                        let _ = sender.send(Ok(response.payload));
                                                    }
                                                }
                                            }
                                        }
                                        request_response::Event::OutboundFailure { request_id, error, .. } => {
                                            if let Some(kind) = pending_peer_record_requests.remove(&request_id) {
                                                clear_pending_peer_record_request(
                                                    &kind,
                                                    &mut pending_connected_peer_records,
                                                    &mut pending_cached_peer_records,
                                                    &mut pending_cached_discovery_peers,
                                                );
                                                push_bounded_clone(
                                                    &event_errors,
                                                    format!("libp2p peer record request failed: {error:?}"),
                                                    max_error_messages,
                                                    "lock errors",
                                                );
                                            } else if let Some(sender) = pending.remove(&request_id) {
                                                let _ = sender.send(Err(WorldError::NetworkProtocolUnavailable { protocol: format!("request failed: {error:?}") }));
                                            }
                                        }
                                        request_response::Event::InboundFailure { peer, error, .. } => {
                                            eprintln!("libp2p inbound failure from {peer:?}: {error:?}");
                                        }
                                        request_response::Event::ResponseSent { .. } => {}
                                    }
                                }
                                SwarmEvent::Behaviour(BehaviourEvent::Kademlia(event)) => {
                                    match event {
                                        kad::Event::OutboundQueryProgressed { id, result, step, .. } => {
                                            let action = if let Some(pending) = pending_dht.get_mut(&id) {
                                                handle_dht_progress(pending, result, step.last)
                                            } else {
                                                DhtProgressAction::None
                                            };
                                            if step.last {
                                                pending_dht.remove(&id);
                                            }
                                            match action {
                                                DhtProgressAction::None => {}
                                                DhtProgressAction::DiscoverPeers(found) => {
                                                    for peer_id in found {
                                                        maybe_queue_discovery_peer_record(
                                                            &mut swarm,
                                                            &mut pending_dht,
                                                            &mut pending_discovery_peer_records,
                                                            peer_id,
                                                            local_peer_id,
                                                            peer_record_template
                                                                .as_ref()
                                                                .map(|record| record.world_id.as_str())
                                                                .unwrap_or_default(),
                                                        );
                                                        maybe_request_cached_peer_record(
                                                            &mut swarm,
                                                            &mut pending_peer_record_requests,
                                                            &mut pending_cached_peer_records,
                                                            peers.as_slice(),
                                                            peer_id,
                                                            local_peer_id,
                                                        );
                                                    }
                                                }
                                                DhtProgressAction::DiscoveryError(err) => {
                                                    push_bounded_clone(
                                                        &event_errors,
                                                        format!("libp2p discovery query failed: {err:?}"),
                                                        max_error_messages,
                                                        "lock errors",
                                                    );
                                                }
                                                DhtProgressAction::DiscoverPeerRecord { peer_id, result } => {
                                                    pending_discovery_peer_records.remove(&peer_id);
                                                    match result {
                                                        Ok(Some(record)) => {
                                                            if let Err(err) = process_discovered_peer_record(
                                                                &mut swarm,
                                                                &mut discovered_peer_records,
                                                                &mut known_transport_paths,
                                                                &mut last_dialed_transport_paths,
                                                                &active_transport_paths,
                                                                &mut failed_transport_path_labels,
                                                                &mut dialed_discovery_addrs,
                                                                peer_record_template.as_ref(),
                                                                &config_clone.peer_manager_policy,
                                                                record,
                                                            ) {
                                                                push_bounded_clone(
                                                                    &event_errors,
                                                                    format!("libp2p discovered peer record rejected peer={peer_id}: {err:?}"),
                                                                    max_error_messages,
                                                                    "lock errors",
                                                                );
                                                                maybe_request_cached_peer_record(
                                                                    &mut swarm,
                                                                    &mut pending_peer_record_requests,
                                                                    &mut pending_cached_peer_records,
                                                                    peers.as_slice(),
                                                                    peer_id,
                                                                    local_peer_id,
                                                                );
                                                            } else {
                                                                (
                                                                    peer_healths_by_id,
                                                                    quarantined_active_peers,
                                                                    admitted_active_peers,
                                                                ) = refresh_peer_manager_healths(
                                                                    &discovered_peer_records,
                                                                    &active_transport_paths,
                                                                    &admitted_active_peers,
                                                                    &config_clone.peer_manager_policy,
                                                                    &event_peer_healths,
                                                                    &event_errors,
                                                                    max_error_messages,
                                                                );
                                                                enforce_peer_manager_quarantine(
                                                                    &mut swarm,
                                                                    &quarantined_active_peers,
                                                                    &mut pending_quarantine_disconnects,
                                                                    &event_errors,
                                                                    max_error_messages,
                                                                );
                                                            }
                                                        }
                                                        Ok(None) => {
                                                            maybe_request_cached_peer_record(
                                                                &mut swarm,
                                                                &mut pending_peer_record_requests,
                                                                &mut pending_cached_peer_records,
                                                                peers.as_slice(),
                                                                peer_id,
                                                                local_peer_id,
                                                            );
                                                        }
                                                        Err(err) => {
                                                            push_bounded_clone(
                                                                &event_errors,
                                                                format!("libp2p discovered peer record load failed peer={peer_id}: {err:?}"),
                                                                max_error_messages,
                                                                "lock errors",
                                                            );
                                                            maybe_request_cached_peer_record(
                                                                &mut swarm,
                                                                &mut pending_peer_record_requests,
                                                                &mut pending_cached_peer_records,
                                                                peers.as_slice(),
                                                                peer_id,
                                                                local_peer_id,
                                                            );
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        kad::Event::RoutingUpdated { peer, addresses, .. } => {
                                            push_bounded_clone(
                                                &event_errors,
                                                format!("libp2p routing updated peer={peer} addrs={addresses:?}"),
                                                max_error_messages,
                                                "lock errors",
                                            );
                                            maybe_queue_discovery_peer_record(
                                                &mut swarm,
                                                &mut pending_dht,
                                                &mut pending_discovery_peer_records,
                                                peer,
                                                local_peer_id,
                                                peer_record_template
                                                    .as_ref()
                                                    .map(|record| record.world_id.as_str())
                                                    .unwrap_or_default(),
                                            );
                                            if peers.contains(&peer) {
                                                maybe_request_connected_peer_record(
                                                    &mut swarm,
                                                    &mut pending_peer_record_requests,
                                                    &mut pending_connected_peer_records,
                                                    peer,
                                                    local_peer_id,
                                                );
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                SwarmEvent::Behaviour(BehaviourEvent::RelayClient(event)) => {
                                    match event {
                                        relay::client::Event::ReservationReqAccepted { relay_peer_id, renewal, .. } => {
                                            note_relay_reservation_accepted(&event_reachability);
                                            push_bounded_clone(
                                                &event_errors,
                                                format!(
                                                    "libp2p relay reservation accepted relay={relay_peer_id} renewal={renewal}"
                                                ),
                                                max_error_messages,
                                                "lock errors",
                                            );
                                            if let Some(template) = peer_record_template.as_ref() {
                                                let _ = publish_configured_peer_record(
                                                    &mut swarm,
                                                    &mut pending_dht,
                                                    &keypair_clone,
                                                    template,
                                                    &event_listening_addrs,
                                                    None,
                                                );
                                                peer_record_last_published_at_ms = Some(now_ms());
                                                publish_discovery_provider(
                                                    &mut swarm,
                                                    &mut provider_keys,
                                                    template.world_id.as_str(),
                                                );
                                            }
                                        }
                                        relay::client::Event::OutboundCircuitEstablished { relay_peer_id, .. } => {
                                            push_bounded_clone(
                                                &event_errors,
                                                format!(
                                                    "libp2p relay outbound circuit established relay={relay_peer_id}"
                                                ),
                                                max_error_messages,
                                                "lock errors",
                                            );
                                        }
                                        relay::client::Event::InboundCircuitEstablished { src_peer_id, .. } => {
                                            push_bounded_clone(
                                                &event_errors,
                                                format!(
                                                    "libp2p relay inbound circuit established src={src_peer_id}"
                                                ),
                                                max_error_messages,
                                                "lock errors",
                                            );
                                        }
                                    }
                                }
                                SwarmEvent::Behaviour(BehaviourEvent::Dcutr(event)) => {
                                    note_hole_punch_result(&event_reachability, event.result.is_ok());
                                    let outcome = match event.result {
                                        Ok(connection_id) => {
                                            format!(
                                                "libp2p dcutr hole-punch upgraded peer={} connection_id={connection_id}",
                                                event.remote_peer_id
                                            )
                                        }
                                        Err(err) => {
                                            format!(
                                                "libp2p dcutr hole-punch failed peer={}: {err}",
                                                event.remote_peer_id
                                            )
                                        }
                                    };
                                    push_bounded_clone(
                                        &event_errors,
                                        outcome,
                                        max_error_messages,
                                        "lock errors",
                                    );
                                }
                                SwarmEvent::Behaviour(BehaviourEvent::RendezvousClient(event)) => {
                                    match event {
                                        rendezvous::client::Event::Registered { rendezvous_node, .. } => {
                                            pending_rendezvous_registers.remove(&rendezvous_node);
                                            registered_rendezvous_nodes.insert(rendezvous_node);
                                        }
                                        rendezvous::client::Event::RegisterFailed { rendezvous_node, namespace, error } => {
                                            pending_rendezvous_registers.remove(&rendezvous_node);
                                            registered_rendezvous_nodes.remove(&rendezvous_node);
                                            push_bounded_clone(
                                                &event_errors,
                                                format!(
                                                    "libp2p rendezvous register rejected peer={rendezvous_node} namespace={namespace}: {error:?}"
                                                ),
                                                max_error_messages,
                                                "lock errors",
                                            );
                                        }
                                        rendezvous::client::Event::Discovered {
                                            rendezvous_node,
                                            registrations,
                                            cookie,
                                        } => {
                                            pending_rendezvous_discovers.remove(&rendezvous_node);
                                            rendezvous_cookies.insert(rendezvous_node, cookie);
                                            handle_rendezvous_discovered(
                                                &mut swarm,
                                                rendezvous_node,
                                                registrations,
                                                &mut pending_dht,
                                                &mut pending_peer_record_requests,
                                                &mut pending_discovery_peer_records,
                                                &mut pending_cached_peer_records,
                                                peers.as_slice(),
                                                local_peer_id,
                                                peer_record_template.as_ref(),
                                                max_error_messages,
                                                &event_errors,
                                            );
                                        }
                                        rendezvous::client::Event::DiscoverFailed { rendezvous_node, namespace, error } => {
                                            pending_rendezvous_discovers.remove(&rendezvous_node);
                                            push_bounded_clone(
                                                &event_errors,
                                                format!(
                                                    "libp2p rendezvous discover rejected peer={rendezvous_node} namespace={namespace:?}: {error:?}"
                                                ),
                                                max_error_messages,
                                                "lock errors",
                                            );
                                        }
                                        rendezvous::client::Event::Expired { peer } => {
                                            rendezvous_cookies.remove(&peer);
                                        }
                                    }
                                }
                                SwarmEvent::Behaviour(BehaviourEvent::RendezvousServer(event)) => {
                                    if let rendezvous::server::Event::PeerNotRegistered { peer, namespace, error } = event {
                                        push_bounded_clone(
                                            &event_errors,
                                            format!(
                                                "libp2p rendezvous server rejected peer={peer} namespace={namespace}: {error:?}"
                                            ),
                                            max_error_messages,
                                            "lock errors",
                                        );
                                    }
                                }
                                SwarmEvent::NewListenAddr { address, .. } => {
                                    swarm.add_external_address(address.clone());
                                    push_bounded_clone(
                                        &event_listening_addrs,
                                        address.clone(),
                                        max_listening_addrs,
                                        "lock listening addrs",
                                    );
                                    let listening_addrs = event_listening_addrs
                                        .lock()
                                        .expect("lock listening addrs")
                                        .clone();
                                    sync_relay_reservation_from_listening_addrs(
                                        &event_reachability,
                                        listening_addrs.as_slice(),
                                    );
                                    if let Some(template) = peer_record_template.as_ref() {
                                        let _ = publish_configured_peer_record(
                                            &mut swarm,
                                            &mut pending_dht,
                                            &keypair_clone,
                                            template,
                                            &event_listening_addrs,
                                            None,
                                        );
                                        peer_record_last_published_at_ms = Some(now_ms());
                                        publish_discovery_provider(
                                            &mut swarm,
                                            &mut provider_keys,
                                            template.world_id.as_str(),
                                        );
                                    }
                                }
                                SwarmEvent::ExpiredListenAddr { address, .. } => {
                                    swarm.remove_external_address(&address);
                                    {
                                        let mut listening_addrs = event_listening_addrs
                                            .lock()
                                            .expect("lock listening addrs");
                                        listening_addrs.retain(|candidate| candidate != &address);
                                        sync_relay_reservation_from_listening_addrs(
                                            &event_reachability,
                                            listening_addrs.as_slice(),
                                        );
                                    }
                                    if let Some(template) = peer_record_template.as_ref() {
                                        let _ = publish_configured_peer_record(
                                            &mut swarm,
                                            &mut pending_dht,
                                            &keypair_clone,
                                            template,
                                            &event_listening_addrs,
                                            None,
                                        );
                                        peer_record_last_published_at_ms = Some(now_ms());
                                        publish_discovery_provider(
                                            &mut swarm,
                                            &mut provider_keys,
                                            template.world_id.as_str(),
                                        );
                                    }
                                }
                                SwarmEvent::ListenerClosed { addresses, .. } => {
                                    for address in addresses.iter() {
                                        swarm.remove_external_address(address);
                                    }
                                    {
                                        let mut listening_addrs = event_listening_addrs
                                            .lock()
                                            .expect("lock listening addrs");
                                        listening_addrs.retain(|candidate| {
                                            !addresses.iter().any(|addr| addr == candidate)
                                        });
                                        sync_relay_reservation_from_listening_addrs(
                                            &event_reachability,
                                            listening_addrs.as_slice(),
                                        );
                                    }
                                    if let Some(template) = peer_record_template.as_ref() {
                                        let _ = publish_configured_peer_record(
                                            &mut swarm,
                                            &mut pending_dht,
                                            &keypair_clone,
                                            template,
                                            &event_listening_addrs,
                                            None,
                                        );
                                        peer_record_last_published_at_ms = Some(now_ms());
                                        publish_discovery_provider(
                                            &mut swarm,
                                            &mut provider_keys,
                                            template.world_id.as_str(),
                                        );
                                    }
                                }
                                SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                                    if !peers.contains(&peer_id) {
                                        peers.push(peer_id);
                                    }
                                    event_connected_peers
                                        .lock()
                                        .expect("lock connected peers")
                                        .insert(peer_id);
                                    push_bounded_clone(
                                        &event_errors,
                                        format!("libp2p connection established peer={peer_id}"),
                                        max_error_messages,
                                        "lock errors",
                                    );
                                    match endpoint {
                                        libp2p::core::connection::ConnectedPoint::Dialer { address, .. } => {
                                            let active_path = note_established_transport_path(
                                                &known_transport_paths,
                                                &mut active_transport_paths,
                                                &mut last_dialed_transport_paths,
                                                &mut failed_transport_path_labels,
                                                peer_id,
                                                &address,
                                            );
                                            push_bounded_clone(
                                                &event_errors,
                                                format!(
                                                    "libp2p transport active peer={peer_id} kind={} flavor={} addr={}",
                                                    active_path.kind_label(),
                                                    active_path.flavor_label(),
                                                    active_path.addr,
                                                ),
                                                max_error_messages,
                                                "lock errors",
                                            );
                                            swarm
                                                .behaviour_mut()
                                                .kademlia
                                                .add_address(&peer_id, address.clone());
                                        }
                                        libp2p::core::connection::ConnectedPoint::Listener { send_back_addr, .. } => {
                                            let active_path = note_established_transport_path(
                                                &known_transport_paths,
                                                &mut active_transport_paths,
                                                &mut last_dialed_transport_paths,
                                                &mut failed_transport_path_labels,
                                                peer_id,
                                                &send_back_addr,
                                            );
                                            push_bounded_clone(
                                                &event_errors,
                                                format!(
                                                    "libp2p transport active peer={peer_id} kind={} flavor={} addr={}",
                                                    active_path.kind_label(),
                                                    active_path.flavor_label(),
                                                    active_path.addr,
                                                ),
                                                max_error_messages,
                                                "lock errors",
                                            );
                                            swarm
                                                .behaviour_mut()
                                                .kademlia
                                                .add_address(&peer_id, send_back_addr.clone());
                                        }
                                    }
                                    maybe_queue_discovery_peer_record(
                                        &mut swarm,
                                        &mut pending_dht,
                                        &mut pending_discovery_peer_records,
                                        peer_id,
                                        local_peer_id,
                                        peer_record_template
                                            .as_ref()
                                            .map(|record| record.world_id.as_str())
                                            .unwrap_or_default(),
                                    );
                                    maybe_request_connected_peer_record(
                                        &mut swarm,
                                        &mut pending_peer_record_requests,
                                        &mut pending_connected_peer_records,
                                        peer_id,
                                        local_peer_id,
                                    );
                                    maybe_request_cached_discovery_peers(
                                        &mut swarm,
                                        &mut pending_peer_record_requests,
                                        &mut pending_cached_discovery_peers,
                                        peer_id,
                                        local_peer_id,
                                    );
                                    if let Some(template) = peer_record_template.as_ref() {
                                        if let Err(err) = maybe_register_rendezvous_namespace(
                                            &mut swarm,
                                            &mut pending_rendezvous_registers,
                                            &registered_rendezvous_nodes,
                                            peer_id,
                                            local_peer_id,
                                            template,
                                        ) {
                                            push_bounded_clone(
                                                &event_errors,
                                                format!(
                                                    "libp2p rendezvous register failed peer={peer_id}: {err:?}"
                                                ),
                                                max_error_messages,
                                                "lock errors",
                                            );
                                        }
                                        if let Err(err) = maybe_discover_rendezvous_namespace(
                                            &mut swarm,
                                            &mut pending_rendezvous_discovers,
                                            &rendezvous_cookies,
                                            peer_id,
                                            local_peer_id,
                                            template,
                                        ) {
                                            push_bounded_clone(
                                                &event_errors,
                                                format!(
                                                    "libp2p rendezvous discover failed peer={peer_id}: {err:?}"
                                                ),
                                                max_error_messages,
                                                "lock errors",
                                            );
                                        }
                                        start_peer_discovery_query(
                                            &mut swarm,
                                            &mut pending_dht,
                                            template,
                                        );
                                    }
                                    (
                                        peer_healths_by_id,
                                        quarantined_active_peers,
                                        admitted_active_peers,
                                    ) = refresh_peer_manager_healths(
                                        &discovered_peer_records,
                                        &active_transport_paths,
                                        &admitted_active_peers,
                                        &config_clone.peer_manager_policy,
                                        &event_peer_healths,
                                        &event_errors,
                                        max_error_messages,
                                    );
                                    enforce_peer_manager_quarantine(
                                        &mut swarm,
                                        &quarantined_active_peers,
                                        &mut pending_quarantine_disconnects,
                                        &event_errors,
                                        max_error_messages,
                                    );
                                    refresh_active_transport_snapshot(
                                        &event_reachability,
                                        &active_transport_paths,
                                    );
                                }
                                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                                    peers.retain(|peer| peer != &peer_id);
                                    admitted_active_peers.remove(&peer_id);
                                    let quarantined = quarantined_active_peers.remove(&peer_id)
                                        || pending_quarantine_disconnects.contains(&peer_id);
                                    pending_quarantine_disconnects.remove(&peer_id);
                                    if quarantined {
                                        active_transport_paths.remove(&peer_id);
                                        last_dialed_transport_paths.remove(&peer_id);
                                        push_bounded_clone(
                                            &event_errors,
                                            format!(
                                                "libp2p peer manager quarantine suppresses failover peer={peer_id}"
                                            ),
                                            max_error_messages,
                                            "lock errors",
                                        );
                                    } else {
                                        match failover_transport_path(
                                            &mut swarm,
                                            &known_transport_paths,
                                            &mut active_transport_paths,
                                            &mut last_dialed_transport_paths,
                                            &mut failed_transport_path_labels,
                                            peer_id,
                                        ) {
                                            Ok(Some((active_path, next_path))) => {
                                                push_bounded_clone(
                                                    &event_errors,
                                                    format!(
                                                        "libp2p transport failover peer={peer_id} from={} to={}",
                                                        active_path.addr,
                                                        next_path.addr,
                                                    ),
                                                    max_error_messages,
                                                    "lock errors",
                                                );
                                            }
                                            Err(err) => {
                                                push_bounded_clone(
                                                    &event_errors,
                                                    format!(
                                                        "libp2p transport failover dial failed peer={peer_id}: {err:?}"
                                                    ),
                                                    max_error_messages,
                                                    "lock errors",
                                                );
                                            }
                                            Ok(None) => {}
                                        }
                                    }
                                    pending_rendezvous_registers.remove(&peer_id);
                                    pending_rendezvous_discovers.remove(&peer_id);
                                    registered_rendezvous_nodes.remove(&peer_id);
                                    rendezvous_cookies.remove(&peer_id);
                                    event_connected_peers
                                        .lock()
                                        .expect("lock connected peers")
                                        .remove(&peer_id);
                                    push_bounded_clone(
                                        &event_errors,
                                        format!("libp2p connection closed peer={peer_id}"),
                                        max_error_messages,
                                        "lock errors",
                                    );
                                    (
                                        peer_healths_by_id,
                                        quarantined_active_peers,
                                        admitted_active_peers,
                                    ) = refresh_peer_manager_healths(
                                        &discovered_peer_records,
                                        &active_transport_paths,
                                        &admitted_active_peers,
                                        &config_clone.peer_manager_policy,
                                        &event_peer_healths,
                                        &event_errors,
                                        max_error_messages,
                                    );
                                    refresh_active_transport_snapshot(
                                        &event_reachability,
                                        &active_transport_paths,
                                    );
                                }
                                SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                                    if let Some(peer_id) = peer_id {
                                        if quarantined_active_peers.contains(&peer_id)
                                            || pending_quarantine_disconnects.contains(&peer_id)
                                        {
                                            last_dialed_transport_paths.remove(&peer_id);
                                            push_bounded_clone(
                                                &event_errors,
                                                format!(
                                                    "libp2p peer manager quarantine suppresses retry peer={peer_id}"
                                                ),
                                                max_error_messages,
                                                "lock errors",
                                            );
                                        } else {
                                            match retry_transport_path_after_error(
                                                &mut swarm,
                                                &known_transport_paths,
                                                &mut last_dialed_transport_paths,
                                                &mut failed_transport_path_labels,
                                                peer_id,
                                            ) {
                                                Ok(Some((last_path, next_path))) => {
                                                    push_bounded_clone(
                                                        &event_errors,
                                                        format!(
                                                            "libp2p transport retry peer={peer_id} from={} to={}",
                                                            last_path.addr,
                                                            next_path.addr,
                                                        ),
                                                        max_error_messages,
                                                        "lock errors",
                                                    );
                                                }
                                                Err(retry_err) => {
                                                    push_bounded_clone(
                                                        &event_errors,
                                                        format!(
                                                            "libp2p transport retry dial failed peer={peer_id}: {retry_err:?}"
                                                        ),
                                                        max_error_messages,
                                                        "lock errors",
                                                    );
                                                }
                                                Ok(None) => {}
                                            }
                                        }
                                    }
                                    push_bounded_clone(
                                        &event_errors,
                                        format!(
                                            "libp2p outgoing connection error peer={peer_id:?}: {error:?}"
                                        ),
                                        max_error_messages,
                                        "lock errors",
                                    );
                                }
                                SwarmEvent::IncomingConnectionError { error, .. } => {
                                    push_bounded_clone(
                                        &event_errors,
                                        format!("libp2p incoming connection error: {error:?}"),
                                        max_error_messages,
                                        "lock errors",
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                }
            });
        });

        let mut bootstrap_tx = command_tx.clone();
        for addr in bootstrap_peers {
            // Best-effort: if the background task exits, dial requests can be dropped.
            if bootstrap_tx.try_send(Command::Dial { addr }).is_err() {
                break;
            }
        }

        Self {
            peer_id,
            keypair,
            command_tx,
            inbox,
            published,
            listening_addrs,
            connected_peers,
            errors,
            peer_healths,
            reachability,
        }
    }
}

impl Drop for Libp2pNetwork {
    fn drop(&mut self) {
        let _ = self.enqueue_command(Command::Shutdown);
    }
}

#[cfg(test)]
mod tests;
