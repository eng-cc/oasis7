use super::*;
use crate::simulator::ResourceOwner;
use crate::simulator::{
    ProviderExecutionMode, DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION,
    DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION,
};
use ed25519_dalek::SigningKey;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

mod auth_actions;
mod authoritative;
mod prompt_control;
mod snapshot_progress;

const VIEWER_AGENT_DECISION_SOURCE_ENV: &str = "OASIS7_AGENT_DECISION_SOURCE";
const VIEWER_AGENT_PROVIDER_BACKEND_ENV: &str = "OASIS7_AGENT_PROVIDER_BACKEND";
const VIEWER_AGENT_PROVIDER_CONTRACT_ENV: &str = "OASIS7_AGENT_PROVIDER_CONTRACT";
const VIEWER_AGENT_PROVIDER_TRANSPORT_ENV: &str = "OASIS7_AGENT_PROVIDER_TRANSPORT";
const VIEWER_AGENT_PROVIDER_URL_ENV: &str = "OASIS7_AGENT_PROVIDER_URL";
const VIEWER_AGENT_PROVIDER_AUTH_TOKEN_ENV: &str = "OASIS7_AGENT_PROVIDER_AUTH_TOKEN";
const VIEWER_AGENT_PROVIDER_CONNECT_TIMEOUT_MS_ENV: &str =
    "OASIS7_AGENT_PROVIDER_CONNECT_TIMEOUT_MS";
const VIEWER_AGENT_PROVIDER_PROFILE_ENV: &str = "OASIS7_AGENT_PROVIDER_PROFILE";
const VIEWER_AGENT_EXECUTION_LANE_ENV: &str = "OASIS7_AGENT_EXECUTION_LANE";
const VIEWER_AGENT_PROVIDER_MODE_ENV: &str = "OASIS7_AGENT_PROVIDER_MODE";
const VIEWER_OPENCLAW_BASE_URL_ENV: &str = "OASIS7_OPENCLAW_BASE_URL";
const VIEWER_OPENCLAW_AUTH_TOKEN_ENV: &str = "OASIS7_OPENCLAW_AUTH_TOKEN";
const VIEWER_OPENCLAW_CONNECT_TIMEOUT_MS_ENV: &str = "OASIS7_OPENCLAW_CONNECT_TIMEOUT_MS";
const VIEWER_OPENCLAW_AGENT_PROFILE_ENV: &str = "OASIS7_OPENCLAW_AGENT_PROFILE";
const VIEWER_OPENCLAW_EXECUTION_MODE_ENV: &str = "OASIS7_OPENCLAW_EXECUTION_MODE";
const RUNTIME_AGENT_CHAT_ECHO_ENV: &str = "OASIS7_RUNTIME_AGENT_CHAT_ECHO";
const HOSTED_STRONG_AUTH_GRANT_PUBLIC_KEY_ENV: &str = "OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY";

fn test_signer(seed: u8) -> (String, String) {
    let private_key = [seed; 32];
    let signing_key = SigningKey::from_bytes(&private_key);
    (
        hex::encode(signing_key.verifying_key().to_bytes()),
        hex::encode(private_key),
    )
}

fn lock_test_llm_env() -> std::sync::MutexGuard<'static, ()> {
    let guard = runtime_openclaw_env_lock().lock().expect("env lock");
    clear_runtime_openclaw_env();
    std::env::set_var(crate::simulator::ENV_LLM_MODEL, "gpt-4o-mini");
    std::env::set_var(
        crate::simulator::ENV_LLM_BASE_URL,
        "https://api.openai.com/v1",
    );
    std::env::set_var(crate::simulator::ENV_LLM_API_KEY, "test-api-key");
    guard
}

fn clear_runtime_openclaw_env() {
    let removed_old_brand_envs = [
        removed_old_brand_runtime_live_env("AGENT_DECISION_SOURCE"),
        removed_old_brand_runtime_live_env("AGENT_PROVIDER_BACKEND"),
        removed_old_brand_runtime_live_env("AGENT_PROVIDER_CONTRACT"),
        removed_old_brand_runtime_live_env("AGENT_PROVIDER_TRANSPORT"),
        removed_old_brand_runtime_live_env("AGENT_PROVIDER_URL"),
        removed_old_brand_runtime_live_env("AGENT_PROVIDER_AUTH_TOKEN"),
        removed_old_brand_runtime_live_env("AGENT_PROVIDER_CONNECT_TIMEOUT_MS"),
        removed_old_brand_runtime_live_env("AGENT_PROVIDER_PROFILE"),
        removed_old_brand_runtime_live_env("AGENT_EXECUTION_LANE"),
        removed_old_brand_runtime_live_env("AGENT_PROVIDER_MODE"),
        removed_old_brand_runtime_live_env("OPENCLAW_BASE_URL"),
        removed_old_brand_runtime_live_env("OPENCLAW_AUTH_TOKEN"),
        removed_old_brand_runtime_live_env("OPENCLAW_CONNECT_TIMEOUT_MS"),
        removed_old_brand_runtime_live_env("OPENCLAW_AGENT_PROFILE"),
        removed_old_brand_runtime_live_env("OPENCLAW_EXECUTION_MODE"),
        removed_old_brand_runtime_live_env("RUNTIME_AGENT_CHAT_ECHO"),
    ];
    for env_name in [
        VIEWER_AGENT_DECISION_SOURCE_ENV,
        VIEWER_AGENT_PROVIDER_BACKEND_ENV,
        VIEWER_AGENT_PROVIDER_CONTRACT_ENV,
        VIEWER_AGENT_PROVIDER_TRANSPORT_ENV,
        VIEWER_AGENT_PROVIDER_URL_ENV,
        VIEWER_AGENT_PROVIDER_AUTH_TOKEN_ENV,
        VIEWER_AGENT_PROVIDER_CONNECT_TIMEOUT_MS_ENV,
        VIEWER_AGENT_PROVIDER_PROFILE_ENV,
        VIEWER_AGENT_EXECUTION_LANE_ENV,
        VIEWER_AGENT_PROVIDER_MODE_ENV,
        VIEWER_OPENCLAW_BASE_URL_ENV,
        VIEWER_OPENCLAW_AUTH_TOKEN_ENV,
        VIEWER_OPENCLAW_CONNECT_TIMEOUT_MS_ENV,
        VIEWER_OPENCLAW_AGENT_PROFILE_ENV,
        VIEWER_OPENCLAW_EXECUTION_MODE_ENV,
        RUNTIME_AGENT_CHAT_ECHO_ENV,
    ] {
        std::env::remove_var(env_name);
    }
    for env_name in removed_old_brand_envs {
        std::env::remove_var(env_name);
    }
}

fn runtime_openclaw_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn clear_hosted_strong_auth_env() {
    std::env::remove_var(HOSTED_STRONG_AUTH_GRANT_PUBLIC_KEY_ENV);
}

fn hosted_strong_auth_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn lock_test_hosted_strong_auth_env() -> std::sync::MutexGuard<'static, ()> {
    let guard = hosted_strong_auth_env_lock().lock().expect("env lock");
    clear_hosted_strong_auth_env();
    guard
}

fn test_now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn connect_runtime_live_client(addr: &str) -> (BufReader<TcpStream>, BufWriter<TcpStream>) {
    let stream = TcpStream::connect(addr).expect("connect runtime live");
    stream.set_nodelay(true).expect("set_nodelay");
    stream
        .set_read_timeout(Some(Duration::from_millis(500)))
        .expect("set_read_timeout");
    stream
        .set_write_timeout(Some(Duration::from_millis(500)))
        .expect("set_write_timeout");
    let reader_stream = stream.try_clone().expect("clone stream");
    let mut reader = BufReader::new(reader_stream);
    let mut writer = BufWriter::new(stream);
    send_runtime_live_request(
        &mut writer,
        &ViewerRequest::Hello {
            client: "runtime-live-test".to_string(),
            version: VIEWER_PROTOCOL_VERSION,
        },
    );
    read_runtime_live_hello_ack(&mut reader);
    (reader, writer)
}

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

fn wait_for_runtime_live_server(addr: &str) {
    for _ in 0..50 {
        if TcpStream::connect(addr).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("runtime live server did not start listening at {addr}");
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

fn removed_old_brand_runtime_live_env(suffix: &str) -> String {
    ["AGENT", "WORLD", suffix].join("_")
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
    let _guard = runtime_openclaw_env_lock().lock().expect("env lock");
    clear_runtime_openclaw_env();
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
fn provider_settings_from_env_defaults_to_none() {
    let _guard = runtime_openclaw_env_lock().lock().expect("env lock");
    clear_runtime_openclaw_env();
    let settings =
        super::control_plane::runtime_provider_settings_from_env().expect("settings parse");
    assert_eq!(settings, None);
}

#[test]
fn provider_settings_from_env_parses_profile_and_timeout() {
    let _guard = runtime_openclaw_env_lock().lock().expect("env lock");
    clear_runtime_openclaw_env();
    std::env::set_var(VIEWER_AGENT_DECISION_SOURCE_ENV, "provider_backed");
    std::env::set_var(VIEWER_AGENT_PROVIDER_BACKEND_ENV, "openclaw");
    std::env::set_var(VIEWER_AGENT_PROVIDER_CONTRACT_ENV, "worldsim_provider_v1");
    std::env::set_var(VIEWER_AGENT_PROVIDER_TRANSPORT_ENV, "loopback_http");
    std::env::set_var(VIEWER_AGENT_PROVIDER_URL_ENV, "http://127.0.0.1:5841");
    std::env::set_var(VIEWER_AGENT_PROVIDER_CONNECT_TIMEOUT_MS_ENV, "4200");
    std::env::set_var(VIEWER_AGENT_PROVIDER_PROFILE_ENV, "oasis7_p0_low_freq_npc");
    std::env::set_var(VIEWER_AGENT_EXECUTION_LANE_ENV, "player_parity");
    std::env::set_var(VIEWER_AGENT_PROVIDER_AUTH_TOKEN_ENV, "secret-token");
    let settings = super::control_plane::runtime_provider_settings_from_env()
        .expect("settings parse")
        .expect("openclaw settings");
    assert_eq!(settings.requested_provider_mode, "provider_backed");
    assert_eq!(settings.base_url, "http://127.0.0.1:5841");
    assert_eq!(settings.connect_timeout_ms, 4200);
    assert_eq!(settings.agent_profile, "oasis7_p0_low_freq_npc");
    assert_eq!(settings.execution_mode, ProviderExecutionMode::PlayerParity);
    assert_eq!(settings.auth_token.as_deref(), Some("secret-token"));
    assert_eq!(settings.fallback_reason, None);
    clear_runtime_openclaw_env();
}

#[test]
fn provider_settings_from_env_rejects_removed_old_brand_prefix() {
    let _guard = runtime_openclaw_env_lock().lock().expect("env lock");
    clear_runtime_openclaw_env();
    std::env::set_var(
        removed_old_brand_runtime_live_env("AGENT_PROVIDER_MODE"),
        "provider_loopback_http",
    );
    std::env::set_var(
        removed_old_brand_runtime_live_env("OPENCLAW_BASE_URL"),
        "http://127.0.0.1:5842",
    );
    std::env::set_var(
        removed_old_brand_runtime_live_env("OPENCLAW_CONNECT_TIMEOUT_MS"),
        "4300",
    );
    std::env::set_var(
        removed_old_brand_runtime_live_env("OPENCLAW_AGENT_PROFILE"),
        "oasis7_p0_low_freq_npc",
    );
    std::env::set_var(
        removed_old_brand_runtime_live_env("OPENCLAW_EXECUTION_MODE"),
        "player_parity",
    );
    std::env::set_var(
        removed_old_brand_runtime_live_env("OPENCLAW_AUTH_TOKEN"),
        "removed-old-brand-token",
    );

    let settings =
        super::control_plane::runtime_provider_settings_from_env().expect("settings parse");
    assert_eq!(settings, None);
    clear_runtime_openclaw_env();
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

    let move_target = GeoPos::new(10.0, 20.0, 30.0);
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
fn runtime_simulator_action_mapping_keeps_unmapped_actions_as_none() {
    let server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let build_factory = crate::simulator::Action::BuildFactory {
        owner: ResourceOwner::Agent {
            agent_id: "agent-1".to_string(),
        },
        location_id: "loc-1".to_string(),
        factory_id: "factory-1".to_string(),
        factory_kind: "smelter".to_string(),
    };
    assert!(control_plane::simulator_action_to_runtime(&build_factory, &server.world).is_none());

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
