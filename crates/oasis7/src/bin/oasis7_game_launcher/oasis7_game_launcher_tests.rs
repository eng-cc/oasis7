use std::env;
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{
    apply_viewer_live_env_overrides, build_game_url, build_oasis7_chain_runtime_args,
    build_oasis7_viewer_live_command, build_viewer_auth_bootstrap_script, content_type_for_path,
    parse_host_port, parse_options, query_runtime_bound_players, resolve_static_asset_path,
    resolve_viewer_auth_bootstrap_for_embedded_server, resolve_viewer_auth_bootstrap_from_path,
    resolve_viewer_static_dir_with_override, sanitize_index_html_for_embedded_server,
    sanitize_relative_request_path, viewer_dev_dist_candidates, CliOptions, ViewerAuthBootstrap,
    BUILTIN_LLM_PROVIDER_MODE, DEFAULT_CHAIN_NODE_ID, DEFAULT_CHAIN_STATUS_BIND,
    DEFAULT_DEPLOYMENT_MODE, DEFAULT_INTERACTIVE_LLM_TIMEOUT_MS, DEFAULT_LIVE_BIND,
    DEFAULT_OPENCLAW_AGENT_PROFILE, DEFAULT_OPENCLAW_CONNECT_TIMEOUT_MS, DEFAULT_SCENARIO,
    DEFAULT_VIEWER_STATIC_DIR, GAME_STATIC_DIR_ENV, LLM_TIMEOUT_MS_ENV,
    OPENCLAW_LOCAL_HTTP_PROVIDER_MODE, VIEWER_AGENT_PROVIDER_MODE_ENV,
    VIEWER_AUTH_BOOTSTRAP_OBJECT, VIEWER_AUTH_PRIVATE_KEY_ENV, VIEWER_AUTH_PUBLIC_KEY_ENV,
    VIEWER_OPENCLAW_AGENT_PROFILE_ENV, VIEWER_OPENCLAW_AUTH_TOKEN_ENV,
    VIEWER_OPENCLAW_BASE_URL_ENV, VIEWER_OPENCLAW_CONNECT_TIMEOUT_MS_ENV,
    VIEWER_OPENCLAW_EXECUTION_MODE_ENV, VIEWER_PLAYER_ID_ENV,
};
use oasis7::simulator::ProviderExecutionMode;
use oasis7::simulator::{WorldConfig, WorldModel, WorldSnapshot};
use oasis7::viewer::{ViewerRequest, ViewerResponse, VIEWER_PROTOCOL_VERSION};
use oasis7_proto::storage_profile::StorageProfile;

fn assert_removed_old_brand_viewer_auth_env_absent(text: &str) {
    assert!(!text.contains(removed_old_brand_viewer_auth_bootstrap_object().as_str()));
    for key in removed_old_brand_viewer_auth_env_keys() {
        assert!(!text.contains(key.as_str()));
    }
}

fn removed_old_brand_viewer_auth_bootstrap_object() -> String {
    format!(
        "__{}",
        ["AGENT", "WORLD", "VIEWER", "AUTH", "ENV"].join("_")
    )
}

fn removed_old_brand_viewer_auth_env_keys() -> [String; 3] {
    [
        ["AGENT", "WORLD", "VIEWER", "PLAYER", "ID"].join("_"),
        ["AGENT", "WORLD", "VIEWER", "AUTH", "PUBLIC", "KEY"].join("_"),
        ["AGENT", "WORLD", "VIEWER", "AUTH", "PRIVATE", "KEY"].join("_"),
    ]
}

fn command_env_value(command: &Command, key: &str) -> Option<Option<String>> {
    command
        .get_envs()
        .find(|(env_key, _)| env_key.to_string_lossy() == key)
        .map(|(_, value)| value.map(|value| value.to_string_lossy().into_owned()))
}

#[test]
fn parse_options_defaults() {
    let options = parse_options(std::iter::empty()).expect("parse should succeed");
    assert_eq!(options.scenario, DEFAULT_SCENARIO);
    assert_eq!(options.live_bind, DEFAULT_LIVE_BIND);
    assert_eq!(options.deployment_mode, DEFAULT_DEPLOYMENT_MODE);
    assert!(options.with_llm);
    assert_eq!(options.agent_provider_mode, BUILTIN_LLM_PROVIDER_MODE);
    assert_eq!(
        options.openclaw_agent_profile,
        DEFAULT_OPENCLAW_AGENT_PROFILE
    );
    assert_eq!(
        options.openclaw_execution_mode,
        ProviderExecutionMode::HeadlessAgent
    );
    assert_eq!(
        options.openclaw_connect_timeout_ms,
        DEFAULT_OPENCLAW_CONNECT_TIMEOUT_MS
    );
    assert!(options.open_browser);
    assert_eq!(options.viewer_static_dir, "web");
    assert!(options.chain_enabled);
    assert_eq!(options.chain_status_bind, DEFAULT_CHAIN_STATUS_BIND);
    assert!(options
        .chain_node_id
        .starts_with(&format!("{DEFAULT_CHAIN_NODE_ID}-fresh-")));
    assert_eq!(options.chain_storage_profile, StorageProfile::DevLocal);
    assert_eq!(options.chain_node_role, "sequencer");
    assert_eq!(options.chain_p2p_user_mode, "auto_join");
    assert!(!options.chain_p2p_accept_public_entry);
    assert_eq!(options.chain_pos_slot_duration_ms, 12_000);
    assert_eq!(options.chain_pos_ticks_per_slot, 10);
    assert_eq!(options.chain_pos_proposal_tick_phase, 9);
    assert!(!options.chain_pos_adaptive_tick_scheduler_enabled);
    assert_eq!(options.chain_pos_slot_clock_genesis_unix_ms, None);
    assert_eq!(options.chain_pos_max_past_slot_lag, 256);
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
            "--chain-node-tick-ms",
            "350",
            "--chain-pos-slot-duration-ms",
            "12000",
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
            "--with-llm",
            "--agent-provider-mode",
            "openclaw_local_http",
            "--openclaw-base-url",
            "http://127.0.0.1:5841",
            "--openclaw-auth-token",
            "secret-token",
            "--openclaw-connect-timeout-ms",
            "3000",
            "--openclaw-agent-profile",
            "oasis7_p0_low_freq_npc",
            "--openclaw-execution-mode",
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
    assert_eq!(options.chain_status_bind, "127.0.0.1:6331");
    assert_eq!(options.chain_node_id, "chain-a");
    assert_eq!(options.chain_storage_profile, StorageProfile::SoakForensics);
    assert_eq!(options.chain_world_id, Some("live-chain-a".to_string()));
    assert_eq!(options.chain_node_role, "storage");
    assert_eq!(options.chain_p2p_user_mode, "public_entry");
    assert!(options.chain_p2p_accept_public_entry);
    assert_eq!(options.chain_node_tick_ms, 350);
    assert_eq!(options.chain_pos_slot_duration_ms, 12_000);
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
    assert!(options.with_llm);
    assert_eq!(
        options.agent_provider_mode,
        OPENCLAW_LOCAL_HTTP_PROVIDER_MODE
    );
    assert_eq!(options.openclaw_base_url, "http://127.0.0.1:5841");
    assert_eq!(options.openclaw_auth_token, "secret-token");
    assert_eq!(options.openclaw_connect_timeout_ms, 3000);
    assert_eq!(options.openclaw_agent_profile, "oasis7_p0_low_freq_npc");
    assert_eq!(
        options.openclaw_execution_mode,
        ProviderExecutionMode::PlayerParity
    );
    assert!(!options.open_browser);
}

#[test]
fn parse_options_accepts_chain_disable() {
    let options = parse_options(["--chain-disable"].into_iter()).expect("parse should succeed");
    assert!(!options.chain_enabled);
}

#[test]
fn parse_options_accepts_agent_direct_connect_alias() {
    let options = parse_options(
        [
            "--with-llm",
            "--agent-provider-mode",
            "agent_direct_connect",
            "--openclaw-base-url",
            "http://127.0.0.1:5841",
            "--openclaw-agent-profile",
            "oasis7_p0_low_freq_npc",
        ]
        .into_iter(),
    )
    .expect("parse should succeed");

    assert_eq!(
        options.agent_provider_mode,
        OPENCLAW_LOCAL_HTTP_PROVIDER_MODE
    );
}

#[test]
fn builtin_viewer_live_env_applies_default_llm_timeout_when_parent_is_unset() {
    let options = CliOptions::default();
    let mut command = Command::new("echo");

    apply_viewer_live_env_overrides(&mut command, &options, false);

    assert_eq!(
        command_env_value(&command, LLM_TIMEOUT_MS_ENV),
        Some(Some(DEFAULT_INTERACTIVE_LLM_TIMEOUT_MS.to_string()))
    );
    assert_eq!(
        command_env_value(&command, VIEWER_AGENT_PROVIDER_MODE_ENV),
        Some(None)
    );
}

#[test]
fn builtin_viewer_live_env_preserves_explicit_parent_llm_timeout() {
    let options = CliOptions::default();
    let mut command = Command::new("echo");

    apply_viewer_live_env_overrides(&mut command, &options, true);

    assert_eq!(command_env_value(&command, LLM_TIMEOUT_MS_ENV), None);
}

#[test]
fn openclaw_viewer_live_env_sets_provider_specific_overrides_without_builtin_llm_timeout() {
    let mut options = CliOptions::default();
    options.agent_provider_mode = OPENCLAW_LOCAL_HTTP_PROVIDER_MODE.to_string();
    options.openclaw_base_url = "http://127.0.0.1:5841".to_string();
    options.openclaw_auth_token = "secret-token".to_string();
    options.openclaw_connect_timeout_ms = 3000;
    options.openclaw_agent_profile = "oasis7_p0_low_freq_npc".to_string();
    options.openclaw_execution_mode = ProviderExecutionMode::PlayerParity;
    let mut command = Command::new("echo");

    apply_viewer_live_env_overrides(&mut command, &options, false);

    assert_eq!(command_env_value(&command, LLM_TIMEOUT_MS_ENV), None);
    assert_eq!(
        command_env_value(&command, VIEWER_AGENT_PROVIDER_MODE_ENV),
        Some(Some(OPENCLAW_LOCAL_HTTP_PROVIDER_MODE.to_string()))
    );
    assert_eq!(
        command_env_value(&command, VIEWER_OPENCLAW_BASE_URL_ENV),
        Some(Some("http://127.0.0.1:5841".to_string()))
    );
    assert_eq!(
        command_env_value(&command, VIEWER_OPENCLAW_AUTH_TOKEN_ENV),
        Some(Some("secret-token".to_string()))
    );
    assert_eq!(
        command_env_value(&command, VIEWER_OPENCLAW_CONNECT_TIMEOUT_MS_ENV),
        Some(Some("3000".to_string()))
    );
    assert_eq!(
        command_env_value(&command, VIEWER_OPENCLAW_AGENT_PROFILE_ENV),
        Some(Some("oasis7_p0_low_freq_npc".to_string()))
    );
    assert_eq!(
        command_env_value(&command, VIEWER_OPENCLAW_EXECUTION_MODE_ENV),
        Some(Some(
            ProviderExecutionMode::PlayerParity.as_str().to_string()
        ))
    );
}

#[test]
fn build_viewer_live_command_wires_llm_timeout_default_into_spawn_path() {
    let options = CliOptions::default();
    let command = build_oasis7_viewer_live_command(Path::new("/bin/echo"), &options, false);
    let args: Vec<String> = command
        .get_args()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();

    assert!(args.contains(&"--llm".to_string()));
    assert_eq!(
        command_env_value(&command, LLM_TIMEOUT_MS_ENV),
        Some(Some(DEFAULT_INTERACTIVE_LLM_TIMEOUT_MS.to_string()))
    );
}

#[test]
fn parse_options_rejects_unknown_deployment_mode() {
    let err = parse_options(["--deployment-mode", "invalid"].into_iter())
        .expect_err("invalid deployment mode should fail");
    assert!(err.contains("trusted_local_only"));
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
fn parse_options_rejects_proposal_tick_phase_out_of_range() {
    let err = parse_options(
        [
            "--chain-pos-ticks-per-slot",
            "4",
            "--chain-pos-proposal-tick-phase",
            "4",
        ]
        .into_iter(),
    )
    .expect_err("should fail");
    assert!(err.contains("--chain-pos-proposal-tick-phase"));
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
    assert!(err.contains("agent_direct_connect"));
    assert!(err.contains("openclaw_local_http"));
}

#[test]
fn parse_options_rejects_invalid_openclaw_execution_mode() {
    let err = parse_options(
        [
            "--with-llm",
            "--agent-provider-mode",
            "openclaw_local_http",
            "--openclaw-execution-mode",
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
            chunk_runtime: Default::default(),
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
    assert!(url.starts_with("http://127.0.0.1:4173/?ws=ws%3A%2F%2F127.0.0.1%3A5011&hosted_access="));
    assert!(url.contains("%22deployment_mode%22%3A%22hosted_public_join%22"));
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
    assert!(url.starts_with("http://[::1]:4173/?ws=ws%3A%2F%2F%5B%3A%3A1%5D%3A5011&hosted_access="));
    assert!(url.contains("%22deployment_mode%22%3A%22trusted_local_only%22"));
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
    assert!(
        args.contains(&"output/chain-runtime/chain-a/reward-runtime-execution-world".to_string())
    );
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

#[test]
fn sanitize_index_html_for_embedded_server_injects_viewer_auth_bootstrap() {
    let html = "<html><head></head><body><div id=\"app\"></div></body></html>";
    let auth = ViewerAuthBootstrap {
        player_id: "viewer-player".to_string(),
        public_key: "pub-hex".to_string(),
        private_key: "priv-hex".to_string(),
    };
    let sanitized = sanitize_index_html_for_embedded_server(
        Path::new("index.html"),
        html.as_bytes(),
        Some(&auth),
    );
    let sanitized = String::from_utf8(sanitized).expect("utf-8");
    assert!(sanitized.contains(VIEWER_AUTH_BOOTSTRAP_OBJECT));
    assert!(sanitized.contains(VIEWER_PLAYER_ID_ENV));
    assert!(sanitized.contains(VIEWER_AUTH_PUBLIC_KEY_ENV));
    assert!(sanitized.contains(VIEWER_AUTH_PRIVATE_KEY_ENV));
    assert_removed_old_brand_viewer_auth_env_absent(&sanitized);
    assert!(sanitized.contains("viewer-player"));
    assert!(sanitized.contains("pub-hex"));
    assert!(sanitized.contains("priv-hex"));
}

#[test]
fn sanitize_index_html_for_embedded_server_injects_viewer_auth_bootstrap_into_non_index_html() {
    let html = "<html><head></head><body><div id=\"safe\"></div></body></html>";
    let auth = ViewerAuthBootstrap {
        player_id: "viewer-player".to_string(),
        public_key: "pub-hex".to_string(),
        private_key: "priv-hex".to_string(),
    };
    let sanitized = sanitize_index_html_for_embedded_server(
        Path::new("software_safe.html"),
        html.as_bytes(),
        Some(&auth),
    );
    let sanitized = String::from_utf8(sanitized).expect("utf-8");
    assert!(sanitized.contains(VIEWER_AUTH_BOOTSTRAP_OBJECT));
    assert_removed_old_brand_viewer_auth_env_absent(&sanitized);
    assert!(sanitized.contains("viewer-player"));
    assert!(sanitized.contains("pub-hex"));
    assert!(sanitized.contains("priv-hex"));
}

#[test]
fn build_viewer_auth_bootstrap_script_contains_expected_window_object() {
    let auth = ViewerAuthBootstrap {
        player_id: "viewer-player".to_string(),
        public_key: "public".to_string(),
        private_key: "private".to_string(),
    };
    let script = build_viewer_auth_bootstrap_script(&auth);
    assert!(script.contains("window."));
    assert!(script.contains(VIEWER_AUTH_BOOTSTRAP_OBJECT));
    assert!(script.contains(VIEWER_PLAYER_ID_ENV));
    assert!(script.contains(VIEWER_AUTH_PUBLIC_KEY_ENV));
    assert!(script.contains(VIEWER_AUTH_PRIVATE_KEY_ENV));
    assert_removed_old_brand_viewer_auth_env_absent(&script);
}

#[test]
fn resolve_viewer_auth_bootstrap_from_path_reads_node_keypair() {
    let temp_dir = make_temp_dir("viewer_auth_bootstrap");
    let config_path = temp_dir.join("config.toml");
    fs::write(
        &config_path,
        "[node]\nprivate_key = \"private-key-hex\"\npublic_key = \"public-key-hex\"\n",
    )
    .expect("write config");

    let auth =
        resolve_viewer_auth_bootstrap_from_path(config_path.as_path()).expect("resolve auth");
    assert_eq!(auth.public_key, "public-key-hex");
    assert_eq!(auth.private_key, "private-key-hex");
    assert!(!auth.player_id.trim().is_empty());
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn hosted_public_join_disables_viewer_auth_bootstrap_resolution() {
    let temp_dir = make_temp_dir("hosted_public_join_no_bootstrap");
    let config_path = temp_dir.join("config.toml");
    fs::write(
        &config_path,
        "[node]\nprivate_key = \"private-key-hex\"\npublic_key = \"public-key-hex\"\n",
    )
    .expect("write config");

    let old_cwd = env::current_dir().expect("cwd");
    env::set_current_dir(&temp_dir).expect("chdir");
    let auth =
        resolve_viewer_auth_bootstrap_for_embedded_server(super::DeploymentMode::HostedPublicJoin);
    env::set_current_dir(old_cwd).expect("restore cwd");

    assert!(auth.is_none());
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn resolve_viewer_static_dir_with_override_prefers_env_for_default_static_dir() {
    let override_dir = make_temp_dir("viewer_static_override");
    let override_raw = override_dir.to_string_lossy().to_string();

    let resolved = resolve_viewer_static_dir_with_override(
        DEFAULT_VIEWER_STATIC_DIR,
        Some((override_raw.as_str(), GAME_STATIC_DIR_ENV)),
    )
    .expect("resolve should succeed");

    assert_eq!(resolved, override_dir);
    let _ = fs::remove_dir_all(override_dir);
}

#[test]
fn resolve_viewer_static_dir_with_override_keeps_explicit_path_priority() {
    let explicit_dir = make_temp_dir("viewer_static_explicit");
    let override_dir = make_temp_dir("viewer_static_override_ignored");
    let explicit_raw = explicit_dir.to_string_lossy().to_string();
    let override_raw = override_dir.to_string_lossy().to_string();

    let resolved = resolve_viewer_static_dir_with_override(
        explicit_raw.as_str(),
        Some((override_raw.as_str(), GAME_STATIC_DIR_ENV)),
    )
    .expect("resolve should succeed");

    assert_eq!(resolved, explicit_dir);
    let _ = fs::remove_dir_all(explicit_dir);
    let _ = fs::remove_dir_all(override_dir);
}

#[test]
fn resolve_viewer_static_dir_with_override_rejects_missing_env_dir() {
    let missing_path = make_missing_temp_path("viewer_static_missing_env");
    let missing_raw = missing_path.to_string_lossy().to_string();

    let err = resolve_viewer_static_dir_with_override(
        DEFAULT_VIEWER_STATIC_DIR,
        Some((missing_raw.as_str(), GAME_STATIC_DIR_ENV)),
    )
    .expect_err("missing override path should fail");

    assert!(err.contains(GAME_STATIC_DIR_ENV));
    assert!(err.contains("not found"));
}

#[test]
fn viewer_dev_dist_candidates_only_return_oasis7_path() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let candidates = viewer_dev_dist_candidates();

    assert_eq!(
        candidates,
        vec![repo_root.join("oasis7_viewer").join("dist")]
    );
}

fn make_temp_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!(
        "oasis7_launcher_test_{label}_{}_{}",
        std::process::id(),
        stamp
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}

fn make_missing_temp_path(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!(
        "oasis7_launcher_missing_{label}_{}_{}",
        std::process::id(),
        stamp
    ));
    path
}
