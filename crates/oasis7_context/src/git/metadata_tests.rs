use super::tests::{fixture, git};
use super::*;
use std::{fs, os::unix::fs::MetadataExt, path::PathBuf};

fn check_operation(root: &Path, restore: bool, replace: impl FnOnce()) {
    let oid = history(root, "machine-test", 10).unwrap().head;
    let directory = crate::fs::Directory::open(root).unwrap();
    let pinned = GitRoot::pinned(root, &directory);
    let failure = if restore {
        read_after_probe(&pinned, "machine-test", &oid, replace)
            .err()
            .unwrap()
    } else {
        history_after_probe(&pinned, "machine-test", 10, replace)
            .err()
            .unwrap()
    };
    assert_eq!(failure.code, "HISTORY_IDENTITY_CHANGED");
    assert!(
        directory.matches_path(root).unwrap(),
        "the store root itself never changed"
    );
}

#[test]
fn replacing_git_directory_after_probe_rejects_history_and_historical_reads() {
    for restore in [false, true] {
        let original = fixture();
        let foreign = fixture();
        let root = original.path().canonicalize().unwrap();
        fs::write(
            foreign.path().join("entries/machine-test.md"),
            b"foreign repository\n",
        )
        .unwrap();
        git(foreign.path(), &["add", "."]);
        git(foreign.path(), &["commit", "-m", "Foreign"]);
        check_operation(&root, restore, || {
            fs::rename(root.join(".git"), root.join("original-git")).unwrap();
            fs::rename(foreign.path().join(".git"), root.join(".git")).unwrap();
        });
    }
}

fn linked_fixture() -> (tempfile::TempDir, tempfile::TempDir, PathBuf, PathBuf) {
    let common = fixture();
    let external = tempfile::tempdir().unwrap();
    let base = external.path().canonicalize().unwrap();
    let root = base.join("linked");
    git(
        common.path(),
        &[
            "worktree",
            "add",
            "--detach",
            root.to_str().unwrap(),
            "HEAD",
        ],
    );
    let initial = history(&root, "machine-test", 10).unwrap();
    assert_eq!(
        read_at_commit(&root, "machine-test", &initial.head)
            .unwrap()
            .entry_bytes,
        b"first\n"
    );
    let bytes = fs::read(root.join(".git")).unwrap();
    let old = PathBuf::from(
        std::str::from_utf8(&bytes)
            .unwrap()
            .strip_prefix("gitdir: ")
            .unwrap()
            .trim(),
    );
    let git_dir = base.join("git-metadata");
    fs::rename(&old, &git_dir).unwrap();
    let common_dir = common.path().canonicalize().unwrap().join(".git");
    fs::write(
        git_dir.join("commondir"),
        format!("{}\n", common_dir.display()),
    )
    .unwrap();
    fs::write(
        root.join(".git"),
        format!("gitdir: {}\n", git_dir.display()),
    )
    .unwrap();
    assert_eq!(
        read_at_commit(
            &root,
            "machine-test",
            &history(&root, "machine-test", 10).unwrap().head
        )
        .unwrap()
        .entry_bytes,
        b"first\n"
    );
    (common, external, root, git_dir)
}

#[test]
fn linked_gitfile_and_target_directory_cannot_retarget_after_probe() {
    for change_file in [true, false] {
        for restore in [false, true] {
            let (_common, _external, root, metadata) = linked_fixture();
            let foreign = fixture();
            let before = fs::metadata(root.join(".git")).unwrap().ino();
            check_operation(&root, restore, || {
                if change_file {
                    fs::write(
                        root.join(".git"),
                        format!("gitdir: {}\n", foreign.path().join(".git").display()),
                    )
                    .unwrap();
                    assert_eq!(
                        fs::metadata(root.join(".git")).unwrap().ino(),
                        before,
                        "retargeting in place keeps file inode"
                    );
                } else {
                    fs::rename(&metadata, metadata.with_extension("original")).unwrap();
                    fs::rename(foreign.path().join(".git"), &metadata).unwrap();
                }
            });
        }
    }
}

#[test]
fn linked_common_file_and_directory_cannot_retarget_after_probe() {
    for change_file in [true, false] {
        for restore in [false, true] {
            let (common, _external, root, metadata) = linked_fixture();
            let foreign = fixture();
            let pointer = metadata.join("commondir");
            let before = fs::metadata(&pointer).unwrap().ino();
            check_operation(&root, restore, || {
                if change_file {
                    fs::write(
                        &pointer,
                        format!("{}\n", foreign.path().join(".git").display()),
                    )
                    .unwrap();
                    assert_eq!(fs::metadata(&pointer).unwrap().ino(), before);
                } else {
                    let original = common.path().join(".git");
                    fs::rename(&original, common.path().join("original-git")).unwrap();
                    fs::rename(foreign.path().join(".git"), &original).unwrap();
                }
            });
        }
    }
}

#[test]
fn absent_metadata_stops_before_repository_probe_or_read() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let opened = GitRoot::unpinned(&root);
    let mut after_probe = false;
    let error = history_after_probe(&opened, "machine-test", 10, || {
        after_probe = true;
    })
    .unwrap_err();
    assert_eq!(error.code, "HISTORY_UNAVAILABLE");
    assert!(!after_probe);
    assert!(!status(&root).unwrap().available);
}
#[test]
fn metadata_replacement_during_child_is_rejected_even_when_git_exits_unsuccessfully() {
    use std::os::unix::fs::PermissionsExt;
    for exit in [0, 1] {
        let original = fixture();
        let root = original.path().canonicalize().unwrap();
        let directory = crate::fs::Directory::open(&root).unwrap();
        let pinned = GitRoot::pinned(&root, &directory);
        probe(&pinned).unwrap();
        let script = root.join("replace-metadata");
        fs::write(&script, format!("#!/bin/sh\n/bin/mv .git original-git\n/bin/mkdir .git\nprintf foreign-output\nexit {exit}\n")).unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            run_program(
                script.to_str().unwrap(),
                Some(&pinned),
                &["rev-parse", "HEAD"]
            )
            .unwrap_err()
            .code,
            "HISTORY_IDENTITY_CHANGED"
        );
        assert!(directory.matches_path(&root).unwrap());
    }
}
