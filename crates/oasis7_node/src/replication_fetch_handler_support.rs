use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::execution_hook::NodeExecutionHook;
use crate::replication::{
    GossipReplicationMessage, NodeReplicationConfig, ReplicationRuntime,
    load_latest_commit_message_from_root,
};
use crate::replication_state_reconcile::parse_replication_commit_payload;
use crate::{NodeError, REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL};

pub(super) fn attach_checkpoint_for_fetch_commit_if_boundary(
    message: Option<GossipReplicationMessage>,
    execution_hook: Option<&Arc<Mutex<Box<dyn NodeExecutionHook>>>>,
    root_dir: &Path,
    world_id: &str,
    node_id: &str,
    replication: &NodeReplicationConfig,
    request_height: u64,
) -> Result<Option<GossipReplicationMessage>, NodeError> {
    let Some(execution_hook) = execution_hook else {
        return Ok(message);
    };
    let Some(message) = message else {
        return Ok(None);
    };
    let latest_commit_height = load_latest_commit_message_from_root(
        root_dir,
        world_id,
        replication.max_hot_commit_messages(),
    )?
    .and_then(|message| {
        parse_replication_commit_payload(message.payload.as_slice()).map(|payload| payload.height)
    });
    if !latest_commit_height
        .map(|latest_height| {
            should_export_checkpoint_for_fetch_commit(request_height, latest_height)
        })
        .unwrap_or(false)
    {
        return Ok(Some(message));
    }

    let checkpoint = execution_hook
        .lock()
        .map_err(|_| NodeError::Execution {
            reason: "execution hook lock poisoned".to_string(),
        })?
        .export_checkpoint_bundle(request_height)
        .map_err(|reason| NodeError::Execution { reason })?;
    let Some(checkpoint) = checkpoint else {
        return Ok(Some(message));
    };
    let mut runtime = ReplicationRuntime::new(replication, node_id)?;
    runtime
        .attach_execution_checkpoint_descriptor_to_message(node_id, &message, &checkpoint)
        .map(Some)
}

pub(super) fn should_export_checkpoint_for_fetch_commit(
    request_height: u64,
    latest_height: u64,
) -> bool {
    request_height == latest_height
        || request_height % REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL == 0
        || request_height % (REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL / 2) == 0
}
