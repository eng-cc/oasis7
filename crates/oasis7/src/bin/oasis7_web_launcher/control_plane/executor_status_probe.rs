use super::*;
use oasis7_launcher_ui::AgentExecutorStatus;

pub(super) fn query_executor_status_endpoint(bind: &str) -> Result<AgentExecutorStatus, String> {
    let (host, port) = parse_host_port(bind, "chain status bind")?;
    let host = runtime_paths::normalize_bind_host_for_local_access(&host);
    let address = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|_| "executor address unavailable")?
        .next()
        .ok_or("executor address unavailable")?;
    let timeout = Duration::from_millis(CHAIN_STATUS_PROBE_TIMEOUT_MS);
    let mut stream =
        TcpStream::connect_timeout(&address, timeout).map_err(|_| "executor status unavailable")?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|_| "executor status unavailable")?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|_| "executor status unavailable")?;
    let request = format!(
        "GET /v1/world/executor-status HTTP/1.1\r\nHost: {}:{port}\r\nConnection: close\r\n\r\n",
        host_for_url(&host)
    );
    std::io::Write::write_all(&mut stream, request.as_bytes())
        .map_err(|_| "executor status unavailable")?;
    let mut bytes = Vec::new();
    stream
        .take(16_385)
        .read_to_end(&mut bytes)
        .map_err(|_| "executor status unavailable")?;
    parse_executor_status_response(&bytes)
}
fn parse_executor_status_response(bytes: &[u8]) -> Result<AgentExecutorStatus, String> {
    if bytes.len() > 16_384 {
        return Err("executor status exceeds limit".into());
    }
    let boundary = bytes
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or("invalid executor response")?;
    let header =
        std::str::from_utf8(&bytes[..boundary]).map_err(|_| "invalid executor response")?;
    if !header.starts_with("HTTP/") {
        return Err("invalid executor response".into());
    }
    let code = header
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|s| s.parse::<u16>().ok())
        .ok_or("invalid executor response")?;
    if code != 200 {
        return Err("executor status unavailable".into());
    }
    let status: AgentExecutorStatus =
        serde_json::from_slice(&bytes[boundary + 4..]).map_err(|_| "invalid executor status")?;
    if status.model.as_ref().is_some_and(|s| s.len() > 256) {
        return Err("invalid executor status".into());
    }
    Ok(status)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn executor_probe_preserves_unknown_budget_and_drops_private_extensions() {
        let status = parse_executor_status_response(b"HTTP/1.1 200 OK\r\n\r\n{\"phase\":\"not_selected\",\"profile\":null,\"secret\":\"must not escape\"}").unwrap();
        assert_eq!(status.reserved_model_calls, None);
        assert_eq!(status.phase_label(false), "Not selected");
        assert!(
            !serde_json::to_string(&status)
                .unwrap()
                .contains("must not escape")
        );
        let budget = parse_executor_status_response(b"HTTP/1.1 200 OK\r\n\r\n{\"phase\":\"blocked\",\"reason\":\"model_budget_exhausted\",\"profile\":\"native_task_industry_v1\",\"execution_location\":\"node_process\",\"max_model_calls\":5,\"reserved_model_calls\":5,\"committed_turns\":0}").unwrap();
        assert_eq!(budget.committed_turns, Some(0));
        assert!(budget.reason_label(false).contains("goal status unknown"));
        assert!(parse_executor_status_response(b"HTTP/1.1 503 Unavailable\r\n\r\n{}").is_err());
        assert!(parse_executor_status_response(b"HTTP/1.1 200 OK\r\n\r\n{\"phase\":1}").is_err());
    }
}
