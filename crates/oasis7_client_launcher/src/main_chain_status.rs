use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(super) struct WebChainReplicationPeerHealth {
    pub(super) peer_id: String,
    pub(super) status: String,
    pub(super) issues: Vec<String>,
    pub(super) discovery_sources: Vec<String>,
    pub(super) active_path_kind: Option<String>,
    pub(super) source_operator: Option<String>,
    pub(super) source_asn: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(super) struct WebChainReplicationStatus {
    pub(super) local_peer_id: String,
    pub(super) connected_peers: Vec<String>,
    pub(super) peer_healths: Vec<WebChainReplicationPeerHealth>,
    #[serde(default)]
    pub(super) request_peer_scores: std::collections::BTreeMap<String, u8>,
}
