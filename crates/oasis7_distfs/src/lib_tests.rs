use super::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DemoEvent {
    id: u64,
    kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct DemoSnapshot {
    tick: u64,
    world: String,
}

fn temp_dir(prefix: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("duration")
        .as_nanos();
    std::env::temp_dir().join(format!("oasis7-distfs-{prefix}-{unique}"))
}

#[test]
fn cas_roundtrip_and_pin() {
    let dir = temp_dir("cas");
    let store = LocalCasStore::new(&dir);

    let bytes = b"hello distfs".to_vec();
    let hash = store.put_bytes(&bytes).expect("put");
    assert!(store.has(&hash).expect("has"));
    assert_eq!(store.get(&hash).expect("get"), bytes);

    store.pin(&hash).expect("pin");
    assert!(store.is_pinned(&hash).expect("is pinned"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cas_sha256_roundtrip_and_verify() {
    let dir = temp_dir("cas-sha256");
    let store = LocalCasStore::new_with_hash_algorithm(&dir, HashAlgorithm::Sha256);

    let bytes = b"hello sha256 distfs".to_vec();
    let hash = store.put_bytes(&bytes).expect("put");
    assert!(store.has(&hash).expect("has"));
    assert_eq!(store.get_verified(&hash).expect("verified get"), bytes);

    let blob_path = store.blobs_dir().join(format!("{hash}.blob"));
    fs::write(&blob_path, b"tampered").expect("tamper blob");
    assert!(matches!(
        store.get_verified(&hash),
        Err(WorldError::BlobHashMismatch { .. })
    ));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cas_transparently_compresses_large_blob_on_disk() {
    let dir = temp_dir("cas-compressed-blob");
    let store = LocalCasStore::new(&dir);

    let bytes = vec![b'a'; 16 * 1024];
    let hash = store.put_bytes(&bytes).expect("put");
    let blob_path = store.blobs_dir().join(format!("{hash}.blob"));
    let disk_bytes = fs::read(blob_path.as_path()).expect("read blob from disk");

    assert!(disk_bytes.starts_with(b"O7CBLOB1"));
    assert!(disk_bytes.len() < bytes.len());
    assert_eq!(store.get(&hash).expect("get"), bytes);
    assert_eq!(store.get_verified(&hash).expect("verified get"), bytes);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn cas_does_not_misdecode_raw_blob_that_looks_like_compressed_payload() {
    let dir = temp_dir("cas-magic-collision");
    let store = LocalCasStore::new(&dir);

    let mut bytes = b"O7CBLOB1".to_vec();
    bytes.extend_from_slice(&(4_u64).to_le_bytes());
    bytes.extend_from_slice(
        zstd::stream::encode_all(b"evil".as_slice(), 3)
            .expect("encode fake compressed suffix")
            .as_slice(),
    );
    let hash = store.put_bytes(&bytes).expect("put");

    assert_eq!(store.get(&hash).expect("get"), bytes);
    assert_eq!(store.get_verified(&hash).expect("verified get"), bytes);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn segment_and_assemble_roundtrip() {
    let dir = temp_dir("segment");
    let store = LocalCasStore::new(&dir);
    let snapshot = DemoSnapshot {
        tick: 42,
        world: "w1".to_string(),
    };
    let events = vec![
        DemoEvent {
            id: 1,
            kind: "a".to_string(),
        },
        DemoEvent {
            id: 2,
            kind: "b".to_string(),
        },
        DemoEvent {
            id: 3,
            kind: "c".to_string(),
        },
    ];

    let manifest = segment_snapshot(
        &snapshot,
        "w1",
        1,
        &store,
        SegmentConfig {
            snapshot_chunk_bytes: 8,
            ..SegmentConfig::default()
        },
    )
    .expect("segment snapshot");

    let segments = segment_journal(
        &events,
        &store,
        SegmentConfig {
            journal_events_per_segment: 2,
            ..SegmentConfig::default()
        },
        |event| event.id,
    )
    .expect("segment journal");

    let snapshot_loaded: DemoSnapshot =
        assemble_snapshot(&manifest, &store).expect("assemble snapshot");
    let events_loaded: Vec<DemoEvent> =
        assemble_journal(&segments, &store, |event: &DemoEvent| event.id)
            .expect("assemble journal");

    assert_eq!(snapshot_loaded, snapshot);
    assert_eq!(events_loaded, events);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn file_store_write_read_roundtrip() {
    let dir = temp_dir("file-roundtrip");
    let store = LocalCasStore::new(&dir);

    let metadata = store
        .write_file("docs/readme.txt", b"hello distfs file")
        .expect("write");
    assert_eq!(metadata.path, "docs/readme.txt");
    assert_eq!(metadata.size_bytes, 17);
    assert!(!metadata.content_hash.is_empty());
    assert!(metadata.updated_at_ms > 0);

    let loaded = store.read_file("docs/readme.txt").expect("read");
    assert_eq!(loaded, b"hello distfs file");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn file_store_overwrite_updates_hash_and_metadata() {
    let dir = temp_dir("file-overwrite");
    let store = LocalCasStore::new(&dir);

    let first = store.write_file("a/note.txt", b"v1").expect("write first");
    let first_blob_path = store
        .blob_path(first.content_hash.as_str())
        .expect("first blob path");
    assert!(
        first_blob_path.exists(),
        "first blob should exist before overwrite"
    );
    let second = store
        .write_file("a/note.txt", b"v2-data")
        .expect("write second");

    assert_eq!(first.path, second.path);
    assert_ne!(first.content_hash, second.content_hash);
    assert!(second.updated_at_ms >= first.updated_at_ms);
    assert_eq!(second.size_bytes, 7);

    let loaded = store.read_file("a/note.txt").expect("read");
    assert_eq!(loaded, b"v2-data");
    let stat = store
        .stat_file("a/note.txt")
        .expect("stat")
        .expect("exists");
    assert_eq!(stat.content_hash, second.content_hash);
    assert_eq!(stat.size_bytes, 7);
    assert!(
        !first_blob_path.exists(),
        "unreferenced overwrite blob should be pruned immediately"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn file_store_delete_removes_mapping() {
    let dir = temp_dir("file-delete");
    let store = LocalCasStore::new(&dir);

    let written = store
        .write_file("workspace/a.log", b"to-delete")
        .expect("write");
    let blob_path = store
        .blob_path(written.content_hash.as_str())
        .expect("blob path");
    assert!(blob_path.exists(), "blob should exist before delete");
    let removed = store.delete_file("workspace/a.log").expect("delete");
    assert!(removed);
    assert!(store.stat_file("workspace/a.log").expect("stat").is_none());
    assert!(!store.delete_file("workspace/a.log").expect("delete again"));
    assert!(
        !blob_path.exists(),
        "delete should prune the now-unreferenced blob"
    );

    let read_result = store.read_file("workspace/a.log");
    assert!(matches!(
        read_result,
        Err(WorldError::DistributedValidationFailed { .. })
    ));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn file_store_compacts_files_index_json() {
    let dir = temp_dir("file-index-compact");
    let store = LocalCasStore::new(&dir);

    store
        .write_file("docs/readme.txt", b"hello distfs file")
        .expect("write");

    let bytes = fs::read(store.files_index_path()).expect("read files index");
    assert!(
        !bytes.contains(&b'\n'),
        "files index should be written without pretty-print newlines"
    );
    assert!(
        !bytes.windows(2).any(|window| window == b"  "),
        "files index should not contain indentation padding"
    );

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn file_store_list_and_stat_return_indexed_entries() {
    let dir = temp_dir("file-list");
    let store = LocalCasStore::new(&dir);

    store.write_file("b/file.txt", b"bbb").expect("write b");
    store.write_file("./a/file.txt", b"aaa").expect("write a");

    let files = store.list_files().expect("list");
    let paths: Vec<String> = files.iter().map(|item| item.path.clone()).collect();
    assert_eq!(
        paths,
        vec!["a/file.txt".to_string(), "b/file.txt".to_string()]
    );

    let stat_a = store
        .stat_file("a/file.txt")
        .expect("stat a")
        .expect("exists");
    assert_eq!(stat_a.size_bytes, 3);
    let bytes = store.read_file("a/file.txt").expect("read a");
    assert_eq!(bytes, b"aaa");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn file_store_rejects_invalid_paths() {
    let dir = temp_dir("file-invalid-path");
    let store = LocalCasStore::new(&dir);

    for invalid in ["", "/", "/root.txt", "..", "../a.txt", "a/../../b"] {
        assert!(matches!(
            store.write_file(invalid, b"bad"),
            Err(WorldError::DistributedValidationFailed { .. })
        ));
        assert!(matches!(
            store.read_file(invalid),
            Err(WorldError::DistributedValidationFailed { .. })
        ));
        assert!(matches!(
            store.delete_file(invalid),
            Err(WorldError::DistributedValidationFailed { .. })
        ));
        assert!(matches!(
            store.stat_file(invalid),
            Err(WorldError::DistributedValidationFailed { .. })
        ));
    }

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn file_store_write_if_match_enforces_hash_precondition() {
    let dir = temp_dir("file-cas-write");
    let store = LocalCasStore::new(&dir);

    let first = store.write_file("state/a.txt", b"v1").expect("write first");
    let second = store
        .write_file_if_match("state/a.txt", Some(first.content_hash.as_str()), b"v2")
        .expect("write with match");
    assert_ne!(first.content_hash, second.content_hash);
    assert_eq!(store.read_file("state/a.txt").expect("read"), b"v2");

    let stale_write =
        store.write_file_if_match("state/a.txt", Some(first.content_hash.as_str()), b"v3");
    assert!(matches!(
        stale_write,
        Err(WorldError::DistributedValidationFailed { .. })
    ));

    let missing_precondition = store.write_file_if_match(
        "state/missing.txt",
        Some(first.content_hash.as_str()),
        b"new",
    );
    assert!(matches!(
        missing_precondition,
        Err(WorldError::DistributedValidationFailed { .. })
    ));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn file_store_delete_if_match_enforces_hash_precondition() {
    let dir = temp_dir("file-cas-delete");
    let store = LocalCasStore::new(&dir);

    let written = store.write_file("state/a.txt", b"v1").expect("write");
    let mismatch = store.delete_file_if_match("state/a.txt", Some("stale-hash"));
    assert!(matches!(
        mismatch,
        Err(WorldError::DistributedValidationFailed { .. })
    ));
    assert!(store.stat_file("state/a.txt").expect("stat").is_some());

    let removed = store
        .delete_file_if_match("state/a.txt", Some(written.content_hash.as_str()))
        .expect("delete with match");
    assert!(removed);
    assert!(store.stat_file("state/a.txt").expect("stat").is_none());
    assert!(!store
        .delete_file_if_match("state/a.txt", None)
        .expect("delete missing without precondition"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn file_store_if_match_rejects_invalid_expected_hash() {
    let dir = temp_dir("file-cas-invalid-hash");
    let store = LocalCasStore::new(&dir);

    let write_result = store.write_file_if_match("a.txt", Some("../bad"), b"hello");
    assert!(matches!(
        write_result,
        Err(WorldError::BlobHashInvalid { .. })
    ));

    let delete_result = store.delete_file_if_match("a.txt", Some("../bad"));
    assert!(matches!(
        delete_result,
        Err(WorldError::BlobHashInvalid { .. })
    ));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn file_index_audit_reports_missing_dangling_and_orphan_blobs() {
    let dir = temp_dir("file-audit");
    let store = LocalCasStore::new(&dir);

    let live = store.write_file("live.txt", b"live").expect("write live");
    let orphan_hash = store.put_bytes(b"orphan").expect("write orphan blob");

    let missing = store
        .write_file("missing.txt", b"missing")
        .expect("write missing");
    let missing_path = store
        .blob_path(missing.content_hash.as_str())
        .expect("missing path");
    fs::remove_file(missing_path).expect("remove missing blob");

    let dangling_pin_hash = store.put_bytes(b"dangling-pin").expect("put dangling");
    store.pin(dangling_pin_hash.as_str()).expect("pin");
    let dangling_path = store
        .blob_path(dangling_pin_hash.as_str())
        .expect("dangling path");
    fs::remove_file(dangling_path).expect("remove dangling blob");

    let report = store.audit_file_index().expect("audit");
    assert_eq!(report.total_indexed_files, 2);
    assert_eq!(report.total_pins, 1);
    assert!(report
        .missing_file_blob_hashes
        .contains(&missing.content_hash));
    assert!(!report.missing_file_blob_hashes.contains(&live.content_hash));
    assert!(report.dangling_pin_hashes.contains(&dangling_pin_hash));
    assert!(report.orphan_blob_hashes.contains(&orphan_hash));
    assert!(!report.is_clean());

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn prune_orphan_blobs_removes_only_unreferenced_data() {
    let dir = temp_dir("prune-orphan");
    let store = LocalCasStore::new(&dir);

    let live = store
        .write_file("live.txt", b"live-data")
        .expect("write live");
    let orphan_hash = store.put_bytes(b"old-data").expect("write orphan");

    let pinned_hash = store.put_bytes(b"pinned-data").expect("put pinned");
    store.pin(pinned_hash.as_str()).expect("pin");

    let freed = store.prune_orphan_blobs().expect("prune orphan");
    assert!(freed > 0);
    assert!(store.has(live.content_hash.as_str()).expect("live exists"));
    assert!(store.has(pinned_hash.as_str()).expect("pinned exists"));
    assert!(!store.has(orphan_hash.as_str()).expect("orphan removed"));

    let _ = fs::remove_dir_all(&dir);
}
