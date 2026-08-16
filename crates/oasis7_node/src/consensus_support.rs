use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

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

pub(super) fn merge_pending_consensus_actions_with_budget(
    pending: &mut BTreeMap<u64, NodeConsensusAction>,
    incoming: Vec<NodeConsensusAction>,
    max_pending_actions: usize,
    queue_bytes: &AtomicUsize,
    max_queue_bytes: usize,
    incoming_already_reserved: bool,
) -> Result<(), NodeError> {
    let max_pending_actions = max_pending_actions.max(1);
    if incoming.is_empty() {
        return Ok(());
    }

    let mut incoming_bytes = 0usize;
    for action in &incoming {
        incoming_bytes = match incoming_bytes.checked_add(action.payload_cbor.len()) {
            Some(total) => total,
            None => {
                if incoming_already_reserved {
                    release_action_payload_bytes(queue_bytes, usize::MAX);
                }
                return Err(NodeError::Consensus {
                    reason: "pending consensus action payload byte count overflow".to_string(),
                });
            }
        };
    }

    let mut unique_incoming = BTreeMap::<u64, (String, String, usize)>::new();
    for action in &incoming {
        if let Err(err) = action.validate() {
            if incoming_already_reserved {
                release_action_payload_bytes(queue_bytes, incoming_bytes);
            }
            return Err(node_consensus_error(err));
        }
        match unique_incoming.get(&action.action_id) {
            Some((payload_hash, submitter_player_id, _))
                if payload_hash == &action.payload_hash
                    && submitter_player_id == &action.submitter_player_id => {}
            Some(_) => {
                if incoming_already_reserved {
                    release_action_payload_bytes(queue_bytes, incoming_bytes);
                }
                return Err(NodeError::Consensus {
                    reason: format!(
                        "conflicting consensus action payload for action_id={}",
                        action.action_id
                    ),
                });
            }
            None => {
                unique_incoming.insert(
                    action.action_id,
                    (
                        action.payload_hash.clone(),
                        action.submitter_player_id.clone(),
                        action.payload_cbor.len(),
                    ),
                );
            }
        }
    }

    for action in &incoming {
        if let Some(existing) = pending.get(&action.action_id) {
            if existing.payload_hash != action.payload_hash {
                if incoming_already_reserved {
                    release_action_payload_bytes(queue_bytes, incoming_bytes);
                }
                return Err(NodeError::Consensus {
                    reason: format!(
                        "conflicting consensus action payload for action_id={}",
                        action.action_id
                    ),
                });
            }
            if existing.submitter_player_id != action.submitter_player_id {
                if incoming_already_reserved {
                    release_action_payload_bytes(queue_bytes, incoming_bytes);
                }
                return Err(NodeError::Consensus {
                    reason: format!(
                        "conflicting consensus action submitter for action_id={}",
                        action.action_id
                    ),
                });
            }
        }
    }

    let unique_new_actions = unique_incoming
        .keys()
        .filter(|action_id| !pending.contains_key(action_id))
        .count();
    let projected = match pending.len().checked_add(unique_new_actions) {
        Some(projected) => projected,
        None => {
            if incoming_already_reserved {
                release_action_payload_bytes(queue_bytes, incoming_bytes);
            }
            return Err(NodeError::Consensus {
                reason: "pending consensus action projected length overflow".to_string(),
            });
        }
    };
    if projected > max_pending_actions {
        if incoming_already_reserved {
            release_action_payload_bytes(queue_bytes, incoming_bytes);
        }
        return Err(NodeError::Consensus {
            reason: format!(
                "pending consensus action engine buffer saturated: current={} incoming_unique={} limit={}",
                pending.len(),
                unique_new_actions,
                max_pending_actions
            ),
        });
    }

    let unique_new_bytes = match unique_incoming
        .iter()
        .filter(|(action_id, _)| !pending.contains_key(action_id))
        .try_fold(0usize, |total, (_, (_, _, bytes))| {
            total.checked_add(*bytes)
        }) {
        Some(bytes) => bytes,
        None => {
            if incoming_already_reserved {
                release_action_payload_bytes(queue_bytes, incoming_bytes);
            }
            return Err(NodeError::Consensus {
                reason: "pending consensus action unique payload byte count overflow".to_string(),
            });
        }
    };
    if incoming_already_reserved {
        let current = queue_bytes.load(Ordering::Acquire);
        if current < incoming_bytes || current > max_queue_bytes {
            release_action_payload_bytes(queue_bytes, incoming_bytes);
            return Err(NodeError::Consensus {
                reason: format!(
                    "pending consensus action queue byte reservation invalid: current={} incoming={} limit={}",
                    current, incoming_bytes, max_queue_bytes
                ),
            });
        }
    } else {
        reserve_action_payload_bytes(queue_bytes, max_queue_bytes, unique_new_bytes)?;
    }

    for action in incoming {
        pending.entry(action.action_id).or_insert(action);
    }
    if incoming_already_reserved {
        release_action_payload_bytes(queue_bytes, incoming_bytes.saturating_sub(unique_new_bytes));
    }
    Ok(())
}

pub(super) fn reserve_action_payload_bytes(
    queue_bytes: &AtomicUsize,
    max_queue_bytes: usize,
    additional_bytes: usize,
) -> Result<(), NodeError> {
    if additional_bytes == 0 {
        return Ok(());
    }
    let mut current = queue_bytes.load(Ordering::Acquire);
    loop {
        let projected =
            current
                .checked_add(additional_bytes)
                .ok_or_else(|| NodeError::Consensus {
                    reason: "pending consensus action queue byte count overflow".to_string(),
                })?;
        if projected > max_queue_bytes {
            return Err(NodeError::Consensus {
                reason: format!(
                    "pending consensus action queue byte budget exceeded: current={} incoming={} limit={}",
                    current, additional_bytes, max_queue_bytes
                ),
            });
        }
        match queue_bytes.compare_exchange(current, projected, Ordering::AcqRel, Ordering::Acquire)
        {
            Ok(_) => return Ok(()),
            Err(observed) => current = observed,
        }
    }
}

pub(super) fn release_action_payload_bytes(queue_bytes: &AtomicUsize, released_bytes: usize) {
    if released_bytes == 0 {
        return;
    }
    let mut current = queue_bytes.load(Ordering::Acquire);
    loop {
        let next = current.saturating_sub(released_bytes);
        match queue_bytes.compare_exchange(current, next, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => return,
            Err(observed) => current = observed,
        }
    }
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
