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
    let decision_count = Arc::new(Mutex::new(0_usize));
    let base_url = spawn_runtime_live_mock_http_server(7, {
        let recorded = Arc::clone(&recorded);
        let decision_count = Arc::clone(&decision_count);
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
                        memory_write_intents: Vec::new(),
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
                        assert!(matches!(
                            (feedback.status.as_str(), feedback.reject_reason.as_deref()),
                            ("pending", Some(_)) | ("rejected", Some("no_effect"))
                        ));
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

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let expected_world_id = server.config.world_id.clone();
    let finality_block_hash =
        crate::simulator::h_v1("oasis7.viewer.test.finality-block.v1", &expected_world_id)
            .to_string();
    server
        .world
        .bind_cognition_runtime(
            expected_world_id.as_str(),
            "provider-context-branch",
            0,
            Some(finality_block_hash.clone()),
            "verified",
            0,
        )
        .expect("bind authoritative runtime cognition context");
    server
        .world
        .install_test_provider_capability_fixture("agent-0")
        .expect("install Runtime provider capability fixture");
    let baseline_event_seq = latest_runtime_event_seq(&server.world);
    let mut first_decision_observed = false;
    for _ in 0..64 {
        server.llm_sidecar.request_decision();
        match server.enqueue_llm_action_from_sidecar() {
            Ok(Some(_)) => {
                first_decision_observed = true;
                break;
            }
            Ok(None) | Err(_) => std::thread::yield_now(),
        }
    }
    assert!(
        first_decision_observed,
        "provider-backed action should complete without advancing the World head"
    );

    let mut transient_failure_observed = false;
    for _ in 0..64 {
        server.llm_sidecar.request_decision();
        if let Err(trace) = server.enqueue_llm_action_from_sidecar() {
            assert!(
                decision_trace_provider_error_retryable(&trace).unwrap_or(false),
                "second provider response should be retryable: {trace:?}"
            );
            transient_failure_observed = true;
            break;
        }
        std::thread::yield_now();
    }
    assert!(
        transient_failure_observed,
        "transient provider failure should surface"
    );

    let mut retry_decision_observed = false;
    for _ in 0..64 {
        server.llm_sidecar.request_decision();
        match server.enqueue_llm_action_from_sidecar() {
            Ok(Some(_)) => {
                retry_decision_observed = true;
                break;
            }
            Ok(None) | Err(_) => std::thread::yield_now(),
        }
    }
    assert!(
        retry_decision_observed,
        "transport retry should complete with the preserved provider turn"
    );
    assert!(
        latest_runtime_event_seq(&server.world) > baseline_event_seq,
        "provider-backed Runtime commit should append an authoritative event"
    );

    let recorded = recorded.lock().expect("recorded lock");
    let decision_records: Vec<&RecordedHttpRequest> = recorded
        .iter()
        .filter(|request| request.path == "/v1/world-simulator/decision-context")
        .collect();
    let feedback_records: Vec<&RecordedHttpRequest> = recorded
        .iter()
        .filter(|request| request.path == "/v1/world-simulator/feedback-context")
        .collect();
    assert_eq!(decision_records.len(), 3);
    assert_eq!(
        feedback_records.len(),
        2,
        "recorded paths: {:?}",
        recorded
            .iter()
            .map(|request| request.path.as_str())
            .collect::<Vec<_>>()
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
            .map(|hash| hash.as_str()),
        Some(finality_block_hash.as_str())
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
    drop(recorded);
    let baseline_time = server.world.state().time;
    let (mut writer, _client) = test_writer_pair();
    let mut session = RuntimeLiveSession::new();
    server
        .advance_runtime(&mut session, &mut writer, "step", 2, None, true)
        .expect("multi-step control should advance each requested iteration");
    assert_eq!(
        server.world.state().time,
        baseline_time + 2,
        "Step {{ count: 2 }} must not lose the second tick after the first iteration"
    );
    clear_runtime_provider_env();
}

#[test]
fn runtime_background_play_replans_stale_provider_response_without_transport_retry() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    let recorded = Arc::new(Mutex::new(Vec::<RecordedHttpRequest>::new()));
    let decision_count = Arc::new(Mutex::new(0_usize));
    let base_url = spawn_runtime_live_mock_http_server(5, {
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
                    assert!(matches!(
                        (feedback.status.as_str(), feedback.reject_reason.as_deref()),
                        ("rejected", Some("stale_base"))
                            | ("pending", Some(_))
                            | ("rejected", Some("no_effect"))
                    ));
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

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let world_id = server.config.world_id.clone();
    let finality_block_hash =
        crate::simulator::h_v1("oasis7.viewer.test.finality-block.v1", &world_id).to_string();
    server
        .world
        .bind_cognition_runtime(
            world_id,
            "main",
            0,
            Some(finality_block_hash),
            "verified",
            0,
        )
        .expect("bind Runtime cognition context");
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
                feedback.status == "rejected"
                    && feedback.reject_reason.as_deref() == Some("no_effect")
            });
        replan_request_seen = decisions.len() >= 2;
        if stale_feedback_seen && replan_request_seen && wait_feedback_seen {
            break;
        }
        drop(recorded);
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        stale_feedback_seen,
        "stale response must produce typed feedback"
    );
    assert!(
        replan_request_seen,
        "stale response must schedule a new request"
    );
    assert_eq!(session.transient_play_failures, 0);

    let recorded = recorded.lock().expect("recorded lock");
    let decisions: Vec<crate::simulator::ContinuousAgentRequestContextV1> = recorded
        .iter()
        .filter(|request| request.path == "/v1/world-simulator/decision-context")
        .map(|request| {
            serde_json::from_slice(request.body.as_slice()).expect("decode decision request")
        })
        .collect();
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
            feedback.status == "rejected" && feedback.reject_reason.as_deref() == Some("no_effect")
        }),
        "a completed provider Wait turn must close as canonical no_effect: {feedbacks:?}"
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
    clear_runtime_provider_env();
}
