use super::*;
#[cfg(not(target_arch = "wasm32"))]
use oasis7::simulator::{
    evaluate_provider_compatibility, ProviderLoopbackHttpClient, ProviderLoopbackHttpError,
};
#[cfg(all(not(target_arch = "wasm32"), test))]
use std::io::{Read, Write};
#[cfg(all(not(target_arch = "wasm32"), test))]
use std::net::{TcpStream, ToSocketAddrs};
#[cfg(all(not(target_arch = "wasm32"), test))]
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(all(not(target_arch = "wasm32"), test))]
pub(crate) fn probe_chain_status_endpoint(bind: &str) -> Result<(), String> {
    let (host, port) = parse_host_port(bind, "chain status bind")?;
    let host = normalize_host_for_connect(host.as_str());
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
    stream
        .write_all(request.as_bytes())
        .map_err(|err| format!("write chain status probe failed: {err}"))?;

    let mut buffer = [0_u8; 256];
    let bytes = stream
        .read(&mut buffer)
        .map_err(|err| format!("read chain status probe failed: {err}"))?;
    if bytes == 0 {
        return Err("chain status probe returned empty response".to_string());
    }
    let response = String::from_utf8_lossy(&buffer[..bytes]);
    let status_line = response.lines().next().unwrap_or_default();
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
    Ok(())
}

#[cfg(all(not(target_arch = "wasm32"), test))]
pub(crate) fn check_provider_loopback_http_provider(
    base_url: &str,
    auth_token: Option<&str>,
    timeout_ms: u64,
) -> Result<ProviderSnapshot, ProviderCheckError> {
    check_provider_http_provider(base_url, auth_token, timeout_ms, "loopback_http")
}

pub(crate) fn check_provider_http_provider(
    base_url: &str,
    auth_token: Option<&str>,
    timeout_ms: u64,
    transport: &str,
) -> Result<ProviderSnapshot, ProviderCheckError> {
    validate_provider_base_url_for_transport(base_url, transport)
        .map_err(ProviderCheckError::InvalidConfig)?;
    let client =
        ProviderLoopbackHttpClient::new_with_transport(base_url, auth_token, timeout_ms, transport)
            .map_err(map_provider_client_error)?;
    let info_started_at = Instant::now();
    let info = client.provider_info().map_err(map_provider_client_error)?;
    let info_latency_ms = info_started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
    let health_started_at = Instant::now();
    let health = client
        .provider_health()
        .map_err(map_provider_client_error)?;
    let health_latency_ms = health_started_at
        .elapsed()
        .as_millis()
        .min(u64::MAX as u128) as u64;
    let compatibility = evaluate_provider_compatibility(&info, Some(&health));
    let status = health
        .status
        .unwrap_or_else(|| if health.ok { "ok" } else { "not_ok" }.to_string());
    Ok(ProviderSnapshot {
        provider_id: info.provider_id,
        name: info.name.unwrap_or_else(|| "Provider".to_string()),
        version: info.version.unwrap_or_else(|| "unknown".to_string()),
        protocol_version: info
            .protocol_version
            .unwrap_or_else(|| "unknown".to_string()),
        capabilities: info.capabilities,
        supported_action_sets: info.supported_action_sets,
        compatibility_status: compatibility.status.into(),
        status,
        queue_depth: health.queue_depth,
        last_error: health.last_error,
        fallback_reason: compatibility.fallback_reason,
        info_latency_ms,
        health_latency_ms,
        total_latency_ms: info_latency_ms.saturating_add(health_latency_ms),
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn map_provider_client_error(error: ProviderLoopbackHttpError) -> ProviderCheckError {
    match error {
        ProviderLoopbackHttpError::InvalidBaseUrl(detail) => {
            ProviderCheckError::InvalidConfig(detail)
        }
        ProviderLoopbackHttpError::Unauthorized { detail, .. } => {
            ProviderCheckError::Unauthorized(detail)
        }
        ProviderLoopbackHttpError::RequestFailed { detail, .. }
        | ProviderLoopbackHttpError::UnexpectedStatus { body: detail, .. }
        | ProviderLoopbackHttpError::DecodeFailed { detail, .. } => {
            ProviderCheckError::Unreachable(detail)
        }
    }
}

pub(crate) fn normalize_host_for_connect(host: &str) -> String {
    let host = host.trim();
    if host == "0.0.0.0" {
        "127.0.0.1".to_string()
    } else if host == "::" || host == "[::]" {
        "::1".to_string()
    } else {
        host.to_string()
    }
}

pub(crate) fn normalize_host_for_url(host: &str) -> String {
    let host = host.trim();
    if host == "0.0.0.0" || host == "::" || host == "[::]" || host.is_empty() {
        "127.0.0.1".to_string()
    } else {
        host.to_string()
    }
}

pub(crate) fn host_for_url(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') && !host.ends_with(']') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

pub(crate) fn parse_http_base_url(base_url: &str, label: &str) -> Result<(String, u16), String> {
    let mut raw = base_url.trim();
    if let Some(stripped) = raw.strip_prefix("http://") {
        raw = stripped;
    } else if let Some(stripped) = raw.strip_prefix("https://") {
        raw = stripped;
    }
    raw = raw.trim_end_matches('/');
    let authority = raw
        .split('/')
        .next()
        .ok_or_else(|| format!("invalid {label}: {base_url}"))?
        .trim();
    if authority.is_empty() {
        return Err(format!("invalid {label}: {base_url}"));
    }
    if authority.starts_with('[') || authority.contains(':') {
        parse_host_port(authority, label)
    } else {
        Ok((authority.to_string(), 80))
    }
}
