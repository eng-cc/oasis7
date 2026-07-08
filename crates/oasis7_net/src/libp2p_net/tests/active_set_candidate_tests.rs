use std::collections::BTreeSet;

use super::super::peer_manager::{PeerManagerHealthStatus, PeerManagerPolicy};
use super::super::peer_manager_active_set::{
    ActivePeerCandidate, ActivePeerSetStats, candidate_status_with_active_set,
    candidate_would_degrade_admitted_peers,
};
use super::*;

#[test]
fn active_set_candidate_status_flags_bucket_overflow_without_full_recompute() {
    let admitted_key = Keypair::generate_ed25519();
    let admitted_peer = PeerId::from(admitted_key.public());
    let candidate_key = Keypair::generate_ed25519();
    let candidate_peer = PeerId::from(candidate_key.public());
    let admitted_record = signed_discovery_peer_record(
        &admitted_key,
        vec![
            crate::dht::PeerDiscoverySource::Dht,
            crate::dht::PeerDiscoverySource::Rendezvous,
        ],
        1,
    );
    let candidate_record = signed_discovery_peer_record(
        &candidate_key,
        vec![
            crate::dht::PeerDiscoverySource::Dht,
            crate::dht::PeerDiscoverySource::Rendezvous,
        ],
        1,
    );
    let admitted_path = active_transport_path_from_endpoint(
        &HashMap::new(),
        admitted_peer,
        &"/ip4/10.0.0.1/udp/4103/quic-v1"
            .parse()
            .expect("admitted endpoint"),
    );
    let candidate_path = active_transport_path_from_endpoint(
        &HashMap::new(),
        candidate_peer,
        &"/ip4/10.0.0.2/udp/4104/quic-v1"
            .parse()
            .expect("candidate endpoint"),
    );
    let stats = ActivePeerSetStats::new(
        &HashMap::from([(admitted_peer, admitted_record)]),
        &HashMap::from([(admitted_peer, admitted_path)]),
    );
    let candidate = ActivePeerCandidate::from_record_and_path(&candidate_record, &candidate_path);

    let policy = PeerManagerPolicy {
        min_active_peers_for_share_limits: 0,
        ..PeerManagerPolicy::default()
    };
    assert_eq!(
        candidate_status_with_active_set(&candidate, &stats, &policy),
        PeerManagerHealthStatus::Blocked
    );
    assert!(candidate_would_degrade_admitted_peers(
        &candidate, &stats, &policy,
    ));
}

#[test]
fn active_set_candidate_observer_light_does_not_degrade_provider_share_limits() {
    let first_key = Keypair::generate_ed25519();
    let second_key = Keypair::generate_ed25519();
    let third_key = Keypair::generate_ed25519();
    let observer_key = Keypair::generate_ed25519();
    let first_peer = PeerId::from(first_key.public());
    let second_peer = PeerId::from(second_key.public());
    let third_peer = PeerId::from(third_key.public());
    let observer_peer = PeerId::from(observer_key.public());
    let discovery_sources = vec![
        crate::dht::PeerDiscoverySource::Dht,
        crate::dht::PeerDiscoverySource::Rendezvous,
    ];
    let mut observer_record =
        signed_discovery_peer_record(&observer_key, discovery_sources.clone(), 4);
    observer_record.record.node_role = PeerNodeRole::ObserverLight.as_str().to_string();
    observer_record.record.capability_lanes =
        PeerNodeRole::ObserverLight.default_capability_lanes();

    let stats = ActivePeerSetStats::new(
        &HashMap::from([
            (
                first_peer,
                signed_discovery_peer_record(&first_key, discovery_sources.clone(), 1),
            ),
            (
                second_peer,
                signed_discovery_peer_record(&second_key, discovery_sources.clone(), 2),
            ),
            (
                third_peer,
                signed_discovery_peer_record(&third_key, discovery_sources, 3),
            ),
        ]),
        &HashMap::from([
            (
                first_peer,
                active_transport_path_from_endpoint(
                    &HashMap::new(),
                    first_peer,
                    &"/ip4/10.0.0.1/udp/4101/quic-v1"
                        .parse()
                        .expect("first endpoint"),
                ),
            ),
            (
                second_peer,
                active_transport_path_from_endpoint(
                    &HashMap::new(),
                    second_peer,
                    &"/ip4/10.0.0.2/udp/4102/quic-v1"
                        .parse()
                        .expect("second endpoint"),
                ),
            ),
            (
                third_peer,
                active_transport_path_from_endpoint(
                    &HashMap::new(),
                    third_peer,
                    &"/ip4/10.20.30.40/udp/4103/quic-v1"
                        .parse()
                        .expect("third endpoint"),
                ),
            ),
        ]),
    );
    let observer_path = active_transport_path_from_endpoint(
        &HashMap::new(),
        observer_peer,
        &"/ip4/10.0.0.89/udp/4104/quic-v1"
            .parse()
            .expect("observer endpoint"),
    );
    let candidate = ActivePeerCandidate::from_record_and_path(&observer_record, &observer_path);

    assert_eq!(
        candidate_status_with_active_set(&candidate, &stats, &PeerManagerPolicy::default()),
        PeerManagerHealthStatus::Active
    );
    assert!(!candidate_would_degrade_admitted_peers(
        &candidate,
        &stats,
        &PeerManagerPolicy::default(),
    ));
}

#[test]
fn active_set_stats_keep_observer_light_discovery_sources() {
    let provider_key = Keypair::generate_ed25519();
    let observer_key = Keypair::generate_ed25519();
    let provider_peer = PeerId::from(provider_key.public());
    let observer_peer = PeerId::from(observer_key.public());
    let provider_record =
        signed_discovery_peer_record(&provider_key, vec![crate::dht::PeerDiscoverySource::Dht], 1);
    let mut observer_record = signed_discovery_peer_record(
        &observer_key,
        vec![crate::dht::PeerDiscoverySource::Rendezvous],
        2,
    );
    observer_record.record.node_role = PeerNodeRole::ObserverLight.as_str().to_string();
    observer_record.record.capability_lanes =
        PeerNodeRole::ObserverLight.default_capability_lanes();

    let stats = ActivePeerSetStats::new(
        &HashMap::from([
            (provider_peer, provider_record),
            (observer_peer, observer_record),
        ]),
        &HashMap::from([
            (
                provider_peer,
                active_transport_path_from_endpoint(
                    &HashMap::new(),
                    provider_peer,
                    &"/ip4/10.0.0.1/udp/4101/quic-v1"
                        .parse()
                        .expect("provider endpoint"),
                ),
            ),
            (
                observer_peer,
                active_transport_path_from_endpoint(
                    &HashMap::new(),
                    observer_peer,
                    &"/ip4/10.0.0.89/udp/4104/quic-v1"
                        .parse()
                        .expect("observer endpoint"),
                ),
            ),
        ]),
    );

    assert_eq!(stats.active_peer_count, 1);
    assert_eq!(
        stats.active_discovery_sources,
        BTreeSet::from(["dht", "rendezvous"])
    );
    assert_eq!(
        stats.ipv4_subnet_counts,
        HashMap::from([("10.0.0".to_string(), 1)])
    );
}

#[test]
fn active_set_candidate_count_limit_allows_third_and_blocks_fourth_subnet_peer() {
    let candidate = ActivePeerCandidate {
        discovery_source_labels: BTreeSet::from(["dht", "rendezvous"]),
        ipv4_subnet_bucket: Some("10.0.0".to_string()),
        relay_domain: None,
        source_operator: None,
        source_asn: None,
        relay_reserved: false,
        counts_toward_share_limits: true,
    };
    let policy = PeerManagerPolicy {
        max_ipv4_subnet_active_peers: Some(3),
        ..PeerManagerPolicy::default()
    };
    let two_peer_stats = ActivePeerSetStats {
        active_peer_count: 2,
        active_discovery_sources: BTreeSet::from(["dht", "rendezvous"]),
        ipv4_subnet_counts: HashMap::from([("10.0.0".to_string(), 2)]),
        ..ActivePeerSetStats::default()
    };
    let three_peer_stats = ActivePeerSetStats {
        active_peer_count: 3,
        active_discovery_sources: BTreeSet::from(["dht", "rendezvous"]),
        ipv4_subnet_counts: HashMap::from([("10.0.0".to_string(), 3)]),
        ..ActivePeerSetStats::default()
    };

    assert_eq!(
        candidate_status_with_active_set(&candidate, &two_peer_stats, &policy),
        PeerManagerHealthStatus::Active
    );
    assert!(!candidate_would_degrade_admitted_peers(
        &candidate,
        &two_peer_stats,
        &policy,
    ));
    assert_eq!(
        candidate_status_with_active_set(&candidate, &three_peer_stats, &policy),
        PeerManagerHealthStatus::Blocked
    );
    assert!(candidate_would_degrade_admitted_peers(
        &candidate,
        &three_peer_stats,
        &policy,
    ));
}

#[test]
fn active_set_candidate_status_admits_distinct_peer_without_degrading_existing_active_set() {
    let admitted_key = Keypair::generate_ed25519();
    let admitted_peer = PeerId::from(admitted_key.public());
    let candidate_key = Keypair::generate_ed25519();
    let candidate_peer = PeerId::from(candidate_key.public());
    let admitted_record = signed_discovery_peer_record(
        &admitted_key,
        vec![
            crate::dht::PeerDiscoverySource::Dht,
            crate::dht::PeerDiscoverySource::Rendezvous,
        ],
        1,
    );
    let candidate_record = signed_discovery_peer_record(
        &candidate_key,
        vec![
            crate::dht::PeerDiscoverySource::Dht,
            crate::dht::PeerDiscoverySource::Rendezvous,
        ],
        1,
    );
    let admitted_path = active_transport_path_from_endpoint(
        &HashMap::new(),
        admitted_peer,
        &"/ip4/10.0.0.1/udp/4103/quic-v1"
            .parse()
            .expect("admitted endpoint"),
    );
    let candidate_path = active_transport_path_from_endpoint(
        &HashMap::new(),
        candidate_peer,
        &"/ip4/10.0.1.2/udp/4104/quic-v1"
            .parse()
            .expect("candidate endpoint"),
    );
    let stats = ActivePeerSetStats::new(
        &HashMap::from([(admitted_peer, admitted_record)]),
        &HashMap::from([(admitted_peer, admitted_path)]),
    );
    let candidate = ActivePeerCandidate::from_record_and_path(&candidate_record, &candidate_path);

    assert_eq!(
        candidate_status_with_active_set(&candidate, &stats, &PeerManagerPolicy::default()),
        PeerManagerHealthStatus::Active
    );
    assert!(!candidate_would_degrade_admitted_peers(
        &candidate,
        &stats,
        &PeerManagerPolicy::default(),
    ));
}

#[test]
fn active_set_candidate_status_counts_unique_candidate_discovery_sources() {
    let candidate = ActivePeerCandidate {
        discovery_source_labels: BTreeSet::from(["dht"]),
        ipv4_subnet_bucket: None,
        relay_domain: None,
        source_operator: None,
        source_asn: None,
        relay_reserved: false,
        counts_toward_share_limits: true,
    };

    assert_eq!(
        candidate_status_with_active_set(
            &candidate,
            &ActivePeerSetStats::default(),
            &PeerManagerPolicy::default(),
        ),
        PeerManagerHealthStatus::Suspect
    );
}

#[test]
fn active_set_candidate_status_projects_unique_active_discovery_source_union() {
    let candidate = ActivePeerCandidate {
        discovery_source_labels: BTreeSet::from(["dht"]),
        ipv4_subnet_bucket: None,
        relay_domain: None,
        source_operator: None,
        source_asn: None,
        relay_reserved: false,
        counts_toward_share_limits: true,
    };
    let active_set_stats = ActivePeerSetStats {
        active_peer_count: 1,
        active_discovery_sources: BTreeSet::from(["rendezvous"]),
        ..ActivePeerSetStats::default()
    };
    let policy = PeerManagerPolicy {
        min_active_discovery_sources: 3,
        min_peer_discovery_sources: 1,
        ..PeerManagerPolicy::default()
    };

    assert_eq!(
        candidate_status_with_active_set(&candidate, &active_set_stats, &policy),
        PeerManagerHealthStatus::Suspect
    );
}
