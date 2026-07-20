use serde::{Deserialize, Serialize};

use super::PlayerAuthProof;

/// A signed request to quote or submit one exact self-owned data collection action.
///
/// The collector, owner, and recipient are intentionally absent: the runtime derives all
/// three from the authenticated player's bound Agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectDataRequest {
    pub electricity_cost: i64,
    pub data_amount: i64,
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectDataPreflight {
    pub collector_agent_id: String,
    pub data_owner_agent_id: String,
    pub data_recipient_agent_id: String,
    pub data_use: String,
    pub permission_status: String,
    pub electricity_cost: i64,
    pub data_amount: i64,
    pub available_electricity: i64,
    pub electricity_after: i64,
    pub can_execute: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recovery_guidance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alternative_action: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum CollectDataCommand {
    Preflight { request: CollectDataRequest },
    Submit { request: CollectDataRequest },
}
