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
fn fixture() -> (tempfile::TempDir, CommandCenter, DocumentInventoryQuery) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    fs::create_dir(&root).unwrap();
    for args in [
        vec!["init"],
        vec!["config", "user.name", "test"],
        vec!["config", "user.email", "test@example.org"],
    ] {
        git(&root, &args);
    }
    fs::create_dir(root.join("doc")).unwrap();
    fs::write(
        root.join("doc/prd.md"),
        "# **产品** `API`\n\n- 状态：draft；目标\n",
    )
    .unwrap();
    fs::write(root.join("doc/empty.md"), "").unwrap();
    fs::write(root.join("doc/deleted.md"), "# Deleted\n").unwrap();
    fs::write(root.join("doc/invalid.md"), "# Original\n").unwrap();
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "fixture"]);
    fs::remove_file(root.join("doc/deleted.md")).unwrap();
    fs::write(root.join("doc/invalid.md"), [0xff]).unwrap();
    let mut core = CommandCenter::open(tmp.path().join("db")).unwrap();
    let snapshot = core.open_project(&root).unwrap();
    let query = DocumentInventoryQuery {
        schema_version: 1,
        project_id: snapshot.project.unwrap().id,
        snapshot_id: None,
        query: String::new(),
        read_filter: "all".into(),
        cursor: None,
    };
    (tmp, core, query)
}

#[test]
fn records_empty_deleted_invalid_and_read_files_without_inventing_content() {
    let (_tmp, core, query) = fixture();
    let page = core.document_inventory(query.clone()).unwrap();
    assert_eq!((page.candidates, page.read, page.omitted), (4, 2, 2));
    assert!(page.partial && !page.stale);
    let empty = page
        .files
        .iter()
        .find(|f| f.path == "doc/empty.md")
        .unwrap();
    assert_eq!(empty.bytes, Some(0));
    assert!(empty.revision.is_some());
    let deleted = page
        .files
        .iter()
        .find(|f| f.path == "doc/deleted.md")
        .unwrap();
    assert_eq!(deleted.state, "deleted");
    assert_eq!(deleted.reason.as_deref(), Some("deleted"));
    assert!(deleted.revision.is_none() && deleted.bytes.is_none() && deleted.dirty);
    let invalid = page
        .files
        .iter()
        .find(|f| f.path == "doc/invalid.md")
        .unwrap();
    assert_eq!(invalid.state, "error");
    assert_eq!(invalid.reason.as_deref(), Some("invalid_utf8"));
    let product = page.files.iter().find(|f| f.path == "doc/prd.md").unwrap();
    assert_eq!(product.title, "产品 API");
    assert_eq!(product.kind, "prd");
    assert_eq!(
        product.lifecycle.as_ref().unwrap().category.as_deref(),
        Some("draft")
    );
    assert_eq!(
        core.document_inventory(DocumentInventoryQuery {
            query: "产品".into(),
            ..query.clone()
        })
        .unwrap()
        .total_matches,
        1
    );
    assert_eq!(
        core.document_inventory(DocumentInventoryQuery {
            read_filter: "omitted".into(),
            ..query
        })
        .unwrap()
        .total_matches,
        2
    );
}

#[test]
fn pagination_binds_snapshot_filter_and_project_and_survives_restart() {
    let (tmp, mut core, query) = fixture();
    let root = tmp.path().join("repo");
    for index in 0..205 {
        fs::write(root.join(format!("doc/file-{index:03}.md")), "").unwrap();
    }
    git(&root, &["add", "doc"]);
    core.refresh().unwrap();
    let first = core.document_inventory(query.clone()).unwrap();
    assert_eq!((first.files.len(), first.total_matches), (100, 208));
    let next = DocumentInventoryQuery {
        snapshot_id: Some(first.snapshot_id.clone()),
        cursor: first.next_cursor.clone(),
        ..query.clone()
    };
    let second = core.document_inventory(next.clone()).unwrap();
    assert_eq!(second.files.len(), 100);
    assert!(first.files.last().unwrap().path < second.files.first().unwrap().path);
    let third = core
        .document_inventory(DocumentInventoryQuery {
            cursor: second.next_cursor,
            ..next.clone()
        })
        .unwrap();
    assert_eq!(third.files.len(), 8);
    assert!(third.next_cursor.is_none());
    assert!(
        core.document_inventory(DocumentInventoryQuery {
            query: "file-".into(),
            ..next.clone()
        })
        .is_err()
    );
    assert!(
        core.document_inventory(DocumentInventoryQuery {
            project_id: "other".into(),
            ..query.clone()
        })
        .is_err()
    );
    assert!(
        core.document_inventory(DocumentInventoryQuery {
            schema_version: 2,
            ..query.clone()
        })
        .is_err()
    );
    drop(core);
    let mut core = CommandCenter::open(tmp.path().join("db")).unwrap();
    assert_eq!(
        core.document_inventory(next.clone()).unwrap().snapshot_id,
        first.snapshot_id
    );
    fs::write(root.join("doc/file-000.md"), "# Changed\n").unwrap();
    core.refresh().unwrap();
    assert!(
        core.document_inventory(next)
            .unwrap_err()
            .to_string()
            .contains("expired")
    );
    assert_ne!(
        core.document_inventory(query).unwrap().snapshot_id,
        first.snapshot_id
    );
}

#[test]
fn failed_transaction_keeps_manifest_sections_and_personal_data_together() {
    let (tmp, mut core, query) = fixture();
    let before = core.document_inventory(query.clone()).unwrap();
    let root = tmp.path().join("repo");
    let mut preferences = core.snapshot().preferences;
    preferences.query = "个人偏好".into();
    core.save_preferences(preferences).unwrap();
    let db = rusqlite::Connection::open(tmp.path().join("db")).unwrap();
    db.execute_batch("CREATE TRIGGER reject_inventory BEFORE INSERT ON cache WHEN new.key LIKE 'document_inventory_v1:%' BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    fs::write(root.join("doc/prd.md"), "# New\n").unwrap();
    assert!(core.refresh().is_err());
    assert_eq!(
        core.document_inventory(query.clone()).unwrap().snapshot_id,
        before.snapshot_id
    );
    drop(core);
    let mut reopened = CommandCenter::open(tmp.path().join("db")).unwrap();
    assert_eq!(reopened.snapshot().preferences.query, "个人偏好");
    assert_eq!(
        reopened
            .document_inventory(query.clone())
            .unwrap()
            .snapshot_id,
        before.snapshot_id
    );
    db.execute_batch("DROP TRIGGER reject_inventory").unwrap();
    reopened.refresh().unwrap();
    assert_ne!(
        reopened.document_inventory(query).unwrap().snapshot_id,
        before.snapshot_id
    );
}

#[test]
fn absent_corrupt_or_future_inventory_does_not_break_old_snapshot_and_can_be_rebuilt() {
    let (tmp, mut core, query) = fixture();
    let db = rusqlite::Connection::open(tmp.path().join("db")).unwrap();
    let key = format!("document_inventory_v1:{}", query.project_id);
    for value in [
        None,
        Some("{broken"),
        Some(
            "{\"schemaVersion\":99,\"projectId\":\"x\",\"baselineOid\":\"x\",\"snapshotId\":\"x\",\"generatedAt\":\"0\",\"sectionsDigest\":\"x\",\"files\":[]}",
        ),
    ] {
        db.execute("DELETE FROM cache WHERE key=?1", [&key])
            .unwrap();
        if let Some(value) = value {
            db.execute("INSERT INTO cache(key,value) VALUES (?1,?2)", [&key, value])
                .unwrap();
        }
        drop(core);
        core = CommandCenter::open(tmp.path().join("db")).unwrap();
        assert!(!core.snapshot().sections.is_empty());
        let error = core
            .document_inventory(query.clone())
            .unwrap_err()
            .to_string();
        if value.is_some_and(|v| v.contains("schemaVersion")) {
            assert!(error.contains("unsupported cached"), "{error}");
        }
        core.rebuild_cache().unwrap();
        assert_eq!(
            core.document_inventory(query.clone()).unwrap().candidates,
            4
        );
        let legacy: oasis7_command_center_core::domain::Snapshot = serde_json::from_str(
            &db.query_row::<String, _, _>(
                "SELECT value FROM cache WHERE key='snapshot'",
                [],
                |r| r.get(0),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(!legacy.sections.is_empty());
    }
}

#[test]
fn source_failure_preserves_inventory_and_marks_it_stale() {
    let (tmp, mut core, query) = fixture();
    let before = core.document_inventory(query.clone()).unwrap();
    fs::rename(tmp.path().join("repo/.git"), tmp.path().join("hidden-git")).unwrap();
    core.refresh().unwrap();
    let stale = core.document_inventory(query).unwrap();
    assert_eq!(stale.snapshot_id, before.snapshot_id);
    assert!(stale.stale && stale.error.is_some());
}

#[test]
fn all_over_budget_paths_remain_in_the_paginated_manifest() {
    let (tmp, mut core, query) = fixture();
    let root = tmp.path().join("repo");
    for index in 0..3001 {
        fs::write(root.join(format!("doc/path-{index:04}.md")), "").unwrap();
    }
    git(&root, &["add", "doc"]);
    core.refresh().unwrap();
    let omitted = core
        .document_inventory(DocumentInventoryQuery {
            read_filter: "omitted".into(),
            ..query
        })
        .unwrap();
    assert_eq!(omitted.candidates, 3004);
    assert_eq!(omitted.total_matches, 5); // Invalid UTF-8 plus four paths beyond the read limit.
    assert_eq!(
        omitted
            .files
            .iter()
            .filter(|file| file.reason.as_deref() == Some("file_count_budget"))
            .count(),
        4
    );
    assert!(omitted.files.iter().all(|file| file.revision.is_none()));
}

#[test]
fn legacy_writer_refreshing_empty_files_cannot_make_old_inventory_look_current() {
    let (tmp, mut core, query) = fixture();
    let root = tmp.path().join("repo");
    fs::write(root.join("doc/prd.md"), "").unwrap();
    fs::write(root.join("doc/invalid.md"), "").unwrap();
    core.refresh().unwrap();
    let before = core.document_inventory(query.clone()).unwrap();
    assert!(core.snapshot().sections.is_empty());
    let db = rusqlite::Connection::open(tmp.path().join("db")).unwrap();
    let raw: String = db
        .query_row("SELECT value FROM cache WHERE key='snapshot'", [], |r| {
            r.get(0)
        })
        .unwrap();
    let mut snapshot: serde_json::Value = serde_json::from_str(&raw).unwrap();
    // An older writer updates only its known Snapshot key, leaving the new cache key untouched.
    snapshot["sources"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["kind"] == "local")
        .unwrap()["generation"] = "999".into();
    db.execute(
        "UPDATE cache SET value=?1 WHERE key='snapshot'",
        [snapshot.to_string()],
    )
    .unwrap();
    drop(core);
    let mut reopened = CommandCenter::open(tmp.path().join("db")).unwrap();
    assert!(
        reopened
            .document_inventory(query.clone())
            .unwrap_err()
            .to_string()
            .contains("expired")
    );
    reopened.refresh().unwrap();
    assert_eq!(
        reopened.document_inventory(query).unwrap().candidates,
        before.candidates
    );
}

#[test]
fn explicit_pairings_use_same_candidate_coverage_without_following_targets() {
    let (tmp, mut core, query) = fixture();
    let root = tmp.path().join("repo");
    fs::write(root.join("doc/prd.md"), r#"# Product

- 配对系统设计：[empty](empty.md)、[fragment](empty.md#section)、[deleted](deleted.md)、[invalid](invalid.md)、[untracked](untracked.md)、[unsafe](../.env)、[encoded](%65mpty.md)
- 普通引用：[not paired](other.md)

> - 配对系统设计：[quoted](quoted.md)

```markdown
- 配对系统设计：[example](example.md)
```

- Example:
  - 配对系统设计：[nested](nested.md)

## Body
- 配对系统设计：[body](body.md)
"#).unwrap();
    fs::write(root.join("doc/untracked.md"), "# Secret preview").unwrap();
    core.refresh().unwrap();
    let page = core.document_inventory(query.clone()).unwrap();
    let file = page.files.iter().find(|f| f.path == "doc/prd.md").unwrap();
    let pairings = file.pairings.as_ref().unwrap();
    assert!(pairings.issue.is_none());
    assert_eq!(pairings.targets.len(), 7);
    assert_eq!(
        pairings
            .targets
            .iter()
            .map(|e| e.resolution.as_str())
            .collect::<Vec<_>>(),
        [
            "resolved",
            "fragment_unchecked",
            "missing",
            "unread",
            "outside_scope",
            "outside_scope",
            "resolved"
        ]
    );
    assert!(pairings.targets.iter().all(|e| e.declaration_line == 3));
    assert_eq!(pairings.targets[6].target.as_deref(), Some("doc/empty.md"));
    let id = page.snapshot_id;
    fs::write(
        root.join("doc/prd.md"),
        "# Product\n\n- 配对系统设计：[invalid](invalid.md)\n",
    )
    .unwrap();
    core.refresh().unwrap();
    assert_ne!(id, core.document_inventory(query).unwrap().snapshot_id);
}

#[test]
fn pairing_bounds_and_legacy_caches_do_not_claim_complete_absence() {
    let (tmp, mut core, query) = fixture();
    let root = tmp.path().join("repo");
    let links = (0..33)
        .map(|i| format!("[target{i}](empty.md)"))
        .collect::<Vec<_>>()
        .join(" ");
    fs::write(
        root.join("doc/prd.md"),
        format!("# Product\n\n- 配对系统设计：{links}\n"),
    )
    .unwrap();
    core.refresh().unwrap();
    let page = core.document_inventory(query.clone()).unwrap();
    let pairs = page
        .files
        .iter()
        .find(|f| f.path == "doc/prd.md")
        .unwrap()
        .pairings
        .as_ref()
        .unwrap();
    assert_eq!(pairs.targets.len(), 32);
    assert!(pairs.issue.as_deref().unwrap().contains("incomplete"));
    drop(core);
    let db = rusqlite::Connection::open(tmp.path().join("db")).unwrap();
    let key = format!("document_inventory_v1:{}", query.project_id);
    let value: String = db
        .query_row("SELECT value FROM cache WHERE key=?1", [&key], |r| r.get(0))
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&value).unwrap();
    for file in value["files"].as_array_mut().unwrap() {
        file.as_object_mut().unwrap().remove("pairings");
    }
    db.execute(
        "UPDATE cache SET value=?1 WHERE key=?2",
        [value.to_string(), key],
    )
    .unwrap();
    let reopened = CommandCenter::open(tmp.path().join("db")).unwrap();
    assert!(
        reopened
            .document_inventory(query)
            .unwrap()
            .files
            .iter()
            .all(|f| f.pairings.is_none())
    );
}

#[test]
fn multiline_pairing_declarations_are_never_reported_as_complete() {
    let (tmp, mut core, query) = fixture();
    fs::write(
        tmp.path().join("repo/doc/prd.md"),
        "# Product\n\n- 配对系统设计：[first](empty.md)\n  [second](deleted.md)\n",
    )
    .unwrap();
    core.refresh().unwrap();
    let page = core.document_inventory(query).unwrap();
    let pairs = page
        .files
        .iter()
        .find(|f| f.path == "doc/prd.md")
        .unwrap()
        .pairings
        .as_ref()
        .unwrap();
    assert_eq!(pairs.targets.len(), 1);
    assert!(pairs.issue.as_deref().unwrap().contains("multiline"));
}
