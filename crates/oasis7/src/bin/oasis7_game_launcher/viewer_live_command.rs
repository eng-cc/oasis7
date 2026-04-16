use super::*;

pub(super) fn build_oasis7_viewer_live_command(
    path: &Path,
    options: &CliOptions,
    parent_has_llm_timeout_ms: bool,
    repo_has_node_config_file: bool,
) -> Command {
    let mut command = Command::new(path);
    command
        .arg(options.scenario.as_str())
        .arg("--bind")
        .arg(options.live_bind.as_str())
        .arg("--web-bind")
        .arg(options.web_bind.as_str())
        .arg("--deployment-mode")
        .arg(options.deployment_mode.as_str());
    if options.chain_enabled {
        command
            .arg("--chain-status-bind")
            .arg(options.chain_status_bind.as_str());
    }
    if options.with_llm {
        command.arg("--llm");
        apply_viewer_live_env_overrides(
            &mut command,
            options,
            parent_has_llm_timeout_ms,
            repo_has_node_config_file,
        );
    } else {
        command.arg("--no-llm");
    }
    command
}

pub(super) fn apply_viewer_live_env_overrides(
    command: &mut Command,
    options: &CliOptions,
    parent_has_llm_timeout_ms: bool,
    repo_has_node_config_file: bool,
) {
    for env_name in [
        VIEWER_AGENT_DECISION_SOURCE_ENV,
        VIEWER_AGENT_PROVIDER_BACKEND_ENV,
        VIEWER_AGENT_PROVIDER_CONTRACT_ENV,
        VIEWER_AGENT_PROVIDER_TRANSPORT_ENV,
        VIEWER_AGENT_PROVIDER_URL_ENV,
        VIEWER_AGENT_PROVIDER_AUTH_TOKEN_ENV,
        VIEWER_AGENT_PROVIDER_CONNECT_TIMEOUT_MS_ENV,
        VIEWER_AGENT_PROVIDER_PROFILE_ENV,
        VIEWER_AGENT_EXECUTION_LANE_ENV,
        VIEWER_AGENT_PROVIDER_MODE_ENV,
    ] {
        command.env_remove(env_name);
    }

    if uses_loopback_provider(options) {
        command.env(
            VIEWER_AGENT_DECISION_SOURCE_ENV,
            PROVIDER_BACKED_DECISION_SOURCE,
        );
        command.env(
            VIEWER_AGENT_PROVIDER_BACKEND_ENV,
            LOCAL_BRIDGE_PROVIDER_BACKEND,
        );
        command.env(
            VIEWER_AGENT_PROVIDER_CONTRACT_ENV,
            WORLDSIM_PROVIDER_CONTRACT,
        );
        command.env(
            VIEWER_AGENT_PROVIDER_TRANSPORT_ENV,
            LOOPBACK_HTTP_PROVIDER_TRANSPORT,
        );
        command.env(
            VIEWER_AGENT_PROVIDER_URL_ENV,
            options.agent_provider_url.as_str(),
        );
        if !options.agent_provider_auth_token.trim().is_empty() {
            command.env(
                VIEWER_AGENT_PROVIDER_AUTH_TOKEN_ENV,
                options.agent_provider_auth_token.as_str(),
            );
        }
        command.env(
            VIEWER_AGENT_PROVIDER_CONNECT_TIMEOUT_MS_ENV,
            options.agent_provider_connect_timeout_ms.to_string(),
        );
        command.env(
            VIEWER_AGENT_PROVIDER_PROFILE_ENV,
            options.agent_provider_profile.as_str(),
        );
        command.env(
            VIEWER_AGENT_EXECUTION_LANE_ENV,
            options.agent_execution_lane.as_str(),
        );
        return;
    }

    if !parent_has_llm_timeout_ms && !repo_has_node_config_file {
        command.env(
            LLM_TIMEOUT_MS_ENV,
            DEFAULT_INTERACTIVE_LLM_TIMEOUT_MS.to_string(),
        );
    }
}
