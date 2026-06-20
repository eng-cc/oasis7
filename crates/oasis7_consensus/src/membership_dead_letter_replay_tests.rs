use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::distributed_net::{DistributedNetwork, InMemoryNetwork};

use crate::{
    FileMembershipRevocationDeadLetterReplayStateStore,
    InMemoryMembershipRevocationAlertDeadLetterStore,
    InMemoryMembershipRevocationAlertRecoveryStore,
    InMemoryMembershipRevocationCoordinatorStateStore,
    InMemoryMembershipRevocationDeadLetterReplayStateStore,
    MembershipRevocationAlertDeadLetterReason, MembershipRevocationAlertDeadLetterRecord,
    MembershipRevocationAlertDeadLetterStore, MembershipRevocationAlertDeliveryMetrics,
    MembershipRevocationAlertRecoveryStore, MembershipRevocationAlertSeverity,
    MembershipRevocationAnomalyAlert, MembershipRevocationCoordinatorStateStore,
    MembershipRevocationDeadLetterReplayPolicy, MembershipRevocationDeadLetterReplayScheduleState,
    MembershipRevocationDeadLetterReplayStateStore, MembershipRevocationPendingAlert,
    MembershipRevocationScheduleCoordinator, MembershipSyncClient,
    StoreBackedMembershipRevocationScheduleCoordinator, error::WorldError,
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
fn replay_revocation_dead_letters_with_policy_rotates_capacity_evicted() {
    let client = sample_client();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 1,
        max_retry_limit_exceeded_streak: 8,
    };
    let mut state = MembershipRevocationDeadLetterReplayScheduleState::default();

    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1000,
            2,
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        ))
        .expect("append retry-limit #1");
    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1001,
            2,
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        ))
        .expect("append retry-limit #2");
    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1002,
            2,
            MembershipRevocationAlertDeadLetterReason::CapacityEvicted,
        ))
        .expect("append capacity #1");
    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1003,
            2,
            MembershipRevocationAlertDeadLetterReason::CapacityEvicted,
        ))
        .expect("append capacity #2");

    let first = client
        .replay_revocation_dead_letters_with_policy(
            "w1",
            "node-a",
            &policy,
            &mut state,
            &recovery_store,
            &dead_letter_store,
        )
        .expect("first policy replay");
    assert_eq!(first, 1);

    let first_pending = recovery_store
        .load_pending("w1", "node-a")
        .expect("load pending after first replay");
    assert_eq!(first_pending.len(), 1);
    assert_eq!(
        first_pending[0].alert.detected_at_ms, 1000,
        "first replay still prioritizes retry-limit"
    );
    assert!(state.prefer_capacity_evicted);

    let second = client
        .replay_revocation_dead_letters_with_policy(
            "w1",
            "node-a",
            &policy,
            &mut state,
            &recovery_store,
            &dead_letter_store,
        )
        .expect("second policy replay");
    assert_eq!(second, 1);

    let second_pending = recovery_store
        .load_pending("w1", "node-a")
        .expect("load pending after second replay");
    assert_eq!(second_pending.len(), 2);
    assert_eq!(
        second_pending[1].alert.detected_at_ms, 1002,
        "second replay rotates into capacity-evicted queue"
    );
}

#[test]
fn run_replay_schedule_with_state_store_persists_interval_gate() {
    let root = temp_membership_dir("dead-letter-replay-state-store");
    fs::create_dir_all(&root).expect("create temp dir");

    let client = sample_client();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let replay_state_store =
        FileMembershipRevocationDeadLetterReplayStateStore::new(&root).expect("create state store");
    let replay_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 1,
        max_retry_limit_exceeded_streak: 2,
    };

    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1000,
            1,
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        ))
        .expect("append dead letter #1");
    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1001,
            1,
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        ))
        .expect("append dead letter #2");

    let first = client
        .run_revocation_dead_letter_replay_schedule_with_state_store(
            "w1",
            "node-a",
            2000,
            100,
            &replay_policy,
            &recovery_store,
            &dead_letter_store,
            &replay_state_store,
        )
        .expect("first schedule run");
    assert_eq!(first, 1);

    let state_after_first = replay_state_store
        .load_state("w1", "node-a")
        .expect("load state after first");
    assert_eq!(state_after_first.last_replay_at_ms, Some(2000));

    let second = client
        .run_revocation_dead_letter_replay_schedule_with_state_store(
            "w1",
            "node-a",
            2050,
            100,
            &replay_policy,
            &recovery_store,
            &dead_letter_store,
            &replay_state_store,
        )
        .expect("second schedule run");
    assert_eq!(
        second, 0,
        "state store should gate run by persisted interval"
    );

    let third = client
        .run_revocation_dead_letter_replay_schedule_with_state_store(
            "w1",
            "node-a",
            2105,
            100,
            &replay_policy,
            &recovery_store,
            &dead_letter_store,
            &replay_state_store,
        )
        .expect("third schedule run");
    assert_eq!(third, 1);

    let pending = recovery_store
        .load_pending("w1", "node-a")
        .expect("load pending");
    assert_eq!(pending.len(), 2);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn run_replay_schedule_with_state_store_rejects_interval_overflow_without_mutation() {
    let client = sample_client();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let replay_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 1,
        max_retry_limit_exceeded_streak: 2,
    };

    replay_state_store
        .save_state(
            "w1",
            "node-a",
            &MembershipRevocationDeadLetterReplayScheduleState {
                last_replay_at_ms: Some(i64::MIN),
                prefer_capacity_evicted: false,
            },
        )
        .expect("seed replay state");
    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1000,
            1,
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        ))
        .expect("append dead letter");

    let error = client
        .run_revocation_dead_letter_replay_schedule_with_state_store(
            "w1",
            "node-a",
            1000,
            100,
            &replay_policy,
            &recovery_store,
            &dead_letter_store,
            &replay_state_store,
        )
        .expect_err("interval overflow should fail");
    let message = format!("{error:?}");
    assert!(
        message.contains("replay schedule elapsed overflow"),
        "unexpected error: {message}"
    );

    let state = replay_state_store
        .load_state("w1", "node-a")
        .expect("load replay state after overflow");
    assert_eq!(state.last_replay_at_ms, Some(i64::MIN));
    let pending = recovery_store
        .load_pending("w1", "node-a")
        .expect("load pending after overflow");
    assert!(pending.is_empty());
}

#[test]
fn run_replay_schedule_coordinated_with_state_store_respects_lease() {
    let client = sample_client();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let coordinator_store: Arc<dyn MembershipRevocationCoordinatorStateStore + Send + Sync> =
        Arc::new(InMemoryMembershipRevocationCoordinatorStateStore::new());
    let coordinator_a =
        StoreBackedMembershipRevocationScheduleCoordinator::new(Arc::clone(&coordinator_store));
    let coordinator_b =
        StoreBackedMembershipRevocationScheduleCoordinator::new(Arc::clone(&coordinator_store));
    let replay_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 2,
        max_retry_limit_exceeded_streak: 2,
    };

    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-target",
            1000,
            1,
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        ))
        .expect("append dead letter");

    let coordination_world_id = "w1::revocation-dead-letter-replay::node-target";
    assert!(
        coordinator_a
            .acquire(coordination_world_id, "node-a", 1000, 500)
            .expect("acquire lease with node-a")
    );

    let blocked = client
        .run_revocation_dead_letter_replay_schedule_coordinated_with_state_store(
            "w1",
            "node-target",
            "node-b",
            1001,
            100,
            &replay_policy,
            &recovery_store,
            &dead_letter_store,
            &replay_state_store,
            &coordinator_b,
            500,
        )
        .expect("run coordinated replay while lease held");
    assert_eq!(blocked, 0);

    coordinator_a
        .release(coordination_world_id, "node-a")
        .expect("release node-a lease");

    let replayed = client
        .run_revocation_dead_letter_replay_schedule_coordinated_with_state_store(
            "w1",
            "node-target",
            "node-b",
            1105,
            100,
            &replay_policy,
            &recovery_store,
            &dead_letter_store,
            &replay_state_store,
            &coordinator_b,
            500,
        )
        .expect("run coordinated replay after lease released");
    assert_eq!(replayed, 1);

    let state = replay_state_store
        .load_state("w1", "node-target")
        .expect("load replay state");
    assert_eq!(state.last_replay_at_ms, Some(1105));
}

#[test]
fn run_replay_schedule_with_state_store_rejects_invalid_policy() {
    let client = sample_client();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let invalid_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 0,
        max_retry_limit_exceeded_streak: 1,
    };

    let error = client
        .run_revocation_dead_letter_replay_schedule_with_state_store(
            "w1",
            "node-a",
            1000,
            100,
            &invalid_policy,
            &recovery_store,
            &dead_letter_store,
            &replay_state_store,
        )
        .expect_err("invalid policy should fail");
    let reason = match error {
        WorldError::DistributedValidationFailed { reason } => reason,
        other => panic!("unexpected error: {other:?}"),
    };
    assert!(reason.contains("max_replay_per_run"));
}

#[test]
fn recommend_replay_policy_increases_max_replay_for_high_backlog() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();

    for offset in 0..8 {
        dead_letter_store
            .append(&sample_dead_letter(
                "w1",
                "node-a",
                1000 + offset,
                1,
                MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
            ))
            .expect("append dead-letter backlog");
    }

    let current_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 2,
        max_retry_limit_exceeded_streak: 3,
    };
    let recommendation = client
        .recommend_revocation_dead_letter_replay_policy(
            "w1",
            "node-a",
            &current_policy,
            &replay_state_store,
            &recovery_store,
            &dead_letter_store,
            8,
            1,
            16,
            8,
        )
        .expect("recommend policy");

    assert!(recommendation.max_replay_per_run > current_policy.max_replay_per_run);
}

#[test]
fn recommend_replay_policy_decreases_max_replay_for_low_backlog_and_healthy_metrics() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
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
    dead_letter_store
        .append_delivery_metrics(
            "w1",
            "node-a",
            1100,
            &MembershipRevocationAlertDeliveryMetrics {
                attempted: 5,
                succeeded: 5,
                failed: 0,
                deferred: 0,
                buffered: 0,
                dropped_capacity: 0,
                dropped_retry_limit: 0,
                dead_lettered: 0,
            },
        )
        .expect("append healthy metrics #2");

    let current_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 5,
        max_retry_limit_exceeded_streak: 3,
    };
    let recommendation = client
        .recommend_revocation_dead_letter_replay_policy(
            "w1",
            "node-a",
            &current_policy,
            &replay_state_store,
            &recovery_store,
            &dead_letter_store,
            4,
            1,
            16,
            8,
        )
        .expect("recommend policy");

    assert_eq!(recommendation.max_replay_per_run, 4);
}

#[test]
fn recommend_replay_policy_reduces_retry_streak_when_capacity_preferred() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    replay_state_store
        .save_state(
            "w1",
            "node-a",
            &MembershipRevocationDeadLetterReplayScheduleState {
                last_replay_at_ms: Some(900),
                prefer_capacity_evicted: true,
            },
        )
        .expect("seed replay state");
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1000,
            1,
            MembershipRevocationAlertDeadLetterReason::CapacityEvicted,
        ))
        .expect("append capacity dead-letter");
    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1001,
            2,
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        ))
        .expect("append retry dead-letter");

    let current_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 4,
        max_retry_limit_exceeded_streak: 4,
    };
    let recommendation = client
        .recommend_revocation_dead_letter_replay_policy(
            "w1",
            "node-a",
            &current_policy,
            &replay_state_store,
            &recovery_store,
            &dead_letter_store,
            4,
            1,
            16,
            8,
        )
        .expect("recommend policy");
    assert_eq!(recommendation.max_retry_limit_exceeded_streak, 3);
}

#[test]
fn recommend_replay_policy_uses_large_ratio_without_saturation_distortion() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1000,
            1,
            MembershipRevocationAlertDeadLetterReason::CapacityEvicted,
        ))
        .expect("append capacity dead-letter");
    dead_letter_store
        .append_delivery_metrics(
            "w1",
            "node-a",
            1200,
            &MembershipRevocationAlertDeliveryMetrics {
                attempted: usize::MAX,
                succeeded: usize::MAX / 2,
                failed: 0,
                deferred: 0,
                buffered: 0,
                dropped_capacity: 0,
                dropped_retry_limit: 0,
                dead_lettered: usize::MAX / 2 + 1,
            },
        )
        .expect("append large metrics");

    let current_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 4,
        max_retry_limit_exceeded_streak: 6,
    };
    let recommendation = client
        .recommend_revocation_dead_letter_replay_policy(
            "w1",
            "node-a",
            &current_policy,
            &replay_state_store,
            &recovery_store,
            &dead_letter_store,
            1,
            1,
            16,
            8,
        )
        .expect("recommend policy");
    assert_eq!(recommendation.max_retry_limit_exceeded_streak, 5);
}

#[test]
fn run_coordinated_replay_with_adaptive_policy_returns_updated_policy() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let coordinator_store: Arc<dyn MembershipRevocationCoordinatorStateStore + Send + Sync> =
        Arc::new(InMemoryMembershipRevocationCoordinatorStateStore::new());
    let coordinator = StoreBackedMembershipRevocationScheduleCoordinator::new(coordinator_store);

    for offset in 0..6 {
        dead_letter_store
            .append(&sample_dead_letter(
                "w1",
                "node-target",
                1000 + offset,
                1,
                MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
            ))
            .expect("append dead-letter");
    }

    let current_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 2,
        max_retry_limit_exceeded_streak: 2,
    };
    let (replayed, updated_policy) = client
        .run_revocation_dead_letter_replay_schedule_coordinated_with_state_store_and_adaptive_policy(
            "w1",
            "node-target",
            "node-runner",
            1200,
            50,
            &current_policy,
            &replay_state_store,
            &recovery_store,
            &dead_letter_store,
            &coordinator,
            500,
            8,
            1,
            16,
            8,
        )
        .expect("run adaptive coordinated replay");

    assert!(updated_policy.max_replay_per_run >= current_policy.max_replay_per_run);
    assert_eq!(replayed, updated_policy.max_replay_per_run.min(6));
}

#[test]
fn recommend_guarded_policy_respects_cooldown_window() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    replay_state_store
        .save_state(
            "w1",
            "node-a",
            &MembershipRevocationDeadLetterReplayScheduleState {
                last_replay_at_ms: Some(1000),
                prefer_capacity_evicted: false,
            },
        )
        .expect("seed replay state");
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    for offset in 0..12 {
        dead_letter_store
            .append(&sample_dead_letter(
                "w1",
                "node-a",
                2000 + offset,
                1,
                MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
            ))
            .expect("append retry dead-letter backlog");
    }

    let current_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 3,
        max_retry_limit_exceeded_streak: 3,
    };
    let guarded = client
        .recommend_revocation_dead_letter_replay_policy_with_adaptive_guard(
            "w1",
            "node-a",
            1200,
            &current_policy,
            &replay_state_store,
            &recovery_store,
            &dead_letter_store,
            8,
            1,
            32,
            8,
            500,
            4,
            2,
        )
        .expect("recommend guarded policy");

    assert_eq!(
        guarded, current_policy,
        "guard should hold policy when cooldown window is active"
    );
}

#[test]
fn recommend_guarded_policy_rejects_cooldown_overflow_without_mutation() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    replay_state_store
        .save_state(
            "w1",
            "node-a",
            &MembershipRevocationDeadLetterReplayScheduleState {
                last_replay_at_ms: Some(i64::MIN),
                prefer_capacity_evicted: false,
            },
        )
        .expect("seed replay state");
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let current_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 3,
        max_retry_limit_exceeded_streak: 3,
    };

    let error = client
        .recommend_revocation_dead_letter_replay_policy_with_adaptive_guard(
            "w1",
            "node-a",
            1000,
            &current_policy,
            &replay_state_store,
            &recovery_store,
            &dead_letter_store,
            8,
            1,
            32,
            8,
            500,
            4,
            2,
        )
        .expect_err("cooldown overflow should fail");
    let message = format!("{error:?}");
    assert!(
        message.contains("policy cooldown elapsed overflow"),
        "unexpected error: {message}"
    );

    let state = replay_state_store
        .load_state("w1", "node-a")
        .expect("load replay state after overflow");
    assert_eq!(state.last_replay_at_ms, Some(i64::MIN));
}

#[test]
fn recommend_guarded_policy_clamps_step_change() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    for offset in 0..40 {
        dead_letter_store
            .append(&sample_dead_letter(
                "w1",
                "node-a",
                3000 + offset,
                1,
                MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
            ))
            .expect("append retry dead-letter backlog");
    }

    let current_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 12,
        max_retry_limit_exceeded_streak: 4,
    };
    let guarded = client
        .recommend_revocation_dead_letter_replay_policy_with_adaptive_guard(
            "w1",
            "node-a",
            5000,
            &current_policy,
            &replay_state_store,
            &recovery_store,
            &dead_letter_store,
            8,
            1,
            64,
            8,
            100,
            2,
            1,
        )
        .expect("recommend guarded policy");

    assert_eq!(
        guarded.max_replay_per_run, 14,
        "guard should cap max_replay_per_run single-step drift"
    );
    assert_eq!(
        guarded.max_retry_limit_exceeded_streak, 5,
        "guard should cap max_retry_limit_exceeded_streak single-step drift"
    );
}

#[test]
fn recommend_guarded_policy_rejects_invalid_guard_bounds() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let current_policy = MembershipRevocationDeadLetterReplayPolicy::default();

    let error = client
        .recommend_revocation_dead_letter_replay_policy_with_adaptive_guard(
            "w1",
            "node-a",
            1000,
            &current_policy,
            &replay_state_store,
            &recovery_store,
            &dead_letter_store,
            8,
            1,
            64,
            8,
            0,
            2,
            1,
        )
        .expect_err("invalid guard bounds should fail");
    let reason = match error {
        WorldError::DistributedValidationFailed { reason } => reason,
        other => panic!("unexpected error: {other:?}"),
    };
    assert!(reason.contains("policy_cooldown_ms"));
}

#[test]
fn run_coordinated_replay_with_guarded_adaptive_policy_uses_guarded_policy() {
    let client = sample_client();
    let replay_state_store = InMemoryMembershipRevocationDeadLetterReplayStateStore::new();
    replay_state_store
        .save_state(
            "w1",
            "node-target",
            &MembershipRevocationDeadLetterReplayScheduleState {
                last_replay_at_ms: Some(1000),
                prefer_capacity_evicted: false,
            },
        )
        .expect("seed replay state");
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let coordinator_store: Arc<dyn MembershipRevocationCoordinatorStateStore + Send + Sync> =
        Arc::new(InMemoryMembershipRevocationCoordinatorStateStore::new());
    let coordinator = StoreBackedMembershipRevocationScheduleCoordinator::new(coordinator_store);

    for offset in 0..6 {
        dead_letter_store
            .append(&sample_dead_letter(
                "w1",
                "node-target",
                4000 + offset,
                1,
                MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
            ))
            .expect("append dead-letter");
    }

    let current_policy = MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: 2,
        max_retry_limit_exceeded_streak: 2,
    };
    let (replayed, guarded_policy) = client
        .run_revocation_dead_letter_replay_schedule_coordinated_with_state_store_and_guarded_adaptive_policy(
            "w1",
            "node-target",
            "node-runner",
            1200,
            50,
            &current_policy,
            &replay_state_store,
            &recovery_store,
            &dead_letter_store,
            &coordinator,
            500,
            8,
            1,
            16,
            8,
            500,
            4,
            2,
        )
        .expect("run guarded adaptive replay");

    assert_eq!(
        guarded_policy, current_policy,
        "cooldown should keep policy unchanged"
    );
    assert_eq!(replayed, 2);
}
