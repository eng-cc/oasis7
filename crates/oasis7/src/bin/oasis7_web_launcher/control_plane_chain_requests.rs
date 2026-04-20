use super::*;

pub(super) fn submit_chain_transfer_remote(
    chain_status_bind: &str,
    request: &ChainTransferSubmitRequest,
) -> Result<ChainTransferSubmitResponse, String> {
    let (host, port) = parse_host_port(chain_status_bind, "chain status bind")?;
    let host = runtime_paths::normalize_bind_host_for_local_access(host.as_str());
    let socket_addr = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|err| format!("resolve chain status server failed: {err}"))?
        .next()
        .ok_or_else(|| "resolve chain status server failed: no socket address".to_string())?;

    let mut stream = TcpStream::connect_timeout(
        &socket_addr,
        Duration::from_millis(CHAIN_TRANSFER_PROXY_TIMEOUT_MS),
    )
    .map_err(|err| format!("connect chain status server failed: {err}"))?;
    let timeout = Some(Duration::from_millis(CHAIN_TRANSFER_PROXY_TIMEOUT_MS));
    let _ = stream.set_read_timeout(timeout);
    let _ = stream.set_write_timeout(timeout);

    let payload = serde_json::to_vec(request)
        .map_err(|err| format!("serialize chain transfer request failed: {err}"))?;
    let host_header = host_for_url(host.as_str());
    let request_head = format!(
        "POST {CHAIN_TRANSFER_SUBMIT_PATH} HTTP/1.1\r\nHost: {host_header}:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    );
    std::io::Write::write_all(&mut stream, request_head.as_bytes())
        .map_err(|err| format!("write chain transfer request header failed: {err}"))?;
    std::io::Write::write_all(&mut stream, payload.as_slice())
        .map_err(|err| format!("write chain transfer request body failed: {err}"))?;
    std::io::Write::flush(&mut stream)
        .map_err(|err| format!("flush chain transfer request failed: {err}"))?;

    let mut response_bytes = Vec::new();
    std::io::Read::read_to_end(&mut stream, &mut response_bytes)
        .map_err(|err| format!("read chain transfer response failed: {err}"))?;
    let (status_code, response) = parse_chain_transfer_submit_response(response_bytes.as_slice())?;

    if !(200..=299).contains(&status_code) && response.ok {
        return Err(format!(
            "chain transfer submit returned HTTP {status_code} with invalid success payload"
        ));
    }
    Ok(response)
}

pub(super) fn submit_chain_feedback_remote(
    chain_status_bind: &str,
    request: &ChainFeedbackSubmitRequest,
) -> Result<ChainFeedbackSubmitResponse, String> {
    let (host, port) = parse_host_port(chain_status_bind, "chain status bind")?;
    let host = runtime_paths::normalize_bind_host_for_local_access(host.as_str());
    let socket_addr = (host.as_str(), port)
        .to_socket_addrs()
        .map_err(|err| format!("resolve chain status server failed: {err}"))?
        .next()
        .ok_or_else(|| "resolve chain status server failed: no socket address".to_string())?;

    let mut stream = TcpStream::connect_timeout(
        &socket_addr,
        Duration::from_millis(CHAIN_TRANSFER_PROXY_TIMEOUT_MS),
    )
    .map_err(|err| format!("connect chain status server failed: {err}"))?;
    let timeout = Some(Duration::from_millis(CHAIN_TRANSFER_PROXY_TIMEOUT_MS));
    let _ = stream.set_read_timeout(timeout);
    let _ = stream.set_write_timeout(timeout);

    let payload = serde_json::to_vec(request)
        .map_err(|err| format!("serialize chain feedback request failed: {err}"))?;
    let host_header = host_for_url(host.as_str());
    let request_head = format!(
        "POST {CHAIN_FEEDBACK_SUBMIT_PATH} HTTP/1.1\r\nHost: {host_header}:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    );
    std::io::Write::write_all(&mut stream, request_head.as_bytes())
        .map_err(|err| format!("write chain feedback request header failed: {err}"))?;
    std::io::Write::write_all(&mut stream, payload.as_slice())
        .map_err(|err| format!("write chain feedback request body failed: {err}"))?;
    std::io::Write::flush(&mut stream)
        .map_err(|err| format!("flush chain feedback request failed: {err}"))?;

    let mut response_bytes = Vec::new();
    std::io::Read::read_to_end(&mut stream, &mut response_bytes)
        .map_err(|err| format!("read chain feedback response failed: {err}"))?;
    let (status_code, response) = parse_chain_feedback_submit_response(response_bytes.as_slice())?;

    if !(200..=299).contains(&status_code) && response.ok {
        return Err(format!(
            "chain feedback submit returned HTTP {status_code} with invalid success payload"
        ));
    }
    Ok(response)
}

fn parse_chain_transfer_submit_response(
    response_bytes: &[u8],
) -> Result<(u16, ChainTransferSubmitResponse), String> {
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

    let response: ChainTransferSubmitResponse = serde_json::from_slice(body)
        .map_err(|err| format!("parse chain transfer response JSON failed: {err}"))?;
    Ok((status_code, response))
}

fn parse_chain_feedback_submit_response(
    response_bytes: &[u8],
) -> Result<(u16, ChainFeedbackSubmitResponse), String> {
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

    let response: ChainFeedbackSubmitResponse = serde_json::from_slice(body)
        .map_err(|err| format!("parse chain feedback response JSON failed: {err}"))?;
    Ok((status_code, response))
}
