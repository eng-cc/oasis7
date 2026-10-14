use super::*;
fn fixture() -> (tempfile::TempDir, Project) {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    for args in [
        vec!["init"],
        vec!["config", "user.name", "test"],
        vec!["config", "user.email", "test@example.org"],
    ] {
        git(root, &args).unwrap();
    }
    std::fs::create_dir(root.join("doc")).unwrap();
    std::fs::write(root.join("doc/prd.md"), "# Product\nOriginal\n").unwrap();
    git(root, &["add", "."]).unwrap();
    git(root, &["commit", "-m", "fixture"]).unwrap();
    let project = project(root).unwrap();
    (tmp, project)
}
#[test]
fn rejects_content_changes_even_when_head_and_dirty_status_do_not_change() {
    let (tmp, project) = fixture();
    let file = tmp.path().join("doc/prd.md");
    std::fs::write(&file, "# Product\nDirty before\n").unwrap();
    let result = read_candidate(&project, || {
        std::fs::write(&file, "# Product\nDirty after!\n").unwrap();
    });
    assert!(
        result.is_err(),
        "must not publish content captured before the edit"
    );
}
#[test]
fn rejects_tracked_candidate_list_changes_without_a_new_commit() {
    let (tmp, project) = fixture();
    let result = read_candidate(&project, || {
        std::fs::write(tmp.path().join("doc/new.md"), "# Added\n").unwrap();
        git(tmp.path(), &["add", "doc/new.md"]).unwrap();
    });
    assert!(
        result.is_err(),
        "must not publish an obsolete candidate list"
    );
}
#[test]
fn unchanged_dirty_documents_are_readable() {
    let (tmp, project) = fixture();
    std::fs::write(tmp.path().join("doc/prd.md"), "# Product\nDirty content\n").unwrap();
    let (sections, _, _, _) = read(&project).unwrap();
    assert!(
        sections
            .iter()
            .any(|s| s.dirty && s.content.contains("Dirty content"))
    );
}

#[test]
fn rejects_deleted_or_oversized_sources_after_reading() {
    for oversized in [false, true] {
        let (tmp, project) = fixture();
        let file = tmp.path().join("doc/prd.md");
        let result = read_candidate(&project, || {
            if oversized {
                std::fs::write(&file, vec![b'x'; 1024 * 1024 + 1]).unwrap();
            } else {
                std::fs::remove_file(&file).unwrap();
            }
        });
        assert!(result.is_err());
    }
}
#[test]
fn rejects_head_changes_even_if_the_documents_are_identical() {
    let (tmp, project) = fixture();
    let result = read_candidate(&project, || {
        git(tmp.path(), &["commit", "--allow-empty", "-m", "advanced"]).unwrap();
    });
    assert!(result.is_err());
}

#[test]
fn unrelated_source_edits_do_not_invalidate_document_indexing() {
    let (tmp, _) = fixture();
    std::fs::write(tmp.path().join("code.rs"), "original").unwrap();
    git(tmp.path(), &["add", "code.rs"]).unwrap();
    git(tmp.path(), &["commit", "-m", "code"]).unwrap();
    let project = super::project(tmp.path()).unwrap();
    assert!(
        read_candidate(&project, || {
            std::fs::write(tmp.path().join("code.rs"), "edited").unwrap();
        })
        .is_ok()
    );
}

#[test]
fn governance_json_changes_do_not_invalidate_markdown_indexing() {
    let (tmp, _) = fixture();
    let file = tmp.path().join("doc/metadata.json");
    std::fs::write(&file, "{}").unwrap();
    git(tmp.path(), &["add", "."]).unwrap();
    git(tmp.path(), &["commit", "-m", "metadata"]).unwrap();
    let project = super::project(tmp.path()).unwrap();
    assert!(
        read_candidate(&project, || {
            std::fs::write(&file, "{\"changed\":true}").unwrap();
            std::fs::write(tmp.path().join("doc/new.json"), "{}").unwrap();
            git(tmp.path(), &["add", "doc/new.json"]).unwrap();
        })
        .is_ok()
    );
}

#[cfg(unix)]
#[test]
fn rejects_dirty_document_replaced_with_fifo_during_verification() {
    use std::os::unix::ffi::OsStrExt;
    let (tmp, project) = fixture();
    let file = tmp.path().join("doc/prd.md");
    std::fs::write(&file, "# Dirty before\n").unwrap();
    let result = read_candidate(&project, || {
        std::fs::remove_file(&file).unwrap();
        let name = std::ffi::CString::new(file.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    });
    assert!(
        result.is_err(),
        "replacement must reject rather than wait for a writer"
    );
}
