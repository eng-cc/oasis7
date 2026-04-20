use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;

#[test]
fn probe_chain_status_endpoint_accepts_http_200_response() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let bind = listener.local_addr().expect("listener addr");
    let serve = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept probe connection");
        let mut request = [0_u8; 512];
        let _ = stream.read(&mut request);
        let _ = stream.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\nConnection: close\r\n\r\n{\"ok\":true}",
        );
    });

    probe_chain_status_endpoint(bind.to_string().as_str()).expect("probe should pass");
    serve.join().expect("server thread should finish");
}

#[test]
fn probe_chain_status_endpoint_reports_connect_failure() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind temp listener");
    let bind = listener.local_addr().expect("listener addr").to_string();
    drop(listener);

    let err = probe_chain_status_endpoint(bind.as_str()).expect_err("probe should fail");
    assert!(err.contains("connect chain status server failed"));
}

#[test]
fn check_provider_loopback_http_provider_accepts_info_and_health_responses() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let bind = listener.local_addr().expect("listener addr");
    let serve = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept probe connection");
            let mut request = [0_u8; 1024];
            let bytes = stream.read(&mut request).expect("read request");
            let request_text = String::from_utf8_lossy(&request[..bytes]);
            let body = if request_text.contains("GET /v1/provider/info") {
                r#"{"provider_id":"provider-local","name":"Local Provider","version":"0.1.0","protocol_version":"v1","capabilities":["decision","feedback"],"supported_action_sets":["phase1_low_frequency"]}"#
            } else {
                r#"{"ok":true,"status":"ready","uptime_ms":42,"last_error":null,"queue_depth":0}"#
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });

    let snapshot =
        check_provider_loopback_http_provider(format!("http://{}", bind).as_str(), None, 200)
            .expect("provider check should pass");
    assert_eq!(snapshot.provider_id, "provider-local");
    assert_eq!(snapshot.name, "Local Provider");
    assert_eq!(snapshot.version, "0.1.0");
    assert_eq!(snapshot.protocol_version, "v1");
    assert_eq!(
        snapshot.compatibility_status,
        ProviderCompatibilityStatus::Ready
    );
    assert_eq!(
        snapshot.capabilities,
        vec!["decision".to_string(), "feedback".to_string()]
    );
    assert_eq!(
        snapshot.supported_action_sets,
        vec!["phase1_low_frequency".to_string()]
    );
    assert_eq!(snapshot.status, "ready");
    assert_eq!(snapshot.queue_depth, Some(0));
    assert_eq!(snapshot.last_error, None);
    assert_eq!(snapshot.fallback_reason, None);
    assert!(snapshot.info_latency_ms <= snapshot.total_latency_ms);
    assert!(snapshot.health_latency_ms <= snapshot.total_latency_ms);
    serve.join().expect("server thread should finish");
}

#[test]
fn check_provider_loopback_http_provider_reports_incompatible_supported_actions() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let bind = listener.local_addr().expect("listener addr");
    let serve = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept probe connection");
            let mut request = [0_u8; 1024];
            let bytes = stream.read(&mut request).expect("read request");
            let request_text = String::from_utf8_lossy(&request[..bytes]);
            let body = if request_text.contains("GET /v1/provider/info") {
                r#"{"provider_id":"provider-local","name":"Local Provider","version":"0.1.0","protocol_version":"v1","capabilities":["decision","feedback"],"supported_action_sets":["wait","move_agent"]}"#
            } else {
                r#"{"ok":true,"status":"ready","uptime_ms":42,"last_error":null,"queue_depth":0}"#
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });

    let snapshot =
        check_provider_loopback_http_provider(format!("http://{}", bind).as_str(), None, 200)
            .expect("provider check should still return snapshot");
    assert_eq!(
        snapshot.compatibility_status,
        ProviderCompatibilityStatus::Incompatible
    );
    assert_eq!(
        snapshot.fallback_reason.as_deref(),
        Some("missing_supported_actions:wait_ticks,speak_to_nearby,inspect_target,simple_interact")
    );
    serve.join().expect("server thread should finish");
}

#[test]
fn check_provider_loopback_http_provider_marks_unhealthy_provider_as_degraded() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let bind = listener.local_addr().expect("listener addr");
    let serve = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept probe connection");
            let mut request = [0_u8; 1024];
            let bytes = stream.read(&mut request).expect("read request");
            let request_text = String::from_utf8_lossy(&request[..bytes]);
            let body = if request_text.contains("GET /v1/provider/info") {
                r#"{"provider_id":"provider-local","name":"Local Provider","version":"0.1.0","protocol_version":"v1","capabilities":["decision","feedback"],"supported_action_sets":["phase1_low_frequency"]}"#
            } else {
                r#"{"ok":false,"status":null,"uptime_ms":42,"last_error":null,"queue_depth":0}"#
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });

    let snapshot =
        check_provider_loopback_http_provider(format!("http://{}", bind).as_str(), None, 200)
            .expect("provider check should still return degraded snapshot");
    assert_eq!(
        snapshot.compatibility_status,
        ProviderCompatibilityStatus::Degraded
    );
    assert_eq!(
        snapshot.fallback_reason.as_deref(),
        Some("provider_health_unhealthy:not_ok")
    );
    serve.join().expect("server thread should finish");
}
