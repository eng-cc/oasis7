use super::{
    maybe_handle_gameplay_submit_request, parse_gameplay_submit_request,
    reset_gameplay_submit_state_for_tests, ChainGameplaySubmitResponse,
};
use ed25519_dalek::SigningKey;
use oasis7::viewer::{sign_gameplay_action_auth_proof, GameplayActionRequest};
use oasis7_node::{
    NodeConfig, NodeExecutionCommitContext, NodeExecutionCommitResult, NodeExecutionHook, NodeRole,
    NodeRuntime,
};
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

fn tcp_stream_pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
    let bind = listener.local_addr().expect("read local addr");
    let client = TcpStream::connect(bind).expect("connect loopback client");
    let (server, _) = listener.accept().expect("accept loopback connection");
    (server, client)
}

#[derive(Debug)]
struct NoopExecutionHook;

impl NodeExecutionHook for NoopExecutionHook {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        Ok(NodeExecutionCommitResult {
            execution_height: context.height,
            execution_block_hash: format!("noop-block-{}", context.height),
            execution_state_root: format!("noop-root-{}", context.height),
        })
    }
}

fn decode_http_json_response<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> (u16, T) {
    let boundary = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("response must include HTTP body separator");
    let header = std::str::from_utf8(&bytes[..boundary]).expect("response header utf-8");
    let status = header
        .split_whitespace()
        .nth(1)
        .and_then(|token| token.parse::<u16>().ok())
        .expect("response status code");
    let payload =
        serde_json::from_slice::<T>(&bytes[(boundary + 4)..]).expect("response json payload");
    (status, payload)
}

fn gameplay_test_signer(seed: u8) -> (String, String) {
    let private_key = [seed; 32];
    let signing_key = SigningKey::from_bytes(&private_key);
    (
        hex::encode(signing_key.verifying_key().to_bytes()),
        hex::encode(private_key),
    )
}

fn signed_gameplay_submit_request(player_id: &str, nonce: u64) -> GameplayActionRequest {
    let (public_key, private_key) = gameplay_test_signer(19);
    let mut request = GameplayActionRequest {
        action_id: "build_factory_smelter_mk1".to_string(),
        target_agent_id: "agent-1".to_string(),
        player_id: player_id.to_string(),
        public_key: Some(public_key.clone()),
        auth: None,
    };
    let proof = sign_gameplay_action_auth_proof(&request, nonce, &public_key, &private_key)
        .expect("sign gameplay submit request");
    request.auth = Some(proof);
    request
}

fn wait_for_committed_height(runtime: &Arc<Mutex<NodeRuntime>>, minimum_height: u64) {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        let height = runtime
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .snapshot()
            .consensus
            .committed_height;
        if height >= minimum_height {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let height = runtime
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .snapshot()
        .consensus
        .committed_height;
    panic!("timed out waiting for committed height >= {minimum_height}, got {height}");
}

fn gameplay_submit_test_guard() -> MutexGuard<'static, ()> {
    static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_MUTEX
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[test]
fn parse_gameplay_submit_request_round_trips_viewer_payload() {
    let _guard = gameplay_submit_test_guard();
    reset_gameplay_submit_state_for_tests();
    let request = signed_gameplay_submit_request("node-gameplay-submit-parse", 7);
    let body = serde_json::to_vec(&request).expect("serialize gameplay submit request");
    let parsed = parse_gameplay_submit_request(body.as_slice()).expect("request should parse");
    assert_eq!(parsed.action_id, request.action_id);
    assert_eq!(parsed.target_agent_id, request.target_agent_id);
    assert_eq!(parsed.player_id, request.player_id);
    assert_eq!(parsed.public_key, request.public_key);
    assert_eq!(parsed.auth, request.auth);
}

#[test]
fn gameplay_submit_handler_rejects_missing_auth_proof() {
    let _guard = gameplay_submit_test_guard();
    reset_gameplay_submit_state_for_tests();
    let runtime = Arc::new(Mutex::new(NodeRuntime::new(
        NodeConfig::new(
            "node-gameplay-submit-bad-auth",
            "world-gameplay-submit-bad-auth",
            NodeRole::Sequencer,
        )
        .expect("node config"),
    )));

    let (mut server_stream, mut client_stream) = tcp_stream_pair();
    let body = r#"{
      "action_id":"build_factory_smelter_mk1",
      "target_agent_id":"agent-1",
      "player_id":"node-gameplay-submit-bad-auth",
      "public_key":"deadbeef"
    }"#;
    let request = format!(
        "POST /v1/chain/gameplay/submit HTTP/1.1\r\nHost: 127.0.0.1:5121\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let handled = maybe_handle_gameplay_submit_request(
        &mut server_stream,
        request.as_bytes(),
        &runtime,
        "POST",
        "/v1/chain/gameplay/submit",
    )
    .expect("handler should process request");
    assert!(handled);
    drop(server_stream);

    client_stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set client timeout");
    let mut response_bytes = Vec::new();
    client_stream
        .read_to_end(&mut response_bytes)
        .expect("read handler response");
    let (status, response): (u16, ChainGameplaySubmitResponse) =
        decode_http_json_response(&response_bytes);
    assert_eq!(status, 401);
    assert!(!response.ok);
    assert_eq!(response.error_code.as_deref(), Some("invalid_auth"));
}

#[test]
fn gameplay_submit_handler_accepts_valid_payload_and_commits_to_runtime() {
    let _guard = gameplay_submit_test_guard();
    reset_gameplay_submit_state_for_tests();
    let config = NodeConfig::new(
        "node-gameplay-submit-ok",
        "world-gameplay-submit-ok",
        NodeRole::Sequencer,
    )
    .expect("node config")
    .with_tick_interval(Duration::from_millis(20))
    .expect("tick interval");
    let mut node_runtime = NodeRuntime::new(config).with_execution_hook(NoopExecutionHook);
    node_runtime.start().expect("start node runtime");
    let runtime = Arc::new(Mutex::new(node_runtime));

    let request = signed_gameplay_submit_request("node-gameplay-submit-ok", 9);
    let body = serde_json::to_string(&request).expect("serialize request");
    let http_request = format!(
        "POST /v1/chain/gameplay/submit HTTP/1.1\r\nHost: 127.0.0.1:5121\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let (mut server_stream, mut client_stream) = tcp_stream_pair();
    let handled = maybe_handle_gameplay_submit_request(
        &mut server_stream,
        http_request.as_bytes(),
        &runtime,
        "POST",
        "/v1/chain/gameplay/submit",
    )
    .expect("handler should process request");
    assert!(handled);
    drop(server_stream);

    client_stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set client timeout");
    let mut response_bytes = Vec::new();
    client_stream
        .read_to_end(&mut response_bytes)
        .expect("read handler response");
    let (status, response): (u16, ChainGameplaySubmitResponse) =
        decode_http_json_response(&response_bytes);
    assert_eq!(status, 200);
    assert!(response.ok);
    assert_eq!(response.action_id, Some(1));

    wait_for_committed_height(&runtime, 1);
    let snapshot = runtime
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .snapshot();
    assert!(snapshot.consensus.committed_height >= 1);
    assert!(snapshot.consensus.latest_height >= 1);

    runtime
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .stop()
        .expect("stop node runtime");
}

#[test]
fn gameplay_submit_handler_rejects_nonce_replay() {
    let _guard = gameplay_submit_test_guard();
    reset_gameplay_submit_state_for_tests();
    let config = NodeConfig::new(
        "node-gameplay-submit-replay",
        "world-gameplay-submit-replay",
        NodeRole::Sequencer,
    )
    .expect("node config");
    let runtime = Arc::new(Mutex::new(NodeRuntime::new(config)));

    let request = signed_gameplay_submit_request("node-gameplay-submit-replay", 11);
    let body = serde_json::to_string(&request).expect("serialize request");
    let http_request = format!(
        "POST /v1/chain/gameplay/submit HTTP/1.1\r\nHost: 127.0.0.1:5121\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );

    let (mut first_server, mut first_client) = tcp_stream_pair();
    maybe_handle_gameplay_submit_request(
        &mut first_server,
        http_request.as_bytes(),
        &runtime,
        "POST",
        "/v1/chain/gameplay/submit",
    )
    .expect("first handler call");
    drop(first_server);
    let mut first_response = Vec::new();
    first_client
        .read_to_end(&mut first_response)
        .expect("read first response");
    let (first_status, first_payload): (u16, ChainGameplaySubmitResponse) =
        decode_http_json_response(&first_response);
    assert_eq!(first_status, 200);
    assert!(first_payload.ok);

    let (mut replay_server, mut replay_client) = tcp_stream_pair();
    maybe_handle_gameplay_submit_request(
        &mut replay_server,
        http_request.as_bytes(),
        &runtime,
        "POST",
        "/v1/chain/gameplay/submit",
    )
    .expect("second handler call");
    drop(replay_server);
    replay_client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set client timeout");
    let mut replay_response = Vec::new();
    replay_client
        .read_to_end(&mut replay_response)
        .expect("read replay response");
    let (replay_status, replay_payload): (u16, ChainGameplaySubmitResponse) =
        decode_http_json_response(&replay_response);
    assert_eq!(replay_status, 409);
    assert!(!replay_payload.ok);
    assert_eq!(
        replay_payload.error_code.as_deref(),
        Some("auth_nonce_replay")
    );
}
