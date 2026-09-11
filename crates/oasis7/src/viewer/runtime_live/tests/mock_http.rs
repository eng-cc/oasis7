use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(super) struct RecordedHttpRequest {
    pub(super) method: String,
    pub(super) path: String,
    pub(super) headers: BTreeMap<String, String>,
    pub(super) body: Vec<u8>,
}

#[derive(Debug, Clone)]
pub(super) struct MockHttpResponse {
    pub(super) status_code: u16,
    pub(super) body: String,
}

pub(super) fn provider_context_response(
    context: &crate::simulator::ContinuousAgentRequestContextV1,
    response: crate::simulator::DecisionResponse,
) -> crate::simulator::ContinuousAgentResponseContextV1 {
    crate::simulator::ContinuousAgentResponseContextV1 {
        response_digest: crate::simulator::cognition_response_digest(&response),
        base_decision_response: response,
        context_discriminator: crate::simulator::CONTINUOUS_AGENT_CONTEXT_DISCRIMINATOR.to_string(),
        context_version: crate::simulator::CONTINUOUS_AGENT_CONTEXT_VERSION,
        agent_session_id: context.agent_session_id.clone(),
        agent_turn_id: context.agent_turn_id.clone(),
        decision_request_id: context.decision_request_id.clone(),
        retry_seq: context.retry_seq,
        transport_attempt: context.transport_attempt,
        request_digest: context.request_digest.clone(),
    }
}

pub(super) fn spawn_runtime_live_mock_http_server<F>(
    expected_connections: usize,
    handler: F,
) -> String
where
    F: Fn(RecordedHttpRequest) -> MockHttpResponse + Send + Sync + 'static,
{
    spawn_runtime_live_mock_http_server_inner(expected_connections, handler, false)
}

/// Spawn a mock that lets the handler observe the provider capability probes.
/// Most runtime-live fixtures only serve POST requests.  An unrelated
/// compatibility snapshot can still issue a GET against a fixture whose
/// provider URL it observes while another test owns the environment lock.  A
/// default 404 for those GETs keeps that connection isolated instead of
/// feeding an empty body to a decision decoder.  The agent-chat fixtures opt
/// into handling the two provider probe routes explicitly.
pub(super) fn spawn_runtime_live_mock_http_server_with_provider_probes<F>(
    expected_connections: usize,
    handler: F,
) -> String
where
    F: Fn(RecordedHttpRequest) -> MockHttpResponse + Send + Sync + 'static,
{
    spawn_runtime_live_mock_http_server_inner(expected_connections, handler, true)
}

fn spawn_runtime_live_mock_http_server_inner<F>(
    expected_connections: usize,
    handler: F,
    pass_provider_probes_to_handler: bool,
) -> String
where
    F: Fn(RecordedHttpRequest) -> MockHttpResponse + Send + Sync + 'static,
{
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind mock http server");
    let bind = listener.local_addr().expect("listener addr");
    let handler = Arc::new(handler);
    std::thread::spawn(move || {
        let mut handled_connections = 0;
        while handled_connections < expected_connections {
            let (mut stream, _) = listener.accept().expect("accept mock request");
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
            let Some(request) = read_runtime_live_http_request(&mut stream) else {
                continue;
            };
            if request.method == "GET" && !pass_provider_probes_to_handler {
                write_runtime_live_json_response(
                    &mut stream,
                    404,
                    r#"{"ok":false,"error":"mock route unavailable"}"#,
                );
                continue;
            }
            handled_connections += 1;
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

fn read_runtime_live_http_request(stream: &mut std::net::TcpStream) -> Option<RecordedHttpRequest> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    let mut header_end = None;
    let mut content_length = 0_usize;

    loop {
        let bytes = match stream.read(&mut chunk) {
            Ok(bytes) => bytes,
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::Interrupted
                        | std::io::ErrorKind::TimedOut
                        | std::io::ErrorKind::WouldBlock
                ) =>
            {
                return None;
            }
            Err(_) => return None,
        };
        if bytes == 0 {
            return None;
        }
        buffer.extend_from_slice(&chunk[..bytes]);
        if header_end.is_none() {
            header_end = find_runtime_live_header_terminator(buffer.as_slice());
            if let Some(boundary) = header_end {
                let header = std::str::from_utf8(&buffer[..boundary]).ok()?;
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

    let boundary = header_end?;
    let header = std::str::from_utf8(&buffer[..boundary]).ok()?;
    let mut lines = header.lines();
    let request_line = lines.next()?;
    let mut request_line_parts = request_line.split_whitespace();
    let method = request_line_parts.next()?.to_string();
    let path = request_line_parts.next()?.to_string();
    let mut headers = BTreeMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    let body = buffer[(boundary + 4)..(boundary + 4 + content_length)].to_vec();

    Some(RecordedHttpRequest {
        method,
        path,
        headers,
        body,
    })
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
    let _ = stream.write_all(response.as_bytes());
}
