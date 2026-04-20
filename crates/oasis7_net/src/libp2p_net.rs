//! Libp2p-based network adapter skeleton (gossipsub + request/response).

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

mod api;
mod config;
mod connection_lifecycle;
mod constructor_support;
mod discovery;
mod error_mapping;
mod kad_queries;
mod peer_manager;
mod peer_manager_active_set;
mod peer_record;
mod peer_record_republish;
mod reachability;
mod runtime_loop;
mod swarm_behaviour;
mod swarm_reachability_events;
mod traffic_metrics;
mod transport_paths;
mod utils;

use futures::channel::mpsc;
use futures::{FutureExt, StreamExt};
use libp2p::gossipsub::{self, TopicHash};
use libp2p::identity::Keypair;
use libp2p::kad::{self};
use libp2p::relay;
use libp2p::rendezvous;
use libp2p::request_response::{self};
use libp2p::swarm::SwarmEvent;
use libp2p::{Multiaddr, PeerId};

use crate::{error::WorldError, util::to_canonical_cbor};
pub use config::Libp2pNetworkConfig;
use connection_lifecycle::{
    clear_disconnected_peer_state, failover_after_disconnect, record_established_connection,
    refresh_active_path_after_connection_close, refresh_peer_manager_views,
};
use constructor_support::{
    enqueue_initial_bootstrap_dials, schedule_bootstrap_redial,
    schedule_periodic_discovery_refresh, schedule_periodic_republish,
};
use discovery::{
    clear_pending_peer_record_request, handle_peer_record_response, handle_rendezvous_discovered,
    handle_request_response_request, maybe_discover_rendezvous_namespace,
    maybe_queue_discovery_peer_record, maybe_register_rendezvous_namespace,
    maybe_request_cached_discovery_peers, maybe_request_cached_peer_record,
    maybe_request_connected_peer_record, peer_record_enables_rendezvous,
    process_discovered_peer_record, publish_discovery_provider, start_peer_discovery_query,
    PendingPeerRecordRequest,
};
use error_mapping::error_response_from_world_error;
use kad_queries::{handle_dht_progress, DhtProgressAction, PendingDhtQuery};
use oasis7_proto::distributed::WorldHeadAnnounce;
use oasis7_proto::distributed_dht::{
    MembershipDirectorySnapshot, PeerRecord, ProviderRecord, SignedPeerRecord,
};
use oasis7_proto::distributed_net::{
    classify_network_protocol, classify_network_topic, push_bounded_inbox_message, NetworkMessage,
    NetworkRequest, NetworkResponse, DEFAULT_SUBSCRIPTION_INBOX_MAX_MESSAGES,
};
pub use peer_manager::{
    PeerManagerBlockArtifact, PeerManagerHealthIssue, PeerManagerHealthStatus,
    PeerManagerPeerHealth, PeerManagerPolicy,
};
use peer_record::{publish_configured_peer_record, put_record_query};
use peer_record_republish::{
    log_external_addr_confirmed_and_republish, log_external_addr_expired_and_republish,
    LocalPeerRecordRepublisher,
};
use reachability::{note_hole_punch_result, note_relay_reservation_accepted, snapshot_clone};
pub use reachability::{
    Libp2pReachabilitySnapshot, LiveAutoNatStatus, LiveHolePunchState, LivePublicPortReachability,
    LiveTransportKind,
};
#[cfg(test)]
use runtime_loop::{
    admitted_active_transport_paths, collect_quarantined_active_peers,
    filter_request_peers_by_health, filter_request_peers_by_lane,
};
use runtime_loop::{
    handle_command, Command, CommandContext, CommandOutcome, CommandStateRefs, PendingResponse,
};
use swarm_behaviour::{build_swarm, dial_addr_with_optional_peer_id, Behaviour, BehaviourEvent};
use swarm_reachability_events::{
    handle_autonat_event, handle_expired_listen_addr, handle_external_addr_candidate,
    handle_listener_closed, handle_new_listen_addr,
};
use traffic_metrics::{
    classify_control_plane_event, init_shared_traffic_metrics, record_control_plane_event,
    record_gossip_inbound, record_request_inbound, record_response_inbound,
    record_response_outbound, snapshot_traffic_metrics, SharedLibp2pTrafficMetrics,
};
pub use traffic_metrics::{
    Libp2pControlPlaneMetricsSnapshot, Libp2pTrafficMetricsSnapshot,
    TrafficDirectionMetricsSnapshot, TrafficLaneMetricsSnapshot,
};
use transport_paths::{retry_transport_path_after_error, TransportPath};
use utils::{
    decode_membership_directory, decode_world_head, now_ms, push_bounded_clone, should_republish,
    try_send_command,
};
const RR_GET_LOCAL_PEER_RECORD: &str = "/aw/rr/1.0.0/get_local_peer_record";
const RR_GET_CACHED_PEER_RECORD: &str = "/aw/rr/1.0.0/get_cached_peer_record";
const RR_GET_CACHED_DISCOVERY_PEERS: &str = "/aw/rr/1.0.0/get_cached_discovery_peers";
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
    peer_block_artifacts: Arc<Mutex<HashMap<String, PeerManagerBlockArtifact>>>,
    reachability: Arc<Mutex<Libp2pReachabilitySnapshot>>,
    traffic_metrics: SharedLibp2pTrafficMetrics,
}

type Handler = Arc<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>;

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
        let peer_block_artifacts = Arc::new(Mutex::new(
            HashMap::<String, PeerManagerBlockArtifact>::new(),
        ));
        let reachability = Arc::new(Mutex::new(Libp2pReachabilitySnapshot::default()));
        let traffic_metrics = init_shared_traffic_metrics();
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
        let event_peer_block_artifacts = Arc::clone(&peer_block_artifacts);
        let event_reachability = Arc::clone(&reachability);
        let event_traffic_metrics = Arc::clone(&traffic_metrics);
        let config_clone = config.clone();
        let keypair_clone = keypair.clone();
        let local_peer_id = peer_id;
        let republish_tx = command_tx.clone();
        let discovery_tx = command_tx.clone();
        let bootstrap_redial_tx = command_tx.clone();
        let bootstrap_peers = config.bootstrap_peers.clone();
        let bootstrap_redial_peers = bootstrap_peers.clone();
        let peer_record_template = config.peer_record.clone();
        let enable_rendezvous = config.enable_rendezvous
            || peer_record_template
                .as_ref()
                .map(peer_record_enables_rendezvous)
                .unwrap_or(false);

        std::thread::spawn(move || {
            let mut swarm = build_swarm(
                &keypair_clone,
                enable_rendezvous,
                config_clone.enable_autonat,
            );
            let mut subscriptions = HashSet::new();
            let mut topic_map: HashMap<TopicHash, String> = HashMap::new();
            let mut topic_inbox_limits: HashMap<String, usize> = HashMap::new();
            let mut handlers: HashMap<String, Handler> = HashMap::new();
            let mut pending: HashMap<request_response::OutboundRequestId, PendingResponse> =
                HashMap::new();
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
            let mut established_transport_paths_by_connection: HashMap<
                libp2p::swarm::ConnectionId,
                TransportPath,
            > = HashMap::new();
            let mut established_connections_by_peer: HashMap<
                PeerId,
                HashSet<libp2p::swarm::ConnectionId>,
            > = HashMap::new();
            let mut peer_healths_by_id: HashMap<PeerId, PeerManagerPeerHealth> = HashMap::new();
            let mut quarantined_active_peers: HashSet<PeerId> = HashSet::new();
            let mut admitted_active_peers: HashSet<PeerId> = HashSet::new();
            let mut failed_transport_path_labels: HashSet<String> = HashSet::new();
            let mut pending_quarantine_disconnects: HashSet<PeerId> = HashSet::new();
            let mut pending_discovery_peer_records: HashSet<PeerId> = HashSet::new();
            let mut pending_connected_peer_records: HashSet<PeerId> = HashSet::new();
            let mut pending_cached_peer_records: HashSet<PeerId> = HashSet::new();
            let mut pending_cached_discovery_peers: HashSet<PeerId> = HashSet::new();
            let mut connected_peer_record_cooldowns: HashMap<PeerId, i64> = HashMap::new();
            let mut cached_peer_record_cooldowns: HashMap<PeerId, i64> = HashMap::new();
            let mut cached_discovery_peer_cooldowns: HashMap<PeerId, i64> = HashMap::new();
            let mut pending_rendezvous_registers: HashSet<PeerId> = HashSet::new();
            let mut pending_rendezvous_discovers: HashSet<PeerId> = HashSet::new();
            let mut registered_rendezvous_nodes: HashSet<PeerId> = HashSet::new();
            let mut rendezvous_cookies: HashMap<PeerId, rendezvous::Cookie> = HashMap::new();
            let mut peer_record_last_published_at_ms = None;
            let bootstrap_redial_interval_ms = config_clone.bootstrap_redial_interval_ms;
            let republish_interval_ms = config_clone.republish_interval_ms;
            let discovery_query_interval_ms = config_clone.discovery_query_interval_ms;
            for addr in config_clone.listen_addrs {
                if let Err(err) = swarm.listen_on(addr) {
                    let msg = format!("libp2p listen failed: {err}");
                    push_bounded_clone(&event_errors, msg, max_error_messages, "lock errors");
                }
            }

            schedule_periodic_republish(republish_tx, republish_interval_ms);
            if peer_record_template.is_some() {
                schedule_periodic_discovery_refresh(discovery_tx, discovery_query_interval_ms);
            }
            schedule_bootstrap_redial(
                bootstrap_redial_tx,
                bootstrap_redial_peers,
                bootstrap_redial_interval_ms,
            );

            async_std::task::block_on(async move {
                let mut command_rx = command_rx;
                let command_ctx = CommandContext {
                    event_published: &event_published,
                    event_errors: &event_errors,
                    event_listening_addrs: &event_listening_addrs,
                    event_reachability: &event_reachability,
                    event_traffic_metrics: &event_traffic_metrics,
                    keypair: &keypair_clone,
                    peer_record_template: peer_record_template.as_ref(),
                    local_peer_id,
                    max_published_messages,
                    max_error_messages,
                    republish_interval_ms,
                    allow_loopback_external_addrs_for_testing: config_clone
                        .allow_loopback_external_addrs_for_testing,
                };
                let peer_record_republisher = LocalPeerRecordRepublisher::new(
                    &keypair_clone,
                    peer_record_template.as_ref(),
                    &event_listening_addrs,
                    &event_reachability,
                    (&event_errors, max_error_messages),
                    config_clone.allow_loopback_external_addrs_for_testing,
                );
                macro_rules! republish_local_peer_record {
                    () => {
                        peer_record_republisher.republish(
                            &mut swarm,
                            &mut pending_dht,
                            &mut provider_keys,
                            &mut peer_record_last_published_at_ms,
                        );
                    };
                }
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
                                    cached_discovery_peer_cooldowns: &mut cached_discovery_peer_cooldowns,
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
                            if let Some(kind) = classify_control_plane_event(&event) {
                                record_control_plane_event(&event_traffic_metrics, kind);
                            }
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
                                    record_gossip_inbound(
                                        &event_traffic_metrics,
                                        topic.as_str(),
                                        message.data.len(),
                                    );
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
                                                    record_request_inbound(
                                                        &event_traffic_metrics,
                                                        request.protocol.as_str(),
                                                        request.payload.len(),
                                                    );
                                                    let reply = handle_request_response_request(
                                                        &request,
                                                        &handlers,
                                                        peer_record_template.as_ref(),
                                                        &keypair_clone,
                                                        &event_listening_addrs,
                                                        &event_reachability,
                                                        config_clone
                                                            .allow_loopback_external_addrs_for_testing,
                                                        &discovered_peer_records,
                                                    );
                                                    let response_bytes = match reply {
                                                        Ok(bytes) => bytes,
                                                        Err(err) => to_canonical_cbor(
                                                            &error_response_from_world_error(&err),
                                                        )
                                                        .unwrap_or_default(),
                                                    };
                                                    record_response_outbound(
                                                        &event_traffic_metrics,
                                                        request.protocol.as_str(),
                                                        response_bytes.len(),
                                                    );
                                                    let response = NetworkResponse { payload: response_bytes };
                                                    swarm.behaviour_mut().request_response.send_response(channel, response).ok();
                                                }
                                                request_response::Message::Response { request_id, response } => {
                                                    if let Some(kind) = pending_peer_record_requests.remove(&request_id) {
                                                        record_response_inbound(
                                                            &event_traffic_metrics,
                                                            kind.protocol_label(),
                                                            response.payload.len(),
                                                        );
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
                                                            peers.as_slice(),
                                                            &event_traffic_metrics,
                                                            &mut failed_transport_path_labels,
                                                            &mut pending_discovery_peer_records,
                                                            &mut cached_peer_record_cooldowns,
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
                                                        ) = refresh_peer_manager_views(
                                                            &mut swarm,
                                                            &discovered_peer_records,
                                                            &active_transport_paths,
                                                            &admitted_active_peers,
                                                            &config_clone.peer_manager_policy,
                                                            &event_peer_healths,
                                                            &event_peer_block_artifacts,
                                                            &mut pending_quarantine_disconnects,
                                                            &event_errors,
                                                            max_error_messages,
                                                                &event_reachability,
                                                        );
                                                    } else if let Some(pending_response) = pending.remove(&request_id) {
                                                        record_response_inbound(
                                                            &event_traffic_metrics,
                                                            pending_response.protocol.as_str(),
                                                            response.payload.len(),
                                                        );
                                                        let _ = pending_response.response.send(Ok(response.payload));
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
                                                let _ = sender.response.send(Err(WorldError::NetworkProtocolUnavailable { protocol: format!("request failed: {error:?}") }));
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
                                                            &mut cached_peer_record_cooldowns,
                                                            &event_traffic_metrics,
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
                                                                    &mut cached_peer_record_cooldowns,
                                                                    &event_traffic_metrics,
                                                                    peers.as_slice(),
                                                                    peer_id,
                                                                    local_peer_id,
                                                                );
                                                            } else {
                                                                (
                                                                    peer_healths_by_id,
                                                                    quarantined_active_peers,
                                                                    admitted_active_peers,
                                                                ) = refresh_peer_manager_views(
                                                                    &mut swarm,
                                                                    &discovered_peer_records,
                                                                    &active_transport_paths,
                                                                    &admitted_active_peers,
                                                                    &config_clone.peer_manager_policy,
                                                                    &event_peer_healths,
                                                                    &event_peer_block_artifacts,
                                                                    &mut pending_quarantine_disconnects,
                                                                    &event_errors,
                                                                    max_error_messages,
                                                                    &event_reachability,
                                                                );
                                                            }
                                                        }
                                                        Ok(None) => {
                                                            maybe_request_cached_peer_record(
                                                                &mut swarm,
                                                                &mut pending_peer_record_requests,
                                                                &mut pending_cached_peer_records,
                                                                &mut cached_peer_record_cooldowns,
                                                                &event_traffic_metrics,
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
                                                                &mut cached_peer_record_cooldowns,
                                                                &event_traffic_metrics,
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
                                                    &mut connected_peer_record_cooldowns,
                                                    &event_traffic_metrics,
                                                    peer,
                                                    local_peer_id,
                                                );
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                SwarmEvent::Behaviour(BehaviourEvent::Autonat(event)) => {
                                    push_bounded_clone(
                                        &event_errors,
                                        handle_autonat_event(&event_reachability, &event),
                                        max_error_messages,
                                        "lock errors",
                                    );
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
                                            republish_local_peer_record!();
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
                                                &mut cached_peer_record_cooldowns,
                                                &event_traffic_metrics,
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
                                SwarmEvent::NewExternalAddrCandidate { address } => {
                                    push_bounded_clone(
                                        &event_errors,
                                        handle_external_addr_candidate(&address),
                                        max_error_messages,
                                        "lock errors",
                                    );
                                }
                                SwarmEvent::ExternalAddrConfirmed { address } => {
                                    log_external_addr_confirmed_and_republish(
                                        &event_errors,
                                        max_error_messages,
                                        &event_reachability,
                                        &address,
                                        &peer_record_republisher,
                                        &mut swarm,
                                        &mut pending_dht,
                                        &mut provider_keys,
                                        &mut peer_record_last_published_at_ms,
                                    );
                                }
                                SwarmEvent::ExternalAddrExpired { address } => {
                                    log_external_addr_expired_and_republish(
                                        &event_errors,
                                        max_error_messages,
                                        &event_reachability,
                                        &address,
                                        &peer_record_republisher,
                                        &mut swarm,
                                        &mut pending_dht,
                                        &mut provider_keys,
                                        &mut peer_record_last_published_at_ms,
                                    );
                                }
                                SwarmEvent::NewListenAddr { address, .. } => {
                                    handle_new_listen_addr(
                                        &mut swarm,
                                        &event_listening_addrs,
                                        &event_reachability,
                                        &address,
                                        config_clone.allow_loopback_external_addrs_for_testing,
                                        max_listening_addrs,
                                    );
                                    republish_local_peer_record!();
                                }
                                SwarmEvent::ExpiredListenAddr { address, .. } => {
                                    handle_expired_listen_addr(
                                        &mut swarm,
                                        &event_listening_addrs,
                                        &event_reachability,
                                        &address,
                                        config_clone.allow_loopback_external_addrs_for_testing,
                                    );
                                    republish_local_peer_record!();
                                }
                                SwarmEvent::ListenerClosed { addresses, .. } => {
                                    handle_listener_closed(
                                        &mut swarm,
                                        &event_listening_addrs,
                                        &event_reachability,
                                        addresses.as_slice(),
                                        config_clone.allow_loopback_external_addrs_for_testing,
                                    );
                                    republish_local_peer_record!();
                                }
                                SwarmEvent::ConnectionEstablished {
                                    peer_id,
                                    connection_id,
                                    endpoint,
                                    ..
                                } => {
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
                                    let (refreshed_active_path, dialed_addr) =
                                        record_established_connection(
                                            &known_transport_paths,
                                            &mut active_transport_paths,
                                            &mut last_dialed_transport_paths,
                                            &mut failed_transport_path_labels,
                                            &mut established_transport_paths_by_connection,
                                            &mut established_connections_by_peer,
                                            peer_id,
                                            connection_id,
                                            &endpoint,
                                        );
                                    if let Some(address) = dialed_addr {
                                        swarm
                                            .behaviour_mut()
                                            .kademlia
                                            .add_address(&peer_id, address);
                                    }
                                    if let Some(active_path) = refreshed_active_path {
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
                                        &mut connected_peer_record_cooldowns,
                                        &event_traffic_metrics,
                                        peer_id,
                                        local_peer_id,
                                    );
                                    maybe_request_cached_discovery_peers(
                                        &mut swarm,
                                        &mut pending_peer_record_requests,
                                        &mut pending_cached_discovery_peers,
                                        &mut cached_discovery_peer_cooldowns,
                                        &event_traffic_metrics,
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
                                    ) = refresh_peer_manager_views(
                                        &mut swarm,
                                        &discovered_peer_records,
                                        &active_transport_paths,
                                        &admitted_active_peers,
                                        &config_clone.peer_manager_policy,
                                        &event_peer_healths,
                                        &event_peer_block_artifacts,
                                        &mut pending_quarantine_disconnects,
                                        &event_errors,
                                        max_error_messages,
                                        &event_reachability,
                                    );
                                }
                                SwarmEvent::ConnectionClosed {
                                    peer_id,
                                    connection_id,
                                    num_established,
                                    ..
                                } => {
                                    if num_established > 0 {
                                        let refreshed_active_path =
                                            refresh_active_path_after_connection_close(
                                                &mut active_transport_paths,
                                                &mut established_transport_paths_by_connection,
                                                &mut established_connections_by_peer,
                                                peer_id,
                                                connection_id,
                                            );
                                        push_bounded_clone(
                                            &event_errors,
                                            format!(
                                                "libp2p connection closed peer={peer_id} num_established={num_established} active_path={}",
                                                refreshed_active_path
                                                    .as_ref()
                                                    .map(|path| path.addr.to_string())
                                                    .unwrap_or_else(|| "none".to_string())
                                            ),
                                            max_error_messages,
                                            "lock errors",
                                        );
                                    } else {
                                        established_transport_paths_by_connection
                                            .remove(&connection_id);
                                        established_connections_by_peer.remove(&peer_id);
                                        let quarantined = clear_disconnected_peer_state(
                                            &mut peers,
                                            &mut admitted_active_peers,
                                            &mut quarantined_active_peers,
                                            &mut pending_quarantine_disconnects,
                                            &mut active_transport_paths,
                                            &mut last_dialed_transport_paths,
                                            &mut connected_peer_record_cooldowns,
                                            &mut cached_peer_record_cooldowns,
                                            &mut cached_discovery_peer_cooldowns,
                                            &mut pending_rendezvous_registers,
                                            &mut pending_rendezvous_discovers,
                                            &mut registered_rendezvous_nodes,
                                            &mut rendezvous_cookies,
                                            &event_connected_peers,
                                            peer_id,
                                        );
                                        if quarantined {
                                            push_bounded_clone(
                                                &event_errors,
                                                format!(
                                                    "libp2p peer manager quarantine suppresses failover peer={peer_id}"
                                                ),
                                                max_error_messages,
                                                "lock errors",
                                            );
                                        } else {
                                            failover_after_disconnect(
                                                &mut swarm,
                                                &known_transport_paths,
                                                &mut active_transport_paths,
                                                &mut last_dialed_transport_paths,
                                                &mut failed_transport_path_labels,
                                                &event_errors,
                                                max_error_messages,
                                                peer_id,
                                            );
                                        }
                                        push_bounded_clone(
                                            &event_errors,
                                            format!("libp2p connection closed peer={peer_id}"),
                                            max_error_messages,
                                            "lock errors",
                                        );
                                    }
                                    (
                                        peer_healths_by_id,
                                        quarantined_active_peers,
                                        admitted_active_peers,
                                    ) = refresh_peer_manager_views(
                                        &mut swarm,
                                        &discovered_peer_records,
                                        &active_transport_paths,
                                        &admitted_active_peers,
                                        &config_clone.peer_manager_policy,
                                        &event_peer_healths,
                                        &event_peer_block_artifacts,
                                        &mut pending_quarantine_disconnects,
                                        &event_errors,
                                        max_error_messages,
                                        &event_reachability,
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
        enqueue_initial_bootstrap_dials(command_tx.clone(), bootstrap_peers);
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
            peer_block_artifacts,
            reachability,
            traffic_metrics,
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
#[cfg(test)]
mod transport_retry_tests;
