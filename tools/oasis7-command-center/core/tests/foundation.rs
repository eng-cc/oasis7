use oasis7_command_center_core::{CommandCenter, domain::DecisionDraft};
use std::process::Command;
fn git(root: &std::path::Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .unwrap()
            .success()
    );
}
#[test]
fn chinese_dirty_index_survives_restart_and_cache_rebuild() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo with spaces");
    std::fs::create_dir(&repo).unwrap();
    git(&repo, &["init"]);
    git(&repo, &["config", "user.name", "test"]);
    git(&repo, &["config", "user.email", "test@example.org"]);
    std::fs::create_dir(repo.join("doc")).unwrap();
    std::fs::write(
        repo.join("doc/prd.md"),
        "# 当前阶段交付目标与全局 P0\n规范 合入\n",
    )
    .unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "fixture"]);
    let db = tmp.path().join("state.sqlite");
    let mut core = CommandCenter::open(&db).unwrap();
    let draft = DecisionDraft {
        id: None,
        content: "决定".into(),
        reason: "理由".into(),
        scope: "工程".into(),
        related_section: None,
        confirmed: true,
    };
    assert!(core.save_decision(draft.clone()).is_err());
    core.open_project(&repo).unwrap();
    for invalid in [
        DecisionDraft {
            reason: " ".into(),
            ..draft.clone()
        },
        DecisionDraft {
            scope: " ".into(),
            ..draft.clone()
        },
        DecisionDraft {
            reason: "x".repeat(32769),
            ..draft.clone()
        },
        DecisionDraft {
            scope: "x".repeat(4097),
            ..draft.clone()
        },
        DecisionDraft {
            id: Some("foreign-decision".into()),
            ..draft.clone()
        },
    ] {
        assert!(core.save_decision(invalid).is_err());
    }
    assert!(core.snapshot().decisions.is_empty());
    let section = core.search("规范", 20).unwrap().pop().unwrap();
    assert!(!section.dirty);
    core.record_reading(&section.id, &section.revision, 2)
        .unwrap();
    core.associate_work(
        "https://github.com/eng-cc/oasis7/pull/1",
        vec![section.id.clone()],
    )
    .unwrap();
    let historical_id = section.id.clone();
    let historical_revision = section.revision.clone();
    let saved = core
        .save_decision(DecisionDraft {
            id: None,
            content: "明确决定".into(),
            reason: "理由".into(),
            scope: "工程".into(),
            related_section: Some(section.id),
            confirmed: true,
        })
        .unwrap();
    // Persist a known historical timestamp, then verify an explicit reconfirmation renews it.
    let key = format!("{}:decisions", core.snapshot().project.unwrap().id);
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute(
        "UPDATE personal SET value=json_set(value,'$[0].confirmedAt','1') WHERE key=?1",
        [key],
    )
    .unwrap();
    drop(core);
    let mut core = CommandCenter::open(&db).unwrap();
    let reconfirmed = core
        .save_decision(DecisionDraft {
            id: Some(saved.id),
            content: "明确决定".into(),
            reason: "更新理由".into(),
            scope: "工程".into(),
            related_section: None,
            confirmed: true,
        })
        .unwrap();
    assert_ne!(reconfirmed.confirmed_at.as_deref(), Some("1"));
    assert_eq!(reconfirmed.reason, "更新理由");
    std::fs::write(
        repo.join("doc/prd.md"),
        "# 当前阶段交付目标与全局 P0\n规范 合入 更改\n",
    )
    .unwrap();
    assert!(core.refresh().unwrap().sections[0].dirty);
    drop(core);
    let mut core = CommandCenter::open(&db).unwrap();
    let snap = core.rebuild_cache().unwrap();
    assert_eq!(snap.decisions.len(), 1);
    assert_eq!(snap.associations.len(), 1);
    assert_eq!(
        core.read_version(&historical_id, &historical_revision)
            .unwrap()
            .unwrap()
            .content,
        "# 当前阶段交付目标与全局 P0\n规范 合入\n"
    );
    assert_eq!(core.search("交付目标", 20).unwrap().len(), 1);
    assert!(core.search("%", 20).unwrap().is_empty());
    assert_eq!(snap.reading_positions.len(), 1);
    assert_eq!(core.search("合入", 20).unwrap().len(), 1);
    assert!(core.document_path("../../etc/passwd").is_err());
    let other = tmp.path().join("other");
    std::fs::create_dir(&other).unwrap();
    git(&other, &["init"]);
    git(&other, &["config", "user.name", "test"]);
    git(&other, &["config", "user.email", "test@example.org"]);
    std::fs::create_dir(other.join("doc")).unwrap();
    std::fs::write(other.join("doc/prd.md"), "# Other\nother\n").unwrap();
    git(&other, &["add", "."]);
    git(&other, &["commit", "-m", "other"]);
    let other_snap = core.open_project(&other).unwrap();
    assert!(other_snap.decisions.is_empty());
    assert!(other_snap.reading_positions.is_empty());
    assert!(other_snap.associations.is_empty());
    let faults = rusqlite::Connection::open(&db).unwrap();
    faults.execute_batch("CREATE TRIGGER fail_index BEFORE INSERT ON search BEGIN SELECT RAISE(FAIL,'storage rejected'); END;").unwrap();
    assert!(core.open_project(&repo).is_err());
    assert_eq!(
        core.snapshot().project.unwrap().path,
        other.canonicalize().unwrap().to_string_lossy()
    );
    faults.execute_batch("DROP TRIGGER fail_index;").unwrap();
    let original = core.open_project(&repo).unwrap();
    assert_eq!(original.decisions.len(), 1);
    assert_eq!(original.associations.len(), 1);
    std::fs::rename(&repo, tmp.path().join("moved")).unwrap();
    let stale = core.refresh().unwrap();
    assert_eq!(stale.sections.len(), 1);
    assert!(
        stale
            .sources
            .iter()
            .any(|s| s.kind == "local" && s.state == "error" && s.last_success.is_some())
    );
}
#[cfg(unix)]
#[test]
fn external_symlink_document_is_not_readable() {
    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    git(&repo, &["init"]);
    git(&repo, &["config", "user.name", "test"]);
    git(&repo, &["config", "user.email", "test@example.org"]);
    std::fs::create_dir(repo.join("doc")).unwrap();
    std::fs::write(tmp.path().join("secret.md"), "secret").unwrap();
    std::os::unix::fs::symlink(tmp.path().join("secret.md"), repo.join("doc/escape.md")).unwrap();
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-m", "fixture"]);
    let mut core = CommandCenter::open(tmp.path().join("state.db")).unwrap();
    core.open_project(&repo).unwrap();
    assert!(core.search("secret", 20).unwrap().is_empty());
    assert!(core.document_path("doc/escape.md").is_err());
}

#[test]
fn future_schema_is_not_opened() {
    let tmp = tempfile::tempdir().unwrap();
    let db = tmp.path().join("future.sqlite");
    let conn = rusqlite::Connection::open(&db).unwrap();
    conn.execute_batch("PRAGMA user_version=99").unwrap();
    drop(conn);
    assert!(CommandCenter::open(&db).is_err());
}
