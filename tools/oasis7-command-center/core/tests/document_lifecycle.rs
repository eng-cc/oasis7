use oasis7_command_center_core::{CommandCenter, domain::DocumentSection};
use std::{path::Path, process::Command};
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
fn fixture(content: &str) -> (tempfile::TempDir, CommandCenter) {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(repo.join("doc/old")).unwrap();
    git(&repo, &["init"]);
    git(&repo, &["config", "user.name", "test"]);
    git(&repo, &["config", "user.email", "test@example.org"]);
    std::fs::write(repo.join("doc/old/rules.md"), content).unwrap();
    std::fs::write(
        repo.join("doc/next.md"),
        "# Next\n<a id=\"source-anchor\"></a>\n## Rule\nnext\n",
    )
    .unwrap();
    std::fs::write(repo.join("AGENTS.md"), "# Agent\n").unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "fixture"]);
    let mut core = CommandCenter::open(tmp.path().join("state.sqlite")).unwrap();
    core.open_project(&repo).unwrap();
    (tmp, core)
}
fn section(core: &CommandCenter) -> DocumentSection {
    core.snapshot()
        .sections
        .into_iter()
        .find(|s| s.path == "doc/old/rules.md")
        .unwrap()
}
#[test]
fn explicit_identity_projects_local_targets_and_preserves_historical_metadata() {
    let old = "# Old\n## 文档身份\n- 生命周期：`retired`\n- 后继文档：[New](../next.md#source-anchor)、`AGENTS.md`\n- 替代理由：明确迁移\n## Rules\nold\n";
    let (tmp, mut core) = fixture(old);
    let saved = section(&core);
    let meta = saved.lifecycle.clone().unwrap();
    assert_eq!(meta.status.as_deref(), Some("retired"));
    assert_eq!(
        meta.successor_targets,
        ["doc/next.md#source-anchor", "AGENTS.md"]
    );
    assert_eq!(meta.successor_reason.as_deref(), Some("明确迁移"));
    assert!(meta.issue.is_none());
    assert!(
        core.snapshot()
            .sections
            .iter()
            .filter(|s| s.path == saved.path)
            .all(|s| s.lifecycle.as_ref() == Some(&meta))
    );
    core.record_reading(&saved.id, &saved.revision, 1).unwrap();
    std::fs::write(
        tmp.path().join("repo/doc/old/rules.md"),
        "# Old\n状态：active\n后继文档：`无`\n## Rules\nnew\n",
    )
    .unwrap();
    core.rebuild_cache().unwrap();
    drop(core);
    let mut core = CommandCenter::open(tmp.path().join("state.sqlite")).unwrap();
    core.open_project(tmp.path().join("repo")).unwrap();
    core.rebuild_cache().unwrap();
    assert_eq!(
        core.read_version(&saved.id, &saved.revision)
            .unwrap()
            .unwrap()
            .lifecycle,
        Some(meta)
    );
    assert_eq!(
        section(&core).lifecycle.unwrap().status.as_deref(),
        Some("active")
    );
    let mut legacy = serde_json::to_value(saved).unwrap();
    legacy.as_object_mut().unwrap().remove("lifecycle");
    assert!(
        serde_json::from_value::<DocumentSection>(legacy)
            .unwrap()
            .lifecycle
            .is_none()
    );
}
#[test]
fn quoted_code_nested_and_explicit_examples_do_not_become_declarations() {
    for fake in [
        "> 状态：retired\n> 后继文档：`doc/next.md`",
        "```md\n状态：retired\n后继文档：`doc/next.md`\n```",
        "- 示例\n  - 状态：retired\n  - 后继文档：`doc/next.md`",
        "例如以下声明：\n状态：retired\n后继文档：`doc/next.md`",
        "example:\n状态：retired\n后继文档：`doc/next.md`",
        "### 示例\n状态：retired\n后继文档：`doc/next.md`",
        "## Examples\n状态：retired\n后继文档：`doc/next.md`",
    ] {
        let (_, core) = fixture(&format!("# Source\n{fake}\n"));
        assert!(section(&core).lifecycle.is_none(), "{fake}");
    }
}
#[test]
fn unsafe_missing_conflicting_and_overbound_targets_have_no_partial_projection() {
    for target in [
        "[ok](../next.md), [bad](https://example.org/x.md)",
        "`../../../outside.md`",
        "`doc/missing.md`",
        "`src/main.md`",
        "[encoded](%2e%2e/%2e%2e/%2e%2e/out.md)",
        "not a link",
    ] {
        let (_, core) = fixture(&format!(
            "# Source\n状态：retired\n后继文档：{target}\n## Rules\n"
        ));
        let meta = section(&core).lifecycle.unwrap();
        assert_eq!(meta.status.as_deref(), Some("retired"));
        assert!(meta.successor_targets.is_empty());
        assert!(meta.issue.is_some(), "{target}");
    }
    for rows in [
        "状态：active\n状态：retired\n后继文档：`doc/next.md`",
        "状态：retired\n后继文档：`doc/next.md`\n后继文档：`AGENTS.md`",
        "状态：retired\n替代理由：one\n替代理由：two\n后继文档：`doc/next.md`",
    ] {
        let (_, core) = fixture(&format!("# Source\n{rows}\n## Rules\n"));
        let meta = section(&core).lifecycle.unwrap();
        assert!(meta.issue.is_some());
        assert!(meta.successor_targets.is_empty());
    }
    let targets = (0..9)
        .map(|i| format!("`doc/next.md#id{i}`"))
        .collect::<Vec<_>>()
        .join(",");
    for value in [targets, "x".repeat(5000)] {
        let (_, core) = fixture(&format!("# Source\n后继文档：{value}\n"));
        let meta = section(&core).lifecycle.unwrap();
        assert!(meta.issue.is_some());
        assert!(meta.successor_targets.is_empty());
        assert!(meta.successor_declaration.unwrap().len() <= 4096);
    }
}
#[test]
fn real_product_identity_and_self_declaration_are_source_facts_only() {
    let real = include_str!("../../../../doc/product/world-infrastructure/prd.md");
    let (_, core) = fixture(real);
    let meta = section(&core).lifecycle.unwrap();
    assert_eq!(meta.status.as_deref(), Some("active"));
    assert_eq!(meta.successor_declaration.as_deref(), Some("`无`"));
    assert!(meta.successor_targets.is_empty());
    assert!(meta.issue.is_none());
    let (_, core) =
        fixture("# Source\n状态：retired\n后继文档：[self](rules.md#unknown-fragment)\n## Rules\n");
    let meta = section(&core).lifecycle.unwrap();
    assert_eq!(
        meta.successor_targets,
        ["doc/old/rules.md#unknown-fragment"]
    );
    assert!(meta.issue.is_none());
}

#[test]
fn commonmark_indentation_preserves_top_level_identity_and_example_boundaries() {
    for indent in [" ", "  ", "   "] {
        let (_, core) = fixture(&format!(
            "# Source\n## 文档身份\n{indent}- 状态：retired\n{indent}- 后继文档：`doc/next.md`\n## Rules\n"
        ));
        let metadata = section(&core).lifecycle.unwrap();
        assert_eq!(metadata.status.as_deref(), Some("retired"));
        assert_eq!(metadata.successor_targets, ["doc/next.md"]);
        let (_, core) = fixture(&format!(
            "# Source\n## 文档身份\n{indent}### 示例\n状态：retired\n后继文档：`doc/next.md`\n"
        ));
        assert!(section(&core).lifecycle.is_none());
    }
    for nested in [
        "- Container\n  - 状态：retired\n  - 后继文档：`doc/next.md`",
        " - Container\n   - 状态：retired\n   - 后继文档：`doc/next.md`",
        "- Container\n  状态：retired\n  后继文档：`doc/next.md`",
    ] {
        let (_, core) = fixture(&format!("# Source\n## 文档身份\n{nested}\n## Rules\n"));
        assert!(section(&core).lifecycle.is_none(), "{nested}");
    }
}

#[test]
fn lifecycle_retains_raw_declaration_and_normalizes_only_the_first_token() {
    for (raw, category) in [
        ("`draft`；目标方案", "draft"),
        ("ACTIVE（owner：runtime）", "active"),
        ("retired(历史)", "retired"),
        ("superseded, replacement pending", "superseded"),
        ("proposed; awaiting decision", "proposed"),
        ("target-contract", "other"),
        ("`draft extra`；说明", "other"),
        ("unknown", "other"),
        ("`draft`` note`", "other"),
        ("``draft` note``", "other"),
        ("``draft``；说明", "draft"),
        ("`ACTIVE`（owner）", "active"),
        ("draft，说明", "draft"),
    ] {
        let (_, core) = fixture(&format!("# Source\n状态：{raw}\n## Rules\n"));
        let meta = section(&core).lifecycle.unwrap();
        let json = serde_json::to_value(meta).unwrap();
        assert_eq!(json["rawStatus"], raw);
        assert_eq!(json["category"], category, "{raw}");
    }
}
#[test]
fn invalid_lifecycle_is_diagnosed_and_old_cache_remains_readable() {
    for raw in ["", "`draft", "``", "`draft``", "；说明"] {
        let (_, core) = fixture(&format!("# Source\n状态：{raw}\n## Rules\n"));
        let meta = section(&core).lifecycle.unwrap();
        let json = serde_json::to_value(&meta).unwrap();
        assert_eq!(json["rawStatus"], raw);
        assert!(json["category"].is_null());
        assert!(meta.issue.is_some(), "{raw}");
    }
    let (_, core) = fixture("# Source\n后继文档：`无`\n");
    assert_eq!(
        section(&core).lifecycle.unwrap().category.as_deref(),
        Some("unknown")
    );
    let (_, core) = fixture("# Source\n状态：active\n状态：retired\n");
    let meta = section(&core).lifecycle.unwrap();
    let mut json = serde_json::to_value(meta).unwrap();
    assert!(json["category"].is_null());
    assert_eq!(json["rawStatus"], "active | retired");
    json.as_object_mut().unwrap().remove("rawStatus");
    json.as_object_mut().unwrap().remove("category");
    let legacy: oasis7_command_center_core::domain::DocumentLifecycle =
        serde_json::from_value(json.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&legacy).unwrap()["status"],
        json["status"]
    );
}

#[test]
fn successor_fragments_decode_once_and_reject_malformed_or_unsafe_values() {
    for fragment in ["%73ource-anchor", "source%2Danchor"] {
        let (_, core) = fixture(&format!(
            "# Source\n后继文档：[Next](../next.md#{fragment})\n"
        ));
        let meta = section(&core).lifecycle.unwrap();
        assert_eq!(meta.successor_targets, ["doc/next.md#source-anchor"]);
        assert!(
            core.snapshot()
                .sections
                .iter()
                .any(|s| s.id == meta.successor_targets[0])
        );
        assert!(meta.issue.is_none());
    }
    for fragment in ["%", "%GG", "%FF", "source%0Aanchor", "source%20anchor"] {
        let (_, core) = fixture(&format!(
            "# Source\n后继文档：[Next](../next.md#{fragment})\n"
        ));
        let meta = section(&core).lifecycle.unwrap();
        assert!(meta.successor_targets.is_empty(), "{fragment}");
        assert!(meta.issue.is_some(), "{fragment}");
    }
    let (_, core) = fixture("# Source\n后继文档：[Next](../next.md#%2573ource-anchor)\n");
    assert_eq!(
        section(&core).lifecycle.unwrap().successor_targets,
        ["doc/next.md#%73ource-anchor"]
    );
}
