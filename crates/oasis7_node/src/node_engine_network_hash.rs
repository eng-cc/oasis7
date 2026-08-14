use super::*;

impl PosNodeEngine {
    pub(super) fn compute_block_hash(
        &self,
        world_id: &str,
        height: u64,
        slot: u64,
        epoch: u64,
        proposer_id: &str,
        parent_block_hash: &str,
        action_root: &str,
    ) -> Result<String, NodeError> {
        let payload = (
            1_u8,
            world_id,
            height,
            slot,
            epoch,
            proposer_id,
            parent_block_hash,
            action_root,
        );
        let bytes = serde_cbor::to_vec(&payload).map_err(|err| NodeError::Consensus {
            reason: format!("encode block hash payload failed: {err}"),
        })?;
        Ok(blake3_hex(bytes.as_slice()))
    }
}

pub(super) fn execution_error_is_peer_mismatch(err: &NodeError) -> bool {
    matches!(
        err,
        NodeError::Execution { reason }
            if reason.contains("execution hook returned peer mismatch")
    )
}

pub(super) fn peer_commit_heads_conflict(
    left: &PeerCommittedHead,
    right: &PeerCommittedHead,
) -> bool {
    left.block_hash != right.block_hash
        || matches!(
            (&left.execution_block_hash, &right.execution_block_hash),
            (Some(left), Some(right)) if left != right
        )
        || matches!(
            (&left.execution_state_root, &right.execution_state_root),
            (Some(left), Some(right)) if left != right
        )
        || (!left.action_root.is_empty()
            && !right.action_root.is_empty()
            && left.action_root != right.action_root)
}

pub(super) fn validated_commits_share_identity_block_action(
    left: &GossipCommitMessage,
    right: &GossipCommitMessage,
) -> bool {
    left.world_id == right.world_id
        && left.node_id == right.node_id
        && left.player_id == right.player_id
        && left.height == right.height
        && left.slot == right.slot
        && left.epoch == right.epoch
        && left.block_hash == right.block_hash
        && left.action_root == right.action_root
        && left.actions == right.actions
}
