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
        lifecycle_filter: None,
        kind_filter: None,
        pairing_filter: None,
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

#[test]
fn lifecycle_filters_keep_unknowns_and_bind_cursors_without_changing_coverage() {
    let files: Vec<_> = (0..105)
        .map(|i| {
            (
                format!("doc/current/{i:03}.design.md"),
                "# Current\n- 状态：draft\n".to_owned(),
            )
        })
        .chain([
            (
                "doc/history/old.design.md".into(),
                "# Old\n- 状态：superseded\n".into(),
            ),
            (
                "doc/history/retired.design.md".into(),
                "# Retired\n- 状态：retired\n".into(),
            ),
            ("doc/unknown/design.md".into(), "# Unknown\n".into()),
            (
                "doc/professional/design.md".into(),
                "# Professional\n- 状态：target-contract\n".into(),
            ),
        ])
        .collect();
    let refs: Vec<_> = files
        .iter()
        .map(|(p, c)| (p.as_str(), c.as_str()))
        .collect();
    let (_tmp, core, mut query) = fixture(&refs);
    let legacy = core.document_directory(query.clone()).unwrap();
    assert_eq!(legacy.matching_files, 109);
    query.lifecycle_filter = Some("current".into());
    let current = core.document_directory(query.clone()).unwrap();
    assert_eq!(current.matching_files, 107);
    assert_eq!(current.hidden_history_files, 2);
    assert_eq!(current.candidates, legacy.candidates);
    assert_eq!(current.eligible_files, legacy.eligible_files);
    query.parent_path = "doc".into();
    assert!(
        !core
            .document_directory(query.clone())
            .unwrap()
            .entries
            .iter()
            .any(|e| e.name == "history")
    );
    query.parent_path = "doc/current".into();
    query.inventory.cursor = core.document_directory(query.clone()).unwrap().next_cursor;
    assert!(query.inventory.cursor.is_some());
    query.lifecycle_filter = Some("draft".into());
    assert!(core.document_directory(query.clone()).is_err());
    query.inventory.cursor = None;
    query.parent_path.clear();
    query.lifecycle_filter = Some("unknown".into());
    assert_eq!(
        core.document_directory(query.clone())
            .unwrap()
            .matching_files,
        1
    );
    query.lifecycle_filter = Some("raw:target-contract".into());
    assert_eq!(
        core.document_directory(query.clone())
            .unwrap()
            .matching_files,
        1
    );
    query.lifecycle_filter = Some("all".into());
    assert_eq!(
        core.document_directory(query.clone())
            .unwrap()
            .matching_files,
        109
    );
    query.lifecycle_filter = Some("superseded".into());
    assert_eq!(
        core.document_directory(query.clone())
            .unwrap()
            .matching_files,
        1
    );
    let legacy_json = serde_json::to_value(&query).unwrap();
    let mut legacy_json = legacy_json.as_object().unwrap().clone();
    legacy_json.remove("lifecycleFilter");
    legacy_json.remove("kindFilter");
    legacy_json.remove("pairingFilter");
    let legacy_request: DocumentDirectoryQuery =
        serde_json::from_value(legacy_json.into()).unwrap();
    assert_eq!(
        core.document_directory(legacy_request)
            .unwrap()
            .matching_files,
        109
    );
    query.lifecycle_filter = Some(format!("raw:{}", "界".repeat(340)));
    assert_eq!(
        core.document_directory(query.clone())
            .unwrap()
            .matching_files,
        0
    );
    query.lifecycle_filter = Some(format!("raw:{}", "界".repeat(341)));
    assert!(core.document_directory(query.clone()).is_err());
    query.lifecycle_filter = Some("invented".into());
    assert!(core.document_directory(query.clone()).is_err());
    query.lifecycle_filter = Some("raw:".into());
    assert!(core.document_directory(query).is_err());
}

#[test]
fn current_view_keeps_only_verified_historical_references_and_their_ancestors() {
    let (_tmp, core, mut query) = fixture(&[
        (
            "doc/old/design.md",
            "# Old\n- 状态：retired\n\n<a id='OLD-1'></a>\n\n## Old clause\n",
        ),
        ("doc/hidden/design.md", "# Hidden\n- 状态：superseded\n"),
        (
            "doc/current/design.md",
            "# Current\n- 状态：draft\n- 上游：[old](../old/design.md#OLD-1)、[same](../old/design.md#OLD-1)、[bad fragment](../hidden/design.md#absent)\n- 配对系统设计：[old](../old/design.md#OLD-1)\n",
        ),
        (
            "doc/retired-source/design.md",
            "# Retired source\n- 状态：retired\n- 上游：[hidden](../hidden/design.md)\n",
        ),
        ("doc/plain.md", "# Plain\n[ordinary](hidden/design.md)\n"),
    ]);
    query.lifecycle_filter = Some("current".into());
    let root = core.document_directory(query.clone()).unwrap();
    assert_eq!(root.matching_files, 2);
    assert_eq!(root.hidden_history_files, 2);
    query.parent_path = "doc".into();
    let doc = core.document_directory(query.clone()).unwrap();
    assert_eq!(
        doc.entries
            .iter()
            .map(|e| e.name.as_str())
            .collect::<Vec<_>>(),
        ["current", "old"]
    );
    query.parent_path = "doc/old".into();
    let old = core.document_directory(query.clone()).unwrap();
    assert_eq!(old.entries[0].historical_reference_files, 1);
    assert_eq!(
        old.entries[0].historical_reference_paths,
        ["doc/current/design.md"]
    );
    assert_eq!(
        old.entries[0]
            .file
            .as_ref()
            .unwrap()
            .lifecycle
            .as_ref()
            .unwrap()
            .category
            .as_deref(),
        Some("retired")
    );
    query.inventory.query = "Old".into();
    let searched = core.document_directory(query.clone()).unwrap();
    assert_eq!(searched.matching_files, 1);
    assert_eq!(searched.entries[0].historical_reference_files, 1);
    query.lifecycle_filter = Some("all".into());
    let all = core.document_directory(query).unwrap();
    assert_eq!(all.entries[0].historical_reference_files, 0);
}

#[test]
fn history_reference_counts_deduplicate_sources_and_bound_path_samples() {
    let mut files: Vec<_> = (0..25)
        .map(|i| {
            (
                format!("doc/{i:02}.design.md"),
                "# Source\n- 上游：[old](old/design.md)、[duplicate](old/design.md)\n".into(),
            )
        })
        .collect();
    files.push((
        "doc/old/design.md".into(),
        "# Old\n- 状态：retired\n".into(),
    ));
    files.push(("doc/table.design.md".into(), "# Allocation\n\n## 需求承接与分配表\n\n| 上游条款 | 本设计条款 |\n| --- | --- |\n| [old](old/design.md) | [old](old/design.md) |\n".into()));
    let refs: Vec<_> = files
        .iter()
        .map(|(p, c): &(String, String)| (p.as_str(), c.as_str()))
        .collect();
    let (_tmp, core, mut query) = fixture(&refs);
    query.lifecycle_filter = Some("current".into());
    query.parent_path = "doc/old".into();
    let page = core.document_directory(query).unwrap();
    let entry = &page.entries[0];
    assert_eq!(entry.historical_reference_files, 26);
    assert_eq!(entry.historical_reference_paths.len(), 20);
    assert_eq!(entry.historical_reference_paths[0], "doc/00.design.md");
    assert_eq!(entry.historical_reference_paths[19], "doc/19.design.md");
    assert_eq!(page.hidden_history_files, 0);
    assert_eq!(page.matching_files, 27);
}

#[test]
fn kind_filters_preserve_coverage_and_cross_kind_history_evidence() {
    let files: Vec<_> = (0..105)
        .map(|i| (format!("doc/{i:03}.prd.md"), "# Requirements\n".into()))
        .chain([
            ("AGENTS.md".into(), "# Entry\n".into()),
            ("doc/README.md".into(), "# Docs\n".into()),
            (
                "doc/old/design.md".into(),
                "# Old\n- 状态：retired\n".into(),
            ),
            (
                "doc/source/prd.md".into(),
                "# Source\n- 配对系统设计：[old](../old/design.md)\n".into(),
            ),
            ("doc/plain.md".into(), "# Body\n".into()),
        ])
        .collect();
    let refs: Vec<_> = files
        .iter()
        .map(|(p, c): &(String, String)| (p.as_str(), c.as_str()))
        .collect();
    let (_tmp, core, mut query) = fixture(&refs);
    let all = core.document_directory(query.clone()).unwrap();
    assert_eq!(all.candidates, 110);
    assert_eq!(all.eligible_files, 109);
    query.kind_filter = Some("design".into());
    query.lifecycle_filter = Some("current".into());
    let design = core.document_directory(query.clone()).unwrap();
    assert_eq!(design.matching_files, 1);
    assert_eq!(design.eligible_files, 109);
    assert_eq!(design.candidates, 110);
    query.parent_path = "doc/old".into();
    let leaf = core.document_directory(query.clone()).unwrap();
    assert_eq!(leaf.entries[0].historical_reference_files, 1);
    assert_eq!(
        leaf.entries[0].historical_reference_paths,
        ["doc/source/prd.md"]
    );
    query.parent_path.clear();
    query.kind_filter = Some("entry".into());
    assert_eq!(
        core.document_directory(query.clone())
            .unwrap()
            .matching_files,
        2
    );
    query.parent_path = "doc".into();
    query.kind_filter = Some("prd".into());
    let prd = core.document_directory(query.clone()).unwrap();
    assert_eq!(prd.matching_files, 106);
    assert!(prd.next_cursor.is_some());
    query.inventory.cursor = prd.next_cursor;
    query.kind_filter = Some("all".into());
    assert!(core.document_directory(query.clone()).is_err());
    query.inventory.cursor = None;
    query.kind_filter = Some("other".into());
    assert!(core.document_directory(query.clone()).is_err());
    query.kind_filter = Some("design".into());
    query.inventory.read_filter = "omitted".into();
    assert_eq!(
        core.document_directory(query.clone())
            .unwrap()
            .matching_files,
        0
    );
    query.inventory.read_filter = "all".into();
    query.inventory.query = "nonmatching".into();
    assert_eq!(core.document_directory(query).unwrap().matching_files, 0);
}

#[test]
fn pairing_gap_filters_use_only_explicit_pairings_and_preserve_coverage() {
    let files: Vec<_> = (0..105)
        .map(|i| {
            (
                format!("doc/{i:03}.prd.md"),
                "# Requirement\n- 配对系统设计：[missing](deleted.design.md)\n".into(),
            )
        })
        .chain([
            ("doc/deleted.design.md".into(), "# Deleted\n".into()),
            (
                "doc/unpaired.prd.md".into(),
                "# Unpaired\n- 上游：[missing](deleted.design.md)\n".into(),
            ),
            (
                "doc/incomplete.prd.md".into(),
                "# Incomplete\n- 配对系统设计：unclear\n".into(),
            ),
        ])
        .collect();
    let refs: Vec<_> = files
        .iter()
        .map(|(p, c): &(String, String)| (p.as_str(), c.as_str()))
        .collect();
    let (tmp, mut core, mut query) = fixture(&refs);
    fs::remove_file(tmp.path().join("repo/doc/deleted.design.md")).unwrap();
    core.refresh().unwrap();
    query.pairing_filter = Some("missing".into());
    query.parent_path = "doc".into();
    let page = core.document_directory(query.clone()).unwrap();
    assert_eq!(page.matching_files, 105);
    assert_eq!(page.candidates, 108);
    assert_eq!(page.eligible_files, 108);
    assert_eq!(page.omitted, 1);
    assert!(page.next_cursor.is_some());
    query.inventory.cursor = page.next_cursor;
    query.pairing_filter = Some("all".into());
    assert!(core.document_directory(query.clone()).is_err());
    query.inventory.cursor = None;
    query.pairing_filter = Some("unpaired".into());
    assert_eq!(
        core.document_directory(query.clone())
            .unwrap()
            .matching_files,
        1
    );
    query.pairing_filter = Some("unparsed".into());
    assert_eq!(
        core.document_directory(query.clone())
            .unwrap()
            .matching_files,
        1
    );
    query.pairing_filter = Some("incomplete".into());
    assert_eq!(
        core.document_directory(query.clone())
            .unwrap()
            .matching_files,
        1
    );
    query.pairing_filter = Some("missing".into());
    query.inventory.query = "unpaired".into();
    assert_eq!(
        core.document_directory(query.clone())
            .unwrap()
            .matching_files,
        0
    );
    query.pairing_filter = Some("invented".into());
    assert!(core.document_directory(query).is_err());
}
