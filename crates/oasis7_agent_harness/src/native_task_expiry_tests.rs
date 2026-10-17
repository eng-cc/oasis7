use super::*;

#[test]
fn expired_ready_context_is_retired_after_all_originals_are_scanned_without_spending_a_turn() {
    let host = Arc::new(FakeHost::new());
    {
        let mut s = host.state.lock().unwrap();
        s.recover
            .insert("operator-run:0".into(), NativeTaskRecovery::Ready);
        s.expired_contexts
            .insert("operator-run:0".into(), NativeTaskRecovery::Settled);
    }
    let report = NativeTaskRunner::new(
        host.clone(),
        provider(vec![vec![call("move", json!({"to":[1,2,3]}))]]),
        config(),
    )
    .unwrap()
    .run()
    .unwrap();
    assert_eq!(report.stop_reason, NativeTaskStopReason::Completed);
    assert_eq!(report.turns_completed, 1);
    assert_eq!(report.model_calls_spent, 1);
    assert_eq!(
        &host.events()[..4],
        [
            "recover:operator-run:0",
            "recover:operator-run:1",
            "recover:operator-run:2",
            "retire:operator-run:0:context_expired"
        ]
    );
    assert!(!host.events().contains(&"prepare:operator-run:0".into()));
}

#[test]
fn later_unknown_original_prevents_even_expired_context_retirement() {
    let host = Arc::new(FakeHost::new());
    {
        let mut s = host.state.lock().unwrap();
        s.recover
            .insert("operator-run:0".into(), NativeTaskRecovery::Ready);
        s.recover
            .insert("operator-run:2".into(), NativeTaskRecovery::Pending);
        s.expired_contexts
            .insert("operator-run:0".into(), NativeTaskRecovery::Settled);
    }
    let report = NativeTaskRunner::new(host.clone(), provider(vec![]), config())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(report.stop_reason, NativeTaskStopReason::PendingOriginal);
    assert!(
        host.events()
            .iter()
            .all(|event| event.starts_with("recover:"))
    );
}

#[test]
fn uncertain_expiry_cleanup_never_prepares_or_spends_a_model_call() {
    for (result, reason) in [
        (
            NativeTaskRecovery::Pending,
            NativeTaskStopReason::PendingOriginal,
        ),
        (
            NativeTaskRecovery::Rejected,
            NativeTaskStopReason::RejectedOriginal,
        ),
        (
            NativeTaskRecovery::Expired,
            NativeTaskStopReason::ExpiredOriginal,
        ),
        (NativeTaskRecovery::Aborted, NativeTaskStopReason::Aborted),
        (NativeTaskRecovery::Ready, NativeTaskStopReason::HostFailure),
    ] {
        let host = Arc::new(FakeHost::new());
        {
            let mut s = host.state.lock().unwrap();
            s.recover
                .insert("operator-run:0".into(), NativeTaskRecovery::Ready);
            s.expired_contexts.insert("operator-run:0".into(), result);
        }
        let report = NativeTaskRunner::new(host.clone(), provider(vec![]), config())
            .unwrap()
            .run()
            .unwrap();
        assert_eq!(report.stop_reason, reason);
        assert_eq!(report.model_calls_spent, 0);
        assert_eq!(report.turns_completed, 0);
        assert!(
            host.events()
                .iter()
                .all(|event| !event.starts_with("prepare:") && !event.starts_with("reserve:"))
        );
    }
}

#[test]
fn expired_cleanup_cannot_reset_exhausted_model_budget() {
    let host = Arc::new(FakeHost::new());
    let mut c = config();
    c.max_model_calls = 1;
    {
        let mut s = host.state.lock().unwrap();
        s.spent = 1;
        s.recover
            .insert(c.operation_id(0), NativeTaskRecovery::Ready);
        s.expired_contexts
            .insert(c.operation_id(0), NativeTaskRecovery::Settled);
    }
    let report = NativeTaskRunner::new(host.clone(), provider(vec![]), c.clone())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(report.stop_reason, NativeTaskStopReason::BudgetExhausted);
    assert_eq!(report.model_calls_spent, 1);
    assert_eq!(report.turns_completed, 0);
    assert_eq!(
        host.state.lock().unwrap().recover[&c.operation_id(0)],
        NativeTaskRecovery::ContextRetired
    );
    let before = host.events().len();
    let restarted = NativeTaskRunner::new(host.clone(), provider(vec![]), c)
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(restarted.stop_reason, NativeTaskStopReason::BudgetExhausted);
    assert_eq!(restarted.model_calls_spent, 1);
    assert_eq!(
        &host.events()[before..],
        ["recover:operator-run:0", "recover:operator-run:1"]
    );
}

#[test]
fn control_change_prevents_expired_cleanup_and_existing_action_skips_it() {
    let mut stopped = FakeHost::new();
    stopped.drift = Some("stop");
    stopped.started.store(true, Ordering::SeqCst);
    let host = Arc::new(stopped);
    {
        let mut s = host.state.lock().unwrap();
        s.recover
            .insert("operator-run:0".into(), NativeTaskRecovery::Ready);
        s.expired_contexts
            .insert("operator-run:0".into(), NativeTaskRecovery::Settled);
    }
    let report = NativeTaskRunner::new(host.clone(), provider(vec![]), config())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(report.stop_reason, NativeTaskStopReason::ControlChanged);
    assert!(
        host.events()
            .iter()
            .all(|event| event.starts_with("recover:"))
    );

    let host = Arc::new(FakeHost::new());
    {
        let mut s = host.state.lock().unwrap();
        s.recover
            .insert("operator-run:0".into(), NativeTaskRecovery::Settled);
        s.expired_contexts
            .insert("operator-run:0".into(), NativeTaskRecovery::Settled);
    }
    let report = NativeTaskRunner::new(host.clone(), provider(vec![]), config())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(report.turns_completed, 1);
    assert!(
        host.events()
            .iter()
            .all(|event| event.starts_with("recover:"))
    );
}

#[test]
fn stop_during_committed_expiry_cleanup_preserves_receipt_without_entering_next_slot() {
    let mut stopped = FakeHost::new();
    stopped.drift = Some("stop");
    let host = Arc::new(stopped);
    {
        let mut s = host.state.lock().unwrap();
        s.stop_after_retirement = true;
        s.recover
            .insert("operator-run:0".into(), NativeTaskRecovery::Ready);
        s.expired_contexts
            .insert("operator-run:0".into(), NativeTaskRecovery::Settled);
    }
    let report = NativeTaskRunner::new(host.clone(), provider(vec![]), config())
        .unwrap()
        .run()
        .unwrap();
    assert_eq!(report.stop_reason, NativeTaskStopReason::ControlChanged);
    assert_eq!(
        host.state.lock().unwrap().recover["operator-run:0"],
        NativeTaskRecovery::ContextRetired
    );
    assert_eq!(report.model_calls_spent, 0);
    assert_eq!(report.turns_completed, 0);
    assert!(
        host.events()
            .iter()
            .all(|event| !event.starts_with("prepare:") && !event.starts_with("reserve:"))
    );
}
