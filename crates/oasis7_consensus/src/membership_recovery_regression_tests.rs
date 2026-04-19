#[test]
fn run_revocation_reconcile_coordinated_with_ack_retry_handles_defer_and_drop() {
    let client = sample_client();
    let subscription = client.subscribe("w1").expect("subscribe");

    let mut local_keyring = sample_keyring();
    local_keyring
        .add_hmac_sha256_key("k2", "membership-secret-v2")
        .expect("add local k2");

    let mut remote_keyring = MembershipDirectorySignerKeyring::new();
    remote_keyring
        .add_hmac_sha256_key("k2", "membership-secret-v2")
        .expect("add remote k2");
    assert!(remote_keyring.revoke_key("k2").expect("revoke remote k2"));

    client
        .publish_revocation_checkpoint("w1", "node-a", 1300, &remote_keyring)
        .expect("publish remote checkpoint");

    let reconcile_policy = MembershipRevocationReconcilePolicy {
        trusted_nodes: vec!["node-a".to_string()],
        auto_revoke_missing_keys: false,
    };
    let schedule_policy = MembershipRevocationReconcileSchedulePolicy {
        checkpoint_interval_ms: 1000,
        reconcile_interval_ms: 300,
    };
    let alert_policy = MembershipRevocationAlertPolicy {
        warn_diverged_threshold: 1,
        critical_rejected_threshold: 1,
    };
    let retry_policy = MembershipRevocationAlertAckRetryPolicy {
        max_pending_alerts: 4,
        max_retry_attempts: 2,
        retry_backoff_ms: 100,
    };

    let schedule_store = InMemoryMembershipRevocationScheduleStateStore::new();
    schedule_store
        .save(
            "w1",
            "node-b",
            &MembershipRevocationReconcileScheduleState {
                last_checkpoint_at_ms: Some(1300),
                last_reconcile_at_ms: Some(1000),
            },
        )
        .expect("seed schedule state");

    let alert_sink = AlwaysFailAlertSink::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let coordinator_store: Arc<dyn MembershipRevocationCoordinatorStateStore + Send + Sync> =
        Arc::new(InMemoryMembershipRevocationCoordinatorStateStore::new());
    let coordinator = StoreBackedMembershipRevocationScheduleCoordinator::new(coordinator_store);

    let first = client
        .run_revocation_reconcile_coordinated_with_recovery_and_ack_retry(
            "w1",
            "node-b",
            1305,
            &subscription,
            &mut local_keyring,
            &reconcile_policy,
            &schedule_policy,
            &alert_policy,
            None,
            None,
            &schedule_store,
            &alert_sink,
            &recovery_store,
            &retry_policy,
            &coordinator,
            1000,
        )
        .expect("first run");
    assert!(first.acquired);
    assert_eq!(first.buffered_alerts, 1);
    assert_eq!(first.deferred_alerts, 0);
    assert_eq!(first.dropped_alerts_retry_limit, 0);

    let second = client
        .run_revocation_reconcile_coordinated_with_recovery_and_ack_retry(
            "w1",
            "node-b",
            1310,
            &subscription,
            &mut local_keyring,
            &reconcile_policy,
            &schedule_policy,
            &alert_policy,
            None,
            None,
            &schedule_store,
            &alert_sink,
            &recovery_store,
            &retry_policy,
            &coordinator,
            1000,
        )
        .expect("second run");
    assert!(second.acquired);
    assert_eq!(second.buffered_alerts, 1);
    assert_eq!(second.deferred_alerts, 1);
    assert_eq!(second.dropped_alerts_retry_limit, 0);

    let third = client
        .run_revocation_reconcile_coordinated_with_recovery_and_ack_retry(
            "w1",
            "node-b",
            1405,
            &subscription,
            &mut local_keyring,
            &reconcile_policy,
            &schedule_policy,
            &alert_policy,
            None,
            None,
            &schedule_store,
            &alert_sink,
            &recovery_store,
            &retry_policy,
            &coordinator,
            1000,
        )
        .expect("third run");
    assert!(third.acquired);
    assert_eq!(third.buffered_alerts, 0);
    assert_eq!(third.deferred_alerts, 0);
    assert_eq!(third.dropped_alerts_retry_limit, 1);

    let pending = recovery_store
        .load_pending("w1", "node-b")
        .expect("load pending");
    assert!(pending.is_empty());
}

#[test]
fn run_revocation_reconcile_coordinated_with_dead_letter_archives_retry_drop() {
    let client = sample_client();
    let subscription = client.subscribe("w1").expect("subscribe");

    let mut local_keyring = sample_keyring();
    local_keyring
        .add_hmac_sha256_key("k2", "membership-secret-v2")
        .expect("add local k2");

    let mut remote_keyring = MembershipDirectorySignerKeyring::new();
    remote_keyring
        .add_hmac_sha256_key("k2", "membership-secret-v2")
        .expect("add remote k2");
    assert!(remote_keyring.revoke_key("k2").expect("revoke remote k2"));

    client
        .publish_revocation_checkpoint("w1", "node-a", 1300, &remote_keyring)
        .expect("publish remote checkpoint");

    let reconcile_policy = MembershipRevocationReconcilePolicy {
        trusted_nodes: vec!["node-a".to_string()],
        auto_revoke_missing_keys: false,
    };
    let schedule_policy = MembershipRevocationReconcileSchedulePolicy {
        checkpoint_interval_ms: 1000,
        reconcile_interval_ms: 300,
    };
    let alert_policy = MembershipRevocationAlertPolicy {
        warn_diverged_threshold: 1,
        critical_rejected_threshold: 1,
    };
    let retry_policy = MembershipRevocationAlertAckRetryPolicy {
        max_pending_alerts: 8,
        max_retry_attempts: 1,
        retry_backoff_ms: 0,
    };

    let schedule_store = InMemoryMembershipRevocationScheduleStateStore::new();
    schedule_store
        .save(
            "w1",
            "node-b",
            &MembershipRevocationReconcileScheduleState {
                last_checkpoint_at_ms: Some(1300),
                last_reconcile_at_ms: Some(1000),
            },
        )
        .expect("seed schedule state");

    let alert_sink = AlwaysFailAlertSink::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let coordinator_store: Arc<dyn MembershipRevocationCoordinatorStateStore + Send + Sync> =
        Arc::new(InMemoryMembershipRevocationCoordinatorStateStore::new());
    let coordinator = StoreBackedMembershipRevocationScheduleCoordinator::new(coordinator_store);

    let report = client
        .run_revocation_reconcile_coordinated_with_recovery_and_ack_retry_with_dead_letter(
            "w1",
            "node-b",
            1305,
            &subscription,
            &mut local_keyring,
            &reconcile_policy,
            &schedule_policy,
            &alert_policy,
            None,
            None,
            &schedule_store,
            &alert_sink,
            &recovery_store,
            &retry_policy,
            &dead_letter_store,
            &coordinator,
            1000,
        )
        .expect("run with dead letter");

    assert!(report.acquired);
    assert_eq!(report.buffered_alerts, 0);
    assert_eq!(report.dropped_alerts_retry_limit, 1);
    assert_eq!(report.delivery_metrics.attempted, 1);
    assert_eq!(report.delivery_metrics.failed, 1);
    assert_eq!(report.delivery_metrics.dead_lettered, 1);

    let pending = recovery_store
        .load_pending("w1", "node-b")
        .expect("load pending");
    assert!(pending.is_empty());

    let dead_letters = dead_letter_store
        .list("w1", "node-b")
        .expect("list dead letters");
    assert_eq!(dead_letters.len(), 1);
    assert_eq!(
        dead_letters[0].reason,
        MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded
    );
}

#[test]
fn run_revocation_reconcile_with_metrics_export_appends_metric_line() {
    let client = sample_client();
    let subscription = client.subscribe("w1").expect("subscribe");

    let mut local_keyring = sample_keyring();
    local_keyring
        .add_hmac_sha256_key("k2", "membership-secret-v2")
        .expect("add local k2");

    let mut remote_keyring = MembershipDirectorySignerKeyring::new();
    remote_keyring
        .add_hmac_sha256_key("k2", "membership-secret-v2")
        .expect("add remote k2");
    assert!(remote_keyring.revoke_key("k2").expect("revoke remote k2"));

    client
        .publish_revocation_checkpoint("w1", "node-a", 1300, &remote_keyring)
        .expect("publish remote checkpoint");

    let reconcile_policy = MembershipRevocationReconcilePolicy {
        trusted_nodes: vec!["node-a".to_string()],
        auto_revoke_missing_keys: false,
    };
    let schedule_policy = MembershipRevocationReconcileSchedulePolicy {
        checkpoint_interval_ms: 1000,
        reconcile_interval_ms: 300,
    };
    let alert_policy = MembershipRevocationAlertPolicy {
        warn_diverged_threshold: 1,
        critical_rejected_threshold: 1,
    };
    let retry_policy = MembershipRevocationAlertAckRetryPolicy {
        max_pending_alerts: 8,
        max_retry_attempts: 2,
        retry_backoff_ms: 100,
    };

    let schedule_store = InMemoryMembershipRevocationScheduleStateStore::new();
    schedule_store
        .save(
            "w1",
            "node-b",
            &MembershipRevocationReconcileScheduleState {
                last_checkpoint_at_ms: Some(1300),
                last_reconcile_at_ms: Some(1000),
            },
        )
        .expect("seed schedule state");

    let alert_sink = FailOnceAlertSink::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let coordinator_store: Arc<dyn MembershipRevocationCoordinatorStateStore + Send + Sync> =
        Arc::new(InMemoryMembershipRevocationCoordinatorStateStore::new());
    let coordinator = StoreBackedMembershipRevocationScheduleCoordinator::new(coordinator_store);

    let report = client
        .run_revocation_reconcile_coordinated_with_recovery_and_ack_retry_with_dead_letter_and_metrics_export(
            "w1",
            "node-b",
            1305,
            &subscription,
            &mut local_keyring,
            &reconcile_policy,
            &schedule_policy,
            &alert_policy,
            None,
            None,
            &schedule_store,
            &alert_sink,
            &recovery_store,
            &retry_policy,
            &dead_letter_store,
            &coordinator,
            1000,
        )
        .expect("run with metrics export");

    assert!(report.acquired);
    let metrics_lines = dead_letter_store
        .list_delivery_metrics("w1", "node-b")
        .expect("list exported metrics");
    assert_eq!(metrics_lines.len(), 1);
    assert_eq!(metrics_lines[0].0, 1305);
    assert_eq!(metrics_lines[0].1, report.delivery_metrics);
}
