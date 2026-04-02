//! Libp2p-based network adapter skeleton (gossipsub + request/response).

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

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
    dht_membership_key, dht_peer_record_key, dht_provider_key, dht_world_head_key,
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

const DEFAULT_COMMAND_BUFFER_CAPACITY: usize = 2048;
const DEFAULT_MAX_PUBLISHED_MESSAGES: usize = 4096;
const DEFAULT_MAX_ERROR_MESSAGES: usize = 4096;
const DEFAULT_MAX_LISTENING_ADDRS: usize = 128;

#[derive(Debug, Clone)]
pub struct Libp2pNetworkConfig {
    pub keypair: Option<Keypair>,
    pub peer_record: Option<PeerRecord>,
    pub listen_addrs: Vec<Multiaddr>,
    pub bootstrap_peers: Vec<Multiaddr>,
    pub republish_interval_ms: i64,
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
    RepublishProviders,
    Shutdown,
}

enum PendingDhtQuery {
    PublishProvider {
        response: Option<oneshot::Sender<Result<(), WorldError>>>,
    },
    GetProviders {
        response: Option<oneshot::Sender<Result<Vec<ProviderRecord>, WorldError>>>,
        providers: HashSet<PeerId>,
        error: Option<WorldError>,
    },
    PutWorldHead {
        response: Option<oneshot::Sender<Result<(), WorldError>>>,
    },
    GetWorldHead {
        response: Option<oneshot::Sender<Result<Option<WorldHeadAnnounce>, WorldError>>>,
        head: Option<WorldHeadAnnounce>,
        error: Option<WorldError>,
    },
    PutMembershipDirectory {
        response: Option<oneshot::Sender<Result<(), WorldError>>>,
    },
    GetMembershipDirectory {
        response: Option<oneshot::Sender<Result<Option<MembershipDirectorySnapshot>, WorldError>>>,
        snapshot: Option<MembershipDirectorySnapshot>,
        error: Option<WorldError>,
    },
    PutPeerRecord {
        response: Option<oneshot::Sender<Result<(), WorldError>>>,
    },
    GetPeerRecord {
        response: Option<oneshot::Sender<Result<Option<SignedPeerRecord>, WorldError>>>,
        record: Option<SignedPeerRecord>,
        error: Option<WorldError>,
    },
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
        let republish_tx = command_tx.clone();
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
            let mut pending_dht: HashMap<kad::QueryId, PendingDhtQuery> = HashMap::new();
            let mut peers: Vec<PeerId> = Vec::new();
            let mut provider_keys: HashMap<String, i64> = HashMap::new();
            let mut peer_record_last_published_at_ms = None;
            let republish_interval_ms = config_clone.republish_interval_ms;

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
                                                    let reply = if let Some(handler) = handlers.get(&request.protocol) {
                                                        handler(&request.payload)
                                                    } else {
                                                        Err(WorldError::NetworkProtocolUnavailable { protocol: request.protocol.clone() })
                                                    };
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
                                            if let Some(sender) = pending.remove(&request_id) {
                                                let _ = sender.send(Ok(response.payload));
                                            }
                                        }
                                            }
                                        }
                                        request_response::Event::OutboundFailure { request_id, error, .. } => {
                                            if let Some(sender) = pending.remove(&request_id) {
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
                                    if let kad::Event::OutboundQueryProgressed { id, result, step, .. } = event {
                                        if let Some(pending) = pending_dht.get_mut(&id) {
                                            handle_dht_progress(pending, result, step.last);
                                        }
                                        if step.last {
                                            pending_dht.remove(&id);
                                        }
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

fn publish_configured_peer_record(
    swarm: &mut Swarm<Behaviour>,
    pending_dht: &mut HashMap<kad::QueryId, PendingDhtQuery>,
    keypair: &Keypair,
    template: &PeerRecord,
    listening_addrs: &Arc<Mutex<Vec<Multiaddr>>>,
    response: Option<oneshot::Sender<Result<(), WorldError>>>,
) -> Result<(), WorldError> {
    let materialized = materialize_peer_record(template, listening_addrs);
    let signed = sign_peer_record(&materialized, keypair)?;
    let key = dht_peer_record_key(materialized.world_id.as_str(), materialized.peer_id.as_str());
    let payload = to_canonical_cbor(&signed)?;
    let query_id = put_record_query(swarm, key, payload)?;
    pending_dht.insert(query_id, PendingDhtQuery::PutPeerRecord { response });
    Ok(())
}

fn materialize_peer_record(
    template: &PeerRecord,
    listening_addrs: &Arc<Mutex<Vec<Multiaddr>>>,
) -> PeerRecord {
    let mut record = template.clone();
    if record.direct_addrs.is_empty() {
        record.direct_addrs = listening_addrs
            .lock()
            .expect("lock listening addrs")
            .iter()
            .map(ToString::to_string)
            .collect();
    }
    record.published_at_ms = now_ms();
    record
}

fn put_record_query(
    swarm: &mut Swarm<Behaviour>,
    key: String,
    payload: Vec<u8>,
) -> Result<kad::QueryId, WorldError> {
    let dht_key = RecordKey::new(&key);
    let record = kad::Record {
        key: dht_key,
        value: payload,
        publisher: None,
        expires: None,
    };
    swarm
        .behaviour_mut()
        .kademlia
        .put_record(record, Quorum::One)
        .map_err(|err| WorldError::NetworkProtocolUnavailable {
            protocol: format!("kad put_record failed: {err}"),
        })
}

fn sign_peer_record(record: &PeerRecord, keypair: &Keypair) -> Result<SignedPeerRecord, WorldError> {
    let mut record = record.clone();
    if record.peer_id.trim().is_empty() {
        record.peer_id = PeerId::from(keypair.public()).to_string();
    }
    let payload = encode_peer_record_signing_payload(&record)?;
    let signature = keypair
        .sign(payload.as_slice())
        .map_err(|err| WorldError::NetworkProtocolUnavailable {
            protocol: format!("peer record sign failed: {err}"),
        })?;
    Ok(SignedPeerRecord {
        record,
        identity_public_key_protobuf_hex: hex::encode(keypair.public().encode_protobuf()),
        signature_hex: hex::encode(signature),
    })
}

fn verify_signed_peer_record(record: &SignedPeerRecord) -> Result<(), WorldError> {
    let public_key_bytes = hex::decode(record.identity_public_key_protobuf_hex.as_str()).map_err(|_| {
        WorldError::NetworkProtocolUnavailable {
            protocol: "peer record public key must be valid hex".to_string(),
        }
    })?;
    let public_key = libp2p::identity::PublicKey::try_decode_protobuf(public_key_bytes.as_slice())
        .map_err(|err| WorldError::NetworkProtocolUnavailable {
            protocol: format!("peer record public key decode failed: {err}"),
        })?;
    if public_key.to_peer_id().to_string() != record.record.peer_id {
        return Err(WorldError::NetworkProtocolUnavailable {
            protocol: "peer record peer_id does not match identity public key".to_string(),
        });
    }
    let signature = hex::decode(record.signature_hex.as_str()).map_err(|_| {
        WorldError::NetworkProtocolUnavailable {
            protocol: "peer record signature must be valid hex".to_string(),
        }
    })?;
    let payload = encode_peer_record_signing_payload(&record.record)?;
    if !public_key.verify(payload.as_slice(), signature.as_slice()) {
        return Err(WorldError::NetworkProtocolUnavailable {
            protocol: "peer record signature verification failed".to_string(),
        });
    }
    Ok(())
}

fn encode_peer_record_signing_payload(record: &PeerRecord) -> Result<Vec<u8>, WorldError> {
    let mut payload = b"oasis7-peer-record-v1|".to_vec();
    payload.extend_from_slice(&to_canonical_cbor(record)?);
    Ok(payload)
}

fn handle_dht_progress(pending: &mut PendingDhtQuery, result: kad::QueryResult, is_last: bool) {
    match pending {
        PendingDhtQuery::PublishProvider { response } => {
            if is_last {
                let outcome = match result {
                    kad::QueryResult::StartProviding(Ok(_))
                    | kad::QueryResult::RepublishProvider(Ok(_)) => Ok(()),
                    kad::QueryResult::StartProviding(Err(err))
                    | kad::QueryResult::RepublishProvider(Err(err)) => {
                        Err(WorldError::NetworkProtocolUnavailable {
                            protocol: format!("kad start_providing failed: {err}"),
                        })
                    }
                    _ => Ok(()),
                };
                if let Some(response) = response.take() {
                    let _ = response.send(outcome);
                }
            }
        }
        PendingDhtQuery::PutWorldHead { response } => {
            if is_last {
                let outcome = match result {
                    kad::QueryResult::PutRecord(Ok(_))
                    | kad::QueryResult::RepublishRecord(Ok(_)) => Ok(()),
                    kad::QueryResult::PutRecord(Err(err))
                    | kad::QueryResult::RepublishRecord(Err(err)) => {
                        Err(WorldError::NetworkProtocolUnavailable {
                            protocol: format!("kad put_record failed: {err}"),
                        })
                    }
                    _ => Ok(()),
                };
                if let Some(response) = response.take() {
                    let _ = response.send(outcome);
                }
            }
        }
        PendingDhtQuery::PutMembershipDirectory { response } => {
            if is_last {
                let outcome = match result {
                    kad::QueryResult::PutRecord(Ok(_))
                    | kad::QueryResult::RepublishRecord(Ok(_)) => Ok(()),
                    kad::QueryResult::PutRecord(Err(err))
                    | kad::QueryResult::RepublishRecord(Err(err)) => {
                        Err(WorldError::NetworkProtocolUnavailable {
                            protocol: format!("kad put_record failed: {err}"),
                        })
                    }
                    _ => Ok(()),
                };
                if let Some(response) = response.take() {
                    let _ = response.send(outcome);
                }
            }
        }
        PendingDhtQuery::PutPeerRecord { response } => {
            if is_last {
                let outcome = match result {
                    kad::QueryResult::PutRecord(Ok(_))
                    | kad::QueryResult::RepublishRecord(Ok(_)) => Ok(()),
                    kad::QueryResult::PutRecord(Err(err))
                    | kad::QueryResult::RepublishRecord(Err(err)) => {
                        Err(WorldError::NetworkProtocolUnavailable {
                            protocol: format!("kad put_record failed: {err}"),
                        })
                    }
                    _ => Ok(()),
                };
                if let Some(response) = response.take() {
                    let _ = response.send(outcome);
                }
            }
        }
        PendingDhtQuery::GetProviders {
            response,
            providers,
            error,
        } => {
            match result {
                kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders {
                    providers: found,
                    ..
                })) => {
                    providers.extend(found);
                }
                kad::QueryResult::GetProviders(Ok(
                    kad::GetProvidersOk::FinishedWithNoAdditionalRecord { .. },
                )) => {}
                kad::QueryResult::GetProviders(Err(err)) => {
                    *error = Some(WorldError::NetworkProtocolUnavailable {
                        protocol: format!("kad get_providers failed: {err}"),
                    });
                }
                _ => {}
            }
            if is_last {
                let outcome = if !providers.is_empty() {
                    Ok(providers
                        .iter()
                        .map(|peer| ProviderRecord {
                            provider_id: peer.to_string(),
                            last_seen_ms: now_ms(),
                            storage_total_bytes: None,
                            storage_available_bytes: None,
                            uptime_ratio_per_mille: None,
                            challenge_pass_ratio_per_mille: None,
                            load_ratio_per_mille: None,
                            p50_read_latency_ms: None,
                        })
                        .collect())
                } else if let Some(err) = error.take() {
                    Err(err)
                } else {
                    Ok(Vec::new())
                };
                if let Some(response) = response.take() {
                    let _ = response.send(outcome);
                }
            }
        }
        PendingDhtQuery::GetWorldHead {
            response,
            head,
            error,
        } => {
            match result {
                kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(record))) => {
                    if let Ok(decoded) = decode_world_head(&record.record.value) {
                        *head = Some(decoded);
                    }
                }
                kad::QueryResult::GetRecord(Ok(
                    kad::GetRecordOk::FinishedWithNoAdditionalRecord { .. },
                )) => {}
                kad::QueryResult::GetRecord(Err(kad::GetRecordError::NotFound { .. })) => {
                    *error = None;
                }
                kad::QueryResult::GetRecord(Err(kad::GetRecordError::QuorumFailed {
                    records,
                    ..
                })) => {
                    if let Some(record) = records.first() {
                        if let Ok(decoded) = decode_world_head(&record.record.value) {
                            *head = Some(decoded);
                        }
                    } else {
                        *error = Some(WorldError::NetworkProtocolUnavailable {
                            protocol: "kad get_record quorum failed".to_string(),
                        });
                    }
                }
                kad::QueryResult::GetRecord(Err(err)) => {
                    *error = Some(WorldError::NetworkProtocolUnavailable {
                        protocol: format!("kad get_record failed: {err}"),
                    });
                }
                _ => {}
            }
            if is_last {
                let outcome = if head.is_some() {
                    Ok(head.clone())
                } else if let Some(err) = error.take() {
                    Err(err)
                } else {
                    Ok(None)
                };
                if let Some(response) = response.take() {
                    let _ = response.send(outcome);
                }
            }
        }
        PendingDhtQuery::GetMembershipDirectory {
            response,
            snapshot,
            error,
        } => {
            match result {
                kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(record))) => {
                    if let Ok(decoded) = decode_membership_directory(&record.record.value) {
                        *snapshot = Some(decoded);
                    }
                }
                kad::QueryResult::GetRecord(Ok(
                    kad::GetRecordOk::FinishedWithNoAdditionalRecord { .. },
                )) => {}
                kad::QueryResult::GetRecord(Err(kad::GetRecordError::NotFound { .. })) => {
                    *error = None;
                }
                kad::QueryResult::GetRecord(Err(kad::GetRecordError::QuorumFailed {
                    records,
                    ..
                })) => {
                    if let Some(record) = records.first() {
                        if let Ok(decoded) = decode_membership_directory(&record.record.value) {
                            *snapshot = Some(decoded);
                        }
                    } else {
                        *error = Some(WorldError::NetworkProtocolUnavailable {
                            protocol: "kad get_record quorum failed".to_string(),
                        });
                    }
                }
                kad::QueryResult::GetRecord(Err(err)) => {
                    *error = Some(WorldError::NetworkProtocolUnavailable {
                        protocol: format!("kad get_record failed: {err}"),
                    });
                }
                _ => {}
            }
            if is_last {
                let outcome = if snapshot.is_some() {
                    Ok(snapshot.clone())
                } else if let Some(err) = error.take() {
                    Err(err)
                } else {
                    Ok(None)
                };
                if let Some(response) = response.take() {
                    let _ = response.send(outcome);
                }
            }
        }
        PendingDhtQuery::GetPeerRecord {
            response,
            record,
            error,
        } => {
            match result {
                kad::QueryResult::GetRecord(Ok(kad::GetRecordOk::FoundRecord(found))) => {
                    match decode_peer_record(&found.record.value) {
                        Ok(decoded) => *record = Some(decoded),
                        Err(err) => *error = Some(err),
                    }
                }
                kad::QueryResult::GetRecord(Ok(
                    kad::GetRecordOk::FinishedWithNoAdditionalRecord { .. },
                )) => {}
                kad::QueryResult::GetRecord(Err(kad::GetRecordError::NotFound { .. })) => {
                    *error = None;
                }
                kad::QueryResult::GetRecord(Err(kad::GetRecordError::QuorumFailed {
                    records,
                    ..
                })) => {
                    if let Some(found) = records.first() {
                        match decode_peer_record(&found.record.value) {
                            Ok(decoded) => *record = Some(decoded),
                            Err(err) => *error = Some(err),
                        }
                    } else {
                        *error = Some(WorldError::NetworkProtocolUnavailable {
                            protocol: "kad get_record quorum failed".to_string(),
                        });
                    }
                }
                kad::QueryResult::GetRecord(Err(err)) => {
                    *error = Some(WorldError::NetworkProtocolUnavailable {
                        protocol: format!("kad get_record failed: {err}"),
                    });
                }
                _ => {}
            }
            if is_last {
                let outcome = if record.is_some() {
                    Ok(record.clone())
                } else if let Some(err) = error.take() {
                    Err(err)
                } else {
                    Ok(None)
                };
                if let Some(response) = response.take() {
                    let _ = response.send(outcome);
                }
            }
        }
    }
}

fn decode_world_head(bytes: &[u8]) -> Result<WorldHeadAnnounce, WorldError> {
    Ok(serde_cbor::from_slice(bytes)?)
}

fn decode_membership_directory(bytes: &[u8]) -> Result<MembershipDirectorySnapshot, WorldError> {
    Ok(serde_cbor::from_slice(bytes)?)
}

fn decode_peer_record(bytes: &[u8]) -> Result<SignedPeerRecord, WorldError> {
    let record: SignedPeerRecord = serde_cbor::from_slice(bytes)?;
    verify_signed_peer_record(&record)?;
    Ok(record)
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
