use std::net::SocketAddr;
use std::path::PathBuf;

use oasis7::runtime::RewardAssetConfig;
use oasis7_node::{
    NodeHolePunchViability, NodeReachabilityAutoDetection, NodeRole, NodeUserMode, PosValidator,
};
use oasis7_proto::distributed_dht::{PeerDeploymentMode, PeerNodeRole, PeerReachabilityClass};
use oasis7_proto::storage_profile::StorageProfile;

use super::distfs_probe_runtime::{parse_distfs_probe_runtime_option, DistfsProbeRuntimeConfig};

pub(super) const DEFAULT_NODE_ID: &str = "viewer-live-node";
pub(super) const DEFAULT_WORLD_ID: &str = "live-llm_bootstrap";
pub(super) const DEFAULT_STATUS_BIND: &str = "127.0.0.1:5121";
pub(super) const DEFAULT_CONFIG_FILE: &str = "config.toml";
pub(super) const DEFAULT_REPLICATION_NETWORK_LISTEN: &str = "/ip4/127.0.0.1/tcp/0";
pub(super) const DEFAULT_NODE_TICK_MS: u64 = 200;
pub(super) const DEFAULT_POS_SLOT_DURATION_MS: u64 = 12_000;
pub(super) const DEFAULT_POS_TICKS_PER_SLOT: u64 = 10;
pub(super) const DEFAULT_POS_PROPOSAL_TICK_PHASE: u64 = 9;
pub(super) const DEFAULT_POS_MAX_PAST_SLOT_LAG: u64 = 256;
pub(super) const DEFAULT_REWARD_RUNTIME_STATE_FILE: &str = "reward-runtime-state.json";
pub(super) const DEFAULT_REWARD_RUNTIME_DISTFS_PROBE_STATE_FILE: &str =
    "reward-runtime-distfs-probe-state.json";
pub(super) const DEFAULT_REWARD_RUNTIME_REPORT_DIR: &str = "reward-runtime-report";
pub(super) const DEFAULT_REWARD_RUNTIME_STORAGE_METRICS_FILE: &str =
    "reward-runtime-storage-metrics.json";
pub(super) const DEFAULT_REWARD_RUNTIME_RESERVE_UNITS: i64 = 100_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CliOptions {
    pub node_id: String,
    pub world_id: String,
    pub status_bind: String,
    pub storage_profile: StorageProfile,
    pub node_role: NodeRole,
    pub p2p_user_mode: NodeUserMode,
    pub p2p_accept_public_entry: bool,
    pub p2p_detected_reachability: Option<PeerReachabilityClass>,
    pub p2p_detected_hole_punch_viability: NodeHolePunchViability,
    pub p2p_detected_relay_available: bool,
    pub p2p_detected_probe_stable: bool,
    pub p2p_deployment_mode: PeerDeploymentMode,
    pub p2p_node_role: PeerNodeRole,
    pub node_tick_ms: u64,
    pub pos_slot_duration_ms: u64,
    pub pos_ticks_per_slot: u64,
    pub pos_proposal_tick_phase: u64,
    pub pos_adaptive_tick_scheduler_enabled: bool,
    pub pos_slot_clock_genesis_unix_ms: Option<i64>,
    pub pos_max_past_slot_lag: u64,
    pub node_auto_attest_all_validators: bool,
    pub node_validators: Vec<PosValidator>,
    pub node_gossip_bind: Option<SocketAddr>,
    pub node_gossip_peers: Vec<SocketAddr>,
    pub config_path: String,
    pub execution_bridge_state_path: Option<PathBuf>,
    pub execution_world_dir: Option<PathBuf>,
    pub execution_records_dir: Option<PathBuf>,
    pub storage_root: Option<PathBuf>,
    pub reward_runtime_enabled: bool,
    pub reward_runtime_signer_node_id: Option<String>,
    pub reward_runtime_epoch_duration_secs: Option<u64>,
    pub reward_points_per_credit: u64,
    pub reward_runtime_auto_redeem: bool,
    pub reward_initial_reserve_power_units: i64,
    pub reward_distfs_probe_config: DistfsProbeRuntimeConfig,
    pub p2p_user_mode_explicit: bool,
    pub p2p_detected_reachability_explicit: bool,
    pub p2p_detected_hole_punch_viability_explicit: bool,
    pub p2p_detected_relay_available_explicit: bool,
    pub p2p_detected_probe_stable_explicit: bool,
    pub p2p_deployment_mode_explicit: bool,
    pub p2p_node_role_explicit: bool,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            node_id: DEFAULT_NODE_ID.to_string(),
            world_id: DEFAULT_WORLD_ID.to_string(),
            status_bind: DEFAULT_STATUS_BIND.to_string(),
            storage_profile: StorageProfile::DevLocal,
            node_role: NodeRole::Sequencer,
            p2p_user_mode: NodeUserMode::AutoJoin,
            p2p_accept_public_entry: false,
            p2p_detected_reachability: None,
            p2p_detected_hole_punch_viability: NodeHolePunchViability::Unknown,
            p2p_detected_relay_available: false,
            p2p_detected_probe_stable: false,
            p2p_deployment_mode: PeerDeploymentMode::Private,
            p2p_node_role: PeerNodeRole::ValidatorCore,
            node_tick_ms: DEFAULT_NODE_TICK_MS,
            pos_slot_duration_ms: DEFAULT_POS_SLOT_DURATION_MS,
            pos_ticks_per_slot: DEFAULT_POS_TICKS_PER_SLOT,
            pos_proposal_tick_phase: DEFAULT_POS_PROPOSAL_TICK_PHASE,
            pos_adaptive_tick_scheduler_enabled: false,
            pos_slot_clock_genesis_unix_ms: None,
            pos_max_past_slot_lag: DEFAULT_POS_MAX_PAST_SLOT_LAG,
            node_auto_attest_all_validators: false,
            node_validators: Vec::new(),
            node_gossip_bind: None,
            node_gossip_peers: Vec::new(),
            config_path: DEFAULT_CONFIG_FILE.to_string(),
            execution_bridge_state_path: None,
            execution_world_dir: None,
            execution_records_dir: None,
            storage_root: None,
            reward_runtime_enabled: true,
            reward_runtime_signer_node_id: None,
            reward_runtime_epoch_duration_secs: None,
            reward_points_per_credit: RewardAssetConfig::default().points_per_credit,
            reward_runtime_auto_redeem: false,
            reward_initial_reserve_power_units: DEFAULT_REWARD_RUNTIME_RESERVE_UNITS,
            reward_distfs_probe_config: DistfsProbeRuntimeConfig::default(),
            p2p_user_mode_explicit: false,
            p2p_detected_reachability_explicit: false,
            p2p_detected_hole_punch_viability_explicit: false,
            p2p_detected_relay_available_explicit: false,
            p2p_detected_probe_stable_explicit: false,
            p2p_deployment_mode_explicit: false,
            p2p_node_role_explicit: false,
        }
    }
}

pub(super) fn parse_options<'a>(args: impl Iterator<Item = &'a str>) -> Result<CliOptions, String> {
    let mut options = CliOptions::default();
    let mut iter = args.peekable();

    while let Some(arg) = iter.next() {
        match arg {
            "--node-id" => options.node_id = parse_required_value(&mut iter, "--node-id")?,
            "--world-id" => options.world_id = parse_required_value(&mut iter, "--world-id")?,
            "--status-bind" => {
                options.status_bind = parse_required_value(&mut iter, "--status-bind")?;
            }
            "--storage-profile" => {
                options.storage_profile = parse_required_value(&mut iter, "--storage-profile")?
                    .parse::<StorageProfile>()?;
            }
            "--node-role" => {
                let raw = parse_required_value(&mut iter, "--node-role")?;
                options.node_role = raw.parse::<NodeRole>().map_err(|_| {
                    "--node-role must be one of: sequencer, storage, observer".to_string()
                })?;
                if !options.p2p_node_role_explicit {
                    options.p2p_node_role = default_p2p_node_role(options.node_role);
                }
            }
            "--p2p-user-mode" => {
                let raw = parse_required_value(&mut iter, "--p2p-user-mode")?;
                options.p2p_user_mode = raw.parse::<NodeUserMode>()?;
                options.p2p_user_mode_explicit = true;
            }
            "--p2p-accept-public-entry" => {
                options.p2p_accept_public_entry = true;
            }
            "--p2p-reject-public-entry" => {
                options.p2p_accept_public_entry = false;
            }
            "--p2p-detected-reachability" => {
                let raw = parse_required_value(&mut iter, "--p2p-detected-reachability")?;
                options.p2p_detected_reachability =
                    Some(parse_peer_reachability_class(raw.as_str())?);
                options.p2p_detected_reachability_explicit = true;
            }
            "--p2p-clear-detected-reachability" => {
                options.p2p_detected_reachability = None;
                options.p2p_detected_reachability_explicit = false;
            }
            "--p2p-detected-hole-punch" => {
                let raw = parse_required_value(&mut iter, "--p2p-detected-hole-punch")?;
                options.p2p_detected_hole_punch_viability =
                    raw.parse::<NodeHolePunchViability>()?;
                options.p2p_detected_hole_punch_viability_explicit = true;
            }
            "--p2p-detected-relay-available" => {
                options.p2p_detected_relay_available = true;
                options.p2p_detected_relay_available_explicit = true;
            }
            "--p2p-detected-relay-unavailable" => {
                options.p2p_detected_relay_available = false;
                options.p2p_detected_relay_available_explicit = true;
            }
            "--p2p-detected-probe-stable" => {
                options.p2p_detected_probe_stable = true;
                options.p2p_detected_probe_stable_explicit = true;
            }
            "--p2p-detected-probe-unstable" => {
                options.p2p_detected_probe_stable = false;
                options.p2p_detected_probe_stable_explicit = true;
            }
            "--p2p-deployment-mode" => {
                let raw = parse_required_value(&mut iter, "--p2p-deployment-mode")?;
                options.p2p_deployment_mode = raw.parse::<PeerDeploymentMode>()?;
                options.p2p_deployment_mode_explicit = true;
            }
            "--p2p-node-role" => {
                let raw = parse_required_value(&mut iter, "--p2p-node-role")?;
                options.p2p_node_role = raw.parse::<PeerNodeRole>()?;
                options.p2p_node_role_explicit = true;
            }
            "--node-tick-ms" => {
                let raw = parse_required_value(&mut iter, "--node-tick-ms")?;
                options.node_tick_ms = raw
                    .parse::<u64>()
                    .ok()
                    .filter(|value| *value > 0)
                    .ok_or_else(|| "--node-tick-ms requires a positive integer".to_string())?;
            }
            "--pos-slot-duration-ms" => {
                let raw = parse_required_value(&mut iter, "--pos-slot-duration-ms")?;
                options.pos_slot_duration_ms = raw
                    .parse::<u64>()
                    .ok()
                    .filter(|value| *value > 0)
                    .ok_or_else(|| {
                    "--pos-slot-duration-ms requires a positive integer".to_string()
                })?;
            }
            "--pos-ticks-per-slot" => {
                let raw = parse_required_value(&mut iter, "--pos-ticks-per-slot")?;
                options.pos_ticks_per_slot = raw
                    .parse::<u64>()
                    .ok()
                    .filter(|value| *value > 0)
                    .ok_or_else(|| {
                        "--pos-ticks-per-slot requires a positive integer".to_string()
                    })?;
            }
            "--pos-proposal-tick-phase" => {
                let raw = parse_required_value(&mut iter, "--pos-proposal-tick-phase")?;
                options.pos_proposal_tick_phase = raw.parse::<u64>().map_err(|_| {
                    "--pos-proposal-tick-phase requires a non-negative integer".to_string()
                })?;
            }
            "--pos-adaptive-tick-scheduler" => options.pos_adaptive_tick_scheduler_enabled = true,
            "--pos-no-adaptive-tick-scheduler" => {
                options.pos_adaptive_tick_scheduler_enabled = false;
            }
            "--pos-slot-clock-genesis-unix-ms" => {
                let raw = parse_required_value(&mut iter, "--pos-slot-clock-genesis-unix-ms")?;
                options.pos_slot_clock_genesis_unix_ms =
                    Some(raw.parse::<i64>().map_err(|_| {
                        "--pos-slot-clock-genesis-unix-ms requires an integer".to_string()
                    })?);
            }
            "--pos-max-past-slot-lag" => {
                let raw = parse_required_value(&mut iter, "--pos-max-past-slot-lag")?;
                options.pos_max_past_slot_lag = raw.parse::<u64>().map_err(|_| {
                    "--pos-max-past-slot-lag requires a non-negative integer".to_string()
                })?;
            }
            "--node-validator" => {
                let raw = parse_required_value(&mut iter, "--node-validator")?;
                options
                    .node_validators
                    .push(parse_validator_spec(raw.as_str())?);
            }
            "--node-auto-attest-all" => options.node_auto_attest_all_validators = true,
            "--node-no-auto-attest-all" => options.node_auto_attest_all_validators = false,
            "--node-gossip-bind" => {
                let raw = parse_required_value(&mut iter, "--node-gossip-bind")?;
                options.node_gossip_bind =
                    Some(parse_socket_addr(raw.as_str(), "--node-gossip-bind")?);
            }
            "--node-gossip-peer" => {
                let raw = parse_required_value(&mut iter, "--node-gossip-peer")?;
                options
                    .node_gossip_peers
                    .push(parse_socket_addr(raw.as_str(), "--node-gossip-peer")?);
            }
            "--config" => options.config_path = parse_required_value(&mut iter, "--config")?,
            "--execution-bridge-state" => {
                let raw = parse_required_value(&mut iter, "--execution-bridge-state")?;
                options.execution_bridge_state_path = Some(PathBuf::from(raw));
            }
            "--execution-world-dir" => {
                let raw = parse_required_value(&mut iter, "--execution-world-dir")?;
                options.execution_world_dir = Some(PathBuf::from(raw));
            }
            "--execution-records-dir" => {
                let raw = parse_required_value(&mut iter, "--execution-records-dir")?;
                options.execution_records_dir = Some(PathBuf::from(raw));
            }
            "--storage-root" => {
                let raw = parse_required_value(&mut iter, "--storage-root")?;
                options.storage_root = Some(PathBuf::from(raw));
            }
            "--reward-runtime-enable" => options.reward_runtime_enabled = true,
            "--reward-runtime-disable" => options.reward_runtime_enabled = false,
            "--reward-runtime-signer-node-id" => {
                options.reward_runtime_signer_node_id = Some(parse_required_value(
                    &mut iter,
                    "--reward-runtime-signer-node-id",
                )?);
            }
            "--reward-runtime-epoch-duration-secs" => {
                let raw = parse_required_value(&mut iter, "--reward-runtime-epoch-duration-secs")?;
                let value = raw.parse::<u64>().ok().filter(|v| *v > 0).ok_or_else(|| {
                    "--reward-runtime-epoch-duration-secs requires a positive integer".to_string()
                })?;
                options.reward_runtime_epoch_duration_secs = Some(value);
            }
            "--reward-points-per-credit" => {
                let raw = parse_required_value(&mut iter, "--reward-points-per-credit")?;
                options.reward_points_per_credit =
                    raw.parse::<u64>().ok().filter(|v| *v > 0).ok_or_else(|| {
                        "--reward-points-per-credit requires a positive integer".to_string()
                    })?;
            }
            "--reward-runtime-auto-redeem" => options.reward_runtime_auto_redeem = true,
            "--reward-runtime-no-auto-redeem" => options.reward_runtime_auto_redeem = false,
            "--reward-initial-reserve-power-units" => {
                let raw = parse_required_value(&mut iter, "--reward-initial-reserve-power-units")?;
                options.reward_initial_reserve_power_units = raw
                    .parse::<i64>()
                    .ok()
                    .filter(|value| *value >= 0)
                    .ok_or_else(|| {
                        "--reward-initial-reserve-power-units requires a non-negative integer"
                            .to_string()
                    })?;
            }
            _ => {
                if parse_distfs_probe_runtime_option(
                    arg,
                    &mut iter,
                    &mut options.reward_distfs_probe_config,
                )? {
                    continue;
                }
                return Err(format!("unknown option: {arg}"));
            }
        }
    }

    parse_host_port(options.status_bind.as_str(), "--status-bind")?;
    if options.node_id.trim().is_empty() {
        return Err("--node-id requires a non-empty value".to_string());
    }
    if options.world_id.trim().is_empty() {
        return Err("--world-id requires a non-empty value".to_string());
    }
    if options.config_path.trim().is_empty() {
        return Err("--config requires a non-empty value".to_string());
    }
    if options.reward_points_per_credit == 0 {
        return Err("--reward-points-per-credit requires a positive integer".to_string());
    }
    if options.reward_initial_reserve_power_units < 0 {
        return Err("--reward-initial-reserve-power-units requires a non-negative integer".into());
    }
    if options.pos_proposal_tick_phase >= options.pos_ticks_per_slot {
        return Err(format!(
            "--pos-proposal-tick-phase={} must be less than --pos-ticks-per-slot={}",
            options.pos_proposal_tick_phase, options.pos_ticks_per_slot
        ));
    }
    if !options.node_gossip_peers.is_empty() && options.node_gossip_bind.is_none() {
        return Err("--node-gossip-peer requires --node-gossip-bind".to_string());
    }

    Ok(options)
}

pub(super) fn p2p_auto_detection_from_options(
    options: &CliOptions,
) -> NodeReachabilityAutoDetection {
    NodeReachabilityAutoDetection {
        observed_reachability: options.p2p_detected_reachability,
        hole_punch_viability: options.p2p_detected_hole_punch_viability,
        relay_available: options.p2p_detected_relay_available,
        probe_stable: options.p2p_detected_probe_stable,
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

fn parse_socket_addr(raw: &str, label: &str) -> Result<SocketAddr, String> {
    raw.parse::<SocketAddr>()
        .map_err(|_| format!("{label} requires <addr:port>"))
}

pub(super) fn parse_host_port(raw: &str, label: &str) -> Result<(String, u16), String> {
    let trimmed = raw.trim();
    let (host, port_text) = trimmed
        .rsplit_once(':')
        .ok_or_else(|| format!("{label} must be in <host:port> format"))?;
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

fn parse_peer_reachability_class(raw: &str) -> Result<PeerReachabilityClass, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "public" => Ok(PeerReachabilityClass::Public),
        "hybrid" => Ok(PeerReachabilityClass::Hybrid),
        "private" => Ok(PeerReachabilityClass::Private),
        "relay_only" => Ok(PeerReachabilityClass::RelayOnly),
        "validator_hidden" => Ok(PeerReachabilityClass::ValidatorHidden),
        _ => Err(
            "detected reachability must be one of: public, hybrid, private, relay_only, validator_hidden"
                .to_string(),
        ),
    }
}

pub(super) fn parse_validator_spec(raw: &str) -> Result<PosValidator, String> {
    let (validator_id, stake_text) = raw
        .rsplit_once(':')
        .ok_or_else(|| "--node-validator requires <validator_id:stake>".to_string())?;
    let validator_id = validator_id.trim();
    if validator_id.is_empty() {
        return Err("--node-validator validator_id cannot be empty".to_string());
    }
    let stake = stake_text
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| "--node-validator stake must be a positive integer".to_string())?;
    Ok(PosValidator {
        validator_id: validator_id.to_string(),
        stake,
    })
}

pub(super) fn print_help() {
    println!(
        "Usage: oasis7_chain_runtime [options]\n\n\
Starts standalone chain/node runtime with status HTTP endpoints.\n\n\
Options:\n\
  --node-id <id>                    node identifier (default: {DEFAULT_NODE_ID})\n\
  --world-id <id>                   world identifier (default: {DEFAULT_WORLD_ID})\n\
  --storage-profile <name>          dev_local|release_default|soak_forensics (default: dev_local)\n\
  --status-bind <host:port>         status HTTP bind (default: {DEFAULT_STATUS_BIND})\n\
  --node-role <role>                sequencer|storage|observer (default: sequencer)\n\
  --p2p-user-mode <mode>            auto_join|private_safe|public_entry (default: auto_join)\n\
  --p2p-accept-public-entry         accept auto-detected public-entry recommendation\n\
  --p2p-reject-public-entry         force conservative fallback when auto-detect suggests public entry (default)\n\
  --p2p-detected-reachability <c>   public|hybrid|private|relay_only|validator_hidden\n\
  --p2p-clear-detected-reachability clear detected reachability hint\n\
  --p2p-detected-hole-punch <s>     unknown|viable|blocked (default: unknown)\n\
  --p2p-detected-relay-available    mark relay fallback as available (default)\n\
  --p2p-detected-relay-unavailable  mark relay fallback as unavailable\n\
  --p2p-detected-probe-stable       mark auto-detection as stable (default)\n\
  --p2p-detected-probe-unstable     mark auto-detection as unstable\n\
  --p2p-deployment-mode <mode>      public|hybrid|private|relay_only|validator_hidden (default: private)\n\
  --p2p-node-role <role>            validator_core|sentry|relay|full_storage|observer_light\n\
  --node-tick-ms <n>                worker poll/fallback interval ms (default: {DEFAULT_NODE_TICK_MS})\n\
  --pos-slot-duration-ms <n>        PoS slot duration in milliseconds (default: {DEFAULT_POS_SLOT_DURATION_MS})\n\
  --pos-ticks-per-slot <n>          logical ticks per PoS slot (default: {DEFAULT_POS_TICKS_PER_SLOT})\n\
  --pos-proposal-tick-phase <n>     proposal trigger phase within slot tick window (default: {DEFAULT_POS_PROPOSAL_TICK_PHASE})\n\
  --pos-adaptive-tick-scheduler     enable adaptive wait to next logical tick boundary\n\
  --pos-no-adaptive-tick-scheduler  disable adaptive scheduler (default)\n\
  --pos-slot-clock-genesis-unix-ms <n>\n\
                                    fixed slot clock genesis unix ms (default: auto)\n\
  --pos-max-past-slot-lag <n>       max accepted inbound stale slot lag (default: {DEFAULT_POS_MAX_PAST_SLOT_LAG})\n\
  --node-validator <id:stake>       add validator stake (repeatable)\n\
  --node-auto-attest-all            enable auto attesting validators\n\
  --node-no-auto-attest-all         disable auto attesting validators (default)\n\
  --node-gossip-bind <addr:port>    UDP gossip bind\n\
  --node-gossip-peer <addr:port>    UDP gossip peer (repeatable, requires --node-gossip-bind)\n\
  --config <path>                   config file path for node keypair (default: {DEFAULT_CONFIG_FILE})\n\
  --execution-bridge-state <path>   override execution bridge state file path\n\
  --execution-world-dir <path>      override execution world directory\n\
  --execution-records-dir <path>    override execution records directory\n\
  --storage-root <path>             override execution CAS/storage root\n\
  --reward-runtime-enable           enable reward runtime worker (default)\n\
  --reward-runtime-disable          disable reward runtime worker\n\
  --reward-runtime-signer-node-id <id>\n\
                                    override reward runtime signer node id (default: --node-id)\n\
  --reward-runtime-epoch-duration-secs <n>\n\
                                    override reward settlement epoch duration seconds\n\
  --reward-points-per-credit <n>    reward points per credit (default: {})\n\
  --reward-runtime-auto-redeem      enable runtime auto redeem\n\
  --reward-runtime-no-auto-redeem   disable runtime auto redeem (default)\n\
  --reward-initial-reserve-power-units <n>\n\
                                    reward reserve power units (default: {DEFAULT_REWARD_RUNTIME_RESERVE_UNITS})\n\
  --reward-distfs-probe-per-tick <n>\n\
                                    distfs challenge probes per tick (default: 1)\n\
  -h, --help                        show help",
        RewardAssetConfig::default().points_per_credit
    );
}

fn default_p2p_node_role(node_role: NodeRole) -> PeerNodeRole {
    match node_role {
        NodeRole::Sequencer => PeerNodeRole::ValidatorCore,
        NodeRole::Storage => PeerNodeRole::FullStorage,
        NodeRole::Observer => PeerNodeRole::ObserverLight,
    }
}
