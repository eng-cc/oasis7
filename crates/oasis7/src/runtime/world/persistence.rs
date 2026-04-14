use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use oasis7_distfs::{assemble_journal, assemble_snapshot};
use oasis7_proto::distributed::SnapshotManifest;
use serde::{Deserialize, Serialize};

use super::super::util::{hash_json, read_json_from_path, write_json_to_path};
use super::super::{
    segment_journal, segment_snapshot, Journal, JournalSegmentRef, LocalCasStore, ModuleCache,
    ModuleStore, RollbackEvent, SegmentConfig, Snapshot, TickConsensusRecord, WorldError,
    WorldEvent, WorldTime,
};
use super::World;
#[path = "persistence_support.rs"]
mod persistence_support;
use self::persistence_support::{
    distfs_world_id, now_unix_ms, persist_sidecar_generation_index, write_distfs_recovery_audit,
};

const JOURNAL_FILE: &str = "journal.json";
const SNAPSHOT_FILE: &str = "snapshot.json";
const DISTFS_STATE_DIR: &str = ".distfs-state";
const DISTFS_SNAPSHOT_MANIFEST_FILE: &str = "snapshot.manifest.json";
const DISTFS_JOURNAL_SEGMENTS_FILE: &str = "journal.segments.json";
const DISTFS_RECOVERY_AUDIT_FILE: &str = "distfs.recovery.audit.json";
const DISTFS_WORLD_ID_FALLBACK: &str = "runtime-world";
const TICK_CONSENSUS_ARCHIVE_FILE: &str = "tick-consensus.archive.json";
const TICK_CONSENSUS_ARCHIVE_INDEX_FILE: &str = "tick-consensus.archive.index.json";
const TICK_CONSENSUS_ARCHIVE_SEGMENTS_DIR: &str = "tick-consensus.archive.segments";
const TICK_CONSENSUS_ARCHIVE_SEGMENT_LEN: usize = 64;
const TICK_CONSENSUS_HOT_LIMIT: usize = 128;
const SIDECAR_GENERATION_INDEX_SCHEMA_V1: u32 = 1;
const SIDECAR_GENERATION_RECORD_SCHEMA_V1: u32 = 1;
const SIDECAR_GENERATION_ROOT_DIR: &str = "sidecar-generations";
const SIDECAR_GENERATION_INDEX_FILE: &str = "index.json";
const SIDECAR_GENERATION_MANIFESTS_DIR: &str = "generations";
const SIDECAR_GENERATION_PAYLOADS_DIR: &str = "payloads";
const SIDECAR_GENERATION_STAGING_DIR: &str = "generation.tmp";
const SIDECAR_GENERATION_KEEP_LATEST: usize = 2;
const SIDECAR_GENERATION_SNAPSHOT_MANIFEST_FILE: &str = "snapshot.manifest.json";
const SIDECAR_GENERATION_JOURNAL_SEGMENTS_FILE: &str = "journal.segments.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SidecarGcResult {
    status: String,
    freed_blob_count: usize,
    freed_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    updated_at_ms: i64,
}

impl SidecarGcResult {
    fn not_run() -> Self {
        Self {
            status: "not_run".to_string(),
            freed_blob_count: 0,
            freed_bytes: 0,
            error: None,
            updated_at_ms: now_unix_ms(),
        }
    }

    fn success(freed_blob_count: usize, freed_bytes: u64) -> Self {
        Self {
            status: "success".to_string(),
            freed_blob_count,
            freed_bytes,
            error: None,
            updated_at_ms: now_unix_ms(),
        }
    }

    fn failed(error: String) -> Self {
        Self {
            status: "failed".to_string(),
            freed_blob_count: 0,
            freed_bytes: 0,
            error: Some(error),
            updated_at_ms: now_unix_ms(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SidecarGenerationRecord {
    schema_version: u32,
    generation_id: String,
    snapshot_manifest_path: String,
    journal_segments_path: String,
    snapshot_manifest_hash: String,
    manifest_hash: String,
    journal_segment_hashes: Vec<String>,
    pinned_blob_hashes: Vec<String>,
    created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct SidecarGenerationIndex {
    schema_version: u32,
    latest_generation: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    rollback_safe_generation: Option<String>,
    generations: BTreeMap<String, SidecarGenerationRecord>,
    last_gc_result: SidecarGcResult,
}

#[derive(Debug, Serialize)]
struct SidecarGenerationHashPayload<'a> {
    generation_id: &'a str,
    snapshot_manifest_path: &'a str,
    journal_segments_path: &'a str,
    snapshot_manifest_hash: &'a str,
    journal_segment_hashes: &'a [String],
    pinned_blob_hashes: &'a [String],
    created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
struct TickConsensusArchiveFile {
    archived_records: Vec<TickConsensusRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
struct TickConsensusArchiveIndex {
    hot_from_tick: Option<WorldTime>,
    hot_to_tick: Option<WorldTime>,
    archived_segments: Vec<TickConsensusArchiveSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TickConsensusArchiveSegment {
    from_tick: WorldTime,
    to_tick: WorldTime,
    content_hash: String,
    record_count: usize,
    hash_chain_anchor: String,
    relative_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
struct TickConsensusArchiveSegmentFile {
    records: Vec<TickConsensusRecord>,
}

#[derive(Debug, Serialize)]
struct DistfsRecoveryAuditRecord {
    timestamp_ms: i64,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

fn tick_consensus_archive_path(dir: &Path) -> std::path::PathBuf {
    dir.join(TICK_CONSENSUS_ARCHIVE_FILE)
}

fn tick_consensus_archive_index_path(dir: &Path) -> std::path::PathBuf {
    dir.join(TICK_CONSENSUS_ARCHIVE_INDEX_FILE)
}

fn tick_consensus_archive_segments_dir(dir: &Path) -> std::path::PathBuf {
    dir.join(TICK_CONSENSUS_ARCHIVE_SEGMENTS_DIR)
}

fn tick_consensus_archive_segment_relative_path(
    from_tick: WorldTime,
    to_tick: WorldTime,
) -> String {
    format!("{TICK_CONSENSUS_ARCHIVE_SEGMENTS_DIR}/segment-{from_tick:020}-{to_tick:020}.json")
}

fn tick_consensus_hot_tick_bounds(
    records: &[TickConsensusRecord],
) -> (Option<WorldTime>, Option<WorldTime>) {
    (
        records.first().map(|record| record.block.header.tick),
        records.last().map(|record| record.block.header.tick),
    )
}

fn split_tick_consensus_snapshot_for_persistence(
    snapshot: &Snapshot,
) -> (Snapshot, Option<TickConsensusArchiveFile>) {
    let mut persisted_snapshot = snapshot.clone();
    persisted_snapshot.tick_consensus_total_record_count = snapshot.tick_consensus_records.len();
    if snapshot.tick_consensus_records.len() <= TICK_CONSENSUS_HOT_LIMIT {
        persisted_snapshot.tick_consensus_archived_record_count = 0;
        let (hot_from_tick, hot_to_tick) =
            tick_consensus_hot_tick_bounds(persisted_snapshot.tick_consensus_records.as_slice());
        persisted_snapshot.tick_consensus_hot_from_tick = hot_from_tick;
        persisted_snapshot.tick_consensus_hot_to_tick = hot_to_tick;
        return (persisted_snapshot, None);
    }

    let archived_record_count = snapshot
        .tick_consensus_records
        .len()
        .saturating_sub(TICK_CONSENSUS_HOT_LIMIT);
    let archived_records = snapshot.tick_consensus_records[..archived_record_count].to_vec();
    persisted_snapshot.tick_consensus_records =
        snapshot.tick_consensus_records[archived_record_count..].to_vec();
    persisted_snapshot.tick_consensus_archived_record_count = archived_records.len();
    let (hot_from_tick, hot_to_tick) =
        tick_consensus_hot_tick_bounds(persisted_snapshot.tick_consensus_records.as_slice());
    persisted_snapshot.tick_consensus_hot_from_tick = hot_from_tick;
    persisted_snapshot.tick_consensus_hot_to_tick = hot_to_tick;
    (
        persisted_snapshot,
        Some(TickConsensusArchiveFile { archived_records }),
    )
}

fn build_tick_consensus_archive_index(
    snapshot: &Snapshot,
    archive: &TickConsensusArchiveFile,
) -> Result<
    (
        TickConsensusArchiveIndex,
        Vec<(String, TickConsensusArchiveSegmentFile)>,
    ),
    WorldError,
> {
    let mut archived_segments = Vec::new();
    let mut segment_files = Vec::new();

    for records_chunk in archive
        .archived_records
        .chunks(TICK_CONSENSUS_ARCHIVE_SEGMENT_LEN)
    {
        if records_chunk.is_empty() {
            continue;
        }
        let segment_file = TickConsensusArchiveSegmentFile {
            records: records_chunk.to_vec(),
        };
        let from_tick = segment_file
            .records
            .first()
            .map(|record| record.block.header.tick)
            .expect("segment records");
        let to_tick = segment_file
            .records
            .last()
            .map(|record| record.block.header.tick)
            .expect("segment records");
        let relative_path = tick_consensus_archive_segment_relative_path(from_tick, to_tick);
        let content_hash = hash_json(&segment_file)?;
        let hash_chain_anchor = segment_file
            .records
            .last()
            .map(|record| record.certificate.block_hash.clone())
            .expect("segment records");
        archived_segments.push(TickConsensusArchiveSegment {
            from_tick,
            to_tick,
            content_hash,
            record_count: segment_file.records.len(),
            hash_chain_anchor,
            relative_path: relative_path.clone(),
        });
        segment_files.push((relative_path, segment_file));
    }

    Ok((
        TickConsensusArchiveIndex {
            hot_from_tick: snapshot.tick_consensus_hot_from_tick,
            hot_to_tick: snapshot.tick_consensus_hot_to_tick,
            archived_segments,
        },
        segment_files,
    ))
}

fn persist_tick_consensus_archive(
    dir: &Path,
    snapshot: &Snapshot,
    archive: Option<&TickConsensusArchiveFile>,
) -> Result<(), WorldError> {
    let legacy_archive_path = tick_consensus_archive_path(dir);
    let archive_index_path = tick_consensus_archive_index_path(dir);
    let archive_segments_dir = tick_consensus_archive_segments_dir(dir);
    match archive {
        Some(archive) if !archive.archived_records.is_empty() => {
            let (archive_index, segment_files) =
                build_tick_consensus_archive_index(snapshot, archive)?;
            if archive_segments_dir.exists() {
                fs::remove_dir_all(archive_segments_dir.as_path())?;
            }
            fs::create_dir_all(archive_segments_dir.as_path())?;
            for (relative_path, segment_file) in segment_files {
                write_json_to_path(&segment_file, dir.join(relative_path.as_str()).as_path())?;
            }
            write_json_to_path(&archive_index, archive_index_path.as_path())?;
            if legacy_archive_path.exists() {
                fs::remove_file(legacy_archive_path.as_path())?;
            }
            Ok(())
        }
        _ => {
            if legacy_archive_path.exists() {
                fs::remove_file(legacy_archive_path.as_path())?;
            }
            if archive_index_path.exists() {
                fs::remove_file(archive_index_path.as_path())?;
            }
            if archive_segments_dir.exists() {
                fs::remove_dir_all(archive_segments_dir.as_path())?;
            }
            Ok(())
        }
    }
}

fn load_tick_consensus_archive_records_from_index(
    dir: &Path,
    snapshot: &Snapshot,
) -> Result<Option<Vec<TickConsensusRecord>>, WorldError> {
    let archive_index_path = tick_consensus_archive_index_path(dir);
    if !archive_index_path.exists() {
        return Ok(None);
    }

    let archive_index: TickConsensusArchiveIndex =
        read_json_from_path(archive_index_path.as_path())?;
    if archive_index.hot_from_tick != snapshot.tick_consensus_hot_from_tick
        || archive_index.hot_to_tick != snapshot.tick_consensus_hot_to_tick
    {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "tick consensus archive index hot range mismatch: expected_from={:?} actual_from={:?} expected_to={:?} actual_to={:?}",
                snapshot.tick_consensus_hot_from_tick,
                archive_index.hot_from_tick,
                snapshot.tick_consensus_hot_to_tick,
                archive_index.hot_to_tick,
            ),
        });
    }

    let indexed_record_count = archive_index
        .archived_segments
        .iter()
        .map(|segment| segment.record_count)
        .sum::<usize>();
    if indexed_record_count != snapshot.tick_consensus_archived_record_count {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "tick consensus archive index count mismatch: expected={} actual={}",
                snapshot.tick_consensus_archived_record_count, indexed_record_count,
            ),
        });
    }

    let mut archived_records = Vec::with_capacity(indexed_record_count);
    let mut previous_to_tick = None;
    for segment in archive_index.archived_segments {
        if let Some(previous_to_tick) = previous_to_tick {
            if segment.from_tick <= previous_to_tick {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "tick consensus archive segment ordering invalid: previous_to_tick={} current_from_tick={}",
                        previous_to_tick,
                        segment.from_tick,
                    ),
                });
            }
        }
        let segment_path = dir.join(segment.relative_path.as_str());
        if !segment_path.exists() {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "tick consensus archive segment missing: path={}",
                    segment_path.display(),
                ),
            });
        }
        let segment_file: TickConsensusArchiveSegmentFile =
            read_json_from_path(segment_path.as_path())?;
        if segment_file.records.len() != segment.record_count {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "tick consensus archive segment count mismatch: expected={} actual={} path={}",
                    segment.record_count,
                    segment_file.records.len(),
                    segment_path.display(),
                ),
            });
        }
        if segment_file
            .records
            .first()
            .map(|record| record.block.header.tick)
            != Some(segment.from_tick)
            || segment_file
                .records
                .last()
                .map(|record| record.block.header.tick)
                != Some(segment.to_tick)
        {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "tick consensus archive segment range mismatch: path={}",
                    segment_path.display(),
                ),
            });
        }
        let content_hash = hash_json(&segment_file)?;
        if content_hash != segment.content_hash {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "tick consensus archive segment content hash mismatch: expected={} actual={} path={}",
                    segment.content_hash,
                    content_hash,
                    segment_path.display(),
                ),
            });
        }
        let hash_chain_anchor = segment_file
            .records
            .last()
            .map(|record| record.certificate.block_hash.clone())
            .unwrap_or_default();
        if hash_chain_anchor != segment.hash_chain_anchor {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "tick consensus archive segment anchor mismatch: expected={} actual={} path={}",
                    segment.hash_chain_anchor,
                    hash_chain_anchor,
                    segment_path.display(),
                ),
            });
        }
        previous_to_tick = Some(segment.to_tick);
        archived_records.extend(segment_file.records);
    }

    Ok(Some(archived_records))
}

fn hydrate_tick_consensus_snapshot_from_archive(
    dir: &Path,
    snapshot: &mut Snapshot,
) -> Result<(), WorldError> {
    let (actual_hot_from_tick, actual_hot_to_tick) =
        tick_consensus_hot_tick_bounds(snapshot.tick_consensus_records.as_slice());
    if snapshot.tick_consensus_hot_from_tick.is_some()
        || snapshot.tick_consensus_hot_to_tick.is_some()
    {
        if snapshot.tick_consensus_hot_from_tick != actual_hot_from_tick
            || snapshot.tick_consensus_hot_to_tick != actual_hot_to_tick
        {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "tick consensus hot summary mismatch: expected_from={:?} actual_from={:?} expected_to={:?} actual_to={:?}",
                    snapshot.tick_consensus_hot_from_tick,
                    actual_hot_from_tick,
                    snapshot.tick_consensus_hot_to_tick,
                    actual_hot_to_tick,
                ),
            });
        }
    }

    if snapshot.tick_consensus_total_record_count == 0 {
        snapshot.tick_consensus_total_record_count = snapshot.tick_consensus_records.len();
    }
    if snapshot.tick_consensus_archived_record_count == 0 {
        return Ok(());
    }

    let archived_records = match load_tick_consensus_archive_records_from_index(dir, snapshot)? {
        Some(records) => records,
        None => {
            let archive_path = tick_consensus_archive_path(dir);
            if !archive_path.exists() {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "tick consensus archive missing: path={}",
                        archive_path.display(),
                    ),
                });
            }
            let archive: TickConsensusArchiveFile = read_json_from_path(archive_path.as_path())?;
            if archive.archived_records.len() != snapshot.tick_consensus_archived_record_count {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "tick consensus archive count mismatch: expected={} actual={}",
                        snapshot.tick_consensus_archived_record_count,
                        archive.archived_records.len(),
                    ),
                });
            }
            archive.archived_records
        }
    };
    let mut records = archived_records;
    records.extend(snapshot.tick_consensus_records.clone());
    if records.len() != snapshot.tick_consensus_total_record_count {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "tick consensus total count mismatch: expected={} actual={}",
                snapshot.tick_consensus_total_record_count,
                records.len(),
            ),
        });
    }
    snapshot.tick_consensus_records = records;
    Ok(())
}

fn load_persisted_tick_consensus_snapshot_from_dir(dir: &Path) -> Result<Snapshot, WorldError> {
    if let Some((mut snapshot, _)) = World::try_load_from_distfs_sidecar(dir)? {
        hydrate_tick_consensus_snapshot_from_archive(dir, &mut snapshot)?;
        return Ok(snapshot);
    }
    let mut snapshot = Snapshot::load_json(dir.join(SNAPSHOT_FILE))?;
    hydrate_tick_consensus_snapshot_from_archive(dir, &mut snapshot)?;
    Ok(snapshot)
}

fn verify_tick_consensus_record_slice(records: &[TickConsensusRecord]) -> Result<(), WorldError> {
    let mut previous_block_hash = None;
    let mut previous_height: Option<u64> = None;
    let mut previous_tick: Option<WorldTime> = None;
    for record in records {
        let block_hash = record.block.block_hash();
        if record.certificate.block_hash != block_hash {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "tick consensus archive block hash mismatch tick={} expected={} actual={}",
                    record.block.header.tick, block_hash, record.certificate.block_hash,
                ),
            });
        }
        if let Some(previous_block_hash) = previous_block_hash.as_ref() {
            if record.block.header.parent_hash != *previous_block_hash {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "tick consensus archive parent hash mismatch tick={} expected={} actual={}",
                        record.block.header.tick,
                        previous_block_hash,
                        record.block.header.parent_hash,
                    ),
                });
            }
        }
        if let Some(previous_height) = previous_height {
            let expected_height = previous_height.saturating_add(1);
            if record.certificate.consensus_height != expected_height {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "tick consensus archive height mismatch tick={} expected={} actual={}",
                        record.block.header.tick,
                        expected_height,
                        record.certificate.consensus_height,
                    ),
                });
            }
        }
        if let Some(previous_tick) = previous_tick {
            if record.block.header.tick <= previous_tick {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "tick consensus archive tick ordering mismatch previous={} current={}",
                        previous_tick, record.block.header.tick,
                    ),
                });
            }
        }
        previous_block_hash = Some(record.certificate.block_hash.clone());
        previous_height = Some(record.certificate.consensus_height);
        previous_tick = Some(record.block.header.tick);
    }
    Ok(())
}

impl World {
    // ---------------------------------------------------------------------
    // Persistence
    // ---------------------------------------------------------------------

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            snapshot_catalog: self.snapshot_catalog.clone(),
            manifest: self.manifest.clone(),
            module_registry: self.module_registry.clone(),
            module_artifacts: self.module_artifacts.clone(),
            module_limits_max: self.module_limits_max.clone(),
            state: self.state.clone(),
            journal_len: self.journal.len(),
            last_event_id: self.next_event_id.saturating_sub(1),
            event_id_era: self.next_event_id_era,
            next_action_id: self.next_action_id,
            action_id_era: self.next_action_id_era,
            next_intent_id: self.next_intent_id,
            intent_id_era: self.next_intent_id_era,
            next_proposal_id: self.next_proposal_id,
            proposal_id_era: self.next_proposal_id_era,
            pending_actions: self.pending_actions.iter().cloned().collect(),
            pending_effects: self.pending_effects.iter().cloned().collect(),
            inflight_effects: self.inflight_effects.clone(),
            module_tick_schedule: self.module_tick_schedule.clone(),
            capabilities: self.capabilities.clone(),
            policies: self.policies.clone(),
            proposals: self.proposals.clone(),
            scheduler_cursor: self.scheduler_cursor.clone(),
            runtime_memory_limits: self.runtime_memory_limits.clone(),
            runtime_backpressure_stats: self.runtime_backpressure_stats.clone(),
            tick_consensus_records: self.tick_consensus_records.clone(),
            tick_consensus_total_record_count: self.tick_consensus_records.len(),
            tick_consensus_archived_record_count: 0,
            tick_consensus_hot_from_tick: self
                .tick_consensus_records
                .first()
                .map(|record| record.block.header.tick),
            tick_consensus_hot_to_tick: self
                .tick_consensus_records
                .last()
                .map(|record| record.block.header.tick),
            tick_consensus_authority_source: self.tick_consensus_authority_source.clone(),
            tick_consensus_rejection_audit_events: self
                .tick_consensus_rejection_audit_events
                .clone(),
            governance_execution_policy: self.governance_execution_policy.clone(),
            governance_emergency_brake_until_tick: self.governance_emergency_brake_until_tick,
            governance_identity_penalties: self.governance_identity_penalties.clone(),
            next_governance_identity_penalty_id: self.next_governance_identity_penalty_id,
        }
    }

    pub fn save_to_dir(&self, dir: impl AsRef<Path>) -> Result<(), WorldError> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir)?;
        let snapshot = self.snapshot();
        let (persisted_snapshot, tick_consensus_archive) =
            split_tick_consensus_snapshot_for_persistence(&snapshot);
        let journal_path = dir.join(JOURNAL_FILE);
        let snapshot_path = dir.join(SNAPSHOT_FILE);
        self.journal.save_json(journal_path)?;
        persisted_snapshot.save_json(snapshot_path)?;
        persist_tick_consensus_archive(dir, &persisted_snapshot, tick_consensus_archive.as_ref())?;
        self.save_distfs_sidecar(dir, &persisted_snapshot)?;
        self.save_module_store_to_dir(dir)?;
        Ok(())
    }

    pub fn save_to_dir_with_modules(&self, dir: impl AsRef<Path>) -> Result<(), WorldError> {
        self.save_to_dir(dir)
    }

    pub fn save_module_store_to_dir(&self, dir: impl AsRef<Path>) -> Result<(), WorldError> {
        let store = ModuleStore::new(dir);
        store.save_registry(&self.module_registry)?;
        for record in self.module_registry.records.values() {
            store.write_meta(&record.manifest)?;
            let wasm_hash = &record.manifest.wasm_hash;
            let bytes = self.module_artifact_bytes.get(wasm_hash).ok_or_else(|| {
                WorldError::ModuleStoreArtifactMissing {
                    wasm_hash: wasm_hash.clone(),
                }
            })?;
            store.write_artifact(wasm_hash, bytes.as_ref())?;
        }
        Ok(())
    }

    pub fn load_from_dir(dir: impl AsRef<Path>) -> Result<Self, WorldError> {
        let dir = dir.as_ref();
        if let Some((mut snapshot, journal)) = Self::try_load_from_distfs_sidecar(dir)? {
            hydrate_tick_consensus_snapshot_from_archive(dir, &mut snapshot)?;
            let mut world = Self::from_snapshot(snapshot, journal)?;
            world.load_module_store_from_dir(dir)?;
            return Ok(world);
        }
        let journal_path = dir.join(JOURNAL_FILE);
        let snapshot_path = dir.join(SNAPSHOT_FILE);
        let journal = Journal::load_json(journal_path)?;
        let mut snapshot = Snapshot::load_json(snapshot_path)?;
        hydrate_tick_consensus_snapshot_from_archive(dir, &mut snapshot)?;
        let mut world = Self::from_snapshot(snapshot, journal)?;
        world.load_module_store_from_dir(dir)?;
        Ok(world)
    }

    pub fn load_from_dir_with_modules(dir: impl AsRef<Path>) -> Result<Self, WorldError> {
        Self::load_from_dir(dir)
    }

    pub fn load_tick_consensus_records_from_dir(
        dir: impl AsRef<Path>,
        tick_from: Option<WorldTime>,
        tick_to: Option<WorldTime>,
    ) -> Result<Vec<TickConsensusRecord>, WorldError> {
        let snapshot = load_persisted_tick_consensus_snapshot_from_dir(dir.as_ref())?;
        Ok(snapshot
            .tick_consensus_records
            .into_iter()
            .filter(|record| {
                tick_from
                    .map(|from_tick| record.block.header.tick >= from_tick)
                    .unwrap_or(true)
                    && tick_to
                        .map(|to_tick| record.block.header.tick <= to_tick)
                        .unwrap_or(true)
            })
            .collect())
    }

    pub fn verify_tick_consensus_archive_from_dir(dir: impl AsRef<Path>) -> Result<(), WorldError> {
        let records = Self::load_tick_consensus_records_from_dir(dir, None, None)?;
        verify_tick_consensus_record_slice(records.as_slice())
    }

    pub fn load_module_store_from_dir(&mut self, dir: impl AsRef<Path>) -> Result<(), WorldError> {
        let store = ModuleStore::new(dir);
        if !store.registry_path().exists() {
            return Ok(());
        }
        let registry = store.load_registry()?;
        self.module_registry = registry;
        self.module_artifacts.clear();
        self.module_artifact_bytes.clear();

        for record in self.module_registry.records.values() {
            let wasm_hash = &record.manifest.wasm_hash;
            let meta = store.read_meta(wasm_hash)?;
            if meta != record.manifest {
                return Err(WorldError::ModuleStoreManifestMismatch {
                    wasm_hash: wasm_hash.clone(),
                });
            }
            let bytes = store.read_artifact(wasm_hash)?;
            let actual_hash = super::super::util::sha256_hex(&bytes);
            if actual_hash != *wasm_hash {
                return Err(WorldError::ModuleStoreManifestMismatch {
                    wasm_hash: wasm_hash.clone(),
                });
            }
            self.validate_module_artifact_identity(&record.manifest)?;
            self.module_artifacts.insert(wasm_hash.clone());
            self.module_artifact_bytes
                .insert(wasm_hash.clone(), bytes.into());
        }
        Ok(())
    }

    pub fn rollback_to_snapshot(
        &mut self,
        snapshot: Snapshot,
        mut journal: Journal,
        reason: impl Into<String>,
    ) -> Result<(), WorldError> {
        if snapshot.journal_len > journal.len() {
            return Err(WorldError::JournalMismatch);
        }

        let prior_len = journal.len();
        journal.events.truncate(snapshot.journal_len);

        let signer = self.receipt_signer.clone();
        let mut world = Self::from_snapshot(snapshot.clone(), journal)?;
        world.receipt_signer = signer;

        let snapshot_hash = hash_json(&snapshot)?;
        let event = RollbackEvent {
            snapshot_hash,
            snapshot_journal_len: snapshot.journal_len,
            prior_journal_len: prior_len,
            reason: reason.into(),
        };
        world.append_event(super::super::WorldEventBody::RollbackApplied(event), None)?;
        *self = world;
        Ok(())
    }

    pub fn rollback_to_snapshot_with_reconciliation(
        &mut self,
        snapshot: Snapshot,
        journal: Journal,
        reason: impl Into<String>,
    ) -> Result<(), WorldError> {
        self.rollback_to_snapshot(snapshot, journal, reason)?;
        if let Some(drift) = self.first_tick_consensus_drift() {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "rollback reconciliation drift detected at tick {}: {}",
                    drift.tick, drift.reason
                ),
            });
        }
        Ok(())
    }

    pub fn from_snapshot(snapshot: Snapshot, journal: Journal) -> Result<Self, WorldError> {
        if snapshot.journal_len > journal.len() {
            return Err(WorldError::JournalMismatch);
        }

        let mut world = Self::new_with_state(snapshot.state);
        world.journal = journal;
        world.manifest = snapshot.manifest;
        world.module_registry = snapshot.module_registry;
        world.module_artifacts = snapshot.module_artifacts;
        world.module_artifact_bytes = BTreeMap::new();
        world.module_cache = ModuleCache::default();
        world.module_limits_max = snapshot.module_limits_max;
        world.snapshot_catalog = snapshot.snapshot_catalog;
        world.next_event_id = snapshot.last_event_id.saturating_add(1).max(1);
        world.next_event_id_era = snapshot.event_id_era;
        world.next_action_id = snapshot.next_action_id.max(1);
        world.next_action_id_era = snapshot.action_id_era;
        world.next_intent_id = snapshot.next_intent_id.max(1);
        world.next_intent_id_era = snapshot.intent_id_era;
        world.next_proposal_id = snapshot.next_proposal_id.max(1);
        world.next_proposal_id_era = snapshot.proposal_id_era;
        world.pending_actions = VecDeque::from(snapshot.pending_actions);
        world.pending_effects = VecDeque::from(snapshot.pending_effects);
        world.inflight_effects = snapshot.inflight_effects;
        world.module_tick_schedule = snapshot.module_tick_schedule;
        world.capabilities = snapshot.capabilities;
        world.policies = snapshot.policies;
        world.proposals = snapshot.proposals;
        world.scheduler_cursor = snapshot.scheduler_cursor;
        world.runtime_memory_limits = snapshot.runtime_memory_limits;
        world.runtime_backpressure_stats = snapshot.runtime_backpressure_stats;
        world.tick_consensus_records = snapshot.tick_consensus_records;
        world.tick_consensus_authority_source =
            snapshot.tick_consensus_authority_source.trim().to_string();
        if world.tick_consensus_authority_source.is_empty() {
            world.tick_consensus_authority_source =
                super::BUILTIN_MODULE_SIGNER_NODE_ID.to_string();
        }
        world.tick_consensus_rejection_audit_events =
            snapshot.tick_consensus_rejection_audit_events;
        world.governance_execution_policy = snapshot.governance_execution_policy;
        Self::validate_governance_execution_policy(&world.governance_execution_policy)?;
        world.governance_emergency_brake_until_tick =
            snapshot.governance_emergency_brake_until_tick;
        world.governance_identity_penalties = snapshot.governance_identity_penalties;
        world.next_governance_identity_penalty_id =
            snapshot.next_governance_identity_penalty_id.max(1);
        world.enforce_pending_action_limit();
        world.enforce_pending_effect_limit();
        world.enforce_inflight_effect_limit();
        world.replay_from(snapshot.journal_len)?;
        world.verify_tick_consensus_chain()?;
        world.enforce_journal_event_limit();
        Ok(world)
    }

    fn save_distfs_sidecar(&self, dir: &Path, snapshot: &Snapshot) -> Result<(), WorldError> {
        let store_root = dir.join(DISTFS_STATE_DIR);
        fs::create_dir_all(store_root.as_path())?;
        let store = LocalCasStore::new(store_root.as_path());
        let config = SegmentConfig::default();
        let world_id = distfs_world_id(dir);
        let epoch = snapshot.state.time;
        let manifest = segment_snapshot(snapshot, world_id.as_str(), epoch, &store, config)?;
        let journal_segments = segment_journal(&self.journal, &store, config)?;

        let restored_snapshot: Snapshot = assemble_snapshot(&manifest, &store)?;
        if restored_snapshot != *snapshot {
            return Err(WorldError::DistributedValidationFailed {
                reason: "distfs snapshot assemble verification mismatch".to_string(),
            });
        }

        let restored_events: Vec<WorldEvent> =
            assemble_journal(&journal_segments, &store, |event: &WorldEvent| event.id)?;
        if restored_events != self.journal.events {
            return Err(WorldError::DistributedValidationFailed {
                reason: "distfs journal assemble verification mismatch".to_string(),
            });
        }

        persist_sidecar_generation_index(
            store_root.as_path(),
            &manifest,
            journal_segments.as_slice(),
        )?;
        let snapshot_manifest_path = dir.join(DISTFS_SNAPSHOT_MANIFEST_FILE);
        let journal_segments_path = dir.join(DISTFS_JOURNAL_SEGMENTS_FILE);
        write_json_to_path(&manifest, snapshot_manifest_path.as_path())?;
        write_json_to_path(&journal_segments, journal_segments_path.as_path())?;
        Ok(())
    }

    fn try_load_from_distfs_sidecar(dir: &Path) -> Result<Option<(Snapshot, Journal)>, WorldError> {
        let snapshot_manifest_path = dir.join(DISTFS_SNAPSHOT_MANIFEST_FILE);
        let journal_segments_path = dir.join(DISTFS_JOURNAL_SEGMENTS_FILE);
        let store_root = dir.join(DISTFS_STATE_DIR);
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

    fn load_from_distfs_sidecar(
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
