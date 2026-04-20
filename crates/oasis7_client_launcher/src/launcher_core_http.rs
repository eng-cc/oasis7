use super::*;

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

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn check_provider_loopback_http_provider(
    base_url: &str,
    auth_token: Option<&str>,
    timeout_ms: u64,
) -> Result<ProviderSnapshot, ProviderCheckError> {
    validate_provider_base_url(base_url).map_err(ProviderCheckError::InvalidConfig)?;
    let info_started_at = Instant::now();
    let info: ProviderInfoResponse =
        http_json_request_with_timeout(base_url, "/v1/provider/info", auth_token, timeout_ms)?;
    let info_latency_ms = info_started_at.elapsed().as_millis().min(u64::MAX as u128) as u64;
    let health_started_at = Instant::now();
    let health: ProviderHealthResponse =
        http_json_request_with_timeout(base_url, "/v1/provider/health", auth_token, timeout_ms)?;
    let health_latency_ms = health_started_at
        .elapsed()
        .as_millis()
        .min(u64::MAX as u128) as u64;
    let provider_info = ProviderInfo {
        provider_id: info.provider_id,
        name: info.name,
        version: info.version,
        protocol_version: info.protocol_version,
        capabilities: info.capabilities,
        supported_action_sets: info.supported_action_sets,
    };
    let provider_health = ProviderHealth {
        ok: health.ok,
        status: health.status,
        uptime_ms: None,
        last_error: health.last_error,
        queue_depth: health.queue_depth,
    };
    let compatibility = evaluate_provider_compatibility(&provider_info, Some(&provider_health));
    let status = provider_health
        .status
        .unwrap_or_else(|| if provider_health.ok { "ok" } else { "not_ok" }.to_string());
    Ok(ProviderSnapshot {
        provider_id: provider_info.provider_id,
        name: provider_info
            .name
            .unwrap_or_else(|| "Local Provider".to_string()),
        version: provider_info
            .version
            .unwrap_or_else(|| "unknown".to_string()),
        protocol_version: provider_info
            .protocol_version
            .unwrap_or_else(|| "unknown".to_string()),
        capabilities: provider_info.capabilities,
        supported_action_sets: provider_info.supported_action_sets,
        compatibility_status: compatibility.status.into(),
        status,
        queue_depth: provider_health.queue_depth,
        last_error: provider_health.last_error,
        fallback_reason: compatibility.fallback_reason,
        info_latency_ms,
        health_latency_ms,
        total_latency_ms: info_latency_ms.saturating_add(health_latency_ms),
    })
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
    } else if raw.starts_with("https://") {
        return Err(format!("{label} must use http:// for localhost provider"));
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

#[cfg(not(target_arch = "wasm32"))]
fn http_json_request_with_timeout<T: DeserializeOwned>(
    base_url: &str,
    path: &str,
    auth_token: Option<&str>,
    timeout_ms: u64,
) -> Result<T, ProviderCheckError> {
    let (status_code, response_body) =
        http_request_with_timeout(base_url, path, auth_token, timeout_ms)?;
    if status_code == 401 {
        return Err(ProviderCheckError::Unauthorized(format!(
            "provider check returned HTTP 401 for {path}"
        )));
    }
    if !(200..=299).contains(&status_code) {
        let body_text = String::from_utf8_lossy(response_body.as_slice());
        return Err(ProviderCheckError::Unreachable(format!(
            "provider check {path} failed with HTTP {status_code}: {body_text}"
        )));
    }
    serde_json::from_slice(response_body.as_slice()).map_err(|err| {
        ProviderCheckError::Unreachable(format!(
            "decode provider check {path} response failed: {err}"
        ))
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn http_request_with_timeout(
    base_url: &str,
    path: &str,
    auth_token: Option<&str>,
    timeout_ms: u64,
) -> Result<(u16, Vec<u8>), ProviderCheckError> {
    let (host, port) = parse_http_base_url(base_url, "provider base url")
        .map_err(ProviderCheckError::InvalidConfig)?;
    let connect_host = normalize_host_for_connect(host.as_str());
    let socket_addr = (connect_host.as_str(), port)
        .to_socket_addrs()
        .map_err(|err| ProviderCheckError::Unreachable(format!("resolve provider failed: {err}")))?
        .next()
        .ok_or_else(|| {
            ProviderCheckError::Unreachable(
                "resolve provider failed: no socket address".to_string(),
            )
        })?;
    let mut stream =
        TcpStream::connect_timeout(&socket_addr, Duration::from_millis(timeout_ms.max(1)))
            .map_err(|err| {
                ProviderCheckError::Unreachable(format!("connect provider failed: {err}"))
            })?;
    let timeout = Some(Duration::from_millis(timeout_ms.max(1)));
    let _ = stream.set_read_timeout(timeout);
    let _ = stream.set_write_timeout(timeout);

    let host_header = host_for_url(host.as_str());
    let mut request =
        format!("GET {path} HTTP/1.1\r\nHost: {host_header}:{port}\r\nConnection: close\r\n");
    if let Some(token) = auth_token.filter(|value| !value.trim().is_empty()) {
        request.push_str(&format!("Authorization: Bearer {}\r\n", token.trim()));
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes()).map_err(|err| {
        ProviderCheckError::Unreachable(format!("write provider check failed: {err}"))
    })?;
    let mut response_bytes = Vec::new();
    stream.read_to_end(&mut response_bytes).map_err(|err| {
        ProviderCheckError::Unreachable(format!("read provider check failed: {err}"))
    })?;
    parse_http_response(response_bytes.as_slice())
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_http_response(bytes: &[u8]) -> Result<(u16, Vec<u8>), ProviderCheckError> {
    let Some(boundary) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
        return Err(ProviderCheckError::Unreachable(
            "invalid HTTP response: missing header terminator".to_string(),
        ));
    };
    let header = std::str::from_utf8(&bytes[..boundary]).map_err(|_| {
        ProviderCheckError::Unreachable("invalid HTTP response: header is not UTF-8".to_string())
    })?;
    let body = bytes[(boundary + 4)..].to_vec();
    let Some(status_line) = header.lines().next() else {
        return Err(ProviderCheckError::Unreachable(
            "invalid HTTP response: missing status line".to_string(),
        ));
    };
    let Some(status_code) = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|token| token.parse::<u16>().ok())
    else {
        return Err(ProviderCheckError::Unreachable(format!(
            "invalid HTTP response status line: {status_line}"
        )));
    };
    Ok((status_code, body))
}
