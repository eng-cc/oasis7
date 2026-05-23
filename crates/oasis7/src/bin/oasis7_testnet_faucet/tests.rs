use super::*;

fn test_service() -> FaucetService {
    FaucetService {
        options: ServeOptions {
            listen: "127.0.0.1:0".to_string(),
            upstream: "http://127.0.0.1:65535".to_string(),
            faucet_public_key: "11".repeat(32),
            faucet_private_key: "22".repeat(32),
            amount: 100,
            cooldown_secs: 60,
            request_timeout_secs: 1,
        },
        faucet_account_id: format!("oc:pk:{}", "aa".repeat(32)),
        client: build_http_client(1).expect("build test client"),
        state: Arc::new(Mutex::new(FaucetState::default())),
    }
}

fn tcp_stream_pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
    let bind = listener.local_addr().expect("read local addr");
    let client = TcpStream::connect(bind).expect("connect loopback client");
    let (server, _) = listener.accept().expect("accept loopback connection");
    (server, client)
}

fn decode_http_response_bytes(bytes: &[u8]) -> (u16, serde_json::Value) {
    let boundary = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("response must include http body separator");
    let header = std::str::from_utf8(&bytes[..boundary]).expect("response header utf-8");
    let status = header
        .split_whitespace()
        .nth(1)
        .and_then(|token| token.parse::<u16>().ok())
        .expect("response status code");
    let payload = serde_json::from_slice::<serde_json::Value>(&bytes[(boundary + 4)..])
        .expect("response json payload");
    (status, payload)
}

#[test]
fn claim_status_code_maps_structured_errors() {
    let mut response = FaucetClaimResponse {
        ok: false,
        faucet_account_id: "oc:pk:test".to_string(),
        amount: 100,
        cooldown_secs: 60,
        action_id: None,
        submitted_at_unix_ms: None,
        error_code: Some("cooldown_active".to_string()),
        error: Some("cooldown".to_string()),
    };
    assert_eq!(faucet_claim_status_code(&response), 429);
    response.error_code = Some("bad_request".to_string());
    assert_eq!(faucet_claim_status_code(&response), 400);
    response.error_code = Some("insufficient_balance".to_string());
    assert_eq!(faucet_claim_status_code(&response), 503);
    response.error_code = Some("upstream_unavailable".to_string());
    assert_eq!(faucet_claim_status_code(&response), 502);
}

#[test]
fn prune_tracker_map_drops_expired_and_caps_size() {
    let mut map = HashMap::new();
    map.insert("expired".to_string(), 10);
    for index in 0..(MAX_TRACKED_FAUCET_CLAIMANTS + 8) {
        map.insert(
            format!("fresh-{index}"),
            1_000 + i64::try_from(index).unwrap_or(i64::MAX),
        );
    }
    prune_tracker_map(&mut map, 100);
    assert!(!map.contains_key("expired"));
    assert!(map.len() <= MAX_TRACKED_FAUCET_CLAIMANTS);
}

#[test]
fn post_claim_invalid_account_returns_http_400_json() {
    let service = test_service();
    let (server_stream, mut client_stream) = tcp_stream_pair();
    let body = serde_json::json!({
        "account_id": "not-an-oc-account"
    })
    .to_string();
    let request = format!(
        "POST /claim HTTP/1.1\r\nHost: 127.0.0.1:6681\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    client_stream
        .write_all(request.as_bytes())
        .expect("write request");
    service
        .handle_connection(server_stream, "127.0.0.1".to_string())
        .expect("handle invalid claim request");
    client_stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set client timeout");
    let mut response_bytes = Vec::new();
    client_stream
        .read_to_end(&mut response_bytes)
        .expect("read faucet response");
    let (status, response) = decode_http_response_bytes(&response_bytes);
    assert_eq!(status, 400);
    assert_eq!(response["ok"], serde_json::json!(false));
    assert_eq!(response["error_code"], serde_json::json!("bad_request"));
}
