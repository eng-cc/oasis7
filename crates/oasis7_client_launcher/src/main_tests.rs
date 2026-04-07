use super::platform_ops::viewer_dev_dist_candidates;
use super::{
    build_chain_runtime_args, build_game_url, build_launcher_args, chain_runtime_status_from_web,
    check_provider_loopback_http_provider, collect_chain_required_config_issues,
    collect_required_config_issues,
    config_ui::{issue_field_ids, StartupGuideTarget},
    encode_query_value, encoded_query_pair,
    explorer_window::{
        resolve_explorer_my_account_candidate, ExplorerQuickShortcut, ExplorerStatusFilter,
        WebExplorerOverviewResponse,
    },
    install_cjk_font, normalize_host_for_url, parse_chain_role, parse_chain_validators,
    parse_host_port, parse_port, probe_chain_status_endpoint, read_named_env_value_with,
    resolve_control_plane_env_with,
    self_guided::{
        resolve_config_guide_target, resolve_next_task_hint, resolve_primary_disabled_cta,
        ConfigGuideTargetHint, DemoModePhase, DisabledActionCta, NextTaskHint, OnboardingStep,
    },
    self_guided_blocked_actions::resolve_disabled_cta_plan,
    self_guided_preflight::{resolve_chain_runtime_preflight_state, PreflightCheckState},
    should_request_auto_chain_start,
    transfer_window::{
        hosted_public_join_transfer_blocked, recommend_default_from_account,
        recommend_transfer_account_ids, resolve_transfer_timeline, transfer_amount_presets,
        TransferTimelineState, WebTransferAccountEntry, WebTransferLifecycleStatus,
    },
    ChainRuntimeStatus, ClientLauncherApp, ConfigIssue, GlossaryTerm, LaunchConfig, LauncherStatus,
    ProviderCompatibilityStatus, UiLanguage, WebChainRecoverySnapshot, WebRequestDomain,
    WebStateSnapshot, DEFAULT_CLIENT_LAUNCHER_CONTROL_BIND, OASIS7_CJK_FONT_NAME,
    OASIS7_CLIENT_LAUNCHER_LANG_ENV,
};
use eframe::egui;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[path = "main_tests_onboarding.rs"]
mod onboarding_tests;
#[test]
fn parse_port_rejects_zero() {
    let err = parse_port("0", "viewer port").expect_err("should fail");
    assert!(err.contains("1..=65535"));
}
#[test]
fn parse_host_port_requires_colon() {
    let err = parse_host_port("127.0.0.1", "web bind").expect_err("should fail");
    assert!(err.contains("<host:port>"));
}
#[test]
fn parse_host_port_accepts_bracketed_ipv6() {
    let (host, port) = parse_host_port("[::1]:5011", "web bind").expect("ok");
    assert_eq!(host, "::1");
    assert_eq!(port, 5011);
}
#[test]
fn parse_host_port_rejects_unbracketed_ipv6() {
    let err = parse_host_port("::1:5011", "web bind").expect_err("should fail");
    assert!(err.contains("wrapped in []"));
}
#[test]
fn build_launcher_args_contains_llm_and_no_open_switches() {
    let config = LaunchConfig {
        llm_enabled: true,
        auto_open_browser: false,
        chain_enabled: false,
        ..LaunchConfig::default()
    };
    let args = build_launcher_args(&config).expect("args should build");
    assert!(args.contains(&"--with-llm".to_string()));
    assert!(args.contains(&"--agent-decision-source".to_string()));
    assert!(args.contains(&"builtin_llm".to_string()));
    assert!(args.contains(&"--deployment-mode".to_string()));
    assert!(args.contains(&"trusted_local_only".to_string()));
    assert!(args.contains(&"--no-open-browser".to_string()));
    assert!(args.contains(&"--viewer-static-dir".to_string()));
    assert!(args.contains(&"--chain-disable".to_string()));
}
#[test]
fn build_launcher_args_rejects_empty_static_dir() {
    let config = LaunchConfig {
        viewer_static_dir: "".to_string(),
        ..LaunchConfig::default()
    };
    let err = build_launcher_args(&config).expect_err("should fail");
    assert!(err.contains("static dir"));
}

#[test]
fn build_launcher_args_accepts_agent_direct_connect_alias() {
    let config = LaunchConfig {
        llm_enabled: true,
        agent_decision_source: "agent_direct_connect".to_string(),
        agent_provider_url: "http://127.0.0.1:5841".to_string(),
        agent_provider_auth_token: "secret-token".to_string(),
        agent_provider_connect_timeout_ms: "15000".to_string(),
        agent_execution_lane: "headless".to_string(),
        agent_provider_profile: "oasis7_p0_low_freq_npc".to_string(),
        ..LaunchConfig::default()
    };
    let args = build_launcher_args(&config).expect("args should build");
    assert!(args.contains(&"--agent-decision-source".to_string()));
    assert!(args.contains(&"provider_backed".to_string()));
    assert!(args.contains(&"--agent-provider-backend".to_string()));
    assert!(args.contains(&"openclaw".to_string()));
    assert!(args.contains(&"--agent-provider-contract".to_string()));
    assert!(args.contains(&"worldsim_provider_v1".to_string()));
    assert!(args.contains(&"--agent-provider-transport".to_string()));
    assert!(args.contains(&"loopback_http".to_string()));
    assert!(args.contains(&"--agent-provider-url".to_string()));
    assert!(args.contains(&"http://127.0.0.1:5841".to_string()));
    assert!(args.contains(&"--agent-provider-auth-token".to_string()));
    assert!(args.contains(&"secret-token".to_string()));
    assert!(args.contains(&"--agent-provider-connect-timeout-ms".to_string()));
    assert!(args.contains(&"15000".to_string()));
    assert!(args.contains(&"--agent-execution-lane".to_string()));
    assert!(args.contains(&"headless_agent".to_string()));
    assert!(args.contains(&"--agent-provider-profile".to_string()));
    assert!(args.contains(&"oasis7_p0_low_freq_npc".to_string()));
}

#[test]
fn hosted_public_join_transfer_barrier_tracks_deployment_mode() {
    assert!(hosted_public_join_transfer_blocked(&LaunchConfig {
        deployment_mode: "hosted_public_join".to_string(),
        ..LaunchConfig::default()
    }));
    assert!(!hosted_public_join_transfer_blocked(
        &LaunchConfig::default()
    ));
}
#[test]
fn build_game_url_rewrites_zero_host() {
    let config = LaunchConfig {
        viewer_host: "0.0.0.0".to_string(),
        viewer_port: "4173".to_string(),
        web_bind: "0.0.0.0:5011".to_string(),
        deployment_mode: "hosted_public_join".to_string(),
        ..LaunchConfig::default()
    };
    let url = build_game_url(&config);
    assert!(url.starts_with("http://127.0.0.1:4173/?ws=ws%3A%2F%2F127.0.0.1%3A5011&hosted_access="));
    assert!(url.contains("%22deployment_mode%22%3A%22hosted_public_join%22"));
}
#[test]
fn build_game_url_brackets_ipv6_hosts() {
    let config = LaunchConfig {
        viewer_host: "::1".to_string(),
        viewer_port: "4173".to_string(),
        web_bind: "[::1]:5011".to_string(),
        ..LaunchConfig::default()
    };
    let url = build_game_url(&config);
    assert!(url.starts_with("http://[::1]:4173/?ws=ws%3A%2F%2F%5B%3A%3A1%5D%3A5011&hosted_access="));
    assert!(url.contains("%22deployment_mode%22%3A%22trusted_local_only%22"));
}

#[test]
fn viewer_dev_dist_candidates_only_return_current_oasis7_name() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let candidates = viewer_dev_dist_candidates();

    assert_eq!(
        candidates,
        vec![repo_root.join("oasis7_viewer").join("dist")]
    );
}

#[test]
fn normalize_host_for_url_maps_empty_and_any() {
    assert_eq!(normalize_host_for_url("0.0.0.0"), "127.0.0.1");
    assert_eq!(normalize_host_for_url(""), "127.0.0.1");
    assert_eq!(normalize_host_for_url("192.168.0.2"), "192.168.0.2");
}
#[test]
fn launch_config_defaults_enable_llm() {
    let config = LaunchConfig::default();
    assert!(config.llm_enabled);
    assert!(config.chain_enabled);
    assert_eq!(config.agent_decision_source, "builtin_llm");
    assert_eq!(config.agent_provider_backend, "openclaw");
    assert_eq!(config.agent_provider_contract, "worldsim_provider_v1");
    assert_eq!(config.agent_provider_transport, "loopback_http");
    assert_eq!(config.agent_provider_url, "http://127.0.0.1:5841");
    assert_eq!(config.agent_provider_connect_timeout_ms, "15000");
    assert_eq!(config.agent_execution_lane, "player_parity");
    assert_eq!(config.agent_provider_profile, "oasis7_p0_low_freq_npc");
    assert!(config.openclaw_auto_discover);
    assert!(config.chain_node_id.starts_with("viewer-live-node-fresh-"));
    assert_eq!(config.chain_p2p_user_mode, "auto_join");
    assert!(!config.chain_p2p_accept_public_entry);
}

#[test]
fn launch_config_deserialize_backfills_missing_openclaw_execution_mode() {
    let config: LaunchConfig = serde_json::from_value(json!({
        "agent_provider_mode": "provider_loopback_http",
        "openclaw_base_url": "http://127.0.0.1:5841",
        "openclaw_connect_timeout_ms": "15000",
        "openclaw_agent_profile": "oasis7_p0_low_freq_npc"
    }))
    .expect("deserialize launch config");
    assert_eq!(config.agent_execution_lane, "player_parity");
    let issues = collect_required_config_issues(&config);
    assert!(!issues.contains(&ConfigIssue::OpenClawExecutionModeInvalid));
}

#[test]
fn collect_required_config_issues_requires_valid_openclaw_execution_mode() {
    let issues = collect_required_config_issues(&LaunchConfig {
        agent_decision_source: "provider_backed".to_string(),
        agent_execution_lane: "gpu_only".to_string(),
        ..LaunchConfig::default()
    });
    assert!(issues.contains(&ConfigIssue::OpenClawExecutionModeInvalid));
}
#[test]
fn build_launcher_args_keeps_chain_disabled_even_when_chain_config_is_set() {
    let config = LaunchConfig {
        chain_enabled: true,
        chain_status_bind: "127.0.0.1:6121".to_string(),
        chain_node_id: "chain-node-a".to_string(),
        chain_world_id: "live-chain-a".to_string(),
        chain_node_role: "storage".to_string(),
        chain_node_tick_ms: "350".to_string(),
        chain_node_validators: "node-a:55,node-b:45".to_string(),
        ..LaunchConfig::default()
    };
    let args = build_launcher_args(&config).expect("args should build");
    assert!(args.contains(&"--chain-disable".to_string()));
    assert!(!args.contains(&"--chain-enable".to_string()));
    assert!(!args.contains(&"--chain-status-bind".to_string()));
}

#[test]
fn build_chain_runtime_args_contains_chain_overrides_when_enabled() {
    let config = LaunchConfig {
        chain_enabled: true,
        chain_status_bind: "127.0.0.1:6121".to_string(),
        chain_node_id: "chain-node-a".to_string(),
        chain_world_id: "live-chain-a".to_string(),
        chain_node_role: "storage".to_string(),
        chain_node_tick_ms: "350".to_string(),
        chain_pos_slot_duration_ms: "12000".to_string(),
        chain_pos_ticks_per_slot: "10".to_string(),
        chain_pos_proposal_tick_phase: "9".to_string(),
        chain_pos_adaptive_tick_scheduler_enabled: true,
        chain_pos_slot_clock_genesis_unix_ms: "1700000000000".to_string(),
        chain_pos_max_past_slot_lag: "32".to_string(),
        chain_node_validators: "node-a:55,node-b:45".to_string(),
        chain_runtime_bin: "/tmp/oasis7_chain_runtime".to_string(),
        ..LaunchConfig::default()
    };
    let args = build_chain_runtime_args(&config).expect("args should build");
    assert!(args.contains(&"--node-id".to_string()));
    assert!(args.contains(&"chain-node-a".to_string()));
    assert!(args.contains(&"--world-id".to_string()));
    assert!(args.contains(&"live-chain-a".to_string()));
    assert!(args.contains(&"--status-bind".to_string()));
    assert!(args.contains(&"127.0.0.1:6121".to_string()));
    assert!(args.contains(&"--node-role".to_string()));
    assert!(args.contains(&"storage".to_string()));
    assert!(args.contains(&"--p2p-user-mode".to_string()));
    assert!(args.contains(&"auto_join".to_string()));
    assert!(args.contains(&"--p2p-decline-public-entry".to_string()));
    assert!(args.contains(&"--node-tick-ms".to_string()));
    assert!(args.contains(&"350".to_string()));
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
    assert!(args.contains(&"--node-validator".to_string()));
    assert!(args.contains(&"node-a:55".to_string()));
    assert!(args.contains(&"node-b:45".to_string()));
}
#[test]
fn parse_chain_role_rejects_invalid_value() {
    let err = parse_chain_role("invalid").expect_err("should fail");
    assert!(err.contains("sequencer|storage|observer"));
}
#[test]
fn parse_chain_validators_rejects_invalid_format() {
    let err = parse_chain_validators("node-a").expect_err("should fail");
    assert!(err.contains("<validator_id:stake>"));
}
#[test]
fn install_cjk_font_registers_font_and_priority() {
    let mut fonts = egui::FontDefinitions::default();
    install_cjk_font(
        &mut fonts,
        OASIS7_CJK_FONT_NAME.to_string(),
        egui::FontData::from_static(&[0u8, 1u8]),
    );

    assert!(fonts.font_data.contains_key(OASIS7_CJK_FONT_NAME));

    let proportional = fonts
        .families
        .get(&egui::FontFamily::Proportional)
        .expect("proportional family");
    assert_eq!(
        proportional.first().map(String::as_str),
        Some(OASIS7_CJK_FONT_NAME)
    );

    let monospace = fonts
        .families
        .get(&egui::FontFamily::Monospace)
        .expect("monospace family");
    assert!(monospace.iter().any(|name| name == OASIS7_CJK_FONT_NAME));
}
#[test]
fn parse_ui_language_supports_zh_and_en_aliases() {
    assert_eq!(UiLanguage::from_tag("zh"), Some(UiLanguage::ZhCn));
    assert_eq!(UiLanguage::from_tag("zh-CN"), Some(UiLanguage::ZhCn));
    assert_eq!(UiLanguage::from_tag("en"), Some(UiLanguage::EnUs));
    assert_eq!(UiLanguage::from_tag("EN_us"), Some(UiLanguage::EnUs));
    assert_eq!(UiLanguage::from_tag("ja"), None);
}

#[test]
fn read_named_env_value_with_rejects_removed_old_brand_key_names() {
    let removed_old_brand_lang_key = removed_old_brand_launcher_env("LANG");
    let values = BTreeMap::from([(removed_old_brand_lang_key.as_str(), "en-US")]);
    let resolved = read_named_env_value_with(
        &|key| values.get(key).map(|value| value.to_string()),
        &[OASIS7_CLIENT_LAUNCHER_LANG_ENV],
    );
    assert_eq!(resolved, None);
}

#[test]
fn ui_language_detect_from_values_prefers_current_launcher_value_and_falls_back_to_lang() {
    assert_eq!(
        UiLanguage::detect_from_values(Some("en-US"), Some("zh-CN")),
        UiLanguage::EnUs
    );
    assert_eq!(
        UiLanguage::detect_from_values(None, Some("en-US")),
        UiLanguage::EnUs
    );
    let removed_old_brand_lang_key = removed_old_brand_launcher_env("LANG");
    assert_eq!(
        UiLanguage::detect_from_values(Some(removed_old_brand_lang_key.as_str()), Some("zh-CN")),
        UiLanguage::ZhCn
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn resolve_control_plane_env_with_rejects_removed_old_brand_key_names() {
    let removed_old_brand_control_url = removed_old_brand_launcher_env("CONTROL_URL");
    let removed_old_brand_control_bind = removed_old_brand_launcher_env("CONTROL_BIND");
    let values = BTreeMap::from([
        (
            removed_old_brand_control_url.as_str(),
            "http://127.0.0.1:9999",
        ),
        (removed_old_brand_control_bind.as_str(), "127.0.0.1:9998"),
    ]);
    let (control_url_from_env, control_listen_bind, control_api_base, control_manage_service) =
        resolve_control_plane_env_with(&|key| values.get(key).map(|value| value.to_string()));

    assert_eq!(control_url_from_env, None);
    assert_eq!(
        control_listen_bind,
        DEFAULT_CLIENT_LAUNCHER_CONTROL_BIND.to_string()
    );
    assert_eq!(control_api_base, "http://127.0.0.1:5410");
    assert!(control_manage_service);
}

fn removed_old_brand_launcher_env(suffix: &str) -> String {
    ["AGENT", "WORLD", "CLIENT", "LAUNCHER", suffix].join("_")
}
#[test]
fn launcher_status_text_is_localized() {
    assert_eq!(LauncherStatus::Idle.text(UiLanguage::ZhCn), "未启动");
    assert_eq!(LauncherStatus::Idle.text(UiLanguage::EnUs), "Not Started");
}

#[test]
fn chain_runtime_status_text_is_localized() {
    assert_eq!(ChainRuntimeStatus::Ready.text(UiLanguage::ZhCn), "已就绪");
    assert_eq!(
        ChainRuntimeStatus::Unreachable("x".to_string()).text(UiLanguage::EnUs),
        "Unreachable"
    );
}

#[test]
fn encode_query_value_percent_encodes_reserved_characters() {
    assert_eq!(
        encode_query_value("player:alice & bob?"),
        "player%3Aalice%20%26%20bob%3F"
    );
    assert_eq!(encode_query_value("你好"), "%E4%BD%A0%E5%A5%BD");
}

#[test]
fn encoded_query_pair_formats_key_value_pair() {
    assert_eq!(
        encoded_query_pair("account_id", "player:alice&bob"),
        "account_id=player%3Aalice%26bob"
    );
}

#[test]
fn transfer_amount_presets_match_product_defaults() {
    assert_eq!(transfer_amount_presets(), &[1, 10, 100]);
}

#[test]
fn recommend_default_from_account_uses_highest_liquid_balance() {
    let accounts = vec![
        WebTransferAccountEntry {
            account_id: "player-a".to_string(),
            liquid_balance: 30,
            vested_balance: 0,
            next_nonce_hint: 3,
        },
        WebTransferAccountEntry {
            account_id: "player-b".to_string(),
            liquid_balance: 90,
            vested_balance: 0,
            next_nonce_hint: 1,
        },
        WebTransferAccountEntry {
            account_id: "player-c".to_string(),
            liquid_balance: 20,
            vested_balance: 0,
            next_nonce_hint: 5,
        },
    ];

    assert_eq!(
        recommend_default_from_account(accounts.as_slice()),
        Some("player-b".to_string())
    );
}

#[test]
fn recommend_transfer_account_ids_excludes_sender_and_sorts_by_balance() {
    let accounts = vec![
        WebTransferAccountEntry {
            account_id: "player-a".to_string(),
            liquid_balance: 50,
            vested_balance: 0,
            next_nonce_hint: 1,
        },
        WebTransferAccountEntry {
            account_id: "player-b".to_string(),
            liquid_balance: 80,
            vested_balance: 0,
            next_nonce_hint: 2,
        },
        WebTransferAccountEntry {
            account_id: "player-c".to_string(),
            liquid_balance: 20,
            vested_balance: 0,
            next_nonce_hint: 3,
        },
        WebTransferAccountEntry {
            account_id: "player-d".to_string(),
            liquid_balance: 70,
            vested_balance: 0,
            next_nonce_hint: 4,
        },
    ];

    assert_eq!(
        recommend_transfer_account_ids(accounts.as_slice(), "player-a", 3),
        vec![
            "player-b".to_string(),
            "player-d".to_string(),
            "player-c".to_string()
        ]
    );
}

#[test]
fn resolve_transfer_timeline_tracks_accepted_pending_final_states() {
    assert_eq!(
        resolve_transfer_timeline(WebTransferLifecycleStatus::Accepted),
        [
            TransferTimelineState::Active,
            TransferTimelineState::Waiting,
            TransferTimelineState::Waiting
        ]
    );
    assert_eq!(
        resolve_transfer_timeline(WebTransferLifecycleStatus::Pending),
        [
            TransferTimelineState::Done,
            TransferTimelineState::Active,
            TransferTimelineState::Waiting
        ]
    );
    assert_eq!(
        resolve_transfer_timeline(WebTransferLifecycleStatus::Confirmed),
        [
            TransferTimelineState::Done,
            TransferTimelineState::Done,
            TransferTimelineState::Done
        ]
    );
    assert_eq!(
        resolve_transfer_timeline(WebTransferLifecycleStatus::Failed),
        [
            TransferTimelineState::Done,
            TransferTimelineState::Done,
            TransferTimelineState::Failed
        ]
    );
}

#[test]
fn resolve_explorer_my_account_candidate_prefers_transfer_sender() {
    assert_eq!(
        resolve_explorer_my_account_candidate("player-a", "player-b", "player-c"),
        Some("player-a".to_string())
    );
    assert_eq!(
        resolve_explorer_my_account_candidate("", "player-b", "player-c"),
        Some("player-b".to_string())
    );
    assert_eq!(
        resolve_explorer_my_account_candidate("", "", "player-c"),
        Some("player-c".to_string())
    );
    assert_eq!(resolve_explorer_my_account_candidate("", "", ""), None);
}

#[test]
fn explorer_quick_shortcut_recent_txs_resets_filters_and_refreshes() {
    let mut app = ClientLauncherApp::default();
    app.explorer_panel_state.account_filter = "player-a".to_string();
    app.explorer_panel_state.action_filter_input = "42".to_string();
    app.explorer_panel_state.status_filter = ExplorerStatusFilter::Failed;
    app.explorer_panel_state.txs_cursor = 20;

    app.apply_explorer_quick_shortcut(ExplorerQuickShortcut::RecentTxs);

    assert!(app.explorer_panel_state.account_filter.is_empty());
    assert!(app.explorer_panel_state.action_filter_input.is_empty());
    assert_eq!(
        app.explorer_panel_state.status_filter,
        ExplorerStatusFilter::All
    );
    assert_eq!(app.explorer_panel_state.txs_cursor, 0);
    assert!(app.explorer_panel_state.pending_txs_refresh);
}

#[test]
fn explorer_quick_shortcut_my_account_logs_when_missing_candidate() {
    let mut app = ClientLauncherApp::default();
    app.ui_language = UiLanguage::EnUs;
    let logs_before = app.logs.len();

    app.apply_explorer_quick_shortcut(ExplorerQuickShortcut::MyAccount);

    assert_eq!(app.logs.len(), logs_before + 1);
    let latest_log = app.logs.back().expect("latest log should exist");
    assert!(latest_log.contains("My Account shortcut is unavailable"));
}

#[test]
fn explorer_quick_shortcut_latest_block_prefills_height_from_overview() {
    let mut app = ClientLauncherApp::default();
    app.explorer_panel_state.overview = Some(WebExplorerOverviewResponse {
        ok: true,
        observed_at_unix_ms: 1,
        node_id: "node-a".to_string(),
        world_id: "world-a".to_string(),
        latest_height: 88,
        committed_height: 88,
        network_committed_height: 88,
        last_block_hash: Some("hash-a".to_string()),
        last_execution_block_hash: Some("hash-b".to_string()),
        tracked_records: 0,
        transfer_total: 0,
        transfer_accepted: 0,
        transfer_pending: 0,
        transfer_confirmed: 0,
        transfer_failed: 0,
        transfer_timeout: 0,
        error_code: None,
        error: None,
    });

    app.apply_explorer_quick_shortcut(ExplorerQuickShortcut::LatestBlock);

    assert_eq!(app.explorer_panel_state.block_height_input, "88");
    assert_eq!(app.explorer_panel_state.pending_block_height, Some(88));
    assert!(app.explorer_panel_state.pending_block_refresh);
}

#[test]
fn glossary_terms_cover_nonce_slot_mempool_action_id() {
    let mut app = ClientLauncherApp::default();
    app.ui_language = UiLanguage::EnUs;
    assert_eq!(app.glossary_term_text(GlossaryTerm::Nonce), "nonce");
    assert_eq!(app.glossary_term_text(GlossaryTerm::Slot), "slot");
    assert_eq!(app.glossary_term_text(GlossaryTerm::Mempool), "mempool");
    assert_eq!(app.glossary_term_text(GlossaryTerm::ActionId), "action_id");

    for term in [
        GlossaryTerm::Nonce,
        GlossaryTerm::Slot,
        GlossaryTerm::Mempool,
        GlossaryTerm::ActionId,
    ] {
        assert!(!app.glossary_term_definition(term).trim().is_empty());
    }
}

#[test]
fn feedback_availability_requires_chain_ready() {
    let mut app = ClientLauncherApp::default();
    app.config.chain_enabled = true;
    app.chain_runtime_status = ChainRuntimeStatus::Ready;
    assert!(app.is_feedback_available());

    app.chain_runtime_status = ChainRuntimeStatus::Starting;
    assert!(!app.is_feedback_available());

    app.chain_runtime_status = ChainRuntimeStatus::Ready;
    app.config.chain_enabled = false;
    assert!(!app.is_feedback_available());
}

#[test]
fn feedback_unavailable_hint_includes_status_reason() {
    let mut app = ClientLauncherApp::default();
    app.ui_language = UiLanguage::EnUs;
    app.chain_runtime_status = ChainRuntimeStatus::Starting;
    let hint = app
        .feedback_unavailable_hint()
        .expect("starting status should provide hint");
    assert!(hint.contains("starting"));

    app.chain_runtime_status = ChainRuntimeStatus::ConfigError("bad bind".to_string());
    let hint = app
        .feedback_unavailable_hint()
        .expect("config error status should provide hint");
    assert!(hint.contains("bad bind"));
}

#[test]
fn web_request_inflight_domains_are_independent() {
    let mut app = ClientLauncherApp::default();
    assert!(!app.any_web_request_inflight());
    assert!(!app.any_transfer_request_inflight());

    app.set_web_request_inflight(WebRequestDomain::StatePoll, true);
    assert!(app.web_request_inflight_for(WebRequestDomain::StatePoll));
    assert!(!app.web_request_inflight_for(WebRequestDomain::ExplorerQuery));
    assert!(app.any_web_request_inflight());

    app.set_web_request_inflight(WebRequestDomain::TransferQuery, true);
    assert!(app.any_transfer_request_inflight());
    assert!(app.web_request_inflight_for(WebRequestDomain::TransferQuery));
    assert!(!app.web_request_inflight_for(WebRequestDomain::TransferSubmit));

    app.set_web_request_inflight(WebRequestDomain::StatePoll, false);
    app.set_web_request_inflight(WebRequestDomain::TransferQuery, false);
    assert!(!app.any_web_request_inflight());
    assert!(!app.any_transfer_request_inflight());
}

#[test]
fn chain_runtime_status_from_web_maps_stale_execution_world() {
    let status = chain_runtime_status_from_web(
        "stale_execution_world",
        Some("stale execution world detected"),
    );
    assert!(matches!(
        status,
        ChainRuntimeStatus::StaleExecutionWorld(ref detail)
            if detail == "stale execution world detected"
    ));
}

#[test]
fn apply_web_snapshot_tracks_chain_recovery_payload() {
    let mut app = ClientLauncherApp::default();
    let snapshot = WebStateSnapshot {
        status: "idle".to_string(),
        detail: None,
        chain_status: "stale_execution_world".to_string(),
        chain_detail: Some("stale execution world detected".to_string()),
        chain_p2p_status: None,
        chain_recovery: Some(WebChainRecoverySnapshot {
            error_code: "stale_execution_world".to_string(),
            reason: "stale execution world detected".to_string(),
            node_id: "viewer-live-node".to_string(),
            execution_world_dir:
                "output/chain-runtime/viewer-live-node/reward-runtime-execution-world".to_string(),
            recovery_mode: "fresh_node_id".to_string(),
            reset_required: false,
            fresh_node_id: "viewer-live-node-fresh-1".to_string(),
            fresh_chain_status_bind: "127.0.0.1:5122".to_string(),
            suggested_config: LaunchConfig {
                chain_node_id: "viewer-live-node-fresh-1".to_string(),
                chain_status_bind: "127.0.0.1:5122".to_string(),
                ..LaunchConfig::default()
            },
        }),
        game_url: "http://127.0.0.1:4173/".to_string(),
        config: LaunchConfig::default(),
        logs: vec![],
    };

    app.apply_web_snapshot(snapshot);
    assert!(matches!(
        app.chain_runtime_status,
        ChainRuntimeStatus::StaleExecutionWorld(_)
    ));
    assert_eq!(
        app.chain_recovery
            .as_ref()
            .map(|value| value.fresh_node_id.as_str()),
        Some("viewer-live-node-fresh-1")
    );
}

#[test]
fn apply_web_snapshot_preserves_local_dirty_config_when_snapshot_differs() {
    let mut app = ClientLauncherApp::default();
    app.config.scenario = "local_edit".to_string();
    app.config_dirty = true;

    let mut remote_config = app.config.clone();
    remote_config.scenario = "remote_value".to_string();
    let snapshot = WebStateSnapshot {
        status: "idle".to_string(),
        detail: None,
        chain_status: "not_started".to_string(),
        chain_detail: None,
        chain_p2p_status: None,
        chain_recovery: None,
        game_url: "http://127.0.0.1:4173/".to_string(),
        config: remote_config,
        logs: vec!["snapshot".to_string()],
    };

    app.apply_web_snapshot(snapshot);
    assert_eq!(app.config.scenario, "local_edit");
    assert!(app.config_dirty);
}

#[test]
fn apply_web_snapshot_clears_dirty_flag_when_snapshot_matches_local_config() {
    let mut app = ClientLauncherApp::default();
    app.config.scenario = "same_value".to_string();
    app.config_dirty = true;
    let snapshot = WebStateSnapshot {
        status: "idle".to_string(),
        detail: None,
        chain_status: "not_started".to_string(),
        chain_detail: None,
        chain_p2p_status: None,
        chain_recovery: None,
        game_url: "http://127.0.0.1:4173/".to_string(),
        config: app.config.clone(),
        logs: vec!["snapshot".to_string()],
    };

    app.apply_web_snapshot(snapshot);
    assert!(!app.config_dirty);
}

#[test]
fn auto_chain_start_waits_for_initial_control_plane_snapshot() {
    assert!(!should_request_auto_chain_start(false, true, false, false));
    assert!(should_request_auto_chain_start(false, true, false, true));
    assert!(!should_request_auto_chain_start(true, true, false, true));
    assert!(!should_request_auto_chain_start(false, true, true, true));
    assert!(!should_request_auto_chain_start(false, false, false, true));
}

#[test]
fn apply_web_snapshot_marks_control_plane_snapshot_received() {
    let mut app = ClientLauncherApp::default();
    assert!(!app.control_plane_snapshot_received);

    let snapshot = WebStateSnapshot {
        status: "idle".to_string(),
        detail: None,
        chain_status: "not_started".to_string(),
        chain_detail: None,
        chain_p2p_status: None,
        chain_recovery: None,
        game_url: "http://127.0.0.1:4173/".to_string(),
        config: app.config.clone(),
        logs: vec![],
    };

    app.apply_web_snapshot(snapshot);
    assert!(app.control_plane_snapshot_received);
}

#[test]
fn clear_transfer_history_filters_resets_filters_and_marks_refresh() {
    let mut app = ClientLauncherApp::default();
    app.transfer_panel_state.history_account_filter = "acc-1".to_string();
    app.transfer_panel_state.history_action_filter = "42".to_string();
    app.transfer_panel_state.pending_history_refresh = false;

    app.clear_transfer_history_filters();

    assert!(app.transfer_panel_state.history_account_filter.is_empty());
    assert!(app.transfer_panel_state.history_action_filter.is_empty());
    assert!(app.transfer_panel_state.pending_history_refresh);
}

#[test]
fn probe_chain_status_endpoint_accepts_http_200_response() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let bind = listener.local_addr().expect("listener addr");
    let serve = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept probe connection");
        let mut request = [0_u8; 512];
        let _ = stream.read(&mut request);
        let _ = stream.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\nConnection: close\r\n\r\n{\"ok\":true}",
        );
    });

    probe_chain_status_endpoint(bind.to_string().as_str()).expect("probe should pass");
    serve.join().expect("server thread should finish");
}

#[test]
fn probe_chain_status_endpoint_reports_connect_failure() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind temp listener");
    let bind = listener.local_addr().expect("listener addr").to_string();
    drop(listener);

    let err = probe_chain_status_endpoint(bind.as_str()).expect_err("probe should fail");
    assert!(err.contains("connect chain status server failed"));
}

#[test]
fn check_provider_loopback_http_provider_accepts_info_and_health_responses() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let bind = listener.local_addr().expect("listener addr");
    let serve = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept probe connection");
            let mut request = [0_u8; 1024];
            let bytes = stream.read(&mut request).expect("read request");
            let request_text = String::from_utf8_lossy(&request[..bytes]);
            let body = if request_text.contains("GET /v1/provider/info") {
                r#"{"provider_id":"openclaw-local","name":"OpenClaw","version":"0.1.0","protocol_version":"v1","capabilities":["decision","feedback"],"supported_action_sets":["phase1_low_frequency"]}"#
            } else {
                r#"{"ok":true,"status":"ready","uptime_ms":42,"last_error":null,"queue_depth":0}"#
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });

    let snapshot =
        check_provider_loopback_http_provider(format!("http://{}", bind).as_str(), None, 200)
            .expect("provider check should pass");
    assert_eq!(snapshot.provider_id, "openclaw-local");
    assert_eq!(snapshot.name, "OpenClaw");
    assert_eq!(snapshot.version, "0.1.0");
    assert_eq!(snapshot.protocol_version, "v1");
    assert_eq!(
        snapshot.compatibility_status,
        ProviderCompatibilityStatus::Ready
    );
    assert_eq!(
        snapshot.capabilities,
        vec!["decision".to_string(), "feedback".to_string()]
    );
    assert_eq!(
        snapshot.supported_action_sets,
        vec!["phase1_low_frequency".to_string()]
    );
    assert_eq!(snapshot.status, "ready");
    assert_eq!(snapshot.queue_depth, Some(0));
    assert_eq!(snapshot.last_error, None);
    assert_eq!(snapshot.fallback_reason, None);
    assert!(snapshot.info_latency_ms <= snapshot.total_latency_ms);
    assert!(snapshot.health_latency_ms <= snapshot.total_latency_ms);
    serve.join().expect("server thread should finish");
}

#[test]
fn check_provider_loopback_http_provider_reports_incompatible_supported_actions() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let bind = listener.local_addr().expect("listener addr");
    let serve = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept probe connection");
            let mut request = [0_u8; 1024];
            let bytes = stream.read(&mut request).expect("read request");
            let request_text = String::from_utf8_lossy(&request[..bytes]);
            let body = if request_text.contains("GET /v1/provider/info") {
                r#"{"provider_id":"openclaw-local","name":"OpenClaw","version":"0.1.0","protocol_version":"v1","capabilities":["decision","feedback"],"supported_action_sets":["wait","move_agent"]}"#
            } else {
                r#"{"ok":true,"status":"ready","uptime_ms":42,"last_error":null,"queue_depth":0}"#
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });

    let snapshot =
        check_provider_loopback_http_provider(format!("http://{}", bind).as_str(), None, 200)
            .expect("provider check should still return snapshot");
    assert_eq!(
        snapshot.compatibility_status,
        ProviderCompatibilityStatus::Incompatible
    );
    assert_eq!(
        snapshot.fallback_reason.as_deref(),
        Some("missing_supported_actions:wait_ticks,speak_to_nearby,inspect_target,simple_interact")
    );
    serve.join().expect("server thread should finish");
}

#[test]
fn check_provider_loopback_http_provider_marks_unhealthy_provider_as_degraded() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let bind = listener.local_addr().expect("listener addr");
    let serve = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept probe connection");
            let mut request = [0_u8; 1024];
            let bytes = stream.read(&mut request).expect("read request");
            let request_text = String::from_utf8_lossy(&request[..bytes]);
            let body = if request_text.contains("GET /v1/provider/info") {
                r#"{"provider_id":"openclaw-local","name":"OpenClaw","version":"0.1.0","protocol_version":"v1","capabilities":["decision","feedback"],"supported_action_sets":["phase1_low_frequency"]}"#
            } else {
                r#"{"ok":false,"status":null,"uptime_ms":42,"last_error":null,"queue_depth":0}"#
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });

    let snapshot =
        check_provider_loopback_http_provider(format!("http://{}", bind).as_str(), None, 200)
            .expect("provider check should still return degraded snapshot");
    assert_eq!(
        snapshot.compatibility_status,
        ProviderCompatibilityStatus::Degraded
    );
    assert_eq!(
        snapshot.fallback_reason.as_deref(),
        Some("provider_health_unhealthy:not_ok")
    );
    serve.join().expect("server thread should finish");
}

#[test]
fn collect_required_config_issues_reports_openclaw_specific_fields() {
    let config = LaunchConfig {
        agent_decision_source: "provider_backed".to_string(),
        agent_provider_url: String::new(),
        openclaw_auto_discover: false,
        agent_provider_connect_timeout_ms: "0".to_string(),
        ..LaunchConfig::default()
    };
    let issues = collect_required_config_issues(&config);
    assert!(issues.contains(&ConfigIssue::OpenClawBaseUrlRequired));
    assert!(issues.contains(&ConfigIssue::OpenClawConnectTimeoutMsInvalid));
    assert!(!issues.contains(&ConfigIssue::OpenClawAgentProfileRequired));
}

#[test]
fn collect_required_config_issues_rejects_no_llm_playability_config() {
    let config = LaunchConfig {
        llm_enabled: false,
        ..LaunchConfig::default()
    };
    let issues = collect_required_config_issues(&config);
    assert!(issues.contains(&ConfigIssue::LlmRequired));
}

#[test]
fn collect_required_config_issues_rejects_non_loopback_openclaw_base_url() {
    let config = LaunchConfig {
        agent_decision_source: "provider_backed".to_string(),
        agent_provider_url: "http://192.168.0.5:5841".to_string(),
        ..LaunchConfig::default()
    };
    let issues = collect_required_config_issues(&config);
    assert!(issues.contains(&ConfigIssue::OpenClawBaseUrlLoopbackRequired));
}

#[test]
fn collect_required_config_issues_requires_openclaw_agent_profile() {
    let config = LaunchConfig {
        agent_decision_source: "provider_backed".to_string(),
        agent_provider_profile: String::new(),
        ..LaunchConfig::default()
    };
    let issues = collect_required_config_issues(&config);
    assert!(issues.contains(&ConfigIssue::OpenClawAgentProfileRequired));
}
#[test]
fn collect_required_config_issues_reports_missing_required_fields() {
    let config = LaunchConfig {
        scenario: "".to_string(),
        live_bind: "127.0.0.1".to_string(),
        web_bind: "127.0.0.1".to_string(),
        viewer_host: "".to_string(),
        viewer_port: "0".to_string(),
        viewer_static_dir: "".to_string(),
        launcher_bin: "".to_string(),
        chain_enabled: true,
        chain_status_bind: "127.0.0.1".to_string(),
        chain_node_id: "".to_string(),
        chain_node_role: "invalid".to_string(),
        chain_node_tick_ms: "0".to_string(),
        chain_node_validators: "node-a".to_string(),
        chain_runtime_bin: "".to_string(),
        ..LaunchConfig::default()
    };

    let issues = collect_required_config_issues(&config);
    assert!(issues.contains(&ConfigIssue::ScenarioRequired));
    assert!(issues.contains(&ConfigIssue::LiveBindInvalid));
    assert!(issues.contains(&ConfigIssue::WebBindInvalid));
    assert!(issues.contains(&ConfigIssue::ViewerHostRequired));
    assert!(issues.contains(&ConfigIssue::ViewerPortInvalid));
    assert!(issues.contains(&ConfigIssue::ViewerStaticDirRequired));
    assert!(issues.contains(&ConfigIssue::LauncherBinRequired));
}

#[test]
fn collect_chain_required_config_issues_reports_missing_required_fields() {
    let config = LaunchConfig {
        chain_enabled: true,
        chain_runtime_bin: "".to_string(),
        chain_status_bind: "127.0.0.1".to_string(),
        chain_node_id: "".to_string(),
        chain_node_role: "invalid".to_string(),
        chain_p2p_user_mode: "invalid".to_string(),
        chain_node_tick_ms: "0".to_string(),
        chain_pos_slot_duration_ms: "0".to_string(),
        chain_pos_ticks_per_slot: "4".to_string(),
        chain_pos_proposal_tick_phase: "4".to_string(),
        chain_pos_slot_clock_genesis_unix_ms: "oops".to_string(),
        chain_pos_max_past_slot_lag: "-1".to_string(),
        chain_node_validators: "node-a".to_string(),
        ..LaunchConfig::default()
    };

    let issues = collect_chain_required_config_issues(&config);
    assert!(issues.contains(&ConfigIssue::ChainRuntimeBinRequired));
    assert!(issues.contains(&ConfigIssue::ChainStatusBindInvalid));
    assert!(issues.contains(&ConfigIssue::ChainNodeIdRequired));
    assert!(issues.contains(&ConfigIssue::ChainRoleInvalid));
    assert!(issues.contains(&ConfigIssue::ChainP2pUserModeInvalid));
    assert!(issues.contains(&ConfigIssue::ChainTickMsInvalid));
    assert!(issues.contains(&ConfigIssue::ChainPosSlotDurationMsInvalid));
    assert!(issues.contains(&ConfigIssue::ChainPosProposalTickPhaseOutOfRange));
    assert!(issues.contains(&ConfigIssue::ChainPosSlotClockGenesisUnixMsInvalid));
    assert!(issues.contains(&ConfigIssue::ChainPosMaxPastSlotLagInvalid));
    assert!(issues.contains(&ConfigIssue::ChainValidatorsInvalid));
}
#[test]
fn collect_required_config_issues_accepts_valid_required_fields() {
    let launcher_bin = std::env::current_exe()
        .expect("current exe")
        .to_string_lossy()
        .to_string();
    let config = LaunchConfig {
        scenario: "llm_bootstrap".to_string(),
        live_bind: "127.0.0.1:5023".to_string(),
        web_bind: "127.0.0.1:5011".to_string(),
        viewer_host: "127.0.0.1".to_string(),
        viewer_port: "4173".to_string(),
        viewer_static_dir: ".".to_string(),
        chain_enabled: false,
        launcher_bin,
        ..LaunchConfig::default()
    };

    let issues = collect_required_config_issues(&config);
    assert!(issues.is_empty());
}

#[test]
fn collect_required_config_issues_accepts_bundle_relative_web_path_from_launcher_bin() {
    let bundle_root = make_temp_dir("client_launcher_bundle_relative");
    let bundle_bin = bundle_root.join("bin");
    let launcher_bin = bundle_bin.join("oasis7_game_launcher");
    let bundle_web = bundle_root.join("web");
    fs::create_dir_all(&bundle_bin).expect("create bundle bin dir");
    fs::create_dir_all(&bundle_web).expect("create bundle web dir");
    fs::write(&launcher_bin, b"#!/bin/sh\n").expect("write fake launcher bin");

    let config = LaunchConfig {
        scenario: "llm_bootstrap".to_string(),
        live_bind: "127.0.0.1:5023".to_string(),
        web_bind: "127.0.0.1:5011".to_string(),
        viewer_host: "127.0.0.1".to_string(),
        viewer_port: "4173".to_string(),
        viewer_static_dir: "web".to_string(),
        chain_enabled: false,
        launcher_bin: launcher_bin.to_string_lossy().to_string(),
        ..LaunchConfig::default()
    };

    let issues = collect_required_config_issues(&config);
    assert!(!issues.contains(&ConfigIssue::ViewerStaticDirMissing));
    assert!(!issues.contains(&ConfigIssue::LauncherBinMissing));

    let _ = fs::remove_dir_all(bundle_root);
}

#[test]
fn collect_chain_required_config_issues_accepts_valid_required_fields() {
    let chain_runtime_bin = std::env::current_exe()
        .expect("current exe")
        .to_string_lossy()
        .to_string();
    let config = LaunchConfig {
        chain_enabled: true,
        chain_runtime_bin,
        chain_status_bind: "127.0.0.1:6121".to_string(),
        chain_node_id: "chain-node-a".to_string(),
        chain_world_id: "live-chain-a".to_string(),
        chain_node_role: "sequencer".to_string(),
        chain_node_tick_ms: "200".to_string(),
        chain_node_validators: "node-a:100".to_string(),
        ..LaunchConfig::default()
    };

    let issues = collect_chain_required_config_issues(&config);
    assert!(issues.is_empty());
}

#[test]
fn collect_chain_required_config_issues_requires_public_entry_confirmation() {
    let issues = collect_chain_required_config_issues(&LaunchConfig {
        chain_enabled: true,
        chain_runtime_bin: std::env::current_exe()
            .expect("current exe")
            .to_string_lossy()
            .to_string(),
        chain_status_bind: "127.0.0.1:6121".to_string(),
        chain_node_id: "chain-node-a".to_string(),
        chain_node_role: "sequencer".to_string(),
        chain_p2p_user_mode: "public_entry".to_string(),
        chain_p2p_accept_public_entry: false,
        chain_node_validators: "node-a:100".to_string(),
        ..LaunchConfig::default()
    });
    assert!(issues.contains(&ConfigIssue::ChainPublicEntryConfirmationRequired));
}

#[test]
fn build_chain_runtime_args_requires_public_entry_confirmation() {
    let err = build_chain_runtime_args(&LaunchConfig {
        chain_enabled: true,
        chain_runtime_bin: "/tmp/oasis7_chain_runtime".to_string(),
        chain_status_bind: "127.0.0.1:6121".to_string(),
        chain_node_id: "chain-node-a".to_string(),
        chain_node_role: "storage".to_string(),
        chain_p2p_user_mode: "public_entry".to_string(),
        chain_p2p_accept_public_entry: false,
        chain_node_validators: "node-a:100".to_string(),
        ..LaunchConfig::default()
    })
    .expect_err("public entry should require explicit confirmation");
    assert!(err.contains("explicit confirmation"));
}

#[test]
fn apply_web_snapshot_tracks_chain_p2p_status_payload() {
    let mut app = ClientLauncherApp::default();
    let snapshot = WebStateSnapshot {
        status: "idle".to_string(),
        detail: None,
        chain_status: "ready".to_string(),
        chain_detail: None,
        chain_p2p_status: Some(super::WebChainP2pStatus {
            requested_user_mode: "auto_join".to_string(),
            recommended_user_mode: "public_entry".to_string(),
            effective_user_mode: "private_safe".to_string(),
            applied_effective_user_mode: Some("private_safe".to_string()),
            requires_explicit_public_entry_confirmation: true,
            detected_reachability: Some("public".to_string()),
            hole_punch_viability: "viable".to_string(),
            relay_available: false,
            probe_stable: true,
            deployment_mode: "private".to_string(),
            node_role_claim: "validator_core".to_string(),
            rationale: vec![
                "observed_reachability=public".to_string(),
                "public entry confirmation pending".to_string(),
            ],
        }),
        chain_recovery: None,
        game_url: "http://127.0.0.1:4173/".to_string(),
        config: app.config.clone(),
        logs: vec![],
    };

    app.apply_web_snapshot(snapshot);
    let status = app
        .chain_p2p_status
        .clone()
        .expect("p2p status should exist");
    assert_eq!(status.recommended_user_mode, "public_entry");
    assert!(status.requires_explicit_public_entry_confirmation);
}

#[test]
fn issue_field_ids_maps_phase_out_of_range_to_related_fields() {
    let ids = issue_field_ids(ConfigIssue::ChainPosProposalTickPhaseOutOfRange);
    assert_eq!(
        ids,
        &["chain_pos_ticks_per_slot", "chain_pos_proposal_tick_phase"]
    );
}

fn make_temp_dir(label: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    path.push(format!(
        "oasis7_client_launcher_test_{label}_{}_{}",
        std::process::id(),
        stamp
    ));
    fs::create_dir_all(&path).expect("create temp dir");
    path
}
