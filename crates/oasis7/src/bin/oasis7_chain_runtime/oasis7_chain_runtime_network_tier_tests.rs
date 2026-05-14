use super::build_chain_status_payload;
use super::cli::parse_options;
use oasis7::network_tier_manifest::{LoadedNetworkTierManifest, NETWORK_TIER_MANIFEST_SCHEMA_V1};
use oasis7::runtime::ReleaseSecurityPolicy;
use oasis7_node::{
    Libp2pReachabilitySnapshot, NodeConsensusSnapshot, NodeNetworkPolicy,
    NodeReachabilityAutoDetection, NodeRole, NodeSnapshot, NodeUserMode,
};
use oasis7_proto::distributed_dht::{PeerDeploymentMode, PeerNodeRole};
use oasis7_proto::storage_profile::{StorageProfile, StorageProfileConfig};
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

fn write_test_network_tier_manifest() -> (PathBuf, PathBuf) {
    let dir = temp_dir("manifest");
    let peers_path = dir.join("bootstrap.txt");
    fs::write(
        &peers_path,
        "/ip4/127.0.0.1/tcp/4100\n/dns4/bootstrap.example/tcp/4101\n",
    )
    .expect("write peers");
    let manifest_path = dir.join("manifest.json");
    fs::write(
        &manifest_path,
        format!(
            r#"{{
  "schema_version": "{NETWORK_TIER_MANIFEST_SCHEMA_V1}",
  "tier": "public_testnet",
  "status": "rehearsal",
  "network_id": "oasis7-public-testnet",
  "chain_id": "oasis7-public-testnet",
  "runtime_refs": {{
    "release_candidate_bundle_ref": "output/release-candidates/public-testnet.json",
    "genesis_ref": "doc/testing/templates/public-testnet-genesis.example.json",
    "bootstrap_peer_ref": "{}"
  }},
  "endpoint_policy": {{
    "rpc_ref": "https://public-testnet.example.invalid/rpc",
    "explorer_ref": "https://public-testnet.example.invalid/explorer",
    "faucet_ref": "https://public-testnet.example.invalid/faucet"
  }},
  "validator_policy": {{
    "governance_mode": "shared_ops",
    "validator_admission": "allowlist_or_governed_candidate",
    "target_validator_count": 4,
    "allow_observer_nodes": true
  }},
  "token_policy": {{
    "symbol": "OC",
    "faucet_mode": "guarded_testnet_faucet",
    "reset_policy": "resettable",
    "value_semantics": "testnet"
  }},
  "claims_policy": {{
    "allowed_claims": ["public_testnet"],
    "denied_claims": ["mainnet_live"]
  }},
  "promotion_policy": {{
    "promote_from": ["shared_devnet"],
    "required_gates": ["public_rpc_ready"]
  }},
  "evidence_refs": ["doc/testing/evidence/public-testnet.md"]
}}"#,
            peers_path.display()
        ),
    )
    .expect("write manifest");
    (dir, manifest_path)
}

#[test]
fn parse_options_loads_network_tier_manifest_and_bootstrap_peers() {
    let (dir, manifest_path) = write_test_network_tier_manifest();
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
    let (dir, manifest_path) = write_test_network_tier_manifest();
    let loaded = LoadedNetworkTierManifest::load(manifest_path.as_path()).expect("load manifest");
    let snapshot = NodeSnapshot {
        node_id: "node-a".to_string(),
        player_id: "player-a".to_string(),
        world_id: "live-a".to_string(),
        role: NodeRole::Observer,
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
