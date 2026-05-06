#![cfg(not(target_arch = "wasm32"))]

use super::*;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

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
fn provider_loopback_adapter_decides_and_pushes_feedback_via_local_http() {
    let fixture = golden_decision_provider_fixtures()
        .into_iter()
        .next()
        .expect("fixture");
    let mut request = fixture.request.clone();
    request.agent_profile = Some("oasis7_p0_low_freq_npc".to_string());
    let feedback = FeedbackEnvelope {
        action_id: 9,
        success: true,
        reject_reason: None,
        emitted_events: vec![],
        world_delta_summary: Some("move completed".to_string()),
    };
    let response = DecisionResponse {
        decision: fixture.expected_decision,
        provider_error: None,
        diagnostics: ProviderDiagnostics {
            provider_id: Some("provider_local_bridge".to_string()),
            provider_version: Some("0.1.0".to_string()),
            latency_ms: Some(37),
            retry_count: 0,
        },
        trace_payload: ProviderTraceEnvelope {
            provider_id: Some("provider_local_bridge".to_string()),
            output_summary: Some("decision=move_agent(to=loc-2)".to_string()),
            latency_ms: Some(37),
            ..ProviderTraceEnvelope::default()
        },
        memory_write_intents: vec![],
    };
    let expected_response = response.clone();
    let request_for_server = request.clone();
    let feedback_for_server = feedback.clone();
    let base_url = spawn_mock_http_server(2, move |incoming| {
        match (incoming.method.as_str(), incoming.path.as_str()) {
            ("POST", "/v1/world-simulator/decision") => {
                let decoded: DecisionRequest = serde_json::from_slice(incoming.body.as_slice())
                    .expect("decode decision request");
                assert_eq!(decoded, request_for_server);
                assert_eq!(
                    incoming.headers.get("authorization").map(String::as_str),
                    Some("Bearer secret-token")
                );
                MockHttpResponse {
                    status_code: 200,
                    body: serde_json::to_string(&expected_response).expect("encode response"),
                }
            }
            ("POST", "/v1/world-simulator/feedback") => {
                let decoded: FeedbackEnvelope = serde_json::from_slice(incoming.body.as_slice())
                    .expect("decode feedback request");
                assert_eq!(decoded, feedback_for_server);
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
    });

    let mut adapter = ProviderLoopbackAdapter::new(base_url.as_str(), Some("secret-token"), 200)
        .expect("adapter");
    assert_eq!(adapter.provider_id(), "provider_loopback_http");

    let decided = adapter.decide(&request).expect("decision response");
    assert_eq!(decided, response);
    adapter.push_feedback(&feedback).expect("feedback ack");
}

#[test]
fn provider_loopback_adapter_rejects_action_ref_outside_phase1_whitelist() {
    let request = golden_decision_provider_fixtures()
        .into_iter()
        .next()
        .expect("fixture")
        .request;
    let disallowed_response = DecisionResponse {
        decision: ProviderDecision::Act {
            action_ref: "build_factory".to_string(),
            action: Action::BuildFactory {
                owner: ResourceOwner::Agent {
                    agent_id: "agent-1".to_string(),
                },
                location_id: "loc-1".to_string(),
                factory_id: "fac-1".to_string(),
                factory_kind: "basic".to_string(),
            },
        },
        provider_error: None,
        diagnostics: ProviderDiagnostics::default(),
        trace_payload: ProviderTraceEnvelope::default(),
        memory_write_intents: vec![],
    };
    let base_url = spawn_mock_http_server(1, move |_| MockHttpResponse {
        status_code: 200,
        body: serde_json::to_string(&disallowed_response).expect("encode response"),
    });

    let mut adapter = ProviderLoopbackAdapter::new(base_url.as_str(), None, 200).expect("adapter");
    let err = adapter.decide(&request).expect_err("should reject action");
    assert_eq!(err.code, "action_ref_not_in_catalog");
}

#[test]
fn provider_loopback_adapter_maps_provider_error_envelope_to_decision_provider_error() {
    let request = golden_decision_provider_fixtures()
        .into_iter()
        .next()
        .expect("fixture")
        .request;
    let provider_error_response = DecisionResponse {
        decision: ProviderDecision::Wait,
        provider_error: Some(ProviderErrorEnvelope {
            code: "provider_timeout".to_string(),
            message: "request exceeded 200ms budget".to_string(),
            retryable: true,
        }),
        diagnostics: ProviderDiagnostics::default(),
        trace_payload: ProviderTraceEnvelope::default(),
        memory_write_intents: vec![],
    };
    let base_url = spawn_mock_http_server(1, move |_| MockHttpResponse {
        status_code: 200,
        body: serde_json::to_string(&provider_error_response).expect("encode response"),
    });

    let mut adapter = ProviderLoopbackAdapter::new(base_url.as_str(), None, 200).expect("adapter");
    let err = adapter
        .decide(&request)
        .expect_err("provider error should surface");
    assert_eq!(err.code, "provider_timeout");
    assert!(err.retryable);
}

#[test]
fn provider_loopback_adapter_rejects_wait_ticks_when_request_catalog_omits_it() {
    let mut request = golden_decision_provider_fixtures()
        .into_iter()
        .next()
        .expect("fixture")
        .request;
    request
        .observation
        .action_catalog
        .retain(|entry| entry.action_ref != "wait_ticks");
    let wait_ticks_response = DecisionResponse {
        decision: ProviderDecision::WaitTicks { ticks: 2 },
        provider_error: None,
        diagnostics: ProviderDiagnostics::default(),
        trace_payload: ProviderTraceEnvelope::default(),
        memory_write_intents: vec![],
    };
    let base_url = spawn_mock_http_server(1, move |_| MockHttpResponse {
        status_code: 200,
        body: serde_json::to_string(&wait_ticks_response).expect("encode response"),
    });

    let mut adapter = ProviderLoopbackAdapter::new(base_url.as_str(), None, 200).expect("adapter");
    let err = adapter
        .decide(&request)
        .expect_err("wait_ticks outside action_catalog should be rejected");
    assert_eq!(err.code, "action_ref_not_in_catalog");
    assert!(err.message.contains("wait_ticks"));
}

fn setup_kernel_with_provider_agent(agent_id: &str) -> WorldKernel {
    let mut kernel = WorldKernel::with_config(WorldConfig {
        move_cost_per_km_electricity: 0,
        ..Default::default()
    });
    kernel.submit_action(Action::RegisterLocation {
        location_id: "loc-1".to_string(),
        name: "base".to_string(),
        pos: pos(0, 0),
        profile: LocationProfile::default(),
    });
    kernel.submit_action(Action::RegisterLocation {
        location_id: "loc-2".to_string(),
        name: "neighbor".to_string(),
        pos: pos(100, 0),
        profile: LocationProfile::default(),
    });
    kernel.submit_action(Action::RegisterAgent {
        agent_id: agent_id.to_string(),
        location_id: "loc-1".to_string(),
    });
    kernel.step_until_empty();
    kernel
}

fn provider_phase1_action_catalog() -> Vec<ActionCatalogEntry> {
    vec![
        ActionCatalogEntry::new("wait", "Skip this tick"),
        ActionCatalogEntry::new("wait_ticks", "Pause for fixed ticks"),
        ActionCatalogEntry::new("move_agent", "Move the acting agent to a visible location"),
        ActionCatalogEntry::new("speak_to_nearby", "Emit a lightweight nearby speech event"),
        ActionCatalogEntry::new("inspect_target", "Inspect a nearby world target"),
        ActionCatalogEntry::new(
            "simple_interact",
            "Perform a lightweight interaction with a target",
        ),
    ]
}

#[test]
fn provider_backed_behavior_executes_provider_loopback_adapter_move_and_records_feedback() {
    let mut kernel = setup_kernel_with_provider_agent("agent-1");
    let response = DecisionResponse {
        decision: ProviderDecision::Act {
            action_ref: "move_agent".to_string(),
            action: Action::MoveAgent {
                agent_id: "agent-1".to_string(),
                to: "loc-2".to_string(),
            },
        },
        provider_error: None,
        diagnostics: ProviderDiagnostics {
            provider_id: Some("provider_local_bridge".to_string()),
            provider_version: Some("0.1.0".to_string()),
            latency_ms: Some(33),
            retry_count: 0,
        },
        trace_payload: ProviderTraceEnvelope {
            provider_id: Some("provider_local_bridge".to_string()),
            output_summary: Some("decision=move_agent(to=loc-2)".to_string()),
            latency_ms: Some(33),
            ..ProviderTraceEnvelope::default()
        },
        memory_write_intents: vec![],
    };
    let response_for_server = response.clone();
    let base_url = spawn_mock_http_server(2, move |incoming| {
        match (incoming.method.as_str(), incoming.path.as_str()) {
            ("POST", "/v1/world-simulator/decision") => {
                let decoded: DecisionRequest = serde_json::from_slice(incoming.body.as_slice())
                    .expect("decode decision request");
                assert_eq!(
                    decoded.agent_profile.as_deref(),
                    Some("oasis7_p0_low_freq_npc")
                );
                assert_eq!(
                    decoded.observation.mode,
                    ProviderExecutionMode::HeadlessAgent
                );
                assert_eq!(
                    decoded.observation.observation_schema_version,
                    DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION
                );
                assert_eq!(
                    decoded.observation.action_schema_version,
                    DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION
                );
                assert_eq!(
                    decoded.observation.environment_class.as_deref(),
                    Some("adapter_test")
                );
                assert_eq!(decoded.fixture_id.as_deref(), Some("fixture.adapter.move"));
                assert_eq!(decoded.replay_id.as_deref(), Some("replay.adapter.move"));
                MockHttpResponse {
                    status_code: 200,
                    body: serde_json::to_string(&response_for_server)
                        .expect("encode decision response"),
                }
            }
            ("POST", "/v1/world-simulator/feedback") => MockHttpResponse {
                status_code: 200,
                body: serde_json::json!({"ok": true}).to_string(),
            },
            _ => MockHttpResponse {
                status_code: 404,
                body: serde_json::json!({"ok": false, "error": "not_found"}).to_string(),
            },
        }
    });

    let adapter = ProviderLoopbackAdapter::new(base_url.as_str(), None, 200).expect("adapter");
    let behavior =
        ProviderBackedAgentBehavior::new("agent-1", adapter, provider_phase1_action_catalog())
            .with_provider_config_ref("provider://loopback-http")
            .with_agent_profile("oasis7_p0_low_freq_npc")
            .with_environment_class("adapter_test")
            .with_fixture_id("fixture.adapter.move")
            .with_replay_id("replay.adapter.move")
            .with_memory_summary("goal=move");
    let mut runner: AgentRunner<ProviderBackedAgentBehavior<ProviderLoopbackAdapter>> =
        AgentRunner::new();
    runner.register(behavior);

    let tick = runner.tick(&mut kernel).expect("runner tick");
    assert!(matches!(
        tick.decision,
        AgentDecision::Act(Action::MoveAgent { .. })
    ));
    assert!(tick.is_success());
    let trace = tick.decision_trace.expect("decision trace");
    assert_eq!(
        trace.llm_diagnostics.and_then(|value| value.latency_ms),
        Some(33)
    );
    assert!(trace
        .llm_output
        .as_deref()
        .unwrap_or_default()
        .contains("move_agent"));

    let agent = kernel.model().agents.get("agent-1").expect("agent exists");
    assert_eq!(agent.location_id, "loc-2");
}

#[test]
fn provider_backed_behavior_executes_provider_loopback_adapter_speak_action() {
    let mut kernel = setup_kernel_with_provider_agent("agent-1");
    let response = DecisionResponse {
        decision: ProviderDecision::Act {
            action_ref: "speak_to_nearby".to_string(),
            action: Action::SpeakToNearby {
                agent_id: "agent-1".to_string(),
                message: "hello nearby".to_string(),
                target_agent_id: None,
            },
        },
        provider_error: None,
        diagnostics: ProviderDiagnostics::default(),
        trace_payload: ProviderTraceEnvelope::default(),
        memory_write_intents: vec![],
    };
    let base_url = spawn_mock_http_server(2, move |incoming| {
        match (incoming.method.as_str(), incoming.path.as_str()) {
            ("POST", "/v1/world-simulator/decision") => MockHttpResponse {
                status_code: 200,
                body: serde_json::to_string(&response).expect("encode response"),
            },
            ("POST", "/v1/world-simulator/feedback") => MockHttpResponse {
                status_code: 200,
                body: serde_json::json!({"ok": true}).to_string(),
            },
            _ => MockHttpResponse {
                status_code: 404,
                body: serde_json::json!({"ok": false, "error": "not_found"}).to_string(),
            },
        }
    });

    let adapter = ProviderLoopbackAdapter::new(base_url.as_str(), None, 200).expect("adapter");
    let behavior =
        ProviderBackedAgentBehavior::new("agent-1", adapter, provider_phase1_action_catalog());
    let mut runner: AgentRunner<ProviderBackedAgentBehavior<ProviderLoopbackAdapter>> =
        AgentRunner::new();
    runner.register(behavior);

    let tick = runner.tick(&mut kernel).expect("runner tick");
    assert!(tick.is_success());
    let action_result = tick.action_result.expect("action result");
    assert!(matches!(
        action_result.event.kind,
        WorldEventKind::AgentSpoke { .. }
    ));
}

#[test]
fn provider_backed_behavior_executes_provider_loopback_adapter_inspect_action() {
    let mut kernel = setup_kernel_with_provider_agent("agent-1");
    let response = DecisionResponse {
        decision: ProviderDecision::Act {
            action_ref: "inspect_target".to_string(),
            action: Action::InspectTarget {
                agent_id: "agent-1".to_string(),
                target_kind: "location".to_string(),
                target_id: "loc-1".to_string(),
            },
        },
        provider_error: None,
        diagnostics: ProviderDiagnostics::default(),
        trace_payload: ProviderTraceEnvelope::default(),
        memory_write_intents: vec![],
    };
    let base_url = spawn_mock_http_server(2, move |incoming| {
        match (incoming.method.as_str(), incoming.path.as_str()) {
            ("POST", "/v1/world-simulator/decision") => MockHttpResponse {
                status_code: 200,
                body: serde_json::to_string(&response).expect("encode response"),
            },
            ("POST", "/v1/world-simulator/feedback") => MockHttpResponse {
                status_code: 200,
                body: serde_json::json!({"ok": true}).to_string(),
            },
            _ => MockHttpResponse {
                status_code: 404,
                body: serde_json::json!({"ok": false, "error": "not_found"}).to_string(),
            },
        }
    });

    let adapter = ProviderLoopbackAdapter::new(base_url.as_str(), None, 200).expect("adapter");
    let behavior =
        ProviderBackedAgentBehavior::new("agent-1", adapter, provider_phase1_action_catalog());
    let mut runner: AgentRunner<ProviderBackedAgentBehavior<ProviderLoopbackAdapter>> =
        AgentRunner::new();
    runner.register(behavior);

    let tick = runner.tick(&mut kernel).expect("runner tick");
    assert!(tick.is_success());
    let action_result = tick.action_result.expect("action result");
    assert!(matches!(
        action_result.event.kind,
        WorldEventKind::TargetInspected { .. }
    ));
}

#[test]
fn provider_backed_behavior_executes_provider_loopback_adapter_simple_interact_action() {
    let mut kernel = setup_kernel_with_provider_agent("agent-1");
    let response = DecisionResponse {
        decision: ProviderDecision::Act {
            action_ref: "simple_interact".to_string(),
            action: Action::SimpleInteract {
                agent_id: "agent-1".to_string(),
                target_kind: "location".to_string(),
                target_id: "loc-1".to_string(),
                interaction: "press_console".to_string(),
            },
        },
        provider_error: None,
        diagnostics: ProviderDiagnostics::default(),
        trace_payload: ProviderTraceEnvelope::default(),
        memory_write_intents: vec![],
    };
    let base_url = spawn_mock_http_server(2, move |incoming| {
        match (incoming.method.as_str(), incoming.path.as_str()) {
            ("POST", "/v1/world-simulator/decision") => MockHttpResponse {
                status_code: 200,
                body: serde_json::to_string(&response).expect("encode response"),
            },
            ("POST", "/v1/world-simulator/feedback") => MockHttpResponse {
                status_code: 200,
                body: serde_json::json!({"ok": true}).to_string(),
            },
            _ => MockHttpResponse {
                status_code: 404,
                body: serde_json::json!({"ok": false, "error": "not_found"}).to_string(),
            },
        }
    });

    let adapter = ProviderLoopbackAdapter::new(base_url.as_str(), None, 200).expect("adapter");
    let behavior =
        ProviderBackedAgentBehavior::new("agent-1", adapter, provider_phase1_action_catalog());
    let mut runner: AgentRunner<ProviderBackedAgentBehavior<ProviderLoopbackAdapter>> =
        AgentRunner::new();
    runner.register(behavior);

    let tick = runner.tick(&mut kernel).expect("runner tick");
    assert!(tick.is_success());
    let action_result = tick.action_result.expect("action result");
    assert!(matches!(
        action_result.event.kind,
        WorldEventKind::SimpleInteractionPerformed { .. }
    ));
}

#[test]
fn provider_backed_behavior_downgrades_provider_loopback_adapter_unsupported_semantics_to_wait() {
    let mut kernel = setup_kernel_with_provider_agent("agent-1");
    let response = DecisionResponse {
        decision: ProviderDecision::Act {
            action_ref: "simple_interact".to_string(),
            action: Action::MoveAgent {
                agent_id: "agent-1".to_string(),
                to: "loc-2".to_string(),
            },
        },
        provider_error: None,
        diagnostics: ProviderDiagnostics::default(),
        trace_payload: ProviderTraceEnvelope::default(),
        memory_write_intents: vec![],
    };
    let base_url = spawn_mock_http_server(1, move |_| MockHttpResponse {
        status_code: 200,
        body: serde_json::to_string(&response).expect("encode response"),
    });

    let adapter = ProviderLoopbackAdapter::new(base_url.as_str(), None, 200).expect("adapter");
    let behavior =
        ProviderBackedAgentBehavior::new("agent-1", adapter, provider_phase1_action_catalog());
    let mut runner: AgentRunner<ProviderBackedAgentBehavior<ProviderLoopbackAdapter>> =
        AgentRunner::new();
    runner.register(behavior);

    let tick = runner.tick(&mut kernel).expect("runner tick");
    assert!(matches!(tick.decision, AgentDecision::Wait));
    assert!(tick.action_result.is_none());
    let trace = tick.decision_trace.expect("error trace emitted");
    assert!(trace
        .llm_error
        .as_deref()
        .unwrap_or_default()
        .contains("action_ref_mismatch"));
}

fn spawn_mock_http_server<F>(expected_connections: usize, handler: F) -> String
where
    F: Fn(RecordedHttpRequest) -> MockHttpResponse + Send + Sync + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock http server");
    let bind = listener.local_addr().expect("listener addr");
    let handler = Arc::new(handler);
    std::thread::spawn(move || {
        for _ in 0..expected_connections {
            let (mut stream, _) = listener.accept().expect("accept mock request");
            let request = read_http_request(&mut stream);
            let response = handler(request);
            write_json_response(&mut stream, response.status_code, response.body.as_str());
        }
    });
    format!("http://{}", bind)
}

fn read_http_request(stream: &mut TcpStream) -> RecordedHttpRequest {
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
            header_end = find_header_terminator(buffer.as_slice());
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

fn find_header_terminator(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn write_json_response(stream: &mut TcpStream, status_code: u16, body: &str) {
    let status_text = match status_code {
        200 => "OK",
        401 => "Unauthorized",
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
