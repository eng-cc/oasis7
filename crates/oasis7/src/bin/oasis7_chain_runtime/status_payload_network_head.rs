use std::collections::{BTreeMap, BTreeSet};

use oasis7::network_tier_manifest::LoadedNetworkTierManifest;
use oasis7_node::{NodePeerCommittedHead, NodeSnapshot};
use serde::Serialize;

const PEER_HEAD_FRESHNESS_TTL_MS: i64 = 10_000;
const DEFAULT_REQUIRED_FRESH_PEER_HEADS: usize = 1;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ChainReadinessPolicyStatus {
    pub(crate) tier: String,
    pub(crate) role: String,
    pub(crate) peer_head_ttl_ms: i64,
    pub(crate) max_network_height_lag: u64,
    pub(crate) sync_stalled_after_ms: i64,
    pub(crate) quorum_mode: String,
    pub(crate) relay_policy: String,
    pub(crate) slashing_policy: String,
    pub(crate) slashing_enforced: bool,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ChainConsensusNetworkHeadStatus {
    pub(crate) source: String,
    pub(crate) height: Option<u64>,
    pub(crate) block_hash: Option<String>,
    pub(crate) execution_block_hash: Option<String>,
    pub(crate) execution_state_root: Option<String>,
    pub(crate) observed_peer_count: usize,
    pub(crate) required_peer_count: usize,
    pub(crate) quorum_mode: String,
    pub(crate) fresh_peer_count: usize,
    pub(crate) stale_peer_count: usize,
    pub(crate) conflicting_peer_count: usize,
    pub(crate) observed_stake: u64,
    pub(crate) required_stake: u64,
    pub(crate) total_stake: u64,
    pub(crate) stake_quorum_met: bool,
    pub(crate) freshness_ttl_ms: i64,
    pub(crate) decision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PeerHeadBucketKey {
    height: u64,
    block_hash: String,
    execution_block_hash: Option<String>,
    execution_state_root: Option<String>,
}

pub(crate) fn build_network_head_status(
    snapshot: &NodeSnapshot,
    observed_at_unix_ms: i64,
    loaded_network_tier_manifest: Option<&LoadedNetworkTierManifest>,
) -> ChainConsensusNetworkHeadStatus {
    let policy = readiness_policy(snapshot, loaded_network_tier_manifest);
    let required_peer_count = required_fresh_peer_heads(snapshot, loaded_network_tier_manifest);
    let mut stale_peer_count = 0usize;
    let mut buckets: BTreeMap<PeerHeadBucketKey, Vec<&NodePeerCommittedHead>> = BTreeMap::new();
    for peer_head in &snapshot.consensus.peer_heads {
        let age_ms = observed_at_unix_ms
            .saturating_sub(peer_head.observed_at_ms)
            .max(0);
        if age_ms > policy.peer_head_ttl_ms {
            stale_peer_count += 1;
            continue;
        }
        buckets
            .entry(PeerHeadBucketKey {
                height: peer_head.height,
                block_hash: peer_head.block_hash.clone(),
                execution_block_hash: peer_head.execution_block_hash.clone(),
                execution_state_root: peer_head.execution_state_root.clone(),
            })
            .or_default()
            .push(peer_head);
    }

    let fresh_peer_count = buckets.values().map(Vec::len).sum::<usize>();
    let mut conflicting_peer_count = 0usize;
    let mut buckets_by_height: BTreeMap<u64, usize> = BTreeMap::new();
    for key in buckets.keys() {
        *buckets_by_height.entry(key.height).or_insert(0) += 1;
    }
    for (height, bucket_count) in buckets_by_height {
        if bucket_count > 1 {
            conflicting_peer_count += buckets
                .iter()
                .filter(|(key, _)| key.height == height)
                .map(|(_, peers)| peers.len())
                .sum::<usize>();
        }
    }

    let selected = buckets
        .iter()
        .filter(|(_, peers)| {
            bucket_quorum_met(
                &policy.quorum_mode,
                required_peer_count,
                snapshot,
                peers.as_slice(),
            )
        })
        .max_by(|left, right| compare_peer_head_buckets(snapshot, *left, *right))
        .or_else(|| {
            buckets
                .iter()
                .max_by(|left, right| compare_peer_head_buckets(snapshot, *left, *right))
        });
    let selected_stake = selected
        .map(|(_, peers)| observed_stake_for_peers(snapshot, peers.as_slice()))
        .unwrap_or(0);
    let required_stake = if policy.quorum_mode == "stake_weighted" {
        snapshot.consensus.required_stake
    } else {
        0
    };
    let stake_quorum_met =
        policy.quorum_mode != "stake_weighted" || selected_stake >= required_stake;

    let (source, decision, selected_key): (&str, &str, Option<PeerHeadBucketKey>) =
        if conflicting_peer_count > 0 {
            (
                "peer_conflict",
                "critical",
                selected.map(|(key, _)| (*key).clone()),
            )
        } else if required_peer_count == 0 {
            (
                "self_only",
                "ready",
                selected.map(|(key, _)| (*key).clone()),
            )
        } else if let Some((key, peers)) = selected {
            if bucket_quorum_met(
                &policy.quorum_mode,
                required_peer_count,
                snapshot,
                peers.as_slice(),
            ) {
                ("peer_quorum", "ready", Some((*key).clone()))
            } else {
                ("peer_single", "degraded", Some((*key).clone()))
            }
        } else if snapshot.replication_enabled {
            ("unknown", "degraded", None)
        } else {
            ("self_only", "ready", None)
        };

    ChainConsensusNetworkHeadStatus {
        source: source.to_string(),
        height: selected_key.as_ref().map(|key| key.height),
        block_hash: selected_key.as_ref().map(|key| key.block_hash.clone()),
        execution_block_hash: selected_key
            .as_ref()
            .and_then(|key| key.execution_block_hash.clone()),
        execution_state_root: selected_key
            .as_ref()
            .and_then(|key| key.execution_state_root.clone()),
        observed_peer_count: snapshot.consensus.peer_heads.len(),
        required_peer_count,
        quorum_mode: policy.quorum_mode,
        fresh_peer_count,
        stale_peer_count,
        conflicting_peer_count,
        observed_stake: selected_stake,
        required_stake,
        total_stake: snapshot.consensus.total_stake,
        stake_quorum_met,
        freshness_ttl_ms: policy.peer_head_ttl_ms,
        decision: decision.to_string(),
    }
}

fn compare_peer_head_buckets(
    snapshot: &NodeSnapshot,
    (left_key, left_peers): (&PeerHeadBucketKey, &Vec<&NodePeerCommittedHead>),
    (right_key, right_peers): (&PeerHeadBucketKey, &Vec<&NodePeerCommittedHead>),
) -> std::cmp::Ordering {
    left_key
        .height
        .cmp(&right_key.height)
        .then_with(|| {
            observed_stake_for_peers(snapshot, left_peers.as_slice())
                .cmp(&observed_stake_for_peers(snapshot, right_peers.as_slice()))
        })
        .then_with(|| left_peers.len().cmp(&right_peers.len()))
}

fn bucket_quorum_met(
    quorum_mode: &str,
    required_peer_count: usize,
    snapshot: &NodeSnapshot,
    peers: &[&NodePeerCommittedHead],
) -> bool {
    if required_peer_count == 0 {
        return true;
    }
    if quorum_mode == "stake_weighted" {
        observed_stake_for_peers(snapshot, peers) >= snapshot.consensus.required_stake
    } else {
        peers.len() >= required_peer_count
    }
}

fn observed_stake_for_peers(snapshot: &NodeSnapshot, peers: &[&NodePeerCommittedHead]) -> u64 {
    let mut seen_validators = BTreeMap::new();
    for peer in peers {
        let Some(validator_id) = peer.validator_id.as_deref() else {
            continue;
        };
        let Some(stake) = snapshot.consensus.validator_stakes.get(validator_id) else {
            continue;
        };
        seen_validators.insert(validator_id.to_string(), *stake);
    }
    seen_validators.values().copied().sum()
}

pub(crate) fn applied_slashing_receipt_hashes(snapshot: &NodeSnapshot) -> BTreeSet<String> {
    snapshot
        .consensus
        .slashing_receipts
        .iter()
        .filter(|receipt| receipt.applied)
        .map(|receipt| receipt.evidence_hash.clone())
        .collect()
}

pub(crate) fn pending_slashing_intent_count(snapshot: &NodeSnapshot) -> usize {
    let applied_receipts = applied_slashing_receipt_hashes(snapshot);
    snapshot
        .consensus
        .slashing_intents
        .iter()
        .filter(|intent| !intent.enforced && !applied_receipts.contains(&intent.evidence_hash))
        .count()
}

pub(crate) fn readiness_policy(
    snapshot: &NodeSnapshot,
    loaded_network_tier_manifest: Option<&LoadedNetworkTierManifest>,
) -> ChainReadinessPolicyStatus {
    let tier = loaded_network_tier_manifest
        .map(|loaded| loaded.manifest.tier.as_str())
        .unwrap_or("unspecified");
    let is_observer = snapshot.role.as_str() == "observer";
    let (peer_head_ttl_ms, max_network_height_lag, sync_stalled_after_ms) = match tier {
        "local_devnet" => (i64::MAX, 0, i64::MAX),
        "shared_devnet" => (15_000, 2, 60_000),
        "public_testnet" => (10_000, 1, 30_000),
        "mainnet" => (5_000, if is_observer { 1 } else { 0 }, 15_000),
        _ => (PEER_HEAD_FRESHNESS_TTL_MS, 0, i64::MAX),
    };
    let stake_ready = snapshot.consensus.required_stake > 0
        && snapshot.consensus.total_stake > 0
        && !snapshot.consensus.validator_stakes.is_empty()
        && !snapshot.consensus.validator_set_hash.is_empty()
        && !snapshot.consensus.validator_stake_root.is_empty()
        && snapshot.consensus.validator_stake_proofs.len()
            == snapshot.consensus.validator_stakes.len();
    let quorum_mode = if tier == "mainnet" && !is_observer && stake_ready {
        "stake_weighted"
    } else if tier == "mainnet" && !is_observer {
        "count_fallback_stake_unavailable"
    } else {
        "count"
    };
    let relay_policy = match (tier, is_observer) {
        ("mainnet", false) => "public_direct_or_governed_relay",
        ("mainnet", true) => "public_direct_or_relay_or_persistent_peer",
        ("public_testnet", false) => "public_direct_or_relay",
        (_, true) => "outbound_or_relay",
        _ => "public_direct_or_relay",
    };
    let slashing_policy = if tier == "mainnet" {
        "evidence_only_readiness_gate"
    } else {
        "disabled_for_non_mainnet"
    };
    ChainReadinessPolicyStatus {
        tier: tier.to_string(),
        role: snapshot.role.as_str().to_string(),
        peer_head_ttl_ms,
        max_network_height_lag,
        sync_stalled_after_ms,
        quorum_mode: quorum_mode.to_string(),
        relay_policy: relay_policy.to_string(),
        slashing_policy: slashing_policy.to_string(),
        slashing_enforced: false,
    }
}

fn required_fresh_peer_heads(
    snapshot: &NodeSnapshot,
    loaded_network_tier_manifest: Option<&LoadedNetworkTierManifest>,
) -> usize {
    if !snapshot.replication_enabled {
        return 0;
    }
    let Some(loaded) = loaded_network_tier_manifest else {
        return DEFAULT_REQUIRED_FRESH_PEER_HEADS;
    };
    let target_validators = loaded.manifest.validator_policy.target_validator_count as usize;
    let max_peer_validators = target_validators.saturating_sub(1).max(1);
    match (loaded.manifest.tier.as_str(), snapshot.role.as_str()) {
        ("local_devnet", _) => 0,
        ("shared_devnet", "observer") => 1,
        ("shared_devnet", _) => 1,
        ("public_testnet", "observer") => 1,
        ("public_testnet", _) => max_peer_validators.min(2).max(1),
        ("mainnet", "observer") => max_peer_validators.min(2).max(1),
        ("mainnet", _) => {
            let two_thirds = ((target_validators * 2) + 2) / 3;
            two_thirds.min(max_peer_validators).max(1)
        }
        (_, _) => DEFAULT_REQUIRED_FRESH_PEER_HEADS,
    }
}
