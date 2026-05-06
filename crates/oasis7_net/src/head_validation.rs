// Head update and replay validation helpers for distributed runtime (net crate facade).

use serde::Serialize;

use super::distributed::{SnapshotManifest, WorldBlock, WorldHeadAnnounce};
use super::error::WorldError;
use super::util::to_canonical_cbor;
use oasis7::runtime::{
    ActionId, CausedBy, Journal, Snapshot, World, WorldError as RuntimeWorldError, WorldEvent,
    WorldEventBody,
};
use oasis7_distfs::{
    assemble_journal as distfs_assemble_journal, assemble_snapshot as distfs_assemble_snapshot,
    blake3_hex, BlobStore,
};
use oasis7_proto::distributed_storage::JournalSegmentRef;

#[derive(Debug, Clone)]
pub struct HeadValidationResult {
    pub block_hash: String,
    pub snapshot: Snapshot,
    pub journal: Journal,
}

pub fn validate_head_update(
    head: &WorldHeadAnnounce,
    block: &WorldBlock,
    snapshot_manifest: &SnapshotManifest,
    journal_segments: &[JournalSegmentRef],
    store: &impl BlobStore,
) -> Result<HeadValidationResult, WorldError> {
    if head.world_id != block.world_id {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "world_id mismatch: head={}, block={}",
                head.world_id, block.world_id
            ),
        });
    }
    if head.height != block.height {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "height mismatch: head={}, block={}",
                head.height, block.height
            ),
        });
    }
    if head.state_root != block.state_root {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "state_root mismatch: head={}, block={}",
                head.state_root, block.state_root
            ),
        });
    }

    let manifest_hash = hash_cbor(snapshot_manifest)?;
    if block.snapshot_ref != manifest_hash {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "snapshot_ref mismatch: block={}, manifest={}",
                block.snapshot_ref, manifest_hash
            ),
        });
    }
    let journal_ref_hash = hash_cbor(&journal_segments)?;
    if block.journal_ref != journal_ref_hash {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "journal_ref mismatch: block={}, segments={}",
                block.journal_ref, journal_ref_hash
            ),
        });
    }

    let snapshot = assemble_snapshot(snapshot_manifest, store)?;
    let journal = assemble_journal(journal_segments, store)?;
    if snapshot.journal_len != journal.len() {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "journal length mismatch: snapshot={}, journal={}",
                snapshot.journal_len,
                journal.len()
            ),
        });
    }

    let action_root = hash_actions(&journal)?;
    if block.action_root != action_root {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "action_root mismatch: block={}, computed={}",
                block.action_root, action_root
            ),
        });
    }
    let event_root = hash_events(&journal)?;
    if block.event_root != event_root {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "event_root mismatch: block={}, computed={}",
                block.event_root, event_root
            ),
        });
    }
    let receipts_root = hash_receipts(&journal)?;
    if block.receipts_root != receipts_root {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "receipts_root mismatch: block={}, computed={}",
                block.receipts_root, receipts_root
            ),
        });
    }

    let block_hash = hash_cbor(block)?;
    if head.block_hash != block_hash {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "block_hash mismatch: head={}, computed={}",
                head.block_hash, block_hash
            ),
        });
    }

    World::from_snapshot(snapshot.clone(), journal.clone())
        .map_err(runtime_world_error_to_proto)?;

    Ok(HeadValidationResult {
        block_hash,
        snapshot,
        journal,
    })
}

pub fn assemble_snapshot(
    manifest: &SnapshotManifest,
    store: &impl BlobStore,
) -> Result<Snapshot, WorldError> {
    distfs_assemble_snapshot(manifest, store)
}

pub fn assemble_journal(
    segments: &[JournalSegmentRef],
    store: &impl BlobStore,
) -> Result<Journal, WorldError> {
    let events = distfs_assemble_journal(segments, store, |event: &WorldEvent| event.id)?;
    Ok(Journal { events })
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

fn runtime_world_error_to_proto(error: RuntimeWorldError) -> WorldError {
    WorldError::DistributedValidationFailed {
        reason: format!("runtime world validation failed: {error:?}"),
    }
}

#[cfg(all(test, feature = "self_tests"))]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use oasis7::runtime::{Action, World};
    use oasis7::GeoPos;
    use oasis7_distfs::LocalCasStore;

    use super::super::distributed_storage::{store_execution_result, ExecutionWriteConfig};
    use super::*;

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("duration since epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("oasis7-net-{prefix}-{unique}"))
    }

    #[test]
    fn validate_head_update_accepts_written_block() {
        let dir = temp_dir("head-validate");
        let store = LocalCasStore::new(&dir);
        let mut world = World::new();

        world.submit_action(Action::RegisterAgent {
            agent_id: "agent-1".to_string(),
            pos: GeoPos::new(0, 0, 0),
        });
        world.step().expect("step world");

        let snapshot = world.snapshot();
        let journal = world.journal().clone();
        let write = store_execution_result(
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
        .expect("write");

        let result = validate_head_update(
            &write.head_announce,
            &write.block,
            &write.snapshot_manifest,
            &write.journal_segments,
            &store,
        )
        .expect("validate");

        assert_eq!(result.block_hash, write.head_announce.block_hash);

        let _ = fs::remove_dir_all(&dir);
    }
}
