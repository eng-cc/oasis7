use super::*;
use crate::simulator::{AgentDecision, ResourceOwner};
use crate::simulator::{
    DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION, DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION,
    ProviderExecutionMode,
};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

mod auth_actions;
mod auth_actions_collect_data;
mod auth_actions_feedback;
mod authoritative;
mod background_play;
mod chain_sync;
pub(super) use chain_sync::TestChainStatusServer;
mod chain_sync_feedback;
mod claim_runway;
mod industrial_progression;
mod prompt_control;
mod provider_settings;
mod snapshot_fallback;
mod snapshot_generated_world;
mod snapshot_micro_depot;
mod snapshot_progress;
mod snapshot_progress_empty_world;
mod snapshot_reprioritize;
#[path = "tests_support.rs"]
mod tests_support;
mod wait_resolution_quote;

use tests_support::*;

fn send_runtime_live_request(writer: &mut BufWriter<TcpStream>, request: &ViewerRequest) {
    serde_json::to_writer(&mut *writer, request).expect("write request");
    writer.write_all(b"\n").expect("write newline");
    writer.flush().expect("flush request");
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

fn seed_agent_chat_oc(server: &mut ViewerRuntimeLiveServer, agent_id: &str) {
    server
        .world
        .set_main_token_supply(crate::runtime::MainTokenSupplyState {
            total_supply: 1_000_000,
            circulating_supply: 1_000_000,
            ..crate::runtime::MainTokenSupplyState::default()
        });
    server
        .world
        .set_main_token_account_balance(agent_id, 1, 0)
        .expect("seed agent chat OC");
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

fn read_available_runtime_live_responses(
    peer: &TcpStream,
    timeout: Duration,
) -> Vec<ViewerResponse> {
    let stream = peer.try_clone().expect("clone test peer");
    stream
        .set_read_timeout(Some(timeout))
        .expect("set read timeout");
    let mut reader = BufReader::new(stream);
    let mut responses = Vec::new();
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim_end();
                if !trimmed.is_empty() {
                    responses.push(serde_json::from_str(trimmed).expect("decode response"));
                }
            }
            Err(err) => {
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) {
                    break;
                }
                panic!("read response line failed: {err}");
            }
        }
    }
    responses
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
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let bytes = match stream.read(&mut buffer) {
            Ok(bytes) => bytes,
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                if Instant::now() >= deadline {
                    panic!("timed out reading test http request chunk: {err}");
                }
                thread::sleep(Duration::from_millis(5));
                continue;
            }
            Err(err) => panic!("read test http request chunk: {err}"),
        };
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
            registration_grant: None,
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
fn runtime_live_default_snapshot_request_does_not_enable_ongoing_streams() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let mut session = RuntimeLiveSession::new();
    let (mut writer, peer) = test_writer_pair();

    server
        .handle_request(ViewerRequest::RequestSnapshot, &mut session, &mut writer)
        .expect("handle snapshot request");
    let responses = read_available_runtime_live_responses(&peer, Duration::from_millis(25));

    assert_eq!(
        responses.len(),
        2,
        "default request should emit one snapshot and recovery metadata only: {responses:?}"
    );
    assert!(matches!(responses[0], ViewerResponse::Snapshot { .. }));
    assert!(matches!(
        responses[1],
        ViewerResponse::AuthoritativeRecoveryAck { .. }
    ));
    assert!(session.uses_default_subscription());

    server
        .emit_background_play_snapshot(&mut session, &mut writer)
        .expect("emit background snapshot");
    let follow_up = read_available_runtime_live_responses(&peer, Duration::from_millis(25));
    assert!(
        follow_up.is_empty(),
        "default subscription must not become ongoing snapshot/events/metrics streaming: {follow_up:?}"
    );
}

#[test]
fn runtime_live_hello_omits_governed_rollback_when_no_durable_sink_exists() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let mut session = RuntimeLiveSession::new();
    let (mut writer, peer) = test_writer_pair();

    server
        .handle_request(
            ViewerRequest::Hello {
                version: VIEWER_PROTOCOL_VERSION,
                client: "red-capability-probe".to_string(),
            },
            &mut session,
            &mut writer,
        )
        .expect("handle hello");
    let responses = read_available_runtime_live_responses(&peer, Duration::from_millis(25));
    let capabilities = match responses.as_slice() {
        [ViewerResponse::HelloAck { capabilities, .. }] => capabilities,
        other => panic!("expected one hello ack, got {other:?}"),
    };
    assert!(
        !capabilities
            .iter()
            .any(|capability| capability
                == crate::viewer::protocol::GOVERNED_ROLLBACK_REPLAY_CAPABILITY),
        "server must not offer governed rollback when it cannot durably commit it"
    );
}

#[test]
fn runtime_live_status_unknown_fence_rejects_mutating_requests() {
    let recovery_dir = std::env::temp_dir().join(format!(
        "oasis7-status-unknown-fence-{}-{}",
        std::process::id(),
        crate::viewer::runtime_live::recovery_receipt::current_unix_time_ms()
    ));
    let generation_root = recovery_dir.join(".distfs-state/sidecar-generations");
    std::fs::create_dir_all(&generation_root).expect("generation root");
    std::fs::write(generation_root.join("index.json"), b"not-json").expect("corrupt index");
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    server.set_authoritative_recovery_dir_override(Some(recovery_dir.clone()));
    server.authoritative_recovery_write_fence = Some("expected-generation-hash".to_string());
    let world_before = server.world.snapshot();
    let mut session = RuntimeLiveSession::new();
    let (mut writer, _peer) = test_writer_pair();

    let error = server
        .handle_request(
            ViewerRequest::LiveControl {
                mode: crate::viewer::protocol::LiveControl::Step { count: 1 },
                request_id: Some(1),
            },
            &mut session,
            &mut writer,
        )
        .expect_err("status-unknown fence must reject writes");
    assert!(format!("{error:?}").contains("read-only"));
    assert_eq!(server.world.snapshot(), world_before);
    assert!(server.authoritative_recovery_write_fence.is_some());
    let _ = std::fs::remove_dir_all(recovery_dir);
}

#[test]
fn runtime_live_events_subscription_requests_recovery_metadata_without_initial_snapshot() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let mut session = RuntimeLiveSession::new();
    let (mut writer, peer) = test_writer_pair();

    server
        .handle_request(
            ViewerRequest::Subscribe {
                streams: vec![ViewerStream::Events],
                event_kinds: Vec::new(),
            },
            &mut session,
            &mut writer,
        )
        .expect("handle subscribe");
    server
        .handle_request(ViewerRequest::RequestSnapshot, &mut session, &mut writer)
        .expect("handle snapshot request");
    let responses = read_available_runtime_live_responses(&peer, Duration::from_millis(25));

    assert!(
        responses
            .iter()
            .any(|response| matches!(response, ViewerResponse::AuthoritativeRecoveryAck { .. })),
        "events subscription should get recovery metadata: {responses:?}"
    );
    assert!(
        !responses
            .iter()
            .any(|response| matches!(response, ViewerResponse::Snapshot { .. })),
        "events-only subscription must not receive an initial snapshot: {responses:?}"
    );
    assert!(session.explicitly_subscribed_to(ViewerStream::Events));
    assert!(!session.explicitly_subscribed_to(ViewerStream::Snapshot));
}

#[test]
fn runtime_live_agent_chat_echo_flushes_virtual_event_immediately_over_socket() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::remove_var(RUNTIME_AGENT_CHAT_ECHO_ENV);
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::remove_var(crate::simulator::ENV_LLM_MODEL);
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::remove_var(crate::simulator::ENV_LLM_BASE_URL);
    }
    // SAFETY: This test/setup code mutates process environment in a controlled scope.
    unsafe {
        oasis7::env_mut::remove_var(crate::simulator::ENV_LLM_API_KEY);
    }

    let listener = TcpListener::bind("127.0.0.1:0").expect("reserve port");
    let addr = listener.local_addr().expect("local addr");
    drop(listener);

    let server_addr = addr.to_string();
    thread::spawn(move || {
        let mut server = ViewerRuntimeLiveServer::new(
            ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
                .with_bind_addr(server_addr)
                .with_decision_mode(ViewerLiveDecisionMode::Llm)
                .with_agent_chat_echo_enabled(true),
        )
        .expect("create server");
        let agent_id = server
            .world
            .state()
            .agents
            .keys()
            .next()
            .cloned()
            .expect("seed agent");
        seed_agent_chat_oc(&mut server, agent_id.as_str());
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
            registration_grant: None,
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
                    } if event_agent_id == &agent_id && message == "[local-mock-receipt] 已收到消息；当前本地 mock provider 不生成真实 Agent 回复：hello runtime echo over socket"
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
fn runtime_background_play_throttles_full_snapshots() {
    let mut session = RuntimeLiveSession::new();
    session.playing = true;

    assert!(
        should_emit_runtime_advance_snapshot(&mut session, "play", false),
        "first background play step should emit a full snapshot"
    );
    assert!(
        !should_emit_runtime_advance_snapshot(&mut session, "play", false),
        "second immediate background play step should be snapshot-throttled"
    );
    assert!(
        should_emit_runtime_advance_snapshot(&mut session, "step", true),
        "manual step should still emit a full snapshot immediately"
    );
    session.playing = false;
    assert!(
        should_emit_runtime_advance_snapshot(&mut session, "play", false),
        "non-playing play control responses are not background auto-play and should not be throttled"
    );
}

#[test]
fn runtime_decision_failure_reason_includes_upstream_trace() {
    let trace = AgentDecisionTrace {
        agent_id: "agent-0".to_string(),
        time: 1,
        decision: AgentDecision::Wait,
        llm_input: None,
        llm_output: Some(
            serde_json::json!({
                "provider_error": {
                    "code": "provider_gateway_unreachable",
                    "retryable": true,
                },
                "upstream_trace": {
                    "stage": "decision_invocation",
                    "diagnostics": {
                        "status_code": 200,
                        "data_event_count": 2,
                    },
                },
            })
            .to_string(),
        ),
        llm_error: Some("provider_gateway_unreachable: upstream failed".to_string()),
        parse_error: None,
        llm_diagnostics: None,
        llm_effect_intents: Vec::new(),
        llm_effect_receipts: Vec::new(),
        llm_step_trace: Vec::new(),
        llm_prompt_section_trace: Vec::new(),
        llm_chat_messages: Vec::new(),
    };

    let reason = append_decision_upstream_trace(
        "provider_gateway_unreachable: upstream failed".to_string(),
        &trace,
    );

    assert!(reason.contains("upstream_trace="));
    assert!(reason.contains("\"data_event_count\":2"));
}

#[test]
fn runtime_decision_failure_reason_truncates_utf8_trace_safely() {
    let trace = AgentDecisionTrace {
        agent_id: "agent-0".to_string(),
        time: 1,
        decision: AgentDecision::Wait,
        llm_input: None,
        llm_output: Some(
            serde_json::json!({
                "provider_error": {
                    "code": "provider_gateway_unreachable",
                    "retryable": true,
                },
                "upstream_trace": {
                    "error_summary": "余额不足".repeat(500),
                },
            })
            .to_string(),
        ),
        llm_error: Some("provider_gateway_unreachable: upstream failed".to_string()),
        parse_error: None,
        llm_diagnostics: None,
        llm_effect_intents: Vec::new(),
        llm_effect_receipts: Vec::new(),
        llm_step_trace: Vec::new(),
        llm_prompt_section_trace: Vec::new(),
        llm_chat_messages: Vec::new(),
    };

    let reason = append_decision_upstream_trace(
        "provider_gateway_unreachable: upstream failed".to_string(),
        &trace,
    );

    assert!(reason.contains("upstream_trace="));
    assert!(reason.ends_with("..."));
}

#[test]
fn runtime_decision_trace_reads_provider_retryable_flag() {
    let trace = AgentDecisionTrace {
        agent_id: "agent-0".to_string(),
        time: 1,
        decision: AgentDecision::Wait,
        llm_input: None,
        llm_output: Some(
            serde_json::json!({
                "provider_error": {
                    "code": "provider_unauthorized",
                    "retryable": false,
                },
                "upstream_trace": {
                    "stage": "decision_invocation",
                },
            })
            .to_string(),
        ),
        llm_error: Some("provider_unauthorized: no token".to_string()),
        parse_error: None,
        llm_diagnostics: None,
        llm_effect_intents: Vec::new(),
        llm_effect_receipts: Vec::new(),
        llm_step_trace: Vec::new(),
        llm_prompt_section_trace: Vec::new(),
        llm_chat_messages: Vec::new(),
    };

    assert_eq!(decision_trace_provider_error_retryable(&trace), Some(false));
}

#[test]
fn runtime_auto_play_uses_shared_server_gate_across_sessions() {
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal).with_auto_play_on_connect(true),
    )
    .expect("runtime server");
    let mut first = RuntimeLiveSession::new();
    let mut second = RuntimeLiveSession::new();

    server.enable_auto_play_for_session_if_available(&mut first);
    assert!(first.playing);
    assert!(server.should_advance_auto_play_step());
    assert!(
        !server.should_advance_auto_play_step(),
        "a second session should not advance the same server-level auto-play tick"
    );

    server.enable_auto_play_for_session_if_available(&mut second);
    assert!(second.playing);
    assert!(
        !server.should_advance_auto_play_step(),
        "joining sessions share the same background play gate instead of becoming owners"
    );

    server.next_auto_play_step_at = Some(Instant::now() - Duration::from_millis(1));
    assert!(server.should_advance_auto_play_step());
}

#[test]
fn runtime_auto_play_pause_and_resume_are_global() {
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal).with_auto_play_on_connect(true),
    )
    .expect("runtime server");
    let mut first = RuntimeLiveSession::new();
    let mut second = RuntimeLiveSession::new();

    server.enable_auto_play_for_session_if_available(&mut first);
    assert!(first.playing);
    server.pause_auto_play(&mut first);
    assert!(!first.playing);
    assert!(!server.should_advance_auto_play_step());

    server.enable_auto_play_for_session_if_available(&mut second);
    assert!(
        !second.playing,
        "new sessions should respect a global pause until the user resumes live play"
    );

    server.resume_auto_play(&mut second);
    assert!(second.playing);
    assert!(server.should_advance_auto_play_step());
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
