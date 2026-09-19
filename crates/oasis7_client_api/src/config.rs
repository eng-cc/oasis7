use serde::{Deserialize, Serialize};

use crate::bootstrap::BootstrapConfig;

pub const AGENT_DECISION_SOURCE_BUILTIN_LLM: &str = "builtin_llm";
pub const AGENT_DECISION_SOURCE_PROVIDER_BACKED: &str = "provider_backed";
pub const AGENT_PROVIDER_BACKEND_LOCAL_BRIDGE: &str = "provider_local_bridge";
pub const AGENT_PROVIDER_CONTRACT_WORLDSIM_V1: &str = "worldsim_provider_v1";
pub const AGENT_PROVIDER_TRANSPORT_LOOPBACK_HTTP: &str = "loopback_http";
pub const AGENT_PROVIDER_TRANSPORT_REMOTE_HTTPS: &str = "remote_https";
pub const AGENT_PROVIDER_MODE_PROVIDER_LOOPBACK_HTTP_ALIAS: &str = "provider_loopback_http";
pub const AGENT_PROVIDER_MODE_DIRECT_CONNECT_ALIAS: &str = "agent_direct_connect";
pub const DEFAULT_AGENT_PROVIDER_URL: &str = "http://127.0.0.1:5841";
pub const DEFAULT_AGENT_PROVIDER_CONNECT_TIMEOUT_MS: &str = "15000";
pub const DEFAULT_AGENT_PROVIDER_PROFILE: &str = "oasis7_p0_low_freq_npc";
pub const AGENT_EXECUTION_LANE_PLAYER_PARITY: &str = "player_parity";
pub const AGENT_EXECUTION_LANE_HEADLESS_AGENT: &str = "headless_agent";

/// Provider settings shared by native and browser clients.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfig {
    #[serde(default = "default_decision_source")]
    pub decision_source: String,
    #[serde(default = "default_provider_backend")]
    pub backend: String,
    #[serde(default = "default_provider_contract")]
    pub contract: String,
    #[serde(default = "default_provider_transport")]
    pub transport: String,
    #[serde(default = "default_provider_url")]
    pub url: String,
    #[serde(default)]
    pub auth_token: String,
    #[serde(default = "default_provider_timeout")]
    pub connect_timeout_ms: String,
    #[serde(default = "default_execution_lane")]
    pub execution_lane: String,
    #[serde(default = "default_provider_profile")]
    pub profile: String,
    #[serde(default = "default_provider_auto_discover")]
    pub auto_discover: bool,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            decision_source: default_decision_source(),
            backend: default_provider_backend(),
            contract: default_provider_contract(),
            transport: default_provider_transport(),
            url: default_provider_url(),
            auth_token: String::new(),
            connect_timeout_ms: default_provider_timeout(),
            execution_lane: default_execution_lane(),
            profile: default_provider_profile(),
            auto_discover: default_provider_auto_discover(),
        }
    }
}

impl ProviderConfig {
    pub fn canonical_decision_source(&self) -> Option<&'static str> {
        canonical_decision_source(self.decision_source.as_str())
    }

    pub fn canonical_backend(&self) -> Option<&'static str> {
        canonical_provider_backend(self.backend.as_str())
    }

    pub fn canonical_contract(&self) -> Option<&'static str> {
        canonical_provider_contract(self.contract.as_str())
    }

    pub fn canonical_transport(&self) -> Option<&'static str> {
        canonical_provider_transport(self.transport.as_str())
    }

    pub fn canonical_execution_lane(&self) -> Option<&'static str> {
        canonical_execution_lane(self.execution_lane.as_str())
    }

    pub fn effective_url(&self) -> Option<&str> {
        let url = self.url.trim();
        if url.is_empty() && self.auto_discover {
            Some(DEFAULT_AGENT_PROVIDER_URL)
        } else if url.is_empty() {
            None
        } else {
            Some(url)
        }
    }

    pub fn connect_timeout_millis(&self) -> Result<u64, String> {
        self.connect_timeout_ms
            .trim()
            .parse::<u64>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| "agent provider connect timeout ms must be a positive integer".into())
    }
}

/// Flat client configuration matching the launcher-facing field names. The
/// launcher keeps its own compatibility wrapper until the A2/A3 slices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientConfig {
    #[serde(default = "default_decision_source")]
    pub agent_decision_source: String,
    #[serde(default = "default_provider_backend")]
    pub agent_provider_backend: String,
    #[serde(default = "default_provider_contract")]
    pub agent_provider_contract: String,
    #[serde(default = "default_provider_transport")]
    pub agent_provider_transport: String,
    #[serde(default = "default_provider_url")]
    pub agent_provider_url: String,
    #[serde(default)]
    pub agent_provider_auth_token: String,
    #[serde(default = "default_provider_timeout")]
    pub agent_provider_connect_timeout_ms: String,
    #[serde(default = "default_execution_lane")]
    pub agent_execution_lane: String,
    #[serde(default = "default_provider_profile")]
    pub agent_provider_profile: String,
    #[serde(default = "default_provider_auto_discover")]
    pub provider_auto_discover: bool,
    #[serde(default = "default_chain_enabled")]
    pub chain_enabled: bool,
    #[serde(default = "default_network_tier")]
    pub chain_network_tier: String,
    #[serde(default)]
    pub chain_network_tier_manifest: String,
    #[serde(default)]
    pub chain_world_id: String,
    #[serde(default = "default_bootstrap_peers_csv")]
    pub chain_replication_bootstrap_peers: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            agent_decision_source: default_decision_source(),
            agent_provider_backend: default_provider_backend(),
            agent_provider_contract: default_provider_contract(),
            agent_provider_transport: default_provider_transport(),
            agent_provider_url: default_provider_url(),
            agent_provider_auth_token: String::new(),
            agent_provider_connect_timeout_ms: default_provider_timeout(),
            agent_execution_lane: default_execution_lane(),
            agent_provider_profile: default_provider_profile(),
            provider_auto_discover: default_provider_auto_discover(),
            chain_enabled: false,
            chain_network_tier: default_network_tier(),
            chain_network_tier_manifest: String::new(),
            chain_world_id: String::new(),
            chain_replication_bootstrap_peers: default_bootstrap_peers_csv(),
        }
    }
}

impl ClientConfig {
    pub fn provider_config(&self) -> ProviderConfig {
        ProviderConfig {
            decision_source: self.agent_decision_source.clone(),
            backend: self.agent_provider_backend.clone(),
            contract: self.agent_provider_contract.clone(),
            transport: self.agent_provider_transport.clone(),
            url: self.agent_provider_url.clone(),
            auth_token: self.agent_provider_auth_token.clone(),
            connect_timeout_ms: self.agent_provider_connect_timeout_ms.clone(),
            execution_lane: self.agent_execution_lane.clone(),
            profile: self.agent_provider_profile.clone(),
            auto_discover: self.provider_auto_discover,
        }
    }

    pub fn bootstrap_config(&self) -> BootstrapConfig {
        BootstrapConfig {
            network_tier: self.chain_network_tier.clone(),
            network_tier_manifest: self.chain_network_tier_manifest.clone(),
            world_id: self.chain_world_id.clone(),
            replication_bootstrap_peers: self
                .chain_replication_bootstrap_peers
                .split([',', ';', ' ', '\n', '\r', '\t'])
                .filter(|peer| !peer.trim().is_empty())
                .map(|peer| peer.trim().to_string())
                .collect(),
        }
    }

    pub fn normalized(&self) -> Self {
        let mut normalized = self.clone();
        normalized.agent_decision_source = normalized.agent_decision_source.trim().into();
        normalized.agent_provider_backend = normalized.agent_provider_backend.trim().into();
        normalized.agent_provider_contract = normalized.agent_provider_contract.trim().into();
        normalized.agent_provider_transport = normalized.agent_provider_transport.trim().into();
        normalized.agent_provider_url = normalized.agent_provider_url.trim().into();
        normalized.agent_provider_auth_token = normalized.agent_provider_auth_token.trim().into();
        normalized.agent_provider_connect_timeout_ms =
            normalized.agent_provider_connect_timeout_ms.trim().into();
        normalized.agent_execution_lane = normalized.agent_execution_lane.trim().into();
        normalized.agent_provider_profile = normalized.agent_provider_profile.trim().into();
        normalized.chain_network_tier = normalized.chain_network_tier.trim().into();
        normalized.chain_network_tier_manifest =
            normalized.chain_network_tier_manifest.trim().into();
        normalized.chain_world_id = normalized.chain_world_id.trim().into();
        normalized.chain_replication_bootstrap_peers = normalized
            .bootstrap_config()
            .replication_bootstrap_peers
            .join(",");
        normalized
    }
}

pub fn canonical_decision_source(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        AGENT_DECISION_SOURCE_BUILTIN_LLM => Some(AGENT_DECISION_SOURCE_BUILTIN_LLM),
        AGENT_DECISION_SOURCE_PROVIDER_BACKED
        | AGENT_PROVIDER_MODE_PROVIDER_LOOPBACK_HTTP_ALIAS
        | AGENT_PROVIDER_MODE_DIRECT_CONNECT_ALIAS => Some(AGENT_DECISION_SOURCE_PROVIDER_BACKED),
        _ => None,
    }
}

pub fn canonical_provider_backend(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        AGENT_PROVIDER_BACKEND_LOCAL_BRIDGE
        | AGENT_PROVIDER_MODE_PROVIDER_LOOPBACK_HTTP_ALIAS
        | AGENT_PROVIDER_MODE_DIRECT_CONNECT_ALIAS => Some(AGENT_PROVIDER_BACKEND_LOCAL_BRIDGE),
        _ => None,
    }
}

pub fn canonical_provider_contract(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        AGENT_PROVIDER_CONTRACT_WORLDSIM_V1
        | AGENT_PROVIDER_MODE_PROVIDER_LOOPBACK_HTTP_ALIAS
        | AGENT_PROVIDER_MODE_DIRECT_CONNECT_ALIAS => Some(AGENT_PROVIDER_CONTRACT_WORLDSIM_V1),
        _ => None,
    }
}

pub fn canonical_provider_transport(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        AGENT_PROVIDER_TRANSPORT_LOOPBACK_HTTP
        | AGENT_PROVIDER_MODE_PROVIDER_LOOPBACK_HTTP_ALIAS
        | AGENT_PROVIDER_MODE_DIRECT_CONNECT_ALIAS => Some(AGENT_PROVIDER_TRANSPORT_LOOPBACK_HTTP),
        AGENT_PROVIDER_TRANSPORT_REMOTE_HTTPS => Some(AGENT_PROVIDER_TRANSPORT_REMOTE_HTTPS),
        _ => None,
    }
}

pub fn canonical_execution_lane(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        AGENT_EXECUTION_LANE_PLAYER_PARITY | "player-parity" | "player" => {
            Some(AGENT_EXECUTION_LANE_PLAYER_PARITY)
        }
        AGENT_EXECUTION_LANE_HEADLESS_AGENT | "headless-agent" | "headless" => {
            Some(AGENT_EXECUTION_LANE_HEADLESS_AGENT)
        }
        _ => None,
    }
}

fn default_decision_source() -> String {
    AGENT_DECISION_SOURCE_BUILTIN_LLM.into()
}
fn default_provider_backend() -> String {
    AGENT_PROVIDER_BACKEND_LOCAL_BRIDGE.into()
}
fn default_provider_contract() -> String {
    AGENT_PROVIDER_CONTRACT_WORLDSIM_V1.into()
}
fn default_provider_transport() -> String {
    AGENT_PROVIDER_TRANSPORT_LOOPBACK_HTTP.into()
}
fn default_provider_url() -> String {
    DEFAULT_AGENT_PROVIDER_URL.into()
}
fn default_provider_timeout() -> String {
    DEFAULT_AGENT_PROVIDER_CONNECT_TIMEOUT_MS.into()
}
fn default_execution_lane() -> String {
    AGENT_EXECUTION_LANE_PLAYER_PARITY.into()
}
fn default_provider_profile() -> String {
    DEFAULT_AGENT_PROVIDER_PROFILE.into()
}
fn default_provider_auto_discover() -> bool {
    true
}
fn default_chain_enabled() -> bool {
    false
}
fn default_network_tier() -> String {
    crate::bootstrap::DEFAULT_CHAIN_NETWORK_TIER.into()
}
fn default_bootstrap_peers_csv() -> String {
    crate::bootstrap::default_chain_replication_bootstrap_peers_csv()
}
