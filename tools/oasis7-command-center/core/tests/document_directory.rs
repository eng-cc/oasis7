use oasis7_command_center_core::{
    CommandCenter, document_directory::DocumentDirectoryQuery,
    document_inventory::DocumentInventoryQuery,
};
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
fn fixture(files: &[(&str, &str)]) -> (tempfile::TempDir, CommandCenter, DocumentDirectoryQuery) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    fs::create_dir_all(&root).unwrap();
    git(&root, &["init"]);
    git(&root, &["config", "user.name", "test"]);
    git(&root, &["config", "user.email", "test@example.org"]);
    for (path, content) in files {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "fixture"]);
    let mut core = CommandCenter::open(tmp.path().join("db")).unwrap();
    let snapshot = core.open_project(&root).unwrap();
    let query = DocumentDirectoryQuery {
        parent_path: String::new(),
        inventory: DocumentInventoryQuery {
            schema_version: 1,
            project_id: snapshot.project.unwrap().id,
            snapshot_id: None,
            query: String::new(),
            read_filter: "all".into(),
            cursor: None,
        },
    };
    (tmp, core, query)
}
#[test]
fn projects_paths_without_semantic_edges_and_retains_search_ancestors() {
    let (tmp, mut core, mut query) = fixture(&[
        ("AGENTS.md", "# Entry\n"),
        ("doc/README.md", "# Docs\n"),
        ("doc/a/prd.md", "# Requirement\n- 文档 ID：UNIQUE-ID\n"),
        ("doc/a/topic.design.md", "# Design\n"),
        ("doc/b/omitted.design.md", "# Deleted\n"),
        ("doc/plain.md", "# Unscoped body\n"),
    ]);
    fs::remove_file(tmp.path().join("repo/doc/b/omitted.design.md")).unwrap();
    core.refresh().unwrap();
    let root = core.document_directory(query.clone()).unwrap();
    assert_eq!(root.candidates, 6);
    assert_eq!(root.eligible_files, 5);
    assert_eq!(root.omitted, 1);
    assert_eq!(
        root.entries
            .iter()
            .map(|e| e.path.as_str())
            .collect::<Vec<_>>(),
        ["AGENTS.md", "doc"]
    );
    query.inventory.query = "unique-id".into();
    let root = core.document_directory(query.clone()).unwrap();
    assert_eq!(root.matching_files, 1);
    assert_eq!(root.entries[0].path, "doc");
    query.parent_path = "doc".into();
    query.inventory.snapshot_id = Some(root.snapshot_id);
    let doc = core.document_directory(query.clone()).unwrap();
    assert_eq!(doc.entries[0].path, "doc/a");
    query.parent_path = "doc/a".into();
    let leaf = core.document_directory(query.clone()).unwrap();
    assert_eq!(leaf.entries.len(), 1);
    assert_eq!(
        leaf.entries[0]
            .file
            .as_ref()
            .unwrap()
            .identity
            .as_ref()
            .unwrap()
            .declared_id
            .as_deref(),
        Some("UNIQUE-ID")
    );
    query.inventory.query.clear();
    query.inventory.read_filter = "omitted".into();
    query.parent_path = "doc/b".into();
    let omitted = core.document_directory(query.clone()).unwrap();
    assert_eq!(omitted.entries[0].file.as_ref().unwrap().state, "deleted");
    query.parent_path = "doc/a".into();
    assert!(core.document_directory(query).unwrap().entries.is_empty());
}
#[test]
fn directory_pagination_binds_parent_filter_snapshot_schema_and_project() {
    let files: Vec<_> = (0..105)
        .map(|i| (format!("doc/{i:03}/design.md"), "# Design\n".to_owned()))
        .collect();
    let refs: Vec<_> = files
        .iter()
        .map(|(p, c)| (p.as_str(), c.as_str()))
        .collect();
    let (tmp, mut core, mut query) = fixture(&refs);
    query.parent_path = "doc".into();
    let first = core.document_directory(query.clone()).unwrap();
    assert_eq!(first.total_children, 105);
    assert_eq!(first.matching_files, 105);
    assert_eq!(first.entries.len(), 100);
    query.inventory.cursor = first.next_cursor.clone();
    query.inventory.snapshot_id = Some(first.snapshot_id.clone());
    let next = core.document_directory(query.clone()).unwrap();
    assert_eq!(next.entries.len(), 5);
    assert_eq!(next.entries[0].path, "doc/100");
    assert!(next.next_cursor.is_none());
    let mut bad = query.clone();
    bad.parent_path.clear();
    assert!(core.document_directory(bad).is_err());
    let mut bad = query.clone();
    bad.inventory.query = "missing".into();
    assert!(core.document_directory(bad).is_err());
    let mut bad = query.clone();
    bad.inventory.schema_version = 2;
    assert!(core.document_directory(bad).is_err());
    let mut bad = query.clone();
    bad.inventory.project_id = "other".into();
    assert!(core.document_directory(bad).is_err());
    let old = core.snapshot();
    drop(core);
    let reopened = CommandCenter::open(tmp.path().join("db")).unwrap();
    assert_eq!(
        serde_json::to_value(old).unwrap(),
        serde_json::to_value(reopened.snapshot()).unwrap()
    );
    assert_eq!(
        reopened
            .document_directory(query.clone())
            .unwrap()
            .entries
            .len(),
        5
    );
    core = reopened;
    fs::write(tmp.path().join("repo/doc/000/design.md"), "# Changed\n").unwrap();
    core.refresh().unwrap();
    assert!(core.document_directory(query).is_err());
}
#[test]
fn invalid_parents_are_rejected_and_deep_paths_remain_visible_as_limited_entries() {
    let deep = format!("doc/{}/design.md", vec!["x"; 65].join("/"));
    let (_tmp, core, mut query) = fixture(&[(&deep, "# Deep\n")]);
    for path in ["/doc", "doc/", "doc//x", "doc/../x", "doc/./x", "absent"] {
        query.parent_path = path.into();
        assert!(core.document_directory(query.clone()).is_err(), "{path}");
    }
    query.parent_path = format!("doc/{}", vec!["x"; 62].join("/"));
    let page = core.document_directory(query.clone()).unwrap();
    assert!(page.entries[0].depth_limited);
    assert_eq!(page.entries[0].matching_files, 1);
    query.parent_path = page.entries[0].path.clone();
    assert!(core.document_directory(query).is_err());
}
