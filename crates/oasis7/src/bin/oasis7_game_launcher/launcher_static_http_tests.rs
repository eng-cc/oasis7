use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::thread;

use super::{
    DeploymentMode, make_temp_dir, sanitize_index_html_for_embedded_server,
    start_static_http_server, stop_static_http_server,
};

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
