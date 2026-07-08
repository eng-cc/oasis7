use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::thread;

use super::{
    BUILTIN_LLM_DECISION_SOURCE, CliOptions, DEFAULT_AGENT_PROVIDER_CONNECT_TIMEOUT_MS,
    DEFAULT_AGENT_PROVIDER_PROFILE, DEFAULT_AGENT_PROVIDER_URL, DEFAULT_CHAIN_LINK_POLICY,
    DEFAULT_CHAIN_NODE_ID, DEFAULT_CHAIN_STATUS_BIND, DEFAULT_DEPLOYMENT_MODE,
    DEFAULT_INTERACTIVE_LLM_TIMEOUT_MS, DEFAULT_LIVE_BIND, DEFAULT_SCENARIO,
    DEFAULT_VIEWER_STATIC_DIR, DeploymentMode, GAME_STATIC_DIR_ENV, LLM_TIMEOUT_MS_ENV,
    LOCAL_BRIDGE_PROVIDER_BACKEND, LOCAL_MOCK_PROVIDER_BACKEND, LOOPBACK_HTTP_PROVIDER_TRANSPORT,
    PROVIDER_BACKED_DECISION_SOURCE, VIEWER_AGENT_DECISION_SOURCE_ENV,
    VIEWER_AGENT_EXECUTION_LANE_ENV, VIEWER_AGENT_PROVIDER_AUTH_TOKEN_ENV,
    VIEWER_AGENT_PROVIDER_BACKEND_ENV, VIEWER_AGENT_PROVIDER_CONNECT_TIMEOUT_MS_ENV,
    VIEWER_AGENT_PROVIDER_CONTRACT_ENV, VIEWER_AGENT_PROVIDER_DECISION_TIMEOUT_MS_ENV,
    VIEWER_AGENT_PROVIDER_MODE_ENV, VIEWER_AGENT_PROVIDER_PROFILE_ENV,
    VIEWER_AGENT_PROVIDER_TRANSPORT_ENV, VIEWER_AGENT_PROVIDER_URL_ENV, WORLDSIM_PROVIDER_CONTRACT,
    apply_viewer_live_env_overrides, build_game_url, build_oasis7_chain_runtime_args,
    build_oasis7_viewer_live_command, content_type_for_path,
    missing_execution_world_persistence_files, parse_host_port, parse_options,
    query_runtime_bound_players, resolve_static_asset_path,
    resolve_viewer_static_dir_with_override, sanitize_index_html_for_embedded_server,
    sanitize_relative_request_path, start_static_http_server, stop_static_http_server,
    viewer_dev_dist_candidates,
};
use oasis7::launcher_bootstrap_peers::DEFAULT_CHAIN_REPLICATION_BOOTSTRAP_PEERS;
use oasis7::simulator::ProviderExecutionMode;
use oasis7::simulator::{WorldConfig, WorldModel, WorldSnapshot};
use oasis7::viewer::{VIEWER_PROTOCOL_VERSION, ViewerRequest, ViewerResponse};
use oasis7_proto::storage_profile::StorageProfile;

#[path = "viewer_static_dir_tests.rs"]
mod viewer_static_dir_tests;
use viewer_static_dir_tests::make_temp_dir;
#[path = "launcher_hosted_public_join_tests.rs"]
mod launcher_hosted_public_join_tests;
#[path = "launcher_static_http_tests.rs"]
mod launcher_static_http_tests;
#[path = "launcher_viewer_auth_bootstrap_tests.rs"]
mod launcher_viewer_auth_bootstrap_tests;

fn command_env_value(command: &Command, key: &str) -> Option<Option<String>> {
    command
        .get_envs()
        .find(|(env_key, _)| env_key.to_string_lossy() == key)
        .map(|(_, value)| value.map(|value| value.to_string_lossy().into_owned()))
}

fn make_generated_world_fixture(label: &str) -> std::path::PathBuf {
    let root = make_temp_dir(label);
    fs::create_dir_all(root.join("generated-scenario-world")).expect("create sidecar dir");
    fs::write(
        root.join("generated-scenario-world").join("snapshot.json"),
        "{}",
    )
    .expect("write sidecar snapshot");
    fs::write(
        root.join("generated-scenario-world").join("journal.json"),
        "{}",
    )
    .expect("write sidecar journal");
    fs::write(root.join("world-generation-provenance.json"), "{}").expect("write provenance");
    root
}

#[test]
fn parse_options_defaults() {
    let options = parse_options(std::iter::empty()).expect("parse should succeed");
    assert_eq!(options.scenario, DEFAULT_SCENARIO);
    assert_eq!(options.live_bind, DEFAULT_LIVE_BIND);
    assert_eq!(options.deployment_mode, DEFAULT_DEPLOYMENT_MODE);
    assert!(options.with_llm);
    assert!(options.auto_play);
    assert!(!options.allow_debug_scenario);
    assert_eq!(options.generated_world_dir, "");
    assert_eq!(
        options.agent_decision_source,
        PROVIDER_BACKED_DECISION_SOURCE
    );
    assert_eq!(
        options.agent_provider_backend,
        LOCAL_BRIDGE_PROVIDER_BACKEND
    );
    assert_eq!(options.agent_provider_contract, WORLDSIM_PROVIDER_CONTRACT);
    assert_eq!(
        options.agent_provider_transport,
        LOOPBACK_HTTP_PROVIDER_TRANSPORT
    );
    assert_eq!(options.agent_provider_url, DEFAULT_AGENT_PROVIDER_URL);
    assert_eq!(options.agent_provider_auth_token, "");
    assert_eq!(
        options.agent_provider_profile,
        DEFAULT_AGENT_PROVIDER_PROFILE
    );
    assert_eq!(
        options.agent_execution_lane,
        ProviderExecutionMode::HeadlessAgent
    );
    assert_eq!(
        options.agent_provider_connect_timeout_ms,
        DEFAULT_AGENT_PROVIDER_CONNECT_TIMEOUT_MS
    );
    assert!(options.open_browser);
    assert_eq!(options.viewer_static_dir, "web");
    assert!(!options.chain_enabled);
    assert_eq!(options.chain_status_bind, DEFAULT_CHAIN_STATUS_BIND);
    assert_eq!(options.chain_link_policy, DEFAULT_CHAIN_LINK_POLICY);
    assert!(
        options
            .chain_node_id
            .starts_with(&format!("{DEFAULT_CHAIN_NODE_ID}-fresh-"))
    );
    assert_eq!(options.chain_storage_profile, StorageProfile::DevLocal);
    assert_eq!(options.chain_node_role, "sequencer");
    assert_eq!(options.chain_p2p_user_mode, "auto_join");
    assert!(!options.chain_p2p_accept_public_entry);
    assert_eq!(
        options.chain_replication_bootstrap_peers,
        DEFAULT_CHAIN_REPLICATION_BOOTSTRAP_PEERS
            .iter()
            .map(|peer| (*peer).to_string())
            .collect::<Vec<_>>()
    );
    let pos_defaults = oasis7::chain_pos_defaults::defaults();
    assert_eq!(
        options.chain_pos_slot_duration_ms,
        pos_defaults.slot_duration_ms
    );
    assert_eq!(
        options.chain_pos_ticks_per_slot,
        pos_defaults.ticks_per_slot
    );
    assert_eq!(
        options.chain_pos_proposal_tick_phase,
        pos_defaults.proposal_tick_phase
    );
    assert!(!options.chain_pos_adaptive_tick_scheduler_enabled);
    assert_eq!(options.chain_pos_slot_clock_genesis_unix_ms, None);
    assert_eq!(
        options.chain_pos_max_past_slot_lag,
        pos_defaults.max_past_slot_lag
    );
    assert_eq!(options.chain_world_id, None);
    assert!(!options.chain_local_standalone_test);
    assert!(!options.chain_node_auto_attest_all_validators);
}

#[test]
fn parse_options_supports_no_auto_play() {
    let options = parse_options(["--no-auto-play"].into_iter()).expect("parse should succeed");
    assert!(!options.auto_play);
}

#[test]
fn parse_options_accepts_generated_world_dir() {
    let root = make_generated_world_fixture("launcher_generated_world");
    let options = parse_options(
        [
            "--generated-world-dir",
            root.to_str().expect("utf8 temp path"),
            "--no-open-browser",
        ]
        .into_iter(),
    )
    .expect("generated world dir");
    assert_eq!(options.generated_world_dir, root.to_string_lossy());
    assert_eq!(options.scenario, "");
    fs::remove_dir_all(root).expect("cleanup generated world fixture");
}

#[test]
fn parse_options_rejects_generated_world_dir_with_scenario() {
    let root = make_generated_world_fixture("launcher_generated_world_scenario");
    let err = parse_options(
        [
            "--scenario",
            "minimal",
            "--generated-world-dir",
            root.to_str().expect("utf8 temp path"),
        ]
        .into_iter(),
    )
    .expect_err("generated world dir conflicts with scenario");
    assert!(err.contains("cannot be combined"));
    fs::remove_dir_all(root).expect("cleanup generated world fixture");
}

#[test]
fn parse_options_rejects_llm_bootstrap_without_debug_opt_in() {
    let err = parse_options(["--scenario", "llm_bootstrap"].into_iter())
        .expect_err("debug scenario should require opt-in");
    assert!(err.contains("seeded debug/LLM scenario"));
    assert!(err.contains("--allow-debug-scenario"));
}

#[test]
fn parse_options_accepts_llm_bootstrap_with_debug_opt_in() {
    let options =
        parse_options(["--scenario", "llm_bootstrap", "--allow-debug-scenario"].into_iter())
            .expect("debug scenario opt-in");
    assert_eq!(options.scenario, "llm_bootstrap");
    assert!(options.allow_debug_scenario);
}

#[test]
fn parse_options_accepts_overrides() {
    let options = parse_options(
        [
            "--scenario",
            "twin_region_bootstrap",
            "--deployment-mode",
            "hosted_public_join",
            "--live-bind",
            "127.0.0.1:6200",
            "--web-bind",
            "127.0.0.1:6201",
            "--viewer-host",
            "0.0.0.0",
            "--viewer-port",
            "4777",
            "--viewer-static-dir",
            "dist",
            "--chain-status-bind",
            "127.0.0.1:6331",
            "--chain-link-policy",
            "shadow",
            "--chain-node-id",
            "chain-a",
            "--chain-storage-profile",
            "soak_forensics",
            "--chain-world-id",
            "live-chain-a",
            "--chain-node-role",
            "storage",
            "--chain-p2p-user-mode",
            "public_entry",
            "--chain-p2p-accept-public-entry",
            "--chain-replication-network-peer",
            "/ip4/127.0.0.1/tcp/4100",
            "--chain-replication-network-peer",
            "/dns4/bootstrap.example/tcp/4101",
            "--chain-node-tick-ms",
            "350",
            "--chain-pos-slot-duration-ms",
            "8000",
            "--chain-pos-ticks-per-slot",
            "10",
            "--chain-pos-proposal-tick-phase",
            "9",
            "--chain-pos-adaptive-tick-scheduler",
            "--chain-pos-slot-clock-genesis-unix-ms",
            "1700000000000",
            "--chain-pos-max-past-slot-lag",
            "32",
            "--chain-node-validator",
            "chain-a:55",
            "--chain-node-auto-attest-all",
            "--with-llm",
            "--auto-play",
            "--agent-decision-source",
            "provider_backed",
            "--agent-provider-backend",
            "provider_local_bridge",
            "--agent-provider-contract",
            "worldsim_provider_v1",
            "--agent-provider-transport",
            "loopback_http",
            "--agent-provider-url",
            "http://127.0.0.1:5841",
            "--agent-provider-auth-token",
            "secret-token",
            "--agent-provider-connect-timeout-ms",
            "3000",
            "--agent-provider-profile",
            "oasis7_p0_low_freq_npc",
            "--agent-execution-lane",
            "player_parity",
            "--no-open-browser",
        ]
        .into_iter(),
    )
    .expect("parse should succeed");

    assert_eq!(options.scenario, "twin_region_bootstrap");
    assert_eq!(options.deployment_mode, "hosted_public_join");
    assert_eq!(options.live_bind, "127.0.0.1:6200");
    assert_eq!(options.web_bind, "127.0.0.1:6201");
    assert_eq!(options.viewer_host, "0.0.0.0");
    assert_eq!(options.viewer_port, 4777);
    assert_eq!(options.viewer_static_dir, "dist");
    assert!(options.auto_play);
    assert_eq!(options.chain_status_bind, "127.0.0.1:6331");
    assert_eq!(options.chain_link_policy, "shadow");
    assert_eq!(options.chain_node_id, "chain-a");
    assert_eq!(options.chain_network_tier_manifest, "");
    assert_eq!(options.chain_storage_profile, StorageProfile::SoakForensics);
    assert_eq!(options.chain_world_id, Some("live-chain-a".to_string()));
    assert_eq!(options.chain_node_role, "storage");
    assert_eq!(options.chain_p2p_user_mode, "public_entry");
    assert!(options.chain_p2p_accept_public_entry);
    assert_eq!(
        options.chain_replication_bootstrap_peers,
        vec![
            "/ip4/127.0.0.1/tcp/4100".to_string(),
            "/dns4/bootstrap.example/tcp/4101".to_string(),
        ]
    );
    assert_eq!(options.chain_node_tick_ms, 350);
    assert_eq!(options.chain_pos_slot_duration_ms, 8_000);
    assert_eq!(options.chain_pos_ticks_per_slot, 10);
    assert_eq!(options.chain_pos_proposal_tick_phase, 9);
    assert!(options.chain_pos_adaptive_tick_scheduler_enabled);
    assert_eq!(
        options.chain_pos_slot_clock_genesis_unix_ms,
        Some(1_700_000_000_000)
    );
    assert_eq!(options.chain_pos_max_past_slot_lag, 32);
    assert_eq!(
        options.chain_node_validators,
        vec!["chain-a:55".to_string()]
    );
    assert!(options.chain_node_auto_attest_all_validators);
    assert!(options.with_llm);
    assert_eq!(
        options.agent_decision_source,
        PROVIDER_BACKED_DECISION_SOURCE
    );
    assert_eq!(
        options.agent_provider_backend,
        LOCAL_BRIDGE_PROVIDER_BACKEND
    );
    assert_eq!(options.agent_provider_contract, WORLDSIM_PROVIDER_CONTRACT);
    assert_eq!(
        options.agent_provider_transport,
        LOOPBACK_HTTP_PROVIDER_TRANSPORT
    );
    assert_eq!(options.agent_provider_url, "http://127.0.0.1:5841");
    assert_eq!(options.agent_provider_auth_token, "secret-token");
    assert_eq!(options.agent_provider_connect_timeout_ms, 3000);
    assert_eq!(options.agent_provider_profile, "oasis7_p0_low_freq_npc");
    assert_eq!(
        options.agent_execution_lane,
        ProviderExecutionMode::PlayerParity
    );
    assert!(!options.chain_enabled);
    assert!(!options.open_browser);
}

#[test]
fn parse_options_accepts_remote_https_provider_transport() {
    let options = parse_options(
        [
            "--with-llm",
            "--agent-decision-source",
            "provider_backed",
            "--agent-provider-backend",
            "provider_local_bridge",
            "--agent-provider-contract",
            "worldsim_provider_v1",
            "--agent-provider-transport",
            "remote_https",
            "--agent-provider-url",
            "https://provider.example",
            "--agent-provider-auth-token",
            "secret-token",
            "--agent-provider-connect-timeout-ms",
            "3000",
            "--agent-provider-profile",
            "oasis7_p0_low_freq_npc",
            "--agent-execution-lane",
            "player_parity",
        ]
        .into_iter(),
    )
    .expect("parse should succeed");

    assert_eq!(options.agent_provider_transport, "remote_https");
    assert_eq!(options.agent_provider_url, "https://provider.example");
}

#[test]
fn parse_options_accepts_local_mock_provider_backend() {
    let options = parse_options(
        [
            "--with-llm",
            "--agent-decision-source",
            "provider_backed",
            "--agent-provider-backend",
            "provider_local_mock",
            "--agent-provider-contract",
            "worldsim_provider_v1",
            "--agent-provider-transport",
            "loopback_http",
            "--agent-provider-url",
            "http://127.0.0.1:5841",
            "--agent-provider-connect-timeout-ms",
            "3000",
            "--agent-provider-profile",
            "oasis7_p0_low_freq_npc",
        ]
        .into_iter(),
    )
    .expect("parse should succeed");

    assert_eq!(options.agent_provider_backend, LOCAL_MOCK_PROVIDER_BACKEND);
    assert_eq!(
        options.agent_provider_transport,
        LOOPBACK_HTTP_PROVIDER_TRANSPORT
    );
    assert_eq!(options.agent_provider_url, DEFAULT_AGENT_PROVIDER_URL);
}

#[test]
fn parse_options_accepts_chain_disable() {
    let options = parse_options(["--chain-disable"].into_iter()).expect("parse should succeed");
    assert!(!options.chain_enabled);
}

#[test]
fn parse_options_collects_repeat_replication_bootstrap_peers() {
    let options = parse_options(
        [
            "--chain-replication-network-peer",
            "/ip4/127.0.0.1/tcp/4100",
            "--chain-replication-network-peer",
            "/dns4/bootstrap.example/tcp/4101",
        ]
        .into_iter(),
    )
    .expect("parse should succeed");

    assert_eq!(
        options.chain_replication_bootstrap_peers,
        vec![
            "/ip4/127.0.0.1/tcp/4100".to_string(),
            "/dns4/bootstrap.example/tcp/4101".to_string(),
        ]
    );
}

#[test]
fn parse_options_accepts_agent_direct_connect_alias() {
    let options = parse_options(
        [
            "--with-llm",
            "--agent-provider-mode",
            "agent_direct_connect",
            "--agent-provider-url",
            "http://127.0.0.1:5841",
            "--agent-provider-profile",
            "oasis7_p0_low_freq_npc",
        ]
        .into_iter(),
    )
    .expect("parse should succeed");

    assert_eq!(
        options.agent_decision_source,
        PROVIDER_BACKED_DECISION_SOURCE
    );
    assert_eq!(
        options.agent_provider_backend,
        LOCAL_BRIDGE_PROVIDER_BACKEND
    );
    assert_eq!(options.agent_provider_contract, WORLDSIM_PROVIDER_CONTRACT);
    assert_eq!(
        options.agent_provider_transport,
        LOOPBACK_HTTP_PROVIDER_TRANSPORT
    );
}

#[test]
fn builtin_viewer_live_env_applies_default_llm_timeout_when_parent_is_unset() {
    let mut options = CliOptions::default();
    options.agent_decision_source = BUILTIN_LLM_DECISION_SOURCE.to_string();
    let mut command = Command::new("echo");

    apply_viewer_live_env_overrides(&mut command, &options, false, false);

    assert_eq!(
        command_env_value(&command, LLM_TIMEOUT_MS_ENV),
        Some(Some(DEFAULT_INTERACTIVE_LLM_TIMEOUT_MS.to_string()))
    );
    assert_eq!(
        command_env_value(&command, VIEWER_AGENT_DECISION_SOURCE_ENV),
        Some(None)
    );
}

#[test]
fn builtin_viewer_live_env_preserves_explicit_parent_llm_timeout() {
    let mut options = CliOptions::default();
    options.agent_decision_source = BUILTIN_LLM_DECISION_SOURCE.to_string();
    let mut command = Command::new("echo");

    apply_viewer_live_env_overrides(&mut command, &options, true, false);

    assert_eq!(command_env_value(&command, LLM_TIMEOUT_MS_ENV), None);
}

#[test]
fn builtin_viewer_live_env_skips_default_llm_timeout_when_repo_config_exists() {
    let mut options = CliOptions::default();
    options.agent_decision_source = BUILTIN_LLM_DECISION_SOURCE.to_string();
    let mut command = Command::new("echo");

    apply_viewer_live_env_overrides(&mut command, &options, false, true);

    assert_eq!(command_env_value(&command, LLM_TIMEOUT_MS_ENV), None);
}

#[test]
fn provider_backed_viewer_live_env_sets_provider_specific_overrides_without_builtin_llm_timeout() {
    let mut options = CliOptions::default();
    options.agent_decision_source = PROVIDER_BACKED_DECISION_SOURCE.to_string();
    options.agent_provider_backend = LOCAL_BRIDGE_PROVIDER_BACKEND.to_string();
    options.agent_provider_contract = WORLDSIM_PROVIDER_CONTRACT.to_string();
    options.agent_provider_transport = LOOPBACK_HTTP_PROVIDER_TRANSPORT.to_string();
    options.agent_provider_url = "http://127.0.0.1:5841".to_string();
    options.agent_provider_auth_token = "secret-token".to_string();
    options.agent_provider_connect_timeout_ms = 3000;
    options.agent_provider_profile = "oasis7_p0_low_freq_npc".to_string();
    options.agent_execution_lane = ProviderExecutionMode::PlayerParity;
    let mut command = Command::new("echo");

    apply_viewer_live_env_overrides(&mut command, &options, false, false);

    assert_eq!(command_env_value(&command, LLM_TIMEOUT_MS_ENV), None);
    assert_eq!(
        command_env_value(&command, VIEWER_AGENT_DECISION_SOURCE_ENV),
        Some(Some(PROVIDER_BACKED_DECISION_SOURCE.to_string()))
    );
    assert_eq!(
        command_env_value(&command, VIEWER_AGENT_PROVIDER_BACKEND_ENV),
        Some(Some(LOCAL_BRIDGE_PROVIDER_BACKEND.to_string()))
    );
    assert_eq!(
        command_env_value(&command, VIEWER_AGENT_PROVIDER_CONTRACT_ENV),
        Some(Some(WORLDSIM_PROVIDER_CONTRACT.to_string()))
    );
    assert_eq!(
        command_env_value(&command, VIEWER_AGENT_PROVIDER_TRANSPORT_ENV),
        Some(Some(LOOPBACK_HTTP_PROVIDER_TRANSPORT.to_string()))
    );
    assert_eq!(
        command_env_value(&command, VIEWER_AGENT_PROVIDER_URL_ENV),
        Some(Some("http://127.0.0.1:5841".to_string()))
    );
    assert_eq!(
        command_env_value(&command, VIEWER_AGENT_PROVIDER_AUTH_TOKEN_ENV),
        Some(Some("secret-token".to_string()))
    );
    assert_eq!(
        command_env_value(&command, VIEWER_AGENT_PROVIDER_CONNECT_TIMEOUT_MS_ENV),
        Some(Some("3000".to_string()))
    );
    assert_eq!(
        command_env_value(&command, VIEWER_AGENT_PROVIDER_DECISION_TIMEOUT_MS_ENV),
        Some(Some("3000".to_string()))
    );
    assert_eq!(
        command_env_value(&command, VIEWER_AGENT_PROVIDER_PROFILE_ENV),
        Some(Some("oasis7_p0_low_freq_npc".to_string()))
    );
    assert_eq!(
        command_env_value(&command, VIEWER_AGENT_EXECUTION_LANE_ENV),
        Some(Some(
            ProviderExecutionMode::PlayerParity.as_str().to_string()
        ))
    );
    assert_eq!(
        command_env_value(&command, VIEWER_AGENT_PROVIDER_MODE_ENV),
        Some(None)
    );
}

#[test]
fn provider_backed_viewer_live_env_preserves_local_mock_backend() {
    let mut options = CliOptions::default();
    options.agent_decision_source = PROVIDER_BACKED_DECISION_SOURCE.to_string();
    options.agent_provider_backend = LOCAL_MOCK_PROVIDER_BACKEND.to_string();
    options.agent_provider_contract = WORLDSIM_PROVIDER_CONTRACT.to_string();
    options.agent_provider_transport = LOOPBACK_HTTP_PROVIDER_TRANSPORT.to_string();
    options.agent_provider_url = "http://127.0.0.1:5841".to_string();
    let mut command = Command::new("echo");

    apply_viewer_live_env_overrides(&mut command, &options, false, false);

    assert_eq!(
        command_env_value(&command, VIEWER_AGENT_DECISION_SOURCE_ENV),
        Some(Some(PROVIDER_BACKED_DECISION_SOURCE.to_string()))
    );
    assert_eq!(
        command_env_value(&command, VIEWER_AGENT_PROVIDER_BACKEND_ENV),
        Some(Some(LOCAL_MOCK_PROVIDER_BACKEND.to_string()))
    );
}

#[test]
fn build_viewer_live_command_wires_agent_chat_echo_flag_from_env() {
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var("OASIS7_RUNTIME_AGENT_CHAT_ECHO", "1");
    }
    let command = build_oasis7_viewer_live_command(
        Path::new("/bin/echo"),
        &CliOptions::default(),
        false,
        false,
    );
    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    assert!(args.iter().any(|arg| arg == "--agent-chat-echo"));
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::remove_var("OASIS7_RUNTIME_AGENT_CHAT_ECHO");
    }
}

#[test]
fn build_viewer_live_command_wires_auto_play_flag() {
    let options = CliOptions::default();
    let command = build_oasis7_viewer_live_command(Path::new("/bin/echo"), &options, false, false);
    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    assert!(args.iter().any(|arg| arg == "--auto-play"));
}

#[test]
fn build_viewer_live_command_wires_no_auto_play_flag() {
    let mut options = CliOptions::default();
    options.auto_play = false;
    let command = build_oasis7_viewer_live_command(Path::new("/bin/echo"), &options, false, false);
    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    assert!(args.iter().any(|arg| arg == "--no-auto-play"));
    assert!(!args.iter().any(|arg| arg == "--auto-play"));
}

#[test]
fn build_viewer_live_command_wires_debug_scenario_opt_in() {
    let mut options = CliOptions::default();
    options.scenario = "llm_bootstrap".to_string();
    options.allow_debug_scenario = true;
    let command = build_oasis7_viewer_live_command(Path::new("/bin/echo"), &options, false, false);
    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    assert!(args.iter().any(|arg| arg == "llm_bootstrap"));
    assert!(args.iter().any(|arg| arg == "--allow-debug-scenario"));
}

#[test]
fn build_viewer_live_command_wires_generated_world_dir() {
    let mut options = CliOptions::default();
    options.generated_world_dir = "output/public-testnet/generated-world".to_string();
    let command = build_oasis7_viewer_live_command(Path::new("/bin/echo"), &options, false, false);
    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    assert!(args.iter().any(|arg| arg == "--generated-world-dir"));
    assert!(
        args.iter()
            .any(|arg| arg == "output/public-testnet/generated-world")
    );
    assert!(!args.iter().any(|arg| arg == DEFAULT_SCENARIO));
}

#[test]
fn build_viewer_live_command_wires_llm_timeout_default_into_spawn_path() {
    let mut options = CliOptions::default();
    options.agent_decision_source = BUILTIN_LLM_DECISION_SOURCE.to_string();
    let command = build_oasis7_viewer_live_command(Path::new("/bin/echo"), &options, false, false);
    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();

    assert!(args.contains(&"--llm".to_string()));
    assert!(args.contains(&"--chain-status-bind".to_string()));
    assert!(args.contains(&options.chain_status_bind));
    assert!(args.contains(&"--chain-link-policy".to_string()));
    assert!(args.contains(&options.chain_link_policy));
    assert!(!args.iter().any(|arg| arg.is_empty()));
    assert!(!args.iter().any(|arg| arg == DEFAULT_SCENARIO));
    assert_eq!(
        command_env_value(&command, LLM_TIMEOUT_MS_ENV),
        Some(Some(DEFAULT_INTERACTIVE_LLM_TIMEOUT_MS.to_string()))
    );
}

#[test]
fn build_viewer_live_command_skips_default_llm_timeout_when_repo_config_exists() {
    let options = CliOptions::default();
    let command = build_oasis7_viewer_live_command(Path::new("/bin/echo"), &options, false, true);
    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();

    assert!(args.contains(&"--llm".to_string()));
    assert_eq!(command_env_value(&command, LLM_TIMEOUT_MS_ENV), None);
}

#[test]
fn parse_options_rejects_unknown_deployment_mode() {
    let err = parse_options(["--deployment-mode", "invalid"].into_iter())
        .expect_err("invalid deployment mode should fail");
    assert!(err.contains("hosted_public_join"));
}

#[test]
fn parse_options_rejects_trusted_local_playtest_without_explicit_allow() {
    let err = parse_options(["--deployment-mode", "trusted_local_only"].into_iter())
        .expect_err("trusted local playtest should require an explicit local flag");
    assert!(err.contains("trusted_local_only"));
    assert!(err.contains("hosted_public_join"));
}

#[test]
fn parse_options_accepts_trusted_local_playtest_with_explicit_allow() {
    let options = parse_options(
        [
            "--deployment-mode",
            "trusted_local_only",
            "--allow-trusted-local-playtest",
        ]
        .into_iter(),
    )
    .expect("explicit local playtest flag should allow trusted local mode");
    assert_eq!(options.deployment_mode, "trusted_local_only");
    assert!(options.allow_trusted_local_playtest);
    assert!(options.chain_enabled);
}

#[test]
fn parse_options_rejects_invalid_chain_role() {
    let err = parse_options(["--chain-node-role", "invalid"].into_iter()).expect_err("should fail");
    assert!(err.contains("sequencer, storage, observer"));
}

#[test]
fn parse_options_rejects_invalid_chain_p2p_user_mode() {
    let err =
        parse_options(["--chain-p2p-user-mode", "wild"].into_iter()).expect_err("should fail");
    assert!(err.contains("auto_join, private_safe, public_entry"));
}

#[test]
fn parse_options_rejects_invalid_chain_link_policy() {
    let err = parse_options(["--chain-link-policy", "observe"].into_iter())
        .expect_err("invalid chain link policy should fail");
    assert!(err.contains("enforcing|shadow"));
}

#[test]
fn parse_options_rejects_invalid_chain_replication_network_peer() {
    let err = parse_options(["--chain-replication-network-peer", "127.0.0.1:4100"].into_iter())
        .expect_err("should fail");
    assert!(err.contains("multiaddr"));
}

#[test]
fn parse_options_ignores_chain_tuning_when_hosted_public_join_disables_chain() {
    let options = parse_options(
        [
            "--chain-pos-ticks-per-slot",
            "4",
            "--chain-pos-proposal-tick-phase",
            "4",
        ]
        .into_iter(),
    )
    .expect("hosted public join disables local chain validation");
    assert_eq!(options.deployment_mode, "hosted_public_join");
    assert!(!options.chain_enabled);
    assert_eq!(options.chain_pos_ticks_per_slot, 4);
    assert_eq!(options.chain_pos_proposal_tick_phase, 4);
}

#[test]
fn parse_options_rejects_unknown_option() {
    let err = parse_options(["--unknown"].into_iter()).expect_err("should fail");
    assert!(err.contains("unknown option"));
}

#[test]
fn parse_options_rejects_unknown_agent_provider_mode() {
    let err = parse_options(["--agent-provider-mode", "wat-provider"].into_iter())
        .expect_err("should fail");
    assert!(err.contains("builtin_llm"));
    assert!(err.contains("provider_backed"));
}

#[test]
fn parse_options_rejects_invalid_provider_execution_lane() {
    let err = parse_options(
        [
            "--with-llm",
            "--agent-decision-source",
            "provider_backed",
            "--agent-execution-lane",
            "gpu_only",
        ]
        .into_iter(),
    )
    .expect_err("should fail");
    assert!(err.contains("player_parity"));
    assert!(err.contains("headless_agent"));
}

#[test]
fn parse_options_rejects_unknown_chain_storage_profile() {
    let err =
        parse_options(["--chain-storage-profile", "unknown"].into_iter()).expect_err("should fail");
    assert!(err.contains("dev_local"));
    assert!(err.contains("release_default"));
    assert!(err.contains("soak_forensics"));
}

#[test]
fn parse_options_rejects_missing_value() {
    let err = parse_options(["--viewer-port"].into_iter()).expect_err("should fail");
    assert!(err.contains("requires a value"));
}

#[test]
fn query_runtime_bound_players_reads_snapshot_bindings() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind probe mock");
    let addr = listener.local_addr().expect("local addr");
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept");
        let reader_stream = stream.try_clone().expect("clone stream");
        let mut reader = BufReader::new(reader_stream);
        let mut writer = BufWriter::new(stream);

        let mut raw_hello = String::new();
        reader.read_line(&mut raw_hello).expect("read hello");
        let hello_request: ViewerRequest =
            serde_json::from_str(raw_hello.trim_end()).expect("decode hello request");
        assert!(matches!(
            hello_request,
            ViewerRequest::Hello {
                version: VIEWER_PROTOCOL_VERSION,
                ..
            }
        ));
        serde_json::to_writer(
            &mut writer,
            &ViewerResponse::HelloAck {
                server: "oasis7".to_string(),
                version: VIEWER_PROTOCOL_VERSION,
                world_id: "test-world".to_string(),
                control_profile: oasis7::viewer::ViewerControlProfile::Playback,
            },
        )
        .expect("write hello ack");
        writer.write_all(b"\n").expect("write newline");
        writer.flush().expect("flush hello ack");

        let mut snapshot_line = String::new();
        reader
            .read_line(&mut snapshot_line)
            .expect("read snapshot request");
        let snapshot_request: ViewerRequest =
            serde_json::from_str(snapshot_line.trim_end()).expect("decode snapshot request");
        assert!(matches!(snapshot_request, ViewerRequest::RequestSnapshot));

        let mut model = WorldModel::default();
        model
            .agent_player_bindings
            .insert("agent-1".to_string(), "player-a".to_string());
        model
            .agent_player_bindings
            .insert("agent-2".to_string(), "player-b".to_string());
        let snapshot = WorldSnapshot {
            version: 1,
            chunk_generation_schema_version: 1,
            time: 0,
            config: WorldConfig::default(),
            model,
            runtime_snapshot: None,
            player_gameplay: None,
            chain_resource_manifest: Default::default(),
            latest_chain_resource_delta: Default::default(),
            chunk_runtime: Default::default(),
            intel_ttl_ticks: 0,
            next_event_id: 0,
            next_action_id: 0,
            pending_actions: Vec::new(),
            journal_len: 0,
        };
        serde_json::to_writer(&mut writer, &ViewerResponse::Snapshot { snapshot })
            .expect("write snapshot");
        writer.write_all(b"\n").expect("write newline");
        writer.flush().expect("flush snapshot");
    });

    let players = query_runtime_bound_players(format!("{addr}").as_str()).expect("query players");
    assert!(players.contains("player-a"));
    assert!(players.contains("player-b"));
    assert_eq!(players.len(), 2);
    handle.join().expect("join mock server");
}

#[test]
fn parse_options_rejects_invalid_port() {
    let err = parse_options(["--viewer-port", "70000"].into_iter()).expect_err("should fail");
    assert!(err.contains("integer"));
}

#[test]
fn parse_options_rejects_invalid_bind_format() {
    let err = parse_options(["--live-bind", "127.0.0.1"].into_iter()).expect_err("should fail");
    assert!(err.contains("<host:port>"));
}

#[test]
fn parse_host_port_parses_valid_value() {
    let (host, port) = parse_host_port("127.0.0.1:5011", "--web-bind").expect("ok");
    assert_eq!(host, "127.0.0.1");
    assert_eq!(port, 5011);
}

#[test]
fn parse_host_port_accepts_bracketed_ipv6() {
    let (host, port) = parse_host_port("[::1]:5011", "--web-bind").expect("ok");
    assert_eq!(host, "::1");
    assert_eq!(port, 5011);
}

#[test]
fn parse_host_port_rejects_unbracketed_ipv6() {
    let err = parse_host_port("::1:5011", "--web-bind").expect_err("should fail");
    assert!(err.contains("wrapped in []"));
}

#[test]
fn parse_host_port_rejects_zero_port() {
    let err = parse_host_port("127.0.0.1:0", "--web-bind").expect_err("should fail");
    assert!(err.contains("1..=65535"));
}

#[test]
fn build_game_url_rewrites_zero_bind_host_to_loopback() {
    let options = CliOptions {
        viewer_host: "0.0.0.0".to_string(),
        deployment_mode: "hosted_public_join".to_string(),
        viewer_port: 4173,
        web_bind: "0.0.0.0:5011".to_string(),
        ..CliOptions::default()
    };
    let url = build_game_url(&options);
    assert!(url.starts_with(
        "http://127.0.0.1:4173/?render_mode=viewer&ws=ws%3A%2F%2F127.0.0.1%3A5011&hosted_access="
    ));
    assert!(url.contains("%22deployment_mode%22%3A%22hosted_public_join%22"));
    assert!(url.contains("%22local_chain_runtime%22%3A%22blocked_for_public_player_plane%22"));
    assert!(url.contains("%22node_admission%22%3A%22operator_managed_node_onboarding_only%22"));
}

#[test]
fn build_game_url_brackets_ipv6_hosts() {
    let options = CliOptions {
        viewer_host: "::1".to_string(),
        viewer_port: 4173,
        web_bind: "[::1]:5011".to_string(),
        ..CliOptions::default()
    };
    let url = build_game_url(&options);
    assert!(url.starts_with(
        "http://[::1]:4173/?render_mode=viewer&ws=ws%3A%2F%2F%5B%3A%3A1%5D%3A5011&hosted_access="
    ));
    assert!(url.contains("%22deployment_mode%22%3A%22hosted_public_join%22"));
}

#[test]
fn build_oasis7_chain_runtime_args_includes_storage_profile() {
    let options = CliOptions {
        scenario: "sandbox".to_string(),
        chain_node_id: "chain-a".to_string(),
        chain_status_bind: "127.0.0.1:6121".to_string(),
        chain_storage_profile: StorageProfile::ReleaseDefault,
        chain_p2p_user_mode: "public_entry".to_string(),
        chain_p2p_accept_public_entry: true,
        chain_replication_bootstrap_peers: vec![
            "/ip4/127.0.0.1/tcp/4100".to_string(),
            "/dns4/bootstrap.example/tcp/4101".to_string(),
        ],
        ..CliOptions::default()
    };
    let args = build_oasis7_chain_runtime_args(&options);
    assert!(args.contains(&"--storage-profile".to_string()));
    assert!(args.contains(&"release_default".to_string()));
    assert!(args.contains(&"--world-id".to_string()));
    assert!(args.contains(&"live-sandbox".to_string()));
    assert!(args.contains(&"--execution-world-dir".to_string()));
    assert!(args.contains(&"--p2p-user-mode".to_string()));
    assert!(args.contains(&"public_entry".to_string()));
    assert!(args.contains(&"--p2p-accept-public-entry".to_string()));
    assert_eq!(
        args.iter()
            .filter(|value| value.as_str() == "--replication-network-peer")
            .count(),
        2
    );
    assert!(args.contains(&"/ip4/127.0.0.1/tcp/4100".to_string()));
    assert!(args.contains(&"/dns4/bootstrap.example/tcp/4101".to_string()));
    assert!(
        args.contains(&"output/chain-runtime/chain-a/reward-runtime-execution-world".to_string())
    );
}

#[test]
fn build_oasis7_chain_runtime_args_derives_world_id_from_explicit_scenario() {
    let options = CliOptions {
        scenario: "sandbox".to_string(),
        chain_node_id: "chain-a".to_string(),
        chain_status_bind: "127.0.0.1:6121".to_string(),
        chain_world_id: None,
        ..CliOptions::default()
    };
    let args = build_oasis7_chain_runtime_args(&options);
    assert!(args.contains(&"--world-id".to_string()));
    assert!(args.contains(&"live-sandbox".to_string()));
}

#[test]
fn parse_options_local_standalone_test_builds_self_validating_private_chain() {
    let options = parse_options(
        [
            "--chain-node-id",
            "local-node-a",
            "--deployment-mode",
            "trusted_local_only",
            "--allow-trusted-local-playtest",
            "--chain-local-standalone-test",
            "--no-open-browser",
        ]
        .into_iter(),
    )
    .expect("local standalone profile should parse");

    assert!(options.chain_local_standalone_test);
    assert_eq!(options.chain_p2p_user_mode, "private_safe");
    assert!(!options.chain_p2p_accept_public_entry);
    assert!(options.chain_replication_bootstrap_peers.is_empty());
    assert!(options.chain_node_auto_attest_all_validators);
    assert_eq!(options.chain_pos_slot_duration_ms, 1_000);
    assert_eq!(options.chain_pos_ticks_per_slot, 1);
    assert_eq!(options.chain_pos_proposal_tick_phase, 0);
    assert_eq!(
        options.chain_node_validators,
        vec!["local-node-a:100".to_string()]
    );

    let args = build_oasis7_chain_runtime_args(&options);
    assert!(args.contains(&"--node-auto-attest-all".to_string()));
    assert!(args.contains(&"--node-validator".to_string()));
    assert!(args.contains(&"local-node-a:100".to_string()));
    assert!(args.contains(&"--config".to_string()));
    assert!(args.contains(&"output/chain-runtime/local-node-a/config.toml".to_string()));
    assert!(!args.contains(&"--replication-network-peer".to_string()));
}

#[test]
fn build_oasis7_chain_runtime_args_supports_all_storage_profiles() {
    for (profile, expected) in [
        (StorageProfile::DevLocal, "dev_local"),
        (StorageProfile::ReleaseDefault, "release_default"),
        (StorageProfile::SoakForensics, "soak_forensics"),
    ] {
        let options = CliOptions {
            scenario: "sandbox".to_string(),
            chain_node_id: format!("chain-{expected}"),
            chain_status_bind: "127.0.0.1:6121".to_string(),
            chain_storage_profile: profile,
            ..CliOptions::default()
        };
        let args = build_oasis7_chain_runtime_args(&options);
        assert!(args.contains(&"--storage-profile".to_string()));
        assert!(args.contains(&expected.to_string()));
    }
}

#[test]
fn build_oasis7_chain_runtime_args_prefers_network_tier_manifest_when_present() {
    let options = CliOptions {
        chain_node_id: "chain-a".to_string(),
        chain_status_bind: "127.0.0.1:6121".to_string(),
        chain_network_tier_manifest: "/tmp/public-testnet.json".to_string(),
        chain_storage_profile: StorageProfile::ReleaseDefault,
        ..CliOptions::default()
    };
    let args = build_oasis7_chain_runtime_args(&options);
    assert!(args.contains(&"--network-tier-manifest".to_string()));
    assert!(args.contains(&"/tmp/public-testnet.json".to_string()));
    assert!(!args.contains(&"--storage-profile".to_string()));
    assert!(args.contains(&"--node-role".to_string()));
    assert!(args.contains(&"sequencer".to_string()));
}

#[test]
fn parse_options_still_validates_chain_node_role_when_manifest_is_present() {
    let err = parse_options(
        [
            "--chain-network-tier-manifest",
            "/tmp/public-testnet.json",
            "--chain-node-role",
            "invalid",
        ]
        .into_iter(),
    )
    .expect_err("should fail");
    assert!(err.contains("--chain-node-role"));
}

#[test]
fn missing_execution_world_persistence_files_reports_snapshot_and_journal() {
    let temp_dir = make_temp_dir("execution_world_missing");
    let missing = missing_execution_world_persistence_files(temp_dir.as_path());
    assert_eq!(missing.len(), 2);
    assert!(missing.iter().any(|path| path.ends_with("snapshot.json")));
    assert!(missing.iter().any(|path| path.ends_with("journal.json")));
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn missing_execution_world_persistence_files_ignores_ready_world_dir() {
    let temp_dir = make_temp_dir("execution_world_ready");
    fs::write(temp_dir.join("snapshot.json"), "{}").expect("write snapshot");
    fs::write(temp_dir.join("journal.json"), "{}").expect("write journal");
    let missing = missing_execution_world_persistence_files(temp_dir.as_path());
    assert!(missing.is_empty());
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn sanitize_relative_request_path_rejects_traversal() {
    let err = sanitize_relative_request_path("/../etc/passwd").expect_err("should fail");
    assert!(err.contains("traversal"));
}

#[test]
fn resolve_static_asset_path_supports_spa_fallback() {
    let temp_dir = make_temp_dir("spa_fallback");
    fs::write(temp_dir.join("index.html"), "<html>ok</html>").expect("write index");
    let resolved = resolve_static_asset_path(temp_dir.as_path(), "/app/route?x=1")
        .expect("resolve should succeed")
        .expect("should fallback to index");
    assert_eq!(resolved, temp_dir.join("index.html"));
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn resolve_static_asset_path_returns_none_for_missing_static_asset() {
    let temp_dir = make_temp_dir("missing_asset");
    fs::write(temp_dir.join("index.html"), "<html>ok</html>").expect("write index");
    let resolved = resolve_static_asset_path(temp_dir.as_path(), "/assets/missing.js")
        .expect("resolve should succeed");
    assert!(resolved.is_none());
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn content_type_for_path_covers_wasm_and_js() {
    assert_eq!(
        content_type_for_path(Path::new("a.wasm")),
        "application/wasm"
    );
    assert_eq!(
        content_type_for_path(Path::new("a.js")),
        "text/javascript; charset=utf-8"
    );
}

#[test]
fn sanitize_index_html_for_embedded_server_removes_trunk_reload_script() {
    let html = concat!(
        "<html><body>",
        "<script>window.bootstrap = true;</script>",
        "<script>const url = 'ws://{{__TRUNK_ADDRESS__}}{{__TRUNK_WS_BASE__}}.well-known/trunk/ws';</script>",
        "</body></html>"
    );
    let sanitized =
        sanitize_index_html_for_embedded_server(Path::new("index.html"), html.as_bytes(), None);
    let sanitized = String::from_utf8(sanitized).expect("utf-8");
    assert!(sanitized.contains("window.bootstrap = true"));
    assert!(!sanitized.contains(".well-known/trunk/ws"));
    assert!(!sanitized.contains("__TRUNK_ADDRESS__"));
}

#[test]
fn sanitize_index_html_for_embedded_server_keeps_non_index_files_unchanged() {
    let body = b"<script>.well-known/trunk/ws</script>";
    let sanitized = sanitize_index_html_for_embedded_server(Path::new("app.js"), body, None);
    assert_eq!(sanitized, body);
}
