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
