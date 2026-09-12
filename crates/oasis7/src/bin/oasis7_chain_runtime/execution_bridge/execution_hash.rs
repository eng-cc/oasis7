use oasis7::runtime::blake3_hex;
use serde::Serialize;

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
