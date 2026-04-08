use std::fs;
use std::path::Path;

use crate::release_security_policy_for_storage_profile;
use oasis7::consensus_action_payload::{
    decode_consensus_action_payload, ConsensusActionPayloadBody,
};
use oasis7::runtime::{
    blake3_hex, BlobStore, Journal as RuntimeJournal, LocalCasStore, ReleaseSecurityPolicy,
    Snapshot as RuntimeSnapshot, World as RuntimeWorld,
};
use oasis7::simulator::{
    Action as SimulatorAction, ActionSubmitter, WorldEventKind, WorldJournal as SimulatorJournal,
    WorldKernel, WorldSnapshot as SimulatorSnapshot,
};
use oasis7_node::{
    compute_consensus_action_root, NodeExecutionCommitContext, NodeExecutionCommitResult,
    NodeExecutionHook, NodeSnapshot,
};
use oasis7_proto::storage_profile::StorageProfileConfig;
use oasis7_wasm_abi::ModuleSandbox;
use oasis7_wasm_executor::{WasmExecutor, WasmExecutorConfig};
use serde::Serialize;

use super::checkpoint::{
    execution_bridge_record_path, load_execution_bridge_record,
    maybe_persist_execution_checkpoint_for_record, persist_execution_bridge_record,
    run_execution_bridge_retention_maintenance,
};
use super::external_effect::{
    build_execution_external_effect_materialization,
    persist_execution_external_effect_materialization,
};
use super::{
    ExecutionBridgeRecord, ExecutionBridgeState, ExecutionSimulatorMirrorRecord,
    EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_INTERVAL_HEIGHTS,
    EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_KEEP_LATEST, EXECUTION_BRIDGE_DEFAULT_HOT_WINDOW_HEIGHTS,
};

#[derive(Debug, Clone, Serialize)]
struct ExecutionHashPayload<'a> {
    world_id: &'a str,
    height: u64,
    prev_execution_block_hash: &'a str,
    execution_state_root: &'a str,
    journal_len: usize,
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
        let mut execution_world = load_execution_world(world_dir.as_path())?;
        execution_world.set_release_security_policy(release_security_policy_for_storage_profile(
            storage_profile.profile,
        ));
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
        height: u64,
        simulator_actions: &[(SimulatorAction, ActionSubmitter)],
    ) -> Result<Option<ExecutionSimulatorMirrorRecord>, String> {
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

        let snapshot_value = self.simulator_mirror.snapshot();
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
        persist_simulator_execution_world(
            self.simulator_world_dir.as_path(),
            &self.simulator_mirror,
        )?;

        Ok(Some(ExecutionSimulatorMirrorRecord {
            action_count: simulator_actions.len(),
            rejected_action_count,
            journal_len: self.simulator_mirror.journal().len(),
            snapshot_ref,
            journal_ref,
            state_root,
        }))
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

        let record = load_execution_bridge_record(record_path.as_path())?;
        if record.world_id != expected_world_id {
            return Err(format!(
                "execution driver stale-height restore world_id mismatch at height {}: expected={} actual={}",
                target_height, expected_world_id, record.world_id
            ));
        }
        let world_policy = self.execution_world.release_security_policy().clone();
        let snapshot_ref = record
            .latest_state_ref
            .as_deref()
            .or(record.snapshot_ref.as_deref())
            .ok_or_else(|| {
                format!(
                    "execution record at height {} missing latest_state_ref",
                    target_height
                )
            })?;
        let journal_ref = record.journal_ref.as_deref().ok_or_else(|| {
            format!(
                "execution record at height {} missing journal_ref",
                target_height
            )
        })?;

        let snapshot_bytes = self.execution_store.get_verified(snapshot_ref).map_err(|err| {
            format!(
                "execution driver restore snapshot ref {} failed at height {}: {:?}",
                snapshot_ref, target_height, err
            )
        })?;
        let journal_bytes = self.execution_store.get_verified(journal_ref).map_err(|err| {
            format!(
                "execution driver restore journal ref {} failed at height {}: {:?}",
                journal_ref, target_height, err
            )
        })?;
        let snapshot =
            serde_cbor::from_slice::<RuntimeSnapshot>(snapshot_bytes.as_slice()).map_err(|err| {
                format!(
                    "execution driver decode runtime snapshot failed at height {}: {}",
                    target_height, err
                )
            })?;
        let journal =
            serde_cbor::from_slice::<RuntimeJournal>(journal_bytes.as_slice()).map_err(|err| {
                format!(
                    "execution driver decode runtime journal failed at height {}: {}",
                    target_height, err
                )
            })?;
        let mut restored_world = RuntimeWorld::from_snapshot(snapshot, journal)
            .map_err(|err| {
                format!(
                    "execution driver rebuild runtime world failed at height {}: {:?}",
                    target_height, err
                )
            })?;
        restored_world.set_release_security_policy(world_policy);
        persist_execution_world(self.world_dir.as_path(), &restored_world)?;
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
            )?;
            self.simulator_mirror = restored_simulator;
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
        if context.height < self.state.last_applied_committed_height {
            let stale_state_height = self.state.last_applied_committed_height;
            if !self.restore_execution_head_from_record(
                context.world_id.as_str(),
                context.height,
            )? {
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
            eprintln!(
                "execution driver detected non-contiguous committed heights: last_applied={} incoming={} (continuing with gap)",
                self.state.last_applied_committed_height, context.height
            );
            self.state.last_applied_committed_height = context.height.saturating_sub(1);
            self.state.last_node_block_hash = None;
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
        let external_effect_ref = persist_execution_external_effect_materialization(
            &self.execution_store,
            &external_effect,
        )?;

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

        for action in decoded_runtime_actions {
            self.execution_world.submit_action(action);
        }
        self.execution_world
            .step_with_modules(&mut *self.execution_sandbox)
            .map_err(|err| {
                format!(
                    "execution driver world.step failed at height {}: {:?}",
                    context.height, err
                )
            })?;
        let simulator_mirror =
            self.apply_simulator_actions(context.height, decoded_simulator_actions.as_slice())?;

        let snapshot_value = self.execution_world.snapshot();
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
        persist_execution_world(self.world_dir.as_path(), &self.execution_world)?;
        if let Err(err) = run_execution_bridge_retention_maintenance(
            self.records_dir.as_path(),
            &self.execution_store,
            self.hot_window_heights,
        ) {
            eprintln!(
                "execution driver retention pin-set sync failed at height {}: {}",
                context.height, err
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
}

pub(crate) fn load_execution_bridge_state(path: &Path) -> Result<ExecutionBridgeState, String> {
    if !path.exists() {
        return Ok(ExecutionBridgeState::default());
    }
    let bytes = fs::read(path).map_err(|err| {
        format!(
            "read execution bridge state {} failed: {}",
            path.display(),
            err
        )
    })?;
    serde_json::from_slice::<ExecutionBridgeState>(bytes.as_slice()).map_err(|err| {
        format!(
            "parse execution bridge state {} failed: {}",
            path.display(),
            err
        )
    })
}

pub(crate) fn persist_execution_bridge_state(
    path: &Path,
    state: &ExecutionBridgeState,
) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(state)
        .map_err(|err| format!("serialize execution bridge state failed: {}", err))?;
    super::write_bytes_atomic(path, bytes.as_slice())
}

pub(crate) fn load_execution_world(world_dir: &Path) -> Result<RuntimeWorld, String> {
    let snapshot_path = world_dir.join("snapshot.json");
    let journal_path = world_dir.join("journal.json");
    if !snapshot_path.exists() || !journal_path.exists() {
        return Ok(RuntimeWorld::new_production_hardened());
    }
    RuntimeWorld::load_from_dir(world_dir)
        .map_err(|err| {
            format!(
                "load execution world from {} failed: {:?}",
                world_dir.display(),
                err
            )
        })
        .map(|world| {
            world.with_release_security_policy(ReleaseSecurityPolicy::production_hardened())
        })
}

pub(crate) fn persist_execution_world(
    world_dir: &Path,
    execution_world: &RuntimeWorld,
) -> Result<(), String> {
    execution_world.save_to_dir(world_dir).map_err(|err| {
        format!(
            "save execution world to {} failed: {:?}",
            world_dir.display(),
            err
        )
    })
}

pub(crate) fn simulator_world_dir_from_execution_world_dir(world_dir: &Path) -> std::path::PathBuf {
    match world_dir.file_name().and_then(|name| name.to_str()) {
        Some(name) if !name.is_empty() => {
            world_dir.with_file_name(format!("{name}-simulator-mirror"))
        }
        _ => world_dir.join("simulator-mirror"),
    }
}

fn load_simulator_execution_world(world_dir: &Path) -> Result<WorldKernel, String> {
    let snapshot_path = world_dir.join("snapshot.json");
    let journal_path = world_dir.join("journal.json");
    if !snapshot_path.exists() || !journal_path.exists() {
        return Ok(WorldKernel::new());
    }
    WorldKernel::load_from_dir(world_dir).map_err(|err| {
        format!(
            "load simulator execution mirror from {} failed: {:?}",
            world_dir.display(),
            err
        )
    })
}

fn persist_simulator_execution_world(
    world_dir: &Path,
    simulator_world: &WorldKernel,
) -> Result<(), String> {
    simulator_world.save_to_dir(world_dir).map_err(|err| {
        format!(
            "save simulator execution mirror to {} failed: {:?}",
            world_dir.display(),
            err
        )
    })
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

        let snapshot_value = execution_world.snapshot();
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
        let node_block_hash = if height == target_height {
            snapshot.consensus.last_block_hash.clone()
        } else {
            None
        };

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
            eprintln!(
                "execution bridge retention pin-set sync failed after height {}: {}",
                target_height, err
            );
        }
    }

    Ok(records)
}
