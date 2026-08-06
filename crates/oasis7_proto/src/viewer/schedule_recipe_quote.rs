use serde::{Deserialize, Serialize};

use super::PlayerAuthProof;

/// A signed, read-only request for the canonical ScheduleRecipe quote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleRecipeQuoteRequest {
    pub factory_id: String,
    pub recipe_id: String,
    pub batches: i64,
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
}

/// Exact player-safe field projection of the simulator's ScheduleRecipe quote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleRecipeQuotePreflight {
    pub owner_agent_id: String,
    pub factory_id: String,
    pub recipe_id: String,
    pub batches: i64,
    pub base_duration_ticks: i64,
    pub electricity_cost: i64,
    pub electricity_after: i64,
    pub hardware_cost: i64,
    pub data_output: i64,
    pub finished_product_id: String,
    pub finished_product_units: i64,
    pub local_shortage_delay_ticks: i64,
    pub shortage_reason: String,
    pub recommended_pre_step: String,
    pub runway_before_ticks: i64,
    pub runway_after_ticks: i64,
    pub downtime_threshold_ppm: i64,
    pub continue_production_risk: String,
    pub maintenance_pressure_delta: String,
    pub recommended_maintenance_action: String,
}
