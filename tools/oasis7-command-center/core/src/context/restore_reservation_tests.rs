use super::*;

#[test]
fn no_effect_restore_cleans_only_current_unwritten_reservations() {
    for target_appears in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let source_root = base.join("source");
        Store::init(&source_root, "fixture", "Fixture").unwrap();
        let input = base.join("snapshot.enc");
        let password = b"restore-none-effect-fixture";
        oasis7_context::snapshot::export(&source_root, &input, &[], password, 0).unwrap();
        let service = ContextService::open(base.join("host.db")).unwrap();
        let source = ContextSnapshotRestoreSource {
            binding_id: None,
            binding_version: None,
            registered_project_id: "fixture".into(),
        };
        let target = base.join("target");
        let digest = recovery::file_digest(&input).unwrap();
        let evidence = service
            .restore_snapshot_evidence(&source, &input, &target, password, &digest)
            .unwrap();
        let ContextRecoveryEvidence::SnapshotRestore {
            parent_identity,
            staging_name: Some(stage),
            ..
        } = &evidence
        else {
            unreachable!()
        };
        let error = service
            .restore_snapshot_frozen_stage_with(
                &source,
                &input,
                &target,
                password,
                &digest,
                parent_identity,
                stage,
                || {
                    assert_eq!(
                        service
                            .db
                            .query_row("SELECT COUNT(*) FROM context_reserved_roots", [], |r| r
                                .get::<_, u64>(0))
                            .unwrap(),
                        2
                    );
                    if target_appears {
                        std::fs::create_dir(&target).unwrap();
                    } else {
                        std::fs::write(&input, b"changed encrypted source").unwrap();
                    }
                },
            )
            .unwrap_err();
        assert!(matches!(error, Error::Context(ref error) if error.effect == "none"));
        assert_eq!(
            service
                .db
                .query_row("SELECT COUNT(*) FROM context_reserved_roots", [], |r| r
                    .get::<_, u64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            service
                .db
                .query_row("SELECT COUNT(*) FROM context_binding_claims", [], |r| r
                    .get::<_, u64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            service
                .db
                .query_row("SELECT COUNT(*) FROM context_owned_reservations", [], |r| r
                    .get::<_, u64>(0))
                .unwrap(),
            0
        );
        assert!(!base.join(stage).exists());
        if target_appears {
            assert!(!target.join("context.json").exists());
            assert_eq!(std::fs::read_dir(&target).unwrap().count(), 1); // only the custody lock metadata
        }
    }
}

#[test]
fn no_effect_restore_keeps_preexisting_custody_and_existing_staging() {
    for stage_appears in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let service = ContextService::open(base.join("host.db")).unwrap();
        let root = base.join("target");
        let stage_name = oasis7_context::snapshot::new_restore_stage();
        let stage = base.join(&stage_name);
        service
            .db
            .execute(
                "INSERT INTO context_reserved_roots(path) VALUES (?1)",
                [root.to_string_lossy()],
            )
            .unwrap(); // legacy, no ownership generation
        let tx = Transaction::new_unchecked(&service.db, TransactionBehavior::Immediate).unwrap();
        let target_reservation = ContextService::reserve_private_root_in(&tx, &root).unwrap();
        let stage_reservation = ContextService::reserve_private_root_in(&tx, &stage).unwrap();
        tx.commit().unwrap();
        if stage_appears {
            std::fs::create_dir(&stage).unwrap();
            std::fs::write(stage.join("private"), b"retained private plaintext").unwrap();
        }
        let parent = oasis7_context::fs::Directory::open(&base).unwrap();
        service
            .release_unwritten_restore_reservations(
                &root,
                &stage,
                &parent,
                &held_directory_identity(&parent).unwrap(),
                &target_reservation,
                &stage_reservation,
            )
            .unwrap();
        assert!(
            service
                .db
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM context_reserved_roots WHERE path=?1)",
                    [root.to_string_lossy()],
                    |r| r.get::<_, bool>(0)
                )
                .unwrap()
        );
        if stage_appears {
            assert_eq!(
                std::fs::read(stage.join("private")).unwrap(),
                b"retained private plaintext"
            );
            assert!(
                service
                    .db
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM context_reserved_roots WHERE path=?1)",
                        [stage.to_string_lossy()],
                        |r| r.get::<_, bool>(0)
                    )
                    .unwrap()
            );
        }
    }
}

#[test]
fn ordinary_restore_overlap_before_or_after_evidence_proves_no_effect() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let source_root = base.join("source");
    Store::init(&source_root, "fixture", "Fixture").unwrap();
    let input = base.join("snapshot.enc");
    let password = b"restore-overlap-fixture";
    oasis7_context::snapshot::export(&source_root, &input, &[], password, 0).unwrap();
    let service = ContextService::open(base.join("host.db")).unwrap();
    let source = ContextSnapshotRestoreSource {
        binding_id: None,
        binding_version: None,
        registered_project_id: "fixture".into(),
    };
    let target = base.join("target");
    let digest = recovery::file_digest(&input).unwrap();
    let evidence = service
        .restore_snapshot_evidence(&source, &input, &target, password, &digest)
        .unwrap();
    register_project(&service.db, &base).unwrap();
    let error = service
        .restore_snapshot_evidence(&source, &input, &target, password, &digest)
        .unwrap_err();
    assert!(
        matches!(error, Error::Context(ref e) if e.code == "INVALID_INPUT" && e.effect == "none")
    );
    let ContextRecoveryEvidence::SnapshotRestore {
        parent_identity,
        staging_name: Some(stage),
        ..
    } = evidence
    else {
        unreachable!()
    };
    let error = service
        .restore_snapshot_frozen_stage(
            &source,
            &input,
            &target,
            password,
            &digest,
            &parent_identity,
            &stage,
        )
        .unwrap_err();
    assert!(
        matches!(error, Error::Context(ref e) if e.code == "INVALID_INPUT" && e.effect == "none")
    );
    assert_eq!(
        service
            .db
            .query_row("SELECT COUNT(*) FROM context_binding_claims", [], |r| r
                .get::<_, u64>(0))
            .unwrap(),
        0
    );
    assert!(!target.exists());
    assert!(!base.join(stage).exists());
}
