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
fn runtime_step_control_requests_llm_decision_and_advances_with_provider_backed_loopback() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    let recorded = Arc::new(Mutex::new(Vec::<RecordedHttpRequest>::new()));
    let decision_count = Arc::new(Mutex::new(0_usize));
    let step_provider_gate = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let base_url = spawn_runtime_live_mock_http_server(6, {
        let recorded = Arc::clone(&recorded);
        let decision_count = Arc::clone(&decision_count);
        let step_provider_gate = Arc::clone(&step_provider_gate);
        move |request| {
            {
                let mut recorded = recorded.lock().expect("recorded lock");
                recorded.push(request.clone());
            }
            match (request.method.as_str(), request.path.as_str()) {
                ("POST", "/v1/world-simulator/decision-context") => {
                    let request_number = {
                        let mut decision_count =
                            decision_count.lock().expect("decision count lock");
                        *decision_count += 1;
                        *decision_count
                    };
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
                    if request_number == 2 {
                        return MockHttpResponse {
                            status_code: 503,
                            body: serde_json::json!({
                                "ok": false,
                                "error": "provider temporarily unavailable"
                            })
                            .to_string(),
                        };
                    }
                    if request_number == 4 {
                        while !step_provider_gate.load(std::sync::atomic::Ordering::Acquire) {
                            std::thread::yield_now();
                        }
                    }
                    let decision = if request_number > 3 {
                        crate::simulator::ProviderDecision::Wait
                    } else {
                        crate::simulator::ProviderDecision::Act {
                            action_ref: "move_agent".to_string(),
                            action: crate::simulator::Action::MoveAgent {
                                agent_id: decoded
                                    .base_decision_request
                                    .observation
                                    .agent_id
                                    .clone(),
                                to: format!("runtime:{request_number}:0:0"),
                            },
                        }
                    };
                    let response = crate::simulator::DecisionResponse {
                        decision,
                        module_command: None,
                        provider_error: None,
                        diagnostics: crate::simulator::ProviderDiagnostics::default(),
                        trace_payload: crate::simulator::ProviderTraceEnvelope::default(),
                        memory_write_intents: if request_number == 1 {
                            vec![crate::simulator::MemoryWriteIntent {
                                scope: "session_private".to_string(),
                                summary: "first provider action committed".to_string(),
                                tags: vec!["e2e".to_string()],
                            }]
                        } else {
                            Vec::new()
                        },
                    };
                    let response = provider_context_response(&decoded, response);
                    MockHttpResponse {
                        status_code: 200,
                        body: serde_json::to_string(&response).expect("encode decision response"),
                    }
                }
                ("POST", "/v1/world-simulator/feedback-context") => {
                    let feedback: crate::simulator::FeedbackEnvelopeV1 =
                        serde_json::from_slice(request.body.as_slice())
                            .expect("decode Runtime feedback");
                    if feedback.status == "committed" {
                        assert!(feedback.runtime_receipt_id.is_some());
                    } else {
                        assert!(
                            matches!(
                                (feedback.status.as_str(), feedback.reject_reason.as_deref()),
                                ("pending", Some(_))
                                    | ("rejected", Some("no_effect"))
                                    | ("rejected", Some("stale_base"))
                                    | ("rejected", Some("action_rejected"))
                            ),
                            "unexpected provider feedback: status={} reason={:?}",
                            feedback.status,
                            feedback.reject_reason
                        );
                    }
                    MockHttpResponse {
                        status_code: 200,
                        body: serde_json::json!({"ok": true}).to_string(),
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

    let test_world_id = format!("live-runtime-{}", WorldScenario::Minimal.as_str());
    let test_finality_block_hash =
        crate::simulator::h_v1("oasis7.viewer.test.finality-block.v1", &test_world_id).to_string();
    let lineage_path = std::env::temp_dir().join(format!(
        "oasis7-runtime-live-provider-e2e-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm)
            .with_provider_lineage_store(lineage_path.clone())
            .with_test_cognition_runtime_binding(
                "provider-context-branch",
                0,
                Some(test_finality_block_hash.clone()),
                "verified",
                0,
            ),
    )
    .expect("runtime server");
    super::provider_continuation_drains::install_cognition_scheduler(&mut server);
    let expected_world_id = server.config.world_id.clone();
    server
        .world
        .install_test_provider_capability_fixture("agent-0")
        .expect("install Runtime provider capability fixture");
    let baseline_event_seq = latest_runtime_event_seq(&server.world);
    let phase_result = (|| {
        wait_for_provider_phase(
            "initial provider-backed action",
            Duration::from_secs(5),
            || {
                server.llm_sidecar.request_decision();
                match server.enqueue_llm_action_from_sidecar() {
                    Ok(Some(_)) => Ok(true),
                    Ok(None) => Ok(false),
                    Err(trace) => Err(format!("initial provider action failed: {trace:?}")),
                }
            },
        )?;
        wait_for_provider_phase("transient provider failure", Duration::from_secs(5), || {
            server.llm_sidecar.request_decision();
            match server.enqueue_llm_action_from_sidecar() {
                Ok(_) => Ok(false),
                Err(trace) => {
                    if decision_trace_provider_error_retryable(&trace).unwrap_or(false) {
                        Ok(true)
                    } else {
                        Err(format!("unexpected provider failure: {trace:?}"))
                    }
                }
            }
        })?;
        wait_for_provider_phase("transport retry", Duration::from_secs(5), || {
            server.llm_sidecar.request_decision();
            match server.enqueue_llm_action_from_sidecar() {
                Ok(Some(_)) => Ok(true),
                Ok(None) => Ok(false),
                Err(trace) => Err(format!("transport retry failed: {trace:?}")),
            }
        })?;
        if latest_runtime_event_seq(&server.world) <= baseline_event_seq {
            return Err(
                "provider-backed Runtime commit did not append an authoritative event".to_string(),
            );
        }
        Ok::<(), String>(())
    })();
    let captured_requests = {
        let recorded = recorded.lock().expect("recorded lock");
        let decisions = recorded
            .iter()
            .filter(|request| request.path == "/v1/world-simulator/decision-context")
            .cloned()
            .collect::<Vec<_>>();
        let feedbacks = recorded
            .iter()
            .filter(|request| request.path == "/v1/world-simulator/feedback-context")
            .cloned()
            .collect::<Vec<_>>();
        let paths = recorded
            .iter()
            .map(|request| request.path.clone())
            .collect::<Vec<_>>();
        (decisions, feedbacks, paths)
    };
    let step_result = {
        let baseline_time = server.world.state().time;
        let (mut writer, _client) = test_writer_pair();
        let mut session = RuntimeLiveSession::new();
        server
            .advance_runtime(&mut session, &mut writer, "step", 2, None, true)
            .map_err(|error| format!("multi-step control failed: {error:?}"))
            .and_then(|_| {
                (server.world.state().time == baseline_time + 2)
                    .then_some(())
                    .ok_or_else(|| {
                        format!(
                            "Step {{ count: 2 }} advanced to {}, expected {}",
                            server.world.state().time,
                            baseline_time + 2
                        )
                    })
            })
    };
    step_provider_gate.store(true, std::sync::atomic::Ordering::Release);
    let step_drain_result =
        super::provider_continuation_drains::drain_step_provider_response(&mut server);
    // Keep provider configuration scoped to polling; release it before assertions.
    clear_runtime_provider_env();
    drop(_guard);
    phase_result.expect("provider context phases should complete");
    step_result.expect("multi-step control should advance each requested iteration");
    step_drain_result.expect("multi-step provider response should be accepted and drained");
    // A simulated Viewer restart restores the adapter checkpoint against the
    // same Runtime world.  The committed identity is terminal and therefore
    // cannot be redispatched as a duplicate provider turn.
    let mut restored_sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
    restored_sidecar.configure_provider_lineage_store(lineage_path.clone());
    restored_sidecar
        .restore_provider_lineage(&server.world)
        .expect("restore production provider lineage after restart");
    assert!(restored_sidecar.pending_actions_empty());
    assert!(restored_sidecar.provider_contexts_empty());
    assert!(restored_sidecar.provider_has_terminal_status("committed"));
    assert_eq!(
        restored_sidecar.provider_memory_store().entries().len(),
        1,
        "committed provider memory projection must survive adapter restart"
    );
    let _ = std::fs::remove_file(&lineage_path);
    // Production-surface lifecycle proof: the actual RuntimeLive sidecar
    // prefixes provider I/O, records the retry as a new transport attempt on
    // the same identity, and closes exactly once at the authoritative receipt.
    let lifecycle_events = server.world.cognition()["cognition_journal"]["events"]
        .as_array()
        .expect("Runtime cognition journal events");
    let dispatches = lifecycle_events
        .iter()
        .filter(|event| event["event_kind"] == "RequestDispatched")
        .collect::<Vec<_>>();
    let retried = dispatches
        .iter()
        .find(|event| event["transport_attempt"] == 2)
        .expect("transport retry must be durably dispatched");
    let turn_id = retried["agent_turn_id"]
        .as_str()
        .expect("retry turn identity");
    let request_id = retried["decision_request_id"]
        .as_str()
        .expect("retry request identity");
    let same_turn = |event: &&serde_json::Value| {
        event["agent_turn_id"].as_str() == Some(turn_id)
            && event["decision_request_id"].as_str() == Some(request_id)
    };
    assert_eq!(
        lifecycle_events
            .iter()
            .filter(|event| same_turn(event) && event["event_kind"] == "TurnStarted")
            .count(),
        1,
        "restart/retry must not duplicate TurnStarted"
    );
    assert_eq!(
        lifecycle_events
            .iter()
            .filter(|event| same_turn(event) && event["event_kind"] == "ContextCaptured")
            .count(),
        1,
        "retry must reuse the captured context"
    );
    let attempts = dispatches
        .iter()
        .filter(|event| same_turn(event))
        .filter_map(|event| event["transport_attempt"].as_u64())
        .collect::<Vec<_>>();
    assert_eq!(attempts, vec![1, 2]);
    for kind in ["ResponseRecorded", "WorldReceiptLinked"] {
        assert_eq!(
            lifecycle_events
                .iter()
                .filter(|event| same_turn(event) && event["event_kind"] == kind)
                .count(),
            1,
            "successful provider retry must emit one {kind}"
        );
    }
    assert_eq!(
        lifecycle_events
            .iter()
            .filter(|event| {
                same_turn(event)
                    && event["event_kind"] == "CognitionTurnCompleted"
                    && event["status"] == "committed"
            })
            .count(),
        1,
        "successful provider retry must close the turn once"
    );
    let memory_entry = server
        .llm_sidecar
        .provider_memory_store()
        .entries()
        .first()
        .expect("committed provider memory intent");
    assert_eq!(memory_entry["scope"], "session_private");
    assert_eq!(memory_entry["provenance"], "runtime_authoritative");
    assert!(memory_entry["receipt_id"].as_str().is_some());

    let (decision_records, feedback_records, recorded_paths) = captured_requests;
    assert_eq!(decision_records.len(), 3);
    assert_eq!(
        feedback_records.len(),
        2,
        "recorded paths: {:?}",
        recorded_paths
    );
    assert_eq!(
        decision_records[0].path,
        "/v1/world-simulator/decision-context"
    );
    assert_eq!(
        decision_records[0]
            .headers
            .get("content-type")
            .map(String::as_str),
        Some("application/json")
    );
    let decision_request: crate::simulator::ContinuousAgentRequestContextV1 =
        serde_json::from_slice(decision_records[0].body.as_slice())
            .expect("decode provider-backed outer decision request");
    assert_eq!(
        decision_request.runtime_binding.world_id, expected_world_id,
        "provider context must preserve the Runtime-owned world identity"
    );
    assert_eq!(
        decision_request.runtime_binding.branch_id, "provider-context-branch",
        "provider context must preserve the Runtime-owned branch identity"
    );
    assert_eq!(decision_request.runtime_binding.finality_epoch, 0);
    assert_eq!(
        decision_request
            .runtime_binding
            .finality_block_hash
            .as_ref()
            .map(crate::simulator::Digest32::as_str),
        Some(test_finality_block_hash.as_str())
    );
    assert_eq!(decision_request.runtime_binding.finality_status, "verified");
    assert_eq!(decision_request.runtime_binding.reorg_epoch, 0);
    let current_state_root = server
        .world
        .current_state_root_hash()
        .expect("current Runtime state root");
    let ad_hoc_state_hash = crate::simulator::h_v1(
        "oasis7.runtime.world-state.v1",
        &crate::simulator::Digest32::from(current_state_root),
    );
    assert_ne!(
        decision_request.runtime_binding.base_world_hash, ad_hoc_state_hash,
        "viewer must not recreate the Runtime binding with an ad-hoc state-root hash"
    );
    let provider_observation = &decision_request
        .base_decision_request
        .observation
        .observation;
    assert!(
        !provider_observation.self_state.location_ref.is_empty(),
        "provider context must carry the observed location"
    );
    assert!(
        provider_observation
            .self_state
            .resource_summary
            .get("Electricity")
            .is_some_and(|amount| *amount > 0),
        "provider context must carry the observed resources"
    );
    let base_decision_request = decision_request.base_decision_request.clone();
    let action_refs: Vec<&str> = base_decision_request
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
    let memory_summary = base_decision_request
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
    let second_request: crate::simulator::ContinuousAgentRequestContextV1 =
        serde_json::from_slice(decision_records[1].body.as_slice())
            .expect("decode second provider-backed outer decision request");
    let next_turn_memory_summary = second_request
        .base_decision_request
        .observation
        .memory_summary
        .as_deref()
        .expect("next provider turn should carry a memory summary");
    assert!(
        next_turn_memory_summary.contains("first provider action committed"),
        "next provider turn must retrieve the committed bounded memory snapshot: {next_turn_memory_summary}"
    );
    assert_eq!(
        decision_request.agent_session_id, second_request.agent_session_id,
        "transport retry must preserve the Agent session"
    );
    assert_eq!(
        decision_request.agent_subject, second_request.agent_subject,
        "transport retry must preserve the Agent subject"
    );
    let retry_request: crate::simulator::ContinuousAgentRequestContextV1 =
        serde_json::from_slice(decision_records[2].body.as_slice())
            .expect("decode transport retry provider-backed request");
    assert_eq!(second_request.transport_attempt, 1);
    assert_eq!(retry_request.transport_attempt, 2);
    assert_eq!(
        second_request.agent_session_id, retry_request.agent_session_id,
        "transport retry must preserve the Agent session"
    );
    assert_eq!(second_request.agent_turn_id, retry_request.agent_turn_id);
    assert_eq!(
        second_request.decision_request_id,
        retry_request.decision_request_id
    );
    assert_eq!(second_request.request_digest, retry_request.request_digest);
    assert_eq!(second_request.retry_seq, retry_request.retry_seq);
    for feedback_record in feedback_records {
        let feedback: crate::simulator::FeedbackEnvelopeV1 =
            serde_json::from_slice(feedback_record.body.as_slice())
                .expect("decode delivered Runtime feedback");
        assert_eq!(feedback.status, "committed");
        assert_eq!(feedback.provenance, "runtime_authoritative");
        assert!(feedback.candidate_action_id.is_some());
        assert!(feedback.runtime_receipt_id.is_some());
    }
}

#[test]
fn runtime_builtin_wait_enters_the_shared_harness_lifecycle() {
    let _guard = lock_test_llm_env();
    let recorded = Arc::new(Mutex::new(Vec::<RecordedHttpRequest>::new()));
    let decision_count = Arc::new(Mutex::new(0_usize));
    let base_url = spawn_runtime_live_mock_http_server(2, {
        let recorded = Arc::clone(&recorded);
        let decision_count = Arc::clone(&decision_count);
        move |request| {
            recorded
                .lock()
                .expect("recorded lock")
                .push(request.clone());
            assert_eq!(request.method, "POST");
            assert_eq!(request.path, "/responses");
            let request_number = {
                let mut count = decision_count.lock().expect("decision count lock");
                *count += 1;
                *count
            };
            let decision_args = if request_number == 1 {
                serde_json::json!({"decision":"wait"}).to_string()
            } else {
                serde_json::json!({
                    "decision":"move_agent",
                    "to":"runtime:0:0:0"
                })
                .to_string()
            };
            let completed = serde_json::json!({
                "type": "response.completed",
                "sequence_number": request_number,
                "response": {
                    "id": format!("builtin_wait_response_{request_number}"),
                    "object": "response",
                    "created_at": 1,
                    "completed_at": 2,
                    "model": "gpt-test",
                    "output": [{
                        "type": "function_call",
                        "call_id": format!("builtin_wait_call_{request_number}"),
                        "name": "agent_submit_decision",
                        "arguments": decision_args
                    }],
                    "status": "completed",
                    "parallel_tool_calls": false
                }
            });
            MockHttpResponse {
                status_code: 200,
                body: format!("data: {completed}\n\n"),
            }
        }
    });
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::set_var(crate::simulator::ENV_LLM_BASE_URL, base_url);
    }

    let world_id = "builtin-harness-lifecycle";
    let finality_block_hash =
        crate::simulator::h_v1("oasis7.viewer.test.finality-block.v1", &world_id).to_string();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm)
            .with_test_cognition_runtime_binding(
                "builtin-branch",
                0,
                Some(finality_block_hash),
                "verified",
                0,
            ),
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
        .expect("install Runtime builtin capability fixture");

    wait_for_provider_phase("builtin Harness Wait", Duration::from_secs(5), || {
        server.llm_sidecar.request_decision();
        match server.enqueue_llm_action_from_sidecar() {
            Ok(Some(_)) => Ok(true),
            Ok(None) => Ok(false),
            Err(trace) => Err(format!("builtin Harness Wait failed: {trace:?}")),
        }
    })
    .expect("builtin Wait should be admitted through Harness and Runtime");

    assert!(
        server.world.cognition()["continuations"]
            .as_array()
            .is_some_and(|continuations| continuations.iter().any(|value| {
                matches!(
                    value["status"].as_str(),
                    Some("scheduled") | Some("pending")
                )
            })),
        "builtin Wait must leave a durable Runtime continuation: {}",
        server.world.cognition()
    );
    server
        .world
        .step()
        .expect("normal Runtime tick wakes builtin Wait");
    server
        .sync_runtime_wake_projection()
        .expect("mirror Runtime-selected builtin wake into Viewer");
    wait_for_provider_phase("builtin wake resume", Duration::from_secs(5), || {
        server.llm_sidecar.request_decision();
        match server.enqueue_llm_action_from_sidecar() {
            Ok(Some(trace))
                if matches!(trace.decision, crate::simulator::AgentDecision::Act(_)) =>
            {
                Ok(true)
            }
            Ok(Some(_)) | Ok(None) => Ok(false),
            Err(trace) => Err(format!("builtin wake resume failed: {trace:?}")),
        }
    })
    .expect("Runtime wake must resume one fresh builtin turn");

    clear_runtime_provider_env();
    drop(_guard);
    let requests = recorded.lock().expect("recorded lock");
    assert_eq!(requests.len(), 2, "builtin must issue one request per turn");
    let journal = server.world.cognition()["cognition_journal"]["events"]
        .as_array()
        .expect("Runtime cognition journal events");
    for event_kind in [
        "TurnStarted",
        "ContextCaptured",
        "RequestDispatched",
        "ResponseRecorded",
        "ContinuationScheduled",
        "ContinuationReplanned",
    ] {
        assert!(
            journal
                .iter()
                .any(|event| event["event_kind"] == event_kind),
            "builtin shared lifecycle must emit {event_kind}: {journal:?}"
        );
    }
    assert!(
        journal.iter().any(|event| {
            event["event_kind"] == "RequestDispatched"
                && event["provider_invocation_key"]
                    .as_str()
                    .is_some_and(|key| !key.is_empty())
                && event["request_digest"]
                    .as_str()
                    .is_some_and(|digest| !digest.is_empty())
        }),
        "builtin request must carry its provider-neutral Runtime context"
    );
}

#[test]
fn runtime_background_play_replans_stale_provider_response_without_transport_retry() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    let recorded = Arc::new(Mutex::new(Vec::<RecordedHttpRequest>::new()));
    let decision_count = Arc::new(Mutex::new(0_usize));
    // A completed stale replan can overlap with the next async request: the
    // control pass may start decision #3 while it drains the durable Wait
    // feedback. Keep capacity for that in-flight request and its feedback
    // instead of making correctness depend on listener scheduling.
    let base_url = spawn_runtime_live_mock_http_server(8, {
        let recorded = Arc::clone(&recorded);
        let decision_count = Arc::clone(&decision_count);
        move |request| {
            recorded
                .lock()
                .expect("recorded lock")
                .push(request.clone());
            match (request.method.as_str(), request.path.as_str()) {
                ("POST", "/v1/world-simulator/decision-context") => {
                    let request_number = {
                        let mut decision_count = decision_count.lock().expect("count lock");
                        *decision_count += 1;
                        *decision_count
                    };
                    let decoded: crate::simulator::ContinuousAgentRequestContextV1 =
                        serde_json::from_slice(request.body.as_slice())
                            .expect("decode outer decision request");
                    decoded
                        .validate_production_lane()
                        .expect("stale test request must be complete");
                    let decision = if request_number == 1 {
                        crate::simulator::ProviderDecision::Act {
                            action_ref: "move_agent".to_string(),
                            action: crate::simulator::Action::MoveAgent {
                                agent_id: decoded
                                    .base_decision_request
                                    .observation
                                    .agent_id
                                    .clone(),
                                to: decoded
                                    .base_decision_request
                                    .observation
                                    .observation
                                    .self_state
                                    .location_ref
                                    .clone(),
                            },
                        }
                    } else {
                        crate::simulator::ProviderDecision::Wait
                    };
                    let response = crate::simulator::DecisionResponse {
                        decision,
                        module_command: None,
                        provider_error: None,
                        diagnostics: crate::simulator::ProviderDiagnostics::default(),
                        trace_payload: crate::simulator::ProviderTraceEnvelope::default(),
                        memory_write_intents: Vec::new(),
                    };
                    MockHttpResponse {
                        status_code: 200,
                        body: serde_json::to_string(&provider_context_response(&decoded, response))
                            .expect("encode decision response"),
                    }
                }
                ("POST", "/v1/world-simulator/feedback-context") => {
                    let feedback: crate::simulator::FeedbackEnvelopeV1 =
                        serde_json::from_slice(request.body.as_slice())
                            .expect("decode stale Runtime feedback");
                    assert!(
                        matches!(
                            (feedback.status.as_str(), feedback.reject_reason.as_deref()),
                            ("rejected", Some("stale_base"))
                                | ("pending", Some(_))
                                | ("rejected", Some("no_effect"))
                        ),
                        "unexpected provider feedback: {feedback:?}"
                    );
                    if feedback.status != "pending" {
                        assert!(feedback.runtime_receipt_id.is_none());
                    }
                    MockHttpResponse {
                        status_code: 200,
                        body: serde_json::json!({"ok": true}).to_string(),
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
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_URL_ENV, base_url);
        oasis7::env_mut::set_var(VIEWER_AGENT_PROVIDER_PROFILE_ENV, "oasis7_p0_low_freq_npc");
        oasis7::env_mut::set_var(VIEWER_AGENT_EXECUTION_LANE_ENV, "player_parity");
    }

    let test_world_id = format!("live-runtime-{}", WorldScenario::Minimal.as_str());
    let test_finality_block_hash =
        crate::simulator::h_v1("oasis7.viewer.test.finality-block.v1", &test_world_id).to_string();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm)
            .with_test_cognition_runtime_binding(
                "main",
                0,
                Some(test_finality_block_hash),
                "verified",
                0,
            ),
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
    let (mut writer, _client) = test_writer_pair();
    let mut session = RuntimeLiveSession::new();
    session.playing = true;

    let mut stale_feedback_seen = false;
    let mut replan_request_seen = false;
    let poll_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < poll_deadline {
        server
            .advance_runtime(&mut session, &mut writer, "play", 1, None, false)
            .expect("stale provider response should be handled");
        let recorded = recorded.lock().expect("recorded lock");
        let decisions: Vec<crate::simulator::ContinuousAgentRequestContextV1> = recorded
            .iter()
            .filter(|request| request.path == "/v1/world-simulator/decision-context")
            .map(|request| {
                serde_json::from_slice(request.body.as_slice()).expect("decode decision request")
            })
            .collect();
        stale_feedback_seen = recorded
            .iter()
            .filter(|request| request.path == "/v1/world-simulator/feedback-context")
            .map(|request| {
                serde_json::from_slice::<crate::simulator::FeedbackEnvelopeV1>(
                    request.body.as_slice(),
                )
                .expect("decode feedback")
            })
            .any(|feedback| {
                feedback.status == "rejected"
                    && feedback.reject_reason.as_deref() == Some("stale_base")
            });
        let wait_feedback_seen = recorded
            .iter()
            .filter(|request| request.path == "/v1/world-simulator/feedback-context")
            .map(|request| {
                serde_json::from_slice::<crate::simulator::FeedbackEnvelopeV1>(
                    request.body.as_slice(),
                )
                .expect("decode wait feedback")
            })
            .any(|feedback| {
                feedback.status == "pending"
                    && feedback.reject_reason.as_deref() == Some("retry_scheduled")
            });
        replan_request_seen = decisions.len() >= 2;
        if stale_feedback_seen && replan_request_seen && wait_feedback_seen {
            break;
        }
        drop(recorded);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    // No provider work remains after the bounded poll. Release the process
    // environment lock before inspecting the recorded evidence so a later
    // assertion failure cannot poison the shared test lock and cascade.
    clear_runtime_provider_env();
    drop(_guard);
    assert!(
        stale_feedback_seen,
        "stale response must produce typed feedback"
    );
    assert!(
        replan_request_seen,
        "stale response must schedule a new request"
    );
    let recorded = recorded.lock().expect("recorded lock");
    let decisions: Vec<crate::simulator::ContinuousAgentRequestContextV1> = recorded
        .iter()
        .filter(|request| request.path == "/v1/world-simulator/decision-context")
        .map(|request| {
            serde_json::from_slice(request.body.as_slice()).expect("decode decision request")
        })
        .collect();
    assert_eq!(
        session.transient_play_failures,
        0,
        "stale replan left transient failures; decisions={}",
        decisions.len()
    );
    assert!(decisions.len() >= 2);
    assert_ne!(decisions[0].agent_turn_id, decisions[1].agent_turn_id);
    assert_ne!(
        decisions[0].decision_request_id,
        decisions[1].decision_request_id
    );
    assert!(decisions[1].retry_seq > decisions[0].retry_seq);
    assert_eq!(decisions[1].transport_attempt, 1);
    let recent_events = &decisions[1]
        .base_decision_request
        .observation
        .recent_event_summary;
    assert!(
        recent_events
            .iter()
            .any(|summary| summary.contains("stale_base_replan")
                && summary.contains(decisions[0].decision_request_id.as_str())
                && summary.contains("replan_count=1")),
        "replan request must retain a bounded causal reference: {recent_events:?}"
    );
    let feedbacks: Vec<crate::simulator::FeedbackEnvelopeV1> = recorded
        .iter()
        .filter(|request| request.path == "/v1/world-simulator/feedback-context")
        .map(|request| serde_json::from_slice(request.body.as_slice()).expect("decode feedback"))
        .collect();
    assert!(
        feedbacks.iter().any(|feedback| {
            feedback.status == "pending"
                && feedback.reject_reason.as_deref() == Some("retry_scheduled")
        }),
        "a provider Wait turn must carry pending feedback: {feedbacks:?}"
    );
    assert!(
        server
            .world
            .cognition()
            .get("cognition_journal")
            .and_then(|journal| journal.get("events"))
            .and_then(serde_json::Value::as_array)
            .is_some_and(|events| {
                events.iter().any(|event| {
                    event.get("event_kind").and_then(serde_json::Value::as_str)
                        == Some("ContinuationScheduled")
                        && event.get("agent_id").and_then(serde_json::Value::as_str)
                            == Some("agent-0")
                })
            }),
        "a provider Wait turn must be durably scheduled by Runtime"
    );
    assert!(
        feedbacks.iter().all(|feedback| {
            feedback.reject_reason.as_deref()
                != Some("provider wait elapsed without a Runtime action")
        }),
        "provider Wait must not emit the legacy ad-hoc expiry reason: {feedbacks:?}"
    );
    assert!(!server.world.journal().events.iter().any(|event| matches!(
        event.body,
        crate::runtime::WorldEventBody::Domain(crate::runtime::DomainEvent::ActionAccepted { .. })
            | crate::runtime::WorldEventBody::EffectQueued(_)
            | crate::runtime::WorldEventBody::ReceiptAppended(_)
    )));
}
