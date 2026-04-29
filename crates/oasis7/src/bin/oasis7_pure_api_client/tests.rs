use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use super::*;

fn fixed_private_key_hex(seed: u8) -> String {
    hex::encode([seed; 32])
}

#[test]
fn derive_public_key_hex_matches_signing_key() {
    let private_key_hex = fixed_private_key_hex(7);
    let derived = derive_public_key_hex(private_key_hex.as_str()).expect("derive public key");
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    assert_eq!(derived, hex::encode(signing_key.verifying_key().to_bytes()));
}

#[test]
fn build_signed_agent_chat_request_attaches_auth_and_intent_seq() {
    let private_key_hex = fixed_private_key_hex(8);
    let request = build_signed_agent_chat_request(
        "agent-0",
        "player-1",
        "hello",
        private_key_hex.as_str(),
        None,
        Some(42),
        Some(9),
    )
    .expect("signed chat request");
    assert_eq!(request.player_id.as_deref(), Some("player-1"));
    assert_eq!(request.intent_tick, Some(42));
    assert_eq!(request.intent_seq, Some(9));
    assert!(request.auth.is_some());
    assert!(request.public_key.is_some());
}

#[test]
fn build_signed_prompt_apply_request_supports_clear_and_set() {
    let private_key_hex = fixed_private_key_hex(9);
    let request = build_signed_prompt_apply_request(
        "agent-0",
        "player-1",
        private_key_hex.as_str(),
        None,
        Some(3),
        Some("tester".to_string()),
        Some(Some("system".to_string())),
        Some(None),
        None,
        false,
    )
    .expect("signed prompt apply request");
    assert_eq!(request.expected_version, Some(3));
    assert_eq!(request.updated_by.as_deref(), Some("tester"));
    assert_eq!(
        request.system_prompt_override,
        Some(Some("system".to_string()))
    );
    assert_eq!(request.short_term_goal_override, Some(None));
    assert!(request.auth.is_some());
}

#[test]
fn build_signed_gameplay_action_request_attaches_auth() {
    let private_key_hex = fixed_private_key_hex(10);
    let request = build_signed_gameplay_action_request(
        "build_factory_smelter_mk1",
        "runtime-agent-0",
        None,
        "player-1",
        private_key_hex.as_str(),
        None,
    )
    .expect("signed gameplay action request");
    assert_eq!(request.action_id, "build_factory_smelter_mk1");
    assert_eq!(request.target_agent_id, "runtime-agent-0");
    assert_eq!(request.player_id, "player-1");
    assert!(request.public_key.is_some());
    assert!(request.auth.is_some());
}

#[test]
fn collect_until_reports_timeout_when_peer_stays_open() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let (request_seen_tx, request_seen_rx) = mpsc::channel();
    let (release_server_tx, release_server_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept client");
        let reader_stream = stream.try_clone().expect("clone reader stream");
        let writer_stream = stream.try_clone().expect("clone writer stream");
        let mut reader = BufReader::new(reader_stream);
        let mut writer = writer_stream;
        let mut line = String::new();
        reader.read_line(&mut line).expect("read hello");
        let hello = ViewerResponse::HelloAck {
            server: "oasis7".to_string(),
            version: VIEWER_PROTOCOL_VERSION,
            world_id: "test-world".to_string(),
            control_profile: oasis7::viewer::ViewerControlProfile::Live,
        };
        writeln!(
            writer,
            "{}",
            serde_json::to_string(&hello).expect("serialize hello")
        )
        .expect("write hello");
        line.clear();
        reader.read_line(&mut line).expect("read request");
        request_seen_tx.send(()).expect("signal request observed");
        release_server_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("wait for timeout assertion to finish");
    });

    let mut conn = ViewerConnection::connect(
        addr.to_string().as_str(),
        "timeout-test-client",
        Duration::from_millis(50),
    )
    .expect("connect viewer");
    conn.send(&ViewerRequest::RequestSnapshot)
        .expect("request snapshot");
    request_seen_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("server observed request");
    let err = conn
        .collect_until(
            Duration::from_millis(50),
            terminal_snapshot,
            "waiting for snapshot response",
        )
        .expect_err("collect_until should time out");
    assert!(err.contains("timeout after"), "unexpected error: {err}");
    release_server_tx
        .send(())
        .expect("release server after timeout assertion");
    server.join().expect("server join");
}
