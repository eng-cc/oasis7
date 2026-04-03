use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::runtime_paths::viewer_dev_dist_candidates;
use super::{
    build_chain_runtime_args, build_game_url, build_launcher_args,
    build_launcher_args_with_launcher_bin, chain_error_code_for_state, execute_gui_agent_action,
    finalize_chain_start_outcome, gui_agent_capabilities_response, parse_chain_validators,
    parse_host_port, parse_options, parse_port, remap_transfer_runtime_target, snapshot_from_state,
    stop_chain_process, stop_process, validate_chain_config, validate_game_config,
    validate_game_config_with_launcher_bin, ChainRecoverySnapshot, ChainRuntimeStatus, CliOptions,
    LauncherConfig, ProcessState, ServiceState, DEFAULT_CHAIN_NODE_ID, DEFAULT_CHAIN_STATUS_BIND,
    DEFAULT_LISTEN_BIND, DEFAULT_SCENARIO,
};
use oasis7_proto::storage_profile::StorageProfile;

#[test]
fn parse_options_defaults() {
    let options = parse_options(std::iter::empty()).expect("parse options");
    assert_eq!(options.listen_bind, DEFAULT_LISTEN_BIND);
    assert_eq!(options.initial_config.deployment_mode, "trusted_local_only");
    assert_eq!(options.initial_config.scenario, DEFAULT_SCENARIO);
    assert_eq!(
        options.initial_config.chain_status_bind,
        DEFAULT_CHAIN_STATUS_BIND
    );
    assert_eq!(
        options.initial_config.chain_storage_profile,
        StorageProfile::DevLocal.as_str()
    );
    assert_eq!(options.initial_config.chain_pos_slot_duration_ms, "12000");
    assert_eq!(options.initial_config.chain_pos_ticks_per_slot, "10");
    assert_eq!(options.initial_config.chain_pos_proposal_tick_phase, "9");
    assert!(
        !options
            .initial_config
            .chain_pos_adaptive_tick_scheduler_enabled
    );
    assert_eq!(
        options.initial_config.chain_pos_slot_clock_genesis_unix_ms,
        ""
    );
    assert_eq!(options.initial_config.chain_pos_max_past_slot_lag, "256");
    assert!(options.initial_config.llm_enabled);
    assert!(options.initial_config.chain_enabled);
    assert!(!options.initial_config.auto_open_browser);
    assert!(options
        .initial_config
        .chain_node_id
        .starts_with(&format!("{DEFAULT_CHAIN_NODE_ID}-fresh-")));
}

#[test]
fn parse_options_accepts_overrides() {
    let options = parse_options(
        [
            "--listen-bind",
            "127.0.0.1:7510",
            "--deployment-mode",
            "hosted_public_join",
            "--launcher-bin",
            "/tmp/oasis7_game_launcher",
            "--chain-runtime-bin",
            "/tmp/oasis7_chain_runtime",
            "--console-static-dir",
            "/tmp/web-launcher-dist",
            "--scenario",
            "sandbox",
            "--live-bind",
            "127.0.0.1:6200",
            "--web-bind",
            "127.0.0.1:6201",
            "--viewer-host",
            "127.0.0.1",
            "--viewer-port",
            "4777",
            "--viewer-static-dir",
            "./web",
            "--with-llm",
            "--chain-disable",
            "--open-browser",
            "--chain-storage-profile",
            "release_default",
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
        ]
        .into_iter(),
    )
    .expect("parse overrides");

    assert_eq!(options.listen_bind, "127.0.0.1:7510");
    assert_eq!(options.initial_config.deployment_mode, "hosted_public_join");
    assert_eq!(options.launcher_bin, "/tmp/oasis7_game_launcher");
    assert_eq!(options.chain_runtime_bin, "/tmp/oasis7_chain_runtime");
    assert_eq!(
        options.console_static_dir,
        PathBuf::from("/tmp/web-launcher-dist")
    );
    assert_eq!(options.initial_config.scenario, "sandbox");
    assert_eq!(options.initial_config.live_bind, "127.0.0.1:6200");
    assert_eq!(options.initial_config.web_bind, "127.0.0.1:6201");
    assert_eq!(options.initial_config.viewer_host, "127.0.0.1");
    assert_eq!(options.initial_config.viewer_port, "4777");
    assert_eq!(
        options.initial_config.launcher_bin,
        "/tmp/oasis7_game_launcher"
    );
    assert_eq!(
        options.initial_config.chain_runtime_bin,
        "/tmp/oasis7_chain_runtime"
    );
    assert_eq!(
        options.initial_config.chain_storage_profile,
        "release_default"
    );
    assert_eq!(options.initial_config.chain_pos_slot_duration_ms, "12000");
    assert_eq!(options.initial_config.chain_pos_ticks_per_slot, "10");
    assert_eq!(options.initial_config.chain_pos_proposal_tick_phase, "9");
    assert!(
        options
            .initial_config
            .chain_pos_adaptive_tick_scheduler_enabled
    );
    assert_eq!(
        options.initial_config.chain_pos_slot_clock_genesis_unix_ms,
        "1700000000000"
    );
    assert_eq!(options.initial_config.chain_pos_max_past_slot_lag, "32");
    assert!(options.initial_config.llm_enabled);
    assert!(!options.initial_config.chain_enabled);
    assert!(options.initial_config.auto_open_browser);
}

#[test]
fn parse_options_collects_repeat_validators() {
    let options = parse_options(
        [
            "--chain-node-validator",
            "node-a:40",
            "--chain-node-validator",
            "node-b:60",
        ]
        .into_iter(),
    )
    .expect("parse validators");

    assert_eq!(
        options.initial_config.chain_node_validators,
        "node-a:40,node-b:60"
    );
}

#[test]
fn parse_options_rejects_unknown_option() {
    let err = parse_options(["--unknown"].into_iter()).expect_err("unknown option should fail");
    assert!(err.contains("unknown option"));
}

#[test]
fn parse_options_rejects_unknown_deployment_mode() {
    let err = parse_options(["--deployment-mode", "invalid"].into_iter())
        .expect_err("invalid deployment mode should fail");
    assert!(err.contains("trusted_local_only"));
}

#[test]
fn parse_options_rejects_out_of_range_chain_pos_proposal_tick_phase() {
    let err = parse_options(
        [
            "--chain-pos-ticks-per-slot",
            "4",
            "--chain-pos-proposal-tick-phase",
            "4",
        ]
        .into_iter(),
    )
    .expect_err("out-of-range proposal tick phase should fail");
    assert!(err.contains("--chain-pos-proposal-tick-phase"));
}

#[test]
fn parse_port_rejects_zero() {
    let err = parse_port("0", "viewer port").expect_err("zero port should fail");
    assert!(err.contains("1..=65535"));
}

#[test]
fn parse_host_port_accepts_ipv6() {
    let (host, port) = parse_host_port("[::1]:5011", "--web-bind").expect("ipv6 host:port");
    assert_eq!(host, "::1");
    assert_eq!(port, 5011);
}

#[test]
fn parse_host_port_rejects_unbracketed_ipv6() {
    let err = parse_host_port("::1:5011", "--web-bind").expect_err("should fail");
    assert!(err.contains("wrapped in []"));
}

#[test]
fn parse_chain_validators_rejects_invalid_format() {
    let err = parse_chain_validators("node-a").expect_err("should fail");
    assert!(err.contains("validator_id:stake"));
}

#[test]
fn build_launcher_args_includes_chain_disable_when_off() {
    let config = LauncherConfig {
        deployment_mode: "hosted_public_join".to_string(),
        chain_enabled: false,
        viewer_static_dir: ".".to_string(),
        ..LauncherConfig::default()
    };
    let args = build_launcher_args(&config).expect("args");
    assert!(args.contains(&"--deployment-mode".to_string()));
    assert!(args.contains(&"hosted_public_join".to_string()));
    assert!(args.contains(&"--chain-disable".to_string()));
    assert!(args.contains(&"--with-llm".to_string()));
    assert!(args.contains(&"--no-open-browser".to_string()));
}

#[test]
fn build_launcher_args_keeps_chain_disabled_even_when_chain_config_is_on() {
    let config = LauncherConfig {
        viewer_static_dir: ".".to_string(),
        chain_enabled: true,
        chain_status_bind: "127.0.0.1:6121".to_string(),
        chain_node_id: "chain-a".to_string(),
        chain_storage_profile: "soak_forensics".to_string(),
        chain_world_id: "live-chain-a".to_string(),
        chain_node_role: "storage".to_string(),
        chain_node_tick_ms: "300".to_string(),
        chain_pos_slot_duration_ms: "12000".to_string(),
        chain_pos_ticks_per_slot: "10".to_string(),
        chain_pos_proposal_tick_phase: "9".to_string(),
        chain_pos_adaptive_tick_scheduler_enabled: true,
        chain_pos_slot_clock_genesis_unix_ms: "1700000000000".to_string(),
        chain_pos_max_past_slot_lag: "32".to_string(),
        chain_node_validators: "chain-a:55,chain-b:45".to_string(),
        ..LauncherConfig::default()
    };
    let args = build_launcher_args(&config).expect("args");
    assert!(args.contains(&"--chain-disable".to_string()));
    assert!(!args.contains(&"--chain-enable".to_string()));
}

#[test]
fn build_chain_runtime_args_includes_chain_overrides_when_on() {
    let config = LauncherConfig {
        viewer_static_dir: ".".to_string(),
        chain_enabled: true,
        chain_status_bind: "127.0.0.1:6121".to_string(),
        chain_node_id: "chain-a".to_string(),
        chain_storage_profile: "soak_forensics".to_string(),
        chain_world_id: "live-chain-a".to_string(),
        chain_node_role: "storage".to_string(),
        chain_node_tick_ms: "300".to_string(),
        chain_pos_slot_duration_ms: "12000".to_string(),
        chain_pos_ticks_per_slot: "10".to_string(),
        chain_pos_proposal_tick_phase: "9".to_string(),
        chain_pos_adaptive_tick_scheduler_enabled: true,
        chain_pos_slot_clock_genesis_unix_ms: "1700000000000".to_string(),
        chain_pos_max_past_slot_lag: "32".to_string(),
        chain_node_validators: "chain-a:55,chain-b:45".to_string(),
        ..LauncherConfig::default()
    };
    let args = build_chain_runtime_args(&config).expect("args");
    assert!(args.contains(&"--status-bind".to_string()));
    assert!(args.contains(&"127.0.0.1:6121".to_string()));
    assert!(args.contains(&"--node-id".to_string()));
    assert!(args.contains(&"--storage-profile".to_string()));
    assert!(args.contains(&"soak_forensics".to_string()));
    assert!(args.contains(&"chain-a".to_string()));
    assert!(args.contains(&"--node-validator".to_string()));
    assert!(args.contains(&"chain-a:55".to_string()));
    assert!(args.contains(&"chain-b:45".to_string()));
    assert!(args.contains(&"--pos-slot-duration-ms".to_string()));
    assert!(args.contains(&"12000".to_string()));
    assert!(args.contains(&"--pos-ticks-per-slot".to_string()));
    assert!(args.contains(&"10".to_string()));
    assert!(args.contains(&"--pos-proposal-tick-phase".to_string()));
    assert!(args.contains(&"9".to_string()));
    assert!(args.contains(&"--pos-adaptive-tick-scheduler".to_string()));
    assert!(args.contains(&"--pos-slot-clock-genesis-unix-ms".to_string()));
    assert!(args.contains(&"1700000000000".to_string()));
    assert!(args.contains(&"--pos-max-past-slot-lag".to_string()));
    assert!(args.contains(&"32".to_string()));
    assert!(args.contains(&"--execution-world-dir".to_string()));
    assert!(
        args.contains(&"output/chain-runtime/chain-a/reward-runtime-execution-world".to_string())
    );
}

#[test]
fn viewer_dev_dist_candidates_only_return_oasis7_path() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let candidates = viewer_dev_dist_candidates();

    assert_eq!(
        candidates,
        vec![repo_root.join("oasis7_viewer").join("dist")]
    );
}

#[test]
fn build_chain_runtime_args_supports_all_storage_profiles() {
    for expected in ["dev_local", "release_default", "soak_forensics"] {
        let config = LauncherConfig {
            viewer_static_dir: ".".to_string(),
            chain_enabled: true,
            chain_status_bind: "127.0.0.1:6121".to_string(),
            chain_node_id: format!("chain-{expected}"),
            chain_storage_profile: expected.to_string(),
            chain_world_id: "live-chain-a".to_string(),
            ..LauncherConfig::default()
        };
        let args = build_chain_runtime_args(&config).expect("args");
        assert!(args.contains(&"--storage-profile".to_string()));
        assert!(args.contains(&expected.to_string()));
    }
}

#[test]
fn build_chain_runtime_args_rejects_unknown_storage_profile() {
    let config = LauncherConfig {
        chain_enabled: true,
        chain_storage_profile: "unknown".to_string(),
        ..LauncherConfig::default()
    };
    let err = build_chain_runtime_args(&config).expect_err("should fail");
    assert!(err.contains("dev_local"));
    assert!(err.contains("release_default"));
    assert!(err.contains("soak_forensics"));
}

#[test]
fn build_game_url_uses_request_host_for_wildcard_bindings() {
    let config = LauncherConfig {
        viewer_host: "0.0.0.0".to_string(),
        viewer_port: "4173".to_string(),
        web_bind: "0.0.0.0:5011".to_string(),
        deployment_mode: "hosted_public_join".to_string(),
        ..LauncherConfig::default()
    };
    let url = build_game_url(&config, Some("10.10.1.8"));
    assert!(url.starts_with("http://10.10.1.8:4173/?ws=ws%3A%2F%2F10.10.1.8%3A5011&hosted_access="));
    assert!(url.contains("%22deployment_mode%22%3A%22hosted_public_join%22"));
    assert!(url.contains("%22action_matrix%22"));
}

#[test]
fn validate_game_config_reports_missing_required_fields() {
    let config = LauncherConfig {
        scenario: "".to_string(),
        live_bind: "127.0.0.1".to_string(),
        web_bind: "127.0.0.1".to_string(),
        viewer_host: "".to_string(),
        viewer_port: "0".to_string(),
        viewer_static_dir: "/missing/dir".to_string(),
        chain_enabled: true,
        chain_status_bind: "127.0.0.1".to_string(),
        chain_node_id: "".to_string(),
        chain_storage_profile: "invalid".to_string(),
        chain_node_role: "invalid".to_string(),
        chain_node_tick_ms: "0".to_string(),
        chain_pos_slot_duration_ms: "0".to_string(),
        chain_pos_ticks_per_slot: "0".to_string(),
        chain_pos_proposal_tick_phase: "x".to_string(),
        chain_pos_slot_clock_genesis_unix_ms: "oops".to_string(),
        chain_pos_max_past_slot_lag: "-1".to_string(),
        chain_node_validators: "node-a".to_string(),
        ..LauncherConfig::default()
    };
    let issues = validate_game_config(&config);
    assert!(!issues.is_empty());
    assert!(issues.iter().any(|item| item.contains("scenario")));
    assert!(issues.iter().any(|item| item.contains("live bind")));
    assert!(issues.iter().any(|item| item.contains("viewer host")));
    assert!(issues
        .iter()
        .any(|item| item.contains("viewer static directory")));
}

#[test]
fn validate_game_config_accepts_minimal_valid_setup() {
    let static_dir = make_temp_dir("oasis7_web_launcher_valid");
    let config = LauncherConfig {
        viewer_static_dir: static_dir.to_string_lossy().to_string(),
        chain_enabled: false,
        ..LauncherConfig::default()
    };
    let issues = validate_game_config(&config);
    assert!(issues.is_empty());
    let _ = fs::remove_dir_all(static_dir);
}

#[test]
fn validate_game_config_rejects_no_llm_playability_config() {
    let static_dir = make_temp_dir("oasis7_web_launcher_no_llm");
    let config = LauncherConfig {
        viewer_static_dir: static_dir.to_string_lossy().to_string(),
        llm_enabled: false,
        chain_enabled: false,
        ..LauncherConfig::default()
    };
    let issues = validate_game_config(&config);
    assert!(issues
        .iter()
        .any(|item| item.contains("llm must stay enabled")));
    let _ = fs::remove_dir_all(static_dir);
}

#[test]
fn validate_game_config_accepts_default_web_alias_using_service_launcher_path() {
    let bundle_root = make_temp_dir("oasis7_web_launcher_bundle");
    let bin_dir = bundle_root.join("bin");
    let web_dir = bundle_root.join("web");
    fs::create_dir_all(&bin_dir).expect("create bin dir");
    fs::create_dir_all(&web_dir).expect("create web dir");
    let launcher_bin = bin_dir.join("oasis7_game_launcher");
    fs::write(&launcher_bin, b"").expect("create launcher stub");

    let config = LauncherConfig {
        viewer_static_dir: "web".to_string(),
        launcher_bin: String::new(),
        chain_enabled: false,
        ..LauncherConfig::default()
    };

    let issues =
        validate_game_config_with_launcher_bin(&config, launcher_bin.to_string_lossy().as_ref());
    assert!(issues.is_empty());

    let args =
        build_launcher_args_with_launcher_bin(&config, launcher_bin.to_string_lossy().as_ref())
            .expect("launcher args");
    let viewer_static_index = args
        .iter()
        .position(|arg| arg == "--viewer-static-dir")
        .expect("viewer static flag");
    assert_eq!(
        fs::canonicalize(PathBuf::from(args[viewer_static_index + 1].as_str()))
            .expect("canonicalize resolved static dir"),
        fs::canonicalize(&web_dir).expect("canonicalize expected static dir")
    );

    let _ = fs::remove_dir_all(bundle_root);
}

#[test]
fn validate_game_config_accepts_bundle_relative_web_path_from_launcher_bin() {
    let bundle_root = make_temp_dir("oasis7_web_launcher_bundle_relative");
    let bundle_bin = bundle_root.join("bin");
    let launcher_bin = bundle_bin.join("oasis7_game_launcher");
    let bundle_web = bundle_root.join("web");
    fs::create_dir_all(&bundle_bin).expect("create bundle bin dir");
    fs::create_dir_all(&bundle_web).expect("create bundle web dir");
    fs::write(&launcher_bin, b"#!/bin/sh\n").expect("write fake launcher bin");

    let config = LauncherConfig {
        launcher_bin: launcher_bin.to_string_lossy().to_string(),
        viewer_static_dir: "web".to_string(),
        chain_enabled: false,
        ..LauncherConfig::default()
    };

    let issues = validate_game_config(&config);
    assert!(issues.is_empty());

    let _ = fs::remove_dir_all(bundle_root);
}

#[test]
fn validate_chain_config_reports_missing_required_fields() {
    let config = LauncherConfig {
        chain_enabled: true,
        chain_status_bind: "127.0.0.1".to_string(),
        chain_node_id: "".to_string(),
        chain_storage_profile: "invalid".to_string(),
        chain_node_role: "invalid".to_string(),
        chain_node_tick_ms: "0".to_string(),
        chain_pos_slot_duration_ms: "0".to_string(),
        chain_pos_ticks_per_slot: "4".to_string(),
        chain_pos_proposal_tick_phase: "4".to_string(),
        chain_pos_slot_clock_genesis_unix_ms: "oops".to_string(),
        chain_pos_max_past_slot_lag: "-1".to_string(),
        chain_node_validators: "node-a".to_string(),
        ..LauncherConfig::default()
    };
    let issues = validate_chain_config(&config);
    assert!(!issues.is_empty());
    assert!(issues.iter().any(|item| item.contains("chain status bind")));
    assert!(issues.iter().any(|item| item.contains("chain node id")));
    assert!(issues
        .iter()
        .any(|item| item.contains("chain storage profile")));
    assert!(issues.iter().any(|item| item.contains("chain pos")));
}

#[test]
fn cli_options_default_launcher_bin_is_not_empty() {
    let options = CliOptions::default();
    assert!(!options.launcher_bin.trim().is_empty());
    assert!(!options.chain_runtime_bin.trim().is_empty());
}

#[test]
fn remap_transfer_runtime_target_preserves_query_parameters() {
    let mapped = remap_transfer_runtime_target(
        "/api/chain/explorer/transactions?status=confirmed&limit=50",
        "/api/chain/explorer/transactions",
        "/v1/chain/explorer/transactions",
    );
    assert_eq!(
        mapped,
        "/v1/chain/explorer/transactions?status=confirmed&limit=50"
    );
}

#[test]
fn remap_transfer_runtime_target_supports_explorer_blocks_pagination() {
    let mapped = remap_transfer_runtime_target(
        "/api/chain/explorer/blocks?cursor=50&limit=25",
        "/api/chain/explorer/blocks",
        "/v1/chain/explorer/blocks",
    );
    assert_eq!(mapped, "/v1/chain/explorer/blocks?cursor=50&limit=25");
}

#[test]
fn remap_transfer_runtime_target_supports_explorer_p1_address_query() {
    let mapped = remap_transfer_runtime_target(
        "/api/chain/explorer/address?account_id=player:alice&limit=20",
        "/api/chain/explorer/address",
        "/v1/chain/explorer/address",
    );
    assert_eq!(
        mapped,
        "/v1/chain/explorer/address?account_id=player:alice&limit=20"
    );
}

#[test]
fn stop_process_noop_preserves_error_state() {
    let config = LauncherConfig {
        chain_enabled: false,
        ..LauncherConfig::default()
    };
    let mut state = ServiceState::new(
        "launcher".to_string(),
        "chain".to_string(),
        PathBuf::from("."),
        config,
    );
    state.process_state = ProcessState::StartFailed("boot failed".to_string());

    stop_process(&mut state).expect("stop no-op should succeed");

    assert!(matches!(
        state.process_state,
        ProcessState::StartFailed(ref detail) if detail == "boot failed"
    ));
}

#[test]
fn stop_chain_process_noop_preserves_error_state() {
    let config = LauncherConfig {
        chain_enabled: true,
        ..LauncherConfig::default()
    };
    let mut state = ServiceState::new(
        "launcher".to_string(),
        "chain".to_string(),
        PathBuf::from("."),
        config,
    );
    state.chain_runtime_status = ChainRuntimeStatus::Unreachable("probe failed".to_string());

    stop_chain_process(&mut state).expect("chain stop no-op should succeed");

    assert!(matches!(
        state.chain_runtime_status,
        ChainRuntimeStatus::Unreachable(ref detail) if detail == "probe failed"
    ));
}

#[test]
fn gui_agent_capabilities_include_expected_actions_and_targets() {
    let capabilities = gui_agent_capabilities_response();
    let encoded = serde_json::to_value(&capabilities).expect("serialize capabilities");
    let actions = encoded
        .get("actions")
        .and_then(serde_json::Value::as_array)
        .expect("actions array");
    let contains_action = |name: &str| {
        actions
            .iter()
            .any(|item| item.as_str().is_some_and(|value| value == name))
    };
    assert!(contains_action("start_game"));
    assert!(contains_action("submit_transfer"));
    assert!(contains_action("query_explorer_mempool"));

    let query_targets = encoded
        .get("query_targets")
        .and_then(serde_json::Value::as_array)
        .expect("query_targets array");
    assert!(query_targets.iter().any(|target| {
        target.get("id").and_then(serde_json::Value::as_str) == Some("transfer.status")
    }));
}

#[test]
fn gui_agent_action_rejects_unknown_action_with_invalid_request() {
    let mut state = ServiceState::new(
        "launcher".to_string(),
        "chain".to_string(),
        PathBuf::from("."),
        LauncherConfig::default(),
    );

    let response = execute_gui_agent_action(
        &mut state,
        br#"{"action":"unknown_action","payload":null}"#,
        Some("127.0.0.1"),
    );
    let encoded = serde_json::to_value(&response).expect("serialize response");

    assert_eq!(encoded["ok"], serde_json::json!(false));
    assert_eq!(encoded["action"], serde_json::json!("unknown_action"));
    assert_eq!(encoded["error_code"], serde_json::json!("invalid_request"));
    assert!(encoded.get("state").is_some());
}

#[test]
fn gui_agent_query_chain_disabled_returns_structured_error() {
    let config = LauncherConfig {
        chain_enabled: false,
        ..LauncherConfig::default()
    };
    let mut state = ServiceState::new(
        "launcher".to_string(),
        "chain".to_string(),
        PathBuf::from("."),
        config,
    );

    let response = execute_gui_agent_action(
        &mut state,
        br#"{"action":"query_transfer_accounts","payload":null}"#,
        Some("127.0.0.1"),
    );
    let encoded = serde_json::to_value(&response).expect("serialize response");

    assert_eq!(encoded["ok"], serde_json::json!(false));
    assert_eq!(
        encoded["action"],
        serde_json::json!("query_transfer_accounts")
    );
    assert_eq!(encoded["error_code"], serde_json::json!("chain_disabled"));
    assert!(encoded
        .get("data")
        .and_then(|value| value.get("error_code"))
        .is_some());
    assert!(encoded.get("state").is_some());
}

#[test]
fn gui_agent_action_response_includes_state_snapshot_fields() {
    let mut state = ServiceState::new(
        "launcher".to_string(),
        "chain".to_string(),
        PathBuf::from("."),
        LauncherConfig::default(),
    );

    let response = execute_gui_agent_action(
        &mut state,
        br#"{"action":"stop_game","payload":null}"#,
        Some("127.0.0.1"),
    );
    let encoded = serde_json::to_value(&response).expect("serialize response");

    assert_eq!(encoded["ok"], serde_json::json!(true));
    assert_eq!(encoded["action"], serde_json::json!("stop_game"));
    assert!(encoded.get("error_code").is_none());
    assert!(encoded.get("state").is_some());
    assert!(encoded
        .get("state")
        .and_then(|value| value.get("status"))
        .and_then(serde_json::Value::as_str)
        .is_some());
    assert!(encoded
        .get("state")
        .and_then(|value| value.get("chain_status"))
        .and_then(serde_json::Value::as_str)
        .is_some());
}

#[test]
fn finalize_chain_start_outcome_reports_stale_execution_world() {
    let mut state = ServiceState::new(
        "launcher".to_string(),
        "chain".to_string(),
        PathBuf::from("."),
        LauncherConfig::default(),
    );
    state.chain_runtime_status =
        ChainRuntimeStatus::StaleExecutionWorld("stale execution world detected".to_string());
    state.chain_recovery = Some(ChainRecoverySnapshot {
        error_code: "stale_execution_world".to_string(),
        reason: "stale execution world detected".to_string(),
        node_id: "viewer-live-node".to_string(),
        execution_world_dir: "output/chain-runtime/viewer-live-node/reward-runtime-execution-world"
            .to_string(),
        recovery_mode: "fresh_node_id".to_string(),
        reset_required: false,
        fresh_node_id: "viewer-live-node-fresh-1".to_string(),
        fresh_chain_status_bind: "127.0.0.1:5122".to_string(),
        suggested_config: LauncherConfig {
            chain_node_id: "viewer-live-node-fresh-1".to_string(),
            chain_status_bind: "127.0.0.1:5122".to_string(),
            ..LauncherConfig::default()
        },
    });

    let err = finalize_chain_start_outcome(&state, Ok(())).expect_err("should surface stale error");
    assert!(err.contains("stale execution world"));
    assert_eq!(
        chain_error_code_for_state(&state, err.as_str()),
        "stale_execution_world"
    );

    let snapshot = snapshot_from_state(&state, Some("127.0.0.1"));
    let encoded = serde_json::to_value(&snapshot).expect("serialize snapshot");
    assert_eq!(
        encoded["chain_status"],
        serde_json::json!("stale_execution_world")
    );
    assert_eq!(
        encoded["chain_recovery"]["fresh_node_id"],
        serde_json::json!("viewer-live-node-fresh-1")
    );
}

#[test]
fn gui_agent_capabilities_include_recover_chain_action() {
    let capabilities = gui_agent_capabilities_response();
    let encoded = serde_json::to_value(&capabilities).expect("serialize capabilities");
    let actions = encoded
        .get("actions")
        .and_then(serde_json::Value::as_array)
        .expect("actions array");
    assert!(actions
        .iter()
        .any(|item| item.as_str() == Some("recover_chain")));
}

fn make_temp_dir(label: &str) -> PathBuf {
    let mut path = env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!(
        "oasis7_oasis7_web_launcher_test_{label}_{}_{}",
        std::process::id(),
        stamp
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}
