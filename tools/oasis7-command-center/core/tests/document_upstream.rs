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
    fs::create_dir_all(root.join("doc")).unwrap();
    git(&root, &["init"]);
    git(&root, &["config", "user.name", "test"]);
    git(&root, &["config", "user.email", "test@example.org"]);
    fs::write(
        root.join("doc/prd.md"),
        "# Requirements\n\n<a id='REQ-1'></a>\n\n## Requirement\n",
    )
    .unwrap();
    fs::write(root.join("doc/parent.design.md"), "# Parent\n").unwrap();
    fs::write(root.join("doc/missing.prd.md"), "# Deleted\n").unwrap();
    fs::write(root.join("doc/unread.prd.md"), [0xff]).unwrap();
    fs::write(root.join("doc/topic.design.md"), r#"# Topic

- 上游：[requirement](prd.md#REQ-1)、[parent](parent.design.md)、[deleted](missing.prd.md)、[unread](unread.prd.md)、[outside](../.env)、[absent](prd.md#absent)
- 配对系统设计：[pair](parent.design.md)

> - 上游：[quoted](quoted.prd.md)

```md
- 上游：[example](example.prd.md)
```

- Example:
  - 上游：[nested](nested.prd.md)

## Body
- 上游：[body](body.prd.md)
"#).unwrap();
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "fixture"]);
    fs::remove_file(root.join("doc/missing.prd.md")).unwrap();
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
fn explicit_upstream_links_preserve_source_order_kind_and_exact_chapters() {
    let (_tmp, core, query) = fixture();
    let page = core.document_inventory(query).unwrap();
    let topic = page
        .files
        .iter()
        .find(|f| f.path == "doc/topic.design.md")
        .unwrap();
    let upstream = topic.upstream.as_ref().unwrap();
    assert!(upstream.issue.is_none());
    assert_eq!(upstream.targets.len(), 6);
    assert!(upstream.targets.iter().all(|e| e.declaration_line == 3));
    assert_eq!(
        upstream
            .targets
            .iter()
            .map(|e| e.resolution.as_str())
            .collect::<Vec<_>>(),
        [
            "resolved",
            "resolved",
            "missing",
            "unread",
            "outside_scope",
            "missing_fragment"
        ]
    );
    assert_eq!(upstream.targets[0].target_kind.as_deref(), Some("prd"));
    assert_eq!(upstream.targets[1].target_kind.as_deref(), Some("design"));
    let snapshot = core.snapshot();
    let target = snapshot
        .sections
        .iter()
        .find(|s| Some(&s.id) == upstream.targets[0].target_section.as_ref())
        .unwrap();
    assert_eq!(target.title, "Requirement");
    assert_eq!(
        Some(&target.revision),
        upstream.targets[0].target_revision.as_ref()
    );
    assert_eq!(topic.pairings.as_ref().unwrap().targets.len(), 1);
    assert!(
        page.files
            .iter()
            .find(|f| f.path == "doc/prd.md")
            .unwrap()
            .upstream
            .is_none()
    );
    assert!(
        page.files
            .iter()
            .find(|f| f.path == "doc/parent.design.md")
            .unwrap()
            .upstream
            .as_ref()
            .unwrap()
            .targets
            .is_empty()
    );
}

#[test]
fn upstream_bounds_and_continuations_are_visible_instead_of_complete_absence() {
    let (tmp, mut core, query) = fixture();
    let path = tmp.path().join("repo/doc/topic.design.md");
    let targets = (0..33)
        .map(|i| format!("[r{i}](prd.md)"))
        .collect::<Vec<_>>()
        .join(" ");
    fs::write(&path, format!("# Topic\n\n- 上游：{targets}\n")).unwrap();
    core.refresh().unwrap();
    let page = core.document_inventory(query.clone()).unwrap();
    let upstream = page
        .files
        .iter()
        .find(|f| f.path == "doc/topic.design.md")
        .unwrap()
        .upstream
        .as_ref()
        .unwrap();
    assert_eq!(upstream.targets.len(), 32);
    assert!(upstream.issue.as_deref().unwrap().contains("incomplete"));
    fs::write(
        &path,
        "# Topic\n\n- 上游: [requirement：详情](prd.md#REQ-1)\n  [continued](parent.design.md)\n",
    )
    .unwrap();
    core.refresh().unwrap();
    let page = core.document_inventory(query).unwrap();
    let upstream = page
        .files
        .iter()
        .find(|f| f.path == "doc/topic.design.md")
        .unwrap()
        .upstream
        .as_ref()
        .unwrap();
    assert_eq!(upstream.targets.len(), 1);
    assert!(upstream.issue.as_deref().unwrap().contains("multiline"));
}

#[test]
fn legacy_inventory_without_upstream_keeps_original_pairings_and_personal_snapshot() {
    let (tmp, core, query) = fixture();
    let before = core.snapshot();
    drop(core);
    let db = rusqlite::Connection::open(tmp.path().join("db")).unwrap();
    let key = format!("document_inventory_v1:{}", query.project_id);
    let raw: String = db
        .query_row("SELECT value FROM cache WHERE key=?1", [&key], |r| r.get(0))
        .unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    for file in value["files"].as_array_mut().unwrap() {
        file.as_object_mut().unwrap().remove("upstream");
    }
    db.execute(
        "UPDATE cache SET value=?1 WHERE key=?2",
        [value.to_string(), key],
    )
    .unwrap();
    let core = CommandCenter::open(tmp.path().join("db")).unwrap();
    let page = core.document_inventory(query).unwrap();
    assert!(page.files.iter().all(|f| f.upstream.is_none()));
    assert_eq!(
        page.files
            .iter()
            .find(|f| f.path == "doc/topic.design.md")
            .unwrap()
            .pairings
            .as_ref()
            .unwrap()
            .targets
            .len(),
        1
    );
    assert_eq!(
        serde_json::to_value(before).unwrap(),
        serde_json::to_value(core.snapshot()).unwrap()
    );
}

#[test]
fn quoted_headings_do_not_end_top_level_upstream_metadata() {
    let (tmp, mut core, query) = fixture();
    fs::write(
        tmp.path().join("repo/doc/topic.design.md"),
        "# Topic\n\n> ## Quoted example\n\n- 上游：[requirement](prd.md#REQ-1)\n",
    )
    .unwrap();
    core.refresh().unwrap();
    let page = core.document_inventory(query).unwrap();
    let upstream = page
        .files
        .iter()
        .find(|f| f.path == "doc/topic.design.md")
        .unwrap()
        .upstream
        .as_ref()
        .unwrap();
    assert_eq!(upstream.targets.len(), 1);
    assert_eq!(upstream.targets[0].declaration_line, 5);
}
