use std::ops::Range;

use serde_json::{Map, Number, Value};

use super::LaunchConfig;

const FEEDBACK_LOG_SNAPSHOT_LIMIT: usize = 200;
const CONFIG_BOOL_ALLOWLIST: &[&str] = &[
    "llm_enabled",
    "provider_auto_discover",
    "chain_enabled",
    "chain_p2p_accept_public_entry",
    "chain_pos_adaptive_tick_scheduler_enabled",
    "auto_open_browser",
];
const CONFIG_ENUM_ALLOWLISTS: &[(&str, &[&str])] = &[
    (
        "deployment_mode",
        &["trusted_local_only", "hosted_public_join"],
    ),
    ("agent_decision_source", &["builtin_llm", "provider_backed"]),
    ("agent_provider_backend", &["provider_local_bridge"]),
    ("agent_provider_contract", &["worldsim_provider_v1"]),
    (
        "agent_provider_transport",
        &["loopback_http", "remote_https"],
    ),
    ("agent_execution_lane", &["player_parity", "headless_agent"]),
    (
        "chain_network_tier",
        &["local_devnet", "public_testnet", "mainnet"],
    ),
    ("chain_node_role", &["sequencer", "validator", "storage"]),
    (
        "chain_p2p_user_mode",
        &["auto_join", "private_safe", "public_entry"],
    ),
];
const CONFIG_NUMBER_ALLOWLIST: &[(&str, u64, u64)] = &[
    ("agent_provider_connect_timeout_ms", 1, 120_000),
    ("chain_node_tick_ms", 1, 60_000),
    ("chain_pos_slot_duration_ms", 1, 60_000),
    ("chain_pos_ticks_per_slot", 1, 10_000),
    ("chain_pos_proposal_tick_phase", 0, 10_000),
    ("chain_pos_max_past_slot_lag", 0, 1_000_000),
];
const CREDENTIAL_KEY_SUFFIXES: &[&str] = &[
    "secretaccesskey",
    "clientsecret",
    "refreshtoken",
    "accesstoken",
    "privatekey",
    "accesskey",
    "apikey",
    "secretkey",
    "authorization",
    "credentials",
    "credential",
    "password",
    "passwd",
    "secret",
    "token",
    "bearer",
];

pub(super) fn capture_feedback_diagnostics(
    config: &LaunchConfig,
    feedback_output_dir: &str,
    logs: &[String],
) -> Result<(Value, Vec<String>), String> {
    let config_value = serde_json::to_value(config)
        .map_err(|_| "failed to serialize launcher diagnostics".to_string())?;
    let config_fields = config_value
        .as_object()
        .ok_or_else(|| "launcher diagnostics did not serialize as an object".to_string())?;

    let mut safe_config = Map::new();
    for field in CONFIG_BOOL_ALLOWLIST {
        if let Some(Value::Bool(value)) = config_fields.get(*field) {
            safe_config.insert((*field).to_string(), Value::Bool(*value));
        }
    }
    for (field, allowed_values) in CONFIG_ENUM_ALLOWLISTS {
        if let Some(raw) = config_fields.get(*field).and_then(Value::as_str)
            && let Some(value) = allowed_values.iter().find(|allowed| **allowed == raw)
        {
            safe_config.insert((*field).to_string(), Value::String((*value).to_string()));
        }
    }
    for (field, minimum, maximum) in CONFIG_NUMBER_ALLOWLIST {
        if let Some(raw) = config_fields.get(*field).and_then(Value::as_str)
            && let Some(value) = bounded_config_number(field, raw)
            && value >= *minimum
            && value <= *maximum
        {
            safe_config.insert((*field).to_string(), Value::Number(Number::from(value)));
        }
    }

    let mut sensitive_values = config_fields
        .iter()
        .filter_map(|(field, value)| {
            let raw = value.as_str()?.trim();
            if raw.is_empty()
                || allowlisted_config_enum(field, raw).is_some()
                || bounded_config_number(field, raw).is_some()
            {
                return None;
            }
            Some(raw.to_string())
        })
        .collect::<Vec<_>>();
    if !feedback_output_dir.trim().is_empty() {
        sensitive_values.push(feedback_output_dir.trim().to_string());
    }
    sensitive_values.sort_by_key(|value| std::cmp::Reverse(value.len()));
    sensitive_values.dedup();

    let mut recent_logs = logs
        .iter()
        .rev()
        .take(FEEDBACK_LOG_SNAPSHOT_LIMIT)
        .map(|line| sanitize_feedback_log_line(line, &sensitive_values))
        .collect::<Vec<_>>();
    recent_logs.reverse();

    Ok((Value::Object(safe_config), recent_logs))
}

fn allowlisted_config_enum<'a>(field: &str, raw: &'a str) -> Option<&'a str> {
    CONFIG_ENUM_ALLOWLISTS
        .iter()
        .find(|(allowed_field, _)| *allowed_field == field)
        .and_then(|(_, allowed_values)| {
            allowed_values
                .iter()
                .find(|allowed| **allowed == raw)
                .map(|_| raw)
        })
}

fn bounded_config_number(field: &str, raw: &str) -> Option<u64> {
    let (_, minimum, maximum) = CONFIG_NUMBER_ALLOWLIST
        .iter()
        .find(|(allowed_field, _, _)| *allowed_field == field)?;
    let parsed = raw.parse::<u64>().ok()?;
    (parsed >= *minimum && parsed <= *maximum).then_some(parsed)
}

fn sanitize_feedback_log_line(line: &str, sensitive_values: &[String]) -> String {
    let mut sanitized = line.to_string();
    for value in sensitive_values {
        if !value.is_empty() {
            sanitized = sanitized.replace(value, "<redacted>");
        }
    }
    let sanitized = redact_credential_assignments(&sanitized);
    let sanitized = redact_urls(&sanitized);
    let sanitized = redact_endpoint_authorities(&sanitized);
    let sanitized = redact_feedback_bundle_paths(&sanitized);
    redact_private_paths(&sanitized)
}

fn redact_credential_assignments(line: &str) -> String {
    let mut sanitized = line.to_string();
    let mut search_from = 0;
    for _ in 0..32 {
        let Some((key_end, key)) = find_credential_key(&sanitized, search_from) else {
            break;
        };
        match credential_value_span(&sanitized, key_end, key) {
            CredentialValueSpan::Value(span) if !span.is_empty() => {
                sanitized.replace_range(span.clone(), "<redacted>");
                search_from = span.start + "<redacted>".len();
            }
            CredentialValueSpan::Value(_) | CredentialValueSpan::NoValue => {
                search_from = key_end;
            }
            CredentialValueSpan::Unbounded => {
                return "<redacted diagnostic line>".to_string();
            }
        }
    }
    sanitized
}

#[derive(Debug)]
enum CredentialValueSpan {
    Value(Range<usize>),
    NoValue,
    Unbounded,
}

fn find_credential_key(line: &str, search_from: usize) -> Option<(usize, &'static str)> {
    let bytes = line.as_bytes();
    let mut cursor = search_from;
    while cursor < bytes.len() {
        if !is_credential_key_byte(bytes[cursor]) {
            cursor += 1;
            continue;
        }
        let start = cursor;
        while bytes
            .get(cursor)
            .is_some_and(|byte| is_credential_key_byte(*byte))
        {
            cursor += 1;
        }
        let key_end = cursor;
        let token = line[start..key_end]
            .bytes()
            .filter(|byte| *byte != b'_' && *byte != b'-')
            .map(|byte| byte.to_ascii_lowercase() as char)
            .collect::<String>();
        if let Some(kind) = CREDENTIAL_KEY_SUFFIXES
            .iter()
            .find(|suffix| token.ends_with(**suffix))
        {
            return Some((key_end, *kind));
        }
    }
    None
}

fn is_credential_key_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

fn credential_value_span(line: &str, key_end: usize, key: &str) -> CredentialValueSpan {
    let bytes = line.as_bytes();
    let mut cursor = key_end;
    if matches!(bytes.get(cursor), Some(b'\'' | b'"')) {
        cursor += 1;
    }
    let after_key = cursor;
    while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }

    if key == "bearer" && cursor > after_key {
        let end = line[cursor..]
            .find([' ', '\t', ',', ';', '&', '}', ']', ')'])
            .map_or(line.len(), |offset| cursor + offset);
        return if end > cursor {
            CredentialValueSpan::Value(cursor..end)
        } else {
            CredentialValueSpan::NoValue
        };
    }

    if !matches!(bytes.get(cursor), Some(b'=' | b':')) {
        return CredentialValueSpan::NoValue;
    }
    cursor += 1;
    while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }

    if matches!(bytes.get(cursor), Some(b'\'' | b'"')) {
        let quote = bytes[cursor];
        let value_start = cursor + 1;
        let mut value_end = value_start;
        while value_end < bytes.len() {
            if bytes[value_end] == b'\\' {
                value_end = value_end.saturating_add(2);
                continue;
            }
            if bytes[value_end] == quote {
                return CredentialValueSpan::Value(value_start..value_end);
            }
            value_end += 1;
        }
        return CredentialValueSpan::Unbounded;
    }

    if cursor == line.len() {
        return CredentialValueSpan::NoValue;
    }
    if key == "authorization" {
        return CredentialValueSpan::Value(cursor..line.len());
    }
    let end = line[cursor..]
        .find([' ', '\t', ',', ';', '&', '}', ']', ')'])
        .map_or(line.len(), |offset| cursor + offset);
    CredentialValueSpan::Value(cursor..end)
}

fn redact_urls(line: &str) -> String {
    const URL_SCHEMES: &[&str] = &["https://", "http://", "wss://", "ws://", "file://"];
    let lowercase = line.to_ascii_lowercase();
    let mut output = String::with_capacity(line.len());
    let mut cursor = 0;
    while cursor < line.len() {
        let next = URL_SCHEMES
            .iter()
            .filter_map(|scheme| {
                lowercase[cursor..]
                    .find(scheme)
                    .map(|offset| cursor + offset)
            })
            .min();
        let Some(start) = next else {
            output.push_str(&line[cursor..]);
            break;
        };
        output.push_str(&line[cursor..start]);
        let mut end = start;
        for (offset, character) in line[start..].char_indices() {
            if offset == 0 {
                continue;
            }
            if character.is_whitespace()
                || matches!(character, '"' | '\'' | '<' | '>' | ')' | ']' | '}')
            {
                break;
            }
            end = start + offset + character.len_utf8();
        }
        if end == start {
            output.push_str(&line[start..]);
            break;
        }
        output.push_str("<endpoint>");
        cursor = end;
    }
    output
}

fn redact_endpoint_authorities(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut spans = Vec::new();
    for colon in 0..bytes.len() {
        if bytes[colon] != b':' || !bytes.get(colon + 1).is_some_and(u8::is_ascii_digit) {
            continue;
        }
        let mut port_end = colon + 1;
        while bytes.get(port_end).is_some_and(u8::is_ascii_digit) {
            port_end += 1;
        }
        if bytes
            .get(port_end)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
        {
            continue;
        }
        let Ok(port) = line[colon + 1..port_end].parse::<u16>() else {
            continue;
        };
        if port == 0 {
            continue;
        }
        let mut host_start = colon;
        while host_start > 0
            && matches!(
                bytes[host_start - 1],
                b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'-' | b'[' | b']' | b':'
            )
        {
            host_start -= 1;
        }
        let authority = &line[host_start..colon];
        let host = authority.trim_matches(['[', ']']);
        if host.contains('.')
            || host.eq_ignore_ascii_case("localhost")
            || (authority.starts_with('[') && authority.ends_with(']'))
        {
            spans.push(host_start..port_end);
        }
    }

    if spans.is_empty() {
        return line.to_string();
    }
    spans.sort_by_key(|span| span.start);
    spans.dedup();
    let mut output = String::with_capacity(line.len());
    let mut cursor = 0;
    for span in spans {
        if span.start < cursor {
            continue;
        }
        output.push_str(&line[cursor..span.start]);
        output.push_str("<endpoint>");
        cursor = span.end;
    }
    output.push_str(&line[cursor..]);
    output
}

fn redact_feedback_bundle_paths(line: &str) -> String {
    let lowercase = line.to_ascii_lowercase();
    let mut spans = Vec::new();
    for suffix in ["-bug.json", "-suggestion.json"] {
        let mut search_from = 0;
        while let Some(offset) = lowercase[search_from..].find(suffix) {
            let suffix_start = search_from + offset;
            let mut start = suffix_start;
            while start > 0
                && !matches!(
                    line.as_bytes()[start - 1],
                    b' ' | b'\t'
                        | b'\n'
                        | b'\r'
                        | b'"'
                        | b'\''
                        | b'('
                        | b'['
                        | b'{'
                        | b'<'
                        | b':'
                        | b'='
                )
            {
                start -= 1;
            }
            spans.push(start..suffix_start + suffix.len());
            search_from = suffix_start + suffix.len();
        }
    }
    if spans.is_empty() {
        return line.to_string();
    }
    spans.sort_by_key(|span| span.start);
    spans.dedup();
    let mut output = String::with_capacity(line.len());
    let mut cursor = 0;
    for span in spans {
        if span.start < cursor {
            continue;
        }
        output.push_str(&line[cursor..span.start]);
        output.push_str("<feedback-bundle-path>");
        cursor = span.end;
    }
    output.push_str(&line[cursor..]);
    output
}

fn redact_private_paths(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut output = String::with_capacity(line.len());
    let mut cursor = 0;
    while cursor < line.len() {
        let Some((start, windows_path)) = find_absolute_path_start(line, cursor) else {
            output.push_str(&line[cursor..]);
            break;
        };
        output.push_str(&line[cursor..start]);
        let mut end = start;
        for (offset, character) in line[start..].char_indices() {
            if offset == 0 {
                continue;
            }
            if character.is_whitespace()
                || matches!(
                    character,
                    '"' | '\'' | '<' | '>' | ',' | ';' | ')' | ']' | '}'
                )
            {
                break;
            }
            end = start + offset + character.len_utf8();
        }
        if end == start {
            output.push(bytes[start] as char);
            cursor = start + 1;
            continue;
        }
        let path = &line[start..end];
        let safe_route = !windows_path
            && ["/api/", "/v1/", "/v2/", "/health", "/status", "/metrics"]
                .iter()
                .any(|prefix| path.starts_with(prefix));
        if safe_route {
            output.push_str(path);
        } else {
            output.push_str("<path>");
        }
        cursor = end;
    }
    output
}

fn find_absolute_path_start(line: &str, from: usize) -> Option<(usize, bool)> {
    let bytes = line.as_bytes();
    for index in from..bytes.len() {
        let previous_is_boundary = index == 0
            || matches!(
                bytes[index - 1],
                b' ' | b'\t' | b'=' | b':' | b'(' | b'[' | b'{' | b'<' | b'"' | b'\'' | b','
            );
        if !previous_is_boundary {
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1).is_some_and(u8::is_ascii_alphabetic) {
            return Some((index, false));
        }
        if index + 2 < bytes.len()
            && bytes[index].is_ascii_alphabetic()
            && bytes[index + 1] == b':'
            && matches!(bytes[index + 2], b'/' | b'\\')
        {
            return Some((index, true));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::capture_feedback_diagnostics;
    use crate::LaunchConfig;

    #[test]
    fn feedback_diagnostic_config_is_a_small_allowlist() {
        let config = LaunchConfig {
            agent_provider_auth_token: "configured-secret".to_string(),
            agent_provider_url: "https://provider.private.test/api".to_string(),
            viewer_static_dir: "/Users/private-user/game".to_string(),
            chain_enabled: true,
            chain_network_tier: "public_testnet".to_string(),
            agent_provider_connect_timeout_ms: "15000".to_string(),
            ..LaunchConfig::default()
        };

        let (snapshot, _) = capture_feedback_diagnostics(&config, "/Users/private-user/out", &[])
            .expect("diagnostics should be captured");

        assert_eq!(snapshot["chain_enabled"], true);
        assert_eq!(snapshot["chain_network_tier"], "public_testnet");
        assert_eq!(snapshot["agent_provider_connect_timeout_ms"], 15000);
        assert!(snapshot.get("agent_provider_auth_token").is_none());
        assert!(snapshot.get("agent_provider_url").is_none());
        assert!(snapshot.get("viewer_static_dir").is_none());
        assert!(!snapshot.to_string().contains("configured-secret"));
    }

    #[test]
    fn feedback_logs_redact_credentials_endpoints_and_private_paths() {
        let config = LaunchConfig {
            agent_provider_auth_token: "configured-secret".to_string(),
            chain_status_bind: "chain.private.test:5121".to_string(),
            ..LaunchConfig::default()
        };
        let logs = vec![
            "provider request failed with configured-secret".to_string(),
            "authorization: Bearer stand-alone-log-secret".to_string(),
            "remote submit at https://old.endpoint.private/v1?token=url-secret failed".to_string(),
            "previous chain error from old-chain.private.test:7341".to_string(),
            "previous bundle at /Users/previous-user/feedback/report.json".to_string(),
            "previous relative bundle feedback/20261008T173012Z-bug.json".to_string(),
            "chain runtime exited with code 7".to_string(),
        ];

        let (_, safe_logs) = capture_feedback_diagnostics(&config, "/tmp/current-feedback", &logs)
            .expect("diagnostics should be captured");
        let bundle_logs = safe_logs.join("\n");

        for (label, secret) in [
            ("configured credential", "configured-secret"),
            ("bearer credential", "stand-alone-log-secret"),
            ("old URL host", "old.endpoint.private"),
            ("URL query credential", "url-secret"),
            ("old endpoint authority", "old-chain.private.test:7341"),
            (
                "previous feedback path",
                "/Users/previous-user/feedback/report.json",
            ),
            (
                "relative feedback bundle path",
                "feedback/20261008T173012Z-bug.json",
            ),
            ("current output path", "/tmp/current-feedback"),
            ("configured endpoint", "chain.private.test:5121"),
        ] {
            assert!(!bundle_logs.contains(secret), "log leaked {label}");
        }
        assert!(bundle_logs.contains("chain runtime exited with code 7"));
        assert!(bundle_logs.contains("<endpoint>"));
        assert!(bundle_logs.contains("<feedback-bundle-path>"));
        assert!(bundle_logs.contains("<path>"));
    }

    #[test]
    fn feedback_logs_redact_env_camel_case_and_escaped_credential_values() {
        let logs = vec![
            "OPENAI_API_KEY=openai-env-secret".to_string(),
            "GITHUB_TOKEN=github-env-secret".to_string(),
            "AWS_SECRET_ACCESS_KEY=aws-secret-access-value".to_string(),
            "OAUTH_CLIENT_SECRET=oauth-client-secret-value".to_string(),
            "accessToken=camel-case-access-secret".to_string(),
            r#"api_key="prefix-\"escaped-secret\"-suffix" safe-context=retained"#.to_string(),
            "client_secret=\"unterminated-secret".to_string(),
        ];
        let (_, safe_logs) = capture_feedback_diagnostics(&LaunchConfig::default(), "", &logs)
            .expect("diagnostics should be captured");

        for (label, secret) in [
            ("OpenAI environment key", "openai-env-secret"),
            ("GitHub token", "github-env-secret"),
            ("AWS secret access key", "aws-secret-access-value"),
            ("OAuth client secret", "oauth-client-secret-value"),
            ("camelCase token", "camel-case-access-secret"),
            ("escaped quoted secret", "escaped-secret"),
            ("unterminated quoted secret", "unterminated-secret"),
        ] {
            assert!(!safe_logs.join("\n").contains(secret), "log leaked {label}");
        }
        assert!(safe_logs[5].contains("safe-context=retained"));
        assert_eq!(safe_logs[6], "<redacted diagnostic line>");
    }

    #[test]
    fn feedback_log_snapshot_retains_the_latest_200_lines() {
        let logs = (0..210)
            .map(|index| format!("safe line {index}"))
            .collect::<Vec<_>>();
        let (_, snapshot) = capture_feedback_diagnostics(&LaunchConfig::default(), "", &logs)
            .expect("diagnostics should be captured");

        assert_eq!(snapshot.len(), 200);
        assert_eq!(snapshot.first().map(String::as_str), Some("safe line 10"));
        assert_eq!(snapshot.last().map(String::as_str), Some("safe line 209"));
    }
}
