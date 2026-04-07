use std::collections::{BTreeSet, HashMap, HashSet};
use std::net::Ipv4Addr;

use libp2p::multiaddr::Protocol;
use libp2p::PeerId;
use oasis7_proto::distributed_dht::{PeerDiscoverySource, SignedPeerRecord};

use super::transport_paths::{TransportPath, TransportPathKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerManagerPolicy {
    pub min_active_discovery_sources: usize,
    pub min_peer_discovery_sources: usize,
    pub max_ipv4_subnet_share_per_mille: u16,
    pub max_relay_domain_share_per_mille: u16,
    pub max_relayed_active_peer_share_per_mille: u16,
}

impl Default for PeerManagerPolicy {
    fn default() -> Self {
        Self {
            min_active_discovery_sources: 2,
            min_peer_discovery_sources: 2,
            max_ipv4_subnet_share_per_mille: 250,
            max_relay_domain_share_per_mille: 250,
            max_relayed_active_peer_share_per_mille: 500,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PeerManagerHealthStatus {
    Active,
    Candidate,
    Suspect,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerManagerHealthIssue {
    MissingPeerRecord,
    SingleSourceDiscovery {
        observed_sources: usize,
        required_sources: usize,
    },
    InsufficientActiveDiscoverySources {
        observed_sources: usize,
        required_sources: usize,
    },
    Ipv4SubnetConcentration {
        subnet: String,
        peers_in_bucket: usize,
        active_peer_count: usize,
        limit_per_mille: u16,
    },
    RelayDomainConcentration {
        relay_domain: String,
        peers_in_bucket: usize,
        active_peer_count: usize,
        limit_per_mille: u16,
    },
    RelayBudgetExceeded {
        relayed_active_peers: usize,
        active_peer_count: usize,
        limit_per_mille: u16,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerManagerPeerHealth {
    pub peer_id: String,
    pub status: PeerManagerHealthStatus,
    pub issues: Vec<PeerManagerHealthIssue>,
    pub discovery_sources: Vec<PeerDiscoverySource>,
    pub active_path_kind: Option<String>,
}

pub(super) fn recompute_peer_manager_healths(
    discovered_peer_records: &HashMap<PeerId, SignedPeerRecord>,
    active_transport_paths: &HashMap<PeerId, TransportPath>,
    policy: &PeerManagerPolicy,
) -> HashMap<PeerId, PeerManagerPeerHealth> {
    let active_peer_count = active_transport_paths.len();
    let mut active_discovery_sources = BTreeSet::new();
    let mut ipv4_subnet_counts: HashMap<String, usize> = HashMap::new();
    let mut relay_domain_counts: HashMap<String, usize> = HashMap::new();
    let mut relayed_active_peers = 0usize;

    for (peer_id, active_path) in active_transport_paths {
        if let Some(record) = discovered_peer_records.get(peer_id) {
            for source in &record.record.discovery_sources {
                active_discovery_sources.insert(discovery_source_label(*source));
            }
        }
        if let Some(bucket) = ipv4_subnet_bucket(active_path) {
            *ipv4_subnet_counts.entry(bucket).or_default() += 1;
        }
        if matches!(active_path.kind, TransportPathKind::RelayReserved) {
            relayed_active_peers += 1;
            if let Some(domain) = relay_domain(active_path) {
                *relay_domain_counts.entry(domain).or_default() += 1;
            }
        }
    }

    let insufficient_active_discovery_sources = active_peer_count > 0
        && active_discovery_sources.len() < policy.min_active_discovery_sources;
    let relay_budget_exceeded = exceeds_share_limit(
        relayed_active_peers,
        active_peer_count,
        policy.max_relayed_active_peer_share_per_mille,
    );

    let peer_ids: HashSet<PeerId> = discovered_peer_records
        .keys()
        .copied()
        .chain(active_transport_paths.keys().copied())
        .collect();
    let mut healths = HashMap::new();

    for peer_id in peer_ids {
        let record = discovered_peer_records.get(&peer_id);
        let active_path = active_transport_paths.get(&peer_id);
        let mut issues = Vec::new();
        let discovery_sources = record
            .map(|record| record.record.discovery_sources.clone())
            .unwrap_or_default();

        if record.is_none() {
            issues.push(PeerManagerHealthIssue::MissingPeerRecord);
        } else if discovery_sources.len() < policy.min_peer_discovery_sources {
            issues.push(PeerManagerHealthIssue::SingleSourceDiscovery {
                observed_sources: discovery_sources.len(),
                required_sources: policy.min_peer_discovery_sources,
            });
        }

        if let Some(active_path) = active_path {
            if insufficient_active_discovery_sources {
                issues.push(PeerManagerHealthIssue::InsufficientActiveDiscoverySources {
                    observed_sources: active_discovery_sources.len(),
                    required_sources: policy.min_active_discovery_sources,
                });
            }
            if let Some(bucket) = ipv4_subnet_bucket(active_path) {
                let bucket_count = ipv4_subnet_counts
                    .get(bucket.as_str())
                    .copied()
                    .unwrap_or(0);
                if bucket_count >= 2
                    && exceeds_share_limit(
                        bucket_count,
                        active_peer_count,
                        policy.max_ipv4_subnet_share_per_mille,
                    )
                {
                    issues.push(PeerManagerHealthIssue::Ipv4SubnetConcentration {
                        subnet: bucket,
                        peers_in_bucket: bucket_count,
                        active_peer_count,
                        limit_per_mille: policy.max_ipv4_subnet_share_per_mille,
                    });
                }
            }
            if matches!(active_path.kind, TransportPathKind::RelayReserved) {
                if relay_budget_exceeded {
                    issues.push(PeerManagerHealthIssue::RelayBudgetExceeded {
                        relayed_active_peers,
                        active_peer_count,
                        limit_per_mille: policy.max_relayed_active_peer_share_per_mille,
                    });
                }
                if let Some(domain) = relay_domain(active_path) {
                    let bucket_count = relay_domain_counts
                        .get(domain.as_str())
                        .copied()
                        .unwrap_or(0);
                    if bucket_count >= 2
                        && exceeds_share_limit(
                            bucket_count,
                            active_peer_count,
                            policy.max_relay_domain_share_per_mille,
                        )
                    {
                        issues.push(PeerManagerHealthIssue::RelayDomainConcentration {
                            relay_domain: domain,
                            peers_in_bucket: bucket_count,
                            active_peer_count,
                            limit_per_mille: policy.max_relay_domain_share_per_mille,
                        });
                    }
                }
            }
        }

        let status = if issues
            .iter()
            .any(|issue| matches!(issue, PeerManagerHealthIssue::MissingPeerRecord))
        {
            PeerManagerHealthStatus::Blocked
        } else if !issues.is_empty() {
            PeerManagerHealthStatus::Suspect
        } else if active_path.is_some() {
            PeerManagerHealthStatus::Active
        } else {
            PeerManagerHealthStatus::Candidate
        };

        healths.insert(
            peer_id,
            PeerManagerPeerHealth {
                peer_id: peer_id.to_string(),
                status,
                issues,
                discovery_sources,
                active_path_kind: active_path.map(|path| path.kind_label().to_string()),
            },
        );
    }

    healths
}

fn exceeds_share_limit(count: usize, total: usize, limit_per_mille: u16) -> bool {
    if total == 0 {
        return false;
    }
    count.saturating_mul(1000) > total.saturating_mul(limit_per_mille as usize)
}

fn ipv4_subnet_bucket(path: &TransportPath) -> Option<String> {
    path.addr.iter().find_map(|protocol| match protocol {
        Protocol::Ip4(ip) if !ip.is_loopback() => Some(ipv4_bucket(ip)),
        _ => None,
    })
}

fn relay_domain(path: &TransportPath) -> Option<String> {
    path.addr.iter().find_map(|protocol| match protocol {
        Protocol::Dns(domain) | Protocol::Dns4(domain) | Protocol::Dns6(domain) => {
            Some(domain.to_string())
        }
        Protocol::Ip4(ip) => Some(ip.to_string()),
        Protocol::Ip6(ip) => Some(ip.to_string()),
        _ => None,
    })
}

fn ipv4_bucket(ip: Ipv4Addr) -> String {
    let octets = ip.octets();
    format!("{}.{}.{}", octets[0], octets[1], octets[2])
}

fn discovery_source_label(source: PeerDiscoverySource) -> &'static str {
    match source {
        PeerDiscoverySource::StaticBootstrap => "static_bootstrap",
        PeerDiscoverySource::Dht => "dht",
        PeerDiscoverySource::Rendezvous => "rendezvous",
        PeerDiscoverySource::PeerExchange => "peer_exchange",
        PeerDiscoverySource::Manual => "manual",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto_dht::{
        PeerDeploymentMode, PeerDiscoverySource, PeerNodeRole, PeerReachabilityClass,
    };
    use crate::proto_net::NetworkLane;
    use libp2p::Multiaddr;

    use super::super::transport_paths::{
        TransportMuxer, TransportSecurity, TransportSessionFlavor,
    };

    fn sample_record(
        peer_id: PeerId,
        discovery_sources: Vec<PeerDiscoverySource>,
    ) -> SignedPeerRecord {
        SignedPeerRecord {
            record: oasis7_proto::distributed_dht::PeerRecord {
                peer_id: peer_id.to_string(),
                node_id: format!("node-{peer_id}"),
                world_id: "world-a".to_string(),
                network_id: "world-a".to_string(),
                node_role: PeerNodeRole::FullStorage.as_str().to_string(),
                deployment_mode: PeerDeploymentMode::Hybrid,
                reachability_class: PeerReachabilityClass::Hybrid,
                direct_addrs: Vec::new(),
                hole_punch_addrs: Vec::new(),
                relay_addrs: Vec::new(),
                discovery_sources,
                capability_lanes: vec![
                    NetworkLane::Sync,
                    NetworkLane::BlobState,
                    NetworkLane::Control,
                ],
                published_at_ms: 0,
                ttl_ms: 60_000,
            },
            identity_public_key_protobuf_hex: "abcd".to_string(),
            signature_hex: "beef".to_string(),
        }
    }

    fn transport_path(peer_id: PeerId, addr: &str, kind: TransportPathKind) -> TransportPath {
        let addr: Multiaddr = addr.parse().expect("multiaddr");
        TransportPath {
            peer_id,
            addr,
            kind,
            flavor: match kind {
                TransportPathKind::Direct => TransportSessionFlavor::Quic,
                TransportPathKind::HolePunched => TransportSessionFlavor::TcpNoiseYamux,
                TransportPathKind::RelayReserved => TransportSessionFlavor::RelayTunnel,
            },
            security: match kind {
                TransportPathKind::Direct | TransportPathKind::HolePunched => {
                    TransportSecurity::QuicTls
                }
                TransportPathKind::RelayReserved => TransportSecurity::Noise,
            },
            muxer: match kind {
                TransportPathKind::Direct | TransportPathKind::HolePunched => TransportMuxer::Quic,
                TransportPathKind::RelayReserved => TransportMuxer::Yamux,
            },
        }
    }

    #[test]
    fn recompute_marks_single_source_active_set_as_suspect() {
        let peer_a = PeerId::random();
        let peer_b = PeerId::random();
        let discovered = HashMap::from([
            (
                peer_a,
                sample_record(peer_a, vec![PeerDiscoverySource::StaticBootstrap]),
            ),
            (
                peer_b,
                sample_record(peer_b, vec![PeerDiscoverySource::StaticBootstrap]),
            ),
        ]);
        let active = HashMap::from([
            (
                peer_a,
                transport_path(
                    peer_a,
                    "/ip4/10.0.0.1/udp/4101/quic-v1",
                    TransportPathKind::Direct,
                ),
            ),
            (
                peer_b,
                transport_path(
                    peer_b,
                    "/ip4/10.0.1.1/udp/4102/quic-v1",
                    TransportPathKind::Direct,
                ),
            ),
        ]);

        let healths =
            recompute_peer_manager_healths(&discovered, &active, &PeerManagerPolicy::default());
        for peer in [peer_a, peer_b] {
            let health = healths.get(&peer).expect("health");
            assert_eq!(health.status, PeerManagerHealthStatus::Suspect);
            assert!(health.issues.iter().any(|issue| matches!(
                issue,
                PeerManagerHealthIssue::SingleSourceDiscovery { .. }
            )));
            assert!(health.issues.iter().any(|issue| matches!(
                issue,
                PeerManagerHealthIssue::InsufficientActiveDiscoverySources { .. }
            )));
        }
    }

    #[test]
    fn recompute_marks_subnet_concentration() {
        let peer_a = PeerId::random();
        let peer_b = PeerId::random();
        let peer_c = PeerId::random();
        let discovery_sources = vec![
            PeerDiscoverySource::StaticBootstrap,
            PeerDiscoverySource::Dht,
        ];
        let discovered = HashMap::from([
            (peer_a, sample_record(peer_a, discovery_sources.clone())),
            (peer_b, sample_record(peer_b, discovery_sources.clone())),
            (peer_c, sample_record(peer_c, discovery_sources)),
        ]);
        let active = HashMap::from([
            (
                peer_a,
                transport_path(
                    peer_a,
                    "/ip4/192.168.10.1/udp/4101/quic-v1",
                    TransportPathKind::Direct,
                ),
            ),
            (
                peer_b,
                transport_path(
                    peer_b,
                    "/ip4/192.168.10.2/udp/4102/quic-v1",
                    TransportPathKind::Direct,
                ),
            ),
            (
                peer_c,
                transport_path(
                    peer_c,
                    "/ip4/10.20.30.40/udp/4103/quic-v1",
                    TransportPathKind::Direct,
                ),
            ),
        ]);

        let healths =
            recompute_peer_manager_healths(&discovered, &active, &PeerManagerPolicy::default());
        assert!(healths[&peer_a].issues.iter().any(|issue| matches!(
            issue,
            PeerManagerHealthIssue::Ipv4SubnetConcentration { subnet, .. } if subnet == "192.168.10"
        )));
        assert!(healths[&peer_b].issues.iter().any(|issue| matches!(
            issue,
            PeerManagerHealthIssue::Ipv4SubnetConcentration { subnet, .. } if subnet == "192.168.10"
        )));
        assert_eq!(healths[&peer_c].status, PeerManagerHealthStatus::Active);
    }

    #[test]
    fn recompute_ignores_loopback_subnet_concentration() {
        let peer_a = PeerId::random();
        let peer_b = PeerId::random();
        let discovered = HashMap::from([
            (
                peer_a,
                sample_record(
                    peer_a,
                    vec![
                        PeerDiscoverySource::StaticBootstrap,
                        PeerDiscoverySource::Dht,
                    ],
                ),
            ),
            (
                peer_b,
                sample_record(
                    peer_b,
                    vec![
                        PeerDiscoverySource::StaticBootstrap,
                        PeerDiscoverySource::Dht,
                    ],
                ),
            ),
        ]);
        let active = HashMap::from([
            (
                peer_a,
                transport_path(
                    peer_a,
                    "/ip4/127.0.0.1/udp/4101/quic-v1",
                    TransportPathKind::Direct,
                ),
            ),
            (
                peer_b,
                transport_path(
                    peer_b,
                    "/ip4/127.0.0.1/udp/4102/quic-v1",
                    TransportPathKind::Direct,
                ),
            ),
        ]);

        let healths =
            recompute_peer_manager_healths(&discovered, &active, &PeerManagerPolicy::default());
        for peer in [peer_a, peer_b] {
            assert!(
                !healths[&peer].issues.iter().any(|issue| matches!(
                    issue,
                    PeerManagerHealthIssue::Ipv4SubnetConcentration { .. }
                )),
                "loopback peers should not trip ipv4 subnet concentration: {:?}",
                healths[&peer].issues
            );
        }
    }

    #[test]
    fn recompute_marks_relay_budget_and_domain_concentration() {
        let peer_a = PeerId::random();
        let peer_b = PeerId::random();
        let peer_c = PeerId::random();
        let discovery_sources = vec![
            PeerDiscoverySource::StaticBootstrap,
            PeerDiscoverySource::Dht,
        ];
        let discovered = HashMap::from([
            (peer_a, sample_record(peer_a, discovery_sources.clone())),
            (peer_b, sample_record(peer_b, discovery_sources.clone())),
            (peer_c, sample_record(peer_c, discovery_sources)),
        ]);
        let relay_peer_a = PeerId::random();
        let relay_peer_b = PeerId::random();
        let active = HashMap::from([
            (
                peer_a,
                transport_path(
                    peer_a,
                    format!("/dns4/relay-a.example/tcp/443/p2p/{relay_peer_a}/p2p-circuit")
                        .as_str(),
                    TransportPathKind::RelayReserved,
                ),
            ),
            (
                peer_b,
                transport_path(
                    peer_b,
                    format!("/dns4/relay-a.example/tcp/443/p2p/{relay_peer_b}/p2p-circuit")
                        .as_str(),
                    TransportPathKind::RelayReserved,
                ),
            ),
            (
                peer_c,
                transport_path(
                    peer_c,
                    "/ip4/10.20.30.40/udp/4103/quic-v1",
                    TransportPathKind::Direct,
                ),
            ),
        ]);

        let healths =
            recompute_peer_manager_healths(&discovered, &active, &PeerManagerPolicy::default());
        for peer in [peer_a, peer_b] {
            let health = &healths[&peer];
            assert_eq!(health.status, PeerManagerHealthStatus::Suspect);
            assert!(health
                .issues
                .iter()
                .any(|issue| matches!(issue, PeerManagerHealthIssue::RelayBudgetExceeded { .. })));
            assert!(health.issues.iter().any(|issue| matches!(
                issue,
                PeerManagerHealthIssue::RelayDomainConcentration { relay_domain, .. } if relay_domain == "relay-a.example"
            )));
        }
        assert_eq!(healths[&peer_c].status, PeerManagerHealthStatus::Active);
    }

    #[test]
    fn recompute_keeps_inactive_discovered_peer_as_candidate() {
        let peer = PeerId::random();
        let discovered = HashMap::from([(
            peer,
            sample_record(
                peer,
                vec![
                    PeerDiscoverySource::StaticBootstrap,
                    PeerDiscoverySource::Dht,
                ],
            ),
        )]);

        let healths = recompute_peer_manager_healths(
            &discovered,
            &HashMap::new(),
            &PeerManagerPolicy::default(),
        );
        let health = healths.get(&peer).expect("health");
        assert_eq!(health.status, PeerManagerHealthStatus::Candidate);
        assert!(health.issues.is_empty());
    }

    #[test]
    fn recompute_marks_single_source_discovered_peer_as_suspect_before_activation() {
        let peer = PeerId::random();
        let discovered = HashMap::from([(
            peer,
            sample_record(peer, vec![PeerDiscoverySource::StaticBootstrap]),
        )]);

        let healths = recompute_peer_manager_healths(
            &discovered,
            &HashMap::new(),
            &PeerManagerPolicy::default(),
        );
        let health = healths.get(&peer).expect("health");
        assert_eq!(health.status, PeerManagerHealthStatus::Suspect);
        assert!(health
            .issues
            .iter()
            .any(|issue| matches!(issue, PeerManagerHealthIssue::SingleSourceDiscovery { .. })));
    }
}
