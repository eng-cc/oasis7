use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use serde::Deserialize;

use super::{
    host_for_url, parse_host_port, runtime_paths, ChainNodeObservabilitySnapshot,
    ChainP2pStatusSnapshot, CHAIN_STATUS_PROBE_TIMEOUT_MS,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ChainStatusProbeSnapshot {
    pub(super) p2p: ChainP2pStatusSnapshot,
    pub(super) observability: ChainNodeObservabilitySnapshot,
}

pub(super) fn query_chain_status_endpoint(bind: &str) -> Result<ChainStatusProbeSnapshot, String> {
    let (host, port) = parse_host_port(bind, "chain status bind")?;
    let host = runtime_paths::normalize_bind_host_for_local_access(host.as_str());
    let socket_addr = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|err| format!("resolve chain status server failed: {err}"))?
        .next()
        .ok_or_else(|| "resolve chain status server failed: no socket address".to_string())?;

    let mut stream = TcpStream::connect_timeout(
        &socket_addr,
        Duration::from_millis(CHAIN_STATUS_PROBE_TIMEOUT_MS),
    )
    .map_err(|err| format!("connect chain status server failed: {err}"))?;
    let timeout = Some(Duration::from_millis(CHAIN_STATUS_PROBE_TIMEOUT_MS));
    let _ = stream.set_read_timeout(timeout);
    let _ = stream.set_write_timeout(timeout);

    let host_header = host_for_url(host.as_str());
    let request = format!(
        "GET /v1/chain/status HTTP/1.1\r\nHost: {host_header}:{port}\r\nConnection: close\r\n\r\n"
    );
    std::io::Write::write_all(&mut stream, request.as_bytes())
        .map_err(|err| format!("write chain status probe failed: {err}"))?;
    std::io::Write::flush(&mut stream)
        .map_err(|err| format!("flush chain status probe failed: {err}"))?;

    let mut response_bytes = Vec::new();
    std::io::Read::read_to_end(&mut stream, &mut response_bytes)
        .map_err(|err| format!("read chain status probe failed: {err}"))?;
    if response_bytes.is_empty() {
        return Err("chain status probe returned empty response".to_string());
    }
    parse_chain_status_probe_response(response_bytes.as_slice())
}

fn parse_chain_status_probe_response(
    response_bytes: &[u8],
) -> Result<ChainStatusProbeSnapshot, String> {
    let Some(boundary) = response_bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
    else {
        return Err("invalid HTTP response: missing header terminator".to_string());
    };
    let header = std::str::from_utf8(&response_bytes[..boundary])
        .map_err(|_| "invalid HTTP response: header is not UTF-8".to_string())?;
    let body = &response_bytes[(boundary + 4)..];

    let status_line = header.lines().next().unwrap_or_default();
    if !status_line.starts_with("HTTP/") {
        return Err("chain status probe received non-HTTP response".to_string());
    }
    let status_code = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|token| token.parse::<u16>().ok())
        .ok_or_else(|| format!("invalid chain status probe status line: {status_line}"))?;
    if !(200..=299).contains(&status_code) {
        return Err(format!("chain status probe returned HTTP {status_code}"));
    }

    #[derive(Deserialize)]
    struct ChainStatusProbeResponse {
        p2p: ChainP2pStatusSnapshot,
        observability: ChainNodeObservabilitySnapshot,
    }

    let payload: ChainStatusProbeResponse = serde_json::from_slice(body)
        .map_err(|err| format!("parse chain status probe JSON failed: {err}"))?;
    Ok(ChainStatusProbeSnapshot {
        p2p: payload.p2p,
        observability: payload.observability,
    })
}
