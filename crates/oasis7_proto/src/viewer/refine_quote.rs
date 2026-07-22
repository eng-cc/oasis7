use serde::{Deserialize, Serialize};

use super::PlayerAuthProof;

/// A signed, read-only request for the kernel's exact compound-refining quote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefineQuoteRequest {
    pub compound_mass_g: i64,
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefineQuotePreflight {
    pub owner_agent_id: String,
    pub compound_mass_g: i64,
    pub electricity_cost: i64,
    pub electricity_after: i64,
    pub hardware_output: i64,
    pub target_id: String,
    pub target_gap_before: i64,
    pub target_gap_after: i64,
    pub target_linkage: String,
    pub recommended_refine_amount: i64,
    pub value_classification: String,
}
