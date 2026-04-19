use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::distributed_net::{DistributedNetwork, InMemoryNetwork};

use crate::error::WorldError;
use crate::*;

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

fn sample_governance_audit_record(
    world_id: &str,
    node_id: &str,
    audited_at_ms: i64,
    governance_level: MembershipRevocationDeadLetterReplayRollbackGovernanceLevel,
    rollback_streak: usize,
) -> MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord {
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord {
        world_id: world_id.to_string(),
        node_id: node_id.to_string(),
        audited_at_ms,
        governance_level,
        rollback_streak,
        rolled_back: rollback_streak > 0,
        applied_policy: MembershipRevocationDeadLetterReplayPolicy {
            max_replay_per_run: 4,
            max_retry_limit_exceeded_streak: 2,
        },
        alert_emitted: false,
    }
}

#[derive(Debug, Clone, Default)]
struct ReplaceFailingGovernanceAuditRetentionStore {
    inner: InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore,
}

impl ReplaceFailingGovernanceAuditRetentionStore {
    fn new() -> Self {
        Self {
            inner:
                InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new(),
        }
    }
}

impl MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore
    for ReplaceFailingGovernanceAuditRetentionStore
{
    fn append(
        &self,
        world_id: &str,
        node_id: &str,
        record: &MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord,
    ) -> Result<(), WorldError> {
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
            &self.inner,
            world_id,
            node_id,
            record,
        )
    }

    fn list(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord>, WorldError>
    {
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::list(
            &self.inner,
            world_id,
            node_id,
        )
    }

    fn replace(
        &self,
        _world_id: &str,
        _node_id: &str,
        _records: &[MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord],
    ) -> Result<(), WorldError> {
        Err(WorldError::Io(
            "injected failure for governance audit retention replace".into(),
        ))
    }
}

#[test]
fn governance_audit_archive_prune_rewrites_file_store() {
    let client = sample_client();
    let root = temp_membership_dir("governance-audit-retention-store");
    fs::create_dir_all(&root).expect("create temp dir");
    let store =
        FileMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new(&root)
            .expect("create governance audit retention file store");
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
        &store,
        "w1",
        "node-a",
        &sample_governance_audit_record(
            "w1",
            "node-a",
            1_000,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Normal,
            0,
        ),
    )
    .expect("append record 1");
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
        &store,
        "w1",
        "node-a",
        &sample_governance_audit_record(
            "w1",
            "node-a",
            1_100,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Stable,
            1,
        ),
    )
    .expect("append record 2");
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
        &store,
        "w1",
        "node-a",
        &sample_governance_audit_record(
            "w1",
            "node-a",
            1_200,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
            2,
        ),
    )
    .expect("append record 3");

    let retention_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionPolicy {
            max_records: 1,
            max_age_ms: 250,
        };
    let report = client
        .prune_revocation_dead_letter_replay_rollback_governance_audit_archive(
            "w1",
            "node-a",
            1_400,
            &retention_policy,
            &store,
        )
        .expect("prune governance audits");
    assert_eq!(report.before, 3);
    assert_eq!(report.after, 1);
    assert_eq!(report.pruned_by_age, 2);
    assert_eq!(report.pruned_by_capacity, 0);

    let kept = MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::list(
        &store, "w1", "node-a",
    )
    .expect("list pruned records");
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].audited_at_ms, 1_200);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn governance_recovery_drill_schedule_state_store_file_round_trip() {
    let root = temp_membership_dir("governance-recovery-drill-schedule-state-store");
    fs::create_dir_all(&root).expect("create temp dir");
    let store = FileMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore::new(
        &root,
    )
    .expect("create schedule state store");
    let state = MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleState {
        last_drill_at_ms: Some(1_234),
    };
    store
        .save_state("w1", "node-a", &state)
        .expect("save schedule state");
    let loaded = store
        .load_state("w1", "node-a")
        .expect("load schedule state");
    assert_eq!(loaded, state);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn governance_audit_archive_prune_rejects_age_overflow_without_mutation() {
    let client = sample_client();
    let store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
        &store,
        "w1",
        "node-a",
        &sample_governance_audit_record(
            "w1",
            "node-a",
            i64::MAX,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Normal,
            0,
        ),
    )
    .expect("append extreme governance audit");

    let retention_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionPolicy {
            max_records: 8,
            max_age_ms: 1_000,
        };
    let err = client
        .prune_revocation_dead_letter_replay_rollback_governance_audit_archive(
            "w1",
            "node-a",
            i64::MIN,
            &retention_policy,
            &store,
        )
        .expect_err("age overflow should fail");
    assert!(matches!(
        err,
        WorldError::DistributedValidationFailed { ref reason }
            if reason.contains("audit age overflow")
    ));

    let records = MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::list(
        &store, "w1", "node-a",
    )
    .expect("list records after overflow");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].audited_at_ms, i64::MAX);
}

#[test]
fn governance_recovery_drill_schedule_executes_when_due_and_persists_state() {
    let client = sample_client();
    let schedule_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore::new();
    let rollback_alert_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackAlertStateStore::new();
    let rollback_governance_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore::new();
    let rollback_governance_audit_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();

    rollback_alert_state_store
        .save_alert_state(
            "w1",
            "node-a",
            &MembershipRevocationDeadLetterReplayRollbackAlertState {
                last_alert_at_ms: Some(900),
            },
        )
        .expect("save alert state");
    rollback_governance_state_store
        .save_governance_state(
            "w1",
            "node-a",
            &MembershipRevocationDeadLetterReplayRollbackGovernanceState {
                rollback_streak: 2,
                last_level: MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
                last_level_updated_at_ms: Some(900),
            },
        )
        .expect("save governance state");
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
        &rollback_governance_audit_store,
        "w1",
        "node-a",
        &sample_governance_audit_record(
            "w1",
            "node-a",
            800,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Normal,
            0,
        ),
    )
    .expect("append record 1");
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
        &rollback_governance_audit_store,
        "w1",
        "node-a",
        &sample_governance_audit_record(
            "w1",
            "node-a",
            900,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Stable,
            1,
        ),
    )
    .expect("append record 2");
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
        &rollback_governance_audit_store,
        "w1",
        "node-a",
        &sample_governance_audit_record(
            "w1",
            "node-a",
            1_000,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
            2,
        ),
    )
    .expect("append record 3");

    let policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillSchedulePolicy {
            drill_interval_ms: 100,
            recent_audit_limit: 2,
        };

    let first = client
        .run_revocation_dead_letter_replay_rollback_governance_recovery_drill_schedule(
            "w1",
            "node-a",
            1_100,
            &policy,
            &schedule_state_store,
            &rollback_alert_state_store,
            &rollback_governance_state_store,
            &rollback_governance_audit_store,
        )
        .expect("first drill schedule run");
    assert!(first.drill_due);
    assert!(first.drill_executed);
    assert_eq!(first.next_due_at_ms, Some(1_200));
    assert_eq!(
        first
            .drill_report
            .as_ref()
            .expect("first drill report")
            .recent_audits
            .len(),
        2
    );

    let second = client
        .run_revocation_dead_letter_replay_rollback_governance_recovery_drill_schedule(
            "w1",
            "node-a",
            1_150,
            &policy,
            &schedule_state_store,
            &rollback_alert_state_store,
            &rollback_governance_state_store,
            &rollback_governance_audit_store,
        )
        .expect("second drill schedule run");
    assert!(!second.drill_due);
    assert!(!second.drill_executed);
    assert!(second.drill_report.is_none());
    assert_eq!(second.next_due_at_ms, Some(1_200));

    let state = schedule_state_store
        .load_state("w1", "node-a")
        .expect("load schedule state");
    assert_eq!(state.last_drill_at_ms, Some(1_100));
}

#[test]
fn governance_recovery_drill_schedule_rejects_next_due_overflow_without_mutation() {
    let client = sample_client();
    let schedule_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore::new();
    schedule_state_store
        .save_state(
            "w1",
            "node-a",
            &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleState {
                last_drill_at_ms: Some(i64::MAX - 1),
            },
        )
        .expect("seed schedule state");
    let rollback_alert_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackAlertStateStore::new();
    let rollback_governance_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore::new();
    let rollback_governance_audit_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();

    let policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillSchedulePolicy {
            drill_interval_ms: 100,
            recent_audit_limit: 1,
        };
    let err = client
        .run_revocation_dead_letter_replay_rollback_governance_recovery_drill_schedule(
            "w1",
            "node-a",
            i64::MAX - 1,
            &policy,
            &schedule_state_store,
            &rollback_alert_state_store,
            &rollback_governance_state_store,
            &rollback_governance_audit_store,
        )
        .expect_err("next due overflow should fail");
    assert!(matches!(
        err,
        WorldError::DistributedValidationFailed { ref reason }
            if reason.contains("next_due_at_ms overflow")
    ));

    let state = schedule_state_store
        .load_state("w1", "node-a")
        .expect("load schedule state after overflow");
    assert_eq!(state.last_drill_at_ms, Some(i64::MAX - 1));
}

#[test]
fn governance_recovery_drill_schedule_rejects_invalid_policy() {
    let client = sample_client();
    let schedule_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore::new();
    let rollback_alert_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackAlertStateStore::new();
    let rollback_governance_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore::new();
    let rollback_governance_audit_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();

    let invalid_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillSchedulePolicy {
            drill_interval_ms: 0,
            recent_audit_limit: 1,
        };
    let error = client
        .run_revocation_dead_letter_replay_rollback_governance_recovery_drill_schedule(
            "w1",
            "node-a",
            1_200,
            &invalid_policy,
            &schedule_state_store,
            &rollback_alert_state_store,
            &rollback_governance_state_store,
            &rollback_governance_audit_store,
        )
        .expect_err("invalid schedule policy should fail");
    let message = format!("{error:?}");
    assert!(
        message.contains("drill_interval_ms must be positive"),
        "unexpected error: {message}"
    );
}

#[test]
fn governance_archive_and_drill_schedule_orchestration_runs_prune_then_drill() {
    let client = sample_client();
    let schedule_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore::new();
    let rollback_alert_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackAlertStateStore::new();
    let rollback_governance_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore::new();
    let rollback_governance_audit_retention_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();

    rollback_alert_state_store
        .save_alert_state(
            "w1",
            "node-a",
            &MembershipRevocationDeadLetterReplayRollbackAlertState {
                last_alert_at_ms: Some(950),
            },
        )
        .expect("save alert state");
    rollback_governance_state_store
        .save_governance_state(
            "w1",
            "node-a",
            &MembershipRevocationDeadLetterReplayRollbackGovernanceState {
                rollback_streak: 1,
                last_level: MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Stable,
                last_level_updated_at_ms: Some(950),
            },
        )
        .expect("save governance state");
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
        &rollback_governance_audit_retention_store,
        "w1",
        "node-a",
        &sample_governance_audit_record(
            "w1",
            "node-a",
            700,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Normal,
            0,
        ),
    )
    .expect("append record 1");
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
        &rollback_governance_audit_retention_store,
        "w1",
        "node-a",
        &sample_governance_audit_record(
            "w1",
            "node-a",
            900,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Stable,
            1,
        ),
    )
    .expect("append record 2");

    let retention_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionPolicy {
            max_records: 1,
            max_age_ms: 10_000,
        };
    let drill_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillSchedulePolicy {
            drill_interval_ms: 100,
            recent_audit_limit: 5,
        };
    let run_report = client
        .run_revocation_dead_letter_replay_rollback_governance_archive_retention_and_recovery_drill_schedule(
            "w1",
            "node-a",
            1_000,
            &retention_policy,
            &drill_policy,
            &rollback_governance_audit_retention_store,
            &schedule_state_store,
            &rollback_alert_state_store,
            &rollback_governance_state_store,
            &rollback_governance_audit_retention_store,
        )
        .expect("run archive and drill schedule orchestration");
    assert_eq!(run_report.prune_report.before, 2);
    assert_eq!(run_report.prune_report.after, 1);
    assert!(run_report.drill_run_report.drill_executed);
    assert_eq!(
        run_report
            .drill_run_report
            .drill_report
            .expect("drill report")
            .recent_audits
            .len(),
        1
    );
}

#[test]
fn governance_audit_tiered_offload_moves_records_by_age_and_capacity() {
    let client = sample_client();
    let hot_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();
    let cold_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();
    for (index, audited_at_ms) in [100, 200, 300, 900, 950].into_iter().enumerate() {
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
            &hot_store,
            "w1",
            "node-a",
            &sample_governance_audit_record(
                "w1",
                "node-a",
                audited_at_ms,
                MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Stable,
                index,
            ),
        )
        .expect("append hot record");
    }

    let offload_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadPolicy {
            hot_max_records: 2,
            offload_min_age_ms: 700,
            max_offload_records: 20,
        };
    let report = client
        .offload_revocation_dead_letter_replay_rollback_governance_audit_archive_tiered(
            "w1",
            "node-a",
            1_000,
            &offload_policy,
            &hot_store,
            &cold_store,
        )
        .expect("offload tiered archive");
    assert_eq!(report.offloaded, 3);
    assert_eq!(report.offloaded_by_age, 3);
    assert_eq!(report.offloaded_by_capacity, 0);
    assert_eq!(report.kept_due_to_rate_limit, 0);

    let hot_records =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::list(
            &hot_store, "w1", "node-a",
        )
        .expect("list hot");
    assert_eq!(hot_records.len(), 2);
    assert_eq!(hot_records[0].audited_at_ms, 900);
    assert_eq!(hot_records[1].audited_at_ms, 950);
    let cold_records =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::list(
            &cold_store,
            "w1",
            "node-a",
        )
        .expect("list cold");
    assert_eq!(cold_records.len(), 3);
    assert_eq!(cold_records[0].audited_at_ms, 100);
    assert_eq!(cold_records[1].audited_at_ms, 200);
    assert_eq!(cold_records[2].audited_at_ms, 300);
}

#[test]
fn governance_audit_tiered_offload_respects_rate_limit() {
    let client = sample_client();
    let hot_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();
    let cold_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();
    for audited_at_ms in [900, 910, 920, 930, 940] {
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
            &hot_store,
            "w1",
            "node-a",
            &sample_governance_audit_record(
                "w1",
                "node-a",
                audited_at_ms,
                MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Normal,
                0,
            ),
        )
        .expect("append hot record");
    }

    let offload_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadPolicy {
            hot_max_records: 2,
            offload_min_age_ms: 10_000,
            max_offload_records: 1,
        };
    let report = client
        .offload_revocation_dead_letter_replay_rollback_governance_audit_archive_tiered(
            "w1",
            "node-a",
            1_000,
            &offload_policy,
            &hot_store,
            &cold_store,
        )
        .expect("offload with rate limit");
    assert_eq!(report.offloaded, 1);
    assert_eq!(report.offloaded_by_age, 0);
    assert_eq!(report.offloaded_by_capacity, 1);
    assert_eq!(report.kept_due_to_rate_limit, 2);

    let hot_records =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::list(
            &hot_store, "w1", "node-a",
        )
        .expect("list hot");
    assert_eq!(hot_records.len(), 4);
    let cold_records =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::list(
            &cold_store,
            "w1",
            "node-a",
        )
        .expect("list cold");
    assert_eq!(cold_records.len(), 1);
}

#[test]
fn governance_audit_tiered_offload_rejects_age_overflow_without_mutation() {
    let client = sample_client();
    let hot_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();
    let cold_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
        &hot_store,
        "w1",
        "node-a",
        &sample_governance_audit_record(
            "w1",
            "node-a",
            1,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Stable,
            1,
        ),
    )
    .expect("append hot record");
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
        &cold_store,
        "w1",
        "node-a",
        &sample_governance_audit_record(
            "w1",
            "node-a",
            800,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Normal,
            0,
        ),
    )
    .expect("append cold record");

    let offload_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadPolicy {
            hot_max_records: 1,
            offload_min_age_ms: 100,
            max_offload_records: 10,
        };
    let error = client
        .offload_revocation_dead_letter_replay_rollback_governance_audit_archive_tiered(
            "w1",
            "node-a",
            i64::MIN,
            &offload_policy,
            &hot_store,
            &cold_store,
        )
        .expect_err("offload age underflow should fail");
    let message = format!("{error:?}");
    assert!(
        message.contains("tiered offload audit age overflow"),
        "unexpected error: {message}"
    );

    let hot_records =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::list(
            &hot_store, "w1", "node-a",
        )
        .expect("list hot after overflow");
    assert_eq!(hot_records.len(), 1);
    assert_eq!(hot_records[0].audited_at_ms, 1);
    let cold_records =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::list(
            &cold_store,
            "w1",
            "node-a",
        )
        .expect("list cold after overflow");
    assert_eq!(cold_records.len(), 1);
    assert_eq!(cold_records[0].audited_at_ms, 800);
}

#[test]
fn governance_audit_tiered_offload_rolls_back_cold_layer_when_hot_replace_fails() {
    let client = sample_client();
    let hot_store = ReplaceFailingGovernanceAuditRetentionStore::new();
    let cold_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::new();
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
        &hot_store,
        "w1",
        "node-a",
        &sample_governance_audit_record(
            "w1",
            "node-a",
            100,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
            3,
        ),
    )
    .expect("append hot");
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
        &cold_store,
        "w1",
        "node-a",
        &sample_governance_audit_record(
            "w1",
            "node-a",
            50,
            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Normal,
            0,
        ),
    )
    .expect("append cold");

    let offload_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadPolicy {
            hot_max_records: 1,
            offload_min_age_ms: 10,
            max_offload_records: 10,
        };
    let error = client
        .offload_revocation_dead_letter_replay_rollback_governance_audit_archive_tiered(
            "w1",
            "node-a",
            1_000,
            &offload_policy,
            &hot_store,
            &cold_store,
        )
        .expect_err("hot replace failure should bubble");
    let message = format!("{error:?}");
    assert!(
        message.contains("cold layer rolled back"),
        "unexpected error: {message}"
    );

    let cold_records =
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::list(
            &cold_store,
            "w1",
            "node-a",
        )
        .expect("list cold after rollback");
    assert_eq!(cold_records.len(), 1);
    assert_eq!(cold_records[0].audited_at_ms, 50);
}

#[test]
fn governance_recovery_drill_alert_emits_and_honors_cooldown() {
    let client = sample_client();
    let alert_state_store =
        InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertStateStore::new();
    let alert_sink = InMemoryMembershipRevocationAlertSink::new();
    let alert_policy =
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertPolicy {
            max_alert_silence_ms: 100,
            rollback_streak_threshold: 2,
            alert_cooldown_ms: 200,
        };
    let drill_run_report =
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduledRunReport {
            world_id: "w1".to_string(),
            node_id: "node-a".to_string(),
            scheduled_at_ms: 1_000,
            drill_due: true,
            drill_executed: true,
            next_due_at_ms: Some(1_100),
            drill_report: Some(
                MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillReport {
                    world_id: "w1".to_string(),
                    node_id: "node-a".to_string(),
                    drill_at_ms: 1_000,
                    alert_state: MembershipRevocationDeadLetterReplayRollbackAlertState {
                        last_alert_at_ms: None,
                    },
                    governance_state: MembershipRevocationDeadLetterReplayRollbackGovernanceState {
                        rollback_streak: 3,
                        last_level:
                            MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
                        last_level_updated_at_ms: Some(980),
                    },
                    recent_audits: vec![sample_governance_audit_record(
                        "w1",
                        "node-a",
                        990,
                        MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency,
                        3,
                    )],
                    has_emergency_history: true,
                },
            ),
        };

    let first = client
        .emit_revocation_dead_letter_replay_rollback_governance_recovery_drill_alert_if_needed(
            "w1",
            "node-a",
            1_000,
            &drill_run_report,
            &alert_policy,
            &alert_state_store,
            &alert_sink,
        )
        .expect("first alert run");
    assert!(first.anomaly_detected);
    assert!(first.alert_emitted);
    assert!(!first.cooldown_blocked);

    let second = client
        .emit_revocation_dead_letter_replay_rollback_governance_recovery_drill_alert_if_needed(
            "w1",
            "node-a",
            1_050,
            &drill_run_report,
            &alert_policy,
            &alert_state_store,
            &alert_sink,
        )
        .expect("second alert run");
    assert!(second.anomaly_detected);
    assert!(!second.alert_emitted);
    assert!(second.cooldown_blocked);

    let alerts = alert_sink.list().expect("list emitted alerts");
    assert_eq!(alerts.len(), 1);
    assert_eq!(alerts[0].code, "rollback_governance_recovery_drill_anomaly");
}
