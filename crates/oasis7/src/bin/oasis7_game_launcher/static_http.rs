use super::hosted_account_identity::{
    HostedAccountIdentityBroker, HostedAccountLoginCompleteResponse,
    HostedAccountLoginStartResponse, HOSTED_ACCOUNT_LOGIN_COMPLETE_ROUTE,
    HOSTED_ACCOUNT_LOGIN_START_ROUTE,
};
use super::hosted_player_session::{
    HostedPlayerSessionAdmissionResponse, HostedPlayerSessionIssueResponse,
    HostedPlayerSessionIssuer, HostedPlayerSessionReleaseResponse,
    HOSTED_PLAYER_SESSION_ADMISSION_ROUTE, HOSTED_PLAYER_SESSION_ISSUE_ROUTE,
    HOSTED_PLAYER_SESSION_REFRESH_ROUTE, HOSTED_PLAYER_SESSION_RELEASE_ROUTE,
};
use super::hosted_strong_auth::{
    issue_hosted_strong_auth_grant, HostedStrongAuthGrantResponse,
    HOSTED_PROMPT_CONTROL_STRONG_AUTH_GRANT_ROUTE, HOSTED_STRONG_AUTH_GRANT_ROUTE,
};
use super::runtime_presence::query_runtime_bound_players;
use super::*;
use serde::Serialize;
use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub(super) fn handle_http_connection(
    mut stream: TcpStream,
    root_dir: &Path,
    live_bind: &str,
    default_viewer_player_id: Option<&str>,
    deployment_mode: DeploymentMode,
    hosted_session_issuer: &Arc<Mutex<HostedPlayerSessionIssuer>>,
    hosted_account_broker: &Arc<Mutex<HostedAccountIdentityBroker>>,
) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|err| format!("failed to set read timeout: {err}"))?;

    let mut buffer = [0u8; 8192];
    let bytes = stream
        .read(&mut buffer)
        .map_err(|err| format!("failed to read request: {err}"))?;
    if bytes == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..bytes]);
    let Some(line) = request.lines().next() else {
        write_http_response(&mut stream, 400, "text/plain", b"Bad Request", false)
            .map_err(|err| format!("failed to write 400 response: {err}"))?;
        return Ok(());
    };

    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");

    let head_only = method.eq_ignore_ascii_case("HEAD");
    let path_only = target.split('?').next().unwrap_or(target);
    let is_admission_route = path_only == HOSTED_PLAYER_SESSION_ADMISSION_ROUTE;
    let is_refresh_route = path_only == HOSTED_PLAYER_SESSION_REFRESH_ROUTE;
    let is_issue_route = path_only == HOSTED_PLAYER_SESSION_ISSUE_ROUTE;
    let is_release_route = path_only == HOSTED_PLAYER_SESSION_RELEASE_ROUTE;
    let is_login_start_route = path_only == HOSTED_ACCOUNT_LOGIN_START_ROUTE;
    let is_login_complete_route = path_only == HOSTED_ACCOUNT_LOGIN_COMPLETE_ROUTE;
    let is_strong_auth_grant_route = path_only == HOSTED_STRONG_AUTH_GRANT_ROUTE
        || path_only == HOSTED_PROMPT_CONTROL_STRONG_AUTH_GRANT_ROUTE;
    let is_hosted_player_route = is_admission_route
        || is_refresh_route
        || is_issue_route
        || is_release_route
        || is_login_start_route
        || is_login_complete_route
        || is_strong_auth_grant_route;
    let allow_post =
        is_release_route || is_refresh_route || is_login_start_route || is_login_complete_route;
    if !method.eq_ignore_ascii_case("GET")
        && !head_only
        && !(allow_post && method.eq_ignore_ascii_case("POST"))
    {
        write_http_response(&mut stream, 405, "text/plain", b"Method Not Allowed", false)
            .map_err(|err| format!("failed to write 405 response: {err}"))?;
        return Ok(());
    }
    if is_hosted_player_route {
        reconcile_hosted_runtime_presence(live_bind, hosted_session_issuer);
    }
    if is_admission_route {
        let response = hosted_player_session_admission(deployment_mode, hosted_session_issuer)?;
        write_json_response(&mut stream, 200, &response, head_only)
            .map_err(|err| format!("failed to write hosted session admission response: {err}"))?;
        return Ok(());
    }
    if is_refresh_route {
        let player_id = parse_query_value(target, "player_id").unwrap_or_default();
        let release_token = parse_query_value(target, "release_token").unwrap_or_default();
        let response = refresh_hosted_player_session(
            deployment_mode,
            player_id.as_str(),
            release_token.as_str(),
            hosted_session_issuer,
        )?;
        write_json_response(&mut stream, 200, &response, head_only)
            .map_err(|err| format!("failed to write hosted session refresh response: {err}"))?;
        return Ok(());
    }
    if is_issue_route {
        let response = issue_hosted_player_session(deployment_mode, hosted_session_issuer)?;
        write_json_response(&mut stream, 200, &response, head_only)
            .map_err(|err| format!("failed to write hosted session issue response: {err}"))?;
        return Ok(());
    }
    if is_release_route {
        let player_id = parse_query_value(target, "player_id").unwrap_or_default();
        let release_token = parse_query_value(target, "release_token").unwrap_or_default();
        let response = release_hosted_player_session(
            deployment_mode,
            player_id.as_str(),
            release_token.as_str(),
            hosted_session_issuer,
        )?;
        write_json_response(&mut stream, 200, &response, head_only)
            .map_err(|err| format!("failed to write hosted session release response: {err}"))?;
        return Ok(());
    }
    if is_login_start_route {
        let login_channel = parse_query_value(target, "channel").unwrap_or_default();
        let login_hint = parse_query_value(target, "handle").unwrap_or_default();
        let response = start_hosted_account_login(
            deployment_mode,
            login_channel.as_str(),
            login_hint.as_str(),
            hosted_account_broker,
        )?;
        write_json_response(&mut stream, 200, &response, head_only)
            .map_err(|err| format!("failed to write hosted account login start response: {err}"))?;
        return Ok(());
    }
    if is_login_complete_route {
        let challenge_id = parse_query_value(target, "challenge_id").unwrap_or_default();
        let otp_code = parse_query_value(target, "otp_code").unwrap_or_default();
        let response = complete_hosted_account_login(
            deployment_mode,
            challenge_id.as_str(),
            otp_code.as_str(),
            hosted_session_issuer,
            hosted_account_broker,
        )?;
        write_json_response(&mut stream, 200, &response, head_only).map_err(|err| {
            format!("failed to write hosted account login complete response: {err}")
        })?;
        return Ok(());
    }
    if is_strong_auth_grant_route {
        let player_id = parse_query_value(target, "player_id").unwrap_or_default();
        let public_key = parse_query_value(target, "public_key").unwrap_or_default();
        let agent_id = parse_query_value(target, "agent_id").unwrap_or_default();
        let action_id = parse_query_value(target, "action_id").unwrap_or_default();
        let approval_code = parse_query_value(target, "approval_code").unwrap_or_default();
        let release_token = parse_query_value(target, "release_token").unwrap_or_default();
        let response = issue_strong_auth_grant(
            deployment_mode,
            player_id.as_str(),
            public_key.as_str(),
            agent_id.as_str(),
            action_id.as_str(),
            approval_code.as_str(),
            release_token.as_str(),
            hosted_session_issuer,
        )?;
        write_json_response(&mut stream, 200, &response, head_only)
            .map_err(|err| format!("failed to write hosted strong-auth grant response: {err}"))?;
        return Ok(());
    }

    let resolved = match resolve_static_asset_path(root_dir, target) {
        Ok(resolved) => resolved,
        Err(_) => {
            write_http_response(&mut stream, 400, "text/plain", b"Bad Request", head_only)
                .map_err(|err| format!("failed to write 400 response: {err}"))?;
            return Ok(());
        }
    };

    match resolved {
        Some(path) => {
            let body = fs::read(&path).map_err(|err| {
                format!("failed to read static asset `{}`: {err}", path.display())
            })?;
            let viewer_auth_bootstrap = resolve_viewer_auth_bootstrap_for_embedded_server(
                deployment_mode,
                default_viewer_player_id,
            );
            let body = sanitize_index_html_for_embedded_server(
                path.as_path(),
                body.as_slice(),
                viewer_auth_bootstrap.as_ref(),
            );
            write_http_response(
                &mut stream,
                200,
                content_type_for_path(path.as_path()),
                body.as_slice(),
                head_only,
            )
            .map_err(|err| format!("failed to write 200 response: {err}"))?;
        }
        None => {
            write_http_response(&mut stream, 404, "text/plain", b"Not Found", head_only)
                .map_err(|err| format!("failed to write 404 response: {err}"))?;
        }
    }

    Ok(())
}

fn start_hosted_account_login(
    deployment_mode: DeploymentMode,
    login_channel: &str,
    login_hint: &str,
    hosted_account_broker: &Arc<Mutex<HostedAccountIdentityBroker>>,
) -> Result<HostedAccountLoginStartResponse, String> {
    let mut broker = hosted_account_broker
        .lock()
        .map_err(|_| "hosted account broker lock poisoned".to_string())?;
    Ok(broker.start_login(deployment_mode, login_channel, login_hint))
}

fn complete_hosted_account_login(
    deployment_mode: DeploymentMode,
    challenge_id: &str,
    otp_code: &str,
    hosted_session_issuer: &Arc<Mutex<HostedPlayerSessionIssuer>>,
    hosted_account_broker: &Arc<Mutex<HostedAccountIdentityBroker>>,
) -> Result<HostedAccountLoginCompleteResponse, String> {
    let mut broker = hosted_account_broker
        .lock()
        .map_err(|_| "hosted account broker lock poisoned".to_string())?;
    let mut issuer = hosted_session_issuer
        .lock()
        .map_err(|_| "hosted session issuer lock poisoned".to_string())?;
    Ok(broker.complete_login(deployment_mode, challenge_id, otp_code, &mut issuer))
}

fn reconcile_hosted_runtime_presence(
    live_bind: &str,
    hosted_session_issuer: &Arc<Mutex<HostedPlayerSessionIssuer>>,
) {
    let probe_result = query_runtime_bound_players(live_bind);
    let Ok(mut issuer) = hosted_session_issuer.lock() else {
        return;
    };
    match probe_result {
        Ok(active_players) => {
            issuer.observe_runtime_active_players(active_players.iter().map(String::as_str));
        }
        Err(err) => issuer.record_runtime_probe_failure(err),
    }
}

fn hosted_player_session_admission(
    deployment_mode: DeploymentMode,
    hosted_session_issuer: &Arc<Mutex<HostedPlayerSessionIssuer>>,
) -> Result<HostedPlayerSessionAdmissionResponse, String> {
    let mut issuer = hosted_session_issuer
        .lock()
        .map_err(|_| "hosted session issuer lock poisoned".to_string())?;
    Ok(issuer.admission(deployment_mode))
}

fn issue_hosted_player_session(
    deployment_mode: DeploymentMode,
    hosted_session_issuer: &Arc<Mutex<HostedPlayerSessionIssuer>>,
) -> Result<HostedPlayerSessionIssueResponse, String> {
    let mut issuer = hosted_session_issuer
        .lock()
        .map_err(|_| "hosted session issuer lock poisoned".to_string())?;
    Ok(issuer.issue(deployment_mode))
}

fn refresh_hosted_player_session(
    deployment_mode: DeploymentMode,
    player_id: &str,
    release_token: &str,
    hosted_session_issuer: &Arc<Mutex<HostedPlayerSessionIssuer>>,
) -> Result<HostedPlayerSessionAdmissionResponse, String> {
    let mut issuer = hosted_session_issuer
        .lock()
        .map_err(|_| "hosted session issuer lock poisoned".to_string())?;
    Ok(issuer.refresh(deployment_mode, player_id, release_token))
}

fn release_hosted_player_session(
    deployment_mode: DeploymentMode,
    player_id: &str,
    release_token: &str,
    hosted_session_issuer: &Arc<Mutex<HostedPlayerSessionIssuer>>,
) -> Result<HostedPlayerSessionReleaseResponse, String> {
    let mut issuer = hosted_session_issuer
        .lock()
        .map_err(|_| "hosted session issuer lock poisoned".to_string())?;
    Ok(issuer.release(deployment_mode, player_id, release_token))
}

fn issue_strong_auth_grant(
    deployment_mode: DeploymentMode,
    player_id: &str,
    public_key: &str,
    agent_id: &str,
    action_id: &str,
    approval_code: &str,
    release_token: &str,
    hosted_session_issuer: &Arc<Mutex<HostedPlayerSessionIssuer>>,
) -> Result<HostedStrongAuthGrantResponse, String> {
    let mut issuer = hosted_session_issuer
        .lock()
        .map_err(|_| "hosted session issuer lock poisoned".to_string())?;
    Ok(issue_hosted_strong_auth_grant(
        deployment_mode,
        player_id,
        public_key,
        agent_id,
        action_id,
        approval_code,
        release_token,
        &mut issuer,
    ))
}

fn parse_query_value(target: &str, key: &str) -> Option<String> {
    let query = target.split('?').nth(1)?;
    for pair in query.split('&') {
        let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
        if raw_key == key {
            return Some(percent_decode(raw_value));
        }
    }
    None
}

fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            let hi = from_hex(bytes[index + 1]);
            let lo = from_hex(bytes[index + 2]);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                decoded.push((hi << 4) | lo);
                index += 3;
                continue;
            }
        }
        decoded.push(if bytes[index] == b'+' {
            b' '
        } else {
            bytes[index]
        });
        index += 1;
    }
    String::from_utf8_lossy(decoded.as_slice()).into_owned()
}

fn from_hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub(super) fn resolve_static_asset_path(
    root_dir: &Path,
    raw_target: &str,
) -> Result<Option<PathBuf>, String> {
    let path_only = raw_target
        .split('?')
        .next()
        .unwrap_or(raw_target)
        .split('#')
        .next()
        .unwrap_or(raw_target);

    let relative = sanitize_relative_request_path(path_only)?;
    let direct_path = if relative.as_os_str().is_empty() {
        root_dir.join("index.html")
    } else {
        root_dir.join(relative.as_path())
    };

    if direct_path.is_file() {
        return Ok(Some(direct_path));
    }

    let has_extension = Path::new(path_only)
        .file_name()
        .and_then(|name| Path::new(name).extension())
        .is_some();
    if !has_extension {
        let spa_index = root_dir.join("index.html");
        if spa_index.is_file() {
            return Ok(Some(spa_index));
        }
    }

    Ok(None)
}

pub(super) fn sanitize_relative_request_path(raw_path: &str) -> Result<PathBuf, String> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return Ok(PathBuf::new());
    }

    let normalized = trimmed.strip_prefix('/').unwrap_or(trimmed);
    let mut cleaned = PathBuf::new();
    for segment in normalized.split('/') {
        if segment.is_empty() || segment == "." {
            continue;
        }
        if segment == ".." || segment.contains('\\') {
            return Err("path traversal is not allowed".to_string());
        }
        cleaned.push(segment);
    }

    Ok(cleaned)
}

pub(super) fn content_type_for_path(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("map") => "application/json; charset=utf-8",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

pub(super) fn sanitize_index_html_for_embedded_server(
    path: &Path,
    body: &[u8],
    viewer_auth_bootstrap: Option<&ViewerAuthBootstrap>,
) -> Vec<u8> {
    if path.extension() != Some(OsStr::new("html")) {
        return body.to_vec();
    }
    let sanitized = if path.file_name() == Some(OsStr::new("index.html")) {
        strip_trunk_autoreload_script(body)
    } else {
        body.to_vec()
    };
    if let Some(viewer_auth_bootstrap) = viewer_auth_bootstrap {
        inject_viewer_auth_bootstrap_script(sanitized.as_slice(), viewer_auth_bootstrap)
    } else {
        sanitized
    }
}

fn strip_trunk_autoreload_script(body: &[u8]) -> Vec<u8> {
    let html = String::from_utf8_lossy(body);
    let marker = ".well-known/trunk/ws";
    let Some(marker_index) = html.find(marker) else {
        return body.to_vec();
    };
    let Some(script_start) = html[..marker_index].rfind("<script") else {
        return body.to_vec();
    };
    let Some(script_end_rel) = html[marker_index..].find("</script>") else {
        return body.to_vec();
    };
    let script_end = marker_index + script_end_rel + "</script>".len();

    let mut sanitized = String::with_capacity(html.len());
    sanitized.push_str(&html[..script_start]);
    sanitized.push_str(&html[script_end..]);
    sanitized.into_bytes()
}

fn inject_viewer_auth_bootstrap_script(body: &[u8], auth: &ViewerAuthBootstrap) -> Vec<u8> {
    let html = String::from_utf8_lossy(body);
    let script = build_viewer_auth_bootstrap_script(auth);
    let insert_at = html
        .rfind("</head>")
        .or_else(|| html.rfind("</body>"))
        .unwrap_or(html.len());
    let mut injected = String::with_capacity(html.len() + script.len() + 1);
    injected.push_str(&html[..insert_at]);
    injected.push_str(script.as_str());
    injected.push_str(&html[insert_at..]);
    injected.into_bytes()
}

pub(super) fn build_viewer_auth_bootstrap_script(auth: &ViewerAuthBootstrap) -> String {
    let payload = serde_json::json!({
        VIEWER_PLAYER_ID_ENV: auth.player_id,
        VIEWER_AUTH_PUBLIC_KEY_ENV: auth.public_key,
        VIEWER_AUTH_PRIVATE_KEY_ENV: auth.private_key,
    });
    let payload = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
    format!(
        "<script>const __oasis7ViewerAuthEnv=Object.freeze({payload});window.{VIEWER_AUTH_BOOTSTRAP_OBJECT}=__oasis7ViewerAuthEnv;</script>"
    )
}

fn write_json_response<T: Serialize>(
    stream: &mut TcpStream,
    status_code: u16,
    payload: &T,
    head_only: bool,
) -> Result<(), String> {
    let body =
        serde_json::to_vec(payload).map_err(|err| format!("serialize JSON failed: {err}"))?;
    write_http_response(
        stream,
        status_code,
        "application/json; charset=utf-8",
        body.as_slice(),
        head_only,
    )
    .map_err(|err| format!("write JSON response failed: {err}"))
}

fn write_http_response(
    stream: &mut TcpStream,
    status_code: u16,
    content_type: &str,
    body: &[u8],
    head_only: bool,
) -> std::io::Result<()> {
    let status_text = match status_code {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        _ => "Internal Server Error",
    };
    let headers = format!(
        "HTTP/1.1 {status_code} {status_text}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes())?;
    if !head_only {
        stream.write_all(body)?;
    }
    stream.flush()?;
    Ok(())
}

pub(super) fn resolve_viewer_auth_bootstrap_from_path(
    path: &Path,
    default_viewer_player_id: Option<&str>,
) -> Result<ViewerAuthBootstrap, String> {
    let content =
        fs::read_to_string(path).map_err(|err| format!("read {} failed: {err}", path.display()))?;
    let value: toml::Value = toml::from_str(content.as_str())
        .map_err(|err| format!("parse {} failed: {err}", path.display()))?;
    let node = value
        .get(NODE_TABLE_KEY)
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("{NODE_TABLE_KEY} table is missing in {}", path.display()))?;
    let private_key =
        resolve_required_toml_string(node, NODE_PRIVATE_KEY_FIELD, "node.private_key")?;
    let public_key = resolve_required_toml_string(node, NODE_PUBLIC_KEY_FIELD, "node.public_key")?;
    let player_id = resolve_viewer_player_id_override(
        env::var(VIEWER_PLAYER_ID_ENV).ok(),
        default_viewer_player_id,
    );
    Ok(ViewerAuthBootstrap {
        player_id,
        public_key,
        private_key,
    })
}

pub(super) fn resolve_viewer_auth_bootstrap_for_embedded_server(
    deployment_mode: DeploymentMode,
    default_viewer_player_id: Option<&str>,
) -> Option<ViewerAuthBootstrap> {
    if deployment_mode.disables_browser_signer_bootstrap() {
        None
    } else {
        resolve_viewer_auth_bootstrap_from_path(
            Path::new(NODE_CONFIG_FILE_NAME),
            default_viewer_player_id,
        )
        .ok()
    }
}
