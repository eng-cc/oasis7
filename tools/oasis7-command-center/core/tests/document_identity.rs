use oasis7_command_center_core::{CommandCenter, document_inventory::DocumentInventoryQuery};
use std::{fs, path::Path, process::Command};
fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .unwrap()
            .status
            .success()
    );
}
#[test]
fn declared_id_search_keeps_duplicate_files_and_original_snapshot() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    fs::create_dir_all(root.join("doc")).unwrap();
    git(&root, &["init"]);
    git(&root, &["config", "user.name", "test"]);
    git(&root, &["config", "user.email", "test@example.org"]);
    for name in ["one.design.md", "two.prd.md"] {
        fs::write(
            root.join("doc").join(name),
            "# Different title\n\n- 文档 ID：`DECLARED-123`\n\n## Body\n",
        )
        .unwrap();
    }
    fs::write(
        root.join("doc/ignored.md"),
        "# Body only\n\n## Body\n- 文档 ID：DECLARED-123\n",
    )
    .unwrap();
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "fixture"]);
    let db_path = tmp.path().join("db");
    let mut core = CommandCenter::open(&db_path).unwrap();
    let snapshot = core.open_project(&root).unwrap();
    let query = DocumentInventoryQuery {
        schema_version: 1,
        project_id: snapshot.project.as_ref().unwrap().id.clone(),
        snapshot_id: None,
        query: "declared-123".into(),
        read_filter: "all".into(),
        cursor: None,
    };
    let page = core.document_inventory(query.clone()).unwrap();
    assert_eq!(page.total_matches, 2);
    assert_ne!(page.files[0].file_key, page.files[1].file_key);
    for file in &page.files {
        let value = serde_json::to_value(file).unwrap();
        assert_eq!(value["identity"]["declaredId"], "DECLARED-123");
        assert_eq!(value["identity"]["declarationLine"], 3);
        assert_eq!(value["identity"]["duplicateFiles"], 1);
    }
    // Older inventory caches must still open without changing personal/Snapshot data.
    drop(core);
    let db = rusqlite::Connection::open(&db_path).unwrap();
    let key = format!("document_inventory_v1:{}", query.project_id);
    let raw: String = db
        .query_row("SELECT value FROM cache WHERE key=?1", [&key], |r| r.get(0))
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    for file in value["files"].as_array_mut().unwrap() {
        file.as_object_mut().unwrap().remove("identity");
    }
    db.execute(
        "UPDATE cache SET value=?1 WHERE key=?2",
        [value.to_string(), key],
    )
    .unwrap();
    let core = CommandCenter::open(&db_path).unwrap();
    assert_eq!(
        serde_json::to_value(snapshot).unwrap(),
        serde_json::to_value(core.snapshot()).unwrap()
    );
    assert_eq!(core.document_inventory(query).unwrap().total_matches, 0);
}

#[test]
fn duplicate_diagnostics_are_bounded_exact_and_independent_of_paging() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    fs::create_dir_all(root.join("doc")).unwrap();
    git(&root, &["init"]);
    git(&root, &["config", "user.name", "test"]);
    git(&root, &["config", "user.email", "test@example.org"]);
    for i in 0..105 {
        fs::write(
            root.join(format!("doc/{i:03}.md")),
            "# File\n- 文档 ID：SAME-ID\n",
        )
        .unwrap();
    }
    fs::write(root.join("doc/lower.md"), "# Lower\n- 文档 ID：same-id\n").unwrap();
    fs::write(root.join("doc/unread.md"), [0xff]).unwrap();
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "fixture"]);
    let mut core = CommandCenter::open(tmp.path().join("db")).unwrap();
    let snapshot = core.open_project(&root).unwrap();
    let mut query = DocumentInventoryQuery {
        schema_version: 1,
        project_id: snapshot.project.unwrap().id,
        snapshot_id: None,
        query: "same-id".into(),
        read_filter: "all".into(),
        cursor: None,
    };
    let first = core.document_inventory(query.clone()).unwrap();
    assert_eq!(first.total_matches, 106);
    assert_eq!(first.files.len(), 100);
    let identity = first.files[0].identity.as_ref().unwrap();
    assert_eq!(identity.duplicate_files, 104);
    assert_eq!(identity.duplicate_paths.len(), 20);
    assert_eq!(identity.duplicate_paths[0], "doc/001.md");
    query.cursor = first.next_cursor;
    query.snapshot_id = Some(first.snapshot_id);
    let second = core.document_inventory(query.clone()).unwrap();
    assert_eq!(second.files.len(), 6);
    assert!(second.next_cursor.is_none());
    assert_eq!(
        second
            .files
            .last()
            .unwrap()
            .identity
            .as_ref()
            .unwrap()
            .duplicate_files,
        0
    );
    fs::write(root.join("doc/000.md"), "# Changed\n- 文档 ID：UNIQUE\n").unwrap();
    core.refresh().unwrap();
    assert!(core.document_inventory(query.clone()).is_err());
    query.cursor = None;
    query.snapshot_id = None;
    query.query = "UNIQUE".into();
    let updated = core.document_inventory(query.clone()).unwrap();
    assert_eq!(updated.total_matches, 1);
    assert_eq!(
        updated.files[0].identity.as_ref().unwrap().duplicate_files,
        0
    );
    query.query = "unread.md".into();
    assert!(
        core.document_inventory(query).unwrap().files[0]
            .identity
            .is_none()
    );
}
