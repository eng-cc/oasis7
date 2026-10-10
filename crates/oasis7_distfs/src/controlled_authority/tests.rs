use super::*;
use ed25519_dalek::SigningKey;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);
impl TestRoot {
    fn new() -> Self {
        loop {
            let path = std::env::temp_dir().join(format!(
                "oasis7-local-authority-{}-{}",
                std::process::id(),
                NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create exclusive test directory: {error}"),
            }
        }
    }
    fn journal(&self) -> PathBuf {
        self.0.join("journal")
    }
}
impl Drop for TestRoot {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn signer() -> SigningKey {
    SigningKey::from_bytes(&[7; 32])
}
fn trusted() -> TrustedLocalAuthority {
    TrustedLocalAuthority {
        world_id: "world-a".into(),
        chain_id: "chain-a".into(),
        genesis_digest: "1".repeat(64),
        signer_public_key_hex: hex::encode(signer().verifying_key().to_bytes()),
        authority_epoch: 4,
        source_binding: LocalSourceBinding {
            execution_engine_ref: "engine/v1".into(),
            governing_rules_ref: "rules/v1".into(),
        },
    }
}
fn request(id: &str) -> LocalRequestIdentity {
    LocalRequestIdentity {
        verified_subject: "verified-player-a".into(),
        operation_domain: "operation-a".into(),
        nonce_scope: "existing-nonce-scope".into(),
        request_id: id.into(),
    }
}
fn prepared() -> PreparedLocalDecision {
    PreparedLocalDecision {
        payload_digest: "2".repeat(64),
        execution_block_hash: "3".repeat(64),
        state_root: "4".repeat(64),
        snapshot_ref: "5".repeat(64),
        journal_ref: "6".repeat(64),
    }
}
fn open(root: &TestRoot, anchor: LocalHeadAnchor) -> ControlledAuthorityLocalJournal {
    ControlledAuthorityLocalJournal::open(&root.journal(), trusted(), signer(), anchor).unwrap()
}
fn record(
    journal: &mut ControlledAuthorityLocalJournal,
    id: &str,
) -> ControlledAuthorityLocalDecisionProofV1 {
    match journal
        .append(4, &journal.head_anchor(), request(id), prepared())
        .unwrap()
    {
        LocalAppendOutcome::LocallyRecorded(proof) => *proof,
        result => panic!("unexpected append result: {result:?}"),
    }
}

#[cfg(unix)]
#[test]
fn durable_append_retry_collision_and_reopen_rebuild_index() {
    let root = TestRoot::new();
    let genesis = trusted().genesis_anchor().unwrap();
    let mut journal = open(&root, genesis.clone());
    let first = record(&mut journal, "first");
    assert_eq!(
        verify_local_decision_proof(&first, &trusted(), &genesis).unwrap(),
        journal.head_anchor()
    );
    // Retried original identity resolves even with the original stale parent.
    assert_eq!(
        journal
            .append(4, &genesis, request("first"), prepared())
            .unwrap(),
        LocalAppendOutcome::LocallyRecorded(Box::new(first.clone()))
    );
    let mut different = prepared();
    different.payload_digest = "7".repeat(64);
    assert!(
        journal
            .append(4, &genesis, request("first"), different)
            .is_err()
    );
    assert_eq!(journal.head_anchor().position, 1);
    let anchor = journal.head_anchor();
    drop(journal);
    let mut reopened = open(&root, anchor);
    assert_eq!(
        reopened.lookup(&request("first")).unwrap(),
        LocalLookupOutcome::LocallyRecorded(Box::new(first))
    );
    assert_eq!(
        reopened.lookup(&request("missing")).unwrap(),
        LocalLookupOutcome::NotLocallyRecorded
    );
    assert_eq!(record(&mut reopened, "next").decision.position, 2);
}

#[cfg(unix)]
#[test]
fn stale_epoch_and_parent_reject_without_changing_snapshot() {
    let root = TestRoot::new();
    let genesis = trusted().genesis_anchor().unwrap();
    let mut journal = open(&root, genesis.clone());
    record(&mut journal, "first");
    let before = fs::read(root.journal().join(storage::SNAPSHOT_FILE)).unwrap();
    for epoch in [0, 3, 5] {
        assert!(
            journal
                .append(epoch, &journal.head_anchor(), request("next"), prepared())
                .is_err()
        );
    }
    assert!(
        journal
            .append(4, &genesis, request("next"), prepared())
            .is_err()
    );
    let mut wrong = journal.head_anchor();
    wrong.proof_hash = "a".repeat(64);
    assert!(
        journal
            .append(4, &wrong, request("next"), prepared())
            .is_err()
    );
    assert_eq!(
        fs::read(root.journal().join(storage::SNAPSHOT_FILE)).unwrap(),
        before
    );
}

#[cfg(unix)]
#[test]
fn local_writer_lock_excludes_second_handle_and_competing_process() {
    let root = TestRoot::new();
    let genesis = trusted().genesis_anchor().unwrap();
    let journal = open(&root, genesis.clone());
    assert!(matches!(
        ControlledAuthorityLocalJournal::open(
            &root.journal(),
            trusted(),
            signer(),
            genesis.clone()
        ),
        Err(LocalJournalError::Locked)
    ));
    assert!(child(&root, "locked", &genesis, 0).success());
    drop(journal);
    assert!(child(&root, "open", &genesis, 0).success());
}

#[cfg(unix)]
#[test]
fn unknown_preserves_identity_poison_and_lookup_resolves_each_write_phase() {
    for phase in phases() {
        let root = TestRoot::new();
        let mut journal = open(&root, trusted().genesis_anchor().unwrap());
        journal.fault = storage::FaultInjection::ErrorAt(phase);
        assert_eq!(
            journal
                .append(4, &journal.head_anchor(), request("uncertain"), prepared())
                .unwrap(),
            LocalAppendOutcome::Unknown {
                request: request("uncertain")
            }
        );
        assert_eq!(
            journal
                .append(4, &journal.head_anchor(), request("next"), prepared())
                .unwrap_err(),
            LocalJournalError::Poisoned
        );
        let lookup = journal.lookup(&request("uncertain")).unwrap();
        let recorded = matches!(
            phase,
            storage::PersistPhase::Renamed | storage::PersistPhase::DirectorySynced
        );
        assert_eq!(
            matches!(lookup, LocalLookupOutcome::LocallyRecorded(_)),
            recorded
        );
        assert_eq!(
            journal
                .append(4, &journal.head_anchor(), request("next"), prepared())
                .unwrap_err(),
            LocalJournalError::Poisoned
        );
        let anchor = journal.head_anchor();
        drop(journal);
        let mut reopened = open(&root, anchor);
        record(&mut reopened, "next");
    }
}

#[cfg(unix)]
#[test]
fn process_crash_at_every_persist_phase_recovers_single_snapshot() {
    for (index, phase) in phases().into_iter().enumerate() {
        let root = TestRoot::new();
        let mut initial = open(&root, trusted().genesis_anchor().unwrap());
        let first = record(&mut initial, "first");
        let anchor = initial.head_anchor();
        drop(initial);
        assert_eq!(child(&root, "crash", &anchor, index).code(), Some(73));
        let mut reopened = open(&root, anchor);
        assert_eq!(
            reopened.lookup(&request("first")).unwrap(),
            LocalLookupOutcome::LocallyRecorded(Box::new(first))
        );
        let recorded = matches!(
            phase,
            storage::PersistPhase::Renamed | storage::PersistPhase::DirectorySynced
        );
        assert_eq!(
            matches!(
                reopened.lookup(&request("crash-request")).unwrap(),
                LocalLookupOutcome::LocallyRecorded(_)
            ),
            recorded
        );
        assert_eq!(
            reopened.head_anchor().position,
            if recorded { 2 } else { 1 }
        );
        record(&mut reopened, "after-recovery");
    }
}

#[cfg(unix)]
#[test]
fn external_anchor_rejects_truncation_and_missing_history() {
    let root = TestRoot::new();
    let genesis = trusted().genesis_anchor().unwrap();
    let mut journal = open(&root, genesis.clone());
    let initial = fs::read(root.journal().join(storage::SNAPSHOT_FILE)).unwrap();
    record(&mut journal, "first");
    let anchor = journal.head_anchor();
    drop(journal);
    fs::write(root.journal().join(storage::SNAPSHOT_FILE), &initial).unwrap();
    assert!(
        ControlledAuthorityLocalJournal::open(&root.journal(), trusted(), signer(), anchor.clone())
            .is_err()
    );
    // Explicit genesis trust cannot detect this historical rollback; no stronger claim.
    drop(open(&root, genesis));
    fs::remove_file(root.journal().join(storage::SNAPSHOT_FILE)).unwrap();
    assert!(
        ControlledAuthorityLocalJournal::open(&root.journal(), trusted(), signer(), anchor)
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn reopen_rejects_wrong_identity_signer_epoch_and_corrupt_history() {
    let root = TestRoot::new();
    let mut journal = open(&root, trusted().genesis_anchor().unwrap());
    record(&mut journal, "first");
    let anchor = journal.head_anchor();
    drop(journal);
    let mut changed = trusted();
    changed.authority_epoch += 1;
    assert!(
        ControlledAuthorityLocalJournal::open(&root.journal(), changed, signer(), anchor.clone())
            .is_err()
    );
    assert!(
        ControlledAuthorityLocalJournal::open(
            &root.journal(),
            trusted(),
            SigningKey::from_bytes(&[9; 32]),
            anchor.clone()
        )
        .is_err()
    );
    let path = root.journal().join(storage::SNAPSHOT_FILE);
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["decisions"][0]["decision"]["prepared"]["state_root"] = serde_json::json!("a".repeat(64));
    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(
        ControlledAuthorityLocalJournal::open(&root.journal(), trusted(), signer(), anchor)
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn offline_proof_rejects_tampering_cross_world_scope_schema_and_source_binding() {
    let root = TestRoot::new();
    let genesis = trusted().genesis_anchor().unwrap();
    let mut journal = open(&root, genesis.clone());
    let proof = record(&mut journal, "first");
    let mut uppercase_signature = proof.clone();
    uppercase_signature.signature_hex = uppercase_signature.signature_hex.to_ascii_uppercase();
    assert_ne!(uppercase_signature.signature_hex, proof.signature_hex);
    assert_eq!(
        hex::decode(&uppercase_signature.signature_hex).unwrap(),
        hex::decode(&proof.signature_hex).unwrap()
    );
    assert!(verify_local_decision_proof(&uppercase_signature, &trusted(), &genesis).is_err());
    let mut variants = Vec::new();
    let mut changed = proof.clone();
    changed.decision.scope = "committed".into();
    variants.push(changed);
    let mut changed = proof.clone();
    changed.decision.schema_version = 2;
    variants.push(changed);
    let mut changed = proof.clone();
    changed.decision.profile = "bft".into();
    variants.push(changed);
    let mut changed = proof.clone();
    changed.decision.authority.world_id = "world-b".into();
    variants.push(changed);
    let mut changed = proof.clone();
    changed.decision.authority.chain_id = "chain-b".into();
    variants.push(changed);
    let mut changed = proof.clone();
    changed.decision.authority.genesis_digest = "a".repeat(64);
    variants.push(changed);
    let mut changed = proof.clone();
    changed
        .decision
        .authority
        .source_binding
        .governing_rules_ref = "other".into();
    variants.push(changed);
    let mut changed = proof.clone();
    changed.decision.prepared.state_root = "a".repeat(64);
    variants.push(changed);
    let mut changed = proof.clone();
    changed.signature_hex = "0".repeat(128);
    variants.push(changed);
    for variant in variants {
        assert!(verify_local_decision_proof(&variant, &trusted(), &genesis).is_err());
    }
    let mut wrong_parent = genesis.clone();
    wrong_parent.proof_hash = "a".repeat(64);
    assert!(verify_local_decision_proof(&proof, &trusted(), &wrong_parent).is_err());
    // Even the trusted signer cannot redefine the fixed genesis root by signing
    // an otherwise well-formed first decision against an arbitrary zero parent.
    let mut false_genesis_body = proof.decision.clone();
    false_genesis_body.parent_hash = wrong_parent.proof_hash.clone();
    let resigned = proof::sign_local_decision(false_genesis_body, &signer()).unwrap();
    assert!(verify_local_decision_proof(&resigned, &trusted(), &wrong_parent).is_err());
}

#[cfg(unix)]
#[test]
fn malformed_and_duplicate_snapshot_history_fail_closed() {
    let root = TestRoot::new();
    let mut journal = open(&root, trusted().genesis_anchor().unwrap());
    let first = record(&mut journal, "first");
    let anchor = journal.head_anchor();
    drop(journal);
    let path = root.journal().join(storage::SNAPSHOT_FILE);
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["decisions"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::to_value(first).unwrap());
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(
        ControlledAuthorityLocalJournal::open(&root.journal(), trusted(), signer(), anchor.clone())
            .is_err()
    );
    fs::write(&path, b"{partial").unwrap();
    assert!(
        ControlledAuthorityLocalJournal::open(&root.journal(), trusted(), signer(), anchor)
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn symlink_snapshot_cannot_be_used_as_authority_storage() {
    use std::os::unix::fs::symlink;
    let root = TestRoot::new();
    let journal = open(&root, trusted().genesis_anchor().unwrap());
    drop(journal);
    let path = root.journal().join(storage::SNAPSHOT_FILE);
    fs::rename(&path, root.0.join("other")).unwrap();
    symlink(root.0.join("other"), path).unwrap();
    assert!(
        ControlledAuthorityLocalJournal::open(
            &root.journal(),
            trusted(),
            signer(),
            trusted().genesis_anchor().unwrap()
        )
        .is_err()
    );
}

#[cfg(unix)]
fn phases() -> [storage::PersistPhase; 5] {
    use storage::PersistPhase::*;
    [
        TempCreated,
        TempWritten,
        TempSynced,
        Renamed,
        DirectorySynced,
    ]
}

#[cfg(unix)]
fn child(
    root: &TestRoot,
    mode: &str,
    anchor: &LocalHeadAnchor,
    phase: usize,
) -> std::process::ExitStatus {
    std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "controlled_authority::tests::process_helper",
            "--ignored",
            "--nocapture",
        ])
        .env("OASIS7_LOCAL_AUTHORITY_TEST_ROOT", &root.0)
        .env("OASIS7_LOCAL_AUTHORITY_TEST_MODE", mode)
        .env(
            "OASIS7_LOCAL_AUTHORITY_TEST_ANCHOR",
            serde_json::to_string(anchor).unwrap(),
        )
        .env("OASIS7_LOCAL_AUTHORITY_TEST_PHASE", phase.to_string())
        .status()
        .unwrap()
}

#[cfg(unix)]
#[test]
#[ignore = "subprocess helper invoked with explicit test-only environment"]
fn process_helper() {
    let root = PathBuf::from(std::env::var_os("OASIS7_LOCAL_AUTHORITY_TEST_ROOT").unwrap());
    let anchor =
        serde_json::from_str(&std::env::var("OASIS7_LOCAL_AUTHORITY_TEST_ANCHOR").unwrap())
            .unwrap();
    let result =
        ControlledAuthorityLocalJournal::open(&root.join("journal"), trusted(), signer(), anchor);
    let mode = std::env::var("OASIS7_LOCAL_AUTHORITY_TEST_MODE").unwrap();
    if mode == "locked" {
        assert!(matches!(result, Err(LocalJournalError::Locked)));
        return;
    }
    let mut journal = result.unwrap();
    if mode == "crash" {
        let phase: usize = std::env::var("OASIS7_LOCAL_AUTHORITY_TEST_PHASE")
            .unwrap()
            .parse()
            .unwrap();
        journal.fault = storage::FaultInjection::ExitAt(phases()[phase]);
        let _ = journal.append(
            4,
            &journal.head_anchor(),
            request("crash-request"),
            prepared(),
        );
        panic!("crash injection did not execute");
    }
}

#[cfg(not(unix))]
#[test]
fn unsupported_platform_refuses_before_any_storage_creation() {
    let root = TestRoot::new();
    assert!(matches!(
        ControlledAuthorityLocalJournal::open(
            &root.journal(),
            trusted(),
            signer(),
            trusted().genesis_anchor().unwrap()
        ),
        Err(LocalJournalError::UnsupportedPlatform)
    ));
    assert!(!root.journal().exists());
}
