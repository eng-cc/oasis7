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
fn fixture(source: &str) -> (tempfile::TempDir, CommandCenter, DocumentInventoryQuery) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("repo");
    fs::create_dir_all(root.join("doc")).unwrap();
    git(&root, &["init"]);
    git(&root, &["config", "user.name", "test"]);
    git(&root, &["config", "user.email", "test@example.org"]);
    fs::write(
        root.join("doc/prd.md"),
        "# Requirements\n\n<a id='REQ-1'></a>\n\n## Requirement\n",
    )
    .unwrap();
    fs::write(root.join("doc/topic.design.md"), source).unwrap();
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "fixture"]);
    let mut core = CommandCenter::open(tmp.path().join("db")).unwrap();
    let snapshot = core.open_project(&root).unwrap();
    let query = DocumentInventoryQuery {
        schema_version: 1,
        project_id: snapshot.project.unwrap().id,
        snapshot_id: None,
        query: "topic.design.md".into(),
        read_filter: "all".into(),
        cursor: None,
    };
    (tmp, core, query)
}
const SOURCE: &str = "# Topic\n- 上游：[header](prd.md#REQ-1)\n\n### 2.1 需求承接与分配表\n\n| 上游条款 | 独立义务与适用条件 | 本设计条款 |\n| --- | --- | --- |\n| [req](prd.md#REQ-1) | obligation | [design](#DES-1) |\n\n<a id='DES-1'></a>\n\n## Design clause\n";
#[test]
fn allocation_rows_keep_both_sides_and_exact_same_candidate_targets() {
    let (_tmp, core, query) = fixture(SOURCE);
    let page = core.document_inventory(query).unwrap();
    let file = &page.files[0];
    let value = serde_json::to_value(file).unwrap();
    assert_eq!(value["allocations"]["rows"].as_array().unwrap().len(), 1);
    assert_eq!(value["allocations"]["rows"][0]["declarationLine"], 8);
}

#[test]
fn allocation_resolution_does_not_follow_targets_or_promote_other_columns() {
    let source = SOURCE
        .replace(
            "[req](prd.md#REQ-1)",
            "[req][r] [missing](prd.md#absent) [unsafe](../.env)",
        )
        .replace("obligation", "[not an edge](outside.md)")
        + "\n[r]: prd.md#REQ-1\n";
    let (_tmp, core, query) = fixture(&source);
    let page = core.document_inventory(query).unwrap();
    let file = &page.files[0];
    let table = file.allocations.as_ref().unwrap();
    assert!(table.issue.is_none());
    let row = &table.rows[0];
    assert!(row.raw_sample.contains("not an edge"));
    assert_eq!(row.upstream.targets.len(), 3);
    assert_eq!(row.design.targets.len(), 1);
    assert_eq!(
        row.upstream
            .targets
            .iter()
            .map(|e| e.resolution.as_str())
            .collect::<Vec<_>>(),
        ["resolved", "missing_fragment", "outside_scope"]
    );
    assert_eq!(row.upstream.targets[0].target_kind.as_deref(), Some("prd"));
    assert_eq!(row.design.targets[0].target_kind.as_deref(), Some("design"));
    for edge in [&row.upstream.targets[0], &row.design.targets[0]] {
        let snapshot = core.snapshot();
        let section = snapshot
            .sections
            .iter()
            .find(|s| Some(&s.id) == edge.target_section.as_ref())
            .unwrap();
        assert_eq!(Some(&section.revision), edge.target_revision.as_ref());
    }
    assert_eq!(file.upstream.as_ref().unwrap().targets.len(), 1);
    assert!(file.pairings.as_ref().unwrap().targets.is_empty());
}

#[test]
fn allocation_sections_are_explicit_and_ignore_examples_and_verification_tables() {
    let table = "| 上游条款 | 本设计条款 |\n| --- | --- |\n| [req](prd.md) | [design](#DES-1) |\n";
    for source in [
        format!("# Other\n{table}"),
        format!("# Other\n## 验证映射表\n{table}"),
        format!("# Other\n## ...需求承接与分配表\n{table}"),
        format!("# Other\n```md\n## 需求承接与分配表\n{table}```\n"),
        format!(
            "# Other\n> ## 需求承接与分配表\n{}",
            table
                .lines()
                .map(|l| format!("> {l}\n"))
                .collect::<String>()
        ),
    ] {
        let (_tmp, core, query) = fixture(&source);
        let value = core.document_inventory(query).unwrap();
        let allocations = value.files[0].allocations.as_ref().unwrap();
        assert!(allocations.rows.is_empty(), "{source}");
        assert!(allocations.issue.is_none());
    }
    let source = format!("# Title\n## 需求承接与分配表\n{table}\n## Other\n{table}");
    let (_tmp, core, query) = fixture(&source);
    assert_eq!(
        core.document_inventory(query).unwrap().files[0]
            .allocations
            .as_ref()
            .unwrap()
            .rows
            .len(),
        1
    );
}

#[test]
fn allocation_limits_and_invalid_headers_report_incomplete_projection() {
    let header = "# Title\n## 需求承接与分配表\n| 上游条款 | 本设计条款 |\n| --- | --- |\n";
    for (source, issue, rows) in [
        (format!("{header}{}", "| [r](prd.md) | [d](#DES-1) |\n".repeat(33)),"allocation row limit 32 exceeded",32),
        (format!("{header}| {} | [d](#DES-1) |\n", "[r](prd.md) ".repeat(65)),"allocation link limit 64 exceeded",1),
        (format!("{header}| {} | [d](#DES-1) |\n", "界".repeat(2000)),"allocation row exceeds 4096 bytes",1),
        ("# Title\n## 需求承接与分配表\n| 上游条款 | 上游条款 | 本设计条款 |\n| --- | --- | --- |\n| a | b | c |\n".into(),"allocation table requires unique upstream and design columns",0),
        ("# Title\n## 需求承接与分配表\nNo table\n".into(),"allocation section has no table",0),
    ] {
        let (_tmp,core,query) = fixture(&source);
        let value = core.document_inventory(query).unwrap();
        let table = value.files[0].allocations.as_ref().unwrap();
        assert_eq!(table.issue.as_deref(),Some(issue));
        assert_eq!(table.rows.len(),rows);
        assert!(table.rows.iter().all(|r| r.raw_sample.chars().count()<=512));
    }
    let (_tmp, core, query) = fixture(&format!("{header}| REQ-plain | 无 |\n"));
    let value = core.document_inventory(query).unwrap();
    let row = &value.files[0].allocations.as_ref().unwrap().rows[0];
    assert!(row.upstream.issue.is_some());
    assert!(row.design.issue.is_none());
}

#[test]
fn legacy_allocation_cache_and_snapshot_survive_reopening() {
    let (tmp, core, query) = fixture(SOURCE);
    let before = core.snapshot();
    drop(core);
    let db = rusqlite::Connection::open(tmp.path().join("db")).unwrap();
    let key = format!("document_inventory_v1:{}", query.project_id);
    let raw: String = db
        .query_row("SELECT value FROM cache WHERE key=?1", [&key], |r| r.get(0))
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    for file in value["files"].as_array_mut().unwrap() {
        file.as_object_mut().unwrap().remove("allocations");
    }
    db.execute(
        "UPDATE cache SET value=?1 WHERE key=?2",
        [value.to_string(), key],
    )
    .unwrap();
    let core = CommandCenter::open(tmp.path().join("db")).unwrap();
    let page = core.document_inventory(query).unwrap();
    assert!(page.files[0].allocations.is_none());
    assert_eq!(page.files[0].upstream.as_ref().unwrap().targets.len(), 1);
    assert_eq!(
        serde_json::to_value(before).unwrap(),
        serde_json::to_value(core.snapshot()).unwrap()
    );
}
