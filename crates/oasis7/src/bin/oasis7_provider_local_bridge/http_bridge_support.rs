use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpStream;

use serde::Serialize;
use serde_json::json;

use super::{AgentInvoker, DecisionRequest, FeedbackEnvelope, ProviderState};

pub(super) fn handle_connection(
    stream: &mut TcpStream,
    state: &ProviderState,
    invoker: &dyn AgentInvoker,
) -> Result<(), String> {
    let request = read_http_request(stream)?;
    let route_label = authorize_request(state, &request)?;
    if route_label.is_none() {
        return write_json_response(stream, 401, &json!({"error":"Unauthorized"}));
    }
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/v1/provider/info") => write_json_response(stream, 200, &state.provider_info()),
        ("GET", "/v1/provider/health") | ("GET", "/health") => {
            write_json_response(stream, 200, &state.provider_health())
        }
        ("POST", "/v1/world-simulator/decision") => {
            let decoded: DecisionRequest = serde_json::from_slice(request.body.as_slice())
                .map_err(|err| format!("decode decision request failed: {err}"))?;
            let response = state.handle_decision(decoded, route_label.as_deref(), invoker);
            write_json_response(stream, 200, &response)
        }
        ("POST", "/v1/world-simulator/feedback") => {
            let decoded: FeedbackEnvelope = serde_json::from_slice(request.body.as_slice())
                .map_err(|err| format!("decode feedback request failed: {err}"))?;
            state.record_feedback(decoded);
            write_json_response(stream, 200, &json!({"ok": true}))
        }
        _ => write_json_response(stream, 404, &json!({"error":"Not Found"})),
    }
}

fn authorize_request(
    state: &ProviderState,
    request: &RecordedHttpRequest,
) -> Result<Option<String>, String> {
    let presented = request
        .headers
        .get("authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if !state.options.auth_route_map.is_empty() {
        let Some(token) = presented else {
            return Ok(None);
        };
        return Ok(state.options.auth_route_map.get(token).cloned());
    }
    if state.options.auth_route_from_bearer {
        let Some(token) = presented else {
            return Ok(None);
        };
        return Ok(state
            .resolve_newapi_bridge_route_label(token)
            .map(str::to_string));
    }
    let Some(expected) = state.options.auth_token.as_deref() else {
        return Ok(Some("default".to_string()));
    };
    Ok(presented
        .filter(|value| *value == expected)
        .map(|_| "default".to_string()))
}

#[derive(Debug)]
struct RecordedHttpRequest {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

fn read_http_request(stream: &mut TcpStream) -> Result<RecordedHttpRequest, String> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 2048];
    let mut header_end = None;
    let mut content_length = 0_usize;

    loop {
        let bytes = stream
            .read(&mut chunk)
            .map_err(|err| format!("read request failed: {err}"))?;
        if bytes == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..bytes]);
        if header_end.is_none() {
            header_end = find_header_terminator(buffer.as_slice());
            if let Some(boundary) = header_end {
                let header = std::str::from_utf8(&buffer[..boundary])
                    .map_err(|err| format!("request header was not utf8: {err}"))?;
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

    let boundary = header_end.ok_or_else(|| "request missing header boundary".to_string())?;
    let header = std::str::from_utf8(&buffer[..boundary])
        .map_err(|err| format!("request header was not utf8: {err}"))?;
    let mut lines = header.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| "request missing request line".to_string())?;
    let mut request_line_parts = request_line.split_whitespace();
    let method = request_line_parts
        .next()
        .ok_or_else(|| "request line missing method".to_string())?
        .to_string();
    let path = request_line_parts
        .next()
        .ok_or_else(|| "request line missing path".to_string())?
        .to_string();
    let mut headers = BTreeMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    let body = buffer[(boundary + 4)..(boundary + 4 + content_length)].to_vec();
    Ok(RecordedHttpRequest {
        method,
        path,
        headers,
        body,
    })
}

fn find_header_terminator(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn write_json_response(
    stream: &mut TcpStream,
    status_code: u16,
    body: &impl Serialize,
) -> Result<(), String> {
    let payload =
        serde_json::to_string(body).map_err(|err| format!("serialize response failed: {err}"))?;
    let status_text = match status_code {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {status_code} {status_text}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        payload.len(),
        payload
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|err| format!("write response failed: {err}"))
}
