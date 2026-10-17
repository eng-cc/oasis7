use super::*;

use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

const CHAIN_TRANSFER_QUERY_PROXY_TIMEOUT_MS: u64 = 1_500;

pub(super) fn query_chain_transfer_json(
    state: &mut ServiceState,
    runtime_target: &str,
) -> serde_json::Value {
    if !state.config.chain_enabled {
        state.append_log("chain transfer query rejected: chain runtime is disabled");
        state.mark_updated();
        return serde_json::json!({
            "ok": false,
            "error_code": "chain_disabled",
            "error": "chain runtime is disabled",
        });
    }

    let chain_status_bind = state.config.chain_status_bind.clone();
    match query_chain_transfer_remote(chain_status_bind.as_str(), runtime_target) {
        Ok(value) => {
            state.append_log(format!(
                "chain transfer query proxied via control plane ({runtime_target})"
            ));
            state.mark_updated();
            value
        }
        Err(err) => {
            state.append_log(format!("chain transfer query proxy failed: {err}"));
            state.mark_updated();
            serde_json::json!({
                "ok": false,
                "error_code": "proxy_error",
                "error": err,
            })
        }
    }
}

fn query_chain_transfer_remote(
    chain_status_bind: &str,
    runtime_target: &str,
) -> Result<serde_json::Value, String> {
    let (host, port) = parse_host_port(chain_status_bind, "chain status bind")?;
    let host = runtime_paths::normalize_bind_host_for_local_access(host.as_str());
    let socket_addr = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|err| format!("resolve chain status server failed: {err}"))?
        .next()
        .ok_or_else(|| "resolve chain status server failed: no socket address".to_string())?;

    let mut stream = TcpStream::connect_timeout(
        &socket_addr,
        Duration::from_millis(CHAIN_TRANSFER_QUERY_PROXY_TIMEOUT_MS),
    )
    .map_err(|err| format!("connect chain status server failed: {err}"))?;
    let timeout = Some(Duration::from_millis(CHAIN_TRANSFER_QUERY_PROXY_TIMEOUT_MS));
    let _ = stream.set_read_timeout(timeout);
    let _ = stream.set_write_timeout(timeout);

    let host_header = host_for_url(host.as_str());
    let request_head = format!(
        "GET {runtime_target} HTTP/1.1\r\nHost: {host_header}:{port}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request_head.as_bytes())
        .map_err(|err| format!("write chain transfer query request failed: {err}"))?;
    stream
        .flush()
        .map_err(|err| format!("flush chain transfer query request failed: {err}"))?;

    let mut response_bytes = Vec::new();
    stream
        .read_to_end(&mut response_bytes)
        .map_err(|err| format!("read chain transfer query response failed: {err}"))?;
    let (status_code, response) = parse_chain_transfer_query_response(response_bytes.as_slice())?;
    if !(200..=299).contains(&status_code) {
        return Err(format!(
            "chain transfer query returned HTTP {status_code}: {}",
            response
                .get("error")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown error")
        ));
    }
    Ok(response)
}

fn parse_chain_transfer_query_response(
    response_bytes: &[u8],
) -> Result<(u16, serde_json::Value), String> {
    let Some(boundary) = response_bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
    else {
        return Err("invalid HTTP response: missing header terminator".to_string());
    };
    let header = std::str::from_utf8(&response_bytes[..boundary])
        .map_err(|_| "invalid HTTP response: header is not UTF-8".to_string())?;
    let body = &response_bytes[(boundary + 4)..];
    let status_code = header
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|token| token.parse::<u16>().ok())
        .ok_or_else(|| "invalid HTTP response: missing status code".to_string())?;
    let response: serde_json::Value = serde_json::from_slice(body)
        .map_err(|err| format!("parse chain transfer query response JSON failed: {err}"))?;
    Ok((status_code, response))
}
