#[test]
fn failed_runtime_worker_spawn_restores_observer_for_retry() {
    run_test_with_timeout(
        "failed_runtime_worker_spawn_restores_observer_for_retry",
        Duration::from_secs(3),
        || {
            let control = Arc::new(RestartHandoffControl::default());
            let config = NodeConfig::new(
                "node-observer-start-retry",
                "world-observer-start-retry",
                NodeRole::Sequencer,
            )
            .expect("config")
            .with_tick_interval(Duration::from_secs(5))
            .expect("tick interval")
            .with_pos_config(signed_pos_config_with_signer_seeds(
                vec![PosValidator {
                    validator_id: "node-observer-start-retry".to_string(),
                    stake: 100,
                }],
                &[("node-observer-start-retry", 103)],
            ))
            .expect("pos config");
            let mut runtime = NodeRuntime::new(config).with_consensus_progress_observer(
                RestartHandoffObserver {
                    control: Arc::clone(&control),
                    blocks: false,
                    generation: 0,
                },
            );

            runtime.fail_next_worker_spawn_for_test();
            assert!(
                matches!(runtime.start(), Err(NodeError::ThreadSpawnFailed { .. })),
                "injected worker spawn failure must fail the first start"
            );
            runtime
                .start()
                .expect("observer must survive the failed start");

            let mut snapshot = NodeConsensusSnapshot::default();
            snapshot.committed_height = 1;
            runtime
                .submit_consensus_progress_for_test(snapshot, 1)
                .expect("submit progress after retried start");
            assert!(
                wait_until(Instant::now() + Duration::from_secs(1), || {
                    !control
                        .observed_progress
                        .lock()
                        .expect("lock retry observer progress")
                        .is_empty()
                }),
                "retried runtime lost the configured consensus progress observer"
            );
            runtime.stop().expect("stop retried runtime");

            assert_eq!(
                control
                    .observed_progress
                    .lock()
                    .expect("lock retry observer progress")[0]
                    .0,
                1,
                "failed start must restore the recreated observer generation"
            );
        },
    );
}
