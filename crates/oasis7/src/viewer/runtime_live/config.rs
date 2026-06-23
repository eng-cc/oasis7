use std::io;
use std::time::Duration;

use super::{RuntimeWorldError, ViewerLiveDecisionMode, WorldScenario};

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
