use super::*;
use oasis7_proto::distributed::WorldHeadAnnounce;
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

#[test]
fn observer_gap_sync_discovers_high_checkpoint_from_world_head() {
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
        let decision = PosDecision {
            height,
            slot: height,
            epoch: 0,
            status: PosConsensusStatus::Committed,
            block_hash: format!("block-{height}"),
            action_root: empty_action_root(),
            committed_actions: Vec::new(),
            approved_stake: 100,
            rejected_stake: 0,
            required_stake: 67,
            total_stake: 100,
        };
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
fn high_checkpoint_gap_sync_does_not_skip_execution_history() {
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
        let decision = PosDecision {
            height,
            slot: height,
            epoch: 0,
            status: PosConsensusStatus::Committed,
            block_hash: format!("block-{height}"),
            action_root: empty_action_root(),
            committed_actions: Vec::new(),
            approved_stake: 100,
            rejected_stake: 0,
            required_stake: 67,
            total_stake: 100,
        };
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
        let execution_block_hash = format!("execution-block-{height}");
        let execution_state_root = format!("execution-state-{height}");
        let decision = PosDecision {
            height,
            slot: height,
            epoch: 0,
            status: PosConsensusStatus::Committed,
            block_hash: format!("block-{height}"),
            action_root: empty_action_root(),
            committed_actions: Vec::new(),
            approved_stake: 60,
            rejected_stake: 0,
            required_stake: 40,
            total_stake: 100,
        };
        replication_a
            .build_local_commit_message(
                "node-a",
                world_id,
                2_000 + i64::try_from(height).expect("height fits i64"),
                &decision,
                Some(execution_block_hash.as_str()),
                Some(execution_state_root.as_str()),
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
        .expect("load checkpoint commit")
        .is_some());
    assert_eq!(engine_b.committed_height, 3);
    assert_eq!(engine_b.replication_persisted_height, 3);
    assert_eq!(engine_b.next_height, 4);
    assert_eq!(engine_b.last_execution_height, 0);
    assert!(engine_b.last_execution_block_hash.is_none());
    assert!(engine_b.execution_binding_for_height(3).is_none());
    assert_eq!(engine_b.last_replication_gap_sync_blocked_height, None);

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}
