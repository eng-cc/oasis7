use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;

#[test]
fn parse_options_accepts_openclaw_provider() {
    let options = parse_options(
        [
            "--provider",
            "openclaw_local_http",
            "--scenario-id",
            "P0-002",
            "--benchmark-run-id",
            "run-1",
            "--openclaw-base-url",
            "http://127.0.0.1:5841",
            "--out-dir",
            ".tmp/parity",
        ]
        .into_iter(),
    )
    .expect("parse options");
    assert_eq!(options.provider, BenchProviderKind::OpenclawLocalHttp);
    assert_eq!(options.scenario_id, "P0-002");
    assert_eq!(options.benchmark_run_id, "run-1");
    assert_eq!(
        options.openclaw_base_url.as_deref(),
        Some("http://127.0.0.1:5841")
    );
    assert_eq!(
        options.openclaw_agent_profile,
        DEFAULT_OPENCLAW_AGENT_PROFILE
    );
}

#[test]
fn parse_options_rejects_openclaw_without_base_url() {
    let err = parse_options(
        [
            "--provider",
            "openclaw_local_http",
            "--benchmark-run-id",
            "run-1",
        ]
        .into_iter(),
    )
    .expect_err("missing base url should fail");
    assert!(err.contains("--openclaw-base-url"));
}

#[test]
fn parse_options_accepts_custom_openclaw_agent_profile() {
    let options = parse_options(
        [
            "--provider",
            "openclaw_local_http",
            "--benchmark-run-id",
            "run-2",
            "--openclaw-base-url",
            "http://127.0.0.1:5841",
            "--openclaw-agent-profile",
            "oasis7_p1_memory_loop",
        ]
        .into_iter(),
    )
    .expect("parse custom profile");
    assert_eq!(options.openclaw_agent_profile, "oasis7_p1_memory_loop");
}

#[test]
fn parse_options_defaults_use_real_provider_timeout_budget() {
    let options =
        parse_options(["--benchmark-run-id", "run-defaults"].into_iter()).expect("parse defaults");
    assert_eq!(options.timeout_ms, 15_000);
    assert_eq!(options.openclaw_connect_timeout_ms, 15_000);
}

#[test]
fn builtin_parity_short_term_goal_matches_memory_summary() {
    assert_eq!(
        builtin_parity_short_term_goal("P0-001").as_deref(),
        parity_memory_summary("P0-001")
    );
    assert_eq!(builtin_parity_short_term_goal("unknown"), None);
}

fn sample_patrol_observation() -> Observation {
    Observation {
        time: 7,
        agent_id: "agent-1".to_string(),
        pos: oasis7::geometry::GeoPos {
            x_cm: 0.0,
            y_cm: 0.0,
            z_cm: 0.0,
        },
        self_resources: Default::default(),
        visibility_range_cm: 1_000,
        visible_agents: Vec::new(),
        visible_locations: vec![
            oasis7::simulator::ObservedLocation {
                location_id: "loc-1".to_string(),
                name: "base".to_string(),
                pos: oasis7::geometry::GeoPos {
                    x_cm: 0.0,
                    y_cm: 0.0,
                    z_cm: 0.0,
                },
                profile: Default::default(),
                distance_cm: 0,
            },
            oasis7::simulator::ObservedLocation {
                location_id: "loc-2".to_string(),
                name: "neighbor".to_string(),
                pos: oasis7::geometry::GeoPos {
                    x_cm: 100.0,
                    y_cm: 0.0,
                    z_cm: 0.0,
                },
                profile: Default::default(),
                distance_cm: 100,
            },
        ],
        module_lifecycle: Default::default(),
        module_market: Default::default(),
        power_market: Default::default(),
        social_state: Default::default(),
    }
}

#[test]
fn builtin_parity_guardrail_reroutes_passive_patrol_decision_to_move() {
    let observation = sample_patrol_observation();
    let (decision, note) =
        apply_builtin_parity_guardrail("P0-001", "agent-1", &observation, AgentDecision::Wait);
    assert_eq!(
        decision,
        AgentDecision::Act(Action::MoveAgent {
            agent_id: "agent-1".to_string(),
            to: "loc-2".to_string(),
        })
    );
    assert!(note
        .unwrap_or_default()
        .contains("builtin_parity_guardrail"));
}

#[test]
fn builtin_parity_guardrail_reroutes_non_move_patrol_decision_to_move() {
    let observation = sample_patrol_observation();
    let (decision, note) = apply_builtin_parity_guardrail(
        "P0-001",
        "agent-1",
        &observation,
        AgentDecision::Act(Action::HarvestRadiation {
            agent_id: "agent-1".to_string(),
            max_amount: 3,
        }),
    );
    assert_eq!(
        decision,
        AgentDecision::Act(Action::MoveAgent {
            agent_id: "agent-1".to_string(),
            to: "loc-2".to_string(),
        })
    );
    assert!(note.unwrap_or_default().contains("act:other"));
}

#[test]
fn builtin_parity_guardrail_keeps_valid_move_agent_decision() {
    let observation = sample_patrol_observation();
    let decision = AgentDecision::Act(Action::MoveAgent {
        agent_id: "agent-1".to_string(),
        to: "loc-2".to_string(),
    });
    let (rewritten, note) =
        apply_builtin_parity_guardrail("P0-001", "agent-1", &observation, decision.clone());
    assert_eq!(rewritten, decision);
    assert_eq!(note, None);
}

#[test]
fn parse_options_accepts_openclaw_player_parity_execution_mode() {
    let options = parse_options(
        [
            "--provider",
            "openclaw_local_http",
            "--benchmark-run-id",
            "run-3",
            "--openclaw-base-url",
            "http://127.0.0.1:5841",
            "--execution-mode",
            "player_parity",
        ]
        .into_iter(),
    )
    .expect("parse parity execution mode");
    assert_eq!(options.execution_mode, ProviderExecutionMode::PlayerParity);
}

#[test]
fn parse_options_rejects_builtin_player_parity_execution_mode() {
    let err = parse_options(
        [
            "--provider",
            "builtin",
            "--benchmark-run-id",
            "run-4",
            "--execution-mode",
            "player_parity",
        ]
        .into_iter(),
    )
    .expect_err("builtin parity mode should fail");
    assert!(err.contains("openclaw_local_http"));
}

#[test]
fn scenario_goal_completed_uses_p0_rules() {
    let mut action_kind_counts = BTreeMap::new();
    action_kind_counts.insert("move_agent".to_string(), 3);
    assert!(scenario_goal_completed(
        "P0-001",
        &action_kind_counts,
        &BTreeMap::new(),
        0,
    ));

    action_kind_counts.clear();
    action_kind_counts.insert("simple_interact".to_string(), 1);
    assert!(scenario_goal_completed(
        "P0-004",
        &action_kind_counts,
        &BTreeMap::new(),
        0,
    ));
    assert!(!scenario_goal_completed(
        "P0-004",
        &action_kind_counts,
        &BTreeMap::new(),
        1,
    ));
}

#[test]
fn classify_trace_error_detects_timeout() {
    let trace = AgentDecisionTrace {
        agent_id: "agent-1".to_string(),
        time: 1,
        decision: AgentDecision::Wait,
        llm_input: None,
        llm_output: None,
        llm_error: Some("timeout: provider request timed out".to_string()),
        parse_error: None,
        llm_diagnostics: None,
        llm_effect_intents: Vec::new(),
        llm_effect_receipts: Vec::new(),
        llm_step_trace: Vec::new(),
        llm_prompt_section_trace: Vec::new(),
        llm_chat_messages: Vec::new(),
    };
    assert_eq!(
        classify_trace_error(Some(&trace), None).as_deref(),
        Some("timeout")
    );
}

#[test]
fn prepare_provider_info_captures_openclaw_compatibility_status() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let bind = listener.local_addr().expect("listener addr");
    let serve = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept probe connection");
            let mut request = [0_u8; 1024];
            let bytes = stream.read(&mut request).expect("read request");
            let request_text = String::from_utf8_lossy(&request[..bytes]);
            let body = if request_text.contains("GET /v1/provider/info") {
                r#"{"provider_id":"openclaw-local","name":"OpenClaw","version":"0.1.0","protocol_version":"world-simulator-openclaw-local-http-v1","capabilities":["decision","feedback"],"supported_action_sets":["wait","wait_ticks","move_agent","speak_to_nearby","inspect_target","simple_interact"]}"#
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

    let options = CliOptions {
        provider: BenchProviderKind::OpenclawLocalHttp,
        openclaw_base_url: Some(format!("http://{bind}")),
        ..CliOptions::default()
    };
    let provider = prepare_provider_info(&options).expect("prepare provider");
    assert_eq!(provider.compatibility_status, "ready");
    assert_eq!(provider.fallback_reason, None);
    assert_eq!(
        provider.capabilities,
        vec!["decision".to_string(), "feedback".to_string()]
    );
    assert_eq!(provider.supported_action_sets.len(), 6);
    serve.join().expect("server thread should finish");
}

#[test]
fn prepare_provider_info_marks_incompatible_supported_actions() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let bind = listener.local_addr().expect("listener addr");
    let serve = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept probe connection");
            let mut request = [0_u8; 1024];
            let bytes = stream.read(&mut request).expect("read request");
            let request_text = String::from_utf8_lossy(&request[..bytes]);
            let body = if request_text.contains("GET /v1/provider/info") {
                r#"{"provider_id":"openclaw-local","name":"OpenClaw","version":"0.1.0","protocol_version":"world-simulator-openclaw-local-http-v1","capabilities":["decision","feedback"],"supported_action_sets":["wait","move_agent"]}"#
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

    let options = CliOptions {
        provider: BenchProviderKind::OpenclawLocalHttp,
        openclaw_base_url: Some(format!("http://{bind}")),
        ..CliOptions::default()
    };
    let provider = prepare_provider_info(&options).expect("prepare provider");
    assert_eq!(provider.compatibility_status, "incompatible");
    assert_eq!(
        provider.fallback_reason.as_deref(),
        Some("missing_supported_actions:wait_ticks,speak_to_nearby,inspect_target,simple_interact")
    );
    serve.join().expect("server thread should finish");
}

#[test]
fn prepare_provider_info_marks_missing_capabilities_as_incompatible() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let bind = listener.local_addr().expect("listener addr");
    let serve = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept probe connection");
            let mut request = [0_u8; 1024];
            let bytes = stream.read(&mut request).expect("read request");
            let request_text = String::from_utf8_lossy(&request[..bytes]);
            let body = if request_text.contains("GET /v1/provider/info") {
                r#"{"provider_id":"openclaw-local","name":"OpenClaw","version":"0.1.0","protocol_version":"world-simulator-openclaw-local-http-v1","capabilities":["decision"],"supported_action_sets":["phase1_low_frequency"]}"#
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

    let options = CliOptions {
        provider: BenchProviderKind::OpenclawLocalHttp,
        openclaw_base_url: Some(format!("http://{bind}")),
        ..CliOptions::default()
    };
    let provider = prepare_provider_info(&options).expect("prepare provider");
    assert_eq!(provider.compatibility_status, "incompatible");
    assert_eq!(
        provider.fallback_reason.as_deref(),
        Some("missing_provider_capabilities:feedback")
    );
    serve.join().expect("server thread should finish");
}
