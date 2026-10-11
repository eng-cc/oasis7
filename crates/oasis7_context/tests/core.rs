use oasis7_context::{format, model::*, query::*, store::Store};
fn entry(id: &str) -> Vec<u8> {
    format::encode(&Entry {
        metadata: Metadata {
            schema_version: "oasis7.context_entry.v1".into(),
            project_id: "test".into(),
            id: id.into(),
            kind: "machine".into(),
            title: "测试 machine".into(),
            environment: "dev".into(),
            owner: "test-owner".into(),
            tags: vec!["build".into()],
            state: "active".into(),
            updated_at: "2026-01-01T00:00:00Z".into(),
            updated_by: "human:test".into(),
            verification: None,
            credential_refs: vec![],
            links: vec![],
        },
        body: "\n中文正文 Test\n".into(),
    })
    .unwrap()
}
#[test]
fn cas_and_verification() {
    let t = tempfile::tempdir().unwrap();
    let s = Store::init(&t.path().join("store"), "test", "Test").unwrap();
    let bytes = entry("a");
    let first = s.put("a", &bytes, WriteCondition::Absent, "test").unwrap();
    assert!(
        !s.put(
            "a",
            &bytes,
            WriteCondition::Revision(first.revision.clone()),
            "other"
        )
        .unwrap()
        .changed
    );
    assert_eq!(
        s.put("a", &bytes, WriteCondition::Absent, "test")
            .unwrap_err()
            .code,
        "REVISION_CONFLICT"
    );
    let v = s
        .verify(
            "a",
            &first.revision,
            "observation",
            "test:evidence",
            None,
            "human:test",
        )
        .unwrap();
    assert_ne!(v.revision, first.revision);
    assert_eq!(
        s.get("a").unwrap().verification_state,
        VerificationState::Current
    );
    let mut changed = s.get("a").unwrap().entry;
    changed.body.push_str("changed");
    s.put(
        "a",
        &format::encode(&changed).unwrap(),
        WriteCondition::Revision(v.revision),
        "test",
    )
    .unwrap();
    assert_eq!(
        s.get("a").unwrap().verification_state,
        VerificationState::ContentChanged
    );
}
#[test]
fn pagination_is_stable_and_detects_changes() {
    let t = tempfile::tempdir().unwrap();
    let s = Store::init(&t.path().join("store"), "test", "Test").unwrap();
    for id in ["a", "b", "c"] {
        s.put(id, &entry(id), WriteCondition::Absent, "test")
            .unwrap();
    }
    let q = ListQuery {
        limit: 1,
        ..Default::default()
    };
    let page = s.list(&q).unwrap();
    assert_eq!(page.total_matches, Some(3));
    assert_eq!(page.entries[0].id, "a");
    let second = s
        .list(&ListQuery {
            cursor: page.next_cursor.clone(),
            ..q.clone()
        })
        .unwrap();
    assert_eq!(second.entries[0].id, "b");
    let old = s.get("c").unwrap();
    s.archive("c", &old.revision, "test").unwrap();
    assert_eq!(
        s.list(&ListQuery {
            cursor: page.next_cursor,
            ..q
        })
        .unwrap_err()
        .code,
        "CURSOR_STALE"
    );
}
#[test]
fn malformed_entries_do_not_claim_complete() {
    let t = tempfile::tempdir().unwrap();
    let s = Store::init(&t.path().join("store"), "test", "Test").unwrap();
    s.put("a", &entry("a"), WriteCondition::Absent, "test")
        .unwrap();
    std::fs::write(s.root.join("entries/b.md"), "bad").unwrap();
    let page = s.list(&ListQuery::default()).unwrap();
    assert!(!page.complete);
    assert_eq!(page.total_matches, None);
    assert_eq!(page.next_cursor, None);
}
#[test]
fn frontmatter_strict_and_revision_raw() {
    let bytes = entry("a");
    let raw = String::from_utf8(bytes.clone()).unwrap();
    let crlf = raw.replace('\n', "\r\n");
    assert_eq!(
        format::parse(&bytes).unwrap(),
        format::parse(crlf.as_bytes()).unwrap()
    );
    assert_ne!(format::revision(&bytes), format::revision(crlf.as_bytes()));
    let duplicate = raw.replace("\"title\":", "\"title\": \"duplicate\", \"title\":");
    assert!(format::parse(duplicate.as_bytes()).is_err());
    assert!(format::strict_json::<serde_json::Value>(br#"{"a":{"b":{"c":{"d":{"e":{"f":{"g":{"h":{"i":{"j":{"k":{"l":{"m":{"n":{"o":{"p":{"q":1}}}}}}}}}}}}}}}}}"#).is_err());
}
#[cfg(unix)]
#[test]
#[allow(unsafe_code)] // SAFETY: mkfifo receives a live NUL-terminated fixture path.
fn filesystem_rejects_symlinks_hardlinks_and_fifo() {
    use std::os::unix::fs::symlink;
    let t = tempfile::tempdir().unwrap();
    let s = Store::init(&t.path().join("store"), "test", "Test").unwrap();
    let out = t.path().join("outside");
    std::fs::write(&out, entry("a")).unwrap();
    symlink(&out, s.root.join("entries/a.md")).unwrap();
    assert!(s.get("a").is_err());
    std::fs::remove_file(s.root.join("entries/a.md")).unwrap();
    std::fs::hard_link(&out, s.root.join("entries/a.md")).unwrap();
    assert_eq!(s.get("a").unwrap_err().code, "UNSAFE_PATH");
    std::fs::remove_file(s.root.join("entries/a.md")).unwrap();
    let path = std::ffi::CString::new(s.root.join("entries/a.md").to_str().unwrap()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    assert_eq!(s.get("a").unwrap_err().code, "UNSAFE_PATH");
}
#[test]
fn held_directory_survives_path_replacement() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("store");
    let s = Store::init(&root, "test", "Test").unwrap();
    std::fs::rename(&root, t.path().join("original")).unwrap();
    let other = Store::init(&root, "test", "Other").unwrap();
    s.put("a", &entry("a"), WriteCondition::Absent, "test")
        .unwrap();
    assert!(other.get("a").is_err());
    assert!(t.path().join("original/entries/a.md").exists());
}
#[test]
fn queries_do_not_create_local_state() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("store");
    let s = Store::init(&root, "test", "Test").unwrap();
    s.put("a", &entry("a"), WriteCondition::Absent, "test")
        .unwrap();
    drop(s);
    std::fs::remove_dir_all(root.join(".context-local")).unwrap();
    let s = Store::open(&root).unwrap();
    s.get("a").unwrap();
    s.list(&ListQuery::default()).unwrap();
    assert!(!root.join(".context-local").exists());
}
#[test]
fn competing_writers_cannot_overwrite() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("store");
    let s = Store::init(&root, "test", "Test").unwrap();
    let revision = s
        .put("a", &entry("a"), WriteCondition::Absent, "test")
        .unwrap()
        .revision;
    let gate = std::sync::Arc::new(std::sync::Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|i| {
            let root = root.clone();
            let revision = revision.clone();
            let gate = gate.clone();
            std::thread::spawn(move || {
                let mut s = Store::open(&root).unwrap();
                s.lock_timeout_ms = 2000;
                let mut e = format::parse(&entry("a")).unwrap();
                e.body.push_str(&i.to_string());
                gate.wait();
                s.put(
                    "a",
                    &format::encode(&e).unwrap(),
                    WriteCondition::Revision(revision),
                    "test",
                )
            })
        })
        .collect();
    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| r.as_ref().is_err_and(|e| e.code == "REVISION_CONFLICT"))
            .count(),
        1
    );
}
#[test]
fn canonical_contract_vector() {
    let e = format::parse(&entry("a")).unwrap();
    assert_eq!(
        format::subject_sha256(&e),
        "48e5b353dc61e3025a63f9abe5d24f4db171cbc6d1f3c84b99ea515c6eddd958"
    );
}

#[test]
fn concurrent_distinct_entries_cannot_exceed_store_byte_capacity() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let store = Store::init(&root, "test", "Test").unwrap();
    let entries_dir = store.directory().child("entries").unwrap();
    for i in 0..63 {
        let id = format!("seed-{i}");
        let mut e = format::parse(&entry(&id)).unwrap();
        let overhead = format::encode(&e).unwrap().len() - e.body.len();
        e.body = "x".repeat(MAX_ENTRY_BYTES - overhead);
        entries_dir
            .write_atomic(&format!("{id}.md"), &format::encode(&e).unwrap(), false)
            .unwrap();
    }
    let mut e = format::parse(&entry("seed-tail")).unwrap();
    e.body = "x".repeat(400 * 1024);
    entries_dir
        .write_atomic("seed-tail.md", &format::encode(&e).unwrap(), false)
        .unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let tasks: Vec<_> = ["create-a", "create-b"]
        .into_iter()
        .map(|id| {
            let root = root.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut store = Store::open(&root).unwrap();
                store.lock_timeout_ms = 30000;
                let mut e = format::parse(&entry(id)).unwrap();
                e.body = "candidate".repeat(400 * 1024 / 9);
                barrier.wait();
                store.put(
                    id,
                    &format::encode(&e).unwrap(),
                    WriteCondition::Absent,
                    "test",
                )
            })
        })
        .collect();
    let results: Vec<_> = tasks.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| r.as_ref().is_err_and(|e| e.code == "LIMIT_EXCEEDED"))
            .count(),
        1,
        "{results:?}"
    );
    assert!(
        store
            .read_all()
            .unwrap()
            .iter()
            .map(|e| e.raw_bytes.len())
            .sum::<usize>()
            <= MAX_STORE_BYTES
    );
}

#[test]
fn concurrent_distinct_entries_cannot_exceed_store_count_capacity() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("store");
    let store = Store::init(&root, "test", "Test").unwrap();
    for i in 0..MAX_ENTRIES - 1 {
        let id = format!("seed-{i}");
        std::fs::write(root.join(format!("entries/{id}.md")), entry(&id)).unwrap();
    }
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let tasks: Vec<_> = ["create-a", "create-b"]
        .into_iter()
        .map(|id| {
            let root = root.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let mut store = Store::open(&root).unwrap();
                store.lock_timeout_ms = 30000;
                barrier.wait();
                store.put(id, &entry(id), WriteCondition::Absent, "test")
            })
        })
        .collect();
    let results: Vec<_> = tasks.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(
        results.iter().filter(|r| r.is_ok()).count(),
        1,
        "{results:?}"
    );
    assert_eq!(
        results
            .iter()
            .filter(|r| r.as_ref().is_err_and(|e| e.code == "LIMIT_EXCEEDED"))
            .count(),
        1
    );
    assert_eq!(store.read_all().unwrap().len(), MAX_ENTRIES);
}
