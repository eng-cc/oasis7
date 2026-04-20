use super::*;

pub(super) fn node_pos_error(err: NodePosError) -> NodeError {
    NodeError::Consensus { reason: err.reason }
}

pub(super) fn node_consensus_error(err: NodeConsensusError) -> NodeError {
    NodeError::Consensus { reason: err.reason }
}

pub(super) fn checked_consensus_successor(
    value: u64,
    field: &str,
    context: &str,
) -> Result<u64, NodeError> {
    value.checked_add(1).ok_or_else(|| NodeError::Consensus {
        reason: format!("{field} overflow while {context}: current={value}"),
    })
}

pub(super) fn checked_replication_successor(
    value: u64,
    field: &str,
    context: &str,
) -> Result<u64, NodeError> {
    value.checked_add(1).ok_or_else(|| NodeError::Replication {
        reason: format!("{field} overflow while {context}: current={value}"),
    })
}

pub fn compute_consensus_action_root(actions: &[NodeConsensusAction]) -> Result<String, NodeError> {
    core_compute_consensus_action_root(actions).map_err(node_consensus_error)
}

pub(super) fn merge_pending_consensus_actions(
    pending: &mut BTreeMap<u64, NodeConsensusAction>,
    incoming: Vec<NodeConsensusAction>,
    max_pending_actions: usize,
) -> Result<(), NodeError> {
    let max_pending_actions = max_pending_actions.max(1);
    let mut unique_new_actions = 0usize;
    for action in &incoming {
        if !pending.contains_key(&action.action_id) {
            unique_new_actions =
                unique_new_actions
                    .checked_add(1)
                    .ok_or_else(|| NodeError::Consensus {
                        reason: "pending consensus action unique count overflow".to_string(),
                    })?;
        }
    }
    let projected = pending
        .len()
        .checked_add(unique_new_actions)
        .ok_or_else(|| NodeError::Consensus {
            reason: "pending consensus action projected length overflow".to_string(),
        })?;
    if projected > max_pending_actions {
        return Err(NodeError::Consensus {
            reason: format!(
                "pending consensus action engine buffer saturated: current={} incoming_unique={} limit={}",
                pending.len(),
                unique_new_actions,
                max_pending_actions
            ),
        });
    }
    core_merge_pending_consensus_actions(pending, incoming).map_err(node_consensus_error)?;
    if pending.len() > max_pending_actions {
        return Err(NodeError::Consensus {
            reason: format!(
                "pending consensus action engine buffer exceeded limit after merge: len={} limit={}",
                pending.len(),
                max_pending_actions
            ),
        });
    }
    Ok(())
}

pub(super) fn dequeue_pending_consensus_actions(
    pending: &mut Vec<NodeConsensusAction>,
    max_count: usize,
) -> Vec<NodeConsensusAction> {
    if max_count == 0 || pending.is_empty() {
        return Vec::new();
    }
    let drain_count = pending.len().min(max_count);
    if drain_count == pending.len() {
        return std::mem::take(pending);
    }
    pending.drain(..drain_count).collect()
}

pub(super) fn drain_ordered_consensus_actions(
    pending: &mut BTreeMap<u64, NodeConsensusAction>,
) -> Vec<NodeConsensusAction> {
    core_drain_ordered_consensus_actions(pending)
}

pub(super) fn validate_consensus_action_root(
    action_root: &str,
    actions: &[NodeConsensusAction],
) -> Result<(), NodeError> {
    core_validate_consensus_action_root(action_root, actions).map_err(node_consensus_error)
}

pub(super) fn sign_commit_message(
    message: &mut GossipCommitMessage,
    signer: &NodeConsensusMessageSigner,
) -> Result<(), NodeError> {
    core_sign_commit_message(message, signer).map_err(node_consensus_error)
}

pub(super) fn sign_proposal_message(
    message: &mut GossipProposalMessage,
    signer: &NodeConsensusMessageSigner,
) -> Result<(), NodeError> {
    core_sign_proposal_message(message, signer).map_err(node_consensus_error)
}

pub(super) fn sign_attestation_message(
    message: &mut GossipAttestationMessage,
    signer: &NodeConsensusMessageSigner,
) -> Result<(), NodeError> {
    core_sign_attestation_message(message, signer).map_err(node_consensus_error)
}

pub(super) fn verify_commit_message_signature(
    message: &GossipCommitMessage,
    enforce: bool,
) -> Result<(), NodeError> {
    core_verify_commit_message_signature(message, enforce).map_err(node_consensus_error)
}

pub(super) fn verify_proposal_message_signature(
    message: &GossipProposalMessage,
    enforce: bool,
) -> Result<(), NodeError> {
    core_verify_proposal_message_signature(message, enforce).map_err(node_consensus_error)
}

pub(super) fn verify_attestation_message_signature(
    message: &GossipAttestationMessage,
    enforce: bool,
) -> Result<(), NodeError> {
    core_verify_attestation_message_signature(message, enforce).map_err(node_consensus_error)
}
