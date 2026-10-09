use super::*;
#[cfg(not(target_arch = "wasm32"))]
use oasis7_client_api::{ProviderHealth, ProviderInfo, evaluate_provider_compatibility};
#[cfg(not(target_arch = "wasm32"))]
use reqwest::blocking::Client;
#[cfg(not(target_arch = "wasm32"))]
use reqwest::{StatusCode, Url};
#[cfg(not(target_arch = "wasm32"))]
use serde::de::DeserializeOwned;
#[cfg(all(not(target_arch = "wasm32"), test))]
use std::io::{Read, Write};
#[cfg(all(not(target_arch = "wasm32"), test))]
use std::net::{TcpStream, ToSocketAddrs};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{Duration, Instant};

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

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn check_provider_http_provider(
    base_url: &str,
    auth_token: Option<&str>,
    timeout_ms: u64,
    transport: &str,
) -> Result<ProviderSnapshot, ProviderCheckError> {
    validate_provider_base_url_for_transport(base_url, transport)
        .map_err(ProviderCheckError::InvalidConfig)?;
    let client = ProviderProbeHttpClient::new(base_url, auth_token, timeout_ms, transport)?;
    let info_started_at = Instant::now();
    let info: ProviderInfo = client.get_json("/v1/provider/info")?;
    let info_latency_ms = info_started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
    let health_started_at = Instant::now();
    let health: ProviderHealth = client.get_json("/v1/provider/health")?;
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
        chain_resource_manifest_schema_version: info.chain_resource_manifest_schema_version,
        chain_resource_delta_schema_version: info.chain_resource_delta_schema_version,
        capabilities: info.capabilities,
        supported_action_sets: info.supported_action_sets,
        compatibility_status: compatibility.status,
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
struct ProviderProbeHttpClient {
    base_url: Url,
    auth_token: Option<String>,
    http: Client,
}

#[cfg(not(target_arch = "wasm32"))]
impl ProviderProbeHttpClient {
    fn new(
        base_url: &str,
        auth_token: Option<&str>,
        timeout_ms: u64,
        transport: &str,
    ) -> Result<Self, ProviderCheckError> {
        let base_url = Url::parse(base_url.trim())
            .map_err(|err| ProviderCheckError::InvalidConfig(err.to_string()))?;
        let mut builder = Client::builder().timeout(Duration::from_millis(timeout_ms.max(1)));
        if transport.trim() == LOOPBACK_HTTP_PROVIDER_TRANSPORT {
            builder = builder.no_proxy();
        }
        let http = builder
            .build()
            .map_err(|err| ProviderCheckError::Unreachable(err.to_string()))?;
        Ok(Self {
            base_url,
            auth_token: auth_token
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned),
            http,
        })
    }

    fn get_json<Response>(&self, path: &str) -> Result<Response, ProviderCheckError>
    where
        Response: DeserializeOwned,
    {
        let url = self
            .base_url
            .join(path.trim_start_matches('/'))
            .map_err(|err| ProviderCheckError::InvalidConfig(err.to_string()))?;
        let mut request = self.http.get(url);
        if let Some(token) = &self.auth_token {
            request = request.bearer_auth(token);
        }
        let response = request
            .send()
            .map_err(|err| ProviderCheckError::Unreachable(err.to_string()))?;
        let status = response.status();
        let body = response
            .bytes()
            .map_err(|err| ProviderCheckError::Unreachable(err.to_string()))?;
        if status == StatusCode::UNAUTHORIZED {
            let detail = String::from_utf8_lossy(body.as_ref()).trim().to_string();
            return Err(ProviderCheckError::Unauthorized(if detail.is_empty() {
                "HTTP 401".to_string()
            } else {
                detail
            }));
        }
        if !status.is_success() {
            return Err(ProviderCheckError::Unreachable(
                String::from_utf8_lossy(body.as_ref()).trim().to_string(),
            ));
        }
        serde_json::from_slice(body.as_ref())
            .map_err(|err| ProviderCheckError::Unreachable(err.to_string()))
    }
}

pub(crate) fn normalize_host_for_connect(host: &str) -> String {
    crate::http_helpers::normalize_connect_host(host)
}

pub(crate) fn normalize_host_for_url(host: &str) -> String {
    crate::http_helpers::normalize_url_host(host)
}

pub(crate) fn host_for_url(host: &str) -> String {
    crate::http_helpers::bracket_ipv6_authority_host(host)
}

pub(crate) fn parse_http_base_url(base_url: &str, label: &str) -> Result<(String, u16), String> {
    let mut raw = base_url.trim();
    let mut default_port = 80;
    if let Some(stripped) = raw.strip_prefix("http://") {
        raw = stripped;
    } else if let Some(stripped) = raw.strip_prefix("https://") {
        raw = stripped;
        default_port = 443;
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
        Ok((authority.to_string(), default_port))
    }
}
