use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use futures::channel::oneshot;
use libp2p::identity::Keypair;
use libp2p::PeerId;
use oasis7_proto::distributed_dht::{PeerDeploymentMode, PeerNodeRole, PeerRecord};

use super::peer_record::sign_peer_record;
use super::*;

fn signed_discovery_peer_record(
    keypair: &Keypair,
    discovery_sources: Vec<crate::dht::PeerDiscoverySource>,
    published_at_ms: i64,
) -> SignedPeerRecord {
    let peer_id = PeerId::from(keypair.public());
    sign_peer_record(
        &PeerRecord {
            peer_id: peer_id.to_string(),
            node_id: format!("node-{peer_id}"),
            world_id: "world-a".to_string(),
            network_id: "network-a".to_string(),
            node_role: PeerNodeRole::FullStorage.as_str().to_string(),
            deployment_mode: PeerDeploymentMode::Hybrid,
            reachability_class: crate::dht::PeerReachabilityClass::Hybrid,
            direct_addrs: vec!["/ip4/127.0.0.1/udp/4103/quic-v1".to_string()],
            hole_punch_addrs: Vec::new(),
            relay_addrs: Vec::new(),
            discovery_sources,
            capability_lanes: PeerNodeRole::FullStorage.default_capability_lanes(),
            source_operator: None,
            source_asn: None,
            published_at_ms,
            ttl_ms: 60_000,
        },
        keypair,
    )
    .expect("sign discovery peer record")
}

#[test]
fn process_discovered_peer_record_retries_failed_candidate_dial_on_rediscovery() {
    let mut swarm = super::swarm_behaviour::build_swarm(&Keypair::generate_ed25519(), false, true);
    let peer_key = Keypair::generate_ed25519();
    let record = signed_discovery_peer_record(
        &peer_key,
        vec![
            crate::dht::PeerDiscoverySource::Dht,
            crate::dht::PeerDiscoverySource::Rendezvous,
        ],
        1,
    );
    let peer_id = PeerId::from(peer_key.public());
    let mut discovered_peer_records = HashMap::new();
    let mut known_transport_paths = HashMap::new();
    let mut last_dialed_transport_paths = HashMap::new();
    let active_transport_paths = HashMap::new();
    let mut failed_transport_path_labels = HashSet::new();

    super::discovery::process_discovered_peer_record(
        &mut swarm,
        &mut discovered_peer_records,
        &mut known_transport_paths,
        &mut last_dialed_transport_paths,
        &active_transport_paths,
        &mut failed_transport_path_labels,
        None,
        &PeerManagerPolicy::default(),
        record.clone(),
    )
    .expect("first discovery dial");
    assert!(last_dialed_transport_paths.contains_key(&peer_id));

    last_dialed_transport_paths.remove(&peer_id);
    let mut retry_swarm =
        super::swarm_behaviour::build_swarm(&Keypair::generate_ed25519(), false, true);

    super::discovery::process_discovered_peer_record(
        &mut retry_swarm,
        &mut discovered_peer_records,
        &mut known_transport_paths,
        &mut last_dialed_transport_paths,
        &active_transport_paths,
        &mut failed_transport_path_labels,
        None,
        &PeerManagerPolicy::default(),
        record,
    )
    .expect("rediscovery must retry dial");

    assert!(last_dialed_transport_paths.contains_key(&peer_id));
}

#[test]
fn request_with_providers_does_not_fallback_outside_provider_subset() {
    let keypair = Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(keypair.public());
    let mut swarm = super::swarm_behaviour::build_swarm(&keypair, false, true);
    let (sender, receiver) = oneshot::channel();
    let mut subscriptions = HashSet::new();
    let mut topic_map = HashMap::new();
    let mut topic_inbox_limits = HashMap::new();
    let mut handlers = HashMap::new();
    let mut pending = HashMap::new();
    let mut pending_peer_record_requests = HashMap::new();
    let mut pending_dht = HashMap::new();
    let mut peers = vec![PeerId::random()];
    let mut provider_keys = HashMap::new();
    let discovered_peer_records = HashMap::new();
    let peer_healths_by_id = HashMap::new();
    let mut pending_cached_discovery_peers = HashSet::new();
    let mut pending_rendezvous_registers = HashSet::new();
    let mut pending_rendezvous_discovers = HashSet::new();
    let registered_rendezvous_nodes = HashSet::new();
    let rendezvous_cookies = HashMap::new();
    let mut peer_record_last_published_at_ms = None;
    let event_published = Arc::new(Mutex::new(Vec::new()));
    let event_errors = Arc::new(Mutex::new(Vec::new()));
    let event_listening_addrs = Arc::new(Mutex::new(Vec::new()));
    let event_reachability = Arc::new(Mutex::new(Libp2pReachabilitySnapshot::default()));
    let event_traffic_metrics = super::traffic_metrics::init_shared_traffic_metrics();
    let provider_outside_subset = PeerId::random();

    let outcome = super::runtime_loop::handle_command(
        &mut swarm,
        Some(Command::Request {
            protocol: "/aw/rr/1.0.0/fetch_blob".to_string(),
            payload: Vec::new(),
            providers: vec![provider_outside_subset.to_string()],
            response: sender,
        }),
        super::runtime_loop::CommandStateRefs {
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
        &super::CommandContext {
            event_published: &event_published,
            event_errors: &event_errors,
            event_listening_addrs: &event_listening_addrs,
            event_reachability: &event_reachability,
            event_traffic_metrics: &event_traffic_metrics,
            keypair: &keypair,
            peer_record_template: None,
            local_peer_id,
            max_published_messages: 8,
            max_error_messages: 8,
            republish_interval_ms: 0,
            allow_loopback_external_addrs_for_testing: false,
        },
    );
    assert!(matches!(outcome, super::CommandOutcome::Continue));

    let err = futures::executor::block_on(receiver)
        .expect("request response")
        .expect_err("provider subset mismatch must fail");
    assert!(matches!(
        err,
        WorldError::NetworkProtocolUnavailable { protocol }
            if protocol.contains("no connected providers")
    ));
}
