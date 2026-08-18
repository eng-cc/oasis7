use oasis7::network_tier_manifest::LoadedNetworkTierManifest;
use oasis7_node::{NodeSnapshot, ReplicationNetworkDebugSnapshot};
use serde::Serialize;

const MAX_CONNECTED_PEER_IDS: usize = 64;
const MAX_ERROR_BYTES: usize = 512;

#[derive(Debug, Serialize)]
pub(super) struct RebuildStatusResponse {
    pub(super) schema_version: &'static str,
    pub(super) ok: bool,
    pub(super) liveness: RebuildLiveness,
    pub(super) readiness: RebuildReadiness,
    pub(super) heights: RebuildHeights,
    pub(super) network_head: RebuildNetworkHead,
    pub(super) checkpoint: Option<RebuildCheckpoint>,
    pub(super) local_peer_id: String,
    pub(super) connected_peers: Vec<String>,
    pub(super) connected_peer_count: usize,
}

#[derive(Debug, Serialize)]
pub(super) struct RebuildLiveness {
    pub(super) running: bool,
    pub(super) last_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct RebuildReadiness {
    pub(super) status: &'static str,
    pub(super) failed_gates: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct RebuildHeights {
    pub(super) committed_height: u64,
    pub(super) network_committed_height: u64,
    pub(super) last_execution_height: u64,
}

#[derive(Debug, Serialize)]
pub(super) struct RebuildNetworkHead {
    pub(super) source: String,
    pub(super) decision: String,
    pub(super) height: Option<u64>,
    pub(super) block_hash: Option<String>,
    pub(super) execution_block_hash: Option<String>,
    pub(super) execution_state_root: Option<String>,
    pub(super) observed_peer_count: usize,
    pub(super) fresh_peer_count: usize,
}

#[derive(Debug, Serialize)]
pub(super) struct RebuildCheckpoint {
    pub(super) schema_version: u32,
    pub(super) checkpoint_id: String,
    pub(super) height: u64,
    pub(super) manifest_hash: String,
}

pub(super) fn build_rebuild_status(
    snapshot: NodeSnapshot,
    network: ReplicationNetworkDebugSnapshot,
    checkpoint: Option<(u32, String, u64, String)>,
    observed_at_unix_ms: i64,
) -> RebuildStatusResponse {
    build_rebuild_status_with_manifest(snapshot, network, checkpoint, observed_at_unix_ms, None)
}

pub(super) fn build_rebuild_status_with_manifest(
    snapshot: NodeSnapshot,
    network: ReplicationNetworkDebugSnapshot,
    checkpoint: Option<(u32, String, u64, String)>,
    observed_at_unix_ms: i64,
    manifest: Option<&LoadedNetworkTierManifest>,
) -> RebuildStatusResponse {
    let network_head =
        super::status_payload::build_network_head_status(&snapshot, observed_at_unix_ms, manifest);
    let mut failed_gates = Vec::new();
    if !snapshot.running {
        failed_gates.push("runtime_not_running".to_string());
    }
    if snapshot.last_error.is_some() {
        failed_gates.push("runtime_last_error".to_string());
    }
    if snapshot.replication_enabled && network_head.decision != "ready" {
        failed_gates.push("network_head_not_ready".to_string());
    }
    if snapshot.consensus.committed_height == 0 {
        failed_gates.push("committed_height_zero".to_string());
    }
    if snapshot.consensus.last_execution_height == 0 {
        failed_gates.push("execution_height_zero".to_string());
    }
    if checkpoint.is_none() {
        failed_gates.push("checkpoint_unavailable".to_string());
    }
    let readiness_status = if failed_gates.is_empty() {
        "ready"
    } else {
        "not_ready"
    };
    let connected_peer_count = network.connected_peers.len();
    let connected_peers = network
        .connected_peers
        .into_iter()
        .take(MAX_CONNECTED_PEER_IDS)
        .collect();
    let last_error = snapshot
        .last_error
        .map(|error| error.chars().take(MAX_ERROR_BYTES).collect::<String>());
    RebuildStatusResponse {
        schema_version: "oasis7.rebuild_status.v1",
        ok: readiness_status == "ready",
        liveness: RebuildLiveness {
            running: snapshot.running,
            last_error,
        },
        readiness: RebuildReadiness {
            status: readiness_status,
            failed_gates,
        },
        heights: RebuildHeights {
            committed_height: snapshot.consensus.committed_height,
            network_committed_height: snapshot.consensus.network_committed_height,
            last_execution_height: snapshot.consensus.last_execution_height,
        },
        network_head: RebuildNetworkHead {
            source: network_head.source,
            decision: network_head.decision,
            height: network_head.height,
            block_hash: network_head.block_hash,
            execution_block_hash: network_head.execution_block_hash,
            execution_state_root: network_head.execution_state_root,
            observed_peer_count: network_head.observed_peer_count,
            fresh_peer_count: network_head.fresh_peer_count,
        },
        checkpoint: checkpoint.map(|(schema_version, checkpoint_id, height, manifest_hash)| {
            RebuildCheckpoint {
                schema_version,
                checkpoint_id,
                height,
                manifest_hash,
            }
        }),
        local_peer_id: network.local_peer_id,
        connected_peers,
        connected_peer_count,
    }
}
