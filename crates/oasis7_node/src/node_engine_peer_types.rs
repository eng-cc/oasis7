#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PeerCommittedHead {
    pub(super) height: u64,
    pub(super) block_hash: String,
    pub(super) committed_at_ms: i64,
    pub(super) observed_at_ms: i64,
    pub(super) execution_block_hash: Option<String>,
    pub(super) execution_state_root: Option<String>,
    pub(super) action_root: String,
    pub(super) public_key_hex: Option<String>,
    pub(super) signature_hex: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConsensusMisbehaviorEvidence {
    pub(super) kind: String,
    pub(super) validator_id: String,
    pub(super) node_id: String,
    pub(super) height: u64,
    pub(super) observed_at_ms: i64,
    pub(super) first_block_hash: String,
    pub(super) second_block_hash: String,
    pub(super) first_execution_block_hash: Option<String>,
    pub(super) second_execution_block_hash: Option<String>,
    pub(super) first_execution_state_root: Option<String>,
    pub(super) second_execution_state_root: Option<String>,
    pub(super) first_action_root: String,
    pub(super) second_action_root: String,
    pub(super) first_public_key_hex: Option<String>,
    pub(super) second_public_key_hex: Option<String>,
    pub(super) first_signature_hex: Option<String>,
    pub(super) second_signature_hex: Option<String>,
    pub(super) slashable_stake: u64,
    pub(super) total_stake: u64,
    pub(super) validator_stake_root: String,
    pub(super) quarantined: bool,
}
