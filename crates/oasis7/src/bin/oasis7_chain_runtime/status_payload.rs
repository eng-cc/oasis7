use std::path::Path;

use oasis7::network_tier_manifest::LoadedNetworkTierManifest;
use oasis7::runtime::ReleaseSecurityPolicy;
use oasis7_node::{
    Libp2pReachabilitySnapshot, NodeNetworkPolicy, NodeReachabilityAutoDetection, NodeSnapshot,
    NodeUserModeRecommendation,
};
use serde::Serialize;

use super::p2p_status::peer_reachability_as_str;
use super::runtime_status_util::{consensus_status_to_string, now_unix_ms};
use super::storage_metrics;
use super::traffic_status::ChainTrafficStatus;
use super::wasm_status::ChainWasmStatus;
#[path = "status_payload_network_head.rs"]
mod status_payload_network_head;
pub(super) use status_payload_network_head::{
    applied_slashing_receipt_hashes, build_network_head_status, pending_slashing_intent_count,
    readiness_policy, ChainConsensusNetworkHeadStatus, ChainReadinessPolicyStatus,
};
#[path = "status_payload_state_sync.rs"]
mod status_payload_state_sync;
use status_payload_state_sync::{
    consensus_participation_hold_reason, state_sync_fallback_reason,
    state_sync_trusted_checkpoint_required_height,
};

const TRANSPORT_STABILITY_MIN_SCORE: u8 = 70;

#[derive(Debug, Serialize)]
pub(super) struct ChainP2pStatus {
    pub(super) requested_user_mode: String,
    pub(super) recommended_user_mode: String,
    pub(super) effective_user_mode: String,
    pub(super) applied_effective_user_mode: Option<String>,
    pub(super) requires_explicit_public_entry_confirmation: bool,
    pub(super) detected_reachability: Option<String>,
    pub(super) hole_punch_viability: String,
    pub(super) autonat_status: String,
    pub(super) public_port_reachability: String,
    pub(super) observed_public_addr: Option<String>,
    pub(super) confirmed_external_direct_addrs: Vec<String>,
    pub(super) relay_available: bool,
    pub(super) probe_stable: bool,
    pub(super) deployment_mode: String,
    pub(super) node_role_claim: String,
    pub(super) rationale: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ChainStatusResponse {
    pub(super) ok: bool,
    pub(super) observed_at_unix_ms: i64,
    pub(super) node_id: String,
    pub(super) world_id: String,
    pub(super) role: String,
    pub(super) running: bool,
    pub(super) liveness: ChainLivenessStatus,
    pub(super) readiness: ChainReadinessStatus,
    pub(super) sync: ChainSyncStatus,
    pub(super) worker_poll_count: u64,
    pub(super) tick_count: u64,
    pub(super) last_tick_unix_ms: Option<i64>,
    pub(super) consensus: ChainConsensusStatus,
    pub(super) last_error: Option<String>,
    pub(super) execution_world_dir: String,
    pub(super) network_tier: Option<ChainNetworkTierStatus>,
    pub(super) p2p: ChainP2pStatus,
    pub(super) observability: ChainNodeObservabilityStatus,
    pub(super) release_security_policy: ReleaseSecurityPolicy,
    pub(super) reward_runtime: super::reward_runtime_worker::RewardRuntimeMetricsSnapshot,
    pub(super) storage: storage_metrics::StorageMetricsSnapshot,
    pub(super) wasm: ChainWasmStatus,
    pub(super) traffic: ChainTrafficStatus,
    pub(super) transactions: super::transfer_submit_api::ChainTransferMetricsStatus,
    pub(super) replication: super::ChainReplicationDebugStatus,
}

#[derive(Debug, Serialize)]
pub(super) struct ChainLivenessStatus {
    pub(super) status: String,
    pub(super) running: bool,
    pub(super) runtime_last_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ChainReadinessStatus {
    pub(super) status: String,
    pub(super) ready: bool,
    pub(super) failed_gates: Vec<String>,
    pub(super) policy: ChainReadinessPolicyStatus,
}

#[derive(Debug, Serialize)]
pub(super) struct ChainSyncStatus {
    pub(super) status: String,
    pub(super) network_head_source: String,
    pub(super) network_height_lag: u64,
    pub(super) fresh_peer_count: usize,
    pub(super) stale_peer_count: usize,
    pub(super) conflicting_peer_count: usize,
}

#[derive(Debug, Serialize)]
pub(super) struct ChainNetworkTierStatus {
    pub(super) source_path: String,
    pub(super) schema_version: String,
    pub(super) tier: String,
    pub(super) status: String,
    pub(super) network_id: String,
    pub(super) chain_id: String,
    pub(super) bootstrap_peer_count: usize,
    pub(super) governance_mode: String,
    pub(super) validator_admission: String,
    pub(super) target_validator_count: u64,
    pub(super) allow_observer_nodes: bool,
    pub(super) token_symbol: String,
    pub(super) faucet_mode: String,
    pub(super) reset_policy: String,
    pub(super) value_semantics: String,
    pub(super) rpc_ref: String,
    pub(super) explorer_ref: String,
    pub(super) faucet_ref: Option<String>,
    pub(super) required_gates: Vec<String>,
    pub(super) allowed_claims: Vec<String>,
    pub(super) denied_claims: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ChainNodeObservabilityStatus {
    pub(super) status: String,
    pub(super) summary: String,
    pub(super) ready: bool,
    pub(super) connected_peer_count: usize,
    pub(super) active_peer_count: usize,
    pub(super) candidate_peer_count: usize,
    pub(super) suspect_peer_count: usize,
    pub(super) blocked_peer_count: usize,
    pub(super) peer_with_issues_count: usize,
    pub(super) known_peer_heads: usize,
    pub(super) network_head_available: bool,
    pub(super) network_height_lag: u64,
    pub(super) transport_stable: bool,
    pub(super) transport_stability_score: u8,
    pub(super) reachability_policy_ok: bool,
    pub(super) misbehavior_evidence_count: usize,
    pub(super) slashing_intent_count: usize,
    pub(super) pending_slashing_intent_count: usize,
    pub(super) slashing_receipt_count: usize,
    pub(super) applied_slashing_receipt_count: usize,
    pub(super) quarantined_validator_count: usize,
    pub(super) slashable_stake_total: u64,
    pub(super) replication_enabled: bool,
    pub(super) replication_persisted_height: u64,
    pub(super) replication_state_gap: u64,
    pub(super) replication_gap_sync_blocked_height: Option<u64>,
    pub(super) recent_replication_error_count: usize,
    pub(super) storage_degraded: bool,
    pub(super) reward_runtime_degraded: bool,
    pub(super) alerts: Vec<ChainNodeObservabilityAlert>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ChainReplicationTransportStability {
    pub(super) stable: bool,
    pub(super) score: u8,
    pub(super) recent_error_count: usize,
    pub(super) connection_closed_count: usize,
    pub(super) insufficient_peers_count: usize,
    pub(super) timeout_count: usize,
    pub(super) protocol_error_count: usize,
}

#[derive(Debug, Serialize)]
pub(super) struct ChainNodeObservabilityAlert {
    pub(super) severity: String,
    pub(super) code: String,
    pub(super) summary: String,
}

#[derive(Debug, Serialize)]
pub(super) struct ChainConsensusStatus {
    pub(super) slot: u64,
    pub(super) epoch: u64,
    pub(super) ticks_per_slot: u64,
    pub(super) tick_phase: u64,
    pub(super) proposal_tick_phase: u64,
    pub(super) last_observed_slot: u64,
    pub(super) missed_slot_count: u64,
    pub(super) last_observed_tick: u64,
    pub(super) missed_tick_count: u64,
    pub(super) adaptive_tick_scheduler_enabled: bool,
    pub(super) latest_height: u64,
    pub(super) committed_height: u64,
    pub(super) last_committed_at_ms: Option<i64>,
    pub(super) last_commit_age_ms: Option<i64>,
    pub(super) network_committed_height: u64,
    pub(super) replication_enabled: bool,
    pub(super) replication_persisted_height: u64,
    pub(super) replication_gap_sync_blocked_height: Option<u64>,
    pub(super) replication_gap_sync_blocked_reason: Option<String>,
    pub(super) replication_gap_sync_repair_attempt_height: Option<u64>,
    pub(super) replication_gap_sync_repair_attempt_summary: Option<String>,
    pub(super) state_sync_fallback_required: bool,
    pub(super) state_sync_snapshot_available: bool,
    pub(super) state_sync_trusted_checkpoint_required_height: Option<u64>,
    pub(super) state_sync_fallback_reason: Option<String>,
    pub(super) consensus_participation_held: bool,
    pub(super) consensus_participation_hold_reason: Option<String>,
    pub(super) recent_finality_latency: ChainFinalityLatencyStatus,
    pub(super) pending_proposal: Option<ChainPendingProposalStatus>,
    pub(super) pending_consensus_actions: ChainPendingConsensusActionsStatus,
    pub(super) inbound_timing_rejections: ChainInboundTimingRejectionsStatus,
    pub(super) last_status: Option<String>,
    pub(super) last_block_hash: Option<String>,
    pub(super) last_execution_height: u64,
    pub(super) last_execution_block_hash: Option<String>,
    pub(super) last_execution_state_root: Option<String>,
    pub(super) known_peer_heads: usize,
    pub(super) validator_set_hash: String,
    pub(super) validator_stake_root: String,
    pub(super) validator_stake_proof_count: usize,
    pub(super) misbehavior_evidence_count: usize,
    pub(super) slashing_intent_count: usize,
    pub(super) pending_slashing_intent_count: usize,
    pub(super) slashing_receipt_count: usize,
    pub(super) applied_slashing_receipt_count: usize,
    pub(super) quarantined_validator_count: usize,
    pub(super) slashable_stake_total: u64,
    pub(super) network_head: ChainConsensusNetworkHeadStatus,
}

#[derive(Debug, Serialize)]
pub(super) struct ChainPendingProposalStatus {
    pub(super) height: u64,
    pub(super) slot: u64,
    pub(super) epoch: u64,
    pub(super) proposer_id: String,
    pub(super) opened_at_ms: i64,
    pub(super) age_ms: i64,
    pub(super) action_count: usize,
    pub(super) action_payload_bytes: usize,
    pub(super) attestation_count: usize,
    pub(super) approved_stake: u64,
    pub(super) rejected_stake: u64,
    pub(super) required_stake: u64,
    pub(super) total_stake: u64,
    pub(super) approval_progress_bps: u16,
    pub(super) rejection_progress_bps: u16,
    pub(super) remaining_approval_stake: u64,
    pub(super) status: String,
}

#[derive(Debug, Serialize)]
pub(super) struct ChainPendingConsensusActionsStatus {
    pub(super) queued_action_count: usize,
    pub(super) queued_payload_bytes: usize,
    pub(super) reserved_requeue_action_count: usize,
    pub(super) reserved_requeue_payload_bytes: usize,
    pub(super) available_capacity: usize,
    pub(super) max_capacity: usize,
    pub(super) submit_buffer_action_count: usize,
    pub(super) submit_buffer_payload_bytes: usize,
    pub(super) submit_buffer_max_capacity: usize,
}

#[derive(Debug, Serialize)]
pub(super) struct ChainInboundTimingRejectionsStatus {
    pub(super) proposal_future_slot: u64,
    pub(super) proposal_stale_slot: u64,
    pub(super) attestation_future_slot: u64,
    pub(super) attestation_stale_slot: u64,
    pub(super) attestation_epoch_mismatch: u64,
    pub(super) last_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ChainFinalityLatencyStatus {
    pub(super) sample_count: usize,
    pub(super) avg_latency_ms: Option<i64>,
    pub(super) max_latency_ms: Option<i64>,
    pub(super) p50_latency_ms: Option<i64>,
    pub(super) p95_latency_ms: Option<i64>,
}

pub(super) fn build_chain_p2p_status(
    live_p2p_recommendation: &NodeUserModeRecommendation,
    applied_effective_user_mode: Option<String>,
    effective_p2p_policy: NodeNetworkPolicy,
    live_snapshot: &Libp2pReachabilitySnapshot,
    p2p_detection: NodeReachabilityAutoDetection,
) -> ChainP2pStatus {
    ChainP2pStatus {
        requested_user_mode: live_p2p_recommendation
            .requested_user_mode
            .as_str()
            .to_string(),
        recommended_user_mode: live_p2p_recommendation
            .recommended_user_mode
            .as_str()
            .to_string(),
        effective_user_mode: live_p2p_recommendation
            .effective_user_mode
            .as_str()
            .to_string(),
        applied_effective_user_mode,
        requires_explicit_public_entry_confirmation: live_p2p_recommendation
            .requires_explicit_public_entry_confirmation,
        detected_reachability: p2p_detection
            .observed_reachability
            .map(peer_reachability_as_str)
            .map(str::to_string),
        hole_punch_viability: p2p_detection.hole_punch_viability.to_string(),
        autonat_status: p2p_detection.autonat_status.to_string(),
        public_port_reachability: p2p_detection.public_port_reachability.to_string(),
        observed_public_addr: live_snapshot.observed_public_addr.clone(),
        confirmed_external_direct_addrs: live_snapshot.confirmed_external_direct_addrs.clone(),
        relay_available: p2p_detection.relay_available,
        probe_stable: p2p_detection.probe_stable,
        deployment_mode: effective_p2p_policy.deployment_mode.as_str().to_string(),
        node_role_claim: effective_p2p_policy.node_role_claim.as_str().to_string(),
        rationale: live_p2p_recommendation.rationale.clone(),
    }
}

fn push_observability_alert(
    alerts: &mut Vec<ChainNodeObservabilityAlert>,
    severity: &str,
    code: &str,
    summary: String,
) {
    alerts.push(ChainNodeObservabilityAlert {
        severity: severity.to_string(),
        code: code.to_string(),
        summary,
    });
}

fn observability_status_for_alerts(alerts: &[ChainNodeObservabilityAlert]) -> String {
    if alerts.iter().any(|alert| alert.severity == "critical") {
        "critical".to_string()
    } else if alerts.iter().any(|alert| alert.severity == "warn") {
        "warn".to_string()
    } else {
        "ok".to_string()
    }
}

fn observability_summary_for_alerts(alerts: &[ChainNodeObservabilityAlert]) -> String {
    match alerts {
        [] => "no active node alerts".to_string(),
        [only] => only.summary.clone(),
        _ => format!("{}; +{} more alerts", alerts[0].summary, alerts.len() - 1),
    }
}

pub(super) fn classify_transport_stability(
    replication: &super::ChainReplicationDebugStatus,
) -> ChainReplicationTransportStability {
    let mut connection_closed_count = 0usize;
    let mut insufficient_peers_count = 0usize;
    let mut timeout_count = 0usize;
    let mut protocol_error_count = 0usize;
    for error in &replication.recent_errors {
        let lower = error.to_ascii_lowercase();
        if lower.contains("connectionclosed") || lower.contains("connection closed") {
            connection_closed_count += 1;
        }
        if lower.contains("insufficientpeers") || lower.contains("insufficient peers") {
            insufficient_peers_count += 1;
        }
        if lower.contains("timeout") {
            timeout_count += 1;
        }
        if lower.contains("protocol")
            || lower.contains("unsupported")
            || lower.contains("mismatch")
            || lower.contains("unavailable")
        {
            protocol_error_count += 1;
        }
    }
    let penalty = connection_closed_count
        .saturating_mul(5)
        .saturating_add(insufficient_peers_count.saturating_mul(10))
        .saturating_add(timeout_count.saturating_mul(10))
        .saturating_add(protocol_error_count.saturating_mul(15))
        .saturating_add(
            replication
                .transport_retry_cooldown_peers
                .len()
                .saturating_mul(15),
        )
        .saturating_add(
            replication
                .protocol_retry_cooldown_peers
                .values()
                .map(Vec::len)
                .sum::<usize>()
                .saturating_mul(20),
        );
    let score = 100u8.saturating_sub(penalty.min(100) as u8);
    ChainReplicationTransportStability {
        stable: score >= TRANSPORT_STABILITY_MIN_SCORE,
        score,
        recent_error_count: replication.recent_errors.len(),
        connection_closed_count,
        insufficient_peers_count,
        timeout_count,
        protocol_error_count,
    }
}

fn reachability_policy_ok(
    snapshot: &NodeSnapshot,
    p2p: &ChainP2pStatus,
    active_peer_count: usize,
    policy: &ChainReadinessPolicyStatus,
) -> bool {
    if !snapshot.replication_enabled {
        return true;
    }
    if snapshot.role.as_str() == "observer" {
        return active_peer_count > 0 || p2p.relay_available;
    }
    let has_public_direct = p2p.detected_reachability.as_deref() == Some("public")
        || p2p.autonat_status == "public"
        || p2p.public_port_reachability == "reachable"
        || !p2p.confirmed_external_direct_addrs.is_empty()
        || p2p.observed_public_addr.is_some();
    if policy.tier == "mainnet" && policy.relay_policy == "public_direct_or_governed_relay" {
        return has_public_direct || (p2p.relay_available && active_peer_count >= 2);
    }
    has_public_direct || p2p.relay_available
}

fn build_liveness_status(snapshot: &NodeSnapshot) -> ChainLivenessStatus {
    let status = if snapshot.running && snapshot.last_error.is_none() {
        "ok"
    } else {
        "critical"
    };
    ChainLivenessStatus {
        status: status.to_string(),
        running: snapshot.running,
        runtime_last_error: snapshot.last_error.clone(),
    }
}

fn build_readiness_status(
    observability: &ChainNodeObservabilityStatus,
    policy: ChainReadinessPolicyStatus,
) -> ChainReadinessStatus {
    let failed_gates = observability
        .alerts
        .iter()
        .map(|alert| alert.code.clone())
        .collect::<Vec<_>>();
    ChainReadinessStatus {
        status: if observability.ready {
            "ready"
        } else {
            "not_ready"
        }
        .to_string(),
        ready: observability.ready,
        failed_gates,
        policy,
    }
}

pub(super) fn build_sync_status(
    network_head: &ChainConsensusNetworkHeadStatus,
    network_height_lag: u64,
    policy: &ChainReadinessPolicyStatus,
    snapshot: &NodeSnapshot,
    observed_at_unix_ms: i64,
) -> ChainSyncStatus {
    let last_commit_age_ms = snapshot
        .consensus
        .last_committed_at_ms
        .map(|last_ms| observed_at_unix_ms.saturating_sub(last_ms).max(0));
    let stalled = network_height_lag > 0
        && last_commit_age_ms
            .map(|age_ms| age_ms > policy.sync_stalled_after_ms)
            .unwrap_or(false);
    let status = if network_head.conflicting_peer_count > 0 {
        "conflicting"
    } else if network_head.fresh_peer_count == 0 && network_head.required_peer_count > 0 {
        "unknown"
    } else if stalled {
        "stalled"
    } else if network_height_lag > 0 {
        "catching_up"
    } else if network_head.stale_peer_count > 0 {
        "stale_peer_view"
    } else {
        "synced"
    };
    ChainSyncStatus {
        status: status.to_string(),
        network_head_source: network_head.source.clone(),
        network_height_lag,
        fresh_peer_count: network_head.fresh_peer_count,
        stale_peer_count: network_head.stale_peer_count,
        conflicting_peer_count: network_head.conflicting_peer_count,
    }
}

pub(super) fn build_chain_node_observability_status(
    snapshot: &NodeSnapshot,
    storage_metrics: &storage_metrics::StorageMetricsSnapshot,
    reward_runtime_metrics: &super::reward_runtime_worker::RewardRuntimeMetricsSnapshot,
    replication: &super::ChainReplicationDebugStatus,
    network_head: &ChainConsensusNetworkHeadStatus,
    p2p: &ChainP2pStatus,
    policy: &ChainReadinessPolicyStatus,
    observed_at_unix_ms: i64,
) -> ChainNodeObservabilityStatus {
    let connected_peer_count = replication.connected_peers.len();
    let mut active_peer_count = 0usize;
    let mut candidate_peer_count = 0usize;
    let mut suspect_peer_count = 0usize;
    let mut blocked_peer_count = 0usize;
    for health in &replication.peer_healths {
        match health.status.as_str() {
            "active" => active_peer_count += 1,
            "candidate" => candidate_peer_count += 1,
            "suspect" => suspect_peer_count += 1,
            "blocked" => blocked_peer_count += 1,
            _ => {}
        }
    }
    let peer_with_issues_count = replication
        .peer_healths
        .iter()
        .filter(|health| !health.issues.is_empty())
        .count();
    let known_peer_heads = snapshot.consensus.known_peer_heads;
    let network_head_available =
        matches!(network_head.source.as_str(), "peer_quorum" | "peer_single");
    let network_height_lag = snapshot
        .consensus
        .network_committed_height
        .saturating_sub(snapshot.consensus.committed_height);
    let replication_state_gap = if snapshot.replication_enabled {
        snapshot
            .consensus
            .committed_height
            .saturating_sub(snapshot.consensus.replication_persisted_height)
    } else {
        0
    };
    let consensus_participation_hold_reason = consensus_participation_hold_reason(
        snapshot,
        network_height_lag,
        replication_state_gap,
        policy.max_network_height_lag,
    );
    let state_sync_fallback_reason =
        state_sync_fallback_reason(snapshot, replication_state_gap, network_height_lag);
    let recent_replication_error_count = replication.recent_errors.len();
    let transport_stability = classify_transport_stability(replication);
    let reachability_policy_ok = reachability_policy_ok(snapshot, p2p, active_peer_count, policy);
    let storage_degraded = storage_metrics.degraded_reason.is_some()
        || matches!(storage_metrics.last_gc_result.as_str(), "failed");
    let reward_runtime_degraded = reward_runtime_metrics.enabled
        && (!reward_runtime_metrics.metrics_available
            || !reward_runtime_metrics.invariant_ok
            || reward_runtime_metrics.last_error.is_some());
    let slashable_stake_total = snapshot
        .consensus
        .misbehavior_evidence
        .iter()
        .map(|evidence| evidence.slashable_stake)
        .sum::<u64>();

    let mut alerts = Vec::new();
    if let Some(err) = snapshot.last_error.as_ref() {
        push_observability_alert(
            &mut alerts,
            "critical",
            "runtime_last_error",
            format!("runtime last_error is set: {err}"),
        );
    }
    if network_height_lag > policy.max_network_height_lag {
        push_observability_alert(
            &mut alerts,
            "warn",
            "consensus_network_lag",
            format!(
                "network committed height is ahead by {network_height_lag}; allowed lag is {}",
                policy.max_network_height_lag
            ),
        );
    }
    if network_height_lag > 0
        && snapshot
            .consensus
            .last_committed_at_ms
            .map(|last_ms| observed_at_unix_ms.saturating_sub(last_ms).max(0))
            .map(|age_ms| age_ms > policy.sync_stalled_after_ms)
            .unwrap_or(false)
    {
        push_observability_alert(
            &mut alerts,
            "critical",
            "consensus_sync_stalled",
            format!(
                "sync lag remains {} and last commit age exceeded {}ms",
                network_height_lag, policy.sync_stalled_after_ms
            ),
        );
    }
    if snapshot.replication_enabled && network_head.fresh_peer_count == 0 {
        push_observability_alert(
            &mut alerts,
            "warn",
            "consensus_peer_head_unavailable",
            "network head is unknown because no peer committed heads are visible".to_string(),
        );
    }
    if network_head.stale_peer_count > 0 {
        push_observability_alert(
            &mut alerts,
            "warn",
            "consensus_peer_head_stale",
            format!(
                "{} peer committed head(s) exceeded freshness ttl {}ms",
                network_head.stale_peer_count, network_head.freshness_ttl_ms
            ),
        );
    }
    if snapshot.replication_enabled
        && policy.quorum_mode != "stake_weighted"
        && network_head.fresh_peer_count > 0
        && network_head.fresh_peer_count < network_head.required_peer_count
    {
        push_observability_alert(
            &mut alerts,
            "warn",
            "consensus_peer_head_quorum_missing",
            format!(
                "fresh peer head quorum missing: observed={} required={}",
                network_head.fresh_peer_count, network_head.required_peer_count
            ),
        );
    }
    if network_head.conflicting_peer_count > 0 {
        push_observability_alert(
            &mut alerts,
            "critical",
            "consensus_peer_head_conflict",
            format!(
                "{} fresh peer head(s) conflict at the same height",
                network_head.conflicting_peer_count
            ),
        );
    }
    if policy.quorum_mode == "stake_weighted" && !network_head.stake_quorum_met {
        push_observability_alert(
            &mut alerts,
            "critical",
            "consensus_stake_quorum_missing",
            format!(
                "stake-weighted head quorum missing: observed_stake={} required_stake={} total_stake={}",
                network_head.observed_stake, network_head.required_stake, network_head.total_stake
            ),
        );
    }
    if policy.quorum_mode == "count_fallback_stake_unavailable" {
        push_observability_alert(
            &mut alerts,
            "critical",
            "consensus_stake_quorum_unavailable",
            "mainnet validator readiness requires stake-weighted peer head quorum, but validator stake snapshot is unavailable".to_string(),
        );
    }
    if !snapshot.consensus.misbehavior_evidence.is_empty() {
        push_observability_alert(
            &mut alerts,
            "critical",
            "consensus_misbehavior_evidence_present",
            format!(
                "consensus misbehavior evidence present: count={} quarantined_validators={} slashable_stake_total={}",
                snapshot.consensus.misbehavior_evidence.len(),
                snapshot.consensus.quarantined_validators.len(),
                slashable_stake_total
            ),
        );
    }
    if !snapshot.consensus.quarantined_validators.is_empty() {
        push_observability_alert(
            &mut alerts,
            "critical",
            "consensus_validator_quarantined",
            format!(
                "{} validator(s) are quarantined from peer-head quorum",
                snapshot.consensus.quarantined_validators.len()
            ),
        );
    }
    let applied_slashing_receipts = applied_slashing_receipt_hashes(snapshot);
    let applied_slashing_receipt_count = applied_slashing_receipts.len();
    let pending_slashing_intent_count = pending_slashing_intent_count(snapshot);
    if pending_slashing_intent_count > 0 {
        push_observability_alert(
            &mut alerts,
            "critical",
            "consensus_slashing_intent_pending",
            format!(
                "{pending_slashing_intent_count} consensus slashing intent(s) are pending governance identity penalty submission"
            ),
        );
    }
    if let Some(height) = snapshot.consensus.replication_gap_sync_blocked_height {
        let reason = snapshot
            .consensus
            .replication_gap_sync_blocked_reason
            .clone()
            .unwrap_or_else(|| format!("replication gap sync blocked at height {height}"));
        push_observability_alert(
            &mut alerts,
            "critical",
            "replication_gap_sync_blocked",
            reason,
        );
    }
    if snapshot.replication_enabled && replication_state_gap > 0 {
        push_observability_alert(
            &mut alerts,
            "critical",
            "consensus_replication_state_gap",
            format!(
                "consensus committed height {} is ahead of contiguous replication persisted height {} by {}",
                snapshot.consensus.committed_height,
                snapshot.consensus.replication_persisted_height,
                replication_state_gap
            ),
        );
    }
    if let Some(reason) = consensus_participation_hold_reason.as_ref() {
        push_observability_alert(
            &mut alerts,
            "critical",
            "consensus_participation_held",
            format!("local consensus participation is held until verified sync recovers: {reason}"),
        );
    }
    if let Some(reason) = state_sync_fallback_reason.as_ref() {
        let severity = if storage_metrics.checkpoint_count > 0 {
            "warn"
        } else {
            "critical"
        };
        push_observability_alert(
            &mut alerts,
            severity,
            "state_sync_fallback_required",
            reason.clone(),
        );
    }
    if suspect_peer_count > 0 || blocked_peer_count > 0 || peer_with_issues_count > 0 {
        push_observability_alert(
            &mut alerts,
            "warn",
            "replication_peer_health_degraded",
            format!(
                "peer health degraded: suspect={suspect_peer_count}, blocked={blocked_peer_count}, peers_with_issues={peer_with_issues_count}"
            ),
        );
    }
    if !replication.peer_healths.is_empty() && connected_peer_count == 0 {
        push_observability_alert(
            &mut alerts,
            "warn",
            "replication_no_connected_peers",
            "replication discovered peers but has no connected peers".to_string(),
        );
    }
    if recent_replication_error_count > 0 {
        push_observability_alert(
            &mut alerts,
            "warn",
            "replication_recent_errors",
            format!(
                "replication reported {recent_replication_error_count} recent transport/protocol errors"
            ),
        );
    }
    if !transport_stability.stable {
        push_observability_alert(
            &mut alerts,
            "warn",
            "replication_transport_unstable",
            format!(
                "replication transport stability score {} below threshold {}: connection_closed={}, insufficient_peers={}, timeout={}, protocol_errors={}",
                transport_stability.score,
                TRANSPORT_STABILITY_MIN_SCORE,
                transport_stability.connection_closed_count,
                transport_stability.insufficient_peers_count,
                transport_stability.timeout_count,
                transport_stability.protocol_error_count
            ),
        );
    }
    if !reachability_policy_ok {
        push_observability_alert(
            &mut alerts,
            "warn",
            "p2p_reachability_degraded",
            format!(
                "p2p reachability does not satisfy role policy: role={} detected={:?} autonat={} public_port={} relay_available={}",
                snapshot.role.as_str(),
                p2p.detected_reachability,
                p2p.autonat_status,
                p2p.public_port_reachability,
                p2p.relay_available
            ),
        );
    }
    if policy.tier == "mainnet" && policy.slashing_policy == "evidence_only_readiness_gate" {
        let evidence_count = snapshot.consensus.misbehavior_evidence.len();
        push_observability_alert(
            &mut alerts,
            "warn",
            "mainnet_slashing_evidence_only",
            format!(
                "mainnet slashing policy is evidence-only in readiness and does not execute protocol penalties; evidence_count={evidence_count}, slashable_stake_total={slashable_stake_total}"
            ),
        );
    }
    if storage_degraded {
        let reason = storage_metrics
            .degraded_reason
            .clone()
            .or_else(|| {
                (storage_metrics.last_gc_result == "failed")
                    .then(|| "latest GC result is failed".to_string())
            })
            .unwrap_or_else(|| "storage reported degraded state".to_string());
        push_observability_alert(
            &mut alerts,
            "warn",
            "storage_degraded",
            format!("storage degraded: {reason}"),
        );
    }
    if reward_runtime_degraded {
        let reason = reward_runtime_metrics
            .last_error
            .clone()
            .unwrap_or_else(|| {
                if !reward_runtime_metrics.metrics_available {
                    "reward runtime metrics unavailable".to_string()
                } else if !reward_runtime_metrics.invariant_ok {
                    "reward runtime invariant failed".to_string()
                } else {
                    "reward runtime degraded".to_string()
                }
            });
        push_observability_alert(
            &mut alerts,
            "warn",
            "reward_runtime_degraded",
            format!("reward runtime degraded: {reason}"),
        );
    }

    let status = observability_status_for_alerts(alerts.as_slice());
    let ready = status != "critical"
        && (!snapshot.replication_enabled || network_head_available)
        && network_head.decision == "ready"
        && transport_stability.stable
        && reachability_policy_ok
        && network_height_lag <= policy.max_network_height_lag
        && policy.quorum_mode != "count_fallback_stake_unavailable";
    ChainNodeObservabilityStatus {
        status,
        summary: observability_summary_for_alerts(alerts.as_slice()),
        ready,
        connected_peer_count,
        active_peer_count,
        candidate_peer_count,
        suspect_peer_count,
        blocked_peer_count,
        peer_with_issues_count,
        known_peer_heads,
        network_head_available,
        network_height_lag,
        transport_stable: transport_stability.stable,
        transport_stability_score: transport_stability.score,
        reachability_policy_ok,
        misbehavior_evidence_count: snapshot.consensus.misbehavior_evidence.len(),
        slashing_intent_count: snapshot.consensus.slashing_intents.len(),
        pending_slashing_intent_count,
        slashing_receipt_count: snapshot.consensus.slashing_receipts.len(),
        applied_slashing_receipt_count,
        quarantined_validator_count: snapshot.consensus.quarantined_validators.len(),
        slashable_stake_total,
        replication_enabled: snapshot.replication_enabled,
        replication_persisted_height: snapshot.consensus.replication_persisted_height,
        replication_state_gap,
        replication_gap_sync_blocked_height: snapshot.consensus.replication_gap_sync_blocked_height,
        recent_replication_error_count,
        storage_degraded,
        reward_runtime_degraded,
        alerts,
    }
}

pub(super) fn build_chain_status_payload(
    snapshot: NodeSnapshot,
    execution_world_dir: &Path,
    loaded_network_tier_manifest: Option<&LoadedNetworkTierManifest>,
    live_p2p_recommendation: &NodeUserModeRecommendation,
    applied_effective_user_mode: Option<String>,
    effective_p2p_policy: NodeNetworkPolicy,
    live_snapshot: &Libp2pReachabilitySnapshot,
    p2p_detection: NodeReachabilityAutoDetection,
    release_security_policy: ReleaseSecurityPolicy,
    reward_runtime_metrics: super::reward_runtime_worker::RewardRuntimeMetricsSnapshot,
    storage_metrics: storage_metrics::StorageMetricsSnapshot,
    wasm: ChainWasmStatus,
    traffic: ChainTrafficStatus,
    transactions: super::transfer_submit_api::ChainTransferMetricsStatus,
    replication: super::ChainReplicationDebugStatus,
) -> ChainStatusResponse {
    let observed_at_unix_ms = now_unix_ms();
    let p2p = build_chain_p2p_status(
        live_p2p_recommendation,
        applied_effective_user_mode,
        effective_p2p_policy,
        live_snapshot,
        p2p_detection,
    );
    let clamped_elapsed_ms = |prior_ms: i64| -> Option<i64> {
        observed_at_unix_ms
            .checked_sub(prior_ms)
            .map(|age_ms| age_ms.max(0))
    };
    let last_status = snapshot
        .consensus
        .last_status
        .map(consensus_status_to_string);
    let readiness_policy = readiness_policy(&snapshot, loaded_network_tier_manifest);
    let network_head =
        build_network_head_status(&snapshot, observed_at_unix_ms, loaded_network_tier_manifest);
    let observability = build_chain_node_observability_status(
        &snapshot,
        &storage_metrics,
        &reward_runtime_metrics,
        &replication,
        &network_head,
        &p2p,
        &readiness_policy,
        observed_at_unix_ms,
    );
    let consensus_participation_hold_reason = consensus_participation_hold_reason(
        &snapshot,
        observability.network_height_lag,
        observability.replication_state_gap,
        readiness_policy.max_network_height_lag,
    );
    let consensus_participation_held = consensus_participation_hold_reason.is_some();
    let state_sync_fallback_reason = state_sync_fallback_reason(
        &snapshot,
        observability.replication_state_gap,
        observability.network_height_lag,
    );
    let state_sync_trusted_checkpoint_required_height =
        state_sync_trusted_checkpoint_required_height(
            &snapshot,
            observability.replication_state_gap,
            observability.network_height_lag,
        );
    let state_sync_fallback_required = state_sync_fallback_reason.is_some();
    let state_sync_snapshot_available = storage_metrics.checkpoint_count > 0;
    let liveness = build_liveness_status(&snapshot);
    let readiness = build_readiness_status(&observability, readiness_policy.clone());
    let sync = build_sync_status(
        &network_head,
        observability.network_height_lag,
        &readiness_policy,
        &snapshot,
        observed_at_unix_ms,
    );
    let last_commit_age_ms = snapshot
        .consensus
        .last_committed_at_ms
        .and_then(clamped_elapsed_ms);
    let slashable_stake_total = snapshot
        .consensus
        .misbehavior_evidence
        .iter()
        .map(|evidence| evidence.slashable_stake)
        .sum::<u64>();
    let pending_slashing_intent_count = pending_slashing_intent_count(&snapshot);
    let applied_slashing_receipt_count = applied_slashing_receipt_hashes(&snapshot).len();
    let pending_proposal = snapshot
        .consensus
        .pending_proposal
        .as_ref()
        .map(|proposal| ChainPendingProposalStatus {
            height: proposal.height,
            slot: proposal.slot,
            epoch: proposal.epoch,
            proposer_id: proposal.proposer_id.clone(),
            opened_at_ms: proposal.opened_at_ms,
            age_ms: clamped_elapsed_ms(proposal.opened_at_ms).unwrap_or(0),
            action_count: proposal.action_count,
            action_payload_bytes: proposal.action_payload_bytes,
            attestation_count: proposal.attestation_count,
            approved_stake: proposal.approved_stake,
            rejected_stake: proposal.rejected_stake,
            required_stake: proposal.required_stake,
            total_stake: proposal.total_stake,
            approval_progress_bps: proposal.approval_progress_bps,
            rejection_progress_bps: proposal.rejection_progress_bps,
            remaining_approval_stake: proposal.remaining_approval_stake,
            status: consensus_status_to_string(proposal.status),
        });

    ChainStatusResponse {
        ok: observability.ready,
        observed_at_unix_ms,
        node_id: snapshot.node_id,
        world_id: snapshot.world_id,
        role: snapshot.role.as_str().to_string(),
        running: snapshot.running,
        liveness,
        readiness,
        sync,
        worker_poll_count: snapshot.tick_count,
        tick_count: snapshot.tick_count,
        last_tick_unix_ms: snapshot.last_tick_unix_ms,
        network_tier: loaded_network_tier_manifest.map(|loaded| ChainNetworkTierStatus {
            source_path: loaded.source_path.clone(),
            schema_version: loaded.manifest.schema_version.clone(),
            tier: loaded.manifest.tier.clone(),
            status: loaded.manifest.status.clone(),
            network_id: loaded.manifest.network_id.clone(),
            chain_id: loaded.manifest.chain_id.clone(),
            bootstrap_peer_count: loaded.bootstrap_peers.len(),
            governance_mode: loaded.manifest.validator_policy.governance_mode.clone(),
            validator_admission: loaded.manifest.validator_policy.validator_admission.clone(),
            target_validator_count: loaded.manifest.validator_policy.target_validator_count,
            allow_observer_nodes: loaded.manifest.validator_policy.allow_observer_nodes,
            token_symbol: loaded.manifest.token_policy.symbol.clone(),
            faucet_mode: loaded.manifest.token_policy.faucet_mode.clone(),
            reset_policy: loaded.manifest.token_policy.reset_policy.clone(),
            value_semantics: loaded.manifest.token_policy.value_semantics.clone(),
            rpc_ref: loaded.manifest.endpoint_policy.rpc_ref.clone(),
            explorer_ref: loaded.manifest.endpoint_policy.explorer_ref.clone(),
            faucet_ref: loaded.manifest.endpoint_policy.faucet_ref.clone(),
            required_gates: loaded.manifest.promotion_policy.required_gates.clone(),
            allowed_claims: loaded.manifest.claims_policy.allowed_claims.clone(),
            denied_claims: loaded.manifest.claims_policy.denied_claims.clone(),
        }),
        consensus: ChainConsensusStatus {
            slot: snapshot.consensus.slot,
            epoch: snapshot.consensus.epoch,
            ticks_per_slot: snapshot.consensus.ticks_per_slot,
            tick_phase: snapshot.consensus.tick_phase,
            proposal_tick_phase: snapshot.consensus.proposal_tick_phase,
            last_observed_slot: snapshot.consensus.last_observed_slot,
            missed_slot_count: snapshot.consensus.missed_slot_count,
            last_observed_tick: snapshot.consensus.last_observed_tick,
            missed_tick_count: snapshot.consensus.missed_tick_count,
            adaptive_tick_scheduler_enabled: snapshot.consensus.adaptive_tick_scheduler_enabled,
            latest_height: snapshot.consensus.latest_height,
            committed_height: snapshot.consensus.committed_height,
            last_committed_at_ms: snapshot.consensus.last_committed_at_ms,
            last_commit_age_ms,
            network_committed_height: snapshot.consensus.network_committed_height,
            replication_enabled: snapshot.replication_enabled,
            replication_persisted_height: snapshot.consensus.replication_persisted_height,
            replication_gap_sync_blocked_height: snapshot
                .consensus
                .replication_gap_sync_blocked_height,
            replication_gap_sync_blocked_reason: snapshot
                .consensus
                .replication_gap_sync_blocked_reason
                .clone(),
            replication_gap_sync_repair_attempt_height: snapshot
                .consensus
                .replication_gap_sync_repair_attempt_height,
            replication_gap_sync_repair_attempt_summary: snapshot
                .consensus
                .replication_gap_sync_repair_attempt_summary
                .clone(),
            state_sync_fallback_required,
            state_sync_snapshot_available,
            state_sync_trusted_checkpoint_required_height,
            state_sync_fallback_reason,
            consensus_participation_held,
            consensus_participation_hold_reason,
            recent_finality_latency: ChainFinalityLatencyStatus {
                sample_count: snapshot.consensus.recent_finality_latency.sample_count,
                avg_latency_ms: snapshot.consensus.recent_finality_latency.avg_latency_ms,
                max_latency_ms: snapshot.consensus.recent_finality_latency.max_latency_ms,
                p50_latency_ms: snapshot.consensus.recent_finality_latency.p50_latency_ms,
                p95_latency_ms: snapshot.consensus.recent_finality_latency.p95_latency_ms,
            },
            pending_proposal,
            pending_consensus_actions: ChainPendingConsensusActionsStatus {
                queued_action_count: snapshot
                    .consensus
                    .pending_consensus_actions
                    .queued_action_count,
                queued_payload_bytes: snapshot
                    .consensus
                    .pending_consensus_actions
                    .queued_payload_bytes,
                reserved_requeue_action_count: snapshot
                    .consensus
                    .pending_consensus_actions
                    .reserved_requeue_action_count,
                reserved_requeue_payload_bytes: snapshot
                    .consensus
                    .pending_consensus_actions
                    .reserved_requeue_payload_bytes,
                available_capacity: snapshot
                    .consensus
                    .pending_consensus_actions
                    .available_capacity,
                max_capacity: snapshot.consensus.pending_consensus_actions.max_capacity,
                submit_buffer_action_count: snapshot
                    .consensus
                    .pending_consensus_actions
                    .submit_buffer_action_count,
                submit_buffer_payload_bytes: snapshot
                    .consensus
                    .pending_consensus_actions
                    .submit_buffer_payload_bytes,
                submit_buffer_max_capacity: snapshot
                    .consensus
                    .pending_consensus_actions
                    .submit_buffer_max_capacity,
            },
            inbound_timing_rejections: ChainInboundTimingRejectionsStatus {
                proposal_future_slot: snapshot.consensus.inbound_rejected_proposal_future_slot,
                proposal_stale_slot: snapshot.consensus.inbound_rejected_proposal_stale_slot,
                attestation_future_slot: snapshot
                    .consensus
                    .inbound_rejected_attestation_future_slot,
                attestation_stale_slot: snapshot.consensus.inbound_rejected_attestation_stale_slot,
                attestation_epoch_mismatch: snapshot
                    .consensus
                    .inbound_rejected_attestation_epoch_mismatch,
                last_reason: snapshot.consensus.last_inbound_timing_reject_reason.clone(),
            },
            last_status,
            last_block_hash: snapshot.consensus.last_block_hash,
            last_execution_height: snapshot.consensus.last_execution_height,
            last_execution_block_hash: snapshot.consensus.last_execution_block_hash,
            last_execution_state_root: snapshot.consensus.last_execution_state_root,
            known_peer_heads: snapshot.consensus.known_peer_heads,
            validator_set_hash: snapshot.consensus.validator_set_hash.clone(),
            validator_stake_root: snapshot.consensus.validator_stake_root.clone(),
            validator_stake_proof_count: snapshot.consensus.validator_stake_proofs.len(),
            misbehavior_evidence_count: snapshot.consensus.misbehavior_evidence.len(),
            slashing_intent_count: snapshot.consensus.slashing_intents.len(),
            pending_slashing_intent_count,
            slashing_receipt_count: snapshot.consensus.slashing_receipts.len(),
            applied_slashing_receipt_count,
            quarantined_validator_count: snapshot.consensus.quarantined_validators.len(),
            slashable_stake_total,
            network_head,
        },
        last_error: snapshot.last_error,
        execution_world_dir: execution_world_dir.display().to_string(),
        p2p,
        observability,
        release_security_policy,
        reward_runtime: reward_runtime_metrics,
        storage: storage_metrics,
        wasm,
        traffic,
        transactions,
        replication,
    }
}
