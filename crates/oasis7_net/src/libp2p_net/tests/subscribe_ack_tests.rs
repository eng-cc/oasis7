use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use futures::channel::oneshot;
use libp2p::{identity::Keypair, PeerId};

use super::super::runtime_loop::{
    handle_command, Command, CommandContext, CommandOutcome, CommandStateRefs,
};
use super::super::{Libp2pReachabilitySnapshot, DEFAULT_SUBSCRIPTION_INBOX_MAX_MESSAGES};

#[test]
fn handle_command_subscribe_acknowledges_success() {
    let keypair = Keypair::generate_ed25519();
    let mut swarm = super::super::swarm_behaviour::build_swarm(
        &keypair,
        false,
        true,
        super::super::wire_bytes::init_shared_wire_byte_counters(),
    );
    let event_published = Arc::new(Mutex::new(Vec::new()));
    let event_errors = Arc::new(Mutex::new(Vec::new()));
    let event_listening_addrs = Arc::new(Mutex::new(Vec::new()));
    let event_reachability = Arc::new(Mutex::new(Libp2pReachabilitySnapshot::default()));
    let event_traffic_metrics = super::super::traffic_metrics::init_shared_traffic_metrics();
    let mut subscriptions = HashSet::new();
    let mut topic_map = HashMap::new();
    let mut topic_inbox_limits = HashMap::new();
    let mut handlers = HashMap::new();
    let mut pending = HashMap::new();
    let mut pending_peer_record_requests = HashMap::new();
    let mut pending_dht = HashMap::new();
    let mut peers = Vec::new();
    let mut provider_keys = HashMap::new();
    let discovered_peer_records = HashMap::new();
    let peer_healths_by_id = HashMap::new();
    let mut pending_cached_discovery_peers = HashSet::new();
    let mut cached_discovery_peer_cooldowns = HashMap::new();
    let mut pending_rendezvous_registers = HashSet::new();
    let mut pending_rendezvous_discovers = HashSet::new();
    let registered_rendezvous_nodes = HashSet::new();
    let rendezvous_cookies = HashMap::new();
    let mut peer_record_last_published_at_ms = None;
    let mut peer_discovery_query_last_started_at_ms = None;
    let (sender, receiver) = oneshot::channel();

    let outcome = handle_command(
        &mut swarm,
        Some(Command::Subscribe {
            topic: "aw.handle-command".to_string(),
            response: sender,
        }),
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
            peer_discovery_query_last_started_at_ms: &mut peer_discovery_query_last_started_at_ms,
        },
        &CommandContext {
            event_published: &event_published,
            event_errors: &event_errors,
            event_listening_addrs: &event_listening_addrs,
            event_reachability: &event_reachability,
            event_traffic_metrics: &event_traffic_metrics,
            keypair: &keypair,
            peer_record_template: None,
            local_peer_id: PeerId::from(keypair.public()),
            max_published_messages: 8,
            max_error_messages: 8,
            republish_interval_ms: 1_000,
            discovery_query_cooldown_ms: 1_000,
            allow_loopback_external_addrs_for_testing: true,
        },
    );

    assert!(matches!(outcome, CommandOutcome::Continue));
    futures::executor::block_on(receiver)
        .expect("oneshot")
        .expect("subscribe ack");
    assert!(subscriptions.contains("aw.handle-command"));
    assert_eq!(topic_map.len(), 1);
    assert_eq!(
        topic_inbox_limits.get("aw.handle-command").copied(),
        Some(DEFAULT_SUBSCRIPTION_INBOX_MAX_MESSAGES)
    );
}
