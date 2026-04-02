//! Libp2p-based network adapter skeleton (gossipsub + request/response).

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

mod kad_queries;
mod peer_record;

use futures::channel::{mpsc, oneshot};
use futures::{FutureExt, StreamExt};
use libp2p::gossipsub::{self, IdentTopic, MessageAuthenticity, TopicHash};
use libp2p::identity::Keypair;
use libp2p::kad::{self, store::MemoryStore, Quorum, RecordKey};
use libp2p::noise;
use libp2p::request_response::{self, ProtocolSupport};
use libp2p::swarm::{NetworkBehaviour, Swarm, SwarmEvent};
use libp2p::{Multiaddr, PeerId, StreamProtocol, Transport as _};
use oasis7_proto::distributed_dht::DistributedDht as ProtoDistributedDht;
use oasis7_proto::distributed_net::DistributedNetwork as ProtoDistributedNetwork;

use crate::error::WorldError;
use oasis7_proto::distributed::{
    dht_membership_key, dht_peer_discovery_key, dht_peer_record_key, dht_provider_key,
    dht_world_head_key,
    DistributedErrorCode, ErrorResponse, WorldHeadAnnounce, RR_PROTOCOL_PREFIX,
};
use oasis7_proto::distributed_dht::{
    MembershipDirectorySnapshot, PeerRecord, ProviderRecord, SignedPeerRecord,
};
use oasis7_proto::distributed_net::{
    push_bounded_inbox_message, NetworkMessage, NetworkRequest, NetworkResponse,
    NetworkSubscription, DEFAULT_SUBSCRIPTION_INBOX_MAX_MESSAGES,
};

use crate::util::{to_canonical_cbor, unix_now_ms_i64};
use kad_queries::{handle_dht_progress, DhtProgressAction, PendingDhtQuery};
use peer_record::{
    build_configured_peer_record, peer_record_dial_addrs, publish_configured_peer_record,
    put_record_query,
    validate_discovered_peer_record,
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

enum PendingPeerRecordRequest {
    ConnectedPeerRecord { peer_id: PeerId },
    CachedPeerRecord { peer_id: PeerId },
    CachedDiscoveryPeers { peer_id: PeerId },
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
                                SwarmEvent::NewListenAddr { address, .. } => {
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
                                        start_peer_discovery_query(
                                            &mut swarm,
                                            &mut pending_dht,
                                            template,
                                        );
                                    }
                                }
                                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                                    peers.retain(|peer| peer != &peer_id);
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

impl ProtoDistributedNetwork<WorldError> for Libp2pNetwork {
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        self.enqueue_command(Command::Publish {
            topic: topic.to_string(),
            payload: payload.to_vec(),
        })
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        self.enqueue_command(Command::Subscribe {
            topic: topic.to_string(),
        })?;
        Ok(NetworkSubscription::new(
            topic.to_string(),
            Arc::clone(&self.inbox),
        ))
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        self.request_with_providers(protocol, payload, &[])
    }

    fn request_with_providers(
        &self,
        protocol: &str,
        payload: &[u8],
        providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        let (sender, receiver) = oneshot::channel();
        self.enqueue_command(Command::Request {
            protocol: protocol.to_string(),
            payload: payload.to_vec(),
            providers: providers.to_vec(),
            response: sender,
        })?;
        futures::executor::block_on(receiver).map_err(|_| {
            WorldError::NetworkProtocolUnavailable {
                protocol: "libp2p".to_string(),
            }
        })?
    }

    fn register_handler(
        &self,
        protocol: &str,
        handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        self.enqueue_command(Command::RegisterHandler {
            protocol: protocol.to_string(),
            handler: Arc::from(handler),
        })
    }
}

impl ProtoDistributedDht<WorldError> for Libp2pNetwork {
    fn publish_provider(
        &self,
        world_id: &str,
        content_hash: &str,
        _provider_id: &str,
    ) -> Result<(), WorldError> {
        let key = dht_provider_key(world_id, content_hash);
        let (sender, receiver) = oneshot::channel();
        self.enqueue_command(Command::PublishProvider {
            key,
            response: sender,
        })?;
        futures::executor::block_on(receiver).map_err(|_| {
            WorldError::NetworkProtocolUnavailable {
                protocol: "libp2p".to_string(),
            }
        })?
    }

    fn get_providers(
        &self,
        world_id: &str,
        content_hash: &str,
    ) -> Result<Vec<ProviderRecord>, WorldError> {
        let key = dht_provider_key(world_id, content_hash);
        let (sender, receiver) = oneshot::channel();
        self.enqueue_command(Command::GetProviders {
            key,
            response: sender,
        })?;
        futures::executor::block_on(receiver).map_err(|_| {
            WorldError::NetworkProtocolUnavailable {
                protocol: "libp2p".to_string(),
            }
        })?
    }

    fn put_world_head(&self, world_id: &str, head: &WorldHeadAnnounce) -> Result<(), WorldError> {
        let key = dht_world_head_key(world_id);
        let payload = to_canonical_cbor(head)?;
        let (sender, receiver) = oneshot::channel();
        self.enqueue_command(Command::PutWorldHead {
            key,
            payload,
            response: sender,
        })?;
        futures::executor::block_on(receiver).map_err(|_| {
            WorldError::NetworkProtocolUnavailable {
                protocol: "libp2p".to_string(),
            }
        })?
    }

    fn get_world_head(&self, world_id: &str) -> Result<Option<WorldHeadAnnounce>, WorldError> {
        let key = dht_world_head_key(world_id);
        let (sender, receiver) = oneshot::channel();
        self.enqueue_command(Command::GetWorldHead {
            key,
            response: sender,
        })?;
        futures::executor::block_on(receiver).map_err(|_| {
            WorldError::NetworkProtocolUnavailable {
                protocol: "libp2p".to_string(),
            }
        })?
    }

    fn put_membership_directory(
        &self,
        world_id: &str,
        snapshot: &MembershipDirectorySnapshot,
    ) -> Result<(), WorldError> {
        let key = dht_membership_key(world_id);
        let payload = to_canonical_cbor(snapshot)?;
        let (sender, receiver) = oneshot::channel();
        self.enqueue_command(Command::PutMembershipDirectory {
            key,
            payload,
            response: sender,
        })?;
        futures::executor::block_on(receiver).map_err(|_| {
            WorldError::NetworkProtocolUnavailable {
                protocol: "libp2p".to_string(),
            }
        })?
    }

    fn get_membership_directory(
        &self,
        world_id: &str,
    ) -> Result<Option<MembershipDirectorySnapshot>, WorldError> {
        let key = dht_membership_key(world_id);
        let (sender, receiver) = oneshot::channel();
        self.enqueue_command(Command::GetMembershipDirectory {
            key,
            response: sender,
        })?;
        futures::executor::block_on(receiver).map_err(|_| {
            WorldError::NetworkProtocolUnavailable {
                protocol: "libp2p".to_string(),
            }
        })?
    }

    fn put_peer_record(&self, world_id: &str, record: &SignedPeerRecord) -> Result<(), WorldError> {
        let key = dht_peer_record_key(world_id, record.record.peer_id.as_str());
        let payload = to_canonical_cbor(record)?;
        let (sender, receiver) = oneshot::channel();
        self.enqueue_command(Command::PutPeerRecord {
            key,
            payload,
            response: sender,
        })?;
        futures::executor::block_on(receiver).map_err(|_| {
            WorldError::NetworkProtocolUnavailable {
                protocol: "libp2p".to_string(),
            }
        })?
    }

    fn get_peer_record(
        &self,
        world_id: &str,
        peer_id: &str,
    ) -> Result<Option<SignedPeerRecord>, WorldError> {
        let key = dht_peer_record_key(world_id, peer_id);
        let (sender, receiver) = oneshot::channel();
        self.enqueue_command(Command::GetPeerRecord {
            key,
            response: sender,
        })?;
        futures::executor::block_on(receiver).map_err(|_| {
            WorldError::NetworkProtocolUnavailable {
                protocol: "libp2p".to_string(),
            }
        })?
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "BehaviourEvent")]
struct Behaviour {
    gossipsub: gossipsub::Behaviour,
    request_response: request_response::cbor::Behaviour<NetworkRequest, NetworkResponse>,
    kademlia: kad::Behaviour<MemoryStore>,
}

#[derive(Debug)]
enum BehaviourEvent {
    Gossipsub(gossipsub::Event),
    RequestResponse(request_response::Event<NetworkRequest, NetworkResponse>),
    Kademlia(kad::Event),
}

impl From<gossipsub::Event> for BehaviourEvent {
    fn from(event: gossipsub::Event) -> Self {
        BehaviourEvent::Gossipsub(event)
    }
}

impl From<request_response::Event<NetworkRequest, NetworkResponse>> for BehaviourEvent {
    fn from(event: request_response::Event<NetworkRequest, NetworkResponse>) -> Self {
        BehaviourEvent::RequestResponse(event)
    }
}

impl From<kad::Event> for BehaviourEvent {
    fn from(event: kad::Event) -> Self {
        BehaviourEvent::Kademlia(event)
    }
}

fn build_swarm(keypair: &Keypair) -> Swarm<Behaviour> {
    let swarm_config = libp2p::swarm::Config::with_async_std_executor()
        .with_idle_connection_timeout(std::time::Duration::from_secs(30));

    let peer_id = PeerId::from(keypair.public());
    let gossipsub = gossipsub::Behaviour::new(
        MessageAuthenticity::Signed(keypair.clone()),
        gossipsub::Config::default(),
    )
    .expect("gossipsub config");

    let protocols = vec![(
        StreamProtocol::new(RR_PROTOCOL_PREFIX),
        ProtocolSupport::Full,
    )];
    let request_response =
        request_response::cbor::Behaviour::new(protocols, request_response::Config::default());

    let store = MemoryStore::new(peer_id);
    let kademlia = kad::Behaviour::new(peer_id, store);

    let behaviour = Behaviour {
        gossipsub,
        request_response,
        kademlia,
    };

    let transport = libp2p::tcp::async_io::Transport::new(libp2p::tcp::Config::default())
        .upgrade(libp2p::core::upgrade::Version::V1)
        .authenticate(noise::Config::new(keypair).expect("noise config"))
        .multiplex(libp2p::yamux::Config::default())
        .boxed();

    Swarm::new(transport, behaviour, peer_id, swarm_config)
}

fn dial_addr_with_optional_peer_id(
    swarm: &mut Swarm<Behaviour>,
    addr: Multiaddr,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (peer_id, dial_addr) = split_peer_id(addr);
    if let Some(peer_id) = peer_id {
        swarm
            .behaviour_mut()
            .kademlia
            .add_address(&peer_id, dial_addr.clone());
        let opts = libp2p::swarm::dial_opts::DialOpts::peer_id(peer_id)
            .addresses(vec![dial_addr])
            .build();
        swarm.dial(opts)?;
    } else {
        swarm.dial(dial_addr)?;
    }
    Ok(())
}

fn split_peer_id(mut addr: Multiaddr) -> (Option<PeerId>, Multiaddr) {
    use libp2p::multiaddr::Protocol;

    let peer_id = match addr.pop() {
        Some(Protocol::P2p(peer)) => Some(peer),
        Some(protocol) => {
            addr.push(protocol);
            None
        }
        None => None,
    };
    (peer_id, addr)
}

fn start_peer_discovery_query(
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

fn publish_discovery_provider(
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
        provider_keys.insert(key, now_ms());
    }
}

fn maybe_queue_discovery_peer_record(
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
    let query_id = swarm.behaviour_mut().kademlia.get_record(RecordKey::new(&key));
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

fn maybe_request_connected_peer_record(
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
    let request_id = swarm
        .behaviour_mut()
        .request_response
        .send_request(
            &peer_id,
            NetworkRequest {
                protocol: RR_GET_LOCAL_PEER_RECORD.to_string(),
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
    let request_id = swarm
        .behaviour_mut()
        .request_response
        .send_request(
            &ask_peer,
            NetworkRequest {
                protocol: RR_GET_CACHED_PEER_RECORD.to_string(),
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

fn maybe_request_cached_peer_record(
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

fn maybe_request_cached_discovery_peers(
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
    let request_id = swarm
        .behaviour_mut()
        .request_response
        .send_request(
            &peer_id,
            NetworkRequest {
                protocol: RR_GET_CACHED_DISCOVERY_PEERS.to_string(),
                payload: Vec::new(),
            },
        );
    pending_cached_discovery_peers.insert(peer_id);
    pending_peer_record_requests.insert(
        request_id,
        PendingPeerRecordRequest::CachedDiscoveryPeers { peer_id },
    );
}

fn process_discovered_peer_record(
    swarm: &mut Swarm<Behaviour>,
    discovered_peer_records: &mut HashMap<PeerId, SignedPeerRecord>,
    dialed_discovery_addrs: &mut HashSet<String>,
    template: Option<&PeerRecord>,
    record: SignedPeerRecord,
) -> Result<(), WorldError> {
    validate_discovered_peer_record(&record, template)?;
    let peer_id = record
        .record
        .peer_id
        .parse::<PeerId>()
        .map_err(|_| WorldError::NetworkProtocolUnavailable {
            protocol: "peer record peer_id must be valid".to_string(),
        })?;
    for addr in peer_record_dial_addrs(&record) {
        let (_, kad_addr) = split_peer_id(addr.clone());
        swarm
            .behaviour_mut()
            .kademlia
            .add_address(&peer_id, kad_addr);
        let addr_label = addr.to_string();
        if dialed_discovery_addrs.insert(addr_label.clone()) {
            let _ = dial_addr_with_optional_peer_id(swarm, addr);
        }
    }
    discovered_peer_records.insert(peer_id, record);
    Ok(())
}

fn handle_request_response_request(
    request: &NetworkRequest,
    handlers: &HashMap<String, Handler>,
    peer_record_template: Option<&PeerRecord>,
    keypair: &Keypair,
    listening_addrs: &Arc<Mutex<Vec<Multiaddr>>>,
    discovered_peer_records: &HashMap<PeerId, SignedPeerRecord>,
) -> Result<Vec<u8>, WorldError> {
    match request.protocol.as_str() {
        RR_GET_LOCAL_PEER_RECORD => {
            let Some(template) = peer_record_template else {
                return Err(WorldError::NetworkProtocolUnavailable {
                    protocol: RR_GET_LOCAL_PEER_RECORD.to_string(),
                });
            };
            let record = build_configured_peer_record(keypair, template, listening_addrs)?;
            to_canonical_cbor(&record)
        }
        RR_GET_CACHED_PEER_RECORD => {
            let peer_id = String::from_utf8(request.payload.clone())
                .map_err(|_| WorldError::NetworkProtocolUnavailable {
                    protocol: "cached peer record payload must be utf-8".to_string(),
                })?
                .parse::<PeerId>()
                .map_err(|_| {
                WorldError::NetworkProtocolUnavailable {
                    protocol: "cached peer record peer_id must be valid".to_string(),
                }
            })?;
            let record = discovered_peer_records.get(&peer_id).ok_or_else(|| {
                WorldError::NetworkProtocolUnavailable {
                    protocol: RR_GET_CACHED_PEER_RECORD.to_string(),
                }
            })?;
            to_canonical_cbor(record)
        }
        RR_GET_CACHED_DISCOVERY_PEERS => {
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

fn handle_peer_record_response(
    swarm: &mut Swarm<Behaviour>,
    kind: PendingPeerRecordRequest,
    payload: &[u8],
    pending_peer_record_requests: &mut HashMap<
        request_response::OutboundRequestId,
        PendingPeerRecordRequest,
    >,
    pending_dht: &mut HashMap<kad::QueryId, PendingDhtQuery>,
    discovered_peer_records: &mut HashMap<PeerId, SignedPeerRecord>,
    pending_discovery_peer_records: &mut HashSet<PeerId>,
    dialed_discovery_addrs: &mut HashSet<String>,
    peer_record_template: Option<&PeerRecord>,
    local_peer_id: PeerId,
    pending_connected_peer_records: &mut HashSet<PeerId>,
    pending_cached_peer_records: &mut HashSet<PeerId>,
    pending_cached_discovery_peers: &mut HashSet<PeerId>,
    max_error_messages: usize,
    event_errors: &Arc<Mutex<Vec<String>>>,
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
                dialed_discovery_addrs,
                peer_record_template,
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

fn clear_pending_peer_record_request(
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

fn decode_optional_peer_record_response(payload: &[u8]) -> Result<Option<SignedPeerRecord>, WorldError> {
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

fn decode_cached_discovery_peers_response(payload: &[u8]) -> Result<Vec<PeerId>, WorldError> {
    let peer_ids: Vec<String> = serde_cbor::from_slice(payload)?;
    let mut decoded = Vec::with_capacity(peer_ids.len());
    for peer_id in peer_ids {
        let peer_id = peer_id
            .parse::<PeerId>()
            .map_err(|_| WorldError::NetworkProtocolUnavailable {
                protocol: "cached discovery peer_id must be valid".to_string(),
            })?;
        decoded.push(peer_id);
    }
    Ok(decoded)
}

fn republish_cached_peer_record(
    swarm: &mut Swarm<Behaviour>,
    pending_dht: &mut HashMap<kad::QueryId, PendingDhtQuery>,
    record: &SignedPeerRecord,
) -> Result<(), WorldError> {
    let key = dht_peer_record_key(record.record.world_id.as_str(), record.record.peer_id.as_str());
    let payload = to_canonical_cbor(record)?;
    let query_id = put_record_query(swarm, key, payload)?;
    pending_dht.insert(
        query_id,
        PendingDhtQuery::PutPeerRecord { response: None },
    );
    Ok(())
}

fn dial_routing_updated_addrs<'a>(
    swarm: &mut Swarm<Behaviour>,
    peer_id: PeerId,
    addrs: impl Iterator<Item = &'a Multiaddr>,
    dialed_discovery_addrs: &mut HashSet<String>,
) {
    for addr in addrs.cloned().map(|addr| ensure_peer_id(addr, peer_id)) {
        let addr_label = addr.to_string();
        if dialed_discovery_addrs.insert(addr_label) {
            let _ = dial_addr_with_optional_peer_id(swarm, addr);
        }
    }
}

fn ensure_peer_id(mut addr: Multiaddr, peer_id: PeerId) -> Multiaddr {
    use libp2p::multiaddr::Protocol;

    let needs_peer_id = !matches!(addr.iter().last(), Some(Protocol::P2p(_)));
    if needs_peer_id {
        addr.push(Protocol::P2p(peer_id.into()));
    }
    addr
}

fn decode_world_head(bytes: &[u8]) -> Result<WorldHeadAnnounce, WorldError> {
    Ok(serde_cbor::from_slice(bytes)?)
}

fn decode_membership_directory(bytes: &[u8]) -> Result<MembershipDirectorySnapshot, WorldError> {
    Ok(serde_cbor::from_slice(bytes)?)
}

fn now_ms() -> i64 {
    unix_now_ms_i64()
}

fn try_send_command(
    command_tx: &mpsc::Sender<Command>,
    command: Command,
) -> Result<(), WorldError> {
    let mut sender = command_tx.clone();
    sender
        .try_send(command)
        .map_err(|err| WorldError::NetworkProtocolUnavailable {
            protocol: if err.is_full() {
                "libp2p command queue saturated".to_string()
            } else {
                "libp2p command queue disconnected".to_string()
            },
        })
}

fn push_bounded_clone<T: Clone>(
    values: &Arc<Mutex<Vec<T>>>,
    value: T,
    max_len: usize,
    lock_label: &str,
) {
    let mut guard = values.lock().expect(lock_label);
    push_bounded_vec(&mut guard, value, max_len);
}

fn push_bounded_vec<T>(values: &mut Vec<T>, value: T, max_len: usize) {
    let max_len = max_len.max(1);
    values.push(value);
    let overflow = values.len().saturating_sub(max_len);
    if overflow > 0 {
        values.drain(0..overflow);
    }
}

fn should_republish(last_ms: i64, now_ms: i64, interval_ms: i64) -> bool {
    if interval_ms <= 0 {
        return false;
    }
    now_ms.saturating_sub(last_ms) >= interval_ms
}

#[cfg(test)]
mod tests;
