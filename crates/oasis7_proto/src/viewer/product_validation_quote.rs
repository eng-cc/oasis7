use serde::{Deserialize, Serialize};

use super::PlayerAuthProof;

/// A signed, read-only request for the authoritative product-validation preflight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductValidationQuoteRequest {
    pub product_id: String,
    pub amount: i64,
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
}

/// The non-mutating, authoritative facts a player needs before product validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductValidationQuotePreflight {
    pub product_id: String,
    pub product_role: String,
    pub tradable: bool,
    pub stage_before: String,
    pub stage_after: String,
    pub unlock_or_value_class: String,
    pub recommended_action: String,
    pub submission_allowed: bool,
    pub missing_prerequisite: String,
    pub reachable_advance_or_recovery: String,
}
