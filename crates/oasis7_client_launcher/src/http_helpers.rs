pub(crate) fn parse_host_port_parts<'a>(
    raw: &'a str,
    label: &str,
) -> Result<(&'a str, &'a str), String> {
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
    Ok((host, port_raw.trim()))
}

pub(crate) fn parse_host_port(raw: &str, label: &str) -> Result<(String, u16), String> {
    let (host, port_raw) = parse_host_port_parts(raw, label)?;
    let port = port_raw
        .parse::<u16>()
        .map_err(|_| format!("{label} port must be in 1..=65535"))?;
    if port == 0 {
        return Err(format!("{label} port must be in 1..=65535"));
    }
    Ok((host.to_string(), port))
}

pub(crate) fn bracket_ipv6_authority_host(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') && !host.ends_with(']') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

pub(crate) fn normalize_connect_host(host: &str) -> String {
    match host.trim() {
        "0.0.0.0" => "127.0.0.1".to_string(),
        "::" | "[::]" => "::1".to_string(),
        value => value.to_string(),
    }
}

pub(crate) fn parse_http_status_code(header: &str) -> Result<u16, String> {
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
