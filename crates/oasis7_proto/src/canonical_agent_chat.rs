use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalAgentChatAuthorityV1 {
    pub branch_id: String,
    pub agent_identity_generation: u64,
}

/// Only populated by an authenticated canonical owner/Agent projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalAgentChatViewV1 {
    pub agent_id: String,
    pub player_id: String,
    pub public_key: String,
    pub world_id: String,
    pub reorg_epoch: u64,
    pub authority_scope: String,
    pub canonical_authority: CanonicalAgentChatAuthorityV1,
    pub current_intent_id: Option<String>,
    pub goal: Option<CanonicalAgentGoalV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalAgentGoalV1 {
    pub intent_id: String,
    pub message: String,
    pub status: String,
    pub event_seq: u64,
    pub logical_time: u64,
}

/// Public identity fence; this type cannot carry goal text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalAgentOwnerFenceV1 {
    pub agent_id: String,
    pub player_id: String,
    pub public_key: String,
    pub world_id: String,
    pub reorg_epoch: u64,
    pub canonical_authority: CanonicalAgentChatAuthorityV1,
    pub current_intent_id: Option<String>,
}
impl From<&CanonicalAgentChatViewV1> for CanonicalAgentOwnerFenceV1 {
    fn from(view: &CanonicalAgentChatViewV1) -> Self {
        Self {
            agent_id: view.agent_id.clone(),
            player_id: view.player_id.clone(),
            public_key: view.public_key.clone(),
            world_id: view.world_id.clone(),
            reorg_epoch: view.reorg_epoch,
            canonical_authority: view.canonical_authority.clone(),
            current_intent_id: view.current_intent_id.clone(),
        }
    }
}
