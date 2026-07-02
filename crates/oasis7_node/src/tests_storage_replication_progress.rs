use super::*;

#[derive(Clone)]
struct HeadlessTestNetwork {
    inner: Arc<TestInMemoryNetwork>,
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError> for HeadlessTestNetwork {
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        self.inner.publish(topic, payload)
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        self.inner.subscribe(topic)
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        if protocol == REPLICATION_GET_HEAD_PROTOCOL {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: protocol.to_string(),
            });
        }
        self.inner.request(protocol, payload)
    }

    fn request_with_providers(
        &self,
        protocol: &str,
        payload: &[u8],
        providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        if protocol == REPLICATION_GET_HEAD_PROTOCOL {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: protocol.to_string(),
            });
        }
        self.inner.request_with_providers(protocol, payload, providers)
    }

    fn register_handler(
        &self,
        protocol: &str,
        handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        self.inner.register_handler(protocol, handler)
    }
}

#[derive(Clone)]
struct SlowResponseTestNetwork {
    inner: Arc<TestInMemoryNetwork>,
    delay: Duration,
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError> for SlowResponseTestNetwork {
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        self.inner.publish(topic, payload)
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        self.inner.subscribe(topic)
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        thread::sleep(self.delay);
        self.inner.request(protocol, payload)
    }

    fn request_with_providers(
        &self,
        protocol: &str,
        payload: &[u8],
        providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        thread::sleep(self.delay);
        self.inner.request_with_providers(protocol, payload, providers)
    }

    fn register_handler(
        &self,
        protocol: &str,
        handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        self.inner.register_handler(protocol, handler)
    }
}

#[test]
fn gap_sync_limits_single_poll_work_to_publish_intermediate_progress() {
    let world_id = "world-gap-sync-single-poll-limit";
    let dir_a = temp_dir("gap-sync-single-poll-limit-a");
    let dir_b = temp_dir("gap-sync-single-poll-limit-b");
    let (_, public_key_a) = deterministic_keypair_hex(166);
    let (_, public_key_b) = deterministic_keypair_hex(167);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 166)],
    );
    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_replication(
            signed_replication_config(dir_a.clone(), 166)
                .with_remote_writer_allowlist(vec![public_key_b.clone()])
                .expect("allowlist a")
                .with_fetch_requester_allowlist(vec![public_key_b])
                .expect("fetch allowlist a"),
        );
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(
            signed_replication_config(dir_b.clone(), 167)
                .with_remote_writer_allowlist(vec![public_key_a.clone()])
                .expect("allowlist b")
                .with_fetch_requester_allowlist(vec![public_key_a.clone()])
                .expect("fetch allowlist b"),
        );

    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    for height in 1..=96_u64 {
        let decision = PosDecision {
            height,
            slot: height,
            epoch: 0,
            proposer_id: "node-a".to_string(),
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
                5_000 + i64::try_from(height).expect("height fits i64"),
                &decision,
                Some(format!("exec-block-{height}").as_str()),
                Some(format!("exec-state-{height}").as_str()),
            )
            .expect("build local message")
            .expect("message");
    }

    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(HeadlessTestNetwork {
        inner: Arc::new(TestInMemoryNetwork::default()),
    });
    let handle_a = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    register_replication_fetch_handlers(
        &handle_a,
        config_a.replication.as_ref().expect("repl a"),
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
    engine_b.network_committed_height = REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL;

    engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            None,
        )
        .expect("gap sync");

    assert_eq!(
        engine_b.committed_height,
        REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL
    );
    assert_eq!(
        engine_b.replication_persisted_height,
        REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL
    );
    assert_eq!(
        replication_b
            .latest_persisted_commit_height(world_id)
            .expect("persisted height after capped poll"),
        REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL
    );
    assert!(
        replication_b
            .load_commit_message_by_height(world_id, REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL + 1)
            .expect("load capped successor")
            .is_none()
    );

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn gap_sync_uses_high_checkpoint_when_network_lag_exceeds_poll_window() {
    let world_id = "world-gap-sync-high-checkpoint";
    let dir_a = temp_dir("gap-sync-high-checkpoint-a");
    let dir_b = temp_dir("gap-sync-high-checkpoint-b");
    let (_, public_key_a) = deterministic_keypair_hex(176);
    let (_, public_key_b) = deterministic_keypair_hex(177);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 176)],
    );
    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_replication(
            signed_replication_config(dir_a.clone(), 176)
                .with_remote_writer_allowlist(vec![public_key_b.clone()])
                .expect("allowlist a")
                .with_fetch_requester_allowlist(vec![public_key_b])
                .expect("fetch allowlist a"),
        );
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(
            signed_replication_config(dir_b.clone(), 177)
                .with_remote_writer_allowlist(vec![public_key_a.clone()])
                .expect("allowlist b")
                .with_fetch_requester_allowlist(vec![public_key_a.clone()])
                .expect("fetch allowlist b"),
        );

    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    let high_height = REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL + 32;
    for height in 1..=high_height {
        let decision = PosDecision {
            height,
            slot: height,
            epoch: 0,
            proposer_id: "node-a".to_string(),
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
                6_000 + i64::try_from(height).expect("height fits i64"),
                &decision,
                Some(format!("exec-block-{height}").as_str()),
                Some(format!("exec-state-{height}").as_str()),
            )
            .expect("build local message")
            .expect("message");
    }

    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let handle_a = NodeReplicationNetworkHandle::new(Arc::clone(&network));
    register_replication_fetch_handlers(
        &handle_a,
        config_a.replication.as_ref().expect("repl a"),
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
    engine_b.network_committed_height = high_height;

    engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            None,
        )
        .expect("gap sync");

    assert_eq!(engine_b.committed_height, high_height);
    assert_eq!(engine_b.replication_persisted_height, high_height);
    assert_eq!(engine_b.last_execution_height, 0);
    assert!(engine_b.last_execution_block_hash.is_none());
    assert!(engine_b.execution_binding_for_height(high_height).is_none());
    assert_eq!(
        replication_b
            .latest_persisted_commit_height(world_id)
            .expect("persisted height after high checkpoint"),
        high_height
    );
    assert!(replication_b
        .load_commit_message_by_height(world_id, 1)
        .expect("load low height")
        .is_none());
    assert!(replication_b
        .load_commit_message_by_height(world_id, high_height)
        .expect("load high checkpoint")
        .is_some());

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn runtime_snapshot_advances_during_slow_gap_sync_before_poll_finishes() {
    let world_id = "world-runtime-slow-gap-sync-progress";
    let dir_a = temp_dir("runtime-slow-gap-sync-a");
    let dir_b = temp_dir("runtime-slow-gap-sync-b");
    let (_, public_key_a) = deterministic_keypair_hex(171);
    let (_, public_key_b) = deterministic_keypair_hex(172);
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
        signed_pos_config_with_signer_seeds(validators, &[("node-a", 171), ("node-b", 172)]);
    let network_impl = Arc::new(TestInMemoryNetwork::default());
    let slow_network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(SlowResponseTestNetwork {
        inner: Arc::clone(&network_impl),
        delay: Duration::from_millis(30),
    });

    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_replication(
            signed_replication_config(dir_a.clone(), 171)
                .with_remote_writer_allowlist(vec![public_key_b.clone()])
                .expect("allowlist a")
                .with_fetch_requester_allowlist(vec![public_key_b])
                .expect("fetch allowlist a"),
        );
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(
            signed_replication_config(dir_b.clone(), 172)
                .with_remote_writer_allowlist(vec![public_key_a.clone()])
                .expect("allowlist b")
                .with_fetch_requester_allowlist(vec![public_key_a.clone()])
                .expect("fetch allowlist b"),
        );

    let target_height = 24_u64;
    let mut replication_a =
        ReplicationRuntime::new(config_a.replication.as_ref().expect("repl a"), "node-a")
            .expect("runtime a");
    for height in 1..=target_height {
        let decision = PosDecision {
            height,
            slot: height,
            epoch: 0,
            proposer_id: "node-a".to_string(),
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
                5_000 + i64::try_from(height).expect("height fits i64"),
                &decision,
                Some(format!("exec-block-{height}").as_str()),
                Some(format!("exec-state-{height}").as_str()),
            )
            .expect("build local message")
            .expect("message");
    }

    let handle_a = NodeReplicationNetworkHandle::new(Arc::clone(&slow_network));
    register_replication_fetch_handlers(
        &handle_a,
        config_a.replication.as_ref().expect("repl a"),
        world_id,
        &config_a.network_policy,
    )
    .expect("register fetch handlers");
    let high_message = replication_a
        .load_commit_message_by_height(world_id, target_height)
        .expect("load target commit")
        .expect("high commit payload");

    let topic = super::network_bridge::default_replication_topic(world_id);
    let high_payload = serde_json::to_vec(&high_message).expect("encode high message");
    slow_network
        .publish(topic.as_str(), high_payload.as_slice())
        .expect("publish high message");

    let mut runtime_b = NodeRuntime::new(config_b)
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&slow_network)))
        .with_replication_network_consensus_enabled(false);
    runtime_b.start().expect("start b");

    let intermediate_visible = wait_until(Instant::now() + Duration::from_secs(3), || {
        let snapshot = runtime_b.snapshot();
        snapshot.consensus.committed_height > 0
            && snapshot.consensus.committed_height < target_height
    });
    let intermediate_snapshot = runtime_b.snapshot();
    assert!(
        intermediate_visible,
        "runtime snapshot did not publish intermediate gap-sync progress: {:?}",
        intermediate_snapshot
    );

    let synced = wait_until(Instant::now() + Duration::from_secs(8), || {
        runtime_b.snapshot().consensus.committed_height >= target_height
    });
    assert!(synced, "observer did not finish sync in time");

    runtime_b.stop().expect("stop b");
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}
