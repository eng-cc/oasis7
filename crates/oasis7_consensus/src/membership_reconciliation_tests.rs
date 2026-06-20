use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::distributed_net::{DistributedNetwork, InMemoryNetwork};

use crate::{
    InMemoryMembershipRevocationAlertSink, InMemoryMembershipRevocationScheduleCoordinator,
    InMemoryMembershipRevocationScheduleStateStore, MembershipDirectorySignerKeyring,
    MembershipRevocationAlertDedupPolicy, MembershipRevocationAlertDedupState,
    MembershipRevocationAlertPolicy, MembershipRevocationAlertSeverity,
    MembershipRevocationAlertSink, MembershipRevocationAnomalyAlert,
    MembershipRevocationReconcilePolicy, MembershipRevocationReconcileSchedulePolicy,
    MembershipRevocationReconcileScheduleState, MembershipRevocationScheduleCoordinator,
    MembershipRevocationScheduleStateStore, MembershipSyncClient, error::WorldError,
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

fn sample_alert(world_id: &str, node_id: &str, code: &str) -> MembershipRevocationAnomalyAlert {
    MembershipRevocationAnomalyAlert {
        world_id: world_id.to_string(),
        node_id: node_id.to_string(),
        detected_at_ms: 1000,
        severity: MembershipRevocationAlertSeverity::Warn,
        code: code.to_string(),
        message: "sample".to_string(),
        drained: 1,
        diverged: 1,
        rejected: 0,
    }
}

fn temp_membership_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("oasis7-consensus-{prefix}-{nanos}"))
}

#[test]
fn deduplicate_revocation_alerts_suppresses_within_window() {
    let client = sample_client();
    let policy = MembershipRevocationAlertDedupPolicy {
        suppress_window_ms: 300,
    };
    let mut state = MembershipRevocationAlertDedupState::default();

    let alerts = vec![
        sample_alert("w1", "node-a", "reconcile_diverged"),
        sample_alert("w1", "node-a", "reconcile_diverged"),
        sample_alert("w1", "node-a", "reconcile_rejected"),
    ];

    let first = client
        .deduplicate_revocation_alerts(alerts.clone(), 1000, &policy, &mut state)
        .expect("first dedup");
    assert_eq!(first.len(), 2);

    let second = client
        .deduplicate_revocation_alerts(alerts.clone(), 1100, &policy, &mut state)
        .expect("second dedup");
    assert_eq!(second.len(), 0);

    let third = client
        .deduplicate_revocation_alerts(alerts, 1400, &policy, &mut state)
        .expect("third dedup");
    assert_eq!(third.len(), 2);
}

#[test]
fn deduplicate_revocation_alerts_rejects_suppress_window_overflow_without_mutation() {
    let client = sample_client();
    let policy = MembershipRevocationAlertDedupPolicy {
        suppress_window_ms: 1,
    };
    let mut state = MembershipRevocationAlertDedupState::default();
    state
        .last_emitted_at_by_key
        .insert("w1:node-a:reconcile_diverged".to_string(), i64::MIN);
    let snapshot = state.clone();

    let err = client
        .deduplicate_revocation_alerts(
            vec![sample_alert("w1", "node-a", "reconcile_diverged")],
            i64::MAX,
            &policy,
            &mut state,
        )
        .expect_err("overflow should fail");
    match err {
        WorldError::DistributedValidationFailed { reason } => {
            assert!(
                reason.contains("dedup suppress window elapsed overflow"),
                "{reason}"
            );
        }
        other => panic!("unexpected error: {other:?}"),
    }

    assert_eq!(state, snapshot);
}

#[test]
fn in_memory_alert_sink_emits_and_lists() {
    let sink = InMemoryMembershipRevocationAlertSink::new();
    let alert = sample_alert("w1", "node-a", "reconcile_diverged");

    sink.emit(&alert).expect("emit alert");
    let alerts = sink.list().expect("list alerts");
    assert_eq!(alerts, vec![alert]);
}

#[test]
fn run_revocation_reconcile_schedule_runs_on_first_tick() {
    let client = sample_client();
    let subscription = client.subscribe("w1").expect("subscribe");
    let mut keyring = sample_keyring();

    let reconcile_policy = MembershipRevocationReconcilePolicy {
        trusted_nodes: vec!["node-b".to_string()],
        auto_revoke_missing_keys: false,
    };
    let schedule_policy = MembershipRevocationReconcileSchedulePolicy {
        checkpoint_interval_ms: 300,
        reconcile_interval_ms: 300,
    };
    let mut schedule_state = MembershipRevocationReconcileScheduleState::default();

    let run_report = client
        .run_revocation_reconcile_schedule(
            "w1",
            "node-b",
            1000,
            &subscription,
            &mut keyring,
            &reconcile_policy,
            &schedule_policy,
            &mut schedule_state,
        )
        .expect("run schedule");

    assert!(run_report.checkpoint_published);
    assert!(run_report.reconcile_executed);
    assert_eq!(schedule_state.last_checkpoint_at_ms, Some(1000));
    assert_eq!(schedule_state.last_reconcile_at_ms, Some(1000));
}

#[test]
fn run_revocation_reconcile_schedule_reconciles_due_and_merges() {
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
        auto_revoke_missing_keys: true,
    };
    let schedule_policy = MembershipRevocationReconcileSchedulePolicy {
        checkpoint_interval_ms: 1000,
        reconcile_interval_ms: 300,
    };
    let mut schedule_state = MembershipRevocationReconcileScheduleState {
        last_checkpoint_at_ms: Some(1300),
        last_reconcile_at_ms: Some(1000),
    };

    let run_report = client
        .run_revocation_reconcile_schedule(
            "w1",
            "node-b",
            1305,
            &subscription,
            &mut local_keyring,
            &reconcile_policy,
            &schedule_policy,
            &mut schedule_state,
        )
        .expect("run schedule");

    assert!(!run_report.checkpoint_published);
    assert!(run_report.reconcile_executed);
    let reconcile_report = run_report.reconcile_report.expect("reconcile report");
    assert_eq!(reconcile_report.drained, 1);
    assert_eq!(reconcile_report.diverged, 1);
    assert_eq!(reconcile_report.merged, 1);
    assert!(local_keyring.is_key_revoked("k2"));
}

#[test]
fn run_revocation_reconcile_schedule_rejects_interval_overflow_without_mutation() {
    let client = sample_client();
    let subscription = client.subscribe("w1").expect("subscribe");
    let mut keyring = sample_keyring();

    let reconcile_policy = MembershipRevocationReconcilePolicy {
        trusted_nodes: vec!["node-a".to_string()],
        auto_revoke_missing_keys: false,
    };
    let schedule_policy = MembershipRevocationReconcileSchedulePolicy {
        checkpoint_interval_ms: 300,
        reconcile_interval_ms: 300,
    };
    let mut schedule_state = MembershipRevocationReconcileScheduleState {
        last_checkpoint_at_ms: Some(i64::MIN),
        last_reconcile_at_ms: Some(1000),
    };
    let snapshot = schedule_state.clone();

    let err = client
        .run_revocation_reconcile_schedule(
            "w1",
            "node-a",
            i64::MAX,
            &subscription,
            &mut keyring,
            &reconcile_policy,
            &schedule_policy,
            &mut schedule_state,
        )
        .expect_err("overflow should fail");
    match err {
        WorldError::DistributedValidationFailed { reason } => {
            assert!(reason.contains("schedule due elapsed overflow"), "{reason}");
        }
        other => panic!("unexpected error: {other:?}"),
    }

    assert_eq!(schedule_state, snapshot);
}

#[test]
fn run_revocation_reconcile_coordinated_reports_not_acquired_when_locked() {
    let client = sample_client();
    let subscription = client.subscribe("w1").expect("subscribe");
    let mut keyring = sample_keyring();

    let reconcile_policy = MembershipRevocationReconcilePolicy {
        trusted_nodes: Vec::new(),
        auto_revoke_missing_keys: false,
    };
    let schedule_policy = MembershipRevocationReconcileSchedulePolicy {
        checkpoint_interval_ms: 300,
        reconcile_interval_ms: 300,
    };
    let alert_policy = MembershipRevocationAlertPolicy::default();

    let schedule_store = InMemoryMembershipRevocationScheduleStateStore::new();
    let alert_sink = InMemoryMembershipRevocationAlertSink::new();
    let coordinator = InMemoryMembershipRevocationScheduleCoordinator::new();

    assert!(
        coordinator
            .acquire("w1", "node-a", 1000, 1000)
            .expect("seed lock")
    );

    let report = client
        .run_revocation_reconcile_coordinated(
            "w1",
            "node-b",
            1100,
            &subscription,
            &mut keyring,
            &reconcile_policy,
            &schedule_policy,
            &alert_policy,
            None,
            None,
            &schedule_store,
            &alert_sink,
            &coordinator,
            1000,
        )
        .expect("run coordinated");

    assert!(!report.acquired);
    assert_eq!(report.emitted_alerts, 0);
    assert!(report.run_report.is_none());

    coordinator
        .release("w1", "node-a")
        .expect("release seed lock");
}

#[test]
fn in_memory_schedule_coordinator_rejects_expiry_overflow_without_mutation() {
    let coordinator = InMemoryMembershipRevocationScheduleCoordinator::new();
    assert!(
        coordinator
            .acquire("w1", "node-a", 1000, 500)
            .expect("seed lease")
    );

    let err = coordinator
        .acquire("w1", "node-a", i64::MAX, 1)
        .expect_err("overflow should fail");
    match err {
        WorldError::DistributedValidationFailed { reason } => {
            assert!(reason.contains("lease expiry overflow"), "{reason}");
        }
        other => panic!("unexpected error: {other:?}"),
    }

    assert_eq!(
        coordinator
            .holder_node("w1")
            .expect("load holder after overflow")
            .as_deref(),
        Some("node-a")
    );
}

#[test]
fn file_schedule_state_store_round_trip() {
    let root = temp_membership_dir("reconcile-schedule-state-store");
    fs::create_dir_all(&root).expect("create temp dir");

    let store = crate::FileMembershipRevocationScheduleStateStore::new(&root).expect("new store");
    let state = MembershipRevocationReconcileScheduleState {
        last_checkpoint_at_ms: Some(2000),
        last_reconcile_at_ms: Some(2100),
    };

    store
        .save("w1", "node-a", &state)
        .expect("save schedule state");
    let loaded = store.load("w1", "node-a").expect("load schedule state");
    assert_eq!(loaded, state);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn file_revocation_alert_sink_tiered_offload_lists_cold_and_hot_records() {
    let root = temp_membership_dir("reconcile-alert-sink-tiered");
    fs::create_dir_all(&root).expect("create temp dir");
    let sink = crate::FileMembershipRevocationAlertSink::new(&root).expect("new sink");

    let base_ms = 7_000_i64;
    let total = 4_100_i64;
    for offset in 0..total {
        let mut alert = sample_alert("w1", "node-a", "reconcile_diverged");
        alert.detected_at_ms = base_ms + offset;
        sink.emit(&alert).expect("emit alert");
    }

    let alerts = sink.list("w1").expect("list alerts");
    assert_eq!(alerts.len(), total as usize);
    assert_eq!(
        alerts.first().map(|alert| alert.detected_at_ms),
        Some(base_ms)
    );
    assert_eq!(
        alerts.last().map(|alert| alert.detected_at_ms),
        Some(base_ms + total - 1)
    );

    let cold_refs_path = root.join("w1.revocation-alerts.cold.refs.jsonl");
    assert!(
        cold_refs_path.exists(),
        "tiered offload should create cold refs when hot window overflows"
    );

    let _ = fs::remove_dir_all(root);
}
