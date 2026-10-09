use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::thread;

use super::super::{content_type_for_path, resolve_static_asset_path};
use super::super::{
    missing_execution_world_persistence_files, sanitize_index_html_for_embedded_server,
    sanitize_relative_request_path, start_static_http_server, stop_static_http_server,
};
use super::{DeploymentMode, make_temp_dir};
use crate::static_http;

// Declared after the environment fixture, so unwind stops all readers before
// the fixture restores process-global values and unlocks.
struct HostedTestHttpServer(crate::StaticHttpServer);

impl Drop for HostedTestHttpServer {
    fn drop(&mut self) {
        stop_static_http_server(&mut self.0);
    }
}

#[test]
fn hosted_test_login_host_gate_accepts_only_loopback_addresses() {
    assert!(static_http::hosted_test_login_allowed_on_host("127.0.0.1"));
    assert!(static_http::hosted_test_login_allowed_on_host("[::1]"));
    assert!(static_http::hosted_test_login_allowed_on_host("localhost"));
    assert!(!static_http::hosted_test_login_allowed_on_host("0.0.0.0"));
    assert!(!static_http::hosted_test_login_allowed_on_host("::"));
    assert!(!static_http::hosted_test_login_allowed_on_host(
        "192.0.2.10"
    ));
}

#[test]
fn sanitize_index_html_for_embedded_server_keeps_non_index_files_unchanged() {
    let body = b"<script>.well-known/trunk/ws</script>";
    let sanitized = sanitize_index_html_for_embedded_server(Path::new("app.js"), body, None);
    assert_eq!(sanitized, body);
}

#[test]
fn missing_execution_world_persistence_files_reports_snapshot_and_journal() {
    let temp_dir = make_temp_dir("execution_world_missing");
    let missing = missing_execution_world_persistence_files(temp_dir.as_path());
    assert_eq!(missing.len(), 2);
    assert!(missing.iter().any(|path| path.ends_with("snapshot.json")));
    assert!(missing.iter().any(|path| path.ends_with("journal.json")));
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn missing_execution_world_persistence_files_ignores_ready_world_dir() {
    let temp_dir = make_temp_dir("execution_world_ready");
    fs::write(temp_dir.join("snapshot.json"), "{}").expect("write snapshot");
    fs::write(temp_dir.join("journal.json"), "{}").expect("write journal");
    let missing = missing_execution_world_persistence_files(temp_dir.as_path());
    assert!(missing.is_empty());
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn sanitize_relative_request_path_rejects_traversal() {
    let err = sanitize_relative_request_path("/../etc/passwd").expect_err("should fail");
    assert!(err.contains("traversal"));
}

#[test]
fn resolve_static_asset_path_supports_spa_fallback() {
    let temp_dir = make_temp_dir("spa_fallback");
    fs::write(temp_dir.join("index.html"), "<html>ok</html>").expect("write index");
    let resolved = resolve_static_asset_path(temp_dir.as_path(), "/app/route?x=1")
        .expect("resolve should succeed")
        .expect("should fallback to index");
    assert_eq!(resolved, temp_dir.join("index.html"));
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn resolve_static_asset_path_returns_none_for_missing_static_asset() {
    let temp_dir = make_temp_dir("missing_asset");
    fs::write(temp_dir.join("index.html"), "<html>ok</html>").expect("write index");
    let resolved = resolve_static_asset_path(temp_dir.as_path(), "/assets/missing.js")
        .expect("resolve should succeed");
    assert!(resolved.is_none());
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn content_type_for_path_covers_wasm_and_js() {
    assert_eq!(
        content_type_for_path(Path::new("a.wasm")),
        "application/wasm"
    );
    assert_eq!(
        content_type_for_path(Path::new("a.js")),
        "text/javascript; charset=utf-8"
    );
}

#[test]
fn sanitize_index_html_for_embedded_server_removes_trunk_reload_script() {
    let html = concat!(
        "<html><body>",
        "<script>window.bootstrap = true;</script>",
        "<script>const url = 'ws://{{__TRUNK_ADDRESS__}}{{__TRUNK_WS_BASE__}}.well-known/trunk/ws';</script>",
        "</body></html>"
    );
    let sanitized =
        sanitize_index_html_for_embedded_server(Path::new("index.html"), html.as_bytes(), None);
    let sanitized = String::from_utf8(sanitized).expect("utf-8");
    assert!(sanitized.contains("window.bootstrap = true"));
    assert!(!sanitized.contains(".well-known/trunk/ws"));
    assert!(!sanitized.contains("__TRUNK_ADDRESS__"));
}

#[test]
fn static_http_server_serves_large_static_asset_completely() {
    let temp_dir = make_temp_dir("large_static_asset");
    let large_body = vec![b'a'; 512 * 1024];
    fs::write(temp_dir.join("viewer.js"), &large_body).expect("write large asset");

    let probe = TcpListener::bind(("127.0.0.1", 0)).expect("bind port probe");
    let port = probe.local_addr().expect("probe addr").port();
    drop(probe);

    let mut server = start_static_http_server(
        DeploymentMode::TrustedLocalOnly,
        "127.0.0.1:0",
        "127.0.0.1:5011",
        "127.0.0.1",
        port,
        temp_dir.as_path(),
        None,
    )
    .expect("start static HTTP server");

    let mut response = Vec::new();
    for _ in 0..50 {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(mut stream) => {
                stream
                    .write_all(b"GET /viewer.js HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
                    .expect("write request");
                stream.read_to_end(&mut response).expect("read response");
                break;
            }
            Err(_) => thread::sleep(std::time::Duration::from_millis(20)),
        }
    }
    stop_static_http_server(&mut server);

    let split_at = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("response headers end")
        + 4;
    assert!(String::from_utf8_lossy(&response[..split_at]).starts_with("HTTP/1.1 200 OK"));
    assert_eq!(&response[split_at..], large_body.as_slice());
}

#[test]
fn hosted_public_unauthenticated_get_cannot_issue_player_session() {
    let temp_dir = make_temp_dir("unauthenticated_issue");
    fs::write(temp_dir.join("index.html"), b"ok").expect("write index");
    let probe = TcpListener::bind(("127.0.0.1", 0)).expect("bind port probe");
    let port = probe.local_addr().expect("probe addr").port();
    drop(probe);
    let mut server = start_static_http_server(
        DeploymentMode::HostedPublicJoin,
        "127.0.0.1:0",
        "127.0.0.1:5011",
        "127.0.0.1",
        port,
        temp_dir.as_path(),
        None,
    )
    .expect("start static HTTP server");

    let mut response = Vec::new();
    for _ in 0..50 {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(mut stream) => {
                stream
                    .write_all(
                        b"GET /api/public/player-session/issue HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
                    )
                    .expect("write request");
                stream.read_to_end(&mut response).expect("read response");
                break;
            }
            Err(_) => thread::sleep(std::time::Duration::from_millis(20)),
        }
    }
    stop_static_http_server(&mut server);

    assert!(
        String::from_utf8_lossy(&response).starts_with("HTTP/1.1 405 Method Not Allowed"),
        "public unauthenticated GET issue route must not allocate a session"
    );
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn hosted_test_login_requires_opt_in_and_returns_server_issued_grant() {
    let issuer_key = hex::encode([71_u8; 32]);
    let disabled_values = [(
        crate::hosted_test_env::ISSUER,
        std::ffi::OsStr::new(&issuer_key),
    )];
    let enabled_values = [
        disabled_values[0],
        (crate::hosted_test_env::LOGIN, std::ffi::OsStr::new("1")),
    ];
    let Some(scenario) = crate::hosted_test_env::run_scenarios(&[
        ("disabled", &disabled_values),
        ("enabled", &enabled_values),
    ]) else {
        return;
    };
    let temp_dir = make_temp_dir("hosted_test_login");
    fs::write(temp_dir.join("index.html"), b"ok").expect("write index");
    let probe = TcpListener::bind(("127.0.0.1", 0)).expect("bind port probe");
    let port = probe.local_addr().expect("probe addr").port();
    drop(probe);
    let mut server = start_static_http_server(
        DeploymentMode::HostedPublicJoin,
        "127.0.0.1:0",
        "127.0.0.1:5011",
        "127.0.0.1",
        port,
        temp_dir.as_path(),
        None,
    )
    .map(HostedTestHttpServer)
    .expect("start static HTTP server");
    let body =
        r#"{"public_key":"4848484848484848484848484848484848484848484848484848484848484848"}"#;
    let request = format!(
        "POST /api/public/hosted-account/test-login HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
        body.len(),
        body
    );
    let send = || {
        let mut response = Vec::new();
        for _ in 0..50 {
            match TcpStream::connect(("127.0.0.1", port)) {
                Ok(mut stream) => {
                    stream.write_all(request.as_bytes()).expect("write request");
                    stream.read_to_end(&mut response).expect("read response");
                    break;
                }
                Err(_) => thread::sleep(std::time::Duration::from_millis(20)),
            }
        }
        response
    };
    if scenario == 0 {
        let disabled = send();
        stop_static_http_server(&mut server.0);
        assert!(
            String::from_utf8_lossy(&disabled).starts_with("HTTP/1.1 404 Not Found"),
            "test login must stay unavailable until explicitly enabled"
        );
        let _ = fs::remove_dir_all(temp_dir);
        return;
    }
    let enabled = send();
    stop_static_http_server(&mut server.0);
    let enabled_text = String::from_utf8_lossy(&enabled);
    assert!(
        enabled_text.starts_with("HTTP/1.1 200 OK"),
        "{enabled_text}"
    );
    assert!(
        enabled_text.contains("\"player_id\":\"hosted-player-"),
        "{enabled_text}"
    );
    assert!(
        enabled_text.contains("\"registration_grant\":"),
        "{enabled_text}"
    );
    assert!(
        !enabled_text.contains("private"),
        "issuer private material must not cross the endpoint"
    );
    let (_, response_body) = enabled_text.split_once("\r\n\r\n").expect("HTTP body");
    let response: serde_json::Value = serde_json::from_str(response_body).expect("login JSON");
    let grant = &response["grant"];
    let token = grant["registration_grant"]
        .as_str()
        .expect("signed registration grant");
    let parts: Vec<_> = token.split('.').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], "v1");
    let payload = hex::decode(parts[1]).expect("registration payload");
    let signature = ed25519_dalek::Signature::from_slice(
        &hex::decode(parts[2]).expect("registration signature bytes"),
    )
    .expect("registration signature");
    ed25519_dalek::SigningKey::from_bytes(&[71; 32])
        .verifying_key()
        .verify_strict(&payload, &signature)
        .expect("real login issuer signature");
    let payload: serde_json::Value = serde_json::from_slice(&payload).expect("grant payload JSON");
    assert_eq!(payload["player_id"], grant["player_id"]);
    assert_eq!(payload["public_key"], hex::encode([72; 32]));
    assert_eq!(payload["device_session_id"], grant["device_session_id"]);

    let wildcard_probe = TcpListener::bind(("127.0.0.1", 0)).expect("bind wildcard port probe");
    let wildcard_port = wildcard_probe
        .local_addr()
        .expect("wildcard probe addr")
        .port();
    drop(wildcard_probe);
    let mut wildcard_server = start_static_http_server(
        DeploymentMode::HostedPublicJoin,
        "127.0.0.1:0",
        "127.0.0.1:5011",
        "0.0.0.0",
        wildcard_port,
        temp_dir.as_path(),
        None,
    )
    .map(HostedTestHttpServer)
    .expect("start wildcard static HTTP server");
    let mut wildcard_response = Vec::new();
    for _ in 0..50 {
        match TcpStream::connect(("127.0.0.1", wildcard_port)) {
            Ok(mut stream) => {
                stream
                    .write_all(request.as_bytes())
                    .expect("write wildcard request");
                stream
                    .read_to_end(&mut wildcard_response)
                    .expect("read wildcard response");
                break;
            }
            Err(_) => thread::sleep(std::time::Duration::from_millis(20)),
        }
    }
    stop_static_http_server(&mut wildcard_server.0);
    assert!(
        String::from_utf8_lossy(&wildcard_response).starts_with("HTTP/1.1 404 Not Found"),
        "wildcard viewer HTTP bind must keep test login unavailable"
    );
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn director_capability_endpoint_reports_explicit_unavailable_state() {
    let temp_dir = make_temp_dir("director_capability_unavailable");
    fs::write(temp_dir.join("index.html"), b"ok").expect("write index");
    let probe = TcpListener::bind(("127.0.0.1", 0)).expect("bind port probe");
    let port = probe.local_addr().expect("probe addr").port();
    drop(probe);
    let mut server = start_static_http_server(
        DeploymentMode::HostedPublicJoin,
        "127.0.0.1:0",
        "127.0.0.1:5011",
        "127.0.0.1",
        port,
        temp_dir.as_path(),
        None,
    )
    .expect("start static HTTP server");

    let mut response = Vec::new();
    for _ in 0..50 {
        match TcpStream::connect(("127.0.0.1", port)) {
            Ok(mut stream) => {
                stream
                    .write_all(
                        b"GET /api/public/director/capability HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
                    )
                    .expect("write request");
                stream.read_to_end(&mut response).expect("read response");
                break;
            }
            Err(_) => thread::sleep(std::time::Duration::from_millis(20)),
        }
    }
    stop_static_http_server(&mut server);

    let body = String::from_utf8_lossy(&response);
    assert!(body.starts_with("HTTP/1.1 200 OK"), "{body}");
    assert!(body.contains("director_capability_unavailable"), "{body}");
    assert!(body.contains("\"availability\":\"unavailable\""), "{body}");
    assert!(
        !body.contains("\"grant\":"),
        "unavailable endpoint must not issue a grant: {body}"
    );
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn static_http_index_trusts_web_bridge_not_raw_runtime_endpoint() {
    let temp_dir = make_temp_dir("distinct_runtime_endpoints");
    fs::write(
        temp_dir.join("index.html"),
        b"<html><head></head><body>viewer</body></html>",
    )
    .expect("write index");
    let probe = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);
    let mut server = start_static_http_server(
        DeploymentMode::TrustedLocalOnly,
        "127.0.0.1:49423",
        "127.0.0.1:49411",
        "127.0.0.1",
        port,
        temp_dir.as_path(),
        None,
    )
    .expect("start server with distinct raw and websocket endpoints");
    let mut response = Vec::new();
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .write_all(b"GET / HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .unwrap();
    stream.read_to_end(&mut response).unwrap();
    stop_static_http_server(&mut server);
    let response = String::from_utf8(response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK"));
    assert!(response.contains(r#""viewerWsEndpoint":"ws://127.0.0.1:49411/""#));
    assert!(!response.contains("ws://127.0.0.1:49423/"));
    let config = static_http::StaticHttpRuntimeConfig::new(
        DeploymentMode::TrustedLocalOnly,
        "127.0.0.1:49423",
        "127.0.0.1:49411",
    )
    .unwrap();
    assert_eq!(
        config.live_bind, "127.0.0.1:49423",
        "session and presence endpoint is unchanged"
    );
}
