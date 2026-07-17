use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use oasis7::runtime::{
    BlobStore, ChainResourceDerivationContext, Journal as RuntimeJournal,
    Snapshot as RuntimeSnapshot, World as RuntimeWorld, blake3_hex,
};
use oasis7_node::{
    NodeExecutionCheckpointBundle, NodeExecutionCheckpointInstallContext, NodeExecutionCommitResult,
};
use serde::{Deserialize, Serialize};

use super::checkpoint::{
    execution_bridge_record_path, execution_checkpoint_latest_path,
    execution_checkpoint_manifest_path, execution_checkpoint_manifest_rel_path,
    persist_execution_bridge_record, persist_execution_checkpoint_manifest,
    run_execution_bridge_retention_maintenance,
};

const CHECKPOINT_INSTALL_TRANSACTION_FILE: &str = "checkpoint-install-transaction.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileBackup {
    path: PathBuf,
    bytes: Option<Vec<u8>>,
}

impl FileBackup {
    fn capture(path: PathBuf) -> Result<Self, String> {
        let bytes = path
            .exists()
            .then(|| fs::read(path.as_path()))
            .transpose()
            .map_err(|err| {
                format!(
                    "read checkpoint install backup {} failed: {}",
                    path.display(),
                    err
                )
            })?;
        Ok(Self { path, bytes })
    }

    fn restore(&self) -> Result<(), String> {
        match &self.bytes {
            Some(bytes) => super::write_bytes_atomic(self.path.as_path(), bytes.as_slice()),
            None => match fs::remove_file(self.path.as_path()) {
                Ok(()) => Ok(()),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(err) => Err(format!(
                    "remove checkpoint install publication {} failed: {}",
                    self.path.display(),
                    err
                )),
            },
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
enum CheckpointInstallTransactionPhase {
    Prepared,
    Committed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CheckpointInstallTransaction {
    phase: CheckpointInstallTransactionPhase,
    previous_state: super::ExecutionBridgeState,
    backups: Vec<FileBackup>,
}

impl CheckpointInstallTransaction {
    fn path(records_dir: &Path) -> PathBuf {
        records_dir.join(CHECKPOINT_INSTALL_TRANSACTION_FILE)
    }

    fn prepare(driver: &NodeRuntimeExecutionDriver, height: u64) -> Result<Self, String> {
        let records_dir = driver.records_dir.as_path();
        let backups = [
            driver.state_path.clone(),
            driver.world_dir.join("snapshot.json"),
            driver.world_dir.join("journal.json"),
            execution_bridge_record_path(records_dir, height),
            records_dir.join("latest.json"),
            execution_checkpoint_manifest_path(records_dir, height),
            execution_checkpoint_latest_path(records_dir),
        ]
        .into_iter()
        .map(FileBackup::capture)
        .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            phase: CheckpointInstallTransactionPhase::Prepared,
            previous_state: driver.state.clone(),
            backups,
        })
    }

    fn persist(&self, records_dir: &Path) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|err| format!("serialize checkpoint install transaction failed: {err}"))?;
        super::write_bytes_atomic(Self::path(records_dir).as_path(), bytes.as_slice())
    }

    fn restore(&self) -> Result<(), String> {
        for backup in &self.backups {
            backup.restore()?;
        }
        Ok(())
    }
}

fn remove_checkpoint_install_transaction(records_dir: &Path) -> Result<(), String> {
    match fs::remove_file(CheckpointInstallTransaction::path(records_dir)) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!(
            "remove checkpoint install transaction failed: {err}"
        )),
    }
}

pub(super) fn recover_checkpoint_install_transaction(
    driver: &mut NodeRuntimeExecutionDriver,
) -> Result<(), String> {
    let path = CheckpointInstallTransaction::path(driver.records_dir.as_path());
    if !path.exists() {
        return Ok(());
    }
    let transaction: CheckpointInstallTransaction = serde_json::from_slice(
        &fs::read(path.as_path())
            .map_err(|err| format!("read checkpoint install transaction failed: {err}"))?,
    )
    .map_err(|err| format!("parse checkpoint install transaction failed: {err}"))?;
    if transaction.phase == CheckpointInstallTransactionPhase::Prepared {
        transaction.restore()?;
        driver.state = transaction.previous_state;
        let policy = driver.execution_world.release_security_policy().clone();
        driver.execution_world = super::driver_persistence::load_execution_world_with_policy(
            driver.world_dir.as_path(),
            policy,
        )?;
    }
    remove_checkpoint_install_transaction(driver.records_dir.as_path())
}

#[cfg(test)]
pub(super) fn prepare_checkpoint_install_transaction_for_test(
    driver: &NodeRuntimeExecutionDriver,
    height: u64,
) -> Result<(), String> {
    let transaction = CheckpointInstallTransaction::prepare(driver, height)?;
    transaction.persist(driver.records_dir.as_path())
}
use super::driver::NodeRuntimeExecutionDriver;
use super::driver_observability::{
    CheckpointInstallObservation, emit_checkpoint_bundle_install_complete,
    emit_checkpoint_bundle_install_start,
};
use super::driver_persistence::{
    persist_execution_bridge_state, persist_execution_world_with_chain_resource_context,
};
use super::{
    EXECUTION_BRIDGE_RECORD_SCHEMA_V3, EXECUTION_CHECKPOINT_MANIFEST_SCHEMA_V2,
    ExecutionBridgeRecord,
};

pub(super) fn install_checkpoint_bundle(
    driver: &mut NodeRuntimeExecutionDriver,
    context: NodeExecutionCheckpointInstallContext,
    bundle: NodeExecutionCheckpointBundle,
) -> Result<NodeExecutionCommitResult, String> {
    let install_started_at = Instant::now();
    if bundle.height != context.height
        || bundle.execution_block_hash != context.execution_block_hash
        || bundle.execution_state_root != context.execution_state_root
    {
        return Err(format!(
            "execution checkpoint bundle does not match install context height={}",
            context.height
        ));
    }
    let manifest_decode_started_at = Instant::now();
    let manifest =
        serde_json::from_slice::<super::ExecutionCheckpointManifest>(&bundle.manifest_json)
            .map_err(|err| {
                format!(
                    "decode execution checkpoint manifest failed at height {}: {}",
                    context.height, err
                )
            })?;
    manifest.validate()?;
    if manifest.schema_version < EXECUTION_CHECKPOINT_MANIFEST_SCHEMA_V2 {
        return Err(format!(
            "execution checkpoint v1 manifest cannot be installed at height {}: v2 predecessor anchor required",
            context.height
        ));
    }
    let manifest_decode_ms = manifest_decode_started_at.elapsed();
    if manifest.world_id != context.world_id
        || manifest.height != context.height
        || manifest.execution_block_hash != context.execution_block_hash
        || manifest.execution_state_root != context.execution_state_root
    {
        return Err(format!(
            "execution checkpoint manifest does not match install context height={}",
            context.height
        ));
    }

    let bundle_blob_count = bundle.blobs.len();
    let bundle_bytes = bundle
        .blobs
        .iter()
        .fold(bundle.manifest_json.len(), |total, blob| {
            total.saturating_add(blob.bytes.len())
        });
    emit_checkpoint_bundle_install_start(
        context.world_id.as_str(),
        context.height,
        manifest.checkpoint_id.as_str(),
        bundle_blob_count,
        bundle_bytes,
        manifest.pinned_refs.len(),
    );

    let blob_store_started_at = Instant::now();
    for blob in &bundle.blobs {
        let actual = blake3_hex(blob.bytes.as_slice());
        if actual != blob.content_hash {
            return Err(format!(
                "execution checkpoint blob hash mismatch expected={} actual={}",
                blob.content_hash, actual
            ));
        }
        driver
            .execution_store
            .put(blob.content_hash.as_str(), blob.bytes.as_slice())
            .map_err(|err| {
                format!(
                    "store execution checkpoint blob {} failed: {:?}",
                    blob.content_hash, err
                )
            })?;
    }
    let mut blob_store_ms = blob_store_started_at.elapsed();
    let pin_check_started_at = Instant::now();
    for content_hash in &manifest.pinned_refs {
        if !driver
            .execution_store
            .has(content_hash.as_str())
            .map_err(|err| {
                format!(
                    "check execution checkpoint blob {} failed: {:?}",
                    content_hash, err
                )
            })?
        {
            return Err(format!(
                "execution checkpoint missing pinned blob {} at height {}",
                content_hash, context.height
            ));
        }
    }
    let pin_check_ms = pin_check_started_at.elapsed();

    let snapshot_blob_started_at = Instant::now();
    let snapshot_bytes = driver
        .execution_store
        .get_verified(manifest.latest_state_ref.as_str())
        .map_err(|err| {
            format!(
                "load execution checkpoint snapshot {} failed: {:?}",
                manifest.latest_state_ref, err
            )
        })?;
    blob_store_ms += snapshot_blob_started_at.elapsed();
    let snapshot_bytes_len = snapshot_bytes.len();
    let snapshot_decode_started_at = Instant::now();
    let snapshot =
        serde_cbor::from_slice::<RuntimeSnapshot>(snapshot_bytes.as_slice()).map_err(|err| {
            format!(
                "decode execution checkpoint snapshot failed at height {}: {}",
                context.height, err
            )
        })?;
    let mut decode_ms = manifest_decode_ms + snapshot_decode_started_at.elapsed();
    let actual_state_root = blake3_hex(snapshot_bytes.as_slice());
    if actual_state_root != context.execution_state_root {
        return Err(format!(
            "execution checkpoint snapshot root mismatch at height {}: expected={} actual={}",
            context.height, context.execution_state_root, actual_state_root
        ));
    }
    let journal_ref = manifest.journal_ref.as_deref().ok_or_else(|| {
        format!(
            "execution checkpoint manifest missing journal_ref at height {}",
            context.height
        )
    })?;
    let journal_blob_started_at = Instant::now();
    let journal_bytes = driver
        .execution_store
        .get_verified(journal_ref)
        .map_err(|err| {
            format!(
                "load execution checkpoint journal {} failed: {:?}",
                journal_ref, err
            )
        })?;
    blob_store_ms += journal_blob_started_at.elapsed();
    let journal_bytes_len = journal_bytes.len();
    let journal_decode_started_at = Instant::now();
    let journal =
        serde_cbor::from_slice::<RuntimeJournal>(journal_bytes.as_slice()).map_err(|err| {
            format!(
                "decode execution checkpoint journal failed at height {}: {}",
                context.height, err
            )
        })?;
    decode_ms += journal_decode_started_at.elapsed();
    let world_policy = driver.execution_world.release_security_policy().clone();
    let restored_resource_manifest = snapshot.chain_resource_manifest.clone();
    let restored_resource_delta = snapshot.latest_chain_resource_delta.clone();
    let rebuild_started_at = Instant::now();
    let mut restored_world =
        RuntimeWorld::from_snapshot(snapshot.clone(), journal).map_err(|err| {
            format!(
                "rebuild execution checkpoint world failed at height {}: {:?}",
                context.height, err
            )
        })?;
    restored_world.set_release_security_policy(world_policy);
    let rebuild_ms = rebuild_started_at.elapsed();
    let restored_commit_block_hash = restored_resource_delta
        .as_ref()
        .and_then(|delta| delta.commit_block_hash.as_deref())
        .or(restored_resource_manifest.created_at_block_hash.as_deref());
    let restored_resource_context = ChainResourceDerivationContext {
        world_id: restored_resource_manifest.world_id.as_str(),
        chain_id: restored_resource_manifest.chain_id.as_str(),
        genesis_ref: restored_resource_manifest.genesis_ref.as_deref(),
        created_at_height: restored_resource_manifest.created_at_height,
        manifest_height: restored_resource_manifest.manifest_height,
        commit_block_hash: restored_commit_block_hash,
        tick: restored_world.state().time,
    };
    let record = ExecutionBridgeRecord {
        schema_version: EXECUTION_BRIDGE_RECORD_SCHEMA_V3,
        world_id: context.world_id.clone(),
        height: context.height,
        node_block_hash: Some(context.node_block_hash.clone()),
        prev_node_block_hash: None,
        proposer_id: None,
        action_root: None,
        execution_block_hash: context.execution_block_hash.clone(),
        execution_state_root: context.execution_state_root.clone(),
        journal_len: snapshot.journal_len,
        latest_state_ref: Some(manifest.latest_state_ref.clone()),
        snapshot_ref: manifest.snapshot_ref.clone(),
        journal_ref: manifest.journal_ref.clone(),
        commit_log_ref: None,
        checkpoint_ref: Some(execution_checkpoint_manifest_rel_path(context.height)),
        external_effect_ref: None,
        world_head_proof_ref: None,
        world_head_proof_hash: None,
        simulator_mirror: None,
        timestamp_ms: context.committed_at_unix_ms,
    };
    let previous_execution_world = driver.execution_world.clone();
    let previous_state = driver.state.clone();
    let mut transaction = CheckpointInstallTransaction::prepare(driver, context.height)?;
    transaction.persist(driver.records_dir.as_path())?;
    let mut persist_ms = Duration::default();
    let install_result = (|| -> Result<Duration, String> {
        let world_persist_started_at = Instant::now();
        persist_execution_world_with_chain_resource_context(
            driver.world_dir.as_path(),
            &restored_world,
            restored_resource_context,
            restored_resource_manifest.world_config_hash.as_str(),
            restored_resource_manifest
                .generation_algorithm_hash
                .as_str(),
        )?;
        persist_ms += world_persist_started_at.elapsed();
        driver.execution_world = restored_world;
        let manifest_persist_started_at = Instant::now();
        persist_execution_checkpoint_manifest(driver.records_dir.as_path(), &manifest)?;
        persist_ms += manifest_persist_started_at.elapsed();
        let record_persist_started_at = Instant::now();
        persist_execution_bridge_record(driver.records_dir.as_path(), &record)?;
        persist_ms += record_persist_started_at.elapsed();
        driver.state.last_applied_committed_height = context.height;
        driver.state.last_execution_block_hash = Some(context.execution_block_hash.clone());
        driver.state.last_execution_state_root = Some(context.execution_state_root.clone());
        driver.state.last_node_block_hash = Some(context.node_block_hash.clone());
        let state_persist_started_at = Instant::now();
        persist_execution_bridge_state(driver.state_path.as_path(), &driver.state)?;
        persist_ms += state_persist_started_at.elapsed();
        transaction.phase = CheckpointInstallTransactionPhase::Committed;
        transaction.persist(driver.records_dir.as_path())?;
        let retention_started_at = Instant::now();
        run_execution_bridge_retention_maintenance(
            driver.records_dir.as_path(),
            &driver.execution_store,
            driver.hot_window_heights,
        )?;
        Ok(retention_started_at.elapsed())
    })();
    let retention_ms = match install_result {
        Ok(value) => value,
        Err(err) => {
            driver.execution_world = previous_execution_world;
            driver.state = previous_state;
            if let Err(rollback_err) = transaction
                .restore()
                .and_then(|_| remove_checkpoint_install_transaction(driver.records_dir.as_path()))
            {
                return Err(format!(
                    "checkpoint install failed: {err}; transaction rollback failed: {rollback_err}"
                ));
            }
            return Err(err);
        }
    };
    remove_checkpoint_install_transaction(driver.records_dir.as_path())?;

    emit_checkpoint_bundle_install_complete(CheckpointInstallObservation {
        world_id: context.world_id.as_str(),
        height: context.height,
        checkpoint_id: manifest.checkpoint_id.as_str(),
        blob_count: bundle_blob_count,
        bundle_bytes,
        pinned_ref_count: manifest.pinned_refs.len(),
        snapshot_bytes: snapshot_bytes_len,
        journal_bytes: journal_bytes_len,
        blob_store_ms,
        pin_check_ms,
        decode_ms,
        rebuild_ms,
        persist_ms,
        retention_ms,
        total_ms: install_started_at.elapsed(),
    });

    Ok(NodeExecutionCommitResult {
        execution_height: context.height,
        execution_block_hash: context.execution_block_hash,
        execution_state_root: context.execution_state_root,
    })
}
