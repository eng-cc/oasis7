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
