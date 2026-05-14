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
use oasis7::observability::{emit_stderr_or_event, init_tracing};
use oasis7::runtime::{
    NodeAssetBalance, NodeRewardMintRecord, ReleaseSecurityPolicy, RewardAssetConfig,
};
use oasis7_node::{
    derive_libp2p_identity_keypair, Libp2pReplicationNetwork, Libp2pReplicationNetworkConfig,
    NodeConfig, NodeNetworkPolicy, NodePosConfig, NodeReplicationConfig,
    NodeReplicationNetworkHandle, NodeRole, NodeRuntime, PosConsensusStatus, PosValidator,
};
use oasis7_proto::distributed_dht::{PeerDiscoverySource, PeerRecord};
use oasis7_proto::storage_profile::{StorageProfile, StorageProfileConfig};
use runtime_status_util::now_unix_ms;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tracing::{error, info, Level};
#[path = "oasis7_chain_runtime/agent_claim_api.rs"]
mod agent_claim_api;
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
#[path = "oasis7_chain_runtime/gameplay_submit_api.rs"]
mod gameplay_submit_api;
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
#[path = "oasis7_chain_runtime/runtime_status_util.rs"]
mod runtime_status_util;
#[path = "oasis7_chain_runtime/status_payload.rs"]
mod status_payload;
#[path = "oasis7_chain_runtime/status_server_support.rs"]
mod status_server_support;
#[path = "oasis7_chain_runtime/storage_metrics.rs"]
mod storage_metrics;
#[path = "oasis7_chain_runtime/traffic_profile.rs"]
mod traffic_profile;
#[path = "oasis7_chain_runtime/traffic_status.rs"]
mod traffic_status;
#[path = "oasis7_chain_runtime/transfer_submit_api.rs"]
mod transfer_submit_api;
#[path = "oasis7_chain_runtime/wasm_status.rs"]
mod wasm_status;
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
    build_node_network_policy,
};
use reward_runtime_worker::{
    init_shared_metrics, poll_worker_error, snapshot_metrics, start_reward_runtime_worker,
    stop_reward_runtime_worker, RewardRuntimeWorkerConfig, SharedRewardRuntimeMetrics,
};
use status_payload::build_chain_status_payload;
#[cfg(test)]
use status_server_support::ChainPeerHealthStatus;
use status_server_support::{
    poll_chain_status_server_error, start_chain_status_server, stop_chain_status_server,
    write_json_response, ChainBalancesResponse, ChainReplicationDebugStatus,
};
#[cfg(test)]
use traffic_profile::feedback_p2p_config_for_role;
use traffic_profile::{
    apply_traffic_profile_to_node_config, apply_traffic_profile_to_replication_network_config,
};
use traffic_status::build_chain_traffic_status;
#[cfg(test)]
use traffic_status::ChainTrafficStatus;
use wasm_status::build_chain_wasm_status;
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

fn main() {
    init_tracing("oasis7_chain_runtime");
    let raw_args: Vec<String> = env::args().skip(1).collect();
    if raw_args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return;
    }

    let options = match parse_options(raw_args.iter().map(|arg| arg.as_str())) {
        Ok(options) => options,
        Err(err) => {
            error!(error = %err, "failed to parse chain runtime options");
            print_help();
            process::exit(1);
        }
    };

    if let Err(err) = run_chain_runtime(options) {
        error!(error = %err, "oasis7_chain_runtime failed");
        process::exit(1);
    }
}

fn run_chain_runtime(options: CliOptions) -> Result<(), String> {
    let mut options = options;
    apply_network_tier_manifest_defaults(&mut options);
    let trace_session_id = oasis7::observability::resolve_trace_session_id("oasis7_chain_runtime");
    info!(
        trace_session_id = %trace_session_id,
        node_id = %options.node_id,
        world_id = %options.world_id,
        node_role = ?options.node_role,
        status_bind = %options.status_bind,
        storage_profile = %options.storage_profile.as_str(),
        traffic_profile = %options.traffic_profile.as_str(),
        reward_runtime_enabled = options.reward_runtime_enabled,
        "starting chain runtime"
    );
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
    let validator_signer_bindings = build_validator_signer_public_keys(
        validators.as_slice(),
        &keypair,
        &options.node_validator_signer_public_keys,
    )?;
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
    config = config
        .with_auto_attest_all_validators(options.node_auto_attest_all_validators)
        .with_allow_local_proposals(matches!(options.node_role, NodeRole::Sequencer));
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

    config = apply_traffic_profile_to_node_config(config, &options)?;
    config = governance_registry::apply_world_governance_registry_overrides(
        config,
        paths.execution_world_dir.as_path(),
    )?;
    let effective_validator_signer_bindings =
        config.pos_config.validator_signer_public_keys.clone();
    let replication_remote_writer_allowlist = build_replication_remote_writer_allowlist(
        effective_validator_signer_bindings.values(),
        &options.replication_remote_writer_public_keys,
    );
    config = config.with_replication(build_node_replication_config(
        options.node_id.as_str(),
        &keypair,
        &storage_profile_config,
        replication_remote_writer_allowlist.as_slice(),
    )?);

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
    let mut reward_runtime_node_identity_bindings = effective_validator_signer_bindings;
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
        emit_stderr_or_event(
            Level::WARN,
            format!("warning: initial storage metrics refresh failed: {err}").as_str(),
            "initial storage metrics refresh failed",
        );
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
        options.loaded_network_tier_manifest.clone(),
        release_security_policy,
        effective_p2p_policy,
        Arc::clone(&reward_runtime_metrics),
        Arc::clone(&storage_metrics),
        feedback_submit_signer,
    )?;

    runtime_status_util::print_runtime_ready_summary(
        &options,
        &paths,
        status_host.as_str(),
        status_port,
    );

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
                    emit_stderr_or_event(
                        Level::WARN,
                        format!("node runtime reported error: {err}").as_str(),
                        "node runtime reported error",
                    );
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
                emit_stderr_or_event(
                    Level::WARN,
                    format!("warning: storage metrics refresh failed: {err}").as_str(),
                    "storage metrics refresh failed",
                );
            }
            last_storage_metrics_refresh = Instant::now();
        }

        thread::sleep(Duration::from_millis(300));
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
            emit_stderr_or_event(
                Level::ERROR,
                "failed to stop node runtime: lock poisoned",
                "failed to stop node runtime: lock poisoned",
            );
            return;
        }
    };
    if let Err(err) = locked.stop() {
        emit_stderr_or_event(
            Level::ERROR,
            format!("failed to stop node runtime: {err:?}").as_str(),
            "failed to stop node runtime",
        );
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

fn build_node_replication_config(
    node_id: &str,
    keypair: &node_keypair_config::NodeKeypairConfig,
    storage_profile: &StorageProfileConfig,
    remote_writer_allowlist: &[String],
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
        .and_then(|cfg| {
            if remote_writer_allowlist.is_empty() {
                Ok(cfg)
            } else {
                cfg.with_remote_writer_allowlist(remote_writer_allowlist.to_vec())
            }
        })
        .map_err(|err| format!("failed to build node replication config: {err:?}"))
}

fn build_replication_remote_writer_allowlist<'a>(
    validator_signer_public_keys: impl IntoIterator<Item = &'a String>,
    explicit_remote_writer_public_keys: &[String],
) -> Vec<String> {
    let mut allowlist: Vec<String> = validator_signer_public_keys.into_iter().cloned().collect();
    allowlist.extend(explicit_remote_writer_public_keys.iter().cloned());
    allowlist.sort();
    allowlist.dedup();
    allowlist
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
    let bootstrap_peers = if options.replication_network_bootstrap_peers.is_empty() {
        options
            .loaded_network_tier_manifest
            .as_ref()
            .map(|loaded| loaded.bootstrap_peers.clone())
            .unwrap_or_default()
    } else {
        options.replication_network_bootstrap_peers.clone()
    };
    config.bootstrap_peers = bootstrap_peers
        .iter()
        .map(|raw| {
            raw.parse()
                .map_err(|err| format!("invalid --replication-network-peer {raw}: {err}"))
        })
        .collect::<Result<_, _>>()?;
    apply_traffic_profile_to_replication_network_config(&mut config, options.traffic_profile);
    Ok(config)
}

fn apply_network_tier_manifest_defaults(options: &mut CliOptions) {
    let Some(loaded_manifest) = options.loaded_network_tier_manifest.as_ref() else {
        return;
    };
    if options.world_id.trim().is_empty() || options.world_id == cli::DEFAULT_WORLD_ID {
        options.world_id = loaded_manifest.manifest.chain_id.clone();
    }
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
    explicit_bindings: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>, String> {
    let mut bindings = BTreeMap::new();
    for validator in validators {
        let validator_id = validator.validator_id.trim();
        if validator_id.is_empty() {
            return Err("validator_id cannot be empty when deriving signer bindings".to_string());
        }
        if let Some(public_key_hex) = explicit_bindings.get(validator_id) {
            bindings.insert(validator_id.to_string(), public_key_hex.clone());
            continue;
        }
        let keypair = derive_node_consensus_signer_keypair(validator_id, root_keypair)?;
        bindings.insert(validator_id.to_string(), keypair.public_key_hex);
    }
    for validator_id in explicit_bindings.keys() {
        if !validators
            .iter()
            .any(|validator| validator.validator_id.trim() == validator_id)
        {
            return Err(format!(
                "validator signer override references unknown validator: {validator_id}"
            ));
        }
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

#[cfg(test)]
#[path = "oasis7_chain_runtime/execution_bridge_real_tests.rs"]
mod execution_bridge_real_tests;

#[cfg(test)]
#[path = "oasis7_chain_runtime/oasis7_chain_runtime_network_tier_tests.rs"]
mod network_tier_tests;
#[cfg(test)]
#[path = "oasis7_chain_runtime/oasis7_chain_runtime_observability_tests.rs"]
mod observability_tests;
#[cfg(test)]
#[path = "oasis7_chain_runtime/oasis7_chain_runtime_tests.rs"]
mod tests;
