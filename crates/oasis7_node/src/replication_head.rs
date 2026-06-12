use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FetchHeadRequest {
    pub world_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ReplicationHeadSummary {
    pub world_id: String,
    pub height: u64,
    pub block_hash: String,
    pub state_root: String,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FetchHeadResponse {
    pub found: bool,
    pub head: Option<ReplicationHeadSummary>,
}
