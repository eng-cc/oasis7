use super::*;
#[cfg(not(target_arch = "wasm32"))]
use oasis7::simulator::{evaluate_provider_compatibility, ProviderHealth, ProviderInfo};
#[cfg(not(target_arch = "wasm32"))]
use serde::de::DeserializeOwned;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{Read, Write};
#[cfg(not(target_arch = "wasm32"))]
use std::net::{TcpStream, ToSocketAddrs};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

#[cfg(not(target_arch = "wasm32"))]
const OASIS7_GAME_STATIC_DIR_ENV: &str = "OASIS7_GAME_STATIC_DIR";
#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_VIEWER_STATIC_DIR: &str = "web";
pub(super) const PROVIDER_LOOPBACK_HTTP_IMPLEMENTATION: &str = "provider_loopback_http";
pub(super) const BUILTIN_LLM_DECISION_SOURCE: &str = "builtin_llm";
pub(super) const PROVIDER_BACKED_DECISION_SOURCE: &str = "provider_backed";
pub(super) const LOCAL_BRIDGE_PROVIDER_BACKEND: &str = "provider_local_bridge";
pub(super) const WORLDSIM_PROVIDER_CONTRACT: &str = "worldsim_provider_v1";
pub(super) const LOOPBACK_HTTP_PROVIDER_TRANSPORT: &str = "loopback_http";
pub(super) const AGENT_DIRECT_CONNECT_PROVIDER_MODE_ALIAS: &str = "agent_direct_connect";
pub(super) const DEFAULT_PROVIDER_DISCOVERY_BASE_URL: &str = DEFAULT_AGENT_PROVIDER_URL;

#[path = "launcher_core_http.rs"]
mod http_support;
#[cfg(all(not(target_arch = "wasm32"), test))]
pub(crate) use self::http_support::probe_chain_status_endpoint;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use self::http_support::{
    check_provider_loopback_http_provider, normalize_host_for_connect,
};
pub(crate) use self::http_support::{host_for_url, normalize_host_for_url, parse_http_base_url};

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Deserialize)]
struct ProviderInfoResponse {
    provider_id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    protocol_version: Option<String>,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    supported_action_sets: Vec<String>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, Deserialize)]
struct ProviderHealthResponse {
    ok: bool,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    last_error: Option<String>,
    #[serde(default)]
    queue_depth: Option<u64>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ProviderCheckError {
    InvalidConfig(String),
    Unauthorized(String),
    Unreachable(String),
}

#[cfg(not(target_arch = "wasm32"))]
impl std::fmt::Display for ProviderCheckError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig(detail)
            | Self::Unauthorized(detail)
            | Self::Unreachable(detail) => write!(f, "{detail}"),
        }
    }
}

pub(super) fn is_provider_loopback_http_mode(config: &LaunchConfig) -> bool {
    canonical_agent_decision_source(config.agent_decision_source.as_str())
        == Some(PROVIDER_BACKED_DECISION_SOURCE)
        && canonical_agent_provider_backend(config.agent_provider_backend.as_str())
            == Some(LOCAL_BRIDGE_PROVIDER_BACKEND)
        && canonical_agent_provider_contract(config.agent_provider_contract.as_str())
            == Some(WORLDSIM_PROVIDER_CONTRACT)
        && canonical_agent_provider_transport(config.agent_provider_transport.as_str())
            == Some(LOOPBACK_HTTP_PROVIDER_TRANSPORT)
}

pub(super) fn validate_agent_decision_source(raw: &str) -> Result<(), String> {
    canonical_agent_decision_source(raw)
        .map(|_| ())
        .ok_or_else(|| "agent decision source must be builtin_llm or provider_backed".to_string())
}

pub(super) fn validate_agent_provider_backend(raw: &str) -> Result<(), String> {
    canonical_agent_provider_backend(raw)
        .map(|_| ())
        .ok_or_else(|| "agent provider backend must be provider_local_bridge".to_string())
}

pub(super) fn validate_agent_provider_contract(raw: &str) -> Result<(), String> {
    canonical_agent_provider_contract(raw)
        .map(|_| ())
        .ok_or_else(|| "agent provider contract must be worldsim_provider_v1".to_string())
}

pub(super) fn validate_agent_provider_transport(raw: &str) -> Result<(), String> {
    canonical_agent_provider_transport(raw)
        .map(|_| ())
        .ok_or_else(|| "agent provider transport must be loopback_http".to_string())
}

pub(super) fn canonical_provider_execution_mode(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "player_parity" | "player-parity" | "player" => Some("player_parity"),
        "headless_agent" | "headless-agent" | "headless" => Some("headless_agent"),
        _ => None,
    }
}

pub(super) fn validate_provider_execution_mode(raw: &str) -> Result<(), String> {
    canonical_provider_execution_mode(raw)
        .map(|_| ())
        .ok_or_else(|| "agent execution lane must be player_parity or headless_agent".to_string())
}

pub(super) fn canonical_agent_decision_source(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        BUILTIN_LLM_DECISION_SOURCE => Some(BUILTIN_LLM_DECISION_SOURCE),
        PROVIDER_BACKED_DECISION_SOURCE
        | PROVIDER_LOOPBACK_HTTP_IMPLEMENTATION
        | AGENT_DIRECT_CONNECT_PROVIDER_MODE_ALIAS => Some(PROVIDER_BACKED_DECISION_SOURCE),
        _ => None,
    }
}

pub(super) fn canonical_agent_provider_backend(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        "" => None,
        LOCAL_BRIDGE_PROVIDER_BACKEND
        | PROVIDER_LOOPBACK_HTTP_IMPLEMENTATION
        | AGENT_DIRECT_CONNECT_PROVIDER_MODE_ALIAS => Some(LOCAL_BRIDGE_PROVIDER_BACKEND),
        _ => None,
    }
}

pub(super) fn canonical_agent_provider_contract(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        "" => None,
        WORLDSIM_PROVIDER_CONTRACT
        | PROVIDER_LOOPBACK_HTTP_IMPLEMENTATION
        | AGENT_DIRECT_CONNECT_PROVIDER_MODE_ALIAS => Some(WORLDSIM_PROVIDER_CONTRACT),
        _ => None,
    }
}

pub(super) fn canonical_agent_provider_transport(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        "" => None,
        LOOPBACK_HTTP_PROVIDER_TRANSPORT
        | PROVIDER_LOOPBACK_HTTP_IMPLEMENTATION
        | AGENT_DIRECT_CONNECT_PROVIDER_MODE_ALIAS => Some(LOOPBACK_HTTP_PROVIDER_TRANSPORT),
        _ => None,
    }
}

pub(super) fn canonical_chain_p2p_user_mode(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "auto_join" | "auto-join" | "auto" => Some("auto_join"),
        "private_safe" | "private-safe" | "private" => Some("private_safe"),
        "public_entry" | "public-entry" | "public" => Some("public_entry"),
        _ => None,
    }
}

pub(super) fn validate_chain_p2p_user_mode(raw: &str) -> Result<(), String> {
    canonical_chain_p2p_user_mode(raw)
        .map(|_| ())
        .ok_or_else(|| {
            "chain P2P user mode must be one of: auto_join, private_safe, public_entry".to_string()
        })
}

pub(super) fn effective_provider_base_url(config: &LaunchConfig) -> Result<String, String> {
    let base_url = config.agent_provider_url.trim();
    if !base_url.is_empty() {
        return Ok(base_url.to_string());
    }
    if config.provider_auto_discover {
        return Ok(DEFAULT_PROVIDER_DISCOVERY_BASE_URL.to_string());
    }
    Err("provider base url is required when auto-discover is disabled".to_string())
}

pub(super) fn parse_agent_provider_connect_timeout_ms(
    config: &LaunchConfig,
) -> Result<u64, String> {
    parse_positive_u64(
        config.agent_provider_connect_timeout_ms.as_str(),
        "agent provider connect timeout ms",
    )
}

pub(super) fn validate_provider_base_url(base_url: &str) -> Result<(String, u16), String> {
    let (host, port) = parse_http_base_url(base_url, "provider base url")?;
    if !is_loopback_host(host.as_str()) {
        return Err(
            "provider base url must use a loopback host (127.0.0.1 / localhost / ::1)".to_string(),
        );
    }
    Ok((host, port))
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_viewer_static_dir_candidate_for_launcher(
    raw: &str,
    launcher_bin: &str,
) -> Option<std::path::PathBuf> {
    let user_path = std::path::PathBuf::from(raw);
    if user_path.is_dir() {
        return Some(user_path);
    }

    if user_path.is_relative() {
        let launcher_bin = launcher_bin.trim();
        if !launcher_bin.is_empty() {
            if let Some(bin_dir) = Path::new(launcher_bin).parent() {
                let sibling_candidate = bin_dir.join("..").join(&user_path);
                if sibling_candidate.is_dir() {
                    return Some(sibling_candidate);
                }
            }
        }
    }

    None
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_viewer_static_dir_for_launcher(
    raw: &str,
    launcher_bin: &str,
) -> Option<std::path::PathBuf> {
    if raw == DEFAULT_VIEWER_STATIC_DIR {
        if let Some((_, override_path)) = read_named_env_value(&[OASIS7_GAME_STATIC_DIR_ENV]) {
            return resolve_viewer_static_dir_candidate_for_launcher(
                override_path.as_str(),
                launcher_bin,
            );
        }
    }

    if let Some(dir) = resolve_viewer_static_dir_candidate_for_launcher(raw, launcher_bin) {
        return Some(dir);
    }

    if raw == DEFAULT_VIEWER_STATIC_DIR {
        if let Some(dev_fallback) = platform_ops::viewer_dev_dist_candidates()
            .into_iter()
            .find(|candidate| candidate.is_dir())
        {
            return Some(dev_fallback);
        }
    }

    None
}

pub(super) fn launcher_text_field_mut<'a>(
    config: &'a mut LaunchConfig,
    field_id: &str,
) -> Option<&'a mut String> {
    match field_id {
        "scenario" => Some(&mut config.scenario),
        "live_bind" => Some(&mut config.live_bind),
        "web_bind" => Some(&mut config.web_bind),
        "viewer_host" => Some(&mut config.viewer_host),
        "viewer_port" => Some(&mut config.viewer_port),
        "agent_decision_source" | "agent_provider_mode" => Some(&mut config.agent_decision_source),
        "agent_provider_backend" => Some(&mut config.agent_provider_backend),
        "agent_provider_contract" => Some(&mut config.agent_provider_contract),
        "agent_provider_transport" => Some(&mut config.agent_provider_transport),
        "agent_provider_url" => Some(&mut config.agent_provider_url),
        "agent_provider_auth_token" => Some(&mut config.agent_provider_auth_token),
        "agent_provider_connect_timeout_ms" => Some(&mut config.agent_provider_connect_timeout_ms),
        "agent_execution_lane" => Some(&mut config.agent_execution_lane),
        "agent_provider_profile" => Some(&mut config.agent_provider_profile),
        "chain_status_bind" => Some(&mut config.chain_status_bind),
        "chain_node_id" => Some(&mut config.chain_node_id),
        "chain_world_id" => Some(&mut config.chain_world_id),
        "chain_node_role" => Some(&mut config.chain_node_role),
        "chain_p2p_user_mode" => Some(&mut config.chain_p2p_user_mode),
        "chain_replication_bootstrap_peers" => Some(&mut config.chain_replication_bootstrap_peers),
        "chain_node_tick_ms" => Some(&mut config.chain_node_tick_ms),
        "chain_pos_slot_duration_ms" => Some(&mut config.chain_pos_slot_duration_ms),
        "chain_pos_ticks_per_slot" => Some(&mut config.chain_pos_ticks_per_slot),
        "chain_pos_proposal_tick_phase" => Some(&mut config.chain_pos_proposal_tick_phase),
        "chain_pos_slot_clock_genesis_unix_ms" => {
            Some(&mut config.chain_pos_slot_clock_genesis_unix_ms)
        }
        "chain_pos_max_past_slot_lag" => Some(&mut config.chain_pos_max_past_slot_lag),
        "chain_node_validators" => Some(&mut config.chain_node_validators),
        "launcher_bin" => Some(&mut config.launcher_bin),
        "chain_runtime_bin" => Some(&mut config.chain_runtime_bin),
        "viewer_static_dir" => Some(&mut config.viewer_static_dir),
        _ => None,
    }
}

pub(super) fn launcher_checkbox_field_mut<'a>(
    config: &'a mut LaunchConfig,
    field_id: &str,
) -> Option<&'a mut bool> {
    match field_id {
        "llm_enabled" => Some(&mut config.llm_enabled),
        "provider_auto_discover" => Some(&mut config.provider_auto_discover),
        "chain_enabled" => Some(&mut config.chain_enabled),
        "chain_p2p_accept_public_entry" => Some(&mut config.chain_p2p_accept_public_entry),
        "chain_pos_adaptive_tick_scheduler_enabled" => {
            Some(&mut config.chain_pos_adaptive_tick_scheduler_enabled)
        }
        "auto_open_browser" => Some(&mut config.auto_open_browser),
        _ => None,
    }
}

pub(super) fn collect_required_config_issues(config: &LaunchConfig) -> Vec<ConfigIssue> {
    let mut issues = Vec::new();

    if !config.llm_enabled {
        issues.push(ConfigIssue::LlmRequired);
    }

    if validate_agent_decision_source(config.agent_decision_source.as_str()).is_err() {
        issues.push(ConfigIssue::AgentProviderModeInvalid);
    }

    if is_provider_loopback_http_mode(config) {
        if validate_agent_provider_backend(config.agent_provider_backend.as_str()).is_err()
            || validate_agent_provider_contract(config.agent_provider_contract.as_str()).is_err()
            || validate_agent_provider_transport(config.agent_provider_transport.as_str()).is_err()
        {
            issues.push(ConfigIssue::AgentProviderModeInvalid);
        }
        if effective_provider_base_url(config).is_err() {
            issues.push(ConfigIssue::ProviderBaseUrlRequired);
        } else if let Ok(base_url) = effective_provider_base_url(config) {
            match validate_provider_base_url(base_url.as_str()) {
                Ok(_) => {}
                Err(err) if err.contains("loopback") => {
                    issues.push(ConfigIssue::ProviderBaseUrlLoopbackRequired);
                }
                Err(_) => issues.push(ConfigIssue::ProviderBaseUrlInvalid),
            }
        }
        if parse_agent_provider_connect_timeout_ms(config).is_err() {
            issues.push(ConfigIssue::ProviderConnectTimeoutMsInvalid);
        }
        if validate_provider_execution_mode(config.agent_execution_lane.as_str()).is_err() {
            issues.push(ConfigIssue::ProviderExecutionModeInvalid);
        }
        if config.agent_provider_profile.trim().is_empty() {
            issues.push(ConfigIssue::ProviderProfileRequired);
        }
    }

    if config.scenario.trim().is_empty() {
        issues.push(ConfigIssue::ScenarioRequired);
    }
    if parse_host_port(config.live_bind.as_str(), "live bind").is_err() {
        issues.push(ConfigIssue::LiveBindInvalid);
    }
    if parse_host_port(config.web_bind.as_str(), "web bind").is_err() {
        issues.push(ConfigIssue::WebBindInvalid);
    }
    if config.viewer_host.trim().is_empty() {
        issues.push(ConfigIssue::ViewerHostRequired);
    }
    if parse_port(config.viewer_port.as_str(), "viewer port").is_err() {
        issues.push(ConfigIssue::ViewerPortInvalid);
    }

    let viewer_static_dir = config.viewer_static_dir.trim();
    if viewer_static_dir.is_empty() {
        issues.push(ConfigIssue::ViewerStaticDirRequired);
    }
    #[cfg(not(target_arch = "wasm32"))]
    if !viewer_static_dir.is_empty()
        && resolve_viewer_static_dir_for_launcher(viewer_static_dir, config.launcher_bin.as_str())
            .is_none()
    {
        issues.push(ConfigIssue::ViewerStaticDirMissing);
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let launcher_bin = config.launcher_bin.trim();
        if launcher_bin.is_empty() {
            issues.push(ConfigIssue::LauncherBinRequired);
        } else if !Path::new(launcher_bin).is_file() {
            issues.push(ConfigIssue::LauncherBinMissing);
        }
    }

    issues
}

pub(super) fn collect_chain_required_config_issues(config: &LaunchConfig) -> Vec<ConfigIssue> {
    let mut issues = Vec::new();
    if !config.chain_enabled {
        return issues;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let chain_runtime_bin = config.chain_runtime_bin.trim();
        if chain_runtime_bin.is_empty() {
            issues.push(ConfigIssue::ChainRuntimeBinRequired);
        } else if !Path::new(chain_runtime_bin).is_file() {
            issues.push(ConfigIssue::ChainRuntimeBinMissing);
        }
    }

    if parse_host_port(config.chain_status_bind.as_str(), "chain status bind").is_err() {
        issues.push(ConfigIssue::ChainStatusBindInvalid);
    }
    if config.chain_node_id.trim().is_empty() {
        issues.push(ConfigIssue::ChainNodeIdRequired);
    }
    if parse_chain_role(config.chain_node_role.as_str()).is_err() {
        issues.push(ConfigIssue::ChainRoleInvalid);
    }
    if validate_chain_p2p_user_mode(config.chain_p2p_user_mode.as_str()).is_err() {
        issues.push(ConfigIssue::ChainP2pUserModeInvalid);
    }
    if canonical_chain_p2p_user_mode(config.chain_p2p_user_mode.as_str()) == Some("public_entry")
        && !config.chain_p2p_accept_public_entry
    {
        issues.push(ConfigIssue::ChainPublicEntryConfirmationRequired);
    }
    if parse_chain_replication_bootstrap_peers(config.chain_replication_bootstrap_peers.as_str())
        .is_err()
    {
        issues.push(ConfigIssue::ChainReplicationBootstrapPeersInvalid);
    }
    if parse_positive_u64(
        config.chain_node_tick_ms.as_str(),
        "chain node poll interval ms",
    )
    .is_err()
    {
        issues.push(ConfigIssue::ChainTickMsInvalid);
    }
    if parse_positive_u64(
        config.chain_pos_slot_duration_ms.as_str(),
        "chain pos slot duration ms",
    )
    .is_err()
    {
        issues.push(ConfigIssue::ChainPosSlotDurationMsInvalid);
    }
    let ticks_per_slot = parse_positive_u64(
        config.chain_pos_ticks_per_slot.as_str(),
        "chain pos ticks per slot",
    );
    if ticks_per_slot.is_err() {
        issues.push(ConfigIssue::ChainPosTicksPerSlotInvalid);
    }
    let proposal_tick_phase = parse_non_negative_u64(
        config.chain_pos_proposal_tick_phase.as_str(),
        "chain pos proposal tick phase",
    );
    if proposal_tick_phase.is_err() {
        issues.push(ConfigIssue::ChainPosProposalTickPhaseInvalid);
    }
    if let (Ok(ticks_per_slot), Ok(proposal_tick_phase)) = (ticks_per_slot, proposal_tick_phase) {
        if proposal_tick_phase >= ticks_per_slot {
            issues.push(ConfigIssue::ChainPosProposalTickPhaseOutOfRange);
        }
    }
    if parse_optional_i64(
        config.chain_pos_slot_clock_genesis_unix_ms.as_str(),
        "chain pos slot clock genesis unix ms",
    )
    .is_err()
    {
        issues.push(ConfigIssue::ChainPosSlotClockGenesisUnixMsInvalid);
    }
    if parse_non_negative_u64(
        config.chain_pos_max_past_slot_lag.as_str(),
        "chain pos max past slot lag",
    )
    .is_err()
    {
        issues.push(ConfigIssue::ChainPosMaxPastSlotLagInvalid);
    }
    if parse_chain_validators(config.chain_node_validators.as_str()).is_err() {
        issues.push(ConfigIssue::ChainValidatorsInvalid);
    }
    issues
}

#[cfg(test)]
pub(super) fn build_launcher_args(config: &LaunchConfig) -> Result<Vec<String>, String> {
    if config.scenario.trim().is_empty() {
        return Err("scenario cannot be empty".to_string());
    }
    let deployment_mode = match config.deployment_mode.trim() {
        "trusted_local_only" => "trusted_local_only",
        "hosted_public_join" => "hosted_public_join",
        _ => {
            return Err(format!(
                "deployment mode must be one of trusted_local_only|hosted_public_join, got `{}`",
                config.deployment_mode.trim()
            ))
        }
    };
    parse_host_port(config.live_bind.as_str(), "live bind")?;
    parse_host_port(config.web_bind.as_str(), "web bind")?;
    let viewer_port = parse_port(config.viewer_port.as_str(), "viewer port")?;
    if config.viewer_host.trim().is_empty() {
        return Err("viewer host cannot be empty".to_string());
    }
    if config.viewer_static_dir.trim().is_empty() {
        return Err("viewer static dir cannot be empty".to_string());
    }
    let mut args = vec![
        "--deployment-mode".to_string(),
        deployment_mode.to_string(),
        "--scenario".to_string(),
        config.scenario.trim().to_string(),
        "--live-bind".to_string(),
        config.live_bind.trim().to_string(),
        "--web-bind".to_string(),
        config.web_bind.trim().to_string(),
        "--viewer-host".to_string(),
        config.viewer_host.trim().to_string(),
        "--viewer-port".to_string(),
        viewer_port.to_string(),
        "--viewer-static-dir".to_string(),
        config.viewer_static_dir.trim().to_string(),
        "--chain-disable".to_string(),
    ];
    if config.llm_enabled {
        args.push("--with-llm".to_string());
        args.push("--agent-decision-source".to_string());
        args.push(
            canonical_agent_decision_source(config.agent_decision_source.as_str())
                .unwrap_or(BUILTIN_LLM_DECISION_SOURCE)
                .to_string(),
        );
        if is_provider_loopback_http_mode(config) {
            args.push("--agent-provider-backend".to_string());
            args.push(LOCAL_BRIDGE_PROVIDER_BACKEND.to_string());
            args.push("--agent-provider-contract".to_string());
            args.push(WORLDSIM_PROVIDER_CONTRACT.to_string());
            args.push("--agent-provider-transport".to_string());
            args.push(LOOPBACK_HTTP_PROVIDER_TRANSPORT.to_string());
            args.push("--agent-provider-url".to_string());
            args.push(effective_provider_base_url(config)?);
            if !config.agent_provider_auth_token.trim().is_empty() {
                args.push("--agent-provider-auth-token".to_string());
                args.push(config.agent_provider_auth_token.trim().to_string());
            }
            args.push("--agent-provider-connect-timeout-ms".to_string());
            args.push(parse_agent_provider_connect_timeout_ms(config)?.to_string());
            args.push("--agent-execution-lane".to_string());
            args.push(
                canonical_provider_execution_mode(config.agent_execution_lane.as_str())
                    .ok_or_else(|| {
                        "agent execution lane must be player_parity or headless_agent".to_string()
                    })?
                    .to_string(),
            );
            let agent_profile = config.agent_provider_profile.trim();
            if agent_profile.is_empty() {
                return Err("agent provider profile cannot be empty".to_string());
            }
            args.push("--agent-provider-profile".to_string());
            args.push(agent_profile.to_string());
        }
    } else {
        args.push("--no-llm".to_string());
    }
    if !config.auto_open_browser {
        args.push("--no-open-browser".to_string());
    }
    Ok(args)
}

#[cfg(test)]
pub(super) fn build_chain_runtime_args(config: &LaunchConfig) -> Result<Vec<String>, String> {
    let chain_runtime_bin = config.chain_runtime_bin.trim();
    if chain_runtime_bin.is_empty() {
        return Err("chain runtime bin cannot be empty".to_string());
    }
    parse_host_port(config.chain_status_bind.as_str(), "chain status bind")?;
    if config.chain_node_id.trim().is_empty() {
        return Err("chain node id cannot be empty".to_string());
    }
    let chain_role = parse_chain_role(config.chain_node_role.as_str())?;
    let chain_p2p_user_mode = canonical_chain_p2p_user_mode(config.chain_p2p_user_mode.as_str())
        .ok_or_else(|| {
            "chain P2P user mode must be one of: auto_join, private_safe, public_entry".to_string()
        })?;
    if chain_p2p_user_mode == "public_entry" && !config.chain_p2p_accept_public_entry {
        return Err(
            "public entry mode requires explicit confirmation via Accept Public Entry".to_string(),
        );
    }
    let chain_tick_ms = parse_positive_u64(
        config.chain_node_tick_ms.as_str(),
        "chain node poll interval ms",
    )?;
    let pos_slot_duration_ms = parse_positive_u64(
        config.chain_pos_slot_duration_ms.as_str(),
        "chain pos slot duration ms",
    )?;
    let pos_ticks_per_slot = parse_positive_u64(
        config.chain_pos_ticks_per_slot.as_str(),
        "chain pos ticks per slot",
    )?;
    let pos_proposal_tick_phase = parse_non_negative_u64(
        config.chain_pos_proposal_tick_phase.as_str(),
        "chain pos proposal tick phase",
    )?;
    if pos_proposal_tick_phase >= pos_ticks_per_slot {
        return Err(format!(
            "chain pos proposal tick phase={} must be less than chain pos ticks per slot={}",
            pos_proposal_tick_phase, pos_ticks_per_slot
        ));
    }
    let pos_slot_clock_genesis_unix_ms = parse_optional_i64(
        config.chain_pos_slot_clock_genesis_unix_ms.as_str(),
        "chain pos slot clock genesis unix ms",
    )?;
    let pos_max_past_slot_lag = parse_non_negative_u64(
        config.chain_pos_max_past_slot_lag.as_str(),
        "chain pos max past slot lag",
    )?;
    let validators = parse_chain_validators(config.chain_node_validators.as_str())?;
    let replication_bootstrap_peers =
        parse_chain_replication_bootstrap_peers(config.chain_replication_bootstrap_peers.as_str())?;
    let scenario = config.scenario.trim();
    let default_world_id = if scenario.is_empty() {
        format!("live-{DEFAULT_SCENARIO}")
    } else {
        format!("live-{scenario}")
    };
    let chain_world_id = if config.chain_world_id.trim().is_empty() {
        default_world_id
    } else {
        config.chain_world_id.trim().to_string()
    };

    let mut args = vec![
        "--node-id".to_string(),
        config.chain_node_id.trim().to_string(),
        "--world-id".to_string(),
        chain_world_id,
        "--status-bind".to_string(),
        config.chain_status_bind.trim().to_string(),
        "--node-role".to_string(),
        chain_role,
        "--p2p-user-mode".to_string(),
        chain_p2p_user_mode.to_string(),
        if config.chain_p2p_accept_public_entry {
            "--p2p-accept-public-entry".to_string()
        } else {
            "--p2p-decline-public-entry".to_string()
        },
        "--node-tick-ms".to_string(),
        chain_tick_ms.to_string(),
        "--pos-slot-duration-ms".to_string(),
        pos_slot_duration_ms.to_string(),
        "--pos-ticks-per-slot".to_string(),
        pos_ticks_per_slot.to_string(),
        "--pos-proposal-tick-phase".to_string(),
        pos_proposal_tick_phase.to_string(),
        if config.chain_pos_adaptive_tick_scheduler_enabled {
            "--pos-adaptive-tick-scheduler".to_string()
        } else {
            "--pos-no-adaptive-tick-scheduler".to_string()
        },
        "--pos-max-past-slot-lag".to_string(),
        pos_max_past_slot_lag.to_string(),
    ];
    if let Some(genesis) = pos_slot_clock_genesis_unix_ms {
        args.push("--pos-slot-clock-genesis-unix-ms".to_string());
        args.push(genesis.to_string());
    }
    for validator in validators {
        args.push("--node-validator".to_string());
        args.push(validator);
    }
    for peer in replication_bootstrap_peers {
        args.push("--replication-network-peer".to_string());
        args.push(peer);
    }
    Ok(args)
}

pub(super) fn build_game_url(config: &LaunchConfig) -> String {
    let viewer_host = normalize_host_for_url(config.viewer_host.as_str());
    let viewer_host = host_for_url(viewer_host.as_str());
    let viewer_port = parse_port(config.viewer_port.as_str(), "viewer port").unwrap_or(4173);
    let (web_host, web_port) = parse_host_port(config.web_bind.as_str(), "web bind")
        .unwrap_or(("127.0.0.1".to_string(), 5011));
    let web_host = normalize_host_for_url(web_host.as_str());
    let web_host = host_for_url(web_host.as_str());
    let ws_url = format!("ws://{web_host}:{web_port}");
    let hosted_access_hint = serde_json::json!({
        "deployment_mode": config.deployment_mode.trim(),
        "verdict": "specified_not_implemented",
        "browser_signer_bootstrap": if config.deployment_mode.trim() == "hosted_public_join" {
            "disabled_for_public_player_plane"
        } else {
            "trusted_local_bootstrap_allowed"
        },
        "session_ladder": ["guest_session", "player_session", "strong_auth"],
        "action_matrix": [
            {
                "action_id": "gameplay_action",
                "required_auth": "player_session",
                "availability": "public_player_plane",
                "reason": "core gameplay input stays on the player_session lane",
            },
            {
                "action_id": "agent_chat",
                "required_auth": "player_session",
                "availability": "public_player_plane",
                "reason": "agent chat currently stays on the low-risk player_session lane",
            },
            {
                "action_id": "prompt_control_preview",
                "required_auth": "strong_auth",
                "availability": if config.deployment_mode.trim() == "hosted_public_join" {
                    "blocked_until_strong_auth"
                } else {
                    "trusted_local_preview_only"
                },
                "reason": if config.deployment_mode.trim() == "hosted_public_join" {
                    "hosted public join keeps this action behind strong_auth/private plane until the dedicated proof lane lands"
                } else {
                    "trusted local preview may still use preview bootstrap; hosted/public strong-auth lane remains pending"
                },
            },
            {
                "action_id": "prompt_control_apply",
                "required_auth": "strong_auth",
                "availability": if config.deployment_mode.trim() == "hosted_public_join" {
                    "blocked_until_strong_auth"
                } else {
                    "trusted_local_preview_only"
                },
                "reason": if config.deployment_mode.trim() == "hosted_public_join" {
                    "hosted public join keeps this action behind strong_auth/private plane until the dedicated proof lane lands"
                } else {
                    "trusted local preview may still use preview bootstrap; hosted/public strong-auth lane remains pending"
                },
            },
            {
                "action_id": "prompt_control_rollback",
                "required_auth": "strong_auth",
                "availability": if config.deployment_mode.trim() == "hosted_public_join" {
                    "blocked_until_strong_auth"
                } else {
                    "trusted_local_preview_only"
                },
                "reason": if config.deployment_mode.trim() == "hosted_public_join" {
                    "hosted public join keeps this action behind strong_auth/private plane until the dedicated proof lane lands"
                } else {
                    "trusted local preview may still use preview bootstrap; hosted/public strong-auth lane remains pending"
                },
            },
            {
                "action_id": "main_token_transfer",
                "required_auth": "strong_auth",
                "availability": if config.deployment_mode.trim() == "hosted_public_join" {
                    "blocked_until_strong_auth"
                } else {
                    "trusted_local_preview_only"
                },
                "reason": if config.deployment_mode.trim() == "hosted_public_join" {
                    "hosted public join keeps this action behind strong_auth/private plane until the dedicated proof lane lands"
                } else {
                    "trusted local preview may still use preview bootstrap; hosted/public strong-auth lane remains pending"
                },
            },
        ],
    })
    .to_string();

    format!(
        "http://{viewer_host}:{viewer_port}/?{}&{}",
        encoded_query_pair("ws", ws_url.as_str()),
        encoded_query_pair("hosted_access", hosted_access_hint.as_str()),
    )
}

pub(super) fn is_loopback_host(host: &str) -> bool {
    matches!(host.trim(), "127.0.0.1" | "localhost" | "::1" | "[::1]")
}
pub(super) fn parse_port(raw: &str, label: &str) -> Result<u16, String> {
    let value = raw.trim();
    let port = value
        .parse::<u16>()
        .map_err(|_| format!("{label} must be integer in 1..=65535"))?;
    if port == 0 {
        return Err(format!("{label} must be in 1..=65535"));
    }
    Ok(port)
}

pub(super) fn parse_positive_u64(raw: &str, label: &str) -> Result<u64, String> {
    let value = raw.trim();
    let parsed = value
        .parse::<u64>()
        .map_err(|_| format!("{label} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{label} must be a positive integer"));
    }
    Ok(parsed)
}

pub(super) fn parse_non_negative_u64(raw: &str, label: &str) -> Result<u64, String> {
    let value = raw.trim();
    value
        .parse::<u64>()
        .map_err(|_| format!("{label} must be a non-negative integer"))
}

pub(super) fn parse_optional_i64(raw: &str, label: &str) -> Result<Option<i64>, String> {
    let value = raw.trim();
    if value.is_empty() {
        return Ok(None);
    }
    value
        .parse::<i64>()
        .map(Some)
        .map_err(|_| format!("{label} must be an integer"))
}

pub(super) fn parse_host_port(raw: &str, label: &str) -> Result<(String, u16), String> {
    let value = raw.trim();
    let (host_raw, port_raw) = if let Some(rest) = value.strip_prefix('[') {
        let (host, remainder) = rest
            .split_once(']')
            .ok_or_else(|| format!("{label} IPv6 host must be in [addr]:port format"))?;
        let port_raw = remainder
            .strip_prefix(':')
            .ok_or_else(|| format!("{label} must be in <host:port> format"))?;
        (host, port_raw)
    } else {
        let (host, port_raw) = value
            .rsplit_once(':')
            .ok_or_else(|| format!("{label} must be in <host:port> format"))?;
        if host.contains(':') {
            return Err(format!("{label} IPv6 host must be wrapped in []"));
        }
        (host, port_raw)
    };
    let host = host_raw.trim();
    if host.trim().is_empty() {
        return Err(format!("{label} host cannot be empty"));
    }
    let port = parse_port(port_raw, label)?;
    Ok((host.trim().to_string(), port))
}

pub(super) fn parse_chain_role(raw: &str) -> Result<String, String> {
    let role = raw.trim().to_ascii_lowercase();
    match role.as_str() {
        "sequencer" | "storage" | "observer" => Ok(role),
        _ => Err("chain role must be one of: sequencer|storage|observer".to_string()),
    }
}

pub(super) fn parse_chain_validators(raw: &str) -> Result<Vec<String>, String> {
    let mut validators = Vec::new();
    for token in raw.split([',', ';', ' ']) {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        let (validator_id, stake) = token
            .rsplit_once(':')
            .ok_or_else(|| "chain validators must be <validator_id:stake>".to_string())?;
        if validator_id.trim().is_empty() {
            return Err("chain validators cannot contain empty validator_id".to_string());
        }
        let stake = stake
            .parse::<u64>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| "chain validator stake must be positive integer".to_string())?;
        validators.push(format!("{}:{}", validator_id.trim(), stake));
    }
    Ok(validators)
}

pub(super) fn parse_chain_replication_bootstrap_peers(raw: &str) -> Result<Vec<String>, String> {
    let mut peers = Vec::new();
    for token in raw.split([',', ';', ' ', '\n', '\r', '\t']) {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        if !token.starts_with('/') {
            return Err(
                "chain replication bootstrap peers must use multiaddr values like /ip4/127.0.0.1/tcp/4100/p2p/<peer-id>"
                    .to_string(),
            );
        }
        peers.push(token.to_string());
    }
    Ok(peers)
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn spawn_child_process(
    bin: &str,
    args: &[String],
    process_label: &str,
) -> Result<RunningProcess, String> {
    let mut child = Command::new(bin)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("spawn process `{bin}` failed: {err}"))?;

    let (log_tx, log_rx) = mpsc::channel::<String>();
    if let Some(stdout) = child.stdout.take() {
        spawn_log_reader(stdout, process_label, "stdout", log_tx.clone());
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_log_reader(stderr, process_label, "stderr", log_tx.clone());
    }

    Ok(RunningProcess { child, log_rx })
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn spawn_log_reader<R: Read + Send + 'static>(
    reader: R,
    process_label: &str,
    source: &'static str,
    tx: Sender<String>,
) {
    let process_label = process_label.to_string();
    std::thread::spawn(move || {
        let buffered = BufReader::new(reader);
        for line in buffered.lines() {
            match line {
                Ok(content) => {
                    let _ = tx.send(format!("[{process_label} {source}] {content}"));
                }
                Err(err) => {
                    let _ = tx.send(format!("[{process_label} {source}] <read error: {err}>"));
                    break;
                }
            }
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn stop_child_process(child: &mut Child) -> Result<(), String> {
    if child
        .try_wait()
        .map_err(|err| format!("query child status failed: {err}"))?
        .is_some()
    {
        return Ok(());
    }

    if let Err(err) = send_interrupt_signal(child) {
        eprintln!("warning: failed to request graceful process stop: {err}");
    } else {
        let deadline = Instant::now() + Duration::from_millis(GRACEFUL_STOP_TIMEOUT_MS);
        while Instant::now() < deadline {
            if child
                .try_wait()
                .map_err(|err| format!("query child status failed: {err}"))?
                .is_some()
            {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(STOP_POLL_INTERVAL_MS));
        }
    }

    if let Ok(None) = child.try_wait() {
        child
            .kill()
            .map_err(|err| format!("kill child failed: {err}"))?;
    }
    child
        .wait()
        .map_err(|err| format!("wait child failed: {err}"))?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn send_interrupt_signal(child: &Child) -> Result<(), String> {
    #[cfg(unix)]
    {
        let pid = child.id() as i32;
        // SAFETY: libc::kill is called with a pid from std::process::Child.
        let rc = unsafe { libc::kill(pid, libc::SIGINT) };
        if rc == 0 {
            return Ok(());
        }
        return Err(format!(
            "send SIGINT failed: {}",
            std::io::Error::last_os_error()
        ));
    }

    #[cfg(not(unix))]
    {
        let _ = child;
        Ok(())
    }
}
