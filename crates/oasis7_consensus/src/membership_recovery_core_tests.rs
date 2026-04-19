use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::distributed_net::{DistributedNetwork, InMemoryNetwork};
use oasis7_distfs::LocalCasStore;

use crate::{
    error::WorldError, FileMembershipRevocationAlertDeadLetterStore,
    FileMembershipRevocationAlertRecoveryStore, FileMembershipRevocationCoordinatorStateStore,
    InMemoryMembershipRevocationAlertDeadLetterStore,
    InMemoryMembershipRevocationAlertRecoveryStore,
    InMemoryMembershipRevocationCoordinatorStateStore,
    InMemoryMembershipRevocationScheduleStateStore, MembershipDirectorySignerKeyring,
    MembershipRevocationAlertAckRetryPolicy, MembershipRevocationAlertDeadLetterReason,
    MembershipRevocationAlertDeadLetterRecord, MembershipRevocationAlertDeadLetterStore,
    MembershipRevocationAlertDeliveryMetrics, MembershipRevocationAlertPolicy,
    MembershipRevocationAlertRecoveryStore, MembershipRevocationAlertSeverity,
    MembershipRevocationAlertSink, MembershipRevocationAnomalyAlert,
    MembershipRevocationCoordinatorLeaseState, MembershipRevocationCoordinatorStateStore,
    MembershipRevocationDeadLetterRetention, MembershipRevocationPendingAlert,
    MembershipRevocationReconcilePolicy, MembershipRevocationReconcileSchedulePolicy,
    MembershipRevocationReconcileScheduleState, MembershipRevocationScheduleCoordinator,
    MembershipRevocationScheduleStateStore, MembershipSyncClient,
    StoreBackedMembershipRevocationScheduleCoordinator,
};

fn sample_client() -> MembershipSyncClient {
    let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
    MembershipSyncClient::new(network)
}

fn sample_keyring() -> MembershipDirectorySignerKeyring {
    let mut keyring = MembershipDirectorySignerKeyring::new();
    keyring
        .add_hmac_sha256_key("k1", "membership-secret-v1")
        .expect("add k1");
    keyring.set_active_key("k1").expect("set active k1");
    keyring
}

fn temp_membership_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("oasis7-consensus-{prefix}-{nanos}"))
}

fn sample_alert(
    world_id: &str,
    node_id: &str,
    detected_at_ms: i64,
) -> MembershipRevocationAnomalyAlert {
    MembershipRevocationAnomalyAlert {
        world_id: world_id.to_string(),
        node_id: node_id.to_string(),
        detected_at_ms,
        severity: MembershipRevocationAlertSeverity::Warn,
        code: "reconcile_diverged".to_string(),
        message: "membership revocation reconcile diverged".to_string(),
        drained: 1,
        diverged: 1,
        rejected: 0,
    }
}

fn sample_pending_alert(
    world_id: &str,
    node_id: &str,
    detected_at_ms: i64,
    attempt: usize,
) -> MembershipRevocationPendingAlert {
    MembershipRevocationPendingAlert {
        alert: sample_alert(world_id, node_id, detected_at_ms),
        attempt,
        next_retry_at_ms: detected_at_ms,
        last_error: None,
    }
}

fn sample_dead_letter(
    world_id: &str,
    node_id: &str,
    detected_at_ms: i64,
    reason: MembershipRevocationAlertDeadLetterReason,
) -> MembershipRevocationAlertDeadLetterRecord {
    MembershipRevocationAlertDeadLetterRecord {
        world_id: world_id.to_string(),
        node_id: node_id.to_string(),
        dropped_at_ms: detected_at_ms,
        reason,
        pending_alert: sample_pending_alert(world_id, node_id, detected_at_ms, 1),
    }
}

#[derive(Default, Clone)]
struct FailOnceAlertSink {
    fail_once: Arc<Mutex<bool>>,
    emitted: Arc<Mutex<Vec<MembershipRevocationAnomalyAlert>>>,
}

impl FailOnceAlertSink {
    fn new() -> Self {
        Self {
            fail_once: Arc::new(Mutex::new(true)),
            emitted: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn emitted(&self) -> Vec<MembershipRevocationAnomalyAlert> {
        self.emitted.lock().expect("lock emitted").clone()
    }
}

impl MembershipRevocationAlertSink for FailOnceAlertSink {
    fn emit(&self, alert: &MembershipRevocationAnomalyAlert) -> Result<(), WorldError> {
        let mut fail_once = self.fail_once.lock().expect("lock fail_once");
        if *fail_once {
            *fail_once = false;
            return Err(WorldError::Io("simulated alert sink failure".to_string()));
        }

        let mut emitted = self.emitted.lock().expect("lock emitted");
        emitted.push(alert.clone());
        Ok(())
    }
}

#[derive(Default, Clone)]
struct AlwaysFailAlertSink {
    emitted_attempts: Arc<Mutex<usize>>,
}

impl AlwaysFailAlertSink {
    fn new() -> Self {
        Self::default()
    }

    fn attempts(&self) -> usize {
        *self.emitted_attempts.lock().expect("lock attempts")
    }
}

impl MembershipRevocationAlertSink for AlwaysFailAlertSink {
    fn emit(&self, _alert: &MembershipRevocationAnomalyAlert) -> Result<(), WorldError> {
        let mut attempts = self.emitted_attempts.lock().expect("lock attempts");
        *attempts = attempts.saturating_add(1);
        Err(WorldError::Io(
            "simulated persistent alert sink failure".to_string(),
        ))
    }
}

#[test]
fn file_coordinator_state_store_round_trip() {
    let root = temp_membership_dir("revocation-coordinator-state-store");
    fs::create_dir_all(&root).expect("create temp dir");

    let store = FileMembershipRevocationCoordinatorStateStore::new(&root).expect("create store");
    let state = MembershipRevocationCoordinatorLeaseState {
        holder_node_id: "node-a".to_string(),
        expires_at_ms: 1200,
    };

    store.save("w1", &state).expect("save state");
    let loaded = store.load("w1").expect("load state");
    assert_eq!(loaded, Some(state.clone()));

    store.clear("w1").expect("clear state");
    let missing = store.load("w1").expect("load missing");
    assert_eq!(missing, None);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_alert_recovery_store_loads_legacy_alert_format() {
    let root = temp_membership_dir("revocation-alert-legacy-format");
    fs::create_dir_all(&root).expect("create temp dir");

    let legacy_path = root.join("w1.node-a.revocation-alert-pending.json");
    let legacy = vec![sample_alert("w1", "node-a", 1000)];
    let legacy_bytes = serde_json::to_vec(&legacy).expect("serialize legacy alerts");
    fs::write(&legacy_path, legacy_bytes).expect("write legacy alerts");

    let store = FileMembershipRevocationAlertRecoveryStore::new(&root).expect("create store");
    let loaded = store.load_pending("w1", "node-a").expect("load pending");

    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].attempt, 0);
    assert_eq!(loaded[0].next_retry_at_ms, 0);
    assert_eq!(loaded[0].alert.detected_at_ms, 1000);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_dead_letter_store_appends_and_lists() {
    let root = temp_membership_dir("revocation-alert-dead-letter-file-store");
    fs::create_dir_all(&root).expect("create temp dir");

    let store = FileMembershipRevocationAlertDeadLetterStore::new(&root).expect("create store");
    let record = MembershipRevocationAlertDeadLetterRecord {
        world_id: "w1".to_string(),
        node_id: "node-a".to_string(),
        dropped_at_ms: 1234,
        reason: MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        pending_alert: MembershipRevocationPendingAlert {
            alert: sample_alert("w1", "node-a", 1230),
            attempt: 3,
            next_retry_at_ms: 1234,
            last_error: Some("io timeout".to_string()),
        },
    };

    store.append(&record).expect("append dead letter");
    let listed = store.list("w1", "node-a").expect("list dead letters");
    assert_eq!(listed, vec![record]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_dead_letter_store_metrics_export_round_trip() {
    let root = temp_membership_dir("revocation-alert-metrics-file-store");
    fs::create_dir_all(&root).expect("create temp dir");

    let store = FileMembershipRevocationAlertDeadLetterStore::new(&root).expect("create store");
    let metrics = MembershipRevocationAlertDeliveryMetrics {
        attempted: 3,
        succeeded: 2,
        failed: 1,
        deferred: 1,
        buffered: 1,
        dropped_capacity: 0,
        dropped_retry_limit: 0,
        dead_lettered: 0,
    };

    store
        .append_delivery_metrics("w1", "node-a", 1200, &metrics)
        .expect("append metrics");
    let listed = store
        .list_delivery_metrics("w1", "node-a")
        .expect("list metrics");
    assert_eq!(listed, vec![(1200, metrics)]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn in_memory_dead_letter_store_retention_keeps_recent_entries() {
    let store = InMemoryMembershipRevocationAlertDeadLetterStore::with_retention(
        MembershipRevocationDeadLetterRetention {
            max_dead_letter_records_per_stream: 2,
            max_delivery_metrics_per_stream: 2,
        },
    );

    store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1000,
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        ))
        .expect("append record 1");
    store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1001,
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        ))
        .expect("append record 2");
    store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1002,
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        ))
        .expect("append record 3");

    let dead_letters = store.list("w1", "node-a").expect("list retained records");
    assert_eq!(dead_letters.len(), 2);
    assert_eq!(dead_letters[0].dropped_at_ms, 1001);
    assert_eq!(dead_letters[1].dropped_at_ms, 1002);

    for exported_at_ms in [2000_i64, 2001_i64, 2002_i64] {
        let metrics = MembershipRevocationAlertDeliveryMetrics {
            attempted: exported_at_ms as usize,
            ..MembershipRevocationAlertDeliveryMetrics::default()
        };
        store
            .append_delivery_metrics("w1", "node-a", exported_at_ms, &metrics)
            .expect("append metrics");
    }

    let listed_metrics = store
        .list_delivery_metrics("w1", "node-a")
        .expect("list retained metrics");
    assert_eq!(listed_metrics.len(), 2);
    assert_eq!(listed_metrics[0].0, 2001);
    assert_eq!(listed_metrics[1].0, 2002);
}

#[test]
fn file_dead_letter_store_retention_compacts_records_to_archive() {
    let root = temp_membership_dir("revocation-alert-dead-letter-retention");
    fs::create_dir_all(&root).expect("create temp dir");

    let store = FileMembershipRevocationAlertDeadLetterStore::with_retention(
        &root,
        MembershipRevocationDeadLetterRetention {
            max_dead_letter_records_per_stream: 2,
            max_delivery_metrics_per_stream: 2,
        },
    )
    .expect("create store");

    for dropped_at_ms in [1000_i64, 1001_i64, 1002_i64] {
        store
            .append(&sample_dead_letter(
                "w1",
                "node-a",
                dropped_at_ms,
                MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
            ))
            .expect("append record");
    }

    let active = store.list("w1", "node-a").expect("list retained records");
    assert_eq!(active.len(), 3);
    assert_eq!(active[0].dropped_at_ms, 1000);
    assert_eq!(active[1].dropped_at_ms, 1001);
    assert_eq!(active[2].dropped_at_ms, 1002);

    let archive_path = root.join("w1.node-a.revocation-alert-dead-letter.archive.refs.jsonl");
    let cas_store = LocalCasStore::new(root.join("cas"));
    let archive =
        crate::tiered_file_log::collect_cold_jsonl_lines(archive_path.as_path(), &cas_store)
            .expect("read archived dead-letter refs");
    let archived: Vec<MembershipRevocationAlertDeadLetterRecord> = archive
        .iter()
        .map(|line| serde_json::from_str(line).expect("decode archived dead-letter line"))
        .collect();
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].dropped_at_ms, 1000);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_dead_letter_store_retention_compacts_metrics_to_archive() {
    let root = temp_membership_dir("revocation-alert-metrics-retention");
    fs::create_dir_all(&root).expect("create temp dir");

    let store = FileMembershipRevocationAlertDeadLetterStore::with_retention(
        &root,
        MembershipRevocationDeadLetterRetention {
            max_dead_letter_records_per_stream: 2,
            max_delivery_metrics_per_stream: 2,
        },
    )
    .expect("create store");

    for exported_at_ms in [1200_i64, 1201_i64, 1202_i64] {
        let metrics = MembershipRevocationAlertDeliveryMetrics {
            attempted: exported_at_ms as usize,
            ..MembershipRevocationAlertDeliveryMetrics::default()
        };
        store
            .append_delivery_metrics("w1", "node-a", exported_at_ms, &metrics)
            .expect("append metrics");
    }

    let active = store
        .list_delivery_metrics("w1", "node-a")
        .expect("list retained metrics");
    assert_eq!(active.len(), 3);
    assert_eq!(active[0].0, 1200);
    assert_eq!(active[1].0, 1201);
    assert_eq!(active[2].0, 1202);

    let archive_path = root.join("w1.node-a.revocation-alert-delivery-metrics.archive.refs.jsonl");
    let cas_store = LocalCasStore::new(root.join("cas"));
    let archive =
        crate::tiered_file_log::collect_cold_jsonl_lines(archive_path.as_path(), &cas_store)
            .expect("read archived metrics refs");
    let archived_export_times: Vec<i64> = archive
        .iter()
        .map(|line| {
            let value: serde_json::Value =
                serde_json::from_str(line).expect("decode archived metrics line");
            value["exported_at_ms"]
                .as_i64()
                .expect("archived exported_at_ms should be i64")
        })
        .collect();
    assert_eq!(archived_export_times, vec![1200]);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_dead_letter_store_replace_rebuilds_archive_without_stale_cold_records() {
    let root = temp_membership_dir("revocation-alert-dead-letter-replace-rebuild");
    fs::create_dir_all(&root).expect("create temp dir");

    let store = FileMembershipRevocationAlertDeadLetterStore::with_retention(
        &root,
        MembershipRevocationDeadLetterRetention {
            max_dead_letter_records_per_stream: 2,
            max_delivery_metrics_per_stream: 2,
        },
    )
    .expect("create store");

    let record_1000 = sample_dead_letter(
        "w1",
        "node-a",
        1000,
        MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
    );
    let record_1001 = sample_dead_letter(
        "w1",
        "node-a",
        1001,
        MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
    );
    let record_1002 = sample_dead_letter(
        "w1",
        "node-a",
        1002,
        MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
    );
    store.append(&record_1000).expect("append record 1000");
    store.append(&record_1001).expect("append record 1001");
    store.append(&record_1002).expect("append record 1002");

    store
        .replace("w1", "node-a", std::slice::from_ref(&record_1002))
        .expect("replace should rebuild retention files from remaining records");

    let listed = store.list("w1", "node-a").expect("list after replace");
    assert_eq!(listed, vec![record_1002]);

    let archive_path = root.join("w1.node-a.revocation-alert-dead-letter.archive.refs.jsonl");
    let cas_store = LocalCasStore::new(root.join("cas"));
    let archived =
        crate::tiered_file_log::collect_cold_jsonl_lines(archive_path.as_path(), &cas_store)
            .expect("read archive refs after replace");
    assert!(
        archived.is_empty(),
        "replace should clear stale cold refs when overflow disappears"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn replay_revocation_dead_letters_moves_records_to_pending() {
    let client = sample_client();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();

    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1000,
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        ))
        .expect("append dead letter 1");
    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1001,
            MembershipRevocationAlertDeadLetterReason::CapacityEvicted,
        ))
        .expect("append dead letter 2");

    let first = client
        .replay_revocation_dead_letters("w1", "node-a", 1, &recovery_store, &dead_letter_store)
        .expect("first replay");
    assert_eq!(first, 1);

    let pending_after_first = recovery_store
        .load_pending("w1", "node-a")
        .expect("load pending after first replay");
    assert_eq!(pending_after_first.len(), 1);
    assert_eq!(pending_after_first[0].alert.detected_at_ms, 1000);

    let dead_letters_after_first = dead_letter_store
        .list("w1", "node-a")
        .expect("list dead letters after first replay");
    assert_eq!(dead_letters_after_first.len(), 1);
    assert_eq!(
        dead_letters_after_first[0]
            .pending_alert
            .alert
            .detected_at_ms,
        1001
    );

    let second = client
        .replay_revocation_dead_letters("w1", "node-a", 4, &recovery_store, &dead_letter_store)
        .expect("second replay");
    assert_eq!(second, 1);

    let pending_after_second = recovery_store
        .load_pending("w1", "node-a")
        .expect("load pending after second replay");
    assert_eq!(pending_after_second.len(), 2);

    let dead_letters_after_second = dead_letter_store
        .list("w1", "node-a")
        .expect("list dead letters after second replay");
    assert!(dead_letters_after_second.is_empty());
}

#[test]
fn run_revocation_dead_letter_replay_schedule_respects_interval() {
    let client = sample_client();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();

    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1000,
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        ))
        .expect("append dead letter 1");
    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1001,
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        ))
        .expect("append dead letter 2");

    let mut last_replay = None;
    let first = client
        .run_revocation_dead_letter_replay_schedule(
            "w1",
            "node-a",
            1000,
            100,
            1,
            &mut last_replay,
            &recovery_store,
            &dead_letter_store,
        )
        .expect("first scheduled replay");
    assert_eq!(first, 1);
    assert_eq!(last_replay, Some(1000));

    let second = client
        .run_revocation_dead_letter_replay_schedule(
            "w1",
            "node-a",
            1050,
            100,
            1,
            &mut last_replay,
            &recovery_store,
            &dead_letter_store,
        )
        .expect("second scheduled replay");
    assert_eq!(second, 0);

    let remaining_after_second = dead_letter_store
        .list("w1", "node-a")
        .expect("list remaining after second replay");
    assert_eq!(remaining_after_second.len(), 1);

    let third = client
        .run_revocation_dead_letter_replay_schedule(
            "w1",
            "node-a",
            1101,
            100,
            1,
            &mut last_replay,
            &recovery_store,
            &dead_letter_store,
        )
        .expect("third scheduled replay");
    assert_eq!(third, 1);

    let remaining_after_third = dead_letter_store
        .list("w1", "node-a")
        .expect("list remaining after third replay");
    assert!(remaining_after_third.is_empty());
}

#[test]
fn run_revocation_dead_letter_replay_schedule_rejects_interval_overflow_without_mutation() {
    let client = sample_client();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();

    dead_letter_store
        .append(&sample_dead_letter(
            "w1",
            "node-a",
            1000,
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded,
        ))
        .expect("append dead letter");

    let mut last_replay = Some(i64::MIN);
    let err = client
        .run_revocation_dead_letter_replay_schedule(
            "w1",
            "node-a",
            1000,
            100,
            1,
            &mut last_replay,
            &recovery_store,
            &dead_letter_store,
        )
        .expect_err("interval overflow should fail");
    match err {
        WorldError::DistributedValidationFailed { reason } => {
            assert!(
                reason.contains("replay schedule elapsed overflow"),
                "{reason}"
            );
        }
        other => panic!("unexpected error: {other:?}"),
    }

    assert_eq!(last_replay, Some(i64::MIN));
    let pending = recovery_store
        .load_pending("w1", "node-a")
        .expect("load pending after overflow");
    assert!(pending.is_empty());
    let dead_letters = dead_letter_store
        .list("w1", "node-a")
        .expect("list dead letters after overflow");
    assert_eq!(dead_letters.len(), 1);
}

#[test]
fn store_backed_schedule_coordinator_blocks_until_expired_or_released() {
    let store: Arc<dyn MembershipRevocationCoordinatorStateStore + Send + Sync> =
        Arc::new(InMemoryMembershipRevocationCoordinatorStateStore::new());
    let coordinator_a = StoreBackedMembershipRevocationScheduleCoordinator::new(Arc::clone(&store));
    let coordinator_b = StoreBackedMembershipRevocationScheduleCoordinator::new(Arc::clone(&store));

    assert!(coordinator_a
        .acquire("w1", "node-a", 1000, 500)
        .expect("acquire node-a"));
    assert!(!coordinator_b
        .acquire("w1", "node-b", 1200, 500)
        .expect("acquire node-b while held"));
    assert!(coordinator_b
        .acquire("w1", "node-b", 1601, 500)
        .expect("acquire node-b after expiry"));

    coordinator_b
        .release("w1", "node-b")
        .expect("release node-b");
    assert!(coordinator_a
        .acquire("w1", "node-a", 1602, 500)
        .expect("acquire node-a after release"));
}

#[test]
fn store_backed_schedule_coordinator_rejects_expiry_overflow_without_mutation() {
    let store: Arc<dyn MembershipRevocationCoordinatorStateStore + Send + Sync> =
        Arc::new(InMemoryMembershipRevocationCoordinatorStateStore::new());
    let coordinator = StoreBackedMembershipRevocationScheduleCoordinator::new(Arc::clone(&store));

    assert!(coordinator
        .acquire("w1", "node-a", 1000, 500)
        .expect("seed lease"));
    let seeded = store
        .load("w1")
        .expect("load seeded lease")
        .expect("seeded lease state");

    let err = coordinator
        .acquire("w1", "node-a", i64::MAX, 1)
        .expect_err("overflow should fail");
    match err {
        WorldError::DistributedValidationFailed { reason } => {
            assert!(reason.contains("lease expiry overflow"), "{reason}");
        }
        other => panic!("unexpected error: {other:?}"),
    }

    let after = store
        .load("w1")
        .expect("load lease after overflow")
        .expect("lease should remain");
    assert_eq!(after, seeded);
}

#[test]
fn emit_revocation_reconcile_alerts_with_recovery_buffers_and_recovers() {
    let client = sample_client();
    let sink = FailOnceAlertSink::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();

    let first = client
        .emit_revocation_reconcile_alerts_with_recovery(
            "w1",
            "node-a",
            &sink,
            &recovery_store,
            vec![
                sample_alert("w1", "node-a", 1000),
                sample_alert("w1", "node-a", 1001),
            ],
        )
        .expect("first emit with recovery");
    assert_eq!(first.recovered, 0);
    assert_eq!(first.emitted_new, 0);
    assert_eq!(first.buffered, 2);
    assert_eq!(first.deferred, 0);
    assert_eq!(first.dropped_capacity, 0);
    assert_eq!(first.dropped_retry_limit, 0);
    assert_eq!(first.delivery_metrics.attempted, 1);
    assert_eq!(first.delivery_metrics.failed, 1);
    assert_eq!(first.delivery_metrics.dead_lettered, 0);

    let pending_after_first = recovery_store
        .load_pending("w1", "node-a")
        .expect("load pending after first");
    assert_eq!(pending_after_first.len(), 2);

    let second = client
        .emit_revocation_reconcile_alerts_with_recovery(
            "w1",
            "node-a",
            &sink,
            &recovery_store,
            Vec::new(),
        )
        .expect("second emit with recovery");
    assert_eq!(second.recovered, 2);
    assert_eq!(second.emitted_new, 0);
    assert_eq!(second.buffered, 0);
    assert_eq!(second.deferred, 0);
    assert_eq!(second.dropped_capacity, 0);
    assert_eq!(second.dropped_retry_limit, 0);
    assert_eq!(second.delivery_metrics.attempted, 2);
    assert_eq!(second.delivery_metrics.succeeded, 2);
    assert_eq!(second.delivery_metrics.dead_lettered, 0);

    let pending_after_second = recovery_store
        .load_pending("w1", "node-a")
        .expect("load pending after second");
    assert!(pending_after_second.is_empty());
    assert_eq!(sink.emitted().len(), 2);
}

#[test]
fn emit_revocation_reconcile_alerts_with_ack_retry_defers_until_backoff_elapsed() {
    let client = sample_client();
    let sink = FailOnceAlertSink::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let policy = MembershipRevocationAlertAckRetryPolicy {
        max_pending_alerts: 8,
        max_retry_attempts: 3,
        retry_backoff_ms: 100,
    };

    let first = client
        .emit_revocation_reconcile_alerts_with_recovery_and_ack_retry(
            "w1",
            "node-a",
            1000,
            &sink,
            &recovery_store,
            vec![sample_alert("w1", "node-a", 1000)],
            &policy,
        )
        .expect("first emit");
    assert_eq!(first.recovered, 0);
    assert_eq!(first.emitted_new, 0);
    assert_eq!(first.buffered, 1);
    assert_eq!(first.deferred, 0);

    let pending_after_first = recovery_store
        .load_pending("w1", "node-a")
        .expect("load pending after first");
    assert_eq!(pending_after_first[0].attempt, 1);
    assert_eq!(pending_after_first[0].next_retry_at_ms, 1100);

    let second = client
        .emit_revocation_reconcile_alerts_with_recovery_and_ack_retry(
            "w1",
            "node-a",
            1050,
            &sink,
            &recovery_store,
            Vec::new(),
            &policy,
        )
        .expect("second emit");
    assert_eq!(second.recovered, 0);
    assert_eq!(second.emitted_new, 0);
    assert_eq!(second.buffered, 1);
    assert_eq!(second.deferred, 1);
    assert_eq!(sink.emitted().len(), 0);

    let third = client
        .emit_revocation_reconcile_alerts_with_recovery_and_ack_retry(
            "w1",
            "node-a",
            1100,
            &sink,
            &recovery_store,
            Vec::new(),
            &policy,
        )
        .expect("third emit");
    assert_eq!(third.recovered, 1);
    assert_eq!(third.emitted_new, 0);
    assert_eq!(third.buffered, 0);
    assert_eq!(third.deferred, 0);
    assert_eq!(sink.emitted().len(), 1);
}

#[test]
fn emit_revocation_reconcile_alerts_with_ack_retry_rejects_retry_timestamp_overflow_without_mutation(
) {
    let client = sample_client();
    let sink = AlwaysFailAlertSink::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let policy = MembershipRevocationAlertAckRetryPolicy {
        max_pending_alerts: 8,
        max_retry_attempts: 3,
        retry_backoff_ms: 1,
    };

    let err = client
        .emit_revocation_reconcile_alerts_with_recovery_and_ack_retry(
            "w1",
            "node-a",
            i64::MAX,
            &sink,
            &recovery_store,
            vec![sample_alert("w1", "node-a", i64::MAX)],
            &policy,
        )
        .expect_err("retry timestamp overflow should fail");
    assert!(matches!(
        err,
        WorldError::DistributedValidationFailed { ref reason }
            if reason.contains("retry timestamp overflow")
    ));

    let pending = recovery_store
        .load_pending("w1", "node-a")
        .expect("load pending after overflow");
    assert!(pending.is_empty());
    assert_eq!(sink.attempts(), 1);
}

#[test]
fn emit_revocation_reconcile_alerts_with_ack_retry_drops_when_retry_limit_reached() {
    let client = sample_client();
    let sink = AlwaysFailAlertSink::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let policy = MembershipRevocationAlertAckRetryPolicy {
        max_pending_alerts: 8,
        max_retry_attempts: 2,
        retry_backoff_ms: 0,
    };

    let first = client
        .emit_revocation_reconcile_alerts_with_recovery_and_ack_retry(
            "w1",
            "node-a",
            1000,
            &sink,
            &recovery_store,
            vec![sample_alert("w1", "node-a", 1000)],
            &policy,
        )
        .expect("first emit");
    assert_eq!(first.buffered, 1);
    assert_eq!(first.dropped_retry_limit, 0);

    let second = client
        .emit_revocation_reconcile_alerts_with_recovery_and_ack_retry(
            "w1",
            "node-a",
            1001,
            &sink,
            &recovery_store,
            Vec::new(),
            &policy,
        )
        .expect("second emit");
    assert_eq!(second.buffered, 0);
    assert_eq!(second.dropped_retry_limit, 1);
    assert_eq!(sink.attempts(), 2);

    let pending = recovery_store
        .load_pending("w1", "node-a")
        .expect("load pending");
    assert!(pending.is_empty());
}

#[test]
fn emit_revocation_reconcile_alerts_with_dead_letter_archives_retry_limit_drop() {
    let client = sample_client();
    let sink = AlwaysFailAlertSink::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let policy = MembershipRevocationAlertAckRetryPolicy {
        max_pending_alerts: 8,
        max_retry_attempts: 1,
        retry_backoff_ms: 0,
    };

    let report = client
        .emit_revocation_reconcile_alerts_with_recovery_and_ack_retry_with_dead_letter(
            "w1",
            "node-a",
            1000,
            &sink,
            &recovery_store,
            vec![sample_alert("w1", "node-a", 1000)],
            &policy,
            &dead_letter_store,
        )
        .expect("emit with dead-letter retry limit");

    assert_eq!(report.buffered, 0);
    assert_eq!(report.dropped_retry_limit, 1);
    assert_eq!(report.delivery_metrics.attempted, 1);
    assert_eq!(report.delivery_metrics.failed, 1);
    assert_eq!(report.delivery_metrics.dead_lettered, 1);

    let dead_letters = dead_letter_store
        .list("w1", "node-a")
        .expect("list dead letters");
    assert_eq!(dead_letters.len(), 1);
    assert_eq!(
        dead_letters[0].reason,
        MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded
    );
}

#[test]
fn emit_revocation_reconcile_alerts_with_ack_retry_enforces_capacity() {
    let client = sample_client();
    let sink = AlwaysFailAlertSink::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let policy = MembershipRevocationAlertAckRetryPolicy {
        max_pending_alerts: 2,
        max_retry_attempts: 3,
        retry_backoff_ms: 0,
    };

    let report = client
        .emit_revocation_reconcile_alerts_with_recovery_and_ack_retry(
            "w1",
            "node-a",
            2000,
            &sink,
            &recovery_store,
            vec![
                sample_alert("w1", "node-a", 2000),
                sample_alert("w1", "node-a", 2001),
                sample_alert("w1", "node-a", 2002),
            ],
            &policy,
        )
        .expect("emit with capacity policy");

    assert_eq!(report.buffered, 2);
    assert_eq!(report.dropped_capacity, 1);

    let pending = recovery_store
        .load_pending("w1", "node-a")
        .expect("load pending");
    assert_eq!(pending.len(), 2);
    assert_eq!(pending[0].alert.detected_at_ms, 2000);
    assert_eq!(pending[1].alert.detected_at_ms, 2001);
}

#[test]
fn emit_revocation_reconcile_alerts_with_dead_letter_archives_capacity_eviction() {
    let client = sample_client();
    let sink = FailOnceAlertSink::new();
    let recovery_store = InMemoryMembershipRevocationAlertRecoveryStore::new();
    let dead_letter_store = InMemoryMembershipRevocationAlertDeadLetterStore::new();
    let policy = MembershipRevocationAlertAckRetryPolicy {
        max_pending_alerts: 1,
        max_retry_attempts: 3,
        retry_backoff_ms: 100,
    };

    let report = client
        .emit_revocation_reconcile_alerts_with_recovery_and_ack_retry_with_dead_letter(
            "w1",
            "node-a",
            2000,
            &sink,
            &recovery_store,
            vec![
                sample_alert("w1", "node-a", 2000),
                sample_alert("w1", "node-a", 2001),
                sample_alert("w1", "node-a", 2002),
            ],
            &policy,
            &dead_letter_store,
        )
        .expect("emit with dead-letter capacity");

    assert_eq!(report.buffered, 1);
    assert_eq!(report.dropped_capacity, 2);
    assert_eq!(report.delivery_metrics.attempted, 1);
    assert_eq!(report.delivery_metrics.failed, 1);
    assert_eq!(report.delivery_metrics.dead_lettered, 2);

    let dead_letters = dead_letter_store
        .list("w1", "node-a")
        .expect("list dead letters");
    assert_eq!(dead_letters.len(), 2);
    assert!(dead_letters.iter().all(|record| {
        record.reason == MembershipRevocationAlertDeadLetterReason::CapacityEvicted
    }));
}

#[test]
fn run_revocation_reconcile_coordinated_with_recovery_replays_pending_alerts() {
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
    let coordinator_store: Arc<dyn MembershipRevocationCoordinatorStateStore + Send + Sync> =
        Arc::new(InMemoryMembershipRevocationCoordinatorStateStore::new());
    let coordinator = StoreBackedMembershipRevocationScheduleCoordinator::new(coordinator_store);

    let first = client
        .run_revocation_reconcile_coordinated_with_recovery(
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
            &coordinator,
            1000,
        )
        .expect("first coordinated recovery run");
    assert!(first.acquired);
    assert_eq!(first.recovered_alerts, 0);
    assert_eq!(first.emitted_alerts, 0);
    assert_eq!(first.buffered_alerts, 1);
    assert_eq!(first.deferred_alerts, 0);
    assert_eq!(first.dropped_alerts_capacity, 0);
    assert_eq!(first.dropped_alerts_retry_limit, 0);
    assert_eq!(first.delivery_metrics.attempted, 1);
    assert_eq!(first.delivery_metrics.failed, 1);

    let second = client
        .run_revocation_reconcile_coordinated_with_recovery(
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
            &coordinator,
            1000,
        )
        .expect("second coordinated recovery run");

    assert!(second.acquired);
    assert_eq!(second.recovered_alerts, 1);
    assert_eq!(second.emitted_alerts, 0);
    assert_eq!(second.buffered_alerts, 0);
    assert_eq!(second.deferred_alerts, 0);
    assert_eq!(second.dropped_alerts_capacity, 0);
    assert_eq!(second.dropped_alerts_retry_limit, 0);
    assert_eq!(second.delivery_metrics.attempted, 1);
    assert_eq!(second.delivery_metrics.succeeded, 1);

    let pending = recovery_store
        .load_pending("w1", "node-b")
        .expect("load pending");
    assert!(pending.is_empty());
    assert_eq!(alert_sink.emitted().len(), 1);
}
