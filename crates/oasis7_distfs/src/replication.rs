use serde::{Deserialize, Serialize};

use crate::{FileMetadata, FileStore, blake3_hex};
use oasis7_proto::world_error::WorldError;

fn default_writer_epoch() -> u64 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileReplicationRecord {
    pub world_id: String,
    pub writer_id: String,
    #[serde(default = "default_writer_epoch")]
    pub writer_epoch: u64,
    pub sequence: u64,
    pub path: String,
    pub content_hash: String,
    pub size_bytes: u64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SingleWriterReplicationGuard {
    pub writer_id: Option<String>,
    #[serde(default = "default_writer_epoch")]
    pub writer_epoch: u64,
    pub last_sequence: u64,
}

impl Default for SingleWriterReplicationGuard {
    fn default() -> Self {
        Self {
            writer_id: None,
            writer_epoch: default_writer_epoch(),
            last_sequence: 0,
        }
    }
}

impl SingleWriterReplicationGuard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn validate_and_advance(
        &mut self,
        record: &FileReplicationRecord,
    ) -> Result<(), WorldError> {
        if record.world_id.trim().is_empty() {
            return Err(WorldError::DistributedValidationFailed {
                reason: "replication record world_id cannot be empty".to_string(),
            });
        }
        if record.writer_id.trim().is_empty() {
            return Err(WorldError::DistributedValidationFailed {
                reason: "replication record writer_id cannot be empty".to_string(),
            });
        }
        if record.sequence == 0 {
            return Err(WorldError::DistributedValidationFailed {
                reason: "replication record sequence must be >= 1".to_string(),
            });
        }
        if record.writer_epoch == 0 {
            return Err(WorldError::DistributedValidationFailed {
                reason: "replication record writer_epoch must be >= 1".to_string(),
            });
        }

        if let Some(existing_writer) = &self.writer_id {
            if existing_writer == &record.writer_id {
                if record.writer_epoch < self.writer_epoch {
                    return Err(WorldError::DistributedValidationFailed {
                        reason: format!(
                            "replication writer_epoch rolled back for writer {}: last={}, got={}",
                            existing_writer, self.writer_epoch, record.writer_epoch
                        ),
                    });
                }
                if record.writer_epoch == self.writer_epoch {
                    if record.sequence <= self.last_sequence {
                        return Err(WorldError::DistributedValidationFailed {
                            reason: format!(
                                "replication sequence not monotonic: last={}, got={}",
                                self.last_sequence, record.sequence
                            ),
                        });
                    }
                } else if record.sequence != 1 {
                    return Err(WorldError::DistributedValidationFailed {
                        reason: format!(
                            "replication sequence must reset to 1 when writer_epoch advances for writer {}: epoch {} -> {}, got sequence {}",
                            existing_writer,
                            self.writer_epoch,
                            record.writer_epoch,
                            record.sequence
                        ),
                    });
                }
            } else {
                if record.writer_epoch <= self.writer_epoch {
                    return Err(WorldError::DistributedValidationFailed {
                        reason: format!(
                            "replication writer conflict requires writer_epoch advance: expected writer={} epoch={}, got writer={} epoch={}",
                            existing_writer,
                            self.writer_epoch,
                            record.writer_id,
                            record.writer_epoch
                        ),
                    });
                }
                if record.sequence != 1 {
                    return Err(WorldError::DistributedValidationFailed {
                        reason: format!(
                            "replication writer switch must start at sequence 1: writer={} epoch={} sequence={}",
                            record.writer_id, record.writer_epoch, record.sequence
                        ),
                    });
                }
            }
        }

        self.writer_id = Some(record.writer_id.clone());
        self.writer_epoch = record.writer_epoch;
        self.last_sequence = record.sequence;
        Ok(())
    }
}

pub fn build_replication_record_with_epoch(
    world_id: &str,
    writer_id: &str,
    writer_epoch: u64,
    sequence: u64,
    path: &str,
    bytes: &[u8],
    updated_at_ms: i64,
) -> Result<FileReplicationRecord, WorldError> {
    if world_id.trim().is_empty() {
        return Err(WorldError::DistributedValidationFailed {
            reason: "world_id cannot be empty".to_string(),
        });
    }
    if writer_id.trim().is_empty() {
        return Err(WorldError::DistributedValidationFailed {
            reason: "writer_id cannot be empty".to_string(),
        });
    }
    if writer_epoch == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "writer_epoch must be >= 1".to_string(),
        });
    }
    if sequence == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "sequence must be >= 1".to_string(),
        });
    }
    Ok(FileReplicationRecord {
        world_id: world_id.to_string(),
        writer_id: writer_id.to_string(),
        writer_epoch,
        sequence,
        path: path.to_string(),
        content_hash: blake3_hex(bytes),
        size_bytes: bytes.len() as u64,
        updated_at_ms,
    })
}

pub fn build_replication_record(
    world_id: &str,
    writer_id: &str,
    sequence: u64,
    path: &str,
    bytes: &[u8],
    updated_at_ms: i64,
) -> Result<FileReplicationRecord, WorldError> {
    if world_id.trim().is_empty() {
        return Err(WorldError::DistributedValidationFailed {
            reason: "world_id cannot be empty".to_string(),
        });
    }
    if writer_id.trim().is_empty() {
        return Err(WorldError::DistributedValidationFailed {
            reason: "writer_id cannot be empty".to_string(),
        });
    }
    build_replication_record_with_epoch(
        world_id,
        writer_id,
        default_writer_epoch(),
        sequence,
        path,
        bytes,
        updated_at_ms,
    )
}

pub fn apply_replication_record(
    store: &impl FileStore,
    guard: &mut SingleWriterReplicationGuard,
    record: &FileReplicationRecord,
    bytes: &[u8],
) -> Result<FileMetadata, WorldError> {
    let mut next_guard = guard.clone();
    next_guard.validate_and_advance(record)?;

    let computed_hash = blake3_hex(bytes);
    if computed_hash != record.content_hash {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "replication content hash mismatch: expected={}, got={}",
                record.content_hash, computed_hash
            ),
        });
    }

    let metadata = store.write_file(record.path.as_str(), bytes)?;
    if metadata.content_hash != record.content_hash {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "replication write hash mismatch: expected={}, got={}",
                record.content_hash, metadata.content_hash
            ),
        });
    }
    *guard = next_guard;
    Ok(metadata)
}

pub fn replay_replication_records(
    store: &impl FileStore,
    guard: &mut SingleWriterReplicationGuard,
    entries: &[(FileReplicationRecord, Vec<u8>)],
) -> Result<Vec<FileMetadata>, WorldError> {
    let mut applied = Vec::with_capacity(entries.len());
    for (record, bytes) in entries {
        let metadata = apply_replication_record(store, guard, record, bytes)?;
        applied.push(metadata);
    }
    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalCasStore;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("duration")
            .as_nanos();
        std::env::temp_dir().join(format!("oasis7-distfs-replication-{prefix}-{unique}"))
    }

    #[test]
    fn apply_replication_record_accepts_single_writer_monotonic_sequence() {
        let dir = temp_dir("single-writer");
        let store = LocalCasStore::new(&dir);
        let mut guard = SingleWriterReplicationGuard::new();

        let record = build_replication_record("w1", "writer-a", 1, "a/file.txt", b"hello", 10)
            .expect("record");
        let meta = apply_replication_record(&store, &mut guard, &record, b"hello").expect("apply");

        assert_eq!(meta.path, "a/file.txt");
        assert_eq!(guard.writer_id.as_deref(), Some("writer-a"));
        assert_eq!(guard.writer_epoch, 1);
        assert_eq!(guard.last_sequence, 1);
        assert_eq!(store.read_file("a/file.txt").expect("read"), b"hello");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_replication_record_rejects_writer_conflict() {
        let dir = temp_dir("writer-conflict");
        let store = LocalCasStore::new(&dir);
        let mut guard = SingleWriterReplicationGuard::new();

        let first =
            build_replication_record("w1", "writer-a", 1, "a/file.txt", b"v1", 10).expect("first");
        apply_replication_record(&store, &mut guard, &first, b"v1").expect("apply first");

        let conflict = build_replication_record("w1", "writer-b", 2, "a/file.txt", b"v2", 20)
            .expect("conflict");
        let result = apply_replication_record(&store, &mut guard, &conflict, b"v2");
        assert!(matches!(
            result,
            Err(WorldError::DistributedValidationFailed { .. })
        ));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_replication_record_accepts_writer_failover_with_epoch_bump() {
        let dir = temp_dir("writer-failover");
        let store = LocalCasStore::new(&dir);
        let mut guard = SingleWriterReplicationGuard::new();

        let first =
            build_replication_record_with_epoch("w1", "writer-a", 1, 1, "a/file.txt", b"v1", 10)
                .expect("first");
        apply_replication_record(&store, &mut guard, &first, b"v1").expect("apply first");

        let rotated =
            build_replication_record_with_epoch("w1", "writer-b", 2, 1, "a/file.txt", b"v2", 20)
                .expect("rotated");
        apply_replication_record(&store, &mut guard, &rotated, b"v2").expect("apply rotated");

        assert_eq!(guard.writer_id.as_deref(), Some("writer-b"));
        assert_eq!(guard.writer_epoch, 2);
        assert_eq!(guard.last_sequence, 1);
        assert_eq!(store.read_file("a/file.txt").expect("read"), b"v2");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_replication_record_rejects_writer_failover_without_epoch_bump() {
        let dir = temp_dir("writer-failover-reject");
        let store = LocalCasStore::new(&dir);
        let mut guard = SingleWriterReplicationGuard::new();

        let first =
            build_replication_record_with_epoch("w1", "writer-a", 2, 3, "a/file.txt", b"v1", 10)
                .expect("first");
        apply_replication_record(&store, &mut guard, &first, b"v1").expect("apply first");

        let conflict =
            build_replication_record_with_epoch("w1", "writer-b", 2, 1, "a/file2.txt", b"v2", 20)
                .expect("conflict");
        let err = apply_replication_record(&store, &mut guard, &conflict, b"v2")
            .expect_err("writer switch without epoch bump must fail");
        assert!(matches!(
            err,
            WorldError::DistributedValidationFailed { .. }
        ));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_replication_record_accepts_same_writer_epoch_advance_with_sequence_reset() {
        let dir = temp_dir("same-writer-epoch-advance");
        let store = LocalCasStore::new(&dir);
        let mut guard = SingleWriterReplicationGuard::new();

        let first =
            build_replication_record_with_epoch("w1", "writer-a", 1, 7, "a/file.txt", b"v1", 10)
                .expect("first");
        apply_replication_record(&store, &mut guard, &first, b"v1").expect("apply first");

        let advanced =
            build_replication_record_with_epoch("w1", "writer-a", 2, 1, "a/file.txt", b"v2", 20)
                .expect("advanced");
        apply_replication_record(&store, &mut guard, &advanced, b"v2").expect("apply advanced");

        assert_eq!(guard.writer_id.as_deref(), Some("writer-a"));
        assert_eq!(guard.writer_epoch, 2);
        assert_eq!(guard.last_sequence, 1);
        assert_eq!(store.read_file("a/file.txt").expect("read"), b"v2");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_replication_record_rejects_same_writer_epoch_advance_without_sequence_reset() {
        let dir = temp_dir("same-writer-epoch-advance-reject");
        let store = LocalCasStore::new(&dir);
        let mut guard = SingleWriterReplicationGuard::new();

        let first =
            build_replication_record_with_epoch("w1", "writer-a", 1, 2, "a/file.txt", b"v1", 10)
                .expect("first");
        apply_replication_record(&store, &mut guard, &first, b"v1").expect("apply first");

        let invalid =
            build_replication_record_with_epoch("w1", "writer-a", 2, 2, "a/file.txt", b"v2", 20)
                .expect("invalid");
        let err = apply_replication_record(&store, &mut guard, &invalid, b"v2")
            .expect_err("epoch advance without sequence reset must fail");
        assert!(matches!(
            err,
            WorldError::DistributedValidationFailed { .. }
        ));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_replication_record_rejects_non_monotonic_sequence() {
        let dir = temp_dir("sequence");
        let store = LocalCasStore::new(&dir);
        let mut guard = SingleWriterReplicationGuard::new();

        let first =
            build_replication_record("w1", "writer-a", 2, "a/file.txt", b"v1", 10).expect("first");
        apply_replication_record(&store, &mut guard, &first, b"v1").expect("apply first");

        let stale =
            build_replication_record("w1", "writer-a", 2, "a/file2.txt", b"v2", 20).expect("stale");
        let result = apply_replication_record(&store, &mut guard, &stale, b"v2");
        assert!(matches!(
            result,
            Err(WorldError::DistributedValidationFailed { .. })
        ));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replication_record_and_guard_compat_json_default_writer_epoch_to_one() {
        let compat_record_json = serde_json::json!({
            "world_id": "w1",
            "writer_id": "writer-a",
            "sequence": 3,
            "path": "legacy/file.txt",
            "content_hash": "abcd",
            "size_bytes": 4,
            "updated_at_ms": 123
        });
        let parsed_record: FileReplicationRecord =
            serde_json::from_value(compat_record_json).expect("parse compat record");
        assert_eq!(parsed_record.writer_epoch, 1);

        let compat_guard_json = serde_json::json!({
            "writer_id": "writer-a",
            "last_sequence": 9
        });
        let parsed_guard: SingleWriterReplicationGuard =
            serde_json::from_value(compat_guard_json).expect("parse compat guard");
        assert_eq!(parsed_guard.writer_epoch, 1);
    }

    #[test]
    fn replay_replication_records_restores_files_in_order() {
        let dir = temp_dir("replay");
        let store = LocalCasStore::new(&dir);
        let mut guard = SingleWriterReplicationGuard::new();

        let entries = vec![
            (
                build_replication_record("w1", "writer-a", 1, "docs/a.txt", b"A", 10).expect("r1"),
                b"A".to_vec(),
            ),
            (
                build_replication_record("w1", "writer-a", 2, "docs/b.txt", b"B", 20).expect("r2"),
                b"B".to_vec(),
            ),
        ];

        let applied = replay_replication_records(&store, &mut guard, &entries).expect("replay");
        assert_eq!(applied.len(), 2);
        assert_eq!(guard.last_sequence, 2);
        assert_eq!(store.read_file("docs/a.txt").expect("read a"), b"A");
        assert_eq!(store.read_file("docs/b.txt").expect("read b"), b"B");

        let _ = fs::remove_dir_all(&dir);
    }

    struct AlwaysFailWriteStore;

    impl FileStore for AlwaysFailWriteStore {
        fn write_file(&self, _path: &str, _bytes: &[u8]) -> Result<FileMetadata, WorldError> {
            Err(WorldError::DistributedValidationFailed {
                reason: "forced write failure".to_string(),
            })
        }

        fn read_file(&self, _path: &str) -> Result<Vec<u8>, WorldError> {
            Err(WorldError::DistributedValidationFailed {
                reason: "not implemented".to_string(),
            })
        }

        fn delete_file(&self, _path: &str) -> Result<bool, WorldError> {
            Err(WorldError::DistributedValidationFailed {
                reason: "not implemented".to_string(),
            })
        }

        fn stat_file(&self, _path: &str) -> Result<Option<FileMetadata>, WorldError> {
            Err(WorldError::DistributedValidationFailed {
                reason: "not implemented".to_string(),
            })
        }

        fn list_files(&self) -> Result<Vec<FileMetadata>, WorldError> {
            Err(WorldError::DistributedValidationFailed {
                reason: "not implemented".to_string(),
            })
        }
    }

    #[test]
    fn apply_replication_record_hash_mismatch_does_not_mutate_guard() {
        let dir = temp_dir("guard-rollback-hash");
        let store = LocalCasStore::new(&dir);
        let original_guard = SingleWriterReplicationGuard {
            writer_id: Some("writer-a".to_string()),
            writer_epoch: 3,
            last_sequence: 7,
        };
        let mut guard = original_guard.clone();
        let record =
            build_replication_record_with_epoch("w1", "writer-a", 3, 8, "a/file.txt", b"good", 10)
                .expect("record");

        let err = apply_replication_record(&store, &mut guard, &record, b"tampered")
            .expect_err("hash mismatch must fail");
        assert!(matches!(
            err,
            WorldError::DistributedValidationFailed { .. }
        ));
        assert_eq!(guard, original_guard);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn apply_replication_record_write_failure_does_not_mutate_guard() {
        let store = AlwaysFailWriteStore;
        let original_guard = SingleWriterReplicationGuard::new();
        let mut guard = original_guard.clone();
        let record =
            build_replication_record_with_epoch("w1", "writer-a", 1, 1, "a/file.txt", b"good", 10)
                .expect("record");

        let err = apply_replication_record(&store, &mut guard, &record, b"good")
            .expect_err("write failure must fail");
        assert!(matches!(
            err,
            WorldError::DistributedValidationFailed { reason } if reason.contains("forced write failure")
        ));
        assert_eq!(guard, original_guard);
    }
}
