use super::*;

use std::sync::atomic::{AtomicUsize, Ordering};

const OBSERVER_FAILURE: &str = "injected consensus progress observer failure";

#[derive(Clone)]
struct AlwaysFailingConsensusProgressObserver {
    calls: Arc<AtomicUsize>,
}

impl NodeConsensusProgressObserver for AlwaysFailingConsensusProgressObserver {
    fn observe_consensus_progress(
        &mut self,
        _snapshot: &NodeConsensusSnapshot,
        _observed_at_ms: i64,
    ) -> Result<(), String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(OBSERVER_FAILURE.to_string())
    }
}

fn snapshot_has_dedicated_consensus_progress_observer_error(snapshot: &NodeSnapshot) -> bool {
    let rendered = format!("{snapshot:?}");
    rendered.contains("consensus_progress_observer_error: Some(")
        && rendered.contains(OBSERVER_FAILURE)
}

fn retained_commit_publication_count(network: &TestInMemoryNetwork, world_id: &str) -> usize {
    let topic = super::super::network_bridge::default_consensus_commit_topic(world_id);
    network
        .retained
        .lock()
        .expect("lock retained consensus publications")
        .get(topic.as_str())
        .map(Vec::len)
        .unwrap_or_default()
}

#[test]
fn consensus_progress_observer_failure_is_non_fatal_observable_and_recoverable() {
    run_test_with_timeout(
        "consensus_progress_observer_failure_is_non_fatal_observable_and_recoverable",
        Duration::from_secs(8),
        || {
            let world_id = "world-consensus-progress-observer-failure";
            let replication_root = temp_dir("consensus-progress-observer-failure");
            let network = Arc::new(TestInMemoryNetwork::default());
            let dht = Arc::new(TestReplicaMaintenanceDht::new("source-a", "node-a"));
            let observer_calls = Arc::new(AtomicUsize::new(0));
            let execution_calls = Arc::new(Mutex::new(Vec::new()));
            let pos_config = signed_pos_config_with_signer_seeds(
                vec![PosValidator {
                    validator_id: "node-a".to_string(),
                    stake: 100,
                }],
                &[("node-a", 101)],
            );
            let config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
                .expect("config")
                .with_tick_interval(Duration::from_millis(10))
                .expect("tick interval")
                .with_pos_config(pos_config)
                .expect("pos config")
                .with_replication(signed_replication_config(replication_root.clone(), 101))
                .with_replica_maintenance(NodeReplicaMaintenanceConfig {
                    max_content_hash_samples_per_round: 2,
                    target_replicas_per_blob: 2,
                    max_repairs_per_round: 2,
                    max_rebalances_per_round: 0,
                    poll_interval_ms: 10,
                    ..NodeReplicaMaintenanceConfig::default()
                })
                .expect("replica maintenance");
            let mut runtime = NodeRuntime::new(config.clone())
                .with_execution_hook(RecordingExecutionHook::new(Arc::clone(&execution_calls)))
                .with_consensus_progress_observer(AlwaysFailingConsensusProgressObserver {
                    calls: Arc::clone(&observer_calls),
                })
                .with_replication_network(NodeReplicationNetworkHandle::new(network.clone()))
                .with_replica_maintenance_dht(dht.clone());

            let action_payload =
                serde_cbor::to_vec(&serde_json::json!({"kind": "observer-failure"}))
                    .expect("action payload");
            runtime
                .submit_consensus_action_payload(1, action_payload)
                .expect("submit action");
            let batches_ready = runtime.committed_action_batches_handle();

            runtime
                .start()
                .expect("start runtime with failing observer");
            let progress_continued = wait_until(Instant::now() + Duration::from_secs(2), || {
                let snapshot = runtime.snapshot();
                snapshot.consensus.committed_height >= 2
                    && snapshot.consensus.last_execution_height >= 2
                    && observer_calls.load(Ordering::SeqCst) >= 2
                    && retained_commit_publication_count(network.as_ref(), world_id) >= 2
                    && !dht.published_records().is_empty()
            });
            let committed_batch_delivered = batches_ready.wait_for_batches(Duration::from_secs(1));
            let running_snapshot = runtime.snapshot();
            runtime.stop().expect("stop runtime with failing observer");
            let committed_batches = runtime.drain_committed_action_batches();

            let persisted = super::super::pos_state_store::PosNodeStateStore::from_replication(
                config.replication.as_ref().expect("replication config"),
            )
            .load()
            .expect("load persisted PoS state")
            .expect("persisted PoS state");

            let recovery_execution_calls = Arc::new(Mutex::new(Vec::new()));
            let mut recovered_runtime = NodeRuntime::new(config)
                .with_execution_hook(RecordingExecutionHook::new(Arc::clone(
                    &recovery_execution_calls,
                )))
                .with_replication_network(NodeReplicationNetworkHandle::new(network.clone()))
                .with_replica_maintenance_dht(dht.clone());
            recovered_runtime
                .start()
                .expect("restart recovered runtime");
            let recovered = wait_until(Instant::now() + Duration::from_secs(2), || {
                recovered_runtime.snapshot().consensus.committed_height > persisted.committed_height
            });
            let recovered_snapshot = recovered_runtime.snapshot();
            recovered_runtime.stop().expect("stop recovered runtime");

            let delivered_action_ids = committed_batches
                .iter()
                .flat_map(|batch| batch.actions.iter().map(|action| action.action_id))
                .collect::<Vec<_>>();
            let execution_call_count = execution_calls.lock().expect("lock execution calls").len();

            let _ = fs::remove_dir_all(&replication_root);

            assert!(
                progress_continued,
                "observer failure aborted runtime progress: snapshot={running_snapshot:?} observer_calls={} execution_calls={} committed_batch_delivered={committed_batch_delivered} delivered_action_ids={delivered_action_ids:?} commit_publications={} maintenance_publications={} persisted_height={} recovered={recovered} recovered_height={}",
                observer_calls.load(Ordering::SeqCst),
                execution_call_count,
                retained_commit_publication_count(network.as_ref(), world_id),
                dht.published_records().len(),
                persisted.committed_height,
                recovered_snapshot.consensus.committed_height,
            );
            assert!(
                committed_batch_delivered && delivered_action_ids.contains(&1),
                "observer failure blocked committed-batch delivery: ready={committed_batch_delivered} action_ids={delivered_action_ids:?}"
            );
            assert!(
                persisted.committed_height >= running_snapshot.consensus.committed_height,
                "observer failure blocked PoS state save: runtime_height={} persisted_height={}",
                running_snapshot.consensus.committed_height,
                persisted.committed_height,
            );
            assert!(
                recovered
                    && recovered_snapshot.consensus.committed_height > persisted.committed_height,
                "runtime did not recover from observer failure persistence: persisted_height={} recovered_snapshot={recovered_snapshot:?}",
                persisted.committed_height,
            );
            assert!(
                running_snapshot.last_error.is_none(),
                "observer failure must not become a NodeRuntime tick failure: {:?}",
                running_snapshot.last_error,
            );
            assert!(
                snapshot_has_dedicated_consensus_progress_observer_error(&running_snapshot),
                "observer failure must remain in dedicated observable field for critical readiness classification: {running_snapshot:?}",
            );
        },
    );
}
