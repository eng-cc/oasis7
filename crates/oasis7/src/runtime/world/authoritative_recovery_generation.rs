use super::*;

impl World {
    /// Durably commits runtime state and opaque Viewer recovery metadata in one generation.
    /// The sidecar generation index replacement is the commit point.
    pub fn save_authoritative_recovery_generation(
        &self,
        dir: impl AsRef<Path>,
        recovery_metadata: &[u8],
    ) -> Result<(), WorldError> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir)?;
        let snapshot = self.snapshot();
        self.save_distfs_sidecar(dir, &snapshot, Some(recovery_metadata))
    }

    pub fn load_authoritative_recovery_metadata(
        dir: impl AsRef<Path>,
    ) -> Result<Option<Vec<u8>>, WorldError> {
        let store_root = dir.as_ref().join(DISTFS_STATE_DIR);
        let Some(index) = persistence_support::load_sidecar_generation_index(store_root.as_path())?
        else {
            return Ok(None);
        };
        for generation_id in std::iter::once(index.latest_generation.as_str())
            .chain(index.rollback_safe_generation.as_deref())
        {
            let Some(record) = index.generations.get(generation_id) else {
                continue;
            };
            if persistence_support::validate_sidecar_generation_record(store_root.as_path(), record)
                .is_err()
            {
                continue;
            }
            return record
                .recovery_metadata_path
                .as_deref()
                .map(|relative| fs::read(store_root.join(relative)).map_err(WorldError::from))
                .transpose();
        }
        Err(WorldError::DistributedValidationFailed {
            reason: "no valid committed sidecar generation is available".to_string(),
        })
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
        world
            .save_authoritative_recovery_generation(&dir, b"receipt-new")
            .expect_err("pre-commit failure");
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
        world
            .save_authoritative_recovery_generation(&dir, b"receipt-new")
            .expect_err("post-commit injected failure");
        fs::remove_file(marker).expect("remove failpoint");
        assert_eq!(
            World::load_authoritative_recovery_metadata(&dir).expect("load committed metadata"),
            Some(b"receipt-new".to_vec())
        );
        let restored = World::load_from_dir(&dir).expect("restore committed runtime generation");
        assert_eq!(restored.snapshot(), world.snapshot());
        let _ = fs::remove_dir_all(dir);
    }
}
