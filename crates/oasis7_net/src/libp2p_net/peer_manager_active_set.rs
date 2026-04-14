use std::collections::{BTreeSet, HashMap};

use libp2p::PeerId;
use oasis7_proto::distributed_dht::{PeerDiscoverySource, SignedPeerRecord};

use super::peer_manager::{
    discovery_source_label, exceeds_share_limit, ipv4_subnet_bucket, meets_or_exceeds_share_limit,
    normalized_source_label, relay_domain, PeerManagerHealthStatus, PeerManagerPolicy,
};
use super::transport_paths::{TransportPath, TransportPathKind};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ActivePeerSetStats {
    pub active_peer_count: usize,
    pub active_discovery_sources: BTreeSet<&'static str>,
    pub ipv4_subnet_counts: HashMap<String, usize>,
    pub relay_domain_counts: HashMap<String, usize>,
    pub operator_counts: HashMap<String, usize>,
    pub asn_counts: HashMap<String, usize>,
    pub relayed_active_peers: usize,
}

impl ActivePeerSetStats {
    pub(super) fn new(
        discovered_peer_records: &HashMap<PeerId, SignedPeerRecord>,
        active_transport_paths: &HashMap<PeerId, TransportPath>,
    ) -> Self {
        let mut stats = Self {
            active_peer_count: active_transport_paths.len(),
            ..Self::default()
        };

        for (peer_id, active_path) in active_transport_paths {
            stats.note_active_path(active_path);
            if let Some(record) = discovered_peer_records.get(peer_id) {
                stats.note_record(record);
            }
        }

        stats
    }

    pub(super) fn add_admitted_peer(
        &mut self,
        record: &SignedPeerRecord,
        active_path: &TransportPath,
    ) {
        self.active_peer_count = self.active_peer_count.saturating_add(1);
        self.note_active_path(active_path);
        self.note_record(record);
    }

    fn note_record(&mut self, record: &SignedPeerRecord) {
        for source in &record.record.discovery_sources {
            self.active_discovery_sources
                .insert(discovery_source_label(*source));
        }
        if let Some(source_operator) =
            normalized_source_label(record.record.source_operator.as_deref())
        {
            *self.operator_counts.entry(source_operator).or_default() += 1;
        }
        if let Some(source_asn) = normalized_source_label(record.record.source_asn.as_deref()) {
            *self.asn_counts.entry(source_asn).or_default() += 1;
        }
    }

    fn note_active_path(&mut self, active_path: &TransportPath) {
        if let Some(bucket) = ipv4_subnet_bucket(active_path) {
            *self.ipv4_subnet_counts.entry(bucket).or_default() += 1;
        }
        if matches!(active_path.kind, TransportPathKind::RelayReserved) {
            self.relayed_active_peers += 1;
            if let Some(domain) = relay_domain(active_path) {
                *self.relay_domain_counts.entry(domain).or_default() += 1;
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ActivePeerCandidate {
    pub discovery_sources: Vec<PeerDiscoverySource>,
    pub ipv4_subnet_bucket: Option<String>,
    pub relay_domain: Option<String>,
    pub source_operator: Option<String>,
    pub source_asn: Option<String>,
    pub relay_reserved: bool,
}

impl ActivePeerCandidate {
    pub(super) fn from_record_and_path(
        record: &SignedPeerRecord,
        active_path: &TransportPath,
    ) -> Self {
        Self {
            discovery_sources: record.record.discovery_sources.clone(),
            ipv4_subnet_bucket: ipv4_subnet_bucket(active_path),
            relay_domain: relay_domain(active_path),
            source_operator: normalized_source_label(record.record.source_operator.as_deref()),
            source_asn: normalized_source_label(record.record.source_asn.as_deref()),
            relay_reserved: matches!(active_path.kind, TransportPathKind::RelayReserved),
        }
    }
}

pub(super) fn candidate_status_with_active_set(
    candidate: &ActivePeerCandidate,
    active_set_stats: &ActivePeerSetStats,
    policy: &PeerManagerPolicy,
) -> PeerManagerHealthStatus {
    let projected_active_peer_count = active_set_stats.active_peer_count.saturating_add(1);
    let mut has_issue = candidate.discovery_sources.len() < policy.min_peer_discovery_sources;
    let mut hard_block = false;

    let projected_active_discovery_sources = candidate
        .discovery_sources
        .iter()
        .map(|source| discovery_source_label(*source))
        .filter(|source| !active_set_stats.active_discovery_sources.contains(source))
        .count()
        .saturating_add(active_set_stats.active_discovery_sources.len());
    if projected_active_peer_count > 0
        && projected_active_discovery_sources < policy.min_active_discovery_sources
    {
        has_issue = true;
    }

    if let Some(bucket) = candidate.ipv4_subnet_bucket.as_deref() {
        let projected_bucket_count = active_set_stats
            .ipv4_subnet_counts
            .get(bucket)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        if projected_bucket_count >= 2
            && meets_or_exceeds_share_limit(
                projected_bucket_count,
                projected_active_peer_count,
                policy.block_ipv4_subnet_share_per_mille,
            )
        {
            hard_block = true;
        } else if projected_bucket_count >= 2
            && exceeds_share_limit(
                projected_bucket_count,
                projected_active_peer_count,
                policy.max_ipv4_subnet_share_per_mille,
            )
        {
            has_issue = true;
        }
    }

    if candidate.relay_reserved {
        let projected_relayed_active_peers =
            active_set_stats.relayed_active_peers.saturating_add(1);
        if exceeds_share_limit(
            projected_relayed_active_peers,
            projected_active_peer_count,
            policy.max_relayed_active_peer_share_per_mille,
        ) {
            has_issue = true;
        }
        if let Some(domain) = candidate.relay_domain.as_deref() {
            let projected_bucket_count = active_set_stats
                .relay_domain_counts
                .get(domain)
                .copied()
                .unwrap_or(0)
                .saturating_add(1);
            if projected_bucket_count >= 2
                && meets_or_exceeds_share_limit(
                    projected_bucket_count,
                    projected_active_peer_count,
                    policy.block_relay_domain_share_per_mille,
                )
            {
                hard_block = true;
            } else if projected_bucket_count >= 2
                && exceeds_share_limit(
                    projected_bucket_count,
                    projected_active_peer_count,
                    policy.max_relay_domain_share_per_mille,
                )
            {
                has_issue = true;
            }
        }
    }

    if let Some(source_operator) = candidate.source_operator.as_deref() {
        let projected_bucket_count = active_set_stats
            .operator_counts
            .get(source_operator)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        if projected_bucket_count >= 2
            && meets_or_exceeds_share_limit(
                projected_bucket_count,
                projected_active_peer_count,
                policy.block_operator_share_per_mille,
            )
        {
            hard_block = true;
        } else if projected_bucket_count >= 2
            && exceeds_share_limit(
                projected_bucket_count,
                projected_active_peer_count,
                policy.max_operator_share_per_mille,
            )
        {
            has_issue = true;
        }
    }

    if let Some(source_asn) = candidate.source_asn.as_deref() {
        let projected_bucket_count = active_set_stats
            .asn_counts
            .get(source_asn)
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        if projected_bucket_count >= 2
            && meets_or_exceeds_share_limit(
                projected_bucket_count,
                projected_active_peer_count,
                policy.block_asn_share_per_mille,
            )
        {
            hard_block = true;
        } else if projected_bucket_count >= 2
            && exceeds_share_limit(
                projected_bucket_count,
                projected_active_peer_count,
                policy.max_asn_share_per_mille,
            )
        {
            has_issue = true;
        }
    }

    if hard_block {
        PeerManagerHealthStatus::Blocked
    } else if has_issue {
        PeerManagerHealthStatus::Suspect
    } else {
        PeerManagerHealthStatus::Active
    }
}

pub(super) fn candidate_would_degrade_admitted_peers(
    candidate: &ActivePeerCandidate,
    active_set_stats: &ActivePeerSetStats,
    policy: &PeerManagerPolicy,
) -> bool {
    let projected_active_peer_count = active_set_stats.active_peer_count.saturating_add(1);

    if let Some(bucket) = candidate.ipv4_subnet_bucket.as_deref() {
        let current_bucket_count = active_set_stats
            .ipv4_subnet_counts
            .get(bucket)
            .copied()
            .unwrap_or(0);
        let projected_bucket_count = current_bucket_count.saturating_add(1);
        if current_bucket_count > 0
            && projected_bucket_count >= 2
            && (meets_or_exceeds_share_limit(
                projected_bucket_count,
                projected_active_peer_count,
                policy.block_ipv4_subnet_share_per_mille,
            ) || exceeds_share_limit(
                projected_bucket_count,
                projected_active_peer_count,
                policy.max_ipv4_subnet_share_per_mille,
            ))
        {
            return true;
        }
    }

    if candidate.relay_reserved {
        let projected_relayed_active_peers =
            active_set_stats.relayed_active_peers.saturating_add(1);
        if active_set_stats.relayed_active_peers > 0
            && exceeds_share_limit(
                projected_relayed_active_peers,
                projected_active_peer_count,
                policy.max_relayed_active_peer_share_per_mille,
            )
        {
            return true;
        }
        if let Some(domain) = candidate.relay_domain.as_deref() {
            let current_bucket_count = active_set_stats
                .relay_domain_counts
                .get(domain)
                .copied()
                .unwrap_or(0);
            let projected_bucket_count = current_bucket_count.saturating_add(1);
            if current_bucket_count > 0
                && projected_bucket_count >= 2
                && (meets_or_exceeds_share_limit(
                    projected_bucket_count,
                    projected_active_peer_count,
                    policy.block_relay_domain_share_per_mille,
                ) || exceeds_share_limit(
                    projected_bucket_count,
                    projected_active_peer_count,
                    policy.max_relay_domain_share_per_mille,
                ))
            {
                return true;
            }
        }
    }

    if let Some(source_operator) = candidate.source_operator.as_deref() {
        let current_bucket_count = active_set_stats
            .operator_counts
            .get(source_operator)
            .copied()
            .unwrap_or(0);
        let projected_bucket_count = current_bucket_count.saturating_add(1);
        if current_bucket_count > 0
            && projected_bucket_count >= 2
            && (meets_or_exceeds_share_limit(
                projected_bucket_count,
                projected_active_peer_count,
                policy.block_operator_share_per_mille,
            ) || exceeds_share_limit(
                projected_bucket_count,
                projected_active_peer_count,
                policy.max_operator_share_per_mille,
            ))
        {
            return true;
        }
    }

    if let Some(source_asn) = candidate.source_asn.as_deref() {
        let current_bucket_count = active_set_stats
            .asn_counts
            .get(source_asn)
            .copied()
            .unwrap_or(0);
        let projected_bucket_count = current_bucket_count.saturating_add(1);
        if current_bucket_count > 0
            && projected_bucket_count >= 2
            && (meets_or_exceeds_share_limit(
                projected_bucket_count,
                projected_active_peer_count,
                policy.block_asn_share_per_mille,
            ) || exceeds_share_limit(
                projected_bucket_count,
                projected_active_peer_count,
                policy.max_asn_share_per_mille,
            ))
        {
            return true;
        }
    }

    false
}
