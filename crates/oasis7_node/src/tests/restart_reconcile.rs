use super::*;

#[test]
fn runtime_restart_reconciles_stale_pos_state_from_persisted_replication_height() {
    let dir = temp_dir("pos-state-restart-reconcile");
    let build_config = || {
        NodeConfig::new("node-a", "world-pos-state-reconcile", NodeRole::Sequencer)
            .expect("config")
            .with_tick_interval(Duration::from_millis(10))
            .expect("tick")
            .with_replication_root(dir.clone())
            .expect("replication")
    };

    let mut runtime = NodeRuntime::new(build_config()).with_execution_hook(
        RecordingExecutionHook::new(Arc::new(Mutex::new(Vec::new()))),
    );
    runtime.start().expect("start first");
    let reached = wait_until(Instant::now() + Duration::from_secs(2), || {
        runtime.snapshot().consensus.committed_height >= 8
    });
    assert!(reached, "runtime did not reach seed height before restart");
    runtime.stop().expect("stop first");
    let first = runtime.snapshot();
    assert!(first.last_error.is_none());

    let replication = super::super::replication::ReplicationRuntime::new(
        &signed_replication_config(dir.clone(), 100),
        "node-a",
    )
    .expect("replication runtime");
    let persisted_height = replication
        .latest_persisted_commit_height("world-pos-state-reconcile")
        .expect("persisted height");
    assert!(
        persisted_height >= first.consensus.committed_height,
        "expected replication persistence to keep latest height, first={} persisted={persisted_height}",
        first.consensus.committed_height
    );

    let state_path = dir.join("node_pos_state.json");
    let mut stale = serde_json::from_slice::<super::super::pos_state_store::PosNodeStateSnapshot>(
        &fs::read(&state_path).expect("read pos state"),
    )
    .expect("parse pos state");
    stale.next_height = 3;
    stale.next_slot = 2;
    stale.last_observed_slot = 2;
    stale.last_observed_tick = 20;
    stale.committed_height = 2;
    stale.network_committed_height = 2;
    stale.last_broadcast_proposal_height = 2;
    stale.last_broadcast_local_attestation_height = 2;
    stale.last_broadcast_committed_height = 2;
    stale.last_committed_block_hash = Some("stale-height-2".to_string());
    stale.last_execution_height = 2;
    stale.last_execution_block_hash = Some("stale-exec-height-2".to_string());
    stale.last_execution_state_root = Some("stale-state-height-2".to_string());
    fs::write(
        &state_path,
        serde_json::to_vec_pretty(&stale).expect("serialize stale state"),
    )
    .expect("write stale state");

    let mut restarted = NodeRuntime::new(build_config()).with_execution_hook(
        RecordingExecutionHook::new(Arc::new(Mutex::new(Vec::new()))),
    );
    restarted.start().expect("start second");
    let mut first_positive_height = 0;
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        let snapshot = restarted.snapshot();
        if snapshot.consensus.committed_height > 0 {
            first_positive_height = snapshot.consensus.committed_height;
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    let first_positive_snapshot = restarted.snapshot();
    assert!(
        first_positive_height >= persisted_height,
        "restart should reconcile to persisted height before new commits: first_positive={} persisted={} final={} last_error={:?}",
        first_positive_height,
        persisted_height,
        first_positive_snapshot.consensus.committed_height,
        first_positive_snapshot.last_error
    );
    let advanced = wait_until(Instant::now() + Duration::from_secs(2), || {
        restarted.snapshot().consensus.committed_height > persisted_height
    });
    restarted.stop().expect("stop second");
    let second = restarted.snapshot();
    assert!(second.last_error.is_none(), "{:?}", second.last_error);
    assert!(
        advanced && second.consensus.committed_height > persisted_height,
        "runtime should continue past persisted height after reconcile: final={} persisted={persisted_height}",
        second.consensus.committed_height
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn observer_restart_replays_persisted_commits_when_execution_head_lags_committed_height() {
    let dir_remote = temp_dir("observer-restart-replay-remote");
    let dir_local = temp_dir("observer-restart-replay-local");
    let world_id = "world-observer-restart-replay";
    let (_, public_key_a) = deterministic_keypair_hex(210);
    let pos_config = signed_pos_config_with_signer_seeds(
        vec![PosValidator {
            validator_id: "node-a".to_string(),
            stake: 100,
        }],
        &[("node-a", 210)],
    );
    let remote_config = signed_replication_config(dir_remote.clone(), 210);
    let local_config = signed_replication_config(dir_local.clone(), 211)
        .with_remote_writer_allowlist(vec![public_key_a])
        .expect("local remote writer allowlist");
    let mut remote_replication =
        super::super::replication::ReplicationRuntime::new(&remote_config, "node-a")
            .expect("remote replication runtime");
    let mut local_replication =
        super::super::replication::ReplicationRuntime::new(&local_config, "node-b")
            .expect("local replication runtime");

    for height in 1..=3_u64 {
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
        let execution_block_hash = format!("exec-block-{height:020}");
        let execution_state_root = format!("exec-state-{height:020}");
        let message = remote_replication
            .build_local_commit_message(
                "node-a",
                world_id,
                1_000 + i64::try_from(height).expect("height fits i64"),
                &decision,
                Some(execution_block_hash.as_str()),
                Some(execution_state_root.as_str()),
            )
            .expect("build remote commit")
            .expect("remote message");
        local_replication
            .apply_remote_message("node-b", world_id, &message)
            .expect("apply remote commit");
    }
    assert_eq!(
        local_replication
            .latest_persisted_commit_height(world_id)
            .expect("latest persisted height"),
        3
    );

    let stale_state = super::super::pos_state_store::PosNodeStateSnapshot {
        next_height: 4,
        next_slot: 3,
        last_observed_slot: 3,
        missed_slot_count: 0,
        last_observed_tick: 30,
        missed_tick_count: 0,
        committed_height: 3,
        network_committed_height: 3,
        last_broadcast_proposal_height: 0,
        last_broadcast_local_attestation_height: 0,
        last_broadcast_committed_height: 3,
        last_committed_block_hash: Some("block-3".to_string()),
        last_execution_height: 0,
        last_execution_block_hash: None,
        last_execution_state_root: None,
    };
    fs::write(
        dir_local.join("node_pos_state.json"),
        serde_json::to_vec_pretty(&stale_state).expect("serialize stale observer state"),
    )
    .expect("write stale observer state");

    let execution_calls: Arc<Mutex<Vec<NodeExecutionCommitContext>>> =
        Arc::new(Mutex::new(Vec::new()));
    let config = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("observer config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("observer tick")
        .with_pos_config(pos_config)
        .expect("observer pos config")
        .with_replication(local_config);
    let mut runtime = NodeRuntime::new(config)
        .with_execution_hook(RecordingExecutionHook::new(Arc::clone(&execution_calls)));
    runtime.start().expect("start observer");
    let reconciled = wait_until(Instant::now() + Duration::from_secs(1), || {
        runtime.snapshot().consensus.last_execution_height >= 3
    });
    runtime.stop().expect("stop observer");
    let snapshot = runtime.snapshot();
    assert!(snapshot.last_error.is_none(), "{:?}", snapshot.last_error);
    assert!(
        reconciled,
        "observer did not replay persisted execution commits: committed_height={} last_execution_height={} last_error={:?}",
        snapshot.consensus.committed_height,
        snapshot.consensus.last_execution_height,
        snapshot.last_error
    );
    assert_eq!(snapshot.consensus.committed_height, 3);
    assert_eq!(snapshot.consensus.last_execution_height, 3);
    let calls = execution_calls
        .lock()
        .expect("lock execution calls after observer replay");
    let heights = calls
        .iter()
        .map(|context| context.height)
        .collect::<Vec<_>>();
    assert_eq!(heights, vec![1, 2, 3]);

    let _ = fs::remove_dir_all(&dir_remote);
    let _ = fs::remove_dir_all(&dir_local);
}
