use super::{
    ChainGameplaySubmitResponse, maybe_handle_gameplay_submit_request,
    parse_gameplay_submit_request, reset_gameplay_submit_state_for_tests,
};
use ed25519_dalek::SigningKey;
use oasis7::consensus_action_payload::{
    ConsensusActionPayloadBody, decode_consensus_action_payload,
};
use oasis7::runtime::{Action, World};
use oasis7::viewer::{
    CollectDataCommand, CollectDataRequest, GameplayActionRequest, sign_collect_data_auth_proof,
    sign_gameplay_action_auth_proof,
};
use oasis7_node::{
    NodeConfig, NodeExecutionCommitContext, NodeExecutionCommitResult, NodeExecutionHook, NodeRole,
    NodeRuntime,
};
use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

fn gameplay_submit_test_guard() -> MutexGuard<'static, ()> {
    static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
    GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

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

#[derive(Debug)]
struct CapturingExecutionHook {
    calls: Arc<Mutex<Vec<NodeExecutionCommitContext>>>,
}

impl NodeExecutionHook for CapturingExecutionHook {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        let height = context.height;
        self.calls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(context);
        Ok(NodeExecutionCommitResult {
            execution_height: height,
            execution_block_hash: format!("capture-block-{height}"),
            execution_state_root: format!("capture-root-{height}"),
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
        actor_agent_id: None,
        player_id: player_id.to_string(),
        public_key: Some(public_key.clone()),
        auth: None,
    };
    let proof = sign_gameplay_action_auth_proof(&request, nonce, &public_key, &private_key)
        .expect("sign gameplay submit request");
    request.auth = Some(proof);
    request
}

fn collect_data_world_dir(
    label: &str,
    player_id: &str,
    agent_id: &str,
    public_key: &str,
) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "oasis7-chain-collect-data-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(path.as_path());
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: agent_id.to_string(),
        pos: oasis7::geometry::GeoPos {
            x_cm: 0,
            y_cm: 0,
            z_cm: 0,
        },
    });
    world.step().expect("register collector");
    world.submit_action(Action::ClaimStarterOc {
        agent_id: agent_id.to_string(),
        player_id: player_id.to_string(),
        public_key: Some(public_key.to_string()),
    });
    world.step().expect("persist player collector binding");
    world
        .save_to_dir(path.as_path())
        .expect("save collect data world");
    path
}

fn gameplay_nonce_world_dir(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "oasis7-chain-gameplay-nonce-{label}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(path.as_path());
    std::fs::create_dir_all(path.as_path()).expect("create gameplay nonce world dir");
    path
}

fn submit_json(
    runtime: &Arc<Mutex<NodeRuntime>>,
    execution_world_dir: &Path,
    body: &str,
) -> (u16, ChainGameplaySubmitResponse) {
    let http_request = format!(
        "POST /v1/chain/gameplay/submit HTTP/1.1\r\nHost: 127.0.0.1:5121\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let (mut server_stream, mut client_stream) = tcp_stream_pair();
    maybe_handle_gameplay_submit_request(
        &mut server_stream,
        http_request.as_bytes(),
        runtime,
        "POST",
        "/v1/chain/gameplay/submit",
        execution_world_dir,
    )
    .expect("handler should process request");
    drop(server_stream);
    client_stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("set client timeout");
    let mut response_bytes = Vec::new();
    client_stream
        .read_to_end(&mut response_bytes)
        .expect("read handler response");
    decode_http_json_response(&response_bytes)
}

fn signed_collect_data_submit(
    player_id: &str,
    nonce: u64,
    electricity_cost: i64,
    data_amount: i64,
    public_key: &str,
    private_key: &str,
) -> CollectDataCommand {
    let mut command = CollectDataCommand::Submit {
        request: CollectDataRequest {
            electricity_cost,
            data_amount,
            player_id: player_id.to_string(),
            public_key: Some(public_key.to_string()),
            auth: None,
        },
    };
    let proof = sign_collect_data_auth_proof(&command, nonce, public_key, private_key)
        .expect("sign collect data submit");
    let CollectDataCommand::Submit { request } = &mut command else {
        unreachable!()
    };
    request.auth = Some(proof);
    command
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

fn wait_for_committed_action(calls: &Arc<Mutex<Vec<NodeExecutionCommitContext>>>) {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(2) {
        if calls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .any(|context| !context.committed_actions.is_empty())
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let committed_action_count = calls
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .map(|context| context.committed_actions.len())
        .sum::<usize>();
    panic!("timed out waiting for committed action, observed {committed_action_count}");
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
        Path::new("unused-gameplay-world"),
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
    let world_dir = gameplay_nonce_world_dir("accept");

    let request = signed_gameplay_submit_request("browser-player-gameplay-submit-ok", 9);
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
        world_dir.as_path(),
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
fn gameplay_submit_handler_persists_strictly_increasing_nonce_across_reload() {
    let _guard = gameplay_submit_test_guard();
    reset_gameplay_submit_state_for_tests();
    let config = NodeConfig::new(
        "node-gameplay-submit-replay",
        "world-gameplay-submit-replay",
        NodeRole::Sequencer,
    )
    .expect("node config");
    let runtime = Arc::new(Mutex::new(NodeRuntime::new(config)));
    let world_dir = gameplay_nonce_world_dir("reload");

    let first = signed_gameplay_submit_request("node-gameplay-submit-replay", 11);
    let (first_status, first_payload) = submit_json(
        &runtime,
        world_dir.as_path(),
        serde_json::to_string(&first)
            .expect("serialize first request")
            .as_str(),
    );
    assert_eq!(first_status, 200);
    assert!(first_payload.ok);

    let increasing = signed_gameplay_submit_request("node-gameplay-submit-replay", 12);
    let (increasing_status, increasing_payload) = submit_json(
        &runtime,
        world_dir.as_path(),
        serde_json::to_string(&increasing)
            .expect("serialize increasing request")
            .as_str(),
    );
    assert_eq!(increasing_status, 200);
    assert!(increasing_payload.ok);

    for rejected_nonce in [10, 12] {
        let rejected =
            signed_gameplay_submit_request("node-gameplay-submit-replay", rejected_nonce);
        let (status, payload) = submit_json(
            &runtime,
            world_dir.as_path(),
            serde_json::to_string(&rejected)
                .expect("serialize rejected request")
                .as_str(),
        );
        assert_eq!(status, 409);
        assert_eq!(payload.error_code.as_deref(), Some("auth_nonce_replay"));
    }

    reset_gameplay_submit_state_for_tests();
    let replay_after_reload = signed_gameplay_submit_request("node-gameplay-submit-replay", 12);
    let (reload_status, reload_payload) = submit_json(
        &runtime,
        world_dir.as_path(),
        serde_json::to_string(&replay_after_reload)
            .expect("serialize replay after reload")
            .as_str(),
    );
    assert_eq!(reload_status, 409);
    assert_eq!(
        reload_payload.error_code.as_deref(),
        Some("auth_nonce_replay")
    );
}

#[test]
fn collect_data_submit_derives_bound_collector_and_commits_exact_action() {
    let _guard = gameplay_submit_test_guard();
    reset_gameplay_submit_state_for_tests();
    let (public_key, private_key) = gameplay_test_signer(31);
    let world_dir = collect_data_world_dir(
        "accept",
        "player-collect",
        "collector-authoritative",
        public_key.as_str(),
    );
    let calls = Arc::new(Mutex::new(Vec::new()));
    let config = NodeConfig::new(
        "node-collect-submit-ok",
        "world-collect-submit-ok",
        NodeRole::Sequencer,
    )
    .expect("node config")
    .with_tick_interval(Duration::from_millis(20))
    .expect("tick interval");
    let mut node_runtime = NodeRuntime::new(config).with_execution_hook(CapturingExecutionHook {
        calls: Arc::clone(&calls),
    });
    node_runtime.start().expect("start node runtime");
    let runtime = Arc::new(Mutex::new(node_runtime));
    let command = signed_collect_data_submit(
        "player-collect",
        41,
        7,
        11,
        public_key.as_str(),
        private_key.as_str(),
    );

    let (status, response) = submit_json(
        &runtime,
        world_dir.as_path(),
        serde_json::to_string(&command)
            .expect("serialize collect data command")
            .as_str(),
    );
    assert_eq!(status, 200);
    assert!(response.ok);
    wait_for_committed_action(&calls);
    let calls = calls
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let committed = calls
        .iter()
        .flat_map(|context| context.committed_actions.iter())
        .next()
        .expect("committed collect_data action");
    let ConsensusActionPayloadBody::RuntimeAction { action } =
        decode_consensus_action_payload(committed.payload_cbor.as_slice())
            .expect("decode collect_data payload")
    else {
        panic!("expected runtime action payload");
    };
    assert_eq!(
        action,
        Action::CollectDataAuthenticated {
            collector_agent_id: "collector-authoritative".to_string(),
            electricity_cost: 7,
            data_amount: 11,
            player_id: "player-collect".to_string(),
            public_key: public_key.clone(),
            nonce: 41,
        }
    );
    drop(calls);
    runtime
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .stop()
        .expect("stop node runtime");
    let _ = std::fs::remove_dir_all(world_dir);
}

#[test]
fn collect_data_submit_rejects_missing_authoritative_player_key_binding() {
    let _guard = gameplay_submit_test_guard();
    reset_gameplay_submit_state_for_tests();
    let (public_key, private_key) = gameplay_test_signer(32);
    let world_dir = collect_data_world_dir(
        "missing-binding",
        "another-player",
        "another-agent",
        public_key.as_str(),
    );
    let runtime = Arc::new(Mutex::new(NodeRuntime::new(
        NodeConfig::new(
            "node-collect-submit-binding",
            "world-collect-submit-binding",
            NodeRole::Sequencer,
        )
        .expect("node config"),
    )));
    let command = signed_collect_data_submit(
        "player-without-binding",
        42,
        7,
        11,
        public_key.as_str(),
        private_key.as_str(),
    );

    let (status, response) = submit_json(
        &runtime,
        world_dir.as_path(),
        serde_json::to_string(&command)
            .expect("serialize collect data command")
            .as_str(),
    );
    assert_eq!(status, 403);
    assert!(!response.ok);
    assert_eq!(response.error_code.as_deref(), Some("invalid_auth"));
    let _ = std::fs::remove_dir_all(world_dir);
}

#[test]
fn collect_data_submit_rejects_cost_tampering() {
    let _guard = gameplay_submit_test_guard();
    reset_gameplay_submit_state_for_tests();
    let (public_key, private_key) = gameplay_test_signer(33);
    let world_dir = collect_data_world_dir(
        "tamper",
        "player-tamper",
        "collector-tamper",
        public_key.as_str(),
    );
    let runtime = Arc::new(Mutex::new(NodeRuntime::new(
        NodeConfig::new(
            "node-collect-submit-tamper",
            "world-collect-submit-tamper",
            NodeRole::Sequencer,
        )
        .expect("node config"),
    )));
    let mut command = signed_collect_data_submit(
        "player-tamper",
        43,
        7,
        11,
        public_key.as_str(),
        private_key.as_str(),
    );
    let CollectDataCommand::Submit { request } = &mut command else {
        unreachable!()
    };
    request.electricity_cost = 8;

    let (status, response) = submit_json(
        &runtime,
        world_dir.as_path(),
        serde_json::to_string(&command)
            .expect("serialize tampered collect data command")
            .as_str(),
    );
    assert_eq!(status, 401);
    assert!(!response.ok);
    assert_eq!(response.error_code.as_deref(), Some("invalid_auth"));

    let authorized = signed_collect_data_submit(
        "player-tamper",
        43,
        7,
        11,
        public_key.as_str(),
        private_key.as_str(),
    );
    let (authorized_status, authorized_response) = submit_json(
        &runtime,
        world_dir.as_path(),
        serde_json::to_string(&authorized)
            .expect("serialize authorized collect data command")
            .as_str(),
    );
    assert_eq!(authorized_status, 200);
    assert!(authorized_response.ok);
    let _ = std::fs::remove_dir_all(world_dir);
}
