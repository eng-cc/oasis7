use super::*;

#[derive(Debug, Clone)]
pub(super) struct PendingCheckpointReceipt {
    pub(super) world_id: String,
    pub(super) node_id: String,
    pub(super) height: u64,
    pub(super) message: replication::GossipReplicationMessage,
    pub(super) descriptor: NodeExecutionCheckpointDescriptor,
    pub(super) bundle: NodeExecutionCheckpointBundle,
    pub(super) fetch_observations: Vec<serde_json::Value>,
    pub(super) probe_nonce: String,
    pub(super) receipt_persisted: bool,
    pub(super) block_hash: String,
    pub(super) committed_at_ms: i64,
}

impl PosNodeEngine {
    pub(super) fn finalize_pending_checkpoint_receipt(
        &mut self,
        endpoint: &ReplicationNetworkEndpoint,
        node_id: &str,
        world_id: &str,
        replication_runtime: &mut ReplicationRuntime,
        checkpoint_height: u64,
        progress_callback: &mut Option<
            &mut dyn FnMut(NodeConsensusSnapshot) -> Result<(), NodeError>,
        >,
    ) -> Result<bool, NodeError> {
        let pending = self
            .pending_checkpoint_receipt
            .clone()
            .expect("pending checkpoint receipt checked");
        if pending.world_id != world_id
            || pending.node_id != node_id
            || pending.height != checkpoint_height
            || self.last_execution_height != pending.height
            || self.last_execution_block_hash.as_deref()
                != Some(pending.descriptor.execution_block_hash.as_str())
            || self.last_execution_state_root.as_deref()
                != Some(pending.descriptor.execution_state_root.as_str())
        {
            return Err(NodeError::Replication {
                reason: format!(
                    "pending checkpoint receipt identity mismatch pending_world={} requested_world={} pending_node={} requested_node={} pending_height={} requested_height={}",
                    pending.world_id,
                    world_id,
                    pending.node_id,
                    node_id,
                    pending.height,
                    checkpoint_height
                ),
            });
        }
        if !pending.receipt_persisted {
            replication_runtime.persist_checkpoint_verification_receipt(
                world_id,
                Some(pending.probe_nonce.as_str()),
                &pending.descriptor,
                Some(&pending.bundle),
                pending.fetch_observations.as_slice(),
            )?;
            self.pending_checkpoint_receipt
                .as_mut()
                .expect("pending checkpoint receipt retained during finalization")
                .receipt_persisted = true;
        }
        self.persist_synced_replication_message(
            endpoint,
            node_id,
            world_id,
            replication_runtime,
            &pending.message,
            pending.height,
        )?;
        self.replication_persisted_height = self.replication_persisted_height.max(pending.height);
        self.record_synced_replication_height(
            pending.height,
            pending.block_hash,
            pending.committed_at_ms,
        )?;
        self.pending_checkpoint_receipt = None;
        self.last_replication_gap_sync_blocked_height = None;
        self.last_replication_gap_sync_blocked_reason = None;
        self.last_replication_gap_sync_blocked_at_ms = None;
        self.last_replication_gap_sync_repair_attempt_height = None;
        self.last_replication_gap_sync_repair_attempt_summary = None;
        self.last_replication_gap_sync_repair_attempt_route_snapshot = None;
        if let Some(callback) = progress_callback.as_deref_mut() {
            let decision = self.idle_pending_decision()?;
            callback(self.snapshot_from_decision(&decision))?;
        }
        Ok(true)
    }
}
