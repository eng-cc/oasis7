use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::{self, BufReader, Read};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use oasis7::chain_pos_defaults;
use oasis7::network_tier_manifest::LoadedNetworkTierManifest;
use oasis7::runtime::RewardAssetConfig;
use oasis7_node::{
    NodeAutoNatStatus, NodeHolePunchViability, NodePublicPortReachability,
    NodeReachabilityAutoDetection, NodeRole, NodeUserMode, PosValidator,
};
use oasis7_proto::distributed_dht::{PeerDeploymentMode, PeerNodeRole, PeerReachabilityClass};
use oasis7_proto::storage_profile::StorageProfile;
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::distfs_probe_runtime::{DistfsProbeRuntimeConfig, parse_distfs_probe_runtime_option};

pub(super) const DEFAULT_NODE_ID: &str = "viewer-live-node";
pub(super) const DEFAULT_WORLD_ID: &str = "oasis7-unified-world-v1";
pub(super) const DEFAULT_STATUS_BIND: &str = "127.0.0.1:5121";
pub(super) const DEFAULT_CONFIG_FILE: &str = "config.toml";
pub(super) const DEFAULT_REPLICATION_NETWORK_LISTEN: &str = "/ip4/127.0.0.1/tcp/0";
pub(super) const DEFAULT_NODE_TICK_MS: u64 = 200;
pub(super) const DEFAULT_REWARD_RUNTIME_STATE_FILE: &str = "reward-runtime-state.json";
pub(super) const DEFAULT_REWARD_RUNTIME_DISTFS_PROBE_STATE_FILE: &str =
    "reward-runtime-distfs-probe-state.json";
pub(super) const DEFAULT_REWARD_RUNTIME_REPORT_DIR: &str = "reward-runtime-report";
pub(super) const DEFAULT_REWARD_RUNTIME_STORAGE_METRICS_FILE: &str =
    "reward-runtime-storage-metrics.json";
pub(super) const DEFAULT_REWARD_RUNTIME_RESERVE_UNITS: i64 = 100_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TrafficProfile {
    Default,
    TriadLowTraffic,
}

impl TrafficProfile {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::TriadLowTraffic => "triad_low_traffic",
        }
    }
}

impl fmt::Display for TrafficProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TrafficProfile {
    type Err = String;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "default" => Ok(Self::Default),
            "triad_low_traffic" => Ok(Self::TriadLowTraffic),
            _ => Err("traffic profile must be one of: default, triad_low_traffic".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CliOptions {
    pub node_id: String,
    pub world_id: String,
    pub status_bind: String,
    pub storage_profile: StorageProfile,
    pub traffic_profile: TrafficProfile,
    pub node_role: NodeRole,
    pub p2p_user_mode: NodeUserMode,
    pub p2p_accept_public_entry: bool,
    pub p2p_detected_reachability: Option<PeerReachabilityClass>,
    pub p2p_detected_hole_punch_viability: NodeHolePunchViability,
    pub p2p_detected_relay_available: bool,
    pub p2p_detected_probe_stable: bool,
    pub p2p_deployment_mode: PeerDeploymentMode,
    pub p2p_node_role: PeerNodeRole,
    pub p2p_source_operator: Option<String>,
    pub p2p_source_asn: Option<String>,
    pub p2p_max_ipv4_subnet_active_peers: Option<usize>,
    pub node_tick_ms: u64,
    pub pos_slot_duration_ms: u64,
    pub pos_ticks_per_slot: u64,
    pub pos_proposal_tick_phase: u64,
    pub pos_adaptive_tick_scheduler_enabled: bool,
    pub pos_slot_clock_genesis_unix_ms: Option<i64>,
    pub pos_max_past_slot_lag: u64,
    pub node_auto_attest_all_validators: bool,
    pub node_validators: Vec<PosValidator>,
    pub node_validator_signer_public_keys: BTreeMap<String, String>,
    pub node_gossip_bind: Option<SocketAddr>,
    pub node_gossip_peers: Vec<SocketAddr>,
    pub replication_network_listen_addrs: Vec<String>,
    pub replication_network_bootstrap_peers: Vec<String>,
    pub replication_remote_writer_public_keys: Vec<String>,
    pub network_tier_manifest_path: Option<PathBuf>,
    pub loaded_network_tier_manifest: Option<LoadedNetworkTierManifest>,
    pub genesis_validator_registry_path: Option<PathBuf>,
    pub config_path: String,
    pub runtime_root: Option<PathBuf>,
    pub execution_bridge_state_path: Option<PathBuf>,
    pub execution_world_dir: Option<PathBuf>,
    pub execution_records_dir: Option<PathBuf>,
    pub storage_root: Option<PathBuf>,
    pub replication_root: Option<PathBuf>,
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
        let pos_defaults = chain_pos_defaults::defaults();
        Self {
            node_id: DEFAULT_NODE_ID.to_string(),
            world_id: DEFAULT_WORLD_ID.to_string(),
            status_bind: DEFAULT_STATUS_BIND.to_string(),
            storage_profile: StorageProfile::DevLocal,
            traffic_profile: TrafficProfile::Default,
            node_role: NodeRole::Sequencer,
            p2p_user_mode: NodeUserMode::AutoJoin,
            p2p_accept_public_entry: false,
            p2p_detected_reachability: None,
            p2p_detected_hole_punch_viability: NodeHolePunchViability::Unknown,
            p2p_detected_relay_available: false,
            p2p_detected_probe_stable: false,
            p2p_deployment_mode: PeerDeploymentMode::Private,
            p2p_node_role: PeerNodeRole::ValidatorCore,
            p2p_source_operator: None,
            p2p_source_asn: None,
            p2p_max_ipv4_subnet_active_peers: None,
            node_tick_ms: DEFAULT_NODE_TICK_MS,
            pos_slot_duration_ms: pos_defaults.slot_duration_ms,
            pos_ticks_per_slot: pos_defaults.ticks_per_slot,
            pos_proposal_tick_phase: pos_defaults.proposal_tick_phase,
            pos_adaptive_tick_scheduler_enabled: false,
            pos_slot_clock_genesis_unix_ms: None,
            pos_max_past_slot_lag: pos_defaults.max_past_slot_lag,
            node_auto_attest_all_validators: false,
            node_validators: Vec::new(),
            node_validator_signer_public_keys: BTreeMap::new(),
            node_gossip_bind: None,
            node_gossip_peers: Vec::new(),
            replication_network_listen_addrs: Vec::new(),
            replication_network_bootstrap_peers: Vec::new(),
            replication_remote_writer_public_keys: Vec::new(),
            network_tier_manifest_path: None,
            loaded_network_tier_manifest: None,
            genesis_validator_registry_path: None,
            config_path: DEFAULT_CONFIG_FILE.to_string(),
            runtime_root: None,
            execution_bridge_state_path: None,
            execution_world_dir: None,
            execution_records_dir: None,
            storage_root: None,
            replication_root: None,
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
            "--traffic-profile" => {
                options.traffic_profile = parse_required_value(&mut iter, "--traffic-profile")?
                    .parse::<TrafficProfile>()?;
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
            "--p2p-source-operator" => {
                let raw = parse_required_value(&mut iter, "--p2p-source-operator")?;
                options.p2p_source_operator = Some(normalize_p2p_metadata_label(
                    raw.as_str(),
                    "--p2p-source-operator",
                )?);
            }
            "--p2p-source-asn" => {
                let raw = parse_required_value(&mut iter, "--p2p-source-asn")?;
                options.p2p_source_asn = Some(normalize_p2p_metadata_label(
                    raw.as_str(),
                    "--p2p-source-asn",
                )?);
            }
            "--p2p-max-ipv4-subnet-active-peers" => {
                let raw = parse_required_value(&mut iter, "--p2p-max-ipv4-subnet-active-peers")?;
                options.p2p_max_ipv4_subnet_active_peers = Some(
                    raw.parse::<usize>()
                        .ok()
                        .filter(|value| *value > 0)
                        .ok_or_else(|| {
                            "--p2p-max-ipv4-subnet-active-peers requires a positive integer"
                                .to_string()
                        })?,
                );
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
            "--node-validator-signer-public-key" => {
                let raw = parse_required_value(&mut iter, "--node-validator-signer-public-key")?;
                let (validator_id, public_key_hex) =
                    parse_validator_signer_public_key_spec(raw.as_str())?;
                options
                    .node_validator_signer_public_keys
                    .insert(validator_id, public_key_hex);
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
            "--replication-network-listen" => {
                let raw = parse_required_value(&mut iter, "--replication-network-listen")?;
                options.replication_network_listen_addrs.push(raw);
            }
            "--replication-network-peer" => {
                let raw = parse_required_value(&mut iter, "--replication-network-peer")?;
                options.replication_network_bootstrap_peers.push(raw);
            }
            "--replication-remote-writer-public-key" => {
                let raw =
                    parse_required_value(&mut iter, "--replication-remote-writer-public-key")?;
                options.replication_remote_writer_public_keys.push(raw);
            }
            "--network-tier-manifest" => {
                let raw = parse_required_value(&mut iter, "--network-tier-manifest")?;
                options.network_tier_manifest_path = Some(PathBuf::from(raw));
            }
            "--genesis-validator-registry" => {
                let raw = parse_required_value(&mut iter, "--genesis-validator-registry")?;
                options.genesis_validator_registry_path = Some(PathBuf::from(raw));
            }
            "--config" => options.config_path = parse_required_value(&mut iter, "--config")?,
            "--runtime-root" => {
                let raw = parse_required_value(&mut iter, "--runtime-root")?;
                options.runtime_root = Some(PathBuf::from(raw));
            }
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
            "--replication-root" => {
                let raw = parse_required_value(&mut iter, "--replication-root")?;
                options.replication_root = Some(PathBuf::from(raw));
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
    if let Some(manifest_path) = options.network_tier_manifest_path.as_ref() {
        let loaded = LoadedNetworkTierManifest::load(manifest_path.as_path())?;
        validate_current_runtime_hash_against_network_tier_bundle(
            manifest_path.as_path(),
            &loaded,
        )?;
        options.loaded_network_tier_manifest = Some(loaded);
    }

    Ok(options)
}

#[derive(Debug, Deserialize)]
struct NetworkTierReleaseCandidateBundle {
    runtime_build: NetworkTierRuntimeBuildRef,
    generated_world_sidecar: Option<NetworkTierArtifactRef>,
    world_generation_provenance: Option<NetworkTierArtifactRef>,
}

#[derive(Debug, Deserialize)]
struct NetworkTierRuntimeBuildRef {
    sha256: String,
}

#[derive(Debug, Deserialize)]
struct NetworkTierArtifactRef {
    #[serde(default)]
    resolved_path: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    sha256: Option<String>,
    #[serde(default)]
    sha256_tree: Option<String>,
    #[serde(default)]
    file_count: Option<u64>,
    #[serde(default)]
    total_bytes: Option<u64>,
}

fn validate_current_runtime_hash_against_network_tier_bundle(
    manifest_path: &Path,
    loaded: &LoadedNetworkTierManifest,
) -> Result<(), String> {
    let bundle_path = resolve_network_tier_runtime_ref_path(
        manifest_path,
        loaded
            .manifest
            .runtime_refs
            .release_candidate_bundle_ref
            .as_str(),
    );
    let bundle_bytes = fs::read(bundle_path.as_path()).map_err(|err| {
        format!(
            "read network tier release_candidate_bundle_ref {} failed: {err}",
            bundle_path.display()
        )
    })?;
    let bundle: NetworkTierReleaseCandidateBundle = serde_json::from_slice(bundle_bytes.as_slice())
        .map_err(|err| {
            format!(
                "parse network tier release_candidate_bundle_ref {} failed: {err}",
                bundle_path.display()
            )
        })?;
    if loaded.manifest.tier == "public_testnet" {
        validate_network_tier_bundle_artifact(
            bundle_path.as_path(),
            "generated_world_sidecar",
            bundle.generated_world_sidecar.as_ref().ok_or_else(|| {
                format!(
                    "network tier release_candidate_bundle_ref {} missing generated_world_sidecar",
                    bundle_path.display()
                )
            })?,
            "directory",
        )?;
        validate_network_tier_bundle_artifact(
            bundle_path.as_path(),
            "world_generation_provenance",
            bundle.world_generation_provenance.as_ref().ok_or_else(|| {
                format!(
                    "network tier release_candidate_bundle_ref {} missing world_generation_provenance",
                    bundle_path.display()
                )
            })?,
            "file",
        )?;
    }
    let expected_sha256 = bundle.runtime_build.sha256.trim().to_ascii_lowercase();
    if expected_sha256.is_empty() {
        return Err(format!(
            "network tier release_candidate_bundle_ref {} missing runtime_build.sha256",
            bundle_path.display()
        ));
    }
    if !is_sha256_hex(expected_sha256.as_str()) {
        return Err(format!(
            "network tier release_candidate_bundle_ref {} has invalid runtime_build.sha256: expected 64 lowercase hex characters",
            bundle_path.display()
        ));
    }

    let current_exe = std::env::current_exe()
        .map_err(|err| format!("resolve current runtime executable path failed: {err}"))?;
    let actual_sha256 = sha256_file_hex(current_exe.as_path()).map_err(|err| {
        format!(
            "hash current runtime executable {} failed: {err}",
            current_exe.display()
        )
    })?;
    if actual_sha256 != expected_sha256 {
        return Err(format!(
            "network tier runtime bundle hash mismatch: executable={} actual_sha256={} bundle_path={} expected_sha256={}",
            current_exe.display(),
            actual_sha256,
            bundle_path.display(),
            expected_sha256
        ));
    }
    Ok(())
}

fn validate_network_tier_bundle_artifact(
    bundle_path: &Path,
    label: &str,
    artifact: &NetworkTierArtifactRef,
    expected_kind: &str,
) -> Result<(), String> {
    if artifact.resolved_path.trim().is_empty() {
        return Err(format!(
            "network tier release_candidate_bundle_ref {} missing {label}.resolved_path",
            bundle_path.display()
        ));
    }
    if artifact.kind.trim() != expected_kind {
        return Err(format!(
            "network tier release_candidate_bundle_ref {} has invalid {label}.kind: expected {expected_kind}, got {}",
            bundle_path.display(),
            artifact.kind.trim()
        ));
    }
    let artifact_path = Path::new(artifact.resolved_path.as_str());
    if !artifact_path.exists() {
        return Err(format!(
            "network tier release_candidate_bundle_ref {} {label} path missing: {}",
            bundle_path.display(),
            artifact_path.display()
        ));
    }
    if expected_kind == "directory" {
        for required in ["snapshot.json", "journal.json"] {
            if !artifact_path.join(required).is_file() {
                return Err(format!(
                    "network tier release_candidate_bundle_ref {} {label} missing {required}: {}",
                    bundle_path.display(),
                    artifact_path.display()
                ));
            }
        }
    }
    match expected_kind {
        "file" => {
            let expected_sha256 = artifact.sha256.as_deref().ok_or_else(|| {
                format!(
                    "network tier release_candidate_bundle_ref {} missing {label}.sha256",
                    bundle_path.display()
                )
            })?;
            if !is_sha256_hex(expected_sha256) {
                return Err(format!(
                    "network tier release_candidate_bundle_ref {} has invalid {label}.sha256",
                    bundle_path.display()
                ));
            }
            let actual_sha256 = sha256_file_hex(artifact_path).map_err(|err| {
                format!(
                    "hash network tier release_candidate_bundle_ref {} {label} {} failed: {err}",
                    bundle_path.display(),
                    artifact_path.display()
                )
            })?;
            if actual_sha256 != expected_sha256 {
                return Err(format!(
                    "network tier release_candidate_bundle_ref {} {label} drift detected: bundle={} current={}",
                    bundle_path.display(),
                    expected_sha256,
                    actual_sha256
                ));
            }
        }
        "directory" => {
            let expected_sha256_tree = artifact.sha256_tree.as_deref().ok_or_else(|| {
                format!(
                    "network tier release_candidate_bundle_ref {} missing {label}.sha256_tree",
                    bundle_path.display()
                )
            })?;
            if !is_sha256_hex(expected_sha256_tree) {
                return Err(format!(
                    "network tier release_candidate_bundle_ref {} has invalid {label}.sha256_tree",
                    bundle_path.display()
                ));
            }
            let actual_tree = sha256_dir_tree_hex(artifact_path).map_err(|err| {
                format!(
                    "hash network tier release_candidate_bundle_ref {} {label} {} failed: {err}",
                    bundle_path.display(),
                    artifact_path.display()
                )
            })?;
            if actual_tree.sha256_tree != expected_sha256_tree {
                return Err(format!(
                    "network tier release_candidate_bundle_ref {} {label} drift detected: bundle={} current={}",
                    bundle_path.display(),
                    expected_sha256_tree,
                    actual_tree.sha256_tree
                ));
            }
            if let Some(expected_file_count) = artifact.file_count {
                if expected_file_count != actual_tree.file_count {
                    return Err(format!(
                        "network tier release_candidate_bundle_ref {} {label} file_count drift: bundle={} current={}",
                        bundle_path.display(),
                        expected_file_count,
                        actual_tree.file_count
                    ));
                }
            }
            if let Some(expected_total_bytes) = artifact.total_bytes {
                if expected_total_bytes != actual_tree.total_bytes {
                    return Err(format!(
                        "network tier release_candidate_bundle_ref {} {label} total_bytes drift: bundle={} current={}",
                        bundle_path.display(),
                        expected_total_bytes,
                        actual_tree.total_bytes
                    ));
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn resolve_network_tier_runtime_ref_path(manifest_path: &Path, raw_ref: &str) -> PathBuf {
    let candidate = PathBuf::from(raw_ref);
    if candidate.is_absolute() {
        return candidate;
    }
    manifest_path
        .parent()
        .map(|parent| parent.join(candidate.clone()))
        .unwrap_or(candidate)
}

fn is_sha256_hex(raw: &str) -> bool {
    raw.len() == 64 && raw.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_file_hex(path: &Path) -> io::Result<String> {
    let file = fs::File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Sha256DirTree {
    sha256_tree: String,
    file_count: u64,
    total_bytes: u64,
}

fn sha256_dir_tree_hex(path: &Path) -> io::Result<Sha256DirTree> {
    let mut files = Vec::new();
    collect_files(path, &mut files)?;
    files.sort();

    let mut combined = Sha256::new();
    let mut total_bytes = 0_u64;
    for file in files.iter() {
        let rel = file.strip_prefix(path).map_err(io::Error::other)?;
        let rel = rel.to_string_lossy().replace('\\', "/");
        let digest = sha256_file_hex(file.as_path())?;
        let size = fs::metadata(file.as_path())?.len();
        combined.update(rel.as_bytes());
        combined.update(b"\0");
        combined.update(digest.as_bytes());
        combined.update(b"\0");
        combined.update(size.to_string().as_bytes());
        combined.update(b"\n");
        total_bytes += size;
    }
    Ok(Sha256DirTree {
        sha256_tree: hex::encode(combined.finalize()),
        file_count: files.len() as u64,
        total_bytes,
    })
}

fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        if child.is_dir() {
            collect_files(child.as_path(), files)?;
        } else if child.is_file() {
            files.push(child);
        }
    }
    Ok(())
}

pub(super) fn p2p_auto_detection_from_options(
    options: &CliOptions,
) -> NodeReachabilityAutoDetection {
    NodeReachabilityAutoDetection {
        observed_reachability: options.p2p_detected_reachability,
        hole_punch_viability: options.p2p_detected_hole_punch_viability,
        autonat_status: NodeAutoNatStatus::Unknown,
        public_port_reachability: NodePublicPortReachability::Unknown,
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

fn normalize_p2p_metadata_label(raw: &str, flag: &str) -> Result<String, String> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(format!("{flag} requires a non-empty value"));
    }
    Ok(normalized)
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

pub(super) fn parse_validator_signer_public_key_spec(
    raw: &str,
) -> Result<(String, String), String> {
    let (validator_id, public_key_hex) = raw.rsplit_once(':').ok_or_else(|| {
        "--node-validator-signer-public-key requires <validator_id:public_key_hex>".to_string()
    })?;
    let validator_id = validator_id.trim();
    if validator_id.is_empty() {
        return Err("--node-validator-signer-public-key validator_id cannot be empty".to_string());
    }
    let public_key_hex = public_key_hex.trim();
    if public_key_hex.is_empty() {
        return Err(
            "--node-validator-signer-public-key public_key_hex cannot be empty".to_string(),
        );
    }
    Ok((validator_id.to_string(), public_key_hex.to_string()))
}

pub(super) fn print_help() {
    let pos_defaults = chain_pos_defaults::defaults();
    println!(
        "Usage: oasis7_chain_runtime [options]\n\n\
Starts standalone chain/node runtime with status HTTP endpoints.\n\n\
Options:\n\
  --node-id <id>                    node identifier (default: {DEFAULT_NODE_ID})\n\
  --world-id <id>                   technical runtime partition id for the unified persistent world (default: {DEFAULT_WORLD_ID})\n\
  --storage-profile <name>          dev_local|release_default|soak_forensics (default: dev_local)\n\
  --traffic-profile <name>          default|triad_low_traffic (default: default)\n\
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
  --p2p-source-operator <label>     canonical operator label for peer diversity policy\n\
  --p2p-source-asn <label>          canonical ASN label for peer diversity policy\n\
  --p2p-max-ipv4-subnet-active-peers <n>\n\
                                    max active peers allowed in one IPv4 /24 before blocking\n\
  --node-tick-ms <n>                worker poll/fallback interval ms (default: {DEFAULT_NODE_TICK_MS})\n\
  --pos-slot-duration-ms <n>        PoS slot duration in milliseconds (default: {slot_duration_ms})\n\
  --pos-ticks-per-slot <n>          logical ticks per PoS slot (default: {ticks_per_slot})\n\
  --pos-proposal-tick-phase <n>     proposal trigger phase within slot tick window (default: {proposal_tick_phase})\n\
  --pos-adaptive-tick-scheduler     enable adaptive wait to next logical tick boundary\n\
  --pos-no-adaptive-tick-scheduler  disable adaptive scheduler (default)\n\
  --pos-slot-clock-genesis-unix-ms <n>\n\
                                    fixed slot clock genesis unix ms (default: auto)\n\
  --pos-max-past-slot-lag <n>       max accepted inbound stale slot lag (default: {max_past_slot_lag})\n\
  --node-validator <id:stake>       add validator stake (repeatable)\n\
  --node-validator-signer-public-key <id:public_key_hex>\n\
                                    override validator signer public key (repeatable)\n\
  --node-auto-attest-all            enable auto attesting validators\n\
  --node-no-auto-attest-all         disable auto attesting validators (default)\n\
  --node-gossip-bind <addr:port>    UDP gossip bind\n\
  --node-gossip-peer <addr:port>    UDP gossip peer (repeatable, requires --node-gossip-bind)\n\
  --replication-network-listen <multiaddr>\n\
                                    libp2p replication listen addr (repeatable, default: {DEFAULT_REPLICATION_NETWORK_LISTEN})\n\
  --replication-network-peer <multiaddr>\n\
                                    libp2p replication bootstrap peer (repeatable)\n\
  --replication-remote-writer-public-key <public_key_hex>\n\
                                    extra authorized replication fetch requester (repeatable)\n\
  --network-tier-manifest <path>    load formal network tier manifest json and bootstrap peer ref\n\
  --genesis-validator-registry <path>\n\
                                    initialize empty execution world validator registry from genesis manifest\n\
  --config <path>                   config file path for node keypair (default: {DEFAULT_CONFIG_FILE})\n\
  --runtime-root <path>             override chain runtime state root directory\n\
  --execution-bridge-state <path>   override execution bridge state file path\n\
  --execution-world-dir <path>      override execution world directory\n\
  --execution-records-dir <path>    override execution records directory\n\
  --storage-root <path>             override execution CAS/storage root\n\
  --replication-root <path>         override replication root directory\n\
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
        RewardAssetConfig::default().points_per_credit,
        slot_duration_ms = pos_defaults.slot_duration_ms,
        ticks_per_slot = pos_defaults.ticks_per_slot,
        proposal_tick_phase = pos_defaults.proposal_tick_phase,
        max_past_slot_lag = pos_defaults.max_past_slot_lag,
    );
}

fn default_p2p_node_role(node_role: NodeRole) -> PeerNodeRole {
    match node_role {
        NodeRole::Sequencer => PeerNodeRole::ValidatorCore,
        NodeRole::Storage => PeerNodeRole::FullStorage,
        NodeRole::Observer => PeerNodeRole::ObserverLight,
    }
}
