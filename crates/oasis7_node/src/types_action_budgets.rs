use crate::{NodeConfig, NodeError};

impl NodeConfig {
    pub fn with_max_pending_consensus_action_queue_bytes(
        mut self,
        max_pending_consensus_action_queue_bytes: usize,
    ) -> Result<Self, NodeError> {
        if max_pending_consensus_action_queue_bytes == 0 {
            return Err(NodeError::InvalidConfig {
                reason: "max_pending_consensus_action_queue_bytes must be positive".to_string(),
            });
        }
        self.max_pending_consensus_action_queue_bytes = max_pending_consensus_action_queue_bytes;
        Ok(self)
    }

    pub fn with_max_committed_action_batch_bytes(
        mut self,
        max_committed_action_batch_bytes: usize,
    ) -> Result<Self, NodeError> {
        if max_committed_action_batch_bytes == 0 {
            return Err(NodeError::InvalidConfig {
                reason: "max_committed_action_batch_bytes must be positive".to_string(),
            });
        }
        self.max_committed_action_batch_bytes = max_committed_action_batch_bytes;
        Ok(self)
    }
}
