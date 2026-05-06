// Execution result storage helpers for distributed runtime (net crate facade).

use serde::Serialize;

use super::distributed::{BlobRef, BlockAnnounce, WorldBlock, WorldHeadAnnounce};
use super::distributed_storage::{
    ExecutionWriteConfig as DistributedExecutionWriteConfig,
    ExecutionWriteResult as DistributedExecutionWriteResult,
};
use super::error::WorldError;
use super::util::to_canonical_cbor;
use oasis7::runtime::{ActionId, CausedBy, Journal, Snapshot, WorldEventBody};
use oasis7_distfs::{blake3_hex, segment_journal, segment_snapshot, BlobStore, FileStore};

const WORLDS_ROOT_DIR: &str = "worlds";
const HEADS_DIR: &str = "heads";
const BLOCKS_DIR: &str = "blocks";
const LATEST_HEAD_FILE: &str = "latest_head.cbor";
const BLOCK_FILE: &str = "block.cbor";
const SNAPSHOT_MANIFEST_FILE: &str = "snapshot_manifest.cbor";
const JOURNAL_SEGMENTS_FILE: &str = "journal_segments.cbor";

pub fn store_execution_result(
    world_id: &str,
    height: u64,
    prev_block_hash: &str,
    proposer_id: &str,
    snapshot_epoch: u64,
    snapshot: &Snapshot,
    journal: &Journal,
    store: &impl BlobStore,
    config: DistributedExecutionWriteConfig,
) -> Result<DistributedExecutionWriteResult, WorldError> {
    let DistributedExecutionWriteConfig {
        segment: segment_config,
        codec,
    } = config;
    let snapshot_manifest =
        segment_snapshot(snapshot, world_id, snapshot_epoch, store, segment_config)?;
    let snapshot_manifest_bytes = to_canonical_cbor(&snapshot_manifest)?;
    let snapshot_manifest_hash = store.put_bytes(&snapshot_manifest_bytes)?;
    let snapshot_manifest_ref = BlobRef {
        content_hash: snapshot_manifest_hash.clone(),
        size_bytes: snapshot_manifest_bytes.len() as u64,
        codec: codec.clone(),
        links: snapshot_manifest
            .chunks
            .iter()
            .map(|chunk| chunk.content_hash.clone())
            .collect(),
    };

    let journal_segments =
        segment_journal(&journal.events, store, segment_config, |event| event.id)?;
    let journal_segments_bytes = to_canonical_cbor(&journal_segments)?;
    let journal_segments_hash = store.put_bytes(&journal_segments_bytes)?;
    let journal_segments_ref = BlobRef {
        content_hash: journal_segments_hash.clone(),
        size_bytes: journal_segments_bytes.len() as u64,
        codec: codec.clone(),
        links: journal_segments
            .iter()
            .map(|segment| segment.content_hash.clone())
            .collect(),
    };

    let action_root = hash_actions(journal)?;
    let event_root = hash_events(journal)?;
    let receipts_root = hash_receipts(journal)?;
    let timestamp_ms = i64::try_from(snapshot.state.time).map_err(|_| {
        WorldError::DistributedValidationFailed {
            reason: format!("snapshot time exceeds i64 range: {}", snapshot.state.time),
        }
    })?;

    let block = WorldBlock {
        world_id: world_id.to_string(),
        height,
        prev_block_hash: prev_block_hash.to_string(),
        action_root,
        event_root: event_root.clone(),
        state_root: snapshot_manifest.state_root.clone(),
        journal_ref: journal_segments_hash.clone(),
        snapshot_ref: snapshot_manifest_hash.clone(),
        receipts_root,
        proposer_id: proposer_id.to_string(),
        timestamp_ms,
        signature: String::new(),
    };

    let block_bytes = to_canonical_cbor(&block)?;
    let block_hash = store.put_bytes(&block_bytes)?;
    let block_ref = BlobRef {
        content_hash: block_hash.clone(),
        size_bytes: block_bytes.len() as u64,
        codec: codec.clone(),
        links: vec![
            snapshot_manifest_hash.clone(),
            journal_segments_hash.clone(),
        ],
    };

    let block_announce = BlockAnnounce {
        world_id: world_id.to_string(),
        height,
        block_hash: block_hash.clone(),
        prev_block_hash: prev_block_hash.to_string(),
        state_root: snapshot_manifest.state_root.clone(),
        event_root,
        timestamp_ms,
        signature: String::new(),
    };

    let head_announce = WorldHeadAnnounce {
        world_id: world_id.to_string(),
        height,
        block_hash: block_hash.clone(),
        state_root: snapshot_manifest.state_root.clone(),
        timestamp_ms,
        signature: String::new(),
    };

    Ok(DistributedExecutionWriteResult {
        block,
        block_hash,
        block_ref,
        block_announce,
        head_announce,
        snapshot_manifest,
        snapshot_manifest_ref,
        journal_segments,
        journal_segments_ref,
    })
}

pub fn store_execution_result_with_path_index(
    world_id: &str,
    height: u64,
    prev_block_hash: &str,
    proposer_id: &str,
    snapshot_epoch: u64,
    snapshot: &Snapshot,
    journal: &Journal,
    store: &(impl BlobStore + FileStore),
    config: DistributedExecutionWriteConfig,
) -> Result<DistributedExecutionWriteResult, WorldError> {
    let result = store_execution_result(
        world_id,
        height,
        prev_block_hash,
        proposer_id,
        snapshot_epoch,
        snapshot,
        journal,
        store,
        config,
    )?;
    write_execution_path_index(world_id, height, &result, store)?;
    Ok(result)
}

pub fn load_block_by_height_from_path_index(
    world_id: &str,
    height: u64,
    store: &impl FileStore,
) -> Result<WorldBlock, WorldError> {
    let path_index = ExecutionPathIndexLayout::new(world_id, height)?;
    let block_bytes = store.read_file(&path_index.block_path)?;
    Ok(serde_cbor::from_slice(&block_bytes)?)
}

pub fn load_latest_head_from_path_index(
    world_id: &str,
    store: &impl FileStore,
) -> Result<WorldHeadAnnounce, WorldError> {
    let world_segment = normalize_world_segment(world_id)?;
    let latest_head_path =
        format!("{WORLDS_ROOT_DIR}/{world_segment}/{HEADS_DIR}/{LATEST_HEAD_FILE}");
    let head_bytes = store.read_file(&latest_head_path)?;
    Ok(serde_cbor::from_slice(&head_bytes)?)
}

fn write_execution_path_index(
    world_id: &str,
    height: u64,
    result: &DistributedExecutionWriteResult,
    store: &impl FileStore,
) -> Result<(), WorldError> {
    let path_index = ExecutionPathIndexLayout::new(world_id, height)?;
    let block_bytes = to_canonical_cbor(&result.block)?;
    let snapshot_manifest_bytes = to_canonical_cbor(&result.snapshot_manifest)?;
    let journal_segments_bytes = to_canonical_cbor(&result.journal_segments)?;
    let latest_head_bytes = to_canonical_cbor(&result.head_announce)?;

    store.write_file(&path_index.latest_head_path, &latest_head_bytes)?;
    store.write_file(&path_index.block_path, &block_bytes)?;
    store.write_file(&path_index.snapshot_manifest_path, &snapshot_manifest_bytes)?;
    store.write_file(&path_index.journal_segments_path, &journal_segments_bytes)?;
    Ok(())
}

#[derive(Debug, Clone)]
struct ExecutionPathIndexLayout {
    latest_head_path: String,
    block_path: String,
    snapshot_manifest_path: String,
    journal_segments_path: String,
}

impl ExecutionPathIndexLayout {
    fn new(world_id: &str, height: u64) -> Result<Self, WorldError> {
        let world_segment = normalize_world_segment(world_id)?;
        let height_segment = format!("{height:020}");
        let block_dir = format!("{WORLDS_ROOT_DIR}/{world_segment}/{BLOCKS_DIR}/{height_segment}");

        Ok(Self {
            latest_head_path: format!(
                "{WORLDS_ROOT_DIR}/{world_segment}/{HEADS_DIR}/{LATEST_HEAD_FILE}"
            ),
            block_path: format!("{block_dir}/{BLOCK_FILE}"),
            snapshot_manifest_path: format!("{block_dir}/{SNAPSHOT_MANIFEST_FILE}"),
            journal_segments_path: format!("{block_dir}/{JOURNAL_SEGMENTS_FILE}"),
        })
    }
}

fn normalize_world_segment(world_id: &str) -> Result<String, WorldError> {
    if world_id.is_empty() {
        return Err(WorldError::DistributedValidationFailed {
            reason: "invalid world_id for path index: empty".to_string(),
        });
    }

    if world_id
        .chars()
        .any(|ch| !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.'))
    {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!("invalid world_id for path index: {world_id}"),
        });
    }

    Ok(world_id.to_string())
}

fn hash_actions(journal: &Journal) -> Result<String, WorldError> {
    let actions: Vec<ActionId> = journal
        .events
        .iter()
        .filter_map(|event| match event.caused_by {
            Some(CausedBy::Action(action_id)) => Some(action_id),
            _ => None,
        })
        .collect();
    hash_cbor(&actions)
}

fn hash_events(journal: &Journal) -> Result<String, WorldError> {
    hash_cbor(&journal.events)
}

fn hash_receipts(journal: &Journal) -> Result<String, WorldError> {
    let receipts = journal
        .events
        .iter()
        .filter_map(|event| match &event.body {
            WorldEventBody::ReceiptAppended(receipt) => Some(receipt.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    hash_cbor(&receipts)
}

fn hash_cbor<T: Serialize>(value: &T) -> Result<String, WorldError> {
    let bytes = to_canonical_cbor(value)?;
    Ok(blake3_hex(&bytes))
}

#[cfg(all(test, feature = "self_tests"))]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use oasis7::runtime::{Action, World};
    use oasis7::GeoPos;
    use oasis7_distfs::{BlobStore as _, FileStore as _, LocalCasStore};

    use super::super::distributed_storage::ExecutionWriteConfig;
    use super::*;

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("duration since epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("oasis7-net-{prefix}-{unique}"))
    }

    #[test]
    fn store_execution_result_writes_block_and_refs() {
        let dir = temp_dir("exec-store");
        let store = LocalCasStore::new(&dir);
        let mut world = World::new();

        world.submit_action(Action::RegisterAgent {
            agent_id: "agent-1".to_string(),
            pos: GeoPos::new(0, 0, 0),
        });
        world.step().expect("step world");

        let snapshot = world.snapshot();
        let journal = world.journal().clone();

        let result = store_execution_result(
            "w1",
            1,
            "genesis",
            "exec-1",
            1,
            &snapshot,
            &journal,
            &store,
            ExecutionWriteConfig::default(),
        )
        .expect("store execution");

        assert!(store.has(&result.block_ref.content_hash).expect("block"));
        assert!(store
            .has(&result.snapshot_manifest_ref.content_hash)
            .expect("manifest"));
        assert!(store
            .has(&result.journal_segments_ref.content_hash)
            .expect("journal index"));
        assert_eq!(
            result.block.snapshot_ref,
            result.snapshot_manifest_ref.content_hash
        );
        assert_eq!(
            result.block.journal_ref,
            result.journal_segments_ref.content_hash
        );
        assert_eq!(result.block.world_id, "w1");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn store_execution_result_with_path_index_writes_lookup_paths() {
        let dir = temp_dir("exec-store-path-index");
        let store = LocalCasStore::new(&dir);
        let mut world = World::new();

        world.submit_action(Action::RegisterAgent {
            agent_id: "agent-1".to_string(),
            pos: GeoPos::new(0, 0, 0),
        });
        world.step().expect("step world");

        let snapshot = world.snapshot();
        let journal = world.journal().clone();

        let result = store_execution_result_with_path_index(
            "w_path_1",
            1,
            "genesis",
            "exec-1",
            1,
            &snapshot,
            &journal,
            &store,
            ExecutionWriteConfig::default(),
        )
        .expect("store execution with path index");

        let height_segment = format!("{:020}", 1);
        let latest_head_path = "worlds/w_path_1/heads/latest_head.cbor";
        let block_path = format!("worlds/w_path_1/blocks/{height_segment}/block.cbor");
        let snapshot_manifest_path =
            format!("worlds/w_path_1/blocks/{height_segment}/snapshot_manifest.cbor");
        let journal_segments_path =
            format!("worlds/w_path_1/blocks/{height_segment}/journal_segments.cbor");

        assert!(store
            .stat_file(latest_head_path)
            .expect("latest head stat")
            .is_some());
        assert!(store.stat_file(&block_path).expect("block stat").is_some());
        assert!(store
            .stat_file(&snapshot_manifest_path)
            .expect("manifest stat")
            .is_some());
        assert!(store
            .stat_file(&journal_segments_path)
            .expect("journal stat")
            .is_some());

        let loaded_block =
            load_block_by_height_from_path_index("w_path_1", 1, &store).expect("load block");
        let loaded_head =
            load_latest_head_from_path_index("w_path_1", &store).expect("load latest head");

        assert_eq!(loaded_block, result.block);
        assert_eq!(loaded_head.block_hash, result.block_hash);
        assert_eq!(loaded_head.world_id, "w_path_1");
        assert_eq!(loaded_head.height, 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn store_execution_result_with_path_index_rejects_invalid_world_id() {
        let dir = temp_dir("exec-store-path-index-invalid-world");
        let store = LocalCasStore::new(&dir);
        let mut world = World::new();

        world.submit_action(Action::RegisterAgent {
            agent_id: "agent-1".to_string(),
            pos: GeoPos::new(0, 0, 0),
        });
        world.step().expect("step world");

        let snapshot = world.snapshot();
        let journal = world.journal().clone();

        let result = store_execution_result_with_path_index(
            "bad/world",
            1,
            "genesis",
            "exec-1",
            1,
            &snapshot,
            &journal,
            &store,
            ExecutionWriteConfig::default(),
        );
        assert!(matches!(
            result,
            Err(WorldError::DistributedValidationFailed { .. })
        ));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn store_execution_result_rejects_snapshot_time_overflow() {
        let dir = temp_dir("exec-store-time-overflow");
        let store = LocalCasStore::new(&dir);
        let mut world = World::new();

        world.submit_action(Action::RegisterAgent {
            agent_id: "agent-1".to_string(),
            pos: GeoPos::new(0, 0, 0),
        });
        world.step().expect("step world");

        let mut snapshot = world.snapshot();
        snapshot.state.time = i64::MAX as u64 + 1;
        let journal = world.journal().clone();

        let result = store_execution_result(
            "w1",
            1,
            "genesis",
            "exec-1",
            1,
            &snapshot,
            &journal,
            &store,
            ExecutionWriteConfig::default(),
        );
        assert!(matches!(
            result,
            Err(WorldError::DistributedValidationFailed { .. })
        ));

        let _ = fs::remove_dir_all(&dir);
    }
}
