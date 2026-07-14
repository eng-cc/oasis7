use std::fs;
use std::time::{Duration, Instant};

use crate::release_security_policy_for_storage_profile;
use oasis7::consensus_action_payload::{
    ConsensusActionPayloadBody, decode_consensus_action_payload,
};
use oasis7::runtime::{
    BlobStore, ChainResourceDerivationContext, Journal as RuntimeJournal, LocalCasStore,
    RuntimeCommittedTickContext, Snapshot as RuntimeSnapshot, World as RuntimeWorld, blake3_hex,
};
use oasis7::simulator::{
    Action as SimulatorAction, ActionSubmitter, WorldEventKind, WorldJournal as SimulatorJournal,
    WorldKernel, WorldSnapshot as SimulatorSnapshot,
};
use oasis7_node::{
    EXECUTION_MISSING_PREDECESSOR_RECORD_SIGNATURE, NodeExecutionCheckpointBlob,
    NodeExecutionCheckpointBundle, NodeExecutionCheckpointInstallContext,
    NodeExecutionCommitContext, NodeExecutionCommitResult, NodeExecutionHook,
    compute_consensus_action_root,
};
use oasis7_proto::storage_profile::StorageProfileConfig;
use oasis7_wasm_abi::ModuleSandbox;
use oasis7_wasm_executor::{WasmExecutor, WasmExecutorConfig};
use serde::Serialize;

use super::checkpoint::{
    begin_execution_bridge_retention_transaction, complete_execution_bridge_retention_transaction,
    execution_bridge_record_path, execution_checkpoint_root_dir,
    fail_execution_bridge_retention_transaction, load_execution_bridge_record,
    load_execution_checkpoint_manifest, maybe_persist_execution_checkpoint_for_record,
    persist_execution_bridge_record, persist_execution_bridge_record_only,
    promote_interrupted_execution_bridge_retention,
    run_execution_bridge_incremental_retention_maintenance,
    run_execution_bridge_retention_maintenance,
};
use super::driver_observability::{
    CommitObservation, RestoreObservation, SimulatorMirrorCommitObservation,
    emit_commit_observation, emit_stale_height_restore_complete, emit_stale_height_restore_start,
    execution_record_recovery_ref_count,
};
pub(crate) use super::driver_persistence::load_execution_world;
pub(crate) use super::driver_persistence::{
    execution_world_persistence_files_missing, load_execution_bridge_state,
    load_execution_world_with_policy, persist_execution_bridge_state, persist_execution_world,
    persist_execution_world_with_chain_resource_context,
    remove_partial_execution_world_persistence_files,
};
use super::external_effect::{
    build_execution_external_effect_materialization,
    persist_execution_external_effect_materialization,
};
pub(crate) use super::simulator_mirror::simulator_world_dir_from_execution_world_dir;
use super::simulator_mirror::{load_simulator_execution_world, persist_simulator_execution_world};
use super::{
    ExecutionBridgeRecord, ExecutionBridgeState, ExecutionSimulatorMirrorRecord,
    persist_world_head_proof_for_record,
};
use crate::{
    EXECUTION_BRIDGE_RETENTION_DEGRADED_MARKER, EXECUTION_BRIDGE_RETENTION_IN_PROGRESS_MARKER,
};

#[derive(Debug, Clone, Serialize)]
pub(super) struct ExecutionHashPayload<'a> {
    pub(super) world_id: &'a str,
    pub(super) height: u64,
    pub(super) prev_execution_block_hash: &'a str,
    pub(super) execution_state_root: &'a str,
    pub(super) journal_len: usize,
}

pub(super) fn execution_resource_created_at_height(height: u64) -> u64 {
    if height == 0 { 0 } else { 1 }
}

pub(super) fn execution_resource_context_hash(world_id: &str) -> String {
    format!("execution_bridge_runtime_context_v1:{world_id}")
}

pub(super) fn execution_resource_commit_hash(world_id: &str, height: u64) -> String {
    blake3_hex(format!("execution_bridge_resource_commit_v1:{world_id}:{height}").as_bytes())
}

pub(crate) struct NodeRuntimeExecutionDriver {
    pub(super) state_path: std::path::PathBuf,
    pub(super) world_dir: std::path::PathBuf,
    pub(super) records_dir: std::path::PathBuf,
    pub(super) simulator_world_dir: std::path::PathBuf,
    pub(super) execution_store: LocalCasStore,
    pub(super) state: ExecutionBridgeState,
    pub(super) execution_world: RuntimeWorld,
    pub(super) simulator_mirror: WorldKernel,
    pub(super) execution_sandbox: Box<dyn ModuleSandbox + Send>,
    pub(super) hot_window_heights: u64,
    pub(super) checkpoint_interval_heights: u64,
    pub(super) checkpoint_keep_latest: usize,
    pub(super) retention_reconcile_pending: bool,
    pub(super) retention_reconcile_next_height: Option<u64>,
}

impl NodeRuntimeExecutionDriver {
    pub(crate) fn new(
        state_path: std::path::PathBuf,
        world_dir: std::path::PathBuf,
        records_dir: std::path::PathBuf,
        storage_root: std::path::PathBuf,
    ) -> Result<Self, String> {
        Self::new_with_storage_profile(
            state_path,
            world_dir,
            records_dir,
            storage_root,
            &StorageProfileConfig::default(),
        )
    }

    pub(crate) fn new_with_storage_profile(
        state_path: std::path::PathBuf,
        world_dir: std::path::PathBuf,
        records_dir: std::path::PathBuf,
        storage_root: std::path::PathBuf,
        storage_profile: &StorageProfileConfig,
    ) -> Result<Self, String> {
        promote_interrupted_execution_bridge_retention(records_dir.as_path())?;
        let state = load_execution_bridge_state(state_path.as_path())?;
        let release_security_policy =
            release_security_policy_for_storage_profile(storage_profile.profile);
        remove_partial_execution_world_persistence_files(world_dir.as_path())?;
        let execution_world_bootstrap_required =
            execution_world_persistence_files_missing(world_dir.as_path());
        let execution_world =
            load_execution_world_with_policy(world_dir.as_path(), release_security_policy)?;
        let execution_sandbox: Box<dyn ModuleSandbox + Send> = Box::new(
            WasmExecutor::new(WasmExecutorConfig::default()).map_err(|err| err.to_string())?,
        );
        let mut driver = Self::new_with_sandbox(
            state_path,
            world_dir,
            records_dir,
            storage_root,
            state,
            execution_world,
            execution_sandbox,
            storage_profile.execution_hot_head_heights,
            storage_profile.execution_checkpoint_interval,
            storage_profile.execution_checkpoint_keep as usize,
        );
        remove_partial_execution_world_persistence_files(driver.simulator_world_dir.as_path())?;
        let simulator_world_bootstrap_required =
            execution_world_persistence_files_missing(driver.simulator_world_dir.as_path());
        driver.simulator_mirror =
            load_simulator_execution_world(driver.simulator_world_dir.as_path())?;
        if execution_world_bootstrap_required {
            persist_execution_world(driver.world_dir.as_path(), &driver.execution_world)?;
        }
        if simulator_world_bootstrap_required {
            persist_simulator_execution_world(
                driver.simulator_world_dir.as_path(),
                &driver.simulator_mirror,
                None,
            )?;
        }
        Ok(driver)
    }

    pub(crate) fn new_with_sandbox(
        state_path: std::path::PathBuf,
        world_dir: std::path::PathBuf,
        records_dir: std::path::PathBuf,
        storage_root: std::path::PathBuf,
        state: ExecutionBridgeState,
        execution_world: RuntimeWorld,
        execution_sandbox: Box<dyn ModuleSandbox + Send>,
        hot_window_heights: u64,
        checkpoint_interval_heights: u64,
        checkpoint_keep_latest: usize,
    ) -> Self {
        let simulator_world_dir = simulator_world_dir_from_execution_world_dir(world_dir.as_path());
        let retention_reconcile_pending = [
            EXECUTION_BRIDGE_RETENTION_DEGRADED_MARKER,
            EXECUTION_BRIDGE_RETENTION_IN_PROGRESS_MARKER,
        ]
        .iter()
        .any(|marker| records_dir.join(marker).exists());
        let retention_reconcile_next_height = retention_reconcile_pending
            .then(|| state.last_applied_committed_height.saturating_add(1));
        Self {
            state_path,
            world_dir,
            records_dir,
            simulator_world_dir,
            execution_store: LocalCasStore::new(storage_root),
            state,
            execution_world,
            simulator_mirror: WorldKernel::new(),
            execution_sandbox,
            hot_window_heights,
            checkpoint_interval_heights,
            checkpoint_keep_latest,
            retention_reconcile_pending,
            retention_reconcile_next_height,
        }
    }

    fn apply_simulator_actions(
        &mut self,
        context: &NodeExecutionCommitContext,
        simulator_actions: &[(SimulatorAction, ActionSubmitter)],
    ) -> Result<
        (
            Option<ExecutionSimulatorMirrorRecord>,
            SimulatorMirrorCommitObservation,
        ),
        String,
    > {
        let height = context.height;
        if simulator_actions.is_empty() {
            return Ok((None, SimulatorMirrorCommitObservation::default()));
        }

        let mut rejected_action_count = 0_usize;
        for (action, submitter) in simulator_actions {
            match submitter {
                ActionSubmitter::System => {
                    self.simulator_mirror
                        .submit_action_from_system(action.clone());
                }
                ActionSubmitter::Agent { agent_id } => {
                    self.simulator_mirror
                        .submit_action_from_agent(agent_id.clone(), action.clone());
                }
                ActionSubmitter::Player { player_id } => {
                    self.simulator_mirror
                        .submit_action_from_player(player_id.clone(), action.clone());
                }
            }

            let event = self.simulator_mirror.step().ok_or_else(|| {
                format!(
                    "execution driver simulator mirror step produced no event at height={height}"
                )
            })?;
            if matches!(event.kind, WorldEventKind::ActionRejected { .. }) {
                rejected_action_count = rejected_action_count.saturating_add(1);
            }
        }

        let resource_commit_hash = execution_resource_commit_hash(&context.world_id, height);
        let resource_context = ChainResourceDerivationContext {
            world_id: context.world_id.as_str(),
            chain_id: context.world_id.as_str(),
            genesis_ref: None,
            created_at_height: 0,
            manifest_height: context.height,
            commit_block_hash: Some(resource_commit_hash.as_str()),
            tick: self.simulator_mirror.time(),
        };
        let snapshot_value = self
            .simulator_mirror
            .snapshot_with_chain_resource_context(resource_context);
        let journal_value = self.simulator_mirror.journal_snapshot();
        let snapshot_bytes = super::to_cbor(snapshot_value)?;
        let journal_bytes = super::to_cbor(journal_value)?;

        let snapshot_ref = self
            .execution_store
            .put_bytes(snapshot_bytes.as_slice())
            .map_err(|err| {
                format!(
                    "execution driver simulator CAS snapshot put failed: {:?}",
                    err
                )
            })?;
        let journal_ref = self
            .execution_store
            .put_bytes(journal_bytes.as_slice())
            .map_err(|err| {
                format!(
                    "execution driver simulator CAS journal put failed: {:?}",
                    err
                )
            })?;
        let state_root = blake3_hex(snapshot_bytes.as_slice());

        let observation = SimulatorMirrorCommitObservation {
            action_count: simulator_actions.len(),
            rejected_action_count,
            snapshot_bytes: snapshot_bytes.len(),
            journal_bytes: journal_bytes.len(),
        };

        Ok((
            Some(ExecutionSimulatorMirrorRecord {
                action_count: simulator_actions.len(),
                rejected_action_count,
                journal_len: self.simulator_mirror.journal().len(),
                snapshot_ref,
                journal_ref,
                state_root,
            }),
            observation,
        ))
    }

    fn recover_runtime_journal_from_loaded_world(
        &self,
        snapshot: &RuntimeSnapshot,
        target_height: u64,
    ) -> Result<RuntimeJournal, String> {
        let loaded_journal = self.execution_world.journal().clone();
        if snapshot.journal_len > loaded_journal.len() {
            return Err(format!(
                "execution record at height {} missing journal_ref and loaded execution world only has {} events, need at least {}",
                target_height,
                loaded_journal.len(),
                snapshot.journal_len
            ));
        }

        let mut recovered = loaded_journal;
        recovered.events.truncate(snapshot.journal_len);
        let recovered_last_event_id = recovered.events.last().map(|event| event.id).unwrap_or(0);
        if recovered_last_event_id != snapshot.last_event_id {
            return Err(format!(
                "execution record at height {} missing journal_ref and loaded execution world journal prefix mismatches snapshot last_event_id expected={} actual={}",
                target_height, snapshot.last_event_id, recovered_last_event_id
            ));
        }

        Ok(recovered)
    }

    fn restore_execution_head_from_record(
        &mut self,
        expected_world_id: &str,
        target_height: u64,
    ) -> Result<bool, String> {
        let restore_started_at = Instant::now();
        let record_path = execution_bridge_record_path(self.records_dir.as_path(), target_height);
        if !record_path.exists() {
            return Ok(false);
        }

        let mut record = load_execution_bridge_record(record_path.as_path())?;
        if record.world_id != expected_world_id {
            return Err(format!(
                "execution driver stale-height restore world_id mismatch at height {}: expected={} actual={}",
                target_height, expected_world_id, record.world_id
            ));
        }
        let world_policy = self.execution_world.release_security_policy().clone();
        let snapshot_ref = record
            .recovery_snapshot_ref()
            .ok_or_else(|| {
                format!(
                    "execution record at height {} missing latest_state_ref",
                    target_height
                )
            })?
            .to_string();
        let pinned_ref_count = execution_record_recovery_ref_count(&record);
        let simulator_mirror_present = record.simulator_mirror.is_some();
        emit_stale_height_restore_start(
            expected_world_id,
            target_height,
            pinned_ref_count,
            simulator_mirror_present,
        );

        let snapshot_blob_started_at = Instant::now();
        let snapshot_bytes = self
            .execution_store
            .get_verified(snapshot_ref.as_str())
            .map_err(|err| {
                format!(
                    "execution driver restore snapshot ref {} failed at height {}: {:?}",
                    snapshot_ref, target_height, err
                )
            })?;
        let mut blob_store_ms = snapshot_blob_started_at.elapsed();
        let mut blob_count = 1_usize;
        let mut bundle_bytes = snapshot_bytes.len();
        let snapshot_bytes_len = snapshot_bytes.len();

        let runtime_decode_started_at = Instant::now();
        let snapshot = serde_cbor::from_slice::<RuntimeSnapshot>(snapshot_bytes.as_slice())
            .map_err(|err| {
                format!(
                    "execution driver decode runtime snapshot failed at height {}: {}",
                    target_height, err
                )
            })?;
        let journal_bytes_len;
        let (journal, recovered_journal_ref) = match record.journal_ref.as_deref() {
            Some(journal_ref) => {
                let journal_blob_started_at = Instant::now();
                let journal_bytes =
                    self.execution_store
                        .get_verified(journal_ref)
                        .map_err(|err| {
                            format!(
                                "execution driver restore journal ref {} failed at height {}: {:?}",
                                journal_ref, target_height, err
                            )
                        })?;
                blob_store_ms += journal_blob_started_at.elapsed();
                blob_count = blob_count.saturating_add(1);
                journal_bytes_len = journal_bytes.len();
                bundle_bytes = bundle_bytes.saturating_add(journal_bytes_len);
                let journal = serde_cbor::from_slice::<RuntimeJournal>(journal_bytes.as_slice())
                    .map_err(|err| {
                        format!(
                            "execution driver decode runtime journal failed at height {}: {}",
                            target_height, err
                        )
                    })?;
                (journal, None)
            }
            None => {
                let journal =
                    self.recover_runtime_journal_from_loaded_world(&snapshot, target_height)?;
                let journal_bytes = super::to_cbor(journal.clone())?;
                journal_bytes_len = journal_bytes.len();
                bundle_bytes = bundle_bytes.saturating_add(journal_bytes_len);
                let recovered_ref = self
                    .execution_store
                    .put_bytes(journal_bytes.as_slice())
                    .map_err(|err| {
                        format!(
                            "execution driver recover journal CAS put failed at height {}: {:?}",
                            target_height, err
                        )
                    })?;
                (journal, Some(recovered_ref))
            }
        };
        let decode_ms = runtime_decode_started_at.elapsed();

        let restored_resource_manifest = snapshot.chain_resource_manifest.clone();
        let restored_resource_delta = snapshot.latest_chain_resource_delta.clone();
        let runtime_rebuild_started_at = Instant::now();
        let mut restored_world = RuntimeWorld::from_snapshot(snapshot, journal).map_err(|err| {
            format!(
                "execution driver rebuild runtime world failed at height {}: {:?}",
                target_height, err
            )
        })?;
        restored_world.set_release_security_policy(world_policy);
        let mut rebuild_ms = runtime_rebuild_started_at.elapsed();
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
        let mut persist_ms = Duration::default();
        let world_persist_started_at = Instant::now();
        persist_execution_world_with_chain_resource_context(
            self.world_dir.as_path(),
            &restored_world,
            restored_resource_context,
            restored_resource_manifest.world_config_hash.as_str(),
            restored_resource_manifest
                .generation_algorithm_hash
                .as_str(),
        )?;
        persist_ms += world_persist_started_at.elapsed();
        self.execution_world = restored_world;

        let simulator_restore_started_at = Instant::now();
        let mut simulator_snapshot_bytes_len = 0_usize;
        let mut simulator_journal_bytes_len = 0_usize;
        if let Some(simulator_mirror) = record.simulator_mirror.as_ref() {
            let simulator_blob_started_at = Instant::now();
            let simulator_snapshot_bytes = self
                .execution_store
                .get_verified(simulator_mirror.snapshot_ref.as_str())
                .map_err(|err| {
                    format!(
                        "execution driver restore simulator snapshot ref {} failed at height {}: {:?}",
                        simulator_mirror.snapshot_ref, target_height, err
                    )
                })?;
            let simulator_journal_bytes = self
                .execution_store
                .get_verified(simulator_mirror.journal_ref.as_str())
                .map_err(|err| {
                    format!(
                        "execution driver restore simulator journal ref {} failed at height {}: {:?}",
                        simulator_mirror.journal_ref, target_height, err
                    )
                })?;
            blob_store_ms += simulator_blob_started_at.elapsed();
            blob_count = blob_count.saturating_add(2);
            simulator_snapshot_bytes_len = simulator_snapshot_bytes.len();
            simulator_journal_bytes_len = simulator_journal_bytes.len();
            bundle_bytes = bundle_bytes
                .saturating_add(simulator_snapshot_bytes_len)
                .saturating_add(simulator_journal_bytes_len);
            let simulator_snapshot =
                serde_cbor::from_slice::<SimulatorSnapshot>(simulator_snapshot_bytes.as_slice())
                    .map_err(|err| {
                        format!(
                            "execution driver decode simulator snapshot failed at height {}: {}",
                            target_height, err
                        )
                    })?;
            let simulator_journal =
                serde_cbor::from_slice::<SimulatorJournal>(simulator_journal_bytes.as_slice())
                    .map_err(|err| {
                        format!(
                            "execution driver decode simulator journal failed at height {}: {}",
                            target_height, err
                        )
                    })?;
            let simulator_rebuild_started_at = Instant::now();
            let restored_simulator =
                WorldKernel::from_snapshot(simulator_snapshot, simulator_journal).map_err(
                    |err| {
                        format!(
                            "execution driver rebuild simulator mirror failed at height {}: {:?}",
                            target_height, err
                        )
                    },
                )?;
            rebuild_ms += simulator_rebuild_started_at.elapsed();
            let simulator_persist_started_at = Instant::now();
            persist_simulator_execution_world(
                self.simulator_world_dir.as_path(),
                &restored_simulator,
                None,
            )?;
            persist_ms += simulator_persist_started_at.elapsed();
            self.simulator_mirror = restored_simulator;
        }
        let simulator_restore_ms = if simulator_mirror_present {
            simulator_restore_started_at.elapsed()
        } else {
            Duration::default()
        };

        if record.latest_state_ref.is_none()
            || record.snapshot_ref.is_none()
            || record.journal_ref.is_none()
        {
            if record.latest_state_ref.is_none() {
                record.latest_state_ref = Some(snapshot_ref.clone());
            }
            if record.snapshot_ref.is_none() {
                record.snapshot_ref = Some(snapshot_ref.clone());
            }
            if record.journal_ref.is_none() {
                record.journal_ref = recovered_journal_ref;
            }
            let record_persist_started_at = Instant::now();
            persist_execution_bridge_record_only(self.records_dir.as_path(), &record)?;
            persist_ms += record_persist_started_at.elapsed();
        }

        self.state.last_applied_committed_height = record.height;
        self.state.last_execution_block_hash = Some(record.execution_block_hash);
        self.state.last_execution_state_root = Some(record.execution_state_root);
        self.state.last_node_block_hash = record.node_block_hash;
        let state_persist_started_at = Instant::now();
        persist_execution_bridge_state(self.state_path.as_path(), &self.state)?;
        persist_ms += state_persist_started_at.elapsed();

        emit_stale_height_restore_complete(RestoreObservation {
            world_id: expected_world_id,
            height: target_height,
            pinned_ref_count,
            blob_count,
            bundle_bytes,
            snapshot_bytes: snapshot_bytes_len,
            journal_bytes: journal_bytes_len,
            simulator_mirror_present,
            simulator_snapshot_bytes: simulator_snapshot_bytes_len,
            simulator_journal_bytes: simulator_journal_bytes_len,
            blob_store_ms,
            decode_ms,
            rebuild_ms,
            simulator_restore_ms,
            persist_ms,
            total_ms: restore_started_at.elapsed(),
        });

        Ok(true)
    }
}

impl NodeExecutionHook for NodeRuntimeExecutionDriver {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        self.on_commit_with_expected(context, None, None)
    }

    fn on_commit_with_expected(
        &mut self,
        context: NodeExecutionCommitContext,
        expected_execution_block_hash: Option<&str>,
        expected_execution_state_root: Option<&str>,
    ) -> Result<NodeExecutionCommitResult, String> {
        if context.height < self.state.last_applied_committed_height {
            let stale_state_height = self.state.last_applied_committed_height;
            if !self
                .restore_execution_head_from_record(context.world_id.as_str(), context.height)?
            {
                return Err(format!(
                    "execution driver received stale height: context={} state={}",
                    context.height, stale_state_height
                ));
            }
        }
        if context.height == self.state.last_applied_committed_height {
            let execution_block_hash =
                self.state
                    .last_execution_block_hash
                    .clone()
                    .ok_or_else(|| {
                        "execution driver missing block hash for current height".to_string()
                    })?;
            let execution_state_root =
                self.state
                    .last_execution_state_root
                    .clone()
                    .ok_or_else(|| {
                        "execution driver missing state root for current height".to_string()
                    })?;
            return Ok(NodeExecutionCommitResult {
                execution_height: context.height,
                execution_block_hash,
                execution_state_root,
            });
        }
        let next_expected_height = self.state.last_applied_committed_height.saturating_add(1);
        if context.height != next_expected_height {
            let predecessor_height = context.height.saturating_sub(1);
            if predecessor_height == 0 {
                return Err(format!(
                    "execution driver received non-contiguous committed height: last_applied={} incoming={}",
                    self.state.last_applied_committed_height, context.height
                ));
            }
            if !self
                .restore_execution_head_from_record(context.world_id.as_str(), predecessor_height)?
            {
                return Err(format!(
                    "{}: last_applied={} incoming={} predecessor={}",
                    EXECUTION_MISSING_PREDECESSOR_RECORD_SIGNATURE,
                    self.state.last_applied_committed_height,
                    context.height,
                    predecessor_height
                ));
            }
            let restored_next_expected_height =
                self.state.last_applied_committed_height.saturating_add(1);
            if context.height != restored_next_expected_height {
                return Err(format!(
                    "execution driver restore failed to close committed height gap: restored_head={} incoming={}",
                    self.state.last_applied_committed_height, context.height
                ));
            }
        }

        let commit_started_at = Instant::now();
        let action_count = context.committed_actions.len();
        let computed_action_root =
            compute_consensus_action_root(context.committed_actions.as_slice())
                .map_err(|err| format!("execution driver compute action root failed: {err:?}"))?;
        if computed_action_root != context.action_root {
            return Err(format!(
                "execution driver action_root mismatch expected={} actual={}",
                computed_action_root, context.action_root
            ));
        }

        let external_effect =
            build_execution_external_effect_materialization(&self.execution_world, &context)?;

        let decode_started_at = Instant::now();
        let mut decoded_runtime_actions = Vec::with_capacity(context.committed_actions.len());
        let mut decoded_simulator_actions = Vec::with_capacity(context.committed_actions.len());
        for action in &context.committed_actions {
            match decode_consensus_action_payload(action.payload_cbor.as_slice()) {
                Ok(ConsensusActionPayloadBody::RuntimeAction { action: decoded }) => {
                    decoded_runtime_actions.push(decoded);
                }
                Ok(ConsensusActionPayloadBody::SimulatorAction { action, submitter }) => {
                    decoded_simulator_actions.push((action, submitter));
                }
                Err(err) => {
                    return Err(format!(
                        "execution driver decode committed action failed action_id={} err={}",
                        action.action_id, err
                    ));
                }
            }
        }
        let decode_ms = decode_started_at.elapsed();
        let runtime_action_count = decoded_runtime_actions.len();

        fs::create_dir_all(self.records_dir.as_path()).map_err(|err| {
            format!(
                "create execution records dir {} failed: {}",
                self.records_dir.display(),
                err
            )
        })?;

        let rollback_state = match (expected_execution_block_hash, expected_execution_state_root) {
            (Some(_), Some(_)) => {
                Some((self.execution_world.clone(), self.simulator_mirror.clone()))
            }
            _ => None,
        };
        let runtime_step_started_at = Instant::now();
        for action in decoded_runtime_actions {
            self.execution_world.submit_action(action);
        }
        let committed_tick_context = RuntimeCommittedTickContext {
            height: context.height,
            slot: context.slot,
            epoch: context.epoch,
            node_block_hash: String::new(),
            action_root: context.action_root.clone(),
            authority_node_id: context.node_id.clone(),
            committed_at_unix_ms: context.committed_at_unix_ms,
        };
        self.execution_world
            .step_with_modules_for_committed_context(
                &mut *self.execution_sandbox,
                &committed_tick_context,
            )
            .map_err(|err| {
                format!(
                    "execution driver world.step failed at height {}: {:?}",
                    context.height, err
                )
            })?;
        let runtime_step_ms = runtime_step_started_at.elapsed();
        let simulator_step_started_at = Instant::now();
        let (simulator_mirror, simulator_observation) =
            self.apply_simulator_actions(&context, decoded_simulator_actions.as_slice())?;
        let simulator_step_ms = simulator_step_started_at.elapsed();

        let runtime_resource_commit_hash =
            execution_resource_commit_hash(&context.world_id, context.height);
        let runtime_resource_context = ChainResourceDerivationContext {
            world_id: context.world_id.as_str(),
            chain_id: context.world_id.as_str(),
            genesis_ref: None,
            created_at_height: execution_resource_created_at_height(context.height),
            manifest_height: context.height,
            commit_block_hash: Some(runtime_resource_commit_hash.as_str()),
            tick: self.execution_world.state().time,
        };
        let runtime_resource_context_hash = execution_resource_context_hash(&context.world_id);
        let snapshot_value = self.execution_world.snapshot_with_chain_resource_context(
            runtime_resource_context,
            runtime_resource_context_hash.clone(),
            runtime_resource_context_hash.clone(),
        );
        let journal_value = self.execution_world.journal().clone();
        let serialize_started_at = Instant::now();
        let snapshot_bytes = super::to_cbor(snapshot_value)?;
        let journal_bytes = super::to_cbor(journal_value)?;
        let serialize_ms = serialize_started_at.elapsed();
        let snapshot_bytes_len = snapshot_bytes.len();
        let journal_bytes_len = journal_bytes.len();

        let cas_put_started_at = Instant::now();
        let snapshot_ref = self
            .execution_store
            .put_bytes(snapshot_bytes.as_slice())
            .map_err(|err| format!("execution driver CAS snapshot put failed: {:?}", err))?;
        let journal_ref = self
            .execution_store
            .put_bytes(journal_bytes.as_slice())
            .map_err(|err| format!("execution driver CAS journal put failed: {:?}", err))?;
        let mut cas_put_ms = cas_put_started_at.elapsed();

        let execution_state_root = blake3_hex(snapshot_bytes.as_slice());
        let prev_execution_block_hash = self
            .state
            .last_execution_block_hash
            .clone()
            .unwrap_or_else(|| "genesis".to_string());
        let hash_payload = ExecutionHashPayload {
            world_id: context.world_id.as_str(),
            height: context.height,
            prev_execution_block_hash: prev_execution_block_hash.as_str(),
            execution_state_root: execution_state_root.as_str(),
            journal_len: self.execution_world.journal().len(),
        };
        let execution_block_hash = blake3_hex(super::to_cbor(hash_payload)?.as_slice());
        if let (Some(expected_block_hash), Some(expected_state_root)) =
            (expected_execution_block_hash, expected_execution_state_root)
        {
            if execution_block_hash != expected_block_hash
                || execution_state_root != expected_state_root
            {
                if let Some((previous_execution_world, previous_simulator_mirror)) = rollback_state
                {
                    self.execution_world = previous_execution_world;
                    self.simulator_mirror = previous_simulator_mirror;
                }
                return Err(format!(
                    "execution driver peer mismatch at height {}: local_block={} peer_block={} local_state={} peer_state={}",
                    context.height,
                    execution_block_hash,
                    expected_block_hash,
                    execution_state_root,
                    expected_state_root
                ));
            }
        }
        let mut simulator_persist_ms = Duration::default();
        if simulator_mirror.is_some() {
            let resource_context = ChainResourceDerivationContext {
                world_id: context.world_id.as_str(),
                chain_id: context.world_id.as_str(),
                genesis_ref: None,
                created_at_height: 0,
                manifest_height: context.height,
                commit_block_hash: None,
                tick: self.simulator_mirror.time(),
            };
            let simulator_persist_started_at = Instant::now();
            persist_simulator_execution_world(
                self.simulator_world_dir.as_path(),
                &self.simulator_mirror,
                Some(resource_context),
            )?;
            simulator_persist_ms = simulator_persist_started_at.elapsed();
        }
        let external_effect_started_at = Instant::now();
        let external_effect_ref = persist_execution_external_effect_materialization(
            &self.execution_store,
            &external_effect,
        )?;
        cas_put_ms += external_effect_started_at.elapsed();
        let prev_node_block_hash = self.state.last_node_block_hash.clone();
        let node_block_hash = Some(context.node_block_hash.clone());

        let mut record = ExecutionBridgeRecord::new_v3(
            context.world_id.clone(),
            context.height,
            node_block_hash.clone(),
            prev_node_block_hash,
            context.proposer_id.clone(),
            context.action_root.clone(),
            execution_block_hash.clone(),
            execution_state_root.clone(),
            self.execution_world.journal().len(),
            snapshot_ref,
            journal_ref,
            Some(external_effect_ref),
            simulator_mirror,
            context.committed_at_unix_ms,
        );
        let checkpoint_started_at = Instant::now();
        record.checkpoint_ref = maybe_persist_execution_checkpoint_for_record(
            self.records_dir.as_path(),
            &record,
            self.checkpoint_interval_heights,
            self.checkpoint_keep_latest,
        )?;
        let checkpoint_ms = checkpoint_started_at.elapsed();
        let checkpoint_manifest = record
            .checkpoint_ref
            .as_deref()
            .map(|checkpoint_ref| {
                load_execution_checkpoint_manifest(
                    execution_checkpoint_root_dir(self.records_dir.as_path())
                        .join(checkpoint_ref)
                        .as_path(),
                )
            })
            .transpose()?;
        let world_head_proof_started_at = Instant::now();
        persist_world_head_proof_for_record(
            &self.execution_store,
            &mut record,
            checkpoint_manifest.as_ref(),
        )?;
        let world_head_proof_ms = world_head_proof_started_at.elapsed();
        let record_persist_started_at = Instant::now();
        begin_execution_bridge_retention_transaction(self.records_dir.as_path())?;
        persist_execution_bridge_record(self.records_dir.as_path(), &record)?;
        let record_persist_ms = record_persist_started_at.elapsed();

        self.state.last_applied_committed_height = context.height;
        self.state.last_execution_block_hash = Some(execution_block_hash);
        self.state.last_execution_state_root = Some(execution_state_root);
        self.state.last_node_block_hash = node_block_hash;

        let state_persist_started_at = Instant::now();
        persist_execution_bridge_state(self.state_path.as_path(), &self.state)?;
        let state_persist_ms = state_persist_started_at.elapsed();
        let world_persist_started_at = Instant::now();
        persist_execution_world_with_chain_resource_context(
            self.world_dir.as_path(),
            &self.execution_world,
            runtime_resource_context,
            runtime_resource_context_hash.as_str(),
            runtime_resource_context_hash.as_str(),
        )?;
        let world_persist_ms = world_persist_started_at.elapsed();
        let persist_world_ms = state_persist_ms + world_persist_ms;
        let retention_started_at = Instant::now();
        let reconcile_due = self.retention_reconcile_pending
            && self
                .retention_reconcile_next_height
                .is_none_or(|height| context.height >= height);
        let retention_result = if reconcile_due {
            run_execution_bridge_retention_maintenance(
                self.records_dir.as_path(),
                &self.execution_store,
                self.hot_window_heights,
            )
        } else {
            run_execution_bridge_incremental_retention_maintenance(
                self.records_dir.as_path(),
                &self.execution_store,
                &record,
                self.hot_window_heights,
                self.checkpoint_interval_heights,
                self.checkpoint_keep_latest,
            )
        };
        let retention_result = retention_result.and_then(|freed_bytes| {
            complete_execution_bridge_retention_transaction(self.records_dir.as_path())?;
            Ok(freed_bytes)
        });
        if let Err(err) = retention_result {
            let was_pending = self.retention_reconcile_pending;
            self.retention_reconcile_pending = true;
            if reconcile_due || !was_pending {
                self.retention_reconcile_next_height = Some(
                    context
                        .height
                        .saturating_add(self.checkpoint_interval_heights.max(1)),
                );
            }
            if let Err(marker_err) =
                fail_execution_bridge_retention_transaction(self.records_dir.as_path())
            {
                oasis7::observability::emit_stderr_or_event(
                    tracing::Level::ERROR,
                    format!(
                        "execution driver failed to publish retention degradation at height {}: {}",
                        context.height, marker_err
                    )
                    .as_str(),
                    "execution bridge retention degradation publication failed",
                );
            }
            oasis7::observability::emit_stderr_or_event(
                tracing::Level::WARN,
                format!(
                    "execution driver retention pin-set sync failed at height {}: {}",
                    context.height, err
                )
                .as_str(),
                "execution bridge retention maintenance failed",
            );
        } else if reconcile_due || !self.retention_reconcile_pending {
            self.retention_reconcile_pending = false;
            self.retention_reconcile_next_height = None;
        }
        let retention_ms = retention_started_at.elapsed();
        super::record_execution_bridge_module_tick_routing_metrics(
            self.execution_world.module_tick_routing_metrics_snapshot(),
        );
        emit_commit_observation(CommitObservation {
            world_id: context.world_id.as_str(),
            height: context.height,
            action_count,
            runtime_action_count,
            simulator: simulator_observation,
            snapshot_bytes: snapshot_bytes_len,
            journal_bytes: journal_bytes_len,
            decode_ms,
            runtime_step_ms,
            simulator_step_ms,
            serialize_ms,
            cas_put_ms,
            simulator_persist_ms,
            world_head_proof_ms,
            record_persist_ms,
            persist_world_ms,
            checkpoint_ms,
            retention_ms,
            total_ms: commit_started_at.elapsed(),
        });

        Ok(NodeExecutionCommitResult {
            execution_height: context.height,
            execution_block_hash: self
                .state
                .last_execution_block_hash
                .clone()
                .ok_or_else(|| "execution driver missing execution_block_hash".to_string())?,
            execution_state_root: self
                .state
                .last_execution_state_root
                .clone()
                .ok_or_else(|| "execution driver missing execution_state_root".to_string())?,
        })
    }

    fn restore_to_height(&mut self, world_id: &str, height: u64) -> Result<bool, String> {
        self.restore_execution_head_from_record(world_id, height)
    }

    fn export_checkpoint_bundle(
        &mut self,
        height: u64,
    ) -> Result<Option<NodeExecutionCheckpointBundle>, String> {
        let record_path = execution_bridge_record_path(self.records_dir.as_path(), height);
        if !record_path.exists() {
            return Ok(None);
        }
        let record = load_execution_bridge_record(record_path.as_path())?;
        let Some(checkpoint_ref) = record.checkpoint_ref.as_deref() else {
            return Ok(None);
        };
        let manifest_path =
            execution_checkpoint_root_dir(self.records_dir.as_path()).join(checkpoint_ref);
        let manifest_json = fs::read(manifest_path.as_path()).map_err(|err| {
            format!(
                "read execution checkpoint manifest {} failed: {}",
                manifest_path.display(),
                err
            )
        })?;
        let manifest = load_execution_checkpoint_manifest(manifest_path.as_path())?;
        if manifest.height != height
            || manifest.execution_block_hash != record.execution_block_hash
            || manifest.execution_state_root != record.execution_state_root
        {
            return Err(format!(
                "execution checkpoint manifest mismatch at height {}",
                height
            ));
        }

        let mut blobs = Vec::with_capacity(manifest.pinned_refs.len());
        for content_hash in &manifest.pinned_refs {
            let bytes = self
                .execution_store
                .get_verified(content_hash.as_str())
                .map_err(|err| {
                    format!(
                        "read execution checkpoint blob {} failed at height {}: {:?}",
                        content_hash, height, err
                    )
                })?;
            blobs.push(NodeExecutionCheckpointBlob {
                content_hash: content_hash.clone(),
                bytes,
            });
        }

        Ok(Some(NodeExecutionCheckpointBundle {
            height,
            execution_block_hash: manifest.execution_block_hash,
            execution_state_root: manifest.execution_state_root,
            manifest_json,
            blobs,
        }))
    }

    fn install_checkpoint_bundle(
        &mut self,
        context: NodeExecutionCheckpointInstallContext,
        bundle: NodeExecutionCheckpointBundle,
    ) -> Result<NodeExecutionCommitResult, String> {
        super::driver_checkpoint_install::install_checkpoint_bundle(self, context, bundle)
    }
}
