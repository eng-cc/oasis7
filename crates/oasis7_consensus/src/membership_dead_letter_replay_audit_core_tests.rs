use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::distributed_net::{DistributedNetwork, InMemoryNetwork};

use crate::{
    FileMembershipRevocationDeadLetterReplayPolicyAuditStore,
    FileMembershipRevocationDeadLetterReplayRollbackAlertStateStore,
    FileMembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore,
    FileMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore,
    InMemoryMembershipRevocationAlertDeadLetterStore,
    InMemoryMembershipRevocationAlertRecoveryStore, InMemoryMembershipRevocationAlertSink,
    InMemoryMembershipRevocationDeadLetterReplayPolicyAuditStore,
    InMemoryMembershipRevocationDeadLetterReplayPolicyStore,
    InMemoryMembershipRevocationDeadLetterReplayRollbackAlertStateStore,
    InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore,
    InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore,
    InMemoryMembershipRevocationDeadLetterReplayStateStore,
    InMemoryMembershipRevocationScheduleCoordinator, MembershipRevocationAlertDeadLetterReason,
    MembershipRevocationAlertDeadLetterRecord, MembershipRevocationAlertDeadLetterStore,
    MembershipRevocationAlertDeliveryMetrics, MembershipRevocationAlertSeverity,
    MembershipRevocationAnomalyAlert, MembershipRevocationDeadLetterReplayPolicy,
    MembershipRevocationDeadLetterReplayPolicyAdoptionAuditDecision,
    MembershipRevocationDeadLetterReplayPolicyAdoptionAuditRecord,
    MembershipRevocationDeadLetterReplayPolicyAuditStore,
    MembershipRevocationDeadLetterReplayPolicyState,
    MembershipRevocationDeadLetterReplayPolicyStore,
    MembershipRevocationDeadLetterReplayRollbackAlertPolicy,
    MembershipRevocationDeadLetterReplayRollbackAlertState,
    MembershipRevocationDeadLetterReplayRollbackAlertStateStore,
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord,
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore,
    MembershipRevocationDeadLetterReplayRollbackGovernanceLevel,
    MembershipRevocationDeadLetterReplayRollbackGovernancePolicy,
    MembershipRevocationDeadLetterReplayRollbackGovernanceState,
    MembershipRevocationDeadLetterReplayRollbackGovernanceStateStore,
    MembershipRevocationDeadLetterReplayRollbackGuard, MembershipRevocationPendingAlert,
    MembershipSyncClient,
};

fn sample_client() -> MembershipSyncClient {
    let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
    MembershipSyncClient::new(network)
}

fn temp_membership_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("oasis7-consensus-{prefix}-{nanos}"))
}

fn sample_dead_letter(
    world_id: &str,
    node_id: &str,
    detected_at_ms: i64,
    attempt: usize,
    reason: MembershipRevocationAlertDeadLetterReason,
) -> MembershipRevocationAlertDeadLetterRecord {
    MembershipRevocationAlertDeadLetterRecord {
        world_id: world_id.to_string(),
        node_id: node_id.to_string(),
        dropped_at_ms: detected_at_ms,
        reason,
        pending_alert: MembershipRevocationPendingAlert {
            alert: MembershipRevocationAnomalyAlert {
                world_id: world_id.to_string(),
                node_id: node_id.to_string(),
                detected_at_ms,
                severity: MembershipRevocationAlertSeverity::Warn,
                code: "reconcile_diverged".to_string(),
                message: "membership revocation reconcile diverged".to_string(),
                drained: 1,
                diverged: 1,
                rejected: 0,
            },
            attempt,
            next_retry_at_ms: detected_at_ms,
            last_error: None,
        },
    }
}

fn unhealthy_metrics() -> MembershipRevocationAlertDeliveryMetrics {
    MembershipRevocationAlertDeliveryMetrics {
        attempted: 10,
        succeeded: 2,
        failed: 6,
        deferred: 0,
        buffered: 2,
        dropped_capacity: 0,
        dropped_retry_limit: 2,
        dead_lettered: 4,
    }
}

fn unstable_policy_state(
    last_rollback_at_ms: Option<i64>,
) -> MembershipRevocationDeadLetterReplayPolicyState {
    MembershipRevocationDeadLetterReplayPolicyState {
        active_policy: MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 8,
            max_retry_limit_exceeded_streak: 1,
        },
        last_stable_policy: MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 4,
            max_retry_limit_exceeded_streak: 2,
        },
        last_policy_update_at_ms: Some(1_000),
        last_stable_at_ms: Some(900),
        last_rollback_at_ms,
    }
}

#[test]
fn replay_policy_audit_store_file_round_trip() {
    let root = temp_membership_dir("dead-letter-replay-policy-audit-store");
    fs::create_dir_all(&root).expect("create temp dir");
    let store = FileMembershipRevocationDeadLetterReplayPolicyAuditStore::new(&root)
        .expect("create replay policy audit store");

    let record = MembershipRevocationDeadLetterReplayPolicyAdoptionAuditRecord {
        world_id: "w1".to_string(),
        node_id: "node-a".to_string(),
        audited_at_ms: 1200,
        decision: MembershipRevocationDeadLetterReplayPolicyAdoptionAuditDecision::Adopted,
        recommended_policy: MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 6,
            max_retry_limit_exceeded_streak: 2,
        },
        applied_policy: MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 6,
            max_retry_limit_exceeded_streak: 2,
        },
        stable_policy: MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 4,
            max_retry_limit_exceeded_streak: 2,
        },
        backlog_dead_letters: 2,
        backlog_pending: 1,
        metrics: MembershipRevocationAlertDeliveryMetrics {
            attempted: 5,
            succeeded: 5,
            failed: 0,
            deferred: 0,
            buffered: 0,
            dropped_capacity: 0,
            dropped_retry_limit: 0,
            dead_lettered: 0,
        },
        rollback_triggered: false,
    };
    store
        .append("w1", "node-a", &record)
        .expect("append audit record");

    let loaded = store.list("w1", "node-a").expect("list audit records");
    assert_eq!(loaded, vec![record]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn run_with_audit_records_rollback_without_emitting_alert_when_below_threshold() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let replay_policy_store = InMemoryMembershipRevocationDeadLetterReplayPolicyStore::new();
    let replay_policy_audit_store =
        InMemoryMembershipRevocationDeadLetterReplayPolicyAuditStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let coordinator = InMemoryMembershipRevocationScheduleCoordinator::new();
    let alert_sink = InMemoryMembershipRevocationAlertSink::new();
    let mut rollback_alert_state =
        MembershipRevocationDeadLetterReplayRollbackAlertState::default();

    replay_policy_store
        .save_policy_state("w1", "node-a", &unstable_policy_state(Some(900)))
        .expect("seed unstable policy state");
    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1000,
            4,
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        ))
        .expect("append dead letter");
    dead_letter_store
        .append_delivery_metrics("w1", "node-a", 1000, &unhealthy_metrics())
        .expect("append metrics");

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
        max_rollbacks_per_window: 2,
        min_attempted: 4,
        alert_cooldown_ms: 500,
    };

    let (replayed, applied_policy, rolled_back, alert_emitted) = client
        .run_revocation_dead_letter_replay_schedule_coordinated_with_state_store_and_persisted_guarded_policy_with_audit_and_alert(
            "w1",
            "node-a",
            "coordinator-1",
            1200,
            1,
            &fallback_policy,
            &replay_state_store,
            &replay_policy_store,
            &replay_policy_audit_store,
            &recovery_store,
            &dead_letter_store,
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
            &mut rollback_alert_state,
            &alert_sink,
        )
        .expect("run with audit and alert");

    assert_eq!(replayed, 1);
    assert!(rolled_back);
    assert!(!alert_emitted);
    assert_eq!(
        applied_policy,
        MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 4,
            max_retry_limit_exceeded_streak: 2,
        }
    );
    let records = replay_policy_audit_store
        .list("w1", "node-a")
        .expect("list policy audit records");
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].decision,
        MembershipRevocationDeadLetterReplayPolicyAdoptionAuditDecision::RolledBackToStable
    );
    assert!(records[0].rollback_triggered);
    assert_eq!(records[0].metrics.attempted, 10);
    assert!(rollback_alert_state.last_alert_at_ms.is_none());
    assert!(alert_sink.list().expect("list emitted alerts").is_empty());
}

#[test]
fn run_with_audit_emits_rollback_alert_and_honors_cooldown() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let replay_policy_store = InMemoryMembershipRevocationDeadLetterReplayPolicyStore::new();
    let replay_policy_audit_store =
        InMemoryMembershipRevocationDeadLetterReplayPolicyAuditStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let coordinator = InMemoryMembershipRevocationScheduleCoordinator::new();
    let alert_sink = InMemoryMembershipRevocationAlertSink::new();
    let mut rollback_alert_state =
        MembershipRevocationDeadLetterReplayRollbackAlertState::default();

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
        alert_cooldown_ms: 200,
    };

    let run_once =
        |now_ms: i64,
         state_last_rollback_at_ms: i64,
         replay_policy_store: &InMemoryMembershipRevocationDeadLetterReplayPolicyStore,
         dead_letter_store: &InMemoryMembershipRevocationAlertDeadLetterStore,
         rollback_alert_state: &mut MembershipRevocationDeadLetterReplayRollbackAlertState|
         -> (bool, bool) {
            replay_policy_store
                .save_policy_state(
                    "w1",
                    "node-a",
                    &unstable_policy_state(Some(state_last_rollback_at_ms)),
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
            let (_, _, rolled_back, alert_emitted) = client
            .run_revocation_dead_letter_replay_schedule_coordinated_with_state_store_and_persisted_guarded_policy_with_audit_and_alert(
                "w1",
                "node-a",
                "coordinator-1",
                now_ms,
                1,
                &fallback_policy,
                &replay_state_store,
                replay_policy_store,
                &replay_policy_audit_store,
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
                rollback_alert_state,
                &alert_sink,
            )
            .expect("run with audit");
            (rolled_back, alert_emitted)
        };

    let first = run_once(
        1200,
        1000,
        &replay_policy_store,
        &dead_letter_store,
        &mut rollback_alert_state,
    );
    assert_eq!(first, (true, true));
    assert_eq!(rollback_alert_state.last_alert_at_ms, Some(1200));

    let second = run_once(
        1300,
        1100,
        &replay_policy_store,
        &dead_letter_store,
        &mut rollback_alert_state,
    );
    assert_eq!(second, (true, false));
    assert_eq!(rollback_alert_state.last_alert_at_ms, Some(1200));

    let third = run_once(
        1500,
        1300,
        &replay_policy_store,
        &dead_letter_store,
        &mut rollback_alert_state,
    );
    assert_eq!(third, (true, true));
    assert_eq!(rollback_alert_state.last_alert_at_ms, Some(1500));

    let alerts = alert_sink.list().expect("list alerts");
    assert_eq!(alerts.len(), 2);
    assert_eq!(alerts[0].code, "dead_letter_replay_policy_rollback_anomaly");
    assert_eq!(alerts[1].code, "dead_letter_replay_policy_rollback_anomaly");

    let records = replay_policy_audit_store
        .list("w1", "node-a")
        .expect("list audit records");
    assert_eq!(records.len(), 3);
    assert!(records.iter().all(|record| record.rollback_triggered));
}

#[test]
fn run_with_audit_rejects_rollback_window_age_overflow_without_mutating_alert_state() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let replay_policy_store = InMemoryMembershipRevocationDeadLetterReplayPolicyStore::new();
    let replay_policy_audit_store =
        InMemoryMembershipRevocationDeadLetterReplayPolicyAuditStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let coordinator = InMemoryMembershipRevocationScheduleCoordinator::new();
    let alert_sink = InMemoryMembershipRevocationAlertSink::new();
    let mut rollback_alert_state =
        MembershipRevocationDeadLetterReplayRollbackAlertState::default();

    replay_policy_store
        .save_policy_state("w1", "node-a", &unstable_policy_state(Some(900)))
        .expect("seed unstable policy state");
    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1_000,
            4,
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        ))
        .expect("append dead letter");
    dead_letter_store
        .append_delivery_metrics("w1", "node-a", 1_000, &unhealthy_metrics())
        .expect("append metrics");
    replay_policy_audit_store
        .append(
            "w1",
            "node-a",
            &MembershipRevocationDeadLetterReplayPolicyAdoptionAuditRecord {
                world_id: "w1".to_string(),
                node_id: "node-a".to_string(),
                audited_at_ms: i64::MIN,
                decision:
                    MembershipRevocationDeadLetterReplayPolicyAdoptionAuditDecision::RolledBackToStable,
                recommended_policy: MembershipRevocationDeadLetterReplayPolicy {
                    max_replay_per_run: 4,
                    max_retry_limit_exceeded_streak: 2,
                },
                applied_policy: MembershipRevocationDeadLetterReplayPolicy {
                    max_replay_per_run: 4,
                    max_retry_limit_exceeded_streak: 2,
                },
                stable_policy: MembershipRevocationDeadLetterReplayPolicy {
                    max_replay_per_run: 4,
                    max_retry_limit_exceeded_streak: 2,
                },
                backlog_dead_letters: 1,
                backlog_pending: 0,
                metrics: unhealthy_metrics(),
                rollback_triggered: true,
            },
        )
        .expect("seed historical audit");

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

    let error = client
        .run_revocation_dead_letter_replay_schedule_coordinated_with_state_store_and_persisted_guarded_policy_with_audit_and_alert(
            "w1",
            "node-a",
            "coordinator-1",
            1_000,
            1,
            &fallback_policy,
            &replay_state_store,
            &replay_policy_store,
            &replay_policy_audit_store,
            &recovery_store,
            &dead_letter_store,
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
            &mut rollback_alert_state,
            &alert_sink,
        )
        .expect_err("rollback window overflow should fail");
    let message = format!("{error:?}");
    assert!(
        message.contains("rollback window age overflow"),
        "unexpected error: {message}"
    );
    assert_eq!(rollback_alert_state.last_alert_at_ms, None);
    assert!(
        alert_sink.list().expect("list alerts").is_empty(),
        "overflow should not emit alerts"
    );
}

#[test]
fn run_with_audit_rejects_cooldown_age_overflow_without_mutating_alert_state() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let replay_policy_store = InMemoryMembershipRevocationDeadLetterReplayPolicyStore::new();
    let replay_policy_audit_store =
        InMemoryMembershipRevocationDeadLetterReplayPolicyAuditStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let coordinator = InMemoryMembershipRevocationScheduleCoordinator::new();
    let alert_sink = InMemoryMembershipRevocationAlertSink::new();
    let mut rollback_alert_state = MembershipRevocationDeadLetterReplayRollbackAlertState {
        last_alert_at_ms: Some(i64::MIN),
    };

    replay_policy_store
        .save_policy_state("w1", "node-a", &unstable_policy_state(Some(900)))
        .expect("seed unstable policy state");
    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1_000,
            4,
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        ))
        .expect("append dead letter");
    dead_letter_store
        .append_delivery_metrics("w1", "node-a", 1_000, &unhealthy_metrics())
        .expect("append metrics");

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

    let error = client
        .run_revocation_dead_letter_replay_schedule_coordinated_with_state_store_and_persisted_guarded_policy_with_audit_and_alert(
            "w1",
            "node-a",
            "coordinator-1",
            1_000,
            1,
            &fallback_policy,
            &replay_state_store,
            &replay_policy_store,
            &replay_policy_audit_store,
            &recovery_store,
            &dead_letter_store,
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
            &mut rollback_alert_state,
            &alert_sink,
        )
        .expect_err("cooldown overflow should fail");
    let message = format!("{error:?}");
    assert!(
        message.contains("alert cooldown age overflow"),
        "unexpected error: {message}"
    );
    assert_eq!(rollback_alert_state.last_alert_at_ms, Some(i64::MIN));
    assert!(
        alert_sink.list().expect("list alerts").is_empty(),
        "overflow should not emit alerts"
    );
}

#[test]
fn run_with_audit_rejects_invalid_rollback_alert_policy() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let replay_policy_store = InMemoryMembershipRevocationDeadLetterReplayPolicyStore::new();
    let replay_policy_audit_store =
        InMemoryMembershipRevocationDeadLetterReplayPolicyAuditStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let coordinator = InMemoryMembershipRevocationScheduleCoordinator::new();
    let alert_sink = InMemoryMembershipRevocationAlertSink::new();
    let mut rollback_alert_state =
        MembershipRevocationDeadLetterReplayRollbackAlertState::default();

    let fallback_policy = MembershipRevocationDeadLetterReplayPolicy::default();
    let rollback_guard = MembershipRevocationDeadLetterReplayRollbackGuard::default();
    let invalid_alert_policy = MembershipRevocationDeadLetterReplayRollbackAlertPolicy {
        rollback_window_ms: 0,
        max_rollbacks_per_window: 1,
        min_attempted: 4,
        alert_cooldown_ms: 10,
    };
    let error = client
        .run_revocation_dead_letter_replay_schedule_coordinated_with_state_store_and_persisted_guarded_policy_with_audit_and_alert(
            "w1",
            "node-a",
            "coordinator-1",
            1200,
            1,
            &fallback_policy,
            &replay_state_store,
            &replay_policy_store,
            &replay_policy_audit_store,
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
            &invalid_alert_policy,
            &mut rollback_alert_state,
            &alert_sink,
        )
        .expect_err("invalid rollback alert policy should fail");
    let message = format!("{error:?}");
    assert!(
        message.contains("rollback_window_ms must be positive"),
        "unexpected error: {message}"
    );
}

#[test]
fn rollback_alert_state_store_file_round_trip() {
    let root = temp_membership_dir("dead-letter-replay-rollback-alert-state-store");
    fs::create_dir_all(&root).expect("create temp dir");
    let store = FileMembershipRevocationDeadLetterReplayRollbackAlertStateStore::new(&root)
        .expect("create rollback alert state store");
    let state = MembershipRevocationDeadLetterReplayRollbackAlertState {
        last_alert_at_ms: Some(1234),
    };
    store
        .save_alert_state("w1", "node-a", &state)
        .expect("save rollback alert state");
    let loaded = store
        .load_alert_state("w1", "node-a")
        .expect("load rollback alert state");
    assert_eq!(loaded, state);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn rollback_governance_state_store_file_round_trip() {
    let root = temp_membership_dir("dead-letter-replay-rollback-governance-state-store");
    fs::create_dir_all(&root).expect("create temp dir");
    let store = FileMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore::new(&root)
        .expect("create rollback governance state store");
    let state = MembershipRevocationDeadLetterReplayRollbackGovernanceState {
        rollback_streak: 3,
        last_level: MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Stable,
        last_level_updated_at_ms: Some(1300),
    };
    store
        .save_governance_state("w1", "node-a", &state)
        .expect("save governance state");
    let loaded = store
        .load_governance_state("w1", "node-a")
        .expect("load governance state");
    assert_eq!(loaded, state);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn rollback_governance_audit_store_file_round_trip() {
    let root = temp_membership_dir("dead-letter-replay-rollback-governance-audit-store");
    fs::create_dir_all(&root).expect("create temp dir");
    let store = FileMembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore::new(&root)
        .expect("create rollback governance audit store");
    let record = MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord {
        world_id: "w1".to_string(),
        node_id: "node-a".to_string(),
        audited_at_ms: 1234,
        governance_level: MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Stable,
        rollback_streak: 2,
        rolled_back: true,
        applied_policy: MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 4,
            max_retry_limit_exceeded_streak: 2,
        },
        alert_emitted: false,
    };
    store
        .append("w1", "node-a", &record)
        .expect("append governance audit record");
    let loaded = store
        .list("w1", "node-a")
        .expect("list governance audit records");
    assert_eq!(loaded, vec![record]);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn run_with_governance_archive_appends_audit_record() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let replay_policy_store = InMemoryMembershipRevocationDeadLetterReplayPolicyStore::new();
    let replay_policy_audit_store =
        InMemoryMembershipRevocationDeadLetterReplayPolicyAuditStore::new();
    let rollback_alert_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackAlertStateStore::new();
    let rollback_governance_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore::new();
    let rollback_governance_audit_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let coordinator = InMemoryMembershipRevocationScheduleCoordinator::new();
    let alert_sink = InMemoryMembershipRevocationAlertSink::new();

    replay_policy_store
        .save_policy_state("w1", "node-a", &unstable_policy_state(Some(1000)))
        .expect("seed unstable policy state");
    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1200,
            4,
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        ))
        .expect("append dead letter");
    dead_letter_store
        .append_delivery_metrics("w1", "node-a", 1200, &unhealthy_metrics())
        .expect("append metrics");

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
    let rollback_governance_policy = MembershipRevocationDeadLetterReplayRollbackGovernancePolicy {
        level_one_rollback_streak: 1,
        level_two_rollback_streak: 3,
        level_two_emergency_policy: MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 1,
            max_retry_limit_exceeded_streak: 1,
        },
    };

    let (replayed, policy, rolled_back, alert_emitted, governance_level) = client
        .run_revocation_dead_letter_replay_schedule_coordinated_with_state_store_and_persisted_guarded_policy_with_audit_alert_store_governance_and_archive(
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
            &rollback_governance_audit_store,
            &recovery_store,
            &dead_letter_store,
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
        .expect("run with governance archive");
    assert_eq!(replayed, 1);
    assert!(rolled_back);
    assert!(!alert_emitted);
    assert_eq!(
        governance_level,
        MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Stable
    );

    let records = rollback_governance_audit_store
        .list("w1", "node-a")
        .expect("list governance audit records");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].governance_level, governance_level);
    assert_eq!(records[0].applied_policy, policy);
    assert!(records[0].rolled_back);
}

#[test]
fn run_with_governance_archive_rejects_rollback_streak_overflow_without_mutation() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let replay_policy_store = InMemoryMembershipRevocationDeadLetterReplayPolicyStore::new();
    let replay_policy_audit_store =
        InMemoryMembershipRevocationDeadLetterReplayPolicyAuditStore::new();
    let rollback_alert_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackAlertStateStore::new();
    let rollback_governance_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore::new();
    let rollback_governance_audit_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let coordinator = InMemoryMembershipRevocationScheduleCoordinator::new();
    let alert_sink = InMemoryMembershipRevocationAlertSink::new();

    replay_policy_store
        .save_policy_state("w1", "node-a", &unstable_policy_state(Some(1000)))
        .expect("seed unstable policy state");
    rollback_governance_state_store
        .save_governance_state(
            "w1",
            "node-a",
            &MembershipRevocationDeadLetterReplayRollbackGovernanceState {
                rollback_streak: usize::MAX,
                last_level: MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
                last_level_updated_at_ms: Some(1100),
            },
        )
        .expect("seed governance state");
    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1200,
            4,
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        ))
        .expect("append dead letter");
    dead_letter_store
        .append_delivery_metrics("w1", "node-a", 1200, &unhealthy_metrics())
        .expect("append metrics");

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
    let rollback_governance_policy = MembershipRevocationDeadLetterReplayRollbackGovernancePolicy {
        level_one_rollback_streak: 1,
        level_two_rollback_streak: 3,
        level_two_emergency_policy: MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 1,
            max_retry_limit_exceeded_streak: 1,
        },
    };

    let error = client
        .run_revocation_dead_letter_replay_schedule_coordinated_with_state_store_and_persisted_guarded_policy_with_audit_alert_store_governance_and_archive(
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
            &rollback_governance_audit_store,
            &recovery_store,
            &dead_letter_store,
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
        .expect_err("rollback streak overflow should fail");
    let message = format!("{error:?}");
    assert!(
        message.contains("rollback_streak overflow"),
        "unexpected error: {message}"
    );

    let governance_state = rollback_governance_state_store
        .load_governance_state("w1", "node-a")
        .expect("load governance state after overflow");
    assert_eq!(governance_state.rollback_streak, usize::MAX);
    let governance_records = rollback_governance_audit_store
        .list("w1", "node-a")
        .expect("list governance audit records after overflow");
    assert!(governance_records.is_empty());
}
