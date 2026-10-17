use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::*;
use crate::BlobStore;

fn temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("duration")
        .as_nanos();
    std::env::temp_dir().join(format!("oasis7-distfs-scheduler-{prefix}-{unique}"))
}

fn blob(size: usize) -> Vec<u8> {
    (0..size)
        .map(|index| ((index % 251) as u8).wrapping_add(11))
        .collect()
}

#[test]
fn probe_with_cursor_advances_and_accumulates_state() {
    let dir = temp_dir("cursor-advance");
    let store = LocalCasStore::new(&dir);
    let _ = store.put_bytes(blob(80).as_slice()).expect("put 1");
    let _ = store.put_bytes(blob(96).as_slice()).expect("put 2");
    let _ = store.put_bytes(blob(112).as_slice()).expect("put 3");
    let config = StorageChallengeProbeConfig {
        max_sample_bytes: 16,
        challenges_per_tick: 2,
        challenge_ttl_ms: 200,
        allowed_clock_skew_ms: 10,
    };
    let mut state = StorageChallengeProbeCursorState::default();

    let first = store
        .probe_storage_challenges_with_cursor("w1", "node-a", 1_000, &config, &mut state)
        .expect("first round");
    assert_eq!(first.total_checks, 2);
    assert_eq!(first.passed_checks, 2);
    assert_eq!(first.failed_checks, 0);
    assert_eq!(state.rounds_executed, 1);
    assert_eq!(state.next_blob_cursor, 2);
    assert_eq!(state.cumulative_total_checks, 2);
    assert_eq!(state.cumulative_passed_checks, 2);
    assert_eq!(state.cumulative_failed_checks, 0);

    let second = store
        .probe_storage_challenges_with_cursor("w1", "node-a", 2_000, &config, &mut state)
        .expect("second round");
    assert_eq!(second.total_checks, 2);
    assert_eq!(second.passed_checks, 2);
    assert_eq!(second.failed_checks, 0);
    assert_eq!(state.rounds_executed, 2);
    assert_eq!(state.next_blob_cursor, 1);
    assert_eq!(state.cumulative_total_checks, 4);
    assert_eq!(state.cumulative_passed_checks, 4);
    assert_eq!(state.cumulative_failed_checks, 0);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn probe_with_cursor_records_hash_mismatch_failure() {
    let dir = temp_dir("hash-mismatch");
    let store = LocalCasStore::new(&dir);
    let hash = store.put_bytes(blob(64).as_slice()).expect("put");
    let blob_path = store.blobs_dir().join(format!("{hash}.blob"));
    fs::write(blob_path, b"tampered").expect("tamper");

    let config = StorageChallengeProbeConfig {
        max_sample_bytes: 16,
        challenges_per_tick: 1,
        challenge_ttl_ms: 200,
        allowed_clock_skew_ms: 0,
    };
    let mut state = StorageChallengeProbeCursorState::default();
    let report = store
        .probe_storage_challenges_with_cursor("w1", "node-b", 3_000, &config, &mut state)
        .expect("probe");
    assert_eq!(report.total_checks, 1);
    assert_eq!(report.passed_checks, 0);
    assert_eq!(report.failed_checks, 1);
    assert_eq!(
        report.failure_reasons.get("HASH_MISMATCH").copied(),
        Some(1)
    );
    assert_eq!(
        state
            .cumulative_failure_reasons
            .get("HASH_MISMATCH")
            .copied(),
        Some(1)
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn probe_with_cursor_allows_empty_blob_set() {
    let dir = temp_dir("empty");
    let store = LocalCasStore::new(&dir);
    let config = StorageChallengeProbeConfig::default();
    let mut state = StorageChallengeProbeCursorState::default();
    let report = store
        .probe_storage_challenges_with_cursor("w1", "node-c", 4_000, &config, &mut state)
        .expect("probe");
    assert_eq!(report.total_checks, 0);
    assert_eq!(report.passed_checks, 0);
    assert_eq!(report.failed_checks, 0);
    assert_eq!(state.rounds_executed, 1);
    assert_eq!(state.next_blob_cursor, 0);
    assert_eq!(state.cumulative_total_checks, 0);
}

#[test]
fn probe_with_policy_limits_checks_per_round() {
    let dir = temp_dir("policy-limit");
    let store = LocalCasStore::new(&dir);
    let _ = store.put_bytes(blob(32).as_slice()).expect("put 1");
    let _ = store.put_bytes(blob(48).as_slice()).expect("put 2");
    let _ = store.put_bytes(blob(64).as_slice()).expect("put 3");

    let config = StorageChallengeProbeConfig {
        max_sample_bytes: 8,
        challenges_per_tick: 3,
        challenge_ttl_ms: 100,
        allowed_clock_skew_ms: 0,
    };
    let policy = StorageChallengeAdaptivePolicy {
        max_checks_per_round: 1,
        ..StorageChallengeAdaptivePolicy::default()
    };
    let mut state = StorageChallengeProbeCursorState::default();
    let report = store
        .probe_storage_challenges_with_policy(
            "w1",
            "node-limit",
            5_000,
            &config,
            &mut state,
            &policy,
        )
        .expect("probe");
    assert_eq!(report.total_checks, 1);
    assert_eq!(state.cumulative_total_checks, 1);
    assert_eq!(state.next_blob_cursor, 1);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn probe_with_policy_applies_backoff_after_failure_round() {
    let dir = temp_dir("policy-backoff");
    let store = LocalCasStore::new(&dir);
    let hash = store.put_bytes(blob(64).as_slice()).expect("put");
    let blob_path = store.blobs_dir().join(format!("{hash}.blob"));
    fs::write(blob_path, b"tampered").expect("tamper");

    let config = StorageChallengeProbeConfig {
        max_sample_bytes: 8,
        challenges_per_tick: 1,
        challenge_ttl_ms: 100,
        allowed_clock_skew_ms: 0,
    };
    let policy = StorageChallengeAdaptivePolicy {
        max_checks_per_round: 1,
        failure_backoff_base_ms: 100,
        failure_backoff_max_ms: 500,
        ..StorageChallengeAdaptivePolicy::default()
    };
    let mut state = StorageChallengeProbeCursorState::default();

    let first = store
        .probe_storage_challenges_with_policy(
            "w1",
            "node-backoff",
            1_000,
            &config,
            &mut state,
            &policy,
        )
        .expect("first");
    assert_eq!(first.total_checks, 1);
    assert_eq!(first.failed_checks, 1);
    assert_eq!(state.consecutive_failure_rounds, 1);
    assert_eq!(state.backoff_until_unix_ms, 1_100);
    assert_eq!(state.cumulative_backoff_skipped_rounds, 0);
    assert_eq!(state.cumulative_backoff_applied_ms, 100);
    assert_eq!(state.last_backoff_duration_ms, 100);
    assert_eq!(state.last_backoff_reason.as_deref(), Some("HASH_MISMATCH"));
    assert_eq!(state.last_backoff_multiplier, 1);

    let second = store
        .probe_storage_challenges_with_policy(
            "w1",
            "node-backoff",
            1_050,
            &config,
            &mut state,
            &policy,
        )
        .expect("second");
    assert_eq!(second.total_checks, 0);
    assert_eq!(state.consecutive_failure_rounds, 1);
    assert_eq!(state.backoff_until_unix_ms, 1_100);
    assert_eq!(state.cumulative_backoff_skipped_rounds, 1);
    assert_eq!(state.cumulative_backoff_applied_ms, 100);

    let third = store
        .probe_storage_challenges_with_policy(
            "w1",
            "node-backoff",
            1_200,
            &config,
            &mut state,
            &policy,
        )
        .expect("third");
    assert_eq!(third.total_checks, 1);
    assert_eq!(third.failed_checks, 1);
    assert_eq!(state.consecutive_failure_rounds, 2);
    assert_eq!(state.backoff_until_unix_ms, 1_400);
    assert_eq!(state.cumulative_backoff_skipped_rounds, 1);
    assert_eq!(state.cumulative_backoff_applied_ms, 300);
    assert_eq!(state.last_backoff_duration_ms, 200);
    assert_eq!(state.last_backoff_reason.as_deref(), Some("HASH_MISMATCH"));
    assert_eq!(state.last_backoff_multiplier, 1);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn probe_cursor_state_deserializes_from_legacy_snapshot() {
    let legacy = serde_json::json!({
        "next_blob_cursor": 2,
        "rounds_executed": 3,
        "cumulative_total_checks": 9,
        "cumulative_passed_checks": 8,
        "cumulative_failed_checks": 1,
        "cumulative_failure_reasons": { "HASH_MISMATCH": 1 }
    });
    let state: StorageChallengeProbeCursorState =
        serde_json::from_value(legacy).expect("deserialize legacy");
    assert_eq!(state.next_blob_cursor, 2);
    assert_eq!(state.rounds_executed, 3);
    assert_eq!(state.cumulative_total_checks, 9);
    assert_eq!(state.consecutive_failure_rounds, 0);
    assert_eq!(state.backoff_until_unix_ms, 0);
    assert!(state.last_probe_unix_ms.is_none());
    assert_eq!(state.cumulative_backoff_skipped_rounds, 0);
    assert_eq!(state.cumulative_backoff_applied_ms, 0);
    assert_eq!(state.last_backoff_duration_ms, 0);
    assert!(state.last_backoff_reason.is_none());
    assert_eq!(state.last_backoff_multiplier, 0);
}

#[test]
fn probe_with_policy_rejects_zero_reason_multiplier() {
    let dir = temp_dir("policy-invalid-multiplier");
    let store = LocalCasStore::new(&dir);
    let _ = store.put_bytes(blob(32).as_slice()).expect("put");

    let config = StorageChallengeProbeConfig::default();
    let mut state = StorageChallengeProbeCursorState::default();
    let policy = StorageChallengeAdaptivePolicy {
        backoff_multiplier_unknown: 0,
        ..StorageChallengeAdaptivePolicy::default()
    };
    let err = store
        .probe_storage_challenges_with_policy(
            "w1",
            "node-invalid",
            2_000,
            &config,
            &mut state,
            &policy,
        )
        .expect_err("zero multiplier should be rejected");
    match err {
        WorldError::DistributedValidationFailed { reason } => {
            assert!(reason.contains("backoff_multiplier_unknown"));
        }
        other => panic!("unexpected error: {other:?}"),
    }

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn advance_probe_cursor_state_applies_reason_multiplier_from_dominant_reason() {
    let policy = StorageChallengeAdaptivePolicy {
        failure_backoff_base_ms: 100,
        failure_backoff_max_ms: 10_000,
        backoff_multiplier_hash_mismatch: 4,
        backoff_multiplier_timeout: 2,
        ..StorageChallengeAdaptivePolicy::default()
    };
    let mut state = StorageChallengeProbeCursorState::default();

    let mut hash_report = StorageChallengeProbeReport {
        node_id: "node-a".to_string(),
        world_id: "w1".to_string(),
        observed_at_unix_ms: 1_000,
        total_checks: 3,
        passed_checks: 0,
        failed_checks: 3,
        failure_reasons: BTreeMap::new(),
        latest_proof_semantics: None,
    };
    hash_report
        .failure_reasons
        .insert("HASH_MISMATCH".to_string(), 2);
    hash_report.failure_reasons.insert("TIMEOUT".to_string(), 1);
    advance_probe_cursor_state(&mut state, &hash_report, 0, 1_000, &policy);
    assert_eq!(state.consecutive_failure_rounds, 1);
    assert_eq!(state.backoff_until_unix_ms, 1_400);
    assert_eq!(state.last_backoff_duration_ms, 400);
    assert_eq!(state.last_backoff_reason.as_deref(), Some("HASH_MISMATCH"));
    assert_eq!(state.last_backoff_multiplier, 4);

    state.consecutive_failure_rounds = 0;
    state.backoff_until_unix_ms = 0;
    let mut timeout_report = hash_report.clone();
    timeout_report.observed_at_unix_ms = 2_000;
    timeout_report
        .failure_reasons
        .insert("HASH_MISMATCH".to_string(), 1);
    timeout_report
        .failure_reasons
        .insert("TIMEOUT".to_string(), 2);
    advance_probe_cursor_state(&mut state, &timeout_report, 0, 2_000, &policy);
    assert_eq!(state.consecutive_failure_rounds, 1);
    assert_eq!(state.backoff_until_unix_ms, 2_200);
    assert_eq!(state.last_backoff_duration_ms, 200);
    assert_eq!(state.last_backoff_reason.as_deref(), Some("TIMEOUT"));
    assert_eq!(state.last_backoff_multiplier, 2);
}
