use super::*;
use serde_json::{Value, json};
use std::{io::Write, net::TcpListener, thread};

#[derive(Clone, Copy, Debug)]
enum Reply {
    Trusted,
    Unsigned,
    WrongSigner,
    WrongPath,
    WrongDigest,
    Tampered,
}

fn service(reply: Reply, message: String) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let task = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
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
        let mut bytes = vec![0; length];
        stream.read_exact(&mut bytes).unwrap();
        let request: SignedReadRequest<Value> = serde_json::from_slice(&bytes).unwrap();
        authority::verify_read_request(LOOKUP_PATH, &request).unwrap();
        let digest = if matches!(reply, Reply::WrongDigest) {
            "wrong-request-digest".into()
        } else {
            authority::request_digest(LOOKUP_PATH, &request).unwrap()
        };
        let path = if matches!(reply, Reply::WrongPath) {
            VIEW_PATH
        } else {
            LOOKUP_PATH
        };
        let key = if matches!(reply, Reply::WrongSigner) {
            [8; 32]
        } else {
            [9; 32]
        };
        let payload = json!({"message": message});
        let body = if matches!(reply, Reply::Unsigned) {
            serde_json::to_vec(&payload).unwrap()
        } else {
            let mut signed =
                authority::sign_service_response(path, digest, payload, &hex::encode(key)).unwrap();
            if matches!(reply, Reply::Tampered) {
                signed.payload["message"] = json!("injected diagnostic");
            }
            serde_json::to_vec(&signed).unwrap()
        };
        write!(
            stream,
            "HTTP/1.1 503 Service Unavailable\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        )
        .unwrap();
        // The confidential transport deliberately need not consume the body.
        let _ = stream.write_all(&body);
    });
    (endpoint, task)
}

fn transport(endpoint: String) -> WorldServiceTransport {
    let mut config = tests::config(endpoint);
    config.timeout = Duration::from_secs(5);
    config.max_response_bytes = 16 * 1024;
    WorldServiceTransport::new(config).unwrap()
}

fn read_request(stream: &mut std::net::TcpStream) {
    let mut request = Vec::new();
    loop {
        let mut chunk = [0; 1024];
        let count = stream.read(&mut chunk).unwrap();
        assert!(count > 0, "client closed before sending a complete request");
        request.extend_from_slice(&chunk[..count]);
        if let Some(header_end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().unwrap())
                })
                .unwrap();
            if request.len() >= header_end + 4 + length {
                return;
            }
        }
    }
}

#[test]
fn private_send_errors_preserve_timeout_classification_and_keep_peer_close_non_timeout() {
    fn short_transport(listener: &TcpListener) -> WorldServiceTransport {
        let mut config = tests::config(format!("http://{}", listener.local_addr().unwrap()));
        config.timeout = Duration::from_millis(100);
        WorldServiceTransport::new(config).unwrap()
    }

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = short_transport(&listener);
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        read_request(&mut stream);
        thread::sleep(Duration::from_millis(250));
    });
    let timeout = client
        .private_response_bytes(LOOKUP_PATH, &json!({"request":"original"}), 16 * 1024)
        .unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        timeout,
        WorldServiceClientError::Transport(ref reason) if reason == "private transport timed out"
    ));

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let client = short_transport(&listener);
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        read_request(&mut stream);
    });
    let closed = client
        .private_response_bytes(LOOKUP_PATH, &json!({"request":"original"}), 16 * 1024)
        .unwrap_err();
    server.join().unwrap();
    assert!(matches!(
        closed,
        WorldServiceClientError::Transport(ref reason)
            if reason.starts_with("private transport ")
                && reason != "private transport timed out"
    ));
}

#[test]
fn shared_signed_rejection_preserves_verified_reason_and_bounds_unicode_characters() {
    for message in [
        "native business CAS read failed".to_owned(),
        "原因".repeat(700),
    ] {
        let (endpoint, task) = service(Reply::Trusted, message.clone());
        let error = transport(endpoint)
            .signed_call::<_, Value>(LOOKUP_PATH, json!({"request": "original"}))
            .unwrap_err();
        task.join().unwrap();
        assert!(
            matches!(error, WorldServiceClientError::Rejected { status: 503, message: observed }
            if observed == message.chars().take(1024).collect::<String>())
        );
    }
}

#[test]
fn shared_signed_rejection_keeps_untrusted_or_misbound_reasons_generic() {
    for reply in [
        Reply::Unsigned,
        Reply::WrongSigner,
        Reply::WrongPath,
        Reply::WrongDigest,
        Reply::Tampered,
    ] {
        let (endpoint, task) = service(reply, "untrusted diagnostic".into());
        let error = transport(endpoint)
            .signed_call::<_, Value>(LOOKUP_PATH, json!({"request": "original"}))
            .unwrap_err();
        task.join().unwrap();
        assert!(
            matches!(error, WorldServiceClientError::Rejected { status: 503, message }
            if message == "service rejected request"),
            "reply={reply:?}"
        );
    }
}

#[test]
fn shared_signed_rejection_does_not_expose_confidential_response_reasons() {
    let (endpoint, task) = service(Reply::Trusted, "private service detail".into());
    let client = transport(endpoint);
    let request = authority::sign_read_request(
        LOOKUP_PATH,
        json!({"request": "original"}),
        &hex::encode([7; 32]),
    )
    .unwrap();
    let error = client
        .private_response_bytes(LOOKUP_PATH, &request, 16 * 1024)
        .unwrap_err();
    task.join().unwrap();
    assert!(
        matches!(error, WorldServiceClientError::Rejected { status: 503, message }
        if message == "private goal request rejected")
    );
}
