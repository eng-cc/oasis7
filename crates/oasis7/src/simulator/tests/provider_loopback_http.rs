#![cfg(not(target_arch = "wasm32"))]

use super::*;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

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
fn provider_loopback_http_client_round_trips_info_health_decision_and_feedback() {
    let recorded = Arc::new(Mutex::new(Vec::<RecordedHttpRequest>::new()));
    let fixture = golden_decision_provider_fixtures()
        .into_iter()
        .next()
        .expect("golden fixture");
    let mut expected_request = fixture.request.clone();
    expected_request.agent_profile = Some("oasis7_p0_low_freq_npc".to_string());
    let expected_feedback = FeedbackEnvelope {
        action_id: 7,
        success: true,
        reject_reason: None,
        emitted_events: vec![],
        world_delta_summary: Some("agent moved to loc-2".to_string()),
    };
    let expected_response = DecisionResponse {
        decision: fixture.expected_decision,
        provider_error: None,
        diagnostics: ProviderDiagnostics {
            provider_id: Some("openclaw-local".to_string()),
            provider_version: Some("0.1.0".to_string()),
            latency_ms: Some(41),
            retry_count: 0,
        },
        trace_payload: ProviderTraceEnvelope {
            provider_id: Some("openclaw-local".to_string()),
            input_summary: Some("fixture=golden.move.visible_location.v1".to_string()),
            output_summary: Some("decision=move_agent(to=loc-2)".to_string()),
            latency_ms: Some(41),
            transcript: vec![ProviderTranscriptEntry {
                role: "assistant".to_string(),
                content: "move to loc-2".to_string(),
            }],
            tool_trace: vec!["selected visible location loc-2".to_string()],
            token_usage: Some(ProviderTokenUsage {
                prompt_tokens: Some(8),
                completion_tokens: Some(5),
                total_tokens: Some(13),
            }),
            cost_cents: Some(1),
            schema_repair_count: 0,
        },
        memory_write_intents: vec![MemoryWriteIntent {
            scope: "short_term".to_string(),
            summary: "agent prefers loc-2 when visible".to_string(),
            tags: vec!["movement".to_string()],
        }],
    };
    let expected_request_for_server = expected_request.clone();
    let expected_feedback_for_server = expected_feedback.clone();
    let response_clone = expected_response.clone();
    let feedback_ack = ProviderFeedbackAck {
        ok: true,
        error_code: None,
        error: None,
    };
    let base_url = spawn_mock_http_server(4, {
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
                        "provider_id": "openclaw-local",
                        "name": "OpenClaw",
                        "version": "0.1.0",
                        "protocol_version": "v1",
                        "capabilities": ["decision", "feedback"],
                        "supported_action_sets": ["phase1_low_frequency"]
                    })
                    .to_string(),
                },
                ("GET", "/v1/provider/health") => MockHttpResponse {
                    status_code: 200,
                    body: serde_json::json!({
                        "ok": true,
                        "status": "ready",
                        "uptime_ms": 1234,
                        "last_error": null,
                        "queue_depth": 0
                    })
                    .to_string(),
                },
                ("POST", "/v1/world-simulator/decision") => {
                    let decoded: DecisionRequest = serde_json::from_slice(request.body.as_slice())
                        .expect("decode decision request");
                    assert_eq!(decoded, expected_request_for_server);
                    MockHttpResponse {
                        status_code: 200,
                        body: serde_json::to_string(&response_clone)
                            .expect("encode decision response"),
                    }
                }
                ("POST", "/v1/world-simulator/feedback") => {
                    let decoded: FeedbackEnvelope = serde_json::from_slice(request.body.as_slice())
                        .expect("decode feedback request");
                    assert_eq!(decoded, expected_feedback_for_server);
                    MockHttpResponse {
                        status_code: 200,
                        body: serde_json::to_string(&feedback_ack).expect("encode feedback ack"),
                    }
                }
                _ => MockHttpResponse {
                    status_code: 404,
                    body: serde_json::json!({"ok": false, "error": "not_found"}).to_string(),
                },
            }
        }
    });

    let client = ProviderLoopbackHttpClient::new(base_url.as_str(), Some("secret-token"), 200)
        .expect("build client");
    let info = client.provider_info().expect("info");
    assert_eq!(info.provider_id, "openclaw-local");
    assert_eq!(info.version.as_deref(), Some("0.1.0"));
    assert_eq!(
        info.supported_action_sets,
        vec!["phase1_low_frequency".to_string()]
    );

    let health = client.provider_health().expect("health");
    assert!(health.ok);
    assert_eq!(health.status.as_deref(), Some("ready"));
    assert_eq!(health.queue_depth, Some(0));

    let decision = client
        .request_decision(&expected_request)
        .expect("decision");
    assert_eq!(decision, expected_response);

    let ack = client
        .submit_feedback(&expected_feedback)
        .expect("feedback ack");
    assert!(ack.ok);

    let recorded = recorded.lock().expect("recorded lock");
    assert_eq!(recorded.len(), 4);
    assert_eq!(recorded[0].method, "GET");
    assert_eq!(recorded[0].path, "/v1/provider/info");
    assert_eq!(
        recorded[0].headers.get("authorization").map(String::as_str),
        Some("Bearer secret-token")
    );
    assert_eq!(recorded[1].path, "/v1/provider/health");
    assert_eq!(recorded[2].path, "/v1/world-simulator/decision");
    assert_eq!(recorded[3].path, "/v1/world-simulator/feedback");
}

#[test]
fn provider_loopback_http_client_rejects_non_loopback_base_url() {
    let err = ProviderLoopbackHttpClient::new("http://192.168.0.5:5841", None, 200)
        .expect_err("non-loopback should fail");
    assert!(err.to_string().contains("loopback"));
}

#[test]
fn provider_loopback_http_client_surfaces_http_401_on_decision() {
    let base_url = spawn_mock_http_server(1, |_| MockHttpResponse {
        status_code: 401,
        body: "unauthorized".to_string(),
    });
    let client = ProviderLoopbackHttpClient::new(base_url.as_str(), Some("bad-token"), 200)
        .expect("build client");
    let request = golden_decision_provider_fixtures()
        .into_iter()
        .next()
        .expect("fixture")
        .request;

    let err = client
        .request_decision(&request)
        .expect_err("401 should surface");
    assert!(matches!(err, ProviderLoopbackHttpError::Unauthorized { .. }));
    assert!(err.to_string().contains("unauthorized"));
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
