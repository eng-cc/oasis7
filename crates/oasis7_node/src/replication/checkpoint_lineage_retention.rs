use super::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const RETENTION_SCHEMA_VERSION: u8 = 1;
const RETENTION_JOURNAL_FILE: &str = "retention.v1.journal.json";
const RETENTION_HEALTH_FILE: &str = "checkpoint-lineage/health.v1.json";
const RETENTION_PHASE_PREPARED: &str = "prepared";
const RETENTION_PHASE_RENAMED: &str = "renamed";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RetentionJournalV1 {
    schema_version: u8,
    generation: u128,
    phase: String,
    staging_dir: String,
    entries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RetentionHealthV1 {
    schema_version: u8,
    generation: u128,
    status: String,
    retained_heights: BTreeMap<String, Vec<u64>>,
    source_files: usize,
    envelope_files: usize,
}

#[derive(Debug, Clone)]
struct SourceCacheEntry {
    path: PathBuf,
    world_id: String,
    height: u64,
}

#[derive(Debug, Clone)]
struct EnvelopeCacheEntry {
    path: PathBuf,
    world_id: String,
    height: u64,
}

impl ReplicationRuntime {
    pub(crate) fn ensure_checkpoint_lineage_healthy(&self) -> Result<(), NodeError> {
        let root = self.checkpoint_lineage_root();
        let journal = root.join(RETENTION_JOURNAL_FILE);
        if journal.exists() {
            return Err(NodeError::Replication {
                reason: "checkpoint lineage retention recovery_required: journal is present"
                    .to_string(),
            });
        }
        let health = self.config.root_dir.join(RETENTION_HEALTH_FILE);
        if !health.exists() {
            return Ok(());
        }
        let bytes = fs::read(&health).map_err(|err| NodeError::Replication {
            reason: format!(
                "read checkpoint lineage health {} failed: {err}",
                health.display()
            ),
        })?;
        let status = serde_json::from_slice::<RetentionHealthV1>(&bytes).map_err(|err| {
            NodeError::Replication {
                reason: format!(
                    "decode checkpoint lineage health {} failed: {err}",
                    health.display()
                ),
            }
        })?;
        if status.schema_version != RETENTION_SCHEMA_VERSION || status.status != "ready" {
            return Err(NodeError::Replication {
                reason: "checkpoint lineage retention recovery_required: health is not ready"
                    .to_string(),
            });
        }
        Ok(())
    }

    pub(crate) fn reconcile_checkpoint_lineage_retention(&self) -> Result<(), NodeError> {
        let root = self.checkpoint_lineage_root();
        fs::create_dir_all(&root).map_err(|err| NodeError::Replication {
            reason: format!(
                "create checkpoint lineage cache {} failed: {err}",
                root.display()
            ),
        })?;
        if root.join(RETENTION_JOURNAL_FILE).exists() {
            self.recover_checkpoint_lineage_retention(&root)?;
        }
        self.ensure_checkpoint_lineage_healthy()?;

        let (sources, envelopes) = match self.scan_checkpoint_lineage_cache(&root) {
            Ok(entries) => entries,
            Err(err) => {
                self.mark_checkpoint_lineage_recovery_required(&root, err.to_string());
                return Err(err);
            }
        };
        let retained =
            retained_checkpoint_heights(&sources, &envelopes, self.config.max_hot_commit_messages);
        let stale = stale_checkpoint_lineage_paths(&sources, &envelopes, &retained);
        let (retained_sources, retained_envelopes) =
            retained_sidecar_counts(&sources, &envelopes, &retained);
        let generation = retention_generation();
        if stale.is_empty() {
            let result = self.write_checkpoint_lineage_health(
                &root,
                generation,
                &retained,
                retained_sources,
                retained_envelopes,
            );
            if let Err(err) = result {
                self.mark_checkpoint_lineage_recovery_required(&root, err.to_string());
                return Err(err);
            }
            return Ok(());
        }

        let staging_name = format!(".retention-v1-{generation}");
        let staging = root.join(staging_name.as_str());
        if staging.exists() {
            let err = NodeError::Replication {
                reason: format!(
                    "checkpoint lineage retention staging already exists: {}",
                    staging.display()
                ),
            };
            self.mark_checkpoint_lineage_recovery_required(&root, err.to_string());
            return Err(err);
        }
        fs::create_dir(&staging).map_err(|err| {
            let failure = NodeError::Replication {
                reason: format!("create checkpoint lineage retention staging failed: {err}"),
            };
            self.mark_checkpoint_lineage_recovery_required(&root, failure.to_string());
            failure
        })?;
        let journal_path = root.join(RETENTION_JOURNAL_FILE);
        let names = stale
            .iter()
            .filter_map(|path| path.file_name()?.to_str().map(str::to_string))
            .collect::<Vec<_>>();
        let journal = RetentionJournalV1 {
            schema_version: RETENTION_SCHEMA_VERSION,
            generation,
            phase: RETENTION_PHASE_PREPARED.to_string(),
            staging_dir: staging_name,
            entries: names,
        };
        if let Err(err) = write_json_compact(&journal_path, &journal).and_then(|_| sync_dir(&root))
        {
            let _ = fs::remove_dir_all(&staging);
            self.mark_checkpoint_lineage_recovery_required(&root, err.to_string());
            return Err(err);
        }

        let mut moved = Vec::new();
        for path in &stale {
            let Some(name) = path.file_name() else {
                let err = NodeError::Replication {
                    reason: format!("invalid checkpoint lineage cache path: {}", path.display()),
                };
                self.restore_checkpoint_lineage_paths(&root, &staging, &moved);
                self.mark_checkpoint_lineage_recovery_required(&root, err.to_string());
                return Err(err);
            };
            let target = staging.join(name);
            if let Err(err) = fs::rename(path, &target) {
                let failure = NodeError::Replication {
                    reason: format!(
                        "stage checkpoint lineage cache {} failed: {err}",
                        path.display()
                    ),
                };
                self.restore_checkpoint_lineage_paths(&root, &staging, &moved);
                self.mark_checkpoint_lineage_recovery_required(&root, failure.to_string());
                return Err(failure);
            }
            moved.push(path.clone());
        }
        if let Err(err) = sync_dir(&root) {
            self.restore_checkpoint_lineage_paths(&root, &staging, &moved);
            self.mark_checkpoint_lineage_recovery_required(&root, err.to_string());
            return Err(err);
        }
        sync_dir(&staging)?;

        let renamed_journal = RetentionJournalV1 {
            phase: RETENTION_PHASE_RENAMED.to_string(),
            ..journal
        };
        if let Err(err) =
            write_json_compact(&journal_path, &renamed_journal).and_then(|_| sync_dir(&root))
        {
            self.mark_checkpoint_lineage_recovery_required(&root, err.to_string());
            return Err(err);
        }
        for path in &moved {
            let Some(name) = path.file_name() else {
                continue;
            };
            let staged = staging.join(name);
            if !staged.is_file() {
                let failure = NodeError::Replication {
                    reason: format!(
                        "checkpoint lineage retention verify missing: {}",
                        staged.display()
                    ),
                };
                self.mark_checkpoint_lineage_recovery_required(&root, failure.to_string());
                return Err(failure);
            }
            if let Err(err) = fs::remove_file(&staged) {
                let failure = NodeError::Replication {
                    reason: format!(
                        "remove staged checkpoint lineage cache {} failed: {err}",
                        staged.display()
                    ),
                };
                self.mark_checkpoint_lineage_recovery_required(&root, failure.to_string());
                return Err(failure);
            }
        }
        fs::remove_dir(&staging).map_err(|err| {
            let failure = NodeError::Replication {
                reason: format!("remove checkpoint lineage staging failed: {err}"),
            };
            self.mark_checkpoint_lineage_recovery_required(&root, failure.to_string());
            failure
        })?;
        sync_dir(&root)?;
        fs::remove_file(&journal_path).map_err(|err| {
            let failure = NodeError::Replication {
                reason: format!("remove checkpoint lineage retention journal failed: {err}"),
            };
            self.mark_checkpoint_lineage_recovery_required(&root, failure.to_string());
            failure
        })?;
        if let Err(err) = sync_dir(&root) {
            self.mark_checkpoint_lineage_recovery_required(&root, err.to_string());
            return Err(err);
        }
        self.write_checkpoint_lineage_health(
            &root,
            generation,
            &retained,
            retained_sources,
            retained_envelopes,
        )
        .map_err(|err| {
            self.mark_checkpoint_lineage_recovery_required(&root, err.to_string());
            err
        })?;
        Ok(())
    }

    fn checkpoint_lineage_root(&self) -> PathBuf {
        self.config.root_dir.join("checkpoint-lineage")
    }

    fn scan_checkpoint_lineage_cache(
        &self,
        root: &Path,
    ) -> Result<(Vec<SourceCacheEntry>, Vec<EnvelopeCacheEntry>), NodeError> {
        let mut sources = Vec::new();
        let mut envelopes = Vec::new();
        for entry in fs::read_dir(root).map_err(|err| NodeError::Replication {
            reason: format!(
                "read checkpoint lineage cache {} failed: {err}",
                root.display()
            ),
        })? {
            let entry = entry.map_err(|err| NodeError::Replication {
                reason: format!("read checkpoint lineage cache entry failed: {err}"),
            })?;
            let path = entry.path();
            let file_type = entry.file_type().map_err(|err| NodeError::Replication {
                reason: format!(
                    "stat checkpoint lineage cache {} failed: {err}",
                    path.display()
                ),
            })?;
            if file_type.is_dir() {
                if path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(".retention-v1-"))
                {
                    return Err(NodeError::Replication {
                        reason: format!(
                            "checkpoint lineage retention recovery_required: staging remains {}",
                            path.display()
                        ),
                    });
                }
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                return Err(NodeError::Replication {
                    reason: format!("invalid checkpoint lineage cache file: {}", path.display()),
                });
            };
            if name == RETENTION_JOURNAL_FILE || name == "health.v1.json" {
                continue;
            }
            if let Some(height) = parse_source_height(name) {
                if format!("source-{height}.json") != name {
                    return Err(NodeError::Replication {
                        reason: format!("non-canonical checkpoint lineage source key: {name}"),
                    });
                }
                let bytes = fs::read(&path).map_err(|err| NodeError::Replication {
                    reason: format!(
                        "read checkpoint lineage source {} failed: {err}",
                        path.display()
                    ),
                })?;
                let message =
                    serde_json::from_slice::<GossipReplicationMessage>(&bytes).map_err(|err| {
                        NodeError::Replication {
                            reason: format!(
                                "decode checkpoint lineage source {} failed: {err}",
                                path.display()
                            ),
                        }
                    })?;
                verify_replication_message_signature(&message)?;
                if oasis7_distfs::blake3_hex(message.payload.as_slice())
                    != message.record.content_hash
                    || message.world_id != message.record.world_id
                    || message.record.writer_id
                        != message.public_key_hex.clone().unwrap_or_default()
                {
                    return Err(NodeError::Replication {
                        reason: format!(
                            "checkpoint lineage source binding mismatch: {}",
                            path.display()
                        ),
                    });
                }
                let payload = serde_json::from_slice::<ReplicatedCommitPayload>(&message.payload)
                    .map_err(|err| NodeError::Replication {
                    reason: format!("decode checkpoint lineage source payload failed: {err}"),
                })?;
                if payload.world_id != message.world_id
                    || payload.node_id != message.node_id
                    || payload.height != height
                    || payload.execution_checkpoint.is_none()
                {
                    return Err(NodeError::Replication {
                        reason: format!(
                            "checkpoint lineage source height/world mismatch: {}",
                            path.display()
                        ),
                    });
                }
                sources.push(SourceCacheEntry {
                    path,
                    world_id: message.world_id,
                    height,
                });
                continue;
            }
            if name.ends_with(".json") {
                let key = name.trim_end_matches(".json");
                if key.len() != 64 || !key.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return Err(NodeError::Replication {
                        reason: format!("invalid checkpoint lineage cache key: {name}"),
                    });
                }
                let bytes = fs::read(&path).map_err(|err| NodeError::Replication {
                    reason: format!(
                        "read checkpoint lineage envelope {} failed: {err}",
                        path.display()
                    ),
                })?;
                let envelope = serde_json::from_slice::<CheckpointLineageEnvelopeV1>(&bytes)
                    .map_err(|err| NodeError::Replication {
                        reason: format!(
                            "decode checkpoint lineage envelope {} failed: {err}",
                            path.display()
                        ),
                    })?;
                envelope
                    .validate_contract()
                    .map_err(|reason| NodeError::Replication { reason })?;
                let computed = checkpoint_lineage_cache_key(&envelope)
                    .map_err(|reason| NodeError::Replication { reason })?;
                if computed != key {
                    return Err(NodeError::Replication {
                        reason: format!("checkpoint lineage envelope key mismatch: {name}"),
                    });
                }
                envelopes.push(EnvelopeCacheEntry {
                    path,
                    world_id: envelope.world_id,
                    height: envelope.checkpoint.height,
                });
                continue;
            }
            return Err(NodeError::Replication {
                reason: format!("unknown checkpoint lineage cache file: {name}"),
            });
        }
        Ok((sources, envelopes))
    }

    fn recover_checkpoint_lineage_retention(&self, root: &Path) -> Result<(), NodeError> {
        let journal_path = root.join(RETENTION_JOURNAL_FILE);
        let bytes = fs::read(&journal_path).map_err(|err| NodeError::Replication {
            reason: format!("read checkpoint lineage retention journal failed: {err}"),
        })?;
        let journal = serde_json::from_slice::<RetentionJournalV1>(&bytes).map_err(|err| {
            let failure = NodeError::Replication {
                reason: format!("decode checkpoint lineage retention journal failed: {err}"),
            };
            self.mark_checkpoint_lineage_recovery_required(root, failure.to_string());
            failure
        })?;
        if journal.schema_version != RETENTION_SCHEMA_VERSION
            || !matches!(
                journal.phase.as_str(),
                RETENTION_PHASE_PREPARED | RETENTION_PHASE_RENAMED
            )
            || !valid_cache_name(&journal.staging_dir)
            || journal.entries.iter().any(|name| !valid_cache_name(name))
        {
            let failure = NodeError::Replication {
                reason: "checkpoint lineage retention journal schema/phase mismatch".to_string(),
            };
            self.mark_checkpoint_lineage_recovery_required(root, failure.to_string());
            return Err(failure);
        }
        let staging = root.join(journal.staging_dir.as_str());
        let mut restored = Vec::new();
        for name in &journal.entries {
            let target = root.join(name);
            let staged = staging.join(name);
            if staged.exists() {
                if target.exists() {
                    let failure = NodeError::Replication {
                        reason: format!("checkpoint lineage retention recovery conflict: {name}"),
                    };
                    self.mark_checkpoint_lineage_recovery_required(root, failure.to_string());
                    return Err(failure);
                }
                fs::rename(&staged, &target).map_err(|err| {
                    let failure = NodeError::Replication {
                        reason: format!("restore checkpoint lineage cache {name} failed: {err}"),
                    };
                    self.mark_checkpoint_lineage_recovery_required(root, failure.to_string());
                    failure
                })?;
                restored.push(target);
            } else if !target.exists() {
                let failure = NodeError::Replication {
                    reason: format!("checkpoint lineage retention recovery missing: {name}"),
                };
                self.mark_checkpoint_lineage_recovery_required(root, failure.to_string());
                return Err(failure);
            }
        }
        if staging.exists() {
            fs::remove_dir_all(&staging).map_err(|err| {
                let failure = NodeError::Replication {
                    reason: format!("remove checkpoint lineage recovery staging failed: {err}"),
                };
                self.mark_checkpoint_lineage_recovery_required(root, failure.to_string());
                failure
            })?;
        }
        fs::remove_file(&journal_path).map_err(|err| NodeError::Replication {
            reason: format!("remove checkpoint lineage retention journal failed: {err}"),
        })?;
        sync_dir(root)?;
        self.write_checkpoint_lineage_health(root, journal.generation, &BTreeMap::new(), 0, 0)
            .map_err(|err| {
                self.mark_checkpoint_lineage_recovery_required(root, err.to_string());
                err
            })?;
        let _ = restored;
        Ok(())
    }

    fn restore_checkpoint_lineage_paths(&self, root: &Path, staging: &Path, paths: &[PathBuf]) {
        for path in paths.iter().rev() {
            if let Some(name) = path.file_name() {
                let staged = staging.join(name);
                if staged.exists() && !path.exists() {
                    let _ = fs::rename(staged, path);
                }
            }
        }
        let _ = fs::remove_dir_all(staging);
        let _ = sync_dir(root);
    }

    fn write_checkpoint_lineage_health(
        &self,
        root: &Path,
        generation: u128,
        retained: &BTreeMap<String, BTreeSet<u64>>,
        source_files: usize,
        envelope_files: usize,
    ) -> Result<(), NodeError> {
        let retained_heights = retained
            .iter()
            .map(|(world, heights)| (world.clone(), heights.iter().copied().collect()))
            .collect();
        let health = RetentionHealthV1 {
            schema_version: RETENTION_SCHEMA_VERSION,
            generation,
            status: "ready".to_string(),
            retained_heights,
            source_files,
            envelope_files,
        };
        write_json_compact(&root.join("health.v1.json"), &health)
    }

    fn mark_checkpoint_lineage_recovery_required(&self, root: &Path, reason: String) {
        let health = RetentionHealthV1 {
            schema_version: RETENTION_SCHEMA_VERSION,
            generation: retention_generation(),
            status: "recovery_required".to_string(),
            retained_heights: BTreeMap::new(),
            source_files: 0,
            envelope_files: 0,
        };
        let _ = write_json_compact(&root.join("health.v1.json"), &health);
        let _ = reason;
    }
}

fn parse_source_height(name: &str) -> Option<u64> {
    name.strip_prefix("source-")
        .and_then(|name| name.strip_suffix(".json"))
        .and_then(|height| height.parse::<u64>().ok())
}

fn valid_cache_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && Path::new(name)
            .file_name()
            .and_then(|file_name| file_name.to_str())
            == Some(name)
}

fn retained_checkpoint_heights(
    sources: &[SourceCacheEntry],
    envelopes: &[EnvelopeCacheEntry],
    max_hot: usize,
) -> BTreeMap<String, BTreeSet<u64>> {
    let mut candidates = BTreeMap::<String, BTreeSet<u64>>::new();
    for source in sources {
        candidates
            .entry(source.world_id.clone())
            .or_default()
            .insert(source.height);
    }
    for envelope in envelopes {
        candidates
            .entry(envelope.world_id.clone())
            .or_default()
            .insert(envelope.height);
    }
    let keep_count = max_hot.max(1);
    candidates
        .into_iter()
        .map(|(world, heights)| {
            let keep = heights.iter().rev().take(keep_count).copied().collect();
            (world, keep)
        })
        .collect()
}

fn stale_checkpoint_lineage_paths(
    sources: &[SourceCacheEntry],
    envelopes: &[EnvelopeCacheEntry],
    retained: &BTreeMap<String, BTreeSet<u64>>,
) -> Vec<PathBuf> {
    sources
        .iter()
        .filter(|entry| {
            !retained
                .get(&entry.world_id)
                .is_some_and(|heights| heights.contains(&entry.height))
        })
        .map(|entry| entry.path.clone())
        .chain(
            envelopes
                .iter()
                .filter(|entry| {
                    !retained
                        .get(&entry.world_id)
                        .is_some_and(|heights| heights.contains(&entry.height))
                })
                .map(|entry| entry.path.clone()),
        )
        .collect()
}

fn retained_sidecar_counts(
    sources: &[SourceCacheEntry],
    envelopes: &[EnvelopeCacheEntry],
    retained: &BTreeMap<String, BTreeSet<u64>>,
) -> (usize, usize) {
    let source_count = sources
        .iter()
        .filter(|entry| {
            retained
                .get(&entry.world_id)
                .is_some_and(|heights| heights.contains(&entry.height))
        })
        .count();
    let envelope_count = envelopes
        .iter()
        .filter(|entry| {
            retained
                .get(&entry.world_id)
                .is_some_and(|heights| heights.contains(&entry.height))
        })
        .count();
    (source_count, envelope_count)
}

fn retention_generation() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn sync_dir(path: &Path) -> Result<(), NodeError> {
    fs::File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|err| NodeError::Replication {
            reason: format!(
                "sync checkpoint lineage directory {} failed: {err}",
                path.display()
            ),
        })
}
