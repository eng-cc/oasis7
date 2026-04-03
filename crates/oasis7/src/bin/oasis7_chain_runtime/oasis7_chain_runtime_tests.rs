use super::{
    build_chain_balances_payload_from_world, build_chain_status_payload, build_default_peer_record,
    build_default_replication_network_config, build_node_replication_config,
    derive_node_consensus_signer_keypair, node_keypair_config, parse_options, parse_validator_spec,
    release_security_policy_for_storage_profile, CliOptions, DEFAULT_NODE_ID,
    DEFAULT_REPLICATION_NETWORK_LISTEN, DEFAULT_STATUS_BIND,
};
use ed25519_dalek::SigningKey;
use oasis7::runtime::{ReleaseSecurityPolicy, World as RuntimeWorld};
use oasis7_node::{NodeConfig, NodeConsensusSnapshot, NodeNetworkPolicy, NodeRole, NodeSnapshot};
use oasis7_proto::distributed_dht::{
    PeerDeploymentMode, PeerDiscoverySource, PeerNodeRole, PeerReachabilityClass,
};
use oasis7_proto::storage_profile::{StorageProfile, StorageProfileConfig};
use std::collections::BTreeMap;
use std::path::Path;

#[test]
fn parse_options_defaults() {
    let options = parse_options(std::iter::empty()).expect("parse should succeed");
    assert_eq!(options.node_id, DEFAULT_NODE_ID);
    assert_eq!(options.status_bind, DEFAULT_STATUS_BIND);
    assert_eq!(options.storage_profile, StorageProfile::DevLocal);
    assert!(!options.node_auto_attest_all_validators);
    assert!(options.node_validators.is_empty());
    assert!(options.reward_runtime_enabled);
    assert!(options.reward_runtime_epoch_duration_secs.is_none());
    assert_eq!(options.pos_slot_duration_ms, 12_000);
    assert_eq!(options.pos_ticks_per_slot, 10);
    assert_eq!(options.pos_proposal_tick_phase, 9);
    assert!(!options.pos_adaptive_tick_scheduler_enabled);
    assert!(options.pos_slot_clock_genesis_unix_ms.is_none());
    assert_eq!(options.pos_max_past_slot_lag, 256);
}

#[test]
fn parse_options_reads_custom_values() {
    let options = parse_options(
        [
            "--node-id",
            "node-a",
            "--world-id",
            "live-foo",
            "--storage-profile",
            "soak_forensics",
            "--status-bind",
            "127.0.0.1:6221",
            "--node-role",
            "storage",
            "--node-tick-ms",
            "350",
            "--pos-slot-duration-ms",
            "12000",
            "--pos-ticks-per-slot",
            "10",
            "--pos-proposal-tick-phase",
            "9",
            "--pos-adaptive-tick-scheduler",
            "--pos-slot-clock-genesis-unix-ms",
            "1700000000000",
            "--pos-max-past-slot-lag",
            "32",
            "--node-validator",
            "node-a:55",
            "--node-validator",
            "node-b:45",
            "--node-auto-attest-all",
            "--execution-world-dir",
            "custom/world",
            "--reward-runtime-epoch-duration-secs",
            "60",
            "--reward-points-per-credit",
            "100",
            "--reward-runtime-auto-redeem",
            "--reward-initial-reserve-power-units",
            "50000",
        ]
        .into_iter(),
    )
    .expect("parse should succeed");

    assert_eq!(options.node_id, "node-a");
    assert_eq!(options.world_id, "live-foo");
    assert_eq!(options.storage_profile, StorageProfile::SoakForensics);
    assert_eq!(options.status_bind, "127.0.0.1:6221");
    assert_eq!(options.node_role.as_str(), "storage");
    assert_eq!(options.p2p_deployment_mode, PeerDeploymentMode::Private);
    assert_eq!(options.p2p_node_role, PeerNodeRole::FullStorage);
    assert_eq!(options.node_tick_ms, 350);
    assert_eq!(options.pos_slot_duration_ms, 12_000);
    assert_eq!(options.pos_ticks_per_slot, 10);
    assert_eq!(options.pos_proposal_tick_phase, 9);
    assert!(options.pos_adaptive_tick_scheduler_enabled);
    assert_eq!(
        options.pos_slot_clock_genesis_unix_ms,
        Some(1_700_000_000_000)
    );
    assert_eq!(options.pos_max_past_slot_lag, 32);
    assert_eq!(options.node_validators.len(), 2);
    assert!(options.node_auto_attest_all_validators);
    assert_eq!(options.reward_runtime_epoch_duration_secs, Some(60));
    assert_eq!(options.reward_points_per_credit, 100);
    assert!(options.reward_runtime_auto_redeem);
    assert_eq!(options.reward_initial_reserve_power_units, 50_000);
    assert_eq!(
        options
            .execution_world_dir
            .as_ref()
            .map(|p| p.to_string_lossy().to_string()),
        Some("custom/world".to_string())
    );
}

#[test]
fn parse_options_rejects_invalid_status_bind() {
    let err = parse_options(["--status-bind", "127.0.0.1"].into_iter())
        .expect_err("should reject invalid bind");
    assert!(err.contains("<host:port>"));
}

#[test]
fn parse_options_rejects_peer_without_bind() {
    let err = parse_options(["--node-gossip-peer", "127.0.0.1:9001"].into_iter())
        .expect_err("should reject peer without bind");
    assert!(err.contains("requires --node-gossip-bind"));
}

#[test]
fn parse_options_rejects_proposal_tick_phase_out_of_range() {
    let err = parse_options(
        [
            "--pos-ticks-per-slot",
            "4",
            "--pos-proposal-tick-phase",
            "4",
        ]
        .into_iter(),
    )
    .expect_err("proposal tick phase out of range");
    assert!(err.contains("--pos-proposal-tick-phase"));
}

#[test]
fn parse_options_rejects_unknown_storage_profile() {
    let err = parse_options(["--storage-profile", "unknown"].into_iter())
        .expect_err("invalid storage profile should fail");
    assert!(err.contains("dev_local"));
    assert!(err.contains("soak_forensics"));
}

#[test]
fn parse_validator_spec_rejects_zero_stake() {
    let err = parse_validator_spec("node-a:0").expect_err("should reject");
    assert!(err.contains("positive integer"));
}

#[test]
fn balances_payload_reports_empty_world_without_error() {
    let world = RuntimeWorld::new();
    let payload = build_chain_balances_payload_from_world(
        "node-a",
        "live-a",
        std::path::Path::new("/tmp/empty"),
        &world,
    );
    assert!(payload.ok);
    assert!(payload.load_error.is_none());
    assert_eq!(payload.node_power_credit_balance, 0);
    assert_eq!(payload.reward_mint_record_count, 0);
    assert!(payload.recent_reward_mint_records.is_empty());
}

#[test]
fn parse_options_rejects_unknown_option() {
    let err = parse_options(["--unknown"].into_iter()).expect_err("should fail");
    assert!(err.contains("unknown option"));
}

#[test]
fn default_runtime_paths_depend_on_node_id() {
    let options = CliOptions {
        node_id: "node-z".to_string(),
        ..CliOptions::default()
    };
    let paths = super::resolve_runtime_paths(&options);
    assert!(paths
        .execution_world_dir
        .to_string_lossy()
        .contains("output/chain-runtime/node-z"));
}

#[test]
fn default_replication_network_config_uses_loopback_ephemeral_listen() {
    let signing_key = SigningKey::from_bytes(&[9_u8; 32]);
    let keypair = node_keypair_config::NodeKeypairConfig {
        private_key_hex: hex::encode(signing_key.to_bytes()),
        public_key_hex: hex::encode(signing_key.verifying_key().to_bytes()),
    };
    let config = build_default_replication_network_config(&CliOptions::default(), &keypair)
        .expect("default replication network config should build");
    assert_eq!(config.listen_addrs.len(), 1);
    assert_eq!(
        config.listen_addrs[0].to_string(),
        DEFAULT_REPLICATION_NETWORK_LISTEN
    );
    assert!(config.bootstrap_peers.is_empty());
    assert!(!config.allow_local_handler_fallback_when_no_peers);
    assert!(config.keypair.is_some());
    let peer_record = config.peer_record.expect("peer record");
    assert_eq!(peer_record.node_id, DEFAULT_NODE_ID);
    assert_eq!(peer_record.node_role, "validator_core");
    assert_eq!(peer_record.deployment_mode, PeerDeploymentMode::Private);
    assert_eq!(
        peer_record.reachability_class,
        PeerReachabilityClass::Private
    );
    assert_eq!(
        peer_record.discovery_sources,
        vec![
            PeerDiscoverySource::StaticBootstrap,
            PeerDiscoverySource::Dht
        ]
    );
}

#[test]
fn build_default_peer_record_tracks_runtime_identity_boundary() {
    let options = CliOptions {
        node_id: "node-p2p".to_string(),
        world_id: "world-p2p".to_string(),
        node_role: NodeRole::Storage,
        p2p_node_role: PeerNodeRole::FullStorage,
        ..CliOptions::default()
    };
    let record = build_default_peer_record(&options);
    assert!(record.peer_id.is_empty());
    assert_eq!(record.node_id, "node-p2p");
    assert_eq!(record.world_id, "world-p2p");
    assert_eq!(record.network_id, "world-p2p");
    assert_eq!(record.node_role, "full_storage");
    assert_eq!(record.deployment_mode, PeerDeploymentMode::Private);
    assert_eq!(record.reachability_class, PeerReachabilityClass::Private);
}

#[test]
fn parse_options_reads_explicit_p2p_policy_overrides() {
    let options = parse_options(
        [
            "--node-role",
            "observer",
            "--p2p-deployment-mode",
            "public",
            "--p2p-node-role",
            "relay",
        ]
        .into_iter(),
    )
    .expect("parse should succeed");
    assert_eq!(options.node_role, NodeRole::Observer);
    assert_eq!(options.p2p_deployment_mode, PeerDeploymentMode::Public);
    assert_eq!(options.p2p_node_role, PeerNodeRole::Relay);
}

#[test]
fn node_network_policy_rejects_incompatible_runtime_role_combo() {
    let err = NodeConfig::new("node-a", "world-a", NodeRole::Observer)
        .expect("config")
        .with_network_policy(NodeNetworkPolicy {
            deployment_mode: PeerDeploymentMode::ValidatorHidden,
            node_role_claim: PeerNodeRole::ValidatorCore,
        })
        .expect_err("observer cannot claim validator_core");
    assert!(
        matches!(err, oasis7_node::NodeError::InvalidConfig { reason } if reason.contains("cannot use network node_role_claim"))
    );
}

#[test]
fn build_node_replication_config_uses_storage_profile_budget() {
    let signing_key = SigningKey::generate(&mut rand_core::OsRng);
    let keypair = node_keypair_config::NodeKeypairConfig {
        private_key_hex: hex::encode(signing_key.to_bytes()),
        public_key_hex: hex::encode(signing_key.verifying_key().to_bytes()),
    };
    let storage_profile = StorageProfileConfig::for_profile(StorageProfile::ReleaseDefault);
    let config = build_node_replication_config("node-a", &keypair, &storage_profile)
        .expect("replication config should build");

    assert_eq!(
        config.max_hot_commit_messages(),
        storage_profile.replication_max_hot_commit_messages
    );
}

#[test]
fn derive_node_consensus_signer_keypair_is_deterministic_for_oasis7_namespace() {
    let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
    let keypair = node_keypair_config::NodeKeypairConfig {
        private_key_hex: hex::encode(signing_key.to_bytes()),
        public_key_hex: hex::encode(signing_key.verifying_key().to_bytes()),
    };

    let signer_a =
        derive_node_consensus_signer_keypair("node-a", &keypair).expect("derive signer a");
    let signer_a_repeat =
        derive_node_consensus_signer_keypair("node-a", &keypair).expect("derive signer a repeat");
    let signer_b =
        derive_node_consensus_signer_keypair("node-b", &keypair).expect("derive signer b");

    assert_eq!(signer_a.private_key_hex, signer_a_repeat.private_key_hex);
    assert_eq!(signer_a.public_key_hex, signer_a_repeat.public_key_hex);
    assert_ne!(signer_a.public_key_hex, signer_b.public_key_hex);
}

#[test]
fn build_chain_status_payload_includes_storage_metrics() {
    let snapshot = NodeSnapshot {
        node_id: "node-a".to_string(),
        player_id: "player-a".to_string(),
        world_id: "live-a".to_string(),
        role: NodeRole::Sequencer,
        running: true,
        tick_count: 42,
        last_tick_unix_ms: Some(1_700_000_000_000),
        consensus: NodeConsensusSnapshot::default(),
        last_error: None,
    };
    let reward_runtime = super::reward_runtime_worker::RewardRuntimeMetricsSnapshot {
        enabled: true,
        metrics_available: true,
        report_dir: "/tmp/reports".to_string(),
        report_count: 2,
        latest_epoch_index: 1,
        latest_report_observed_at_unix_ms: 1_700_000_000_000,
        latest_total_distributed_points: 10,
        latest_minted_record_count: 1,
        cumulative_minted_record_count: 1,
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
        bytes_by_dir: BTreeMap::from([("runtime_root".to_string(), 128)]),
        blob_counts: BTreeMap::from([("execution_store_blobs".to_string(), 2)]),
        ref_count: 5,
        pin_count: 3,
        retained_heights: vec![1, 2],
        checkpoint_count: 1,
        replay_summary: super::storage_metrics::StorageReplaySummary {
            retained_height_count: 2,
            earliest_retained_height: Some(1),
            latest_retained_height: Some(2),
            earliest_checkpoint_height: Some(2),
            latest_checkpoint_height: Some(2),
            mode: "checkpoint_plus_log".to_string(),
        },
        orphan_blob_count: 0,
        last_gc_at_ms: Some(1_700_000_000_000),
        last_gc_result: "failed".to_string(),
        last_gc_error: Some("gc failed".to_string()),
        degraded_reason: Some("storage degraded".to_string()),
    };

    let payload = build_chain_status_payload(
        snapshot,
        Path::new("/tmp/execution-world"),
        ReleaseSecurityPolicy::default(),
        reward_runtime,
        storage.clone(),
    );

    assert_eq!(payload.storage.storage_profile, "dev_local");
    assert_eq!(payload.storage.ref_count, 5);
    assert_eq!(payload.storage.pin_count, 3);
    assert_eq!(payload.storage.checkpoint_count, 1);
    assert_eq!(
        payload.storage.effective_budget.profile,
        StorageProfile::DevLocal
    );
    assert_eq!(payload.storage.replay_summary.mode, "checkpoint_plus_log");
    assert_eq!(
        payload.storage.replay_summary.latest_retained_height,
        Some(2)
    );
    assert_eq!(payload.storage.last_gc_result, "failed");
    assert_eq!(payload.storage.last_gc_error.as_deref(), Some("gc failed"));
    assert_eq!(
        payload.storage.degraded_reason.as_deref(),
        Some("storage degraded")
    );
    assert_eq!(
        payload.release_security_policy,
        ReleaseSecurityPolicy::default()
    );
}

#[test]
fn production_release_policy_status_payload_reports_effective_policy() {
    let snapshot = NodeSnapshot {
        node_id: "node-a".to_string(),
        player_id: "player-a".to_string(),
        world_id: "live-a".to_string(),
        role: NodeRole::Sequencer,
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
        release_security_policy.clone(),
        reward_runtime,
        storage,
    );

    assert_eq!(
        payload.release_security_policy,
        ReleaseSecurityPolicy::production_hardened()
    );
    assert!(payload.release_security_policy.is_production_hardened());
}
