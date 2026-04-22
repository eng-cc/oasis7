use std::path::Path;

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
    pub(super) worker_poll_count: u64,
    pub(super) tick_count: u64,
    pub(super) last_tick_unix_ms: Option<i64>,
    pub(super) consensus: ChainConsensusStatus,
    pub(super) last_error: Option<String>,
    pub(super) execution_world_dir: String,
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
pub(super) struct ChainNodeObservabilityStatus {
    pub(super) status: String,
    pub(super) summary: String,
    pub(super) connected_peer_count: usize,
    pub(super) active_peer_count: usize,
    pub(super) candidate_peer_count: usize,
    pub(super) suspect_peer_count: usize,
    pub(super) blocked_peer_count: usize,
    pub(super) peer_with_issues_count: usize,
    pub(super) known_peer_heads: usize,
    pub(super) network_height_lag: u64,
    pub(super) recent_replication_error_count: usize,
    pub(super) storage_degraded: bool,
    pub(super) reward_runtime_degraded: bool,
    pub(super) alerts: Vec<ChainNodeObservabilityAlert>,
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

fn build_chain_node_observability_status(
    snapshot: &NodeSnapshot,
    storage_metrics: &storage_metrics::StorageMetricsSnapshot,
    reward_runtime_metrics: &super::reward_runtime_worker::RewardRuntimeMetricsSnapshot,
    replication: &super::ChainReplicationDebugStatus,
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
    let network_height_lag = snapshot
        .consensus
        .network_committed_height
        .saturating_sub(snapshot.consensus.committed_height);
    let recent_replication_error_count = replication.recent_errors.len();
    let storage_degraded = storage_metrics.degraded_reason.is_some()
        || matches!(storage_metrics.last_gc_result.as_str(), "failed");
    let reward_runtime_degraded = reward_runtime_metrics.enabled
        && (!reward_runtime_metrics.metrics_available
            || !reward_runtime_metrics.invariant_ok
            || reward_runtime_metrics.last_error.is_some());

    let mut alerts = Vec::new();
    if let Some(err) = snapshot.last_error.as_ref() {
        push_observability_alert(
            &mut alerts,
            "critical",
            "runtime_last_error",
            format!("runtime last_error is set: {err}"),
        );
    }
    if network_height_lag > 0 {
        push_observability_alert(
            &mut alerts,
            "warn",
            "consensus_network_lag",
            format!("network committed height is ahead by {network_height_lag}"),
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

    ChainNodeObservabilityStatus {
        status: observability_status_for_alerts(alerts.as_slice()),
        summary: observability_summary_for_alerts(alerts.as_slice()),
        connected_peer_count,
        active_peer_count,
        candidate_peer_count,
        suspect_peer_count,
        blocked_peer_count,
        peer_with_issues_count,
        known_peer_heads,
        network_height_lag,
        recent_replication_error_count,
        storage_degraded,
        reward_runtime_degraded,
        alerts,
    }
}

pub(super) fn build_chain_status_payload(
    snapshot: NodeSnapshot,
    execution_world_dir: &Path,
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
    let last_status = snapshot
        .consensus
        .last_status
        .map(consensus_status_to_string);
    let observability = build_chain_node_observability_status(
        &snapshot,
        &storage_metrics,
        &reward_runtime_metrics,
        &replication,
    );
    let last_commit_age_ms = snapshot
        .consensus
        .last_committed_at_ms
        .map(|committed_at_ms| observed_at_unix_ms.saturating_sub(committed_at_ms));
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
            age_ms: observed_at_unix_ms.saturating_sub(proposal.opened_at_ms),
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
        ok: true,
        observed_at_unix_ms,
        node_id: snapshot.node_id,
        world_id: snapshot.world_id,
        role: snapshot.role.as_str().to_string(),
        running: snapshot.running,
        worker_poll_count: snapshot.tick_count,
        tick_count: snapshot.tick_count,
        last_tick_unix_ms: snapshot.last_tick_unix_ms,
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
        },
        last_error: snapshot.last_error,
        execution_world_dir: execution_world_dir.display().to_string(),
        p2p: build_chain_p2p_status(
            live_p2p_recommendation,
            applied_effective_user_mode,
            effective_p2p_policy,
            live_snapshot,
            p2p_detection,
        ),
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
