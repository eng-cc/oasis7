use super::*;
use serde_json::Value;
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

#[test]
fn environment_timeout_is_honored_and_invalid_values_fail_closed() {
    const CHILD: &str = "OASIS7_TEST_SERVICE_TIMEOUT_CHILD";
    if let Ok(expected) = std::env::var(CHILD) {
        let result = WorldServiceClientConfig::from_env();
        if expected == "invalid" {
            assert!(result.is_err(), "invalid timeout must reject configuration");
        } else {
            assert_eq!(
                result.unwrap().unwrap().timeout,
                Duration::from_millis(expected.parse().unwrap())
            );
        }
        return;
    }
    for (value, expected) in [
        (None, "1500"),
        (Some("10000"), "10000"),
        (Some("100"), "100"),
        (Some("30000"), "30000"),
        (Some("0"), "invalid"),
        (Some("99"), "invalid"),
        (Some("30001"), "invalid"),
        (Some("invalid"), "invalid"),
        (Some("18446744073709551616"), "invalid"),
    ] {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap());
        child
            .args([
                "--exact",
                "tests::environment_timeout_is_honored_and_invalid_values_fail_closed",
                "--nocapture",
            ])
            .env(CHILD, expected)
            .env("OASIS7_WORLD_SERVICE_ENDPOINT", "http://127.0.0.1:1")
            .env("OASIS7_WORLD_SERVICE_PUBLIC_KEY", "09".repeat(32))
            .env("OASIS7_WORLD_SERVICE_WORLD_ID", "timeout-config-test")
            .env("OASIS7_WORLD_SERVICE_GENESIS_DIGEST", "test-genesis")
            .env("OASIS7_WORLD_SERVICE_SCOPE", "public")
            .env("OASIS7_WORLD_SERVICE_READ_PRIVATE_KEY", "07".repeat(32));
        match value {
            Some(value) => child.env("OASIS7_WORLD_SERVICE_TIMEOUT_MS", value),
            None => child.env_remove("OASIS7_WORLD_SERVICE_TIMEOUT_MS"),
        };
        let output = child.output().unwrap();
        assert!(
            output.status.success(),
            "timeout {value:?}: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
pub(super) fn config(endpoint: String) -> WorldServiceClientConfig {
    let key = ed25519_dalek::SigningKey::from_bytes(&[9; 32]);
    WorldServiceClientConfig {
        endpoint,
        trusted_service_public_key: hex::encode(key.verifying_key().to_bytes()),
        expected_world: WorldIdentity {
            world_id: "world".into(),
            genesis_digest: "genesis".into(),
        },
        scope_id: "agent:a".into(),
        read_private_key_hex: hex::encode([7; 32]),
        timeout: Duration::from_millis(500),
        max_response_bytes: 4096,
    }
}
fn serve(
    transform: impl FnOnce(&mut SignedServiceResponse<Value>) + Send + 'static,
) -> (String, thread::JoinHandle<()>) {
    serve_payload(VIEW_PATH, serde_json::json!({"ok":true}), transform)
}
pub(super) fn serve_payload(
    path: &'static str,
    payload: Value,
    transform: impl FnOnce(&mut SignedServiceResponse<Value>) + Send + 'static,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let task = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut request = Vec::new();
        let (head_end, length) = loop {
            let mut buf = [0; 1024];
            let n = stream.read(&mut buf).unwrap();
            assert!(n > 0);
            request.extend_from_slice(&buf[..n]);
            if let Some(at) = request.windows(4).position(|v| v == b"\r\n\r\n") {
                let head = String::from_utf8_lossy(&request[..at]).to_lowercase();
                let length: usize = head
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .unwrap()
                    .trim()
                    .parse()
                    .unwrap();
                if request.len() >= at + 4 + length {
                    break (at + 4, length);
                }
            }
        };
        let signed: SignedReadRequest<Value> =
            serde_json::from_slice(&request[head_end..head_end + length]).unwrap();
        authority::verify_read_request(path, &signed).unwrap();
        let digest = authority::request_digest(path, &signed).unwrap();
        let mut response =
            authority::sign_service_response(path, digest, payload, &hex::encode([9; 32])).unwrap();
        transform(&mut response);
        let body = serde_json::to_vec(&response).unwrap();
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        stream.write_all(&body).unwrap();
    });
    (endpoint, task)
}
#[test]
fn response_is_bound_to_trusted_key_request_and_operation() {
    let (endpoint, task) = serve(|_| {});
    let client = WorldServiceTransport::new(config(endpoint)).unwrap();
    let result: Value = client
        .signed_call(VIEW_PATH, serde_json::json!({"scope":"agent:a"}))
        .unwrap();
    assert_eq!(result["ok"], true);
    task.join().unwrap();
    let (endpoint, task) = serve(|response| response.payload = serde_json::json!({"ok":false}));
    let client = WorldServiceTransport::new(config(endpoint)).unwrap();
    assert!(matches!(
        client.signed_call::<_, Value>(VIEW_PATH, serde_json::json!({"scope":"agent:a"})),
        Err(WorldServiceClientError::Assurance(_))
    ));
    task.join().unwrap();
}

#[test]
fn cooldown_advice_is_bounded_and_shared_only_by_explicit_clone() {
    let state = WorldServiceQueryState::default();
    assert_eq!(state.observe(503, None).unwrap(), None);
    assert!(state.check().is_ok());
    assert_eq!(
        state.observe(429, Some("999999")).unwrap(),
        Some(Duration::from_secs(2))
    );
    assert!(matches!(
        state.clone().check(),
        Err(WorldServiceClientError::Cooldown { status: 429, .. })
    ));
    assert!(WorldServiceQueryState::default().check().is_ok());
    state.expire_for_test();
    assert!(state.check().is_ok());
    assert_eq!(
        state.observe(503, Some("invalid")).unwrap(),
        Some(Duration::from_secs(1))
    );
}

fn raw_response(status: &str, headers: &str, body: Vec<u8>) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let status = status.to_owned();
    let headers = headers.to_owned();
    let task = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut head = Vec::new();
        while !head.ends_with(b"\r\n\r\n") {
            let mut byte = [0];
            stream.read_exact(&mut byte).unwrap();
            head.push(byte[0]);
        }
        let head = String::from_utf8(head).unwrap();
        let length: usize = head
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse().unwrap())
            })
            .unwrap();
        stream.read_exact(&mut vec![0; length]).unwrap();
        write!(
            stream,
            "HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        let _ = stream.write_all(&body);
    });
    (endpoint, task)
}

#[test]
fn public_and_confidential_transport_bound_responses_and_never_follow_redirects() {
    for private in [false, true] {
        let (endpoint, task) = raw_response("200 OK", "", vec![b'x'; 129]);
        let mut cfg = config(endpoint);
        cfg.max_response_bytes = 128;
        let client = WorldServiceTransport::new(cfg).unwrap();
        let error = if private {
            client
                .private_response_bytes(VIEW_PATH, &(), 65536)
                .unwrap_err()
        } else {
            client.call::<_, Value>(VIEW_PATH, &()).unwrap_err()
        };
        assert!(
            matches!(error, WorldServiceClientError::Assurance(reason) if reason.contains("exceeds bound"))
        );
        task.join().unwrap();
        let destination = TcpListener::bind("127.0.0.1:0").unwrap();
        destination.set_nonblocking(true).unwrap();
        let (endpoint, task) = raw_response(
            "307 Temporary Redirect",
            &format!(
                "Location: http://{}/secret\r\n",
                destination.local_addr().unwrap()
            ),
            Vec::new(),
        );
        let client = WorldServiceTransport::new(config(endpoint)).unwrap();
        let error = if private {
            client
                .private_response_bytes(VIEW_PATH, &(), 65536)
                .unwrap_err()
        } else {
            client.call::<_, Value>(VIEW_PATH, &()).unwrap_err()
        };
        assert!(matches!(
            error,
            WorldServiceClientError::Rejected { status: 307, .. }
        ));
        assert_eq!(
            destination.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        task.join().unwrap();
    }
}

#[test]
fn unbounded_public_configuration_does_not_overflow_response_read() {
    let (endpoint, task) = serve(|_| {});
    let mut cfg = config(endpoint);
    cfg.max_response_bytes = usize::MAX;
    let client = WorldServiceTransport::new(cfg).unwrap();
    assert_eq!(
        client.signed_call::<_, Value>(VIEW_PATH, ()).unwrap()["ok"],
        true
    );
    task.join().unwrap();
}

#[test]
fn response_rejects_wrong_operation_request_or_trusted_service() {
    for field in ["path", "request", "service"] {
        let (endpoint, task) = serve(move |response| {
            let payload = response.payload.clone();
            let path = if field == "path" {
                DESCRIBE_PATH
            } else {
                VIEW_PATH
            };
            let digest = if field == "request" {
                "other-request".to_owned()
            } else {
                response.request_digest.clone()
            };
            let key = if field == "service" { [8; 32] } else { [9; 32] };
            *response =
                authority::sign_service_response(path, digest, payload, &hex::encode(key)).unwrap();
        });
        let client = WorldServiceTransport::new(config(endpoint)).unwrap();
        assert!(matches!(
            client.signed_call::<_, Value>(VIEW_PATH, ()),
            Err(WorldServiceClientError::Assurance(_))
        ));
        task.join().unwrap();
    }
}

#[test]
fn confidential_decoder_rejects_duplicate_keys_and_ignored_envelope_fields() {
    for body in [
        br#"{"request_digest":"a","request_digest":"b","payload":{},"service_public_key":"key","signature_hex":"sig"}"#.to_vec(),
        br#"{"request_digest":"a","payload":{},"service_public_key":"key","signature_hex":"sig","unsupported":"field"}"#.to_vec(),
    ] {
        let (endpoint, task) = raw_response("200 OK", "", body);
        let client = WorldServiceTransport::new(config(endpoint)).unwrap();
        assert!(matches!(client.private_signed_response::<_, Value>(VIEW_PATH, &(), 65536), Err(WorldServiceClientError::Assurance(_))));
        task.join().unwrap();
    }
}
