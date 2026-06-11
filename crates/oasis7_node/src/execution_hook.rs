use serde::{Deserialize, Serialize};

use crate::NodeConsensusAction;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeExecutionCommitContext {
    pub world_id: String,
    pub node_id: String,
    pub height: u64,
    pub slot: u64,
    pub epoch: u64,
    pub node_block_hash: String,
    pub action_root: String,
    #[serde(default)]
    pub committed_actions: Vec<NodeConsensusAction>,
    pub committed_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeExecutionCommitResult {
    pub execution_height: u64,
    pub execution_block_hash: String,
    pub execution_state_root: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeExecutionCheckpointBlob {
    pub content_hash: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeExecutionCheckpointBlobRef {
    pub content_hash: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeExecutionCheckpointDescriptor {
    pub height: u64,
    pub execution_block_hash: String,
    pub execution_state_root: String,
    pub manifest_ref: String,
    pub manifest_size_bytes: u64,
    #[serde(default)]
    pub blobs: Vec<NodeExecutionCheckpointBlobRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeExecutionCheckpointBundle {
    pub height: u64,
    pub execution_block_hash: String,
    pub execution_state_root: String,
    pub manifest_json: Vec<u8>,
    #[serde(default)]
    pub blobs: Vec<NodeExecutionCheckpointBlob>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeExecutionCheckpointInstallContext {
    pub world_id: String,
    pub node_id: String,
    pub height: u64,
    pub node_block_hash: String,
    pub execution_block_hash: String,
    pub execution_state_root: String,
    pub committed_at_unix_ms: i64,
}

pub trait NodeExecutionHook: Send {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String>;

    fn export_checkpoint_bundle(
        &mut self,
        _height: u64,
    ) -> Result<Option<NodeExecutionCheckpointBundle>, String> {
        Ok(None)
    }

    fn install_checkpoint_bundle(
        &mut self,
        _context: NodeExecutionCheckpointInstallContext,
        _bundle: NodeExecutionCheckpointBundle,
    ) -> Result<NodeExecutionCommitResult, String> {
        Err("execution checkpoint install is not supported by this hook".to_string())
    }
}
