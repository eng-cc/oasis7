use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use oasis7_distfs::LocalCasStore;
use serde::{Deserialize, Serialize};

use super::super::error::WorldError;
use crate::tiered_file_log;

use super::types::{
    MembershipRevocationAlertDeadLetterRecord, MembershipRevocationAlertDeliveryMetrics,
};

const DEFAULT_MAX_DEAD_LETTER_RECORDS_PER_STREAM: usize = 10_000;
const DEFAULT_MAX_DELIVERY_METRICS_PER_STREAM: usize = 10_000;
const DEAD_LETTER_ARCHIVE_COLD_SEGMENT_MAX_LINES: usize = 256;

type DeadLetterRecords =
    Arc<Mutex<BTreeMap<(String, String), Vec<MembershipRevocationAlertDeadLetterRecord>>>>;
type DeliveryMetricsRecords =
    Arc<Mutex<BTreeMap<(String, String), Vec<(i64, MembershipRevocationAlertDeliveryMetrics)>>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MembershipRevocationDeadLetterRetention {
    pub max_dead_letter_records_per_stream: usize,
    pub max_delivery_metrics_per_stream: usize,
}

impl MembershipRevocationDeadLetterRetention {
    fn normalized(self) -> Self {
        Self {
            max_dead_letter_records_per_stream: self.max_dead_letter_records_per_stream.max(1),
            max_delivery_metrics_per_stream: self.max_delivery_metrics_per_stream.max(1),
        }
    }
}

impl Default for MembershipRevocationDeadLetterRetention {
    fn default() -> Self {
        Self {
            max_dead_letter_records_per_stream: DEFAULT_MAX_DEAD_LETTER_RECORDS_PER_STREAM,
            max_delivery_metrics_per_stream: DEFAULT_MAX_DELIVERY_METRICS_PER_STREAM,
        }
    }
}

pub trait MembershipRevocationAlertDeadLetterStore {
    fn append(&self, record: &MembershipRevocationAlertDeadLetterRecord) -> Result<(), WorldError>;

    fn list(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<MembershipRevocationAlertDeadLetterRecord>, WorldError>;

    fn replace(
        &self,
        world_id: &str,
        node_id: &str,
        records: &[MembershipRevocationAlertDeadLetterRecord],
    ) -> Result<(), WorldError>;

    fn append_delivery_metrics(
        &self,
        world_id: &str,
        node_id: &str,
        exported_at_ms: i64,
        metrics: &MembershipRevocationAlertDeliveryMetrics,
    ) -> Result<(), WorldError>;

    fn list_delivery_metrics(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<(i64, MembershipRevocationAlertDeliveryMetrics)>, WorldError>;
}

fn retain_recent_entries<T>(entries: &mut Vec<T>, max_entries: usize) {
    let overflow = entries.len().saturating_sub(max_entries.max(1));
    if overflow > 0 {
        entries.drain(0..overflow);
    }
}

#[derive(Debug, Clone, Default)]
pub struct NoopMembershipRevocationAlertDeadLetterStore;

impl MembershipRevocationAlertDeadLetterStore for NoopMembershipRevocationAlertDeadLetterStore {
    fn append(
        &self,
        _record: &MembershipRevocationAlertDeadLetterRecord,
    ) -> Result<(), WorldError> {
        Ok(())
    }

    fn list(
        &self,
        _world_id: &str,
        _node_id: &str,
    ) -> Result<Vec<MembershipRevocationAlertDeadLetterRecord>, WorldError> {
        Ok(Vec::new())
    }

    fn replace(
        &self,
        _world_id: &str,
        _node_id: &str,
        _records: &[MembershipRevocationAlertDeadLetterRecord],
    ) -> Result<(), WorldError> {
        Ok(())
    }

    fn append_delivery_metrics(
        &self,
        _world_id: &str,
        _node_id: &str,
        _exported_at_ms: i64,
        _metrics: &MembershipRevocationAlertDeliveryMetrics,
    ) -> Result<(), WorldError> {
        Ok(())
    }

    fn list_delivery_metrics(
        &self,
        _world_id: &str,
        _node_id: &str,
    ) -> Result<Vec<(i64, MembershipRevocationAlertDeliveryMetrics)>, WorldError> {
        Ok(Vec::new())
    }
}

#[derive(Debug, Clone)]
pub struct InMemoryMembershipRevocationAlertDeadLetterStore {
    records: DeadLetterRecords,
    delivery_metrics: DeliveryMetricsRecords,
    retention: MembershipRevocationDeadLetterRetention,
}

impl Default for InMemoryMembershipRevocationAlertDeadLetterStore {
    fn default() -> Self {
        Self {
            records: Arc::new(Mutex::new(BTreeMap::new())),
            delivery_metrics: Arc::new(Mutex::new(BTreeMap::new())),
            retention: MembershipRevocationDeadLetterRetention::default(),
        }
    }
}

impl InMemoryMembershipRevocationAlertDeadLetterStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_retention(retention: MembershipRevocationDeadLetterRetention) -> Self {
        Self {
            retention: retention.normalized(),
            ..Self::default()
        }
    }

    pub fn list(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<MembershipRevocationAlertDeadLetterRecord>, WorldError> {
        let key = super::normalized_schedule_key(world_id, node_id)?;
        let guard = self.records.lock().map_err(|_| {
            WorldError::Io("membership revocation dead-letter store lock poisoned".into())
        })?;
        Ok(guard.get(&key).cloned().unwrap_or_default())
    }

    pub fn list_delivery_metrics(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<(i64, MembershipRevocationAlertDeliveryMetrics)>, WorldError> {
        let key = super::normalized_schedule_key(world_id, node_id)?;
        let guard = self.delivery_metrics.lock().map_err(|_| {
            WorldError::Io("membership revocation dead-letter metrics lock poisoned".into())
        })?;
        Ok(guard.get(&key).cloned().unwrap_or_default())
    }
}

impl MembershipRevocationAlertDeadLetterStore for InMemoryMembershipRevocationAlertDeadLetterStore {
    fn append(&self, record: &MembershipRevocationAlertDeadLetterRecord) -> Result<(), WorldError> {
        let key = super::normalized_schedule_key(&record.world_id, &record.node_id)?;
        let mut guard = self.records.lock().map_err(|_| {
            WorldError::Io("membership revocation dead-letter store lock poisoned".into())
        })?;
        let records = guard.entry(key).or_default();
        records.push(record.clone());
        retain_recent_entries(records, self.retention.max_dead_letter_records_per_stream);
        Ok(())
    }

    fn list(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<MembershipRevocationAlertDeadLetterRecord>, WorldError> {
        InMemoryMembershipRevocationAlertDeadLetterStore::list(self, world_id, node_id)
    }

    fn replace(
        &self,
        world_id: &str,
        node_id: &str,
        records: &[MembershipRevocationAlertDeadLetterRecord],
    ) -> Result<(), WorldError> {
        let key = super::normalized_schedule_key(world_id, node_id)?;
        let mut guard = self.records.lock().map_err(|_| {
            WorldError::Io("membership revocation dead-letter store lock poisoned".into())
        })?;
        if records.is_empty() {
            guard.remove(&key);
        } else {
            let mut bounded = records.to_vec();
            retain_recent_entries(
                &mut bounded,
                self.retention.max_dead_letter_records_per_stream,
            );
            guard.insert(key, bounded);
        }
        Ok(())
    }

    fn append_delivery_metrics(
        &self,
        world_id: &str,
        node_id: &str,
        exported_at_ms: i64,
        metrics: &MembershipRevocationAlertDeliveryMetrics,
    ) -> Result<(), WorldError> {
        let key = super::normalized_schedule_key(world_id, node_id)?;
        let mut guard = self.delivery_metrics.lock().map_err(|_| {
            WorldError::Io("membership revocation dead-letter metrics lock poisoned".into())
        })?;
        let values = guard.entry(key).or_default();
        values.push((exported_at_ms, metrics.clone()));
        retain_recent_entries(values, self.retention.max_delivery_metrics_per_stream);
        Ok(())
    }

    fn list_delivery_metrics(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<(i64, MembershipRevocationAlertDeliveryMetrics)>, WorldError> {
        InMemoryMembershipRevocationAlertDeadLetterStore::list_delivery_metrics(
            self, world_id, node_id,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct FileMembershipRevocationAlertDeliveryMetricsLine {
    exported_at_ms: i64,
    metrics: MembershipRevocationAlertDeliveryMetrics,
}

#[derive(Debug, Clone)]
pub struct FileMembershipRevocationAlertDeadLetterStore {
    root_dir: PathBuf,
    cas_store: LocalCasStore,
    retention: MembershipRevocationDeadLetterRetention,
}

impl FileMembershipRevocationAlertDeadLetterStore {
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, WorldError> {
        Self::with_retention(root_dir, MembershipRevocationDeadLetterRetention::default())
    }

    pub fn with_retention(
        root_dir: impl Into<PathBuf>,
        retention: MembershipRevocationDeadLetterRetention,
    ) -> Result<Self, WorldError> {
        let root_dir = root_dir.into();
        fs::create_dir_all(&root_dir)?;
        Ok(Self {
            cas_store: LocalCasStore::new(root_dir.join("cas")),
            root_dir,
            retention: retention.normalized(),
        })
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    fn dead_letter_path(&self, world_id: &str, node_id: &str) -> Result<PathBuf, WorldError> {
        let (world_id, node_id) = super::normalized_schedule_key(world_id, node_id)?;
        Ok(self.root_dir.join(format!(
            "{world_id}.{node_id}.revocation-alert-dead-letter.jsonl"
        )))
    }

    fn dead_letter_archive_path(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<PathBuf, WorldError> {
        let (world_id, node_id) = super::normalized_schedule_key(world_id, node_id)?;
        Ok(self.root_dir.join(format!(
            "{world_id}.{node_id}.revocation-alert-dead-letter.archive.refs.jsonl"
        )))
    }

    fn delivery_metrics_path(&self, world_id: &str, node_id: &str) -> Result<PathBuf, WorldError> {
        let (world_id, node_id) = super::normalized_schedule_key(world_id, node_id)?;
        Ok(self.root_dir.join(format!(
            "{world_id}.{node_id}.revocation-alert-delivery-metrics.jsonl"
        )))
    }

    fn delivery_metrics_archive_path(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<PathBuf, WorldError> {
        let (world_id, node_id) = super::normalized_schedule_key(world_id, node_id)?;
        Ok(self.root_dir.join(format!(
            "{world_id}.{node_id}.revocation-alert-delivery-metrics.archive.refs.jsonl"
        )))
    }

    fn compact_jsonl_with_archive(
        &self,
        path: &Path,
        archive_path: &Path,
        max_entries: usize,
    ) -> Result<(), WorldError> {
        tiered_file_log::compact_jsonl_with_cas_offload(
            path,
            archive_path,
            &self.cas_store,
            max_entries,
            DEAD_LETTER_ARCHIVE_COLD_SEGMENT_MAX_LINES,
        )
    }

    pub fn list(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<MembershipRevocationAlertDeadLetterRecord>, WorldError> {
        let path = self.dead_letter_path(world_id, node_id)?;
        let archive_path = self.dead_letter_archive_path(world_id, node_id)?;
        let lines = tiered_file_log::collect_jsonl_lines_with_cas_refs(
            path.as_path(),
            archive_path.as_path(),
            &self.cas_store,
        )?;
        let mut records = Vec::new();
        for line in lines {
            records.push(serde_json::from_str(&line)?);
        }
        Ok(records)
    }

    pub fn list_delivery_metrics(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<(i64, MembershipRevocationAlertDeliveryMetrics)>, WorldError> {
        let path = self.delivery_metrics_path(world_id, node_id)?;
        let archive_path = self.delivery_metrics_archive_path(world_id, node_id)?;
        let raw_lines = tiered_file_log::collect_jsonl_lines_with_cas_refs(
            path.as_path(),
            archive_path.as_path(),
            &self.cas_store,
        )?;
        let mut metrics = Vec::new();
        for line in raw_lines {
            let parsed: FileMembershipRevocationAlertDeliveryMetricsLine =
                serde_json::from_str(&line)?;
            metrics.push((parsed.exported_at_ms, parsed.metrics));
        }
        Ok(metrics)
    }
}

impl MembershipRevocationAlertDeadLetterStore for FileMembershipRevocationAlertDeadLetterStore {
    fn append(&self, record: &MembershipRevocationAlertDeadLetterRecord) -> Result<(), WorldError> {
        let path = self.dead_letter_path(&record.world_id, &record.node_id)?;
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }

        let line = serde_json::to_string(record)?;
        let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        let archive_path = self.dead_letter_archive_path(&record.world_id, &record.node_id)?;
        self.compact_jsonl_with_archive(
            path.as_path(),
            archive_path.as_path(),
            self.retention.max_dead_letter_records_per_stream,
        )?;
        Ok(())
    }

    fn list(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<MembershipRevocationAlertDeadLetterRecord>, WorldError> {
        FileMembershipRevocationAlertDeadLetterStore::list(self, world_id, node_id)
    }

    fn replace(
        &self,
        world_id: &str,
        node_id: &str,
        records: &[MembershipRevocationAlertDeadLetterRecord],
    ) -> Result<(), WorldError> {
        let path = self.dead_letter_path(world_id, node_id)?;
        let archive_path = self.dead_letter_archive_path(world_id, node_id)?;
        if records.is_empty() {
            if path.exists() {
                fs::remove_file(&path)?;
            }
            if archive_path.exists() {
                fs::remove_file(archive_path)?;
            }
            return Ok(());
        }

        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }

        let mut payload = Vec::new();
        for record in records {
            payload.push(serde_json::to_string(record)?);
        }
        if archive_path.exists() {
            fs::remove_file(&archive_path)?;
        }
        fs::write(path.as_path(), payload.join("\n") + "\n")?;
        self.compact_jsonl_with_archive(
            path.as_path(),
            archive_path.as_path(),
            self.retention.max_dead_letter_records_per_stream,
        )?;
        Ok(())
    }

    fn append_delivery_metrics(
        &self,
        world_id: &str,
        node_id: &str,
        exported_at_ms: i64,
        metrics: &MembershipRevocationAlertDeliveryMetrics,
    ) -> Result<(), WorldError> {
        let path = self.delivery_metrics_path(world_id, node_id)?;
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }

        let line = serde_json::to_string(&FileMembershipRevocationAlertDeliveryMetricsLine {
            exported_at_ms,
            metrics: metrics.clone(),
        })?;
        let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        let archive_path = self.delivery_metrics_archive_path(world_id, node_id)?;
        self.compact_jsonl_with_archive(
            path.as_path(),
            archive_path.as_path(),
            self.retention.max_delivery_metrics_per_stream,
        )?;
        Ok(())
    }

    fn list_delivery_metrics(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<(i64, MembershipRevocationAlertDeliveryMetrics)>, WorldError> {
        FileMembershipRevocationAlertDeadLetterStore::list_delivery_metrics(self, world_id, node_id)
    }
}
