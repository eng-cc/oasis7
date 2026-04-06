use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ExecutionBridgeState {
    pub last_applied_committed_height: u64,
    pub last_execution_block_hash: Option<String>,
    pub last_execution_state_root: Option<String>,
    pub last_node_block_hash: Option<String>,
}

pub(super) const EXECUTION_BRIDGE_RECORD_SCHEMA_V1: u32 = 1;
pub(super) const EXECUTION_BRIDGE_RECORD_SCHEMA_V2: u32 = 2;
pub(super) const EXECUTION_BRIDGE_DEFAULT_HOT_WINDOW_HEIGHTS: u64 = 32;
pub(super) const EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_INTERVAL_HEIGHTS: u64 = 32;
pub(super) const EXECUTION_BRIDGE_DEFAULT_CHECKPOINT_KEEP_LATEST: usize = 4;

fn execution_bridge_record_schema_v1() -> u32 {
    EXECUTION_BRIDGE_RECORD_SCHEMA_V1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "ExecutionBridgeRecordWire")]
pub(super) struct ExecutionBridgeRecord {
    pub schema_version: u32,
    pub world_id: String,
    pub height: u64,
    pub node_block_hash: Option<String>,
    pub execution_block_hash: String,
    pub execution_state_root: String,
    pub journal_len: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_state_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub journal_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit_log_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_effect_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub simulator_mirror: Option<ExecutionSimulatorMirrorRecord>,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct ExecutionBridgeRecordWire {
    #[serde(default = "execution_bridge_record_schema_v1")]
    pub schema_version: u32,
    pub world_id: String,
    pub height: u64,
    #[serde(default)]
    pub node_block_hash: Option<String>,
    pub execution_block_hash: String,
    pub execution_state_root: String,
    pub journal_len: usize,
    #[serde(default)]
    pub latest_state_ref: Option<String>,
    #[serde(default)]
    pub snapshot_ref: Option<String>,
    #[serde(default)]
    pub journal_ref: Option<String>,
    #[serde(default)]
    pub commit_log_ref: Option<String>,
    #[serde(default)]
    pub checkpoint_ref: Option<String>,
    #[serde(default)]
    pub external_effect_ref: Option<String>,
    #[serde(default)]
    pub simulator_mirror: Option<ExecutionSimulatorMirrorRecord>,
    pub timestamp_ms: i64,
}

impl From<ExecutionBridgeRecordWire> for ExecutionBridgeRecord {
    fn from(record: ExecutionBridgeRecordWire) -> Self {
        let snapshot_ref = record.snapshot_ref;
        let latest_state_ref = record.latest_state_ref.or_else(|| snapshot_ref.clone());
        Self {
            schema_version: record.schema_version.max(EXECUTION_BRIDGE_RECORD_SCHEMA_V1),
            world_id: record.world_id,
            height: record.height,
            node_block_hash: record.node_block_hash,
            execution_block_hash: record.execution_block_hash,
            execution_state_root: record.execution_state_root,
            journal_len: record.journal_len,
            latest_state_ref,
            snapshot_ref,
            journal_ref: record.journal_ref,
            commit_log_ref: record.commit_log_ref,
            checkpoint_ref: record.checkpoint_ref,
            external_effect_ref: record.external_effect_ref,
            simulator_mirror: record.simulator_mirror,
            timestamp_ms: record.timestamp_ms,
        }
    }
}

impl ExecutionBridgeRecord {
    pub(super) fn new_v2(
        world_id: String,
        height: u64,
        node_block_hash: Option<String>,
        execution_block_hash: String,
        execution_state_root: String,
        journal_len: usize,
        snapshot_ref: String,
        journal_ref: String,
        external_effect_ref: Option<String>,
        simulator_mirror: Option<ExecutionSimulatorMirrorRecord>,
        timestamp_ms: i64,
    ) -> Self {
        Self {
            schema_version: EXECUTION_BRIDGE_RECORD_SCHEMA_V2,
            world_id,
            height,
            node_block_hash,
            execution_block_hash,
            execution_state_root,
            journal_len,
            latest_state_ref: Some(snapshot_ref.clone()),
            snapshot_ref: Some(snapshot_ref),
            journal_ref: Some(journal_ref),
            commit_log_ref: None,
            checkpoint_ref: None,
            external_effect_ref,
            simulator_mirror,
            timestamp_ms,
        }
    }
}

pub(super) const EXECUTION_CHECKPOINT_MANIFEST_SCHEMA_V1: u32 = 1;

fn execution_checkpoint_manifest_schema_v1() -> u32 {
    EXECUTION_CHECKPOINT_MANIFEST_SCHEMA_V1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ExecutionCheckpointManifest {
    #[serde(default = "execution_checkpoint_manifest_schema_v1")]
    pub schema_version: u32,
    pub checkpoint_id: String,
    pub world_id: String,
    pub height: u64,
    pub execution_block_hash: String,
    pub execution_state_root: String,
    pub latest_state_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub journal_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pinned_refs: Vec<String>,
    pub manifest_hash: String,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ExecutionCheckpointLatestPointer {
    #[serde(default = "execution_checkpoint_manifest_schema_v1")]
    pub schema_version: u32,
    pub checkpoint_id: String,
    pub height: u64,
    pub manifest_hash: String,
    pub manifest_rel_path: String,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct ExecutionCheckpointManifestHashPayload<'a> {
    pub schema_version: u32,
    pub checkpoint_id: &'a str,
    pub world_id: &'a str,
    pub height: u64,
    pub execution_block_hash: &'a str,
    pub execution_state_root: &'a str,
    pub latest_state_ref: &'a str,
    pub snapshot_ref: Option<&'a str>,
    pub journal_ref: Option<&'a str>,
    pub pinned_refs: &'a [String],
    pub created_at_ms: i64,
}

pub(super) const EXECUTION_EXTERNAL_EFFECT_SCHEMA_V1: u32 = 1;
pub(super) const EXECUTION_EXTERNAL_EFFECT_CONTRACT_CLOSED_WORLD_V1: &str = "closed_world_v1";

fn execution_external_effect_schema_v1() -> u32 {
    EXECUTION_EXTERNAL_EFFECT_SCHEMA_V1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ExecutionExternalEffectMaterialization {
    #[serde(default = "execution_external_effect_schema_v1")]
    pub schema_version: u32,
    pub contract: String,
    pub world_id: String,
    pub node_id: String,
    pub height: u64,
    pub slot: u64,
    pub epoch: u64,
    pub node_block_hash: String,
    pub action_root: String,
    pub committed_at_unix_ms: i64,
    pub pre_step_execution_state_root: String,
    pub world_manifest_hash: String,
    pub active_modules_hash: String,
    pub committed_actions_hash: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_modules: Vec<ExecutionModuleResolutionAnchor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub committed_actions: Vec<ExecutionCommittedActionAnchor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved_inputs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ExecutionModuleResolutionAnchor {
    pub instance_id: String,
    pub module_id: String,
    pub module_version: String,
    pub wasm_hash: String,
    pub install_target: oasis7::simulator::ModuleInstallTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ExecutionCommittedActionAnchor {
    pub action_id: u64,
    pub submitter_player_id: String,
    pub payload_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExecutionReplayRecordInput {
    pub record: ExecutionBridgeRecord,
    pub external_effect: Option<ExecutionExternalEffectMaterialization>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExecutionReplayPlan {
    pub target_height: u64,
    pub start_height: u64,
    pub checkpoint: Option<ExecutionCheckpointManifest>,
    pub records: Vec<ExecutionReplayRecordInput>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ExecutionBridgePinSet {
    pub latest_height: Option<u64>,
    pub hot_window_start_height: Option<u64>,
    pub checkpoint_heights: BTreeSet<u64>,
    pub pinned_refs: BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ExecutionSimulatorMirrorRecord {
    pub action_count: usize,
    pub rejected_action_count: usize,
    pub journal_len: usize,
    pub snapshot_ref: String,
    pub journal_ref: String,
    pub state_root: String,
}

fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    super::write_bytes_atomic(path, bytes)
}

fn to_cbor<T: Serialize>(value: T) -> Result<Vec<u8>, String> {
    serde_cbor::to_vec(&value).map_err(|err| format!("serialize to cbor failed: {}", err))
}

mod checkpoint;
mod driver;
mod external_effect;
#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub(super) use self::driver::{load_execution_world, NodeRuntimeExecutionDriver};
