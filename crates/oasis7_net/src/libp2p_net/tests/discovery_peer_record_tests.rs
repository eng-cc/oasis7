use super::*;

#[test]
fn process_discovered_peer_record_keeps_single_source_bootstrap_peer_dial_eligible() {
    let mut swarm = super::super::swarm_behaviour::build_swarm(&Keypair::generate_ed25519(), false);
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
    let mut swarm = super::super::swarm_behaviour::build_swarm(&Keypair::generate_ed25519(), false);
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
    let mut swarm = super::super::swarm_behaviour::build_swarm(&Keypair::generate_ed25519(), false);
    let local_peer_id = PeerId::random();
    let target_peer_id = PeerId::random();
    let mut pending_peer_record_requests = HashMap::new();
    let mut pending_cached_peer_records = HashSet::new();

    let requested = super::super::discovery::maybe_request_cached_peer_record(
        &mut swarm,
        &mut pending_peer_record_requests,
        &mut pending_cached_peer_records,
        &[target_peer_id],
        target_peer_id,
        local_peer_id,
    );

    assert!(!requested);
    assert!(pending_peer_record_requests.is_empty());
    assert!(pending_cached_peer_records.is_empty());
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
    let mut swarm = super::super::swarm_behaviour::build_swarm(&Keypair::generate_ed25519(), false);
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
    let event_errors = Arc::new(Mutex::new(Vec::new()));

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
        &mut failed_transport_path_labels,
        &mut pending_discovery_peer_records,
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
    let mut swarm = super::super::swarm_behaviour::build_swarm(&Keypair::generate_ed25519(), false);
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
    let event_errors = Arc::new(Mutex::new(Vec::new()));

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
        &mut failed_transport_path_labels,
        &mut pending_discovery_peer_records,
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
