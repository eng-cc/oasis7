use serde::{Deserialize, Serialize};

use super::PlayerAuthProof;

/// A signed, read-only request for the impact of declaring a social relationship edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclareSocialEdgeQuoteRequest {
    pub schema_id: String,
    pub relation_kind: String,
    pub from_agent_id: String,
    pub to_agent_id: String,
    pub weight_bps: i64,
    pub backing_fact_ids: Vec<u64>,
    pub ttl_ticks: Option<u64>,
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
}

/// The non-mutating, authoritative consequences a player needs before declaring a social edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclareSocialEdgeQuotePreflight {
    pub actor_id: String,
    pub action_kind: String,
    pub schema_id: String,
    pub subject_id: Option<String>,
    pub object_id: Option<String>,
    pub claim_summary: String,
    pub confidence_ppm: Option<i64>,
    pub stake_at_risk: i64,
    pub ttl_ticks: Option<u64>,
    pub affected_relationships: Vec<String>,
    pub affected_social_surfaces: Vec<String>,
    pub cooperation_opportunity_delta: String,
    pub blacklist_or_dispute_risk: String,
    pub governance_or_claim_relevance: String,
    pub recommended_social_action: String,
    pub why_this_action_matters: String,
}
