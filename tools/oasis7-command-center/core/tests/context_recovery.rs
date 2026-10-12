use oasis7_command_center_core::{context::*, tools::jobs::ToolJobs};
use std::path::Path;
fn candidate() -> String {
    let metadata = serde_json::json!({"schema_version":"oasis7.context_entry.v1","project_id":"fixture","id":"machine","kind":"machine","title":"Private title","environment":"development","owner":"test","tags":[],"state":"active","updated_at":"2026-01-01T00:00:00Z","updated_by":"fixture","verification":null,"credential_refs":[],"links":[]});
    format!(
        "---\n{}\n---\nsecret body never persisted to job database\n",
        serde_json::to_string_pretty(&metadata).unwrap()
    )
}
fn fixture(temp: &tempfile::TempDir) -> (ContextService, ContextBinding, ContextSaveRequest) {
    let service = ContextService::open(temp.path().join("host.db")).unwrap();
    let binding = service
        .bind(&temp.path().join("private"), "fixture", "Test", true)
        .unwrap();
    let request = ContextSaveRequest {
        binding_id: binding.id.clone(),
        binding_version: binding.binding_version,
        id: "machine".into(),
        raw_text: candidate(),
        expected_revision: None,
        actor: "desktop:test".into(),
    };
    (service, binding, request)
}
#[test]
fn crashed_save_is_reconciled_by_exact_semantics_without_replaying() {
    let temp = tempfile::tempdir().unwrap();
    let (service, binding, request) = fixture(&temp);
    let path = temp.path().join("jobs.db");
    let jobs = ToolJobs::open(&path).unwrap();
    let evidence = service.save_evidence(&request).unwrap();
    let serialized = serde_json::to_string(&evidence).unwrap();
    assert!(!serialized.contains("secret body"));
    assert!(!serialized.contains("Private title"));
    jobs.admit_once_with_evidence("save", "frozen-input", "entry", "save", &evidence)
        .unwrap();
    service.save(&request).unwrap();
    drop(jobs);
    let jobs = ToolJobs::open(&path).unwrap();
    assert_eq!(
        jobs.reconcile_context("save", &service, None)
            .unwrap()
            .state,
        "succeeded"
    );
    assert!(
        service
            .get_bound(&binding.id, binding.binding_version, "machine")
            .unwrap()
            .raw_text
            .contains("secret body")
    );
    jobs.admit("new", "new-input", "entry", "save").unwrap();
}
#[test]
fn absent_create_and_conflicting_revision_have_distinct_recovery_outcomes() {
    let temp = tempfile::tempdir().unwrap();
    let (service, _, request) = fixture(&temp);
    let evidence = service.save_evidence(&request).unwrap();
    assert_eq!(service.reconcile(&evidence, None).unwrap().state, "failed");
    service.save(&request).unwrap();
    let mut changed = request.clone();
    changed.raw_text = changed.raw_text.replace("secret body", "another body");
    let evidence = service.save_evidence(&changed).unwrap();
    assert_eq!(service.reconcile(&evidence, None).unwrap().state, "unknown");
}
#[test]
fn encrypted_export_and_new_directory_restore_reconcile_exact_revisions() {
    let temp = tempfile::tempdir().unwrap();
    let (service, binding, request) = fixture(&temp);
    service.save(&request).unwrap();
    let output = temp.path().join("snapshot.enc");
    let pass = b"private-test-password";
    let export = service
        .snapshot_export_evidence(&binding.id, binding.binding_version, &output, &[])
        .unwrap();
    let digest = match &export {
        ContextRecoveryEvidence::SnapshotExport { source_digest, .. } => source_digest,
        _ => unreachable!(),
    };
    service
        .snapshot_export_frozen(
            &binding.id,
            binding.binding_version,
            &output,
            &[],
            pass,
            digest,
        )
        .unwrap();
    assert_eq!(service.reconcile(&export, None).unwrap().state, "unknown");
    assert!(service.reconcile(&export, Some(b"wrong")).is_err());
    assert_eq!(
        service.reconcile(&export, Some(pass)).unwrap().state,
        "succeeded"
    );
    let target = temp.path().join("restored");
    let restore = service
        .snapshot_restore_evidence(&binding.id, binding.binding_version, &output, &target, pass)
        .unwrap();
    assert!(
        !serde_json::to_string(&restore)
            .unwrap()
            .contains("private-test-password")
    );
    let rejected = temp.path().join("rejected");
    assert!(
        service
            .snapshot_restore_frozen(
                &binding.id,
                binding.binding_version,
                &output,
                &rejected,
                pass,
                &"0".repeat(64)
            )
            .is_err()
    );
    assert!(!rejected.exists());
    service
        .snapshot_restore(&binding.id, binding.binding_version, &output, &target, pass)
        .unwrap();
    assert_eq!(
        service.reconcile(&restore, None).unwrap().state,
        "succeeded"
    );
    let store = oasis7_context::store::Store::open(Path::new(&target)).unwrap();
    let entry = store.get("machine").unwrap();
    store
        .put(
            "machine",
            String::from_utf8(entry.raw_bytes)
                .unwrap()
                .replace("secret body", "tampered body")
                .as_bytes(),
            oasis7_context::model::WriteCondition::Revision(entry.revision),
            "test",
        )
        .unwrap();
    assert_eq!(service.reconcile(&restore, None).unwrap().state, "unknown");
}
#[test]
fn source_change_is_rejected_under_snapshot_collection_lock() {
    let temp = tempfile::tempdir().unwrap();
    let (service, binding, request) = fixture(&temp);
    service.save(&request).unwrap();
    let output = temp.path().join("snapshot.enc");
    let digest = service
        .snapshot_input_identity(&binding.id, binding.binding_version, &[])
        .unwrap();
    let current = service
        .get_bound(&binding.id, binding.binding_version, "machine")
        .unwrap();
    let mut changed = request;
    changed.expected_revision = Some(current.revision);
    changed.raw_text = current.raw_text.replace("secret body", "new body");
    service.save(&changed).unwrap();
    assert!(
        service
            .snapshot_export_frozen(
                &binding.id,
                binding.binding_version,
                &output,
                &[],
                b"password",
                &digest
            )
            .is_err()
    );
    assert!(!output.exists());
}

#[test]
fn initialized_but_unbound_library_is_known_partial_without_replay() {
    let temp = tempfile::tempdir().unwrap();
    let service = ContextService::open(temp.path().join("host.db")).unwrap();
    let root = temp.path().join("new-library");
    let evidence = service.binding_evidence(&root, "fixture", true).unwrap();
    oasis7_context::store::Store::init(&root, "fixture", "Test").unwrap();
    let jobs = ToolJobs::open(":memory:").unwrap();
    jobs.admit_once_with_evidence("bind", "fingerprint", "root", "bind", &evidence)
        .unwrap();
    jobs.finish("bind", "unknown", None).unwrap();
    assert_eq!(
        jobs.reconcile_context("bind", &service, None)
            .unwrap()
            .state,
        "partial"
    );
    assert!(service.bindings().unwrap().is_empty());
    assert!(!jobs.quit_pending().unwrap());
    assert_eq!(
        jobs.finish("bind", "unknown", None).unwrap().state,
        "partial"
    );
    jobs.admit("connect", "new-fingerprint", "root", "bind")
        .unwrap();
}

#[test]
fn encrypted_backup_restores_after_source_loss_and_on_a_fresh_machine() {
    let temp = tempfile::tempdir().unwrap();
    let (service, binding, request) = fixture(&temp);
    service.save(&request).unwrap();
    let encrypted = temp.path().join("backup.enc");
    let password = b"restore-only-password";
    service
        .snapshot_export(
            &binding.id,
            binding.binding_version,
            &encrypted,
            &[],
            password,
        )
        .unwrap();
    std::fs::remove_dir_all(&binding.root).unwrap();
    assert!(
        service
            .validate_binding(&binding.id, binding.binding_version)
            .is_err()
    );
    let restored = temp.path().join("restored-with-record");
    service
        .snapshot_restore(
            &binding.id,
            binding.binding_version,
            &encrypted,
            &restored,
            password,
        )
        .unwrap();
    assert!(restored.join("entries/machine.md").exists());
    let fresh = ContextService::open(temp.path().join("fresh-host.db")).unwrap();
    let source = ContextSnapshotRestoreSource {
        binding_id: None,
        binding_version: None,
        registered_project_id: "fresh-project".into(),
    };
    let target = temp.path().join("fresh-restored");
    let frozen_digest =
        oasis7_local_signer::identity::sha256_hex(&std::fs::read(&encrypted).unwrap());
    let stale_target = temp.path().join("stale-evidence-target");
    assert!(
        fresh
            .restore_snapshot_evidence(
                &source,
                &encrypted,
                &stale_target,
                password,
                &"0".repeat(64)
            )
            .is_err()
    );
    assert!(!stale_target.exists());
    let db = rusqlite::Connection::open(temp.path().join("fresh-host.db")).unwrap();
    assert_eq!(
        db.query_row("SELECT COUNT(*) FROM context_reserved_roots", [], |r| r
            .get::<_, u64>(0))
            .unwrap(),
        0
    );
    let evidence = fresh
        .restore_snapshot_evidence(&source, &encrypted, &target, password, &frozen_digest)
        .unwrap();
    let digest = match &evidence {
        ContextRecoveryEvidence::SnapshotRestore { input_sha256, .. } => input_sha256,
        _ => unreachable!(),
    };
    let wrong = temp.path().join("wrong-password");
    assert!(
        fresh
            .restore_snapshot_frozen(&source, &encrypted, &wrong, b"wrong", digest)
            .is_err()
    );
    assert!(!wrong.exists());
    let result = fresh
        .restore_snapshot_frozen(&source, &encrypted, &target, password, digest)
        .unwrap();
    assert_eq!(result.store_id, binding.store_id);
    assert_eq!(result.project_id, binding.project_id);
    assert_eq!(result.registered_project_id, "fresh-project");
    assert_eq!(fresh.reconcile(&evidence, None).unwrap().state, "succeeded");
}

#[test]
fn lost_volume_exclusions_allow_unrelated_project_and_explicit_recovery() {
    use oasis7_command_center_core::CommandCenter;
    fn project(root: &Path) {
        std::fs::create_dir_all(root).unwrap();
        for args in [
            vec!["init", "-q"],
            vec![
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "--allow-empty",
                "-qm",
                "Initial",
            ],
        ] {
            assert!(
                std::process::Command::new("git")
                    .arg("-C")
                    .arg(root)
                    .args(args)
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let volume = temp.path().join("lost-volume");
    let old_project = volume.join("project");
    project(&old_project);
    let db = temp.path().join("host.db");
    let mut center = CommandCenter::open(&db).unwrap();
    let old_id = center
        .open_project(&old_project)
        .unwrap()
        .project
        .unwrap()
        .id;
    let root = volume.join("private");
    oasis7_context::store::Store::init(&root, "fixture", "Test").unwrap();
    let service = ContextService::open(&db).unwrap();
    let binding = service.bind(&root, &old_id, "Test", false).unwrap();
    service
        .save(&ContextSaveRequest {
            binding_id: binding.id.clone(),
            binding_version: binding.binding_version,
            id: "machine".into(),
            raw_text: candidate(),
            expected_revision: None,
            actor: "test".into(),
        })
        .unwrap();
    let encrypted = temp.path().join("backup.enc");
    let password = b"volume-recovery";
    service
        .snapshot_export(
            &binding.id,
            binding.binding_version,
            &encrypted,
            &[],
            password,
        )
        .unwrap();
    std::fs::remove_dir_all(&volume).unwrap();
    let unrelated = temp.path().join("unrelated-project");
    project(&unrelated);
    let current = center.open_project(&unrelated).unwrap().project.unwrap();
    let source = ContextSnapshotRestoreSource {
        binding_id: Some(binding.id.clone()),
        binding_version: Some(binding.binding_version),
        registered_project_id: current.id.clone(),
    };
    let target = temp.path().join("new-private");
    let digest = oasis7_local_signer::identity::sha256_hex(&std::fs::read(&encrypted).unwrap());
    let restored = service
        .restore_snapshot_frozen(&source, &encrypted, &target, password, &digest)
        .unwrap();
    assert_eq!(restored.registered_project_id, current.id);
    assert_eq!(restored.store_id, binding.store_id);
    // The vanished namespace remains private even if someone creates a new Git repo there.
    project(&root);
    assert!(center.open_project(&root).is_err());
}

#[test]
fn restore_rejects_replaced_prepared_parent_before_plaintext_or_binding() {
    let temp = tempfile::tempdir().unwrap();
    let (service, binding, request) = fixture(&temp);
    service.save(&request).unwrap();
    let input = temp.path().join("backup.enc");
    let password = b"parent-identity";
    service
        .snapshot_export(&binding.id, binding.binding_version, &input, &[], password)
        .unwrap();
    let source = ContextSnapshotRestoreSource {
        binding_id: Some(binding.id.clone()),
        binding_version: Some(binding.binding_version),
        registered_project_id: binding.registered_project_id.clone(),
    };
    let parent = temp.path().join("selected-parent");
    std::fs::create_dir(&parent).unwrap();
    let target = parent.join("restore");
    let digest = oasis7_local_signer::identity::sha256_hex(&std::fs::read(&input).unwrap());
    let evidence = service
        .restore_snapshot_evidence(&source, &input, &target, password, &digest)
        .unwrap();
    let parent_identity = match &evidence {
        ContextRecoveryEvidence::SnapshotRestore {
            parent_identity, ..
        } => parent_identity,
        _ => unreachable!(),
    };
    let original = temp.path().join("original-parent");
    std::fs::rename(&parent, &original).unwrap();
    std::fs::create_dir(&parent).unwrap();
    let before = service.bindings().unwrap().len();
    assert!(
        service
            .restore_snapshot_frozen_parent(
                &source,
                &input,
                &target,
                password,
                &digest,
                parent_identity
            )
            .is_err()
    );
    assert!(!target.exists());
    assert!(!original.join("restore").exists());
    assert_eq!(std::fs::read_dir(&parent).unwrap().count(), 0);
    assert_eq!(std::fs::read_dir(&original).unwrap().count(), 0);
    assert_eq!(service.bindings().unwrap().len(), before);
    assert_eq!(service.reconcile(&evidence, None).unwrap().state, "unknown");
}
