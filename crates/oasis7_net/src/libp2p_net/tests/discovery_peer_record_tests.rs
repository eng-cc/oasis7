use super::*;

#[test]
fn process_discovered_peer_record_keeps_single_source_bootstrap_peer_dial_eligible() {
    let mut swarm = super::super::swarm_behaviour::build_swarm(
        &Keypair::generate_ed25519(),
        false,
        true,
        super::super::wire_bytes::init_shared_wire_byte_counters(),
    );
    let peer_key = Keypair::generate_ed25519();
    let peer_id = PeerId::from(peer_key.public());
    let suspect_record = super::signed_discovery_peer_record(
        &peer_key,
        vec![crate::dht::PeerDiscoverySource::StaticBootstrap],
        1,
    );
    let upgraded_record = super::signed_discovery_peer_record(
        &peer_key,
        vec![
            crate::dht::PeerDiscoverySource::Dht,
            crate::dht::PeerDiscoverySource::Rendezvous,
        ],
        2,
    );
    let mut discovered_peer_records = HashMap::new();
    let mut known_transport_paths = HashMap::new();
    let mut last_dialed_transport_paths = HashMap::new();
    let active_transport_paths = HashMap::new();
    let mut failed_transport_path_labels = HashSet::new();

    super::super::discovery::process_discovered_peer_record(
        &mut swarm,
        &mut discovered_peer_records,
        &mut known_transport_paths,
        &mut last_dialed_transport_paths,
        &active_transport_paths,
        &mut failed_transport_path_labels,
        None,
        &PeerManagerPolicy::default(),
        suspect_record,
    )
    .expect("process suspect peer record");

    assert!(discovered_peer_records.contains_key(&peer_id));
    assert!(last_dialed_transport_paths.contains_key(&peer_id));

    super::super::discovery::process_discovered_peer_record(
        &mut swarm,
        &mut discovered_peer_records,
        &mut known_transport_paths,
        &mut last_dialed_transport_paths,
        &active_transport_paths,
        &mut failed_transport_path_labels,
        None,
        &PeerManagerPolicy::default(),
        upgraded_record,
    )
    .expect("process upgraded peer record");

    assert!(last_dialed_transport_paths.contains_key(&peer_id));
}

#[test]
fn process_discovered_peer_record_keeps_dht_only_suspect_peer_non_dialable() {
    let mut swarm = super::super::swarm_behaviour::build_swarm(
        &Keypair::generate_ed25519(),
        false,
        true,
        super::super::wire_bytes::init_shared_wire_byte_counters(),
    );
    let peer_key = Keypair::generate_ed25519();
    let peer_id = PeerId::from(peer_key.public());
    let suspect_record = super::signed_discovery_peer_record(
        &peer_key,
        vec![crate::dht::PeerDiscoverySource::Dht],
        1,
    );
    let mut discovered_peer_records = HashMap::new();
    let mut known_transport_paths = HashMap::new();
    let mut last_dialed_transport_paths = HashMap::new();
    let active_transport_paths = HashMap::new();
    let mut failed_transport_path_labels = HashSet::new();

    super::super::discovery::process_discovered_peer_record(
        &mut swarm,
        &mut discovered_peer_records,
        &mut known_transport_paths,
        &mut last_dialed_transport_paths,
        &active_transport_paths,
        &mut failed_transport_path_labels,
        None,
        &PeerManagerPolicy::default(),
        suspect_record,
    )
    .expect("process dht-only suspect peer record");

    assert!(discovered_peer_records.contains_key(&peer_id));
    assert!(!last_dialed_transport_paths.contains_key(&peer_id));
}

#[test]
fn maybe_request_cached_peer_record_does_not_use_target_peer_as_proxy() {
    let mut swarm = super::super::swarm_behaviour::build_swarm(
        &Keypair::generate_ed25519(),
        false,
        true,
        super::super::wire_bytes::init_shared_wire_byte_counters(),
    );
    let local_peer_id = PeerId::random();
    let target_peer_id = PeerId::random();
    let mut pending_peer_record_requests = HashMap::new();
    let mut pending_cached_peer_records = HashSet::new();
    let mut cached_peer_record_cooldowns = HashMap::new();
    let traffic_metrics = super::super::traffic_metrics::init_shared_traffic_metrics();

    let requested = super::super::discovery::maybe_request_cached_peer_record(
        &mut swarm,
        &mut pending_peer_record_requests,
        &mut pending_cached_peer_records,
        &mut cached_peer_record_cooldowns,
        &traffic_metrics,
        &[target_peer_id],
        target_peer_id,
        local_peer_id,
    );

    assert!(!requested);
    assert!(pending_peer_record_requests.is_empty());
    assert!(pending_cached_peer_records.is_empty());
}

#[test]
fn maybe_request_connected_peer_record_respects_short_cooldown() {
    let mut swarm = super::super::swarm_behaviour::build_swarm(
        &Keypair::generate_ed25519(),
        false,
        true,
        super::super::wire_bytes::init_shared_wire_byte_counters(),
    );
    let local_peer_id = PeerId::random();
    let target_peer_id = PeerId::random();
    let mut pending_peer_record_requests = HashMap::new();
    let mut pending_connected_peer_records = HashSet::new();
    let mut connected_peer_record_cooldowns = HashMap::new();
    let traffic_metrics = super::super::traffic_metrics::init_shared_traffic_metrics();

    let requested = super::super::discovery::maybe_request_connected_peer_record(
        &mut swarm,
        &mut pending_peer_record_requests,
        &mut pending_connected_peer_records,
        &mut connected_peer_record_cooldowns,
        &traffic_metrics,
        target_peer_id,
        local_peer_id,
    );
    assert!(requested);
    assert_eq!(pending_peer_record_requests.len(), 1);

    pending_peer_record_requests.clear();
    pending_connected_peer_records.clear();

    let requested_during_cooldown = super::super::discovery::maybe_request_connected_peer_record(
        &mut swarm,
        &mut pending_peer_record_requests,
        &mut pending_connected_peer_records,
        &mut connected_peer_record_cooldowns,
        &traffic_metrics,
        target_peer_id,
        local_peer_id,
    );
    assert!(!requested_during_cooldown);
    assert!(pending_peer_record_requests.is_empty());

    connected_peer_record_cooldowns.insert(target_peer_id, 0);

    let requested_after_expiry = super::super::discovery::maybe_request_connected_peer_record(
        &mut swarm,
        &mut pending_peer_record_requests,
        &mut pending_connected_peer_records,
        &mut connected_peer_record_cooldowns,
        &traffic_metrics,
        target_peer_id,
        local_peer_id,
    );
    assert!(requested_after_expiry);
    assert_eq!(pending_peer_record_requests.len(), 1);
}

#[test]
fn maybe_request_cached_peer_record_respects_short_cooldown() {
    let mut swarm = super::super::swarm_behaviour::build_swarm(
        &Keypair::generate_ed25519(),
        false,
        true,
        super::super::wire_bytes::init_shared_wire_byte_counters(),
    );
    let local_peer_id = PeerId::random();
    let target_peer_id = PeerId::random();
    let proxy_peer_id = PeerId::random();
    let mut pending_peer_record_requests = HashMap::new();
    let mut pending_cached_peer_records = HashSet::new();
    let mut cached_peer_record_cooldowns = HashMap::new();
    let traffic_metrics = super::super::traffic_metrics::init_shared_traffic_metrics();

    let requested = super::super::discovery::maybe_request_cached_peer_record(
        &mut swarm,
        &mut pending_peer_record_requests,
        &mut pending_cached_peer_records,
        &mut cached_peer_record_cooldowns,
        &traffic_metrics,
        &[proxy_peer_id],
        target_peer_id,
        local_peer_id,
    );
    assert!(requested);
    assert_eq!(pending_peer_record_requests.len(), 1);

    pending_peer_record_requests.clear();
    pending_cached_peer_records.clear();

    let requested_during_cooldown = super::super::discovery::maybe_request_cached_peer_record(
        &mut swarm,
        &mut pending_peer_record_requests,
        &mut pending_cached_peer_records,
        &mut cached_peer_record_cooldowns,
        &traffic_metrics,
        &[proxy_peer_id],
        target_peer_id,
        local_peer_id,
    );
    assert!(!requested_during_cooldown);
    assert!(pending_peer_record_requests.is_empty());

    cached_peer_record_cooldowns.insert(target_peer_id, 0);

    let requested_after_expiry = super::super::discovery::maybe_request_cached_peer_record(
        &mut swarm,
        &mut pending_peer_record_requests,
        &mut pending_cached_peer_records,
        &mut cached_peer_record_cooldowns,
        &traffic_metrics,
        &[proxy_peer_id],
        target_peer_id,
        local_peer_id,
    );
    assert!(requested_after_expiry);
    assert_eq!(pending_peer_record_requests.len(), 1);
}

#[test]
fn maybe_request_cached_discovery_peers_respects_short_cooldown() {
    let mut swarm = super::super::swarm_behaviour::build_swarm(
        &Keypair::generate_ed25519(),
        false,
        true,
        super::super::wire_bytes::init_shared_wire_byte_counters(),
    );
    let local_peer_id = PeerId::random();
    let target_peer_id = PeerId::random();
    let mut pending_peer_record_requests = HashMap::new();
    let mut pending_cached_discovery_peers = HashSet::new();
    let mut cached_discovery_peer_cooldowns = HashMap::new();
    let traffic_metrics = super::super::traffic_metrics::init_shared_traffic_metrics();

    let requested = super::super::discovery::maybe_request_cached_discovery_peers(
        &mut swarm,
        &mut pending_peer_record_requests,
        &mut pending_cached_discovery_peers,
        &mut cached_discovery_peer_cooldowns,
        &traffic_metrics,
        target_peer_id,
        local_peer_id,
    );
    assert!(requested);
    assert_eq!(pending_peer_record_requests.len(), 1);

    pending_peer_record_requests.clear();
    pending_cached_discovery_peers.clear();

    let requested_during_cooldown = super::super::discovery::maybe_request_cached_discovery_peers(
        &mut swarm,
        &mut pending_peer_record_requests,
        &mut pending_cached_discovery_peers,
        &mut cached_discovery_peer_cooldowns,
        &traffic_metrics,
        target_peer_id,
        local_peer_id,
    );
    assert!(!requested_during_cooldown);
    assert!(pending_peer_record_requests.is_empty());

    cached_discovery_peer_cooldowns.insert(target_peer_id, 0);

    let requested_after_expiry = super::super::discovery::maybe_request_cached_discovery_peers(
        &mut swarm,
        &mut pending_peer_record_requests,
        &mut pending_cached_discovery_peers,
        &mut cached_discovery_peer_cooldowns,
        &traffic_metrics,
        target_peer_id,
        local_peer_id,
    );
    assert!(requested_after_expiry);
    assert_eq!(pending_peer_record_requests.len(), 1);
}

#[test]
fn start_peer_discovery_query_respects_pending_query_and_cooldown() {
    let mut swarm =
        super::super::swarm_behaviour::build_swarm(&Keypair::generate_ed25519(), false, true);
    let template = super::signed_discovery_peer_record(
        &Keypair::generate_ed25519(),
        vec![crate::dht::PeerDiscoverySource::Dht],
        1,
    )
    .record;
    let mut pending_dht = HashMap::new();
    let mut last_started_at_ms = None;

    let first_started = super::super::discovery::start_peer_discovery_query(
        &mut swarm,
        &mut pending_dht,
        &template,
        &mut last_started_at_ms,
        10_000,
        60_000,
    );
    assert!(first_started);
    assert_eq!(pending_dht.len(), 1);
    assert_eq!(last_started_at_ms, Some(10_000));

    let started_while_pending = super::super::discovery::start_peer_discovery_query(
        &mut swarm,
        &mut pending_dht,
        &template,
        &mut last_started_at_ms,
        20_000,
        60_000,
    );
    assert!(!started_while_pending);
    assert_eq!(pending_dht.len(), 1);

    pending_dht.clear();

    let started_during_cooldown = super::super::discovery::start_peer_discovery_query(
        &mut swarm,
        &mut pending_dht,
        &template,
        &mut last_started_at_ms,
        20_000,
        60_000,
    );
    assert!(!started_during_cooldown);
    assert!(pending_dht.is_empty());

    let started_after_cooldown = super::super::discovery::start_peer_discovery_query(
        &mut swarm,
        &mut pending_dht,
        &template,
        &mut last_started_at_ms,
        70_000,
        60_000,
    );
    assert!(started_after_cooldown);
    assert_eq!(pending_dht.len(), 1);
    assert_eq!(last_started_at_ms, Some(70_000));
}

#[test]
fn clear_disconnected_peer_state_removes_peer_record_cooldowns() {
    let mut swarm = super::super::swarm_behaviour::build_swarm(
        &Keypair::generate_ed25519(),
        false,
        true,
        super::super::wire_bytes::init_shared_wire_byte_counters(),
    );
    let local_peer_id = PeerId::random();
    let target_peer_id = PeerId::random();
    let proxy_peer_id = PeerId::random();
    let cooldown_until_ms = super::super::now_ms().saturating_add(60_000);
    let mut peers = vec![target_peer_id];
    let mut admitted_active_peers = HashSet::from([target_peer_id]);
    let mut quarantined_active_peers = HashSet::new();
    let mut pending_quarantine_disconnects = HashSet::new();
    let mut active_transport_paths = HashMap::new();
    let mut last_dialed_transport_paths = HashMap::new();
    let mut connected_peer_record_cooldowns = HashMap::from([(target_peer_id, cooldown_until_ms)]);
    let mut cached_peer_record_cooldowns = HashMap::from([(target_peer_id, cooldown_until_ms)]);
    let mut cached_discovery_peer_cooldowns = HashMap::from([(target_peer_id, cooldown_until_ms)]);
    let mut pending_rendezvous_registers = HashSet::new();
    let mut pending_rendezvous_discovers = HashSet::new();
    let mut registered_rendezvous_nodes = HashSet::new();
    let mut rendezvous_cookies = HashMap::new();
    let event_connected_peers = Arc::new(Mutex::new(HashSet::from([target_peer_id])));
    let mut pending_peer_record_requests = HashMap::new();
    let mut pending_connected_peer_records = HashSet::new();
    let mut pending_cached_peer_records = HashSet::new();
    let mut pending_cached_discovery_peers = HashSet::new();
    let traffic_metrics = super::super::traffic_metrics::init_shared_traffic_metrics();

    assert!(
        !super::super::discovery::maybe_request_connected_peer_record(
            &mut swarm,
            &mut pending_peer_record_requests,
            &mut pending_connected_peer_records,
            &mut connected_peer_record_cooldowns,
            &traffic_metrics,
            target_peer_id,
            local_peer_id,
        )
    );
    assert!(!super::super::discovery::maybe_request_cached_peer_record(
        &mut swarm,
        &mut pending_peer_record_requests,
        &mut pending_cached_peer_records,
        &mut cached_peer_record_cooldowns,
        &traffic_metrics,
        &[proxy_peer_id],
        target_peer_id,
        local_peer_id,
    ));
    assert!(
        !super::super::discovery::maybe_request_cached_discovery_peers(
            &mut swarm,
            &mut pending_peer_record_requests,
            &mut pending_cached_discovery_peers,
            &mut cached_discovery_peer_cooldowns,
            &traffic_metrics,
            target_peer_id,
            local_peer_id,
        )
    );
    assert!(pending_peer_record_requests.is_empty());

    let quarantined = super::super::connection_lifecycle::clear_disconnected_peer_state(
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
        target_peer_id,
    );

    assert!(!quarantined);
    assert!(!connected_peer_record_cooldowns.contains_key(&target_peer_id));
    assert!(!cached_peer_record_cooldowns.contains_key(&target_peer_id));
    assert!(!cached_discovery_peer_cooldowns.contains_key(&target_peer_id));
    assert!(!peers.contains(&target_peer_id));
    assert!(!admitted_active_peers.contains(&target_peer_id));
    assert!(!event_connected_peers
        .lock()
        .expect("lock connected peers")
        .contains(&target_peer_id));

    let connected_requested = super::super::discovery::maybe_request_connected_peer_record(
        &mut swarm,
        &mut pending_peer_record_requests,
        &mut pending_connected_peer_records,
        &mut connected_peer_record_cooldowns,
        &traffic_metrics,
        target_peer_id,
        local_peer_id,
    );
    assert!(connected_requested);
    pending_peer_record_requests.clear();
    pending_connected_peer_records.clear();

    let cached_requested = super::super::discovery::maybe_request_cached_peer_record(
        &mut swarm,
        &mut pending_peer_record_requests,
        &mut pending_cached_peer_records,
        &mut cached_peer_record_cooldowns,
        &traffic_metrics,
        &[proxy_peer_id],
        target_peer_id,
        local_peer_id,
    );
    assert!(cached_requested);
    pending_peer_record_requests.clear();
    pending_cached_peer_records.clear();

    let cached_discovery_requested = super::super::discovery::maybe_request_cached_discovery_peers(
        &mut swarm,
        &mut pending_peer_record_requests,
        &mut pending_cached_discovery_peers,
        &mut cached_discovery_peer_cooldowns,
        &traffic_metrics,
        target_peer_id,
        local_peer_id,
    );
    assert!(cached_discovery_requested);
}

#[test]
fn cached_peer_record_request_handler_reports_not_found_on_cache_miss() {
    let request = NetworkRequest {
        protocol: super::super::RR_GET_CACHED_PEER_RECORD.to_string(),
        payload: PeerId::random().to_string().into_bytes(),
    };
    let handlers = HashMap::new();
    let keypair = Keypair::generate_ed25519();
    let listening_addrs = Arc::new(Mutex::new(Vec::new()));
    let reachability = Arc::new(Mutex::new(Libp2pReachabilitySnapshot::default()));
    let discovered_peer_records = HashMap::new();

    let err = super::super::discovery::handle_request_response_request(
        &request,
        &handlers,
        None,
        &keypair,
        &listening_addrs,
        &reachability,
        false,
        &discovered_peer_records,
    )
    .expect_err("cache miss should surface not found");

    assert!(matches!(
        err,
        WorldError::NetworkRequestFailed {
            code: oasis7_proto::distributed::DistributedErrorCode::ErrNotFound,
            retryable: true,
            ..
        }
    ));
}

#[test]
fn cached_peer_record_not_found_retries_via_another_connected_peer() {
    let mut swarm = super::super::swarm_behaviour::build_swarm(
        &Keypair::generate_ed25519(),
        false,
        true,
        super::super::wire_bytes::init_shared_wire_byte_counters(),
    );
    let target_peer_id = PeerId::random();
    let first_proxy = PeerId::random();
    let fallback_proxy = PeerId::random();
    let local_peer_id = PeerId::random();
    let not_found_payload = to_canonical_cbor(&oasis7_proto::distributed::ErrorResponse {
        code: oasis7_proto::distributed::DistributedErrorCode::ErrNotFound,
        message: "missing cached peer record".to_string(),
        retryable: true,
    })
    .expect("encode error response");
    let mut pending_peer_record_requests = HashMap::new();
    let mut pending_dht = HashMap::new();
    let mut discovered_peer_records = HashMap::new();
    let mut known_transport_paths = HashMap::new();
    let mut last_dialed_transport_paths = HashMap::new();
    let active_transport_paths = HashMap::new();
    let mut failed_transport_path_labels = HashSet::new();
    let mut pending_discovery_peer_records = HashSet::new();
    let mut pending_connected_peer_records = HashSet::new();
    let mut pending_cached_peer_records = HashSet::from([target_peer_id]);
    let mut pending_cached_discovery_peers = HashSet::new();
    let mut cached_peer_record_cooldowns = HashMap::new();
    let event_errors = Arc::new(Mutex::new(Vec::new()));
    let traffic_metrics = super::super::traffic_metrics::init_shared_traffic_metrics();

    super::super::discovery::handle_peer_record_response(
        &mut swarm,
        super::super::discovery::PendingPeerRecordRequest::CachedPeerRecord {
            ask_peer: first_proxy,
            peer_id: target_peer_id,
            tried_proxies: vec![first_proxy],
        },
        not_found_payload.as_slice(),
        &mut pending_peer_record_requests,
        &mut pending_dht,
        &mut discovered_peer_records,
        &mut known_transport_paths,
        &mut last_dialed_transport_paths,
        &active_transport_paths,
        &[first_proxy, fallback_proxy],
        &traffic_metrics,
        &mut failed_transport_path_labels,
        &mut pending_discovery_peer_records,
        &mut cached_peer_record_cooldowns,
        None,
        local_peer_id,
        &mut pending_connected_peer_records,
        &mut pending_cached_peer_records,
        &mut pending_cached_discovery_peers,
        16,
        &event_errors,
        &PeerManagerPolicy::default(),
    );

    assert_eq!(pending_peer_record_requests.len(), 1);
    assert!(pending_cached_peer_records.contains(&target_peer_id));
    let retried = pending_peer_record_requests
        .values()
        .next()
        .expect("retried cached peer record request");
    assert!(matches!(
        retried,
        super::super::discovery::PendingPeerRecordRequest::CachedPeerRecord {
            ask_peer,
            peer_id,
            tried_proxies
        } if *ask_peer == fallback_proxy
            && *peer_id == target_peer_id
            && *tried_proxies == vec![first_proxy, fallback_proxy]
    ));
}

#[test]
fn cached_peer_record_not_found_stops_after_all_connected_proxies_are_tried() {
    let mut swarm = super::super::swarm_behaviour::build_swarm(
        &Keypair::generate_ed25519(),
        false,
        true,
        super::super::wire_bytes::init_shared_wire_byte_counters(),
    );
    let target_peer_id = PeerId::random();
    let first_proxy = PeerId::random();
    let fallback_proxy = PeerId::random();
    let local_peer_id = PeerId::random();
    let not_found_payload = to_canonical_cbor(&oasis7_proto::distributed::ErrorResponse {
        code: oasis7_proto::distributed::DistributedErrorCode::ErrNotFound,
        message: "missing cached peer record".to_string(),
        retryable: true,
    })
    .expect("encode error response");
    let mut pending_peer_record_requests = HashMap::new();
    let mut pending_dht = HashMap::new();
    let mut discovered_peer_records = HashMap::new();
    let mut known_transport_paths = HashMap::new();
    let mut last_dialed_transport_paths = HashMap::new();
    let active_transport_paths = HashMap::new();
    let mut failed_transport_path_labels = HashSet::new();
    let mut pending_discovery_peer_records = HashSet::new();
    let mut pending_connected_peer_records = HashSet::new();
    let mut pending_cached_peer_records = HashSet::from([target_peer_id]);
    let mut pending_cached_discovery_peers = HashSet::new();
    let mut cached_peer_record_cooldowns = HashMap::new();
    let event_errors = Arc::new(Mutex::new(Vec::new()));
    let traffic_metrics = super::super::traffic_metrics::init_shared_traffic_metrics();

    super::super::discovery::handle_peer_record_response(
        &mut swarm,
        super::super::discovery::PendingPeerRecordRequest::CachedPeerRecord {
            ask_peer: fallback_proxy,
            peer_id: target_peer_id,
            tried_proxies: vec![first_proxy, fallback_proxy],
        },
        not_found_payload.as_slice(),
        &mut pending_peer_record_requests,
        &mut pending_dht,
        &mut discovered_peer_records,
        &mut known_transport_paths,
        &mut last_dialed_transport_paths,
        &active_transport_paths,
        &[first_proxy, fallback_proxy],
        &traffic_metrics,
        &mut failed_transport_path_labels,
        &mut pending_discovery_peer_records,
        &mut cached_peer_record_cooldowns,
        None,
        local_peer_id,
        &mut pending_connected_peer_records,
        &mut pending_cached_peer_records,
        &mut pending_cached_discovery_peers,
        16,
        &event_errors,
        &PeerManagerPolicy::default(),
    );

    assert!(pending_peer_record_requests.is_empty());
    assert!(!pending_cached_peer_records.contains(&target_peer_id));
}
