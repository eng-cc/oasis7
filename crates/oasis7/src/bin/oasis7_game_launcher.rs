use oasis7::simulator::ProviderExecutionMode;
use oasis7_proto::storage_profile::StorageProfile;
use std::collections::BTreeSet;
use std::env;
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{self, Child, Command};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
#[path = "../hosted_access.rs"]
mod hosted_access;
#[path = "oasis7_game_launcher/hosted_player_session.rs"]
mod hosted_player_session;
#[path = "oasis7_game_launcher/hosted_strong_auth.rs"]
mod hosted_strong_auth;
#[path = "oasis7_game_launcher/runtime_paths.rs"]
mod runtime_paths;
#[path = "oasis7_game_launcher/runtime_presence.rs"]
mod runtime_presence;
#[path = "oasis7_game_launcher/static_http.rs"]
mod static_http;
#[path = "oasis7_game_launcher/url_encoding.rs"]
mod url_encoding;
use hosted_access::{DeploymentMode, DEFAULT_DEPLOYMENT_MODE};
use hosted_player_session::HostedPlayerSessionIssuer;
use runtime_paths::{
    resolve_oasis7_chain_runtime_binary, resolve_oasis7_viewer_live_binary,
    resolve_viewer_static_dir, resolve_viewer_static_dir_with_override, viewer_dev_dist_candidates,
};
#[cfg(test)]
use runtime_presence::query_runtime_bound_players;
use runtime_presence::run_runtime_presence_monitor;
use static_http::{
    build_viewer_auth_bootstrap_script, content_type_for_path, handle_http_connection,
    resolve_static_asset_path, resolve_viewer_auth_bootstrap_for_embedded_server,
    resolve_viewer_auth_bootstrap_from_path, sanitize_index_html_for_embedded_server,
    sanitize_relative_request_path,
};
use url_encoding::encoded_query_pair;
const DEFAULT_SCENARIO: &str = "llm_bootstrap";
const DEFAULT_LIVE_BIND: &str = "127.0.0.1:5023";
const DEFAULT_WEB_BIND: &str = "127.0.0.1:5011";
const DEFAULT_VIEWER_HOST: &str = "127.0.0.1";
const DEFAULT_VIEWER_PORT: u16 = 4173;
const DEFAULT_VIEWER_STATIC_DIR: &str = "web";
const GAME_STATIC_DIR_ENV: &str = "OASIS7_GAME_STATIC_DIR";
const OASIS7_VIEWER_LIVE_BIN_ENV: &str = "OASIS7_VIEWER_LIVE_BIN";
const OASIS7_CHAIN_RUNTIME_BIN_ENV: &str = "OASIS7_CHAIN_RUNTIME_BIN";
const BUILTIN_LLM_DECISION_SOURCE: &str = "builtin_llm";
const PROVIDER_BACKED_DECISION_SOURCE: &str = "provider_backed";
const PROVIDER_LOOPBACK_HTTP_IMPLEMENTATION: &str = "provider_loopback_http";
const OPENCLAW_LOCAL_HTTP_COMPAT_ALIAS: &str = "openclaw_local_http";
const OPENCLAW_PROVIDER_BACKEND: &str = "openclaw";
const WORLDSIM_PROVIDER_CONTRACT: &str = "worldsim_provider_v1";
const LOOPBACK_HTTP_PROVIDER_TRANSPORT: &str = "loopback_http";
const AGENT_DIRECT_CONNECT_PROVIDER_MODE_ALIAS: &str = "agent_direct_connect";
const DEFAULT_AGENT_PROVIDER_URL: &str = "http://127.0.0.1:5841";
const DEFAULT_AGENT_PROVIDER_CONNECT_TIMEOUT_MS: u64 = 15_000;
const DEFAULT_AGENT_PROVIDER_PROFILE: &str = "oasis7_p0_low_freq_npc";
const DEFAULT_INTERACTIVE_LLM_TIMEOUT_MS: u64 = 10_000;
const LLM_TIMEOUT_MS_ENV: &str = "OASIS7_LLM_TIMEOUT_MS";
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
const DEFAULT_VIEWER_PLAYER_ID: &str = "viewer-player";
const DEFAULT_CHAIN_STATUS_BIND: &str = "127.0.0.1:5121";
const DEFAULT_CHAIN_NODE_ID: &str = "viewer-live-node";
const DEFAULT_CHAIN_NODE_ROLE: &str = "sequencer";
const DEFAULT_CHAIN_P2P_USER_MODE: &str = "auto_join";
const DEFAULT_CHAIN_NODE_TICK_MS: u64 = 200;
const DEFAULT_CHAIN_POS_SLOT_DURATION_MS: u64 = 12_000;
const DEFAULT_CHAIN_POS_TICKS_PER_SLOT: u64 = 10;
const DEFAULT_CHAIN_POS_PROPOSAL_TICK_PHASE: u64 = 9;
const DEFAULT_CHAIN_POS_MAX_PAST_SLOT_LAG: u64 = 256;
const VIEWER_PLAYER_ID_ENV: &str = "OASIS7_VIEWER_PLAYER_ID";
const VIEWER_AUTH_PUBLIC_KEY_ENV: &str = "OASIS7_VIEWER_AUTH_PUBLIC_KEY";
const VIEWER_AUTH_PRIVATE_KEY_ENV: &str = "OASIS7_VIEWER_AUTH_PRIVATE_KEY";
const VIEWER_AUTH_BOOTSTRAP_OBJECT: &str = "__OASIS7_VIEWER_AUTH_ENV";
const NODE_CONFIG_FILE_NAME: &str = "config.toml";
const NODE_TABLE_KEY: &str = "node";
const NODE_PRIVATE_KEY_FIELD: &str = "private_key";
const NODE_PUBLIC_KEY_FIELD: &str = "public_key";
static TERMINATION_REQUESTED: AtomicBool = AtomicBool::new(false);
static SIGNAL_HANDLER_INSTALL: OnceLock<Result<(), String>> = OnceLock::new();

fn default_chain_node_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{DEFAULT_CHAIN_NODE_ID}-fresh-{}-{now}", process::id())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ViewerAuthBootstrap {
    player_id: String,
    public_key: String,
    private_key: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CliOptions {
    deployment_mode: String,
    scenario: String,
    live_bind: String,
    web_bind: String,
    viewer_host: String,
    viewer_port: u16,
    viewer_static_dir: String,
    with_llm: bool,
    agent_decision_source: String,
    agent_provider_backend: String,
    agent_provider_contract: String,
    agent_provider_transport: String,
    agent_provider_url: String,
    agent_provider_auth_token: String,
    agent_provider_connect_timeout_ms: u64,
    agent_provider_profile: String,
    agent_execution_lane: ProviderExecutionMode,
    open_browser: bool,
    chain_enabled: bool,
    chain_status_bind: String,
    chain_node_id: String,
    chain_storage_profile: StorageProfile,
    chain_world_id: Option<String>,
    chain_node_role: String,
    chain_p2p_user_mode: String,
    chain_p2p_accept_public_entry: bool,
    chain_node_tick_ms: u64,
    chain_pos_slot_duration_ms: u64,
    chain_pos_ticks_per_slot: u64,
    chain_pos_proposal_tick_phase: u64,
    chain_pos_adaptive_tick_scheduler_enabled: bool,
    chain_pos_slot_clock_genesis_unix_ms: Option<i64>,
    chain_pos_max_past_slot_lag: u64,
    chain_node_validators: Vec<String>,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            deployment_mode: DEFAULT_DEPLOYMENT_MODE.to_string(),
            scenario: DEFAULT_SCENARIO.to_string(),
            live_bind: DEFAULT_LIVE_BIND.to_string(),
            web_bind: DEFAULT_WEB_BIND.to_string(),
            viewer_host: DEFAULT_VIEWER_HOST.to_string(),
            viewer_port: DEFAULT_VIEWER_PORT,
            viewer_static_dir: DEFAULT_VIEWER_STATIC_DIR.to_string(),
            with_llm: true,
            agent_decision_source: BUILTIN_LLM_DECISION_SOURCE.to_string(),
            agent_provider_backend: OPENCLAW_PROVIDER_BACKEND.to_string(),
            agent_provider_contract: WORLDSIM_PROVIDER_CONTRACT.to_string(),
            agent_provider_transport: LOOPBACK_HTTP_PROVIDER_TRANSPORT.to_string(),
            agent_provider_url: DEFAULT_AGENT_PROVIDER_URL.to_string(),
            agent_provider_auth_token: String::new(),
            agent_provider_connect_timeout_ms: DEFAULT_AGENT_PROVIDER_CONNECT_TIMEOUT_MS,
            agent_provider_profile: DEFAULT_AGENT_PROVIDER_PROFILE.to_string(),
            agent_execution_lane: ProviderExecutionMode::HeadlessAgent,
            open_browser: true,
            chain_enabled: true,
            chain_status_bind: DEFAULT_CHAIN_STATUS_BIND.to_string(),
            chain_node_id: default_chain_node_id(),
            chain_storage_profile: StorageProfile::DevLocal,
            chain_world_id: None,
            chain_node_role: DEFAULT_CHAIN_NODE_ROLE.to_string(),
            chain_p2p_user_mode: DEFAULT_CHAIN_P2P_USER_MODE.to_string(),
            chain_p2p_accept_public_entry: false,
            chain_node_tick_ms: DEFAULT_CHAIN_NODE_TICK_MS,
            chain_pos_slot_duration_ms: DEFAULT_CHAIN_POS_SLOT_DURATION_MS,
            chain_pos_ticks_per_slot: DEFAULT_CHAIN_POS_TICKS_PER_SLOT,
            chain_pos_proposal_tick_phase: DEFAULT_CHAIN_POS_PROPOSAL_TICK_PHASE,
            chain_pos_adaptive_tick_scheduler_enabled: false,
            chain_pos_slot_clock_genesis_unix_ms: None,
            chain_pos_max_past_slot_lag: DEFAULT_CHAIN_POS_MAX_PAST_SLOT_LAG,
            chain_node_validators: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct StaticHttpServer {
    stop_requested: Arc<AtomicBool>,
    stop_tx: Sender<()>,
    error_rx: Receiver<String>,
    join_handle: Option<thread::JoinHandle<()>>,
    runtime_presence_join_handle: Option<thread::JoinHandle<()>>,
}

fn main() {
    let raw_args: Vec<String> = env::args().skip(1).collect();
    if raw_args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return;
    }

    let options = match parse_options(raw_args.iter().map(|arg| arg.as_str())) {
        Ok(options) => options,
        Err(err) => {
            eprintln!("{err}");
            print_help();
            process::exit(1);
        }
    };

    if let Err(err) = run_launcher(&options) {
        eprintln!("launcher failed: {err}");
        process::exit(1);
    }
}

fn run_launcher(options: &CliOptions) -> Result<(), String> {
    install_signal_handler()?;
    TERMINATION_REQUESTED.store(false, Ordering::SeqCst);
    if !options.with_llm {
        return Err(
            "oasis7 gameplay requires --with-llm; no-LLM launch is no longer a playable entry path"
                .to_string(),
        );
    }

    let oasis7_viewer_live_bin = resolve_oasis7_viewer_live_binary()?;
    let oasis7_chain_runtime_bin = if options.chain_enabled {
        Some(resolve_oasis7_chain_runtime_binary()?)
    } else {
        None
    };
    let viewer_static_dir = resolve_viewer_static_dir(options.viewer_static_dir.as_str())?;

    let mut chain_child = if let Some(chain_bin) = oasis7_chain_runtime_bin.as_ref() {
        Some(spawn_oasis7_chain_runtime(chain_bin.as_path(), options)?)
    } else {
        None
    };
    let mut world_child = match spawn_oasis7_viewer_live(&oasis7_viewer_live_bin, options) {
        Ok(child) => child,
        Err(err) => {
            if let Some(child) = chain_child.as_mut() {
                terminate_child(child);
            }
            return Err(err);
        }
    };
    let mut server = match start_static_http_server(
        deployment_mode_from_options(options),
        options.live_bind.as_str(),
        options.viewer_host.as_str(),
        options.viewer_port,
        viewer_static_dir.as_path(),
    ) {
        Ok(server) => server,
        Err(err) => {
            terminate_child(&mut world_child);
            if let Some(child) = chain_child.as_mut() {
                terminate_child(child);
            }
            return Err(err);
        }
    };

    let ready_result = wait_until_ready(options, &mut world_child, chain_child.as_mut());
    if let Err(err) = ready_result {
        stop_static_http_server(&mut server);
        terminate_child(&mut world_child);
        if let Some(child) = chain_child.as_mut() {
            terminate_child(child);
        }
        return Err(err);
    }

    let game_url = build_game_url(options);
    println!("Launcher stack is ready.");
    println!("- URL: {game_url}");
    println!("- oasis7_viewer_live pid: {}", world_child.id());
    if let Some(chain_child) = chain_child.as_ref() {
        println!("- oasis7_chain_runtime pid: {}", chain_child.id());
        println!(
            "- chain status: http://{}/v1/chain/status",
            options.chain_status_bind
        );
    } else {
        println!("- oasis7_chain_runtime: disabled");
    }
    println!("- web static root: {}", viewer_static_dir.display());
    println!("Press Ctrl+C to stop.");

    if options.open_browser {
        if let Err(err) = open_browser(&game_url) {
            eprintln!("warning: failed to open browser automatically: {err}");
            eprintln!("open this URL manually: {game_url}");
        }
    }

    let monitor_result =
        monitor_world_chain_and_server(&mut world_child, chain_child.as_mut(), &mut server);
    stop_static_http_server(&mut server);
    terminate_child(&mut world_child);
    if let Some(child) = chain_child.as_mut() {
        terminate_child(child);
    }
    monitor_result
}

fn install_signal_handler() -> Result<(), String> {
    SIGNAL_HANDLER_INSTALL
        .get_or_init(|| {
            ctrlc::set_handler(|| {
                TERMINATION_REQUESTED.store(true, Ordering::SeqCst);
            })
            .map_err(|err| format!("failed to install signal handler: {err}"))
        })
        .clone()
}

fn spawn_oasis7_viewer_live(path: &Path, options: &CliOptions) -> Result<Child, String> {
    let mut command =
        build_oasis7_viewer_live_command(path, options, env::var_os(LLM_TIMEOUT_MS_ENV).is_some());
    command.spawn().map_err(|err| {
        format!(
            "failed to start oasis7_viewer_live from `{}`: {err}",
            path.display()
        )
    })
}

fn build_oasis7_viewer_live_command(
    path: &Path,
    options: &CliOptions,
    parent_has_llm_timeout_ms: bool,
) -> Command {
    let mut command = Command::new(path);
    command
        .arg(options.scenario.as_str())
        .arg("--bind")
        .arg(options.live_bind.as_str())
        .arg("--web-bind")
        .arg(options.web_bind.as_str())
        .arg("--deployment-mode")
        .arg(options.deployment_mode.as_str());
    if options.with_llm {
        command.arg("--llm");
        apply_viewer_live_env_overrides(&mut command, options, parent_has_llm_timeout_ms);
    } else {
        command.arg("--no-llm");
    }
    command
}

fn apply_viewer_live_env_overrides(
    command: &mut Command,
    options: &CliOptions,
    parent_has_llm_timeout_ms: bool,
) {
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
    ] {
        command.env_remove(env_name);
    }

    if uses_openclaw_provider(options) {
        command.env(
            VIEWER_AGENT_DECISION_SOURCE_ENV,
            PROVIDER_BACKED_DECISION_SOURCE,
        );
        command.env(VIEWER_AGENT_PROVIDER_BACKEND_ENV, OPENCLAW_PROVIDER_BACKEND);
        command.env(
            VIEWER_AGENT_PROVIDER_CONTRACT_ENV,
            WORLDSIM_PROVIDER_CONTRACT,
        );
        command.env(
            VIEWER_AGENT_PROVIDER_TRANSPORT_ENV,
            LOOPBACK_HTTP_PROVIDER_TRANSPORT,
        );
        command.env(
            VIEWER_AGENT_PROVIDER_URL_ENV,
            options.agent_provider_url.as_str(),
        );
        if !options.agent_provider_auth_token.trim().is_empty() {
            command.env(
                VIEWER_AGENT_PROVIDER_AUTH_TOKEN_ENV,
                options.agent_provider_auth_token.as_str(),
            );
        }
        command.env(
            VIEWER_AGENT_PROVIDER_CONNECT_TIMEOUT_MS_ENV,
            options.agent_provider_connect_timeout_ms.to_string(),
        );
        command.env(
            VIEWER_AGENT_PROVIDER_PROFILE_ENV,
            options.agent_provider_profile.as_str(),
        );
        command.env(
            VIEWER_AGENT_EXECUTION_LANE_ENV,
            options.agent_execution_lane.as_str(),
        );
        return;
    }

    if !parent_has_llm_timeout_ms {
        command.env(
            LLM_TIMEOUT_MS_ENV,
            DEFAULT_INTERACTIVE_LLM_TIMEOUT_MS.to_string(),
        );
    }
}

fn spawn_oasis7_chain_runtime(path: &Path, options: &CliOptions) -> Result<Child, String> {
    let mut command = Command::new(path);
    command.args(build_oasis7_chain_runtime_args(options));
    command.spawn().map_err(|err| {
        format!(
            "failed to start oasis7_chain_runtime from `{}`: {err}",
            path.display()
        )
    })
}

fn chain_world_id(options: &CliOptions) -> String {
    options
        .chain_world_id
        .clone()
        .unwrap_or_else(|| format!("live-{}", options.scenario))
}

fn chain_execution_world_dir(node_id: &str) -> String {
    Path::new("output")
        .join("chain-runtime")
        .join(node_id)
        .join("reward-runtime-execution-world")
        .to_string_lossy()
        .into_owned()
}

fn build_oasis7_chain_runtime_args(options: &CliOptions) -> Vec<String> {
    let execution_world_dir = chain_execution_world_dir(options.chain_node_id.as_str());
    let mut args = vec![
        "--node-id".to_string(),
        options.chain_node_id.clone(),
        "--world-id".to_string(),
        chain_world_id(options),
        "--status-bind".to_string(),
        options.chain_status_bind.clone(),
        "--storage-profile".to_string(),
        options.chain_storage_profile.as_str().to_string(),
        "--execution-world-dir".to_string(),
        execution_world_dir,
        "--node-role".to_string(),
        options.chain_node_role.clone(),
        "--p2p-user-mode".to_string(),
        options.chain_p2p_user_mode.clone(),
        "--node-tick-ms".to_string(),
        options.chain_node_tick_ms.to_string(),
        "--pos-slot-duration-ms".to_string(),
        options.chain_pos_slot_duration_ms.to_string(),
        "--pos-ticks-per-slot".to_string(),
        options.chain_pos_ticks_per_slot.to_string(),
        "--pos-proposal-tick-phase".to_string(),
        options.chain_pos_proposal_tick_phase.to_string(),
        if options.chain_pos_adaptive_tick_scheduler_enabled {
            "--pos-adaptive-tick-scheduler".to_string()
        } else {
            "--pos-no-adaptive-tick-scheduler".to_string()
        },
        "--pos-max-past-slot-lag".to_string(),
        options.chain_pos_max_past_slot_lag.to_string(),
    ];
    args.push(if options.chain_p2p_accept_public_entry {
        "--p2p-accept-public-entry".to_string()
    } else {
        "--p2p-reject-public-entry".to_string()
    });
    if let Some(genesis) = options.chain_pos_slot_clock_genesis_unix_ms {
        args.push("--pos-slot-clock-genesis-unix-ms".to_string());
        args.push(genesis.to_string());
    }
    for validator in &options.chain_node_validators {
        args.push("--node-validator".to_string());
        args.push(validator.clone());
    }
    args
}

fn start_static_http_server(
    deployment_mode: DeploymentMode,
    live_bind: &str,
    host: &str,
    port: u16,
    root_dir: &Path,
) -> Result<StaticHttpServer, String> {
    let listener = TcpListener::bind((host, port))
        .map_err(|err| format!("failed to bind static HTTP server at {host}:{port}: {err}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|err| format!("failed to set static HTTP listener nonblocking: {err}"))?;

    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let (error_tx, error_rx) = mpsc::channel::<String>();
    let stop_requested = Arc::new(AtomicBool::new(false));
    let root_dir = Arc::new(root_dir.to_path_buf());
    let live_bind = Arc::new(live_bind.to_string());
    let hosted_session_issuer = Arc::new(Mutex::new(HostedPlayerSessionIssuer::default()));
    let runtime_presence_join_handle = if deployment_mode == DeploymentMode::HostedPublicJoin {
        let stop_requested = Arc::clone(&stop_requested);
        let live_bind = Arc::clone(&live_bind);
        let hosted_session_issuer = Arc::clone(&hosted_session_issuer);
        Some(thread::spawn(move || {
            run_runtime_presence_monitor(stop_requested, live_bind, hosted_session_issuer)
        }))
    } else {
        None
    };
    let join_handle = thread::spawn(move || {
        if let Err(err) = run_static_http_loop(
            listener,
            deployment_mode,
            root_dir,
            live_bind,
            hosted_session_issuer,
            stop_rx,
        ) {
            let _ = error_tx.send(err);
        }
    });

    Ok(StaticHttpServer {
        stop_requested,
        stop_tx,
        error_rx,
        join_handle: Some(join_handle),
        runtime_presence_join_handle,
    })
}

fn run_static_http_loop(
    listener: TcpListener,
    deployment_mode: DeploymentMode,
    root_dir: Arc<PathBuf>,
    live_bind: Arc<String>,
    hosted_session_issuer: Arc<Mutex<HostedPlayerSessionIssuer>>,
    stop_rx: Receiver<()>,
) -> Result<(), String> {
    loop {
        match stop_rx.try_recv() {
            Ok(_) | Err(TryRecvError::Disconnected) => return Ok(()),
            Err(TryRecvError::Empty) => {}
        }

        match listener.accept() {
            Ok((stream, _addr)) => {
                let root_dir = Arc::clone(&root_dir);
                let live_bind = Arc::clone(&live_bind);
                let deployment_mode = deployment_mode;
                let hosted_session_issuer = Arc::clone(&hosted_session_issuer);
                thread::spawn(move || {
                    if let Err(err) = handle_http_connection(
                        stream,
                        root_dir.as_path(),
                        live_bind.as_str(),
                        deployment_mode,
                        &hosted_session_issuer,
                    ) {
                        eprintln!("warning: static HTTP connection failed: {err}");
                    }
                });
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(err) => {
                return Err(format!("static HTTP accept failed: {err}"));
            }
        }
    }
}

fn resolve_viewer_player_id_override(value: Option<String>) -> String {
    value
        .map(|raw| raw.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_VIEWER_PLAYER_ID.to_string())
}

fn resolve_required_toml_string(
    table: &toml::value::Table,
    key: &str,
    label: &str,
) -> Result<String, String> {
    let value = table
        .get(key)
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{label} is missing or empty"))?;
    Ok(value.to_string())
}

fn wait_until_ready(
    options: &CliOptions,
    world_child: &mut Child,
    mut chain_child: Option<&mut Child>,
) -> Result<(), String> {
    let (viewer_host, viewer_port) = normalize_http_target(
        options.viewer_host.as_str(),
        options.viewer_port,
        "viewer host/port",
    )?;
    poll_startup_health(world_child, chain_child.as_deref_mut())?;
    wait_for_http_ready(
        viewer_host.as_str(),
        viewer_port,
        Duration::from_secs(30),
        world_child,
        chain_child.as_deref_mut(),
    )
    .map_err(|err| {
        format!("viewer HTTP did not become ready at {viewer_host}:{viewer_port}: {err}")
    })?;
    poll_startup_health(world_child, chain_child.as_deref_mut())?;

    let (bridge_host, bridge_port) = parse_host_port(options.web_bind.as_str(), "--web-bind")?;
    wait_for_tcp_ready(
        bridge_host.as_str(),
        bridge_port,
        Duration::from_secs(60),
        world_child,
        chain_child.as_deref_mut(),
    )
    .map_err(|err| {
        format!("web bridge did not become ready at {bridge_host}:{bridge_port}: {err}")
    })?;
    poll_startup_health(world_child, chain_child.as_deref_mut())?;

    if options.chain_enabled {
        let (chain_status_host, chain_status_port) =
            parse_host_port(options.chain_status_bind.as_str(), "--chain-status-bind")?;
        let chain_status_host = normalize_bind_host_for_local_access(chain_status_host.as_str());
        wait_for_http_ready(
            chain_status_host.as_str(),
            chain_status_port,
            Duration::from_secs(30),
            world_child,
            chain_child.as_deref_mut(),
        )
        .map_err(|err| {
            format!(
                "chain status HTTP did not become ready at {}:{}: {}",
                chain_status_host, chain_status_port, err
            )
        })?;
    }
    Ok(())
}

fn monitor_world_chain_and_server(
    world_child: &mut Child,
    mut chain_child: Option<&mut Child>,
    server: &mut StaticHttpServer,
) -> Result<(), String> {
    loop {
        if TERMINATION_REQUESTED.load(Ordering::SeqCst) {
            return Ok(());
        }
        if let Some(status) = world_child
            .try_wait()
            .map_err(|err| format!("failed to query oasis7_viewer_live status: {err}"))?
        {
            return Err(format!("oasis7_viewer_live exited unexpectedly: {status}"));
        }
        if let Some(chain_child) = chain_child.as_deref_mut() {
            if let Some(status) = chain_child
                .try_wait()
                .map_err(|err| format!("failed to query oasis7_chain_runtime status: {err}"))?
            {
                return Err(format!(
                    "oasis7_chain_runtime exited unexpectedly: {status}"
                ));
            }
        }

        match server.error_rx.try_recv() {
            Ok(err) => return Err(format!("static HTTP server failed: {err}")),
            Err(TryRecvError::Disconnected) => {
                return Err("static HTTP server channel disconnected unexpectedly".to_string());
            }
            Err(TryRecvError::Empty) => {}
        }

        if let Some(handle) = server.join_handle.as_ref() {
            if handle.is_finished() {
                return Err("static HTTP server exited unexpectedly".to_string());
            }
        }
        if let Some(handle) = server.runtime_presence_join_handle.as_ref() {
            if handle.is_finished() {
                return Err("runtime presence monitor exited unexpectedly".to_string());
            }
        }

        thread::sleep(Duration::from_millis(400));
    }
}

fn stop_static_http_server(server: &mut StaticHttpServer) {
    server.stop_requested.store(true, Ordering::SeqCst);
    let _ = server.stop_tx.send(());
    if let Some(handle) = server.join_handle.take() {
        let _ = handle.join();
    }
    if let Some(handle) = server.runtime_presence_join_handle.take() {
        let _ = handle.join();
    }
}

fn terminate_child(child: &mut Child) {
    if let Ok(None) = child.try_wait() {
        let _ = child.kill();
        let _ = child.wait();
    }
}

fn wait_for_tcp_ready(
    host: &str,
    port: u16,
    timeout: Duration,
    world_child: &mut Child,
    mut chain_child: Option<&mut Child>,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        poll_startup_health(world_child, chain_child.as_deref_mut())?;
        match TcpStream::connect((host, port)) {
            Ok(_) => {
                poll_startup_health(world_child, chain_child.as_deref_mut())?;
                return Ok(());
            }
            Err(_) => thread::sleep(Duration::from_millis(200)),
        }
    }
    poll_startup_health(world_child, chain_child.as_deref_mut())?;
    Err(format!("timeout after {}s", timeout.as_secs()))
}

fn wait_for_http_ready(
    host: &str,
    port: u16,
    timeout: Duration,
    world_child: &mut Child,
    mut chain_child: Option<&mut Child>,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    let request = format!("GET / HTTP/1.1\r\nHost: {host}:{port}\r\nConnection: close\r\n\r\n");

    while Instant::now() < deadline {
        poll_startup_health(world_child, chain_child.as_deref_mut())?;
        if let Ok(mut stream) = TcpStream::connect((host, port)) {
            let _ = stream.write_all(request.as_bytes());
            let mut buf = [0u8; 256];
            match stream.read(&mut buf) {
                Ok(0) => {}
                Ok(bytes) => {
                    let response = String::from_utf8_lossy(&buf[..bytes]);
                    if response.starts_with("HTTP/") {
                        poll_startup_health(world_child, chain_child.as_deref_mut())?;
                        return Ok(());
                    }
                }
                Err(_) => {}
            }
        }
        thread::sleep(Duration::from_millis(200));
    }

    poll_startup_health(world_child, chain_child.as_deref_mut())?;
    Err(format!("timeout after {}s", timeout.as_secs()))
}

fn poll_startup_health(
    world_child: &mut Child,
    chain_child: Option<&mut Child>,
) -> Result<(), String> {
    if TERMINATION_REQUESTED.load(Ordering::SeqCst) {
        return Err("termination requested".to_string());
    }
    if let Some(status) = world_child
        .try_wait()
        .map_err(|err| format!("failed to query oasis7_viewer_live status during startup: {err}"))?
    {
        return Err(format!(
            "oasis7_viewer_live exited during startup: {status}"
        ));
    }
    if let Some(chain_child) = chain_child {
        if let Some(status) = chain_child.try_wait().map_err(|err| {
            format!("failed to query oasis7_chain_runtime status during startup: {err}")
        })? {
            return Err(format!(
                "oasis7_chain_runtime exited during startup: {status}"
            ));
        }
    }
    Ok(())
}

fn build_game_url(options: &CliOptions) -> String {
    let viewer_host = normalize_bind_host_for_local_access(options.viewer_host.as_str());
    let viewer_host = host_for_url(viewer_host.as_str());
    let (bridge_host, bridge_port) = parse_host_port(options.web_bind.as_str(), "--web-bind")
        .unwrap_or_else(|_| ("127.0.0.1".to_string(), 5011));
    let bridge_host = normalize_bind_host_for_local_access(bridge_host.as_str());
    let bridge_host = host_for_url(bridge_host.as_str());
    let ws_url = format!("ws://{bridge_host}:{bridge_port}");
    let hosted_access_hint = serde_json::to_string(&hosted_access::hosted_viewer_access_hint(
        deployment_mode_from_options(options),
    ))
    .unwrap_or_else(|_| "{}".to_string());
    format!(
        "http://{viewer_host}:{}/?{}&{}",
        options.viewer_port,
        encoded_query_pair("ws", ws_url.as_str()),
        encoded_query_pair("hosted_access", hosted_access_hint.as_str()),
    )
}

fn normalize_http_target(host: &str, port: u16, label: &str) -> Result<(String, u16), String> {
    let normalized = normalize_bind_host_for_local_access(host);
    if normalized.trim().is_empty() {
        return Err(format!("{label} host cannot be empty"));
    }
    Ok((normalized, port))
}

fn normalize_bind_host_for_local_access(host: &str) -> String {
    let trimmed = host.trim();
    if trimmed == "0.0.0.0" || trimmed == "::" || trimmed == "[::]" {
        "127.0.0.1".to_string()
    } else {
        trimmed.to_string()
    }
}

fn host_for_url(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') && !host.ends_with(']') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

fn parse_options<'a>(args: impl Iterator<Item = &'a str>) -> Result<CliOptions, String> {
    let mut options = CliOptions::default();
    let mut iter = args.peekable();

    while let Some(arg) = iter.next() {
        match arg {
            "--deployment-mode" => {
                options.deployment_mode = parse_required_value(&mut iter, "--deployment-mode")?;
            }
            "--scenario" => {
                options.scenario = parse_required_value(&mut iter, "--scenario")?;
            }
            "--live-bind" => {
                options.live_bind = parse_required_value(&mut iter, "--live-bind")?;
            }
            "--web-bind" => {
                options.web_bind = parse_required_value(&mut iter, "--web-bind")?;
            }
            "--viewer-host" => {
                options.viewer_host = parse_required_value(&mut iter, "--viewer-host")?;
            }
            "--viewer-port" => {
                let raw = parse_required_value(&mut iter, "--viewer-port")?;
                options.viewer_port = raw.parse::<u16>().map_err(|_| {
                    format!("--viewer-port must be an integer in 1..=65535, got `{raw}`")
                })?;
                if options.viewer_port == 0 {
                    return Err("--viewer-port must be in 1..=65535".to_string());
                }
            }
            "--viewer-static-dir" => {
                options.viewer_static_dir = parse_required_value(&mut iter, "--viewer-static-dir")?;
            }
            "--with-llm" => {
                options.with_llm = true;
            }
            "--agent-decision-source" => {
                options.agent_decision_source =
                    parse_required_value(&mut iter, "--agent-decision-source")?;
            }
            "--agent-provider-backend" => {
                options.agent_provider_backend =
                    parse_required_value(&mut iter, "--agent-provider-backend")?;
            }
            "--agent-provider-contract" => {
                options.agent_provider_contract =
                    parse_required_value(&mut iter, "--agent-provider-contract")?;
            }
            "--agent-provider-transport" => {
                options.agent_provider_transport =
                    parse_required_value(&mut iter, "--agent-provider-transport")?;
            }
            "--agent-provider-url" => {
                options.agent_provider_url =
                    parse_required_value(&mut iter, "--agent-provider-url")?;
            }
            "--agent-provider-auth-token" => {
                options.agent_provider_auth_token =
                    parse_required_value(&mut iter, "--agent-provider-auth-token")?;
            }
            "--agent-provider-connect-timeout-ms" => {
                let raw = parse_required_value(&mut iter, "--agent-provider-connect-timeout-ms")?;
                options.agent_provider_connect_timeout_ms = raw.parse::<u64>().map_err(|_| {
                    format!("--agent-provider-connect-timeout-ms must be a positive integer, got `{raw}`")
                })?;
                if options.agent_provider_connect_timeout_ms == 0 {
                    return Err(
                        "--agent-provider-connect-timeout-ms must be a positive integer"
                            .to_string(),
                    );
                }
            }
            "--agent-provider-profile" => {
                options.agent_provider_profile =
                    parse_required_value(&mut iter, "--agent-provider-profile")?;
            }
            "--agent-execution-lane" => {
                let raw = parse_required_value(&mut iter, "--agent-execution-lane")?;
                options.agent_execution_lane = ProviderExecutionMode::parse(raw.as_str())
                    .ok_or_else(|| {
                        format!(
                            "--agent-execution-lane must be one of player_parity or headless_agent, got `{raw}`"
                        )
                    })?;
            }
            "--agent-provider-mode" => {
                options.agent_decision_source =
                    parse_required_value(&mut iter, "--agent-provider-mode")?;
            }
            "--openclaw-base-url" => {
                options.agent_provider_url =
                    parse_required_value(&mut iter, "--openclaw-base-url")?;
            }
            "--openclaw-auth-token" => {
                options.agent_provider_auth_token =
                    parse_required_value(&mut iter, "--openclaw-auth-token")?;
            }
            "--openclaw-connect-timeout-ms" => {
                let raw = parse_required_value(&mut iter, "--openclaw-connect-timeout-ms")?;
                options.agent_provider_connect_timeout_ms = raw.parse::<u64>().map_err(|_| {
                    format!("--openclaw-connect-timeout-ms must be a positive integer, got `{raw}`")
                })?;
                if options.agent_provider_connect_timeout_ms == 0 {
                    return Err(
                        "--openclaw-connect-timeout-ms must be a positive integer".to_string()
                    );
                }
            }
            "--openclaw-agent-profile" => {
                options.agent_provider_profile =
                    parse_required_value(&mut iter, "--openclaw-agent-profile")?;
            }
            "--openclaw-execution-mode" => {
                let raw = parse_required_value(&mut iter, "--openclaw-execution-mode")?;
                options.agent_execution_lane = ProviderExecutionMode::parse(raw.as_str())
                    .ok_or_else(|| {
                        format!(
                            "--openclaw-execution-mode must be one of player_parity or headless_agent, got `{raw}`"
                        )
                    })?;
            }
            "--no-open-browser" => {
                options.open_browser = false;
            }
            "--chain-enable" => {
                options.chain_enabled = true;
            }
            "--chain-disable" => {
                options.chain_enabled = false;
            }
            "--chain-status-bind" => {
                options.chain_status_bind = parse_required_value(&mut iter, "--chain-status-bind")?;
            }
            "--chain-node-id" => {
                options.chain_node_id = parse_required_value(&mut iter, "--chain-node-id")?;
            }
            "--chain-storage-profile" => {
                options.chain_storage_profile =
                    parse_required_value(&mut iter, "--chain-storage-profile")?
                        .parse::<StorageProfile>()?;
            }
            "--chain-world-id" => {
                options.chain_world_id = Some(parse_required_value(&mut iter, "--chain-world-id")?);
            }
            "--chain-node-role" => {
                let raw = parse_required_value(&mut iter, "--chain-node-role")?;
                options.chain_node_role = parse_chain_node_role(raw.as_str())?;
            }
            "--chain-p2p-user-mode" => {
                let raw = parse_required_value(&mut iter, "--chain-p2p-user-mode")?;
                options.chain_p2p_user_mode = parse_chain_p2p_user_mode(raw.as_str())?;
            }
            "--chain-p2p-accept-public-entry" => {
                options.chain_p2p_accept_public_entry = true;
            }
            "--chain-p2p-reject-public-entry" => {
                options.chain_p2p_accept_public_entry = false;
            }
            "--chain-node-tick-ms" => {
                let raw = parse_required_value(&mut iter, "--chain-node-tick-ms")?;
                options.chain_node_tick_ms = raw.parse::<u64>().map_err(|_| {
                    format!("--chain-node-tick-ms must be a positive integer, got `{raw}`")
                })?;
                if options.chain_node_tick_ms == 0 {
                    return Err("--chain-node-tick-ms must be a positive integer".to_string());
                }
            }
            "--chain-pos-slot-duration-ms" => {
                let raw = parse_required_value(&mut iter, "--chain-pos-slot-duration-ms")?;
                options.chain_pos_slot_duration_ms = raw.parse::<u64>().map_err(|_| {
                    format!("--chain-pos-slot-duration-ms must be a positive integer, got `{raw}`")
                })?;
                if options.chain_pos_slot_duration_ms == 0 {
                    return Err(
                        "--chain-pos-slot-duration-ms must be a positive integer".to_string()
                    );
                }
            }
            "--chain-pos-ticks-per-slot" => {
                let raw = parse_required_value(&mut iter, "--chain-pos-ticks-per-slot")?;
                options.chain_pos_ticks_per_slot = raw.parse::<u64>().map_err(|_| {
                    format!("--chain-pos-ticks-per-slot must be a positive integer, got `{raw}`")
                })?;
                if options.chain_pos_ticks_per_slot == 0 {
                    return Err("--chain-pos-ticks-per-slot must be a positive integer".to_string());
                }
            }
            "--chain-pos-proposal-tick-phase" => {
                let raw = parse_required_value(&mut iter, "--chain-pos-proposal-tick-phase")?;
                options.chain_pos_proposal_tick_phase = raw.parse::<u64>().map_err(|_| {
                    format!("--chain-pos-proposal-tick-phase must be a non-negative integer, got `{raw}`")
                })?;
            }
            "--chain-pos-adaptive-tick-scheduler" => {
                options.chain_pos_adaptive_tick_scheduler_enabled = true;
            }
            "--chain-pos-no-adaptive-tick-scheduler" => {
                options.chain_pos_adaptive_tick_scheduler_enabled = false;
            }
            "--chain-pos-slot-clock-genesis-unix-ms" => {
                let raw =
                    parse_required_value(&mut iter, "--chain-pos-slot-clock-genesis-unix-ms")?;
                options.chain_pos_slot_clock_genesis_unix_ms =
                    Some(raw.parse::<i64>().map_err(|_| {
                        format!(
                            "--chain-pos-slot-clock-genesis-unix-ms must be an integer, got `{raw}`"
                        )
                    })?);
            }
            "--chain-pos-max-past-slot-lag" => {
                let raw = parse_required_value(&mut iter, "--chain-pos-max-past-slot-lag")?;
                options.chain_pos_max_past_slot_lag = raw.parse::<u64>().map_err(|_| {
                    format!(
                        "--chain-pos-max-past-slot-lag must be a non-negative integer, got `{raw}`"
                    )
                })?;
            }
            "--chain-node-validator" => {
                let value = parse_required_value(&mut iter, "--chain-node-validator")?;
                validate_chain_node_validator(value.as_str())?;
                options.chain_node_validators.push(value);
            }
            _ => return Err(format!("unknown option: {arg}")),
        }
    }

    let _ = parse_host_port(options.live_bind.as_str(), "--live-bind")?;
    let _ = parse_host_port(options.web_bind.as_str(), "--web-bind")?;
    DeploymentMode::parse(options.deployment_mode.as_str(), "--deployment-mode")?;
    validate_agent_decision_source(options.agent_decision_source.as_str())?;
    options.agent_decision_source =
        canonical_agent_decision_source(options.agent_decision_source.as_str()).to_string();
    if options.agent_decision_source == PROVIDER_BACKED_DECISION_SOURCE {
        validate_agent_provider_backend(options.agent_provider_backend.as_str())?;
        validate_agent_provider_contract(options.agent_provider_contract.as_str())?;
        validate_agent_provider_transport(options.agent_provider_transport.as_str())?;
        options.agent_provider_backend =
            canonical_agent_provider_backend(options.agent_provider_backend.as_str()).to_string();
        options.agent_provider_contract =
            canonical_agent_provider_contract(options.agent_provider_contract.as_str()).to_string();
        options.agent_provider_transport = canonical_agent_provider_transport(
            options.agent_provider_transport.as_str(),
        )
        .to_string();
        if options.agent_provider_url.trim().is_empty() {
            return Err("--agent-provider-url requires a non-empty value".to_string());
        }
        if options.agent_provider_profile.trim().is_empty() {
            return Err("--agent-provider-profile requires a non-empty value".to_string());
        }
    }
    normalize_http_target(
        options.viewer_host.as_str(),
        options.viewer_port,
        "viewer host/port",
    )?;
    if options.chain_enabled {
        let _ = parse_host_port(options.chain_status_bind.as_str(), "--chain-status-bind")?;
        if options.chain_node_id.trim().is_empty() {
            return Err("--chain-node-id requires a non-empty value".to_string());
        }
        parse_chain_node_role(options.chain_node_role.as_str())?;
        parse_chain_p2p_user_mode(options.chain_p2p_user_mode.as_str())?;
        if options.chain_node_tick_ms == 0 {
            return Err("--chain-node-tick-ms must be a positive integer".to_string());
        }
        if options.chain_pos_slot_duration_ms == 0 {
            return Err("--chain-pos-slot-duration-ms must be a positive integer".to_string());
        }
        if options.chain_pos_ticks_per_slot == 0 {
            return Err("--chain-pos-ticks-per-slot must be a positive integer".to_string());
        }
        if options.chain_pos_proposal_tick_phase >= options.chain_pos_ticks_per_slot {
            return Err(format!(
                "--chain-pos-proposal-tick-phase={} must be less than --chain-pos-ticks-per-slot={}",
                options.chain_pos_proposal_tick_phase, options.chain_pos_ticks_per_slot
            ));
        }
        for validator in &options.chain_node_validators {
            validate_chain_node_validator(validator.as_str())?;
        }
    }

    Ok(options)
}

fn deployment_mode_from_options(options: &CliOptions) -> DeploymentMode {
    DeploymentMode::parse(options.deployment_mode.as_str(), "deployment_mode")
        .unwrap_or(DeploymentMode::TrustedLocalOnly)
}

fn uses_openclaw_provider(options: &CliOptions) -> bool {
    options.agent_decision_source == PROVIDER_BACKED_DECISION_SOURCE
        && options.agent_provider_backend == OPENCLAW_PROVIDER_BACKEND
        && options.agent_provider_contract == WORLDSIM_PROVIDER_CONTRACT
        && options.agent_provider_transport == LOOPBACK_HTTP_PROVIDER_TRANSPORT
}

fn validate_agent_decision_source(raw: &str) -> Result<(), String> {
    match raw.trim() {
        BUILTIN_LLM_DECISION_SOURCE
        | PROVIDER_BACKED_DECISION_SOURCE
        | PROVIDER_LOOPBACK_HTTP_IMPLEMENTATION
        | OPENCLAW_LOCAL_HTTP_COMPAT_ALIAS
        | AGENT_DIRECT_CONNECT_PROVIDER_MODE_ALIAS => Ok(()),
        _ => Err(
            "--agent-decision-source must be builtin_llm or provider_backed".to_string(),
        ),
    }
}

fn canonical_agent_decision_source(raw: &str) -> &'static str {
    match raw.trim() {
        BUILTIN_LLM_DECISION_SOURCE => BUILTIN_LLM_DECISION_SOURCE,
        PROVIDER_BACKED_DECISION_SOURCE
        | PROVIDER_LOOPBACK_HTTP_IMPLEMENTATION
        | OPENCLAW_LOCAL_HTTP_COMPAT_ALIAS
        | AGENT_DIRECT_CONNECT_PROVIDER_MODE_ALIAS => PROVIDER_BACKED_DECISION_SOURCE,
        _ => BUILTIN_LLM_DECISION_SOURCE,
    }
}

fn validate_agent_provider_backend(raw: &str) -> Result<(), String> {
    match raw.trim() {
        OPENCLAW_PROVIDER_BACKEND
        | PROVIDER_LOOPBACK_HTTP_IMPLEMENTATION
        | OPENCLAW_LOCAL_HTTP_COMPAT_ALIAS
        | AGENT_DIRECT_CONNECT_PROVIDER_MODE_ALIAS => Ok(()),
        _ => Err("--agent-provider-backend must be openclaw".to_string()),
    }
}

fn canonical_agent_provider_backend(raw: &str) -> &'static str {
    match raw.trim() {
        OPENCLAW_PROVIDER_BACKEND
        | PROVIDER_LOOPBACK_HTTP_IMPLEMENTATION
        | OPENCLAW_LOCAL_HTTP_COMPAT_ALIAS
        | AGENT_DIRECT_CONNECT_PROVIDER_MODE_ALIAS => OPENCLAW_PROVIDER_BACKEND,
        _ => OPENCLAW_PROVIDER_BACKEND,
    }
}

fn validate_agent_provider_contract(raw: &str) -> Result<(), String> {
    match raw.trim() {
        WORLDSIM_PROVIDER_CONTRACT
        | PROVIDER_LOOPBACK_HTTP_IMPLEMENTATION
        | OPENCLAW_LOCAL_HTTP_COMPAT_ALIAS
        | AGENT_DIRECT_CONNECT_PROVIDER_MODE_ALIAS => Ok(()),
        _ => Err("--agent-provider-contract must be worldsim_provider_v1".to_string()),
    }
}

fn canonical_agent_provider_contract(raw: &str) -> &'static str {
    match raw.trim() {
        WORLDSIM_PROVIDER_CONTRACT
        | PROVIDER_LOOPBACK_HTTP_IMPLEMENTATION
        | OPENCLAW_LOCAL_HTTP_COMPAT_ALIAS
        | AGENT_DIRECT_CONNECT_PROVIDER_MODE_ALIAS => WORLDSIM_PROVIDER_CONTRACT,
        _ => WORLDSIM_PROVIDER_CONTRACT,
    }
}

fn validate_agent_provider_transport(raw: &str) -> Result<(), String> {
    match raw.trim() {
        LOOPBACK_HTTP_PROVIDER_TRANSPORT
        | PROVIDER_LOOPBACK_HTTP_IMPLEMENTATION
        | OPENCLAW_LOCAL_HTTP_COMPAT_ALIAS
        | AGENT_DIRECT_CONNECT_PROVIDER_MODE_ALIAS => Ok(()),
        _ => Err("--agent-provider-transport must be loopback_http".to_string()),
    }
}

fn canonical_agent_provider_transport(raw: &str) -> &'static str {
    match raw.trim() {
        LOOPBACK_HTTP_PROVIDER_TRANSPORT
        | PROVIDER_LOOPBACK_HTTP_IMPLEMENTATION
        | OPENCLAW_LOCAL_HTTP_COMPAT_ALIAS
        | AGENT_DIRECT_CONNECT_PROVIDER_MODE_ALIAS => LOOPBACK_HTTP_PROVIDER_TRANSPORT,
        _ => LOOPBACK_HTTP_PROVIDER_TRANSPORT,
    }
}

fn parse_required_value<'a, I>(
    iter: &mut std::iter::Peekable<I>,
    flag: &str,
) -> Result<String, String>
where
    I: Iterator<Item = &'a str>,
{
    let Some(value) = iter.next() else {
        return Err(format!("{flag} requires a value"));
    };
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{flag} requires a non-empty value"));
    }
    Ok(value.to_string())
}

fn parse_host_port(raw: &str, label: &str) -> Result<(String, u16), String> {
    let trimmed = raw.trim();
    let (host_raw, port_text) = if let Some(rest) = trimmed.strip_prefix('[') {
        let (host, remainder) = rest
            .split_once(']')
            .ok_or_else(|| format!("{label} IPv6 host must be in [addr]:port format"))?;
        let port_text = remainder
            .strip_prefix(':')
            .ok_or_else(|| format!("{label} must be in <host:port> format"))?;
        (host, port_text)
    } else {
        let (host, port_text) = trimmed
            .rsplit_once(':')
            .ok_or_else(|| format!("{label} must be in <host:port> format"))?;
        if host.contains(':') {
            return Err(format!("{label} IPv6 host must be wrapped in []"));
        }
        (host, port_text)
    };
    let host = host_raw.trim();
    if host.trim().is_empty() {
        return Err(format!("{label} host cannot be empty"));
    }
    let port = port_text
        .parse::<u16>()
        .map_err(|_| format!("{label} port must be an integer in 1..=65535"))?;
    if port == 0 {
        return Err(format!("{label} port must be in 1..=65535"));
    }
    Ok((host.trim().to_string(), port))
}

fn parse_chain_node_role(raw: &str) -> Result<String, String> {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "sequencer" | "storage" | "observer" => Ok(normalized),
        _ => Err("--chain-node-role must be one of: sequencer, storage, observer".to_string()),
    }
}

fn parse_chain_p2p_user_mode(raw: &str) -> Result<String, String> {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "auto_join" | "private_safe" | "public_entry" => Ok(normalized),
        _ => Err(
            "--chain-p2p-user-mode must be one of: auto_join, private_safe, public_entry"
                .to_string(),
        ),
    }
}

fn validate_chain_node_validator(raw: &str) -> Result<(), String> {
    let (validator_id, stake) = raw.rsplit_once(':').ok_or_else(|| {
        "--chain-node-validator must be in <validator_id:stake> format".to_string()
    })?;
    if validator_id.trim().is_empty() {
        return Err("--chain-node-validator validator_id cannot be empty".to_string());
    }
    let stake = stake
        .parse::<u64>()
        .map_err(|_| "--chain-node-validator stake must be a positive integer".to_string())?;
    if stake == 0 {
        return Err("--chain-node-validator stake must be a positive integer".to_string());
    }
    Ok(())
}

fn open_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("open")
            .arg(url)
            .status()
            .map_err(|err| format!("failed to execute `open`: {err}"))?;
        if status.success() {
            return Ok(());
        }
        return Err(format!("`open` exited with status {status}"));
    }

    #[cfg(target_os = "windows")]
    {
        let status = Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("")
            .arg(url)
            .status()
            .map_err(|err| format!("failed to execute `cmd /C start`: {err}"))?;
        if status.success() {
            return Ok(());
        }
        return Err(format!("`cmd /C start` exited with status {status}"));
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        let status = Command::new("xdg-open")
            .arg(url)
            .status()
            .map_err(|err| format!("failed to execute `xdg-open`: {err}"))?;
        if status.success() {
            return Ok(());
        }
        Err(format!("`xdg-open` exited with status {status}"))
    }
}

fn print_help() {
    println!(
        "Usage: oasis7_game_launcher [options]\n\n\
Start player stack with one command:\n\
- start oasis7_chain_runtime (default)\n\
- start oasis7_viewer_live\n\
- start built-in static web server\n\
- print URL and optionally open browser\n\n\
Options:\n\
  --deployment-mode <mode>    trusted_local_only|hosted_public_join (default: {DEFAULT_DEPLOYMENT_MODE})\n\
  --scenario <name>            oasis7_viewer_live scenario (default: {DEFAULT_SCENARIO})\n\
  --live-bind <host:port>      oasis7_viewer_live bind (default: {DEFAULT_LIVE_BIND})\n\
  --web-bind <host:port>       oasis7_viewer_live web bridge bind (default: {DEFAULT_WEB_BIND})\n\
  --viewer-host <host>         web viewer host (default: {DEFAULT_VIEWER_HOST})\n\
  --viewer-port <port>         web viewer port (default: {DEFAULT_VIEWER_PORT})\n\
  --viewer-static-dir <path>   prebuilt web asset dir (default: {DEFAULT_VIEWER_STATIC_DIR})\n\
  --chain-enable               enable oasis7_chain_runtime (default)\n\
  --chain-disable              disable oasis7_chain_runtime\n\
  --chain-status-bind <addr>   oasis7_chain_runtime status bind (default: {DEFAULT_CHAIN_STATUS_BIND})\n\
  --chain-node-id <id>         oasis7_chain_runtime node id (default: {DEFAULT_CHAIN_NODE_ID})\n\
  --chain-storage-profile <name> oasis7_chain_runtime storage profile (default: dev_local)\n\
  --chain-world-id <id>        oasis7_chain_runtime world id (default: live-<scenario>)\n\
  --chain-node-role <role>     oasis7_chain_runtime role (default: {DEFAULT_CHAIN_NODE_ROLE})\n\
  --chain-p2p-user-mode <mode> oasis7_chain_runtime user mode: auto_join|private_safe|public_entry (default: {DEFAULT_CHAIN_P2P_USER_MODE})\n\
  --chain-p2p-accept-public-entry\n\
                               accept auto-detected public-entry recommendation\n\
  --chain-p2p-reject-public-entry\n\
                               keep conservative fallback when auto mode suggests public entry (default)\n\
  --chain-node-tick-ms <n>     oasis7_chain_runtime worker poll/fallback interval ms (default: {DEFAULT_CHAIN_NODE_TICK_MS})\n\
  --chain-pos-slot-duration-ms <n>\n\
                               oasis7_chain_runtime PoS slot duration ms (default: {DEFAULT_CHAIN_POS_SLOT_DURATION_MS})\n\
  --chain-pos-ticks-per-slot <n>\n\
                               oasis7_chain_runtime PoS logical ticks per slot (default: {DEFAULT_CHAIN_POS_TICKS_PER_SLOT})\n\
  --chain-pos-proposal-tick-phase <n>\n\
                               oasis7_chain_runtime proposal phase in slot tick window (default: {DEFAULT_CHAIN_POS_PROPOSAL_TICK_PHASE})\n\
  --chain-pos-adaptive-tick-scheduler\n\
                               enable oasis7_chain_runtime adaptive tick scheduler\n\
  --chain-pos-no-adaptive-tick-scheduler\n\
                               disable oasis7_chain_runtime adaptive scheduler (default)\n\
  --chain-pos-slot-clock-genesis-unix-ms <n>\n\
                               oasis7_chain_runtime fixed slot clock genesis unix ms (default: auto)\n\
  --chain-pos-max-past-slot-lag <n>\n\
                               oasis7_chain_runtime max accepted stale slot lag (default: {DEFAULT_CHAIN_POS_MAX_PAST_SLOT_LAG})\n\
  --chain-node-validator <v:s> oasis7_chain_runtime validator (repeatable)\n\
  --with-llm                   enable llm mode (default; required for gameplay)\n\
  --agent-decision-source <src>\n\
                               agent decision source: builtin_llm|provider_backed\n\
  --agent-provider-backend <id>\n\
                               provider backend: openclaw (default when provider_backed)\n\
  --agent-provider-contract <id>\n\
                               provider contract: worldsim_provider_v1 (default when provider_backed)\n\
  --agent-provider-transport <id>\n\
                               provider transport: loopback_http (default when provider_backed)\n\
  --agent-provider-url <url>   provider URL (default: {DEFAULT_AGENT_PROVIDER_URL})\n\
  --agent-provider-auth-token <tok>\n\
                               provider bearer token\n\
  --agent-provider-connect-timeout-ms <ms>\n\
                               provider connect timeout ms (default: {DEFAULT_AGENT_PROVIDER_CONNECT_TIMEOUT_MS})\n\
  --agent-provider-profile <id>\n\
                               provider profile (default: {DEFAULT_AGENT_PROVIDER_PROFILE})\n\
  --agent-execution-lane <mode>\n\
                               execution lane: player_parity|headless_agent (default: headless_agent)\n\
  --agent-provider-mode <mode> legacy alias for --agent-decision-source; accepts agent_direct_connect/provider_loopback_http/openclaw_local_http\n\
  --openclaw-base-url <url>    legacy alias for --agent-provider-url\n\
  --openclaw-auth-token <tok>  legacy alias for --agent-provider-auth-token\n\
  --openclaw-connect-timeout-ms <ms>\n\
                               legacy alias for --agent-provider-connect-timeout-ms\n\
  --openclaw-agent-profile <id>\n\
                               legacy alias for --agent-provider-profile\n\
  --openclaw-execution-mode <mode>\n\
                               legacy alias for --agent-execution-lane\n\
  --no-open-browser            do not auto open browser\n\
  -h, --help                   show help\n\n\
Env:\n\
  OASIS7_VIEWER_LIVE_BIN              explicit path of oasis7_viewer_live binary\n\
  OASIS7_CHAIN_RUNTIME_BIN            explicit path of oasis7_chain_runtime binary\n\
  OASIS7_GAME_STATIC_DIR              override default viewer static dir when --viewer-static-dir is omitted"
    );
}

#[cfg(test)]
#[path = "oasis7_game_launcher/oasis7_game_launcher_tests.rs"]
mod oasis7_game_launcher_tests;
