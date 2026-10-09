//! One immutable record pins snapshot, journal and proof. Mutable cache paths
//! are never a read authority for WorldService.
use super::{ExecutionBridgeRecord, WorldHeadProofV1, checkpoint};
use oasis7::runtime::{Journal, LocalCasStore, Snapshot, World, blake3_hex};
use oasis7::world_service::*;
use std::path::Path;

pub(crate) struct PinnedWorld {
    pub world: World,
    pub record: ExecutionBridgeRecord,
    pub commit: CommitRef,
}

pub(crate) fn identity(world_dir: &Path, expected_world_id: &str) -> Result<WorldIdentity, String> {
    let bytes = std::fs::read(world_dir.join("world-service-identity.json"))
        .map_err(|_| "service identity configuration unavailable")?;
    let identity: WorldIdentity = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    identity.validate().map_err(|e| e.to_string())?;
    if identity.world_id != expected_world_id {
        return Err("service identity differs from runtime world".into());
    }
    Ok(identity)
}

pub(crate) fn pin(
    records: &Path,
    storage: &Path,
    identity: &WorldIdentity,
    fixed: Option<&CommitRef>,
) -> Result<PinnedWorld, String> {
    load(
        records,
        storage,
        identity,
        fixed.map(|commit| commit.position),
        fixed,
    )
}

pub(crate) fn pin_at_height(
    records: &Path,
    storage: &Path,
    identity: &WorldIdentity,
    height: u64,
) -> Result<PinnedWorld, String> {
    load(records, storage, identity, Some(height), None)
}

fn load(
    records: &Path,
    storage: &Path,
    identity: &WorldIdentity,
    height: Option<u64>,
    fixed: Option<&CommitRef>,
) -> Result<PinnedWorld, String> {
    if let Some(commit) = fixed {
        if &commit.world != identity {
            return Err("fixed commit world mismatch".into());
        }
    }
    let record = if let Some(height) = height {
        checkpoint::load_execution_bridge_record(&checkpoint::execution_bridge_record_path(
            records, height,
        ))?
    } else {
        checkpoint::load_highest_valid_execution_bridge_record(records)?
            .ok_or("committed view unavailable")?
    };
    if record.world_id != identity.world_id || record.height == 0 {
        return Err("record world or height invalid".into());
    }
    let store = LocalCasStore::new(storage.to_path_buf());
    let snapshot_ref = record
        .snapshot_ref
        .as_deref()
        .ok_or("record snapshot ref unavailable")?;
    let journal_ref = record
        .journal_ref
        .as_deref()
        .ok_or("record journal ref unavailable")?;
    if record.latest_state_ref.as_deref() != Some(snapshot_ref) {
        return Err("record snapshot refs disagree".into());
    }
    let snapshot_bytes = store
        .get_verified(snapshot_ref)
        .map_err(|e| format!("snapshot CAS: {e:?}"))?;
    let journal_bytes = store
        .get_verified(journal_ref)
        .map_err(|e| format!("journal CAS: {e:?}"))?;
    let snapshot: Snapshot = serde_cbor::from_slice(&snapshot_bytes).map_err(|e| e.to_string())?;
    let journal: Journal = serde_cbor::from_slice(&journal_bytes).map_err(|e| e.to_string())?;
    if blake3_hex(&snapshot_bytes) != record.execution_state_root
        || snapshot.journal_len != record.journal_len
        || journal.len() != record.journal_len
        || journal.events.last().map_or(0, |e| e.id) != snapshot.last_event_id
    {
        return Err("pinned snapshot/journal root or boundary mismatch".into());
    }
    let proof_ref = record
        .world_head_proof_ref
        .as_deref()
        .ok_or("record world head proof unavailable")?;
    let proof_bytes = store
        .get_verified(proof_ref)
        .map_err(|e| format!("proof CAS: {e:?}"))?;
    let proof: WorldHeadProofV1 =
        serde_cbor::from_slice(&proof_bytes).map_err(|e| e.to_string())?;
    proof.validate_contract()?;
    if Some(proof.proof_hash()?.as_str()) != record.world_head_proof_hash.as_deref()
        || proof.world_id != record.world_id
        || proof.height != record.height
        || proof.timestamp_ms != record.timestamp_ms
        || proof.execution.execution_state_root != record.execution_state_root
        || proof.execution.execution_block_hash != record.execution_block_hash
        || proof.execution.node_block_hash != record.node_block_hash.as_deref().unwrap_or("")
        || proof.execution.action_root != record.action_root.as_deref().unwrap_or("")
        || proof.snapshot_manifest_ref.content_hash != snapshot_ref
        || proof.journal_segments_ref.content_hash != journal_ref
    {
        return Err("pinned proof does not bind execution record and CAS refs".into());
    }
    let world =
        World::from_snapshot(snapshot, journal).map_err(|e| format!("pinned world: {e:?}"))?;
    let binding = world
        .current_cognition_runtime_binding()
        .map_err(|e| format!("binding unavailable: {e:?}"))?;
    if binding.world_id != identity.world_id {
        return Err("pinned binding world mismatch".into());
    }
    let commit = CommitRef {
        world: identity.clone(),
        binding: ExecutionBinding {
            provider_world_id: binding.world_id,
            branch_id: binding.branch_id,
            finality_ref: authority::request_digest(
                "finality",
                &(
                    binding.finality_epoch,
                    binding.finality_status,
                    binding.finality_block_hash,
                ),
            )?,
            reorg_generation: binding.reorg_epoch,
            governing_manifest_ref: binding.runtime_manifest_hash.to_string(),
            authority_generation: world
                .capability_revocation_state()
                .authority_records
                .values()
                .map(|record| record.governance_epoch)
                .max()
                .unwrap_or(0),
            permission_generation: world.capability_revocation_state().epoch,
        },
        position: record.height,
        execution_block_hash: record.execution_block_hash.clone(),
        state_root_ref: snapshot_ref.into(),
    };
    commit.validate().map_err(|e| e.to_string())?;
    if fixed.is_some_and(|fixed| fixed != &commit) {
        return Err("fixed commit reference conflict".into());
    }
    Ok(PinnedWorld {
        world,
        record,
        commit,
    })
}
