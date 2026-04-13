use super::*;

#[test]
fn replication_network_handle_rejects_empty_topic() {
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());
    let err = NodeReplicationNetworkHandle::new(network)
        .with_topic("   ")
        .expect_err("empty topic");
    assert!(matches!(err, NodeError::InvalidConfig { .. }));
}

#[test]
fn runtime_network_replication_respects_topic_isolation() {
    let dir_a = temp_dir("network-topic-a");
    let dir_b = temp_dir("network-topic-b");
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
        signed_pos_config_with_signer_seeds(validators, &[("node-a", 81), ("node-b", 82)]);
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());

    let config_a = NodeConfig::new("node-a", "world-topic-repl", NodeRole::Sequencer)
        .expect("config a")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick a")
        .with_pos_config(pos_config.clone())
        .expect("pos config a")
        .with_auto_attest_all_validators(true)
        .with_replication(signed_replication_config(dir_a.clone(), 81));
    let config_b = NodeConfig::new("node-b", "world-topic-repl", NodeRole::Observer)
        .expect("config b")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick b")
        .with_pos_config(pos_config)
        .expect("pos config b")
        .with_replication(signed_replication_config(dir_b.clone(), 82));

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(config_a))
        .with_replication_network(
            NodeReplicationNetworkHandle::new(Arc::clone(&network))
                .with_topic("aw.world-topic-repl.replication.a")
                .expect("topic a"),
        );
    let mut runtime_b = NodeRuntime::new(config_b).with_replication_network(
        NodeReplicationNetworkHandle::new(Arc::clone(&network))
            .with_topic("aw.world-topic-repl.replication.b")
            .expect("topic b"),
    );
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
fn runtime_non_sequencer_followers_do_not_advance_without_sequencer() {
    let dir_a = temp_dir("no-local-proposal-a");
    let dir_b = temp_dir("no-local-proposal-b");
    let dir_c = temp_dir("no-local-proposal-c");
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
        &[("node-a", 131), ("node-b", 132), ("node-c", 133)],
    );
    let network: Arc<
        dyn oasis7_proto::distributed_net::DistributedNetwork<WorldError> + Send + Sync,
    > = Arc::new(TestInMemoryNetwork::default());

    let build_sequencer = || {
        NodeConfig::new("node-a", "world-no-local-proposal", NodeRole::Sequencer)
            .expect("sequencer config")
            .with_tick_interval(Duration::from_millis(10))
            .expect("sequencer tick")
            .with_pos_config(pos_config.clone())
            .expect("sequencer pos config")
            .with_auto_attest_all_validators(true)
            .with_replication(signed_replication_config(dir_a.clone(), 131))
    };
    let build_storage = || {
        NodeConfig::new("node-b", "world-no-local-proposal", NodeRole::Storage)
            .expect("storage config")
            .with_tick_interval(Duration::from_millis(10))
            .expect("storage tick")
            .with_pos_config(pos_config.clone())
            .expect("storage pos config")
            .with_auto_attest_all_validators(true)
            .with_allow_local_proposals(false)
            .with_replication(signed_replication_config(dir_b.clone(), 132))
    };
    let build_observer = || {
        NodeConfig::new("node-c", "world-no-local-proposal", NodeRole::Observer)
            .expect("observer config")
            .with_tick_interval(Duration::from_millis(10))
            .expect("observer tick")
            .with_pos_config(pos_config.clone())
            .expect("observer pos config")
            .with_auto_attest_all_validators(true)
            .with_allow_local_proposals(false)
            .with_replication(signed_replication_config(dir_c.clone(), 133))
    };

    let mut runtime_a = with_noop_execution_hook(NodeRuntime::new(build_sequencer()))
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));
    let mut runtime_b = NodeRuntime::new(build_storage())
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));
    let mut runtime_c = NodeRuntime::new(build_observer())
        .with_replication_network(NodeReplicationNetworkHandle::new(Arc::clone(&network)));
    runtime_a.start().expect("start sequencer");
    runtime_b.start().expect("start storage");
    runtime_c.start().expect("start observer");

    let converged = wait_until(Instant::now() + Duration::from_secs(5), || {
        let snapshot_a = runtime_a.snapshot();
        let snapshot_b = runtime_b.snapshot();
        let snapshot_c = runtime_c.snapshot();
        let target_height = snapshot_a.consensus.committed_height;
        target_height >= 3
            && snapshot_b.consensus.committed_height >= target_height
            && snapshot_c.consensus.committed_height >= target_height
    });
    let snapshot_a = runtime_a.snapshot();
    let snapshot_b = runtime_b.snapshot();
    let snapshot_c = runtime_c.snapshot();
    assert!(
        converged,
        "cluster did not converge before sequencer stop: a_height={} a_error={:?} b_height={} b_error={:?} c_height={} c_error={:?}",
        snapshot_a.consensus.committed_height,
        snapshot_a.last_error,
        snapshot_b.consensus.committed_height,
        snapshot_b.last_error,
        snapshot_c.consensus.committed_height,
        snapshot_c.last_error
    );

    runtime_a.stop().expect("stop sequencer");
    thread::sleep(Duration::from_millis(150));
    let baseline_b = runtime_b.snapshot();
    let baseline_c = runtime_c.snapshot();
    let advanced_without_sequencer =
        wait_until(Instant::now() + Duration::from_millis(700), || {
            runtime_b.snapshot().consensus.committed_height > baseline_b.consensus.committed_height
                || runtime_c.snapshot().consensus.committed_height
                    > baseline_c.consensus.committed_height
        });
    let final_b = runtime_b.snapshot();
    let final_c = runtime_c.snapshot();
    assert!(
        !advanced_without_sequencer,
        "followers advanced without sequencer: baseline_b={} final_b={} b_error={:?} baseline_c={} final_c={} c_error={:?}",
        baseline_b.consensus.committed_height,
        final_b.consensus.committed_height,
        final_b.last_error,
        baseline_c.consensus.committed_height,
        final_c.consensus.committed_height,
        final_c.last_error
    );

    runtime_b.stop().expect("stop storage");
    runtime_c.stop().expect("stop observer");
    let _ = fs::remove_dir_all(&dir_a);
    let _ = fs::remove_dir_all(&dir_b);
    let _ = fs::remove_dir_all(&dir_c);
}
