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
    std::env::temp_dir().join(format!("oasis7-distfs-challenge-{prefix}-{unique}"))
}

fn make_blob(size: usize) -> Vec<u8> {
    (0..size)
        .map(|index| ((index % 251) as u8).wrapping_add(3))
        .collect()
}

#[test]
fn issue_storage_challenge_is_deterministic_and_within_bounds() {
    let dir = temp_dir("issue");
    let store = LocalCasStore::new(&dir);
    let bytes = make_blob(96);
    let content_hash = store.put_bytes(bytes.as_slice()).expect("put bytes");

    let request = StorageChallengeRequest {
        challenge_id: "challenge-a".to_string(),
        world_id: "world-1".to_string(),
        node_id: "node-storage-1".to_string(),
        content_hash,
        max_sample_bytes: 32,
        issued_at_unix_ms: 100,
        challenge_ttl_ms: 2_000,
        vrf_seed: "seed-1".to_string(),
    };

    let challenge_a = store.issue_storage_challenge(&request).expect("issue a");
    let challenge_b = store.issue_storage_challenge(&request).expect("issue b");
    assert_eq!(challenge_a, challenge_b);
    assert_eq!(challenge_a.version, STORAGE_CHALLENGE_VERSION);
    assert!(challenge_a.sample_size_bytes <= request.max_sample_bytes);
    assert!(challenge_a.sample_offset + challenge_a.sample_size_bytes as u64 <= bytes.len() as u64);
    assert_eq!(challenge_a.expires_at_unix_ms, 2_100);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn answer_storage_challenge_returns_matching_receipt() {
    let dir = temp_dir("answer");
    let store = LocalCasStore::new(&dir);
    let bytes = make_blob(128);
    let content_hash = store.put_bytes(bytes.as_slice()).expect("put bytes");

    let request = StorageChallengeRequest {
        challenge_id: "challenge-b".to_string(),
        world_id: "world-1".to_string(),
        node_id: "node-storage-2".to_string(),
        content_hash,
        max_sample_bytes: 24,
        issued_at_unix_ms: 200,
        challenge_ttl_ms: 1_000,
        vrf_seed: "seed-2".to_string(),
    };
    let challenge = store.issue_storage_challenge(&request).expect("issue");
    let receipt = store
        .answer_storage_challenge(&challenge, 250)
        .expect("answer challenge");

    assert_eq!(receipt.version, STORAGE_CHALLENGE_VERSION);
    assert_eq!(receipt.challenge_id, challenge.challenge_id);
    assert_eq!(receipt.node_id, challenge.node_id);
    assert_eq!(receipt.content_hash, challenge.content_hash);
    assert_eq!(receipt.sample_offset, challenge.sample_offset);
    assert_eq!(receipt.sample_size_bytes, challenge.sample_size_bytes);
    assert_eq!(receipt.sample_hash, challenge.expected_sample_hash);
    assert_eq!(receipt.failure_reason, None);
    assert_eq!(
        receipt.proof_kind,
        STORAGE_CHALLENGE_PROOF_KIND_CHUNK_HASH_V1
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn verify_storage_challenge_receipt_accepts_valid_receipt() {
    let dir = temp_dir("verify-valid");
    let store = LocalCasStore::new(&dir);
    let content_hash = store
        .put_bytes(make_blob(160).as_slice())
        .expect("put bytes");
    let request = StorageChallengeRequest {
        challenge_id: "challenge-verify".to_string(),
        world_id: "world-1".to_string(),
        node_id: "node-storage-3".to_string(),
        content_hash,
        max_sample_bytes: 40,
        issued_at_unix_ms: 500,
        challenge_ttl_ms: 1_000,
        vrf_seed: "seed-verify".to_string(),
    };
    let challenge = store
        .issue_storage_challenge(&request)
        .expect("issue challenge");
    let receipt = store
        .answer_storage_challenge(&challenge, 900)
        .expect("answer challenge");
    verify_storage_challenge_receipt(&challenge, &receipt, 50).expect("verify receipt");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn verify_storage_challenge_receipt_rejects_hash_mismatch() {
    let dir = temp_dir("verify-hash-mismatch");
    let store = LocalCasStore::new(&dir);
    let content_hash = store
        .put_bytes(make_blob(80).as_slice())
        .expect("put bytes");
    let request = StorageChallengeRequest {
        challenge_id: "challenge-hash-mismatch".to_string(),
        world_id: "world-1".to_string(),
        node_id: "node-storage-4".to_string(),
        content_hash,
        max_sample_bytes: 16,
        issued_at_unix_ms: 1_000,
        challenge_ttl_ms: 500,
        vrf_seed: "seed-hash".to_string(),
    };
    let challenge = store
        .issue_storage_challenge(&request)
        .expect("issue challenge");
    let mut receipt = store
        .answer_storage_challenge(&challenge, 1_100)
        .expect("answer challenge");
    receipt.sample_hash = blake3_hex(b"tampered");

    let verified = verify_storage_challenge_receipt(&challenge, &receipt, 10);
    assert!(matches!(
        verified,
        Err(WorldError::DistributedValidationFailed { .. })
    ));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn verify_storage_challenge_receipt_rejects_expired_response() {
    let dir = temp_dir("verify-expired");
    let store = LocalCasStore::new(&dir);
    let content_hash = store
        .put_bytes(make_blob(64).as_slice())
        .expect("put bytes");
    let request = StorageChallengeRequest {
        challenge_id: "challenge-expired".to_string(),
        world_id: "world-1".to_string(),
        node_id: "node-storage-5".to_string(),
        content_hash,
        max_sample_bytes: 16,
        issued_at_unix_ms: 2_000,
        challenge_ttl_ms: 100,
        vrf_seed: "seed-expired".to_string(),
    };
    let challenge = store
        .issue_storage_challenge(&request)
        .expect("issue challenge");
    let receipt = store
        .answer_storage_challenge(&challenge, challenge.expires_at_unix_ms + 200)
        .expect("answer challenge");
    let verified = verify_storage_challenge_receipt(&challenge, &receipt, 50);
    assert!(matches!(
        verified,
        Err(WorldError::DistributedValidationFailed { .. })
    ));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn receipt_to_proof_semantics_projects_expected_fields() {
    let dir = temp_dir("proof-semantics");
    let store = LocalCasStore::new(&dir);
    let content_hash = store
        .put_bytes(make_blob(88).as_slice())
        .expect("put bytes");
    let request = StorageChallengeRequest {
        challenge_id: "challenge-semantics".to_string(),
        world_id: "world-1".to_string(),
        node_id: "node-storage-6".to_string(),
        content_hash,
        max_sample_bytes: 20,
        issued_at_unix_ms: 3_000,
        challenge_ttl_ms: 100,
        vrf_seed: "seed-semantics".to_string(),
    };
    let challenge = store
        .issue_storage_challenge(&request)
        .expect("issue challenge");
    let receipt = store
        .answer_storage_challenge(&challenge, 3_050)
        .expect("answer challenge");
    let semantics = storage_challenge_receipt_to_proof_semantics(&challenge, &receipt);

    assert_eq!(semantics.node_id, challenge.node_id);
    assert_eq!(
        semantics.sample_source,
        StorageChallengeSampleSource::LocalStoreIndex
    );
    assert_eq!(
        semantics.sample_reference,
        challenge_sample_reference(&challenge)
    );
    assert_eq!(semantics.failure_reason, None);
    assert_eq!(
        semantics.proof_kind_hint,
        STORAGE_CHALLENGE_PROOF_KIND_CHUNK_HASH_V1
    );
    assert_eq!(
        semantics.vrf_seed_hint.as_deref(),
        Some(challenge.vrf_seed.as_str())
    );
    assert_eq!(
        semantics.post_commitment_hint.as_deref(),
        Some(challenge.expected_sample_hash.as_str())
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn summarize_node_storage_challenge_stats_counts_pass_and_failure_reasons() {
    let dir = temp_dir("summarize-stats");
    let store = LocalCasStore::new(&dir);
    let hash_a = store.put_bytes(make_blob(120).as_slice()).expect("put a");
    let hash_b = store.put_bytes(make_blob(96).as_slice()).expect("put b");

    let request_a_pass = StorageChallengeRequest {
        challenge_id: "challenge-a-pass".to_string(),
        world_id: "world-1".to_string(),
        node_id: "node-a".to_string(),
        content_hash: hash_a.clone(),
        max_sample_bytes: 24,
        issued_at_unix_ms: 10,
        challenge_ttl_ms: 100,
        vrf_seed: "seed-a1".to_string(),
    };
    let challenge_a_pass = store
        .issue_storage_challenge(&request_a_pass)
        .expect("issue a pass");
    let receipt_a_pass = store
        .answer_storage_challenge(&challenge_a_pass, 50)
        .expect("answer a pass");

    let request_a_fail = StorageChallengeRequest {
        challenge_id: "challenge-a-fail".to_string(),
        world_id: "world-1".to_string(),
        node_id: "node-a".to_string(),
        content_hash: hash_a,
        max_sample_bytes: 24,
        issued_at_unix_ms: 20,
        challenge_ttl_ms: 100,
        vrf_seed: "seed-a2".to_string(),
    };
    let challenge_a_fail = store
        .issue_storage_challenge(&request_a_fail)
        .expect("issue a fail");
    let mut receipt_a_fail = store
        .answer_storage_challenge(&challenge_a_fail, 60)
        .expect("answer a fail");
    receipt_a_fail.sample_hash = blake3_hex(b"mismatch");

    let request_b_timeout = StorageChallengeRequest {
        challenge_id: "challenge-b-timeout".to_string(),
        world_id: "world-1".to_string(),
        node_id: "node-b".to_string(),
        content_hash: hash_b,
        max_sample_bytes: 16,
        issued_at_unix_ms: 100,
        challenge_ttl_ms: 10,
        vrf_seed: "seed-b1".to_string(),
    };
    let challenge_b_timeout = store
        .issue_storage_challenge(&request_b_timeout)
        .expect("issue b timeout");
    let receipt_b_timeout = store
        .answer_storage_challenge(&challenge_b_timeout, 200)
        .expect("answer b timeout");

    let report = summarize_node_storage_challenge_stats(
        &[
            (challenge_a_pass, receipt_a_pass),
            (challenge_a_fail, receipt_a_fail),
            (challenge_b_timeout, receipt_b_timeout),
        ],
        0,
    )
    .expect("summarize");
    assert_eq!(report.len(), 2);

    let node_a = report
        .iter()
        .find(|entry| entry.node_id == "node-a")
        .expect("node-a stats");
    assert_eq!(node_a.total_checks, 2);
    assert_eq!(node_a.passed_checks, 1);
    assert_eq!(node_a.failed_checks, 1);
    assert_eq!(
        node_a.failures_by_reason.get("HASH_MISMATCH").copied(),
        Some(1)
    );

    let node_b = report
        .iter()
        .find(|entry| entry.node_id == "node-b")
        .expect("node-b stats");
    assert_eq!(node_b.total_checks, 1);
    assert_eq!(node_b.passed_checks, 0);
    assert_eq!(node_b.failed_checks, 1);
    assert_eq!(node_b.failures_by_reason.get("TIMEOUT").copied(), Some(1));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn summarize_node_storage_challenge_stats_accepts_empty_entries() {
    let report = summarize_node_storage_challenge_stats(&[], 0).expect("summarize");
    assert!(report.is_empty());
}

#[test]
fn probe_storage_challenges_reports_passed_checks_for_valid_blobs() {
    let dir = temp_dir("probe-pass");
    let store = LocalCasStore::new(&dir);
    let _ = store.put_bytes(make_blob(90).as_slice()).expect("put one");
    let _ = store.put_bytes(make_blob(110).as_slice()).expect("put two");

    let report = store
        .probe_storage_challenges(
            "world-1",
            "node-storage-7",
            7_000,
            &StorageChallengeProbeConfig {
                max_sample_bytes: 24,
                challenges_per_tick: 2,
                challenge_ttl_ms: 100,
                allowed_clock_skew_ms: 10,
            },
        )
        .expect("probe");
    assert_eq!(report.total_checks, 2);
    assert_eq!(report.passed_checks, 2);
    assert_eq!(report.failed_checks, 0);
    assert!(report.failure_reasons.is_empty());
    let semantics = report.latest_proof_semantics.expect("latest semantics");
    assert_eq!(semantics.node_id, "node-storage-7");
    assert_eq!(
        semantics.sample_source,
        StorageChallengeSampleSource::LocalStoreIndex
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn probe_storage_challenges_records_failure_reason_on_blob_hash_mismatch() {
    let dir = temp_dir("probe-hash-mismatch");
    let store = LocalCasStore::new(&dir);
    let hash = store.put_bytes(make_blob(64).as_slice()).expect("put");
    let blob_path = store.blobs_dir().join(format!("{hash}.blob"));
    fs::write(blob_path, b"tampered-blob").expect("tamper");

    let report = store
        .probe_storage_challenges(
            "world-1",
            "node-storage-8",
            8_000,
            &StorageChallengeProbeConfig {
                max_sample_bytes: 16,
                challenges_per_tick: 1,
                challenge_ttl_ms: 100,
                allowed_clock_skew_ms: 0,
            },
        )
        .expect("probe");
    assert_eq!(report.total_checks, 1);
    assert_eq!(report.passed_checks, 0);
    assert_eq!(report.failed_checks, 1);
    assert_eq!(
        report.failure_reasons.get("HASH_MISMATCH").copied(),
        Some(1)
    );
    assert!(report.latest_proof_semantics.is_none());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn issue_storage_challenge_rejects_invalid_request() {
    let dir = temp_dir("invalid-request");
    let store = LocalCasStore::new(&dir);
    let content_hash = store.put_bytes(b"ok").expect("put bytes");

    let request = StorageChallengeRequest {
        challenge_id: " ".to_string(),
        world_id: "world-1".to_string(),
        node_id: "node-1".to_string(),
        content_hash,
        max_sample_bytes: 0,
        issued_at_unix_ms: 0,
        challenge_ttl_ms: 0,
        vrf_seed: "seed".to_string(),
    };
    let issued = store.issue_storage_challenge(&request);
    assert!(matches!(
        issued,
        Err(WorldError::DistributedValidationFailed { .. })
    ));

    let _ = fs::remove_dir_all(&dir);
}
