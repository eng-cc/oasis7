use oasis7::network_tier_manifest::LoadedNetworkTierManifest;
use oasis7::runtime::Snapshot as RuntimeSnapshot;
use oasis7::runtime::{
    CHAIN_RESOURCE_DELTA_SCHEMA_V1, CHAIN_RESOURCE_MANIFEST_SCHEMA_V1, CHUNK_GENERATION_SCHEMA_V1,
    ChainChunkResourceStatus, ChainResourceDelta, ChainResourceDeltaEntry, ChainResourceManifest,
};
use oasis7::simulator::WorldSnapshot as SimulatorSnapshot;
use oasis7_node::NodeSnapshot;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Serialize)]
pub(crate) struct ChainWorldResourceStatus {
    pub(crate) schema_version: String,
    pub(crate) delta_schema_version: String,
    pub(crate) world_id: String,
    pub(crate) chain_id: String,
    pub(crate) world_seed: u64,
    pub(crate) chunk_generation_schema_version: String,
    pub(crate) seed_manifest_hash: String,
    pub(crate) starter_chunk_manifest_hash: Option<String>,
    pub(crate) latest_resource_commit_height: u64,
    pub(crate) latest_resource_commit_hash: Option<String>,
    pub(crate) committed_chunk_count: u64,
    pub(crate) provisional_chunk_count: u64,
    pub(crate) pending_delta_count: u64,
    pub(crate) last_delta_id: Option<String>,
    pub(crate) last_delta_commit_height: Option<u64>,
    pub(crate) readiness_status: String,
    pub(crate) failed_gates: Vec<String>,
}

pub(super) fn build_world_resource_status(
    snapshot: &NodeSnapshot,
    execution_world_dir: &Path,
    loaded_network_tier_manifest: Option<&LoadedNetworkTierManifest>,
) -> ChainWorldResourceStatus {
    let chain_id = loaded_network_tier_manifest
        .map(|loaded| loaded.manifest.chain_id.clone())
        .unwrap_or_else(|| snapshot.world_id.clone());
    let (manifest, latest_delta, snapshot_load_error) = load_latest_world_resource_snapshot(
        execution_world_dir,
        snapshot.world_id.as_str(),
        chain_id.as_str(),
    );
    let mut failed_gates = Vec::new();
    if snapshot.world_id.trim().is_empty() {
        failed_gates.push("world_resource_world_id_missing".to_string());
    }
    if chain_id.trim().is_empty() {
        failed_gates.push("world_resource_chain_id_missing".to_string());
    }
    if let Some(error) = snapshot_load_error {
        failed_gates.push(format!("world_resource_snapshot_unavailable:{error}"));
    }
    let seed_manifest_hash = manifest
        .as_ref()
        .map(|manifest| manifest.manifest_hash.clone())
        .unwrap_or_default();
    let world_seed = manifest
        .as_ref()
        .map(|manifest| manifest.world_seed)
        .unwrap_or(0);
    if world_seed == 0 {
        failed_gates.push("world_resource_seed_missing".to_string());
    }
    if manifest
        .as_ref()
        .is_none_or(|manifest| manifest.world_id != snapshot.world_id)
    {
        failed_gates.push("world_resource_world_id_mismatch".to_string());
    }
    if manifest
        .as_ref()
        .is_none_or(|manifest| manifest.chain_id != chain_id)
    {
        failed_gates.push("world_resource_chain_id_mismatch".to_string());
    }
    if manifest
        .as_ref()
        .is_none_or(|manifest| !manifest.is_schema_current())
    {
        failed_gates.push("world_resource_manifest_hash_mismatch".to_string());
    }
    let starter_chunk_manifest_hash = starter_chunk_manifest_hash(manifest.as_ref());
    if starter_chunk_manifest_hash.is_none() {
        failed_gates.push("world_resource_starter_chunk_missing".to_string());
    }
    let committed_chunk_count = manifest
        .as_ref()
        .map(committed_chunk_count)
        .unwrap_or_default();
    let provisional_chunk_count = manifest
        .as_ref()
        .map(provisional_chunk_count)
        .unwrap_or_default();
    let pending_delta_count = latest_delta
        .as_ref()
        .map(|delta| u64::from(delta.entries.is_empty() || !delta.is_schema_current()))
        .unwrap_or(1);
    if committed_chunk_count == 0 {
        failed_gates.push("world_resource_committed_chunk_missing".to_string());
    }
    if provisional_chunk_count > 0 {
        failed_gates.push("world_resource_provisional_chunk_present".to_string());
    }
    if pending_delta_count > 0 {
        failed_gates.push("world_resource_pending_delta_present".to_string());
    }
    if latest_delta
        .as_ref()
        .is_some_and(|delta| delta.resulting_manifest_hash != seed_manifest_hash)
    {
        failed_gates.push("world_resource_delta_manifest_mismatch".to_string());
    }
    if latest_delta
        .as_ref()
        .is_none_or(|delta| delta.commit_block_hash.as_deref().is_none_or(str::is_empty))
    {
        failed_gates.push("world_resource_delta_commit_hash_missing".to_string());
    }
    if latest_delta.as_ref().is_some_and(|delta| {
        delta.block_height != snapshot.consensus.committed_height
            || delta.ordering_key.height != snapshot.consensus.committed_height
    }) {
        failed_gates.push("world_resource_delta_height_mismatch".to_string());
    }
    if latest_delta
        .as_ref()
        .is_none_or(|delta| !delta_has_nonzero_resource_effect(delta))
    {
        failed_gates.push("world_resource_delta_zero_effect".to_string());
    }
    ChainWorldResourceStatus {
        schema_version: CHAIN_RESOURCE_MANIFEST_SCHEMA_V1.to_string(),
        delta_schema_version: CHAIN_RESOURCE_DELTA_SCHEMA_V1.to_string(),
        world_id: snapshot.world_id.clone(),
        chain_id,
        world_seed,
        chunk_generation_schema_version: CHUNK_GENERATION_SCHEMA_V1.to_string(),
        seed_manifest_hash,
        starter_chunk_manifest_hash,
        latest_resource_commit_height: snapshot.consensus.committed_height,
        latest_resource_commit_hash: snapshot.consensus.last_block_hash.clone(),
        committed_chunk_count,
        provisional_chunk_count,
        pending_delta_count,
        last_delta_id: latest_delta.as_ref().map(|delta| delta.delta_id.clone()),
        last_delta_commit_height: latest_delta.as_ref().map(|delta| delta.block_height),
        readiness_status: if failed_gates.is_empty() {
            "ready".to_string()
        } else {
            "not_ready".to_string()
        },
        failed_gates,
    }
}

fn load_latest_world_resource_snapshot(
    execution_world_dir: &Path,
    world_id: &str,
    chain_id: &str,
) -> (
    Option<ChainResourceManifest>,
    Option<ChainResourceDelta>,
    Option<String>,
) {
    let simulator_snapshot_path =
        simulator_world_dir_from_execution_world_dir(execution_world_dir).join("snapshot.json");
    let mut load_errors = Vec::new();
    if simulator_snapshot_path.exists() {
        match SimulatorSnapshot::load_json(simulator_snapshot_path.as_path()) {
            Ok(snapshot) if !snapshot.chain_resource_manifest.generated_chunks.is_empty() => {
                return validate_world_resource_snapshot(
                    snapshot.chain_resource_manifest,
                    Some(snapshot.latest_chain_resource_delta),
                    world_id,
                    chain_id,
                );
            }
            Ok(_) => load_errors.push("simulator_snapshot_empty_resource_manifest".to_string()),
            Err(err) => load_errors.push(format!("simulator_snapshot:{err:?}")),
        }
    } else {
        load_errors.push("simulator_snapshot_missing".to_string());
    }

    let runtime_snapshot_path = execution_world_dir.join("snapshot.json");
    if runtime_snapshot_path.exists() {
        match RuntimeSnapshot::load_json(runtime_snapshot_path.as_path()) {
            Ok(snapshot) if !snapshot.chain_resource_manifest.generated_chunks.is_empty() => {
                return validate_world_resource_snapshot(
                    snapshot.chain_resource_manifest,
                    snapshot.latest_chain_resource_delta,
                    world_id,
                    chain_id,
                );
            }
            Ok(_) => load_errors.push("runtime_snapshot_empty_resource_manifest".to_string()),
            Err(err) => load_errors.push(format!("runtime_snapshot:{err:?}")),
        }
    } else {
        load_errors.push("runtime_snapshot_missing".to_string());
    }

    (None, None, Some(load_errors.join(",")))
}

fn validate_world_resource_snapshot(
    manifest: ChainResourceManifest,
    latest_delta: Option<ChainResourceDelta>,
    _world_id: &str,
    _chain_id: &str,
) -> (
    Option<ChainResourceManifest>,
    Option<ChainResourceDelta>,
    Option<String>,
) {
    (Some(manifest), latest_delta, None)
}

fn simulator_world_dir_from_execution_world_dir(world_dir: &Path) -> std::path::PathBuf {
    match world_dir.file_name().and_then(|name| name.to_str()) {
        Some(name) if !name.is_empty() => {
            world_dir.with_file_name(format!("{name}-simulator-mirror"))
        }
        _ => world_dir.join("simulator-mirror"),
    }
}

fn starter_chunk_manifest_hash(manifest: Option<&ChainResourceManifest>) -> Option<String> {
    manifest
        .and_then(|manifest| manifest.generated_chunks.values().next())
        .map(|entry| entry.manifest_hash.clone())
        .filter(|hash| !hash.trim().is_empty())
}

fn committed_chunk_count(manifest: &ChainResourceManifest) -> u64 {
    manifest
        .generated_chunks
        .values()
        .filter(|entry| entry.chunk_status == ChainChunkResourceStatus::Committed)
        .count() as u64
}

fn provisional_chunk_count(manifest: &ChainResourceManifest) -> u64 {
    manifest
        .generated_chunks
        .values()
        .filter(|entry| {
            matches!(
                entry.chunk_status,
                ChainChunkResourceStatus::ChainPending | ChainChunkResourceStatus::Provisional
            )
        })
        .count() as u64
}

fn delta_has_nonzero_resource_effect(delta: &ChainResourceDelta) -> bool {
    delta.entries.iter().any(|entry| match entry {
        ChainResourceDeltaEntry::RuntimeResource { delta, .. } => *delta != 0,
        ChainResourceDeltaEntry::MaterialLedger { delta, .. } => *delta != 0,
        ChainResourceDeltaEntry::ChunkResource {
            total_delta_g,
            remaining_delta_g,
            resulting_remaining_g,
            ..
        } => *total_delta_g != 0 || *remaining_delta_g != 0 || *resulting_remaining_g != 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use oasis7::geometry::GeoPos;
    use oasis7::runtime::{Action, World as RuntimeWorld};
    use oasis7_node::{NodeConsensusSnapshot, NodeRole};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("duration")
            .as_nanos();
        std::env::temp_dir().join(format!("oasis7-status-world-resource-{prefix}-{unique}"))
    }

    #[test]
    fn status_uses_runtime_execution_snapshot_when_simulator_mirror_is_empty() {
        let dir = temp_dir("runtime-fallback");
        let mut world = RuntimeWorld::new();
        world.submit_action(Action::RegisterAgent {
            agent_id: "starter-agent-0".to_string(),
            pos: GeoPos::new(0, 0, 0),
        });
        world.step().expect("register agent");
        world
            .save_to_dir(dir.as_path())
            .expect("save runtime world");

        let snapshot = NodeSnapshot {
            node_id: "node-a".to_string(),
            player_id: "player-a".to_string(),
            world_id: "world-a".to_string(),
            role: NodeRole::Sequencer,
            replication_enabled: false,
            running: true,
            tick_count: 1,
            last_tick_unix_ms: None,
            consensus: NodeConsensusSnapshot {
                committed_height: 1,
                last_block_hash: Some("block-a".to_string()),
                ..NodeConsensusSnapshot::default()
            },
            last_error: None,
        };

        let status = build_world_resource_status(&snapshot, dir.as_path(), None);

        assert_eq!(status.world_id, "world-a");
        assert_eq!(status.chain_id, "world-a");
        assert_eq!(status.readiness_status, "not_ready");
        assert_eq!(status.committed_chunk_count, 1);
        assert_eq!(status.pending_delta_count, 0);
        assert!(
            status
                .failed_gates
                .contains(&"world_resource_delta_commit_hash_missing".to_string()),
            "{:?}",
            status.failed_gates
        );
        assert!(
            status
                .failed_gates
                .contains(&"world_resource_delta_zero_effect".to_string()),
            "{:?}",
            status.failed_gates
        );
        assert!(
            status
                .failed_gates
                .contains(&"world_resource_world_id_mismatch".to_string()),
            "{:?}",
            status.failed_gates
        );
        assert!(
            status
                .failed_gates
                .contains(&"world_resource_chain_id_mismatch".to_string()),
            "{:?}",
            status.failed_gates
        );
        assert!(status.starter_chunk_manifest_hash.is_some());

        let _ = fs::remove_dir_all(dir);
    }
}
