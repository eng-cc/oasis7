use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;

#[test]
fn prepare_provider_info_captures_provider_compatibility_status() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let bind = listener.local_addr().expect("listener addr");
    let serve = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept probe connection");
            let mut request = [0_u8; 1024];
            let bytes = stream.read(&mut request).expect("read request");
            let request_text = String::from_utf8_lossy(&request[..bytes]);
            let body = if request_text.contains("GET /v1/provider/info") {
                r#"{"provider_id":"provider_local_bridge","name":"Provider Local Bridge","version":"0.1.0","protocol_version":"world-simulator-provider-loopback-http-v1","chain_resource_manifest_schema_version":"oasis7.world_resource_manifest.v1","chain_resource_delta_schema_version":"oasis7.world_resource_delta.v1","capabilities":["decision","feedback"],"supported_action_sets":["wait","wait_ticks","move_agent","speak_to_nearby","inspect_target","simple_interact"]}"#
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

    let options = CliOptions {
        provider: BenchProviderKind::ProviderLoopbackHttp,
        provider_base_url: Some(format!("http://{bind}")),
        ..CliOptions::default()
    };
    let provider = prepare_provider_info(&options).expect("prepare provider");
    assert_eq!(provider.compatibility_status, "ready");
    assert_eq!(provider.fallback_reason, None);
    assert_eq!(
        provider.capabilities,
        vec!["decision".to_string(), "feedback".to_string()]
    );
    assert_eq!(provider.supported_action_sets.len(), 6);
    serve.join().expect("server thread should finish");
}

#[test]
fn prepare_provider_info_marks_incompatible_supported_actions() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let bind = listener.local_addr().expect("listener addr");
    let serve = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept probe connection");
            let mut request = [0_u8; 1024];
            let bytes = stream.read(&mut request).expect("read request");
            let request_text = String::from_utf8_lossy(&request[..bytes]);
            let body = if request_text.contains("GET /v1/provider/info") {
                r#"{"provider_id":"provider_local_bridge","name":"Provider Local Bridge","version":"0.1.0","protocol_version":"world-simulator-provider-loopback-http-v1","chain_resource_manifest_schema_version":"oasis7.world_resource_manifest.v1","chain_resource_delta_schema_version":"oasis7.world_resource_delta.v1","capabilities":["decision","feedback"],"supported_action_sets":["wait","move_agent"]}"#
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

    let options = CliOptions {
        provider: BenchProviderKind::ProviderLoopbackHttp,
        provider_base_url: Some(format!("http://{bind}")),
        ..CliOptions::default()
    };
    let provider = prepare_provider_info(&options).expect("prepare provider");
    assert_eq!(provider.compatibility_status, "incompatible");
    assert_eq!(
        provider.fallback_reason.as_deref(),
        Some("missing_supported_actions:wait_ticks,speak_to_nearby,inspect_target,simple_interact")
    );
    serve.join().expect("server thread should finish");
}

#[test]
fn prepare_provider_info_marks_missing_capabilities_as_incompatible() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let bind = listener.local_addr().expect("listener addr");
    let serve = std::thread::spawn(move || {
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept probe connection");
            let mut request = [0_u8; 1024];
            let bytes = stream.read(&mut request).expect("read request");
            let request_text = String::from_utf8_lossy(&request[..bytes]);
            let body = if request_text.contains("GET /v1/provider/info") {
                r#"{"provider_id":"provider_local_bridge","name":"Provider Local Bridge","version":"0.1.0","protocol_version":"world-simulator-provider-loopback-http-v1","chain_resource_manifest_schema_version":"oasis7.world_resource_manifest.v1","chain_resource_delta_schema_version":"oasis7.world_resource_delta.v1","capabilities":["decision"],"supported_action_sets":["phase1_low_frequency"]}"#
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

    let options = CliOptions {
        provider: BenchProviderKind::ProviderLoopbackHttp,
        provider_base_url: Some(format!("http://{bind}")),
        ..CliOptions::default()
    };
    let provider = prepare_provider_info(&options).expect("prepare provider");
    assert_eq!(provider.compatibility_status, "incompatible");
    assert_eq!(
        provider.fallback_reason.as_deref(),
        Some("missing_provider_capabilities:feedback")
    );
    serve.join().expect("server thread should finish");
}
