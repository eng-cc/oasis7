use serde::{Deserialize, Serialize};

use super::PlayerAuthProof;

/// One material contribution the player intends to use for a recipe submission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketQuoteMaterialRequest {
    pub material: String,
    pub amount: i64,
}

/// A signed, read-only request for the market costs that would apply before recipe submission.
///
/// The consuming ledger is deliberately derived from the authenticated player's bound Agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketQuoteDecisionRequest {
    pub consume: Vec<MarketQuoteMaterialRequest>,
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
}

/// The player-readable contribution from one requested material to a conditional market quote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketQuoteMaterialContribution {
    pub material: String,
    pub requested_amount: i64,
    pub local_available_amount: i64,
    pub world_available_amount: i64,
    pub world_cover_amount: i64,
    pub shortfall_amount: i64,
    pub transit_loss_bps: i64,
    pub governance_tax_bps: u16,
    pub effective_cost_index_ppm: i64,
}

/// A conditional, read-only pre-submit explanation of the market costs for one recipe input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketQuoteDecisionPreflight {
    pub consuming_agent_id: String,
    pub contributions: Vec<MarketQuoteMaterialContribution>,
    pub total_shortfall_amount: i64,
    pub submission_allowed: bool,
    pub conditional_notice: String,
    pub recommendation: String,
    pub rationale: String,
    pub next_action: String,
}
