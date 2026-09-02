use super::auth_actions::{
    MockHttpResponse, RecordedHttpRequest, provider_context_response,
    spawn_runtime_live_mock_http_server,
};
use super::*;
use std::sync::{Arc, Mutex};

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
                ("POST", "/v1/world-simulator/decision-context") => {
                    let decoded: crate::simulator::ContinuousAgentRequestContextV1 =
                        serde_json::from_slice(request.body.as_slice())
                            .expect("decode outer decision request");
                    assert!(
                        decoded.retry_seq > 0,
                        "semantic retry lineage must be nonzero"
                    );
                    assert!(
                        decoded.transport_attempt > 0,
                        "transport attempt lineage must be nonzero"
                    );
                    decoded
                        .validate_production_lane()
                        .expect("live caller must provide a valid outer request");
                    assert!(
                        decoded
                            .runtime_binding
                            .base_world_hash
                            .is_canonical_blake3(),
                        "runtime binding must carry a canonical world hash"
                    );
                    assert!(
                        decoded
                            .runtime_binding
                            .runtime_manifest_hash
                            .is_canonical_blake3(),
                        "runtime binding must carry a canonical manifest hash"
                    );
                    let response = crate::simulator::DecisionResponse {
                        decision: crate::simulator::ProviderDecision::Act {
                            action_ref: "speak_to_nearby".to_string(),
                            action: crate::simulator::Action::SpeakToNearby {
                                agent_id: decoded
                                    .base_decision_request
                                    .observation
                                    .agent_id
                                    .clone(),
                                message: "runtime-live step ok".to_string(),
                                target_agent_id: None,
                            },
                        },
                        module_command: None,
                        provider_error: None,
                        diagnostics: crate::simulator::ProviderDiagnostics::default(),
                        trace_payload: crate::simulator::ProviderTraceEnvelope::default(),
                        memory_write_intents: Vec::new(),
                    };
                    let response = provider_context_response(&decoded, response);
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
        oasis7::env_mut::set_var(VIEWER_AGENT_EXECUTION_LANE_ENV, "player_parity");
    }

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
    assert_eq!(recorded[0].path, "/v1/world-simulator/decision-context");
    assert_eq!(
        recorded[0].headers.get("content-type").map(String::as_str),
        Some("application/json")
    );
    let decision_request: crate::simulator::ContinuousAgentRequestContextV1 =
        serde_json::from_slice(recorded[0].body.as_slice())
            .expect("decode provider-backed outer decision request");
    let decision_request = decision_request.base_decision_request;
    let action_refs: Vec<&str> = decision_request
        .observation
        .action_catalog
        .iter()
        .map(|entry| entry.action_ref.as_str())
        .collect();
    for expected_action_ref in [
        "harvest_radiation",
        "mine_compound",
        "refine_compound",
        "build_factory",
        "schedule_recipe",
    ] {
        assert!(
            action_refs.contains(&expected_action_ref),
            "provider-backed catalog should expose {expected_action_ref}: {:?}",
            action_refs
        );
    }
    let memory_summary = decision_request
        .observation
        .memory_summary
        .as_deref()
        .expect("provider-backed catalog should seed memory summary");
    assert!(
        memory_summary.contains("post_onboarding.establish_first_capability"),
        "unexpected memory summary: {memory_summary}"
    );
    assert!(
        memory_summary.contains("build_factory(factory.smelter.mk1)"),
        "unexpected memory summary: {memory_summary}"
    );
    assert!(
        memory_summary.contains("schedule_recipe("),
        "unexpected memory summary: {memory_summary}"
    );
    clear_runtime_provider_env();
}
