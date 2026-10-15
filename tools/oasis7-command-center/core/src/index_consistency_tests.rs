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
fn navigation_entries_survive_body_documents_exhausting_the_byte_budget() {
    let (tmp, _) = fixture();
    let root = tmp.path();
    let entries = [
        "AGENTS.md",
        "doc/README.md",
        "doc/product/README.md",
        "doc/engineering/README.md",
        "doc/engineering/command-center/README.md",
        "skills/oasis7-command-center/references/project-map.md",
    ];
    for entry in entries {
        std::fs::create_dir_all(root.join(entry).parent().unwrap()).unwrap();
        std::fs::write(root.join(entry), "# Navigation entry\n").unwrap();
    }
    // Product bodies used to precede professional README/project-map entries.
    let body = format!("# Body\n{}", "x".repeat(1024 * 1024 - 7));
    for index in 0..15 {
        std::fs::write(root.join(format!("doc/product/body-{index:02}.md")), &body).unwrap();
    }
    // Fill the remaining bytes after the previously preferred small sources.
    let preferred_bytes = "# Product\nOriginal\n".len() + 2 * "# Navigation entry\n".len();
    std::fs::write(
        root.join("doc/product/zz-tail.md"),
        &body[..body.len() - preferred_bytes],
    )
    .unwrap();
    git(root, &["add", "."]).unwrap();
    git(root, &["commit", "-m", "budget fixture"]).unwrap();
    let project = super::project(root).unwrap();
    let (sections, _, coverage, partial) = read(&project).unwrap();
    assert!(partial);
    assert!(coverage.contains("total_budget"), "{coverage}");
    for entry in entries {
        assert!(
            sections.iter().any(|section| section.path == entry),
            "missing {entry}: {coverage}"
        );
    }
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

#[test]
fn rejects_repaired_omitted_dirty_document_before_publication() {
    let (tmp, project) = fixture();
    let file = tmp.path().join("doc/prd.md");
    std::fs::write(&file, [0xff]).unwrap();
    let result = read_candidate(&project, || {
        std::fs::write(&file, "# Repaired\n").unwrap();
    });
    assert!(
        result.is_err(),
        "old omission must not publish after the source becomes readable"
    );
}

#[cfg(unix)]
#[test]
fn linked_documents_and_ancestors_are_omitted_without_reading_targets() {
    use std::os::unix::fs::symlink;
    for ancestor in [false, true] {
        let (tmp, project) = fixture();
        let private = tmp.path().join("private");
        std::fs::create_dir(&private).unwrap();
        std::fs::write(private.join("prd.md"), "# Private secret\n").unwrap();
        if ancestor {
            std::fs::remove_file(tmp.path().join("doc/prd.md")).unwrap();
            std::fs::remove_dir(tmp.path().join("doc")).unwrap();
            symlink(&private, tmp.path().join("doc")).unwrap();
        } else {
            std::fs::remove_file(tmp.path().join("doc/prd.md")).unwrap();
            symlink(private.join("prd.md"), tmp.path().join("doc/prd.md")).unwrap();
        }
        let (sections, _, coverage, partial) = read(&project).unwrap();
        assert!(
            sections.is_empty(),
            "linked target must not enter the index"
        );
        assert!(partial);
        assert!(coverage.contains("symlink 1"), "{coverage}");
    }
}

#[cfg(unix)]
#[test]
fn replacing_read_document_with_identical_link_rejects_candidate_and_source_open() {
    use std::os::unix::fs::symlink;
    let (tmp, project) = fixture();
    let file = tmp.path().join("doc/prd.md");
    let target = tmp.path().join("private.md");
    std::fs::write(&file, "# Dirty before\n").unwrap();
    std::fs::write(&target, "# Dirty before\n").unwrap();
    let result = read_candidate(&project, || {
        std::fs::remove_file(&file).unwrap();
        symlink(&target, &file).unwrap();
    });
    assert!(result.is_err());
    assert!(safe_path(Path::new(&project.path), "doc/prd.md").is_err());
}

#[cfg(unix)]
#[test]
fn external_and_dangling_links_are_classified_without_target_reads() {
    use std::os::unix::fs::symlink;
    let (tmp, project) = fixture();
    let external = tempfile::tempdir().unwrap();
    let target = external.path().join("secret.md");
    std::fs::write(&target, "# External secret\n").unwrap();
    let file = tmp.path().join("doc/prd.md");
    std::fs::remove_file(&file).unwrap();
    symlink(&target, &file).unwrap();
    for deleted in [false, true] {
        if deleted {
            std::fs::remove_file(&target).unwrap();
        }
        let (sections, _, coverage, partial) = read(&project).unwrap();
        assert!(sections.is_empty());
        assert!(partial);
        assert!(coverage.contains("symlink 1"), "{coverage}");
    }
}
