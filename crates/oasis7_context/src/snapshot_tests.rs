use super::*;
fn fixture() -> Snapshot {
    let entry = crate::model::Entry {
        metadata: crate::model::Metadata {
            schema_version: "oasis7.context_entry.v1".into(),
            project_id: "test".into(),
            id: "machine-test".into(),
            kind: "machine".into(),
            title: "虚构测试机器".into(),
            environment: "test".into(),
            owner: "human:test".into(),
            tags: vec![],
            state: "active".into(),
            updated_at: "2026-10-11T00:00:00Z".into(),
            updated_by: "human:test".into(),
            verification: None,
            credential_refs: vec![],
            links: vec![],
        },
        body: "Test fixture; no real credentials.\n".into(),
    };
    let bytes = crate::format::encode(&entry).unwrap();
    Snapshot {
        schema_version: "oasis7.context_snapshot.v1".into(),
        store: StoreConfig {
            schema_version: "oasis7.context_store.v1".into(),
            store_id: "0123456789abcdef0123456789abcdef".into(),
            project_id: "test".into(),
            name: "Synthetic fixtures".into(),
        },
        created_at: "2026-10-11T00:00:00Z".into(),
        scope: Scope {
            mode: "all".into(),
            entry_ids: vec!["machine-test".into()],
        },
        source: Source {
            kind: "working_tree".into(),
            git_head: None,
            working_tree_dirty: None,
            git_history_included: false,
        },
        entries: vec![Manifest {
            id: "machine-test".into(),
            byte_length: bytes.len(),
            sha256: digest(&bytes),
            content_utf8: String::from_utf8(bytes).unwrap(),
        }],
    }
}
#[test]
fn rejects_manifest_disagreement_and_path_traversal() {
    let mut s = fixture();
    assert_eq!(s.validate().unwrap().entry_count, 1);
    s.entries[0].byte_length += 1;
    assert!(s.validate().is_err());
    let mut s = fixture();
    s.entries[0].sha256 = "0".repeat(64);
    assert!(s.validate().is_err());
    let mut s = fixture();
    s.scope.entry_ids.clear();
    assert!(s.validate().is_err());
    let mut s = fixture();
    s.entries[0].id = "../secret".into();
    s.scope.entry_ids = vec!["../secret".into()];
    assert!(s.validate().is_err());
    let mut s = fixture();
    s.store.project_id = "other".into();
    assert!(s.validate().is_err());
    let mut s = fixture();
    s.source.git_history_included = true;
    assert!(s.validate().is_err());
}
#[test]
fn rejects_duplicate_unknown_and_deep_json() {
    let bytes = serde_json::to_vec(&fixture()).unwrap();
    let mut v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    v["secret_extra"] = "forbidden".into();
    assert!(
        crate::format::strict_json_bounded::<Snapshot>(
            &serde_json::to_vec(&v).unwrap(),
            MAX_PLAINTEXT_BYTES
        )
        .is_err()
    );
    let mut bytes = String::from_utf8(bytes).unwrap();
    bytes.insert_str(1, "\"schema_version\":\"oasis7.context_snapshot.v1\",");
    assert!(
        crate::format::strict_json_bounded::<Snapshot>(bytes.as_bytes(), MAX_PLAINTEXT_BYTES)
            .is_err()
    );
}
#[test]
fn export_creates_only_external_envelope_and_holds_store_identity() {
    let base = tempfile::tempdir().unwrap();
    let base_path = base.path().canonicalize().unwrap();
    let root = base_path.join("source");
    let store = crate::store::Store::init(&root, "test", "Synthetic fixtures").unwrap();
    let entry = &fixture().entries[0].content_utf8;
    store
        .put(
            "machine-test",
            entry.as_bytes(),
            crate::model::WriteCondition::Absent,
            "human:test",
        )
        .unwrap();
    let pass = b"synthetic snapshot passphrase";
    assert!(export(&root, &root.join("leak.enc"), &[], pass, 0).is_err());
    assert!(!root.join("leak.enc").exists());
    let output = base_path.join("snapshot.enc");
    let summary = export(&root, &output, &[], pass, 0).unwrap();
    assert_eq!(summary.store_id, store.config.store_id);
    let original = std::fs::read(&output).unwrap();
    assert!(export(&root, &output, &[], pass, 0).is_err());
    assert_eq!(std::fs::read(&output).unwrap(), original);
    assert_eq!(
        inspect(&output, pass).unwrap().entry_ids,
        vec!["machine-test"]
    );
}
#[test]
fn authenticated_invalid_manifest_creates_no_restore_target() {
    let base = tempfile::tempdir().unwrap();
    let base_path = base.path().canonicalize().unwrap();
    let input = base_path.join("bad.enc");
    let target = base_path.join("restored");
    let pass = b"synthetic snapshot passphrase";
    let mut snapshot = fixture();
    snapshot.entries[0].byte_length += 1;
    let bytes = serde_json::to_vec(&snapshot).unwrap();
    std::fs::write(
        &input,
        oasis7_secure_envelope::seal(KIND, &bytes, pass).unwrap(),
    )
    .unwrap();
    assert_eq!(
        restore(&input, &target, None, None, pass).unwrap_err().code,
        "SNAPSHOT_INVALID"
    );
    assert!(!target.exists());
    assert_eq!(std::fs::read_dir(base.path()).unwrap().count(), 1);
}
#[test]
fn restored_store_preserves_identity_and_excludes_local_state() {
    let base = std::env::temp_dir().join(format!(
        "oasis7-context-snapshot-test-{}",
        random_id().unwrap()
    ));
    std::fs::create_dir(&base).unwrap();
    let base = base.canonicalize().unwrap();
    let input = base.join("snapshot.enc");
    let target = base.join("restored");
    let pass = b"synthetic test passphrase";
    let bytes = serde_json::to_vec(&fixture()).unwrap();
    std::fs::write(
        &input,
        oasis7_secure_envelope::seal(KIND, &bytes, pass).unwrap(),
    )
    .unwrap();
    let s = restore(&input, &target, Some("test"), None, pass).unwrap();
    assert_eq!(s.store_id, "0123456789abcdef0123456789abcdef");
    assert!(target.join("entries/machine-test.md").is_file());
    assert!(!target.join(".git").exists());
    assert!(!target.join(".context-local/lock").exists());
    assert_eq!(
        restore(&input, &target, None, None, pass).unwrap_err().code,
        "RESTORE_TARGET_EXISTS"
    );
    let invalid_target = base.join("invalid");
    assert!(restore(&input, &invalid_target, Some("other"), None, pass).is_err());
    assert!(!invalid_target.exists());
    assert!(inspect(&input, b"wrong test passphrase").is_err());
    std::fs::remove_dir_all(base).unwrap();
}
