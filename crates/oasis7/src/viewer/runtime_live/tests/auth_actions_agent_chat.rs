use super::*;

fn seed_agent_chat_oc(server: &mut ViewerRuntimeLiveServer, agent_id: &str) {
    server
        .world
        .set_main_token_supply(crate::runtime::MainTokenSupplyState {
            total_supply: 1_000_000,
            circulating_supply: 1_000_000,
            ..crate::runtime::MainTokenSupplyState::default()
        });
    server
        .world
        .set_main_token_account_balance(agent_id, 1, 0)
        .expect("seed agent chat OC");
}

#[test]
fn runtime_agent_chat_provider_mode_reports_feedback_failure() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_DECISION_SOURCE_ENV, "provider_backed");
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_BACKEND_ENV, "provider_local_bridge");
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_CONTRACT_ENV, "worldsim_provider_v1");
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_TRANSPORT_ENV, "loopback_http");
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_URL_ENV, "http://127.0.0.1:9");
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_PROFILE_ENV, "oasis7_p0_low_freq_npc");
    }
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
    seed_agent_chat_oc(&mut server, agent_id.as_str());
    let (public_key, private_key) = test_signer(35);
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        34,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );
    let request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id,
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello".to_string(),
            intent_tick: Some(35),
            intent_seq: Some(35),
        },
        35,
        public_key.as_str(),
        private_key.as_str(),
    );
    let err = server
        .handle_agent_chat(request)
        .expect_err("provider feedback failure should reject chat ack");
    assert_eq!(err.code, "provider_unreachable");
    clear_runtime_provider_env();
}

#[test]
fn runtime_agent_chat_provider_mode_accepts_feedback_without_echo_receipt() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    let recorded = Arc::new(Mutex::new(Vec::<RecordedHttpRequest>::new()));
    let base_url = spawn_runtime_live_mock_http_server(5, {
        let recorded = Arc::clone(&recorded);
        move |request| {
            recorded
                .lock()
                .expect("recorded lock")
                .push(request.clone());
            match (request.method.as_str(), request.path.as_str()) {
                ("GET", "/v1/provider/info") => MockHttpResponse {
                    status_code: 200,
                    body: serde_json::json!({
                        "provider_id": "provider_local_bridge",
                        "capabilities": ["decision", "feedback", "agent_chat"],
                        "supported_action_sets": ["phase1_low_frequency"]
                    })
                    .to_string(),
                },
                ("GET", "/v1/provider/health") => MockHttpResponse {
                    status_code: 200,
                    body: serde_json::json!({"ok": true, "status": "ok"}).to_string(),
                },
                ("POST", "/v1/world-simulator/feedback") => MockHttpResponse {
                    status_code: 200,
                    body: serde_json::json!({"ok": true}).to_string(),
                },
                ("POST", "/v1/world-simulator/agent-chat") => MockHttpResponse {
                    status_code: 200,
                    body: serde_json::json!({
                        "agent_id": "agent-0",
                        "message": "我在测试地点，资源是 electricity=32 data=8。"
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
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_MODE_ENV, "provider_loopback_http");
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_URL_ENV, base_url);
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_PROFILE_ENV, "oasis7_p0_low_freq_npc");
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(RUNTIME_AGENT_CHAT_ECHO_ENV, "0");
    }
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    assert!(server.llm_sidecar.supports_agent_chat());
    let gameplay = server
        .compat_snapshot()
        .player_gameplay
        .expect("player gameplay snapshot");
    assert!(
        gameplay
            .available_actions
            .iter()
            .any(|action| action.protocol_action == "agent_chat")
    );
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    seed_agent_chat_oc(&mut server, agent_id.as_str());
    let (public_key, private_key) = test_signer(34);
    let request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello provider feedback".to_string(),
            intent_tick: Some(12),
            intent_seq: Some(34),
        },
        34,
        public_key.as_str(),
        private_key.as_str(),
    );
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        33,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );

    let ack = server.handle_agent_chat(request).expect("chat accepted");
    assert_eq!(ack.agent_id, agent_id);
    server.enqueue_pending_provider_agent_chat_replies();
    let recorded = recorded.lock().expect("recorded lock");
    assert_eq!(recorded.len(), 5);
    assert_eq!(recorded[0].path, "/v1/provider/info");
    assert_eq!(recorded[1].path, "/v1/provider/health");
    assert_eq!(recorded[2].method, "POST");
    assert_eq!(recorded[2].path, "/v1/world-simulator/feedback");
    assert_eq!(recorded[3].method, "GET");
    assert_eq!(recorded[3].path, "/v1/provider/info");
    assert_eq!(recorded[4].method, "POST");
    assert_eq!(recorded[4].path, "/v1/world-simulator/agent-chat");
    let feedback: crate::simulator::FeedbackEnvelope =
        serde_json::from_slice(recorded[2].body.as_slice()).expect("decode feedback");
    assert_eq!(
        feedback.world_delta_summary.as_deref(),
        Some("player_message: hello provider feedback")
    );
    assert!(server.pending_virtual_events.iter().any(|event| matches!(
        &event.kind,
        crate::simulator::WorldEventKind::AgentSpoke { agent_id: event_agent_id, message, .. }
            if event_agent_id == &agent_id && message == "我在测试地点，资源是 electricity=32 data=8。"
    )));
    assert!(!server.pending_virtual_events.iter().any(|event| matches!(
        &event.kind,
        crate::simulator::WorldEventKind::AgentSpoke { agent_id: event_agent_id, message, .. }
            if event_agent_id == &agent_id && message.contains("[local-mock-receipt]")
    )));
    clear_runtime_provider_env();
}

#[test]
fn runtime_agent_chat_provider_mode_skips_reply_without_agent_chat_capability() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    let recorded = Arc::new(Mutex::new(Vec::<RecordedHttpRequest>::new()));
    let base_url = spawn_runtime_live_mock_http_server(2, {
        let recorded = Arc::clone(&recorded);
        move |request| {
            recorded
                .lock()
                .expect("recorded lock")
                .push(request.clone());
            match (request.method.as_str(), request.path.as_str()) {
                ("GET", "/v1/provider/info") => MockHttpResponse {
                    status_code: 200,
                    body: serde_json::json!({
                        "provider_id": "phase1_provider",
                        "capabilities": ["decision", "feedback"],
                        "supported_action_sets": ["phase1_low_frequency"]
                    })
                    .to_string(),
                },
                ("GET", "/v1/provider/health") => MockHttpResponse {
                    status_code: 200,
                    body: serde_json::json!({"ok": true, "status": "ok"}).to_string(),
                },
                ("POST", "/v1/world-simulator/feedback") => MockHttpResponse {
                    status_code: 200,
                    body: serde_json::json!({"ok": true}).to_string(),
                },
                ("POST", "/v1/world-simulator/agent-chat") => MockHttpResponse {
                    status_code: 500,
                    body: serde_json::json!({"ok": false, "error": "should_not_call"}).to_string(),
                },
                _ => MockHttpResponse {
                    status_code: 404,
                    body: serde_json::json!({"ok": false, "error": "not_found"}).to_string(),
                },
            }
        }
    });
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_DECISION_SOURCE_ENV, "provider_backed");
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_BACKEND_ENV, "provider_local_bridge");
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_CONTRACT_ENV, "worldsim_provider_v1");
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_TRANSPORT_ENV, "loopback_http");
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_URL_ENV, base_url);
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_PROFILE_ENV, "oasis7_p0_low_freq_npc");
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(RUNTIME_AGENT_CHAT_ECHO_ENV, "0");
    }
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
    seed_agent_chat_oc(&mut server, agent_id.as_str());
    let (public_key, private_key) = test_signer(36);
    let request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello phase1 provider".to_string(),
            intent_tick: Some(13),
            intent_seq: Some(36),
        },
        36,
        public_key.as_str(),
        private_key.as_str(),
    );
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        35,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );

    let ack = server.handle_agent_chat(request).expect("chat accepted");
    assert_eq!(ack.agent_id, agent_id);
    server.enqueue_pending_provider_agent_chat_replies();
    let recorded = recorded.lock().expect("recorded lock");
    assert_eq!(recorded.len(), 2);
    assert_eq!(recorded[0].method, "POST");
    assert_eq!(recorded[0].path, "/v1/world-simulator/feedback");
    assert_eq!(recorded[1].method, "GET");
    assert_eq!(recorded[1].path, "/v1/provider/info");
    assert!(
        !recorded
            .iter()
            .any(|request| request.path == "/v1/world-simulator/agent-chat")
    );
    assert!(server.pending_virtual_events.iter().all(|event| !matches!(
        &event.kind,
        crate::simulator::WorldEventKind::AgentSpoke { .. }
    )));
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
fn runtime_agent_chat_requires_starter_oc_balance() {
    let _guard = lock_test_llm_env();
    // SAFETY: This test holds the runtime LLM env lock while mutating process env.
    unsafe {
        oasis7::env_mut::set_var(RUNTIME_AGENT_CHAT_ECHO_ENV, "1");
    }
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
    let (public_key, private_key) = test_signer(37);
    let request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello without OC".to_string(),
            intent_tick: Some(14),
            intent_seq: Some(37),
        },
        37,
        public_key.as_str(),
        private_key.as_str(),
    );
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        36,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );

    let err = server
        .handle_agent_chat(request)
        .expect_err("starter OC should be required before agent chat");
    assert_eq!(err.code, "starter_oc_required");
    assert_eq!(
        server.llm_sidecar.player_auth_last_nonce.get("player-a"),
        Some(&36)
    );
}

#[test]
fn runtime_agent_chat_echo_env_enqueues_agent_spoke_virtual_event() {
    let _guard = lock_test_llm_env();
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(RUNTIME_AGENT_CHAT_ECHO_ENV, "1");
    }
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
    seed_agent_chat_oc(&mut server, agent_id.as_str());
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
    let feedback = server
        .latest_player_gameplay_feedback
        .as_ref()
        .expect("agent-chat feedback recorded");
    assert_eq!(feedback.action, "agent_chat");
    assert_eq!(feedback.stage, "accepted");
    assert_eq!(feedback.target_agent_id.as_deref(), Some(agent_id.as_str()));
    assert!(
        feedback
            .intent_summary
            .as_deref()
            .is_some_and(|summary| summary.contains(agent_id.as_str()))
    );

    let events: Vec<_> = server.pending_virtual_events.drain(..).collect();
    assert!(events.iter().any(|event| matches!(
        &event.kind,
        crate::simulator::WorldEventKind::AgentSpoke { agent_id: event_agent_id, message, .. }
            if event_agent_id == &agent_id && message == "[local-mock-receipt] 已收到消息；当前本地 mock provider 不生成真实 Agent 回复：hello runtime echo"
    )));
}

#[test]
fn runtime_agent_chat_echo_env_accepts_chat_without_llm_runner_config() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(RUNTIME_AGENT_CHAT_ECHO_ENV, "1");
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::remove_var(crate::simulator::ENV_LLM_MODEL);
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::remove_var(crate::simulator::ENV_LLM_BASE_URL);
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::remove_var(crate::simulator::ENV_LLM_API_KEY);
    }
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
    seed_agent_chat_oc(&mut server, agent_id.as_str());
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
            if event_agent_id == &agent_id && message == "[local-mock-receipt] 已收到消息；当前本地 mock provider 不生成真实 Agent 回复：hello runtime echo without llm config"
    )));
}

#[test]
fn runtime_agent_chat_echo_removed_old_brand_env_is_ignored() {
    let _guard = lock_test_llm_env();
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(
            removed_old_brand_runtime_live_env("RUNTIME_AGENT_CHAT_ECHO"),
            "1",
        );
    }
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
    seed_agent_chat_oc(&mut server, agent_id.as_str());
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
            if event_agent_id == &agent_id && message == "[local-mock-receipt] 已收到消息；当前本地 mock provider 不生成真实 Agent 回复：hello removed old brand runtime echo"
    )));
}
