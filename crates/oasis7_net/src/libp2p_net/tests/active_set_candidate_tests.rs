use std::collections::BTreeSet;

use super::super::peer_manager::{PeerManagerHealthStatus, PeerManagerPolicy};
use super::super::peer_manager_active_set::{
    candidate_status_with_active_set, candidate_would_degrade_admitted_peers, ActivePeerCandidate,
    ActivePeerSetStats,
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

    assert_eq!(
        candidate_status_with_active_set(&candidate, &stats, &PeerManagerPolicy::default()),
        PeerManagerHealthStatus::Blocked
    );
    assert!(candidate_would_degrade_admitted_peers(
        &candidate,
        &stats,
        &PeerManagerPolicy::default(),
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
        discovery_sources: vec![
            crate::dht::PeerDiscoverySource::Dht,
            crate::dht::PeerDiscoverySource::Dht,
        ],
        ipv4_subnet_bucket: None,
        relay_domain: None,
        source_operator: None,
        source_asn: None,
        relay_reserved: false,
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
        discovery_sources: vec![
            crate::dht::PeerDiscoverySource::Dht,
            crate::dht::PeerDiscoverySource::Dht,
        ],
        ipv4_subnet_bucket: None,
        relay_domain: None,
        source_operator: None,
        source_asn: None,
        relay_reserved: false,
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
