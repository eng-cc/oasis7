use super::*;

pub struct CommittedAuthoritativeRecoveryGeneration {
    pub generation_id: String,
    pub world: World,
    pub recovery_metadata: Vec<u8>,
}

pub enum AuthoritativeRecoveryCommitStatus {
    Committed(CommittedAuthoritativeRecoveryGeneration),
    NotCommitted {
        current_generation_id: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthoritativeRecoveryCommitError {
    StatusUnknown { reason: String },
}

impl World {
    pub fn load_authoritative_recovery_generation(
        dir: impl AsRef<Path>,
    ) -> Result<Option<CommittedAuthoritativeRecoveryGeneration>, WorldError> {
        let dir = dir.as_ref();
        let store_root = dir.join(DISTFS_STATE_DIR);
        let Some(index) = persistence_support::load_sidecar_generation_index(&store_root)? else {
            return Ok(None);
        };
        let mut saw_authoritative_generation = false;
        for generation_id in std::iter::once(index.latest_generation.as_str())
            .chain(index.rollback_safe_generation.as_deref())
        {
            let Some(record) = index.generations.get(generation_id) else {
                continue;
            };
            if record.recovery_metadata_path.is_none() && record.recovery_metadata_hash.is_none() {
                continue;
            }
            saw_authoritative_generation = true;
            if persistence_support::validate_sidecar_generation_record(&store_root, record).is_err()
            {
                continue;
            }
            let metadata_path = record.recovery_metadata_path.as_deref().ok_or_else(|| {
                WorldError::DistributedValidationFailed {
                    reason: format!(
                        "authoritative recovery generation has no metadata: generation_id={generation_id}"
                    ),
                }
            })?;
            let recovery_metadata = fs::read(store_root.join(metadata_path))?;
            let (manifest, journal_segments) =
                persistence_support::read_sidecar_generation_payloads(&store_root, record)?;
            let store = LocalCasStore::new(&store_root);
            let snapshot: Snapshot = assemble_snapshot(&manifest, &store)?;
            let events: Vec<WorldEvent> =
                assemble_journal(&journal_segments, &store, |event: &WorldEvent| event.id)?;
            let world = World::from_snapshot(snapshot, Journal { events })?;
            return Ok(Some(CommittedAuthoritativeRecoveryGeneration {
                generation_id: generation_id.to_string(),
                world,
                recovery_metadata,
            }));
        }
        if saw_authoritative_generation {
            Err(WorldError::DistributedValidationFailed {
                reason: "no valid committed authoritative recovery generation is available"
                    .to_string(),
            })
        } else {
            Ok(None)
        }
    }

    /// Durably commits runtime state and opaque Viewer recovery metadata in one generation.
    /// The sidecar generation index replacement is the commit point.
    pub fn save_authoritative_recovery_generation(
        &self,
        dir: impl AsRef<Path>,
        recovery_metadata: &[u8],
    ) -> Result<(), WorldError> {
        match self.commit_authoritative_recovery_generation(dir, recovery_metadata) {
            Ok(AuthoritativeRecoveryCommitStatus::Committed(_)) => Ok(()),
            Ok(AuthoritativeRecoveryCommitStatus::NotCommitted {
                current_generation_id,
            }) => Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "authoritative recovery generation was not committed; current_generation_id={current_generation_id:?}"
                ),
            }),
            Err(AuthoritativeRecoveryCommitError::StatusUnknown { reason }) => {
                Err(WorldError::DistributedValidationFailed {
                    reason: format!("authoritative recovery commit status unknown: {reason}"),
                })
            }
        }
    }

    pub fn commit_authoritative_recovery_generation(
        &self,
        dir: impl AsRef<Path>,
        recovery_metadata: &[u8],
    ) -> Result<AuthoritativeRecoveryCommitStatus, AuthoritativeRecoveryCommitError> {
        let dir = dir.as_ref();
        let expected_metadata_hash = super::super::super::util::sha256_hex(recovery_metadata);
        let snapshot = self.snapshot();
        let write_result = fs::create_dir_all(dir)
            .map_err(WorldError::from)
            .and_then(|_| self.save_distfs_sidecar(dir, &snapshot, Some(recovery_metadata)));
        let readback =
            Self::readback_authoritative_recovery_generation(dir, expected_metadata_hash.as_str());
        match readback {
            Ok(status) => Ok(status),
            Err(error) => {
                let _ = write_result;
                Err(error)
            }
        }
    }

    pub fn readback_authoritative_recovery_generation(
        dir: impl AsRef<Path>,
        expected_metadata_hash: &str,
    ) -> Result<AuthoritativeRecoveryCommitStatus, AuthoritativeRecoveryCommitError> {
        let dir = dir.as_ref();
        let store_root = dir.join(DISTFS_STATE_DIR);
        let index = persistence_support::load_sidecar_generation_index(store_root.as_path())
            .map_err(|err| AuthoritativeRecoveryCommitError::StatusUnknown {
                reason: format!("authoritative recovery index read failed: {err:?}"),
            })?;
        let Some(index) = index else {
            return Ok(AuthoritativeRecoveryCommitStatus::NotCommitted {
                current_generation_id: None,
            });
        };
        let record = index
            .generations
            .get(index.latest_generation.as_str())
            .ok_or_else(|| AuthoritativeRecoveryCommitError::StatusUnknown {
                reason: format!(
                    "authoritative recovery index latest generation is missing: {}",
                    index.latest_generation
                ),
            })?;
        persistence_support::validate_sidecar_generation_record(store_root.as_path(), record)
            .map_err(|err| AuthoritativeRecoveryCommitError::StatusUnknown {
                reason: format!(
                    "authoritative recovery latest generation failed validation: generation_id={} err={err:?}",
                    record.generation_id
                ),
            })?;
        if record.recovery_metadata_hash.as_deref() != Some(expected_metadata_hash) {
            return Ok(AuthoritativeRecoveryCommitStatus::NotCommitted {
                current_generation_id: Some(record.generation_id.clone()),
            });
        }
        let metadata_path = record.recovery_metadata_path.as_deref().ok_or_else(|| {
            AuthoritativeRecoveryCommitError::StatusUnknown {
                reason: format!(
                    "matching recovery generation has no metadata path: generation_id={}",
                    record.generation_id
                ),
            }
        })?;
        let recovery_metadata = fs::read(store_root.join(metadata_path)).map_err(|err| {
            AuthoritativeRecoveryCommitError::StatusUnknown {
                reason: format!(
                    "matching recovery metadata read failed: generation_id={} err={err}",
                    record.generation_id
                ),
            }
        })?;
        let (manifest, journal_segments) = persistence_support::read_sidecar_generation_payloads(
            store_root.as_path(),
            record,
        )
        .map_err(|err| AuthoritativeRecoveryCommitError::StatusUnknown {
            reason: format!(
                "matching recovery generation payload read failed: generation_id={} err={err:?}",
                record.generation_id
            ),
        })?;
        let store = LocalCasStore::new(store_root.as_path());
        let snapshot: Snapshot = assemble_snapshot(&manifest, &store).map_err(|err| {
            AuthoritativeRecoveryCommitError::StatusUnknown {
                reason: format!(
                    "matching recovery snapshot assemble failed: generation_id={} err={err:?}",
                    record.generation_id
                ),
            }
        })?;
        let events: Vec<WorldEvent> =
            assemble_journal(journal_segments.as_slice(), &store, |event: &WorldEvent| {
                event.id
            })
            .map_err(|err| AuthoritativeRecoveryCommitError::StatusUnknown {
                reason: format!(
                    "matching recovery journal assemble failed: generation_id={} err={err:?}",
                    record.generation_id
                ),
            })?;
        let world = World::from_snapshot(snapshot, Journal { events }).map_err(|err| {
            AuthoritativeRecoveryCommitError::StatusUnknown {
                reason: format!(
                    "matching recovery world restore failed: generation_id={} err={err:?}",
                    record.generation_id
                ),
            }
        })?;
        Ok(AuthoritativeRecoveryCommitStatus::Committed(
            CommittedAuthoritativeRecoveryGeneration {
                generation_id: record.generation_id.clone(),
                world,
                recovery_metadata,
            },
        ))
    }

    pub fn load_authoritative_recovery_metadata(
        dir: impl AsRef<Path>,
    ) -> Result<Option<Vec<u8>>, WorldError> {
        Ok(Self::load_authoritative_recovery_generation(dir)?
            .map(|generation| generation.recovery_metadata))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_world_dir(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "oasis7-{label}-{}-{}",
            std::process::id(),
            now_unix_ms()
        ));
        fs::create_dir_all(&path).expect("create temp world dir");
        path
    }

    #[test]
    fn recovery_generation_pre_commit_failure_keeps_previous_metadata() {
        let dir = temp_world_dir("recovery-generation-pre-commit");
        let world = World::new();
        world
            .save_authoritative_recovery_generation(&dir, b"receipt-old")
            .expect("commit initial generation");
        let marker = dir
            .join(DISTFS_STATE_DIR)
            .join(SIDECAR_GENERATION_ROOT_DIR)
            .join(".test-fail-before-index-commit");
        fs::write(&marker, b"fail").expect("install failpoint");
        let status = world
            .commit_authoritative_recovery_generation(&dir, b"receipt-new")
            .expect("pre-commit write failure must resolve through readback");
        assert!(matches!(
            status,
            AuthoritativeRecoveryCommitStatus::NotCommitted {
                current_generation_id: Some(_)
            }
        ));
        fs::remove_file(marker).expect("remove failpoint");
        assert_eq!(
            World::load_authoritative_recovery_metadata(&dir).expect("load committed metadata"),
            Some(b"receipt-old".to_vec())
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn recovery_generation_post_commit_failure_recovers_new_metadata() {
        let dir = temp_world_dir("recovery-generation-post-commit");
        let world = World::new();
        world
            .save_authoritative_recovery_generation(&dir, b"receipt-old")
            .expect("commit initial generation");
        let marker = dir
            .join(DISTFS_STATE_DIR)
            .join(SIDECAR_GENERATION_ROOT_DIR)
            .join(".test-fail-after-index-commit");
        fs::write(&marker, b"fail").expect("install failpoint");
        let status = world
            .commit_authoritative_recovery_generation(&dir, b"receipt-new")
            .expect("post-commit write failure must resolve as committed");
        let AuthoritativeRecoveryCommitStatus::Committed(committed) = status else {
            panic!("post-index readback must classify the new generation as committed");
        };
        assert!(!committed.generation_id.is_empty());
        assert_eq!(committed.recovery_metadata, b"receipt-new");
        assert_eq!(committed.world.snapshot(), world.snapshot());
        fs::remove_file(marker).expect("remove failpoint");
        assert_eq!(
            World::load_authoritative_recovery_metadata(&dir).expect("load committed metadata"),
            Some(b"receipt-new".to_vec())
        );
        let restored = World::load_from_dir(&dir).expect("restore committed runtime generation");
        assert_eq!(restored.snapshot(), world.snapshot());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn authoritative_recovery_loader_returns_world_and_metadata_from_one_generation() {
        let dir = temp_world_dir("recovery-generation-atomic-loader");
        let mut world = World::new();
        world
            .save_authoritative_recovery_generation(&dir, b"receipt-old")
            .expect("commit initial generation");
        world.step().expect("advance replacement generation");
        world
            .save_authoritative_recovery_generation(&dir, b"receipt-new")
            .expect("commit replacement generation");

        let loaded = World::load_authoritative_recovery_generation(&dir)
            .expect("load one committed generation")
            .expect("committed generation");
        assert_eq!(loaded.recovery_metadata, b"receipt-new");
        assert_eq!(loaded.world.snapshot(), world.snapshot());
        assert_eq!(loaded.world.journal(), world.journal());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn recovery_generation_unreadable_index_reports_status_unknown() {
        let dir = temp_world_dir("recovery-generation-status-unknown");
        let world = World::new();
        world
            .save_authoritative_recovery_generation(&dir, b"receipt-old")
            .expect("commit initial generation");
        let index_path = dir
            .join(DISTFS_STATE_DIR)
            .join(SIDECAR_GENERATION_ROOT_DIR)
            .join(SIDECAR_GENERATION_INDEX_FILE);
        fs::write(index_path, b"not-json").expect("corrupt generation index");

        let result = World::readback_authoritative_recovery_generation(
            &dir,
            super::super::super::super::util::sha256_hex(b"receipt-new").as_str(),
        );
        let Err(error) = result else {
            panic!("unreadable authoritative index must not be classified as not committed");
        };
        assert!(matches!(
            error,
            AuthoritativeRecoveryCommitError::StatusUnknown { .. }
        ));
        let _ = fs::remove_dir_all(dir);
    }
}
