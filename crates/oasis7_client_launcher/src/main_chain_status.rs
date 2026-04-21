use serde::Deserialize;

use crate::LaunchConfig;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(super) struct WebChainRecoverySnapshot {
    pub(super) error_code: String,
    pub(super) reason: String,
    pub(super) node_id: String,
    pub(super) execution_world_dir: String,
    pub(super) recovery_mode: String,
    pub(super) reset_required: bool,
    pub(super) fresh_node_id: String,
    pub(super) fresh_chain_status_bind: String,
    pub(super) suggested_config: LaunchConfig,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(super) struct WebChainP2pStatus {
    pub(super) requested_user_mode: String,
    pub(super) recommended_user_mode: String,
    pub(super) effective_user_mode: String,
    pub(super) applied_effective_user_mode: Option<String>,
    pub(super) requires_explicit_public_entry_confirmation: bool,
    pub(super) detected_reachability: Option<String>,
    pub(super) hole_punch_viability: String,
    pub(super) relay_available: bool,
    pub(super) probe_stable: bool,
    pub(super) deployment_mode: String,
    pub(super) node_role_claim: String,
    pub(super) rationale: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(super) struct WebChainNodeObservabilityAlert {
    pub(super) severity: String,
    pub(super) code: String,
    pub(super) summary: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(super) struct WebChainNodeObservabilityStatus {
    pub(super) status: String,
    pub(super) summary: String,
    pub(super) connected_peer_count: usize,
    pub(super) active_peer_count: usize,
    pub(super) candidate_peer_count: usize,
    pub(super) suspect_peer_count: usize,
    pub(super) blocked_peer_count: usize,
    pub(super) peer_with_issues_count: usize,
    pub(super) known_peer_heads: usize,
    pub(super) network_height_lag: u64,
    pub(super) recent_replication_error_count: usize,
    pub(super) storage_degraded: bool,
    pub(super) reward_runtime_degraded: bool,
    pub(super) alerts: Vec<WebChainNodeObservabilityAlert>,
}

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
}
