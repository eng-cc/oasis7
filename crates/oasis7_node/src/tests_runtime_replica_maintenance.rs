use super::*;
use std::fs;

#[test]
fn config_rejects_non_positive_replica_maintenance_poll_interval() {
    let err = NodeConfig::new("node-maint", "world-maint", NodeRole::Observer)
        .expect("config")
        .with_replica_maintenance(NodeReplicaMaintenanceConfig {
            poll_interval_ms: 0,
            ..NodeReplicaMaintenanceConfig::default()
        })
        .expect_err("non-positive poll interval should be rejected");
    assert!(
        matches!(err, NodeError::InvalidConfig { reason } if reason.contains("replica_maintenance.poll_interval_ms"))
    );
}

#[test]
fn runtime_replica_maintenance_poll_executes_local_target_tasks() {
    let replication_root = temp_dir("runtime-replica-maintenance");
    let config = NodeConfig::new("node-a", "world-maint", NodeRole::Sequencer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick interval")
        .with_require_execution_on_commit(false)
        .with_replication_root(replication_root.clone())
        .expect("replication root")
        .with_replica_maintenance(NodeReplicaMaintenanceConfig {
            max_content_hash_samples_per_round: 2,
            target_replicas_per_blob: 2,
            max_repairs_per_round: 2,
            max_rebalances_per_round: 0,
            poll_interval_ms: 10,
            ..NodeReplicaMaintenanceConfig::default()
        })
        .expect("replica maintenance");
    let network = Arc::new(TestInMemoryNetwork::default());
    let dht = Arc::new(TestReplicaMaintenanceDht::new("source-a", "node-a"));
    let mut runtime = NodeRuntime::new(config)
        .with_replication_network(NodeReplicationNetworkHandle::new(network))
        .with_replica_maintenance_dht(dht.clone());
    runtime.start().expect("start");
    let committed_ready = wait_until(Instant::now() + Duration::from_secs(2), || {
        runtime.snapshot().consensus.committed_height >= 2
    });
    let published_ready = wait_until(Instant::now() + Duration::from_secs(2), || {
        !dht.published_records().is_empty()
    });
    runtime.stop().expect("stop");

    let snapshot = runtime.snapshot();
    assert!(snapshot.last_error.is_none(), "{:?}", snapshot.last_error);
    assert!(
        committed_ready,
        "expected at least 2 committed heights before maintenance checks"
    );
    assert!(published_ready, "expected maintenance publish records");
    let published = dht.published_records();
    assert!(
        !published.is_empty(),
        "expected maintenance publish records"
    );
    assert!(published
        .iter()
        .any(|(_, _, provider_id)| provider_id == "node-a"));

    let _ = fs::remove_dir_all(replication_root);
}

#[test]
fn runtime_replica_maintenance_poll_skips_without_dht() {
    let replication_root = temp_dir("runtime-replica-maintenance-no-dht");
    let config = NodeConfig::new("node-a", "world-maint-no-dht", NodeRole::Sequencer)
        .expect("config")
        .with_tick_interval(Duration::from_millis(10))
        .expect("tick interval")
        .with_require_execution_on_commit(false)
        .with_replication_root(replication_root.clone())
        .expect("replication root")
        .with_replica_maintenance(NodeReplicaMaintenanceConfig {
            poll_interval_ms: 10,
            ..NodeReplicaMaintenanceConfig::default()
        })
        .expect("replica maintenance");
    let network = Arc::new(TestInMemoryNetwork::default());
    let mut runtime = NodeRuntime::new(config)
        .with_replication_network(NodeReplicationNetworkHandle::new(network));
    runtime.start().expect("start");
    thread::sleep(Duration::from_millis(120));
    runtime.stop().expect("stop");

    let snapshot = runtime.snapshot();
    assert!(snapshot.last_error.is_none(), "{:?}", snapshot.last_error);

    let _ = fs::remove_dir_all(replication_root);
}
