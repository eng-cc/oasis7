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
    pub(super) slot: u64,
    pub(super) epoch: u64,
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

pub(super) fn reconcile_engine_with_persisted_replication<'a>(
    engine: &mut PosNodeEngine,
    replication: &ReplicationRuntime,
    world_id: &str,
    execution_hook_ptr: Option<*mut (dyn NodeExecutionHook + 'a)>,
) -> Result<(), NodeError> {
    let latest_persisted_height = replication.latest_persisted_commit_height(world_id)?;
    if latest_persisted_height <= engine.committed_height {
        return Ok(());
    }
    let mut height = engine.committed_height.saturating_add(1);
    while height <= latest_persisted_height {
        let message = replication
            .load_commit_message_by_height(world_id, height)?
            .ok_or_else(|| NodeError::Replication {
                reason: format!(
                    "persisted commit missing for world={} height={}",
                    world_id, height
                ),
            })?;
        if message.world_id != world_id || message.record.world_id != world_id {
            return Err(NodeError::Replication {
                reason: format!(
                    "persisted commit world mismatch at height {} expected={} actual_message={} actual_record={}",
                    height, world_id, message.world_id, message.record.world_id
                ),
            });
        }
        let payload =
            parse_replication_commit_payload(message.payload.as_slice()).ok_or_else(|| {
                NodeError::Replication {
                    reason: format!(
                        "persisted commit payload decode failed for world={} height={}",
                        world_id, height
                    ),
                }
            })?;
        if payload.world_id != world_id {
            return Err(NodeError::Replication {
                reason: format!(
                    "persisted commit payload world mismatch at height {} expected={} actual={}",
                    height, world_id, payload.world_id
                ),
            });
        }
        if payload.node_id != message.node_id {
            return Err(NodeError::Replication {
                reason: format!(
                    "persisted commit payload node mismatch at height {} expected={} actual={}",
                    height, message.node_id, payload.node_id
                ),
            });
        }
        if payload.height != height {
            return Err(NodeError::Replication {
                reason: format!(
                    "persisted commit payload height mismatch expected={} actual={}",
                    height, payload.height
                ),
            });
        }
        if payload.block_hash.trim().is_empty() {
            return Err(NodeError::Replication {
                reason: format!(
                    "persisted commit payload block_hash is empty at height={}",
                    height
                ),
            });
        }
        validate_consensus_action_root(payload.action_root.as_str(), payload.actions.as_slice())
            .map_err(|err| NodeError::Replication {
                reason: format!(
                    "persisted commit action_root validation failed at height {}: {:?}",
                    height, err
                ),
            })?;
        engine.apply_synced_replication_commit(world_id, &payload, unsafe {
            reborrow_execution_hook_ptr(execution_hook_ptr)
        })?;
        height = height.saturating_add(1);
    }
    Ok(())
}
