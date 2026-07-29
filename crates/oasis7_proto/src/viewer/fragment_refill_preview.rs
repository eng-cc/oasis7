use serde::{Deserialize, Serialize};

use super::PlayerAuthProof;

/// Wire representation of a chunk coordinate, kept in the protocol crate so it does not depend
/// on the simulator crate's world-model types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FragmentRefillPreviewChunk {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// Signed, read-only wire request for a fragment-replenishment forecast.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FragmentRefillRequest {
    pub chunk: FragmentRefillPreviewChunk,
    pub player_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<PlayerAuthProof>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FragmentRefillElementRemaining {
    pub element: String,
    pub remaining_g: i64,
}

/// Serializable runtime-live response mirroring the kernel's non-mutating forecast.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FragmentRefillResponse {
    pub chunk: FragmentRefillPreviewChunk,
    pub target_frag_id: Option<String>,
    pub current_frag_remaining_summary: String,
    pub chunk_remaining_summary: String,
    pub remaining_by_element_g: Vec<FragmentRefillElementRemaining>,
    pub replenishment_enabled: bool,
    pub replenishment_due: bool,
    pub next_replenish_tick: Option<u64>,
    pub ticks_until_replenish: Option<u64>,
    pub wait_cost_ticks: u64,
    pub estimated_replenished_frag_count: i64,
    pub estimated_replenished_resource_hint: String,
    pub next_industrial_goal_relevance: String,
    pub wait_cost_summary: String,
    pub recommended_resource_action: String,
}

pub type FragmentRefillPreviewProtocolRequest = FragmentRefillRequest;
pub type FragmentRefillPreviewResponse = FragmentRefillResponse;
