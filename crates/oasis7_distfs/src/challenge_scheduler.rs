use std::collections::BTreeMap;

use oasis7_proto::distributed::StorageChallengeFailureReason;
use oasis7_proto::world_error::WorldError;
use serde::{Deserialize, Serialize};

use super::{
    LocalCasStore, STORAGE_CHALLENGE_PROOF_KIND_CHUNK_HASH_V1, StorageChallenge,
    StorageChallengeProbeConfig, StorageChallengeProbeReport, StorageChallengeReceipt,
    StorageChallengeRequest, storage_challenge_receipt_to_proof_semantics,
    verify_storage_challenge_receipt,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StorageChallengeProbeCursorState {
    pub next_blob_cursor: usize,
    pub rounds_executed: u64,
    pub cumulative_total_checks: u64,
    pub cumulative_passed_checks: u64,
    pub cumulative_failed_checks: u64,
    pub cumulative_failure_reasons: BTreeMap<String, u64>,
    #[serde(default)]
    pub consecutive_failure_rounds: u64,
    #[serde(default)]
    pub backoff_until_unix_ms: i64,
    #[serde(default)]
    pub last_probe_unix_ms: Option<i64>,
    #[serde(default)]
    pub cumulative_backoff_skipped_rounds: u64,
    #[serde(default)]
    pub cumulative_backoff_applied_ms: i64,
    #[serde(default)]
    pub last_backoff_duration_ms: i64,
    #[serde(default)]
    pub last_backoff_reason: Option<String>,
    #[serde(default)]
    pub last_backoff_multiplier: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageChallengeAdaptivePolicy {
    pub max_checks_per_round: u32,
    pub failure_backoff_base_ms: i64,
    pub failure_backoff_max_ms: i64,
    pub backoff_multiplier_hash_mismatch: u32,
    pub backoff_multiplier_missing_sample: u32,
    pub backoff_multiplier_timeout: u32,
    pub backoff_multiplier_read_io_error: u32,
    pub backoff_multiplier_signature_invalid: u32,
    pub backoff_multiplier_unknown: u32,
}

impl Default for StorageChallengeAdaptivePolicy {
    fn default() -> Self {
        Self {
            max_checks_per_round: u32::MAX,
            failure_backoff_base_ms: 0,
            failure_backoff_max_ms: 0,
            backoff_multiplier_hash_mismatch: 1,
            backoff_multiplier_missing_sample: 1,
            backoff_multiplier_timeout: 1,
            backoff_multiplier_read_io_error: 1,
            backoff_multiplier_signature_invalid: 1,
            backoff_multiplier_unknown: 1,
        }
    }
}

impl LocalCasStore {
    pub fn probe_storage_challenges_with_cursor(
        &self,
        world_id: &str,
        node_id: &str,
        observed_at_unix_ms: i64,
        config: &StorageChallengeProbeConfig,
        state: &mut StorageChallengeProbeCursorState,
    ) -> Result<StorageChallengeProbeReport, WorldError> {
        self.probe_storage_challenges_with_policy(
            world_id,
            node_id,
            observed_at_unix_ms,
            config,
            state,
            &StorageChallengeAdaptivePolicy::default(),
        )
    }

    pub fn probe_storage_challenges_with_policy(
        &self,
        world_id: &str,
        node_id: &str,
        observed_at_unix_ms: i64,
        config: &StorageChallengeProbeConfig,
        state: &mut StorageChallengeProbeCursorState,
        policy: &StorageChallengeAdaptivePolicy,
    ) -> Result<StorageChallengeProbeReport, WorldError> {
        validate_probe_config(config)?;
        validate_adaptive_policy(policy)?;
        validate_non_empty(world_id, "world_id")?;
        validate_non_empty(node_id, "node_id")?;

        if observed_at_unix_ms < state.backoff_until_unix_ms {
            state.rounds_executed = state.rounds_executed.saturating_add(1);
            state.last_probe_unix_ms = Some(observed_at_unix_ms);
            state.cumulative_backoff_skipped_rounds =
                state.cumulative_backoff_skipped_rounds.saturating_add(1);
            return Ok(StorageChallengeProbeReport {
                node_id: node_id.to_string(),
                world_id: world_id.to_string(),
                observed_at_unix_ms,
                total_checks: 0,
                passed_checks: 0,
                failed_checks: 0,
                failure_reasons: BTreeMap::new(),
                latest_proof_semantics: None,
            });
        }

        let hashes = self.list_blob_hashes()?;
        let mut report = StorageChallengeProbeReport {
            node_id: node_id.to_string(),
            world_id: world_id.to_string(),
            observed_at_unix_ms,
            total_checks: 0,
            passed_checks: 0,
            failed_checks: 0,
            failure_reasons: BTreeMap::new(),
            latest_proof_semantics: None,
        };

        if hashes.is_empty() {
            advance_probe_cursor_state(state, &report, 0, observed_at_unix_ms, policy);
            return Ok(report);
        }

        let max_checks = config.challenges_per_tick.min(policy.max_checks_per_round);
        let checks = (max_checks as usize).min(hashes.len());
        let start = state.next_blob_cursor % hashes.len();
        for index in 0..checks {
            let hash = hashes[(start + index) % hashes.len()].clone();
            report.total_checks = report.total_checks.saturating_add(1);
            let request = StorageChallengeRequest {
                challenge_id: build_scheduled_challenge_id(
                    node_id,
                    observed_at_unix_ms,
                    state.next_blob_cursor,
                    index as u32,
                    hash.as_str(),
                ),
                world_id: world_id.to_string(),
                node_id: node_id.to_string(),
                content_hash: hash,
                max_sample_bytes: config.max_sample_bytes,
                issued_at_unix_ms: observed_at_unix_ms,
                challenge_ttl_ms: config.challenge_ttl_ms,
                vrf_seed: build_scheduled_challenge_seed(
                    world_id,
                    node_id,
                    observed_at_unix_ms,
                    state.next_blob_cursor,
                    index as u32,
                ),
            };

            let challenge = match self.issue_storage_challenge(&request) {
                Ok(challenge) => challenge,
                Err(err) => {
                    report.failed_checks = report.failed_checks.saturating_add(1);
                    increment_reason(
                        &mut report.failure_reasons,
                        failure_reason_key(classify_world_error_reason(&err)),
                    );
                    continue;
                }
            };

            let receipt = match self.answer_storage_challenge(&challenge, observed_at_unix_ms) {
                Ok(receipt) => receipt,
                Err(err) => {
                    report.failed_checks = report.failed_checks.saturating_add(1);
                    increment_reason(
                        &mut report.failure_reasons,
                        failure_reason_key(classify_world_error_reason(&err)),
                    );
                    continue;
                }
            };

            match verify_storage_challenge_receipt(
                &challenge,
                &receipt,
                config.allowed_clock_skew_ms,
            ) {
                Ok(()) => {
                    report.passed_checks = report.passed_checks.saturating_add(1);
                    report.latest_proof_semantics = Some(
                        storage_challenge_receipt_to_proof_semantics(&challenge, &receipt),
                    );
                }
                Err(_) => {
                    report.failed_checks = report.failed_checks.saturating_add(1);
                    increment_reason(
                        &mut report.failure_reasons,
                        failure_reason_key(classify_receipt_reason(
                            &challenge,
                            &receipt,
                            config.allowed_clock_skew_ms,
                        )),
                    );
                }
            }
        }

        advance_probe_cursor_state(state, &report, hashes.len(), observed_at_unix_ms, policy);
        Ok(report)
    }
}

fn validate_probe_config(config: &StorageChallengeProbeConfig) -> Result<(), WorldError> {
    if config.max_sample_bytes == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "probe max_sample_bytes must be >= 1".to_string(),
        });
    }
    if config.challenges_per_tick == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "probe challenges_per_tick must be >= 1".to_string(),
        });
    }
    if config.challenge_ttl_ms <= 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "probe challenge_ttl_ms must be > 0".to_string(),
        });
    }
    if config.allowed_clock_skew_ms < 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "probe allowed_clock_skew_ms must be >= 0".to_string(),
        });
    }
    Ok(())
}

fn validate_adaptive_policy(policy: &StorageChallengeAdaptivePolicy) -> Result<(), WorldError> {
    if policy.max_checks_per_round == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "adaptive policy max_checks_per_round must be >= 1".to_string(),
        });
    }
    if policy.failure_backoff_base_ms < 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "adaptive policy failure_backoff_base_ms must be >= 0".to_string(),
        });
    }
    if policy.failure_backoff_max_ms < 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "adaptive policy failure_backoff_max_ms must be >= 0".to_string(),
        });
    }
    if policy.failure_backoff_max_ms < policy.failure_backoff_base_ms {
        return Err(WorldError::DistributedValidationFailed {
            reason: "adaptive policy failure_backoff_max_ms must be >= failure_backoff_base_ms"
                .to_string(),
        });
    }
    validate_backoff_multiplier(
        policy.backoff_multiplier_hash_mismatch,
        "backoff_multiplier_hash_mismatch",
    )?;
    validate_backoff_multiplier(
        policy.backoff_multiplier_missing_sample,
        "backoff_multiplier_missing_sample",
    )?;
    validate_backoff_multiplier(
        policy.backoff_multiplier_timeout,
        "backoff_multiplier_timeout",
    )?;
    validate_backoff_multiplier(
        policy.backoff_multiplier_read_io_error,
        "backoff_multiplier_read_io_error",
    )?;
    validate_backoff_multiplier(
        policy.backoff_multiplier_signature_invalid,
        "backoff_multiplier_signature_invalid",
    )?;
    validate_backoff_multiplier(
        policy.backoff_multiplier_unknown,
        "backoff_multiplier_unknown",
    )?;
    Ok(())
}

fn validate_backoff_multiplier(value: u32, field: &str) -> Result<(), WorldError> {
    if value == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!("adaptive policy {field} must be >= 1"),
        });
    }
    Ok(())
}

fn validate_non_empty(value: &str, field: &str) -> Result<(), WorldError> {
    if value.trim().is_empty() {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!("probe field {} cannot be empty", field),
        });
    }
    Ok(())
}

fn build_scheduled_challenge_id(
    node_id: &str,
    observed_at_unix_ms: i64,
    cursor: usize,
    probe_index: u32,
    content_hash: &str,
) -> String {
    let prefix_len = content_hash.len().min(12);
    let prefix = &content_hash[..prefix_len];
    format!(
        "sched-probe:{}:{}:{}:{}:{}",
        node_id, observed_at_unix_ms, cursor, probe_index, prefix
    )
}

fn build_scheduled_challenge_seed(
    world_id: &str,
    node_id: &str,
    observed_at_unix_ms: i64,
    cursor: usize,
    probe_index: u32,
) -> String {
    format!(
        "{}:{}:{}:{}:{}",
        world_id, node_id, observed_at_unix_ms, cursor, probe_index
    )
}

fn advance_probe_cursor_state(
    state: &mut StorageChallengeProbeCursorState,
    report: &StorageChallengeProbeReport,
    blob_count: usize,
    observed_at_unix_ms: i64,
    policy: &StorageChallengeAdaptivePolicy,
) {
    state.rounds_executed = state.rounds_executed.saturating_add(1);
    state.last_probe_unix_ms = Some(observed_at_unix_ms);
    state.cumulative_total_checks = state
        .cumulative_total_checks
        .saturating_add(report.total_checks);
    state.cumulative_passed_checks = state
        .cumulative_passed_checks
        .saturating_add(report.passed_checks);
    state.cumulative_failed_checks = state
        .cumulative_failed_checks
        .saturating_add(report.failed_checks);
    for (reason, count) in &report.failure_reasons {
        let entry = state
            .cumulative_failure_reasons
            .entry(reason.clone())
            .or_insert(0);
        *entry = entry.saturating_add(*count);
    }

    if report.failed_checks > 0 && report.passed_checks == 0 {
        state.consecutive_failure_rounds = state.consecutive_failure_rounds.saturating_add(1);
        let dominant_reason = dominant_failure_reason(&report.failure_reasons);
        let dominant_reason_key = failure_reason_key(dominant_reason);
        let reason_multiplier = backoff_multiplier_for_reason(policy, dominant_reason);
        let backoff_ms = compute_backoff_ms(
            policy.failure_backoff_base_ms,
            policy.failure_backoff_max_ms,
            state.consecutive_failure_rounds,
            reason_multiplier,
        );
        state.backoff_until_unix_ms = observed_at_unix_ms.saturating_add(backoff_ms);
        state.cumulative_backoff_applied_ms = state
            .cumulative_backoff_applied_ms
            .saturating_add(backoff_ms);
        state.last_backoff_duration_ms = backoff_ms;
        state.last_backoff_reason = Some(dominant_reason_key.to_string());
        state.last_backoff_multiplier = reason_multiplier;
    } else {
        state.consecutive_failure_rounds = 0;
        state.backoff_until_unix_ms = 0;
    }

    if blob_count == 0 {
        state.next_blob_cursor = 0;
        return;
    }
    let advance = (report.total_checks as usize) % blob_count;
    state.next_blob_cursor = (state.next_blob_cursor + advance) % blob_count;
}

fn compute_backoff_ms(
    base_ms: i64,
    max_ms: i64,
    failure_rounds: u64,
    reason_multiplier: u32,
) -> i64 {
    if base_ms <= 0 || max_ms <= 0 || failure_rounds == 0 {
        return 0;
    }
    let exponent = failure_rounds.saturating_sub(1).min(16) as u32;
    let multiplier = (1_u64 << exponent).min(i64::MAX as u64) as i64;
    base_ms
        .saturating_mul(multiplier)
        .saturating_mul(reason_multiplier as i64)
        .min(max_ms)
}

fn dominant_failure_reason(
    failure_reasons: &BTreeMap<String, u64>,
) -> StorageChallengeFailureReason {
    let mut dominant = StorageChallengeFailureReason::Unknown;
    let mut dominant_count = 0_u64;
    for (reason, count) in failure_reasons {
        if *count > dominant_count {
            dominant = parse_failure_reason_key(reason.as_str());
            dominant_count = *count;
        }
    }
    dominant
}

fn parse_failure_reason_key(reason: &str) -> StorageChallengeFailureReason {
    match reason {
        "MISSING_SAMPLE" => StorageChallengeFailureReason::MissingSample,
        "HASH_MISMATCH" => StorageChallengeFailureReason::HashMismatch,
        "TIMEOUT" => StorageChallengeFailureReason::Timeout,
        "READ_IO_ERROR" => StorageChallengeFailureReason::ReadIoError,
        "SIGNATURE_INVALID" => StorageChallengeFailureReason::SignatureInvalid,
        _ => StorageChallengeFailureReason::Unknown,
    }
}

fn backoff_multiplier_for_reason(
    policy: &StorageChallengeAdaptivePolicy,
    reason: StorageChallengeFailureReason,
) -> u32 {
    match reason {
        StorageChallengeFailureReason::HashMismatch => policy.backoff_multiplier_hash_mismatch,
        StorageChallengeFailureReason::MissingSample => policy.backoff_multiplier_missing_sample,
        StorageChallengeFailureReason::Timeout => policy.backoff_multiplier_timeout,
        StorageChallengeFailureReason::ReadIoError => policy.backoff_multiplier_read_io_error,
        StorageChallengeFailureReason::SignatureInvalid => {
            policy.backoff_multiplier_signature_invalid
        }
        StorageChallengeFailureReason::Unknown => policy.backoff_multiplier_unknown,
    }
}

fn increment_reason(map: &mut BTreeMap<String, u64>, key: &str) {
    let entry = map.entry(key.to_string()).or_insert(0);
    *entry = entry.saturating_add(1);
}

fn classify_world_error_reason(error: &WorldError) -> StorageChallengeFailureReason {
    match error {
        WorldError::BlobNotFound { .. } => StorageChallengeFailureReason::MissingSample,
        WorldError::BlobHashMismatch { .. } | WorldError::BlobHashInvalid { .. } => {
            StorageChallengeFailureReason::HashMismatch
        }
        WorldError::Io(_) => StorageChallengeFailureReason::ReadIoError,
        WorldError::DistributedValidationFailed { reason } => {
            let lower = reason.to_ascii_lowercase();
            if lower.contains("timeout") || lower.contains("window") {
                StorageChallengeFailureReason::Timeout
            } else if lower.contains("hash") {
                StorageChallengeFailureReason::HashMismatch
            } else if lower.contains("sample") || lower.contains("missing") {
                StorageChallengeFailureReason::MissingSample
            } else if lower.contains("signature") || lower.contains("proof") {
                StorageChallengeFailureReason::SignatureInvalid
            } else {
                StorageChallengeFailureReason::Unknown
            }
        }
        WorldError::SignatureKeyInvalid => StorageChallengeFailureReason::SignatureInvalid,
        WorldError::NetworkProtocolUnavailable { .. }
        | WorldError::NetworkRequestFailed { .. }
        | WorldError::Serde(_) => StorageChallengeFailureReason::Unknown,
    }
}

fn classify_receipt_reason(
    challenge: &StorageChallenge,
    receipt: &StorageChallengeReceipt,
    allowed_clock_skew_ms: i64,
) -> StorageChallengeFailureReason {
    if let Some(reason) = receipt.failure_reason {
        return reason;
    }
    if receipt.proof_kind != STORAGE_CHALLENGE_PROOF_KIND_CHUNK_HASH_V1 {
        return StorageChallengeFailureReason::SignatureInvalid;
    }
    if challenge.sample_offset != receipt.sample_offset
        || challenge.sample_size_bytes != receipt.sample_size_bytes
    {
        return StorageChallengeFailureReason::MissingSample;
    }
    if challenge.content_hash != receipt.content_hash
        || challenge.expected_sample_hash != receipt.sample_hash
    {
        return StorageChallengeFailureReason::HashMismatch;
    }
    let min_time = challenge
        .issued_at_unix_ms
        .saturating_sub(allowed_clock_skew_ms);
    let max_time = challenge
        .expires_at_unix_ms
        .saturating_add(allowed_clock_skew_ms);
    if receipt.responded_at_unix_ms < min_time || receipt.responded_at_unix_ms > max_time {
        return StorageChallengeFailureReason::Timeout;
    }
    StorageChallengeFailureReason::Unknown
}

fn failure_reason_key(reason: StorageChallengeFailureReason) -> &'static str {
    match reason {
        StorageChallengeFailureReason::MissingSample => "MISSING_SAMPLE",
        StorageChallengeFailureReason::HashMismatch => "HASH_MISMATCH",
        StorageChallengeFailureReason::Timeout => "TIMEOUT",
        StorageChallengeFailureReason::ReadIoError => "READ_IO_ERROR",
        StorageChallengeFailureReason::SignatureInvalid => "SIGNATURE_INVALID",
        StorageChallengeFailureReason::Unknown => "UNKNOWN",
    }
}

#[cfg(test)]
mod tests {
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
}
