use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ed25519_dalek::SigningKey;
use oasis7::runtime::{
    NodeAssetBalance, NodeRewardMintRecord, ReleaseSecurityPolicy, RewardAssetConfig,
};
use oasis7_node::{
    derive_libp2p_identity_keypair, Libp2pReplicationNetwork, Libp2pReplicationNetworkConfig,
    NodeConfig, NodeFeedbackP2pConfig, NodeNetworkPolicy, NodePosConfig,
    NodeReachabilityAutoDetection, NodeReplicationConfig, NodeReplicationNetworkHandle, NodeRole,
    NodeRuntime, NodeSnapshot, NodeUserModeRecommendation, PosConsensusStatus, PosValidator,
};
use oasis7_proto::distributed_dht::{PeerDiscoverySource, PeerRecord};
use oasis7_proto::storage_profile::{StorageProfile, StorageProfileConfig};
use serde::Serialize;
use sha2::{Digest, Sha256};
#[path = "oasis7_chain_runtime/balances_api.rs"]
mod balances_api;
#[path = "oasis7_chain_runtime/cli.rs"]
mod cli;
#[path = "oasis7_chain_runtime/distfs_probe_runtime.rs"]
mod distfs_probe_runtime;
#[cfg(not(test))]
#[allow(dead_code)]
#[path = "oasis7_chain_runtime/execution_bridge/mod.rs"]
mod execution_bridge;
#[path = "oasis7_chain_runtime/explorer_p0_api.rs"]
mod explorer_p0_api;
#[path = "oasis7_chain_runtime/feedback_submit_api.rs"]
mod feedback_submit_api;
#[path = "oasis7_chain_runtime/governance_registry.rs"]
mod governance_registry;
#[path = "oasis7_chain_runtime/module_release_attestation_submit_api.rs"]
mod module_release_attestation_submit_api;
#[path = "oasis7_chain_runtime/node_keypair_config.rs"]
mod node_keypair_config;
#[path = "oasis7_chain_runtime/p2p_status.rs"]
mod p2p_status;
#[path = "oasis7_chain_runtime/reward_runtime_settlement.rs"]
mod reward_runtime_settlement;
#[path = "oasis7_chain_runtime/reward_runtime_worker.rs"]
mod reward_runtime_worker;
#[path = "oasis7_chain_runtime/storage_metrics.rs"]
mod storage_metrics;
#[path = "oasis7_chain_runtime/transfer_submit_api.rs"]
mod transfer_submit_api;
#[cfg(test)]
use self::cli::{parse_validator_spec, DEFAULT_NODE_ID, DEFAULT_STATUS_BIND};
use balances_api::build_chain_balances_payload;
#[cfg(test)]
use balances_api::build_chain_balances_payload_from_world;
use cli::{
    parse_host_port, parse_options, print_help, CliOptions, DEFAULT_REPLICATION_NETWORK_LISTEN,
    DEFAULT_REWARD_RUNTIME_DISTFS_PROBE_STATE_FILE, DEFAULT_REWARD_RUNTIME_REPORT_DIR,
    DEFAULT_REWARD_RUNTIME_STATE_FILE, DEFAULT_REWARD_RUNTIME_STORAGE_METRICS_FILE,
};
use execution_bridge::NodeRuntimeExecutionDriver;
use feedback_submit_api::{
    build_feedback_create_request, extract_http_json_body, parse_feedback_submit_request,
    write_feedback_submit_error, ChainFeedbackSubmitResponse, FeedbackSubmitSigner,
};
use p2p_status::{
    applied_runtime_user_mode_label, build_live_node_network_policy_recommendation,
    build_node_network_policy, peer_reachability_as_str,
};
use reward_runtime_worker::{
    init_shared_metrics, poll_worker_error, snapshot_metrics, start_reward_runtime_worker,
    stop_reward_runtime_worker, RewardRuntimeWorkerConfig, SharedRewardRuntimeMetrics,
};
#[cfg(test)]
mod execution_bridge {
    use std::path::Path;

    use oasis7::runtime::{ReleaseSecurityPolicy, World as RuntimeWorld};
    use oasis7_node::{NodeExecutionCommitContext, NodeExecutionCommitResult, NodeExecutionHook};
    use oasis7_proto::storage_profile::StorageProfileConfig;

    #[derive(Debug)]
    pub(super) struct NodeRuntimeExecutionDriver;

    #[allow(dead_code)]
    impl NodeRuntimeExecutionDriver {
        pub(super) fn new(
            _state_path: std::path::PathBuf,
            _world_dir: std::path::PathBuf,
            _records_dir: std::path::PathBuf,
            _storage_root: std::path::PathBuf,
        ) -> Result<Self, String> {
            Ok(Self)
        }

        pub(super) fn new_with_storage_profile(
            _state_path: std::path::PathBuf,
            _world_dir: std::path::PathBuf,
            _records_dir: std::path::PathBuf,
            _storage_root: std::path::PathBuf,
            _storage_profile: &StorageProfileConfig,
        ) -> Result<Self, String> {
            Ok(Self)
        }
    }

    impl NodeExecutionHook for NodeRuntimeExecutionDriver {
        fn on_commit(
            &mut self,
            context: NodeExecutionCommitContext,
        ) -> Result<NodeExecutionCommitResult, String> {
            Ok(NodeExecutionCommitResult {
                execution_height: context.height,
                execution_block_hash: String::new(),
                execution_state_root: String::new(),
            })
        }
    }

    pub(super) fn load_execution_world(world_dir: &Path) -> Result<RuntimeWorld, String> {
        let snapshot_path = world_dir.join("snapshot.json");
        let journal_path = world_dir.join("journal.json");
        if !snapshot_path.exists() || !journal_path.exists() {
            return Ok(RuntimeWorld::new_production_hardened());
        }
        RuntimeWorld::load_from_dir(world_dir)
            .map_err(|err| {
                format!(
                    "load execution world from {} failed: {:?}",
                    world_dir.display(),
                    err
                )
            })
            .map(|world| {
                world.with_release_security_policy(ReleaseSecurityPolicy::production_hardened())
            })
    }
}

const DEFAULT_RECENT_MINT_RECORD_LIMIT: usize = 20;

#[derive(Debug)]
struct RuntimePaths {
    runtime_root: PathBuf,
    execution_bridge_state_path: PathBuf,
    execution_world_dir: PathBuf,
    execution_records_dir: PathBuf,
    storage_root: PathBuf,
    replication_root: PathBuf,
    reward_runtime_state_path: PathBuf,
    reward_runtime_distfs_probe_state_path: PathBuf,
    reward_runtime_report_dir: PathBuf,
    reward_runtime_storage_metrics_path: PathBuf,
}

#[derive(Debug)]
struct ChainStatusServer {
    stop_tx: Sender<()>,
    error_rx: Receiver<String>,
    join_handle: Option<thread::JoinHandle<()>>,
}

#[derive(Debug, Serialize)]
struct ChainStatusResponse {
    ok: bool,
    observed_at_unix_ms: i64,
    node_id: String,
    world_id: String,
    role: String,
    running: bool,
    worker_poll_count: u64,
    // Legacy alias kept for existing tooling; same value as worker_poll_count.
    tick_count: u64,
    last_tick_unix_ms: Option<i64>,
    consensus: ChainConsensusStatus,
    last_error: Option<String>,
    execution_world_dir: String,
    p2p: ChainP2pStatus,
    release_security_policy: ReleaseSecurityPolicy,
    reward_runtime: reward_runtime_worker::RewardRuntimeMetricsSnapshot,
    storage: storage_metrics::StorageMetricsSnapshot,
}

#[derive(Debug, Serialize)]
struct ChainP2pStatus {
    requested_user_mode: String,
    recommended_user_mode: String,
    effective_user_mode: String,
    applied_effective_user_mode: Option<String>,
    requires_explicit_public_entry_confirmation: bool,
    detected_reachability: Option<String>,
    hole_punch_viability: String,
    relay_available: bool,
    probe_stable: bool,
    deployment_mode: String,
    node_role_claim: String,
    rationale: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ChainConsensusStatus {
    slot: u64,
    epoch: u64,
    ticks_per_slot: u64,
    tick_phase: u64,
    proposal_tick_phase: u64,
    last_observed_slot: u64,
    missed_slot_count: u64,
    last_observed_tick: u64,
    missed_tick_count: u64,
    adaptive_tick_scheduler_enabled: bool,
    latest_height: u64,
    committed_height: u64,
    network_committed_height: u64,
    last_status: Option<String>,
    last_block_hash: Option<String>,
    last_execution_height: u64,
    last_execution_block_hash: Option<String>,
    last_execution_state_root: Option<String>,
    known_peer_heads: usize,
}

#[derive(Debug, Serialize)]
struct ChainBalancesResponse {
    ok: bool,
    observed_at_unix_ms: i64,
    node_id: String,
    world_id: String,
    execution_world_dir: String,
    load_error: Option<String>,
    node_asset_balance: Option<NodeAssetBalance>,
    node_power_credit_balance: u64,
    node_main_token_account: Option<String>,
    node_main_token_liquid_balance: u64,
    reward_mint_record_count: usize,
    recent_reward_mint_records: Vec<NodeRewardMintRecord>,
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

    if let Err(err) = run_chain_runtime(options) {
        eprintln!("oasis7_chain_runtime failed: {err}");
        process::exit(1);
    }
}

fn run_chain_runtime(options: CliOptions) -> Result<(), String> {
    let paths = resolve_runtime_paths(&options);
    let keypair = node_keypair_config::ensure_node_keypair_in_config(Path::new(
        options.config_path.as_str(),
    ))?;
    let storage_profile_config = StorageProfileConfig::from(options.storage_profile);
    let release_security_policy =
        release_security_policy_for_storage_profile(options.storage_profile);

    let mut config = NodeConfig::new(
        options.node_id.clone(),
        options.world_id.clone(),
        options.node_role,
    )
    .and_then(|cfg| cfg.with_tick_interval(Duration::from_millis(options.node_tick_ms)))
    .map_err(|err| format!("failed to build node config: {err:?}"))?;
    config = config
        .with_network_policy(build_node_network_policy(&options))
        .map_err(|err| format!("failed to apply network policy: {err:?}"))?;

    let validators = if options.node_validators.is_empty() {
        config.pos_config.validators.clone()
    } else {
        options.node_validators.clone()
    };
    let validator_signer_bindings =
        build_validator_signer_public_keys(validators.as_slice(), &keypair)?;
    let mut pos_config = NodePosConfig::ethereum_like(validators.clone())
        .with_validator_signer_public_keys(validator_signer_bindings.clone())
        .map_err(|err| format!("failed to apply validator signer bindings: {err:?}"))?;
    pos_config.slot_duration_ms = options.pos_slot_duration_ms;
    pos_config.slot_clock_genesis_unix_ms = options.pos_slot_clock_genesis_unix_ms;
    pos_config = pos_config
        .with_ticks_per_slot(options.pos_ticks_per_slot)
        .and_then(|cfg| cfg.with_proposal_tick_phase(options.pos_proposal_tick_phase))
        .and_then(|cfg| cfg.with_max_past_slot_lag(options.pos_max_past_slot_lag))
        .map_err(|err| format!("failed to apply PoS clock options: {err:?}"))?
        .with_adaptive_tick_scheduler_enabled(options.pos_adaptive_tick_scheduler_enabled);
    config = config
        .with_pos_config(pos_config)
        .map_err(|err| format!("failed to apply node pos config: {err:?}"))?;
    config = config.with_auto_attest_all_validators(options.node_auto_attest_all_validators);

    let require_execution = matches!(options.node_role, NodeRole::Sequencer);
    config = config
        .with_require_execution_on_commit(require_execution)
        .with_require_peer_execution_hashes(require_execution);

    if !options.node_gossip_peers.is_empty() && options.node_gossip_bind.is_none() {
        return Err("--node-gossip-peer requires --node-gossip-bind".to_string());
    }
    if let Some(bind_addr) = options.node_gossip_bind {
        config = config.with_gossip_optional(bind_addr, options.node_gossip_peers.clone());
    }

    config = config.with_replication(build_node_replication_config(
        options.node_id.as_str(),
        &keypair,
        &storage_profile_config,
    )?);
    if let Some(feedback_p2p_config) = feedback_p2p_config_for_role(options.node_role) {
        config = config
            .with_feedback_p2p(feedback_p2p_config)
            .map_err(|err| format!("failed to enable node feedback p2p: {err:?}"))?;
    }
    config = governance_registry::apply_world_governance_registry_overrides(
        config,
        paths.execution_world_dir.as_path(),
    )?;

    let mut runtime = NodeRuntime::new(config);
    if require_execution {
        let execution_driver = NodeRuntimeExecutionDriver::new_with_storage_profile(
            paths.execution_bridge_state_path.clone(),
            paths.execution_world_dir.clone(),
            paths.execution_records_dir.clone(),
            paths.storage_root.clone(),
            &storage_profile_config,
        )
        .map_err(|err| format!("failed to initialize execution driver: {err}"))?;
        runtime = runtime.with_execution_hook(execution_driver);
    }
    let (mut runtime, replication_network) =
        attach_default_replication_network(runtime, &options, &keypair)?;

    runtime
        .start()
        .map_err(|err| format!("failed to start node runtime: {err:?}"))?;

    let runtime = Arc::new(Mutex::new(runtime));
    let signer_node_id = options
        .reward_runtime_signer_node_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| options.node_id.clone());
    let signer_keypair = derive_node_consensus_signer_keypair(signer_node_id.as_str(), &keypair)
        .map_err(|err| {
            format!("failed to derive reward runtime signer keypair for {signer_node_id}: {err}")
        })?;
    let mut reward_runtime_node_identity_bindings = validator_signer_bindings;
    if !reward_runtime_node_identity_bindings.contains_key(signer_node_id.as_str()) {
        reward_runtime_node_identity_bindings.insert(
            signer_node_id.clone(),
            signer_keypair.public_key_hex.clone(),
        );
    }
    if !reward_runtime_node_identity_bindings.contains_key(options.node_id.as_str()) {
        let local_keypair =
            derive_node_consensus_signer_keypair(options.node_id.as_str(), &keypair)
                .map_err(|err| format!("failed to derive local node signer keypair: {err}"))?;
        reward_runtime_node_identity_bindings
            .insert(options.node_id.clone(), local_keypair.public_key_hex);
    }
    let reward_runtime_config = RewardRuntimeWorkerConfig {
        enabled: options.reward_runtime_enabled,
        poll_interval: Duration::from_millis(options.node_tick_ms),
        world_id: options.world_id.clone(),
        local_node_id: options.node_id.clone(),
        report_dir: paths.reward_runtime_report_dir.clone(),
        state_path: paths.reward_runtime_state_path.clone(),
        distfs_probe_state_path: paths.reward_runtime_distfs_probe_state_path.clone(),
        storage_root: paths.storage_root.clone(),
        signer_node_id,
        signer_private_key_hex: signer_keypair.private_key_hex,
        reward_runtime_epoch_duration_secs: options.reward_runtime_epoch_duration_secs,
        reward_runtime_auto_redeem: options.reward_runtime_auto_redeem,
        reward_asset_config: RewardAssetConfig {
            points_per_credit: options.reward_points_per_credit,
            ..RewardAssetConfig::default()
        },
        reward_initial_reserve_power_units: options.reward_initial_reserve_power_units,
        reward_runtime_node_identity_bindings,
        reward_distfs_probe_config: options.reward_distfs_probe_config,
    };
    let reward_runtime_metrics = init_shared_metrics(&reward_runtime_config);
    let storage_metrics = storage_metrics::init_shared_storage_metrics(options.storage_profile);
    if let Err(err) = storage_metrics::refresh_shared_storage_metrics(
        &storage_metrics,
        &paths,
        options.storage_profile,
        None,
    ) {
        eprintln!("warning: initial storage metrics refresh failed: {err}");
    }
    let mut reward_runtime_worker = start_reward_runtime_worker(
        Arc::clone(&runtime),
        reward_runtime_config,
        Arc::clone(&reward_runtime_metrics),
    )?;
    let feedback_submit_signer = build_feedback_submit_signer(options.node_id.as_str(), &keypair)?;
    let effective_p2p_policy = build_node_network_policy(&options);
    let (status_host, status_port) =
        parse_host_port(options.status_bind.as_str(), "--status-bind")?;
    let mut status_server = start_chain_status_server(
        status_host.as_str(),
        status_port,
        Arc::clone(&runtime),
        Arc::clone(&replication_network),
        options.clone(),
        options.node_id.clone(),
        options.world_id.clone(),
        paths.execution_world_dir.clone(),
        release_security_policy,
        effective_p2p_policy,
        Arc::clone(&reward_runtime_metrics),
        Arc::clone(&storage_metrics),
        feedback_submit_signer,
    )?;

    println!("oasis7_chain_runtime ready.");
    println!("- node_id: {}", options.node_id);
    println!("- world_id: {}", options.world_id);
    println!("- storage_profile: {}", options.storage_profile.as_str());
    println!("- role: {}", options.node_role.as_str());
    println!(
        "- status: http://{}:{}/v1/chain/status",
        status_host, status_port
    );
    println!(
        "- balances: http://{}:{}/v1/chain/balances",
        status_host, status_port
    );
    println!(
        "- feedback_submit: http://{}:{}/v1/chain/feedback/submit",
        status_host, status_port
    );
    println!(
        "- module_release_attestation_submit: http://{}:{}/v1/chain/module-release/attestation/submit",
        status_host, status_port
    );
    println!(
        "- reward_runtime: {} ({})",
        if options.reward_runtime_enabled {
            "enabled"
        } else {
            "disabled"
        },
        paths.reward_runtime_report_dir.display()
    );
    println!("Press Ctrl+C to stop.");

    let mut last_error = String::new();
    let mut current_degraded_reason: Option<String> = None;
    let mut last_storage_metrics_refresh = Instant::now()
        .checked_sub(Duration::from_secs(2))
        .unwrap_or_else(Instant::now);
    loop {
        if let Some(server_err) = poll_chain_status_server_error(&mut status_server)? {
            stop_chain_status_server(&mut status_server);
            stop_reward_runtime_worker(&mut reward_runtime_worker);
            stop_runtime(&runtime);
            return Err(server_err);
        }
        if let Some(worker) = reward_runtime_worker.as_mut() {
            if let Some(worker_err) = poll_worker_error(worker)? {
                stop_chain_status_server(&mut status_server);
                stop_reward_runtime_worker(&mut reward_runtime_worker);
                stop_runtime(&runtime);
                return Err(worker_err);
            }
        }

        if let Ok(snapshot) = runtime.lock().map(|locked| locked.snapshot()) {
            current_degraded_reason = snapshot.last_error.clone();
            if let Some(err) = snapshot.last_error {
                if err != last_error {
                    eprintln!("node runtime reported error: {err}");
                    last_error = err;
                }
            }
        }
        if last_storage_metrics_refresh.elapsed() >= Duration::from_secs(1) {
            if let Err(err) = storage_metrics::refresh_shared_storage_metrics(
                &storage_metrics,
                &paths,
                options.storage_profile,
                current_degraded_reason.clone(),
            ) {
                eprintln!("warning: storage metrics refresh failed: {err}");
            }
            last_storage_metrics_refresh = Instant::now();
        }

        thread::sleep(Duration::from_millis(300));
    }
}

fn feedback_p2p_config_for_role(node_role: NodeRole) -> Option<NodeFeedbackP2pConfig> {
    match node_role {
        NodeRole::Observer => None,
        NodeRole::Sequencer | NodeRole::Storage => Some(NodeFeedbackP2pConfig::default()),
    }
}

fn resolve_runtime_paths(options: &CliOptions) -> RuntimePaths {
    let root = Path::new("output")
        .join("chain-runtime")
        .join(options.node_id.as_str());
    RuntimePaths {
        runtime_root: root.clone(),
        execution_bridge_state_path: options
            .execution_bridge_state_path
            .clone()
            .unwrap_or_else(|| root.join("reward-runtime-execution-bridge-state.json")),
        execution_world_dir: options
            .execution_world_dir
            .clone()
            .unwrap_or_else(|| root.join("reward-runtime-execution-world")),
        execution_records_dir: options
            .execution_records_dir
            .clone()
            .unwrap_or_else(|| root.join("reward-runtime-execution-records")),
        storage_root: options
            .storage_root
            .clone()
            .unwrap_or_else(|| root.join("store")),
        replication_root: Path::new("output")
            .join("node-distfs")
            .join(options.node_id.as_str()),
        reward_runtime_state_path: root.join(DEFAULT_REWARD_RUNTIME_STATE_FILE),
        reward_runtime_distfs_probe_state_path: root
            .join(DEFAULT_REWARD_RUNTIME_DISTFS_PROBE_STATE_FILE),
        reward_runtime_report_dir: root.join(DEFAULT_REWARD_RUNTIME_REPORT_DIR),
        reward_runtime_storage_metrics_path: root.join(DEFAULT_REWARD_RUNTIME_STORAGE_METRICS_FILE),
    }
}

fn stop_runtime(runtime: &Arc<Mutex<NodeRuntime>>) {
    let mut locked = match runtime.lock() {
        Ok(locked) => locked,
        Err(_) => {
            eprintln!("failed to stop node runtime: lock poisoned");
            return;
        }
    };
    if let Err(err) = locked.stop() {
        eprintln!("failed to stop node runtime: {err:?}");
    }
}

fn start_chain_status_server(
    host: &str,
    port: u16,
    runtime: Arc<Mutex<NodeRuntime>>,
    replication_network: Arc<Libp2pReplicationNetwork>,
    options: CliOptions,
    node_id: String,
    world_id: String,
    execution_world_dir: PathBuf,
    release_security_policy: ReleaseSecurityPolicy,
    effective_p2p_policy: NodeNetworkPolicy,
    reward_runtime_metrics: SharedRewardRuntimeMetrics,
    storage_metrics: storage_metrics::SharedStorageMetrics,
    feedback_submit_signer: FeedbackSubmitSigner,
) -> Result<ChainStatusServer, String> {
    let listener = TcpListener::bind((host, port))
        .map_err(|err| format!("failed to bind status server at {host}:{port}: {err}"))?;
    listener
        .set_nonblocking(true)
        .map_err(|err| format!("failed to set status server listener nonblocking: {err}"))?;

    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let (error_tx, error_rx) = mpsc::channel::<String>();

    let join_handle = thread::spawn(move || {
        if let Err(err) = run_chain_status_server_loop(
            listener,
            stop_rx,
            runtime,
            replication_network,
            options,
            node_id,
            world_id,
            execution_world_dir,
            release_security_policy,
            effective_p2p_policy,
            reward_runtime_metrics,
            storage_metrics,
            feedback_submit_signer,
        ) {
            let _ = error_tx.send(err);
        }
    });

    Ok(ChainStatusServer {
        stop_tx,
        error_rx,
        join_handle: Some(join_handle),
    })
}

fn run_chain_status_server_loop(
    listener: TcpListener,
    stop_rx: Receiver<()>,
    runtime: Arc<Mutex<NodeRuntime>>,
    replication_network: Arc<Libp2pReplicationNetwork>,
    options: CliOptions,
    node_id: String,
    world_id: String,
    execution_world_dir: PathBuf,
    release_security_policy: ReleaseSecurityPolicy,
    effective_p2p_policy: NodeNetworkPolicy,
    reward_runtime_metrics: SharedRewardRuntimeMetrics,
    storage_metrics: storage_metrics::SharedStorageMetrics,
    feedback_submit_signer: FeedbackSubmitSigner,
) -> Result<(), String> {
    loop {
        match stop_rx.try_recv() {
            Ok(_) | Err(TryRecvError::Disconnected) => return Ok(()),
            Err(TryRecvError::Empty) => {}
        }

        match listener.accept() {
            Ok((stream, _addr)) => {
                let runtime = Arc::clone(&runtime);
                let replication_network = Arc::clone(&replication_network);
                let options = options.clone();
                let node_id = node_id.clone();
                let world_id = world_id.clone();
                let execution_world_dir = execution_world_dir.clone();
                let release_security_policy = release_security_policy.clone();
                let effective_p2p_policy = effective_p2p_policy.clone();
                let reward_runtime_metrics = Arc::clone(&reward_runtime_metrics);
                let storage_metrics = Arc::clone(&storage_metrics);
                let feedback_submit_signer = feedback_submit_signer.clone();
                thread::spawn(move || {
                    if let Err(err) = handle_chain_status_connection(
                        stream,
                        runtime,
                        replication_network,
                        &options,
                        node_id.as_str(),
                        world_id.as_str(),
                        execution_world_dir.as_path(),
                        &release_security_policy,
                        effective_p2p_policy,
                        reward_runtime_metrics,
                        storage_metrics,
                        &feedback_submit_signer,
                    ) {
                        eprintln!("warning: chain status connection failed: {err}");
                    }
                });
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(err) => {
                return Err(format!("chain status server accept failed: {err}"));
            }
        }
    }
}

fn handle_chain_status_connection(
    mut stream: TcpStream,
    runtime: Arc<Mutex<NodeRuntime>>,
    replication_network: Arc<Libp2pReplicationNetwork>,
    options: &CliOptions,
    node_id: &str,
    world_id: &str,
    execution_world_dir: &Path,
    release_security_policy: &ReleaseSecurityPolicy,
    effective_p2p_policy: NodeNetworkPolicy,
    reward_runtime_metrics: SharedRewardRuntimeMetrics,
    storage_metrics: storage_metrics::SharedStorageMetrics,
    feedback_submit_signer: &FeedbackSubmitSigner,
) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|err| format!("failed to set read timeout: {err}"))?;

    let mut buffer = [0_u8; 65_536];
    let bytes = stream
        .read(&mut buffer)
        .map_err(|err| format!("failed to read request: {err}"))?;
    if bytes == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..bytes]);
    let Some(line) = request.lines().next() else {
        write_json_response(&mut stream, 400, b"{\"error\":\"bad request\"}", false)
            .map_err(|err| format!("failed to write 400 response: {err}"))?;
        return Ok(());
    };

    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let target = parts.next().unwrap_or_default();
    let path = target.split('?').next().unwrap_or(target);
    let head_only = method.eq_ignore_ascii_case("HEAD");

    if transfer_submit_api::maybe_handle_transfer_submit_request(
        &mut stream,
        &buffer[..bytes],
        &runtime,
        method,
        path,
        node_id,
        world_id,
        execution_world_dir,
    )? {
        return Ok(());
    }

    if module_release_attestation_submit_api::maybe_handle_module_release_attestation_submit_request(
        &mut stream,
        &buffer[..bytes],
        &runtime,
        method,
        path,
    )? {
        return Ok(());
    }

    if method.eq_ignore_ascii_case("POST") && path == "/v1/chain/feedback/submit" {
        let body = match extract_http_json_body(&buffer[..bytes]) {
            Ok(body) => body,
            Err(err) => {
                write_feedback_submit_error(&mut stream, 400, err.as_str())?;
                return Ok(());
            }
        };
        let submit_request = match parse_feedback_submit_request(body) {
            Ok(request) => request,
            Err(err) => {
                write_feedback_submit_error(&mut stream, 400, err.as_str())?;
                return Ok(());
            }
        };
        let submit_ip = stream
            .peer_addr()
            .map(|addr| addr.ip().to_string())
            .unwrap_or_else(|_| "127.0.0.1".to_string());
        let create_request = match build_feedback_create_request(
            submit_request,
            feedback_submit_signer,
            node_id,
            submit_ip.as_str(),
            now_unix_ms(),
        ) {
            Ok(request) => request,
            Err(err) => {
                write_feedback_submit_error(&mut stream, 400, err.as_str())?;
                return Ok(());
            }
        };
        let receipt = match runtime
            .lock()
            .map_err(|_| "failed to lock node runtime for feedback submit".to_string())?
            .submit_feedback(create_request)
        {
            Ok(receipt) => receipt,
            Err(err) => {
                write_feedback_submit_error(
                    &mut stream,
                    502,
                    format!("feedback submit failed: {err}").as_str(),
                )?;
                return Ok(());
            }
        };
        let response = ChainFeedbackSubmitResponse::success(&receipt);
        let body = serde_json::to_vec_pretty(&response)
            .map_err(|err| format!("failed to encode feedback submit response: {err}"))?;
        write_json_response(&mut stream, 200, body.as_slice(), false)
            .map_err(|err| format!("failed to write /v1/chain/feedback/submit response: {err}"))?;
        return Ok(());
    }

    if !method.eq_ignore_ascii_case("GET") && !head_only {
        write_json_response(
            &mut stream,
            405,
            b"{\"error\":\"method not allowed\"}",
            head_only,
        )
        .map_err(|err| format!("failed to write 405 response: {err}"))?;
        return Ok(());
    }

    match path {
        "/healthz" => {
            write_json_response(&mut stream, 200, b"{\"ok\":true}", head_only)
                .map_err(|err| format!("failed to write /healthz response: {err}"))?;
        }
        "/v1/chain/status" => {
            let snapshot = runtime
                .lock()
                .map_err(|_| "failed to read node runtime snapshot: lock poisoned".to_string())?
                .snapshot();
            let live_snapshot = replication_network.reachability_snapshot();
            let (p2p_recommendation, p2p_detection) =
                build_live_node_network_policy_recommendation(options, Some(&live_snapshot))?;
            let applied_effective_user_mode =
                applied_runtime_user_mode_label(options).map(str::to_string);
            let payload = build_chain_status_payload(
                snapshot,
                execution_world_dir,
                &p2p_recommendation,
                applied_effective_user_mode,
                effective_p2p_policy,
                p2p_detection,
                release_security_policy.clone(),
                snapshot_metrics(&reward_runtime_metrics),
                storage_metrics::snapshot_storage_metrics(&storage_metrics),
            );
            let body = serde_json::to_vec_pretty(&payload)
                .map_err(|err| format!("failed to encode status payload: {err}"))?;
            write_json_response(&mut stream, 200, body.as_slice(), head_only)
                .map_err(|err| format!("failed to write /v1/chain/status response: {err}"))?;
        }
        "/v1/chain/balances" => {
            let payload = build_chain_balances_payload(node_id, world_id, execution_world_dir);
            let body = serde_json::to_vec_pretty(&payload)
                .map_err(|err| format!("failed to encode balances payload: {err}"))?;
            write_json_response(&mut stream, 200, body.as_slice(), head_only)
                .map_err(|err| format!("failed to write /v1/chain/balances response: {err}"))?;
        }
        _ => {
            write_json_response(&mut stream, 404, b"{\"error\":\"not found\"}", head_only)
                .map_err(|err| format!("failed to write 404 response: {err}"))?;
        }
    }

    Ok(())
}

fn build_chain_status_payload(
    snapshot: NodeSnapshot,
    execution_world_dir: &Path,
    live_p2p_recommendation: &NodeUserModeRecommendation,
    applied_effective_user_mode: Option<String>,
    effective_p2p_policy: NodeNetworkPolicy,
    p2p_detection: NodeReachabilityAutoDetection,
    release_security_policy: ReleaseSecurityPolicy,
    reward_runtime_metrics: reward_runtime_worker::RewardRuntimeMetricsSnapshot,
    storage_metrics: storage_metrics::StorageMetricsSnapshot,
) -> ChainStatusResponse {
    let last_status = snapshot
        .consensus
        .last_status
        .map(consensus_status_to_string);

    ChainStatusResponse {
        ok: true,
        observed_at_unix_ms: now_unix_ms(),
        node_id: snapshot.node_id,
        world_id: snapshot.world_id,
        role: snapshot.role.as_str().to_string(),
        running: snapshot.running,
        worker_poll_count: snapshot.tick_count,
        tick_count: snapshot.tick_count,
        last_tick_unix_ms: snapshot.last_tick_unix_ms,
        consensus: ChainConsensusStatus {
            slot: snapshot.consensus.slot,
            epoch: snapshot.consensus.epoch,
            ticks_per_slot: snapshot.consensus.ticks_per_slot,
            tick_phase: snapshot.consensus.tick_phase,
            proposal_tick_phase: snapshot.consensus.proposal_tick_phase,
            last_observed_slot: snapshot.consensus.last_observed_slot,
            missed_slot_count: snapshot.consensus.missed_slot_count,
            last_observed_tick: snapshot.consensus.last_observed_tick,
            missed_tick_count: snapshot.consensus.missed_tick_count,
            adaptive_tick_scheduler_enabled: snapshot.consensus.adaptive_tick_scheduler_enabled,
            latest_height: snapshot.consensus.latest_height,
            committed_height: snapshot.consensus.committed_height,
            network_committed_height: snapshot.consensus.network_committed_height,
            last_status,
            last_block_hash: snapshot.consensus.last_block_hash,
            last_execution_height: snapshot.consensus.last_execution_height,
            last_execution_block_hash: snapshot.consensus.last_execution_block_hash,
            last_execution_state_root: snapshot.consensus.last_execution_state_root,
            known_peer_heads: snapshot.consensus.known_peer_heads,
        },
        last_error: snapshot.last_error,
        execution_world_dir: execution_world_dir.display().to_string(),
        p2p: ChainP2pStatus {
            requested_user_mode: live_p2p_recommendation
                .requested_user_mode
                .as_str()
                .to_string(),
            recommended_user_mode: live_p2p_recommendation
                .recommended_user_mode
                .as_str()
                .to_string(),
            effective_user_mode: live_p2p_recommendation
                .effective_user_mode
                .as_str()
                .to_string(),
            applied_effective_user_mode,
            requires_explicit_public_entry_confirmation: live_p2p_recommendation
                .requires_explicit_public_entry_confirmation,
            detected_reachability: p2p_detection
                .observed_reachability
                .map(peer_reachability_as_str)
                .map(str::to_string),
            hole_punch_viability: p2p_detection.hole_punch_viability.to_string(),
            relay_available: p2p_detection.relay_available,
            probe_stable: p2p_detection.probe_stable,
            deployment_mode: effective_p2p_policy.deployment_mode.as_str().to_string(),
            node_role_claim: effective_p2p_policy.node_role_claim.as_str().to_string(),
            rationale: live_p2p_recommendation.rationale.clone(),
        },
        release_security_policy,
        reward_runtime: reward_runtime_metrics,
        storage: storage_metrics,
    }
}

pub(crate) fn release_security_policy_for_storage_profile(
    storage_profile: StorageProfile,
) -> ReleaseSecurityPolicy {
    if matches!(storage_profile, StorageProfile::ReleaseDefault) {
        ReleaseSecurityPolicy::production_hardened()
    } else {
        ReleaseSecurityPolicy::default()
    }
}

fn poll_chain_status_server_error(
    server: &mut ChainStatusServer,
) -> Result<Option<String>, String> {
    match server.error_rx.try_recv() {
        Ok(err) => Ok(Some(format!("status server failed: {err}"))),
        Err(TryRecvError::Disconnected) => Ok(Some(
            "status server channel disconnected unexpectedly".to_string(),
        )),
        Err(TryRecvError::Empty) => {
            if let Some(handle) = server.join_handle.as_ref() {
                if handle.is_finished() {
                    return Ok(Some("status server exited unexpectedly".to_string()));
                }
            }
            Ok(None)
        }
    }
}

fn stop_chain_status_server(server: &mut ChainStatusServer) {
    let _ = server.stop_tx.send(());
    if let Some(handle) = server.join_handle.take() {
        let _ = handle.join();
    }
}

fn write_json_response(
    stream: &mut TcpStream,
    status_code: u16,
    body: &[u8],
    head_only: bool,
) -> std::io::Result<()> {
    let status_text = match status_code {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        502 => "Bad Gateway",
        _ => "Internal Server Error",
    };
    let headers = format!(
        "HTTP/1.1 {status_code} {status_text}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes())?;
    if !head_only {
        stream.write_all(body)?;
    }
    stream.flush()?;
    Ok(())
}

fn consensus_status_to_string(status: PosConsensusStatus) -> String {
    match status {
        PosConsensusStatus::Pending => "pending".to_string(),
        PosConsensusStatus::Committed => "committed".to_string(),
        PosConsensusStatus::Rejected => "rejected".to_string(),
    }
}

fn build_node_replication_config(
    node_id: &str,
    keypair: &node_keypair_config::NodeKeypairConfig,
    storage_profile: &StorageProfileConfig,
) -> Result<NodeReplicationConfig, String> {
    let signer_keypair = derive_node_consensus_signer_keypair(node_id, keypair)?;
    let replication_root = Path::new("output").join("node-distfs").join(node_id);
    NodeReplicationConfig::new(replication_root)
        .and_then(|cfg| {
            cfg.with_max_hot_commit_messages(storage_profile.replication_max_hot_commit_messages)
        })
        .and_then(|cfg| {
            cfg.with_signing_keypair(
                signer_keypair.private_key_hex,
                signer_keypair.public_key_hex,
            )
        })
        .map_err(|err| format!("failed to build node replication config: {err:?}"))
}

fn attach_default_replication_network(
    runtime: NodeRuntime,
    options: &CliOptions,
    root_keypair: &node_keypair_config::NodeKeypairConfig,
) -> Result<(NodeRuntime, Arc<Libp2pReplicationNetwork>), String> {
    let mut network_config = build_default_replication_network_config(options, root_keypair)?;
    network_config.allow_local_handler_fallback_when_no_peers =
        network_config.bootstrap_peers.is_empty();
    let network = Arc::new(Libp2pReplicationNetwork::new(network_config));
    let handle_network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<oasis7_proto::world_error::WorldError>
            + Send
            + Sync,
    > = network.clone();
    let handle_dht: Arc<
        dyn oasis7_proto::distributed_dht::DistributedDht<oasis7_proto::world_error::WorldError>
            + Send
            + Sync,
    > = network.clone();
    let runtime = runtime
        .with_replication_network(
            NodeReplicationNetworkHandle::new(handle_network)
                .with_dht(handle_dht.clone())
                .with_local_provider_id(network.peer_id().to_string()),
        )
        .with_replica_maintenance_dht(handle_dht)
        .with_replication_network_consensus_enabled(false);
    Ok((runtime, network))
}

fn build_default_replication_network_config(
    options: &CliOptions,
    root_keypair: &node_keypair_config::NodeKeypairConfig,
) -> Result<Libp2pReplicationNetworkConfig, String> {
    let mut config = Libp2pReplicationNetworkConfig::default();
    let libp2p_identity_keypair =
        derive_node_libp2p_identity_keypair_config(options.node_id.as_str(), root_keypair)?;
    config.keypair = Some(
        derive_libp2p_identity_keypair(libp2p_identity_keypair.private_key_hex.as_str())
            .map_err(|err| format!("failed to derive libp2p identity keypair: {err:?}"))?,
    );
    config.peer_record = Some(build_default_peer_record(options));
    if options.replication_network_listen_addrs.is_empty() {
        config
            .listen_addrs
            .push(DEFAULT_REPLICATION_NETWORK_LISTEN.parse().map_err(|err| {
                format!(
                    "failed to parse default replication listen address {}: {err}",
                    DEFAULT_REPLICATION_NETWORK_LISTEN
                )
            })?);
    } else {
        config.listen_addrs = options
            .replication_network_listen_addrs
            .iter()
            .map(|raw| {
                raw.parse()
                    .map_err(|err| format!("invalid --replication-network-listen {raw}: {err}"))
            })
            .collect::<Result<_, _>>()?;
    }
    config.bootstrap_peers = options
        .replication_network_bootstrap_peers
        .iter()
        .map(|raw| {
            raw.parse()
                .map_err(|err| format!("invalid --replication-network-peer {raw}: {err}"))
        })
        .collect::<Result<_, _>>()?;
    Ok(config)
}

fn build_default_peer_record(options: &CliOptions) -> PeerRecord {
    let network_policy = build_node_network_policy(options);
    PeerRecord {
        peer_id: String::new(),
        node_id: options.node_id.clone(),
        world_id: options.world_id.clone(),
        network_id: options.world_id.clone(),
        node_role: network_policy.node_role_claim.as_str().to_string(),
        deployment_mode: network_policy.deployment_mode,
        reachability_class: network_policy.advertised_reachability_class(),
        direct_addrs: Vec::new(),
        hole_punch_addrs: Vec::new(),
        relay_addrs: Vec::new(),
        discovery_sources: vec![
            PeerDiscoverySource::StaticBootstrap,
            PeerDiscoverySource::Dht,
        ],
        capability_lanes: network_policy.node_role_claim.default_capability_lanes(),
        source_operator: options.p2p_source_operator.clone(),
        source_asn: options.p2p_source_asn.clone(),
        published_at_ms: 0,
        ttl_ms: 60 * 60 * 1000,
    }
}

fn build_feedback_submit_signer(
    node_id: &str,
    root_keypair: &node_keypair_config::NodeKeypairConfig,
) -> Result<FeedbackSubmitSigner, String> {
    let feedback_signer_id = format!("{node_id}-feedback-submit");
    let signer = derive_node_consensus_signer_keypair(feedback_signer_id.as_str(), root_keypair)?;
    Ok(FeedbackSubmitSigner {
        private_key_hex: signer.private_key_hex,
        public_key_hex: signer.public_key_hex,
    })
}

fn derive_node_consensus_signer_keypair(
    node_id: &str,
    root_keypair: &node_keypair_config::NodeKeypairConfig,
) -> Result<node_keypair_config::NodeKeypairConfig, String> {
    derive_node_scoped_keypair(
        node_id,
        root_keypair,
        b"oasis7-node-consensus-signer-v1",
        "node consensus signer",
    )
}

fn derive_node_libp2p_identity_keypair_config(
    node_id: &str,
    root_keypair: &node_keypair_config::NodeKeypairConfig,
) -> Result<node_keypair_config::NodeKeypairConfig, String> {
    derive_node_scoped_keypair(
        node_id,
        root_keypair,
        b"oasis7-node-libp2p-identity-v1",
        "node libp2p identity",
    )
}

fn derive_node_scoped_keypair(
    node_id: &str,
    root_keypair: &node_keypair_config::NodeKeypairConfig,
    namespace: &[u8],
    label: &str,
) -> Result<node_keypair_config::NodeKeypairConfig, String> {
    let node_id = node_id.trim();
    if node_id.is_empty() {
        return Err(format!("{label} derivation requires non-empty node_id"));
    }
    let root_private_bytes = hex::decode(root_keypair.private_key_hex.as_str())
        .map_err(|_| "root node.private_key must be valid hex".to_string())?;
    let root_private: [u8; 32] = root_private_bytes
        .try_into()
        .map_err(|_| "root node.private_key must be 32-byte hex".to_string())?;

    let mut hasher = Sha256::new();
    hasher.update(namespace);
    hasher.update(root_private);
    hasher.update(b"|");
    hasher.update(node_id.as_bytes());
    let digest = hasher.finalize();

    let mut derived_private = [0_u8; 32];
    derived_private.copy_from_slice(&digest[..32]);
    let signing_key = SigningKey::from_bytes(&derived_private);
    Ok(node_keypair_config::NodeKeypairConfig {
        private_key_hex: hex::encode(signing_key.to_bytes()),
        public_key_hex: hex::encode(signing_key.verifying_key().to_bytes()),
    })
}

fn build_validator_signer_public_keys(
    validators: &[PosValidator],
    root_keypair: &node_keypair_config::NodeKeypairConfig,
) -> Result<BTreeMap<String, String>, String> {
    let mut bindings = BTreeMap::new();
    for validator in validators {
        let validator_id = validator.validator_id.trim();
        if validator_id.is_empty() {
            return Err("validator_id cannot be empty when deriving signer bindings".to_string());
        }
        let keypair = derive_node_consensus_signer_keypair(validator_id, root_keypair)?;
        bindings.insert(validator_id.to_string(), keypair.public_key_hex);
    }
    Ok(bindings)
}

#[allow(dead_code)]
fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("create state dir {} failed: {}", parent.display(), err))?;
        }
    }
    let temp_path = path.with_extension("json.tmp");
    fs::write(&temp_path, bytes)
        .map_err(|err| format!("write state temp {} failed: {}", temp_path.display(), err))?;
    fs::rename(&temp_path, path).map_err(|err| {
        format!(
            "rename state temp {} -> {} failed: {}",
            temp_path.display(),
            path.display(),
            err
        )
    })
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "oasis7_chain_runtime/execution_bridge_real_tests.rs"]
mod execution_bridge_real_tests;

#[cfg(test)]
#[path = "oasis7_chain_runtime/oasis7_chain_runtime_tests.rs"]
mod tests;
