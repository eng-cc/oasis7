use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use oasis7::runtime::LocalCasStore;

use super::{
    write_bytes_atomic, ExecutionBridgePinSet, ExecutionBridgeRecord,
    ExecutionCheckpointLatestPointer, ExecutionCheckpointManifest,
    ExecutionCheckpointManifestHashPayload, EXECUTION_BRIDGE_RECORD_SCHEMA_V2,
    EXECUTION_CHECKPOINT_MANIFEST_SCHEMA_V1,
};

impl ExecutionCheckpointManifest {
    pub(super) fn new(
        world_id: String,
        height: u64,
        execution_block_hash: String,
        execution_state_root: String,
        latest_state_ref: String,
        snapshot_ref: Option<String>,
        journal_ref: Option<String>,
        created_at_ms: i64,
    ) -> Result<Self, String> {
        let checkpoint_id = execution_checkpoint_id(height, execution_block_hash.as_str());
        let mut pinned_refs = vec![latest_state_ref.clone()];
        if let Some(snapshot_ref) = snapshot_ref.as_ref() {
            pinned_refs.push(snapshot_ref.clone());
        }
        if let Some(journal_ref) = journal_ref.as_ref() {
            pinned_refs.push(journal_ref.clone());
        }
        pinned_refs.sort();
        pinned_refs.dedup();

        let mut manifest = Self {
            schema_version: EXECUTION_CHECKPOINT_MANIFEST_SCHEMA_V1,
            checkpoint_id,
            world_id,
            height,
            execution_block_hash,
            execution_state_root,
            latest_state_ref,
            snapshot_ref,
            journal_ref,
            pinned_refs,
            manifest_hash: String::new(),
            created_at_ms,
        };
        manifest.manifest_hash = manifest.compute_manifest_hash()?;
        Ok(manifest)
    }

    fn compute_manifest_hash(&self) -> Result<String, String> {
        let payload = ExecutionCheckpointManifestHashPayload {
            schema_version: self.schema_version,
            checkpoint_id: self.checkpoint_id.as_str(),
            world_id: self.world_id.as_str(),
            height: self.height,
            execution_block_hash: self.execution_block_hash.as_str(),
            execution_state_root: self.execution_state_root.as_str(),
            latest_state_ref: self.latest_state_ref.as_str(),
            snapshot_ref: self.snapshot_ref.as_deref(),
            journal_ref: self.journal_ref.as_deref(),
            pinned_refs: self.pinned_refs.as_slice(),
            created_at_ms: self.created_at_ms,
        };
        Ok(oasis7::runtime::blake3_hex(super::to_cbor(payload)?.as_slice()))
    }

    pub(super) fn validate(&self) -> Result<(), String> {
        if self.schema_version < EXECUTION_CHECKPOINT_MANIFEST_SCHEMA_V1 {
            return Err(format!(
                "execution checkpoint manifest {} has invalid schema_version={}",
                self.checkpoint_id, self.schema_version
            ));
        }
        if self.height == 0 {
            return Err(format!(
                "execution checkpoint manifest {} has invalid height=0",
                self.checkpoint_id
            ));
        }
        if self.latest_state_ref.is_empty() {
            return Err(format!(
                "execution checkpoint manifest {} missing latest_state_ref",
                self.checkpoint_id
            ));
        }
        let mut expected_pins = vec![self.latest_state_ref.clone()];
        if let Some(snapshot_ref) = self.snapshot_ref.as_ref() {
            expected_pins.push(snapshot_ref.clone());
        }
        if let Some(journal_ref) = self.journal_ref.as_ref() {
            expected_pins.push(journal_ref.clone());
        }
        expected_pins.sort();
        expected_pins.dedup();
        if expected_pins != self.pinned_refs {
            return Err(format!(
                "execution checkpoint manifest {} pin-set mismatch expected={:?} actual={:?}",
                self.checkpoint_id, expected_pins, self.pinned_refs
            ));
        }
        let expected_hash = self.compute_manifest_hash()?;
        if self.manifest_hash != expected_hash {
            return Err(format!(
                "execution checkpoint manifest {} hash mismatch expected={} actual={}",
                self.checkpoint_id, expected_hash, self.manifest_hash
            ));
        }
        Ok(())
    }
}

fn execution_checkpoint_id(height: u64, execution_block_hash: &str) -> String {
    let short_hash: String = execution_block_hash.chars().take(16).collect();
    format!("checkpoint-{:020}-{short_hash}", height)
}

pub(super) fn execution_checkpoint_root_dir(execution_records_dir: &Path) -> std::path::PathBuf {
    execution_records_dir.join("checkpoints")
}

pub(super) fn execution_checkpoint_manifest_path(
    execution_records_dir: &Path,
    height: u64,
) -> std::path::PathBuf {
    execution_checkpoint_root_dir(execution_records_dir)
        .join(format!("{:020}", height))
        .join("manifest.json")
}

pub(super) fn execution_checkpoint_latest_path(
    execution_records_dir: &Path,
) -> std::path::PathBuf {
    execution_checkpoint_root_dir(execution_records_dir).join("latest.json")
}

pub(super) fn execution_checkpoint_manifest_rel_path(height: u64) -> String {
    format!("{:020}/manifest.json", height)
}

pub(super) fn list_execution_checkpoint_heights(
    execution_records_dir: &Path,
) -> Result<Vec<u64>, String> {
    let checkpoint_root = execution_checkpoint_root_dir(execution_records_dir);
    if !checkpoint_root.exists() {
        return Ok(Vec::new());
    }

    let mut heights = Vec::new();
    for entry in fs::read_dir(checkpoint_root.as_path()).map_err(|err| {
        format!(
            "read execution checkpoint root {} failed: {}",
            checkpoint_root.display(),
            err
        )
    })? {
        let entry = entry.map_err(|err| {
            format!(
                "read execution checkpoint dir entry under {} failed: {}",
                checkpoint_root.display(),
                err
            )
        })?;
        let file_type = entry.file_type().map_err(|err| {
            format!(
                "read execution checkpoint dir entry type {} failed: {}",
                entry.path().display(),
                err
            )
        })?;
        if !file_type.is_dir() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
            continue;
        };
        let Ok(height) = name.parse::<u64>() else {
            continue;
        };
        if execution_checkpoint_manifest_path(execution_records_dir, height).exists() {
            heights.push(height);
        }
    }

    heights.sort_unstable();
    heights.dedup();
    Ok(heights)
}

pub(super) fn persist_execution_checkpoint_manifest(
    execution_records_dir: &Path,
    manifest: &ExecutionCheckpointManifest,
) -> Result<(), String> {
    manifest.validate()?;
    let manifest_path = execution_checkpoint_manifest_path(execution_records_dir, manifest.height);
    let manifest_parent = manifest_path.parent().ok_or_else(|| {
        format!(
            "execution checkpoint manifest path {} missing parent",
            manifest_path.display()
        )
    })?;
    fs::create_dir_all(manifest_parent).map_err(|err| {
        format!(
            "create execution checkpoint dir {} failed: {}",
            manifest_parent.display(),
            err
        )
    })?;
    let manifest_bytes = serde_json::to_vec_pretty(manifest)
        .map_err(|err| format!("serialize execution checkpoint manifest failed: {}", err))?;
    write_bytes_atomic(manifest_path.as_path(), manifest_bytes.as_slice())?;

    let root_dir = execution_checkpoint_root_dir(execution_records_dir);
    fs::create_dir_all(root_dir.as_path()).map_err(|err| {
        format!(
            "create execution checkpoint root {} failed: {}",
            root_dir.display(),
            err
        )
    })?;
    let latest = ExecutionCheckpointLatestPointer {
        schema_version: EXECUTION_CHECKPOINT_MANIFEST_SCHEMA_V1,
        checkpoint_id: manifest.checkpoint_id.clone(),
        height: manifest.height,
        manifest_hash: manifest.manifest_hash.clone(),
        manifest_rel_path: execution_checkpoint_manifest_rel_path(manifest.height),
        updated_at_ms: manifest.created_at_ms,
    };
    let latest_bytes = serde_json::to_vec_pretty(&latest).map_err(|err| {
        format!(
            "serialize execution checkpoint latest pointer failed: {}",
            err
        )
    })?;
    let latest_path = execution_checkpoint_latest_path(execution_records_dir);
    write_bytes_atomic(latest_path.as_path(), latest_bytes.as_slice())
}

pub(super) fn load_execution_checkpoint_manifest(
    path: &Path,
) -> Result<ExecutionCheckpointManifest, String> {
    let bytes = fs::read(path).map_err(|err| {
        format!(
            "read execution checkpoint manifest {} failed: {}",
            path.display(),
            err
        )
    })?;
    let manifest = serde_json::from_slice::<ExecutionCheckpointManifest>(bytes.as_slice())
        .map_err(|err| {
            format!(
                "parse execution checkpoint manifest {} failed: {}",
                path.display(),
                err
            )
        })?;
    manifest.validate()?;
    Ok(manifest)
}

pub(super) fn load_latest_execution_checkpoint_manifest(
    execution_records_dir: &Path,
) -> Result<Option<ExecutionCheckpointManifest>, String> {
    let latest_path = execution_checkpoint_latest_path(execution_records_dir);
    if !latest_path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(latest_path.as_path()).map_err(|err| {
        format!(
            "read execution checkpoint latest pointer {} failed: {}",
            latest_path.display(),
            err
        )
    })?;
    let latest = serde_json::from_slice::<ExecutionCheckpointLatestPointer>(bytes.as_slice())
        .map_err(|err| {
            format!(
                "parse execution checkpoint latest pointer {} failed: {}",
                latest_path.display(),
                err
            )
        })?;
    let manifest_path =
        execution_checkpoint_root_dir(execution_records_dir).join(latest.manifest_rel_path);
    let manifest = load_execution_checkpoint_manifest(manifest_path.as_path())?;
    if manifest.height != latest.height {
        return Err(format!(
            "execution checkpoint latest pointer height mismatch expected={} actual={}",
            latest.height, manifest.height
        ));
    }
    if manifest.manifest_hash != latest.manifest_hash {
        return Err(format!(
            "execution checkpoint latest pointer hash mismatch expected={} actual={}",
            latest.manifest_hash, manifest.manifest_hash
        ));
    }
    if manifest.checkpoint_id != latest.checkpoint_id {
        return Err(format!(
            "execution checkpoint latest pointer id mismatch expected={} actual={}",
            latest.checkpoint_id, manifest.checkpoint_id
        ));
    }
    Ok(Some(manifest))
}

pub(super) fn execution_bridge_record_path(
    execution_records_dir: &Path,
    height: u64,
) -> std::path::PathBuf {
    execution_records_dir.join(format!("{:020}.json", height))
}

pub(super) fn list_execution_bridge_record_heights(
    execution_records_dir: &Path,
) -> Result<Vec<u64>, String> {
    if !execution_records_dir.exists() {
        return Ok(Vec::new());
    }

    let mut heights = Vec::new();
    for entry in fs::read_dir(execution_records_dir).map_err(|err| {
        format!(
            "read execution records dir {} failed: {}",
            execution_records_dir.display(),
            err
        )
    })? {
        let entry = entry.map_err(|err| {
            format!(
                "read execution record dir entry under {} failed: {}",
                execution_records_dir.display(),
                err
            )
        })?;
        let file_type = entry.file_type().map_err(|err| {
            format!(
                "read execution record dir entry type {} failed: {}",
                entry.path().display(),
                err
            )
        })?;
        if !file_type.is_file() {
            continue;
        }
        let Some(file_name) = entry.file_name().to_str().map(ToOwned::to_owned) else {
            continue;
        };
        if file_name == "latest.json" || !file_name.ends_with(".json") {
            continue;
        }
        let Some(stem) = file_name.strip_suffix(".json") else {
            continue;
        };
        let Ok(height) = stem.parse::<u64>() else {
            continue;
        };
        heights.push(height);
    }

    heights.sort_unstable();
    heights.dedup();
    Ok(heights)
}

fn maybe_insert_pin_ref(pinned_refs: &mut BTreeSet<String>, content_ref: Option<&str>) {
    if let Some(content_ref) = content_ref.filter(|content_ref| !content_ref.is_empty()) {
        pinned_refs.insert(content_ref.to_string());
    }
}

fn collect_execution_bridge_record_retained_refs(
    record: &ExecutionBridgeRecord,
    retain_latest_head: bool,
    retain_hot_window: bool,
    pinned_refs: &mut BTreeSet<String>,
) {
    maybe_insert_pin_ref(pinned_refs, record.commit_log_ref.as_deref());
    maybe_insert_pin_ref(pinned_refs, record.external_effect_ref.as_deref());

    if retain_latest_head {
        maybe_insert_pin_ref(pinned_refs, record.latest_state_ref.as_deref());
    }
    if retain_latest_head || retain_hot_window {
        maybe_insert_pin_ref(pinned_refs, record.snapshot_ref.as_deref());
        maybe_insert_pin_ref(pinned_refs, record.journal_ref.as_deref());
        if let Some(simulator_mirror) = record.simulator_mirror.as_ref() {
            pinned_refs.insert(simulator_mirror.snapshot_ref.clone());
            pinned_refs.insert(simulator_mirror.journal_ref.clone());
        }
    }
}

fn collect_execution_checkpoint_retained_refs(
    execution_records_dir: &Path,
    pinned_refs: &mut BTreeSet<String>,
) -> Result<(), String> {
    let checkpoint_root = execution_checkpoint_root_dir(execution_records_dir);
    if !checkpoint_root.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(checkpoint_root.as_path()).map_err(|err| {
        format!(
            "read execution checkpoint root {} failed: {}",
            checkpoint_root.display(),
            err
        )
    })? {
        let entry = entry.map_err(|err| {
            format!(
                "read execution checkpoint dir entry under {} failed: {}",
                checkpoint_root.display(),
                err
            )
        })?;
        let file_type = entry.file_type().map_err(|err| {
            format!(
                "read execution checkpoint dir entry type {} failed: {}",
                entry.path().display(),
                err
            )
        })?;
        if !file_type.is_dir() {
            continue;
        }
        let manifest_path = entry.path().join("manifest.json");
        if !manifest_path.exists() {
            continue;
        }
        let manifest = load_execution_checkpoint_manifest(manifest_path.as_path())?;
        pinned_refs.extend(manifest.pinned_refs);
    }

    Ok(())
}

fn build_execution_bridge_pin_set(
    execution_records_dir: &Path,
    hot_window_heights: u64,
) -> Result<ExecutionBridgePinSet, String> {
    let record_heights = list_execution_bridge_record_heights(execution_records_dir)?;
    let checkpoint_heights = list_execution_checkpoint_heights(execution_records_dir)?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let Some(latest_height) = record_heights.last().copied() else {
        let mut pin_set = ExecutionBridgePinSet {
            checkpoint_heights,
            ..ExecutionBridgePinSet::default()
        };
        collect_execution_checkpoint_retained_refs(
            execution_records_dir,
            &mut pin_set.pinned_refs,
        )?;
        return Ok(pin_set);
    };

    let retained_hot_window = hot_window_heights.max(1);
    let hot_window_start_height =
        latest_height.saturating_sub(retained_hot_window.saturating_sub(1));
    let mut pin_set = ExecutionBridgePinSet {
        latest_height: Some(latest_height),
        hot_window_start_height: Some(hot_window_start_height),
        checkpoint_heights,
        pinned_refs: BTreeSet::new(),
    };

    for height in record_heights {
        let record = load_execution_bridge_record(
            execution_bridge_record_path(execution_records_dir, height).as_path(),
        )?;
        collect_execution_bridge_record_retained_refs(
            &record,
            record.height == latest_height,
            record.height >= hot_window_start_height,
            &mut pin_set.pinned_refs,
        );
    }
    collect_execution_checkpoint_retained_refs(execution_records_dir, &mut pin_set.pinned_refs)?;

    Ok(pin_set)
}

pub(super) fn sync_execution_bridge_pin_set(
    execution_records_dir: &Path,
    execution_store: &LocalCasStore,
    hot_window_heights: u64,
) -> Result<ExecutionBridgePinSet, String> {
    let pin_set = build_execution_bridge_pin_set(execution_records_dir, hot_window_heights)?;
    let current_pins = execution_store
        .list_pins()
        .map_err(|err| format!("list execution store pins failed: {:?}", err))?
        .into_iter()
        .collect::<BTreeSet<_>>();

    for stale_ref in current_pins.difference(&pin_set.pinned_refs) {
        execution_store
            .unpin(stale_ref.as_str())
            .map_err(|err| format!("unpin execution store ref {} failed: {:?}", stale_ref, err))?;
    }
    for pinned_ref in pin_set.pinned_refs.difference(&current_pins) {
        execution_store
            .pin(pinned_ref.as_str())
            .map_err(|err| format!("pin execution store ref {} failed: {:?}", pinned_ref, err))?;
    }

    Ok(pin_set)
}

fn list_execution_bridge_pre_v2_heights(
    execution_records_dir: &Path,
) -> Result<Vec<u64>, String> {
    let mut pre_v2_heights = Vec::new();
    for height in list_execution_bridge_record_heights(execution_records_dir)? {
        let record = load_execution_bridge_record(
            execution_bridge_record_path(execution_records_dir, height).as_path(),
        )?;
        if record.schema_version < EXECUTION_BRIDGE_RECORD_SCHEMA_V2 {
            pre_v2_heights.push(height);
        }
    }
    Ok(pre_v2_heights)
}

fn update_execution_bridge_record_checkpoint_ref(
    execution_records_dir: &Path,
    height: u64,
    checkpoint_ref: Option<String>,
) -> Result<(), String> {
    let path = execution_bridge_record_path(execution_records_dir, height);
    if !path.exists() {
        return Ok(());
    }
    let mut record = load_execution_bridge_record(path.as_path())?;
    record.checkpoint_ref = checkpoint_ref;
    persist_execution_bridge_record_only(execution_records_dir, &record)?;
    Ok(())
}

fn prune_execution_checkpoint_manifests(
    execution_records_dir: &Path,
    checkpoint_keep_latest: usize,
) -> Result<Vec<u64>, String> {
    let checkpoint_heights = list_execution_checkpoint_heights(execution_records_dir)?;
    let retained_count = checkpoint_keep_latest.max(1);
    if checkpoint_heights.len() <= retained_count {
        return Ok(Vec::new());
    }

    let prune_heights = checkpoint_heights[..checkpoint_heights.len() - retained_count].to_vec();
    for height in &prune_heights {
        let manifest_path = execution_checkpoint_manifest_path(execution_records_dir, *height);
        let manifest_dir = manifest_path.parent().ok_or_else(|| {
            format!(
                "execution checkpoint manifest path {} missing parent",
                manifest_path.display()
            )
        })?;
        if manifest_dir.exists() {
            fs::remove_dir_all(manifest_dir).map_err(|err| {
                format!(
                    "remove execution checkpoint dir {} failed: {}",
                    manifest_dir.display(),
                    err
                )
            })?;
        }
        update_execution_bridge_record_checkpoint_ref(execution_records_dir, *height, None)?;
    }

    Ok(prune_heights)
}

pub(super) fn maybe_persist_execution_checkpoint_for_record(
    execution_records_dir: &Path,
    record: &ExecutionBridgeRecord,
    checkpoint_interval_heights: u64,
    checkpoint_keep_latest: usize,
) -> Result<Option<String>, String> {
    if checkpoint_interval_heights == 0
        || record.height == 0
        || record.height % checkpoint_interval_heights != 0
    {
        return Ok(None);
    }

    let latest_state_ref = record.latest_state_ref.clone().ok_or_else(|| {
        format!(
            "execution checkpoint height {} missing latest_state_ref",
            record.height
        )
    })?;
    let manifest = ExecutionCheckpointManifest::new(
        record.world_id.clone(),
        record.height,
        record.execution_block_hash.clone(),
        record.execution_state_root.clone(),
        latest_state_ref,
        record.snapshot_ref.clone(),
        record.journal_ref.clone(),
        record.timestamp_ms,
    )?;
    persist_execution_checkpoint_manifest(execution_records_dir, &manifest)?;
    let checkpoint_ref = execution_checkpoint_manifest_rel_path(record.height);
    prune_execution_checkpoint_manifests(execution_records_dir, checkpoint_keep_latest)?;
    Ok(Some(checkpoint_ref))
}

fn compact_execution_bridge_records(
    execution_records_dir: &Path,
    pin_set: &ExecutionBridgePinSet,
) -> Result<usize, String> {
    let record_heights = list_execution_bridge_record_heights(execution_records_dir)?;
    let latest_height = pin_set.latest_height;
    let hot_window_start_height = pin_set.hot_window_start_height.unwrap_or(u64::MAX);
    let mut rewritten_records = 0_usize;

    for height in record_heights {
        let path = execution_bridge_record_path(execution_records_dir, height);
        let mut record = load_execution_bridge_record(path.as_path())?;
        let original_record = record.clone();
        let retain_latest_head = latest_height == Some(height);
        let retain_hot_window = height >= hot_window_start_height;
        let retain_checkpoint = pin_set.checkpoint_heights.contains(&height);

        if !retain_latest_head && !retain_hot_window {
            record.latest_state_ref = None;
            record.snapshot_ref = None;
            record.journal_ref = None;
            record.simulator_mirror = None;
            if !retain_checkpoint {
                record.checkpoint_ref = None;
            }
        }

        if record != original_record {
            if retain_latest_head {
                persist_execution_bridge_record(execution_records_dir, &record)?;
            } else {
                persist_execution_bridge_record_only(execution_records_dir, &record)?;
            }
            rewritten_records = rewritten_records.saturating_add(1);
        }
    }

    Ok(rewritten_records)
}

pub(super) fn run_execution_bridge_retention_maintenance(
    execution_records_dir: &Path,
    execution_store: &LocalCasStore,
    hot_window_heights: u64,
) -> Result<u64, String> {
    let pre_v2_heights = list_execution_bridge_pre_v2_heights(execution_records_dir)?;
    if !pre_v2_heights.is_empty() {
        sync_execution_bridge_pin_set(execution_records_dir, execution_store, hot_window_heights)?;
        return Ok(0);
    }

    let pin_set = build_execution_bridge_pin_set(execution_records_dir, hot_window_heights)?;
    compact_execution_bridge_records(execution_records_dir, &pin_set)?;
    sync_execution_bridge_pin_set(execution_records_dir, execution_store, hot_window_heights)?;
    execution_store
        .prune_orphan_blobs()
        .map_err(|err| format!("prune execution store orphan blobs failed: {:?}", err))
}

pub(super) fn load_execution_bridge_record(path: &Path) -> Result<ExecutionBridgeRecord, String> {
    let bytes = fs::read(path).map_err(|err| {
        format!(
            "read execution bridge record {} failed: {}",
            path.display(),
            err
        )
    })?;
    serde_json::from_slice::<ExecutionBridgeRecord>(bytes.as_slice()).map_err(|err| {
        format!(
            "parse execution bridge record {} failed: {}",
            path.display(),
            err
        )
    })
}

fn normalize_execution_bridge_record_for_persist(
    record: &ExecutionBridgeRecord,
) -> ExecutionBridgeRecord {
    let mut normalized = record.clone();
    normalized.schema_version = EXECUTION_BRIDGE_RECORD_SCHEMA_V2;
    if normalized.latest_state_ref.is_none() {
        normalized.latest_state_ref = normalized.snapshot_ref.clone();
    }
    normalized
}

pub(super) fn persist_execution_bridge_record_only(
    execution_records_dir: &Path,
    record: &ExecutionBridgeRecord,
) -> Result<Vec<u8>, String> {
    let normalized = normalize_execution_bridge_record_for_persist(record);
    let bytes = serde_json::to_vec_pretty(&normalized)
        .map_err(|err| format!("serialize execution bridge record failed: {}", err))?;
    let path = execution_bridge_record_path(execution_records_dir, normalized.height);
    write_bytes_atomic(path.as_path(), bytes.as_slice())?;
    Ok(bytes)
}

pub(super) fn persist_execution_bridge_record(
    execution_records_dir: &Path,
    record: &ExecutionBridgeRecord,
) -> Result<(), String> {
    let bytes = persist_execution_bridge_record_only(execution_records_dir, record)?;
    let latest_path = execution_records_dir.join("latest.json");
    write_bytes_atomic(latest_path.as_path(), bytes.as_slice())
}
