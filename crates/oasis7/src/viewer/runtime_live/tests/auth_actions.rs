use super::*;
use std::collections::BTreeMap;
use std::io::Read;
use std::sync::{Arc, Mutex};

#[path = "auth_actions_authoritative_recovery.rs"]
mod authoritative_recovery_tests;
#[path = "auth_actions_claims.rs"]
mod claim_action_tests;

#[test]
fn runtime_agent_chat_script_mode_requires_llm_mode() {
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Script),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let err = server
        .handle_agent_chat(crate::viewer::AgentChatRequest {
            agent_id,
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello".to_string(),
            intent_tick: None,
            intent_seq: None,
        })
        .expect_err("script mode should reject chat");
    assert_eq!(err.code, "llm_mode_required");
}

#[test]
fn runtime_gameplay_action_requires_auth() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let err = server
        .handle_gameplay_action(crate::viewer::GameplayActionRequest {
            action_id: "build_factory_smelter_mk1".to_string(),
            target_agent_id: agent_id,
            actor_agent_id: None,
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
        })
        .expect_err("missing auth should fail");
    assert_eq!(err.code, "auth_proof_required");
}

#[test]
fn runtime_gameplay_action_script_mode_requires_llm_mode() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(87);
    let request = signed_gameplay_action_request(
        crate::viewer::GameplayActionRequest {
            action_id: "build_factory_smelter_mk1".to_string(),
            target_agent_id: agent_id,
            actor_agent_id: None,
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
        },
        87,
        public_key.as_str(),
        private_key.as_str(),
    );
    let err = server
        .handle_gameplay_action(request)
        .expect_err("script mode should reject gameplay actions");
    assert_eq!(err.code, "llm_mode_required");
}

#[test]
fn runtime_step_control_reports_blocked_without_llm_mode() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let (mut writer, client) = test_writer_pair();
    let mut session = RuntimeLiveSession::new();

    server
        .apply_control_mode(
            ViewerControl::Step { count: 1 },
            Some(1),
            &mut session,
            &mut writer,
        )
        .expect("control handled");
    writer.flush().expect("flush response");

    let ack =
        read_control_completion_ack(&client, Duration::from_millis(250)).expect("blocked step ack");
    assert_eq!(ack.status, ControlCompletionStatus::Blocked);
    assert_eq!(ack.error_code.as_deref(), Some("llm_mode_required"));
    assert!(ack
        .error_message
        .as_deref()
        .is_some_and(|message| message.contains("--llm")));

    let feedback = server
        .latest_player_gameplay_feedback
        .as_ref()
        .expect("blocked feedback recorded");
    assert_eq!(feedback.stage, "blocked");
    assert!(feedback
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("--llm")));
}

#[test]
fn runtime_step_control_reports_llm_init_failed_when_provider_unavailable() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    std::env::remove_var(crate::simulator::ENV_LLM_MODEL);
    std::env::remove_var(crate::simulator::ENV_LLM_BASE_URL);
    std::env::remove_var(crate::simulator::ENV_LLM_API_KEY);

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let (mut writer, client) = test_writer_pair();
    let mut session = RuntimeLiveSession::new();

    server
        .apply_control_mode(
            ViewerControl::Step { count: 1 },
            Some(7),
            &mut session,
            &mut writer,
        )
        .expect("control handled");
    writer.flush().expect("flush response");

    let ack = read_control_completion_ack(&client, Duration::from_millis(250))
        .expect("blocked init failure ack");
    assert_eq!(ack.status, ControlCompletionStatus::Blocked);
    assert_eq!(ack.error_code.as_deref(), Some("llm_init_failed"));
    assert!(ack
        .error_message
        .as_deref()
        .is_some_and(|message| message.contains("configured and reachable LLM provider")));

    let feedback = server
        .latest_player_gameplay_feedback
        .as_ref()
        .expect("blocked feedback recorded");
    assert_eq!(feedback.stage, "blocked");
    assert!(feedback
        .reason
        .as_deref()
        .is_some_and(|reason| { reason.contains("configured and reachable LLM provider") }));
}

#[test]
fn runtime_background_play_stops_when_llm_access_is_unavailable() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    std::env::remove_var(crate::simulator::ENV_LLM_MODEL);
    std::env::remove_var(crate::simulator::ENV_LLM_BASE_URL);
    std::env::remove_var(crate::simulator::ENV_LLM_API_KEY);

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let baseline_time = server.world.state().time;
    let (mut writer, _client) = test_writer_pair();
    let mut session = RuntimeLiveSession::new();
    session.playing = true;

    server
        .advance_runtime(&mut session, &mut writer, "play", 1, None, false)
        .expect("play loop handled");

    assert!(
        !session.playing,
        "background play should stop without LLM access"
    );
    assert_eq!(
        server.world.state().time,
        baseline_time,
        "background play must not advance world time without active LLM access"
    );
    let feedback = server
        .latest_player_gameplay_feedback
        .as_ref()
        .expect("blocked feedback recorded");
    assert_eq!(feedback.action, "play");
    assert_eq!(feedback.stage, "blocked");
    assert!(feedback
        .reason
        .as_deref()
        .is_some_and(|reason| { reason.contains("configured and reachable LLM provider") }));
}

#[test]
fn runtime_background_play_tolerates_transient_llm_failure_after_confirmed_progress() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    let request_count = Arc::new(Mutex::new(0_usize));
    let base_url = spawn_runtime_live_mock_http_server(2, {
        let request_count = Arc::clone(&request_count);
        move |request| {
            let mut count = request_count.lock().expect("request count lock");
            *count += 1;
            match (request.method.as_str(), request.path.as_str(), *count) {
                ("POST", "/v1/world-simulator/decision", 1) => {
                    let decoded: crate::simulator::DecisionRequest =
                        serde_json::from_slice(request.body.as_slice())
                            .expect("decode decision request");
                    let response = crate::simulator::DecisionResponse {
                        decision: crate::simulator::ProviderDecision::Act {
                            action_ref: "speak_to_nearby".to_string(),
                            action: crate::simulator::Action::SpeakToNearby {
                                agent_id: decoded.observation.agent_id,
                                message: "runtime-live play ok".to_string(),
                                target_agent_id: None,
                            },
                        },
                        provider_error: None,
                        diagnostics: crate::simulator::ProviderDiagnostics::default(),
                        trace_payload: crate::simulator::ProviderTraceEnvelope::default(),
                        memory_write_intents: Vec::new(),
                    };
                    MockHttpResponse {
                        status_code: 200,
                        body: serde_json::to_string(&response).expect("encode decision response"),
                    }
                }
                ("POST", "/v1/world-simulator/decision", _) => MockHttpResponse {
                    status_code: 503,
                    body: serde_json::json!({
                        "ok": false,
                        "error": "provider temporarily unavailable"
                    })
                    .to_string(),
                },
                _ => MockHttpResponse {
                    status_code: 404,
                    body: serde_json::json!({"ok": false, "error": "not_found"}).to_string(),
                },
            }
        }
    });
    std::env::set_var(VIEWER_AGENT_PROVIDER_MODE_ENV, "provider_loopback_http");
    std::env::set_var(VIEWER_AGENT_PROVIDER_URL_ENV, base_url);
    std::env::set_var(VIEWER_AGENT_PROVIDER_PROFILE_ENV, "oasis7_p0_low_freq_npc");
    std::env::set_var(VIEWER_AGENT_EXECUTION_LANE_ENV, "player_parity");

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let (mut writer, _client) = test_writer_pair();
    let mut session = RuntimeLiveSession::new();
    session.playing = true;

    server
        .advance_runtime(&mut session, &mut writer, "play", 1, None, false)
        .expect("first background play tick advances");
    let advanced_time = server.world.state().time;
    assert!(
        server.confirmed_player_gameplay_progress_time.is_some(),
        "successful background play should confirm gameplay progress"
    );

    server
        .advance_runtime(&mut session, &mut writer, "play", 1, None, false)
        .expect("transient provider failure should be tolerated");

    assert!(
        session.playing,
        "background play should remain active during transient failure budget"
    );
    assert_eq!(session.transient_play_failures, 1);
    assert_eq!(
        server.world.state().time,
        advanced_time,
        "transient failure should not advance world time"
    );
    let feedback = server
        .latest_player_gameplay_feedback
        .as_ref()
        .expect("blocked feedback recorded");
    assert_eq!(feedback.action, "play");
    assert_eq!(feedback.stage, "blocked");
    assert!(feedback
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains("provider temporarily unavailable")));
    clear_runtime_provider_env();
}

#[test]
fn runtime_step_control_surfaces_runtime_failure_as_blocked_ack() {
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Script),
    )
    .expect("runtime server");
    let missing_agent = "missing-agent".to_string();

    let (mut writer, client) = test_writer_pair();
    let mut session = RuntimeLiveSession::new();

    server
        .block_runtime_control(
            &mut session,
            &mut writer,
            "step",
            "runtime step aborted because world advance failed",
            ViewerRuntimeLiveServerError::Runtime(crate::runtime::WorldError::AgentNotFound {
                agent_id: missing_agent.clone(),
            }),
            Some(19),
            1,
            0,
            true,
        )
        .expect("control handled");
    writer.flush().expect("flush response");

    let ack =
        read_control_completion_ack(&client, Duration::from_millis(250)).expect("blocked step ack");
    assert_eq!(ack.status, ControlCompletionStatus::Blocked);
    assert_eq!(ack.error_code.as_deref(), Some("agent_not_found"));
    assert_eq!(ack.delta_logical_time, 1);
    assert!(ack
        .error_message
        .as_deref()
        .is_some_and(|message| message.contains(missing_agent.as_str())));

    let feedback = server
        .latest_player_gameplay_feedback
        .as_ref()
        .expect("blocked feedback recorded");
    assert_eq!(feedback.stage, "blocked");
    assert_eq!(feedback.delta_logical_time, 1);
    assert!(feedback
        .reason
        .as_deref()
        .is_some_and(|reason| reason.contains(missing_agent.as_str())));
    assert!(feedback
        .hint
        .as_deref()
        .is_some_and(|hint| hint.contains("restore the missing agent")));
}

#[derive(Debug, Clone)]
struct RecordedHttpRequest {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

#[derive(Debug, Clone)]
struct MockHttpResponse {
    status_code: u16,
    body: String,
}

#[test]
fn runtime_step_control_requests_llm_decision_and_advances_with_provider_backed_loopback() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    let recorded = Arc::new(Mutex::new(Vec::<RecordedHttpRequest>::new()));
    let base_url = spawn_runtime_live_mock_http_server(1, {
        let recorded = Arc::clone(&recorded);
        move |request| {
            recorded
                .lock()
                .expect("recorded lock")
                .push(request.clone());
            match (request.method.as_str(), request.path.as_str()) {
                ("POST", "/v1/world-simulator/decision") => {
                    let decoded: crate::simulator::DecisionRequest =
                        serde_json::from_slice(request.body.as_slice())
                            .expect("decode decision request");
                    let response = crate::simulator::DecisionResponse {
                        decision: crate::simulator::ProviderDecision::Act {
                            action_ref: "speak_to_nearby".to_string(),
                            action: crate::simulator::Action::SpeakToNearby {
                                agent_id: decoded.observation.agent_id,
                                message: "runtime-live step ok".to_string(),
                                target_agent_id: None,
                            },
                        },
                        provider_error: None,
                        diagnostics: crate::simulator::ProviderDiagnostics::default(),
                        trace_payload: crate::simulator::ProviderTraceEnvelope::default(),
                        memory_write_intents: Vec::new(),
                    };
                    MockHttpResponse {
                        status_code: 200,
                        body: serde_json::to_string(&response).expect("encode decision response"),
                    }
                }
                _ => MockHttpResponse {
                    status_code: 404,
                    body: serde_json::json!({"ok": false, "error": "not_found"}).to_string(),
                },
            }
        }
    });
    std::env::set_var(VIEWER_AGENT_PROVIDER_MODE_ENV, "provider_loopback_http");
    std::env::set_var(VIEWER_AGENT_PROVIDER_URL_ENV, base_url);
    std::env::set_var(VIEWER_AGENT_PROVIDER_PROFILE_ENV, "oasis7_p0_low_freq_npc");
    std::env::set_var(VIEWER_AGENT_EXECUTION_LANE_ENV, "player_parity");

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let baseline_time = server.world.state().time;
    let (mut writer, client) = test_writer_pair();
    let mut session = RuntimeLiveSession::new();

    server
        .apply_control_mode(
            ViewerControl::Step { count: 1 },
            Some(9),
            &mut session,
            &mut writer,
        )
        .expect("control handled");
    writer.flush().expect("flush response");

    let ack = read_control_completion_ack(&client, Duration::from_millis(500))
        .expect("step should advance with provider-backed decision");
    assert_eq!(ack.status, ControlCompletionStatus::Advanced);
    assert!(
        ack.delta_logical_time > 0 || ack.delta_event_seq > 0,
        "step should report logical or event progress"
    );
    assert!(
        server.world.state().time > baseline_time,
        "step should advance runtime time after requesting provider decision"
    );
    let feedback = server
        .latest_player_gameplay_feedback
        .as_ref()
        .expect("recent feedback recorded");
    assert_eq!(feedback.stage, "completed_advanced");

    let recorded = recorded.lock().expect("recorded lock");
    assert_eq!(
        recorded.len(),
        1,
        "step should request one provider decision"
    );
    assert_eq!(recorded[0].path, "/v1/world-simulator/decision");
    assert_eq!(
        recorded[0].headers.get("content-type").map(String::as_str),
        Some("application/json")
    );
    clear_runtime_provider_env();
}

#[test]
fn runtime_agent_chat_requires_explicit_session_registration() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(24);
    let request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id,
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello".to_string(),
            intent_tick: Some(1),
            intent_seq: Some(2),
        },
        1,
        public_key.as_str(),
        private_key.as_str(),
    );
    let err = server
        .handle_agent_chat(request)
        .expect_err("session register should be required before agent chat");
    assert_eq!(err.code, "session_not_found");
}

fn spawn_runtime_live_mock_http_server<F>(expected_connections: usize, handler: F) -> String
where
    F: Fn(RecordedHttpRequest) -> MockHttpResponse + Send + Sync + 'static,
{
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind mock http server");
    let bind = listener.local_addr().expect("listener addr");
    let handler = Arc::new(handler);
    std::thread::spawn(move || {
        for _ in 0..expected_connections {
            let (mut stream, _) = listener.accept().expect("accept mock request");
            let request = read_runtime_live_http_request(&mut stream);
            let response = handler(request);
            write_runtime_live_json_response(
                &mut stream,
                response.status_code,
                response.body.as_str(),
            );
        }
    });
    format!("http://{}", bind)
}

fn read_runtime_live_http_request(stream: &mut std::net::TcpStream) -> RecordedHttpRequest {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    let mut header_end = None;
    let mut content_length = 0_usize;

    loop {
        let bytes = stream.read(&mut chunk).expect("read request bytes");
        if bytes == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..bytes]);
        if header_end.is_none() {
            header_end = find_runtime_live_header_terminator(buffer.as_slice());
            if let Some(boundary) = header_end {
                let header = std::str::from_utf8(&buffer[..boundary]).expect("utf8 header");
                content_length = header
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        if name.eq_ignore_ascii_case("content-length") {
                            value.trim().parse::<usize>().ok()
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);
            }
        }
        if let Some(boundary) = header_end {
            if buffer.len() >= boundary + 4 + content_length {
                break;
            }
        }
    }

    let boundary = header_end.expect("header boundary");
    let header = std::str::from_utf8(&buffer[..boundary]).expect("utf8 header");
    let mut lines = header.lines();
    let request_line = lines.next().expect("request line");
    let mut request_line_parts = request_line.split_whitespace();
    let method = request_line_parts.next().expect("method").to_string();
    let path = request_line_parts.next().expect("path").to_string();
    let mut headers = BTreeMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    let body = buffer[(boundary + 4)..(boundary + 4 + content_length)].to_vec();

    RecordedHttpRequest {
        method,
        path,
        headers,
        body,
    }
}

fn find_runtime_live_header_terminator(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn write_runtime_live_json_response(
    stream: &mut std::net::TcpStream,
    status_code: u16,
    body: &str,
) {
    let status_text = match status_code {
        200 => "OK",
        404 => "Not Found",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {status_code} {status_text}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(response.as_bytes())
        .expect("write mock response");
}

#[test]
fn runtime_session_register_rejects_same_player_binding_to_second_agent() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::TwoBases)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_ids: Vec<_> = server
        .world
        .state()
        .agents
        .keys()
        .cloned()
        .take(2)
        .collect();
    assert!(
        agent_ids.len() >= 2,
        "expected at least two agents in two_bases scenario"
    );
    let (public_key, private_key) = test_signer(25);

    let first_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_ids[0].as_str()),
        1,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        first_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );
    assert_eq!(first_ack.agent_id.as_deref(), Some(agent_ids[0].as_str()));

    let conflict_request = signed_session_register_request(
        crate::viewer::AuthoritativeSessionRegisterRequest {
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
            requested_agent_id: Some(agent_ids[1].clone()),
            force_rebind: false,
        },
        2,
        public_key.as_str(),
        private_key.as_str(),
    );
    let err = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RegisterSession {
            request: conflict_request,
        })
        .expect_err("same player should not silently rebind to another agent");
    assert_eq!(err.code, "player_bind_failed");
    assert!(err.message.contains("explicit rebind required"));
}

#[test]
fn runtime_session_register_allows_same_player_rebind_with_force_rebind() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::TwoBases)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_ids: Vec<_> = server
        .world
        .state()
        .agents
        .keys()
        .cloned()
        .take(2)
        .collect();
    assert!(
        agent_ids.len() >= 2,
        "expected at least two agents in two_bases scenario"
    );
    let (public_key, private_key) = test_signer(26);

    let first_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_ids[0].as_str()),
        1,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        first_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );
    assert_eq!(first_ack.agent_id.as_deref(), Some(agent_ids[0].as_str()));

    let second_ack = register_runtime_session_with_options(
        &mut server,
        "player-a",
        Some(agent_ids[1].as_str()),
        true,
        2,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        second_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );
    assert_eq!(second_ack.agent_id.as_deref(), Some(agent_ids[1].as_str()));
    assert_eq!(
        server.llm_sidecar.bound_agent_for_player("player-a"),
        Some(agent_ids[1].as_str())
    );
    assert_eq!(
        server
            .llm_sidecar
            .agent_player_bindings
            .get(agent_ids[0].as_str()),
        None
    );
}

#[test]
fn runtime_agent_chat_provider_mode_reports_unsupported() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    std::env::set_var(VIEWER_AGENT_PROVIDER_MODE_ENV, "provider_loopback_http");
    std::env::set_var(VIEWER_AGENT_PROVIDER_URL_ENV, "http://127.0.0.1:5841");
    std::env::set_var(VIEWER_AGENT_PROVIDER_PROFILE_ENV, "oasis7_p0_low_freq_npc");
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let err = server
        .handle_agent_chat(crate::viewer::AgentChatRequest {
            agent_id,
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello".to_string(),
            intent_tick: None,
            intent_seq: None,
        })
        .expect_err("provider mode should reject chat");
    assert_eq!(err.code, "agent_provider_chat_unsupported");
    clear_runtime_provider_env();
}

#[test]
fn runtime_agent_chat_replay_returns_idempotent_ack() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(21);
    let request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello".to_string(),
            intent_tick: Some(7),
            intent_seq: Some(5),
        },
        5,
        public_key.as_str(),
        private_key.as_str(),
    );
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        4,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );

    let first = server
        .handle_agent_chat(request.clone())
        .expect("first request accepted");
    assert_eq!(first.intent_tick, Some(7));
    assert_eq!(first.intent_seq, Some(5));
    assert!(!first.idempotent_replay);

    let replay = server
        .handle_agent_chat(request)
        .expect("replay request accepted");
    assert_eq!(replay.agent_id, first.agent_id);
    assert_eq!(replay.accepted_at_tick, first.accepted_at_tick);
    assert_eq!(replay.message_len, first.message_len);
    assert_eq!(replay.player_id, first.player_id);
    assert_eq!(replay.intent_tick, first.intent_tick);
    assert_eq!(replay.intent_seq, first.intent_seq);
    assert!(replay.idempotent_replay);
    assert_eq!(
        server
            .llm_sidecar
            .player_auth_last_nonce
            .get("player-a")
            .copied(),
        Some(5)
    );
}

#[test]
fn runtime_agent_chat_echo_env_enqueues_agent_spoke_virtual_event() {
    let _guard = lock_test_llm_env();
    std::env::set_var(RUNTIME_AGENT_CHAT_ECHO_ENV, "1");
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(31);
    let request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello runtime echo".to_string(),
            intent_tick: Some(9),
            intent_seq: Some(31),
        },
        31,
        public_key.as_str(),
        private_key.as_str(),
    );
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        30,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );

    let ack = server.handle_agent_chat(request).expect("chat accepted");
    assert_eq!(ack.agent_id, agent_id);

    let events: Vec<_> = server.pending_virtual_events.drain(..).collect();
    assert!(events.iter().any(|event| matches!(
        &event.kind,
        crate::simulator::WorldEventKind::AgentSpoke { agent_id: event_agent_id, message, .. }
            if event_agent_id == &agent_id && message == "[qa-echo] hello runtime echo"
    )));
}

#[test]
fn runtime_agent_chat_echo_env_accepts_chat_without_llm_runner_config() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    std::env::set_var(RUNTIME_AGENT_CHAT_ECHO_ENV, "1");
    std::env::remove_var(crate::simulator::ENV_LLM_MODEL);
    std::env::remove_var(crate::simulator::ENV_LLM_BASE_URL);
    std::env::remove_var(crate::simulator::ENV_LLM_API_KEY);
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(33);
    let request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello runtime echo without llm config".to_string(),
            intent_tick: Some(11),
            intent_seq: Some(33),
        },
        33,
        public_key.as_str(),
        private_key.as_str(),
    );
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        32,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );

    let ack = server.handle_agent_chat(request).expect("chat accepted");
    assert_eq!(ack.agent_id, agent_id);

    let events: Vec<_> = server.pending_virtual_events.drain(..).collect();
    assert!(events.iter().any(|event| matches!(
        &event.kind,
        crate::simulator::WorldEventKind::AgentSpoke { agent_id: event_agent_id, message, .. }
            if event_agent_id == &agent_id && message == "[qa-echo] hello runtime echo without llm config"
    )));
}

#[test]
fn runtime_agent_chat_echo_removed_old_brand_env_is_ignored() {
    let _guard = lock_test_llm_env();
    std::env::set_var(
        removed_old_brand_runtime_live_env("RUNTIME_AGENT_CHAT_ECHO"),
        "1",
    );
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(32);
    let request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello removed old brand runtime echo".to_string(),
            intent_tick: Some(10),
            intent_seq: Some(32),
        },
        32,
        public_key.as_str(),
        private_key.as_str(),
    );
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        31,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );

    let ack = server.handle_agent_chat(request).expect("chat accepted");
    assert_eq!(ack.agent_id, agent_id);

    let events: Vec<_> = server.pending_virtual_events.drain(..).collect();
    assert!(!events.iter().any(|event| matches!(
        &event.kind,
        crate::simulator::WorldEventKind::AgentSpoke { agent_id: event_agent_id, message, .. }
            if event_agent_id == &agent_id && message == "[qa-echo] hello removed old brand runtime echo"
    )));
}
