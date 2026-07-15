use super::*;

use std::sync::Condvar;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const OBSERVER_FAILURE: &str = "injected consensus progress observer failure";
const OBSERVER_BACKPRESSURE: &str = "consensus progress observer queue saturated";
const STALE_GENERATION_FAILURE: &str = "stale observer generation failure";

#[derive(Clone)]
struct ControllableConsensusProgressObserver {
    calls: Arc<AtomicUsize>,
    should_fail: Arc<AtomicBool>,
}

impl NodeConsensusProgressObserver for ControllableConsensusProgressObserver {
    fn observe_consensus_progress(
        &mut self,
        _snapshot: &NodeConsensusSnapshot,
        _observed_at_ms: i64,
    ) -> Result<(), NodeConsensusProgressObserverError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.should_fail.load(Ordering::SeqCst) {
            Err(NodeConsensusProgressObserverError::coded(
                "state_persist_failed",
                OBSERVER_FAILURE,
            ))
        } else {
            Ok(())
        }
    }
}

#[derive(Default)]
struct BlockingObserverControl {
    entered: AtomicBool,
    release: AtomicBool,
    calls: AtomicUsize,
    observed_heights: Mutex<Vec<u64>>,
    signal: Condvar,
}

struct BlockingConsensusProgressObserver {
    control: Arc<BlockingObserverControl>,
}

impl NodeConsensusProgressObserver for BlockingConsensusProgressObserver {
    fn observe_consensus_progress(
        &mut self,
        snapshot: &NodeConsensusSnapshot,
        _observed_at_ms: i64,
    ) -> Result<(), NodeConsensusProgressObserverError> {
        self.control.calls.fetch_add(1, Ordering::SeqCst);
        let mut observed_heights = self
            .control
            .observed_heights
            .lock()
            .expect("lock blocking observer heights");
        observed_heights.push(snapshot.committed_height);
        if !self.control.entered.swap(true, Ordering::SeqCst) {
            self.control.signal.notify_all();
            while !self.control.release.load(Ordering::SeqCst) {
                observed_heights = self
                    .control
                    .signal
                    .wait(observed_heights)
                    .expect("wait for blocking observer release");
            }
        }
        Ok(())
    }
}

#[derive(Default)]
struct OrderedObserverControl {
    entered: AtomicBool,
    release: AtomicBool,
    observed_progress: Mutex<Vec<(u64, u64)>>,
    signal: Condvar,
}

struct OrderedBlockingConsensusProgressObserver {
    control: Arc<OrderedObserverControl>,
}

impl NodeConsensusProgressObserver for OrderedBlockingConsensusProgressObserver {
    fn observe_consensus_progress(
        &mut self,
        snapshot: &NodeConsensusSnapshot,
        _observed_at_ms: i64,
    ) -> Result<(), NodeConsensusProgressObserverError> {
        let mut observed_progress = self
            .control
            .observed_progress
            .lock()
            .expect("lock ordered observer progress");
        observed_progress.push((snapshot.committed_height, snapshot.network_committed_height));
        if !self.control.entered.swap(true, Ordering::SeqCst) {
            self.control.signal.notify_all();
            while !self.control.release.load(Ordering::SeqCst) {
                observed_progress = self
                    .control
                    .signal
                    .wait(observed_progress)
                    .expect("wait for ordered observer release");
            }
        }
        Ok(())
    }
}

#[derive(Default)]
struct RestartGenerationObserverControl {
    entered: AtomicBool,
    release: AtomicBool,
    calls: AtomicUsize,
    dropped: AtomicBool,
    signal: Condvar,
    wait_lock: Mutex<()>,
}

struct RestartGenerationConsensusProgressObserver {
    control: Arc<RestartGenerationObserverControl>,
}

impl NodeConsensusProgressObserver for RestartGenerationConsensusProgressObserver {
    fn observe_consensus_progress(
        &mut self,
        _snapshot: &NodeConsensusSnapshot,
        _observed_at_ms: i64,
    ) -> Result<(), NodeConsensusProgressObserverError> {
        let call = self.control.calls.fetch_add(1, Ordering::SeqCst);
        if call == 0 {
            let mut guard = self
                .control
                .wait_lock
                .lock()
                .expect("lock restart observer");
            self.control.entered.store(true, Ordering::SeqCst);
            self.control.signal.notify_all();
            while !self.control.release.load(Ordering::SeqCst) {
                guard = self
                    .control
                    .signal
                    .wait(guard)
                    .expect("wait for restart observer release");
            }
            return Err(STALE_GENERATION_FAILURE.into());
        }
        Ok(())
    }
}

impl Drop for RestartGenerationConsensusProgressObserver {
    fn drop(&mut self) {
        self.control.dropped.store(true, Ordering::SeqCst);
        self.control.signal.notify_all();
    }
}

struct RestartObserverReleaseGuard(Arc<RestartGenerationObserverControl>);

impl Drop for RestartObserverReleaseGuard {
    fn drop(&mut self) {
        self.0.release.store(true, Ordering::SeqCst);
        self.0.signal.notify_all();
    }
}

#[derive(Default)]
struct RestartHandoffControl {
    entered: AtomicBool,
    release: AtomicBool,
    observed_progress: Mutex<Vec<(usize, u64, u64)>>,
    signal: Condvar,
}

struct RestartHandoffObserver {
    control: Arc<RestartHandoffControl>,
    blocks: bool,
    generation: usize,
}

impl NodeConsensusProgressObserver for RestartHandoffObserver {
    fn observe_consensus_progress(
        &mut self,
        snapshot: &NodeConsensusSnapshot,
        _observed_at_ms: i64,
    ) -> Result<(), NodeConsensusProgressObserverError> {
        let mut observed = self
            .control
            .observed_progress
            .lock()
            .expect("lock restart handoff progress");
        observed.push((
            self.generation,
            snapshot.committed_height,
            snapshot.network_committed_height,
        ));
        if self.blocks && !self.control.entered.swap(true, Ordering::SeqCst) {
            self.control.signal.notify_all();
            while !self.control.release.load(Ordering::SeqCst) {
                observed = self
                    .control
                    .signal
                    .wait(observed)
                    .expect("wait for restart handoff release");
            }
        }
        Ok(())
    }

    fn recreate_for_restart(&self) -> Option<Box<dyn NodeConsensusProgressObserver>> {
        Some(Box::new(Self {
            control: Arc::clone(&self.control),
            blocks: false,
            generation: self.generation + 1,
        }))
    }
}

#[derive(Default)]
struct RestartAuthorityControl {
    entered: AtomicUsize,
    resumed: AtomicUsize,
    release: AtomicBool,
    next_owner: AtomicUsize,
    active_owner: AtomicUsize,
    mutation_owners: Mutex<Vec<usize>>,
    signal: Condvar,
    wait_lock: Mutex<()>,
}

struct RestartAuthorityObserver {
    control: Arc<RestartAuthorityControl>,
    owner: usize,
    authority: Option<ObserverLifecycleAuthority>,
}

impl NodeConsensusProgressObserver for RestartAuthorityObserver {
    fn observe_consensus_progress(
        &mut self,
        _snapshot: &NodeConsensusSnapshot,
        _observed_at_ms: i64,
    ) -> Result<(), NodeConsensusProgressObserverError> {
        if self.owner != self.control.active_owner.load(Ordering::SeqCst) {
            let mut guard = self.control.wait_lock.lock().expect("lock authority gate");
            self.control.entered.fetch_add(1, Ordering::SeqCst);
            self.control.signal.notify_all();
            while !self.control.release.load(Ordering::SeqCst) {
                guard = self
                    .control
                    .signal
                    .wait(guard)
                    .expect("wait for authority release");
            }
            self.control.resumed.fetch_add(1, Ordering::SeqCst);
        }
        let Some(authority) = self.authority.as_ref() else {
            return Err("missing observer lifecycle authority".into());
        };
        let Some(_mutation_guard) = authority.acquire_durable_mutation() else {
            return Ok(());
        };
        self.control
            .mutation_owners
            .lock()
            .expect("lock durable mutation owners")
            .push(self.owner);
        Ok(())
    }

    fn recreate_for_restart(&self) -> Option<Box<dyn NodeConsensusProgressObserver>> {
        Some(Box::new(Self {
            control: Arc::clone(&self.control),
            owner: self.control.next_owner.fetch_add(1, Ordering::SeqCst) + 1,
            authority: None,
        }))
    }

    fn bind_lifecycle_authority(&mut self, authority: ObserverLifecycleAuthority) {
        self.authority = Some(authority);
    }
}

#[derive(Default)]
struct RestartSequenceControl {
    release_calls: AtomicUsize,
    observed: Mutex<Vec<(usize, i64, u64, u64)>>,
    signal: Condvar,
}

struct RestartSequenceObserver {
    control: Arc<RestartSequenceControl>,
    generation: usize,
}

impl NodeConsensusProgressObserver for RestartSequenceObserver {
    fn observe_consensus_progress(
        &mut self,
        snapshot: &NodeConsensusSnapshot,
        observed_at_ms: i64,
    ) -> Result<(), NodeConsensusProgressObserverError> {
        let mut observed = self.control.observed.lock().expect("lock restart sequence");
        let call = observed.len();
        observed.push((
            self.generation,
            observed_at_ms,
            snapshot.committed_height,
            snapshot.network_committed_height,
        ));
        self.control.signal.notify_all();
        while self.control.release_calls.load(Ordering::SeqCst) <= call {
            observed = self
                .control
                .signal
                .wait(observed)
                .expect("wait for restart sequence release");
        }
        Ok(())
    }

    fn recreate_for_restart(&self) -> Option<Box<dyn NodeConsensusProgressObserver>> {
        Some(Box::new(Self {
            control: Arc::clone(&self.control),
            generation: self.generation + 1,
        }))
    }
}

#[derive(Default)]
struct CooperativeAuthorityControl {
    old_authority_acquired: AtomicBool,
    successor_entered: AtomicBool,
    release: AtomicBool,
    commits: Mutex<Vec<usize>>,
    signal: Condvar,
    wait_lock: Mutex<()>,
}

struct CooperativeAuthorityObserver {
    control: Arc<CooperativeAuthorityControl>,
    generation: usize,
    authority: Option<ObserverLifecycleAuthority>,
}

impl NodeConsensusProgressObserver for CooperativeAuthorityObserver {
    fn observe_consensus_progress(
        &mut self,
        _snapshot: &NodeConsensusSnapshot,
        _observed_at_ms: i64,
    ) -> Result<(), NodeConsensusProgressObserverError> {
        let authority = self
            .authority
            .as_ref()
            .expect("dispatcher must bind lifecycle authority");
        if self.generation == 0 {
            let guard = authority
                .acquire_durable_mutation()
                .expect("first generation must acquire durable authority");
            self.control
                .old_authority_acquired
                .store(true, Ordering::SeqCst);
            self.control.signal.notify_all();
            let mut wait = self
                .control
                .wait_lock
                .lock()
                .expect("lock old authority wait");
            while !self.control.release.load(Ordering::SeqCst) {
                wait = self
                    .control
                    .signal
                    .wait(wait)
                    .expect("wait for old authority release");
            }
            drop(wait);
            drop(guard);
        } else {
            self.control.successor_entered.store(true, Ordering::SeqCst);
            self.control.signal.notify_all();
            let mut wait = self
                .control
                .wait_lock
                .lock()
                .expect("lock successor authority wait");
            while !self.control.release.load(Ordering::SeqCst) {
                wait = self
                    .control
                    .signal
                    .wait(wait)
                    .expect("wait for successor authority release");
            }
        }

        if authority.acquire_durable_mutation().is_some() {
            self.control
                .commits
                .lock()
                .expect("lock durable commits")
                .push(self.generation);
        }
        Ok(())
    }

    fn recreate_for_restart(&self) -> Option<Box<dyn NodeConsensusProgressObserver>> {
        Some(Box::new(Self {
            control: Arc::clone(&self.control),
            generation: self.generation + 1,
            authority: None,
        }))
    }

    fn bind_lifecycle_authority(&mut self, authority: ObserverLifecycleAuthority) {
        self.authority = Some(authority);
    }
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
fn blocking_consensus_progress_observer_is_bounded_coalescing_and_never_stalls_tick() {
    run_test_with_timeout(
        "blocking_consensus_progress_observer_is_bounded_coalescing_and_never_stalls_tick",
        Duration::from_secs(8),
        || {
            let world_id = "world-blocking-consensus-progress-observer";
            let replication_root = temp_dir("blocking-consensus-progress-observer");
            let network = Arc::new(TestInMemoryNetwork::default());
            let dht = Arc::new(TestReplicaMaintenanceDht::new("source-a", "node-a"));
            let observer = Arc::new(BlockingObserverControl::default());
            let execution_calls = Arc::new(Mutex::new(Vec::new()));
            let config = NodeConfig::new("node-a", world_id, NodeRole::Sequencer)
                .expect("config")
                .with_tick_interval(Duration::from_millis(10))
                .expect("tick interval")
                .with_pos_config(signed_pos_config_with_signer_seeds(
                    vec![PosValidator {
                        validator_id: "node-a".to_string(),
                        stake: 100,
                    }],
                    &[("node-a", 101)],
                ))
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
                .with_consensus_progress_observer(BlockingConsensusProgressObserver {
                    control: Arc::clone(&observer),
                })
                .with_replication_network(NodeReplicationNetworkHandle::new(network.clone()))
                .with_replica_maintenance_dht(dht.clone());

            runtime
                .submit_consensus_action_payload(
                    1,
                    serde_cbor::to_vec(&serde_json::json!({"kind": "blocking-observer"}))
                        .expect("action payload"),
                )
                .expect("submit action");
            let batches_ready = runtime.committed_action_batches_handle();
            runtime.start().expect("start runtime");

            let entered = wait_until(Instant::now() + Duration::from_secs(2), || {
                observer.entered.load(Ordering::SeqCst)
            });
            assert!(
                entered,
                "observer never entered its deliberately blocking call"
            );
            let blocked_at = runtime.snapshot();
            let publications_at = retained_commit_publication_count(network.as_ref(), world_id);
            let maintenance_at = dht.published_records().len();
            let blocked_progress_observed =
                wait_until(Instant::now() + Duration::from_secs(2), || {
                    let snapshot = runtime.snapshot();
                    let persisted =
                        super::super::pos_state_store::PosNodeStateStore::from_replication(
                            config.replication.as_ref().expect("replication config"),
                        )
                        .load()
                        .ok()
                        .flatten();
                    snapshot.tick_count >= blocked_at.tick_count.saturating_add(5)
                        && snapshot.consensus.committed_height
                            >= blocked_at.consensus.committed_height.saturating_add(2)
                        && snapshot.consensus.last_execution_height
                            >= blocked_at.consensus.last_execution_height.saturating_add(2)
                        && retained_commit_publication_count(network.as_ref(), world_id)
                            > publications_at
                        && dht.published_records().len() > maintenance_at
                        && persisted.as_ref().is_some_and(|state| {
                            state.committed_height
                                >= blocked_at.consensus.committed_height.saturating_add(2)
                        })
                        && batches_ready.wait_for_batches(Duration::ZERO)
                        && !execution_calls
                            .lock()
                            .expect("lock execution calls")
                            .is_empty()
                });

            let while_blocked = runtime.snapshot();
            let publications_while_blocked =
                retained_commit_publication_count(network.as_ref(), world_id);
            let maintenance_while_blocked = dht.published_records().len();
            let persisted_while_blocked =
                super::super::pos_state_store::PosNodeStateStore::from_replication(
                    config.replication.as_ref().expect("replication config"),
                )
                .load()
                .ok()
                .flatten();
            let committed_batch_while_blocked = batches_ready.wait_for_batches(Duration::ZERO);
            let calls_while_blocked = observer.calls.load(Ordering::SeqCst);
            let latest_height_while_blocked = while_blocked.consensus.committed_height;

            observer.release.store(true, Ordering::SeqCst);
            observer.signal.notify_all();
            let recovered = wait_until(Instant::now() + Duration::from_secs(2), || {
                let snapshot = runtime.snapshot();
                observer
                    .observed_heights
                    .lock()
                    .expect("lock observed heights")
                    .last()
                    .is_some_and(|height| *height >= latest_height_while_blocked)
                    && snapshot.consensus_progress_observer_error.is_none()
            });
            let recovered_snapshot = runtime.snapshot();
            runtime.stop().expect("stop runtime");
            let observed_heights = observer
                .observed_heights
                .lock()
                .expect("lock observed heights")
                .clone();
            let total_observer_calls = observer.calls.load(Ordering::SeqCst);
            let execution_call_count = execution_calls.lock().expect("lock execution calls").len();
            let _ = fs::remove_dir_all(&replication_root);

            assert!(
                blocked_progress_observed,
                "blocking observer delayed tick/execution progress: blocked_at={blocked_at:?} while_blocked={while_blocked:?}",
            );
            assert!(
                committed_batch_while_blocked && execution_call_count > 0,
                "blocking observer delayed execution or committed-batch delivery: batch_ready={committed_batch_while_blocked} execution_calls={execution_call_count}",
            );
            assert!(
                publications_while_blocked > publications_at,
                "blocking observer delayed consensus rebroadcast: before={publications_at} during={publications_while_blocked}",
            );
            assert!(
                maintenance_while_blocked > maintenance_at,
                "blocking observer delayed replica maintenance: before={maintenance_at} during={maintenance_while_blocked}",
            );
            assert!(
                persisted_while_blocked.as_ref().is_some_and(|persisted| {
                    persisted.committed_height >= while_blocked.consensus.committed_height
                }),
                "blocking observer delayed engine save: persisted={persisted_while_blocked:?} while_blocked_height={}",
                while_blocked.consensus.committed_height,
            );
            assert!(
                while_blocked
                    .consensus_progress_observer_error
                    .as_deref()
                    .is_some_and(|error| error.contains(OBSERVER_BACKPRESSURE)),
                "bounded queue saturation must be visible through the dedicated observer error: {:?}",
                while_blocked.consensus_progress_observer_error,
            );
            assert_eq!(
                calls_while_blocked, 1,
                "a blocked observer must have at most one active invocation",
            );
            assert!(
                recovered && total_observer_calls >= 2,
                "observer queue was not bounded/coalescing or did not recover: recovered={recovered} calls={total_observer_calls} heights={observed_heights:?} snapshot={recovered_snapshot:?}",
            );
            assert!(
                observed_heights
                    .last()
                    .is_some_and(|height| { *height >= while_blocked.consensus.committed_height }),
                "coalescing observer did not receive the latest pending snapshot: heights={observed_heights:?} while_blocked={while_blocked:?}",
            );
            assert_eq!(recovered_snapshot.consensus_progress_observer_error, None);
        },
    );
}

#[test]
fn blocked_dispatcher_delivers_equal_head_between_lagging_snapshots_in_order() {
    run_test_with_timeout(
        "blocked_dispatcher_delivers_equal_head_between_lagging_snapshots_in_order",
        Duration::from_secs(3),
        || {
            let state = Arc::new(Mutex::new(
                super::super::node_runtime_core::RuntimeState::default(),
            ));
            let control = Arc::new(OrderedObserverControl::default());
            let dispatcher = super::super::consensus_progress_observer::ConsensusProgressObserverDispatcher::spawn(
                "ordered-publication-transition",
                Arc::clone(&state),
                0,
                Box::new(OrderedBlockingConsensusProgressObserver {
                    control: Arc::clone(&control),
                }),
            )
            .expect("spawn ordered observer dispatcher");
            let submitter = dispatcher.submitter();

            let mut lag = NodeConsensusSnapshot::default();
            lag.committed_height = 10;
            lag.network_committed_height = 9;
            let mut equal_head = lag.clone();
            equal_head.network_committed_height = 10;
            let mut fresh_lag = equal_head.clone();
            fresh_lag.committed_height = 11;
            fresh_lag.network_committed_height = 10;

            submitter.submit(lag, 1);
            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    control.entered.load(Ordering::SeqCst)
                }),
                "observer never became occupied by the first lagging snapshot"
            );
            submitter.submit(equal_head, 2);
            submitter.submit(fresh_lag, 3);
            control.release.store(true, Ordering::SeqCst);
            control.signal.notify_all();

            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    control
                        .observed_progress
                        .lock()
                        .expect("lock ordered observer progress")
                        .len()
                        == 3
                }),
                "dispatcher lost the equal-head catch-up transition while the observer was occupied: observed={:?}",
                control
                    .observed_progress
                    .lock()
                    .expect("lock ordered observer progress")
            );
            let observed = control
                .observed_progress
                .lock()
                .expect("lock ordered observer progress")
                .clone();
            drop(dispatcher);

            assert_eq!(
                observed,
                vec![(10, 9), (10, 10), (11, 10)],
                "publication lifecycle must receive lag -> equal-head -> fresh lag in order"
            );
        },
    );
}

#[test]
fn stopped_observer_generation_cannot_write_into_restarted_runtime() {
    run_test_with_timeout(
        "stopped_observer_generation_cannot_write_into_restarted_runtime",
        Duration::from_secs(8),
        || {
            let observer = Arc::new(RestartGenerationObserverControl::default());
            let release_guard = RestartObserverReleaseGuard(Arc::clone(&observer));
            let config = NodeConfig::new(
                "node-observer-restart-generation",
                "world-observer-restart-generation",
                NodeRole::Sequencer,
            )
            .expect("config")
            .with_tick_interval(Duration::from_millis(10))
            .expect("tick interval")
            .with_pos_config(signed_pos_config_with_signer_seeds(
                vec![PosValidator {
                    validator_id: "node-observer-restart-generation".to_string(),
                    stake: 100,
                }],
                &[("node-observer-restart-generation", 101)],
            ))
            .expect("pos config");
            let mut runtime = NodeRuntime::new(config).with_consensus_progress_observer(
                RestartGenerationConsensusProgressObserver {
                    control: Arc::clone(&observer),
                },
            );

            runtime.start().expect("start first generation");
            assert!(
                wait_until(Instant::now() + Duration::from_secs(2), || {
                    observer.entered.load(Ordering::SeqCst)
                }),
                "first-generation observer never entered"
            );
            assert!(
                wait_until(Instant::now() + Duration::from_secs(2), || {
                    runtime
                        .snapshot()
                        .consensus_progress_observer_error
                        .as_deref()
                        == Some(OBSERVER_BACKPRESSURE)
                }),
                "first generation never accumulated a pending/coalesced observer snapshot"
            );

            let stop_started = Instant::now();
            runtime.stop().expect("bounded first-generation stop");
            let first_stop_elapsed = stop_started.elapsed();
            assert!(
                first_stop_elapsed < Duration::from_millis(500),
                "stop joined a blocked observer: elapsed={first_stop_elapsed:?}"
            );

            // Keep the restarted generation quiescent while the old observer returns. Any second
            // observer call therefore came from pending work that stop was required to discard.
            runtime.config.tick_interval = Duration::from_secs(5);
            runtime.start().expect("start second generation");
            let restarted_before_release = runtime.snapshot();
            assert_eq!(
                restarted_before_release.consensus_progress_observer_error, None,
                "restart must begin with generation-local observer state"
            );

            release_guard.0.release.store(true, Ordering::SeqCst);
            release_guard.0.signal.notify_all();
            let old_worker_released = wait_until(Instant::now() + Duration::from_secs(2), || {
                observer.dropped.load(Ordering::SeqCst)
            });
            let restarted_after_release = runtime.snapshot();
            let calls_after_release = observer.calls.load(Ordering::SeqCst);
            runtime.stop().expect("stop second generation");
            drop(release_guard);

            assert!(
                old_worker_released,
                "stopped observer worker/resource remained alive after its bounded call returned"
            );
            assert_eq!(
                calls_after_release, 1,
                "stop did not clear old-generation pending observer work"
            );
            assert_eq!(
                restarted_after_release.consensus_progress_observer_error, None,
                "stale observer result changed the restarted generation: {restarted_after_release:?}"
            );
        },
    );
}

#[test]
fn bounded_stop_hands_pending_lag_equal_and_fresh_lag_to_recreated_observer_in_order() {
    run_test_with_timeout(
        "bounded_stop_hands_pending_lag_equal_and_fresh_lag_to_recreated_observer_in_order",
        Duration::from_secs(3),
        || {
            let state = Arc::new(Mutex::new(
                super::super::node_runtime_core::RuntimeState::default(),
            ));
            let control = Arc::new(RestartHandoffControl::default());
            let mut dispatcher = super::super::consensus_progress_observer::ConsensusProgressObserverDispatcher::spawn(
                "restart-publication-handoff",
                Arc::clone(&state),
                0,
                Box::new(RestartHandoffObserver {
                    control: Arc::clone(&control),
                    blocks: true,
                    generation: 0,
                }),
            )
            .expect("spawn blocked restart dispatcher");
            let submitter = dispatcher.submitter();

            let mut lag = NodeConsensusSnapshot::default();
            lag.committed_height = 10;
            lag.network_committed_height = 9;
            let mut equal_head = lag.clone();
            equal_head.network_committed_height = 10;
            let mut fresh_lag = equal_head.clone();
            fresh_lag.committed_height = 11;
            fresh_lag.network_committed_height = 10;

            submitter.submit(lag, 1);
            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    control.entered.load(Ordering::SeqCst)
                }),
                "first-generation observer never became occupied"
            );
            submitter.submit(equal_head, 2);
            submitter.submit(fresh_lag, 3);

            let stop_started = Instant::now();
            let recreated = dispatcher
                .shutdown()
                .expect("restartable observer must survive bounded stop");
            assert!(
                stop_started.elapsed() < Duration::from_millis(500),
                "bounded stop joined a blocked observer: elapsed={:?}",
                stop_started.elapsed()
            );
            let restarted = super::super::consensus_progress_observer::ConsensusProgressObserverDispatcher::spawn(
                "restart-publication-handoff",
                Arc::clone(&state),
                1,
                recreated,
            )
            .expect("spawn recreated observer");

            control.release.store(true, Ordering::SeqCst);
            control.signal.notify_all();
            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    control
                        .observed_progress
                        .lock()
                        .expect("lock restart handoff progress")
                        .len()
                        == 3
                }),
                "bounded restart discarded pending publication transitions instead of handing them to the recreated observer: observed={:?}",
                control
                    .observed_progress
                    .lock()
                    .expect("lock restart handoff progress")
            );
            assert_eq!(
                *control
                    .observed_progress
                    .lock()
                    .expect("lock restart handoff progress"),
                vec![(0, 10, 9), (1, 10, 10), (1, 11, 10)],
                "recreated observer must receive pending equal-head -> fresh lag in order after the original lag"
            );
            drop(restarted);
        },
    );
}

#[test]
fn restart_handoff_keeps_sequences_monotonic_and_backpressure_until_newest_delivery() {
    run_test_with_timeout(
        "restart_handoff_keeps_sequences_monotonic_and_backpressure_until_newest_delivery",
        Duration::from_secs(4),
        || {
            let state = Arc::new(Mutex::new(
                super::super::node_runtime_core::RuntimeState::default(),
            ));
            let control = Arc::new(RestartSequenceControl::default());
            let mut dispatcher = super::super::consensus_progress_observer::ConsensusProgressObserverDispatcher::spawn(
                "restart-sequence-continuity",
                Arc::clone(&state),
                0,
                Box::new(RestartSequenceObserver {
                    control: Arc::clone(&control),
                    generation: 0,
                }),
            )
            .expect("spawn first restart sequence dispatcher");
            let first_submitter = dispatcher.submitter();

            let mut lag = NodeConsensusSnapshot::default();
            lag.committed_height = 10;
            lag.network_committed_height = 9;
            let mut equal_head = lag.clone();
            equal_head.network_committed_height = 10;
            let mut inherited_lag = equal_head.clone();
            inherited_lag.committed_height = 11;

            first_submitter.submit(lag, 1);
            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    control
                        .observed
                        .lock()
                        .expect("lock first restart sequence observation")
                        .len()
                        == 1
                }),
                "first generation never blocked after its first sequence"
            );
            first_submitter.submit(equal_head, 2);
            first_submitter.submit(inherited_lag.clone(), 3);

            let recreated = dispatcher
                .shutdown()
                .expect("restartable observer must preserve inherited pending work");
            let restarted = super::super::consensus_progress_observer::ConsensusProgressObserverDispatcher::spawn(
                "restart-sequence-continuity",
                Arc::clone(&state),
                1,
                recreated,
            )
            .expect("spawn restarted sequence dispatcher");
            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    control
                        .observed
                        .lock()
                        .expect("lock inherited restart sequence observation")
                        .len()
                        == 2
                }),
                "restarted dispatcher did not begin inherited pending delivery"
            );

            let restart_submitter = restarted.submitter();
            let mut post_restart_equal = inherited_lag.clone();
            post_restart_equal.network_committed_height = 11;
            let mut newest_lag = post_restart_equal.clone();
            newest_lag.committed_height = 12;
            restart_submitter.submit(post_restart_equal, 4);
            restart_submitter.submit(newest_lag, 5);
            assert!(
                state
                    .lock()
                    .expect("lock restart sequence runtime state")
                    .consensus_progress_observer_error
                    .as_deref()
                    .is_some_and(|error| error.contains(OBSERVER_BACKPRESSURE)),
                "post-restart coalescing must report backpressure before inherited delivery resumes"
            );

            control.release_calls.store(2, Ordering::SeqCst);
            control.signal.notify_all();
            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    control
                        .observed
                        .lock()
                        .expect("lock post-restart sequence observation")
                        .len()
                        == 3
                }),
                "restarted dispatcher did not continue into coalesced post-restart delivery"
            );
            assert!(
                state
                    .lock()
                    .expect("lock restart sequence runtime state")
                    .consensus_progress_observer_error
                    .as_deref()
                    .is_some_and(|error| error.contains(OBSERVER_BACKPRESSURE)),
                "backpressure cleared before the newest post-restart sequence completed"
            );

            control.release_calls.store(4, Ordering::SeqCst);
            control.signal.notify_all();
            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    control
                        .observed
                        .lock()
                        .expect("lock final restart sequence observation")
                        .len()
                        == 4
                        && state
                            .lock()
                            .expect("lock final restart sequence state")
                            .consensus_progress_observer_error
                            .is_none()
                }),
                "newest post-restart sequence did not complete and clear backpressure"
            );
            let observed = control
                .observed
                .lock()
                .expect("lock completed restart sequence observation")
                .clone();
            drop(restarted);

            assert_eq!(
                observed,
                vec![
                    (0, 1, 10, 9),
                    (1, 2, 10, 10),
                    (1, 4, 11, 11),
                    (1, 5, 12, 11)
                ],
                "restart handoff and immediate coalescing must deliver strictly increasing observation sequences without replaying stale pending work"
            );
        },
    );
}

#[test]
fn cooperative_durable_authority_holder_does_not_block_restart_or_commit_after_revocation() {
    run_test_with_timeout(
        "cooperative_durable_authority_holder_does_not_block_restart_or_commit_after_revocation",
        Duration::from_secs(4),
        || {
            let state = Arc::new(Mutex::new(
                super::super::node_runtime_core::RuntimeState::default(),
            ));
            let control = Arc::new(CooperativeAuthorityControl::default());
            let mut dispatcher = super::super::consensus_progress_observer::ConsensusProgressObserverDispatcher::spawn(
                "cooperative-authority-restart",
                Arc::clone(&state),
                0,
                Box::new(CooperativeAuthorityObserver {
                    control: Arc::clone(&control),
                    generation: 0,
                    authority: None,
                }),
            )
            .expect("spawn durable authority holder");
            dispatcher
                .submitter()
                .submit(NodeConsensusSnapshot::default(), 1);
            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    control.old_authority_acquired.load(Ordering::SeqCst)
                }),
                "old observer never acquired durable authority before its cooperative block"
            );

            let (handoff_tx, handoff_rx) = std::sync::mpsc::sync_channel(1);
            std::thread::spawn(move || {
                let _ = handoff_tx.send(dispatcher.shutdown());
            });
            let recreated = match handoff_rx.recv_timeout(Duration::from_millis(500)) {
                Ok(Some(observer)) => observer,
                Ok(None) => panic!("restartable observer was not returned from shutdown"),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    control.release.store(true, Ordering::SeqCst);
                    control.signal.notify_all();
                    let _ = handoff_rx.recv_timeout(Duration::from_secs(1));
                    panic!(
                        "bounded dispatcher shutdown waited for a cooperative observer holding durable authority"
                    );
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    panic!("shutdown worker disconnected before handing off observer")
                }
            };
            let restarted = super::super::consensus_progress_observer::ConsensusProgressObserverDispatcher::spawn(
                "cooperative-authority-restart",
                Arc::clone(&state),
                1,
                recreated,
            )
            .expect("successor activation must not wait for old durable authority release");
            restarted
                .submitter()
                .submit(NodeConsensusSnapshot::default(), 2);
            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    control.successor_entered.load(Ordering::SeqCst)
                }),
                "successor activation blocked behind the old authority holder"
            );

            control.release.store(true, Ordering::SeqCst);
            control.signal.notify_all();
            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    *control.commits.lock().expect("lock durable commits") == vec![1]
                }),
                "revoked old generation committed or successor did not commit after release: commits={:?}",
                control.commits.lock().expect("lock durable commits")
            );
            drop(restarted);
        },
    );
}

#[test]
fn repeated_bounded_restarts_fence_old_durable_mutation_owners() {
    run_test_with_timeout(
        "repeated_bounded_restarts_fence_old_durable_mutation_owners",
        Duration::from_secs(4),
        || {
            let state = Arc::new(Mutex::new(
                super::super::node_runtime_core::RuntimeState::default(),
            ));
            let control = Arc::new(RestartAuthorityControl::default());
            control.active_owner.store(usize::MAX, Ordering::SeqCst);
            let mut observer: Box<dyn NodeConsensusProgressObserver> =
                Box::new(RestartAuthorityObserver {
                    control: Arc::clone(&control),
                    owner: 0,
                    authority: None,
                });

            for generation in 0..3_u64 {
                let mut dispatcher = super::super::consensus_progress_observer::ConsensusProgressObserverDispatcher::spawn(
                    "restart-publication-authority",
                    Arc::clone(&state),
                    generation,
                    observer,
                )
                .expect("spawn restart generation");
                dispatcher
                    .submitter()
                    .submit(NodeConsensusSnapshot::default(), generation as i64);
                assert!(
                    wait_until(Instant::now() + Duration::from_secs(1), || {
                        control.entered.load(Ordering::SeqCst) == generation as usize + 1
                    }),
                    "generation {generation} never reached the controlled durable mutation gate"
                );
                let stop_started = Instant::now();
                observer = dispatcher
                    .shutdown()
                    .expect("restartable observer must remain available after bounded stop");
                assert!(
                    stop_started.elapsed() < Duration::from_millis(500),
                    "restart generation {generation} exceeded bounded stop: elapsed={:?}",
                    stop_started.elapsed()
                );
            }

            control.active_owner.store(3, Ordering::SeqCst);
            let current = super::super::consensus_progress_observer::ConsensusProgressObserverDispatcher::spawn(
                "restart-publication-authority",
                Arc::clone(&state),
                3,
                observer,
            )
            .expect("spawn current generation");
            current
                .submitter()
                .submit(NodeConsensusSnapshot::default(), 3);
            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    control
                        .mutation_owners
                        .lock()
                        .expect("lock durable mutation owners")
                        .contains(&3)
                }),
                "current generation never acquired durable mutation authority"
            );

            control.release.store(true, Ordering::SeqCst);
            control.signal.notify_all();
            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    control.resumed.load(Ordering::SeqCst) == 3
                }),
                "old blocked observers did not resume for the authority-fence regression"
            );
            let owners = control
                .mutation_owners
                .lock()
                .expect("lock durable mutation owners")
                .clone();
            drop(current);

            assert_eq!(
                owners,
                vec![3],
                "after new ownership begins, old generations must not mutate durable lifecycle state or accumulate mutation owners"
            );
        },
    );
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
            let observer_should_fail = Arc::new(AtomicBool::new(true));
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
                .with_consensus_progress_observer(ControllableConsensusProgressObserver {
                    calls: Arc::clone(&observer_calls),
                    should_fail: Arc::clone(&observer_should_fail),
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
            let failing_snapshot = runtime.snapshot();
            assert_eq!(
                failing_snapshot
                    .consensus_progress_observer_error
                    .as_deref(),
                Some(OBSERVER_FAILURE),
                "observer failure must remain in the dedicated field",
            );
            assert_eq!(
                failing_snapshot
                    .consensus_progress_observer_error
                    .as_ref()
                    .and_then(|error| error.code.as_deref()),
                Some("state_persist_failed"),
                "observer failure code must survive the asynchronous runtime boundary",
            );
            assert!(
                failing_snapshot.last_error.is_none(),
                "observer failure must not become a NodeRuntime tick failure: {:?}",
                failing_snapshot.last_error,
            );

            observer_should_fail.store(false, Ordering::SeqCst);
            let recovered_in_process = wait_until(Instant::now() + Duration::from_secs(2), || {
                let snapshot = runtime.snapshot();
                snapshot.consensus.committed_height > failing_snapshot.consensus.committed_height
                    && snapshot.consensus.last_execution_height
                        > failing_snapshot.consensus.last_execution_height
                    && snapshot.consensus_progress_observer_error.is_none()
            });
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
                recovered_in_process,
                "successful observer call did not clear the dedicated error while progress continued: failing_snapshot={failing_snapshot:?} running_snapshot={running_snapshot:?}",
            );
            assert_eq!(
                running_snapshot.consensus_progress_observer_error, None,
                "successful observer call must clear the dedicated error in the same process",
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
            assert_eq!(running_snapshot.last_error, None);
        },
    );
}
