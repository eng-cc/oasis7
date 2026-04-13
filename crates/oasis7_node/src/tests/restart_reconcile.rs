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
    restarted.stop().expect("stop second");
    let second = restarted.snapshot();
    assert!(
        first_positive_height >= persisted_height,
        "restart should reconcile to persisted height before new commits: first_positive={} persisted={} final={} last_error={:?}",
        first_positive_height,
        persisted_height,
        second.consensus.committed_height,
        second.last_error
    );
    assert!(second.last_error.is_none(), "{:?}", second.last_error);
    assert!(
        second.consensus.committed_height > persisted_height,
        "runtime should continue past persisted height after reconcile: final={} persisted={persisted_height}",
        second.consensus.committed_height
    );

    let _ = fs::remove_dir_all(&dir);
}
