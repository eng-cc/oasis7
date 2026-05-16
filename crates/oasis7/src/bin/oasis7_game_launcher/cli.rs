use super::*;
use oasis7::launcher_bootstrap_peers::parse_chain_replication_bootstrap_peer;

pub(super) fn parse_options<'a>(args: impl Iterator<Item = &'a str>) -> Result<CliOptions, String> {
    let mut options = CliOptions::default();
    let mut iter = args.peekable();
    let mut explicit_chain_replication_bootstrap_peers = false;

    while let Some(arg) = iter.next() {
        match arg {
            "--deployment-mode" => {
                options.deployment_mode = parse_required_value(&mut iter, "--deployment-mode")?;
            }
            "--scenario" => {
                options.scenario = parse_required_value(&mut iter, "--scenario")?;
            }
            "--live-bind" => {
                options.live_bind = parse_required_value(&mut iter, "--live-bind")?;
            }
            "--web-bind" => {
                options.web_bind = parse_required_value(&mut iter, "--web-bind")?;
            }
            "--viewer-host" => {
                options.viewer_host = parse_required_value(&mut iter, "--viewer-host")?;
            }
            "--viewer-port" => {
                let raw = parse_required_value(&mut iter, "--viewer-port")?;
                options.viewer_port = raw.parse::<u16>().map_err(|_| {
                    format!("--viewer-port must be an integer in 1..=65535, got `{raw}`")
                })?;
                if options.viewer_port == 0 {
                    return Err("--viewer-port must be in 1..=65535".to_string());
                }
            }
            "--viewer-static-dir" => {
                options.viewer_static_dir = parse_required_value(&mut iter, "--viewer-static-dir")?;
            }
            "--with-llm" => {
                options.with_llm = true;
            }
            "--agent-decision-source" => {
                options.agent_decision_source =
                    parse_required_value(&mut iter, "--agent-decision-source")?;
            }
            "--agent-provider-backend" => {
                options.agent_provider_backend =
                    parse_required_value(&mut iter, "--agent-provider-backend")?;
            }
            "--agent-provider-contract" => {
                options.agent_provider_contract =
                    parse_required_value(&mut iter, "--agent-provider-contract")?;
            }
            "--agent-provider-transport" => {
                options.agent_provider_transport =
                    parse_required_value(&mut iter, "--agent-provider-transport")?;
            }
            "--agent-provider-url" => {
                options.agent_provider_url =
                    parse_required_value(&mut iter, "--agent-provider-url")?;
            }
            "--agent-provider-auth-token" => {
                options.agent_provider_auth_token =
                    parse_required_value(&mut iter, "--agent-provider-auth-token")?;
            }
            "--agent-provider-connect-timeout-ms" => {
                let raw = parse_required_value(&mut iter, "--agent-provider-connect-timeout-ms")?;
                options.agent_provider_connect_timeout_ms = raw.parse::<u64>().map_err(|_| {
                    format!("--agent-provider-connect-timeout-ms must be a positive integer, got `{raw}`")
                })?;
                if options.agent_provider_connect_timeout_ms == 0 {
                    return Err(
                        "--agent-provider-connect-timeout-ms must be a positive integer"
                            .to_string(),
                    );
                }
            }
            "--agent-provider-profile" => {
                options.agent_provider_profile =
                    parse_required_value(&mut iter, "--agent-provider-profile")?;
            }
            "--agent-execution-lane" => {
                let raw = parse_required_value(&mut iter, "--agent-execution-lane")?;
                options.agent_execution_lane = ProviderExecutionMode::parse(raw.as_str())
                    .ok_or_else(|| {
                        format!(
                            "--agent-execution-lane must be one of player_parity or headless_agent, got `{raw}`"
                        )
                    })?;
            }
            "--agent-provider-mode" => {
                options.agent_decision_source =
                    parse_required_value(&mut iter, "--agent-provider-mode")?;
            }
            "--no-open-browser" => {
                options.open_browser = false;
            }
            "--chain-enable" => {
                options.chain_enabled = true;
            }
            "--chain-disable" => {
                options.chain_enabled = false;
            }
            "--chain-status-bind" => {
                options.chain_status_bind = parse_required_value(&mut iter, "--chain-status-bind")?;
            }
            "--chain-node-id" => {
                options.chain_node_id = parse_required_value(&mut iter, "--chain-node-id")?;
            }
            "--chain-network-tier-manifest" => {
                options.chain_network_tier_manifest =
                    parse_required_value(&mut iter, "--chain-network-tier-manifest")?;
            }
            "--chain-storage-profile" => {
                options.chain_storage_profile =
                    parse_required_value(&mut iter, "--chain-storage-profile")?
                        .parse::<StorageProfile>()?;
            }
            "--chain-world-id" => {
                options.chain_world_id = Some(parse_required_value(&mut iter, "--chain-world-id")?);
            }
            "--chain-node-role" => {
                let raw = parse_required_value(&mut iter, "--chain-node-role")?;
                options.chain_node_role = parse_chain_node_role(raw.as_str())?;
            }
            "--chain-p2p-user-mode" => {
                let raw = parse_required_value(&mut iter, "--chain-p2p-user-mode")?;
                options.chain_p2p_user_mode = parse_chain_p2p_user_mode(raw.as_str())?;
            }
            "--chain-p2p-accept-public-entry" => {
                options.chain_p2p_accept_public_entry = true;
            }
            "--chain-p2p-reject-public-entry" => {
                options.chain_p2p_accept_public_entry = false;
            }
            "--chain-replication-network-peer" => {
                let value = parse_required_value(&mut iter, "--chain-replication-network-peer")?;
                validate_chain_replication_network_peer(value.as_str())?;
                if !explicit_chain_replication_bootstrap_peers {
                    // The first explicit bootstrap peer switches CLI behavior from
                    // "use bundled defaults" to "use only the caller-provided list".
                    options.chain_replication_bootstrap_peers.clear();
                    explicit_chain_replication_bootstrap_peers = true;
                }
                options.chain_replication_bootstrap_peers.push(value);
            }
            "--chain-node-tick-ms" => {
                let raw = parse_required_value(&mut iter, "--chain-node-tick-ms")?;
                options.chain_node_tick_ms = raw.parse::<u64>().map_err(|_| {
                    format!("--chain-node-tick-ms must be a positive integer, got `{raw}`")
                })?;
                if options.chain_node_tick_ms == 0 {
                    return Err("--chain-node-tick-ms must be a positive integer".to_string());
                }
            }
            "--chain-pos-slot-duration-ms" => {
                let raw = parse_required_value(&mut iter, "--chain-pos-slot-duration-ms")?;
                options.chain_pos_slot_duration_ms = raw.parse::<u64>().map_err(|_| {
                    format!("--chain-pos-slot-duration-ms must be a positive integer, got `{raw}`")
                })?;
                if options.chain_pos_slot_duration_ms == 0 {
                    return Err(
                        "--chain-pos-slot-duration-ms must be a positive integer".to_string()
                    );
                }
            }
            "--chain-pos-ticks-per-slot" => {
                let raw = parse_required_value(&mut iter, "--chain-pos-ticks-per-slot")?;
                options.chain_pos_ticks_per_slot = raw.parse::<u64>().map_err(|_| {
                    format!("--chain-pos-ticks-per-slot must be a positive integer, got `{raw}`")
                })?;
                if options.chain_pos_ticks_per_slot == 0 {
                    return Err("--chain-pos-ticks-per-slot must be a positive integer".to_string());
                }
            }
            "--chain-pos-proposal-tick-phase" => {
                let raw = parse_required_value(&mut iter, "--chain-pos-proposal-tick-phase")?;
                options.chain_pos_proposal_tick_phase = raw.parse::<u64>().map_err(|_| {
                    format!("--chain-pos-proposal-tick-phase must be a non-negative integer, got `{raw}`")
                })?;
            }
            "--chain-pos-adaptive-tick-scheduler" => {
                options.chain_pos_adaptive_tick_scheduler_enabled = true;
            }
            "--chain-pos-no-adaptive-tick-scheduler" => {
                options.chain_pos_adaptive_tick_scheduler_enabled = false;
            }
            "--chain-pos-slot-clock-genesis-unix-ms" => {
                let raw =
                    parse_required_value(&mut iter, "--chain-pos-slot-clock-genesis-unix-ms")?;
                options.chain_pos_slot_clock_genesis_unix_ms =
                    Some(raw.parse::<i64>().map_err(|_| {
                        format!(
                            "--chain-pos-slot-clock-genesis-unix-ms must be an integer, got `{raw}`"
                        )
                    })?);
            }
            "--chain-pos-max-past-slot-lag" => {
                let raw = parse_required_value(&mut iter, "--chain-pos-max-past-slot-lag")?;
                options.chain_pos_max_past_slot_lag = raw.parse::<u64>().map_err(|_| {
                    format!(
                        "--chain-pos-max-past-slot-lag must be a non-negative integer, got `{raw}`"
                    )
                })?;
            }
            "--chain-node-validator" => {
                let value = parse_required_value(&mut iter, "--chain-node-validator")?;
                validate_chain_node_validator(value.as_str())?;
                options.chain_node_validators.push(value);
            }
            _ => return Err(format!("unknown option: {arg}")),
        }
    }

    let _ = parse_host_port(options.live_bind.as_str(), "--live-bind")?;
    let _ = parse_host_port(options.web_bind.as_str(), "--web-bind")?;
    let deployment_mode =
        DeploymentMode::parse(options.deployment_mode.as_str(), "--deployment-mode")?;
    if !deployment_mode.allows_local_chain_runtime() {
        options.chain_enabled = false;
    }
    validate_agent_decision_source(options.agent_decision_source.as_str())?;
    options.agent_decision_source =
        canonical_agent_decision_source(options.agent_decision_source.as_str()).to_string();
    if options.agent_decision_source == PROVIDER_BACKED_DECISION_SOURCE {
        validate_agent_provider_backend(options.agent_provider_backend.as_str())?;
        validate_agent_provider_contract(options.agent_provider_contract.as_str())?;
        validate_agent_provider_transport(options.agent_provider_transport.as_str())?;
        options.agent_provider_backend =
            canonical_agent_provider_backend(options.agent_provider_backend.as_str()).to_string();
        options.agent_provider_contract =
            canonical_agent_provider_contract(options.agent_provider_contract.as_str()).to_string();
        options.agent_provider_transport =
            canonical_agent_provider_transport(options.agent_provider_transport.as_str())
                .to_string();
        if options.agent_provider_url.trim().is_empty() {
            return Err("--agent-provider-url requires a non-empty value".to_string());
        }
        if options.agent_provider_profile.trim().is_empty() {
            return Err("--agent-provider-profile requires a non-empty value".to_string());
        }
    }
    normalize_http_target(
        options.viewer_host.as_str(),
        options.viewer_port,
        "viewer host/port",
    )?;
    if options.chain_enabled {
        let _ = parse_host_port(options.chain_status_bind.as_str(), "--chain-status-bind")?;
        if options.chain_node_id.trim().is_empty() {
            return Err("--chain-node-id requires a non-empty value".to_string());
        }
        parse_chain_node_role(options.chain_node_role.as_str())?;
        if options.chain_network_tier_manifest.trim().is_empty()
            && options.chain_storage_profile.as_str().trim().is_empty()
        {
            return Err("--chain-storage-profile requires a non-empty value".to_string());
        }
        parse_chain_p2p_user_mode(options.chain_p2p_user_mode.as_str())?;
        if options.chain_node_tick_ms == 0 {
            return Err("--chain-node-tick-ms must be a positive integer".to_string());
        }
        if options.chain_pos_slot_duration_ms == 0 {
            return Err("--chain-pos-slot-duration-ms must be a positive integer".to_string());
        }
        if options.chain_pos_ticks_per_slot == 0 {
            return Err("--chain-pos-ticks-per-slot must be a positive integer".to_string());
        }
        if options.chain_pos_proposal_tick_phase >= options.chain_pos_ticks_per_slot {
            return Err(format!(
                "--chain-pos-proposal-tick-phase={} must be less than --chain-pos-ticks-per-slot={}",
                options.chain_pos_proposal_tick_phase, options.chain_pos_ticks_per_slot
            ));
        }
        for validator in &options.chain_node_validators {
            validate_chain_node_validator(validator.as_str())?;
        }
        for peer in &options.chain_replication_bootstrap_peers {
            validate_chain_replication_network_peer(peer.as_str())?;
        }
    }

    Ok(options)
}

pub(super) fn deployment_mode_from_options(options: &CliOptions) -> DeploymentMode {
    DeploymentMode::parse(options.deployment_mode.as_str(), "deployment_mode")
        .unwrap_or(DeploymentMode::TrustedLocalOnly)
}

pub(super) fn uses_loopback_provider(options: &CliOptions) -> bool {
    options.agent_decision_source == PROVIDER_BACKED_DECISION_SOURCE
        && options.agent_provider_backend == LOCAL_BRIDGE_PROVIDER_BACKEND
        && options.agent_provider_contract == WORLDSIM_PROVIDER_CONTRACT
        && options.agent_provider_transport == LOOPBACK_HTTP_PROVIDER_TRANSPORT
}

fn parse_required_value<'a, I>(
    iter: &mut std::iter::Peekable<I>,
    flag: &str,
) -> Result<String, String>
where
    I: Iterator<Item = &'a str>,
{
    let Some(value) = iter.next() else {
        return Err(format!("{flag} requires a value"));
    };
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{flag} requires a non-empty value"));
    }
    Ok(value.to_string())
}

pub(super) fn parse_host_port(raw: &str, label: &str) -> Result<(String, u16), String> {
    let trimmed = raw.trim();
    let (host_raw, port_text) = if let Some(rest) = trimmed.strip_prefix('[') {
        let (host, remainder) = rest
            .split_once(']')
            .ok_or_else(|| format!("{label} IPv6 host must be in [addr]:port format"))?;
        let port_text = remainder
            .strip_prefix(':')
            .ok_or_else(|| format!("{label} must be in <host:port> format"))?;
        (host, port_text)
    } else {
        let (host, port_text) = trimmed
            .rsplit_once(':')
            .ok_or_else(|| format!("{label} must be in <host:port> format"))?;
        if host.contains(':') {
            return Err(format!("{label} IPv6 host must be wrapped in []"));
        }
        (host, port_text)
    };
    let host = host_raw.trim();
    if host.trim().is_empty() {
        return Err(format!("{label} host cannot be empty"));
    }
    let port = port_text
        .parse::<u16>()
        .map_err(|_| format!("{label} port must be an integer in 1..=65535"))?;
    if port == 0 {
        return Err(format!("{label} port must be in 1..=65535"));
    }
    Ok((host.trim().to_string(), port))
}

fn parse_chain_node_role(raw: &str) -> Result<String, String> {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "sequencer" | "storage" | "observer" => Ok(normalized),
        _ => Err("--chain-node-role must be one of: sequencer, storage, observer".to_string()),
    }
}

fn parse_chain_p2p_user_mode(raw: &str) -> Result<String, String> {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "auto_join" | "private_safe" | "public_entry" => Ok(normalized),
        _ => Err(
            "--chain-p2p-user-mode must be one of: auto_join, private_safe, public_entry"
                .to_string(),
        ),
    }
}

fn validate_chain_node_validator(raw: &str) -> Result<(), String> {
    let (validator_id, stake) = raw.rsplit_once(':').ok_or_else(|| {
        "--chain-node-validator must be in <validator_id:stake> format".to_string()
    })?;
    if validator_id.trim().is_empty() {
        return Err("--chain-node-validator validator_id cannot be empty".to_string());
    }
    let stake = stake
        .parse::<u64>()
        .map_err(|_| "--chain-node-validator stake must be a positive integer".to_string())?;
    if stake == 0 {
        return Err("--chain-node-validator stake must be a positive integer".to_string());
    }
    Ok(())
}

fn validate_chain_replication_network_peer(raw: &str) -> Result<(), String> {
    parse_chain_replication_bootstrap_peer(raw)
        .map(|_| ())
        .map_err(|err| format!("--chain-replication-network-peer invalid: {err}"))
}

pub(super) fn print_help() {
    println!(
        "Usage: oasis7_game_launcher [options]\n\n\
Start player stack with one command:\n\
- start oasis7_chain_runtime (default)\n\
- start oasis7_viewer_live\n\
- start built-in static web server\n\
- print URL and optionally open browser\n\n\
Options:\n\
  --deployment-mode <mode>    trusted_local_only|hosted_public_join (default: {DEFAULT_DEPLOYMENT_MODE})\n\
  --scenario <name>            optional debug scenario; default uses formal release fixed world\n\
  --live-bind <host:port>      oasis7_viewer_live bind (default: {DEFAULT_LIVE_BIND})\n\
  --web-bind <host:port>       oasis7_viewer_live web bridge bind (default: {DEFAULT_WEB_BIND})\n\
  --viewer-host <host>         web viewer host (default: {DEFAULT_VIEWER_HOST})\n\
  --viewer-port <port>         web viewer port (default: {DEFAULT_VIEWER_PORT})\n\
  --viewer-static-dir <path>   prebuilt web asset dir (default: {DEFAULT_VIEWER_STATIC_DIR})\n\
  --chain-enable               enable oasis7_chain_runtime (default)\n\
  --chain-disable              disable oasis7_chain_runtime\n\
  --chain-status-bind <addr>   oasis7_chain_runtime status bind (default: {DEFAULT_CHAIN_STATUS_BIND})\n\
  --chain-node-id <id>         oasis7_chain_runtime node id (default: {DEFAULT_CHAIN_NODE_ID})\n\
  --chain-network-tier-manifest <path>\n\
                               formal network tier manifest json; when set, chain bootstrap peers/status tier metadata load from manifest and explicit storage profile becomes optional\n\
  --chain-storage-profile <name> oasis7_chain_runtime storage profile (default: dev_local)\n\
  --chain-world-id <id>        oasis7_chain_runtime world id (default: live-<scenario>)\n\
  --chain-node-role <role>     oasis7_chain_runtime role (default: {DEFAULT_CHAIN_NODE_ROLE})\n\
  --chain-p2p-user-mode <mode> oasis7_chain_runtime user mode: auto_join|private_safe|public_entry (default: {DEFAULT_CHAIN_P2P_USER_MODE})\n\
  --chain-p2p-accept-public-entry\n\
                               accept auto-detected public-entry recommendation\n\
  --chain-p2p-reject-public-entry\n\
                               keep conservative fallback when auto mode suggests public entry (default)\n\
  --chain-replication-network-peer <multiaddr>\n\
                               oasis7_chain_runtime replication bootstrap peer multiaddr (repeatable; first explicit value replaces bundled defaults)\n\
  --chain-node-tick-ms <n>     oasis7_chain_runtime worker poll/fallback interval ms (default: {DEFAULT_CHAIN_NODE_TICK_MS})\n\
  --chain-pos-slot-duration-ms <n>\n\
                               oasis7_chain_runtime PoS slot duration ms (default: {DEFAULT_CHAIN_POS_SLOT_DURATION_MS})\n\
  --chain-pos-ticks-per-slot <n>\n\
                               oasis7_chain_runtime PoS logical ticks per slot (default: {DEFAULT_CHAIN_POS_TICKS_PER_SLOT})\n\
  --chain-pos-proposal-tick-phase <n>\n\
                               oasis7_chain_runtime proposal phase in slot tick window (default: {DEFAULT_CHAIN_POS_PROPOSAL_TICK_PHASE})\n\
  --chain-pos-adaptive-tick-scheduler\n\
                               enable oasis7_chain_runtime adaptive tick scheduler\n\
  --chain-pos-no-adaptive-tick-scheduler\n\
                               disable oasis7_chain_runtime adaptive scheduler (default)\n\
  --chain-pos-slot-clock-genesis-unix-ms <n>\n\
                               oasis7_chain_runtime fixed slot clock genesis unix ms (default: auto)\n\
  --chain-pos-max-past-slot-lag <n>\n\
                               oasis7_chain_runtime max accepted stale slot lag (default: {DEFAULT_CHAIN_POS_MAX_PAST_SLOT_LAG})\n\
  --chain-node-validator <v:s> oasis7_chain_runtime validator (repeatable)\n\
  --with-llm                   enable llm mode (default; required for gameplay)\n\
  --agent-decision-source <src>\n\
                               agent decision source: builtin_llm|provider_backed\n\
  --agent-provider-backend <id>\n\
                               provider backend: provider_local_bridge (default when provider_backed)\n\
  --agent-provider-contract <id>\n\
                               provider contract: worldsim_provider_v1 (default when provider_backed)\n\
  --agent-provider-transport <id>\n\
                               provider transport: loopback_http (default when provider_backed)\n\
  --agent-provider-url <url>   provider URL (default: {DEFAULT_AGENT_PROVIDER_URL})\n\
  --agent-provider-auth-token <tok>\n\
                               provider bearer token\n\
  --agent-provider-connect-timeout-ms <ms>\n\
                               provider connect timeout ms (default: {DEFAULT_AGENT_PROVIDER_CONNECT_TIMEOUT_MS})\n\
  --agent-provider-profile <id>\n\
                               provider profile (default: {DEFAULT_AGENT_PROVIDER_PROFILE})\n\
  --agent-execution-lane <mode>\n\
                               execution lane: player_parity|headless_agent (default: headless_agent)\n\
  --agent-provider-mode <mode> legacy alias for --agent-decision-source; accepts agent_direct_connect/provider_loopback_http\n\\
  --no-open-browser            do not auto open browser\n\
  -h, --help                   show help\n\n\
Env:\n\
  OASIS7_VIEWER_LIVE_BIN              explicit path of oasis7_viewer_live binary\n\
  OASIS7_CHAIN_RUNTIME_BIN            explicit path of oasis7_chain_runtime binary\n\
  OASIS7_GAME_STATIC_DIR              override default viewer static dir when --viewer-static-dir is omitted"
    );
}
