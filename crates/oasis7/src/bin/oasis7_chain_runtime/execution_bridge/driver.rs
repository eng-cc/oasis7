use std::fs;
use std::path::Path;

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
    NodeExecutionCommitContext, NodeExecutionCommitResult, NodeExecutionHook, NodeSnapshot,
    compute_consensus_action_root,
};
use oasis7_proto::storage_profile::StorageProfileConfig;
use oasis7_wasm_abi::ModuleSandbox;
use oasis7_wasm_executor::{WasmExecutor, WasmExecutorConfig};
use serde::Serialize;

use super::checkpoint::{
    execution_bridge_record_path, execution_checkpoint_manifest_rel_path,
    execution_checkpoint_root_dir, load_execution_bridge_record,
    load_execution_checkpoint_manifest, maybe_persist_execution_checkpoint_for_record,
    persist_execution_bridge_record, persist_execution_bridge_record_only,
    persist_execution_checkpoint_manifest, run_execution_bridge_retention_maintenance,
};
pub(crate) use super::driver_persistence::{
    load_execution_bridge_state, load_execution_world_with_policy, persist_execution_bridge_state,
    persist_execution_world_with_chain_resource_context,
};
#[cfg(test)]
pub(crate) use super::driver_persistence::{load_execution_world, persist_execution_world};
use super::external_effect::{
    build_execution_external_effect_materialization,
    persist_execution_external_effect_materialization,
};
pub(crate) use super::simulator_mirror::simulator_world_dir_from_execution_world_dir;
use super::simulator_mirror::{load_simulator_execution_world, persist_simulator_execution_world};
use super::{
    EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_INTERVAL_HEIGHTS,
    EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_KEEP_LATEST, EXECUTION_BRIDGE_DEFAULT_HOT_WINDOW_HEIGHTS,
    ExecutionBridgeRecord, ExecutionBridgeState, ExecutionSimulatorMirrorRecord,
};

#[derive(Debug, Clone, Serialize)]
struct ExecutionHashPayload<'a> {
    world_id: &'a str,
    height: u64,
    prev_execution_block_hash: &'a str,
    execution_state_root: &'a str,
    journal_len: usize,
}

fn execution_resource_created_at_height(height: u64) -> u64 {
    if height == 0 { 0 } else { 1 }
}

fn execution_resource_context_hash(world_id: &str) -> String {
    format!("execution_bridge_runtime_context_v1:{world_id}")
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
}

fn remove_partial_execution_world_persistence_files(world_dir: &Path) -> Result<(), String> {
    let snapshot_path = world_dir.join("snapshot.json");
    let journal_path = world_dir.join("journal.json");
    let snapshot_exists = snapshot_path.exists();
    let journal_exists = journal_path.exists();
    if snapshot_exists == journal_exists {
        return Ok(());
    }
    for path in [snapshot_path, journal_path] {
        match fs::remove_file(path.as_path()) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(format!(
                    "remove partial execution world persistence file {} failed: {}",
                    path.display(),
                    err
                ));
            }
        }
    }
    Ok(())
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
        let state = load_execution_bridge_state(state_path.as_path())?;
        let release_security_policy =
            release_security_policy_for_storage_profile(storage_profile.profile);
        remove_partial_execution_world_persistence_files(world_dir.as_path())?;
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
        driver.simulator_mirror =
            load_simulator_execution_world(driver.simulator_world_dir.as_path())?;
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
        }
    }

    fn apply_simulator_actions(
        &mut self,
        context: &NodeExecutionCommitContext,
        simulator_actions: &[(SimulatorAction, ActionSubmitter)],
    ) -> Result<Option<ExecutionSimulatorMirrorRecord>, String> {
        let height = context.height;
        if simulator_actions.is_empty() {
            return Ok(None);
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

        let resource_context = ChainResourceDerivationContext {
            world_id: context.world_id.as_str(),
            chain_id: context.world_id.as_str(),
            genesis_ref: None,
            created_at_height: 0,
            manifest_height: context.height,
            commit_block_hash: None,
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

        Ok(Some(ExecutionSimulatorMirrorRecord {
            action_count: simulator_actions.len(),
            rejected_action_count,
            journal_len: self.simulator_mirror.journal().len(),
            snapshot_ref,
            journal_ref,
            state_root,
        }))
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

        let snapshot_bytes = self
            .execution_store
            .get_verified(snapshot_ref.as_str())
            .map_err(|err| {
                format!(
                    "execution driver restore snapshot ref {} failed at height {}: {:?}",
                    snapshot_ref, target_height, err
                )
            })?;
        let snapshot = serde_cbor::from_slice::<RuntimeSnapshot>(snapshot_bytes.as_slice())
            .map_err(|err| {
                format!(
                    "execution driver decode runtime snapshot failed at height {}: {}",
                    target_height, err
                )
            })?;
        let (journal, recovered_journal_ref) = match record.journal_ref.as_deref() {
            Some(journal_ref) => {
                let journal_bytes =
                    self.execution_store
                        .get_verified(journal_ref)
                        .map_err(|err| {
                            format!(
                                "execution driver restore journal ref {} failed at height {}: {:?}",
                                journal_ref, target_height, err
                            )
                        })?;
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
        let restored_resource_manifest = snapshot.chain_resource_manifest.clone();
        let restored_resource_delta = snapshot.latest_chain_resource_delta.clone();
        let mut restored_world = RuntimeWorld::from_snapshot(snapshot, journal).map_err(|err| {
            format!(
                "execution driver rebuild runtime world failed at height {}: {:?}",
                target_height, err
            )
        })?;
        restored_world.set_release_security_policy(world_policy);
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
        persist_execution_world_with_chain_resource_context(
            self.world_dir.as_path(),
            &restored_world,
            restored_resource_context,
            restored_resource_manifest.world_config_hash.as_str(),
            restored_resource_manifest
                .generation_algorithm_hash
                .as_str(),
        )?;
        self.execution_world = restored_world;

        if let Some(simulator_mirror) = record.simulator_mirror.as_ref() {
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
            let restored_simulator =
                WorldKernel::from_snapshot(simulator_snapshot, simulator_journal).map_err(
                    |err| {
                        format!(
                            "execution driver rebuild simulator mirror failed at height {}: {:?}",
                            target_height, err
                        )
                    },
                )?;
            persist_simulator_execution_world(
                self.simulator_world_dir.as_path(),
                &restored_simulator,
                None,
            )?;
            self.simulator_mirror = restored_simulator;
        }

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
            persist_execution_bridge_record_only(self.records_dir.as_path(), &record)?;
        }

        self.state.last_applied_committed_height = record.height;
        self.state.last_execution_block_hash = Some(record.execution_block_hash);
        self.state.last_execution_state_root = Some(record.execution_state_root);
        self.state.last_node_block_hash = record.node_block_hash;
        persist_execution_bridge_state(self.state_path.as_path(), &self.state)?;

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

        fs::create_dir_all(self.records_dir.as_path()).map_err(|err| {
            format!(
                "create execution records dir {} failed: {}",
                self.records_dir.display(),
                err
            )
        })?;

        let previous_execution_world = self.execution_world.clone();
        let previous_simulator_mirror = self.simulator_mirror.clone();
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
        let simulator_mirror =
            self.apply_simulator_actions(&context, decoded_simulator_actions.as_slice())?;

        let runtime_resource_context = ChainResourceDerivationContext {
            world_id: context.world_id.as_str(),
            chain_id: context.world_id.as_str(),
            genesis_ref: None,
            created_at_height: execution_resource_created_at_height(context.height),
            manifest_height: context.height,
            commit_block_hash: None,
            tick: self.execution_world.state().time,
        };
        let runtime_resource_context_hash = execution_resource_context_hash(&context.world_id);
        let snapshot_value = self.execution_world.snapshot_with_chain_resource_context(
            runtime_resource_context,
            runtime_resource_context_hash.clone(),
            runtime_resource_context_hash.clone(),
        );
        let journal_value = self.execution_world.journal().clone();
        let snapshot_bytes = super::to_cbor(snapshot_value)?;
        let journal_bytes = super::to_cbor(journal_value)?;

        let snapshot_ref = self
            .execution_store
            .put_bytes(snapshot_bytes.as_slice())
            .map_err(|err| format!("execution driver CAS snapshot put failed: {:?}", err))?;
        let journal_ref = self
            .execution_store
            .put_bytes(journal_bytes.as_slice())
            .map_err(|err| format!("execution driver CAS journal put failed: {:?}", err))?;

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
                self.execution_world = previous_execution_world;
                self.simulator_mirror = previous_simulator_mirror;
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
            persist_simulator_execution_world(
                self.simulator_world_dir.as_path(),
                &self.simulator_mirror,
                Some(resource_context),
            )?;
        }
        let external_effect_ref = persist_execution_external_effect_materialization(
            &self.execution_store,
            &external_effect,
        )?;
        let node_block_hash = Some(context.node_block_hash.clone());

        let mut record = ExecutionBridgeRecord::new_v2(
            context.world_id.clone(),
            context.height,
            node_block_hash.clone(),
            execution_block_hash.clone(),
            execution_state_root.clone(),
            self.execution_world.journal().len(),
            snapshot_ref,
            journal_ref,
            Some(external_effect_ref),
            simulator_mirror,
            context.committed_at_unix_ms,
        );
        record.checkpoint_ref = maybe_persist_execution_checkpoint_for_record(
            self.records_dir.as_path(),
            &record,
            self.checkpoint_interval_heights,
            self.checkpoint_keep_latest,
        )?;
        persist_execution_bridge_record(self.records_dir.as_path(), &record)?;

        self.state.last_applied_committed_height = context.height;
        self.state.last_execution_block_hash = Some(execution_block_hash);
        self.state.last_execution_state_root = Some(execution_state_root);
        self.state.last_node_block_hash = node_block_hash;

        persist_execution_bridge_state(self.state_path.as_path(), &self.state)?;
        persist_execution_world_with_chain_resource_context(
            self.world_dir.as_path(),
            &self.execution_world,
            runtime_resource_context,
            runtime_resource_context_hash.as_str(),
            runtime_resource_context_hash.as_str(),
        )?;
        if let Err(err) = run_execution_bridge_retention_maintenance(
            self.records_dir.as_path(),
            &self.execution_store,
            self.hot_window_heights,
        ) {
            oasis7::observability::emit_stderr_or_event(
                tracing::Level::WARN,
                format!(
                    "execution driver retention pin-set sync failed at height {}: {}",
                    context.height, err
                )
                .as_str(),
                "execution bridge retention maintenance failed",
            );
        }

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
        if bundle.height != context.height
            || bundle.execution_block_hash != context.execution_block_hash
            || bundle.execution_state_root != context.execution_state_root
        {
            return Err(format!(
                "execution checkpoint bundle does not match install context height={}",
                context.height
            ));
        }
        let manifest =
            serde_json::from_slice::<super::ExecutionCheckpointManifest>(&bundle.manifest_json)
                .map_err(|err| {
                    format!(
                        "decode execution checkpoint manifest failed at height {}: {}",
                        context.height, err
                    )
                })?;
        manifest.validate()?;
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

        for blob in &bundle.blobs {
            let actual = blake3_hex(blob.bytes.as_slice());
            if actual != blob.content_hash {
                return Err(format!(
                    "execution checkpoint blob hash mismatch expected={} actual={}",
                    blob.content_hash, actual
                ));
            }
            self.execution_store
                .put(blob.content_hash.as_str(), blob.bytes.as_slice())
                .map_err(|err| {
                    format!(
                        "store execution checkpoint blob {} failed: {:?}",
                        blob.content_hash, err
                    )
                })?;
        }
        for content_hash in &manifest.pinned_refs {
            if !self
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

        let snapshot_bytes = self
            .execution_store
            .get_verified(manifest.latest_state_ref.as_str())
            .map_err(|err| {
                format!(
                    "load execution checkpoint snapshot {} failed: {:?}",
                    manifest.latest_state_ref, err
                )
            })?;
        let snapshot = serde_cbor::from_slice::<RuntimeSnapshot>(snapshot_bytes.as_slice())
            .map_err(|err| {
                format!(
                    "decode execution checkpoint snapshot failed at height {}: {}",
                    context.height, err
                )
            })?;
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
        let journal_bytes = self
            .execution_store
            .get_verified(journal_ref)
            .map_err(|err| {
                format!(
                    "load execution checkpoint journal {} failed: {:?}",
                    journal_ref, err
                )
            })?;
        let journal =
            serde_cbor::from_slice::<RuntimeJournal>(journal_bytes.as_slice()).map_err(|err| {
                format!(
                    "decode execution checkpoint journal failed at height {}: {}",
                    context.height, err
                )
            })?;
        let world_policy = self.execution_world.release_security_policy().clone();
        let restored_resource_manifest = snapshot.chain_resource_manifest.clone();
        let restored_resource_delta = snapshot.latest_chain_resource_delta.clone();
        let mut restored_world =
            RuntimeWorld::from_snapshot(snapshot.clone(), journal).map_err(|err| {
                format!(
                    "rebuild execution checkpoint world failed at height {}: {:?}",
                    context.height, err
                )
            })?;
        restored_world.set_release_security_policy(world_policy);
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
        persist_execution_world_with_chain_resource_context(
            self.world_dir.as_path(),
            &restored_world,
            restored_resource_context,
            restored_resource_manifest.world_config_hash.as_str(),
            restored_resource_manifest
                .generation_algorithm_hash
                .as_str(),
        )?;
        self.execution_world = restored_world;
        persist_execution_checkpoint_manifest(self.records_dir.as_path(), &manifest)?;

        let record = ExecutionBridgeRecord {
            schema_version: super::EXECUTION_BRIDGE_RECORD_SCHEMA_V2,
            world_id: context.world_id.clone(),
            height: context.height,
            node_block_hash: Some(context.node_block_hash.clone()),
            execution_block_hash: context.execution_block_hash.clone(),
            execution_state_root: context.execution_state_root.clone(),
            journal_len: snapshot.journal_len,
            latest_state_ref: Some(manifest.latest_state_ref.clone()),
            snapshot_ref: manifest.snapshot_ref.clone(),
            journal_ref: manifest.journal_ref.clone(),
            commit_log_ref: None,
            checkpoint_ref: Some(execution_checkpoint_manifest_rel_path(context.height)),
            external_effect_ref: None,
            simulator_mirror: None,
            timestamp_ms: context.committed_at_unix_ms,
        };
        persist_execution_bridge_record(self.records_dir.as_path(), &record)?;

        self.state.last_applied_committed_height = context.height;
        self.state.last_execution_block_hash = Some(context.execution_block_hash.clone());
        self.state.last_execution_state_root = Some(context.execution_state_root.clone());
        self.state.last_node_block_hash = Some(context.node_block_hash);
        persist_execution_bridge_state(self.state_path.as_path(), &self.state)?;
        run_execution_bridge_retention_maintenance(
            self.records_dir.as_path(),
            &self.execution_store,
            self.hot_window_heights,
        )?;

        Ok(NodeExecutionCommitResult {
            execution_height: context.height,
            execution_block_hash: context.execution_block_hash,
            execution_state_root: context.execution_state_root,
        })
    }
}

pub(crate) fn bridge_committed_heights(
    snapshot: &NodeSnapshot,
    observed_at_unix_ms: i64,
    execution_world: &mut RuntimeWorld,
    execution_sandbox: &mut dyn ModuleSandbox,
    execution_store: &LocalCasStore,
    execution_records_dir: &Path,
    state: &mut ExecutionBridgeState,
) -> Result<Vec<ExecutionBridgeRecord>, String> {
    bridge_committed_heights_with_policy(
        snapshot,
        observed_at_unix_ms,
        execution_world,
        execution_sandbox,
        execution_store,
        execution_records_dir,
        state,
        EXECUTION_BRIDGE_DEFAULT_HOT_WINDOW_HEIGHTS,
        EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_INTERVAL_HEIGHTS,
        EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_KEEP_LATEST,
    )
}

fn bridge_committed_heights_with_policy(
    snapshot: &NodeSnapshot,
    observed_at_unix_ms: i64,
    execution_world: &mut RuntimeWorld,
    execution_sandbox: &mut dyn ModuleSandbox,
    execution_store: &LocalCasStore,
    execution_records_dir: &Path,
    state: &mut ExecutionBridgeState,
    hot_window_heights: u64,
    checkpoint_interval_heights: u64,
    checkpoint_keep_latest: usize,
) -> Result<Vec<ExecutionBridgeRecord>, String> {
    let target_height = snapshot.consensus.committed_height;
    if target_height <= state.last_applied_committed_height {
        return Ok(Vec::new());
    }

    fs::create_dir_all(execution_records_dir).map_err(|err| {
        format!(
            "create execution records dir {} failed: {}",
            execution_records_dir.display(),
            err
        )
    })?;

    let mut records = Vec::new();
    for height in (state.last_applied_committed_height + 1)..=target_height {
        execution_world
            .step_with_modules(execution_sandbox)
            .map_err(|err| {
                format!(
                    "execution bridge world.step failed at height {}: {:?}",
                    height, err
                )
            })?;

        let node_block_hash = if height == target_height {
            snapshot.consensus.last_block_hash.clone()
        } else {
            None
        };
        let runtime_resource_context = ChainResourceDerivationContext {
            world_id: snapshot.world_id.as_str(),
            chain_id: snapshot.world_id.as_str(),
            genesis_ref: None,
            created_at_height: execution_resource_created_at_height(height),
            manifest_height: height,
            commit_block_hash: None,
            tick: execution_world.state().time,
        };
        let runtime_resource_context_hash = execution_resource_context_hash(&snapshot.world_id);
        let snapshot_value = execution_world.snapshot_with_chain_resource_context(
            runtime_resource_context,
            runtime_resource_context_hash.clone(),
            runtime_resource_context_hash.clone(),
        );
        let journal_value = execution_world.journal().clone();
        let snapshot_bytes = super::to_cbor(snapshot_value)?;
        let journal_bytes = super::to_cbor(journal_value)?;

        let snapshot_ref = execution_store
            .put_bytes(snapshot_bytes.as_slice())
            .map_err(|err| format!("execution bridge CAS snapshot put failed: {:?}", err))?;
        let journal_ref = execution_store
            .put_bytes(journal_bytes.as_slice())
            .map_err(|err| format!("execution bridge CAS journal put failed: {:?}", err))?;

        let execution_state_root = blake3_hex(snapshot_bytes.as_slice());
        let prev_execution_block_hash = state
            .last_execution_block_hash
            .clone()
            .unwrap_or_else(|| "genesis".to_string());
        let hash_payload = ExecutionHashPayload {
            world_id: snapshot.world_id.as_str(),
            height,
            prev_execution_block_hash: prev_execution_block_hash.as_str(),
            execution_state_root: execution_state_root.as_str(),
            journal_len: execution_world.journal().len(),
        };
        let execution_block_hash = blake3_hex(super::to_cbor(hash_payload)?.as_slice());
        let mut record = ExecutionBridgeRecord::new_v2(
            snapshot.world_id.clone(),
            height,
            node_block_hash.clone(),
            execution_block_hash.clone(),
            execution_state_root.clone(),
            execution_world.journal().len(),
            snapshot_ref,
            journal_ref,
            None,
            None,
            observed_at_unix_ms,
        );
        record.checkpoint_ref = maybe_persist_execution_checkpoint_for_record(
            execution_records_dir,
            &record,
            checkpoint_interval_heights,
            checkpoint_keep_latest,
        )?;
        persist_execution_bridge_record(execution_records_dir, &record)?;

        state.last_applied_committed_height = height;
        state.last_execution_block_hash = Some(execution_block_hash);
        state.last_execution_state_root = Some(execution_state_root);
        state.last_node_block_hash = node_block_hash;
        records.push(record);
    }

    if !records.is_empty() {
        if let Err(err) = run_execution_bridge_retention_maintenance(
            execution_records_dir,
            execution_store,
            hot_window_heights,
        ) {
            oasis7::observability::emit_stderr_or_event(
                tracing::Level::WARN,
                format!(
                    "execution bridge retention pin-set sync failed after height {}: {}",
                    target_height, err
                )
                .as_str(),
                "execution bridge retention maintenance failed after replay",
            );
        }
    }

    Ok(records)
}
