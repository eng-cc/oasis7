use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use ed25519_dalek::SigningKey;
use oasis7_proto::distributed_dht::{DistributedDht, MembershipDirectorySnapshot};

use crate::distributed_dht::InMemoryDht;
use crate::distributed_net::{DistributedNetwork, InMemoryNetwork};
use crate::error::WorldError;
use crate::membership::{
    MembershipDirectorySigner, MembershipDirectorySignerKeyring, MembershipKeyRevocationAnnounce,
    MembershipRevocationSyncPolicy, MembershipSnapshotRestorePolicy, MembershipSyncClient,
};
use crate::quorum::{
    ConsensusConfig, ConsensusMembershipChange, ConsensusMembershipChangeRequest,
    ConsensusMembershipChangeResult, QuorumConsensus,
};

fn membership_request(
    requester_id: &str,
    requested_at_ms: i64,
    change: ConsensusMembershipChange,
) -> ConsensusMembershipChangeRequest {
    ConsensusMembershipChangeRequest {
        requester_id: requester_id.to_string(),
        requested_at_ms,
        reason: None,
        change,
    }
}

fn sample_consensus() -> QuorumConsensus {
    QuorumConsensus::new(ConsensusConfig::majority(vec![
        "seq-1".to_string(),
        "seq-2".to_string(),
        "seq-3".to_string(),
    ]))
    .expect("consensus")
}

fn sample_snapshot() -> MembershipDirectorySnapshot {
    MembershipDirectorySnapshot {
        world_id: "w1".to_string(),
        requester_id: "seq-1".to_string(),
        requested_at_ms: 400,
        reason: Some("restart".to_string()),
        validators: vec![
            "seq-1".to_string(),
            "seq-2".to_string(),
            "seq-3".to_string(),
            "seq-4".to_string(),
        ],
        quorum_threshold: 3,
        signature_key_id: None,
        signature: None,
    }
}

fn sample_revocation() -> MembershipKeyRevocationAnnounce {
    MembershipKeyRevocationAnnounce {
        world_id: "w1".to_string(),
        requester_id: "seq-1".to_string(),
        requested_at_ms: 401,
        key_id: "k1".to_string(),
        reason: Some("rotate".to_string()),
        signature_key_id: Some("k1".to_string()),
        signature: None,
    }
}

fn ed25519_keypair_hex(seed: u8) -> (String, String) {
    let private_key_hex = hex::encode([seed; 32]);
    let signing_key = SigningKey::from_bytes(&[seed; 32]);
    let public_key_hex = hex::encode(signing_key.verifying_key().to_bytes());
    (private_key_hex, public_key_hex)
}

fn ed25519_signer(seed: u8) -> MembershipDirectorySigner {
    let (private_key_hex, public_key_hex) = ed25519_keypair_hex(seed);
    MembershipDirectorySigner::ed25519(private_key_hex.as_str(), public_key_hex.as_str())
        .expect("ed25519 signer")
}

fn temp_membership_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    std::env::temp_dir().join(format!("oasis7-consensus-{prefix}-{nanos}"))
}

#[test]
fn membership_snapshot_signer_round_trip() {
    let signer = MembershipDirectorySigner::hmac_sha256("membership-secret");
    let mut snapshot = sample_snapshot();
    let signature = signer.sign_snapshot(&snapshot).expect("sign snapshot");
    snapshot.signature = Some(signature);
    signer.verify_snapshot(&snapshot).expect("verify snapshot");
}

#[test]
fn membership_snapshot_ed25519_signer_round_trip() {
    let signer = ed25519_signer(7);
    let mut snapshot = sample_snapshot();
    let signature = signer.sign_snapshot(&snapshot).expect("sign snapshot");
    snapshot.signature = Some(signature);
    signer.verify_snapshot(&snapshot).expect("verify snapshot");
}

#[test]
fn membership_revocation_ed25519_signer_round_trip() {
    let signer = ed25519_signer(9);
    let mut announce = sample_revocation();
    let signature = signer.sign_revocation(&announce).expect("sign revocation");
    announce.signature = Some(signature);
    signer
        .verify_revocation(&announce)
        .expect("verify revocation");
}

#[test]
fn membership_keyring_sign_and_verify_round_trip() {
    let mut keyring = MembershipDirectorySignerKeyring::new();
    keyring
        .add_hmac_sha256_key("k1", "membership-secret-v1")
        .expect("add key");
    keyring.set_active_key("k1").expect("set active key");

    let mut snapshot = sample_snapshot();
    let (key_id, signature) = keyring
        .sign_snapshot_with_active_key(&snapshot)
        .expect("sign with active key");
    snapshot.signature_key_id = Some(key_id);
    snapshot.signature = Some(signature);

    keyring.verify_snapshot(&snapshot).expect("verify snapshot");
}

#[test]
fn membership_keyring_ed25519_sign_and_verify_round_trip() {
    let mut keyring = MembershipDirectorySignerKeyring::new();
    let (private_key_hex, public_key_hex) = ed25519_keypair_hex(17);
    keyring
        .add_ed25519_key("k-ed", private_key_hex.as_str(), public_key_hex.as_str())
        .expect("add key");
    keyring.set_active_key("k-ed").expect("set active key");

    let mut snapshot = sample_snapshot();
    let (key_id, signature) = keyring
        .sign_snapshot_with_active_key(&snapshot)
        .expect("sign with active key");
    snapshot.signature_key_id = Some(key_id);
    snapshot.signature = Some(signature);

    keyring.verify_snapshot(&snapshot).expect("verify snapshot");
}

#[test]
fn publish_and_drain_membership_change_announcement() {
    let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
    let sync_client = MembershipSyncClient::new(Arc::clone(&network));
    let subscription = sync_client.subscribe("w1").expect("subscribe");

    let request = membership_request(
        "seq-1",
        100,
        ConsensusMembershipChange::AddValidator {
            validator_id: "seq-4".to_string(),
        },
    );
    let result = ConsensusMembershipChangeResult {
        applied: true,
        validators: vec![
            "seq-1".to_string(),
            "seq-2".to_string(),
            "seq-3".to_string(),
            "seq-4".to_string(),
        ],
        quorum_threshold: 3,
    };

    let published = sync_client
        .publish_membership_change("w1", &request, &result)
        .expect("publish");

    let drained = sync_client
        .drain_announcements(&subscription)
        .expect("drain announcements");
    assert_eq!(drained, vec![published]);
}

#[test]
fn file_membership_audit_store_tiered_offload_lists_cold_and_hot_records() {
    let root = temp_membership_dir("membership-audit-store-tiered");
    fs::create_dir_all(&root).expect("create temp dir");
    let store = crate::FileMembershipAuditStore::new(&root);

    let total = 4_100_i64;
    for requested_at_ms in 0..total {
        crate::MembershipAuditStore::append(
            &store,
            &crate::MembershipSnapshotAuditRecord {
                world_id: "w1".to_string(),
                requester_id: Some(format!("seq-{}", requested_at_ms % 3)),
                requested_at_ms: Some(requested_at_ms),
                signature_key_id: Some("k1".to_string()),
                outcome: crate::MembershipSnapshotAuditOutcome::Applied,
                reason: format!("audit-{requested_at_ms}"),
            },
        )
        .expect("append audit");
    }

    let records = crate::MembershipAuditStore::list(&store, "w1").expect("list audits");
    assert_eq!(records.len(), total as usize);
    assert_eq!(
        records.first().and_then(|record| record.requested_at_ms),
        Some(0)
    );
    assert_eq!(
        records.last().and_then(|record| record.requested_at_ms),
        Some(total - 1)
    );

    let cold_refs_path = root.join("w1.cold.refs.jsonl");
    assert!(
        cold_refs_path.exists(),
        "tiered offload should create cold refs when hot window overflows"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn restore_membership_from_dht_rejects_unsigned_snapshot_by_default() {
    let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
    let dht = InMemoryDht::new();
    let sync_client = MembershipSyncClient::new(Arc::clone(&network));
    let mut consensus = sample_consensus();
    let snapshot = sample_snapshot();
    dht.put_membership_directory("w1", &snapshot)
        .expect("put snapshot");

    let err = sync_client
        .restore_membership_from_dht("w1", &mut consensus, &dht)
        .expect_err("unsigned snapshot should be rejected by default policy");
    let WorldError::DistributedValidationFailed { reason } = err else {
        panic!("expected DistributedValidationFailed");
    };
    assert!(reason.contains("missing signature"));
}

#[test]
fn publish_membership_change_with_dht_signed_with_ed25519_keyring_restores_with_policy() {
    let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
    let dht = InMemoryDht::new();
    let sync_client = MembershipSyncClient::new(Arc::clone(&network));

    let request = membership_request(
        "seq-1",
        100,
        ConsensusMembershipChange::AddValidator {
            validator_id: "seq-4".to_string(),
        },
    );
    let result = ConsensusMembershipChangeResult {
        applied: true,
        validators: vec![
            "seq-1".to_string(),
            "seq-2".to_string(),
            "seq-3".to_string(),
            "seq-4".to_string(),
        ],
        quorum_threshold: 3,
    };

    let mut keyring = MembershipDirectorySignerKeyring::new();
    let (private_key_hex, public_key_hex) = ed25519_keypair_hex(23);
    keyring
        .add_ed25519_key("k-ed", private_key_hex.as_str(), public_key_hex.as_str())
        .expect("add key");
    keyring.set_active_key("k-ed").expect("set active key");

    let published = sync_client
        .publish_membership_change_with_dht_signed_with_keyring(
            "w1", &request, &result, &dht, &keyring,
        )
        .expect("publish signed membership change");
    assert_eq!(published.signature_key_id.as_deref(), Some("k-ed"));
    assert!(
        published
            .signature
            .as_deref()
            .expect("signature")
            .starts_with("ed25519:v1:")
    );

    let mut consensus = sample_consensus();
    let policy = MembershipSnapshotRestorePolicy {
        require_signature: true,
        require_signature_key_id: true,
        accepted_signature_key_ids: vec!["k-ed".to_string()],
        accepted_signature_signer_public_keys: vec![public_key_hex.clone()],
        ..MembershipSnapshotRestorePolicy::default()
    };
    let restored = sync_client
        .restore_membership_from_dht_verified_with_keyring(
            "w1",
            &mut consensus,
            &dht,
            Some(&keyring),
            &policy,
        )
        .expect("restore membership")
        .expect("restored result");

    assert!(restored.applied);
    assert!(restored.validators.iter().any(|id| id == "seq-4"));
}

#[test]
fn sync_key_revocations_with_policy_accepts_ed25519_signed_revocation() {
    let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
    let sync_client = MembershipSyncClient::new(Arc::clone(&network));
    let subscription = sync_client.subscribe("w1").expect("subscribe");

    let mut keyring = MembershipDirectorySignerKeyring::new();
    let (private_key_hex, public_key_hex) = ed25519_keypair_hex(31);
    keyring
        .add_ed25519_key("k-ed", private_key_hex.as_str(), public_key_hex.as_str())
        .expect("add key");
    keyring
        .add_hmac_sha256_key("k-old", "legacy-hmac")
        .expect("add old key");
    keyring.set_active_key("k-ed").expect("set active key");

    let _published = sync_client
        .publish_key_revocation_signed_with_keyring(
            "w1",
            "seq-1",
            201,
            "k-old",
            Some("rotate to ed25519".to_string()),
            &keyring,
        )
        .expect("publish key revocation");

    let policy = MembershipRevocationSyncPolicy {
        trusted_requesters: vec!["seq-1".to_string()],
        require_signature: true,
        require_signature_key_id: true,
        accepted_signature_key_ids: vec!["k-ed".to_string()],
        accepted_signature_signer_public_keys: vec![public_key_hex.clone()],
        ..MembershipRevocationSyncPolicy::default()
    };
    let report = sync_client
        .sync_key_revocations_with_policy("w1", &subscription, &mut keyring, None, &policy)
        .expect("sync revocations");
    assert_eq!(report.drained, 1);
    assert_eq!(report.applied, 1);
    assert_eq!(report.rejected, 0);
    assert!(keyring.is_key_revoked("k-old"));
}

#[test]
fn restore_membership_from_dht_rejects_unaccepted_signature_signer_public_key() {
    let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
    let dht = InMemoryDht::new();
    let sync_client = MembershipSyncClient::new(Arc::clone(&network));

    let request = membership_request(
        "seq-1",
        100,
        ConsensusMembershipChange::AddValidator {
            validator_id: "seq-4".to_string(),
        },
    );
    let result = ConsensusMembershipChangeResult {
        applied: true,
        validators: vec![
            "seq-1".to_string(),
            "seq-2".to_string(),
            "seq-3".to_string(),
            "seq-4".to_string(),
        ],
        quorum_threshold: 3,
    };

    let mut keyring = MembershipDirectorySignerKeyring::new();
    let (private_key_hex, public_key_hex) = ed25519_keypair_hex(41);
    keyring
        .add_ed25519_key("k-ed", private_key_hex.as_str(), public_key_hex.as_str())
        .expect("add key");
    keyring.set_active_key("k-ed").expect("set active key");
    sync_client
        .publish_membership_change_with_dht_signed_with_keyring(
            "w1", &request, &result, &dht, &keyring,
        )
        .expect("publish signed membership change");

    let mut consensus = sample_consensus();
    let unaccepted_public_key_hex = hex::encode([0x42; 32]);
    assert_ne!(unaccepted_public_key_hex, public_key_hex);
    let policy = MembershipSnapshotRestorePolicy {
        require_signature: true,
        require_signature_key_id: true,
        accepted_signature_key_ids: vec!["k-ed".to_string()],
        accepted_signature_signer_public_keys: vec![unaccepted_public_key_hex],
        ..MembershipSnapshotRestorePolicy::default()
    };
    let audit = sync_client
        .restore_membership_from_dht_verified_with_audit(
            "w1",
            &mut consensus,
            &dht,
            None,
            Some(&keyring),
            &policy,
        )
        .expect("restore audit");
    assert!(audit.restored.is_none());
    assert!(audit.audit.reason.contains("signature signer public key"));
}

#[test]
fn restore_membership_from_dht_rejects_hmac_when_signature_signer_public_key_required() {
    let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
    let dht = InMemoryDht::new();
    let sync_client = MembershipSyncClient::new(Arc::clone(&network));

    let request = membership_request(
        "seq-1",
        100,
        ConsensusMembershipChange::AddValidator {
            validator_id: "seq-4".to_string(),
        },
    );
    let result = ConsensusMembershipChangeResult {
        applied: true,
        validators: vec![
            "seq-1".to_string(),
            "seq-2".to_string(),
            "seq-3".to_string(),
            "seq-4".to_string(),
        ],
        quorum_threshold: 3,
    };

    let mut keyring = MembershipDirectorySignerKeyring::new();
    keyring
        .add_hmac_sha256_key("k-hmac", "membership-secret-v1")
        .expect("add key");
    keyring.set_active_key("k-hmac").expect("set active key");
    sync_client
        .publish_membership_change_with_dht_signed_with_keyring(
            "w1", &request, &result, &dht, &keyring,
        )
        .expect("publish signed membership change");

    let mut consensus = sample_consensus();
    let accepted_signer_public_key = hex::encode([0x24; 32]);
    let policy = MembershipSnapshotRestorePolicy {
        require_signature: true,
        require_signature_key_id: true,
        accepted_signature_key_ids: vec!["k-hmac".to_string()],
        accepted_signature_signer_public_keys: vec![accepted_signer_public_key],
        ..MembershipSnapshotRestorePolicy::default()
    };
    let audit = sync_client
        .restore_membership_from_dht_verified_with_audit(
            "w1",
            &mut consensus,
            &dht,
            None,
            Some(&keyring),
            &policy,
        )
        .expect("restore audit");
    assert!(audit.restored.is_none());
    assert!(audit.audit.reason.contains("signer public key is required"));
}

#[test]
fn restore_membership_from_dht_accepts_uppercase_signature_signer_public_key_policy() {
    let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
    let dht = InMemoryDht::new();
    let sync_client = MembershipSyncClient::new(Arc::clone(&network));

    let request = membership_request(
        "seq-1",
        100,
        ConsensusMembershipChange::AddValidator {
            validator_id: "seq-4".to_string(),
        },
    );
    let result = ConsensusMembershipChangeResult {
        applied: true,
        validators: vec![
            "seq-1".to_string(),
            "seq-2".to_string(),
            "seq-3".to_string(),
            "seq-4".to_string(),
        ],
        quorum_threshold: 3,
    };

    let mut keyring = MembershipDirectorySignerKeyring::new();
    let (private_key_hex, public_key_hex) = ed25519_keypair_hex(49);
    keyring
        .add_ed25519_key("k-ed", private_key_hex.as_str(), public_key_hex.as_str())
        .expect("add key");
    keyring.set_active_key("k-ed").expect("set active key");
    sync_client
        .publish_membership_change_with_dht_signed_with_keyring(
            "w1", &request, &result, &dht, &keyring,
        )
        .expect("publish signed membership change");

    let mut consensus = sample_consensus();
    let policy = MembershipSnapshotRestorePolicy {
        require_signature: true,
        require_signature_key_id: true,
        accepted_signature_key_ids: vec!["k-ed".to_string()],
        accepted_signature_signer_public_keys: vec![public_key_hex.to_uppercase()],
        ..MembershipSnapshotRestorePolicy::default()
    };
    let restored = sync_client
        .restore_membership_from_dht_verified_with_keyring(
            "w1",
            &mut consensus,
            &dht,
            Some(&keyring),
            &policy,
        )
        .expect("restore membership")
        .expect("restored result");
    assert!(restored.applied);
    assert!(restored.validators.iter().any(|id| id == "seq-4"));
}

#[test]
fn restore_membership_from_dht_verified_with_audit_rejects_invalid_signer_public_key_policy() {
    let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
    let dht = InMemoryDht::new();
    let sync_client = MembershipSyncClient::new(Arc::clone(&network));
    let mut consensus = sample_consensus();

    let policy = MembershipSnapshotRestorePolicy {
        accepted_signature_signer_public_keys: vec!["not-hex".to_string()],
        ..MembershipSnapshotRestorePolicy::default()
    };
    let err = sync_client
        .restore_membership_from_dht_verified_with_audit(
            "w1",
            &mut consensus,
            &dht,
            None,
            None,
            &policy,
        )
        .expect_err("invalid policy should fail fast");
    let WorldError::DistributedValidationFailed { reason } = err else {
        panic!("expected DistributedValidationFailed");
    };
    assert!(reason.contains("accepted_signature_signer_public_keys"));
    assert!(reason.contains("valid hex"));
}

#[test]
fn sync_key_revocations_with_policy_rejects_duplicate_normalized_signer_public_keys() {
    let network: Arc<dyn DistributedNetwork + Send + Sync> = Arc::new(InMemoryNetwork::new());
    let sync_client = MembershipSyncClient::new(Arc::clone(&network));
    let subscription = sync_client.subscribe("w1").expect("subscribe");
    let mut keyring = MembershipDirectorySignerKeyring::new();

    let (_private_key_hex, public_key_hex) = ed25519_keypair_hex(57);
    let policy = MembershipRevocationSyncPolicy {
        accepted_signature_signer_public_keys: vec![
            public_key_hex.clone(),
            public_key_hex.to_uppercase(),
        ],
        ..MembershipRevocationSyncPolicy::default()
    };
    let err = sync_client
        .sync_key_revocations_with_policy("w1", &subscription, &mut keyring, None, &policy)
        .expect_err("duplicate signer key policy should fail fast");
    let WorldError::DistributedValidationFailed { reason } = err else {
        panic!("expected DistributedValidationFailed");
    };
    assert!(reason.contains("duplicate signer public key"));
}
