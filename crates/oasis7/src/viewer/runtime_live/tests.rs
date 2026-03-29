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
use std::time::Duration;

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
        removed_old_brand_runtime_live_env("AGENT_PROVIDER_MODE"),
        removed_old_brand_runtime_live_env("OPENCLAW_BASE_URL"),
        removed_old_brand_runtime_live_env("OPENCLAW_AUTH_TOKEN"),
        removed_old_brand_runtime_live_env("OPENCLAW_CONNECT_TIMEOUT_MS"),
        removed_old_brand_runtime_live_env("OPENCLAW_AGENT_PROFILE"),
        removed_old_brand_runtime_live_env("OPENCLAW_EXECUTION_MODE"),
        removed_old_brand_runtime_live_env("RUNTIME_AGENT_CHAT_ECHO"),
    ];
    for env_name in [
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

fn wait_for_runtime_live_server(addr: &str) {
    for _ in 0..50 {
        if TcpStream::connect(addr).is_ok() {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("runtime live server did not start listening at {addr}");
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
fn openclaw_settings_from_env_defaults_to_none() {
    let _guard = runtime_openclaw_env_lock().lock().expect("env lock");
    clear_runtime_openclaw_env();
    let settings =
        super::control_plane::runtime_openclaw_settings_from_env().expect("settings parse");
    assert_eq!(settings, None);
}

#[test]
fn openclaw_settings_from_env_parses_profile_and_timeout() {
    let _guard = runtime_openclaw_env_lock().lock().expect("env lock");
    clear_runtime_openclaw_env();
    std::env::set_var(VIEWER_AGENT_PROVIDER_MODE_ENV, "openclaw_local_http");
    std::env::set_var(VIEWER_OPENCLAW_BASE_URL_ENV, "http://127.0.0.1:5841");
    std::env::set_var(VIEWER_OPENCLAW_CONNECT_TIMEOUT_MS_ENV, "4200");
    std::env::set_var(VIEWER_OPENCLAW_AGENT_PROFILE_ENV, "oasis7_p0_low_freq_npc");
    std::env::set_var(VIEWER_OPENCLAW_EXECUTION_MODE_ENV, "player_parity");
    std::env::set_var(VIEWER_OPENCLAW_AUTH_TOKEN_ENV, "secret-token");
    let settings = super::control_plane::runtime_openclaw_settings_from_env()
        .expect("settings parse")
        .expect("openclaw settings");
    assert_eq!(settings.base_url, "http://127.0.0.1:5841");
    assert_eq!(settings.connect_timeout_ms, 4200);
    assert_eq!(settings.agent_profile, "oasis7_p0_low_freq_npc");
    assert_eq!(settings.execution_mode, ProviderExecutionMode::PlayerParity);
    assert_eq!(settings.auth_token.as_deref(), Some("secret-token"));
    clear_runtime_openclaw_env();
}

#[test]
fn openclaw_settings_from_env_rejects_removed_old_brand_prefix() {
    let _guard = runtime_openclaw_env_lock().lock().expect("env lock");
    clear_runtime_openclaw_env();
    std::env::set_var(
        removed_old_brand_runtime_live_env("AGENT_PROVIDER_MODE"),
        "openclaw_local_http",
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
        super::control_plane::runtime_openclaw_settings_from_env().expect("settings parse");
    assert_eq!(settings, None);
    clear_runtime_openclaw_env();
}

fn removed_old_brand_runtime_live_env(suffix: &str) -> String {
    ["AGENT", "WORLD", suffix].join("_")
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

#[test]
fn runtime_prompt_control_script_mode_requires_llm_mode() {
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Script),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(11);
    let request = signed_prompt_control_apply_request(
        crate::viewer::PromptControlApplyRequest {
            agent_id: agent_id.clone(),
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
            strong_auth_grant: None,
            expected_version: Some(0),
            updated_by: None,
            system_prompt_override: Some(Some("system".to_string())),
            short_term_goal_override: None,
            long_term_goal_override: None,
        },
        crate::viewer::PromptControlAuthIntent::Apply,
        1,
        public_key.as_str(),
        private_key.as_str(),
    );
    let err = server
        .handle_prompt_control(crate::viewer::PromptControlCommand::Apply { request })
        .expect_err("script mode should reject prompt control");
    assert_eq!(err.code, "llm_mode_required");
    assert!(server.llm_sidecar.prompt_profiles.is_empty());
}

#[test]
fn runtime_prompt_control_hosted_public_join_requires_strong_auth() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm)
            .with_hosted_public_join_mode(true),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(27);
    let request = signed_prompt_control_apply_request(
        crate::viewer::PromptControlApplyRequest {
            agent_id: agent_id.clone(),
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
            strong_auth_grant: None,
            expected_version: Some(0),
            updated_by: None,
            system_prompt_override: Some(Some("system".to_string())),
            short_term_goal_override: None,
            long_term_goal_override: None,
        },
        crate::viewer::PromptControlAuthIntent::Apply,
        27,
        public_key.as_str(),
        private_key.as_str(),
    );
    let err = server
        .handle_prompt_control(crate::viewer::PromptControlCommand::Apply { request })
        .expect_err("hosted public join should require strong auth");
    assert_eq!(err.code, "strong_auth_required");
    assert!(err
        .message
        .contains("prompt_control requires hosted strong auth"));
    assert!(server.llm_sidecar.prompt_profiles.is_empty());
}

#[test]
fn runtime_prompt_control_hosted_public_join_accepts_valid_backend_grant() {
    let _llm_guard = lock_test_llm_env();
    let _strong_auth_guard = lock_test_hosted_strong_auth_env();
    let (backend_public_key, backend_private_key) = test_signer(28);
    std::env::set_var(
        HOSTED_STRONG_AUTH_GRANT_PUBLIC_KEY_ENV,
        backend_public_key.as_str(),
    );

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm)
            .with_hosted_public_join_mode(true),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (player_public_key, player_private_key) = test_signer(29);
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        1,
        player_public_key.as_str(),
        player_private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );
    let mut request = signed_prompt_control_apply_request(
        crate::viewer::PromptControlApplyRequest {
            agent_id: agent_id.clone(),
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
            strong_auth_grant: None,
            expected_version: Some(0),
            updated_by: Some("player-a".to_string()),
            system_prompt_override: Some(Some("system".to_string())),
            short_term_goal_override: Some(Some("goal".to_string())),
            long_term_goal_override: None,
        },
        crate::viewer::PromptControlAuthIntent::Apply,
        2,
        player_public_key.as_str(),
        player_private_key.as_str(),
    );
    let issued_at_unix_ms = test_now_unix_ms().saturating_sub(1_000);
    let grant = crate::viewer::sign_hosted_prompt_control_strong_auth_grant(
        "prompt_control_apply",
        "player-a",
        player_public_key.as_str(),
        agent_id.as_str(),
        issued_at_unix_ms,
        issued_at_unix_ms.saturating_add(60_000),
        backend_public_key.as_str(),
        backend_private_key.as_str(),
    )
    .expect("backend strong-auth grant");
    request.strong_auth_grant = Some(grant);

    let ack = server
        .handle_prompt_control(crate::viewer::PromptControlCommand::Apply { request })
        .expect("hosted apply with backend grant");
    assert_eq!(ack.version, 1);
    clear_hosted_strong_auth_env();
}

#[test]
fn runtime_prompt_control_hosted_public_join_rejects_expired_backend_grant() {
    let _llm_guard = lock_test_llm_env();
    let _strong_auth_guard = lock_test_hosted_strong_auth_env();
    let (backend_public_key, backend_private_key) = test_signer(30);
    std::env::set_var(
        HOSTED_STRONG_AUTH_GRANT_PUBLIC_KEY_ENV,
        backend_public_key.as_str(),
    );

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm)
            .with_hosted_public_join_mode(true),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (player_public_key, player_private_key) = test_signer(31);
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        1,
        player_public_key.as_str(),
        player_private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );
    let mut request = signed_prompt_control_apply_request(
        crate::viewer::PromptControlApplyRequest {
            agent_id: agent_id.clone(),
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
            strong_auth_grant: None,
            expected_version: Some(0),
            updated_by: None,
            system_prompt_override: Some(Some("system".to_string())),
            short_term_goal_override: None,
            long_term_goal_override: None,
        },
        crate::viewer::PromptControlAuthIntent::Apply,
        2,
        player_public_key.as_str(),
        player_private_key.as_str(),
    );
    let issued_at_unix_ms = test_now_unix_ms().saturating_sub(10_000);
    let grant = crate::viewer::sign_hosted_prompt_control_strong_auth_grant(
        "prompt_control_apply",
        "player-a",
        player_public_key.as_str(),
        agent_id.as_str(),
        issued_at_unix_ms,
        issued_at_unix_ms.saturating_add(1_000),
        backend_public_key.as_str(),
        backend_private_key.as_str(),
    )
    .expect("backend strong-auth grant");
    request.strong_auth_grant = Some(grant);

    let err = server
        .handle_prompt_control(crate::viewer::PromptControlCommand::Apply { request })
        .expect_err("expired hosted strong-auth grant must fail");
    assert_eq!(err.code, "strong_auth_grant_invalid");
    assert!(err.message.contains("expired"));
    clear_hosted_strong_auth_env();
}

#[test]
fn runtime_prompt_control_hosted_public_join_rejects_replayed_auth_nonce_even_with_valid_grant() {
    let _llm_guard = lock_test_llm_env();
    let _strong_auth_guard = lock_test_hosted_strong_auth_env();
    let (backend_public_key, backend_private_key) = test_signer(32);
    std::env::set_var(
        HOSTED_STRONG_AUTH_GRANT_PUBLIC_KEY_ENV,
        backend_public_key.as_str(),
    );

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm)
            .with_hosted_public_join_mode(true),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (player_public_key, player_private_key) = test_signer(33);
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        1,
        player_public_key.as_str(),
        player_private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );

    let issued_at_unix_ms = test_now_unix_ms().saturating_sub(1_000);
    let grant = crate::viewer::sign_hosted_prompt_control_strong_auth_grant(
        "prompt_control_apply",
        "player-a",
        player_public_key.as_str(),
        agent_id.as_str(),
        issued_at_unix_ms,
        issued_at_unix_ms.saturating_add(60_000),
        backend_public_key.as_str(),
        backend_private_key.as_str(),
    )
    .expect("backend strong-auth grant");
    let build_request = || {
        let request = signed_prompt_control_apply_request(
            crate::viewer::PromptControlApplyRequest {
                agent_id: agent_id.clone(),
                player_id: "player-a".to_string(),
                public_key: None,
                auth: None,
                strong_auth_grant: Some(grant.clone()),
                expected_version: Some(0),
                updated_by: Some("player-a".to_string()),
                system_prompt_override: Some(Some("system".to_string())),
                short_term_goal_override: Some(Some("goal".to_string())),
                long_term_goal_override: None,
            },
            crate::viewer::PromptControlAuthIntent::Apply,
            2,
            player_public_key.as_str(),
            player_private_key.as_str(),
        );
        request
    };

    let first_ack = server
        .handle_prompt_control(crate::viewer::PromptControlCommand::Apply {
            request: build_request(),
        })
        .expect("first hosted apply should succeed");
    assert_eq!(first_ack.version, 1);

    let replay_err = server
        .handle_prompt_control(crate::viewer::PromptControlCommand::Apply {
            request: build_request(),
        })
        .expect_err("replayed nonce should fail even with valid hosted grant");
    assert_eq!(replay_err.code, "auth_nonce_replay");
    clear_hosted_strong_auth_env();
}

#[test]
fn runtime_prompt_control_hosted_public_join_rejects_revoked_session_even_with_valid_grant() {
    let _llm_guard = lock_test_llm_env();
    let _strong_auth_guard = lock_test_hosted_strong_auth_env();
    let (backend_public_key, backend_private_key) = test_signer(34);
    std::env::set_var(
        HOSTED_STRONG_AUTH_GRANT_PUBLIC_KEY_ENV,
        backend_public_key.as_str(),
    );

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm)
            .with_hosted_public_join_mode(true),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (player_public_key, player_private_key) = test_signer(35);
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        1,
        player_public_key.as_str(),
        player_private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );

    let issued_at_unix_ms = test_now_unix_ms().saturating_sub(1_000);
    let grant = crate::viewer::sign_hosted_prompt_control_strong_auth_grant(
        "prompt_control_apply",
        "player-a",
        player_public_key.as_str(),
        agent_id.as_str(),
        issued_at_unix_ms,
        issued_at_unix_ms.saturating_add(60_000),
        backend_public_key.as_str(),
        backend_private_key.as_str(),
    )
    .expect("backend strong-auth grant");

    let _ = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RevokeSession {
            request: AuthoritativeSessionRevokeRequest {
                player_id: "player-a".to_string(),
                session_pubkey: Some(player_public_key.clone()),
                revoke_reason: "abuse-drill".to_string(),
                revoked_by: Some("qa".to_string()),
            },
        })
        .expect("revoke session");

    let request = signed_prompt_control_apply_request(
        crate::viewer::PromptControlApplyRequest {
            agent_id: agent_id.clone(),
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
            strong_auth_grant: Some(grant),
            expected_version: Some(0),
            updated_by: Some("player-a".to_string()),
            system_prompt_override: Some(Some("system".to_string())),
            short_term_goal_override: Some(Some("goal".to_string())),
            long_term_goal_override: None,
        },
        crate::viewer::PromptControlAuthIntent::Apply,
        2,
        player_public_key.as_str(),
        player_private_key.as_str(),
    );

    let err = server
        .handle_prompt_control(crate::viewer::PromptControlCommand::Apply { request })
        .expect_err("revoked session should fail even with valid hosted grant");
    assert_eq!(err.code, "session_revoked");
    clear_hosted_strong_auth_env();
}

#[test]
fn runtime_prompt_control_openclaw_mode_reports_unsupported() {
    let _guard = runtime_openclaw_env_lock().lock().expect("env lock");
    clear_runtime_openclaw_env();
    std::env::set_var(VIEWER_AGENT_PROVIDER_MODE_ENV, "openclaw_local_http");
    std::env::set_var(VIEWER_OPENCLAW_BASE_URL_ENV, "http://127.0.0.1:5841");
    std::env::set_var(VIEWER_OPENCLAW_AGENT_PROFILE_ENV, "oasis7_p0_low_freq_npc");
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(31);
    let request = signed_prompt_control_apply_request(
        crate::viewer::PromptControlApplyRequest {
            agent_id: agent_id.clone(),
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
            strong_auth_grant: None,
            expected_version: Some(0),
            updated_by: None,
            system_prompt_override: Some(Some("system".to_string())),
            short_term_goal_override: None,
            long_term_goal_override: None,
        },
        crate::viewer::PromptControlAuthIntent::Apply,
        31,
        public_key.as_str(),
        private_key.as_str(),
    );
    let err = server
        .handle_prompt_control(crate::viewer::PromptControlCommand::Apply { request })
        .expect_err("openclaw mode should reject prompt control");
    assert_eq!(err.code, "agent_provider_prompt_control_unsupported");
    clear_runtime_openclaw_env();
}

#[test]
fn runtime_prompt_control_apply_updates_snapshot_and_bindings() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::TwoBases)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(12);
    let request = signed_prompt_control_apply_request(
        crate::viewer::PromptControlApplyRequest {
            agent_id: agent_id.clone(),
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
            strong_auth_grant: None,
            expected_version: Some(0),
            updated_by: None,
            system_prompt_override: Some(Some("system".to_string())),
            short_term_goal_override: None,
            long_term_goal_override: None,
        },
        crate::viewer::PromptControlAuthIntent::Apply,
        2,
        public_key.as_str(),
        private_key.as_str(),
    );
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        1,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );

    let ack = server
        .handle_prompt_control(crate::viewer::PromptControlCommand::Apply { request })
        .expect("llm mode apply");
    assert_eq!(ack.version, 1);
    let snapshot = server.compat_snapshot();
    let profile = snapshot
        .model
        .agent_prompt_profiles
        .get(agent_id.as_str())
        .expect("profile in snapshot");
    assert_eq!(profile.version, 1);
    assert_eq!(
        snapshot
            .model
            .agent_player_bindings
            .get(agent_id.as_str())
            .map(String::as_str),
        Some("player-a")
    );
    assert_eq!(
        snapshot
            .model
            .player_auth_last_nonce
            .get("player-a")
            .copied(),
        Some(2)
    );
}

#[test]
fn runtime_agent_chat_script_mode_requires_llm_mode() {
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Script),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let err = server
        .handle_agent_chat(crate::viewer::AgentChatRequest {
            agent_id,
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello".to_string(),
            intent_tick: None,
            intent_seq: None,
        })
        .expect_err("script mode should reject chat");
    assert_eq!(err.code, "llm_mode_required");
}

#[test]
fn runtime_openclaw_compat_snapshot_exposes_agent_execution_debug_contexts() {
    let _guard = runtime_openclaw_env_lock().lock().expect("env lock");
    clear_runtime_openclaw_env();
    std::env::set_var(VIEWER_AGENT_PROVIDER_MODE_ENV, "openclaw_local_http");
    std::env::set_var(VIEWER_OPENCLAW_BASE_URL_ENV, "http://127.0.0.1:5841");
    std::env::set_var(VIEWER_OPENCLAW_AGENT_PROFILE_ENV, "oasis7_p0_low_freq_npc");
    std::env::set_var(VIEWER_OPENCLAW_EXECUTION_MODE_ENV, "player_parity");
    let server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");

    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let snapshot = server.compat_snapshot();
    let context = snapshot
        .model
        .agent_execution_debug_contexts
        .get(agent_id.as_str())
        .expect("debug context in snapshot");
    assert_eq!(
        context.provider_mode.as_deref(),
        Some("openclaw_local_http")
    );
    assert_eq!(context.execution_mode.as_deref(), Some("player_parity"));
    assert_eq!(
        context.observation_schema_version.as_deref(),
        Some(DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION)
    );
    assert_eq!(
        context.action_schema_version.as_deref(),
        Some(DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION)
    );
    assert_eq!(context.environment_class.as_deref(), Some("runtime_live"));
    assert_eq!(
        context.agent_profile.as_deref(),
        Some("oasis7_p0_low_freq_npc")
    );
    clear_runtime_openclaw_env();
}

#[test]
fn compat_snapshot_exposes_player_gameplay_snapshot() {
    let server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let snapshot = server.compat_snapshot();
    let gameplay = snapshot
        .player_gameplay
        .as_ref()
        .expect("player gameplay snapshot");
    assert_eq!(
        gameplay.stage_id,
        crate::simulator::PlayerGameplayStageId::FirstSessionLoop
    );
    assert_eq!(
        gameplay.goal_id,
        "first_session_loop.create_first_world_feedback"
    );
    assert_eq!(
        gameplay.available_actions[0].protocol_action,
        "request_snapshot"
    );
    if super::player_gameplay::supports_runtime_gameplay_actions() {
        assert!(gameplay
            .available_actions
            .iter()
            .any(|action| action.action_id == "build_factory_smelter_mk1"));
    }
    assert!(!gameplay
        .available_actions
        .iter()
        .any(|action| action.action_id == "chat_first_agent"));
    assert!(gameplay.recent_feedback.is_none());
}

#[test]
fn compat_snapshot_exposes_player_agent_claim_overview() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let primary_agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("primary agent");

    server
        .world
        .set_governance_execution_policy(crate::runtime::GovernanceExecutionPolicy {
            epoch_length_ticks: 1,
            ..crate::runtime::GovernanceExecutionPolicy::default()
        })
        .expect("set governance policy");
    server
        .world
        .set_agent_reputation_score(primary_agent_id.as_str(), 0)
        .expect("set reputation");
    server
        .world
        .set_main_token_supply(crate::runtime::MainTokenSupplyState {
            total_supply: 1_000,
            circulating_supply: 1_000,
            ..crate::runtime::MainTokenSupplyState::default()
        });
    server
        .world
        .set_main_token_account_balance(primary_agent_id.as_str(), 1_000, 0)
        .expect("seed main token balance");
    server
        .world
        .submit_action(crate::runtime::Action::RegisterAgent {
            agent_id: "agent-claim-target".to_string(),
            pos: crate::geometry::GeoPos::new(0.0, 0.0, 0.0),
        });
    server.world.step().expect("register claim target");
    server
        .world
        .submit_action(crate::runtime::Action::ClaimAgent {
            claimer_agent_id: primary_agent_id.clone(),
            target_agent_id: "agent-claim-target".to_string(),
        });
    server.world.step().expect("claim target");
    server
        .world
        .submit_action(crate::runtime::Action::ReleaseAgentClaim {
            claimer_agent_id: primary_agent_id.clone(),
            target_agent_id: "agent-claim-target".to_string(),
        });
    server.world.step().expect("request release");

    let snapshot = server.compat_snapshot();
    let claim = snapshot
        .player_gameplay
        .as_ref()
        .and_then(|gameplay| gameplay.agent_claim.as_ref())
        .expect("player agent claim snapshot");
    assert_eq!(claim.claimer_agent_id, primary_agent_id);
    assert_eq!(claim.claim_cap, 1);
    assert_eq!(claim.owned_claim_count, 1);
    assert_eq!(claim.current_epoch, snapshot.time);
    assert_eq!(claim.restricted_starter_claim_balance, 0);
    assert_eq!(claim.slot_1_eligible_claim_balance, 650);

    let quote = claim.next_claim_quote.as_ref().expect("next claim quote");
    assert_eq!(quote.transferable_liquid_balance, 650);
    assert_eq!(quote.restricted_starter_claim_balance, 0);
    assert_eq!(
        quote.blocked_reason.as_deref(),
        Some("agent claim cap exceeded: owned=1 cap=1")
    );

    let owned = claim.owned_claims.first().expect("owned claim entry");
    assert_eq!(owned.target_agent_id, "agent-claim-target");
    assert_eq!(owned.status, "release_cooldown");
    assert_eq!(owned.claim_bond_locked_restricted_amount, 0);
    assert_eq!(owned.claim_bond_locked_liquid_amount, 200);
    assert!(owned.release_ready_in_epochs.is_some());
    assert!(owned.forced_reclaim_in_epochs.is_some());
}

#[test]
fn compat_snapshot_promotes_to_post_onboarding_after_control_feedback() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    server.latest_player_gameplay_feedback = Some(crate::simulator::PlayerGameplayRecentFeedback {
        action: "step".to_string(),
        stage: "completed_advanced".to_string(),
        effect: "world advanced: logicalTime +1, eventSeq +1".to_string(),
        reason: None,
        hint: None,
        delta_logical_time: 1,
        delta_event_seq: 1,
    });
    let snapshot = server.compat_snapshot();
    let gameplay = snapshot
        .player_gameplay
        .as_ref()
        .expect("player gameplay snapshot");
    assert_eq!(
        gameplay.stage_id,
        crate::simulator::PlayerGameplayStageId::PostOnboarding
    );
    assert!(gameplay.goal_id.starts_with("post_onboarding."));
    assert_eq!(
        gameplay
            .recent_feedback
            .as_ref()
            .expect("recent feedback")
            .stage,
        "completed_advanced"
    );
}

#[test]
fn runtime_gameplay_action_requires_auth() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let err = server
        .handle_gameplay_action(crate::viewer::GameplayActionRequest {
            action_id: "build_factory_smelter_mk1".to_string(),
            target_agent_id: agent_id,
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
        })
        .expect_err("missing auth should fail");
    assert_eq!(err.code, "auth_proof_required");
}

#[test]
fn runtime_agent_chat_requires_explicit_session_registration() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(24);
    let request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id,
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello".to_string(),
            intent_tick: Some(1),
            intent_seq: Some(2),
        },
        1,
        public_key.as_str(),
        private_key.as_str(),
    );
    let err = server
        .handle_agent_chat(request)
        .expect_err("session register should be required before agent chat");
    assert_eq!(err.code, "session_not_found");
}

#[test]
fn runtime_session_register_rejects_same_player_binding_to_second_agent() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::TwoBases)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_ids: Vec<_> = server
        .world
        .state()
        .agents
        .keys()
        .cloned()
        .take(2)
        .collect();
    assert!(
        agent_ids.len() >= 2,
        "expected at least two agents in two_bases scenario"
    );
    let (public_key, private_key) = test_signer(25);

    let first_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_ids[0].as_str()),
        1,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        first_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );
    assert_eq!(first_ack.agent_id.as_deref(), Some(agent_ids[0].as_str()));

    let conflict_request = signed_session_register_request(
        crate::viewer::AuthoritativeSessionRegisterRequest {
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
            requested_agent_id: Some(agent_ids[1].clone()),
            force_rebind: false,
        },
        2,
        public_key.as_str(),
        private_key.as_str(),
    );
    let err = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RegisterSession {
            request: conflict_request,
        })
        .expect_err("same player should not silently rebind to another agent");
    assert_eq!(err.code, "player_bind_failed");
    assert!(err.message.contains("explicit rebind required"));
}

#[test]
fn runtime_session_register_allows_same_player_rebind_with_force_rebind() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::TwoBases)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_ids: Vec<_> = server
        .world
        .state()
        .agents
        .keys()
        .cloned()
        .take(2)
        .collect();
    assert!(
        agent_ids.len() >= 2,
        "expected at least two agents in two_bases scenario"
    );
    let (public_key, private_key) = test_signer(26);

    let first_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_ids[0].as_str()),
        1,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        first_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );
    assert_eq!(first_ack.agent_id.as_deref(), Some(agent_ids[0].as_str()));

    let second_ack = register_runtime_session_with_options(
        &mut server,
        "player-a",
        Some(agent_ids[1].as_str()),
        true,
        2,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        second_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );
    assert_eq!(second_ack.agent_id.as_deref(), Some(agent_ids[1].as_str()));
    assert_eq!(
        server.llm_sidecar.bound_agent_for_player("player-a"),
        Some(agent_ids[1].as_str())
    );
    assert_eq!(
        server
            .llm_sidecar
            .agent_player_bindings
            .get(agent_ids[0].as_str()),
        None
    );
}

#[test]
fn runtime_gameplay_action_can_reach_first_capability_milestone_without_ui() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(31);

    let build_request = signed_gameplay_action_request(
        crate::viewer::GameplayActionRequest {
            action_id: "build_factory_smelter_mk1".to_string(),
            target_agent_id: agent_id.clone(),
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
        },
        31,
        public_key.as_str(),
        private_key.as_str(),
    );
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        30,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );
    let build_ack = server
        .handle_gameplay_action(build_request)
        .expect("queue smelter build");
    assert_eq!(build_ack.action_id, "build_factory_smelter_mk1");
    for _ in 0..2 {
        server.world.step().expect("settle smelter build");
    }
    assert!(server.world.has_factory("factory.smelter.mk1"));

    let recipe_request = signed_gameplay_action_request(
        crate::viewer::GameplayActionRequest {
            action_id: "schedule_recipe_smelter_iron_ingot".to_string(),
            target_agent_id: agent_id,
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
        },
        32,
        public_key.as_str(),
        private_key.as_str(),
    );
    let recipe_ack = server
        .handle_gameplay_action(recipe_request)
        .expect("queue iron ingot recipe");
    assert_eq!(recipe_ack.action_id, "schedule_recipe_smelter_iron_ingot");
    for _ in 0..4 {
        server.world.step().expect("settle recipe");
        if server.world.material_balance("iron_ingot") > 0 {
            break;
        }
    }

    assert!(server.world.material_balance("iron_ingot") > 0);
    let snapshot = server.compat_snapshot();
    let gameplay = snapshot
        .player_gameplay
        .expect("player gameplay after industrial progress");
    assert_eq!(gameplay.goal_id, "post_onboarding.choose_midloop_path");
    assert_eq!(gameplay.progress_percent, 100);
}

#[test]
fn runtime_agent_chat_openclaw_mode_reports_unsupported() {
    let _guard = runtime_openclaw_env_lock().lock().expect("env lock");
    clear_runtime_openclaw_env();
    std::env::set_var(VIEWER_AGENT_PROVIDER_MODE_ENV, "openclaw_local_http");
    std::env::set_var(VIEWER_OPENCLAW_BASE_URL_ENV, "http://127.0.0.1:5841");
    std::env::set_var(VIEWER_OPENCLAW_AGENT_PROFILE_ENV, "oasis7_p0_low_freq_npc");
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let err = server
        .handle_agent_chat(crate::viewer::AgentChatRequest {
            agent_id,
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello".to_string(),
            intent_tick: None,
            intent_seq: None,
        })
        .expect_err("openclaw mode should reject chat");
    assert_eq!(err.code, "agent_provider_chat_unsupported");
    clear_runtime_openclaw_env();
}

#[test]
fn runtime_agent_chat_replay_returns_idempotent_ack() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(21);
    let request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello".to_string(),
            intent_tick: Some(7),
            intent_seq: Some(5),
        },
        5,
        public_key.as_str(),
        private_key.as_str(),
    );
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        4,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );

    let first = server
        .handle_agent_chat(request.clone())
        .expect("first request accepted");
    assert_eq!(first.intent_tick, Some(7));
    assert_eq!(first.intent_seq, Some(5));
    assert!(!first.idempotent_replay);

    let replay = server
        .handle_agent_chat(request)
        .expect("replay request accepted");
    assert_eq!(replay.agent_id, first.agent_id);
    assert_eq!(replay.accepted_at_tick, first.accepted_at_tick);
    assert_eq!(replay.message_len, first.message_len);
    assert_eq!(replay.player_id, first.player_id);
    assert_eq!(replay.intent_tick, first.intent_tick);
    assert_eq!(replay.intent_seq, first.intent_seq);
    assert!(replay.idempotent_replay);
    assert_eq!(
        server
            .llm_sidecar
            .player_auth_last_nonce
            .get("player-a")
            .copied(),
        Some(5)
    );
}

#[test]
fn runtime_agent_chat_echo_env_enqueues_agent_spoke_virtual_event() {
    let _guard = lock_test_llm_env();
    std::env::set_var(RUNTIME_AGENT_CHAT_ECHO_ENV, "1");
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(31);
    let request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello runtime echo".to_string(),
            intent_tick: Some(9),
            intent_seq: Some(31),
        },
        31,
        public_key.as_str(),
        private_key.as_str(),
    );
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        30,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );

    let ack = server.handle_agent_chat(request).expect("chat accepted");
    assert_eq!(ack.agent_id, agent_id);

    let events: Vec<_> = server.pending_virtual_events.drain(..).collect();
    assert!(events.iter().any(|event| matches!(
        &event.kind,
        crate::simulator::WorldEventKind::AgentSpoke { agent_id: event_agent_id, message, .. }
            if event_agent_id == &agent_id && message == "[qa-echo] hello runtime echo"
    )));
}

#[test]
fn runtime_agent_chat_echo_removed_old_brand_env_is_ignored() {
    let _guard = lock_test_llm_env();
    std::env::set_var(
        removed_old_brand_runtime_live_env("RUNTIME_AGENT_CHAT_ECHO"),
        "1",
    );
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(32);
    let request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello removed old brand runtime echo".to_string(),
            intent_tick: Some(10),
            intent_seq: Some(32),
        },
        32,
        public_key.as_str(),
        private_key.as_str(),
    );
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        31,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );

    let ack = server.handle_agent_chat(request).expect("chat accepted");
    assert_eq!(ack.agent_id, agent_id);

    let events: Vec<_> = server.pending_virtual_events.drain(..).collect();
    assert!(!events.iter().any(|event| matches!(
        &event.kind,
        crate::simulator::WorldEventKind::AgentSpoke { agent_id: event_agent_id, message, .. }
            if event_agent_id == &agent_id && message == "[qa-echo] hello removed old brand runtime echo"
    )));
}

#[test]
fn runtime_agent_chat_rejects_intent_seq_conflict_on_payload_change() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(22);
    let first = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello".to_string(),
            intent_tick: Some(10),
            intent_seq: Some(6),
        },
        6,
        public_key.as_str(),
        private_key.as_str(),
    );
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        5,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );
    server
        .handle_agent_chat(first)
        .expect("first request accepted");

    let conflict = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id,
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "changed".to_string(),
            intent_tick: Some(10),
            intent_seq: Some(6),
        },
        6,
        public_key.as_str(),
        private_key.as_str(),
    );
    let err = server
        .handle_agent_chat(conflict)
        .expect_err("same seq with different payload must fail");
    assert_eq!(err.code, "intent_seq_conflict");
}

#[test]
fn runtime_agent_chat_rejects_intent_seq_nonce_mismatch() {
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key, private_key) = test_signer(23);
    let request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello".to_string(),
            intent_tick: Some(3),
            intent_seq: Some(8),
        },
        9,
        public_key.as_str(),
        private_key.as_str(),
    );
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        8,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );
    let err = server
        .handle_agent_chat(request)
        .expect_err("intent seq mismatch should fail");
    assert_eq!(err.code, "intent_seq_invalid");
}

fn commit_single_authoritative_batch(
    server: &mut ViewerRuntimeLiveServer,
) -> AuthoritativeBatchFinality {
    let journal_start = server.world.journal().events.len();
    server.script.enqueue(&mut server.world);
    server.world.step().expect("runtime step");

    let mut mapped_events = Vec::new();
    for runtime_event in &server.world.journal().events[journal_start..] {
        mapped_events.push(map_runtime_event(runtime_event, &server.snapshot_config));
    }
    mapped_events.extend(server.pending_virtual_events.drain(..));

    server
        .register_authoritative_batch(mapped_events.as_slice())
        .expect("register authoritative batch")
}

#[test]
fn runtime_authoritative_batch_commit_records_required_roots() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let batch = commit_single_authoritative_batch(&mut server);
    assert_eq!(batch.finality_state, AuthoritativeFinalityState::Pending);
    assert!(!batch.batch_id.is_empty());
    assert!(is_valid_root_hash(batch.state_root.as_str()));
    assert!(is_valid_root_hash(batch.data_root.as_str()));
    assert_eq!(server.authoritative_batches.len(), 1);
}

#[test]
fn runtime_authoritative_batch_finality_is_monotonic_and_final_only_gates_settlement() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let pending = commit_single_authoritative_batch(&mut server);
    assert!(!pending.settlement_ready);
    assert!(!pending.ranking_ready);

    let confirmed_updates = server
        .advance_authoritative_batch_finality(pending.confirm_height)
        .expect("advance to confirmed");
    assert_eq!(confirmed_updates.len(), 1);
    let confirmed = &confirmed_updates[0];
    assert_eq!(
        confirmed.finality_state,
        AuthoritativeFinalityState::Confirmed
    );
    assert!(!confirmed.settlement_ready);
    assert!(!confirmed.ranking_ready);

    let final_updates = server
        .advance_authoritative_batch_finality(pending.final_height)
        .expect("advance to final");
    assert_eq!(final_updates.len(), 1);
    let final_update = &final_updates[0];
    assert_eq!(
        final_update.finality_state,
        AuthoritativeFinalityState::Final
    );
    assert!(final_update.settlement_ready);
    assert!(final_update.ranking_ready);

    let no_regression = server
        .advance_authoritative_batch_finality(pending.confirm_height)
        .expect("finality should be monotonic");
    assert!(no_regression.is_empty());
    let stored = server.authoritative_batches.back().expect("stored batch");
    assert_eq!(stored.finality_state, AuthoritativeFinalityState::Final);
}

#[test]
fn runtime_authoritative_batch_data_root_mismatch_blocks_confirmation() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let pending = commit_single_authoritative_batch(&mut server);
    let tampered_root = "f".repeat(64);
    let batch = server
        .authoritative_batches
        .back_mut()
        .expect("stored batch for tamper");
    batch.data_root = tampered_root;

    let updates = server
        .advance_authoritative_batch_finality(pending.final_height.saturating_add(10))
        .expect("advance finality");
    assert!(updates.is_empty());

    let stored = server.authoritative_batches.back().expect("stored batch");
    assert_eq!(stored.finality_state, AuthoritativeFinalityState::Pending);
    let wire = stored.as_wire(&server.settlement_ranking_gate);
    assert!(!wire.settlement_ready);
    assert!(!wire.ranking_ready);
}

#[test]
fn runtime_authoritative_challenge_submit_opens_challenge_and_blocks_finality_progress() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let pending = commit_single_authoritative_batch(&mut server);
    let (_, maybe_batch_update) = server
        .handle_authoritative_challenge(AuthoritativeChallengeCommand::Submit {
            request: AuthoritativeChallengeSubmitRequest {
                batch_id: pending.batch_id.clone(),
                watcher_id: "watcher-1".to_string(),
                recomputed_state_root: pending.state_root.clone(),
                recomputed_data_root: pending.data_root.clone(),
                challenge_id: Some("challenge-1".to_string()),
            },
        })
        .expect("submit challenge");
    let batch_update = maybe_batch_update.expect("batch update");
    assert!(batch_update.challenge_open);
    assert_eq!(
        batch_update.active_challenge_id.as_deref(),
        Some("challenge-1")
    );

    let updates = server
        .advance_authoritative_batch_finality(pending.final_height.saturating_add(10))
        .expect("advance while challenged");
    assert!(updates.is_empty());
    let stored = server.authoritative_batches.back().expect("stored batch");
    assert_ne!(stored.finality_state, AuthoritativeFinalityState::Final);
}

#[test]
fn runtime_authoritative_challenge_resolve_no_fraud_unblocks_finality_without_slash() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let pending = commit_single_authoritative_batch(&mut server);
    let (submit_ack, _) = server
        .handle_authoritative_challenge(AuthoritativeChallengeCommand::Submit {
            request: AuthoritativeChallengeSubmitRequest {
                batch_id: pending.batch_id.clone(),
                watcher_id: "watcher-2".to_string(),
                recomputed_state_root: pending.state_root.clone(),
                recomputed_data_root: pending.data_root.clone(),
                challenge_id: None,
            },
        })
        .expect("submit challenge");
    assert_eq!(submit_ack.status, AuthoritativeChallengeStatus::Challenged);

    let (resolve_ack, maybe_batch_update) = server
        .handle_authoritative_challenge(AuthoritativeChallengeCommand::Resolve {
            request: AuthoritativeChallengeResolveRequest {
                challenge_id: submit_ack.challenge_id.clone(),
                resolved_by: Some("arbiter-1".to_string()),
            },
        })
        .expect("resolve challenge");
    assert_eq!(
        resolve_ack.status,
        AuthoritativeChallengeStatus::ResolvedNoFraud
    );
    assert!(!resolve_ack.slash_applied);
    let batch_update = maybe_batch_update.expect("batch update");
    assert!(!batch_update.challenge_open);
    assert!(!batch_update.slashed);

    let final_updates = server
        .advance_authoritative_batch_finality(pending.final_height)
        .expect("advance after resolve");
    assert!(final_updates.iter().any(|update| {
        update.batch_id == pending.batch_id
            && update.finality_state == AuthoritativeFinalityState::Final
            && !update.slashed
    }));
}

#[test]
fn runtime_authoritative_challenge_resolve_fraud_slashes_and_blocks_finality() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let pending = commit_single_authoritative_batch(&mut server);
    let (submit_ack, _) = server
        .handle_authoritative_challenge(AuthoritativeChallengeCommand::Submit {
            request: AuthoritativeChallengeSubmitRequest {
                batch_id: pending.batch_id.clone(),
                watcher_id: "watcher-3".to_string(),
                recomputed_state_root: "f".repeat(64),
                recomputed_data_root: pending.data_root.clone(),
                challenge_id: None,
            },
        })
        .expect("submit challenge");

    let (resolve_ack, maybe_batch_update) = server
        .handle_authoritative_challenge(AuthoritativeChallengeCommand::Resolve {
            request: AuthoritativeChallengeResolveRequest {
                challenge_id: submit_ack.challenge_id,
                resolved_by: Some("arbiter-1".to_string()),
            },
        })
        .expect("resolve challenge");
    assert_eq!(
        resolve_ack.status,
        AuthoritativeChallengeStatus::ResolvedFraudSlashed
    );
    assert!(resolve_ack.slash_applied);
    assert_eq!(
        resolve_ack.slash_reason.as_deref(),
        Some("state_root_mismatch")
    );
    let batch_update = maybe_batch_update.expect("batch update");
    assert!(batch_update.slashed);
    assert!(!batch_update.challenge_open);

    let updates = server
        .advance_authoritative_batch_finality(pending.final_height.saturating_add(10))
        .expect("advance after slash");
    assert!(updates
        .iter()
        .all(|update| update.batch_id != pending.batch_id));
    let stored = server.authoritative_batches.back().expect("stored batch");
    assert_eq!(
        stored.challenge_state,
        RuntimeBatchChallengeState::ResolvedFraudSlashed
    );
    assert_ne!(stored.finality_state, AuthoritativeFinalityState::Final);
}

#[test]
fn runtime_authoritative_challenge_duplicate_resolve_is_rejected() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let pending = commit_single_authoritative_batch(&mut server);
    let (submit_ack, _) = server
        .handle_authoritative_challenge(AuthoritativeChallengeCommand::Submit {
            request: AuthoritativeChallengeSubmitRequest {
                batch_id: pending.batch_id,
                watcher_id: "watcher-4".to_string(),
                recomputed_state_root: pending.state_root,
                recomputed_data_root: pending.data_root,
                challenge_id: Some("challenge-dup".to_string()),
            },
        })
        .expect("submit challenge");
    let _ = server
        .handle_authoritative_challenge(AuthoritativeChallengeCommand::Resolve {
            request: AuthoritativeChallengeResolveRequest {
                challenge_id: submit_ack.challenge_id.clone(),
                resolved_by: None,
            },
        })
        .expect("first resolve");

    let err = server
        .handle_authoritative_challenge(AuthoritativeChallengeCommand::Resolve {
            request: AuthoritativeChallengeResolveRequest {
                challenge_id: submit_ack.challenge_id,
                resolved_by: None,
            },
        })
        .expect_err("duplicate resolve should reject");
    assert_eq!(err.code, "challenge_already_resolved");
}

#[test]
fn runtime_authoritative_recovery_rollback_prunes_fork_batches() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let first = commit_single_authoritative_batch(&mut server);
    let updates = server
        .advance_authoritative_batch_finality(first.final_height)
        .expect("finalize first batch");
    assert!(updates.iter().any(|batch| {
        batch.batch_id == first.batch_id
            && batch.finality_state == AuthoritativeFinalityState::Final
    }));
    assert_eq!(server.stable_checkpoints.len(), 1);

    let second = commit_single_authoritative_batch(&mut server);
    assert_eq!(server.authoritative_batches.len(), 2);
    assert_eq!(server.authoritative_batches[1].batch_id, second.batch_id);

    let (ack, emit_snapshot_after_ack) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::Rollback {
            request: AuthoritativeRollbackRequest {
                target_batch_id: Some(first.batch_id.clone()),
                reason: "test_reorg".to_string(),
                requested_by: Some("ops".to_string()),
            },
        })
        .expect("rollback to first stable batch");
    assert!(emit_snapshot_after_ack);
    assert_eq!(ack.status, AuthoritativeRecoveryStatus::RolledBack);
    assert_eq!(
        ack.stable_batch_id.as_deref(),
        Some(first.batch_id.as_str())
    );
    assert_eq!(server.reorg_epoch, 1);
    assert_eq!(server.authoritative_batches.len(), 1);
    assert_eq!(server.authoritative_batches[0].batch_id, first.batch_id);
}

#[test]
fn runtime_authoritative_recovery_reconnect_detects_reorg_epoch_mismatch() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("runtime server");

    let first = commit_single_authoritative_batch(&mut server);
    let _ = server
        .advance_authoritative_batch_finality(first.final_height)
        .expect("finalize first batch");
    let initial_cursor = latest_runtime_event_seq(&server.world);

    let (initial_ack, emit_snapshot_after_ack) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::ReconnectSync {
            request: AuthoritativeReconnectSyncRequest {
                player_id: "player-a".to_string(),
                session_pubkey: None,
                last_known_log_cursor: Some(initial_cursor),
                expected_reorg_epoch: Some(0),
            },
        })
        .expect("initial reconnect sync");
    assert!(!emit_snapshot_after_ack);
    assert_eq!(
        initial_ack.status,
        AuthoritativeRecoveryStatus::CatchUpReady
    );
    assert_eq!(initial_ack.message.as_deref(), Some("delta_replay_allowed"));

    let _ = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::Rollback {
            request: AuthoritativeRollbackRequest {
                target_batch_id: Some(first.batch_id),
                reason: "force_reorg".to_string(),
                requested_by: None,
            },
        })
        .expect("rollback");
    assert_eq!(server.reorg_epoch, 1);

    let (stale_ack, emit_snapshot_after_ack) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::ReconnectSync {
            request: AuthoritativeReconnectSyncRequest {
                player_id: "player-a".to_string(),
                session_pubkey: None,
                last_known_log_cursor: Some(initial_cursor),
                expected_reorg_epoch: Some(0),
            },
        })
        .expect("stale reconnect sync");
    assert!(!emit_snapshot_after_ack);
    assert_eq!(stale_ack.status, AuthoritativeRecoveryStatus::CatchUpReady);
    assert!(stale_ack
        .message
        .as_deref()
        .is_some_and(|message| message.contains("snapshot_reload_required")));
}

#[test]
fn runtime_authoritative_recovery_rotate_and_revoke_session_enforced_for_agent_chat() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let (public_key_v1, private_key_v1) = test_signer(31);
    let (public_key_v2, private_key_v2) = test_signer(32);

    let first_request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "hello".to_string(),
            intent_tick: Some(1),
            intent_seq: Some(2),
        },
        2,
        public_key_v1.as_str(),
        private_key_v1.as_str(),
    );
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        1,
        public_key_v1.as_str(),
        private_key_v1.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );
    assert_eq!(register_ack.agent_id.as_deref(), Some(agent_id.as_str()));
    let _ = server
        .handle_agent_chat(first_request)
        .expect("first key should be accepted");

    let (rotate_ack, emit_snapshot_after_ack) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RotateSession {
            request: AuthoritativeSessionRotateRequest {
                player_id: "player-a".to_string(),
                old_session_pubkey: public_key_v1.clone(),
                new_session_pubkey: public_key_v2.clone(),
                rotate_reason: "security_rotation".to_string(),
                rotated_by: Some("ops".to_string()),
            },
        })
        .expect("rotate session");
    assert!(!emit_snapshot_after_ack);
    assert_eq!(
        rotate_ack.status,
        AuthoritativeRecoveryStatus::SessionRotated
    );
    assert_eq!(
        rotate_ack.session_pubkey.as_deref(),
        Some(public_key_v1.as_str())
    );
    assert_eq!(
        rotate_ack.replaced_by_pubkey.as_deref(),
        Some(public_key_v2.as_str())
    );

    let stale_request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "stale".to_string(),
            intent_tick: Some(2),
            intent_seq: Some(2),
        },
        2,
        public_key_v1.as_str(),
        private_key_v1.as_str(),
    );
    let stale_err = server
        .handle_agent_chat(stale_request)
        .expect_err("old key should be rejected after rotation");
    assert_eq!(stale_err.code, "session_revoked");

    let rotated_request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id: agent_id.clone(),
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "rotated".to_string(),
            intent_tick: Some(3),
            intent_seq: Some(1),
        },
        1,
        public_key_v2.as_str(),
        private_key_v2.as_str(),
    );
    let _ = server
        .handle_agent_chat(rotated_request)
        .expect("new key should be accepted");

    let (revoke_ack, emit_snapshot_after_ack) = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RevokeSession {
            request: AuthoritativeSessionRevokeRequest {
                player_id: "player-a".to_string(),
                session_pubkey: Some(public_key_v2.clone()),
                revoke_reason: "compromised".to_string(),
                revoked_by: Some("ops".to_string()),
            },
        })
        .expect("revoke session");
    assert!(!emit_snapshot_after_ack);
    assert_eq!(
        revoke_ack.status,
        AuthoritativeRecoveryStatus::SessionRevoked
    );
    assert_eq!(revoke_ack.revoke_reason.as_deref(), Some("compromised"));
    assert_eq!(revoke_ack.revoked_by.as_deref(), Some("ops"));

    let revoked_reconnect_err = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::ReconnectSync {
            request: AuthoritativeReconnectSyncRequest {
                player_id: "player-a".to_string(),
                session_pubkey: Some(public_key_v2.clone()),
                last_known_log_cursor: None,
                expected_reorg_epoch: None,
            },
        })
        .expect_err("reconnect should surface revoke metadata");
    assert_eq!(revoked_reconnect_err.code, "session_revoked");
    assert_eq!(
        revoked_reconnect_err.revoke_reason.as_deref(),
        Some("compromised")
    );
    assert_eq!(revoked_reconnect_err.revoked_by.as_deref(), Some("ops"));

    let revoked_register_request = signed_session_register_request(
        crate::viewer::AuthoritativeSessionRegisterRequest {
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
            requested_agent_id: Some(agent_id.clone()),
            force_rebind: false,
        },
        5,
        public_key_v2.as_str(),
        private_key_v2.as_str(),
    );
    let revoked_register_err = server
        .handle_authoritative_recovery(AuthoritativeRecoveryCommand::RegisterSession {
            request: revoked_register_request,
        })
        .expect_err("register should surface revoke metadata");
    assert_eq!(revoked_register_err.code, "session_revoked");
    assert_eq!(
        revoked_register_err.revoke_reason.as_deref(),
        Some("compromised")
    );
    assert_eq!(revoked_register_err.revoked_by.as_deref(), Some("ops"));

    let revoked_request = signed_agent_chat_request(
        crate::viewer::AgentChatRequest {
            agent_id,
            player_id: Some("player-a".to_string()),
            public_key: None,
            auth: None,
            message: "revoked".to_string(),
            intent_tick: Some(4),
            intent_seq: Some(2),
        },
        2,
        public_key_v2.as_str(),
        private_key_v2.as_str(),
    );
    let revoked_err = server
        .handle_agent_chat(revoked_request)
        .expect_err("revoked key should be rejected");
    assert_eq!(revoked_err.code, "session_revoked");
}
