use std::io;
use std::path::PathBuf;
use std::time::Duration;

use super::{RuntimeWorldError, ViewerLiveDecisionMode, WorldScenario};
use crate::runtime::MajorWorldEventVisibilityPermission;

pub(crate) const DEFAULT_PROMPT_RESULT_CACHE_CAPACITY: usize = 256;
pub(crate) const DEFAULT_PROMPT_RESULT_RECEIPT_MAX_BYTES: usize = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainLinkPolicy {
    Enforcing,
    Shadow,
}

impl ChainLinkPolicy {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim() {
            "enforcing" => Some(Self::Enforcing),
            "shadow" => Some(Self::Shadow),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Enforcing => "enforcing",
            Self::Shadow => "shadow",
        }
    }

    pub(super) fn records_player_facing_chain_failures(self) -> bool {
        matches!(self, Self::Enforcing)
    }
}

#[derive(Debug, Clone)]
pub struct ViewerRuntimeLiveServerConfig {
    pub bind_addr: String,
    pub scenario: Option<WorldScenario>,
    pub world_id: String,
    pub decision_mode: ViewerLiveDecisionMode,
    pub play_step_interval: Duration,
    pub chain_poll_interval: Duration,
    pub auto_play_on_connect: bool,
    pub hosted_public_join_mode: bool,
    pub chain_status_bind: Option<String>,
    pub chain_submit_bind: Option<String>,
    pub chain_link_policy: ChainLinkPolicy,
    pub agent_chat_echo_enabled: bool,
    /// Explicit operator/session audience decision for Major World Events.
    /// Defaults to Unknown; authentication, control, and Director access do
    /// not implicitly grant event visibility.
    pub major_world_event_visibility: MajorWorldEventVisibilityPermission,
    pub generated_world_dir: Option<PathBuf>,
    /// Optional durable provider lineage path. Generated worlds continue to
    /// default beside their sidecar; formal/synthetic worlds must opt into a
    /// stable operator-owned path explicitly.
    pub provider_lineage_store: Option<PathBuf>,
    /// Runtime-only idempotency ledger capacity. A zero value is invalid.
    pub prompt_result_cache_capacity: usize,
    /// Maximum serialized result receipt size. A zero value is invalid.
    pub prompt_result_receipt_max_bytes: usize,
    #[cfg(test)]
    pub(crate) test_cognition_runtime_binding: Option<(String, u64, Option<String>, String, u64)>,
}

impl ViewerRuntimeLiveServerConfig {
    pub(crate) fn validate_prompt_result_limits(&self) -> Result<(), String> {
        if self.prompt_result_cache_capacity == 0 {
            return Err("prompt result cache capacity must be greater than zero".to_string());
        }
        if self.prompt_result_receipt_max_bytes == 0 {
            return Err("prompt result receipt max bytes must be greater than zero".to_string());
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum ViewerRuntimeLiveServerError {
    Io(io::Error),
    Serde(String),
    Init(String),
    Runtime(RuntimeWorldError),
}

impl From<io::Error> for ViewerRuntimeLiveServerError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<RuntimeWorldError> for ViewerRuntimeLiveServerError {
    fn from(err: RuntimeWorldError) -> Self {
        Self::Runtime(err)
    }
}
