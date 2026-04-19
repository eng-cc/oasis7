#[test]
fn rollback_governance_recovery_drill_reports_recent_audits_and_emergency_history() {
    let client = sample_client();
    let rollback_alert_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackAlertStateStore::new();
    let rollback_governance_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore::new();
    let rollback_governance_audit_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore::new();

    let alert_state = MembershipRevocationDeadLetterReplayRollbackAlertState {
        last_alert_at_ms: Some(1400),
    };
    rollback_alert_state_store
        .save_alert_state("w1", "node-a", &alert_state)
        .expect("save alert state");
    let governance_state = MembershipRevocationDeadLetterReplayRollbackGovernanceState {
        rollback_streak: 3,
        last_level: MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
        last_level_updated_at_ms: Some(1500),
    };
    rollback_governance_state_store
        .save_governance_state("w1", "node-a", &governance_state)
        .expect("save governance state");

    let normal_record = MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord {
        world_id: "w1".to_string(),
        node_id: "node-a".to_string(),
        audited_at_ms: 1200,
        governance_level: MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Normal,
        rollback_streak: 0,
        rolled_back: false,
        applied_policy: MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 6,
            max_retry_limit_exceeded_streak: 3,
        },
        alert_emitted: false,
    };
    let emergency_record = MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord {
        world_id: "w1".to_string(),
        node_id: "node-a".to_string(),
        audited_at_ms: 1300,
        governance_level: MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
        rollback_streak: 2,
        rolled_back: true,
        applied_policy: MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 1,
            max_retry_limit_exceeded_streak: 1,
        },
        alert_emitted: true,
    };
    let stable_record = MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord {
        world_id: "w1".to_string(),
        node_id: "node-a".to_string(),
        audited_at_ms: 1400,
        governance_level: MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Stable,
        rollback_streak: 1,
        rolled_back: true,
        applied_policy: MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 4,
            max_retry_limit_exceeded_streak: 2,
        },
        alert_emitted: false,
    };
    rollback_governance_audit_store
        .append("w1", "node-a", &normal_record)
        .expect("append normal governance record");
    rollback_governance_audit_store
        .append("w1", "node-a", &emergency_record)
        .expect("append emergency governance record");
    rollback_governance_audit_store
        .append("w1", "node-a", &stable_record)
        .expect("append stable governance record");

    let report = client
        .run_revocation_dead_letter_replay_rollback_governance_recovery_drill(
            "w1",
            "node-a",
            1600,
            2,
            &rollback_alert_state_store,
            &rollback_governance_state_store,
            &rollback_governance_audit_store,
        )
        .expect("run governance recovery drill");

    assert_eq!(report.world_id, "w1");
    assert_eq!(report.node_id, "node-a");
    assert_eq!(report.drill_at_ms, 1600);
    assert_eq!(report.alert_state, alert_state);
    assert_eq!(report.governance_state, governance_state);
    assert_eq!(report.recent_audits, vec![emergency_record, stable_record]);
    assert!(report.has_emergency_history);
}

#[test]
fn rollback_governance_recovery_drill_rejects_zero_recent_audit_limit() {
    let client = sample_client();
    let rollback_alert_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackAlertStateStore::new();
    let rollback_governance_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore::new();
    let rollback_governance_audit_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore::new();

    let error = client
        .run_revocation_dead_letter_replay_rollback_governance_recovery_drill(
            "w1",
            "node-a",
            1600,
            0,
            &rollback_alert_state_store,
            &rollback_governance_state_store,
            &rollback_governance_audit_store,
        )
        .expect_err("zero recent_audit_limit should fail");
    let message = format!("{error:?}");
    assert!(
        message.contains("recent_audit_limit must be positive"),
        "unexpected error: {message}"
    );
}

#[test]
fn run_with_alert_state_store_persists_cooldown_between_runs() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let replay_policy_store = InMemoryMembershipRevocationDeadLetterReplayPolicyStore::new();
    let replay_policy_audit_store =
        InMemoryMembershipRevocationDeadLetterReplayPolicyAuditStore::new();
    let rollback_alert_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackAlertStateStore::new();
    let rollback_governance_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let coordinator = InMemoryMembershipRevocationScheduleCoordinator::new();
    let alert_sink = InMemoryMembershipRevocationAlertSink::new();

    let fallback_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 4,
        max_retry_limit_exceeded_streak: 2,
    };
    let rollback_guard = MembershipRevocationDeadLetterReplayRollbackGuard {
        min_attempted: 4,
        failure_ratio_per_mille: 500,
        dead_letter_ratio_per_mille: 350,
        rollback_cooldown_ms: 100,
    };
    let rollback_alert_policy = MembershipRevocationDeadLetterReplayRollbackAlertPolicy {
        rollback_window_ms: 5_000,
        max_rollbacks_per_window: 1,
        min_attempted: 4,
        alert_cooldown_ms: 500,
    };
    let rollback_governance_policy = MembershipRevocationDeadLetterReplayRollbackGovernancePolicy {
        level_one_rollback_streak: 10,
        level_two_rollback_streak: 20,
        level_two_emergency_policy: MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 1,
            max_retry_limit_exceeded_streak: 1,
        },
    };

    let run_once =
        |now_ms: i64,
         seeded_last_rollback_at_ms: i64,
         replay_policy_store: &InMemoryMembershipRevocationDeadLetterReplayPolicyStore,
         dead_letter_store: &InMemoryMembershipRevocationAlertDeadLetterStore|
         -> (bool, bool) {
            replay_policy_store
                .save_policy_state(
                    "w1",
                    "node-a",
                    &unstable_policy_state(Some(seeded_last_rollback_at_ms)),
                )
                .expect("seed unstable policy state");
            dead_letter_store
                .append(&sample_dead_letter(
                    "w1",
                    "node-a",
                    now_ms,
                    4,
                    MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
                ))
                .expect("append dead letter");
            dead_letter_store
                .append_delivery_metrics("w1", "node-a", now_ms, &unhealthy_metrics())
                .expect("append metrics");
            let (_, _, rolled_back, alert_emitted, _) = client
                .run_revocation_dead_letter_replay_schedule_coordinated_with_state_store_and_persisted_guarded_policy_with_audit_alert_store_and_governance(
                    "w1",
                    "node-a",
                    "coordinator-1",
                    now_ms,
                    1,
                    &fallback_policy,
                    &replay_state_store,
                    replay_policy_store,
                    &replay_policy_audit_store,
                    &rollback_alert_state_store,
                    &rollback_governance_state_store,
                    &recovery_store,
                    dead_letter_store,
                    &coordinator,
                    500,
                    4,
                    2,
                    16,
                    4,
                    20,
                    4,
                    2,
                    &rollback_guard,
                    &rollback_alert_policy,
                    &rollback_governance_policy,
                    &alert_sink,
                )
                .expect("run with governance");
            (rolled_back, alert_emitted)
        };

    let first = run_once(1200, 1000, &replay_policy_store, &dead_letter_store);
    assert_eq!(first, (true, true));
    let second = run_once(1300, 1100, &replay_policy_store, &dead_letter_store);
    assert_eq!(second, (true, false));

    let alert_state = rollback_alert_state_store
        .load_alert_state("w1", "node-a")
        .expect("load stored rollback alert state");
    assert_eq!(alert_state.last_alert_at_ms, Some(1200));
}

#[test]
fn run_with_governance_escalates_level_and_overrides_policy() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let replay_policy_store = InMemoryMembershipRevocationDeadLetterReplayPolicyStore::new();
    let replay_policy_audit_store =
        InMemoryMembershipRevocationDeadLetterReplayPolicyAuditStore::new();
    let rollback_alert_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackAlertStateStore::new();
    let rollback_governance_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let coordinator = InMemoryMembershipRevocationScheduleCoordinator::new();
    let alert_sink = InMemoryMembershipRevocationAlertSink::new();

    let fallback_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 4,
        max_retry_limit_exceeded_streak: 2,
    };
    let rollback_guard = MembershipRevocationDeadLetterReplayRollbackGuard {
        min_attempted: 4,
        failure_ratio_per_mille: 500,
        dead_letter_ratio_per_mille: 350,
        rollback_cooldown_ms: 100,
    };
    let rollback_alert_policy = MembershipRevocationDeadLetterReplayRollbackAlertPolicy {
        rollback_window_ms: 5_000,
        max_rollbacks_per_window: 10,
        min_attempted: 4,
        alert_cooldown_ms: 500,
    };
    let emergency_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 1,
        max_retry_limit_exceeded_streak: 1,
    };
    let rollback_governance_policy = MembershipRevocationDeadLetterReplayRollbackGovernancePolicy {
        level_one_rollback_streak: 1,
        level_two_rollback_streak: 2,
        level_two_emergency_policy: emergency_policy.clone(),
    };

    let run_once =
        |now_ms: i64,
         seeded_last_rollback_at_ms: i64,
         replay_policy_store: &InMemoryMembershipRevocationDeadLetterReplayPolicyStore,
         dead_letter_store: &InMemoryMembershipRevocationAlertDeadLetterStore|
         -> (
            MembershipRevocationDeadLetterReplayPolicy,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel,
        ) {
            replay_policy_store
                .save_policy_state(
                    "w1",
                    "node-a",
                    &unstable_policy_state(Some(seeded_last_rollback_at_ms)),
                )
                .expect("seed unstable policy state");
            dead_letter_store
                .append(&sample_dead_letter(
                    "w1",
                    "node-a",
                    now_ms,
                    4,
                    MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
                ))
                .expect("append dead letter");
            dead_letter_store
                .append_delivery_metrics("w1", "node-a", now_ms, &unhealthy_metrics())
                .expect("append metrics");
            let (_, policy, rolled_back, _, governance_level) = client
            .run_revocation_dead_letter_replay_schedule_coordinated_with_state_store_and_persisted_guarded_policy_with_audit_alert_store_and_governance(
                "w1",
                "node-a",
                "coordinator-1",
                now_ms,
                1,
                &fallback_policy,
                &replay_state_store,
                replay_policy_store,
                &replay_policy_audit_store,
                &rollback_alert_state_store,
                &rollback_governance_state_store,
                &recovery_store,
                dead_letter_store,
                &coordinator,
                500,
                4,
                2,
                16,
                4,
                20,
                4,
                2,
                &rollback_guard,
                &rollback_alert_policy,
                &rollback_governance_policy,
                &alert_sink,
            )
            .expect("run with governance");
            assert!(rolled_back);
            (policy, governance_level)
        };

    let first = run_once(1200, 1000, &replay_policy_store, &dead_letter_store);
    assert_eq!(
        first,
        (
            MembershipRevocationDeadLetterReplayPolicy {
                max_replay_per_run: 4,
                max_retry_limit_exceeded_streak: 2
            },
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Stable
        )
    );

    let second = run_once(1400, 1200, &replay_policy_store, &dead_letter_store);
    assert_eq!(
        second,
        (
            emergency_policy.clone(),
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency
        )
    );
    let policy_state = replay_policy_store
        .load_policy_state("w1", "node-a")
        .expect("load policy state");
    assert_eq!(policy_state.active_policy, emergency_policy);
    let governance_state = rollback_governance_state_store
        .load_governance_state("w1", "node-a")
        .expect("load governance state");
    assert_eq!(governance_state.rollback_streak, 2);
    assert_eq!(
        governance_state.last_level,
        MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency
    );
}

#[test]
fn run_with_governance_rejects_invalid_policy() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let replay_policy_store = InMemoryMembershipRevocationDeadLetterReplayPolicyStore::new();
    let replay_policy_audit_store =
        InMemoryMembershipRevocationDeadLetterReplayPolicyAuditStore::new();
    let rollback_alert_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackAlertStateStore::new();
    let rollback_governance_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let coordinator = InMemoryMembershipRevocationScheduleCoordinator::new();
    let alert_sink = InMemoryMembershipRevocationAlertSink::new();

    let fallback_policy = MembershipRevocationDeadLetterReplayPolicy::default();
    let rollback_guard = MembershipRevocationDeadLetterReplayRollbackGuard::default();
    let rollback_alert_policy = MembershipRevocationDeadLetterReplayRollbackAlertPolicy::default();
    let invalid_governance_policy = MembershipRevocationDeadLetterReplayRollbackGovernancePolicy {
        level_one_rollback_streak: 2,
        level_two_rollback_streak: 1,
        level_two_emergency_policy: MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 1,
            max_retry_limit_exceeded_streak: 1,
        },
    };

    let error = client
        .run_revocation_dead_letter_replay_schedule_coordinated_with_state_store_and_persisted_guarded_policy_with_audit_alert_store_and_governance(
            "w1",
            "node-a",
            "coordinator-1",
            1200,
            1,
            &fallback_policy,
            &replay_state_store,
            &replay_policy_store,
            &replay_policy_audit_store,
            &rollback_alert_state_store,
            &rollback_governance_state_store,
            &recovery_store,
            &dead_letter_store,
            &coordinator,
            500,
            4,
            1,
            16,
            4,
            20,
            4,
            2,
            &rollback_guard,
            &rollback_alert_policy,
            &invalid_governance_policy,
            &alert_sink,
        )
        .expect_err("invalid governance policy should fail");
    let message = format!("{error:?}");
    assert!(
        message.contains("thresholds are invalid"),
        "unexpected error: {message}"
    );
}
