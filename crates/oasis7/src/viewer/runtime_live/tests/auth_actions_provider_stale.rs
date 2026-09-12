use super::auth_actions::{
    MockHttpResponse, RecordedHttpRequest, provider_context_response,
    spawn_runtime_live_mock_http_server,
};
use super::*;
use std::sync::{Arc, Mutex};
#[test]
fn runtime_background_play_replans_stale_provider_response_without_transport_retry() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    let recorded = Arc::new(Mutex::new(Vec::<RecordedHttpRequest>::new()));
    let decision_count = Arc::new(Mutex::new(0_usize));
    // Keep capacity for the stale rejection and replan Wait feedback. The
    // bounded listener must outlive any request already in flight while the
    // test observes the durable result.
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

    let read_progress = |world: &crate::runtime::World| {
        let recorded = recorded.lock().expect("recorded lock");
        let decisions = recorded
            .iter()
            .filter(|request| request.path == "/v1/world-simulator/decision-context")
            .count();
        let stale_feedback_seen = recorded
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
        let wait_scheduled = world
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
            });
        (
            stale_feedback_seen,
            decisions >= 2,
            wait_feedback_seen,
            wait_scheduled,
        )
    };
    let poll_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while std::time::Instant::now() < poll_deadline {
        let (stale_feedback_seen, replan_request_seen, wait_feedback_seen, wait_scheduled) =
            read_progress(&server.world);
        if stale_feedback_seen && replan_request_seen {
            if wait_feedback_seen {
                break;
            }
            if wait_scheduled {
                // The provider outcome has already been admitted durably.
                // Wait for its feedback request without advancing into the
                // continuation's next Runtime turn.
                std::thread::sleep(std::time::Duration::from_millis(5));
                continue;
            }
            // The actor may still be completing its request. Continue
            // polling until the durable Wait admission is visible.
        }
        server
            .advance_runtime(&mut session, &mut writer, "play", 1, None, false)
            .expect("stale provider response should be handled");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let (stale_feedback_seen, replan_request_seen, wait_feedback_seen, _wait_scheduled) =
        read_progress(&server.world);
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
