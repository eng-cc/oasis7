use serde::{Deserialize, Serialize};

use super::{explorer_window, transfer_window, LaunchConfig};

#[derive(Debug, Clone)]
pub(crate) enum WebApiEvent {
    State(Result<WebStateSnapshot, String>),
    Action(Result<WebApiResponse, String>),
    #[cfg(target_arch = "wasm32")]
    Feedback(Result<WebFeedbackSubmitResponse, String>),
    Transfer(Result<WebTransferSubmitResponse, String>),
    TransferQuery(Result<transfer_window::TransferQueryResponse, String>),
    ExplorerQuery(Result<explorer_window::ExplorerQueryResponse, String>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WebRequestDomain {
    StatePoll,
    ControlAction,
    #[cfg(target_arch = "wasm32")]
    FeedbackSubmit,
    TransferSubmit,
    TransferQuery,
    ExplorerQuery,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct WebRequestInflight {
    pub(crate) state_poll: bool,
    pub(crate) control_action: bool,
    #[cfg(target_arch = "wasm32")]
    pub(crate) feedback_submit: bool,
    pub(crate) transfer_submit: bool,
    pub(crate) transfer_query: bool,
    pub(crate) explorer_query: bool,
}

impl WebRequestInflight {
    #[cfg(all(test, not(target_arch = "wasm32")))]
    pub(crate) fn any(self) -> bool {
        self.state_poll
            || self.control_action
            || self.transfer_submit
            || self.transfer_query
            || self.explorer_query
    }

    #[cfg(all(test, target_arch = "wasm32"))]
    pub(crate) fn any(self) -> bool {
        self.state_poll
            || self.control_action
            || self.feedback_submit
            || self.transfer_submit
            || self.transfer_query
            || self.explorer_query
    }

    pub(crate) fn transfer_any(self) -> bool {
        self.transfer_submit || self.transfer_query
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(crate) struct WebChainRecoverySnapshot {
    pub(crate) error_code: String,
    pub(crate) reason: String,
    pub(crate) node_id: String,
    pub(crate) execution_world_dir: String,
    pub(crate) recovery_mode: String,
    pub(crate) reset_required: bool,
    pub(crate) fresh_node_id: String,
    pub(crate) fresh_chain_status_bind: String,
    pub(crate) suggested_config: LaunchConfig,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(crate) struct WebChainP2pStatus {
    pub(crate) requested_user_mode: String,
    pub(crate) recommended_user_mode: String,
    pub(crate) effective_user_mode: String,
    pub(crate) applied_effective_user_mode: Option<String>,
    pub(crate) requires_explicit_public_entry_confirmation: bool,
    pub(crate) detected_reachability: Option<String>,
    pub(crate) hole_punch_viability: String,
    pub(crate) relay_available: bool,
    pub(crate) probe_stable: bool,
    pub(crate) deployment_mode: String,
    pub(crate) node_role_claim: String,
    pub(crate) rationale: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(crate) struct WebChainNodeObservabilityAlert {
    pub(crate) severity: String,
    pub(crate) code: String,
    pub(crate) summary: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub(crate) struct WebChainNodeObservabilityStatus {
    pub(crate) status: String,
    pub(crate) summary: String,
    pub(crate) connected_peer_count: usize,
    pub(crate) active_peer_count: usize,
    pub(crate) candidate_peer_count: usize,
    pub(crate) suspect_peer_count: usize,
    pub(crate) blocked_peer_count: usize,
    pub(crate) peer_with_issues_count: usize,
    pub(crate) known_peer_heads: usize,
    pub(crate) network_height_lag: u64,
    pub(crate) recent_replication_error_count: usize,
    pub(crate) storage_degraded: bool,
    pub(crate) reward_runtime_degraded: bool,
    pub(crate) alerts: Vec<WebChainNodeObservabilityAlert>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WebStateSnapshot {
    pub(crate) status: String,
    pub(crate) detail: Option<String>,
    pub(crate) chain_status: String,
    pub(crate) chain_detail: Option<String>,
    pub(crate) chain_p2p_status: Option<WebChainP2pStatus>,
    pub(crate) chain_observability_status: Option<WebChainNodeObservabilityStatus>,
    pub(crate) chain_recovery: Option<WebChainRecoverySnapshot>,
    pub(crate) game_url: String,
    pub(crate) config: LaunchConfig,
    pub(crate) logs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WebApiResponse {
    pub(crate) ok: bool,
    pub(crate) error: Option<String>,
    pub(crate) state: WebStateSnapshot,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Serialize)]
pub(crate) struct WebFeedbackSubmitRequest {
    pub(crate) category: String,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) platform: String,
    pub(crate) game_version: String,
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WebFeedbackSubmitResponse {
    pub(crate) ok: bool,
    pub(crate) feedback_id: Option<String>,
    pub(crate) event_id: Option<String>,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct WebTransferSubmitRequest {
    pub(crate) from_account_id: String,
    pub(crate) to_account_id: String,
    pub(crate) amount: u64,
    pub(crate) nonce: u64,
    pub(crate) public_key: String,
    pub(crate) signature: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WebTransferSubmitResponse {
    pub(crate) ok: bool,
    pub(crate) action_id: Option<u64>,
    pub(crate) submitted_at_unix_ms: Option<i64>,
    pub(crate) lifecycle_status: Option<transfer_window::WebTransferLifecycleStatus>,
    pub(crate) error_code: Option<String>,
    pub(crate) error: Option<String>,
}

pub(crate) fn encode_query_value(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push(hex_upper(byte >> 4));
            encoded.push(hex_upper(byte & 0x0f));
        }
    }
    encoded
}

pub(crate) fn encoded_query_pair(key: &str, value: &str) -> String {
    format!("{key}={}", encode_query_value(value))
}

fn hex_upper(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'A' + (nibble - 10)) as char,
        _ => unreachable!("nibble must be in 0..=15"),
    }
}
