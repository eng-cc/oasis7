use super::*;
use oasis7_proto::distributed::WorldHeadAnnounce;
use oasis7_proto::distributed_checkpoint_lineage::CheckpointLineageHeadV1;
use oasis7_proto::distributed_dht::DistributedDht;

struct GapWaitingExecutionHook;

impl NodeExecutionHook for GapWaitingExecutionHook {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        Err(format!(
            "{} at height {}",
            EXECUTION_MISSING_PREDECESSOR_RECORD_SIGNATURE, context.height
        ))
    }
}

struct CheckpointInstallingExecutionHook {
    installed: Vec<u64>,
}

impl NodeExecutionHook for CheckpointInstallingExecutionHook {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        Err(format!(
            "{} at height {}",
            EXECUTION_MISSING_PREDECESSOR_RECORD_SIGNATURE, context.height
        ))
    }

    fn install_checkpoint_bundle(
        &mut self,
        context: NodeExecutionCheckpointInstallContext,
        bundle: NodeExecutionCheckpointBundle,
    ) -> Result<NodeExecutionCommitResult, String> {
        if bundle.height != context.height
            || bundle.execution_block_hash != context.execution_block_hash
            || bundle.execution_state_root != context.execution_state_root
        {
            return Err("checkpoint bundle mismatch".to_string());
        }
        self.installed.push(context.height);
        Ok(NodeExecutionCommitResult {
            execution_height: context.height,
            execution_block_hash: context.execution_block_hash,
            execution_state_root: context.execution_state_root,
        })
    }
}

struct CheckpointExportingExecutionHook {
    bundles: std::collections::BTreeMap<u64, NodeExecutionCheckpointBundle>,
}

impl NodeExecutionHook for CheckpointExportingExecutionHook {
    fn on_commit(
        &mut self,
        context: NodeExecutionCommitContext,
    ) -> Result<NodeExecutionCommitResult, String> {
        Ok(NodeExecutionCommitResult {
            execution_height: context.height,
            execution_block_hash: format!("exec-block-{}", context.height),
            execution_state_root: format!("exec-state-{}", context.height),
        })
    }

    fn export_checkpoint_bundle(
        &mut self,
        height: u64,
    ) -> Result<Option<NodeExecutionCheckpointBundle>, String> {
        Ok(self.bundles.get(&height).cloned())
    }
}

pub(crate) fn test_execution_checkpoint_bundle(
    height: u64,
    execution_block_hash: &str,
    execution_state_root: &str,
) -> NodeExecutionCheckpointBundle {
    let snapshot_bytes =
        format!("checkpoint-snapshot-{height}-{execution_state_root}").into_bytes();
    NodeExecutionCheckpointBundle {
        height,
        execution_block_hash: execution_block_hash.to_string(),
        execution_state_root: execution_state_root.to_string(),
        manifest_json: br#"{"test":"manifest"}"#.to_vec(),
        blobs: vec![NodeExecutionCheckpointBlob {
            content_hash: oasis7_distfs::blake3_hex(snapshot_bytes.as_slice()),
            bytes: snapshot_bytes,
        }],
    }
}

#[test]
fn fetch_commit_exports_checkpoints_only_for_head_or_checkpoint_boundaries() {
    assert!(!should_export_checkpoint_for_fetch_commit(65, 16_715));
    assert!(should_export_checkpoint_for_fetch_commit(16_715, 16_715));
    assert!(should_export_checkpoint_for_fetch_commit(16_704, 16_715));
    assert!(should_export_checkpoint_for_fetch_commit(16_672, 16_715));
}

fn committed_decision(height: u64, approved_stake: u64, required_stake: u64) -> PosDecision {
    PosDecision {
        height,
        slot: height,
        epoch: 0,
            proposer_id: "node-a".to_string(),
        status: PosConsensusStatus::Committed,
        block_hash: format!("block-{height}"),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake,
        rejected_stake: 0,
        required_stake,
        total_stake: 100,
    }
}

#[test]
fn observer_gap_sync_installs_exported_checkpoint_for_legacy_commit_payload() {
    let _nonce_lock =
        super::storage_replication_first_ready_checkpoint_tests::lock_checkpoint_probe_nonce();
    let world_id = "world-gap-sync-exported-legacy-checkpoint";
    let dir_a = temp_dir("gap-sync-exported-legacy-checkpoint-a");
    let dir_b = temp_dir("gap-sync-exported-legacy-checkpoint-b");
    let dir_c = temp_dir("gap-sync-exported-legacy-checkpoint-c");
    let (_, public_key_a) = deterministic_keypair_hex(246);
    let (_, public_key_b) = deterministic_keypair_hex(247);
    let (_, public_key_c) = deterministic_keypair_hex(248);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 246)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 246)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b.clone()])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 247)
        .with_remote_writer_allowlist(vec![public_key_c.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_c.clone()])
        .expect("fetch allowlist b");
    let replication_config_c = signed_replication_config(dir_c.clone(), 248)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist c")
        .with_fetch_requester_allowlist(vec![public_key_b.clone()])
        .expect("fetch allowlist c");
    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_replication(replication_config_a.clone());
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(replication_config_b.clone());

    let high_height = REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL + 32;
    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    let mut replication_c =
        ReplicationRuntime::new(&replication_config_c, "node-c").expect("runtime c");
    for height in 1..=high_height {
        let decision = committed_decision(height, 100, 67);
        let message = replication_a
            .build_local_commit_message(
                "node-a",
                world_id,
                6_500 + i64::try_from(height).expect("height fits i64"),
                &decision,
                Some(format!("exec-block-{height}").as_str()),
                Some(format!("exec-state-{height}").as_str()),
            )
            .expect("build legacy local message")
            .expect("message");
        replication_c
            .apply_remote_message("node-c", world_id, &message)
            .expect("storage provider applies sequencer commit");
    }

    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let mut bundles = std::collections::BTreeMap::new();
    bundles.insert(
        high_height,
        test_execution_checkpoint_bundle(
            high_height,
            format!("exec-block-{high_height}").as_str(),
            format!("exec-state-{high_height}").as_str(),
        ),
    );
    let export_hook: Arc<Mutex<Box<dyn NodeExecutionHook>>> =
        Arc::new(Mutex::new(Box::new(CheckpointExportingExecutionHook {
            bundles,
        })));
    register_replication_fetch_handlers_with_checkpoint_export(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_c,
        "node-c",
        world_id,
        &config_a.network_policy,
        Some(export_hook),
    )
    .expect("register fetch handlers");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, false, &config_b.network_policy)
            .expect("endpoint b");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("engine b");
    engine_b.network_committed_height = high_height;
    let mut install_hook = CheckpointInstallingExecutionHook {
        installed: Vec::new(),
    };

    let result = engine_b.sync_missing_replication_commits(
        &endpoint_b,
        "node-b",
        world_id,
        Some(&mut replication_b),
        Some(&mut install_hook),
    );
    assert!(
        result.is_err(),
        "provider-exported legacy payload without source lineage must fail closed: {result:?}"
    );
    assert!(install_hook.installed.is_empty());
    assert_eq!(engine_b.committed_height, 0);
    assert_eq!(engine_b.replication_persisted_height, 0);
    assert_eq!(engine_b.last_execution_height, 0);
    assert!(replication_b
        .load_commit_message_by_height(world_id, 1)
        .expect("load low height")
        .is_none());

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
    let _ = fs::remove_dir_all(&dir_c);
}

#[test]
fn observer_gap_sync_discovers_high_checkpoint_from_world_head() {
    let _nonce_lock =
        super::storage_replication_first_ready_checkpoint_tests::lock_checkpoint_probe_nonce();
    let world_id = "world-gap-sync-dht-head";
    let dir_a = temp_dir("gap-sync-dht-head-a");
    let dir_b = temp_dir("gap-sync-dht-head-b");
    let (_, public_key_a) = deterministic_keypair_hex(214);
    let (_, public_key_b) = deterministic_keypair_hex(215);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 214)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 214)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 215)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a.clone()])
        .expect("fetch allowlist b");
    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_replication(replication_config_a.clone());
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(replication_config_b.clone());

    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    for height in 1..=3 {
        let decision = committed_decision(height, 100, 67);
        replication_a
            .build_local_commit_message(
                "node-a",
                world_id,
                2_500 + i64::try_from(height).expect("height fits i64"),
                &decision,
                Some(format!("exec-block-{height}").as_str()),
                Some(format!("exec-state-{height}").as_str()),
            )
            .expect("build local message")
            .expect("message");
    }
    std::fs::remove_file(dir_a.join("replication_commit_messages/00000000000000000001.json"))
        .expect("remove low commit 1");
    std::fs::remove_file(dir_a.join("replication_commit_messages/00000000000000000002.json"))
        .expect("remove low commit 2");

    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let dht = Arc::new(TestReplicaMaintenanceDht::new("node-a-provider", "node-b-provider"));
    dht.put_world_head(
        world_id,
        &WorldHeadAnnounce {
            world_id: world_id.to_string(),
            height: 3,
            block_hash: "block-3".to_string(),
            state_root: "exec-state-3".to_string(),
            timestamp_ms: 3_000,
            signature: "test-signature".to_string(),
        },
    )
    .expect("put world head");
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register fetch handlers");

    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network)).with_dht(dht);
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, false, &config_b.network_policy)
            .expect("endpoint b");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("engine b");
    engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            None,
        )
        .expect("dht-head high checkpoint sync");

    assert_eq!(engine_b.network_committed_height, 3);
    assert_eq!(engine_b.committed_height, 3);
    assert_eq!(engine_b.replication_persisted_height, 3);
    assert!(replication_b
        .load_commit_message_by_height(world_id, 1)
        .expect("load low height")
        .is_none());
    assert!(replication_b
        .load_commit_message_by_height(world_id, 3)
        .expect("load high checkpoint")
        .is_some());
    assert_eq!(engine_b.last_replication_gap_sync_blocked_height, None);

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn observer_gap_sync_discovers_high_checkpoint_from_peer_world_head_request() {
    let _nonce_lock =
        super::storage_replication_first_ready_checkpoint_tests::lock_checkpoint_probe_nonce();
    let world_id = "world-gap-sync-peer-head";
    let dir_a = temp_dir("gap-sync-peer-head-a");
    let dir_b = temp_dir("gap-sync-peer-head-b");
    let (_, public_key_a) = deterministic_keypair_hex(216);
    let (_, public_key_b) = deterministic_keypair_hex(217);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 216)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 216)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 217)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a.clone()])
        .expect("fetch allowlist b");
    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_replication(replication_config_a.clone());
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(replication_config_b.clone());

    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    for height in 1..=3 {
        let decision = committed_decision(height, 100, 67);
        let execution_block_hash = format!("exec-block-{height}");
        let execution_state_root = format!("exec-state-{height}");
        let checkpoint = (height == 3).then(|| {
            test_execution_checkpoint_bundle(
                height,
                execution_block_hash.as_str(),
                execution_state_root.as_str(),
            )
        });
        replication_a
            .build_local_commit_message_with_checkpoint(
                "node-a",
                world_id,
                2_700 + i64::try_from(height).expect("height fits i64"),
                &decision,
                Some(execution_block_hash.as_str()),
                Some(execution_state_root.as_str()),
                checkpoint,
            )
            .expect("build local message")
            .expect("message");
    }
    std::fs::remove_file(dir_a.join("replication_commit_messages/00000000000000000001.json"))
        .expect("remove low commit 1");
    std::fs::remove_file(dir_a.join("replication_commit_messages/00000000000000000002.json"))
        .expect("remove low commit 2");

    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register fetch handlers");

    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, false, &config_b.network_policy)
            .expect("endpoint b");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("engine b");
    engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            None,
        )
        .expect("peer-head high checkpoint sync");

    assert_eq!(engine_b.network_committed_height, 0);
    assert_eq!(engine_b.committed_height, 0);
    assert_eq!(engine_b.replication_persisted_height, 0);
    assert!(replication_b
        .load_commit_message_by_height(world_id, 1)
        .expect("load low height")
        .is_none());
    assert!(replication_b
        .load_commit_message_by_height(world_id, 3)
        .expect("load unsigned high checkpoint")
        .is_none());
    assert_eq!(engine_b.next_height, 1);
    assert_eq!(engine_b.last_replication_gap_sync_blocked_height, Some(1));

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn observer_gap_sync_installs_execution_checkpoint_bundle_when_low_history_is_missing() {
    let _nonce_lock =
        super::storage_replication_first_ready_checkpoint_tests::lock_checkpoint_probe_nonce();
    let world_id = "world-gap-sync-execution-checkpoint";
    let dir_a = temp_dir("gap-sync-execution-checkpoint-a");
    let dir_b = temp_dir("gap-sync-execution-checkpoint-b");
    let (_, public_key_a) = deterministic_keypair_hex(224);
    let (_, public_key_b) = deterministic_keypair_hex(225);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 224)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 224)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 225)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a.clone()])
        .expect("fetch allowlist b");
    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_replication(replication_config_a.clone());
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(replication_config_b.clone());

    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    let mut queued_low_message = None;
    for height in 1..=3 {
        let decision = committed_decision(height, 100, 67);
        let execution_block_hash = format!("exec-block-{height}");
        let execution_state_root = format!("exec-state-{height}");
        let checkpoint = (height == 3).then(|| {
            test_execution_checkpoint_bundle(height, &execution_block_hash, &execution_state_root)
        });
        let message = replication_a
            .build_local_commit_message_with_checkpoint(
                "node-a",
                world_id,
                5_500 + i64::try_from(height).expect("height fits i64"),
                &decision,
                Some(execution_block_hash.as_str()),
                Some(execution_state_root.as_str()),
                checkpoint,
            )
            .expect("build local message")
            .expect("message");
        if height == 1 {
            queued_low_message = Some(message);
        }
    }
    let engine_a = PosNodeEngine::new(&config_a).expect("lineage authority engine");
    super::storage_replication_live_retained_boundary_tests::attach_production_lineage_envelope(
        &mut replication_a,
        world_id,
        3,
        CheckpointLineageHeadV1 {
            height: 3,
            block_hash: "block-3".to_string(),
            state_root: "exec-state-3".to_string(),
            execution_block_hash: "exec-block-3".to_string(),
            execution_state_root: "exec-state-3".to_string(),
        },
        &[&engine_a],
    );
    std::fs::remove_file(dir_a.join("replication_commit_messages/00000000000000000001.json"))
        .expect("remove low commit 1");
    std::fs::remove_file(dir_a.join("replication_commit_messages/00000000000000000002.json"))
        .expect("remove low commit 2");

    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register fetch handlers");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let mut endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, false, &config_b.network_policy)
            .expect("endpoint b");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("engine b");
    engine_b.network_committed_height = 3;
    let mut execution_hook = CheckpointInstallingExecutionHook {
        installed: Vec::new(),
    };

    endpoint_b
        .publish_replication(
            queued_low_message
                .as_ref()
                .expect("queue low-height replication before observer tick"),
        )
        .expect("publish low-height replication");
    engine_b
        .tick(
            "node-b",
            world_id,
            6_000,
            None,
            Some(&mut replication_b),
            Some(&mut endpoint_b),
            None,
            Vec::new(),
            Some(&mut execution_hook),
        )
        .expect("high checkpoint must install before queued low-height tail replay");

    assert_eq!(execution_hook.installed, vec![3]);
    assert_eq!(engine_b.committed_height, 3);
    assert_eq!(engine_b.replication_persisted_height, 3);
    assert_eq!(engine_b.last_execution_height, 3);
    assert_eq!(
        engine_b.execution_binding_for_height(3),
        Some(("exec-block-3", "exec-state-3"))
    );
    assert_eq!(engine_b.last_replication_gap_sync_blocked_height, None);

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn observer_gap_sync_does_not_persist_sparse_execution_checkpoint_without_hook() {
    let _nonce_lock =
        super::storage_replication_first_ready_checkpoint_tests::lock_checkpoint_probe_nonce();
    let world_id = "world-gap-sync-checkpoint-no-hook";
    let dir_a = temp_dir("gap-sync-checkpoint-no-hook-a");
    let dir_b = temp_dir("gap-sync-checkpoint-no-hook-b");
    let (_, public_key_a) = deterministic_keypair_hex(244);
    let (_, public_key_b) = deterministic_keypair_hex(245);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 244)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 244)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 245)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a.clone()])
        .expect("fetch allowlist b");
    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_replication(replication_config_a.clone());
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(replication_config_b.clone());

    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    for height in 1..=3 {
        let decision = committed_decision(height, 100, 67);
        let execution_block_hash = format!("exec-block-{height}");
        let execution_state_root = format!("exec-state-{height}");
        let checkpoint = (height == 3).then(|| {
            test_execution_checkpoint_bundle(height, &execution_block_hash, &execution_state_root)
        });
        replication_a
            .build_local_commit_message_with_checkpoint(
                "node-a",
                world_id,
                5_700 + i64::try_from(height).expect("height fits i64"),
                &decision,
                Some(execution_block_hash.as_str()),
                Some(execution_state_root.as_str()),
                checkpoint,
            )
            .expect("build local message")
            .expect("message");
    }
    std::fs::remove_file(dir_a.join("replication_commit_messages/00000000000000000001.json"))
        .expect("remove low commit 1");
    std::fs::remove_file(dir_a.join("replication_commit_messages/00000000000000000002.json"))
        .expect("remove low commit 2");

    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register fetch handlers");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, false, &config_b.network_policy)
            .expect("endpoint b");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("engine b");
    engine_b.network_committed_height = 3;

    engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            None,
        )
        .expect("no hook sparse checkpoint sync should not error");

    assert_eq!(engine_b.committed_height, 0);
    assert_eq!(engine_b.replication_persisted_height, 0);
    assert_eq!(engine_b.last_execution_height, 0);
    assert!(replication_b
        .load_commit_message_by_height(world_id, 3)
        .expect("load high checkpoint")
        .is_none());

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn observer_gap_sync_probes_checkpoint_boundary_below_non_checkpoint_head() {
    let _nonce_lock =
        super::storage_replication_first_ready_checkpoint_tests::lock_checkpoint_probe_nonce();
    let world_id = "world-gap-sync-boundary-below-head";
    let dir_a = temp_dir("gap-sync-boundary-below-head-a");
    let dir_b = temp_dir("gap-sync-boundary-below-head-b");
    let (_, public_key_a) = deterministic_keypair_hex(226);
    let (_, public_key_b) = deterministic_keypair_hex(227);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 226)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 226)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 227)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a])
        .expect("fetch allowlist b");
    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_replication(replication_config_a.clone());
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(replication_config_b.clone());

    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    for height in 1..=70 {
        let decision = committed_decision(height, 100, 67);
        let execution_block_hash = format!("exec-block-{height}");
        let execution_state_root = format!("exec-state-{height}");
        let checkpoint = (height == 64).then(|| {
            test_execution_checkpoint_bundle(height, &execution_block_hash, &execution_state_root)
        });
        replication_a
            .build_local_commit_message_with_checkpoint(
                "node-a",
                world_id,
                6_000 + i64::try_from(height).expect("height fits i64"),
                &decision,
                Some(execution_block_hash.as_str()),
                Some(execution_state_root.as_str()),
                checkpoint,
            )
            .expect("build local message")
            .expect("message");
    }
    for height in 1..64 {
        let _ = std::fs::remove_file(
            dir_a
                .join("replication_commit_messages")
                .join(format!("{height:020}.json")),
        );
    }

    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register fetch handlers");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, false, &config_b.network_policy)
            .expect("endpoint b");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("engine b");
    engine_b.network_committed_height = 70;
    let mut execution_hook = CheckpointInstallingExecutionHook {
        installed: Vec::new(),
    };

    engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            Some(&mut execution_hook),
        )
        .expect("boundary checkpoint gap sync");

    assert_eq!(execution_hook.installed, vec![64]);
    assert_eq!(engine_b.committed_height, 64);
    assert_eq!(engine_b.replication_persisted_height, 64);
    assert_eq!(engine_b.last_execution_height, 64);
    assert_eq!(
        engine_b.execution_binding_for_height(64),
        Some(("exec-block-64", "exec-state-64"))
    );

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn observer_gap_sync_rejects_mismatched_world_head_checkpoint() {
    let _nonce_lock =
        super::storage_replication_first_ready_checkpoint_tests::lock_checkpoint_probe_nonce();
    let world_id = "world-gap-sync-dht-head-mismatch";
    let dir_a = temp_dir("gap-sync-dht-head-mismatch-a");
    let dir_b = temp_dir("gap-sync-dht-head-mismatch-b");
    let (_, public_key_a) = deterministic_keypair_hex(218);
    let (_, public_key_b) = deterministic_keypair_hex(219);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 218)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 218)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 219)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a])
        .expect("fetch allowlist b");
    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_replication(replication_config_a.clone());
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(replication_config_b.clone());

    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    for height in 1..=3 {
        let decision = committed_decision(height, 100, 67);
        replication_a
            .build_local_commit_message(
                "node-a",
                world_id,
                4_500 + i64::try_from(height).expect("height fits i64"),
                &decision,
                Some(format!("exec-block-{height}").as_str()),
                Some(format!("exec-state-{height}").as_str()),
            )
            .expect("build local message")
            .expect("message");
    }
    std::fs::remove_file(dir_a.join("replication_commit_messages/00000000000000000001.json"))
        .expect("remove low commit 1");
    std::fs::remove_file(dir_a.join("replication_commit_messages/00000000000000000002.json"))
        .expect("remove low commit 2");

    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let dht = Arc::new(TestReplicaMaintenanceDht::new("node-a-provider", "node-b-provider"));
    dht.put_world_head(
        world_id,
        &WorldHeadAnnounce {
            world_id: world_id.to_string(),
            height: 3,
            block_hash: "forged-block-3".to_string(),
            state_root: "forged-state-3".to_string(),
            timestamp_ms: 4_800,
            signature: "test-signature".to_string(),
        },
    )
    .expect("put forged world head");
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register fetch handlers");

    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network)).with_dht(dht);
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, false, &config_b.network_policy)
            .expect("endpoint b");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("engine b");

    engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            None,
        )
        .expect("mismatched dht-head should not fail the poll");

    assert_eq!(engine_b.network_committed_height, 0);
    assert_eq!(engine_b.committed_height, 0);
    assert_eq!(engine_b.replication_persisted_height, 0);
    assert!(replication_b
        .load_commit_message_by_height(world_id, 3)
        .expect("load rejected high checkpoint")
        .is_none());
    assert_eq!(engine_b.last_replication_gap_sync_blocked_height, Some(1));

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn high_checkpoint_gap_sync_does_not_skip_execution_history() {
    let _nonce_lock =
        super::storage_replication_first_ready_checkpoint_tests::lock_checkpoint_probe_nonce();
    let world_id = "world-gap-sync-high-checkpoint-execution-history";
    let dir_a = temp_dir("gap-sync-high-checkpoint-execution-history-a");
    let dir_b = temp_dir("gap-sync-high-checkpoint-execution-history-b");
    let (_, public_key_a) = deterministic_keypair_hex(216);
    let (_, public_key_b) = deterministic_keypair_hex(217);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 216)],
    );
    let replication_config_a = signed_replication_config(dir_a.clone(), 216)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 217)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a])
        .expect("fetch allowlist b");
    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_replication(replication_config_a.clone());
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(replication_config_b.clone());

    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    for height in 1..=3 {
        let decision = committed_decision(height, 100, 67);
        replication_a
            .build_local_commit_message(
                "node-a",
                world_id,
                3_500 + i64::try_from(height).expect("height fits i64"),
                &decision,
                Some(format!("exec-block-{height}").as_str()),
                Some(format!("exec-state-{height}").as_str()),
            )
            .expect("build local message")
            .expect("message");
    }
    std::fs::remove_file(dir_a.join("replication_commit_messages/00000000000000000001.json"))
        .expect("remove low commit 1");
    std::fs::remove_file(dir_a.join("replication_commit_messages/00000000000000000002.json"))
        .expect("remove low commit 2");

    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register fetch handlers");
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, false, &config_b.network_policy)
            .expect("endpoint b");
    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("engine b");
    engine_b.network_committed_height = 3;
    let mut execution_hook = GapWaitingExecutionHook;

    engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            Some(&mut execution_hook),
        )
        .expect("gap sync should report blocked low history without skipping execution");

    assert_eq!(engine_b.committed_height, 0);
    assert_eq!(engine_b.replication_persisted_height, 0);
    assert_eq!(engine_b.last_execution_height, 0);
    assert_eq!(engine_b.last_replication_gap_sync_blocked_height, Some(1));
    assert!(replication_b
        .load_commit_message_by_height(world_id, 3)
        .expect("load high checkpoint")
        .is_none());

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn observer_gap_sync_bootstraps_from_high_checkpoint_when_low_commits_are_unavailable() {
    let _nonce_lock =
        super::storage_replication_first_ready_checkpoint_tests::lock_checkpoint_probe_nonce();
    let world_id = "world-gap-sync-high-checkpoint";
    let dir_a = temp_dir("gap-sync-high-checkpoint-a");
    let dir_b = temp_dir("gap-sync-high-checkpoint-b");
    let (_, public_key_a) = deterministic_keypair_hex(164);
    let (_, public_key_b) = deterministic_keypair_hex(165);
    let validators = vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 60,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 40,
        },
    ];
    let pos_config =
        signed_pos_config_with_signer_seeds(validators, &[("node-a", 164), ("node-b", 165)]);
    let replication_config_a = signed_replication_config(dir_a.clone(), 164)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a")
        .with_fetch_requester_allowlist(vec![public_key_b.clone()])
        .expect("fetch allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 165)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b")
        .with_fetch_requester_allowlist(vec![public_key_a.clone()])
        .expect("fetch allowlist b");
    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_replication(replication_config_a.clone());
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(replication_config_b.clone());

    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    for height in 1..=3 {
        let decision = committed_decision(height, 60, 40);
        replication_a
            .build_local_commit_message(
                "node-a",
                world_id,
                2_000 + i64::try_from(height).expect("height fits i64"),
                &decision,
                Some(format!("execution-block-{height}").as_str()),
                Some(format!("execution-state-{height}").as_str()),
            )
            .expect("build local message")
            .expect("message");
    }
    let network_impl = Arc::new(TestInMemoryNetwork::default());
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();
    let handle_b = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    let endpoint_b =
        ReplicationNetworkEndpoint::new(&handle_b, world_id, false, &config_b.network_policy)
            .expect("endpoint b");
    std::fs::remove_file(dir_a.join("replication_commit_messages/00000000000000000001.json"))
        .expect("remove low commit 1");
    std::fs::remove_file(dir_a.join("replication_commit_messages/00000000000000000002.json"))
        .expect("remove low commit 2");
    register_replication_fetch_handlers(
        &NodeReplicationNetworkHandle::new(Arc::clone(&network)),
        &replication_config_a,
        world_id,
        &config_a.network_policy,
    )
    .expect("register fetch handlers");

    let mut replication_b =
        ReplicationRuntime::new(config_b.replication.as_ref().expect("repl b"), "node-b")
            .expect("runtime b");
    let mut engine_b = PosNodeEngine::new(&config_b).expect("engine b");
    engine_b.network_committed_height = 3;

    engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            None,
        )
        .expect("high checkpoint sync");

    assert!(
        replication_b
            .load_commit_message_by_height(world_id, 1)
            .expect("load missing low commit")
            .is_none(),
        "test fixture should not silently fall back to manual low-height seeding"
    );
    assert!(replication_b
        .load_commit_message_by_height(world_id, 3)
        .expect("load unsigned checkpoint candidate")
        .is_none());
    assert_eq!(engine_b.committed_height, 0);
    assert_eq!(engine_b.replication_persisted_height, 0);
    assert_eq!(engine_b.next_height, 1);
    assert_eq!(engine_b.last_execution_height, 0);
    assert!(engine_b.last_execution_block_hash.is_none());
    assert!(engine_b.execution_binding_for_height(3).is_none());
    assert_eq!(engine_b.last_replication_gap_sync_blocked_height, Some(1));

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}
