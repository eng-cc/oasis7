use super::*;

#[test]
fn ordinary_root_rejection_precedes_admission_and_execution_race_proves_none() {
    for register_after_preparation in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap().join("selected");
        std::fs::create_dir(&root).unwrap();
        let service = ContextService::open(temp.path().join("host.db")).unwrap();
        let jobs = crate::tools::jobs::ToolJobs::open(":memory:").unwrap();
        if !register_after_preparation {
            register_project(&service.db, &root).unwrap();
        }
        let result = service
            .binding_evidence(&root, "project", true)
            .and_then(|evidence| {
                jobs.admit("bind", "frozen", "root", "bind")?;
                register_project(&service.db, &root)?;
                service.bind_evidenced(&root, "project", "Test", true, &evidence)
            });
        assert!(
            matches!(result, Err(Error::Context(ref error)) if error.code == "INVALID_INPUT" && error.effect == "none")
        );
        assert_eq!(
            jobs.list().unwrap().len(),
            usize::from(register_after_preparation)
        );
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
        assert!(!root.join("context.json").exists());
        assert!(!root.join(".context-local").exists());
        assert!(matches!(
            check_ordinary_overlap(&service.db, &root),
            Err(Error::Invalid(_))
        ));
    }
}

#[test]
fn active_cli_store_lock_prevents_no_effect_cleanup_until_custody_can_be_rechecked() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap().join("selected");
    std::fs::create_dir(&root).unwrap();
    let service = ContextService::open(temp.path().join("host.db")).unwrap();
    let dir = oasis7_context::fs::Directory::open(&root).unwrap();
    let expected = held_directory_identity(&dir).unwrap();
    let reservation = service.reserve_private_root(&root).unwrap();
    let external = oasis7_context::fs::Directory::open(&root).unwrap();
    let active = external.lock(0).unwrap();
    service
        .release_unwritten_binding_reservation(&root, &reservation, &expected, &dir)
        .unwrap();
    assert!(check_project(&service.db, &root).is_err());
    drop(active);
    service
        .release_unwritten_binding_reservation(&root, &reservation, &expected, &dir)
        .unwrap();
    check_project(&service.db, &root).unwrap();
}

#[test]
fn rollback_cannot_unprotect_replaced_or_newly_initialized_custody() {
    for replace_directory in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let root = base.join("selected");
        std::fs::create_dir(&root).unwrap();
        let service = ContextService::open(base.join("host.db")).unwrap();
        let original_directory = oasis7_context::fs::Directory::open(&root).unwrap();
        let expected = directory_identity(&root).unwrap();
        let reservation = service.reserve_private_root(&root).unwrap();
        if replace_directory {
            std::fs::rename(&root, base.join("original")).unwrap();
            std::fs::create_dir(&root).unwrap();
        } else {
            std::fs::write(root.join("context.json"), b"new private store metadata").unwrap();
        }
        std::fs::write(root.join("private-body"), b"preserve unknown custody").unwrap();
        service
            .release_unwritten_binding_reservation(
                &root,
                &reservation,
                &expected,
                &original_directory,
            )
            .unwrap();
        assert!(check_project(&service.db, &root).is_err());
        assert_eq!(
            service
                .db
                .query_row("SELECT COUNT(*) FROM context_binding_claims", [], |r| r
                    .get::<_, u64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            std::fs::read(root.join("private-body")).unwrap(),
            b"preserve unknown custody"
        );
    }
}

#[test]
fn releasing_parent_owner_keeps_another_actors_nested_private_exclusion() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap().join("selected");
    std::fs::create_dir(&root).unwrap();
    let db = temp.path().join("host.db");
    let owner = ContextService::open(&db).unwrap();
    let other = ContextService::open(&db).unwrap();
    let original = owner.reserve_private_root(&root).unwrap();
    let child = root.join("nested");
    std::fs::create_dir(&child).unwrap();
    let nested = other.reserve_private_root(&child).unwrap();
    std::fs::write(child.join("private-body"), b"unknown private state").unwrap();
    owner
        .release_unwritten_binding_reservation(
            &root,
            &original,
            &directory_identity(&root).unwrap(),
            &oasis7_context::fs::Directory::open(&root).unwrap(),
        )
        .unwrap();
    assert!(check_project(&other.db, &root).is_err());
    assert_eq!(
        other
            .db
            .query_row("SELECT path FROM context_reserved_roots", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        child.to_string_lossy()
    );
    assert_eq!(
        other
            .db
            .query_row("SELECT id FROM context_binding_claims", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        nested.0
    );
}

#[test]
fn another_actors_unknown_plaintext_claim_survives_actual_validation_failure() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap().join("selected");
    std::fs::create_dir(&root).unwrap();
    let db = temp.path().join("host.db");
    let service = ContextService::open(&db).unwrap();
    let evidence = service.binding_evidence(&root, "project", true).unwrap();
    let other = ContextService::open(&db).unwrap();
    let unknown = other.reserve_private_root(&root).unwrap();
    std::fs::write(root.join("private-body"), b"private partial state").unwrap();
    assert!(
        matches!(service.bind_evidenced(&root, "project", "Test", true, &evidence), Err(Error::Context(ref e)) if e.effect == "none" && e.code == "INVALID_INPUT")
    );
    assert!(check_project(&service.db, &root).is_err());
    assert_eq!(
        service
            .db
            .query_row("SELECT id FROM context_binding_claims", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        unknown.0
    );
    assert_eq!(
        std::fs::read(root.join("private-body")).unwrap(),
        b"private partial state"
    );
}

#[test]
fn validation_failure_after_preparation_releases_only_its_unwritten_root() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap().join("selected");
    std::fs::create_dir(&root).unwrap();
    let service = ContextService::open(temp.path().join("host.db")).unwrap();
    let evidence = service.binding_evidence(&root, "project", true).unwrap();
    // Deterministically introduce the invalid file after read-only preparation.
    std::fs::write(root.join("user-note"), b"ordinary user file").unwrap();
    let error = service
        .bind_evidenced(&root, "project", "Test", true, &evidence)
        .unwrap_err();
    assert!(
        matches!(error, Error::Context(ref e) if e.code == "INVALID_INPUT" && e.effect == "none")
    );
    assert!(!root.join("context.json").exists());
    // The cross-process lock directory is infrastructure; no store/entries are initialized.
    assert!(!root.join("entries").exists());
    assert_eq!(
        std::fs::read(root.join("user-note")).unwrap(),
        b"ordinary user file"
    );
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
    register_project(&service.db, &root).unwrap();
}

#[test]
fn no_effect_failure_cannot_remove_historical_restore_or_binding_exclusion() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    for existing_binding in [false, true] {
        let root = base.join(if existing_binding { "bound" } else { "restore" });
        let service = ContextService::open(base.join(if existing_binding {
            "bound.db"
        } else {
            "restore.db"
        }))
        .unwrap();
        if existing_binding {
            service.bind(&root, "project", "Test", true).unwrap();
            std::fs::remove_file(root.join("context.json")).unwrap();
        } else {
            std::fs::create_dir(&root).unwrap();
            service
                .db
                .execute(
                    "INSERT INTO context_reserved_roots(path) VALUES (?1)",
                    [root.to_string_lossy()],
                )
                .unwrap();
        }
        let evidence = service.binding_evidence(&root, "project", true).unwrap();
        std::fs::write(root.join("user-note"), b"retained").unwrap();
        assert!(
            service
                .bind_evidenced(&root, "project", "Test", true, &evidence)
                .is_err()
        );
        assert!(check_project(&service.db, &root).is_err());
        assert_eq!(
            service
                .db
                .query_row("SELECT COUNT(*) FROM context_reserved_roots", [], |r| r
                    .get::<_, u64>(0))
                .unwrap(),
            1
        );
    }
}

#[test]
fn overlapping_process_claim_prevents_owner_failure_from_unprotecting_active_root() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap().join("selected");
    std::fs::create_dir(&root).unwrap();
    let db = temp.path().join("host.db");
    let owner = ContextService::open(&db).unwrap();
    let other = ContextService::open(&db).unwrap();
    let original = owner.reserve_private_root(&root).unwrap();
    let competing = other.reserve_private_root(&root).unwrap();
    assert_eq!(original.1, Some(original.0.clone()));
    assert_eq!(competing.1, original.1);
    owner
        .release_unwritten_binding_reservation(
            &root,
            &original,
            &directory_identity(&root).unwrap(),
            &oasis7_context::fs::Directory::open(&root).unwrap(),
        )
        .unwrap();
    assert!(check_project(&other.db, &root).is_err());
    assert_eq!(
        other
            .db
            .query_row("SELECT id FROM context_binding_claims", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        competing.0
    );
    // Both claims now prove none; the last releases only this generation's new root.
    other
        .release_unwritten_binding_reservation(
            &root,
            &competing,
            &directory_identity(&root).unwrap(),
            &oasis7_context::fs::Directory::open(&root).unwrap(),
        )
        .unwrap();
    check_project(&other.db, &root).unwrap();
    assert_eq!(
        other
            .db
            .query_row("SELECT COUNT(*) FROM context_owned_reservations", [], |r| r
                .get::<_, u64>(0))
            .unwrap(),
        0
    );
}
