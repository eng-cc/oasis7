//! Explicit diagnostic-path contract tests: no process environment mutations.
use super::*;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::Duration,
};
const SECRET: &str = "private-goal-and-key-must-not-escape";
fn body(stream: &mut TcpStream) {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut b = [0; 4096];
    let end = loop {
        let n = stream.read(&mut b).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&b[..n]);
        if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
    };
    let head = String::from_utf8_lossy(&bytes[..end]);
    let length: usize = head
        .lines()
        .find_map(|line| {
            let (name, v) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| v.trim().parse().unwrap())
        })
        .unwrap();
    while bytes.len() < end + length {
        let n = stream.read(&mut b).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&b[..n]);
    }
}
fn server(
    events: Vec<String>,
    delay: Duration,
) -> (String, Arc<AtomicUsize>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
    let calls = Arc::new(AtomicUsize::new(0));
    let counter = calls.clone();
    let worker = thread::spawn(move || {
        for event in events {
            let (mut stream, _) = listener.accept().unwrap();
            body(&mut stream);
            counter.fetch_add(1, Ordering::SeqCst);
            thread::sleep(delay);
            let body = format!("data: {event}\n\n");
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            );
        }
    });
    (endpoint, calls, worker)
}
fn diagnostic_client(endpoint: &str, timeout: u64) -> OpenAiChatCompletionClient {
    let config = ModelProviderConfig::from_values_for_agent(
        |key| match key {
            "OASIS7_LLM_BASE_URL" => Some(endpoint.into()),
            "OASIS7_LLM_API_KEY" => Some(SECRET.into()),
            "OASIS7_LLM_TIMEOUT_MS" => Some(timeout.to_string()),
            _ => None,
        },
        "agent",
    )
    .unwrap();
    let mut client = OpenAiChatCompletionClient::from_config(&config).unwrap();
    client.stream_diagnostics_enabled = true;
    client
}
fn request(max_model_calls: Option<u32>) -> LlmCompletionRequest {
    LlmCompletionRequest {
        model: "local".into(),
        system_prompt: "system".into(),
        user_prompt: SECRET.into(),
        debug_mode: false,
        max_model_calls,
    }
}
fn concurrency_error() -> String {
    serde_json::json!({"error":{"type":"rate_limit_error","message":format!("Concurrency limit exceeded {SECRET}")},"private":SECRET}).to_string()
}
fn completed() -> String {
    serde_json::json!({"type":"response.completed","sequence_number":1,"response":{
    "id":"response","object":"response","created_at":1,"completed_at":2,"model":"local",
    "output":[{"type":"function_call","call_id":"call","name":"agent_submit_decision","arguments":serde_json::json!({"decision":"wait"}).to_string()}],
    "status":"completed","parallel_tool_calls":false}}).to_string()
}
#[test]
fn diagnostic_decode_classification_retains_only_safe_structural_retry_marker() {
    use crate::openai_stream_diagnostics::sanitized_stream_decode_failure;
    let classified = sanitized_stream_decode_failure(&concurrency_error());
    assert!(is_concurrency_limit_error(&classified));
    assert!(!classified.contains(SECRET));
    for raw in [
        format!("invalid SSE {SECRET}"),
        serde_json::json!({"goal":SECRET}).to_string(),
    ] {
        let classified = sanitized_stream_decode_failure(&raw);
        assert!(!classified.contains(SECRET));
        assert!(!is_concurrency_limit_error(&classified));
    }
}
#[test]
fn diagnostic_legacy_concurrency_error_retries_once_but_native_budget_does_not() {
    let (endpoint, calls, worker) = server(vec![concurrency_error(), completed()], Duration::ZERO);
    let result =
        LlmCompletionClient::complete(&diagnostic_client(&endpoint, 2000), &request(None)).unwrap();
    assert!(result.output.contains("wait"));
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    worker.join().unwrap();
    let (endpoint, calls, worker) = server(vec![concurrency_error()], Duration::ZERO);
    let error =
        LlmCompletionClient::complete(&diagnostic_client(&endpoint, 2000), &request(Some(1)))
            .unwrap_err();
    assert!(matches!(error, LlmClientError::DecodeResponse { .. }));
    assert!(!error.to_string().contains(SECRET));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    worker.join().unwrap();
}
#[test]
fn diagnostic_timeout_keeps_standard_timeout_contract_and_redacted_metadata() {
    let (endpoint, calls, worker) = server(vec![completed()], Duration::from_millis(250));
    let error = LlmCompletionClient::complete(&diagnostic_client(&endpoint, 80), &request(None))
        .unwrap_err();
    let LlmClientError::Http { message } = error else {
        panic!("expected HTTP timeout")
    };
    assert!(
        message.contains("request timed out after 80ms"),
        "{message}"
    );
    assert!(message.contains("http_status=absent"));
    assert!(!message.contains(SECRET));
    assert!(!message.contains(&endpoint));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    worker.join().unwrap();
}
