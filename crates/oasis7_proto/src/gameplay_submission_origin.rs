use serde::{Deserialize, Serialize};

/// Verified browser submission identity carried by the consensus payload.
/// This is correlation metadata, never an authorization grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameplaySubmissionOrigin {
    pub verified_player_id: String,
    pub public_key: String,
    pub auth_nonce: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hosted_registration_nonce: Option<String>,
    pub requester_agent_id: String,
    pub factory_id: String,
    pub recipe_id: String,
}

impl GameplaySubmissionOrigin {
    pub fn matches_recipe(&self, requester: &str, factory: &str, recipe: &str) -> bool {
        self.auth_nonce > 0
            && !self.verified_player_id.trim().is_empty()
            && self.public_key.len() == 64
            && self
                .public_key
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            && self
                .hosted_registration_nonce
                .as_ref()
                .is_none_or(|v| !v.trim().is_empty())
            && self.requester_agent_id == requester
            && self.factory_id == factory
            && self.recipe_id == recipe
    }
}
