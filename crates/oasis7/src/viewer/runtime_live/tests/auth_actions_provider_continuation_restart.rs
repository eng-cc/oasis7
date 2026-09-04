use super::auth_actions::{
    MockHttpResponse, RecordedHttpRequest, provider_context_response,
    spawn_runtime_live_mock_http_server,
};
use super::*;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn wait_for_provider_phase(
    label: &str,
    timeout: Duration,
    mut poll: impl FnMut() -> Result<bool, String>,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    loop {
        if poll()? {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(format!("{label} did not complete within {timeout:?}"));
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn runtime_provider_backed_wake_resumes_with_fresh_request_and_origin_lineage() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    let recorded = Arc::new(Mutex::new(Vec::<RecordedHttpRequest>::new()));
    let decision_count = Arc::new(Mutex::new(0_usize));
    let base_url = spawn_runtime_live_mock_http_server(4, {
        let recorded = Arc::clone(&recorded);
        let decision_count = Arc::clone(&decision_count);
        move |request| {
            recorded
                .lock()
                .expect("recorded lock")
                .push(request.clone());
            if request.path == "/v1/world-simulator/feedback-context" {
                return MockHttpResponse {
                    status_code: 200,
                    body: serde_json::json!({"ok": true}).to_string(),
                };
            }
            if request.path != "/v1/world-simulator/decision-context" {
                return MockHttpResponse {
                    status_code: 404,
                    body: serde_json::json!({"ok": false, "error": "not_found"}).to_string(),
                };
            }
            let request_number = {
                let mut count = decision_count.lock().expect("decision count lock");
                *count += 1;
                *count
            };
            let decoded: crate::simulator::ContinuousAgentRequestContextV1 =
                serde_json::from_slice(request.body.as_slice())
                    .expect("decode provider decision request");
            decoded
                .validate_production_lane()
                .expect("provider request must be production-valid");
            let response = crate::simulator::DecisionResponse {
                decision: if request_number == 1 {
                    crate::simulator::ProviderDecision::Wait
                } else {
                    crate::simulator::ProviderDecision::Act {
                        action_ref: "move_agent".to_string(),
                        action: crate::simulator::Action::MoveAgent {
                            agent_id: decoded.base_decision_request.observation.agent_id.clone(),
                            to: "runtime:0:0:0".to_string(),
                        },
                    }
                },
                module_command: None,
                provider_error: None,
                diagnostics: crate::simulator::ProviderDiagnostics::default(),
                trace_payload: crate::simulator::ProviderTraceEnvelope::default(),
                memory_write_intents: Vec::new(),
            };
            MockHttpResponse {
                status_code: 200,
                body: serde_json::to_string(&provider_context_response(&decoded, response))
                    .expect("encode provider response"),
            }
        }
    });
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_MODE_ENV, "provider_loopback_http");
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_URL_ENV, base_url);
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_PROFILE_ENV, "oasis7_p0_low_freq_npc");
        oasis7::env_mut::set_var(VIEWER_AGENT_EXECUTION_LANE_ENV, "player_parity");
    }
    let world_id = format!("wake-runtime-{}", WorldScenario::Minimal.as_str());
    let finality_block_hash =
        crate::simulator::h_v1("oasis7.viewer.test.finality-block.v1", &world_id).to_string();
    let lineage_path = std::env::temp_dir().join(format!(
        "oasis7-runtime-live-provider-restart-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let runtime_config = || {
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm)
            .with_test_cognition_runtime_binding(
                "continuation-branch",
                0,
                Some(finality_block_hash.clone()),
                "verified",
                0,
            )
    };
    let mut server = ViewerRuntimeLiveServer::new(
        runtime_config().with_provider_lineage_store(lineage_path.clone()),
    )
    .expect("runtime server");
    server.world = server.world.clone().with_cognition_scheduler(
        serde_json::from_value(serde_json::json!({
            "schema_version": "scheduler-policy.v1",
            "max_total_wakes_per_tick": 8,
            "max_wakes_per_agent_per_tick": 1,
            "aging_after_ticks": 2,
            "max_starvation_ticks": 4,
            "initial_priority": 0,
            "comparator": "deadline_due_desc,next_wake_tick_asc,effective_priority_desc,starvation_deadline_tick_asc,cursor_distance_asc,agent_id_asc,continuation_id_asc,wake_seq_asc",
            "service_order": "stable_round_robin"
        }))
        .expect("decode continuation scheduler policy"),
        8,
    );
    server
        .world
        .install_test_provider_capability_fixture("agent-0")
        .expect("install Runtime provider capability fixture");
    let mut wait_trace = None;
    wait_for_provider_phase("provider Wait admission", Duration::from_secs(5), || {
        server.llm_sidecar.request_decision();
        match server.enqueue_llm_action_from_sidecar() {
            Ok(Some(trace)) if matches!(trace.decision, crate::simulator::AgentDecision::Wait) => {
                wait_trace = Some(trace);
                Ok(true)
            }
            Ok(Some(_)) | Ok(None) => Ok(false),
            Err(trace) => Err(format!("provider Wait admission failed: {trace:?}")),
        }
    })
    .expect("ordinary provider Wait must be admitted through Harness and Runtime");
    assert!(wait_trace.is_some());
    let checkpoint: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&lineage_path).expect("provider lineage checkpoint after Wait"),
    )
    .expect("decode provider lineage checkpoint after Wait");
    assert_eq!(
        checkpoint["provider_continuation_proposals"]
            .as_object()
            .map(serde_json::Map::len),
        Some(1),
        "Wait must persist the exact Harness proposal before Runtime restart"
    );
    assert!(
        server.world.cognition()["continuations"]
            .as_array()
            .is_some_and(|continuations| continuations.iter().any(|value| {
                matches!(
                    value["status"].as_str(),
                    Some("scheduled") | Some("pending")
                )
            })),
        "provider Wait must leave a durable Runtime continuation: {}",
        server.world.cognition()
    );
    server.world.step().expect("normal Runtime tick wakes Wait");
    let restarted_world = server.world.clone();
    let mut restarted = ViewerRuntimeLiveServer::new(
        runtime_config().with_provider_lineage_store(lineage_path.clone()),
    )
    .expect("restarted server");
    restarted.world = restarted_world;
    restarted
        .llm_sidecar
        .restore_provider_lineage(&restarted.world)
        .expect("restore provider lineage before wake resume");
    restarted
        .sync_runtime_wake_projection()
        .expect("mirror Runtime-selected wake into restarted Viewer");
    let mut action_trace = None;
    wait_for_provider_phase("provider wake resume", Duration::from_secs(5), || {
        restarted.llm_sidecar.request_decision();
        match restarted.enqueue_llm_action_from_sidecar() {
            Ok(Some(trace))
                if matches!(trace.decision, crate::simulator::AgentDecision::Act(_)) =>
            {
                action_trace = Some(trace);
                Ok(true)
            }
            Ok(Some(_)) | Ok(None) => Ok(false),
            Err(trace) => Err(format!("provider wake resume failed: {trace:?}")),
        }
    })
    .expect("Runtime wake must resume one fresh provider turn");
    assert!(action_trace.is_some());
    super::provider_continuation_drains::drain_final_continuation(&mut restarted)
        .expect("final continuation wake must terminate at Runtime budget exhaustion");
    let requests = recorded.lock().expect("recorded requests");
    let decisions = requests
        .iter()
        .filter(|request| request.path == "/v1/world-simulator/decision-context")
        .map(|request| {
            serde_json::from_slice::<crate::simulator::ContinuousAgentRequestContextV1>(
                request.body.as_slice(),
            )
            .expect("decode recorded provider request")
        })
        .collect::<Vec<_>>();
    assert_eq!(decisions.len(), 2);
    assert_ne!(
        decisions[0].continuation_digest,
        crate::simulator::Digest32::default(),
        "normal provider request must carry the canonical no-continuation sentinel"
    );
    assert_ne!(
        decisions[1].continuation_digest,
        crate::simulator::Digest32::default(),
        "resumed provider request must carry the Runtime-selected continuation"
    );
    assert_ne!(decisions[0].agent_turn_id, decisions[1].agent_turn_id);
    assert_ne!(decisions[0].agent_session_id, decisions[1].agent_session_id);
    let cognition = restarted.world.cognition();
    let events = cognition["cognition_journal"]["events"]
        .as_array()
        .expect("Runtime cognition journal events");
    assert!(
        events
            .iter()
            .any(|event| event["event_kind"] == "ContinuationScheduled"),
        "ordinary Wait must append Runtime continuation admission evidence"
    );
    assert!(
        events
            .iter()
            .any(|event| event["event_kind"] == "ContinuationReplanned"),
        "normal wake must append Runtime continuation replan evidence"
    );
    assert!(
        cognition["commit_records"]
            .as_array()
            .is_some_and(|records| records.iter().any(|record| record["status"] == "committed")),
        "resumed provider action must commit through Runtime"
    );
    drop(requests);
    restarted.llm_sidecar.request_decision();
    restarted
        .enqueue_llm_action_from_sidecar()
        .expect("ordinary next turn AgentBusy");
    let _ = std::fs::remove_file(&lineage_path);
    clear_runtime_provider_env();
    drop(_guard);
}
