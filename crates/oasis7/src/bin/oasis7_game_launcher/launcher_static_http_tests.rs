use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::Mutex;
use std::thread;

static HOSTED_TEST_LOGIN_ENV_LOCK: Mutex<()> = Mutex::new(());

use super::{
    DeploymentMode, make_temp_dir, sanitize_index_html_for_embedded_server,
    start_static_http_server, stop_static_http_server,
};
use crate::static_http;

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
    let _guard = HOSTED_TEST_LOGIN_ENV_LOCK
        .lock()
        .expect("test login env lock");
    unsafe {
        std::env::remove_var("OASIS7_HOSTED_TEST_LOGIN_ENABLED");
        std::env::set_var(
            oasis7::viewer::HOSTED_REGISTRATION_ISSUER_PRIVATE_KEY_ENV,
            hex::encode([71_u8; 32]),
        );
    }
    let temp_dir = make_temp_dir("hosted_test_login");
    fs::write(temp_dir.join("index.html"), b"ok").expect("write index");
    let probe = TcpListener::bind(("127.0.0.1", 0)).expect("bind port probe");
    let port = probe.local_addr().expect("probe addr").port();
    drop(probe);
    let mut server = start_static_http_server(
        DeploymentMode::HostedPublicJoin,
        "127.0.0.1:0",
        "127.0.0.1",
        port,
        temp_dir.as_path(),
        None,
    )
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
    let disabled = send();
    assert!(
        String::from_utf8_lossy(&disabled).starts_with("HTTP/1.1 404 Not Found"),
        "test login must stay unavailable until explicitly enabled"
    );
    unsafe { std::env::set_var("OASIS7_HOSTED_TEST_LOGIN_ENABLED", "1") };
    let enabled = send();
    stop_static_http_server(&mut server);
    unsafe {
        std::env::remove_var("OASIS7_HOSTED_TEST_LOGIN_ENABLED");
        std::env::remove_var(oasis7::viewer::HOSTED_REGISTRATION_ISSUER_PRIVATE_KEY_ENV);
    }
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

    let wildcard_probe = TcpListener::bind(("127.0.0.1", 0)).expect("bind wildcard port probe");
    let wildcard_port = wildcard_probe
        .local_addr()
        .expect("wildcard probe addr")
        .port();
    drop(wildcard_probe);
    let mut wildcard_server = start_static_http_server(
        DeploymentMode::HostedPublicJoin,
        "127.0.0.1:0",
        "0.0.0.0",
        wildcard_port,
        temp_dir.as_path(),
        None,
    )
    .expect("start wildcard static HTTP server");
    unsafe { std::env::set_var("OASIS7_HOSTED_TEST_LOGIN_ENABLED", "1") };
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
    stop_static_http_server(&mut wildcard_server);
    assert!(
        String::from_utf8_lossy(&wildcard_response).starts_with("HTTP/1.1 404 Not Found"),
        "wildcard viewer HTTP bind must keep test login unavailable"
    );
    unsafe { std::env::remove_var("OASIS7_HOSTED_TEST_LOGIN_ENABLED") };
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
