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
    pub(super) release_security_policy: ReleaseSecurityPolicy,
    pub(super) reward_runtime: super::reward_runtime_worker::RewardRuntimeMetricsSnapshot,
    pub(super) storage: storage_metrics::StorageMetricsSnapshot,
    pub(super) traffic: ChainTrafficStatus,
    pub(super) replication: super::ChainReplicationDebugStatus,
}

#[derive(Debug, Serialize)]
pub(super) struct ChainConsensusStatus {
    slot: u64,
    epoch: u64,
    ticks_per_slot: u64,
    tick_phase: u64,
    proposal_tick_phase: u64,
    last_observed_slot: u64,
    missed_slot_count: u64,
    last_observed_tick: u64,
    missed_tick_count: u64,
    adaptive_tick_scheduler_enabled: bool,
    latest_height: u64,
    committed_height: u64,
    network_committed_height: u64,
    last_status: Option<String>,
    last_block_hash: Option<String>,
    last_execution_height: u64,
    last_execution_block_hash: Option<String>,
    last_execution_state_root: Option<String>,
    known_peer_heads: usize,
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
    traffic: ChainTrafficStatus,
) -> ChainStatusResponse {
    let last_status = snapshot
        .consensus
        .last_status
        .map(consensus_status_to_string);

    ChainStatusResponse {
        ok: true,
        observed_at_unix_ms: now_unix_ms(),
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
            network_committed_height: snapshot.consensus.network_committed_height,
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
        release_security_policy,
        reward_runtime: reward_runtime_metrics,
        storage: storage_metrics,
        traffic,
        replication: super::ChainReplicationDebugStatus::default(),
    }
}
