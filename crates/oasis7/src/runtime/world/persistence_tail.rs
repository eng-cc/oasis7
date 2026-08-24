use super::*;

impl World {
    pub(super) fn try_load_from_distfs_sidecar(
        dir: &Path,
    ) -> Result<Option<(Snapshot, Journal)>, WorldError> {
        let snapshot_manifest_path = dir.join(DISTFS_SNAPSHOT_MANIFEST_FILE);
        let journal_segments_path = dir.join(DISTFS_JOURNAL_SEGMENTS_FILE);
        let store_root = dir.join(DISTFS_STATE_DIR);
        if store_root.exists()
            && let Some(index) = persistence_support::load_sidecar_generation_index(&store_root)?
        {
            for generation_id in std::iter::once(index.latest_generation.as_str())
                .chain(index.rollback_safe_generation.as_deref())
            {
                let Some(record) = index.generations.get(generation_id) else {
                    continue;
                };
                if persistence_support::validate_sidecar_generation_record(&store_root, record)
                    .is_err()
                {
                    continue;
                }
                let (manifest, journal_segments) =
                    persistence_support::read_sidecar_generation_payloads(&store_root, record)?;
                if let Err(err) = validate_compatible_legacy_distfs_payloads(
                    dir,
                    &manifest,
                    journal_segments.as_slice(),
                ) {
                    let _ = write_distfs_recovery_audit(
                        dir,
                        "fallback_json",
                        Some(format!("distfs_restore_failed: {:?}", err)),
                    );
                    return Ok(None);
                }
                let store = LocalCasStore::new(&store_root);
                let mut snapshot: Snapshot = assemble_snapshot(&manifest, &store)?;
                if let Some(archive) =
                    persistence_support::load_sidecar_tick_consensus_archive(&store_root, record)?
                {
                    hydrate_tick_consensus_snapshot_from_archived_records(
                        &mut snapshot,
                        archive.archived_records,
                    )?;
                }
                let events: Vec<WorldEvent> =
                    assemble_journal(&journal_segments, &store, |event: &WorldEvent| event.id)?;
                let _ = write_distfs_recovery_audit(
                    dir,
                    if record.recovery_metadata_path.is_some() {
                        if generation_id == index.latest_generation {
                            "generation_restored"
                        } else {
                            "rollback_safe_generation_restored"
                        }
                    } else {
                        "distfs_restored"
                    },
                    None,
                );
                return Ok(Some((snapshot, Journal { events })));
            }
            return Err(WorldError::DistributedValidationFailed {
                reason: "sidecar generation index has no valid latest or rollback-safe generation"
                    .to_string(),
            });
        }
        if !snapshot_manifest_path.exists()
            || !journal_segments_path.exists()
            || !store_root.exists()
        {
            return Ok(None);
        }

        let restored = Self::load_from_distfs_sidecar(
            snapshot_manifest_path.as_path(),
            journal_segments_path.as_path(),
            store_root.as_path(),
        );
        match restored {
            Ok(value) => {
                let _ = write_distfs_recovery_audit(dir, "distfs_restored", None);
                Ok(Some(value))
            }
            Err(err) => {
                let _ = write_distfs_recovery_audit(
                    dir,
                    "fallback_json",
                    Some(format!("distfs_restore_failed: {:?}", err)),
                );
                Ok(None)
            }
        }
    }

    pub(super) fn has_authoritative_recovery_generation(dir: &Path) -> Result<bool, WorldError> {
        let store_root = dir.join(DISTFS_STATE_DIR);
        Ok(
            persistence_support::load_sidecar_generation_index(&store_root)?
                .and_then(|index| {
                    index
                        .generations
                        .get(index.latest_generation.as_str())
                        .cloned()
                })
                .is_some_and(|record| record.recovery_metadata_path.is_some()),
        )
    }

    pub(super) fn load_from_distfs_sidecar(
        snapshot_manifest_path: &Path,
        journal_segments_path: &Path,
        store_root: &Path,
    ) -> Result<(Snapshot, Journal), WorldError> {
        let manifest: SnapshotManifest = read_json_from_path(snapshot_manifest_path)?;
        let journal_segments: Vec<JournalSegmentRef> = read_json_from_path(journal_segments_path)?;
        let store = LocalCasStore::new(store_root);
        let snapshot: Snapshot = assemble_snapshot(&manifest, &store)?;
        let events: Vec<WorldEvent> =
            assemble_journal(&journal_segments, &store, |event: &WorldEvent| event.id)?;
        Ok((snapshot, Journal { events }))
    }
}
