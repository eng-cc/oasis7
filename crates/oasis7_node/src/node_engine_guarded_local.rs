//! Recovery of an exact previously captured local decision, never network finality.
use crate::{
    NodeError, NodeLocalExecutionContinuation, PosConsensusStatus, PosDecision, PosNodeEngine,
    compute_consensus_action_root,
};

impl PosNodeEngine {
    pub(super) fn restore_guarded_local_decision(
        &mut self,
        saved: &NodeLocalExecutionContinuation,
        node_id: &str,
        world_id: &str,
    ) -> Result<PosDecision, NodeError> {
        let c = &saved.context;
        let invalid = || NodeError::Execution {
            reason: "guarded local continuation does not match original decision/counters".into(),
        };
        let bytes = self.validate_inbound_proposal_actions(&c.committed_actions)?;
        let before_execution = self.last_execution_height == saved.predecessor_execution_height
            && self.last_execution_block_hash == saved.predecessor_execution_block_hash
            && self.last_execution_state_root == saved.predecessor_execution_state_root;
        // A qualified candidate can survive local dissemination failure. Its
        // hook still verifies the stored proof before returning the old result.
        let qualified_execution = self.last_execution_height == c.height;
        if saved.schema_version != 1
            || c.node_id != node_id
            || c.proposer_id != node_id
            || c.world_id != world_id
            || c.height
                != saved
                    .predecessor_committed_height
                    .checked_add(1)
                    .ok_or_else(invalid)?
            || self.committed_height != saved.predecessor_committed_height
            || self.last_committed_block_hash != saved.predecessor_node_block_hash
            || c.epoch != self.slot_epoch(c.slot)
            || self.compute_block_hash(
                world_id,
                c.height,
                c.slot,
                c.epoch,
                &c.proposer_id,
                saved
                    .predecessor_node_block_hash
                    .as_deref()
                    .unwrap_or("genesis"),
                &c.action_root,
            )? != c.node_block_hash
            || saved.predecessor_execution_height != saved.predecessor_committed_height
            || (!before_execution && !qualified_execution)
            || self.next_height != saved.next_height
            || self.next_slot != saved.next_slot
            || saved.required_stake != self.required_stake
            || saved.total_stake != self.total_stake
            || saved.approved_stake < saved.required_stake
            || saved.approved_stake > saved.total_stake
            || saved
                .approved_stake
                .checked_add(saved.rejected_stake)
                .is_none_or(|n| n > saved.total_stake)
            || c.committed_actions.len() > self.max_pending_consensus_actions
            || crate::node_runtime_batch_retention::action_payload_bytes(c.committed_actions.iter())
                > self.max_pending_consensus_action_queue_bytes
            || compute_consensus_action_root(&c.committed_actions).map_err(|_| invalid())?
                != c.action_root
            || saved.reserved_action_bytes != bytes
            || c.node_block_hash.trim().is_empty()
        {
            return Err(invalid());
        }
        if let Some(pending) = self.pending.as_ref()
            && (pending.height != c.height
                || pending.slot != c.slot
                || pending.epoch != c.epoch
                || pending.proposer_id != c.proposer_id
                || pending.block_hash != c.node_block_hash
                || pending.action_root != c.action_root
                || pending.committed_actions != c.committed_actions)
        {
            return Err(invalid());
        }
        self.adjust_pending_proposal_reservation(bytes)?;
        self.set_pending_action_reservation(bytes);
        Ok(PosDecision {
            height: c.height,
            slot: c.slot,
            epoch: c.epoch,
            proposer_id: c.proposer_id.clone(),
            status: PosConsensusStatus::Committed,
            block_hash: c.node_block_hash.clone(),
            action_root: c.action_root.clone(),
            committed_actions: c.committed_actions.clone(),
            approved_stake: saved.approved_stake,
            rejected_stake: saved.rejected_stake,
            required_stake: saved.required_stake,
            total_stake: saved.total_stake,
        })
    }
}

pub(super) fn ensure_ordinary_network_recovery(
    hook: Option<&dyn crate::NodeExecutionHook>,
) -> Result<(), NodeError> {
    if hook.is_some_and(|h| h.guarded_local_execution()) {
        Err(NodeError::Execution {
            reason: "guarded engineering execution rejects synthetic network decisions".into(),
        })
    } else {
        Ok(())
    }
}
