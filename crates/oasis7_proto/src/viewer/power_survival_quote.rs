use serde::{Deserialize, Serialize};

use super::PlayerAuthProof;

/// The recovery action evaluated by a power-survival preflight.
///
/// `BuyPower` remains the wire default so existing signed clients retain their prior request
/// meaning while newer clients can request an on-site radiation harvest preview.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerSurvivalRecoveryAction {
    #[default]
    BuyPower,
    HarvestRadiation,
}

impl PowerSurvivalRecoveryAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BuyPower => "buy_power",
            Self::HarvestRadiation => "harvest_radiation",
        }
    }
}

/// A signed, read-only request for the simulator's exact `BuyPower` survival quote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerSurvivalQuoteRequest {
    pub seller_agent_id: String,
    pub amount: i64,
    pub requested_price_per_pu: i64,
    #[serde(default)]
    pub recovery_action: PowerSurvivalRecoveryAction,
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
}

/// The non-mutating, authoritative facts a player needs before submitting `BuyPower`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PowerSurvivalQuotePreflight {
    pub buyer_agent_id: String,
    pub seller_agent_id: String,
    pub current_power_level: i64,
    pub power_state_before: String,
    pub recovery_action: String,
    pub recovery_amount: i64,
    pub power_gain_estimate: i64,
    pub requested_price_per_pu: i64,
    pub price_per_pu: i64,
    pub price_or_time_cost: i64,
    pub power_state_after_recovery: String,
    pub survival_runway_ticks: i64,
    pub next_action_affordability_after_recovery: String,
    pub shutdown_avoidance_reason: String,
    pub recommended_power_action: String,
}
