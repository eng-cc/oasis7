use super::*;

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ReplicationCommitPayloadView {
    pub(super) height: u64,
    pub(super) block_hash: String,
    pub(super) committed_at_ms: i64,
    #[serde(default)]
    pub(super) execution_block_hash: Option<String>,
    #[serde(default)]
    pub(super) execution_state_root: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct ReplicationCommitPayload {
    pub(super) world_id: String,
    pub(super) node_id: String,
    pub(super) height: u64,
    pub(super) block_hash: String,
    pub(super) action_root: String,
    pub(super) actions: Vec<NodeConsensusAction>,
    pub(super) committed_at_ms: i64,
    #[serde(default)]
    pub(super) execution_block_hash: Option<String>,
    #[serde(default)]
    pub(super) execution_state_root: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NodeEngineTickResult {
    pub(super) consensus_snapshot: NodeConsensusSnapshot,
    pub(super) committed_action_batch: Option<NodeCommittedActionBatch>,
}

pub(super) fn parse_replication_commit_payload_view(
    payload: &[u8],
) -> Option<ReplicationCommitPayloadView> {
    serde_json::from_slice::<ReplicationCommitPayloadView>(payload).ok()
}

pub(super) fn parse_replication_commit_payload(payload: &[u8]) -> Option<ReplicationCommitPayload> {
    serde_json::from_slice::<ReplicationCommitPayload>(payload).ok()
}

pub(super) fn reconcile_engine_with_persisted_replication(
    engine: &mut PosNodeEngine,
    replication: &ReplicationRuntime,
    world_id: &str,
) -> Result<(), NodeError> {
    let latest_persisted_height = replication.latest_persisted_commit_height(world_id)?;
    if latest_persisted_height <= engine.committed_height {
        return Ok(());
    }

    let message = replication
        .load_commit_message_by_height(world_id, latest_persisted_height)?
        .ok_or_else(|| NodeError::Replication {
            reason: format!(
                "latest persisted commit missing for world={} height={}",
                world_id, latest_persisted_height
            ),
        })?;
    if message.world_id != world_id || message.record.world_id != world_id {
        return Err(NodeError::Replication {
            reason: format!(
                "latest persisted commit world mismatch expected={} actual_message={} actual_record={}",
                world_id, message.world_id, message.record.world_id
            ),
        });
    }
    let payload =
        parse_replication_commit_payload(message.payload.as_slice()).ok_or_else(|| {
            NodeError::Replication {
                reason: format!(
                    "latest persisted commit payload decode failed for world={} height={}",
                    world_id, latest_persisted_height
                ),
            }
        })?;
    if payload.world_id != world_id {
        return Err(NodeError::Replication {
            reason: format!(
                "latest persisted commit payload world mismatch expected={} actual={}",
                world_id, payload.world_id
            ),
        });
    }
    if payload.node_id != message.node_id {
        return Err(NodeError::Replication {
            reason: format!(
                "latest persisted commit payload node mismatch expected={} actual={}",
                message.node_id, payload.node_id
            ),
        });
    }
    if payload.height != latest_persisted_height {
        return Err(NodeError::Replication {
            reason: format!(
                "latest persisted commit payload height mismatch expected={} actual={}",
                latest_persisted_height, payload.height
            ),
        });
    }
    if payload.block_hash.trim().is_empty() {
        return Err(NodeError::Replication {
            reason: format!(
                "latest persisted commit payload block_hash is empty at height={}",
                latest_persisted_height
            ),
        });
    }
    validate_consensus_action_root(payload.action_root.as_str(), payload.actions.as_slice())
        .map_err(|err| NodeError::Replication {
            reason: format!(
                "latest persisted commit action_root validation failed at height {}: {:?}",
                latest_persisted_height, err
            ),
        })?;

    engine.record_synced_replication_height(
        latest_persisted_height,
        payload.block_hash,
        payload.committed_at_ms,
    )?;

    if payload.execution_block_hash.is_some() != payload.execution_state_root.is_some() {
        return Err(NodeError::Replication {
            reason: format!(
                "latest persisted commit execution binding malformed at height {}",
                latest_persisted_height
            ),
        });
    }
    if let (Some(execution_block_hash), Some(execution_state_root)) =
        (payload.execution_block_hash, payload.execution_state_root)
    {
        if latest_persisted_height >= engine.last_execution_height {
            engine.last_execution_height = latest_persisted_height;
            engine.last_execution_block_hash = Some(execution_block_hash);
            engine.last_execution_state_root = Some(execution_state_root);
            engine.remember_execution_binding_for_height(latest_persisted_height);
        }
    }

    Ok(())
}
