use super::*;

#[test]
fn provider_settings_from_env_defaults_to_none() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    let settings =
        super::control_plane::runtime_provider_settings_from_env().expect("settings parse");
    assert_eq!(settings, None);
}

#[test]
fn provider_settings_from_env_parses_profile_and_timeout() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    std::env::set_var(VIEWER_AGENT_DECISION_SOURCE_ENV, "provider_backed");
    std::env::set_var(VIEWER_AGENT_PROVIDER_BACKEND_ENV, "provider_local_bridge");
    std::env::set_var(VIEWER_AGENT_PROVIDER_CONTRACT_ENV, "worldsim_provider_v1");
    std::env::set_var(VIEWER_AGENT_PROVIDER_TRANSPORT_ENV, "loopback_http");
    std::env::set_var(VIEWER_AGENT_PROVIDER_URL_ENV, "http://127.0.0.1:5841");
    std::env::set_var(VIEWER_AGENT_PROVIDER_CONNECT_TIMEOUT_MS_ENV, "4200");
    std::env::set_var(VIEWER_AGENT_PROVIDER_PROFILE_ENV, "oasis7_p0_low_freq_npc");
    std::env::set_var(VIEWER_AGENT_EXECUTION_LANE_ENV, "player_parity");
    std::env::set_var(VIEWER_AGENT_PROVIDER_AUTH_TOKEN_ENV, "secret-token");
    let settings = super::control_plane::runtime_provider_settings_from_env()
        .expect("settings parse")
        .expect("provider settings");
    assert_eq!(settings.requested_provider_mode, "provider_backed");
    assert_eq!(settings.provider_transport, "loopback_http");
    assert_eq!(settings.base_url, "http://127.0.0.1:5841");
    assert_eq!(settings.connect_timeout_ms, 4200);
    assert_eq!(settings.agent_profile, "oasis7_p0_low_freq_npc");
    assert_eq!(settings.execution_mode, ProviderExecutionMode::PlayerParity);
    assert_eq!(settings.auth_token.as_deref(), Some("secret-token"));
    assert_eq!(settings.fallback_reason, None);
    clear_runtime_provider_env();
}

#[test]
fn provider_settings_from_env_accepts_remote_https_transport() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    std::env::set_var(VIEWER_AGENT_DECISION_SOURCE_ENV, "provider_backed");
    std::env::set_var(VIEWER_AGENT_PROVIDER_BACKEND_ENV, "provider_local_bridge");
    std::env::set_var(VIEWER_AGENT_PROVIDER_CONTRACT_ENV, "worldsim_provider_v1");
    std::env::set_var(VIEWER_AGENT_PROVIDER_TRANSPORT_ENV, "remote_https");
    std::env::set_var(VIEWER_AGENT_PROVIDER_URL_ENV, "https://provider.example");
    std::env::set_var(VIEWER_AGENT_PROVIDER_CONNECT_TIMEOUT_MS_ENV, "4200");
    std::env::set_var(VIEWER_AGENT_PROVIDER_PROFILE_ENV, "oasis7_p0_low_freq_npc");
    std::env::set_var(VIEWER_AGENT_EXECUTION_LANE_ENV, "player_parity");
    std::env::set_var(VIEWER_AGENT_PROVIDER_AUTH_TOKEN_ENV, "secret-token");
    let settings = super::control_plane::runtime_provider_settings_from_env()
        .expect("settings parse")
        .expect("provider settings");
    assert_eq!(settings.provider_transport, "remote_https");
    assert_eq!(settings.base_url, "https://provider.example");
    assert_eq!(settings.auth_token.as_deref(), Some("secret-token"));
    clear_runtime_provider_env();
}

#[test]
fn provider_settings_from_env_rejects_removed_old_brand_prefix() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    std::env::set_var(
        removed_old_brand_runtime_live_env("AGENT_PROVIDER_MODE"),
        "provider_loopback_http",
    );
    std::env::set_var(
        removed_old_brand_runtime_live_env("AGENT_PROVIDER_URL"),
        "http://127.0.0.1:5842",
    );
    std::env::set_var(
        removed_old_brand_runtime_live_env("AGENT_PROVIDER_CONNECT_TIMEOUT_MS"),
        "4300",
    );
    std::env::set_var(
        removed_old_brand_runtime_live_env("AGENT_PROVIDER_PROFILE"),
        "oasis7_p0_low_freq_npc",
    );
    std::env::set_var(
        removed_old_brand_runtime_live_env("AGENT_EXECUTION_LANE"),
        "player_parity",
    );
    std::env::set_var(
        removed_old_brand_runtime_live_env("AGENT_PROVIDER_AUTH_TOKEN"),
        "removed-old-brand-token",
    );

    let settings =
        super::control_plane::runtime_provider_settings_from_env().expect("settings parse");
    assert_eq!(settings, None);
    clear_runtime_provider_env();
}
