use super::build_chain_status_payload;
use super::cli::parse_options;
use oasis7::network_tier_manifest::{LoadedNetworkTierManifest, NETWORK_TIER_MANIFEST_SCHEMA_V1};
use oasis7::runtime::ReleaseSecurityPolicy;
use oasis7_node::{
    Libp2pReachabilitySnapshot, NodeConsensusSnapshot, NodeNetworkPolicy, NodePeerCommittedHead,
    NodeReachabilityAutoDetection, NodeRole, NodeSnapshot, NodeUserMode,
    NodeValidatorStakeProofSnapshot,
};
use oasis7_proto::distributed_dht::{PeerDeploymentMode, PeerNodeRole};
use oasis7_proto::storage_profile::{StorageProfile, StorageProfileConfig};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("oasis7-chain-runtime-{label}-{nonce}"));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn current_test_binary_sha256() -> String {
    let current_exe = std::env::current_exe().expect("current exe path");
    let current_bytes = fs::read(current_exe).expect("read current exe bytes");
    sha256_hex(current_bytes.as_slice())
}

fn write_test_network_tier_manifest(runtime_sha256: &str) -> (PathBuf, PathBuf) {
    write_test_network_tier_manifest_for_tier(runtime_sha256, "public_testnet")
}

fn write_test_network_tier_manifest_for_tier(
    runtime_sha256: &str,
    tier: &str,
) -> (PathBuf, PathBuf) {
    let dir = temp_dir("manifest");
    let peers_path = dir.join("bootstrap.txt");
    let genesis_path = dir.join("genesis.json");
    let bundle_path = dir.join("public-testnet.bundle.json");
    fs::write(
        &peers_path,
        "/ip4/127.0.0.1/tcp/4100\n/dns4/bootstrap.example/tcp/4101\n",
    )
    .expect("write peers");
    fs::write(&genesis_path, "{}\n").expect("write genesis");
    fs::write(
        &bundle_path,
        format!(
            r#"{{
  "runtime_build": {{
    "sha256": "{runtime_sha256}"
  }}
}}"#
        ),
    )
    .expect("write bundle");
    let manifest_path = dir.join("manifest.json");
    fs::write(
        &manifest_path,
        format!(
            r#"{{
  "schema_version": "{NETWORK_TIER_MANIFEST_SCHEMA_V1}",
  "tier": "{tier}",
  "status": "rehearsal",
  "network_id": "oasis7-{tier}",
  "chain_id": "oasis7-{tier}",
  "runtime_refs": {{
    "release_candidate_bundle_ref": "{}",
    "genesis_ref": "{}",
    "bootstrap_peer_ref": "{}"
  }},
  "endpoint_policy": {{
    "rpc_ref": "https://public-testnet.example.invalid/rpc",
    "explorer_ref": "https://public-testnet.example.invalid/explorer",
    "faucet_ref": {}
  }},
  "validator_policy": {{
    "governance_mode": "{}",
    "validator_admission": "{}",
    "target_validator_count": 4,
    "allow_observer_nodes": true
  }},
  "token_policy": {{
    "symbol": "OC",
    "faucet_mode": "{}",
    "reset_policy": "{}",
    "value_semantics": "{}"
  }},
  "claims_policy": {{
    "allowed_claims": [{}],
    "denied_claims": [{}]
  }},
  "promotion_policy": {{
    "promote_from": ["shared_devnet"],
    "required_gates": [{}]
  }},
  "evidence_refs": ["doc/testing/evidence/public-testnet.md"]
}}"#,
            bundle_path.display(),
            genesis_path.display(),
            peers_path.display(),
            if tier == "mainnet" {
                "null".to_string()
            } else {
                r#""https://public-testnet.example.invalid/faucet""#.to_string()
            },
            if tier == "mainnet" {
                "governance_registry"
            } else {
                "shared_ops"
            },
            if tier == "mainnet" {
                "governance_registry_only"
            } else {
                "allowlist_or_governed_candidate"
            },
            if tier == "mainnet" {
                "none"
            } else {
                "guarded_testnet_faucet"
            },
            if tier == "mainnet" {
                "frozen"
            } else {
                "resettable"
            },
            if tier == "mainnet" {
                "production"
            } else {
                "testnet"
            },
            if tier == "mainnet" {
                r#""mainnet_live""#.to_string()
            } else {
                r#""public_testnet""#.to_string()
            },
            if tier == "mainnet" {
                r#""faucet_claims""#.to_string()
            } else {
                r#""mainnet_live", "production_oc_settlement""#.to_string()
            },
            if tier == "mainnet" {
                r#""MAINNET-1", "MAINNET-2", "MAINNET-3", "MAINNET-4""#.to_string()
            } else {
                r#""shared_devnet_pass", "public_rpc_ready", "faucet_guard_ready", "reset_policy_announced""#.to_string()
            },
        ),
    )
    .expect("write manifest");
    (dir, manifest_path)
}

fn minimal_storage_metrics() -> super::storage_metrics::StorageMetricsSnapshot {
    super::storage_metrics::StorageMetricsSnapshot {
        storage_profile: "dev_local".to_string(),
        effective_budget: StorageProfileConfig::from(StorageProfile::DevLocal),
        bytes_by_dir: BTreeMap::new(),
        blob_counts: BTreeMap::new(),
        ref_count: 0,
        pin_count: 0,
        retained_heights: Vec::new(),
        checkpoint_count: 0,
        replay_summary: super::storage_metrics::StorageReplaySummary::default(),
        orphan_blob_count: 0,
        last_gc_at_ms: None,
        last_gc_result: "not_available".to_string(),
        last_gc_error: None,
        degraded_reason: None,
    }
}

fn minimal_reward_metrics() -> super::reward_runtime_worker::RewardRuntimeMetricsSnapshot {
    super::reward_runtime_worker::RewardRuntimeMetricsSnapshot {
        enabled: false,
        metrics_available: true,
        report_dir: String::new(),
        report_count: 0,
        latest_epoch_index: 0,
        latest_report_observed_at_unix_ms: 0,
        latest_total_distributed_points: 0,
        latest_minted_record_count: 0,
        cumulative_minted_record_count: 0,
        distfs_total_checks: 0,
        distfs_failed_checks: 0,
        distfs_failure_ratio: 0.0,
        settlement_apply_attempts_total: 0,
        settlement_apply_failures_total: 0,
        settlement_apply_failure_ratio: 0.0,
        invariant_ok: true,
        last_error: None,
    }
}

fn public_p2p_status() -> super::status_payload::ChainP2pStatus {
    super::status_payload::ChainP2pStatus {
        requested_user_mode: "auto_join".to_string(),
        recommended_user_mode: "public_entry".to_string(),
        effective_user_mode: "public_entry".to_string(),
        applied_effective_user_mode: Some("public_entry".to_string()),
        requires_explicit_public_entry_confirmation: false,
        detected_reachability: Some("public".to_string()),
        hole_punch_viability: "viable".to_string(),
        autonat_status: "public".to_string(),
        public_port_reachability: "reachable".to_string(),
        observed_public_addr: Some("/ip4/203.0.113.10/tcp/4001".to_string()),
        confirmed_external_direct_addrs: vec!["/ip4/203.0.113.10/tcp/4001".to_string()],
        relay_available: false,
        probe_stable: true,
        deployment_mode: "public".to_string(),
        node_role_claim: "validator_core".to_string(),
        rationale: Vec::new(),
    }
}

fn replication_with_active_peers(peer_count: usize) -> super::ChainReplicationDebugStatus {
    super::ChainReplicationDebugStatus {
        local_peer_id: "peer-local".to_string(),
        connected_peers: (0..peer_count)
            .map(|index| format!("peer-{index}"))
            .collect(),
        peer_healths: (0..peer_count)
            .map(|index| super::ChainPeerHealthStatus {
                peer_id: format!("peer-{index}"),
                status: "active".to_string(),
                issues: Vec::new(),
                discovery_sources: vec!["static_bootstrap".to_string()],
                active_path_kind: Some("direct".to_string()),
                source_operator: None,
                source_asn: None,
            })
            .collect(),
        registered_protocols: Vec::new(),
        protocol_retry_cooldown_peers: BTreeMap::new(),
        transport_retry_cooldown_peers: Vec::new(),
        request_peer_scores: BTreeMap::new(),
        recent_errors: Vec::new(),
    }
}

fn peer_head(
    node_id: &str,
    validator_id: Option<&str>,
    height: u64,
    observed_at_ms: i64,
) -> NodePeerCommittedHead {
    NodePeerCommittedHead {
        node_id: node_id.to_string(),
        validator_id: validator_id.map(str::to_string),
        height,
        block_hash: format!("block-{height}"),
        committed_at_ms: observed_at_ms,
        observed_at_ms,
        execution_block_hash: Some(format!("execution-{height}")),
        execution_state_root: Some(format!("state-{height}")),
    }
}

fn add_test_stake_proof_chain(consensus: &mut NodeConsensusSnapshot) {
    consensus.validator_set_hash = "test-validator-set-hash".to_string();
    consensus.validator_stake_root = "test-validator-stake-root".to_string();
    consensus.validator_stake_proofs = consensus
        .validator_stakes
        .iter()
        .map(|(validator_id, stake)| NodeValidatorStakeProofSnapshot {
            validator_id: validator_id.clone(),
            player_id: format!("player-{validator_id}"),
            stake: *stake,
            signer_public_key_hex: None,
            leaf_hash: format!("leaf-{validator_id}-{stake}"),
            proof: Vec::new(),
        })
        .collect();
}

#[test]
fn parse_options_loads_network_tier_manifest_and_bootstrap_peers() {
    let runtime_sha256 = current_test_binary_sha256();
    let (dir, manifest_path) = write_test_network_tier_manifest(runtime_sha256.as_str());
    let options = parse_options(
        [
            "--network-tier-manifest",
            manifest_path.to_string_lossy().as_ref(),
        ]
        .into_iter(),
    )
    .expect("parse should succeed");

    let loaded = options
        .loaded_network_tier_manifest
        .as_ref()
        .expect("manifest should load");
    assert_eq!(loaded.manifest.tier, "public_testnet");
    assert_eq!(loaded.bootstrap_peers.len(), 2);

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn status_payload_exposes_loaded_network_tier_manifest() {
    let runtime_sha256 = current_test_binary_sha256();
    let (dir, manifest_path) = write_test_network_tier_manifest(runtime_sha256.as_str());
    let loaded = LoadedNetworkTierManifest::load(manifest_path.as_path()).expect("load manifest");
    let snapshot = NodeSnapshot {
        node_id: "node-a".to_string(),
        player_id: "player-a".to_string(),
        world_id: "live-a".to_string(),
        role: NodeRole::Observer,
        replication_enabled: false,
        running: true,
        tick_count: 1,
        last_tick_unix_ms: Some(1_700_000_000_000),
        consensus: NodeConsensusSnapshot::default(),
        last_error: None,
    };
    let reward_runtime = super::reward_runtime_worker::RewardRuntimeMetricsSnapshot {
        enabled: true,
        metrics_available: true,
        report_dir: "/tmp/reports".to_string(),
        report_count: 0,
        latest_epoch_index: 0,
        latest_report_observed_at_unix_ms: 0,
        latest_total_distributed_points: 0,
        latest_minted_record_count: 0,
        cumulative_minted_record_count: 0,
        distfs_total_checks: 0,
        distfs_failed_checks: 0,
        distfs_failure_ratio: 0.0,
        settlement_apply_attempts_total: 0,
        settlement_apply_failures_total: 0,
        settlement_apply_failure_ratio: 0.0,
        invariant_ok: true,
        last_error: None,
    };
    let storage = super::storage_metrics::StorageMetricsSnapshot {
        storage_profile: "dev_local".to_string(),
        effective_budget: StorageProfileConfig::from(StorageProfile::DevLocal),
        bytes_by_dir: BTreeMap::new(),
        blob_counts: BTreeMap::new(),
        ref_count: 0,
        pin_count: 0,
        retained_heights: Vec::new(),
        checkpoint_count: 0,
        replay_summary: super::storage_metrics::StorageReplaySummary::default(),
        orphan_blob_count: 0,
        last_gc_at_ms: None,
        last_gc_result: "not_available".to_string(),
        last_gc_error: None,
        degraded_reason: None,
    };
    let recommendation = NodeNetworkPolicy::recommend_for_user_mode(
        NodeRole::Observer,
        NodeUserMode::PrivateSafe,
        NodeReachabilityAutoDetection::default(),
        false,
    )
    .expect("recommendation");
    let payload = build_chain_status_payload(
        snapshot,
        Path::new("/tmp/execution-world"),
        Some(&loaded),
        &recommendation,
        None,
        NodeNetworkPolicy {
            deployment_mode: PeerDeploymentMode::Public,
            node_role_claim: PeerNodeRole::Relay,
        },
        &Libp2pReachabilitySnapshot::default(),
        NodeReachabilityAutoDetection::default(),
        ReleaseSecurityPolicy::default(),
        reward_runtime,
        storage,
        super::wasm_status::ChainWasmStatus {
            metrics_available: false,
            observed_since_unix_ms: None,
            degraded_reason: None,
            build: super::wasm_status::ChainWasmBuildStatus {
                metrics_available: false,
                observed_since_unix_ms: None,
                degraded_reason: None,
                total_build_wall_ms: None,
                cargo_build_ms: None,
                canonicalize_ms: None,
                hash_ms: None,
                receipt_write_ms: None,
                metadata_write_ms: None,
                wasm_size_bytes: None,
            },
            executor: oasis7_wasm_executor::WasmExecutorMetricsSnapshot::empty(),
            router: oasis7_wasm_router::WasmRouterMetricsSnapshot::empty(),
        },
        super::ChainTrafficStatus {
            udp_gossip: None,
            libp2p_replication: oasis7_node::Libp2pTrafficMetricsSnapshot::default(),
        },
        super::transfer_submit_api::ChainTransferMetricsStatus {
            tracked_records: 0,
            accepted_count: 0,
            pending_count: 0,
            confirmed_count: 0,
            failed_count: 0,
            timeout_count: 0,
            inflight_count: 0,
            oldest_inflight_age_ms: None,
            recent_confirmation_latency:
                super::transfer_submit_api::ChainTransferLatencySummaryStatus {
                    sample_count: 0,
                    avg_latency_ms: None,
                    max_latency_ms: None,
                    p50_latency_ms: None,
                    p95_latency_ms: None,
                },
        },
        super::ChainReplicationDebugStatus::default(),
    );
    let tier = payload.network_tier.expect("network tier should exist");
    assert_eq!(tier.tier, "public_testnet");
    assert_eq!(tier.bootstrap_peer_count, 2);
    assert_eq!(tier.token_symbol, "OC");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn public_testnet_validator_network_head_uses_manifest_quorum() {
    let runtime_sha256 = current_test_binary_sha256();
    let (dir, manifest_path) = write_test_network_tier_manifest(runtime_sha256.as_str());
    let loaded = LoadedNetworkTierManifest::load(manifest_path.as_path()).expect("load manifest");
    let mut consensus = NodeConsensusSnapshot::default();
    consensus.committed_height = 10;
    consensus.network_committed_height = 10;
    consensus.replication_persisted_height = 10;
    consensus.known_peer_heads = 1;
    consensus.peer_heads = vec![NodePeerCommittedHead {
        node_id: "peer-a".to_string(),
        validator_id: None,
        height: 10,
        block_hash: "block-a".to_string(),
        committed_at_ms: 1_000,
        observed_at_ms: i64::MAX,
        execution_block_hash: Some("execution-a".to_string()),
        execution_state_root: Some("state-a".to_string()),
    }];
    let snapshot = NodeSnapshot {
        node_id: "node-a".to_string(),
        player_id: "player-a".to_string(),
        world_id: "live-a".to_string(),
        role: NodeRole::Sequencer,
        replication_enabled: true,
        running: true,
        tick_count: 1,
        last_tick_unix_ms: Some(1_700_000_000_000),
        consensus,
        last_error: None,
    };

    let network_head = super::status_payload::build_network_head_status(
        &snapshot,
        1_700_000_000_000,
        Some(&loaded),
    );

    assert_eq!(network_head.required_peer_count, 2);
    assert_eq!(network_head.fresh_peer_count, 1);
    assert_eq!(network_head.decision, "degraded");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn mainnet_validator_network_head_uses_stake_weighted_quorum() {
    let runtime_sha256 = current_test_binary_sha256();
    let (dir, manifest_path) =
        write_test_network_tier_manifest_for_tier(runtime_sha256.as_str(), "mainnet");
    let loaded = LoadedNetworkTierManifest::load(manifest_path.as_path()).expect("load manifest");
    let mut consensus = NodeConsensusSnapshot::default();
    consensus.committed_height = 10;
    consensus.network_committed_height = 10;
    consensus.replication_persisted_height = 10;
    consensus.known_peer_heads = 2;
    consensus.validator_stakes = BTreeMap::from([
        ("validator-a".to_string(), 40),
        ("validator-b".to_string(), 34),
        ("validator-c".to_string(), 26),
    ]);
    consensus.required_stake = 67;
    consensus.total_stake = 100;
    add_test_stake_proof_chain(&mut consensus);
    consensus.peer_heads = vec![
        peer_head("node-b", Some("validator-b"), 10, i64::MAX),
        peer_head("node-c", Some("validator-c"), 10, i64::MAX),
    ];
    let snapshot = NodeSnapshot {
        node_id: "validator-a".to_string(),
        player_id: "player-a".to_string(),
        world_id: "mainnet-a".to_string(),
        role: NodeRole::Sequencer,
        replication_enabled: true,
        running: true,
        tick_count: 1,
        last_tick_unix_ms: Some(1_700_000_000_000),
        consensus,
        last_error: None,
    };

    let network_head = super::status_payload::build_network_head_status(
        &snapshot,
        1_700_000_000_000,
        Some(&loaded),
    );

    assert_eq!(network_head.quorum_mode, "stake_weighted");
    assert_eq!(network_head.observed_stake, 60);
    assert_eq!(network_head.required_stake, 67);
    assert!(!network_head.stake_quorum_met);
    assert_eq!(network_head.decision, "degraded");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn mainnet_validator_readiness_blocks_when_stake_mapping_is_unavailable() {
    let runtime_sha256 = current_test_binary_sha256();
    let (dir, manifest_path) =
        write_test_network_tier_manifest_for_tier(runtime_sha256.as_str(), "mainnet");
    let loaded = LoadedNetworkTierManifest::load(manifest_path.as_path()).expect("load manifest");
    let mut consensus = NodeConsensusSnapshot::default();
    consensus.committed_height = 10;
    consensus.network_committed_height = 10;
    consensus.replication_persisted_height = 10;
    consensus.known_peer_heads = 3;
    consensus.peer_heads = vec![
        peer_head("node-b", None, 10, i64::MAX),
        peer_head("node-c", None, 10, i64::MAX),
        peer_head("node-d", None, 10, i64::MAX),
    ];
    let snapshot = NodeSnapshot {
        node_id: "validator-a".to_string(),
        player_id: "player-a".to_string(),
        world_id: "mainnet-a".to_string(),
        role: NodeRole::Sequencer,
        replication_enabled: true,
        running: true,
        tick_count: 1,
        last_tick_unix_ms: Some(1_700_000_000_000),
        consensus,
        last_error: None,
    };
    let p2p = public_p2p_status();
    let policy = super::status_payload::readiness_policy(&snapshot, Some(&loaded));
    let network_head = super::status_payload::build_network_head_status(
        &snapshot,
        1_700_000_000_000,
        Some(&loaded),
    );
    let status = super::status_payload::build_chain_node_observability_status(
        &snapshot,
        &minimal_storage_metrics(),
        &minimal_reward_metrics(),
        &replication_with_active_peers(3),
        &network_head,
        &p2p,
        &policy,
        1_700_000_000_000,
    );

    assert_eq!(policy.quorum_mode, "count_fallback_stake_unavailable");
    assert!(!status.ready);
    assert!(status
        .alerts
        .iter()
        .any(|alert| alert.code == "consensus_stake_quorum_unavailable"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn mainnet_sync_lag_stalls_after_policy_window() {
    let runtime_sha256 = current_test_binary_sha256();
    let (dir, manifest_path) =
        write_test_network_tier_manifest_for_tier(runtime_sha256.as_str(), "mainnet");
    let loaded = LoadedNetworkTierManifest::load(manifest_path.as_path()).expect("load manifest");
    let mut consensus = NodeConsensusSnapshot::default();
    consensus.committed_height = 10;
    consensus.network_committed_height = 12;
    consensus.replication_persisted_height = 10;
    consensus.last_committed_at_ms = Some(1_700_000_000_000);
    consensus.known_peer_heads = 2;
    consensus.validator_stakes = BTreeMap::from([
        ("validator-b".to_string(), 40),
        ("validator-c".to_string(), 34),
    ]);
    consensus.required_stake = 67;
    consensus.total_stake = 100;
    add_test_stake_proof_chain(&mut consensus);
    consensus.peer_heads = vec![
        peer_head("node-b", Some("validator-b"), 12, i64::MAX),
        peer_head("node-c", Some("validator-c"), 12, i64::MAX),
    ];
    let snapshot = NodeSnapshot {
        node_id: "validator-a".to_string(),
        player_id: "player-a".to_string(),
        world_id: "mainnet-a".to_string(),
        role: NodeRole::Sequencer,
        replication_enabled: true,
        running: true,
        tick_count: 1,
        last_tick_unix_ms: Some(1_700_000_020_000),
        consensus,
        last_error: None,
    };
    let policy = super::status_payload::readiness_policy(&snapshot, Some(&loaded));
    let network_head = super::status_payload::build_network_head_status(
        &snapshot,
        1_700_000_020_000,
        Some(&loaded),
    );
    let status = super::status_payload::build_chain_node_observability_status(
        &snapshot,
        &minimal_storage_metrics(),
        &minimal_reward_metrics(),
        &replication_with_active_peers(2),
        &network_head,
        &public_p2p_status(),
        &policy,
        1_700_000_020_000,
    );

    assert!(!status.ready);
    assert!(status
        .alerts
        .iter()
        .any(|alert| alert.code == "consensus_sync_stalled"));
    let sync = super::status_payload::build_sync_status(
        &network_head,
        status.network_height_lag,
        &policy,
        &snapshot,
        1_700_000_020_000,
    );
    assert_eq!(sync.status, "stalled");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn mainnet_validator_relay_policy_requires_governed_redundancy_and_surfaces_slashing_boundary() {
    let runtime_sha256 = current_test_binary_sha256();
    let (dir, manifest_path) =
        write_test_network_tier_manifest_for_tier(runtime_sha256.as_str(), "mainnet");
    let loaded = LoadedNetworkTierManifest::load(manifest_path.as_path()).expect("load manifest");
    let mut consensus = NodeConsensusSnapshot::default();
    consensus.committed_height = 10;
    consensus.network_committed_height = 10;
    consensus.replication_persisted_height = 10;
    consensus.known_peer_heads = 2;
    consensus.validator_stakes = BTreeMap::from([
        ("validator-b".to_string(), 40),
        ("validator-c".to_string(), 34),
    ]);
    consensus.required_stake = 67;
    consensus.total_stake = 100;
    add_test_stake_proof_chain(&mut consensus);
    consensus.peer_heads = vec![
        peer_head("node-b", Some("validator-b"), 10, i64::MAX),
        peer_head("node-c", Some("validator-c"), 10, i64::MAX),
    ];
    let snapshot = NodeSnapshot {
        node_id: "validator-a".to_string(),
        player_id: "player-a".to_string(),
        world_id: "mainnet-a".to_string(),
        role: NodeRole::Sequencer,
        replication_enabled: true,
        running: true,
        tick_count: 1,
        last_tick_unix_ms: Some(1_700_000_000_000),
        consensus,
        last_error: None,
    };
    let mut p2p = public_p2p_status();
    p2p.detected_reachability = None;
    p2p.autonat_status = "unknown".to_string();
    p2p.public_port_reachability = "unknown".to_string();
    p2p.observed_public_addr = None;
    p2p.confirmed_external_direct_addrs = Vec::new();
    p2p.relay_available = true;
    let policy = super::status_payload::readiness_policy(&snapshot, Some(&loaded));
    let network_head = super::status_payload::build_network_head_status(
        &snapshot,
        1_700_000_000_000,
        Some(&loaded),
    );
    let status = super::status_payload::build_chain_node_observability_status(
        &snapshot,
        &minimal_storage_metrics(),
        &minimal_reward_metrics(),
        &replication_with_active_peers(1),
        &network_head,
        &p2p,
        &policy,
        1_700_000_000_000,
    );

    assert_eq!(policy.relay_policy, "public_direct_or_governed_relay");
    assert_eq!(policy.slashing_policy, "evidence_only_readiness_gate");
    assert!(!policy.slashing_enforced);
    assert!(!status.reachability_policy_ok);
    assert!(status
        .alerts
        .iter()
        .any(|alert| alert.code == "p2p_reachability_degraded"));
    assert!(status
        .alerts
        .iter()
        .any(|alert| alert.code == "mainnet_slashing_evidence_only"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn parse_options_rejects_network_tier_manifest_when_runtime_bundle_hash_mismatches_current_binary()
{
    let (dir, manifest_path) = write_test_network_tier_manifest(
        "0000000000000000000000000000000000000000000000000000000000000000",
    );
    let err = parse_options(
        [
            "--network-tier-manifest",
            manifest_path.to_string_lossy().as_ref(),
        ]
        .into_iter(),
    )
    .expect_err("parse should fail on runtime bundle drift");
    assert!(
        err.contains("network tier runtime bundle hash mismatch"),
        "unexpected mismatch error: {err}"
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn parse_options_rejects_network_tier_manifest_when_runtime_bundle_hash_is_malformed() {
    let (dir, manifest_path) = write_test_network_tier_manifest("not-a-sha256");
    let err = parse_options(
        [
            "--network-tier-manifest",
            manifest_path.to_string_lossy().as_ref(),
        ]
        .into_iter(),
    )
    .expect_err("parse should fail on malformed runtime bundle hash");
    assert!(
        err.contains("invalid runtime_build.sha256"),
        "unexpected malformed hash error: {err}"
    );

    let _ = fs::remove_dir_all(dir);
}
