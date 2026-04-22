use std::collections::BTreeMap;

use oasis7_proto::distributed_pos::{
    decide_pos_status, slot_epoch as shared_slot_epoch, PosDecisionStatus,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodePosAttestation {
    pub validator_id: String,
    pub approve: bool,
    pub source_epoch: u64,
    pub target_epoch: u64,
    pub voted_at_ms: i64,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodePosPendingProposal<TAction, TStatus> {
    pub height: u64,
    pub slot: u64,
    pub epoch: u64,
    pub opened_at_ms: i64,
    pub proposer_id: String,
    pub block_hash: String,
    pub action_root: String,
    pub committed_actions: Vec<TAction>,
    pub attestations: BTreeMap<String, NodePosAttestation>,
    pub approved_stake: u64,
    pub rejected_stake: u64,
    pub status: TStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodePosDecision<TAction, TStatus> {
    pub height: u64,
    pub slot: u64,
    pub epoch: u64,
    pub status: TStatus,
    pub block_hash: String,
    pub action_root: String,
    pub committed_actions: Vec<TAction>,
    pub approved_stake: u64,
    pub rejected_stake: u64,
    pub required_stake: u64,
    pub total_stake: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodePosError {
    pub reason: String,
}

impl std::fmt::Display for NodePosError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.reason.as_str())
    }
}

impl std::error::Error for NodePosError {}

pub trait NodePosStatusAdapter: Copy + Clone + PartialEq + Eq {
    fn pending() -> Self;
    fn committed() -> Self;
    fn rejected() -> Self;
}

pub fn decision_from_proposal<TAction: Clone, TStatus: Copy>(
    proposal: &NodePosPendingProposal<TAction, TStatus>,
    required_stake: u64,
    total_stake: u64,
) -> NodePosDecision<TAction, TStatus> {
    NodePosDecision {
        height: proposal.height,
        slot: proposal.slot,
        epoch: proposal.epoch,
        status: proposal.status,
        block_hash: proposal.block_hash.clone(),
        action_root: proposal.action_root.clone(),
        committed_actions: proposal.committed_actions.clone(),
        approved_stake: proposal.approved_stake,
        rejected_stake: proposal.rejected_stake,
        required_stake,
        total_stake,
    }
}

pub fn insert_attestation<TAction, TStatus: NodePosStatusAdapter>(
    validators: &BTreeMap<String, u64>,
    total_stake: u64,
    required_stake: u64,
    proposal: &mut NodePosPendingProposal<TAction, TStatus>,
    validator_id: &str,
    approve: bool,
    voted_at_ms: i64,
    source_epoch: u64,
    target_epoch: u64,
    reason: Option<String>,
) -> Result<(), NodePosError> {
    let stake = validators
        .get(validator_id)
        .copied()
        .ok_or_else(|| NodePosError {
            reason: format!("validator not found: {}", validator_id),
        })?;
    if proposal.attestations.contains_key(validator_id) {
        return Ok(());
    }

    let (next_approved_stake, next_rejected_stake) = if approve {
        (
            proposal
                .approved_stake
                .checked_add(stake)
                .ok_or_else(|| NodePosError {
                    reason: format!(
                        "approved stake overflow: validator={} current={} delta={}",
                        validator_id, proposal.approved_stake, stake
                    ),
                })?,
            proposal.rejected_stake,
        )
    } else {
        (
            proposal.approved_stake,
            proposal
                .rejected_stake
                .checked_add(stake)
                .ok_or_else(|| NodePosError {
                    reason: format!(
                        "rejected stake overflow: validator={} current={} delta={}",
                        validator_id, proposal.rejected_stake, stake
                    ),
                })?,
        )
    };
    let next_status = decide_status(
        total_stake,
        required_stake,
        next_approved_stake,
        next_rejected_stake,
    );

    proposal.attestations.insert(
        validator_id.to_string(),
        NodePosAttestation {
            validator_id: validator_id.to_string(),
            approve,
            source_epoch,
            target_epoch,
            voted_at_ms,
            reason,
        },
    );
    proposal.approved_stake = next_approved_stake;
    proposal.rejected_stake = next_rejected_stake;
    proposal.status = next_status;
    Ok(())
}

pub fn propose_next_head<TAction: Clone, TStatus: NodePosStatusAdapter>(
    validators: &BTreeMap<String, u64>,
    total_stake: u64,
    required_stake: u64,
    epoch_length_slots: u64,
    next_height: &mut u64,
    next_slot: &mut u64,
    pending: &mut Option<NodePosPendingProposal<TAction, TStatus>>,
    proposer_id: String,
    block_hash: String,
    action_root: String,
    committed_actions: Vec<TAction>,
    accepted_by_node_id: &str,
    proposed_at_ms: i64,
) -> Result<NodePosDecision<TAction, TStatus>, NodePosError> {
    let slot = *next_slot;
    let epoch = shared_slot_epoch(epoch_length_slots, slot);
    let mut proposal = NodePosPendingProposal {
        height: *next_height,
        slot,
        epoch,
        opened_at_ms: proposed_at_ms,
        proposer_id: proposer_id.clone(),
        block_hash,
        action_root,
        committed_actions,
        attestations: BTreeMap::new(),
        approved_stake: 0,
        rejected_stake: 0,
        status: TStatus::pending(),
    };

    insert_attestation(
        validators,
        total_stake,
        required_stake,
        &mut proposal,
        proposer_id.as_str(),
        true,
        proposed_at_ms,
        epoch.saturating_sub(1),
        epoch,
        Some(format!("proposal accepted by {accepted_by_node_id}")),
    )?;

    let next_slot_value = next_slot.checked_add(1).ok_or_else(|| NodePosError {
        reason: format!("slot overflow at {}", *next_slot),
    })?;
    *next_slot = next_slot_value;
    let decision = decision_from_proposal(&proposal, required_stake, total_stake);
    *pending = Some(proposal);
    Ok(decision)
}

pub fn advance_pending_attestations<TAction: Clone, TStatus: NodePosStatusAdapter>(
    validators: &BTreeMap<String, u64>,
    total_stake: u64,
    required_stake: u64,
    local_validator_id: &str,
    auto_attest_all_validators: bool,
    pending: &mut Option<NodePosPendingProposal<TAction, TStatus>>,
    now_ms: i64,
) -> Result<NodePosDecision<TAction, TStatus>, NodePosError> {
    let mut proposal = pending.clone().ok_or_else(|| NodePosError {
        reason: "missing pending proposal".to_string(),
    })?;

    if auto_attest_all_validators {
        for validator_id in validators.keys() {
            if proposal.attestations.contains_key(validator_id.as_str()) {
                continue;
            }
            let epoch = proposal.epoch;
            insert_attestation(
                validators,
                total_stake,
                required_stake,
                &mut proposal,
                validator_id.as_str(),
                true,
                now_ms,
                epoch.saturating_sub(1),
                epoch,
                Some("node mainloop auto attestation".to_string()),
            )?;
            if is_terminal_status(proposal.status) {
                break;
            }
        }
    } else if validators.contains_key(local_validator_id)
        && !proposal.attestations.contains_key(local_validator_id)
    {
        let epoch = proposal.epoch;
        insert_attestation(
            validators,
            total_stake,
            required_stake,
            &mut proposal,
            local_validator_id,
            true,
            now_ms,
            epoch.saturating_sub(1),
            epoch,
            Some("node local validator attestation".to_string()),
        )?;
    }

    let decision = decision_from_proposal(&proposal, required_stake, total_stake);
    *pending = Some(proposal);
    Ok(decision)
}

fn decide_status<TStatus: NodePosStatusAdapter>(
    total_stake: u64,
    required_stake: u64,
    approved_stake: u64,
    rejected_stake: u64,
) -> TStatus {
    match decide_pos_status(total_stake, required_stake, approved_stake, rejected_stake) {
        PosDecisionStatus::Pending => TStatus::pending(),
        PosDecisionStatus::Committed => TStatus::committed(),
        PosDecisionStatus::Rejected => TStatus::rejected(),
    }
}

fn is_terminal_status<TStatus: NodePosStatusAdapter>(status: TStatus) -> bool {
    status == TStatus::committed() || status == TStatus::rejected()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestStatus {
        Pending,
        Committed,
        Rejected,
    }

    impl NodePosStatusAdapter for TestStatus {
        fn pending() -> Self {
            TestStatus::Pending
        }

        fn committed() -> Self {
            TestStatus::Committed
        }

        fn rejected() -> Self {
            TestStatus::Rejected
        }
    }

    fn sample_proposal() -> NodePosPendingProposal<(), TestStatus> {
        NodePosPendingProposal {
            height: 1,
            slot: 0,
            epoch: 0,
            opened_at_ms: 10,
            proposer_id: "val-a".to_string(),
            block_hash: "b1".to_string(),
            action_root: "a1".to_string(),
            committed_actions: Vec::new(),
            attestations: BTreeMap::new(),
            approved_stake: 0,
            rejected_stake: 0,
            status: TestStatus::Pending,
        }
    }

    #[test]
    fn insert_attestation_rejects_overflow_without_mutating_proposal() {
        let validators = BTreeMap::from([("val-a".to_string(), u64::MAX)]);
        let mut proposal = sample_proposal();
        proposal.approved_stake = 1;

        let err = insert_attestation(
            &validators,
            u64::MAX,
            1,
            &mut proposal,
            "val-a",
            true,
            10,
            0,
            0,
            None,
        )
        .expect_err("must reject stake overflow");
        assert!(err.reason.contains("approved stake overflow"));
        assert!(proposal.attestations.is_empty());
        assert_eq!(proposal.approved_stake, 1);
        assert_eq!(proposal.rejected_stake, 0);
        assert_eq!(proposal.status, TestStatus::Pending);
    }

    #[test]
    fn propose_next_head_rejects_slot_overflow_without_pending_mutation() {
        let validators = BTreeMap::from([("val-a".to_string(), 10_u64)]);
        let mut next_height = 1;
        let mut next_slot = u64::MAX;
        let mut pending: Option<NodePosPendingProposal<(), TestStatus>> = None;

        let err = propose_next_head(
            &validators,
            10,
            7,
            32,
            &mut next_height,
            &mut next_slot,
            &mut pending,
            "val-a".to_string(),
            "block-hash".to_string(),
            "action-root".to_string(),
            Vec::new(),
            "node-a",
            100,
        )
        .expect_err("must reject slot overflow");
        assert!(err.reason.contains("slot overflow"));
        assert_eq!(next_slot, u64::MAX);
        assert!(pending.is_none());
    }
}
