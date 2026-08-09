use serde::{Deserialize, Serialize};

use super::PlayerAuthProof;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceVoteQuoteRequest {
    pub proposal_key: String,
    pub option: String,
    pub weight: u32,
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceVoteQuotePreflight {
    pub proposal_id: String,
    pub proposal_topic: String,
    pub actor_id: String,
    pub action_kind: String,
    pub closes_at_tick: u64,
    pub ticks_remaining: u64,
    pub current_quorum_weight: u64,
    pub required_quorum_weight: u64,
    pub current_pass_bps: u16,
    pub required_pass_bps: u16,
    pub actor_vote_weight: u32,
    pub vote_swing_potential: u32,
    pub likely_outcome_before_action: String,
    pub likely_outcome_after_action: String,
    pub affected_rule_or_priority: String,
    pub world_change_if_passed: String,
    pub cost_or_cooldown_if_failed: String,
    pub recommended_governance_action: String,
    pub why_this_vote_matters: String,
}
