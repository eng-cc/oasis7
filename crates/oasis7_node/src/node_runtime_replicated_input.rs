use std::sync::atomic::Ordering;

use crate::consensus_support::reserve_action_payload_bytes;
use crate::node_runtime_batch_retention::action_payload_bytes;
use crate::{
    NodeConsensusAction, NodeError, NodeReplicatedExecutionInputV1, NodeRuntime,
    REPLICATED_EXECUTION_INPUT_ACTION_ID, REPLICATED_EXECUTION_INPUT_SUBMITTER,
    bind_replicated_execution_input_action,
};

impl NodeRuntime {
    /// Queue a producer-authored execution input for the next canonical
    /// proposal. The input is committed as a reserved consensus action, so
    /// every materializing node receives the same bytes through the block and
    /// action root. The target height is bound after persisted engine state is
    /// restored and before the worker starts; a non-proposer's stale input is
    /// dropped when that height is committed by the network.
    pub fn submit_replicated_execution_input(
        &self,
        input: NodeReplicatedExecutionInputV1,
    ) -> Result<(), NodeError> {
        if self.running.load(Ordering::Acquire) {
            return Err(NodeError::Consensus {
                reason: "replicated execution input must be queued before runtime start"
                    .to_string(),
            });
        }
        if input.target_height != 0 {
            return Err(NodeError::Consensus {
                reason: format!(
                    "producer replicated execution input must be unbound before start: target_height={}",
                    input.target_height
                ),
            });
        }
        let payload_cbor = input
            .encode()
            .map_err(|reason| NodeError::Consensus { reason })?;
        if payload_cbor.len() > self.config.max_consensus_action_payload_bytes {
            return Err(NodeError::Consensus {
                reason: format!(
                    "replicated execution input payload too large: bytes={} limit={}",
                    payload_cbor.len(),
                    self.config.max_consensus_action_payload_bytes
                ),
            });
        }
        let action = NodeConsensusAction::from_payload(
            REPLICATED_EXECUTION_INPUT_ACTION_ID,
            REPLICATED_EXECUTION_INPUT_SUBMITTER,
            payload_cbor,
        )
        .map_err(|err| NodeError::Consensus { reason: err.reason })?;
        let mut pending = self
            .pending_consensus_actions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if self.running.load(Ordering::Acquire) {
            return Err(NodeError::Consensus {
                reason: "replicated execution input must be queued before runtime start"
                    .to_string(),
            });
        }
        if pending
            .iter()
            .any(|existing| existing.action_id == REPLICATED_EXECUTION_INPUT_ACTION_ID)
        {
            return Err(NodeError::Consensus {
                reason: "a replicated execution input is already queued".to_string(),
            });
        }
        if pending.len() >= self.config.max_pending_consensus_actions {
            return Err(NodeError::Consensus {
                reason: format!(
                    "pending consensus actions queue saturated: len={} limit={}",
                    pending.len(),
                    self.config.max_pending_consensus_actions
                ),
            });
        }
        reserve_action_payload_bytes(
            &self.pending_consensus_action_queue_bytes,
            self.config.max_pending_consensus_action_queue_bytes,
            action.payload_cbor.len(),
        )?;
        pending.push(action);
        Ok(())
    }
}

pub(super) fn bind_pending_replicated_execution_inputs(
    runtime: &NodeRuntime,
    target_height: u64,
) -> Result<(), NodeError> {
    let mut pending = runtime
        .pending_consensus_actions
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for action in pending.iter_mut() {
        if let Err(reason) = bind_replicated_execution_input_action(action, target_height) {
            return Err(NodeError::Consensus { reason });
        }
    }
    let pending_bytes = action_payload_bytes(pending.iter());
    if pending_bytes > runtime.config.max_pending_consensus_action_queue_bytes {
        return Err(NodeError::Consensus {
            reason: format!(
                "pending consensus action payload bytes exceed limit after execution input binding: bytes={} limit={}",
                pending_bytes, runtime.config.max_pending_consensus_action_queue_bytes
            ),
        });
    }
    runtime
        .pending_consensus_action_queue_bytes
        .store(pending_bytes, Ordering::Release);
    Ok(())
}
