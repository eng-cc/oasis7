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

#[derive(Default)]
struct FinalCommitRaceControl {
    predecessor_staged_for_final_commit: AtomicBool,
    release_predecessor: AtomicBool,
    successor_committed: AtomicBool,
    durable_path: std::path::PathBuf,
    signal: Condvar,
    wait_lock: Mutex<()>,
}

struct FinalCommitRaceObserver {
    control: Arc<FinalCommitRaceControl>,
    generation: usize,
    authority: Option<ObserverLifecycleAuthority>,
}

impl NodeConsensusProgressObserver for FinalCommitRaceObserver {
    fn observe_consensus_progress(
        &mut self,
        _snapshot: &NodeConsensusSnapshot,
        _observed_at_ms: i64,
    ) -> Result<(), NodeConsensusProgressObserverError> {
        let authority = self
            .authority
            .as_ref()
            .expect("dispatcher must bind final-commit authority");
        let mutation = authority
            .acquire_durable_mutation()
            .expect("active generation must acquire durable mutation authority");
        let temp = self.control.durable_path.with_extension(if self.generation == 0 {
            "predecessor.tmp"
        } else {
            "successor.tmp"
        });
        fs::write(
            &temp,
            if self.generation == 0 {
                b"predecessor".as_slice()
            } else {
                b"successor".as_slice()
            },
        )
        .expect("write generation-specific temporary durable state");
        if self.generation == 0 {
            self.control
                .predecessor_staged_for_final_commit
                .store(true, Ordering::SeqCst);
            self.control.signal.notify_all();
            let mut wait = self
                .control
                .wait_lock
                .lock()
                .expect("lock predecessor final-commit pause");
            while !self.control.release_predecessor.load(Ordering::SeqCst) {
                wait = self
                    .control
                    .signal
                    .wait(wait)
                    .expect("wait to release predecessor final commit");
            }
        }
        mutation
            .publish_staged_file(&temp, &self.control.durable_path)
            .map_err(|error| {
                NodeConsensusProgressObserverError::coded(error.code(), error.to_string())
            })?;
        if self.generation > 0 {
            self.control
                .successor_committed
                .store(true, Ordering::SeqCst);
            self.control.signal.notify_all();
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

struct FinalCommitReleaseGuard(Arc<FinalCommitRaceControl>);

impl Drop for FinalCommitReleaseGuard {
    fn drop(&mut self) {
        self.0.release_predecessor.store(true, Ordering::SeqCst);
        self.0.signal.notify_all();
    }
}

#[test]
fn successor_publication_overtakes_paused_predecessor_final_commit_without_stale_overwrite() {
    run_test_with_timeout(
        "successor_publication_overtakes_paused_predecessor_final_commit_without_stale_overwrite",
        Duration::from_secs(4),
        || {
            let state = Arc::new(Mutex::new(
                super::super::node_runtime_core::RuntimeState::default(),
            ));
            let durable_root = temp_dir("paused-final-commit-red");
            fs::create_dir_all(&durable_root).expect("create durable publication state root");
            let durable_path = durable_root.join("sequencer-publication-lag-state.json");
            let control = Arc::new(FinalCommitRaceControl {
                durable_path: durable_path.clone(),
                ..FinalCommitRaceControl::default()
            });
            let release_guard = FinalCommitReleaseGuard(Arc::clone(&control));
            let mut predecessor = super::super::consensus_progress_observer::ConsensusProgressObserverDispatcher::spawn(
                "paused-final-commit",
                Arc::clone(&state),
                0,
                Box::new(FinalCommitRaceObserver {
                    control: Arc::clone(&control),
                    generation: 0,
                    authority: None,
                }),
            )
            .expect("spawn predecessor final-commit observer");
            predecessor
                .submitter()
                .submit(NodeConsensusSnapshot::default(), 1);
            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    control
                        .predecessor_staged_for_final_commit
                        .load(Ordering::SeqCst)
                }),
                "predecessor never staged publication after acquiring mutation authority"
            );

            let stop_started = Instant::now();
            let recreated = predecessor
                .shutdown()
                .expect("restartable observer must survive bounded shutdown");
            assert!(
                stop_started.elapsed() < Duration::from_millis(500),
                "bounded shutdown waited for paused predecessor final commit: elapsed={:?}",
                stop_started.elapsed()
            );
            let successor = super::super::consensus_progress_observer::ConsensusProgressObserverDispatcher::spawn(
                "paused-final-commit",
                Arc::clone(&state),
                1,
                recreated,
            )
            .expect("spawn successor final-commit observer");
            successor
                .submitter()
                .submit(NodeConsensusSnapshot::default(), 2);
            let successor_committed_before_release =
                wait_until(Instant::now() + Duration::from_millis(250), || {
                    control.successor_committed.load(Ordering::SeqCst)
                });

            release_guard
                .0
                .release_predecessor
                .store(true, Ordering::SeqCst);
            release_guard.0.signal.notify_all();
            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    control.successor_committed.load(Ordering::SeqCst)
                }),
                "successor never completed durable publication after predecessor release"
            );
            let final_state = fs::read_to_string(&durable_path)
                .expect("read final durable publication state after both generations");
            drop(successor);
            fs::remove_dir_all(durable_root).expect("remove durable publication state root");

            assert!(
                successor_committed_before_release,
                "successor publication blocked behind a predecessor holding mutation authority"
            );
            assert_eq!(
                final_state, "successor",
                "stale predecessor final commit overwrote successor durable publication state"
            );
        },
    );
}

#[test]
fn in_flight_only_restart_preserves_sequence_and_clears_backpressure_after_newest_delivery() {
    run_test_with_timeout(
        "in_flight_only_restart_preserves_sequence_and_clears_backpressure_after_newest_delivery",
        Duration::from_secs(4),
        || {
            let state = Arc::new(Mutex::new(
                super::super::node_runtime_core::RuntimeState::default(),
            ));
            let control = Arc::new(RestartSequenceControl::default());
            let mut predecessor = super::super::consensus_progress_observer::ConsensusProgressObserverDispatcher::spawn(
                "in-flight-only-sequence",
                Arc::clone(&state),
                0,
                Box::new(RestartSequenceObserver {
                    control: Arc::clone(&control),
                    generation: 0,
                }),
            )
            .expect("spawn in-flight-only predecessor");
            let predecessor_submitter = predecessor.submitter();
            predecessor_submitter.submit(NodeConsensusSnapshot::default(), 1);
            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    control
                        .observed
                        .lock()
                        .expect("lock predecessor sequence observation")
                        .len()
                        == 1
                }),
                "predecessor record never became the sole in-flight observation"
            );
            let predecessor_sequence = predecessor_submitter.current_sequence_for_test();

            let recreated = predecessor
                .shutdown()
                .expect("restartable observer must preserve sequence state");
            let successor = super::super::consensus_progress_observer::ConsensusProgressObserverDispatcher::spawn(
                "in-flight-only-sequence",
                Arc::clone(&state),
                1,
                recreated,
            )
            .expect("spawn in-flight-only successor");
            let successor_submitter = successor.submitter();
            successor_submitter.submit(NodeConsensusSnapshot::default(), 2);
            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    control
                        .observed
                        .lock()
                        .expect("lock successor sequence observation")
                        .len()
                        == 2
                }),
                "successor record never became in flight"
            );
            let successor_sequence = successor_submitter.current_sequence_for_test();

            successor_submitter.submit(NodeConsensusSnapshot::default(), 3);
            successor_submitter.submit(NodeConsensusSnapshot::default(), 4);
            successor_submitter.submit(NodeConsensusSnapshot::default(), 5);
            assert!(
                state
                    .lock()
                    .expect("lock saturated successor state")
                    .consensus_progress_observer_error
                    .as_deref()
                    .is_some_and(|error| error.contains(OBSERVER_BACKPRESSURE)),
                "successor coalescing did not report backpressure"
            );

            control.release_calls.store(2, Ordering::SeqCst);
            control.signal.notify_all();
            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    control
                        .observed
                        .lock()
                        .expect("lock intermediate successor observations")
                        .len()
                        >= 3
                }),
                "successor did not advance to retained pending delivery"
            );
            assert!(
                state
                    .lock()
                    .expect("lock intermediate successor state")
                    .consensus_progress_observer_error
                    .is_some(),
                "backpressure cleared before newest retained delivery"
            );

            control.release_calls.store(usize::MAX, Ordering::SeqCst);
            control.signal.notify_all();
            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    control
                        .observed
                        .lock()
                        .expect("lock completed successor observations")
                        .len()
                        == 4
                        && state
                            .lock()
                            .expect("lock completed successor state")
                            .consensus_progress_observer_error
                            .is_none()
                }),
                "newest retained successor delivery did not clear backpressure"
            );
            drop(successor);

            assert!(
                successor_sequence > predecessor_sequence,
                "restart reused observer sequence id: predecessor={predecessor_sequence} successor={successor_sequence}"
            );
        },
    );
}
