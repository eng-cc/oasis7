use oasis7_command_center_core::{
    CommandCenter, document_inventory::DocumentInventoryQuery,
    document_semantic_tree::DocumentSemanticTreeQuery,
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
fn fixture(
    files: &[(&str, &str)],
) -> (tempfile::TempDir, CommandCenter, DocumentSemanticTreeQuery) {
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
    let query = DocumentSemanticTreeQuery {
        parent_occurrence: String::new(),
        view: "professional".into(),
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
fn set_parent(core: &CommandCenter, query: &mut DocumentSemanticTreeQuery, route: &str) {
    let mut root = query.clone();
    root.parent_occurrence.clear();
    root.inventory.cursor = None;
    let snapshot = core.document_semantic_tree(root).unwrap().snapshot_id;
    query.parent_occurrence = format!("{snapshot}-{}:{route}", query.view);
}
#[test]
fn professional_explicit_structure_diagnostics_cycles_and_search_ancestors() {
    let (_tmp, core, mut query) = fixture(&[
        (
            "doc/README.md",
            "# Docs\n## 工程模块入口矩阵\n| 模块 | PRD 主文档 | 设计主文档 | 描述 |\n| --- | --- | --- | --- |\n| engineering | `doc/engineering/prd.md` | `doc/engineering/design.md` | module |\n",
        ),
        ("doc/engineering/prd.md", "# Engineering\n"),
        (
            "doc/engineering/design.md",
            "# Design\n## 阅读顺序\n`doc/engineering/README.md`\n## 集成点\n[not nav](../product/world-infrastructure/prd.md)\n",
        ),
        (
            "doc/engineering/README.md",
            "# Landing\n## 按主题进入\n- [index](prd.index.md)\n- [bad fragment](prd.index.md#absent)\n- [cross](../product/world-infrastructure/prd.md)\n",
        ),
        (
            "doc/engineering/prd.index.md",
            "# Index\n- [unique topic](topic.design.md)\n- [cycle](README.md)\n- [missing](absent.design.md)\n",
        ),
        (
            "doc/engineering/topic.design.md",
            "# Needle\n- 文档 ID：NEEDLE-ID\n",
        ),
        ("doc/engineering/unassigned.design.md", "# Unassigned\n"),
        ("doc/product/world-infrastructure/prd.md", "# Product\n"),
    ]);
    let root = core.document_semantic_tree(query.clone()).unwrap();
    assert_eq!(root.entries.len(), 2);
    assert_eq!(root.entries[0].label, "engineering");
    assert!(root.entries[1].label.contains("未归属"));
    assert_eq!(root.eligible_files, 6);
    set_parent(&core, &mut query, "0/1/0");
    let landing = core.document_semantic_tree(query.clone()).unwrap();
    assert_eq!(landing.entries.len(), 3);
    assert_eq!(landing.entries[0].source_line, Some(3));
    assert_eq!(
        landing.entries[1].relationship.as_ref().unwrap().resolution,
        "missing_fragment"
    );
    assert!(!landing.entries[1].has_children);
    assert_eq!(landing.entries[2].boundary.as_deref(), Some("cross_domain"));
    assert!(!landing.entries[2].has_children);
    set_parent(&core, &mut query, "0/1/0/0");
    let index = core.document_semantic_tree(query.clone()).unwrap();
    assert!(index.entries[1].cycle);
    assert!(!index.entries[1].has_children);
    assert_eq!(
        index.entries[2].relationship.as_ref().unwrap().resolution,
        "outside_scope"
    );
    set_parent(&core, &mut query, "0/1/0/0/1");
    assert!(core.document_semantic_tree(query.clone()).is_err());
    query.parent_occurrence.clear();
    query.inventory.query = "needle-id".into();
    let filtered = core.document_semantic_tree(query.clone()).unwrap();
    assert_eq!(filtered.matching_files, 1);
    assert_eq!(filtered.entries[0].label, "engineering");
    assert_eq!(filtered.entries[0].matching_files, 1);
    set_parent(&core, &mut query, "0/1/0");
    let filtered = core.document_semantic_tree(query).unwrap();
    assert!(filtered.entries.iter().any(|entry| entry.path.as_deref()
        == Some("doc/engineering/prd.index.md")
        && entry.has_children));
}
#[test]
fn products_use_exact_four_modules_active_topics_and_distinct_parent_occurrences() {
    let (_tmp, core, mut query) = fixture(&[
        (
            "doc/product/README.md",
            "# Product\n## 四大产品模块\n| 产品模块 | 唯一入口 | 职责 |\n| --- | --- | --- |\n| World | [world](world-rules-core-gameplay/prd.md) | one |\n| Infra | [infra](world-infrastructure/prd.md) | two |\n| Agent | [agent](agents-world-simulation/prd.md) | three |\n| Entry | [entry](player-entry-distribution/prd.md) | four |\n\n[ordinary](world-rules-core-gameplay/orphan.prd.md)\n",
        ),
        (
            "doc/product/world-rules-core-gameplay/prd.md",
            "# World\n### 活跃产品专题\n- [one](topic.prd.md)\n- [same second declaration](topic.prd.md)\n- [cross](../world-infrastructure/topic.prd.md)\n### 非权威迁移索引\n- [orphan](orphan.prd.md)\n",
        ),
        (
            "doc/product/world-infrastructure/prd.md",
            "# Infra\n### 活跃产品专题\n- [own](topic.prd.md)\n",
        ),
        ("doc/product/agents-world-simulation/prd.md", "# Agent\n"),
        ("doc/product/player-entry-distribution/prd.md", "# Entry\n"),
        (
            "doc/product/world-rules-core-gameplay/topic.prd.md",
            "# Topic\n",
        ),
        (
            "doc/product/world-infrastructure/topic.prd.md",
            "# Infra topic\n",
        ),
        (
            "doc/product/world-rules-core-gameplay/orphan.prd.md",
            "# Orphan\n",
        ),
    ]);
    query.view = "product".into();
    let root = core.document_semantic_tree(query.clone()).unwrap();
    assert_eq!(root.entries.len(), 5);
    assert!(root.issues.is_empty());
    set_parent(&core, &mut query, "0");
    let world = core.document_semantic_tree(query).unwrap();
    assert_eq!(world.entries.len(), 3);
    assert_eq!(world.entries[0].path, world.entries[1].path);
    assert_ne!(
        world.entries[0].occurrence_id,
        world.entries[1].occurrence_id
    );
    assert_eq!(world.entries[2].boundary.as_deref(), Some("cross_domain"));
    assert_eq!(world.entries[0].source_line, Some(3));
}
#[test]
fn semantic_pagination_is_snapshot_view_parent_and_filter_bound() {
    let mut files = vec![("doc/README.md".into(), "# Docs\n".into())];
    for i in 0..105 {
        files.push((
            format!("doc/engineering/{i:03}.design.md"),
            format!("# Design {i}\n"),
        ));
    }
    let refs: Vec<_> = files
        .iter()
        .map(|(p, c)| (p.as_str(), c.as_str()))
        .collect();
    let (tmp, mut core, mut query) = fixture(&refs);
    set_parent(&core, &mut query, "0");
    let first = core.document_semantic_tree(query.clone()).unwrap();
    assert_eq!(first.total_children, 105);
    assert_eq!(first.entries.len(), 100);
    query.inventory.cursor = first.next_cursor;
    let second = core.document_semantic_tree(query.clone()).unwrap();
    assert_eq!(second.entries.len(), 5);
    let mut wrong = query.clone();
    wrong.view = "product".into();
    assert!(core.document_semantic_tree(wrong).is_err());
    let mut wrong = query.clone();
    wrong.parent_occurrence.clear();
    assert!(core.document_semantic_tree(wrong).is_err());
    let mut wrong = query.clone();
    wrong.kind_filter = Some("prd".into());
    assert!(core.document_semantic_tree(wrong).is_err());
    fs::write(tmp.path().join("repo/doc/README.md"), "# Changed\n").unwrap();
    core.refresh().unwrap();
    assert!(core.document_semantic_tree(query).is_err());
}
#[test]
fn history_count_is_current_only_and_preserves_existing_explicit_references() {
    let (_tmp, core, mut query) = fixture(&[
        ("doc/README.md", "# Docs\n"),
        (
            "doc/engineering/README.md",
            "# Source\n- 配对系统设计：[old](old.design.md)\n## 入口\n- [same source navigation](old.design.md)\n",
        ),
        (
            "doc/engineering/old.design.md",
            "# Old\n- 生命周期：retired\n",
        ),
        (
            "doc/engineering/hidden.design.md",
            "# Hidden\n- 生命周期：superseded\n",
        ),
        (
            "doc/engineering/draft.design.md",
            "# Draft\n- 生命周期：draft\n",
        ),
    ]);
    query.lifecycle_filter = Some("current".into());
    let current = core.document_semantic_tree(query.clone()).unwrap();
    assert_eq!(current.hidden_history_files, 1);
    assert_eq!(current.matching_files, 3);
    set_parent(&core, &mut query, "0");
    let leaves = core.document_semantic_tree(query.clone()).unwrap();
    let historical = leaves
        .entries
        .iter()
        .find(|entry| entry.path.as_deref() == Some("doc/engineering/old.design.md"))
        .unwrap();
    assert_eq!(historical.historical_reference_files, 1);
    assert_eq!(
        historical.historical_reference_paths,
        ["doc/engineering/README.md"]
    );
    query.lifecycle_filter = Some("all".into());
    let all = core.document_semantic_tree(query.clone()).unwrap();
    assert!(
        all.entries
            .iter()
            .all(|entry| entry.historical_reference_files == 0
                && entry.historical_reference_paths.is_empty())
    );
    query.parent_occurrence.clear();
    query.lifecycle_filter = Some("draft".into());
    let draft = core.document_semantic_tree(query).unwrap();
    assert_eq!(draft.hidden_history_files, 0);
    assert_eq!(draft.matching_files, 1);
}
#[test]
fn navigation_target_diagnostics_use_same_snapshot_candidate_and_source_lines() {
    let (tmp, mut core, mut query) = fixture(&[
        (
            "doc/README.md",
            "## 工程模块入口矩阵\n| 模块 | PRD 主文档 | 设计主文档 |\n| --- | --- | --- |\n| engineering | `doc/engineering/prd.md` | `doc/engineering/design.md` |\n",
        ),
        ("doc/engineering/prd.md", "# PRD\n"),
        (
            "doc/engineering/design.md",
            "# Design\n## 专题导航\n- [deleted](deleted.design.md)\n- [unread](unread.design.md)\n- [ambiguous](anchors.design.md#dup)\n- [unsafe](../../../../outside.md)\n",
        ),
        ("doc/engineering/deleted.design.md", "# Deleted\n"),
        ("doc/engineering/unread.design.md", "# Unread\n"),
        (
            "doc/engineering/anchors.design.md",
            "# Anchors\n<a id=\"dup\"></a>\n## One\n<a id=\"dup\"></a>\n## Two\n",
        ),
    ]);
    fs::remove_file(tmp.path().join("repo/doc/engineering/deleted.design.md")).unwrap();
    fs::write(
        tmp.path().join("repo/doc/engineering/unread.design.md"),
        [255u8, 254],
    )
    .unwrap();
    core.refresh().unwrap();
    set_parent(&core, &mut query, "0/1");
    let page = core.document_semantic_tree(query).unwrap();
    assert_eq!(
        page.entries
            .iter()
            .map(|e| e.relationship.as_ref().unwrap().resolution.as_str())
            .collect::<Vec<_>>(),
        ["missing", "unread", "ambiguous", "outside_scope"]
    );
    assert_eq!(
        page.entries
            .iter()
            .map(|e| e.source_line.unwrap())
            .collect::<Vec<_>>(),
        [3, 4, 5, 6]
    );
    assert!(page.entries.iter().all(|entry| !entry.has_children));
    assert_eq!(page.omitted, 2);
}
#[test]
fn product_identity_pairing_resolves_without_navigation_and_legacy_cache_remains_readable() {
    let (tmp, core, query) = fixture(&[
        (
            "doc/product/agents-world-simulation/external-agent-runtime-play.prd.md",
            "# 外部 Agent Runtime 接入与持续游玩\n\n## 文档身份\n\n- 产品模块：智能体、世界模拟与交互\n- 生命周期：active\n- 配对产品 design：[external-agent-runtime-play.design.md](external-agent-runtime-play.design.md)\n\n## 产品目标\n- 配对产品 design：[not declaration](other.design.md)\n",
        ),
        (
            "doc/product/agents-world-simulation/external-agent-runtime-play.design.md",
            "# External Runtime Design\n",
        ),
    ]);
    let page = core.document_inventory(query.inventory.clone()).unwrap();
    let prd = page.files.iter().find(|file| file.kind == "prd").unwrap();
    let pairings = prd.pairings.as_ref().unwrap();
    assert_eq!(pairings.targets.len(), 1);
    assert_eq!(pairings.targets[0].declaration_line, 7);
    assert_eq!(pairings.targets[0].resolution, "resolved");
    assert!(pairings.targets[0].target_section.is_some());
    assert_eq!(
        prd.navigation.as_ref().unwrap().relationships.targets.len(),
        0
    );
    let section_count = core.snapshot().sections.len();
    drop(core);
    let db = rusqlite::Connection::open(tmp.path().join("db")).unwrap();
    let key = format!("document_inventory_v1:{}", query.inventory.project_id);
    let raw: String = db
        .query_row("SELECT value FROM cache WHERE key=?1", [&key], |row| {
            row.get(0)
        })
        .unwrap();
    let mut old: serde_json::Value = serde_json::from_str(&raw).unwrap();
    for file in old["files"].as_array_mut().unwrap() {
        file.as_object_mut().unwrap().remove("navigation");
        if file["kind"] == "prd" {
            file["pairings"] = serde_json::json!({"targets":[],"issue":null});
        }
    }
    let legacy_raw = old.to_string();
    db.execute(
        "UPDATE cache SET value=?1 WHERE key=?2",
        [&legacy_raw, &key],
    )
    .unwrap();
    let mut reopened = CommandCenter::open(tmp.path().join("db")).unwrap();
    assert_eq!(reopened.snapshot().sections.len(), section_count);
    let page = reopened
        .document_inventory(query.inventory.clone())
        .unwrap();
    assert!(
        page.files
            .iter()
            .find(|file| file.kind == "prd")
            .unwrap()
            .pairings
            .is_none()
    );
    let unchanged: String = db
        .query_row("SELECT value FROM cache WHERE key=?1", [&key], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(unchanged, legacy_raw);
    reopened.refresh().unwrap();
    let fresh = reopened.document_inventory(query.inventory).unwrap();
    assert_eq!(
        fresh
            .files
            .iter()
            .find(|file| file.kind == "prd")
            .unwrap()
            .pairings
            .as_ref()
            .unwrap()
            .targets[0]
            .resolution,
        "resolved"
    );
}
