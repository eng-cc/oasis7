use super::support::{resolve_launcher_bin_from_config, resolve_viewer_static_dir_for_launcher};
use super::*;

#[cfg(test)]
pub(in super::super) fn build_launcher_args(
    config: &LauncherConfig,
) -> Result<Vec<String>, String> {
    let launcher_bin = resolve_launcher_bin_from_config(config, config.launcher_bin.as_str());
    build_launcher_args_with_launcher_bin(config, launcher_bin.as_str())
}

pub(in super::super) fn build_launcher_args_with_launcher_bin(
    config: &LauncherConfig,
    launcher_bin: &str,
) -> Result<Vec<String>, String> {
    parse_host_port(config.live_bind.as_str(), "live bind")?;
    parse_host_port(config.web_bind.as_str(), "web bind")?;
    let viewer_port = parse_port(config.viewer_port.as_str(), "viewer port")?;
    if config.viewer_host.trim().is_empty() {
        return Err("viewer host cannot be empty".to_string());
    }
    if config.viewer_static_dir.trim().is_empty() {
        return Err("viewer static dir cannot be empty".to_string());
    }
    let viewer_static_dir =
        resolve_viewer_static_dir_for_launcher(config.viewer_static_dir.trim(), launcher_bin)
            .ok_or_else(|| {
                format!(
                    "viewer static directory does not exist or is not a directory: {}",
                    config.viewer_static_dir.trim()
                )
            })?;

    let mut args = vec![
        "--deployment-mode".to_string(),
        DeploymentMode::parse_user_facing(config.deployment_mode.as_str(), "deployment_mode")?
            .as_str()
            .to_string(),
        "--live-bind".to_string(),
        config.live_bind.trim().to_string(),
        "--web-bind".to_string(),
        config.web_bind.trim().to_string(),
        "--viewer-host".to_string(),
        config.viewer_host.trim().to_string(),
        "--viewer-port".to_string(),
        viewer_port.to_string(),
        "--viewer-static-dir".to_string(),
        viewer_static_dir.to_string_lossy().to_string(),
    ];

    if config.llm_enabled {
        args.push("--with-llm".to_string());
        args.push("--agent-decision-source".to_string());
        args.push(
            canonical_agent_decision_source(config.agent_decision_source.as_str())
                .ok_or_else(|| {
                    "agent decision source must be builtin_llm or provider_backed".to_string()
                })?
                .to_string(),
        );
        if provider_backed_is_requested(config) {
            let provider_backend =
                canonical_agent_provider_backend(config.agent_provider_backend.as_str())
                    .ok_or_else(|| {
                        "agent provider backend must be provider_local_bridge".to_string()
                    })?;
            let provider_contract =
                canonical_agent_provider_contract(config.agent_provider_contract.as_str())
                    .ok_or_else(|| {
                        "agent provider contract must be worldsim_provider_v1".to_string()
                    })?;
            let provider_transport =
                canonical_agent_provider_transport(config.agent_provider_transport.as_str())
                    .ok_or_else(|| {
                        "agent provider transport must be loopback_http or remote_https".to_string()
                    })?;
            let provider_base_url = effective_provider_base_url(config)?;
            validate_provider_base_url_for_transport(
                provider_base_url.as_str(),
                provider_transport,
            )?;
            args.push("--agent-provider-backend".to_string());
            args.push(provider_backend.to_string());
            args.push("--agent-provider-contract".to_string());
            args.push(provider_contract.to_string());
            args.push("--agent-provider-transport".to_string());
            args.push(provider_transport.to_string());
            args.push("--agent-provider-url".to_string());
            args.push(provider_base_url);
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
    args.push("--chain-disable".to_string());
    if !config.scenario.trim().is_empty() {
        args.splice(
            2..2,
            ["--scenario".to_string(), config.scenario.trim().to_string()],
        );
    }

    Ok(args)
}

pub(super) fn collect_agent_provider_config_issues(
    config: &LauncherConfig,
    issues: &mut Vec<String>,
) {
    if validate_agent_decision_source(config.agent_decision_source.as_str()).is_err() {
        issues.push("agent decision source must be builtin_llm or provider_backed".to_string());
    }
    if !provider_backed_is_requested(config) {
        return;
    }

    let provider_backend = canonical_agent_provider_backend(config.agent_provider_backend.as_str());
    let provider_contract =
        canonical_agent_provider_contract(config.agent_provider_contract.as_str());
    let provider_transport =
        canonical_agent_provider_transport(config.agent_provider_transport.as_str());
    if provider_backend != Some(AGENT_PROVIDER_BACKEND_LOCAL_BRIDGE)
        || provider_contract != Some(AGENT_PROVIDER_CONTRACT_WORLDSIM_V1)
        || !matches!(
            provider_transport,
            Some(AGENT_PROVIDER_TRANSPORT_LOOPBACK_HTTP)
                | Some(AGENT_PROVIDER_TRANSPORT_REMOTE_HTTPS)
        )
    {
        issues.push(
            "agent provider mode must use a supported backend/contract/transport".to_string(),
        );
    }
    match effective_provider_base_url(config) {
        Ok(base_url) => {
            if let Some(transport) = provider_transport {
                if validate_provider_base_url_for_transport(base_url.as_str(), transport).is_err() {
                    issues.push(
                        "agent provider URL is invalid for the selected transport".to_string(),
                    );
                }
            }
        }
        Err(_) => {
            issues.push("agent provider URL is required when auto-discover is disabled".to_string())
        }
    }
    if parse_agent_provider_connect_timeout_ms(config).is_err() {
        issues.push("agent provider connect timeout ms must be a positive integer".to_string());
    }
    if canonical_provider_execution_mode(config.agent_execution_lane.as_str()).is_none() {
        issues.push("agent execution lane must be player_parity or headless_agent".to_string());
    }
    if config.agent_provider_profile.trim().is_empty() {
        issues.push("agent provider profile is required".to_string());
    }
}

pub(super) fn provider_backed_is_requested(config: &LauncherConfig) -> bool {
    canonical_agent_decision_source(config.agent_decision_source.as_str())
        == Some(AGENT_DECISION_SOURCE_PROVIDER_BACKED)
}

pub(super) fn validate_agent_decision_source(raw: &str) -> Result<(), String> {
    canonical_agent_decision_source(raw)
        .map(|_| ())
        .ok_or_else(|| "agent decision source must be builtin_llm or provider_backed".to_string())
}

pub(super) fn validate_provider_base_url_for_transport(
    base_url: &str,
    transport: &str,
) -> Result<(), String> {
    let (host, _) = parse_http_base_url(base_url, "agent provider URL")?;
    match canonical_agent_provider_transport(transport) {
        Some(AGENT_PROVIDER_TRANSPORT_LOOPBACK_HTTP) => {
            if !base_url.trim().starts_with("http://") {
                return Err("agent provider URL must use http:// for loopback_http".to_string());
            }
            if !is_loopback_host(host.as_str()) {
                return Err(
                    "agent provider URL must use a loopback host for loopback_http".to_string(),
                );
            }
        }
        Some(AGENT_PROVIDER_TRANSPORT_REMOTE_HTTPS) => {
            if !base_url.trim().starts_with("https://") {
                return Err("agent provider URL must use https:// for remote_https".to_string());
            }
            if !is_public_remote_https_host(host.as_str()) {
                return Err(
                    "agent provider URL must use a public https host for remote_https".to_string(),
                );
            }
        }
        _ => {
            return Err(
                "agent provider transport must be loopback_http or remote_https".to_string(),
            )
        }
    }
    Ok(())
}

pub(super) fn parse_http_base_url(base_url: &str, label: &str) -> Result<(String, u16), String> {
    let mut raw = base_url.trim();
    let mut default_port = 80;
    if let Some(stripped) = raw.strip_prefix("http://") {
        raw = stripped;
    } else if let Some(stripped) = raw.strip_prefix("https://") {
        raw = stripped;
        default_port = 443;
    }
    raw = raw.trim_end_matches('/');
    let authority = raw
        .split('/')
        .next()
        .ok_or_else(|| format!("invalid {label}: {base_url}"))?
        .trim();
    if authority.is_empty() {
        return Err(format!("invalid {label}: {base_url}"));
    }
    if authority.starts_with('[') || authority.contains(':') {
        parse_host_port(authority, label)
    } else {
        Ok((authority.to_string(), default_port))
    }
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host.trim(), "127.0.0.1" | "localhost" | "::1" | "[::1]")
}

fn is_public_remote_https_host(host: &str) -> bool {
    let normalized = host.trim().trim_start_matches('[').trim_end_matches(']');
    if is_loopback_host(host) || normalized.eq_ignore_ascii_case("localhost") {
        return false;
    }
    let Ok(ip) = normalized.parse::<std::net::IpAddr>() else {
        return true;
    };
    match ip {
        std::net::IpAddr::V4(ipv4) => {
            !ipv4.is_private() && !ipv4.is_loopback() && !ipv4.is_link_local()
        }
        std::net::IpAddr::V6(ipv6) => !ipv6.is_loopback() && !ipv6.is_unicast_link_local(),
    }
}

pub(super) fn canonical_agent_decision_source(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        AGENT_DECISION_SOURCE_BUILTIN_LLM => Some(AGENT_DECISION_SOURCE_BUILTIN_LLM),
        AGENT_DECISION_SOURCE_PROVIDER_BACKED
        | AGENT_PROVIDER_MODE_PROVIDER_LOOPBACK_HTTP_ALIAS
        | AGENT_PROVIDER_MODE_DIRECT_CONNECT_ALIAS => Some(AGENT_DECISION_SOURCE_PROVIDER_BACKED),
        _ => None,
    }
}

pub(super) fn canonical_agent_provider_backend(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        AGENT_PROVIDER_BACKEND_LOCAL_BRIDGE
        | AGENT_PROVIDER_MODE_PROVIDER_LOOPBACK_HTTP_ALIAS
        | AGENT_PROVIDER_MODE_DIRECT_CONNECT_ALIAS => Some(AGENT_PROVIDER_BACKEND_LOCAL_BRIDGE),
        _ => None,
    }
}

pub(super) fn canonical_agent_provider_contract(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        AGENT_PROVIDER_CONTRACT_WORLDSIM_V1
        | AGENT_PROVIDER_MODE_PROVIDER_LOOPBACK_HTTP_ALIAS
        | AGENT_PROVIDER_MODE_DIRECT_CONNECT_ALIAS => Some(AGENT_PROVIDER_CONTRACT_WORLDSIM_V1),
        _ => None,
    }
}

pub(super) fn canonical_agent_provider_transport(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        AGENT_PROVIDER_TRANSPORT_LOOPBACK_HTTP
        | AGENT_PROVIDER_MODE_PROVIDER_LOOPBACK_HTTP_ALIAS
        | AGENT_PROVIDER_MODE_DIRECT_CONNECT_ALIAS => Some(AGENT_PROVIDER_TRANSPORT_LOOPBACK_HTTP),
        AGENT_PROVIDER_TRANSPORT_REMOTE_HTTPS => Some(AGENT_PROVIDER_TRANSPORT_REMOTE_HTTPS),
        _ => None,
    }
}

pub(super) fn canonical_provider_execution_mode(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "player_parity" | "player-parity" | "player" => Some(AGENT_EXECUTION_LANE_PLAYER_PARITY),
        "headless_agent" | "headless-agent" | "headless" => {
            Some(AGENT_EXECUTION_LANE_HEADLESS_AGENT)
        }
        _ => None,
    }
}

pub(super) fn effective_provider_base_url(config: &LauncherConfig) -> Result<String, String> {
    let base_url = config.agent_provider_url.trim();
    if !base_url.is_empty() {
        return Ok(base_url.to_string());
    }
    if config.provider_auto_discover {
        return Ok(DEFAULT_AGENT_PROVIDER_URL.to_string());
    }
    Err("agent provider URL is required when auto-discover is disabled".to_string())
}

pub(super) fn parse_agent_provider_connect_timeout_ms(
    config: &LauncherConfig,
) -> Result<u64, String> {
    parse_positive_u64(
        config.agent_provider_connect_timeout_ms.as_str(),
        "agent provider connect timeout ms",
    )
}
