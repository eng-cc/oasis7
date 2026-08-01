use serde::{Deserialize, Serialize};

use super::PlayerAuthProof;

/// A signed, read-only request for the current core war-settlement projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarDeclarationQuoteRequest {
    pub aggressor_alliance_id: String,
    pub defender_alliance_id: String,
    pub intensity: u32,
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
}

/// Non-mutating, advisory facts returned before a war declaration is submitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WarDeclarationQuotePreflight {
    pub actor_alliance_id: String,
    pub target_alliance_id: String,
    pub action_kind: String,
    pub intensity: u32,
    pub settlement_path: String,
    pub conflict_status: String,
    pub minimum_winning_intensity: Option<u32>,
    pub war_duration_ticks: u64,
    pub aggressor_score_estimate: i64,
    pub defender_score_estimate: i64,
    pub likely_winner_before_action: String,
    pub projected_outcome: String,
    pub victory_margin_estimate: i64,
    pub conflict_window_blocked_until: u64,
    pub reentry_cooldown_or_active_conflict_blocker: String,
    pub expected_narrative_or_module_reward: String,
    pub settlement_risk: String,
    pub settlement_risk_code: String,
    pub alternative_action: String,
    pub recommended_war_action: String,
    pub why_this_war_is_worth_or_risky: String,
    pub mobilization_electricity_required: i64,
    pub mobilization_electricity_current: i64,
    pub mobilization_electricity_after: i64,
    pub mobilization_data_required: i64,
    pub mobilization_data_current: i64,
    pub mobilization_data_after: i64,
    pub mobilization_affordable: bool,
    pub quoted_at_tick: u64,
    pub state_fingerprint: String,
}
