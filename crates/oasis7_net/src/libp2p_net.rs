//! Libp2p-based network adapter skeleton (gossipsub + request/response).

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

mod api;
mod discovery;
mod kad_queries;
mod peer_record;
mod swarm_behaviour;
mod utils;

use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, StreamExt};
use libp2p::gossipsub::{self, IdentTopic, TopicHash};
use libp2p::identity::Keypair;
use libp2p::kad::{self, Quorum, RecordKey};
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
    push_bounded_inbox_message, NetworkMessage, NetworkRequest, NetworkResponse,
    DEFAULT_SUBSCRIPTION_INBOX_MAX_MESSAGES,
};

use crate::util::to_canonical_cbor;
use discovery::{
    clear_pending_peer_record_request, dial_routing_updated_addrs, handle_peer_record_response,
    handle_rendezvous_discovered, handle_request_response_request,
    maybe_discover_rendezvous_namespace, maybe_queue_discovery_peer_record,
    maybe_register_rendezvous_namespace, maybe_request_cached_discovery_peers,
    maybe_request_cached_peer_record, maybe_request_connected_peer_record,
    process_discovered_peer_record, publish_discovery_provider, start_peer_discovery_query,
    PendingPeerRecordRequest,
};
use kad_queries::{handle_dht_progress, DhtProgressAction, PendingDhtQuery};
use peer_record::{publish_configured_peer_record, put_record_query};
use swarm_behaviour::{build_swarm, dial_addr_with_optional_peer_id, Behaviour, BehaviourEvent};
use utils::{
    decode_membership_directory, decode_world_head, now_ms, push_bounded_clone, should_republish,
    try_send_command,
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
                loop {
                    futures::select! {
                        command = command_rx.next().fuse() => {
                            match command {
                                Some(Command::Publish { topic, payload }) => {
                                    let message = NetworkMessage { topic: topic.clone(), payload: payload.clone() };
                                    push_bounded_clone(
                                        &event_published,
                                        message,
                                        max_published_messages,
                                        "lock published",
                                    );
                                    let topic_handle = IdentTopic::new(topic.clone());
                                    let _ = swarm.behaviour_mut().gossipsub.publish(topic_handle, payload);
                                }
                                Some(Command::Subscribe { topic }) => {
                                    if subscriptions.insert(topic.clone()) {
                                        let topic_handle = IdentTopic::new(topic.clone());
                                        if swarm.behaviour_mut().gossipsub.subscribe(&topic_handle).is_ok() {
                                            topic_map.insert(topic_handle.hash(), topic);
                                        }
                                    }
                                }
                                Some(Command::Dial { addr }) => {
                                    if let Err(err) = dial_addr_with_optional_peer_id(&mut swarm, addr) {
                                        push_bounded_clone(
                                            &event_errors,
                                            format!("libp2p dial failed: {err}"),
                                            max_error_messages,
                                            "lock errors",
                                        );
                                    }
                                }
                                Some(Command::Request { protocol, payload, providers, response }) => {
                                    if peers.is_empty() {
                                        if let Some(handler) = handlers.get(&protocol) {
                                            let reply = handler(&payload).map_err(|err| err);
                                            let _ = response.send(reply);
                                        } else {
                                            let _ = response.send(Err(WorldError::NetworkProtocolUnavailable { protocol }));
                                        }
                                        continue;
                                    }
                                    let mut selected_peer = None;
                                    if !providers.is_empty() {
                                        for provider in providers {
                                            if let Ok(peer_id) = provider.parse::<PeerId>() {
                                                if peers.contains(&peer_id) {
                                                    selected_peer = Some(peer_id);
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    let peer = selected_peer.unwrap_or_else(|| peers[0]);
                                    let request = NetworkRequest { protocol: protocol.clone(), payload };
                                    let request_id = swarm.behaviour_mut().request_response.send_request(&peer, request);
                                    pending.insert(request_id, response);
                                }
                                Some(Command::RegisterHandler { protocol, handler }) => {
                                    handlers.insert(protocol, handler);
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
                                }
                                Some(Command::PutWorldHead { key, payload, response }) => {
                                    let dht_key = RecordKey::new(&key);
                                    let record = kad::Record {
                                        key: dht_key,
                                        value: payload,
                                        publisher: None,
                                        expires: None,
                                    };
                                    match swarm.behaviour_mut().kademlia.put_record(record, Quorum::One) {
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
                                }
                                Some(Command::PutMembershipDirectory { key, payload, response }) => {
                                    let dht_key = RecordKey::new(&key);
                                    let record = kad::Record {
                                        key: dht_key,
                                        value: payload,
                                        publisher: None,
                                        expires: None,
                                    };
                                    match swarm.behaviour_mut().kademlia.put_record(record, Quorum::One) {
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
                                }
                                Some(Command::PutPeerRecord { key, payload, response }) => {
                                    match put_record_query(&mut swarm, key, payload) {
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
                                }
                                Some(Command::RefreshPeerDiscovery) => {
                                    if let Some(template) = peer_record_template.as_ref() {
                                        let _ = publish_configured_peer_record(
                                            &mut swarm,
                                            &mut pending_dht,
                                            &keypair_clone,
                                            template,
                                            &event_listening_addrs,
                                            None,
                                        );
                                        publish_discovery_provider(
                                            &mut swarm,
                                            &mut provider_keys,
                                            template.world_id.as_str(),
                                        );
                                        start_peer_discovery_query(
                                            &mut swarm,
                                            &mut pending_dht,
                                            template,
                                        );
                                        let connected_peers = peers.clone();
                                        for peer_id in connected_peers {
                                            maybe_request_cached_discovery_peers(
                                                &mut swarm,
                                                &mut pending_peer_record_requests,
                                                &mut pending_cached_discovery_peers,
                                                peer_id,
                                                local_peer_id,
                                            );
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
                                        }
                                    }
                                }
                                Some(Command::RepublishProviders) => {
                                    if republish_interval_ms > 0 {
                                        let now = now_ms();
                                        let keys: Vec<String> = provider_keys
                                            .iter()
                                            .filter_map(|(key, last_publish)| {
                                                if should_republish(*last_publish, now, republish_interval_ms) {
                                                    Some(key.clone())
                                                } else {
                                                    None
                                                }
                                            })
                                            .collect();
                                        for key in keys {
                                            let dht_key = RecordKey::new(&key);
                                            if swarm.behaviour_mut().kademlia.start_providing(dht_key).is_ok() {
                                                provider_keys.insert(key, now);
                                            }
                                        }
                                            if let Some(template) = peer_record_template.as_ref() {
                                                if peer_record_last_published_at_ms
                                                    .map(|last_ms| should_republish(last_ms, now, republish_interval_ms))
                                                    .unwrap_or(true)
                                                {
                                                if publish_configured_peer_record(
                                                    &mut swarm,
                                                    &mut pending_dht,
                                                    &keypair_clone,
                                                    template,
                                                    &event_listening_addrs,
                                                    None,
                                                )
                                                .is_ok()
                                                {
                                                    publish_discovery_provider(
                                                        &mut swarm,
                                                        &mut provider_keys,
                                                        template.world_id.as_str(),
                                                    );
                                                    peer_record_last_published_at_ms = Some(now);
                                                }
                                            }
                                        }
                                    }
                                }
                                Some(Command::Shutdown) | None => {
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
                                    push_bounded_inbox_message(
                                        &event_inbox,
                                        topic.as_str(),
                                        message.data,
                                        DEFAULT_SUBSCRIPTION_INBOX_MAX_MESSAGES,
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
                                                            &mut pending_discovery_peer_records,
                                                            &mut dialed_discovery_addrs,
                                                            peer_record_template.as_ref(),
                                                            local_peer_id,
                                                            &mut pending_connected_peer_records,
                                                            &mut pending_cached_peer_records,
                                                            &mut pending_cached_discovery_peers,
                                                            max_error_messages,
                                                            &event_errors,
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
                                                                &mut dialed_discovery_addrs,
                                                                peer_record_template.as_ref(),
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
                                            dial_routing_updated_addrs(
                                                &mut swarm,
                                                peer,
                                                addresses.iter(),
                                                &mut dialed_discovery_addrs,
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
                                                &mut dialed_discovery_addrs,
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
                                            swarm
                                                .behaviour_mut()
                                                .kademlia
                                                .add_address(&peer_id, address.clone());
                                        }
                                        libp2p::core::connection::ConnectedPoint::Listener { send_back_addr, .. } => {
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
                                }
                                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                                    peers.retain(|peer| peer != &peer_id);
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
                                }
                                SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
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
        }
    }

    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }

    pub fn published(&self) -> Vec<NetworkMessage> {
        self.published.lock().expect("lock published").clone()
    }

    pub fn dial(&self, addr: Multiaddr) -> Result<(), WorldError> {
        self.enqueue_command(Command::Dial { addr })
    }

    pub fn listening_addrs(&self) -> Vec<Multiaddr> {
        self.listening_addrs
            .lock()
            .expect("lock listening addrs")
            .clone()
    }

    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.connected_peers
            .lock()
            .expect("lock connected peers")
            .iter()
            .cloned()
            .collect()
    }

    pub fn debug_errors(&self) -> Vec<String> {
        self.errors.lock().expect("lock errors").clone()
    }

    fn enqueue_command(&self, command: Command) -> Result<(), WorldError> {
        try_send_command(&self.command_tx, command)
    }
}

impl Drop for Libp2pNetwork {
    fn drop(&mut self) {
        let _ = self.enqueue_command(Command::Shutdown);
    }
}

#[cfg(test)]
mod tests;
