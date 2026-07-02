use super::cli::TrafficProfile;
use super::{build_chain_status_payload, release_security_policy_for_storage_profile};
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

fn temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    std::env::temp_dir().join(format!("oasis7-status-payload-{prefix}-{unique}"))
}

fn minimal_reward_runtime_metrics() -> super::reward_runtime_worker::RewardRuntimeMetricsSnapshot {
    super::reward_runtime_worker::RewardRuntimeMetricsSnapshot {
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
    }
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

fn minimal_wasm_status() -> super::wasm_status::ChainWasmStatus {
    super::wasm_status::ChainWasmStatus {
        metrics_available: false,
        observed_since_unix_ms: None,
        degraded_reason: Some("build metrics path not configured".to_string()),
        build: super::wasm_status::ChainWasmBuildStatus {
            metrics_available: false,
            observed_since_unix_ms: None,
            degraded_reason: Some("build metrics path not configured".to_string()),
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
    }
}

fn minimal_transfer_status() -> super::transfer_submit_api::ChainTransferMetricsStatus {
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
    }
}

fn build_minimal_status_payload(
    execution_records_dir: Option<&Path>,
) -> super::status_payload::ChainStatusResponse {
    let snapshot = NodeSnapshot {
        node_id: "node-a".to_string(),
        player_id: "player-a".to_string(),
        world_id: "live-a".to_string(),
        role: NodeRole::Sequencer,
        replication_enabled: false,
        running: true,
        tick_count: 1,
        last_tick_unix_ms: Some(1_700_000_000_000),
        consensus: NodeConsensusSnapshot::default(),
        last_error: None,
    };
    let recommendation = NodeNetworkPolicy::recommend_for_user_mode(
        NodeRole::Sequencer,
        NodeUserMode::PrivateSafe,
        NodeReachabilityAutoDetection::default(),
        false,
    )
    .expect("recommendation");

    build_chain_status_payload(
        snapshot,
        Path::new("/tmp/execution-world"),
        execution_records_dir,
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
        minimal_reward_runtime_metrics(),
        minimal_storage_metrics(),
        minimal_wasm_status(),
        super::ChainTrafficStatus {
            udp_gossip: None,
            libp2p_replication: oasis7_node::Libp2pTrafficMetricsSnapshot::default(),
        },
        minimal_transfer_status(),
        super::ChainReplicationDebugStatus::default(),
    )
}

#[test]
fn production_release_policy_status_payload_reports_effective_policy() {
    let snapshot = NodeSnapshot {
        node_id: "node-a".to_string(),
        player_id: "player-a".to_string(),
        world_id: "live-a".to_string(),
        role: NodeRole::Sequencer,
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
        storage_profile: "release_default".to_string(),
        effective_budget: StorageProfileConfig::from(StorageProfile::ReleaseDefault),
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
    let release_security_policy =
        release_security_policy_for_storage_profile(StorageProfile::ReleaseDefault);

    let payload = build_chain_status_payload(
        snapshot,
        Path::new("/tmp/execution-world"),
        None,
        None,
        &NodeNetworkPolicy::recommend_for_user_mode(
            NodeRole::Sequencer,
            NodeUserMode::PrivateSafe,
            NodeReachabilityAutoDetection::default(),
            false,
        )
        .expect("recommendation"),
        Some("private_safe".to_string()),
        NodeNetworkPolicy {
            deployment_mode: PeerDeploymentMode::Private,
            node_role_claim: PeerNodeRole::ValidatorCore,
        },
        &Libp2pReachabilitySnapshot::default(),
        NodeReachabilityAutoDetection::default(),
        release_security_policy.clone(),
        reward_runtime,
        storage,
        super::wasm_status::ChainWasmStatus {
            metrics_available: false,
            observed_since_unix_ms: None,
            degraded_reason: Some("build metrics path not configured".to_string()),
            build: super::wasm_status::ChainWasmBuildStatus {
                metrics_available: false,
                observed_since_unix_ms: None,
                degraded_reason: Some("build metrics path not configured".to_string()),
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

    assert_eq!(
        payload.release_security_policy,
        ReleaseSecurityPolicy::production_hardened()
    );
    assert!(payload.release_security_policy.is_production_hardened());
    assert_eq!(payload.p2p.effective_user_mode, "private_safe");
    assert_eq!(
        payload.p2p.applied_effective_user_mode.as_deref(),
        Some("private_safe")
    );
}

#[test]
fn status_payload_reports_effective_policy_when_raw_override_differs_from_recommendation() {
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
        None,
        None,
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
            degraded_reason: Some("build metrics path not configured".to_string()),
            build: super::wasm_status::ChainWasmBuildStatus {
                metrics_available: false,
                observed_since_unix_ms: None,
                degraded_reason: Some("build metrics path not configured".to_string()),
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

    assert_eq!(payload.p2p.requested_user_mode, "private_safe");
    assert_eq!(payload.p2p.effective_user_mode, "private_safe");
    assert_eq!(payload.p2p.applied_effective_user_mode, None);
    assert_eq!(payload.p2p.deployment_mode, "public");
    assert_eq!(payload.p2p.node_role_claim, "relay");
}

#[test]
fn feedback_p2p_is_disabled_for_observer_role() {
    assert!(
        super::feedback_p2p_config_for_role(NodeRole::Observer, TrafficProfile::Default).is_none()
    );
    assert!(
        super::feedback_p2p_config_for_role(NodeRole::Sequencer, TrafficProfile::Default).is_some()
    );
    assert!(
        super::feedback_p2p_config_for_role(NodeRole::Storage, TrafficProfile::Default).is_some()
    );
}

#[test]
fn status_payload_reports_chain_proof_unavailable_without_records_dir() {
    let payload = build_minimal_status_payload(None);

    assert_eq!(payload.chain_proof.status, "unavailable");
    assert!(payload.chain_proof.latest_world_head_proof.is_none());
    assert_eq!(
        payload.chain_proof.load_error.as_deref(),
        Some("execution_records_dir_unconfigured")
    );
    assert!(
        payload
            .chain_proof
            .does_not_claim
            .contains(&"ready_for_live_candidate".to_string())
    );
    assert_ne!(payload.readiness.status, "ready_for_live_candidate");
}

#[test]
fn status_payload_reports_latest_chain_proof_metadata_from_execution_record() {
    let dir = temp_dir("chain-proof-available");
    fs::create_dir_all(dir.as_path()).expect("create records dir");
    fs::write(
        dir.join("latest.json"),
        br#"{
          "schema_version": 3,
          "world_id": "live-a",
          "height": 42,
          "node_block_hash": "node-block-42",
          "action_root": "action-root-42",
          "execution_block_hash": "exec-block-42",
          "execution_state_root": "exec-state-42",
          "world_head_proof_ref": "proof-ref-42",
          "world_head_proof_hash": "proof-hash-42",
          "checkpoint_ref": "00000000000000000042/manifest.json"
        }"#,
    )
    .expect("write latest record");

    let payload = build_minimal_status_payload(Some(dir.as_path()));

    assert_eq!(payload.chain_proof.status, "available");
    assert_eq!(
        payload.chain_proof.schema_version,
        "oasis7.chain_proof_status.v1"
    );
    assert_eq!(payload.chain_proof.proof_contract, "WorldHeadProofV1");
    assert_eq!(
        payload.chain_proof.claim_boundary,
        "head_execution_checkpoint_evidence_only_not_light_client_or_mainnet_readiness"
    );
    let expected_source_record_path = dir.join("latest.json").display().to_string();
    assert_eq!(
        payload.chain_proof.source_record_path.as_deref(),
        Some(expected_source_record_path.as_str())
    );
    assert!(payload.chain_proof.load_error.is_none());
    let proof = payload
        .chain_proof
        .latest_world_head_proof
        .as_ref()
        .expect("latest proof metadata");
    assert_eq!(proof.schema_version, 1);
    assert_eq!(proof.world_id, "live-a");
    assert_eq!(proof.height, 42);
    assert_eq!(proof.execution_block_hash, "exec-block-42");
    assert_eq!(proof.execution_state_root, "exec-state-42");
    assert_eq!(proof.node_block_hash, "node-block-42");
    assert_eq!(proof.action_root, "action-root-42");
    assert_eq!(proof.world_head_proof_ref, "proof-ref-42");
    assert_eq!(proof.proof_hash, "proof-hash-42");
    assert_eq!(
        proof.checkpoint_ref.as_deref(),
        Some("00000000000000000042/manifest.json")
    );
    assert_ne!(payload.readiness.status, "ready_for_live_candidate");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn status_payload_marks_chain_proof_stale_when_pointer_missing() {
    let dir = temp_dir("chain-proof-missing-pointer");
    fs::create_dir_all(dir.as_path()).expect("create records dir");
    fs::write(
        dir.join("latest.json"),
        br#"{
          "schema_version": 3,
          "world_id": "live-a",
          "height": 42,
          "node_block_hash": "node-block-42",
          "action_root": "action-root-42",
          "execution_block_hash": "exec-block-42",
          "execution_state_root": "exec-state-42",
          "world_head_proof_hash": "proof-hash-42"
        }"#,
    )
    .expect("write latest record");

    let payload = build_minimal_status_payload(Some(dir.as_path()));

    assert_eq!(payload.chain_proof.status, "stale_or_invalid");
    assert!(payload.chain_proof.latest_world_head_proof.is_none());
    assert!(
        payload
            .chain_proof
            .load_error
            .as_deref()
            .unwrap_or_default()
            .contains("world_head_proof_ref")
    );
    assert_ne!(payload.readiness.status, "ready_for_live_candidate");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn status_payload_marks_chain_proof_stale_for_malformed_latest_record() {
    let dir = temp_dir("chain-proof-malformed");
    fs::create_dir_all(dir.as_path()).expect("create records dir");
    fs::write(dir.join("latest.json"), b"{not-json").expect("write malformed latest record");

    let payload = build_minimal_status_payload(Some(dir.as_path()));

    assert_eq!(payload.chain_proof.status, "stale_or_invalid");
    assert!(payload.chain_proof.latest_world_head_proof.is_none());
    assert!(
        payload
            .chain_proof
            .load_error
            .as_deref()
            .unwrap_or_default()
            .contains("parse latest execution record failed")
    );
    assert_ne!(payload.readiness.status, "ready_for_live_candidate");

    let _ = fs::remove_dir_all(dir);
}
