use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::distributed_net::{DistributedNetwork, InMemoryNetwork};

use crate::{
    FileMembershipRevocationDeadLetterReplayPolicyStore,
    InMemoryMembershipRevocationAlertDeadLetterStore,
    InMemoryMembershipRevocationAlertRecoveryStore,
    InMemoryMembershipRevocationCoordinatorStateStore,
    InMemoryMembershipRevocationDeadLetterReplayPolicyStore,
    InMemoryMembershipRevocationDeadLetterReplayStateStore,
    MembershipRevocationAlertDeadLetterReason, MembershipRevocationAlertDeadLetterRecord,
    MembershipRevocationAlertDeadLetterStore, MembershipRevocationAlertDeliveryMetrics,
    MembershipRevocationAlertSeverity, MembershipRevocationAnomalyAlert,
    MembershipRevocationCoordinatorStateStore, MembershipRevocationDeadLetterReplayPolicy,
    MembershipRevocationDeadLetterReplayPolicyState,
    MembershipRevocationDeadLetterReplayPolicyStore,
    MembershipRevocationDeadLetterReplayRollbackGuard, MembershipRevocationPendingAlert,
    MembershipSyncClient, StoreBackedMembershipRevocationScheduleCoordinator, error::WorldError,
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

#[test]
fn replay_policy_store_file_round_trip() {
    let root = temp_membership_dir("dead-letter-replay-policy-store");
    fs::create_dir_all(&root).expect("create temp dir");
    let store = FileMembershipRevocationDeadLetterReplayPolicyStore::new(&root)
        .expect("create replay policy store");

    let state = MembershipRevocationDeadLetterReplayPolicyState {
        active_policy: MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 6,
            max_retry_limit_exceeded_streak: 2,
        },
        last_stable_policy: MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 4,
            max_retry_limit_exceeded_streak: 3,
        },
        last_policy_update_at_ms: Some(1_200),
        last_stable_at_ms: Some(1_000),
        last_rollback_at_ms: Some(1_100),
    };
    store
        .save_policy_state("w1", "node-a", &state)
        .expect("save policy state");

    let loaded = store
        .load_policy_state("w1", "node-a")
        .expect("load policy state");
    assert_eq!(loaded, state);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn recommend_with_persistence_promotes_stable_policy_when_healthy() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let replay_policy_store = InMemoryMembershipRevocationDeadLetterReplayPolicyStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();

    dead_letter_store
        .append_delivery_metrics(
            "w1",
            "node-a",
            1000,
            &MembershipRevocationAlertDeliveryMetrics {
                attempted: 6,
                succeeded: 6,
                failed: 0,
                deferred: 0,
                buffered: 0,
                dropped_capacity: 0,
                dropped_retry_limit: 0,
                dead_lettered: 0,
            },
        )
        .expect("append healthy metrics");

    let fallback_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 4,
        max_retry_limit_exceeded_streak: 3,
    };
    let rollback_guard = MembershipRevocationDeadLetterReplayRollbackGuard {
        min_attempted: 4,
        failure_ratio_per_mille: 500,
        dead_letter_ratio_per_mille: 500,
        rollback_cooldown_ms: 200,
    };
    let (recommended, rolled_back) = client
        .recommend_revocation_dead_letter_replay_policy_with_persistence_and_rollback_guard(
            "w1",
            "node-a",
            1500,
            &fallback_policy,
            &replay_state_store,
            &replay_policy_store,
            &recovery_store,
            &dead_letter_store,
            8,
            1,
            16,
            8,
            100,
            3,
            2,
            &rollback_guard,
        )
        .expect("recommend persisted policy");

    assert!(!rolled_back);
    assert_eq!(recommended.max_replay_per_run, 3);
    let stored = replay_policy_store
        .load_policy_state("w1", "node-a")
        .expect("load persisted policy state");
    assert_eq!(stored.active_policy, recommended);
    assert_eq!(stored.last_stable_policy, recommended);
    assert_eq!(stored.last_policy_update_at_ms, Some(1500));
    assert_eq!(stored.last_stable_at_ms, Some(1500));
}

#[test]
fn recommend_with_persistence_rolls_back_when_metrics_unhealthy() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let replay_policy_store = InMemoryMembershipRevocationDeadLetterReplayPolicyStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();

    replay_policy_store
        .save_policy_state(
            "w1",
            "node-a",
            &MembershipRevocationDeadLetterReplayPolicyState {
                active_policy: MembershipRevocationDeadLetterReplayPolicy {
                    max_replay_per_run: 9,
                    max_retry_limit_exceeded_streak: 1,
                },
                last_stable_policy: MembershipRevocationDeadLetterReplayPolicy {
                    max_replay_per_run: 3,
                    max_retry_limit_exceeded_streak: 3,
                },
                last_policy_update_at_ms: Some(1000),
                last_stable_at_ms: Some(900),
                last_rollback_at_ms: None,
            },
        )
        .expect("seed policy state");

    dead_letter_store
        .append_delivery_metrics(
            "w1",
            "node-a",
            1200,
            &MembershipRevocationAlertDeliveryMetrics {
                attempted: 12,
                succeeded: 1,
                failed: 8,
                deferred: 0,
                buffered: 0,
                dropped_capacity: 1,
                dropped_retry_limit: 1,
                dead_lettered: 2,
            },
        )
        .expect("append unhealthy metrics");

    let rollback_guard = MembershipRevocationDeadLetterReplayRollbackGuard {
        min_attempted: 8,
        failure_ratio_per_mille: 500,
        dead_letter_ratio_per_mille: 120,
        rollback_cooldown_ms: 100,
    };
    let fallback_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 4,
        max_retry_limit_exceeded_streak: 3,
    };
    let (recommended, rolled_back) = client
        .recommend_revocation_dead_letter_replay_policy_with_persistence_and_rollback_guard(
            "w1",
            "node-a",
            1300,
            &fallback_policy,
            &replay_state_store,
            &replay_policy_store,
            &recovery_store,
            &dead_letter_store,
            8,
            1,
            16,
            8,
            100,
            3,
            2,
            &rollback_guard,
        )
        .expect("recommend persisted policy");

    assert!(rolled_back);
    assert_eq!(
        recommended,
        MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 3,
            max_retry_limit_exceeded_streak: 3
        }
    );
    let state = replay_policy_store
        .load_policy_state("w1", "node-a")
        .expect("load policy state after rollback");
    assert_eq!(state.active_policy, recommended);
    assert_eq!(state.last_rollback_at_ms, Some(1300));
}

#[test]
fn recommend_with_persistence_rejects_rollback_cooldown_overflow_without_partial_update() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let replay_policy_store = InMemoryMembershipRevocationDeadLetterReplayPolicyStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();

    let seeded_state = MembershipRevocationDeadLetterReplayPolicyState {
        active_policy: MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 9,
            max_retry_limit_exceeded_streak: 1,
        },
        last_stable_policy: MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 3,
            max_retry_limit_exceeded_streak: 3,
        },
        last_policy_update_at_ms: Some(1000),
        last_stable_at_ms: Some(900),
        last_rollback_at_ms: Some(i64::MIN),
    };
    replay_policy_store
        .save_policy_state("w1", "node-a", &seeded_state)
        .expect("seed policy state");

    dead_letter_store
        .append_delivery_metrics(
            "w1",
            "node-a",
            1200,
            &MembershipRevocationAlertDeliveryMetrics {
                attempted: 12,
                succeeded: 1,
                failed: 8,
                deferred: 0,
                buffered: 0,
                dropped_capacity: 1,
                dropped_retry_limit: 1,
                dead_lettered: 2,
            },
        )
        .expect("append unhealthy metrics");

    let rollback_guard = MembershipRevocationDeadLetterReplayRollbackGuard {
        min_attempted: 8,
        failure_ratio_per_mille: 500,
        dead_letter_ratio_per_mille: 120,
        rollback_cooldown_ms: 100,
    };
    let fallback_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 4,
        max_retry_limit_exceeded_streak: 3,
    };
    let error = client
        .recommend_revocation_dead_letter_replay_policy_with_persistence_and_rollback_guard(
            "w1",
            "node-a",
            1300,
            &fallback_policy,
            &replay_state_store,
            &replay_policy_store,
            &recovery_store,
            &dead_letter_store,
            8,
            1,
            16,
            8,
            100,
            3,
            2,
            &rollback_guard,
        )
        .expect_err("rollback cooldown overflow should fail");
    let reason = match error {
        WorldError::DistributedValidationFailed { reason } => reason,
        other => panic!("unexpected error: {other:?}"),
    };
    assert!(
        reason.contains("rollback cooldown elapsed overflow"),
        "unexpected error: {reason}"
    );

    let state = replay_policy_store
        .load_policy_state("w1", "node-a")
        .expect("load policy state after overflow");
    assert_eq!(state, seeded_state);
}

#[test]
fn run_coordinated_replay_with_persisted_guarded_policy_reports_rollback() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let replay_policy_store = InMemoryMembershipRevocationDeadLetterReplayPolicyStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let coordinator_store: Arc<dyn MembershipRevocationCoordinatorStateStore + Send + Sync> =
        Arc::new(InMemoryMembershipRevocationCoordinatorStateStore::new());
    let coordinator = StoreBackedMembershipRevocationScheduleCoordinator::new(coordinator_store);

    replay_policy_store
        .save_policy_state(
            "w1",
            "node-target",
            &MembershipRevocationDeadLetterReplayPolicyState {
                active_policy: MembershipRevocationDeadLetterReplayPolicy {
                    max_replay_per_run: 6,
                    max_retry_limit_exceeded_streak: 1,
                },
                last_stable_policy: MembershipRevocationDeadLetterReplayPolicy {
                    max_replay_per_run: 2,
                    max_retry_limit_exceeded_streak: 3,
                },
                last_policy_update_at_ms: Some(1000),
                last_stable_at_ms: Some(900),
                last_rollback_at_ms: None,
            },
        )
        .expect("seed policy state");

    dead_letter_store
        .append_delivery_metrics(
            "w1",
            "node-target",
            2000,
            &MembershipRevocationAlertDeliveryMetrics {
                attempted: 10,
                succeeded: 2,
                failed: 6,
                deferred: 0,
                buffered: 0,
                dropped_capacity: 1,
                dropped_retry_limit: 0,
                dead_lettered: 2,
            },
        )
        .expect("append unhealthy metrics");
    for offset in 0..4 {
        dead_letter_store
            .append(&sample_dead_letter(
                "w1",
                "node-target",
                3000 + offset,
                1,
                MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
            ))
            .expect("append dead-letter");
    }

    let fallback_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 4,
        max_retry_limit_exceeded_streak: 3,
    };
    let rollback_guard = MembershipRevocationDeadLetterReplayRollbackGuard {
        min_attempted: 8,
        failure_ratio_per_mille: 500,
        dead_letter_ratio_per_mille: 150,
        rollback_cooldown_ms: 100,
    };
    let (replayed, policy, rolled_back) = client
        .run_revocation_dead_letter_replay_schedule_coordinated_with_state_store_and_persisted_guarded_policy(
            "w1",
            "node-target",
            "node-runner",
            2100,
            50,
            &fallback_policy,
            &replay_state_store,
            &replay_policy_store,
            &recovery_store,
            &dead_letter_store,
            &coordinator,
            500,
            8,
            1,
            16,
            8,
            100,
            3,
            2,
            &rollback_guard,
        )
        .expect("run persisted guarded replay");

    assert!(rolled_back);
    assert_eq!(policy.max_replay_per_run, 2);
    assert_eq!(replayed, 2);
}

#[test]
fn recommend_with_persistence_rejects_invalid_rollback_guard() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let replay_policy_store = InMemoryMembershipRevocationDeadLetterReplayPolicyStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let fallback_policy = MembershipRevocationDeadLetterReplayPolicy::default();
    let invalid_guard = MembershipRevocationDeadLetterReplayRollbackGuard {
        min_attempted: 0,
        ..Default::default()
    };

    let error = client
        .recommend_revocation_dead_letter_replay_policy_with_persistence_and_rollback_guard(
            "w1",
            "node-a",
            1000,
            &fallback_policy,
            &replay_state_store,
            &replay_policy_store,
            &recovery_store,
            &dead_letter_store,
            8,
            1,
            16,
            8,
            100,
            3,
            2,
            &invalid_guard,
        )
        .expect_err("invalid rollback guard should fail");
    let reason = match error {
        WorldError::DistributedValidationFailed { reason } => reason,
        other => panic!("unexpected error: {other:?}"),
    };
    assert!(reason.contains("min_attempted"));
}
