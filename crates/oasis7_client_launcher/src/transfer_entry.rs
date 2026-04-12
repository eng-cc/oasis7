use serde::Deserialize;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use super::WebTransferSubmitRequest;

const CHAIN_TRANSFER_SUBMIT_PATH: &str = "/v1/chain/transfer/submit";
const HTTP_TIMEOUT_MS: u64 = 3_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TransferDraft {
    pub(crate) from_account_id: String,
    pub(crate) to_account_id: String,
    pub(crate) amount: String,
    pub(crate) nonce: String,
}

impl Default for TransferDraft {
    fn default() -> Self {
        Self {
            from_account_id: String::new(),
            to_account_id: String::new(),
            amount: "1".to_string(),
            nonce: "1".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferDraftIssue {
    FromAccountRequired,
    ToAccountRequired,
    SameAccount,
    AmountInvalid,
    NonceInvalid,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct TransferSubmitResponse {
    pub(crate) ok: bool,
    pub(crate) action_id: Option<u64>,
    pub(crate) submitted_at_unix_ms: Option<i64>,
    pub(crate) error_code: Option<String>,
    pub(crate) error: Option<String>,
}

pub(crate) fn validate_transfer_draft(draft: &TransferDraft) -> Vec<TransferDraftIssue> {
    let mut issues = Vec::new();
    if draft.from_account_id.trim().is_empty() {
        issues.push(TransferDraftIssue::FromAccountRequired);
    }
    if draft.to_account_id.trim().is_empty() {
        issues.push(TransferDraftIssue::ToAccountRequired);
    }
    if !draft.from_account_id.trim().is_empty()
        && !draft.to_account_id.trim().is_empty()
        && draft.from_account_id.trim() == draft.to_account_id.trim()
    {
        issues.push(TransferDraftIssue::SameAccount);
    }
    if parse_positive_u64(draft.amount.as_str()).is_none() {
        issues.push(TransferDraftIssue::AmountInvalid);
    }
    if parse_positive_u64(draft.nonce.as_str()).is_none() {
        issues.push(TransferDraftIssue::NonceInvalid);
    }
    issues
}

pub(crate) fn submit_transfer_remote(
    draft: &TransferDraft,
    chain_status_bind: &str,
) -> Result<TransferSubmitResponse, String> {
    let request = build_transfer_submit_request(draft)?;
    let payload = serde_json::to_vec(&request)
        .map_err(|err| format!("serialize transfer submit request failed: {err}"))?;
    let response = post_json_request(chain_status_bind, CHAIN_TRANSFER_SUBMIT_PATH, &payload)?;
    Ok(response)
}

fn build_transfer_submit_request(
    draft: &TransferDraft,
) -> Result<WebTransferSubmitRequest, String> {
    let issues = validate_transfer_draft(draft);
    if !issues.is_empty() {
        return Err("transfer draft has invalid required fields".to_string());
    }
    let amount = parse_positive_u64(draft.amount.as_str())
        .ok_or_else(|| "transfer amount must be a positive integer".to_string())?;
    let nonce = parse_positive_u64(draft.nonce.as_str())
        .ok_or_else(|| "transfer nonce must be a positive integer".to_string())?;

    crate::transfer_auth::build_signed_web_transfer_submit_request(
        draft.from_account_id.as_str(),
        draft.to_account_id.as_str(),
        amount,
        nonce,
    )
}

fn parse_positive_u64(raw: &str) -> Option<u64> {
    raw.trim().parse::<u64>().ok().filter(|value| *value > 0)
}

fn post_json_request(
    bind: &str,
    path: &str,
    payload: &[u8],
) -> Result<TransferSubmitResponse, String> {
    let (host, port) = parse_host_port(bind, "chain status bind")?;
    let host = normalize_connect_host(host.as_str());
    let socket_host = host_for_socket(host.as_str());
    let mut stream = TcpStream::connect(format!("{socket_host}:{port}"))
        .map_err(|err| format!("connect chain status server failed: {err}"))?;
    let timeout = Some(Duration::from_millis(HTTP_TIMEOUT_MS));
    let _ = stream.set_read_timeout(timeout);
    let _ = stream.set_write_timeout(timeout);

    let host_header = host_for_http(host.as_str());
    let mut request_head = String::new();
    request_head.push_str(&format!("POST {path} HTTP/1.1\r\n"));
    request_head.push_str(&format!("Host: {host_header}:{port}\r\n"));
    request_head.push_str("Content-Type: application/json\r\n");
    request_head.push_str(&format!("Content-Length: {}\r\n", payload.len()));
    request_head.push_str("Connection: close\r\n\r\n");

    stream
        .write_all(request_head.as_bytes())
        .map_err(|err| format!("write request header failed: {err}"))?;
    stream
        .write_all(payload)
        .map_err(|err| format!("write request body failed: {err}"))?;
    stream
        .flush()
        .map_err(|err| format!("flush request failed: {err}"))?;

    let mut response_bytes = Vec::new();
    stream
        .read_to_end(&mut response_bytes)
        .map_err(|err| format!("read response failed: {err}"))?;
    parse_http_json_response(&response_bytes)
}

fn parse_http_json_response(bytes: &[u8]) -> Result<TransferSubmitResponse, String> {
    let Some(boundary) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
        return Err("invalid HTTP response: missing header terminator".to_string());
    };
    let header = std::str::from_utf8(&bytes[..boundary])
        .map_err(|_| "invalid HTTP response: header is not UTF-8".to_string())?;
    let body = &bytes[(boundary + 4)..];
    let status_code = parse_http_status_code(header)?;
    let response: TransferSubmitResponse =
        serde_json::from_slice(body).map_err(|err| format!("parse response json failed: {err}"))?;

    if !(200..=299).contains(&status_code) {
        return Err(format!(
            "remote transfer submit failed with HTTP {}: {}",
            status_code,
            response
                .error
                .clone()
                .unwrap_or_else(|| "unknown error".to_string())
        ));
    }
    Ok(response)
}

fn parse_http_status_code(header: &str) -> Result<u16, String> {
    let Some(status_line) = header.lines().next() else {
        return Err("invalid HTTP response: missing status line".to_string());
    };
    let Some(code) = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|token| token.parse::<u16>().ok())
    else {
        return Err(format!("invalid HTTP response status line: {status_line}"));
    };
    Ok(code)
}

fn parse_host_port(raw: &str, label: &str) -> Result<(String, u16), String> {
    let value = raw.trim();
    let (host_raw, port_raw) = if let Some(rest) = value.strip_prefix('[') {
        let (host, remainder) = rest
            .split_once(']')
            .ok_or_else(|| format!("{label} IPv6 host must be in [addr]:port format"))?;
        let port_raw = remainder
            .strip_prefix(':')
            .ok_or_else(|| format!("{label} must be in <host:port> format"))?;
        (host, port_raw)
    } else {
        let (host, port_raw) = value
            .rsplit_once(':')
            .ok_or_else(|| format!("{label} must be in <host:port> format"))?;
        if host.contains(':') {
            return Err(format!("{label} IPv6 host must be wrapped in []"));
        }
        (host, port_raw)
    };
    let host = host_raw.trim();
    if host.is_empty() {
        return Err(format!("{label} host cannot be empty"));
    }
    let port = port_raw
        .trim()
        .parse::<u16>()
        .map_err(|_| format!("{label} port must be in 1..=65535"))?;
    if port == 0 {
        return Err(format!("{label} port must be in 1..=65535"));
    }
    Ok((host.to_string(), port))
}

fn host_for_socket(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') && !host.ends_with(']') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

fn host_for_http(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') && !host.ends_with(']') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

fn normalize_connect_host(host: &str) -> String {
    match host.trim() {
        "0.0.0.0" => "127.0.0.1".to_string(),
        "::" | "[::]" => "::1".to_string(),
        value => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_transfer_submit_request, parse_http_json_response, submit_transfer_remote,
        validate_transfer_draft, TransferDraft, TransferDraftIssue,
    };
    use ed25519_dalek::SigningKey;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::Duration;

    fn transfer_test_signer(seed: u8) -> (String, String) {
        let private_key = [seed; 32];
        let signing_key = SigningKey::from_bytes(&private_key);
        (
            hex::encode(signing_key.verifying_key().to_bytes()),
            hex::encode(private_key),
        )
    }

    fn read_http_request(stream: &mut std::net::TcpStream) -> Vec<u8> {
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set timeout");
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 1024];

        loop {
            let read = stream.read(&mut buffer).expect("read request");
            if read == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..read]);

            let Some(boundary) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
                continue;
            };
            let header =
                std::str::from_utf8(&bytes[..boundary]).expect("request header should be UTF-8");
            let content_length = header
                .lines()
                .find_map(|line| line.strip_prefix("Content-Length:"))
                .and_then(|value| value.trim().parse::<usize>().ok())
                .unwrap_or(0);
            if bytes.len() >= boundary + 4 + content_length {
                break;
            }
        }

        bytes
    }

    fn write_http_json_response(stream: &mut std::net::TcpStream, status: &str, body: &str) {
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response should succeed");
    }

    #[test]
    fn validate_transfer_draft_reports_missing_and_invalid_fields() {
        let draft = TransferDraft {
            from_account_id: " ".to_string(),
            to_account_id: String::new(),
            amount: "0".to_string(),
            nonce: "x".to_string(),
        };
        let issues = validate_transfer_draft(&draft);
        assert!(issues.contains(&TransferDraftIssue::FromAccountRequired));
        assert!(issues.contains(&TransferDraftIssue::ToAccountRequired));
        assert!(issues.contains(&TransferDraftIssue::AmountInvalid));
        assert!(issues.contains(&TransferDraftIssue::NonceInvalid));
    }

    #[test]
    fn validate_transfer_draft_rejects_same_account() {
        let draft = TransferDraft {
            from_account_id: "player:alice".to_string(),
            to_account_id: " player:alice ".to_string(),
            amount: "1".to_string(),
            nonce: "1".to_string(),
        };
        let issues = validate_transfer_draft(&draft);
        assert!(issues.contains(&TransferDraftIssue::SameAccount));
    }

    #[test]
    fn build_transfer_submit_request_parses_trimmed_values() {
        let (public_key, private_key) = transfer_test_signer(41);
        let draft = TransferDraft {
            from_account_id: format!(" oc:pk:{public_key} "),
            to_account_id: "protocol:treasury".to_string(),
            amount: "7".to_string(),
            nonce: "3".to_string(),
        };
        std::env::set_var("OASIS7_VIEWER_AUTH_PUBLIC_KEY", public_key.as_str());
        std::env::set_var("OASIS7_VIEWER_AUTH_PRIVATE_KEY", private_key.as_str());
        let request = build_transfer_submit_request(&draft).expect("request");
        assert_eq!(request.from_account_id, format!("oc:pk:{public_key}"));
        assert_eq!(request.to_account_id, "protocol:treasury");
        assert_eq!(request.amount, 7);
        assert_eq!(request.nonce, 3);
        assert_eq!(request.public_key, public_key);
        assert!(request.signature.starts_with("octransferauth:v1:"));
        std::env::remove_var("OASIS7_VIEWER_AUTH_PUBLIC_KEY");
        std::env::remove_var("OASIS7_VIEWER_AUTH_PRIVATE_KEY");
    }

    #[test]
    fn parse_http_json_response_reads_success_payload() {
        let raw = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{\"ok\":true,\"action_id\":7,\"submitted_at_unix_ms\":123}";
        let response = parse_http_json_response(raw).expect("parse");
        assert!(response.ok);
        assert_eq!(response.action_id, Some(7));
        assert_eq!(response.submitted_at_unix_ms, Some(123));
    }

    #[test]
    fn parse_http_json_response_returns_error_for_non_2xx() {
        let raw = b"HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\n\r\n{\"ok\":false,\"error_code\":\"invalid_request\",\"error\":\"bad payload\"}";
        let err = parse_http_json_response(raw).expect_err("non-2xx should fail");
        assert!(err.contains("HTTP 400"));
        assert!(err.contains("bad payload"));
    }

    #[test]
    fn submit_transfer_remote_posts_expected_payload_and_reads_success() {
        let (public_key, private_key) = transfer_test_signer(43);
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock chain server");
        let bind = listener.local_addr().expect("read local addr");
        let expected_from = format!("oc:pk:{public_key}");
        let public_key_for_server = public_key.clone();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept launcher request");
            let request = read_http_request(&mut stream);
            let request_text = String::from_utf8_lossy(&request);
            assert!(request_text.starts_with("POST /v1/chain/transfer/submit HTTP/1.1"));
            assert!(request_text.contains(&format!("\"from_account_id\":\"{expected_from}\"")));
            assert!(request_text.contains("\"to_account_id\":\"protocol:treasury\""));
            assert!(request_text.contains("\"amount\":7"));
            assert!(request_text.contains("\"nonce\":9"));
            assert!(request_text.contains(&format!("\"public_key\":\"{public_key_for_server}\"")));
            assert!(request_text.contains("\"signature\":\"octransferauth:v1:"));

            write_http_json_response(
                &mut stream,
                "200 OK",
                "{\"ok\":true,\"action_id\":17,\"submitted_at_unix_ms\":123}",
            );
        });

        let draft = TransferDraft {
            from_account_id: format!("oc:pk:{public_key}"),
            to_account_id: "protocol:treasury".to_string(),
            amount: "7".to_string(),
            nonce: "9".to_string(),
        };
        std::env::set_var("OASIS7_VIEWER_AUTH_PUBLIC_KEY", public_key.as_str());
        std::env::set_var("OASIS7_VIEWER_AUTH_PRIVATE_KEY", private_key.as_str());
        let response =
            submit_transfer_remote(&draft, format!("127.0.0.1:{}", bind.port()).as_str())
                .expect("submit transfer should succeed");
        assert!(response.ok);
        assert_eq!(response.action_id, Some(17));
        assert_eq!(response.submitted_at_unix_ms, Some(123));
        std::env::remove_var("OASIS7_VIEWER_AUTH_PUBLIC_KEY");
        std::env::remove_var("OASIS7_VIEWER_AUTH_PRIVATE_KEY");

        server.join().expect("mock chain server should finish");
    }

    #[test]
    fn submit_transfer_remote_returns_error_for_rejected_response() {
        let (public_key, private_key) = transfer_test_signer(45);
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock chain server");
        let bind = listener.local_addr().expect("read local addr");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept launcher request");
            let _request = read_http_request(&mut stream);
            write_http_json_response(
                &mut stream,
                "400 Bad Request",
                "{\"ok\":false,\"error_code\":\"invalid_request\",\"error\":\"nonce replay\"}",
            );
        });

        let draft = TransferDraft {
            from_account_id: format!("oc:pk:{public_key}"),
            to_account_id: "protocol:treasury".to_string(),
            amount: "7".to_string(),
            nonce: "9".to_string(),
        };
        std::env::set_var("OASIS7_VIEWER_AUTH_PUBLIC_KEY", public_key.as_str());
        std::env::set_var("OASIS7_VIEWER_AUTH_PRIVATE_KEY", private_key.as_str());
        let err = submit_transfer_remote(&draft, format!("127.0.0.1:{}", bind.port()).as_str())
            .expect_err("submit should return structured remote rejection");
        assert!(err.contains("HTTP 400"));
        assert!(err.contains("nonce replay"));
        std::env::remove_var("OASIS7_VIEWER_AUTH_PUBLIC_KEY");
        std::env::remove_var("OASIS7_VIEWER_AUTH_PRIVATE_KEY");

        server.join().expect("mock chain server should finish");
    }
}
