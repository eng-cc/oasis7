use super::GossipReplicationMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FetchCommitRequest {
    pub world_id: String,
    pub height: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requester_public_key_hex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requester_signature_hex: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FetchCommitResponse {
    pub found: bool,
    pub message: Option<GossipReplicationMessage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FetchBlobRequest {
    pub content_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requester_public_key_hex: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requester_signature_hex: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct FetchBlobResponse {
    pub found: bool,
    pub blob: Option<Vec<u8>>,
}
