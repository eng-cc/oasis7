use serde::{Deserialize, Serialize};

use super::PlayerAuthProof;

/// The bounded classifications available to a first player-to-player contact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FirstContactClass {
    TradeOrService,
    MutualAid,
    InformationExchange,
    DeferContact,
    OrganizationEscalation,
}

/// A signed, read-only request for a bounded first-contact preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialContactQuoteRequest {
    pub contact_purpose: String,
    pub first_contact_class: FirstContactClass,
    /// The candidate Agent for this contact, bound into the authenticated request.
    pub candidate_agent_id: String,
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
}

/// The non-mutating, player-readable consequences of a first-contact preview.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialContactQuotePreflight {
    pub first_contact_class: FirstContactClass,
    pub contact_purpose: String,
    pub expected_mutual_value: String,
    pub risk_or_commitment: String,
    pub solo_lane_preserved: bool,
    pub recommended_contact_action: String,
    pub defer_reason: String,
}

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

/// A player-supplied resource stake considered by a social-fact preflight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishSocialFactQuoteStake {
    pub kind: String,
    pub amount: i64,
}

/// A signed, read-only request for the impact of publishing a social fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishSocialFactQuoteRequest {
    pub schema_id: String,
    pub subject_agent_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_agent_id: Option<String>,
    pub claim: String,
    pub confidence_ppm: i64,
    pub evidence_event_ids: Vec<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_ticks: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stake: Option<PublishSocialFactQuoteStake>,
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
}

/// The non-mutating, authoritative consequences a player needs before publishing a social fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublishSocialFactQuotePreflight {
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
