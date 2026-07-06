use super::cli::TrafficProfile;
use super::{build_chain_status_payload, release_security_policy_for_storage_profile};
use ed25519_dalek::{Signer, SigningKey};
use oasis7::runtime::{
    Manifest, ModuleAbiContract, ModuleActivation, ModuleArtifactIdentity, ModuleChangeSet,
    ModuleKind, ModuleLimits, ModuleManifest, ModuleRole, ModuleSubscription,
    ModuleSubscriptionStage, PolicySet, ProposalDecision, ReleaseSecurityPolicy, World,
};
use oasis7::simulator::{
    RuntimePerfBottleneck, RuntimePerfHealth, RuntimePerfSeriesSnapshot, RuntimePerfSnapshot,
};
use oasis7_node::{
    Libp2pReachabilitySnapshot, NodeConsensusSnapshot, NodeNetworkPolicy,
    NodeReachabilityAutoDetection, NodeRole, NodeSnapshot, NodeUserMode,
};
use oasis7_proto::distributed_dht::{PeerDeploymentMode, PeerNodeRole};
use oasis7_proto::storage_profile::{StorageProfile, StorageProfileConfig};
use oasis7_wasm_abi::{ModuleCallFailure, ModuleCallRequest, ModuleOutput, ModuleSandbox};
use sha2::Digest;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const TEST_MODULE_ARTIFACT_SIGNER_NODE_ID: &str = "test.module.release.signer";

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
    build_minimal_status_payload_with_world_dir(
        Path::new("/tmp/execution-world"),
        execution_records_dir,
    )
}

fn build_minimal_status_payload_with_world_dir(
    execution_world_dir: &Path,
    execution_records_dir: Option<&Path>,
) -> super::status_payload::ChainStatusResponse {
    build_minimal_status_payload_with_world_dir_and_runtime_perf(
        execution_world_dir,
        execution_records_dir,
        None,
    )
}

fn build_minimal_status_payload_with_runtime_perf(
    runtime_perf: RuntimePerfSnapshot,
) -> super::status_payload::ChainStatusResponse {
    build_minimal_status_payload_with_world_dir_and_runtime_perf(
        Path::new("/tmp/execution-world"),
        None,
        Some(runtime_perf),
    )
}

fn build_minimal_status_payload_with_world_dir_and_runtime_perf(
    execution_world_dir: &Path,
    execution_records_dir: Option<&Path>,
    runtime_perf: Option<RuntimePerfSnapshot>,
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
        execution_world_dir,
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
        runtime_perf,
        super::ChainTrafficStatus {
            udp_gossip: None,
            libp2p_replication: oasis7_node::Libp2pTrafficMetricsSnapshot::default(),
        },
        minimal_transfer_status(),
        super::ChainReplicationDebugStatus::default(),
    )
}

fn runtime_perf_series(p95_ms: f64, over_budget_ratio_ppm: u64) -> RuntimePerfSeriesSnapshot {
    RuntimePerfSeriesSnapshot {
        samples_total: 10,
        samples_window: 10,
        budget_ms: 20.0,
        last_ms: p95_ms,
        avg_ms: p95_ms,
        min_ms: p95_ms,
        max_ms: p95_ms,
        p50_ms: p95_ms,
        p95_ms,
        p99_ms: p95_ms,
        over_budget_total: 1,
        over_budget_ratio_ppm,
    }
}

#[derive(Default)]
struct CountingSandbox {
    calls: usize,
}

impl ModuleSandbox for CountingSandbox {
    fn call(&mut self, _request: &ModuleCallRequest) -> Result<ModuleOutput, ModuleCallFailure> {
        self.calls += 1;
        Ok(ModuleOutput {
            new_state: None,
            effects: Vec::new(),
            emits: Vec::new(),
            tick_lifecycle: None,
            output_bytes: 0,
        })
    }
}

fn install_tick_module(world: &mut World) {
    world.set_policy(PolicySet::allow_all());
    world
        .bind_node_identity(
            TEST_MODULE_ARTIFACT_SIGNER_NODE_ID,
            test_module_artifact_signer_public_key_hex().as_str(),
        )
        .expect("bind test module signer identity");
    let wasm_bytes = b"status-payload-tick-routing-observability";
    let wasm_hash = sha256_hex(wasm_bytes);
    world
        .register_module_artifact(wasm_hash.clone(), wasm_bytes)
        .expect("register module artifact");
    let module_manifest = ModuleManifest {
        module_id: "m.status.tick".to_string(),
        name: "Status Tick".to_string(),
        version: "0.1.0".to_string(),
        kind: ModuleKind::Reducer,
        role: ModuleRole::Domain,
        wasm_hash: wasm_hash.clone(),
        interface_version: "wasm-1".to_string(),
        abi_contract: ModuleAbiContract::default(),
        exports: vec!["reduce".to_string()],
        subscriptions: vec![ModuleSubscription {
            event_kinds: Vec::new(),
            action_kinds: Vec::new(),
            stage: Some(ModuleSubscriptionStage::Tick),
            filters: None,
        }],
        required_caps: Vec::new(),
        artifact_identity: Some(signed_test_artifact_identity(wasm_hash.as_str())),
        limits: ModuleLimits::unbounded(),
    };
    let changes = ModuleChangeSet {
        register: vec![module_manifest.clone()],
        activate: vec![ModuleActivation {
            module_id: module_manifest.module_id.clone(),
            version: module_manifest.version.clone(),
        }],
        ..ModuleChangeSet::default()
    };
    let proposal_id = world
        .propose_manifest_update(
            Manifest {
                version: 2,
                content: serde_json::json!({ "module_changes": changes }),
            },
            "alice",
        )
        .expect("propose manifest");
    world.shadow_proposal(proposal_id).expect("shadow proposal");
    world
        .approve_proposal(proposal_id, "bob", ProposalDecision::Approve)
        .expect("approve proposal");
    world.apply_proposal(proposal_id).expect("apply proposal");
}

fn signed_test_artifact_identity(wasm_hash: &str) -> ModuleArtifactIdentity {
    let source_hash = sha256_hex(format!("test-src:{wasm_hash}").as_bytes());
    let build_manifest_hash = sha256_hex(b"test-build-manifest-v1");
    let payload = ModuleArtifactIdentity::signing_payload_v1(
        wasm_hash,
        source_hash.as_str(),
        build_manifest_hash.as_str(),
        TEST_MODULE_ARTIFACT_SIGNER_NODE_ID,
    );
    let signing_key = test_module_artifact_signing_key();
    let signature = signing_key.sign(payload.as_slice());
    ModuleArtifactIdentity {
        source_hash,
        build_manifest_hash,
        signer_node_id: TEST_MODULE_ARTIFACT_SIGNER_NODE_ID.to_string(),
        signature_scheme: ModuleArtifactIdentity::SIGNATURE_SCHEME_ED25519.to_string(),
        artifact_signature: format!(
            "{}{}",
            ModuleArtifactIdentity::SIGNATURE_PREFIX_ED25519_V1,
            hex::encode(signature.to_bytes())
        ),
    }
}

fn test_module_artifact_signing_key() -> SigningKey {
    let seed = sha256_hex(b"oasis7-test-module-artifact-signer-v1");
    let seed_bytes = hex::decode(seed).expect("decode test module signing seed");
    let private_key_bytes: [u8; 32] = seed_bytes
        .as_slice()
        .try_into()
        .expect("test module signing seed is 32 bytes");
    SigningKey::from_bytes(&private_key_bytes)
}

fn test_module_artifact_signer_public_key_hex() -> String {
    hex::encode(
        test_module_artifact_signing_key()
            .verifying_key()
            .to_bytes(),
    )
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = sha2::Sha256::digest(bytes);
    hex::encode(digest)
}

#[test]
fn build_chain_status_payload_surfaces_runtime_perf_snapshot() {
    let runtime_perf = RuntimePerfSnapshot {
        sample_window: 512,
        tick: runtime_perf_series(31.5, 0),
        decision: runtime_perf_series(24.2, 125_000),
        action_execution: runtime_perf_series(14.8, 0),
        callback: runtime_perf_series(3.1, 0),
        llm_api: runtime_perf_series(980.0, 0),
        health: RuntimePerfHealth::Warn,
        bottleneck: RuntimePerfBottleneck::Decision,
    };

    let payload = build_minimal_status_payload_with_runtime_perf(runtime_perf);

    let runtime_perf = payload.runtime_perf.expect("runtime perf snapshot");
    assert_eq!(runtime_perf.health, RuntimePerfHealth::Warn);
    assert_eq!(runtime_perf.bottleneck, RuntimePerfBottleneck::Decision);
    assert_eq!(runtime_perf.decision.p95_ms, 24.2);
    assert_eq!(runtime_perf.decision.over_budget_ratio_ppm, 125_000);
}

#[test]
fn build_chain_status_payload_warns_observability_for_runtime_perf_degradation() {
    let runtime_perf = RuntimePerfSnapshot {
        sample_window: 512,
        tick: runtime_perf_series(31.5, 0),
        decision: runtime_perf_series(24.2, 125_000),
        action_execution: runtime_perf_series(14.8, 0),
        callback: runtime_perf_series(3.1, 0),
        llm_api: runtime_perf_series(980.0, 0),
        health: RuntimePerfHealth::Warn,
        bottleneck: RuntimePerfBottleneck::Decision,
    };

    let payload = build_minimal_status_payload_with_runtime_perf(runtime_perf);

    assert_eq!(payload.observability.status, "warn");
    assert!(payload.observability.ready);
    let alert = payload
        .observability
        .alerts
        .iter()
        .find(|alert| alert.code == "runtime_perf_degraded")
        .expect("runtime perf observability alert");
    assert_eq!(alert.severity, "warn");
    assert!(alert.summary.contains("health=warn"));
    assert!(alert.summary.contains("bottleneck=decision"));
}

#[test]
fn build_chain_status_payload_marks_runtime_perf_critical_not_ready() {
    let runtime_perf = RuntimePerfSnapshot {
        sample_window: 512,
        tick: runtime_perf_series(80.0, 250_000),
        decision: runtime_perf_series(24.2, 0),
        action_execution: runtime_perf_series(14.8, 0),
        callback: runtime_perf_series(3.1, 0),
        llm_api: runtime_perf_series(980.0, 0),
        health: RuntimePerfHealth::Critical,
        bottleneck: RuntimePerfBottleneck::Tick,
    };

    let payload = build_minimal_status_payload_with_runtime_perf(runtime_perf);

    assert_eq!(payload.observability.status, "critical");
    assert!(!payload.observability.ready);
    assert!(payload.observability.runtime_perf_available);
    assert_eq!(payload.observability.runtime_perf_health, "critical");
    assert_eq!(payload.observability.runtime_perf_bottleneck, "tick");
    assert!(payload.observability.runtime_perf_degraded);
    let alert = payload
        .observability
        .alerts
        .iter()
        .find(|alert| alert.code == "runtime_perf_degraded")
        .expect("runtime perf observability alert");
    assert_eq!(alert.severity, "critical");
    assert!(alert.summary.contains("health=critical"));
    assert!(alert.summary.contains("bottleneck=tick"));
}

#[test]
fn build_chain_status_payload_marks_runtime_perf_unavailable_without_source() {
    let payload = build_minimal_status_payload(None);

    assert!(payload.runtime_perf.is_none());
    assert!(!payload.observability.runtime_perf_available);
    assert_eq!(payload.observability.runtime_perf_health, "unavailable");
    assert_eq!(payload.observability.runtime_perf_bottleneck, "none");
    assert!(!payload.observability.runtime_perf_degraded);
    assert!(
        payload
            .observability
            .alerts
            .iter()
            .all(|alert| alert.code != "runtime_perf_degraded")
    );
}

#[test]
fn runtime_perf_snapshot_from_execution_bridge_timing_requires_commit_samples() {
    let timing = super::execution_bridge::ExecutionBridgeCommitTimingSnapshot::default();

    let runtime_perf =
        super::status_payload::build_runtime_perf_snapshot_from_execution_bridge_timing(&timing);

    assert!(runtime_perf.is_none());
}

#[test]
fn runtime_perf_snapshot_from_execution_bridge_timing_warns_for_slow_commits() {
    let timing = super::execution_bridge::ExecutionBridgeCommitTimingSnapshot {
        window_capacity: 128,
        recent_commit_count: 4,
        p50_total_ms: Some(780),
        p95_total_ms: Some(1_250),
        max_total_ms: Some(1_250),
        slow_count: 1,
        last_slow_stage: Some("runtime_step".to_string()),
        stages: BTreeMap::new(),
    };

    let runtime_perf =
        super::status_payload::build_runtime_perf_snapshot_from_execution_bridge_timing(&timing)
            .expect("runtime perf snapshot");

    assert_eq!(runtime_perf.health, RuntimePerfHealth::Warn);
    assert_eq!(
        runtime_perf.bottleneck,
        RuntimePerfBottleneck::ActionExecution
    );
    assert_eq!(runtime_perf.action_execution.samples_total, 4);
    assert_eq!(runtime_perf.action_execution.budget_ms, 1_000.0);
    assert_eq!(runtime_perf.action_execution.p50_ms, 780.0);
    assert_eq!(runtime_perf.action_execution.p95_ms, 1_250.0);
    assert_eq!(runtime_perf.action_execution.over_budget_total, 0);
    assert_eq!(runtime_perf.action_execution.over_budget_ratio_ppm, 0);
}

#[test]
fn runtime_perf_snapshot_from_execution_bridge_timing_marks_very_slow_commits_critical() {
    let timing = super::execution_bridge::ExecutionBridgeCommitTimingSnapshot {
        window_capacity: 128,
        recent_commit_count: 4,
        p50_total_ms: Some(1_500),
        p95_total_ms: Some(2_500),
        max_total_ms: Some(2_500),
        slow_count: 4,
        last_slow_stage: Some("runtime_step".to_string()),
        stages: BTreeMap::new(),
    };

    let runtime_perf =
        super::status_payload::build_runtime_perf_snapshot_from_execution_bridge_timing(&timing)
            .expect("runtime perf snapshot");

    assert_eq!(runtime_perf.health, RuntimePerfHealth::Critical);
    assert_eq!(
        runtime_perf.bottleneck,
        RuntimePerfBottleneck::ActionExecution
    );
    assert_eq!(runtime_perf.action_execution.p95_ms, 2_500.0);
}

#[test]
fn build_chain_status_payload_surfaces_persisted_module_tick_routing_metrics() {
    let dir = temp_dir("module-routing-status");
    let mut world = World::new();
    install_tick_module(&mut world);
    let mut sandbox = CountingSandbox::default();
    world
        .route_tick_to_modules(&mut sandbox)
        .expect("route tick module");
    assert_eq!(sandbox.calls, 1);
    world.save_to_dir(&dir).expect("save execution world");
    let snapshot_path = dir.join("snapshot.json");
    let mut snapshot_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(snapshot_path.as_path()).expect("read snapshot"))
            .expect("parse snapshot");
    snapshot_json["module_tick_routing_metrics"] = serde_json::json!({
        "last_due_count": 1,
        "last_invoked_count": 1,
        "missing_invocation_count": 0,
        "last_missing_invocation_count": 0,
        "oldest_overdue_ticks": 0,
        "routing_count": 1
    });
    fs::write(
        snapshot_path.as_path(),
        serde_json::to_vec_pretty(&snapshot_json).expect("encode snapshot"),
    )
    .expect("write snapshot with module routing metrics");

    let payload = build_minimal_status_payload_with_world_dir(dir.as_path(), None);
    assert!(payload.module_tick_routing.available);
    assert_eq!(payload.module_tick_routing.source, "execution_world");
    assert!(payload.module_tick_routing.load_error.is_none());
    let metrics = payload
        .module_tick_routing
        .metrics
        .as_ref()
        .expect("module tick routing metrics");
    assert_eq!(metrics["routing_count"], 1);
    assert_eq!(metrics["last_due_count"], 1);
    assert_eq!(metrics["last_invoked_count"], 1);
    assert!(
        metrics.get("duration_buckets").is_none(),
        "persisted module routing metrics must stay deterministic"
    );
    assert!(
        metrics.get("last_route_duration_ms").is_none(),
        "wall-clock route duration must not enter canonical snapshots"
    );

    let _ = fs::remove_dir_all(&dir);
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
        None,
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
        None,
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
