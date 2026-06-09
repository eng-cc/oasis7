use super::*;
use oasis7::network_tier_manifest::LoadedNetworkTierManifest;
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
    let dir = std::env::temp_dir().join(format!("oasis7-state-sync-{label}-{nonce}"));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn public_testnet_manifest_fixture() -> (PathBuf, LoadedNetworkTierManifest) {
    let dir = temp_dir("public-testnet-manifest");
    let peers_path = dir.join("bootstrap.txt");
    let genesis_path = dir.join("genesis.json");
    let bundle_path = dir.join("bundle.json");
    let manifest_path = dir.join("manifest.json");
    fs::write(&peers_path, "/ip4/127.0.0.1/tcp/4100\n").expect("write peers");
    fs::write(&genesis_path, "{}\n").expect("write genesis");
    fs::write(
        &bundle_path,
        "{ \"runtime_build\": { \"sha256\": \"fixture\" } }\n",
    )
    .expect("write bundle");
    fs::write(
        &manifest_path,
        format!(
            r#"{{
  "schema_version": "oasis7.network_tier_manifest.v1",
  "tier": "public_testnet",
  "status": "rehearsal",
  "network_id": "oasis7-public-testnet",
  "chain_id": "oasis7-public-testnet",
  "runtime_refs": {{
    "release_candidate_bundle_ref": "{}",
    "genesis_ref": "{}",
    "bootstrap_peer_ref": "{}"
  }},
  "endpoint_policy": {{
    "rpc_ref": "https://rpc.example.invalid",
    "explorer_ref": "https://explorer.example.invalid",
    "faucet_ref": "https://faucet.example.invalid"
  }},
  "validator_policy": {{
    "governance_mode": "shared_ops",
    "validator_admission": "allowlist_or_governed_candidate",
    "target_validator_count": 2,
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
    "denied_claims": ["mainnet_live", "production_oc_settlement"]
  }},
  "promotion_policy": {{
    "promote_from": ["shared_devnet"],
    "required_gates": ["shared_devnet_pass"]
  }},
  "evidence_refs": []
}}"#,
            bundle_path.display(),
            genesis_path.display(),
            peers_path.display()
        ),
    )
    .expect("write manifest");
    let loaded = LoadedNetworkTierManifest::load(manifest_path.as_path()).expect("load manifest");
    (dir, loaded)
}

#[test]
fn build_chain_status_payload_marks_replication_gap_blocked_unhealthy() {
    let mut consensus = NodeConsensusSnapshot::default();
    consensus.committed_height = 10;
    consensus.network_committed_height = 30;
    consensus.replication_persisted_height = 4;
    consensus.replication_gap_sync_blocked_height = Some(5);
    consensus.replication_gap_sync_blocked_reason =
        Some("replication gap sync blocked: missing commit height 5".to_string());
    consensus.replication_gap_sync_repair_attempt_height = Some(5);
    consensus.replication_gap_sync_repair_attempt_summary =
        Some("generic:found=false;peer:p1:found=false".to_string());
    let snapshot = NodeSnapshot {
        node_id: "node-gap".to_string(),
        player_id: "player-gap".to_string(),
        world_id: "world-gap".to_string(),
        role: NodeRole::Sequencer,
        replication_enabled: true,
        running: true,
        tick_count: 1,
        last_tick_unix_ms: None,
        consensus,
        last_error: None,
    };
    let recommendation = NodeNetworkPolicy::recommend_for_user_mode(
        NodeRole::Sequencer,
        NodeUserMode::PrivateSafe,
        NodeReachabilityAutoDetection::default(),
        false,
    )
    .expect("recommendation");
    let payload = build_chain_status_payload(
        snapshot,
        Path::new("/tmp/execution-world"),
        None,
        &recommendation,
        None,
        NodeNetworkPolicy {
            deployment_mode: PeerDeploymentMode::Private,
            node_role_claim: PeerNodeRole::ValidatorCore,
        },
        &Libp2pReachabilitySnapshot::default(),
        NodeReachabilityAutoDetection::default(),
        ReleaseSecurityPolicy::default(),
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
        },
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
        },
        super::observability_tests::sample_wasm_status(),
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

    assert!(!payload.ok);
    assert_eq!(payload.observability.status, "critical");
    assert_eq!(payload.observability.replication_state_gap, 6);
    let consensus = &payload.consensus;
    assert_eq!(consensus.replication_gap_sync_blocked_height, Some(5));
    assert!(consensus.state_sync_fallback_required);
    assert!(!consensus.state_sync_snapshot_available);
    assert_eq!(
        consensus.state_sync_trusted_checkpoint_required_height,
        Some(30)
    );
    assert_eq!(
        consensus.replication_gap_sync_repair_attempt_height,
        Some(5)
    );
    assert_eq!(
        consensus
            .replication_gap_sync_repair_attempt_summary
            .as_deref(),
        Some("generic:found=false;peer:p1:found=false")
    );
    assert!(payload.consensus.consensus_participation_held);
    assert_eq!(
        consensus.consensus_participation_hold_reason.as_deref(),
        Some("replication_gap_sync_blocked_height=5")
    );
    let alert_codes = payload
        .observability
        .alerts
        .iter()
        .map(|alert| alert.code.as_str())
        .collect::<Vec<_>>();
    assert!(alert_codes.contains(&"replication_gap_sync_blocked"));
    assert!(alert_codes.contains(&"consensus_replication_state_gap"));
    assert!(alert_codes.contains(&"consensus_participation_held"));
    assert!(alert_codes.contains(&"state_sync_fallback_required"));
}

#[test]
fn build_chain_status_payload_keeps_catching_up_observer_out_of_fallback_mode() {
    let mut consensus = NodeConsensusSnapshot::default();
    consensus.committed_height = 257;
    consensus.network_committed_height = 7090;
    consensus.replication_persisted_height = 257;
    let snapshot = NodeSnapshot {
        node_id: "node-catching-up".to_string(),
        player_id: "player-catching-up".to_string(),
        world_id: "world-catching-up".to_string(),
        role: NodeRole::Observer,
        replication_enabled: true,
        running: true,
        tick_count: 6,
        last_tick_unix_ms: None,
        consensus,
        last_error: None,
    };
    let recommendation = NodeNetworkPolicy::recommend_for_user_mode(
        NodeRole::Observer,
        NodeUserMode::AutoJoin,
        NodeReachabilityAutoDetection::default(),
        false,
    )
    .expect("recommendation");
    let payload = build_chain_status_payload(
        snapshot,
        Path::new("/tmp/execution-world"),
        None,
        &recommendation,
        None,
        NodeNetworkPolicy {
            deployment_mode: PeerDeploymentMode::Private,
            node_role_claim: PeerNodeRole::ObserverLight,
        },
        &Libp2pReachabilitySnapshot::default(),
        NodeReachabilityAutoDetection::default(),
        ReleaseSecurityPolicy::default(),
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
        },
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
        },
        super::observability_tests::sample_wasm_status(),
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

    let consensus = &payload.consensus;
    assert!(consensus.consensus_participation_held);
    assert!(consensus
        .consensus_participation_hold_reason
        .as_deref()
        .is_some_and(|reason| reason.starts_with("network_height_lag=6833 allowed=")));
    assert!(!consensus.state_sync_fallback_required);
    assert_eq!(
        consensus.state_sync_trusted_checkpoint_required_height,
        None
    );
    assert_eq!(consensus.state_sync_fallback_reason, None);

    let alert_codes = payload
        .observability
        .alerts
        .iter()
        .map(|alert| alert.code.as_str())
        .collect::<Vec<_>>();
    assert!(alert_codes.contains(&"consensus_network_lag"));
    assert!(alert_codes.contains(&"consensus_participation_held"));
    assert!(!alert_codes.contains(&"state_sync_fallback_required"));
}

#[test]
fn build_chain_status_payload_marks_stalled_observer_fallback_required() {
    let mut consensus = NodeConsensusSnapshot::default();
    consensus.committed_height = 257;
    consensus.network_committed_height = 7090;
    consensus.replication_persisted_height = 257;
    consensus.last_committed_at_ms = Some(0);
    let snapshot = NodeSnapshot {
        node_id: "node-stalled".to_string(),
        player_id: "player-stalled".to_string(),
        world_id: "world-stalled".to_string(),
        role: NodeRole::Observer,
        replication_enabled: true,
        running: true,
        tick_count: 6,
        last_tick_unix_ms: Some(i64::MAX),
        consensus,
        last_error: None,
    };
    let recommendation = NodeNetworkPolicy::recommend_for_user_mode(
        NodeRole::Observer,
        NodeUserMode::AutoJoin,
        NodeReachabilityAutoDetection::default(),
        false,
    )
    .expect("recommendation");
    let (_manifest_dir, loaded_manifest) = public_testnet_manifest_fixture();
    let payload = build_chain_status_payload(
        snapshot,
        Path::new("/tmp/execution-world"),
        Some(&loaded_manifest),
        &recommendation,
        None,
        NodeNetworkPolicy {
            deployment_mode: PeerDeploymentMode::Private,
            node_role_claim: PeerNodeRole::ObserverLight,
        },
        &Libp2pReachabilitySnapshot::default(),
        NodeReachabilityAutoDetection::default(),
        ReleaseSecurityPolicy::default(),
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
        },
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
        },
        super::observability_tests::sample_wasm_status(),
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

    let consensus = &payload.consensus;
    assert!(consensus.state_sync_fallback_required);
    assert_eq!(
        consensus.state_sync_trusted_checkpoint_required_height,
        Some(7090)
    );
    assert!(consensus
        .state_sync_fallback_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("network_height_lag=6833")));

    let alert_codes = payload
        .observability
        .alerts
        .iter()
        .map(|alert| alert.code.as_str())
        .collect::<Vec<_>>();
    assert!(alert_codes.contains(&"consensus_sync_stalled"));
    assert!(alert_codes.contains(&"state_sync_fallback_required"));
}
