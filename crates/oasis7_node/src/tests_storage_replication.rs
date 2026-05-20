#[path = "tests_storage_challenge_gate.rs"]
mod storage_challenge_gate_tests;

#[test]
fn runtime_network_replication_gap_sync_reports_error_after_retries_exhausted() {
    let world_id = "world-network-gap-retry-exhausted";
    let dir_a = temp_dir("network-gap-retry-a");
    let dir_b = temp_dir("network-gap-retry-b");
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
        signed_pos_config_with_signer_seeds(validators, &[("node-a", 89), ("node-b", 90)]);
    let network_impl = Arc::new(TestInMemoryNetwork::default());
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();

    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_auto_attest_all_validators(true)
        .with_replication(signed_replication_config(dir_a.clone(), 89));
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config b")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(signed_replication_config(dir_b.clone(), 90));

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(config_a))
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));
    runtime_a.start().expect("start a");
    let reached = wait_until(Instant::now() + Duration::from_secs(2), || {
        runtime_a.snapshot().consensus.committed_height >= 3
    });
    assert!(reached, "sequencer did not reach target height in time");
    let target_height = runtime_a.snapshot().consensus.committed_height;
    runtime_a.stop().expect("stop a");

    let request = signed_fetch_commit_request_for_test(world_id, target_height, 89);
    let payload = serde_json::to_vec(&request).expect("encode commit request");
    let response_payload = network
        .request(
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            payload.as_slice(),
        )
        .expect("fetch commit");
    let response: super::replication::FetchCommitResponse =
        serde_json::from_slice(&response_payload).expect("decode commit response");
    assert!(response.found, "missing high commit");
    let high_message = response.message.expect("high commit payload");

    let topic = super::network_bridge::default_replication_topic(world_id);
    network_impl.clear_topic(topic.as_str());
    network_impl
        .clear_topic(super::network_bridge::default_consensus_proposal_topic(world_id).as_str());
    network_impl
        .clear_topic(super::network_bridge::default_consensus_attestation_topic(world_id).as_str());
    network_impl
        .clear_topic(super::network_bridge::default_consensus_commit_topic(world_id).as_str());
    let high_payload = serde_json::to_vec(&high_message).expect("encode high message");
    network
        .publish(topic.as_str(), high_payload.as_slice())
        .expect("publish high message");

    let mut runtime_b = NodeRuntime::new(config_b)
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));
    runtime_b.start().expect("start b");
    network
        .register_handler(
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            Box::new(move |_payload| {
                Err(WorldError::NetworkProtocolUnavailable {
                    protocol: "forced-gap-sync-retry-failure".to_string(),
                })
            }),
        )
        .expect("register commit retry-failure handler");
    let errored = wait_until(Instant::now() + Duration::from_secs(3), || {
        runtime_b
            .snapshot()
            .last_error
            .as_deref()
            .map(|reason| {
                reason.contains("gap sync height")
                    && reason.contains("failed after 3 attempts")
                    && reason.contains("attempt 3/3 failed")
            })
            .unwrap_or(false)
    });
    let snapshot_b = runtime_b.snapshot();
    assert!(
        errored,
        "observer did not report gap sync retry exhaustion: committed_height={} network_committed_height={} last_error={:?}",
        snapshot_b.consensus.committed_height,
        snapshot_b.consensus.network_committed_height,
        snapshot_b.last_error
    );

    runtime_b.stop().expect("stop b");
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn replication_gap_sync_backfills_when_consensus_height_already_advanced() {
    let world_id = "world-gap-sync-consensus-ahead";
    let dir_a = temp_dir("gap-sync-consensus-ahead-a");
    let dir_b = temp_dir("gap-sync-consensus-ahead-b");
    let (_, public_key_a) = deterministic_keypair_hex(143);
    let (_, public_key_b) = deterministic_keypair_hex(144);
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
        signed_pos_config_with_signer_seeds(validators, &[("node-a", 143), ("node-b", 144)]);
    let replication_config_a = signed_replication_config(dir_a.clone(), 143)
        .with_remote_writer_allowlist(vec![public_key_b.clone()])
        .expect("allowlist a");
    let replication_config_b = signed_replication_config(dir_b.clone(), 144)
        .with_remote_writer_allowlist(vec![public_key_a.clone()])
        .expect("allowlist b");
    let config_a = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("config a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_replication(replication_config_a.clone());
    let config_b = NodeConfig::new("node-b", world_id, NodeRole::Storage)
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
            approved_stake: 60,
            rejected_stake: 0,
            required_stake: 40,
            total_stake: 100,
        };
        replication_a
            .build_local_commit_message(
                "node-a",
                world_id,
                1_000 + i64::try_from(height).expect("height fits i64"),
                &decision,
                None,
                None,
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
    engine_b.committed_height = 3;
    engine_b.network_committed_height = 3;
    engine_b.next_height = 4;
    engine_b.last_committed_block_hash = Some("block-3".to_string());

    assert_eq!(
        replication_b
            .latest_persisted_commit_height(world_id)
            .expect("initial persisted height"),
        0
    );
    engine_b
        .sync_missing_replication_commits(
            &endpoint_b,
            "node-b",
            world_id,
            Some(&mut replication_b),
            None,
        )
        .expect("gap sync");

    assert!(replication_b
        .load_commit_message_by_height(world_id, 1)
        .expect("load commit 1")
        .is_some());
    assert!(replication_b
        .load_commit_message_by_height(world_id, 3)
        .expect("load commit 3")
        .is_some());
    assert_eq!(
        replication_b
            .latest_persisted_commit_height(world_id)
            .expect("persisted height after sync"),
        3
    );
    assert_eq!(engine_b.committed_height, 3);
    assert_eq!(engine_b.next_height, 4);

    let store_b = LocalCasStore::new(dir_b.join("store"));
    assert!(
        store_b
            .list_files()
            .expect("list files")
            .iter()
            .any(|item| item.path == "consensus/commits/00000000000000000003.json"),
        "expected synced commit file to be present"
    );

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn observer_replication_runtime_starts_without_registering_data_service_handlers() {
    let world_id = "world-observer-lane-gate";
    let dir = temp_dir("observer-lane-gate");
    let network_impl = Arc::new(TestInMemoryNetwork::default());
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = network_impl.clone();
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 91)],
    );
    let config = NodeConfig::new("node-observer", world_id, NodeRole::Observer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(signed_replication_config(dir.clone(), 92));

    let mut runtime = with_noop_execution_hook(NodeRuntime::new(config))
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));
    runtime.start().expect("start runtime");

    let commit_request = signed_fetch_commit_request_for_test(world_id, 1, 92);
    let commit_payload = serde_json::to_vec(&commit_request).expect("encode commit request");
    let commit_err = network
        .request(
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
            commit_payload.as_slice(),
        )
        .expect_err("observer should not serve commit sync protocol");
    assert!(matches!(
        commit_err,
        WorldError::NetworkProtocolUnavailable { .. }
    ));

    let blob_request = signed_fetch_blob_request_for_test("content-hash-1", 92);
    let blob_payload = serde_json::to_vec(&blob_request).expect("encode blob request");
    let blob_err = network
        .request(
            super::replication::REPLICATION_FETCH_BLOB_PROTOCOL,
            blob_payload.as_slice(),
        )
        .expect_err("observer should not serve blob/state protocol");
    assert!(matches!(
        blob_err,
        WorldError::NetworkProtocolUnavailable { .. }
    ));

    runtime.stop().expect("stop runtime");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn non_proposer_committed_decision_does_not_persist_local_replication() {
    let world_id = "world-non-proposer-replication-guard";
    let dir = temp_dir("non-proposer-replication-guard");
    let validators = vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 100,
        },
        PosValidator {
            validator_id: "node-c".to_string(),
            stake: 100,
        },
    ];
    let pos_config = signed_pos_config_with_signer_seeds(
        validators.clone(),
        &[("node-a", 31), ("node-b", 32), ("node-c", 33)],
    );
    let probe_config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
        .expect("probe config")
        .with_pos_config(pos_config.clone())
        .expect("probe pos config");
    let probe_engine = PosNodeEngine::new(&probe_config).expect("probe engine");
    let slot = 0;
    let expected_proposer = probe_engine
        .expected_proposer(slot)
        .expect("expected proposer");
    let non_proposer = validators
        .iter()
        .map(|validator| validator.validator_id.as_str())
        .find(|validator_id| *validator_id != expected_proposer)
        .expect("non proposer");
    let signer_seed = match non_proposer {
        "node-a" => 31,
        "node-b" => 32,
        "node-c" => 33,
        other => panic!("unexpected validator {other}"),
    };

    let config = NodeConfig::new(non_proposer, world_id, NodeRole::Sequencer)
        .expect("config")
        .with_pos_config(pos_config)
        .expect("pos config")
        .with_replication(signed_replication_config(dir.clone(), signer_seed));
    let mut engine = PosNodeEngine::new(&config).expect("engine");
    engine.last_execution_height = 1;
    engine.last_execution_block_hash = Some("exec-block-1".to_string());
    engine.last_execution_state_root = Some("exec-state-1".to_string());

    let mut replication = ReplicationRuntime::new(
        config.replication.as_ref().expect("replication"),
        non_proposer,
    )
    .expect("replication runtime");
    let decision = PosDecision {
        height: 1,
        slot,
        epoch: 0,
        status: PosConsensusStatus::Committed,
        block_hash: "block-1".to_string(),
        action_root: empty_action_root(),
        committed_actions: Vec::new(),
        approved_stake: 200,
        rejected_stake: 0,
        required_stake: 201,
        total_stake: 300,
    };

    engine
        .broadcast_local_replication(
            None,
            None,
            non_proposer,
            world_id,
            1_000,
            &decision,
            Some(&mut replication),
        )
        .expect("broadcast local replication");

    assert_eq!(
        replication
            .latest_persisted_commit_height(world_id)
            .expect("latest persisted height"),
        0
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn runtime_remote_replication_ingest_publishes_blob_provider_to_dht() {
    let dir_a = temp_dir("publish-remote-provider-a");
    let dir_b = temp_dir("publish-remote-provider-b");
    let dht = Arc::new(TestReplicaMaintenanceDht::new("peer-seq", "peer-store"));
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![
            PosValidator {
                validator_id: "node-a".to_string(),
                stake: 60,
            },
            PosValidator {
                validator_id: "node-b".to_string(),
                stake: 40,
            },
        ],
        &[("node-a", 95), ("node-b", 96)],
    );
    let config_a = NodeConfig::new(
        "node-a",
        "world-publish-remote-provider",
        NodeRole::Sequencer,
    )
    .expect("config a")
    .with_tick_interval(Duration::from_millis(10))
    .expect("tick a")
    .with_pos_config(pos_config.clone())
    .expect("pos config a")
    .with_auto_attest_all_validators(true)
    .with_replication(signed_replication_config(dir_a.clone(), 95));
    let config_b = NodeConfig::new("node-b", "world-publish-remote-provider", NodeRole::Storage)
        .expect("config b")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(signed_replication_config(dir_b.clone(), 96));

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(config_a))
        .with_replication_network(
            NodeReplicationNetworkHandle::new(Arc::clone(&network))
                .with_dht(dht.clone())
                .with_local_provider_id("peer-seq"),
        );
    let mut runtime_b = NodeRuntime::new(config_b).with_replication_network(
        NodeReplicationNetworkHandle::new(Arc::clone(&network))
            .with_dht(dht.clone())
            .with_local_provider_id("peer-store"),
    );

    runtime_a.start().expect("start a");
    runtime_b.start().expect("start b");

    let published = wait_until(Instant::now() + Duration::from_secs(3), || {
        dht.published_records()
            .iter()
            .any(|(_, _, provider_id)| provider_id == "peer-store")
    });
    assert!(
        published,
        "expected storage ingest path to publish peer-store provider, got {:?}",
        dht.published_records()
    );

    runtime_a.stop().expect("stop a");
    runtime_b.stop().expect("stop b");
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn runtime_gossip_replication_with_signature_applies_files() {
    let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
    let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
    let addr_a = socket_a.local_addr().expect("addr a");
    let addr_b = socket_b.local_addr().expect("addr b");
    drop(socket_a);
    drop(socket_b);

    let dir_a = temp_dir("signed-repl-a");
    let dir_b = temp_dir("signed-repl-b");
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
        signed_pos_config_with_signer_seeds(validators, &[("node-a", 11), ("node-b", 22)]);

    let config_a = NodeConfig::new("node-a", "world-signed", NodeRole::Sequencer)
        .expect("config a")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_auto_attest_all_validators(true)
        .with_gossip_optional(addr_a, vec![addr_b])
        .with_replication(signed_replication_config(dir_a.clone(), 11));
    let config_b = NodeConfig::new("node-b", "world-signed", NodeRole::Observer)
        .expect("config b")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_gossip_optional(addr_b, vec![addr_a])
        .with_replication(signed_replication_config(dir_b.clone(), 22));

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(config_a));
    let mut runtime_b = NodeRuntime::new(config_b);
    runtime_a.start().expect("start a");
    runtime_b.start().expect("start b");

    let store_b = LocalCasStore::new(dir_b.join("store"));
    let replicated = wait_until(Instant::now() + Duration::from_secs(3), || {
        store_b
            .list_files()
            .map(|files| {
                files
                    .iter()
                    .any(|item| item.path.starts_with("consensus/commits/"))
            })
            .unwrap_or(false)
    });
    let files = store_b.list_files().expect("list files");
    assert!(
        replicated,
        "expected signed gossip replication to apply commit files, got {files:?}"
    );

    runtime_a.stop().expect("stop a");
    runtime_b.stop().expect("stop b");

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn runtime_gossip_replication_rejects_unsigned_when_signature_enforced() {
    let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
    let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
    let addr_a = socket_a.local_addr().expect("addr a");
    let addr_b = socket_b.local_addr().expect("addr b");
    drop(socket_a);
    drop(socket_b);

    let dir_a = temp_dir("unsigned-a");
    let dir_b = temp_dir("enforced-b");
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
        signed_pos_config_with_signer_seeds(validators, &[("node-a", 11), ("node-b", 33)]);

    let config_a = NodeConfig::new("node-a", "world-enforced", NodeRole::Sequencer)
        .expect("config a")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_gossip_optional(addr_a, vec![addr_b])
        .with_replication_root(dir_a.clone())
        .expect("replication a");
    let config_b = NodeConfig::new("node-b", "world-enforced", NodeRole::Observer)
        .expect("config b")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_gossip_optional(addr_b, vec![addr_a])
        .with_replication(signed_replication_config(dir_b.clone(), 33));

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(config_a));
    let mut runtime_b = NodeRuntime::new(config_b);
    runtime_a.start().expect("start a");
    runtime_b.start().expect("start b");
    thread::sleep(Duration::from_millis(220));

    runtime_a.stop().expect("stop a");
    runtime_b.stop().expect("stop b");

    let store_b = LocalCasStore::new(dir_b.join("store"));
    let files = store_b.list_files().expect("list files");
    assert!(files.is_empty());

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn runtime_gossip_replication_persists_guard_across_restart() {
    let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
    let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
    let addr_a = socket_a.local_addr().expect("addr a");
    let addr_b = socket_b.local_addr().expect("addr b");
    drop(socket_a);
    drop(socket_b);

    let dir_a = temp_dir("restart-a");
    let dir_b = temp_dir("restart-b");
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
        signed_pos_config_with_signer_seeds(validators, &[("node-a", 55), ("node-b", 66)]);

    let build_config_a = || {
        NodeConfig::new("node-a", "world-restart", NodeRole::Sequencer)
            .expect("config a")
            .with_tick_interval(Duration::from_millis(10))
            .expect("tick a")
            .with_pos_config(pos_config.clone())
            .expect("pos config a")
            .with_auto_attest_all_validators(true)
            .with_gossip_optional(addr_a, vec![addr_b])
            .with_replication(signed_replication_config(dir_a.clone(), 55))
    };
    let build_config_b = || {
        NodeConfig::new("node-b", "world-restart", NodeRole::Observer)
            .expect("config b")
            .with_tick_interval(Duration::from_millis(10))
            .expect("tick b")
            .with_pos_config(pos_config.clone())
            .expect("pos config b")
            .with_gossip_optional(addr_b, vec![addr_a])
            .with_replication(signed_replication_config(dir_b.clone(), 66))
    };

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(build_config_a()));
    let mut runtime_b = NodeRuntime::new(build_config_b());
    runtime_a.start().expect("start a first");
    runtime_b.start().expect("start b first");
    thread::sleep(Duration::from_millis(220));
    let snapshot_b_first = runtime_b.snapshot();
    runtime_a.stop().expect("stop a first");
    runtime_b.stop().expect("stop b first");
    assert!(snapshot_b_first.last_error.is_none());

    let guard_path = dir_b.join("replication_guard.json");
    let guard_before: SingleWriterReplicationGuard =
        serde_json::from_slice(&fs::read(&guard_path).expect("read guard before"))
            .expect("parse guard before");
    assert!(guard_before.last_sequence >= 1);

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(build_config_a()));
    let mut runtime_b = NodeRuntime::new(build_config_b());
    runtime_a.start().expect("start a second");
    runtime_b.start().expect("start b second");
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let maybe_guard = fs::read(&guard_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<SingleWriterReplicationGuard>(&bytes).ok());
        if maybe_guard
            .as_ref()
            .is_some_and(|guard| guard.last_sequence > guard_before.last_sequence)
        {
            break;
        }
        if Instant::now() >= deadline {
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    let snapshot_b_second = runtime_b.snapshot();
    runtime_a.stop().expect("stop a second");
    runtime_b.stop().expect("stop b second");
    assert!(snapshot_b_second.last_error.is_none());

    let guard_after: SingleWriterReplicationGuard =
        serde_json::from_slice(&fs::read(&guard_path).expect("read guard after"))
            .expect("parse guard after");
    assert_eq!(guard_after.writer_id, guard_before.writer_id);
    assert!(guard_after.last_sequence > guard_before.last_sequence);

    let store_b = LocalCasStore::new(dir_b.join("store"));
    let files = store_b.list_files().expect("list files");
    assert!(files.len() >= 2);

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
}

#[test]
fn runtime_network_replication_accepts_writer_failover_with_epoch_rotation() {
    let dir_a = temp_dir("failover-a");
    let dir_b = temp_dir("failover-b");
    let dir_c = temp_dir("failover-c");
    let validators = vec![
        PosValidator {
            validator_id: "node-a".to_string(),
            stake: 34,
        },
        PosValidator {
            validator_id: "node-b".to_string(),
            stake: 33,
        },
        PosValidator {
            validator_id: "node-c".to_string(),
            stake: 33,
        },
    ];
    let pos_config = signed_pos_config_with_signer_seeds(
        validators,
        &[("node-a", 91), ("node-b", 92), ("node-c", 93)],
    );
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());

    let build_observer = || {
        NodeConfig::new("node-b", "world-failover-repl", NodeRole::Observer)
            .expect("observer config")
            .with_tick_interval(Duration::from_millis(10))
            .expect("observer tick")
            .with_pos_config(pos_config.clone())
            .expect("observer pos config")
            .with_replication(signed_replication_config(dir_b.clone(), 92))
    };
    let build_sequencer_a = || {
        NodeConfig::new("node-a", "world-failover-repl", NodeRole::Sequencer)
            .expect("sequencer a config")
            .with_tick_interval(Duration::from_millis(10))
            .expect("sequencer a tick")
            .with_pos_config(pos_config.clone())
            .expect("sequencer a pos config")
            .with_auto_attest_all_validators(true)
            .with_replication(signed_replication_config(dir_a.clone(), 91))
    };
    let build_sequencer_c = || {
        NodeConfig::new("node-c", "world-failover-repl", NodeRole::Sequencer)
            .expect("sequencer c config")
            .with_tick_interval(Duration::from_millis(10))
            .expect("sequencer c tick")
            .with_pos_config(pos_config.clone())
            .expect("sequencer c pos config")
            .with_auto_attest_all_validators(true)
            .with_replication(signed_replication_config(dir_c.clone(), 93))
    };

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(build_sequencer_a()))
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));
    let mut runtime_b = NodeRuntime::new(build_observer())
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));
    runtime_a.start().expect("start a");
    runtime_b.start().expect("start b with a");
    thread::sleep(Duration::from_millis(220));
    runtime_a.stop().expect("stop a");
    runtime_b.stop().expect("stop b after a");

    let guard_path = dir_b.join("replication_guard.json");
    let guard_before: SingleWriterReplicationGuard =
        serde_json::from_slice(&fs::read(&guard_path).expect("read guard before"))
            .expect("parse guard before");
    assert!(guard_before.last_sequence >= 1);
    assert!(guard_before.writer_epoch >= 1);
    let writer_before = guard_before.writer_id.clone();

    let mut runtime_c = with_noop_execution_hook(NodeRuntime::new(build_sequencer_c()))
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));
    let mut runtime_b = NodeRuntime::new(build_observer())
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));
    runtime_c.start().expect("start c");
    runtime_b.start().expect("start b with c");
    thread::sleep(Duration::from_millis(260));
    runtime_c.stop().expect("stop c");
    runtime_b.stop().expect("stop b after c");

    let guard_after: SingleWriterReplicationGuard =
        serde_json::from_slice(&fs::read(&guard_path).expect("read guard after"))
            .expect("parse guard after");
    assert!(guard_after.last_sequence >= 1);
    assert!(guard_after.writer_epoch > guard_before.writer_epoch);
    assert_ne!(guard_after.writer_id, writer_before);

    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
    let _ = fs::remove_dir_all(&dir_c);
}
