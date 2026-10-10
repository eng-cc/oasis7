use super::*;

fn backup_ready(admin: &crate::admin::AdminStore) {
    admin
        .set_purpose_enabled("rollback_strict_audit_v1", false)
        .unwrap();
    for d in ["control", "control/grants", "control/revoked-grants"] {
        fs::set_permissions(admin.root.join(d), fs::Permissions::from_mode(0o750)).unwrap();
    }
}
fn empty_restore_fixture(store: &SignerStore) {
    for d in ["keys", "control", "state"] {
        fs::remove_dir_all(store.root.join(d)).unwrap();
        create_private_dir(&store.root.join(d)).unwrap();
    }
    for d in ["control/grants", "control/revoked-grants", "state/records"] {
        create_private_dir(&store.root.join(d)).unwrap();
    }
    for d in ["control", "control/grants", "control/revoked-grants"] {
        fs::set_permissions(store.root.join(d), fs::Permissions::from_mode(0o750)).unwrap();
    }
}
#[test]
fn backup_restores_actual_committed_record_without_reenabling_authority() {
    let (_scratch, store, request, payload, request_key, _) = fixture();
    store
        .handle_request_at(request.clone(), unix_time_ms().unwrap())
        .unwrap();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    backup_ready(&admin);
    assert!(matches!(
        admin.backup_encrypted(b"a sufficiently long passphrase", false),
        Err(SignerError::AuthorizationDenied)
    ));
    let encrypted = admin
        .backup_encrypted(b"a sufficiently long passphrase", true)
        .unwrap();
    let (_target_scratch, target, _, _, _, _) = fixture();
    empty_restore_fixture(&target);
    let restore = crate::admin::AdminStore::from_signer_fixture(&target);
    let plan = restore
        .restore_plan(&encrypted, b"a sufficiently long passphrase")
        .unwrap();
    restore
        .restore_commit(&encrypted, b"a sufficiently long passphrase", &plan)
        .unwrap();
    restore
        .restore_commit(&encrypted, b"a sufficiently long passphrase", &plan)
        .unwrap();
    let dir = target.root.join(RECORDS_DIR).join(&request_key);
    let reservation: Reservation = read_json(&dir.join("reservation.json")).unwrap();
    let (response, _, _) = read_committed_result(&dir, &reservation).unwrap();
    verify_response_signature(&response, &reservation.public_key_sha256, &payload).unwrap();
    let policy: Policy = read_control(
        &target.root.join(POLICY_FILE),
        target.control_owner_uid,
        target.control_group_gid,
    )
    .unwrap();
    assert!(policy.enabled_purposes.is_empty());
    assert!(
        target
            .root
            .join(REVOKED_GRANTS_DIR)
            .join("grant-01.json")
            .exists()
    );
    assert!(matches!(
        target.handle_request_at(request, unix_time_ms().unwrap()),
        Err(SignerError::AuthorizationDenied)
    ));
}
#[test]
fn backup_rejects_corrupt_committed_record_and_incomplete_key_before_restore_write() {
    let (_scratch, store, request, _, request_key, _) = fixture();
    store
        .handle_request_at(request, unix_time_ms().unwrap())
        .unwrap();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    backup_ready(&admin);
    let password = b"a sufficiently long passphrase";
    let encrypted = admin.backup_encrypted(password, true).unwrap();
    let plain = crate::key_envelope::open("backup", &encrypted, password).unwrap();
    let snapshot: serde_json::Value = serde_json::from_slice(&plain).unwrap();
    let (_target_scratch, target, _, _, _, _) = fixture();
    empty_restore_fixture(&target);
    let restore = crate::admin::AdminStore::from_signer_fixture(&target);
    for path in [
        format!("state/records/{request_key}/result/manifest.json"),
        "keys/signer-01/seed.bin".into(),
    ] {
        let mut changed = snapshot.clone();
        changed["files"].as_object_mut().unwrap().remove(&path);
        let envelope =
            crate::key_envelope::seal("backup", &canonical_json(&changed).unwrap(), password)
                .unwrap();
        assert!(restore.restore_plan(&envelope, password).is_err());
        assert!(!target.root.join("control/restore.json").exists());
        assert_eq!(fs::read_dir(target.root.join("keys")).unwrap().count(), 0);
    }
    let mut changed = snapshot;
    let policy_path = "control/policy.json";
    let raw = base64::engine::general_purpose::STANDARD
        .decode(changed["files"][policy_path].as_str().unwrap())
        .unwrap();
    let mut policy: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    policy["unknown"] = true.into();
    changed["files"][policy_path] = base64::engine::general_purpose::STANDARD
        .encode(canonical_json(&policy).unwrap())
        .into();
    let envelope =
        crate::key_envelope::seal("backup", &canonical_json(&changed).unwrap(), password).unwrap();
    assert!(restore.restore_plan(&envelope, password).is_err());
}

#[test]
fn backup_restore_retry_syncs_existing_tree_after_directory_sync_failure() {
    let (_scratch, store, request, _, _, _) = fixture();
    store
        .handle_request_at(request, unix_time_ms().unwrap())
        .unwrap();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    backup_ready(&admin);
    let password = b"a sufficiently long passphrase";
    let encrypted = admin.backup_encrypted(password, true).unwrap();
    let (_target_scratch, target, _, _, _, _) = fixture();
    empty_restore_fixture(&target);
    let restore = crate::admin::AdminStore::from_signer_fixture(&target);
    let plan = restore.restore_plan(&encrypted, password).unwrap();
    crate::backup::FAIL_RESTORE_DIR_SYNC.with(|f| f.set(true));
    assert!(matches!(
        restore.restore_commit(&encrypted, password, &plan),
        Err(SignerError::PersistenceFailed(_))
    ));
    let receipt: serde_json::Value =
        serde_json::from_slice(&fs::read(target.root.join("control/restore.json")).unwrap())
            .unwrap();
    assert_eq!(receipt["phase"], "prepared");
    assert_eq!(
        restore.read_maintenance().unwrap().unwrap().phase,
        "prepared"
    );
    restore.restore_commit(&encrypted, password, &plan).unwrap();
    let receipt: serde_json::Value =
        serde_json::from_slice(&fs::read(target.root.join("control/restore.json")).unwrap())
            .unwrap();
    assert_eq!(receipt["phase"], "committed");
    assert_eq!(
        restore.read_maintenance().unwrap().unwrap().phase,
        "committed"
    );
}

#[test]
fn backup_nonexportable_check_covers_deleted_seed_residue_under_lock() {
    let (_scratch, store, _, _, _, _) = fixture();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    backup_ready(&admin);
    let mut catalog = admin.catalog().unwrap();
    catalog.keys = admin.key_list().unwrap();
    assert_eq!(catalog.keys.len(), 1);
    catalog.keys[0].exportable = true;
    catalog.keys[0].state = "deleted".into();
    write_control_fixture(
        &store.root.join(crate::key_management::CATALOG),
        &canonical_json(&catalog).unwrap(),
    );
    assert!(store.root.join("keys/signer-01/seed.bin").exists());
    assert!(matches!(
        admin.backup_encrypted(b"a sufficiently long passphrase", false),
        Err(SignerError::AuthorizationDenied)
    ));
}

#[test]
fn inactive_catalog_key_blocks_worker_sign_and_committed_inspect_replay() {
    let (_scratch, store, request, _, _, _) = fixture();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    admin.key_set_state("signer-01", "active").unwrap();
    let now = unix_time_ms().unwrap();
    assert!(
        matches!(store.handle_request_at(request.clone(),now),Ok(IpcResponse::Sign(SignResult{status,..})) if status=="committed")
    );
    let inspect = IpcRequest::Inspect {
        schema_version: IPC_SCHEMA.into(),
        installation_id: "install-01".into(),
        request_id: "request-01".into(),
        purpose: "rollback_strict_audit_v1".into(),
        provider_id: ExplicitNull(None),
    };
    assert!(matches!(
        store.handle_request_at(inspect.clone(), now + 1),
        Ok(IpcResponse::Inspect(_))
    ));
    admin.key_set_state("signer-01", "inactive").unwrap();
    assert!(matches!(
        store.handle_request_at(request, now + 2),
        Err(SignerError::AuthorizationDenied)
    ));
    assert!(matches!(
        store.handle_request_at(inspect, now + 2),
        Err(SignerError::AuthorizationDenied)
    ));
}

#[test]
fn doctor_is_unready_for_each_inactive_lifecycle_and_disabled_purpose() {
    let (_scratch, store, _, _, _, _) = fixture();
    let admin = crate::admin::AdminStore::from_signer_fixture(&store);
    let doctor = || match store.doctor().unwrap() {
        IpcResponse::Doctor(response) => response,
        _ => panic!("expected doctor response"),
    };
    assert!(doctor().ready);
    for state in ["inactive", "archived"] {
        admin.key_set_state("signer-01", state).unwrap();
        let response = doctor();
        assert!(!response.ready);
        assert_eq!(response.status_code, "AUTHORIZATION_DENIED");
        admin.key_set_state("signer-01", "active").unwrap();
        assert!(doctor().ready);
    }
    admin.key_set_state("signer-01", "inactive").unwrap();
    admin.key_set_state("signer-01", "deleted").unwrap();
    assert!(!doctor().ready);
    let mut policy: Policy = read_control(
        &store.root.join(POLICY_FILE),
        store.control_owner_uid,
        store.control_group_gid,
    )
    .unwrap();
    assert_eq!(policy.enabled_purposes, vec!["rollback_strict_audit_v1"]);
    policy.enabled_purposes.clear();
    fs::remove_file(store.root.join(POLICY_FILE)).unwrap();
    write_control_fixture(
        &store.root.join(POLICY_FILE),
        &canonical_json(&policy).unwrap(),
    );
    let response = doctor();
    assert!(!response.ready);
    assert_eq!(response.status_code, "DISABLED");
}
