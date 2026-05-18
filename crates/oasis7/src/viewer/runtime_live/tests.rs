use super::*;
use crate::simulator::ResourceOwner;
use crate::simulator::{
    ProviderExecutionMode, DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION,
    DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION,
};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

mod auth_actions;
mod authoritative;
mod background_play;
mod chain_sync_feedback;
mod industrial_progression;
mod prompt_control;
mod provider_settings;
mod snapshot_progress;
#[path = "tests_support.rs"]
mod tests_support;

use tests_support::*;

fn send_runtime_live_request(writer: &mut BufWriter<TcpStream>, request: &ViewerRequest) {
    serde_json::to_writer(&mut *writer, request).expect("write request");
    writer.write_all(b"\n").expect("write newline");
    writer.flush().expect("flush request");
}

fn read_runtime_live_hello_ack(reader: &mut BufReader<TcpStream>) {
    loop {
        let response = read_runtime_live_response(reader);
        if matches!(response, ViewerResponse::HelloAck { .. }) {
            return;
        }
    }
}

fn read_runtime_live_snapshot(reader: &mut BufReader<TcpStream>) -> WorldSnapshot {
    loop {
        let response = read_runtime_live_response(reader);
        if let ViewerResponse::Snapshot { snapshot } = response {
            return snapshot;
        }
    }
}

fn read_runtime_live_response(reader: &mut BufReader<TcpStream>) -> ViewerResponse {
    let mut line = String::new();
    reader.read_line(&mut line).expect("read response");
    serde_json::from_str(line.trim_end()).expect("decode response")
}

fn test_writer_pair() -> (BufWriter<TcpStream>, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test listener");
    let addr = listener.local_addr().expect("listener local addr");
    let client = TcpStream::connect(addr).expect("connect test client");
    let (server, _) = listener.accept().expect("accept test peer");
    (BufWriter::new(server), client)
}

fn read_response_line(peer: &TcpStream, timeout: Duration) -> Option<String> {
    let stream = peer.try_clone().expect("clone test peer");
    stream
        .set_read_timeout(Some(timeout))
        .expect("set read timeout");
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) => None,
        Ok(_) => Some(line),
        Err(err) => {
            if matches!(
                err.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) {
                None
            } else {
                panic!("read response line failed: {err}");
            }
        }
    }
}

fn read_control_completion_ack(
    peer: &TcpStream,
    timeout: Duration,
) -> Option<crate::viewer::ControlCompletionAck> {
    let stream = peer.try_clone().expect("clone test peer");
    stream
        .set_read_timeout(Some(Duration::from_millis(100)))
        .expect("set read timeout");
    let mut reader = BufReader::new(stream);
    let start = Instant::now();
    let mut line = String::new();
    while start.elapsed() < timeout {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) => continue,
            Ok(_) => {}
            Err(err) => {
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) {
                    continue;
                }
                panic!("read response line failed: {err}");
            }
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(response) = serde_json::from_str::<crate::viewer::ViewerResponse>(trimmed) else {
            continue;
        };
        if let crate::viewer::ViewerResponse::ControlCompletionAck { ack } = response {
            return Some(ack);
        }
    }
    None
}

fn read_test_http_request(stream: &mut TcpStream) -> Vec<u8> {
    let mut request = Vec::new();
    let mut buffer = [0_u8; 4096];
    let mut expected_len = None;
    loop {
        let bytes = stream
            .read(&mut buffer)
            .expect("read test http request chunk");
        if bytes == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..bytes]);
        if expected_len.is_none() {
            if let Some(boundary) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                let content_length = parse_test_http_content_length(&request[..boundary]);
                expected_len = Some(boundary + 4 + content_length);
            }
        }
        if let Some(expected_len) = expected_len {
            if request.len() >= expected_len {
                break;
            }
        }
    }
    request
}

fn parse_test_http_content_length(header_bytes: &[u8]) -> usize {
    let header = std::str::from_utf8(header_bytes).expect("test request header utf-8");
    header
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            if name.trim().eq_ignore_ascii_case("content-length") {
                Some(
                    value
                        .trim()
                        .parse::<usize>()
                        .expect("test request content-length"),
                )
            } else {
                None
            }
        })
        .unwrap_or(0)
}

fn wait_for_runtime_live_server(addr: &str) {
    for _ in 0..50 {
        if TcpStream::connect(addr).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("runtime live server did not start listening at {addr}");
}

struct TestChainStatusServer {
    addr: String,
    committed_height: Arc<AtomicU64>,
    release_security_policy: ReleaseSecurityPolicy,
    submitted_gameplay_requests: Arc<Mutex<Vec<crate::viewer::GameplayActionRequest>>>,
    stop: Arc<AtomicBool>,
    join_handle: Option<thread::JoinHandle<()>>,
}

impl TestChainStatusServer {
    fn start(execution_world_dir: std::path::PathBuf) -> Self {
        Self::start_with_release_security_policy(
            execution_world_dir,
            ReleaseSecurityPolicy::production_hardened(),
        )
    }

    fn start_with_release_security_policy(
        execution_world_dir: std::path::PathBuf,
        release_security_policy: ReleaseSecurityPolicy,
    ) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind chain status server");
        listener
            .set_nonblocking(true)
            .expect("set chain status listener nonblocking");
        let addr = listener.local_addr().expect("chain status local addr");
        let committed_height = Arc::new(AtomicU64::new(0));
        let submitted_gameplay_requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let committed_height_for_thread = Arc::clone(&committed_height);
        let submitted_requests_for_thread = Arc::clone(&submitted_gameplay_requests);
        let next_gameplay_action_id_for_thread = Arc::new(AtomicU64::new(1));
        let stop_for_thread = Arc::clone(&stop);
        let execution_world_dir_for_thread = execution_world_dir.clone();
        let release_security_policy_for_thread = release_security_policy.clone();
        let join_handle = thread::spawn(move || loop {
            if stop_for_thread.load(Ordering::SeqCst) {
                break;
            }
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let request = read_test_http_request(&mut stream);
                    let request_bytes = request.as_slice();
                    let request_text = String::from_utf8_lossy(request_bytes);
                    let mut parts = request_text
                        .lines()
                        .next()
                        .unwrap_or_default()
                        .split_whitespace();
                    let method = parts.next().unwrap_or_default();
                    let path = parts
                        .next()
                        .unwrap_or_default()
                        .split('?')
                        .next()
                        .unwrap_or_default();

                    match (method, path) {
                        ("GET", "/v1/chain/status") => {
                            let body = serde_json::json!({
                                "consensus": {
                                    "committed_height": committed_height_for_thread.load(Ordering::SeqCst),
                                },
                                "execution_world_dir": execution_world_dir_for_thread,
                                "release_security_policy": release_security_policy_for_thread,
                            });
                            let body = serde_json::to_vec(&body).expect("encode chain status body");
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                body.len()
                            );
                            stream
                                .write_all(response.as_bytes())
                                .expect("write chain status header");
                            stream
                                .write_all(body.as_slice())
                                .expect("write chain status body");
                            stream.flush().expect("flush chain status response");
                        }
                        ("POST", "/v1/chain/gameplay/submit") => {
                            let boundary = request_bytes
                                .windows(4)
                                .position(|window| window == b"\r\n\r\n")
                                .expect("gameplay submit body boundary");
                            let body = &request_bytes[(boundary + 4)..];
                            let gameplay_request = serde_json::from_slice::<
                                crate::viewer::GameplayActionRequest,
                            >(body)
                            .expect("decode gameplay submit request");
                            submitted_requests_for_thread
                                .lock()
                                .expect("lock submitted requests")
                                .push(gameplay_request);
                            let action_id =
                                next_gameplay_action_id_for_thread.fetch_add(1, Ordering::SeqCst);
                            let body = serde_json::json!({
                                "ok": true,
                                "action_id": action_id,
                                "submitted_at_unix_ms": test_now_unix_ms(),
                            });
                            let body =
                                serde_json::to_vec(&body).expect("encode gameplay submit body");
                            let response = format!(
                                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                body.len()
                            );
                            stream
                                .write_all(response.as_bytes())
                                .expect("write gameplay submit header");
                            stream
                                .write_all(body.as_slice())
                                .expect("write gameplay submit body");
                            stream.flush().expect("flush gameplay submit response");
                        }
                        _ => {
                            stream
                                .write_all(
                                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 21\r\nConnection: close\r\n\r\n{\"error\":\"not found\"}",
                                )
                                .expect("write 404 response");
                            stream.flush().expect("flush 404 response");
                        }
                    }
                }
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::WouldBlock {
                        thread::sleep(Duration::from_millis(10));
                        continue;
                    }
                    panic!("accept chain status connection failed: {err}");
                }
            }
        });
        Self {
            addr: addr.to_string(),
            committed_height,
            release_security_policy,
            submitted_gameplay_requests,
            stop,
            join_handle: Some(join_handle),
        }
    }

    fn submitted_gameplay_requests(&self) -> Vec<crate::viewer::GameplayActionRequest> {
        self.submitted_gameplay_requests
            .lock()
            .expect("lock submitted requests")
            .clone()
    }
}

impl Drop for TestChainStatusServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(self.addr.as_str());
        if let Some(join_handle) = self.join_handle.take() {
            join_handle.join().expect("join chain status server");
        }
    }
}

fn runtime_live_temp_dir(label: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "oasis7_runtime_live_chain_status_{label}_{}_{}",
        std::process::id(),
        test_now_unix_ms()
    ));
    std::fs::create_dir_all(&dir).expect("create runtime live temp dir");
    dir
}

fn signed_prompt_control_apply_request(
    mut request: crate::viewer::PromptControlApplyRequest,
    intent: crate::viewer::PromptControlAuthIntent,
    nonce: u64,
    public_key_hex: &str,
    private_key_hex: &str,
) -> crate::viewer::PromptControlApplyRequest {
    request.public_key = Some(public_key_hex.to_string());
    let proof = crate::viewer::sign_prompt_control_apply_auth_proof(
        intent,
        &request,
        nonce,
        public_key_hex,
        private_key_hex,
    )
    .expect("sign prompt auth");
    request.auth = Some(proof);
    request
}

fn signed_agent_chat_request(
    mut request: crate::viewer::AgentChatRequest,
    nonce: u64,
    public_key_hex: &str,
    private_key_hex: &str,
) -> crate::viewer::AgentChatRequest {
    request.public_key = Some(public_key_hex.to_string());
    if request.intent_seq.is_none() {
        request.intent_seq = Some(nonce);
    }
    let proof =
        crate::viewer::sign_agent_chat_auth_proof(&request, nonce, public_key_hex, private_key_hex)
            .expect("sign agent chat auth");
    request.auth = Some(proof);
    request
}

fn signed_gameplay_action_request(
    mut request: crate::viewer::GameplayActionRequest,
    nonce: u64,
    public_key_hex: &str,
    private_key_hex: &str,
) -> crate::viewer::GameplayActionRequest {
    request.public_key = Some(public_key_hex.to_string());
    let proof = crate::viewer::sign_gameplay_action_auth_proof(
        &request,
        nonce,
        public_key_hex,
        private_key_hex,
    )
    .expect("sign gameplay action auth");
    request.auth = Some(proof);
    request
}

fn signed_session_register_request(
    mut request: crate::viewer::AuthoritativeSessionRegisterRequest,
    nonce: u64,
    public_key_hex: &str,
    private_key_hex: &str,
) -> crate::viewer::AuthoritativeSessionRegisterRequest {
    request.public_key = Some(public_key_hex.to_string());
    let proof = crate::viewer::sign_session_register_auth_proof(
        &request,
        nonce,
        public_key_hex,
        private_key_hex,
    )
    .expect("sign session register auth");
    request.auth = Some(proof);
    request
}

fn register_runtime_session(
    server: &mut ViewerRuntimeLiveServer,
    player_id: &str,
    agent_id: Option<&str>,
    nonce: u64,
    public_key_hex: &str,
    private_key_hex: &str,
) -> AuthoritativeRecoveryAck<u64> {
    register_runtime_session_with_options(
        server,
        player_id,
        agent_id,
        false,
        nonce,
        public_key_hex,
        private_key_hex,
    )
}

fn register_runtime_session_with_options(
    server: &mut ViewerRuntimeLiveServer,
    player_id: &str,
    agent_id: Option<&str>,
    force_rebind: bool,
    nonce: u64,
    public_key_hex: &str,
    private_key_hex: &str,
) -> AuthoritativeRecoveryAck<u64> {
    let request = signed_session_register_request(
        crate::viewer::AuthoritativeSessionRegisterRequest {
            player_id: player_id.to_string(),
            public_key: None,
            auth: None,
            requested_agent_id: agent_id.map(ToOwned::to_owned),
            force_rebind,
        },
        nonce,
        public_key_hex,
        private_key_hex,
    );
    let (ack, emit_snapshot_after_ack) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RegisterSession { request })
        .expect("register session");
    assert!(!emit_snapshot_after_ack);
    ack
}

#[test]
fn runtime_live_run_accepts_probe_while_viewer_session_is_open() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("reserve port");
    let addr = listener.local_addr().expect("local addr");
    drop(listener);

    let server_addr = addr.to_string();
    thread::spawn(move || {
        let server = ViewerRuntimeLiveServer::new(
            ViewerRuntimeLiveServerConfig::new(WorldScenario::LlmBootstrap)
                .with_bind_addr(server_addr),
        )
        .expect("create server");
        server.run().expect("run server");
    });
    wait_for_runtime_live_server(addr.to_string().as_str());

    let (mut viewer_reader, mut viewer_writer) =
        connect_runtime_live_client(addr.to_string().as_str());
    send_runtime_live_request(
        &mut viewer_writer,
        &ViewerRequest::Subscribe {
            streams: vec![
                ViewerStream::Snapshot,
                ViewerStream::Events,
                ViewerStream::Metrics,
            ],
            event_kinds: Vec::new(),
        },
    );
    send_runtime_live_request(&mut viewer_writer, &ViewerRequest::RequestSnapshot);
    let viewer_snapshot = read_runtime_live_snapshot(&mut viewer_reader);
    assert!(
        !viewer_snapshot.model.agents.is_empty(),
        "expected seeded agents in runtime snapshot"
    );

    let (mut probe_reader, mut probe_writer) =
        connect_runtime_live_client(addr.to_string().as_str());
    send_runtime_live_request(&mut probe_writer, &ViewerRequest::RequestSnapshot);
    let probe_snapshot = read_runtime_live_snapshot(&mut probe_reader);
    assert_eq!(
        probe_snapshot.model.agents.len(),
        viewer_snapshot.model.agents.len()
    );
}

#[test]
fn runtime_live_agent_chat_echo_flushes_virtual_event_immediately_over_socket() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    std::env::set_var(RUNTIME_AGENT_CHAT_ECHO_ENV, "1");
    std::env::remove_var(crate::simulator::ENV_LLM_MODEL);
    std::env::remove_var(crate::simulator::ENV_LLM_BASE_URL);
    std::env::remove_var(crate::simulator::ENV_LLM_API_KEY);

    let listener = TcpListener::bind("127.0.0.1:0").expect("reserve port");
    let addr = listener.local_addr().expect("local addr");
    drop(listener);

    let server_addr = addr.to_string();
    thread::spawn(move || {
        let server = ViewerRuntimeLiveServer::new(
            ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
                .with_bind_addr(server_addr)
                .with_decision_mode(ViewerLiveDecisionMode::Llm),
        )
        .expect("create server");
        server.run().expect("run server");
    });
    wait_for_runtime_live_server(addr.to_string().as_str());

    let (mut reader, mut writer) = connect_runtime_live_client(addr.to_string().as_str());
    send_runtime_live_request(&mut writer, &ViewerRequest::RequestSnapshot);
    let snapshot = read_runtime_live_snapshot(&mut reader);
    let agent_id = snapshot
        .model
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    match read_runtime_live_response(&mut reader) {
        ViewerResponse::AuthoritativeRecoveryAck { ack } => {
            assert_eq!(ack.status, AuthoritativeRecoveryStatus::CatchUpReady);
        }
        other => panic!("expected recovery ack after snapshot request, got {other:?}"),
    }

    send_runtime_live_request(
        &mut writer,
        &ViewerRequest::Subscribe {
            streams: vec![ViewerStream::Events],
            event_kinds: Vec::new(),
        },
    );

    let (public_key, private_key) = test_signer(34);
    let register_request = signed_session_register_request(
        crate::viewer::AuthoritativeSessionRegisterRequest {
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
            requested_agent_id: Some(agent_id.clone()),
            force_rebind: false,
        },
        34,
        public_key.as_str(),
        private_key.as_str(),
    );
    send_runtime_live_request(
        &mut writer,
        &ViewerRequest::AuthoritativeRecovery {
            command: AuthoritativeRecoveryCommand::RegisterSession {
                request: register_request,
            },
        },
    );
    match read_runtime_live_response(&mut reader) {
        ViewerResponse::AuthoritativeRecoveryAck { ack } => {
            assert_eq!(ack.status, AuthoritativeRecoveryStatus::SessionRegistered);
            assert_eq!(ack.player_id.as_deref(), Some("player-a"));
            assert_eq!(ack.agent_id.as_deref(), Some(agent_id.as_str()));
        }
        other => panic!("expected session register ack, got {other:?}"),
    }

    let chat_request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello runtime echo over socket".to_string(),
            intent_tick: Some(snapshot.time),
            intent_seq: Some(35),
        },
        35,
        public_key.as_str(),
        private_key.as_str(),
    );
    send_runtime_live_request(
        &mut writer,
        &ViewerRequest::AgentChat {
            request: chat_request,
        },
    );

    match read_runtime_live_response(&mut reader) {
        ViewerResponse::AgentChatAck { ack } => {
            assert_eq!(ack.agent_id, agent_id);
            assert_eq!(ack.player_id.as_deref(), Some("player-a"));
        }
        other => panic!("expected agent chat ack, got {other:?}"),
    }
    let mut saw_echo_event = false;
    loop {
        match read_runtime_live_response(&mut reader) {
            ViewerResponse::Event { event } => {
                saw_echo_event |= matches!(
                    &event.kind,
                    crate::simulator::WorldEventKind::AgentSpoke {
                        agent_id: event_agent_id,
                        message,
                        ..
                    } if event_agent_id == &agent_id && message == "[qa-echo] hello runtime echo over socket"
                );
            }
            ViewerResponse::AuthoritativeBatch { .. } => {
                assert!(
                    saw_echo_event,
                    "expected qa echo event before authoritative batch flush"
                );
                break;
            }
            other => {
                panic!("expected event stream or authoritative batch after chat ack, got {other:?}")
            }
        }
    }
}

#[test]
fn runtime_simulator_action_mapping_equivalence_covers_core_gameplay_and_economy() {
    let server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let assert_mapped = |action: crate::simulator::Action, expected: RuntimeAction| {
        let mapped = control_plane::simulator_action_to_runtime(&action, &server.world)
            .expect("action should map to runtime");
        assert_eq!(mapped, expected);
    };

    let move_target = GeoPos::new(10, 20, 30);
    assert_mapped(
        crate::simulator::Action::MoveAgent {
            agent_id: "agent-1".to_string(),
            to: location_id_for_pos(move_target),
        },
        RuntimeAction::MoveAgent {
            agent_id: "agent-1".to_string(),
            to: move_target,
        },
    );
    assert_mapped(
        crate::simulator::Action::TransferResource {
            from: ResourceOwner::Agent {
                agent_id: "agent-1".to_string(),
            },
            to: ResourceOwner::Agent {
                agent_id: "agent-2".to_string(),
            },
            kind: ResourceKind::Electricity,
            amount: 3,
        },
        RuntimeAction::TransferResource {
            from_agent_id: "agent-1".to_string(),
            to_agent_id: "agent-2".to_string(),
            kind: ResourceKind::Electricity,
            amount: 3,
        },
    );
    assert_mapped(
        crate::simulator::Action::DeclareWar {
            initiator_agent_id: "agent-1".to_string(),
            war_id: "war.alpha".to_string(),
            aggressor_alliance_id: "alliance.a".to_string(),
            defender_alliance_id: "alliance.b".to_string(),
            objective: "expand".to_string(),
            intensity: 2,
        },
        RuntimeAction::DeclareWar {
            initiator_agent_id: "agent-1".to_string(),
            war_id: "war.alpha".to_string(),
            aggressor_alliance_id: "alliance.a".to_string(),
            defender_alliance_id: "alliance.b".to_string(),
            objective: "expand".to_string(),
            intensity: 2,
        },
    );
    assert_mapped(
        crate::simulator::Action::OpenEconomicContract {
            creator_agent_id: "agent-1".to_string(),
            contract_id: "contract.alpha".to_string(),
            counterparty_agent_id: "agent-2".to_string(),
            settlement_kind: ResourceKind::Data,
            settlement_amount: 5,
            reputation_stake: 7,
            expires_at: 99,
            description: "trade".to_string(),
        },
        RuntimeAction::OpenEconomicContract {
            creator_agent_id: "agent-1".to_string(),
            contract_id: "contract.alpha".to_string(),
            counterparty_agent_id: "agent-2".to_string(),
            settlement_kind: ResourceKind::Data,
            settlement_amount: 5,
            reputation_stake: 7,
            expires_at: 99,
            description: "trade".to_string(),
        },
    );
}

#[test]
fn runtime_live_server_config_play_interval_defaults_and_clamps() {
    let config = ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal);
    assert_eq!(config.play_step_interval, Duration::from_millis(800));

    let clamped = config.with_play_step_interval(Duration::from_millis(10));
    assert_eq!(clamped.play_step_interval, Duration::from_millis(50));
}

#[test]
fn runtime_live_session_play_step_is_interval_gated() {
    let mut session = RuntimeLiveSession::new();
    session.playing = true;

    assert!(session.should_advance_play_step(Duration::from_millis(40)));
    assert!(!session.should_advance_play_step(Duration::from_millis(40)));
    std::thread::sleep(Duration::from_millis(50));
    assert!(session.should_advance_play_step(Duration::from_millis(40)));
}

#[test]
fn runtime_simulator_action_mapping_covers_module_artifact_actions() {
    let server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let mut source_files = std::collections::BTreeMap::new();
    source_files.insert("module.toml".to_string(), b"manifest".to_vec());
    source_files.insert("src/lib.rs".to_string(), b"pub fn run() {}".to_vec());

    let compile = crate::simulator::Action::CompileModuleArtifactFromSource {
        publisher_agent_id: "agent-1".to_string(),
        module_id: "module.alpha".to_string(),
        manifest_path: "module.toml".to_string(),
        source_files: source_files.clone(),
    };
    let compile_mapped = control_plane::simulator_action_to_runtime(&compile, &server.world)
        .expect("compile action should map");
    assert_eq!(
        compile_mapped,
        RuntimeAction::CompileModuleArtifactFromSource {
            publisher_agent_id: "agent-1".to_string(),
            module_id: "module.alpha".to_string(),
            source_package: crate::runtime::ModuleSourcePackage {
                manifest_path: "module.toml".to_string(),
                files: source_files,
            },
        }
    );

    let deploy = crate::simulator::Action::DeployModuleArtifact {
        publisher_agent_id: "agent-1".to_string(),
        wasm_hash: "hash.alpha".to_string(),
        wasm_bytes: vec![0xAA, 0xBB],
        module_id_hint: Some("module.alpha".to_string()),
    };
    let deploy_mapped = control_plane::simulator_action_to_runtime(&deploy, &server.world)
        .expect("deploy action should map");
    assert_eq!(
        deploy_mapped,
        RuntimeAction::DeployModuleArtifact {
            publisher_agent_id: "agent-1".to_string(),
            wasm_hash: "hash.alpha".to_string(),
            wasm_bytes: vec![0xAA, 0xBB],
        }
    );

    let list = crate::simulator::Action::ListModuleArtifactForSale {
        seller_agent_id: "agent-1".to_string(),
        wasm_hash: "hash.alpha".to_string(),
        price_kind: ResourceKind::Data,
        price_amount: 9,
    };
    let list_mapped = control_plane::simulator_action_to_runtime(&list, &server.world)
        .expect("list action should map");
    assert_eq!(
        list_mapped,
        RuntimeAction::ListModuleArtifactForSale {
            seller_agent_id: "agent-1".to_string(),
            wasm_hash: "hash.alpha".to_string(),
            price_kind: ResourceKind::Data,
            price_amount: 9,
        }
    );

    let buy = crate::simulator::Action::BuyModuleArtifact {
        buyer_agent_id: "agent-2".to_string(),
        wasm_hash: "hash.alpha".to_string(),
    };
    let buy_mapped = control_plane::simulator_action_to_runtime(&buy, &server.world)
        .expect("buy action should map");
    assert_eq!(
        buy_mapped,
        RuntimeAction::BuyModuleArtifact {
            buyer_agent_id: "agent-2".to_string(),
            wasm_hash: "hash.alpha".to_string(),
        }
    );
}

#[test]
fn runtime_simulator_action_mapping_includes_industrial_actions() {
    let server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let build_factory = crate::simulator::Action::BuildFactory {
        owner: ResourceOwner::Agent {
            agent_id: "agent-1".to_string(),
        },
        location_id: "runtime:10:20:0".to_string(),
        factory_id: "factory.alpha".to_string(),
        factory_kind: "factory.assembler.mk1".to_string(),
    };
    let build_mapped = control_plane::simulator_action_to_runtime(&build_factory, &server.world)
        .expect("build factory action should map");
    assert_eq!(
        build_mapped,
        RuntimeAction::BuildFactory {
            builder_agent_id: "agent-1".to_string(),
            site_id: "runtime:10:20:0".to_string(),
            spec: crate::runtime::FactoryModuleSpec {
                factory_id: "factory.alpha".to_string(),
                display_name: "Assembler MK1".to_string(),
                tier: 3,
                tags: vec!["assembler".to_string(), "precision".to_string()],
                build_cost: vec![
                    crate::runtime::MaterialStack::new("structural_frame", 8),
                    crate::runtime::MaterialStack::new("iron_ingot", 10),
                    crate::runtime::MaterialStack::new("copper_wire", 8),
                ],
                build_time_ticks: 1,
                base_power_draw: 20,
                recipe_slots: 2,
                throughput_bps: 10_000,
                maintenance_per_tick: 1,
            },
        }
    );

    let schedule_recipe = crate::simulator::Action::ScheduleRecipe {
        owner: ResourceOwner::Agent {
            agent_id: "agent-1".to_string(),
        },
        factory_id: "factory.alpha".to_string(),
        recipe_id: "recipe.assembler.control_chip".to_string(),
        batches: 3,
    };
    let schedule_mapped =
        control_plane::simulator_action_to_runtime(&schedule_recipe, &server.world)
            .expect("schedule recipe action should map");
    assert_eq!(
        schedule_mapped,
        RuntimeAction::ScheduleRecipe {
            requester_agent_id: "agent-1".to_string(),
            factory_id: "factory.alpha".to_string(),
            recipe_id: "recipe.assembler.control_chip".to_string(),
            plan: crate::runtime::RecipeExecutionPlan::accepted(
                3,
                vec![
                    crate::runtime::MaterialStack::new("copper_wire", 12),
                    crate::runtime::MaterialStack::new("polymer_resin", 6),
                ],
                vec![crate::runtime::MaterialStack::new("control_chip", 3)],
                vec![crate::runtime::MaterialStack::new("waste_resin", 3)],
                18,
                1,
            ),
        }
    );

    let transfer_to_location = crate::simulator::Action::TransferResource {
        from: ResourceOwner::Agent {
            agent_id: "agent-1".to_string(),
        },
        to: ResourceOwner::Location {
            location_id: "loc-1".to_string(),
        },
        kind: ResourceKind::Electricity,
        amount: 1,
    };
    assert!(
        control_plane::simulator_action_to_runtime(&transfer_to_location, &server.world).is_none()
    );
}

#[test]
fn chain_linked_runtime_sync_advances_without_play() {
    let execution_world_dir = runtime_live_temp_dir("chain_sync_progress");
    let mut execution_world = crate::runtime::World::new_production_hardened();
    execution_world.submit_action(RuntimeAction::RegisterAgent {
        agent_id: "chain-agent".to_string(),
        pos: crate::geometry::GeoPos::new(1, 2, 0),
    });
    execution_world.step().expect("advance execution world");
    execution_world.submit_action(RuntimeAction::MoveAgent {
        agent_id: "chain-agent".to_string(),
        to: crate::geometry::GeoPos::new(5, 2, 0),
    });
    execution_world
        .step()
        .expect("advance execution world again");
    execution_world
        .save_to_dir(execution_world_dir.as_path())
        .expect("persist execution world");

    let chain_status = TestChainStatusServer::start(execution_world_dir.clone());
    chain_status.committed_height.store(1, Ordering::SeqCst);

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_chain_status_bind(chain_status.addr.clone())
            .with_chain_poll_interval(Duration::from_millis(50)),
    )
    .expect("runtime server");
    let mut session = RuntimeLiveSession::new();
    session.playing = false;
    session.subscribed.insert(ViewerStream::Events);
    session.subscribed.insert(ViewerStream::Snapshot);
    let (mut writer, peer) = test_writer_pair();

    let progressed = server
        .sync_chain_linked_runtime(&mut session, &mut writer)
        .expect("chain sync should succeed");

    assert!(progressed, "chain-linked sync should report progress");
    assert_eq!(server.world.state().time, execution_world.state().time);
    let line =
        read_response_line(&peer, Duration::from_millis(200)).expect("expected sync response");
    assert!(!line.trim().is_empty());
}

#[test]
fn chain_linked_runtime_empty_poll_does_not_advance_world() {
    let execution_world_dir = runtime_live_temp_dir("chain_sync_idle");
    let execution_world = crate::runtime::World::new_production_hardened();
    execution_world
        .save_to_dir(execution_world_dir.as_path())
        .expect("persist empty execution world");

    let chain_status = TestChainStatusServer::start(execution_world_dir);
    chain_status.committed_height.store(0, Ordering::SeqCst);

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_chain_status_bind(chain_status.addr.clone())
            .with_chain_poll_interval(Duration::from_millis(50)),
    )
    .expect("runtime server");
    server.latest_player_gameplay_feedback = Some(crate::simulator::PlayerGameplayRecentFeedback {
        action: "chain_sync".to_string(),
        stage: "blocked".to_string(),
        effect: "committed runtime sync failed before the viewer could observe new world state"
            .to_string(),
        intent_summary: None,
        target_agent_id: None,
        reason: Some("simulated missing persistence".to_string()),
        hint: Some("wait for execution world persistence".to_string()),
        delta_logical_time: 0,
        delta_event_seq: 0,
    });
    let mut session = RuntimeLiveSession::new();
    session.playing = false;
    session.subscribed.insert(ViewerStream::Events);
    session.subscribed.insert(ViewerStream::Snapshot);
    let initial_time = server.world.state().time;
    let (mut writer, peer) = test_writer_pair();

    let progressed = server
        .sync_chain_linked_runtime(&mut session, &mut writer)
        .expect("chain sync should succeed");

    assert!(!progressed, "idle chain poll should not report progress");
    assert_eq!(server.world.state().time, initial_time);
    assert!(read_response_line(&peer, Duration::from_millis(100)).is_none());
    assert_eq!(server.last_chain_committed_height, 0);
    assert!(
        server.latest_player_gameplay_feedback.is_none(),
        "successful zero-delta chain sync should clear stale chain_sync feedback"
    );
}

#[test]
fn chain_linked_runtime_zero_delta_does_not_accept_committed_height() {
    let execution_world_dir = runtime_live_temp_dir("chain_sync_zero_delta_height");
    let execution_world = crate::runtime::World::new_production_hardened();
    execution_world
        .save_to_dir(execution_world_dir.as_path())
        .expect("persist empty execution world");

    let chain_status = TestChainStatusServer::start(execution_world_dir);
    chain_status.committed_height.store(1, Ordering::SeqCst);

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_chain_status_bind(chain_status.addr.clone())
            .with_chain_poll_interval(Duration::from_millis(50)),
    )
    .expect("runtime server");
    let mut session = RuntimeLiveSession::new();
    session.playing = false;
    session.subscribed.insert(ViewerStream::Events);
    session.subscribed.insert(ViewerStream::Snapshot);
    let initial_time = server.world.state().time;
    let (mut writer, peer) = test_writer_pair();

    let progressed = server
        .sync_chain_linked_runtime(&mut session, &mut writer)
        .expect("chain sync should succeed");

    assert!(
        !progressed,
        "zero-delta chain poll should not report progress"
    );
    assert_eq!(server.world.state().time, initial_time);
    assert_eq!(server.last_chain_committed_height, 0);
    assert!(read_response_line(&peer, Duration::from_millis(100)).is_none());
}

#[test]
fn chain_linked_runtime_committed_height_zero_skips_bootstrap_execution_world_validation() {
    let execution_world_dir = runtime_live_temp_dir("chain_sync_zero_committed_height");
    let mut execution_world = crate::runtime::World::new_production_hardened();
    execution_world.submit_action(RuntimeAction::RegisterAgent {
        agent_id: "chain-agent".to_string(),
        pos: crate::geometry::GeoPos::new(1, 2, 0),
    });
    execution_world.step().expect("advance execution world");
    execution_world
        .save_to_dir(execution_world_dir.as_path())
        .expect("persist execution world");

    let chain_status = TestChainStatusServer::start_with_release_security_policy(
        execution_world_dir,
        ReleaseSecurityPolicy::default(),
    );
    chain_status.committed_height.store(0, Ordering::SeqCst);

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_chain_status_bind(chain_status.addr.clone())
            .with_chain_poll_interval(Duration::from_millis(50)),
    )
    .expect("runtime server");
    server.latest_player_gameplay_feedback = Some(crate::simulator::PlayerGameplayRecentFeedback {
        action: "chain_sync".to_string(),
        stage: "blocked".to_string(),
        effect: "stale bootstrap execution world should be ignored before the first commit"
            .to_string(),
        intent_summary: None,
        target_agent_id: None,
        reason: Some("bootstrap-only".to_string()),
        hint: Some("wait for first committed height".to_string()),
        delta_logical_time: 0,
        delta_event_seq: 0,
    });
    let mut session = RuntimeLiveSession::new();
    session.playing = false;
    session.subscribed.insert(ViewerStream::Events);
    session.subscribed.insert(ViewerStream::Snapshot);
    let initial_time = server.world.state().time;
    let (mut writer, peer) = test_writer_pair();

    let progressed = server
        .sync_chain_linked_runtime(&mut session, &mut writer)
        .expect("chain sync should ignore zero-height bootstrap state");

    assert!(!progressed);
    assert_eq!(server.world.state().time, initial_time);
    assert_eq!(server.last_chain_committed_height, 0);
    assert!(server.latest_player_gameplay_feedback.is_none());
    assert!(read_response_line(&peer, Duration::from_millis(100)).is_none());
}
