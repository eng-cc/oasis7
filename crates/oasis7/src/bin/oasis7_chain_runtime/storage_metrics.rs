use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

use oasis7::runtime::{measure_directory_storage_bytes, LocalCasStore};
use oasis7_proto::storage_profile::{StorageProfile, StorageProfileConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::RuntimePaths;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct StorageReplaySummary {
    pub retained_height_count: usize,
    pub earliest_retained_height: Option<u64>,
    pub latest_retained_height: Option<u64>,
    pub earliest_checkpoint_height: Option<u64>,
    pub latest_checkpoint_height: Option<u64>,
    pub mode: String,
}

impl Default for StorageReplaySummary {
    fn default() -> Self {
        Self {
            retained_height_count: 0,
            earliest_retained_height: None,
            latest_retained_height: None,
            earliest_checkpoint_height: None,
            latest_checkpoint_height: None,
            mode: "latest_only".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct StorageMetricsSnapshot {
    pub storage_profile: String,
    pub effective_budget: StorageProfileConfig,
    pub bytes_by_dir: BTreeMap<String, u64>,
    pub blob_counts: BTreeMap<String, u64>,
    pub ref_count: u64,
    pub pin_count: u64,
    pub retained_heights: Vec<u64>,
    pub checkpoint_count: usize,
    pub replay_summary: StorageReplaySummary,
    pub orphan_blob_count: u64,
    pub last_gc_at_ms: Option<i64>,
    pub last_gc_result: String,
    pub last_gc_error: Option<String>,
    pub degraded_reason: Option<String>,
}

impl StorageMetricsSnapshot {
    fn empty(profile: StorageProfile) -> Self {
        Self {
            storage_profile: profile.as_str().to_string(),
            effective_budget: StorageProfileConfig::from(profile),
            bytes_by_dir: BTreeMap::new(),
            blob_counts: BTreeMap::new(),
            ref_count: 0,
            pin_count: 0,
            retained_heights: Vec::new(),
            checkpoint_count: 0,
            replay_summary: StorageReplaySummary::default(),
            orphan_blob_count: 0,
            last_gc_at_ms: None,
            last_gc_result: "not_available".to_string(),
            last_gc_error: None,
            degraded_reason: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct ExecutionRefCountCache {
    record_ref_counts: BTreeMap<u64, CachedRefCount>,
    checkpoint_ref_counts: BTreeMap<u64, CachedRefCount>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CachedRefCount {
    file_stamp: FileStamp,
    ref_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileStamp {
    len: u64,
    modified_at_ms: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SharedStorageMetricsState {
    snapshot: StorageMetricsSnapshot,
    execution_ref_count_cache: ExecutionRefCountCache,
}

pub(super) type SharedStorageMetrics = Arc<Mutex<SharedStorageMetricsState>>;

pub(super) fn init_shared_storage_metrics(profile: StorageProfile) -> SharedStorageMetrics {
    Arc::new(Mutex::new(SharedStorageMetricsState {
        snapshot: StorageMetricsSnapshot::empty(profile),
        execution_ref_count_cache: ExecutionRefCountCache::default(),
    }))
}

pub(super) fn snapshot_storage_metrics(metrics: &SharedStorageMetrics) -> StorageMetricsSnapshot {
    match metrics.lock() {
        Ok(locked) => locked.snapshot.clone(),
        Err(_) => StorageMetricsSnapshot {
            degraded_reason: Some("storage metrics lock poisoned".to_string()),
            ..StorageMetricsSnapshot::empty(StorageProfile::DevLocal)
        },
    }
}

pub(super) fn refresh_shared_storage_metrics(
    metrics: &SharedStorageMetrics,
    paths: &RuntimePaths,
    profile: StorageProfile,
    degraded_reason: Option<String>,
) -> Result<StorageMetricsSnapshot, String> {
    let mut locked = metrics
        .lock()
        .map_err(|_| "storage metrics lock poisoned".to_string())?;
    let snapshot = collect_storage_metrics(
        paths,
        profile,
        degraded_reason,
        &mut locked.execution_ref_count_cache,
    );
    locked.snapshot = snapshot.clone();
    persist_storage_metrics_snapshot(
        paths.reward_runtime_storage_metrics_path.as_path(),
        &snapshot,
    )?;
    Ok(snapshot)
}

pub(super) fn collect_storage_metrics(
    paths: &RuntimePaths,
    profile: StorageProfile,
    degraded_reason: Option<String>,
    execution_ref_count_cache: &mut ExecutionRefCountCache,
) -> StorageMetricsSnapshot {
    let mut snapshot = StorageMetricsSnapshot::empty(profile);
    let mut issues = Vec::new();

    snapshot.bytes_by_dir.insert(
        "runtime_root".to_string(),
        measure_directory_storage_bytes(paths.runtime_root.as_path()),
    );
    snapshot.bytes_by_dir.insert(
        "execution_world_dir".to_string(),
        measure_directory_storage_bytes(paths.execution_world_dir.as_path()),
    );
    snapshot.bytes_by_dir.insert(
        "execution_records_dir".to_string(),
        measure_directory_storage_bytes(paths.execution_records_dir.as_path()),
    );
    snapshot.bytes_by_dir.insert(
        "execution_store_root".to_string(),
        measure_directory_storage_bytes(paths.storage_root.as_path()),
    );
    snapshot.bytes_by_dir.insert(
        "reward_runtime_report_dir".to_string(),
        measure_directory_storage_bytes(paths.reward_runtime_report_dir.as_path()),
    );
    snapshot.bytes_by_dir.insert(
        "replication_root".to_string(),
        measure_directory_storage_bytes(paths.replication_root.as_path()),
    );

    snapshot.blob_counts.insert(
        "execution_store_blobs".to_string(),
        safe_blob_count(paths.storage_root.as_path(), &mut issues),
    );
    let sidecar_store_root = sidecar_store_root(paths.execution_world_dir.as_path());
    let sidecar_blob_count = safe_blob_count(sidecar_store_root.as_path(), &mut issues);
    snapshot
        .blob_counts
        .insert("execution_sidecar_blobs".to_string(), sidecar_blob_count);
    snapshot.blob_counts.insert(
        "replication_blobs".to_string(),
        safe_blob_count(paths.replication_root.as_path(), &mut issues),
    );

    let mut checkpoint_heights = Vec::new();
    match list_retained_heights(paths.execution_records_dir.as_path()) {
        Ok(heights) => snapshot.retained_heights = heights,
        Err(err) => issues.push(err),
    }
    match list_checkpoint_heights(paths.execution_records_dir.as_path()) {
        Ok(heights) => {
            snapshot.checkpoint_count = heights.len();
            checkpoint_heights = heights;
        }
        Err(err) => issues.push(err),
    }
    snapshot.replay_summary = build_replay_summary(
        snapshot.retained_heights.as_slice(),
        checkpoint_heights.as_slice(),
    );
    match count_execution_refs(
        paths.execution_records_dir.as_path(),
        snapshot.retained_heights.as_slice(),
        checkpoint_heights.as_slice(),
        execution_ref_count_cache,
    ) {
        Ok(ref_count) => snapshot.ref_count = ref_count,
        Err(err) => issues.push(err),
    }
    match read_sidecar_metrics(sidecar_store_root.as_path()) {
        Ok(sidecar_metrics) => {
            snapshot.pin_count = sidecar_metrics.pin_count;
            snapshot.orphan_blob_count =
                sidecar_blob_count.saturating_sub(sidecar_metrics.pin_count);
            snapshot.last_gc_at_ms = sidecar_metrics.last_gc_at_ms;
            snapshot.last_gc_result = sidecar_metrics.last_gc_result;
            snapshot.last_gc_error = sidecar_metrics.last_gc_error;
        }
        Err(err) => issues.push(err),
    }

    if !issues.is_empty() {
        snapshot.degraded_reason = Some(issues.join("; "));
    }
    if degraded_reason.is_some() {
        snapshot.degraded_reason = degraded_reason;
    }
    snapshot
}

fn persist_storage_metrics_snapshot(
    path: &Path,
    snapshot: &StorageMetricsSnapshot,
) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(snapshot)
        .map_err(|err| format!("serialize storage metrics snapshot failed: {err}"))?;
    super::write_bytes_atomic(path, bytes.as_slice())
}

fn sidecar_store_root(execution_world_dir: &Path) -> PathBuf {
    execution_world_dir.join(".distfs-state")
}

fn safe_blob_count(root: &Path, issues: &mut Vec<String>) -> u64 {
    match LocalCasStore::new(root).list_blob_hashes() {
        Ok(hashes) => hashes.len() as u64,
        Err(err) => {
            if root.exists() {
                issues.push(format!(
                    "list blobs under {} failed: {err:?}",
                    root.display()
                ));
            }
            0
        }
    }
}

fn list_retained_heights(execution_records_dir: &Path) -> Result<Vec<u64>, String> {
    let mut heights = Vec::new();
    if !execution_records_dir.exists() {
        return Ok(heights);
    }
    for entry in fs::read_dir(execution_records_dir).map_err(|err| {
        format!(
            "read execution records dir {} failed: {err}",
            execution_records_dir.display()
        )
    })? {
        let entry = entry.map_err(|err| format!("read execution records entry failed: {err}"))?;
        let file_type = entry
            .file_type()
            .map_err(|err| format!("read execution records entry type failed: {err}"))?;
        if !file_type.is_file() {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        if stem == "latest" {
            continue;
        }
        if stem.len() != 20 || !stem.chars().all(|ch| ch.is_ascii_digit()) {
            continue;
        }
        let height = stem
            .parse::<u64>()
            .map_err(|err| format!("parse retained height `{stem}` failed: {err}"))?;
        heights.push(height);
    }
    heights.sort_unstable();
    Ok(heights)
}

fn list_checkpoint_heights(execution_records_dir: &Path) -> Result<Vec<u64>, String> {
    let checkpoint_root = execution_records_dir.join("checkpoints");
    if !checkpoint_root.exists() {
        return Ok(Vec::new());
    }
    let mut heights = Vec::new();
    for entry in fs::read_dir(checkpoint_root.as_path()).map_err(|err| {
        format!(
            "read checkpoint root {} failed: {err}",
            checkpoint_root.display()
        )
    })? {
        let entry = entry.map_err(|err| format!("read checkpoint entry failed: {err}"))?;
        if !entry
            .file_type()
            .map_err(|err| format!("read checkpoint entry type failed: {err}"))?
            .is_dir()
        {
            continue;
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Ok(height) = name.parse::<u64>() else {
            continue;
        };
        if path.join("manifest.json").exists() {
            heights.push(height);
        }
    }
    heights.sort_unstable();
    Ok(heights)
}

fn build_replay_summary(
    retained_heights: &[u64],
    checkpoint_heights: &[u64],
) -> StorageReplaySummary {
    let retained_height_count = retained_heights.len();
    let mode = if retained_heights.is_empty() {
        "latest_only"
    } else if checkpoint_heights.is_empty() {
        "full_log_only"
    } else {
        "checkpoint_plus_log"
    };
    StorageReplaySummary {
        retained_height_count,
        earliest_retained_height: retained_heights.first().copied(),
        latest_retained_height: retained_heights.last().copied(),
        earliest_checkpoint_height: checkpoint_heights.first().copied(),
        latest_checkpoint_height: checkpoint_heights.last().copied(),
        mode: mode.to_string(),
    }
}

fn count_execution_refs(
    execution_records_dir: &Path,
    retained_heights: &[u64],
    checkpoint_heights: &[u64],
    cache: &mut ExecutionRefCountCache,
) -> Result<u64, String> {
    let retained_set: BTreeSet<u64> = retained_heights.iter().copied().collect();
    cache
        .record_ref_counts
        .retain(|height, _| retained_set.contains(height));
    for height in retained_heights {
        let path = execution_records_dir.join(format!("{height:020}.json"));
        refresh_cached_ref_count(path.as_path(), &mut cache.record_ref_counts, *height)?;
    }

    let checkpoint_set: BTreeSet<u64> = checkpoint_heights.iter().copied().collect();
    cache
        .checkpoint_ref_counts
        .retain(|height, _| checkpoint_set.contains(height));
    for height in checkpoint_heights {
        let manifest_path = execution_records_dir
            .join("checkpoints")
            .join(format!("{height:020}"))
            .join("manifest.json");
        refresh_cached_ref_count(
            manifest_path.as_path(),
            &mut cache.checkpoint_ref_counts,
            *height,
        )?;
    }

    Ok(cache
        .record_ref_counts
        .values()
        .map(|entry| entry.ref_count)
        .sum::<u64>()
        + cache
            .checkpoint_ref_counts
            .values()
            .map(|entry| entry.ref_count)
            .sum::<u64>())
}

fn count_refs_in_json_file(path: &Path) -> Result<u64, String> {
    let bytes =
        fs::read(path).map_err(|err| format!("read JSON file {} failed: {err}", path.display()))?;
    let value: Value = serde_json::from_slice(bytes.as_slice())
        .map_err(|err| format!("parse JSON file {} failed: {err}", path.display()))?;
    Ok(count_ref_like_fields(&value))
}

fn count_ref_like_fields(value: &Value) -> u64 {
    match value {
        Value::Array(items) => items.iter().map(count_ref_like_fields).sum(),
        Value::Object(map) => map
            .iter()
            .map(|(key, value)| {
                let mut total = count_ref_like_fields(value);
                if key.ends_with("_ref")
                    && value
                        .as_str()
                        .map(|item| !item.trim().is_empty())
                        .unwrap_or(false)
                {
                    total = total.saturating_add(1);
                }
                total
            })
            .sum(),
        _ => 0,
    }
}

fn refresh_cached_ref_count(
    path: &Path,
    cache: &mut BTreeMap<u64, CachedRefCount>,
    height: u64,
) -> Result<(), String> {
    let metadata = fs::metadata(path)
        .map_err(|err| format!("read metadata for {} failed: {err}", path.display()))?;
    let file_stamp = file_stamp_from_metadata(path, &metadata)?;
    let should_refresh = cache
        .get(&height)
        .map(|cached| cached.file_stamp != file_stamp)
        .unwrap_or(true);
    if !should_refresh {
        return Ok(());
    }
    let ref_count = count_refs_in_json_file(path)?;
    cache.insert(
        height,
        CachedRefCount {
            file_stamp,
            ref_count,
        },
    );
    Ok(())
}

fn file_stamp_from_metadata(path: &Path, metadata: &Metadata) -> Result<FileStamp, String> {
    let modified = metadata
        .modified()
        .map_err(|err| format!("read modified time for {} failed: {err}", path.display()))?;
    let modified_at_ms = modified
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("convert modified time for {} failed: {err}", path.display()))?
        .as_millis();
    Ok(FileStamp {
        len: metadata.len(),
        modified_at_ms,
    })
}

#[derive(Debug, Default, Deserialize)]
struct SidecarGenerationRecordWire {
    #[serde(default)]
    pinned_blob_hashes: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
struct SidecarGcResultWire {
    #[serde(default)]
    status: String,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    updated_at_ms: i64,
}

#[derive(Debug, Default, Deserialize)]
struct SidecarGenerationIndexWire {
    #[serde(default)]
    latest_generation: String,
    #[serde(default)]
    rollback_safe_generation: Option<String>,
    #[serde(default)]
    generations: BTreeMap<String, SidecarGenerationRecordWire>,
    #[serde(default)]
    last_gc_result: SidecarGcResultWire,
}

#[derive(Debug)]
struct SidecarMetricsSnapshot {
    pin_count: u64,
    last_gc_at_ms: Option<i64>,
    last_gc_result: String,
    last_gc_error: Option<String>,
}

fn read_sidecar_metrics(sidecar_store_root: &Path) -> Result<SidecarMetricsSnapshot, String> {
    let index_path = sidecar_store_root.join("sidecar-generations/index.json");
    if !index_path.exists() {
        return Ok(SidecarMetricsSnapshot {
            pin_count: 0,
            last_gc_at_ms: None,
            last_gc_result: "not_available".to_string(),
            last_gc_error: None,
        });
    }
    let bytes = fs::read(index_path.as_path()).map_err(|err| {
        format!(
            "read sidecar generation index {} failed: {err}",
            index_path.display()
        )
    })?;
    let index: SidecarGenerationIndexWire =
        serde_json::from_slice(bytes.as_slice()).map_err(|err| {
            format!(
                "parse sidecar generation index {} failed: {err}",
                index_path.display()
            )
        })?;
    let mut active_generation_ids = BTreeSet::new();
    if !index.latest_generation.trim().is_empty() {
        active_generation_ids.insert(index.latest_generation.trim().to_string());
    }
    if let Some(rollback_safe_generation) = index.rollback_safe_generation.as_ref() {
        if !rollback_safe_generation.trim().is_empty() {
            active_generation_ids.insert(rollback_safe_generation.trim().to_string());
        }
    }
    let mut pinned_blob_hashes = BTreeSet::new();
    for generation_id in active_generation_ids {
        if let Some(record) = index.generations.get(generation_id.as_str()) {
            for hash in &record.pinned_blob_hashes {
                if !hash.trim().is_empty() {
                    pinned_blob_hashes.insert(hash.trim().to_string());
                }
            }
        }
    }
    Ok(SidecarMetricsSnapshot {
        pin_count: pinned_blob_hashes.len() as u64,
        last_gc_at_ms: Some(index.last_gc_result.updated_at_ms).filter(|value| *value > 0),
        last_gc_result: if index.last_gc_result.status.trim().is_empty() {
            "unknown".to_string()
        } else {
            index.last_gc_result.status
        },
        last_gc_error: index.last_gc_result.error,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use oasis7::runtime::World as RuntimeWorld;
    use oasis7_proto::storage_profile::StorageProfile;

    use super::super::RuntimePaths;
    use super::{
        collect_storage_metrics, init_shared_storage_metrics, refresh_shared_storage_metrics,
        ExecutionRefCountCache,
    };

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("duration")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "oasis7_storage_metrics_{label}_{}_{}",
            std::process::id(),
            unique
        ));
        fs::create_dir_all(dir.as_path()).expect("create temp dir");
        dir
    }

    fn write_blob(root: &Path, hash: &str) {
        let blobs_dir = root.join("blobs");
        fs::create_dir_all(blobs_dir.as_path()).expect("create blobs dir");
        fs::write(blobs_dir.join(format!("{hash}.blob")), hash.as_bytes()).expect("write blob");
    }

    #[test]
    fn collect_storage_metrics_reports_storage_snapshot() {
        let root = temp_dir("collect");
        let paths = RuntimePaths {
            runtime_root: root.clone(),
            execution_bridge_state_path: root.join("bridge-state.json"),
            execution_world_dir: root.join("reward-runtime-execution-world"),
            execution_records_dir: root.join("reward-runtime-execution-records"),
            storage_root: root.join("store"),
            replication_root: root.join("replication"),
            reward_runtime_state_path: root.join("reward-runtime-state.json"),
            reward_runtime_distfs_probe_state_path: root
                .join("reward-runtime-distfs-probe-state.json"),
            reward_runtime_report_dir: root.join("reward-runtime-report"),
            reward_runtime_storage_metrics_path: root.join("reward-runtime-storage-metrics.json"),
        };
        fs::create_dir_all(
            paths
                .execution_world_dir
                .join(".distfs-state/sidecar-generations"),
        )
        .expect("create sidecar index dir");
        fs::create_dir_all(
            paths
                .execution_records_dir
                .join("checkpoints/00000000000000000002"),
        )
        .expect("create checkpoints dir");
        fs::create_dir_all(paths.reward_runtime_report_dir.as_path()).expect("create report dir");

        fs::write(
            paths
                .execution_records_dir
                .join("00000000000000000001.json"),
            r#"{"latest_state_ref":"cas:a","snapshot_ref":"cas:b","journal_ref":"cas:c"}"#,
        )
        .expect("write record 1");
        fs::write(
            paths
                .execution_records_dir
                .join("00000000000000000002.json"),
            r#"{"latest_state_ref":"cas:d","commit_log_ref":"cas:e"}"#,
        )
        .expect("write record 2");
        fs::write(
            paths.execution_records_dir.join("latest.json"),
            r#"{"latest_state_ref":"cas:d"}"#,
        )
        .expect("write latest record");
        fs::write(
            paths
                .execution_records_dir
                .join("checkpoints/00000000000000000002/manifest.json"),
            r#"{"snapshot_ref":"cas:f","journal_ref":"cas:g"}"#,
        )
        .expect("write checkpoint manifest");

        let sidecar_index = r#"{
  "latest_generation": "g2",
  "rollback_safe_generation": "g1",
  "generations": {
    "g1": {"pinned_blob_hashes": ["aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"]},
    "g2": {"pinned_blob_hashes": ["bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"]}
  },
  "last_gc_result": {
    "status": "success",
    "updated_at_ms": 1700000000000,
    "error": null
  }
}"#;
        fs::write(
            paths
                .execution_world_dir
                .join(".distfs-state/sidecar-generations/index.json"),
            sidecar_index,
        )
        .expect("write sidecar index");

        write_blob(
            paths.storage_root.as_path(),
            "1111111111111111111111111111111111111111111111111111111111111111",
        );
        write_blob(
            paths.storage_root.as_path(),
            "2222222222222222222222222222222222222222222222222222222222222222",
        );
        write_blob(
            paths.execution_world_dir.join(".distfs-state").as_path(),
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        write_blob(
            paths.execution_world_dir.join(".distfs-state").as_path(),
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        );
        write_blob(
            paths.execution_world_dir.join(".distfs-state").as_path(),
            "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        );
        write_blob(
            paths.execution_world_dir.join(".distfs-state").as_path(),
            "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
        );
        write_blob(
            paths.replication_root.as_path(),
            "9999999999999999999999999999999999999999999999999999999999999999",
        );

        let mut cache = ExecutionRefCountCache::default();
        let snapshot =
            collect_storage_metrics(&paths, StorageProfile::ReleaseDefault, None, &mut cache);
        assert_eq!(snapshot.storage_profile, "release_default");
        assert_eq!(
            snapshot.effective_budget.profile,
            StorageProfile::ReleaseDefault
        );
        assert_eq!(snapshot.retained_heights, vec![1, 2]);
        assert_eq!(snapshot.checkpoint_count, 1);
        assert_eq!(snapshot.replay_summary.retained_height_count, 2);
        assert_eq!(snapshot.replay_summary.latest_checkpoint_height, Some(2));
        assert_eq!(snapshot.replay_summary.mode, "checkpoint_plus_log");
        assert_eq!(snapshot.pin_count, 3);
        assert_eq!(snapshot.orphan_blob_count, 1);
        assert_eq!(snapshot.last_gc_result, "success");
        assert_eq!(snapshot.last_gc_at_ms, Some(1_700_000_000_000));
        assert_eq!(snapshot.ref_count, 7);
        assert!(
            snapshot
                .bytes_by_dir
                .get("runtime_root")
                .copied()
                .unwrap_or_default()
                > 0
        );
        assert_eq!(
            snapshot.blob_counts.get("execution_store_blobs").copied(),
            Some(2)
        );
        assert_eq!(
            snapshot.blob_counts.get("execution_sidecar_blobs").copied(),
            Some(4)
        );
        assert_eq!(
            snapshot.blob_counts.get("replication_blobs").copied(),
            Some(1)
        );
    }

    #[test]
    fn collect_storage_metrics_sidecar_orphan_recovers_after_successful_save() {
        let root = temp_dir("sidecar-orphan-recovery");
        let paths = RuntimePaths {
            runtime_root: root.clone(),
            execution_bridge_state_path: root.join("bridge-state.json"),
            execution_world_dir: root.join("reward-runtime-execution-world"),
            execution_records_dir: root.join("reward-runtime-execution-records"),
            storage_root: root.join("store"),
            replication_root: root.join("replication"),
            reward_runtime_state_path: root.join("reward-runtime-state.json"),
            reward_runtime_distfs_probe_state_path: root
                .join("reward-runtime-distfs-probe-state.json"),
            reward_runtime_report_dir: root.join("reward-runtime-report"),
            reward_runtime_storage_metrics_path: root.join("reward-runtime-storage-metrics.json"),
        };
        fs::create_dir_all(paths.execution_world_dir.as_path())
            .expect("create execution world dir");

        let world = RuntimeWorld::new();
        world
            .save_to_dir(paths.execution_world_dir.as_path())
            .expect("initial save should succeed");

        let sidecar_root = paths.execution_world_dir.join(".distfs-state");
        write_blob(
            sidecar_root.as_path(),
            "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
        );

        let mut cache = ExecutionRefCountCache::default();
        let before_recovery =
            collect_storage_metrics(&paths, StorageProfile::ReleaseDefault, None, &mut cache);
        assert!(before_recovery.orphan_blob_count > 0);

        world
            .save_to_dir(paths.execution_world_dir.as_path())
            .expect("save should sweep injected orphan");

        let recovered =
            collect_storage_metrics(&paths, StorageProfile::ReleaseDefault, None, &mut cache);
        assert_eq!(recovered.orphan_blob_count, 0);
        assert_eq!(recovered.last_gc_result, "success");
        assert!(recovered.pin_count > 0);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn refresh_shared_storage_metrics_writes_state_file() {
        let root = temp_dir("persist");
        let paths = RuntimePaths {
            runtime_root: root.clone(),
            execution_bridge_state_path: root.join("bridge-state.json"),
            execution_world_dir: root.join("reward-runtime-execution-world"),
            execution_records_dir: root.join("reward-runtime-execution-records"),
            storage_root: root.join("store"),
            replication_root: root.join("replication"),
            reward_runtime_state_path: root.join("reward-runtime-state.json"),
            reward_runtime_distfs_probe_state_path: root
                .join("reward-runtime-distfs-probe-state.json"),
            reward_runtime_report_dir: root.join("reward-runtime-report"),
            reward_runtime_storage_metrics_path: root.join("reward-runtime-storage-metrics.json"),
        };
        let shared = init_shared_storage_metrics(StorageProfile::DevLocal);
        let snapshot = refresh_shared_storage_metrics(
            &shared,
            &paths,
            StorageProfile::DevLocal,
            Some("runtime degraded".to_string()),
        )
        .expect("refresh should persist storage metrics");
        let bytes = fs::read(paths.reward_runtime_storage_metrics_path.as_path())
            .expect("read storage metrics state file");
        let text = String::from_utf8(bytes).expect("utf8");
        assert!(text.contains("dev_local"));
        assert!(text.contains("runtime degraded"));
        assert!(text.contains("effective_budget"));
        assert!(text.contains("replay_summary"));
        assert_eq!(
            snapshot.degraded_reason,
            Some("runtime degraded".to_string())
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn collect_storage_metrics_reuses_ref_count_cache_for_unchanged_records() {
        let root = temp_dir("ref-cache");
        let paths = RuntimePaths {
            runtime_root: root.clone(),
            execution_bridge_state_path: root.join("bridge-state.json"),
            execution_world_dir: root.join("reward-runtime-execution-world"),
            execution_records_dir: root.join("reward-runtime-execution-records"),
            storage_root: root.join("store"),
            replication_root: root.join("replication"),
            reward_runtime_state_path: root.join("reward-runtime-state.json"),
            reward_runtime_distfs_probe_state_path: root
                .join("reward-runtime-distfs-probe-state.json"),
            reward_runtime_report_dir: root.join("reward-runtime-report"),
            reward_runtime_storage_metrics_path: root.join("reward-runtime-storage-metrics.json"),
        };
        fs::create_dir_all(paths.execution_records_dir.as_path()).expect("create records dir");

        fs::write(
            paths
                .execution_records_dir
                .join("00000000000000000001.json"),
            r#"{"latest_state_ref":"cas:a","snapshot_ref":"cas:b"}"#,
        )
        .expect("write record 1");

        let mut cache = ExecutionRefCountCache::default();
        let first =
            collect_storage_metrics(&paths, StorageProfile::ReleaseDefault, None, &mut cache);
        assert_eq!(first.ref_count, 2);
        assert_eq!(cache.record_ref_counts.len(), 1);

        fs::write(
            paths
                .execution_records_dir
                .join("00000000000000000002.json"),
            r#"{"journal_ref":"cas:c","commit_log_ref":"cas:d","nested":{"module_ref":"cas:e"}}"#,
        )
        .expect("write record 2");

        let second =
            collect_storage_metrics(&paths, StorageProfile::ReleaseDefault, None, &mut cache);
        assert_eq!(second.ref_count, 5);
        assert_eq!(cache.record_ref_counts.len(), 2);
        assert_eq!(cache.record_ref_counts.get(&1).map(|entry| entry.ref_count), Some(2));
        assert_eq!(cache.record_ref_counts.get(&2).map(|entry| entry.ref_count), Some(3));

        fs::remove_file(
            paths
                .execution_records_dir
                .join("00000000000000000001.json"),
        )
        .expect("remove record 1");

        let third =
            collect_storage_metrics(&paths, StorageProfile::ReleaseDefault, None, &mut cache);
        assert_eq!(third.ref_count, 3);
        assert_eq!(cache.record_ref_counts.len(), 1);
        assert_eq!(
            cache.record_ref_counts.get(&2).map(|entry| entry.ref_count),
            Some(3)
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn collect_storage_metrics_refreshes_cached_ref_count_after_in_place_record_update() {
        let root = temp_dir("ref-cache-in-place-update");
        let paths = RuntimePaths {
            runtime_root: root.clone(),
            execution_bridge_state_path: root.join("bridge-state.json"),
            execution_world_dir: root.join("reward-runtime-execution-world"),
            execution_records_dir: root.join("reward-runtime-execution-records"),
            storage_root: root.join("store"),
            replication_root: root.join("replication"),
            reward_runtime_state_path: root.join("reward-runtime-state.json"),
            reward_runtime_distfs_probe_state_path: root
                .join("reward-runtime-distfs-probe-state.json"),
            reward_runtime_report_dir: root.join("reward-runtime-report"),
            reward_runtime_storage_metrics_path: root.join("reward-runtime-storage-metrics.json"),
        };
        fs::create_dir_all(paths.execution_records_dir.as_path()).expect("create records dir");

        let record_path = paths
            .execution_records_dir
            .join("00000000000000000032.json");
        fs::write(
            record_path.as_path(),
            r#"{"latest_state_ref":"cas:a","snapshot_ref":"cas:b","checkpoint_ref":"checkpoint:32"}"#,
        )
        .expect("write initial record");

        let mut cache = ExecutionRefCountCache::default();
        let first =
            collect_storage_metrics(&paths, StorageProfile::ReleaseDefault, None, &mut cache);
        assert_eq!(first.ref_count, 3);
        assert_eq!(
            cache
                .record_ref_counts
                .get(&32)
                .map(|entry| entry.ref_count),
            Some(3)
        );

        std::thread::sleep(std::time::Duration::from_millis(5));
        fs::write(
            record_path.as_path(),
            r#"{"latest_state_ref":"cas:a","snapshot_ref":"cas:b"}"#,
        )
        .expect("rewrite record without checkpoint ref");

        let second =
            collect_storage_metrics(&paths, StorageProfile::ReleaseDefault, None, &mut cache);
        assert_eq!(second.ref_count, 2);
        assert_eq!(
            cache
                .record_ref_counts
                .get(&32)
                .map(|entry| entry.ref_count),
            Some(2)
        );

        let _ = fs::remove_dir_all(root);
    }
}
