mod support;
use std::{
    path::Path,
    process::{Command, Stdio},
};
fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("/usr/bin/git")
        .current_dir(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success(), "Git fixture setup failed");
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}
#[test]
fn restore_forms_a_new_change_without_resetting_history() {
    let h = support::Harness::new(env!("CARGO_BIN_EXE_oasis7_context"), &[]);
    h.init();
    let candidate = h.candidate("machine-test", "original fixture content\n");
    let first = h.put("machine-test", &candidate, None);
    git(&h.root, &["init"]);
    git(&h.root, &["config", "user.name", "Fixture"]);
    git(
        &h.root,
        &["config", "user.email", "fixture@example.invalid"],
    );
    git(&h.root, &["add", "."]);
    git(&h.root, &["commit", "-m", "Original"]);
    let first_head = git(&h.root, &["rev-parse", "HEAD"]);
    let candidate = h.candidate("machine-test", "later fixture content\n");
    let second = h.put(
        "machine-test",
        &candidate,
        Some(first["data"]["revision"].as_str().unwrap()),
    );
    git(&h.root, &["add", "."]);
    git(&h.root, &["commit", "-m", "Later"]);
    let head = git(&h.root, &["rev-parse", "HEAD"]);
    let root = h.root.to_str().unwrap();
    let revision = second["data"]["revision"].as_str().unwrap();
    let restored = h.call(
        &[
            "restore",
            "machine-test",
            "--root",
            root,
            "--from-commit",
            &first_head,
            "--expected-revision",
            revision,
            "--actor",
            "human:fixture",
            "--json",
        ],
        0,
    );
    assert_ne!(restored["data"]["revision"], second["data"]["revision"]);
    assert_eq!(git(&h.root, &["rev-parse", "HEAD"]), head);
    let current = h.call(&["get", "machine-test", "--root", root, "--json"], 0);
    assert!(
        current["data"]["raw_text"]
            .as_str()
            .unwrap()
            .contains("original fixture content")
    );
    let status = h.call(&["status", "--root", root, "--json"], 0);
    assert_eq!(status["data"]["remote_contacted"], false);
    assert_eq!(status["data"]["comparison"], "raw_bytes");
    assert_eq!(status["data"]["working_tree_dirty"], true);
    h.call(
        &[
            "restore",
            "machine-test",
            "--root",
            root,
            "--from-commit",
            &first_head,
            "--expected-revision",
            revision,
            "--actor",
            "human:fixture",
            "--json",
        ],
        4,
    );
}

#[test]
fn held_store_rejects_history_and_restore_from_replaced_same_identity_checkout() {
    use oasis7_context::{application, model::WriteCondition, store::Store};
    let h = support::Harness::new(env!("CARGO_BIN_EXE_oasis7_context"), &[]);
    h.init();
    let candidate = h.candidate("machine-test", "original bound private content\n");
    h.put("machine-test", &candidate, None);
    git(&h.root, &["init"]);
    git(&h.root, &["config", "user.name", "Fixture"]);
    git(
        &h.root,
        &["config", "user.email", "fixture@example.invalid"],
    );
    git(&h.root, &["add", "."]);
    git(&h.root, &["commit", "-m", "Original bound history"]);
    let store = Store::open(&h.root).unwrap();
    let before = store.get("machine-test").unwrap();
    assert_eq!(
        application::history(&store, "machine-test", 10)
            .unwrap()
            .commits[0]
            .title,
        "Original bound history"
    );
    let mut pinned_child = Command::new("/usr/bin/git");
    pinned_child.args(["log", "-1", "--format=%s"]);
    store
        .directory()
        .pin_command_cwd(&mut pinned_child)
        .unwrap();
    let original = h.temp.path().join("original-held-directory");
    std::fs::rename(&h.root, &original).unwrap();
    let replacement = Store::init(&h.root, "fixture", "Replacement").unwrap();
    // Matching store/project IDs alone cannot distinguish the replacement repository.
    std::fs::write(h.root.join("context.json"), store.config_raw().unwrap()).unwrap();
    replacement
        .put(
            "machine-test",
            &std::fs::read(h.candidate("machine-test", "wrong replacement content\n")).unwrap(),
            WriteCondition::Absent,
            "fixture",
        )
        .unwrap();
    git(&h.root, &["init"]);
    git(&h.root, &["config", "user.name", "Fixture"]);
    git(
        &h.root,
        &["config", "user.email", "fixture@example.invalid"],
    );
    git(&h.root, &["add", "."]);
    git(&h.root, &["commit", "-m", "Replacement private history"]);
    let replacement_commit = git(&h.root, &["rev-parse", "HEAD"]);
    let pinned_output = pinned_child.output().unwrap();
    assert!(pinned_output.status.success());
    assert_eq!(
        String::from_utf8(pinned_output.stdout).unwrap().trim(),
        "Original bound history"
    );
    assert_eq!(
        oasis7_context::git::history(&h.root, "machine-test", 10)
            .unwrap()
            .commits[0]
            .title,
        "Replacement private history"
    );
    let restore = application::restore_entry(
        &store,
        "machine-test",
        &replacement_commit,
        WriteCondition::Revision(before.revision),
        "desktop:test",
    );
    let history = application::history(&store, "machine-test", 10);
    assert!(
        restore.is_err(),
        "historical replacement must not modify original pinned store"
    );
    assert!(
        history.is_err(),
        "replacement history must not escape original binding"
    );
    assert_eq!(
        store.get("machine-test").unwrap().raw_bytes,
        before.raw_bytes
    );
}
