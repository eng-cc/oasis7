use super::*;

pub(super) fn env_requests_provider_backend() -> bool {
    named_env_var_any(&[
        VIEWER_AGENT_DECISION_SOURCE_ENV,
        VIEWER_AGENT_PROVIDER_MODE_ENV,
    ])
    .map(|value| value.trim().to_string())
    .as_deref()
    .and_then(canonical_agent_decision_source)
    .is_some_and(|value| value == PROVIDER_BACKED_DECISION_SOURCE)
}

pub(in crate::viewer::runtime_live) fn provider_settings_from_env(
) -> Result<Option<ProviderDecisionSettings>, String> {
    let decision_source = named_env_var_any(&[
        VIEWER_AGENT_DECISION_SOURCE_ENV,
        VIEWER_AGENT_PROVIDER_MODE_ENV,
    ])
    .unwrap_or_default();
    let decision_source = decision_source.trim();
    if decision_source.is_empty() || decision_source == BUILTIN_LLM_DECISION_SOURCE {
        return Ok(None);
    }
    let Some(_) = canonical_agent_decision_source(decision_source) else {
        return Err(format!(
            "unsupported agent decision source `{decision_source}`; expected builtin_llm or provider_backed"
        ));
    };

    let backend = named_env_var_any(&[
        VIEWER_AGENT_PROVIDER_BACKEND_ENV,
        VIEWER_AGENT_PROVIDER_MODE_ENV,
    ])
    .unwrap_or_else(|| LOCAL_BRIDGE_PROVIDER_BACKEND.to_string());
    let Some(_) = canonical_agent_provider_backend(backend.as_str()) else {
        return Err(format!(
            "unsupported agent provider backend `{backend}`; expected provider_local_bridge"
        ));
    };
    let contract = named_env_var_any(&[
        VIEWER_AGENT_PROVIDER_CONTRACT_ENV,
        VIEWER_AGENT_PROVIDER_MODE_ENV,
    ])
    .unwrap_or_else(|| WORLDSIM_PROVIDER_CONTRACT.to_string());
    let Some(_) = canonical_agent_provider_contract(contract.as_str()) else {
        return Err(format!(
            "unsupported agent provider contract `{contract}`; expected worldsim_provider_v1"
        ));
    };
    let transport = named_env_var_any(&[
        VIEWER_AGENT_PROVIDER_TRANSPORT_ENV,
        VIEWER_AGENT_PROVIDER_MODE_ENV,
    ])
    .unwrap_or_else(|| LOOPBACK_HTTP_PROVIDER_TRANSPORT.to_string());
    let Some(_) = canonical_agent_provider_transport(transport.as_str()) else {
        return Err(format!(
            "unsupported agent provider transport `{transport}`; expected loopback_http or remote_https"
        ));
    };

    let base_url = named_env_var_any(&[VIEWER_AGENT_PROVIDER_URL_ENV]).unwrap_or_default();
    let base_url = base_url.trim();
    if base_url.is_empty() {
        return Err(format!(
            "{VIEWER_AGENT_PROVIDER_URL_ENV} is required for provider_backed/provider_local_bridge"
        ));
    }

    let connect_timeout_ms = named_env_var_any(&[VIEWER_AGENT_PROVIDER_CONNECT_TIMEOUT_MS_ENV])
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(|value| {
            value.parse::<u64>().map_err(|err| {
                format!(
                    "invalid {VIEWER_AGENT_PROVIDER_CONNECT_TIMEOUT_MS_ENV} value `{value}`: {err}"
                )
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_PROVIDER_CONNECT_TIMEOUT_MS);
    if connect_timeout_ms == 0 {
        return Err(format!(
            "{VIEWER_AGENT_PROVIDER_CONNECT_TIMEOUT_MS_ENV} must be greater than zero"
        ));
    }

    let agent_profile = named_env_var_any(&[VIEWER_AGENT_PROVIDER_PROFILE_ENV])
        .unwrap_or_else(|| DEFAULT_PROVIDER_AGENT_PROFILE.to_string());
    let agent_profile = agent_profile.trim();
    if agent_profile.is_empty() {
        return Err(format!(
            "{VIEWER_AGENT_PROVIDER_PROFILE_ENV} cannot be empty for provider_backed/provider_local_bridge"
        ));
    }

    let auth_token = named_env_var_any(&[VIEWER_AGENT_PROVIDER_AUTH_TOKEN_ENV])
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let execution_mode = named_env_var_any(&[VIEWER_AGENT_EXECUTION_LANE_ENV])
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(|value| {
            ProviderExecutionMode::parse(value.as_str()).ok_or_else(|| {
                format!(
                    "invalid {VIEWER_AGENT_EXECUTION_LANE_ENV} value `{value}`: expected player_parity or headless_agent"
                )
            })
        })
        .transpose()?
        .unwrap_or(ProviderExecutionMode::HeadlessAgent);

    Ok(Some(ProviderDecisionSettings {
        requested_provider_mode: decision_source.to_string(),
        provider_transport: canonical_agent_provider_transport(transport.as_str())
            .unwrap_or(LOOPBACK_HTTP_PROVIDER_TRANSPORT)
            .to_string(),
        base_url: base_url.to_string(),
        auth_token,
        connect_timeout_ms,
        agent_profile: agent_profile.to_string(),
        execution_mode,
        fallback_reason: provider_mode_fallback_reason(decision_source),
    }))
}

pub(super) fn provider_phase1_action_catalog() -> Vec<ActionCatalogEntry> {
    vec![
        ActionCatalogEntry::new("wait", "yield current turn without acting"),
        ActionCatalogEntry::new("wait_ticks", "sleep for a bounded number of ticks"),
        ActionCatalogEntry::new("move_agent", "move to a neighboring location"),
        ActionCatalogEntry::new(
            "harvest_radiation",
            "recover electricity before industrial expansion or recipe execution",
        ),
        ActionCatalogEntry::new(
            "mine_compound",
            "extract raw compound mass when recovery needs material inputs",
        ),
        ActionCatalogEntry::new(
            "refine_compound",
            "convert compound mass into hardware output for recovery",
        ),
        ActionCatalogEntry::new(
            "build_factory",
            "start the first compatible factory line for industrial progression",
        ),
        ActionCatalogEntry::new(
            "schedule_recipe",
            "run the next compatible recipe on an existing factory line",
        ),
        ActionCatalogEntry::new("speak_to_nearby", "emit a lightweight nearby speech event"),
        ActionCatalogEntry::new(
            "inspect_target",
            "emit a lightweight target inspection event",
        ),
        ActionCatalogEntry::new(
            "simple_interact",
            "emit a lightweight simple interaction event",
        ),
    ]
}

pub(super) fn provider_phase1_memory_summary() -> String {
    concat!(
        "goal=post_onboarding.establish_first_capability; ",
        "开局默认种子资源通常已足够首个 smelter（例如 electricity>=10 且 data>=5）；",
        "若当前没有 factory.smelter.mk1，不要先 harvest_radiation，优先 build_factory(factory.smelter.mk1)。",
        "只有在 electricity<10 时才先 harvest_radiation；只有在 data<5 时才先 mine_compound/refine_compound。 ",
        "build_factory 成功后立刻 schedule_recipe(",
        "recipe.smelter.iron_ingot|recipe.smelter.copper_wire|recipe.smelter.polymer_resin|recipe.smelter.alloy_plate",
        ")；不要长期停留在 wait/move/speak/inspect。"
    )
    .to_string()
}

fn canonical_agent_decision_source(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        BUILTIN_LLM_DECISION_SOURCE => Some(BUILTIN_LLM_DECISION_SOURCE),
        PROVIDER_BACKED_DECISION_SOURCE
        | PROVIDER_LOOPBACK_HTTP_IMPLEMENTATION
        | AGENT_DIRECT_CONNECT_PROVIDER_MODE_ALIAS => Some(PROVIDER_BACKED_DECISION_SOURCE),
        _ => None,
    }
}

fn canonical_agent_provider_backend(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        LOCAL_BRIDGE_PROVIDER_BACKEND
        | PROVIDER_LOOPBACK_HTTP_IMPLEMENTATION
        | AGENT_DIRECT_CONNECT_PROVIDER_MODE_ALIAS => Some(LOCAL_BRIDGE_PROVIDER_BACKEND),
        _ => None,
    }
}

fn canonical_agent_provider_contract(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        WORLDSIM_PROVIDER_CONTRACT
        | PROVIDER_LOOPBACK_HTTP_IMPLEMENTATION
        | AGENT_DIRECT_CONNECT_PROVIDER_MODE_ALIAS => Some(WORLDSIM_PROVIDER_CONTRACT),
        _ => None,
    }
}

fn canonical_agent_provider_transport(raw: &str) -> Option<&'static str> {
    match raw.trim() {
        LOOPBACK_HTTP_PROVIDER_TRANSPORT
        | PROVIDER_LOOPBACK_HTTP_IMPLEMENTATION
        | AGENT_DIRECT_CONNECT_PROVIDER_MODE_ALIAS => Some(LOOPBACK_HTTP_PROVIDER_TRANSPORT),
        REMOTE_HTTPS_PROVIDER_TRANSPORT => Some(REMOTE_HTTPS_PROVIDER_TRANSPORT),
        _ => None,
    }
}

fn named_env_var_any(env_names: &[&str]) -> Option<String> {
    env_names
        .iter()
        .find_map(|env_name| std::env::var(env_name).ok())
}

fn provider_mode_fallback_reason(provider_mode: &str) -> Option<String> {
    match provider_mode.trim() {
        AGENT_DIRECT_CONNECT_PROVIDER_MODE_ALIAS => {
            Some("provider_mode_alias:agent_direct_connect".to_string())
        }
        _ => None,
    }
}
