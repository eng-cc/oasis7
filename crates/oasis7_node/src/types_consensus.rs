#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct NodeValidatorStakeProofSnapshot {
    pub validator_id: String,
    pub player_id: String,
    pub stake: u64,
    pub signer_public_key_hex: Option<String>,
    pub leaf_hash: String,
    pub proof: Vec<NodeValidatorStakeProofStepSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct NodeValidatorStakeProofStepSnapshot {
    pub side: String,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct NodeConsensusMisbehaviorEvidenceSnapshot {
    pub kind: String,
    pub evidence_hash: String,
    pub validator_id: String,
    pub node_id: String,
    pub height: u64,
    pub observed_at_ms: i64,
    pub first_block_hash: String,
    pub second_block_hash: String,
    pub first_execution_block_hash: Option<String>,
    pub second_execution_block_hash: Option<String>,
    pub first_execution_state_root: Option<String>,
    pub second_execution_state_root: Option<String>,
    pub first_action_root: String,
    pub second_action_root: String,
    pub first_public_key_hex: Option<String>,
    pub second_public_key_hex: Option<String>,
    pub first_signature_hex: Option<String>,
    pub second_signature_hex: Option<String>,
    pub slashable_stake: u64,
    pub total_stake: u64,
    pub validator_stake_root: String,
    pub quarantined: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct NodeConsensusSlashingIntentSnapshot {
    pub intent_id: String,
    pub evidence_hash: String,
    pub kind: String,
    pub validator_id: String,
    pub target_agent_id: String,
    pub reason: String,
    pub slash_stake: u64,
    pub appeal_window_ticks: u64,
    pub validator_stake_root: String,
    pub governance_method: String,
    pub status: String,
    pub enforced: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct NodeConsensusSlashingReceiptSnapshot {
    pub penalty_id: u64,
    pub intent_id: String,
    pub evidence_hash: String,
    pub validator_id: String,
    pub target_agent_id: String,
    pub slash_stake: u64,
    pub status: String,
    pub evidence_chain_hash: String,
    pub appeal_deadline_tick: u64,
    pub applied: bool,
}
