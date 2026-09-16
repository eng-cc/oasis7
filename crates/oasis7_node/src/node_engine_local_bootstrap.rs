use super::*;

impl PosNodeEngine {
    pub(super) fn apply_local_execution_bootstrap(
        &mut self,
        bootstrap: &crate::NodeExecutionBootstrap,
    ) -> Result<(), NodeError> {
        if bootstrap.height == 0 {
            return Err(NodeError::InvalidConfig {
                reason: "local execution bootstrap height must be > 0".to_string(),
            });
        }
        if bootstrap.consensus_block_hash.trim().is_empty()
            || bootstrap.execution_block_hash.trim().is_empty()
            || bootstrap.execution_state_root.trim().is_empty()
        {
            return Err(NodeError::InvalidConfig {
                reason: "local execution bootstrap hashes must be non-empty".to_string(),
            });
        }
        let next_height =
            bootstrap
                .height
                .checked_add(1)
                .ok_or_else(|| NodeError::InvalidConfig {
                    reason: format!(
                        "local execution bootstrap height {} has no successor",
                        bootstrap.height
                    ),
                })?;
        self.committed_height = bootstrap.height;
        self.network_committed_height = bootstrap.height;
        self.replication_persisted_height = bootstrap.height;
        self.next_height = next_height;
        self.last_broadcast_proposal_height = bootstrap.height;
        self.last_broadcast_local_attestation_height = bootstrap.height;
        self.last_broadcast_gossip_committed_height = bootstrap.height;
        self.last_broadcast_network_committed_height = bootstrap.height;
        self.last_broadcast_gossip_replicated_committed_height = bootstrap.height;
        self.last_broadcast_network_replicated_committed_height = bootstrap.height;
        self.last_committed_block_hash = Some(bootstrap.consensus_block_hash.clone());
        self.last_execution_height = bootstrap.height;
        self.last_execution_block_hash = Some(bootstrap.execution_block_hash.clone());
        self.last_execution_state_root = Some(bootstrap.execution_state_root.clone());
        self.execution_bindings.clear();
        self.remember_execution_binding_for_height(bootstrap.height);
        Ok(())
    }

    pub(super) fn validate_local_execution_bootstrap(
        &self,
        bootstrap: &crate::NodeExecutionBootstrap,
    ) -> Result<(), NodeError> {
        let matches = self.committed_height == bootstrap.height
            && self.network_committed_height >= bootstrap.height
            && self.next_height == bootstrap.height.saturating_add(1)
            && self.last_committed_block_hash.as_deref()
                == Some(bootstrap.consensus_block_hash.as_str())
            && self.last_execution_height >= bootstrap.height
            && self.last_execution_block_hash.as_deref()
                == Some(bootstrap.execution_block_hash.as_str())
            && self.last_execution_state_root.as_deref()
                == Some(bootstrap.execution_state_root.as_str());
        if matches {
            Ok(())
        } else {
            Err(NodeError::Replication {
                reason: format!(
                    "persisted node state does not match local execution bootstrap boundary at height {}",
                    bootstrap.height
                ),
            })
        }
    }
}
