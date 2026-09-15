use super::super::ChainReplicationDebugStatus;
use super::super::execution_bridge::ExecutionBridgeCommitTimingSnapshot;
use super::super::runtime_authority::RuntimeAuthorityBinding;
use super::super::traffic_status::ChainTrafficStatus;
use super::super::wasm_status::ChainWasmStatus;
use super::ChainNodeObservabilityStatus;
use super::status_payload_chain_proof::ChainProofStatus;
use super::status_payload_consensus::ChainConsensusStatus;
use super::status_payload_lifecycle::{ChainLivenessStatus, ChainReadinessStatus, ChainSyncStatus};
use super::status_payload_module_tick_routing::ChainModuleTickRoutingStatus;
use super::status_payload_network_tier::ChainNetworkTierStatus;
use super::status_payload_p2p::ChainP2pStatus;
use super::status_payload_world_resource::ChainWorldResourceStatus;
use super::storage_metrics;
use oasis7::runtime::ReleaseSecurityPolicy;
use oasis7::simulator::RuntimePerfSnapshot;
use oasis7_node::{NodeRole, NodeSnapshot, NodeValidatorStakeProofSnapshot};
use serde::Serialize;

const TRIAD_STATUS_PROJECTION_SCHEMA: &str = "oasis7.chain_validator_provider_status.v1";

pub(crate) fn build_chain_validator_status(
    snapshot: &NodeSnapshot,
    authority_binding: Option<&RuntimeAuthorityBinding>,
) -> ChainValidatorStatus {
    let local_validator = snapshot
        .consensus
        .validator_stake_proofs
        .iter()
        .find(|proof| proof.validator_id == snapshot.node_id);
    let stake = snapshot
        .consensus
        .validator_stakes
        .get(snapshot.node_id.as_str())
        .copied();
    let membership_active = local_validator.is_some()
        && stake.is_some_and(|stake| stake > 0)
        && !snapshot
            .consensus
            .quarantined_validators
            .iter()
            .any(|validator| validator == &snapshot.node_id)
        && !snapshot.consensus.validator_set_hash.is_empty()
        && !snapshot.consensus.validator_stake_root.is_empty();
    ChainValidatorStatus {
        schema_version: TRIAD_STATUS_PROJECTION_SCHEMA.to_string(),
        role: if membership_active {
            "validator".to_string()
        } else {
            "non_validator".to_string()
        },
        membership: if membership_active {
            "active".to_string()
        } else {
            "inactive".to_string()
        },
        stake,
        signer_binding: local_validator.map(|proof| proof.validator_id.clone()),
        signer_public_key_hex: local_validator
            .and_then(|proof| proof.signer_public_key_hex.clone()),
        stake_proof: local_validator.cloned(),
        validator_set_hash: snapshot.consensus.validator_set_hash.clone(),
        stake_root: snapshot.consensus.validator_stake_root.clone(),
        registry_ref: authority_binding.map(|binding| binding.registry_ref.clone()),
        registry_sha256: authority_binding.map(|binding| binding.registry_sha256.clone()),
        inventory_ref: authority_binding.map(|binding| binding.inventory_ref.clone()),
        inventory_sha256: authority_binding.map(|binding| binding.inventory_sha256.clone()),
    }
}

pub(crate) fn build_chain_provider_status(
    snapshot: &NodeSnapshot,
    world_resource: &ChainWorldResourceStatus,
    chain_proof: &ChainProofStatus,
    storage_metrics: &storage_metrics::StorageMetricsSnapshot,
    replication: &ChainReplicationDebugStatus,
) -> ChainProviderStatus {
    let provider_id =
        (!replication.local_peer_id.trim().is_empty()).then(|| replication.local_peer_id.clone());
    let storage_runtime_healthy = matches!(snapshot.role, NodeRole::Storage)
        && snapshot.replication_enabled
        && provider_id.is_some()
        && storage_metrics.degraded_reason.is_none()
        && world_resource.failed_gates.is_empty()
        && storage_metrics.checkpoint_count > 0
        && storage_metrics
            .replay_summary
            .latest_checkpoint_height
            .is_some();
    let checkpoint = chain_proof
        .latest_execution_checkpoint
        .as_ref()
        .zip(chain_proof.latest_world_head_proof.as_ref())
        .is_some_and(|(checkpoint, proof)| {
            storage_runtime_healthy
                && chain_proof.status == "available"
                && checkpoint.schema_version >= 2
                && checkpoint.height > 0
                && storage_metrics.replay_summary.latest_checkpoint_height
                    == Some(checkpoint.height)
                && checkpoint.height == proof.height
                && !checkpoint.checkpoint_id.trim().is_empty()
                && !checkpoint.manifest_hash.trim().is_empty()
                && !proof.proof_hash.trim().is_empty()
                && proof.world_id == snapshot.world_id
                && proof.world_id == world_resource.world_id
                && !world_resource.chain_id.trim().is_empty()
                && proof
                    .checkpoint_ref
                    .as_deref()
                    .is_some_and(|value| !value.is_empty())
        });
    let (checkpoint_proof, full_storage_proof) = if checkpoint {
        let checkpoint = chain_proof
            .latest_execution_checkpoint
            .as_ref()
            .expect("checkpoint proof checked above");
        let proof = chain_proof
            .latest_world_head_proof
            .as_ref()
            .expect("world head proof checked above");
        let provider_id = provider_id.clone().expect("provider id checked above");
        let checkpoint_proof = ChainProviderCheckpointProof {
            schema_version: checkpoint.schema_version,
            checkpoint_id: checkpoint.checkpoint_id.clone(),
            height: checkpoint.height,
            manifest_hash: checkpoint.manifest_hash.clone(),
            proof_hash: proof.proof_hash.clone(),
            world_id: proof.world_id.clone(),
            chain_id: world_resource.chain_id.clone(),
        };
        let full_storage_proof = ChainProviderFullStorageProof {
            status: "ready".to_string(),
            provider_id,
            world_id: proof.world_id.clone(),
            chain_id: world_resource.chain_id.clone(),
            manifest_hash: checkpoint.manifest_hash.clone(),
            height: checkpoint.height,
        };
        (Some(checkpoint_proof), Some(full_storage_proof))
    } else {
        (None, None)
    };
    ChainProviderStatus {
        schema_version: TRIAD_STATUS_PROJECTION_SCHEMA.to_string(),
        node_id: snapshot.node_id.clone(),
        provider_id,
        checkpoint,
        // Full-storage readiness is only claimable once the local runtime has
        // an actual retained checkpoint that is bound to the world-head proof.
        full_storage: checkpoint,
        checkpoint_proof,
        full_storage_proof,
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct ChainStatusResponse {
    pub(crate) ok: bool,
    pub(crate) observed_at_unix_ms: i64,
    pub(crate) node_id: String,
    pub(crate) world_id: String,
    pub(crate) role: String,
    pub(crate) running: bool,
    pub(crate) liveness: ChainLivenessStatus,
    pub(crate) readiness: ChainReadinessStatus,
    pub(crate) sync: ChainSyncStatus,
    pub(crate) worker_poll_count: u64,
    pub(crate) tick_count: u64,
    pub(crate) last_tick_unix_ms: Option<i64>,
    pub(crate) consensus: ChainConsensusStatus,
    pub(crate) chain_proof: ChainProofStatus,
    pub(crate) consensus_progress_observer_error: Option<String>,
    pub(crate) last_error: Option<String>,
    pub(crate) execution_world_dir: String,
    pub(crate) network_tier: Option<ChainNetworkTierStatus>,
    pub(crate) world_resource: ChainWorldResourceStatus,
    pub(crate) p2p: ChainP2pStatus,
    pub(crate) observability: ChainNodeObservabilityStatus,
    pub(crate) release_security_policy: ReleaseSecurityPolicy,
    pub(crate) reward_runtime: super::super::reward_runtime_worker::RewardRuntimeMetricsSnapshot,
    pub(crate) storage: storage_metrics::StorageMetricsSnapshot,
    pub(crate) wasm: ChainWasmStatus,
    pub(crate) runtime_perf: Option<RuntimePerfSnapshot>,
    pub(crate) traffic: ChainTrafficStatus,
    pub(crate) transactions: super::super::transfer_submit_api::ChainTransferMetricsStatus,
    pub(crate) replication: ChainReplicationDebugStatus,
    pub(crate) execution_bridge_commit_timing: ExecutionBridgeCommitTimingSnapshot,
    pub(crate) module_tick_routing: ChainModuleTickRoutingStatus,
    /// Effective local validator admission data. This is derived only from
    /// the node's consensus snapshot; collectors must not infer it when the
    /// runtime cannot provide it.
    pub(crate) validator: ChainValidatorStatus,
    /// Effective local provider data. Provider readiness requires a real
    /// local peer id, storage capability, and an authenticated checkpoint.
    pub(crate) provider: ChainProviderStatus,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChainValidatorStatus {
    pub(crate) schema_version: String,
    pub(crate) role: String,
    pub(crate) membership: String,
    pub(crate) stake: Option<u64>,
    pub(crate) signer_binding: Option<String>,
    pub(crate) signer_public_key_hex: Option<String>,
    pub(crate) stake_proof: Option<NodeValidatorStakeProofSnapshot>,
    pub(crate) validator_set_hash: String,
    pub(crate) stake_root: String,
    /// These bindings remain optional until deployment supplies immutable
    /// runtime paths/digests. Fleet health treats absence as a hard failure.
    pub(crate) registry_ref: Option<String>,
    pub(crate) registry_sha256: Option<String>,
    pub(crate) inventory_ref: Option<String>,
    pub(crate) inventory_sha256: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChainProviderStatus {
    pub(crate) schema_version: String,
    pub(crate) node_id: String,
    pub(crate) provider_id: Option<String>,
    pub(crate) checkpoint: bool,
    pub(crate) full_storage: bool,
    pub(crate) checkpoint_proof: Option<ChainProviderCheckpointProof>,
    pub(crate) full_storage_proof: Option<ChainProviderFullStorageProof>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChainProviderCheckpointProof {
    pub(crate) schema_version: u32,
    pub(crate) checkpoint_id: String,
    pub(crate) height: u64,
    pub(crate) manifest_hash: String,
    pub(crate) proof_hash: String,
    pub(crate) world_id: String,
    pub(crate) chain_id: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChainProviderFullStorageProof {
    pub(crate) status: String,
    pub(crate) provider_id: String,
    pub(crate) world_id: String,
    pub(crate) chain_id: String,
    pub(crate) manifest_hash: String,
    pub(crate) height: u64,
}
