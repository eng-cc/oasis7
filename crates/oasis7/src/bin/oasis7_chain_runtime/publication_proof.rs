use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub(super) const MAX_PUBLICATION_EPISODE_RECORD_SCAN: usize = 255;

#[derive(Clone, Debug, Deserialize)]
pub(super) struct PublicationExecutionRecord {
    pub(super) world_id: String,
    pub(super) height: u64,
    pub(super) node_block_hash: Option<String>,
    #[serde(default)]
    pub(super) prev_node_block_hash: Option<String>,
    pub(super) execution_block_hash: String,
    pub(super) execution_state_root: String,
    pub(super) timestamp_ms: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PublicationProofErrorKind {
    RecordMissing,
    RecordMalformed,
    ContinuityInvalid,
    AncestryInvalid,
    ChronologyInvalid,
    ScanLimitExceeded,
}

#[derive(Clone, Debug)]
pub(super) struct PublicationProofError {
    pub(super) kind: PublicationProofErrorKind,
    pub(super) detail: String,
}

impl PublicationProofError {
    fn new(kind: PublicationProofErrorKind, detail: impl Into<String>) -> Self {
        Self {
            kind,
            detail: detail.into(),
        }
    }
}

pub(super) fn load_record(
    records_dir: &Path,
    height: u64,
) -> Result<PublicationExecutionRecord, PublicationProofError> {
    let bytes = std::fs::read(record_path(records_dir, height)).map_err(|error| {
        let kind = if error.kind() == ErrorKind::NotFound {
            PublicationProofErrorKind::RecordMissing
        } else {
            PublicationProofErrorKind::RecordMalformed
        };
        PublicationProofError::new(kind, format!("height={height}"))
    })?;
    serde_json::from_slice(&bytes).map_err(|_| {
        PublicationProofError::new(
            PublicationProofErrorKind::RecordMalformed,
            format!("height={height}"),
        )
    })
}

pub(super) fn validate_record(
    record: &PublicationExecutionRecord,
    expected_height: u64,
    expected_world_id: &str,
) -> Result<(), PublicationProofError> {
    if record.height != expected_height || record.world_id != expected_world_id {
        return Err(PublicationProofError::new(
            PublicationProofErrorKind::ContinuityInvalid,
            format!("expected_height={expected_height}"),
        ));
    }
    if nonempty(record.node_block_hash.as_deref()).is_none()
        || nonempty(record.prev_node_block_hash.as_deref()).is_none()
    {
        return Err(PublicationProofError::new(
            PublicationProofErrorKind::AncestryInvalid,
            format!("height={expected_height}"),
        ));
    }
    Ok(())
}

pub(super) fn validate_edge(
    child: &PublicationExecutionRecord,
    parent: &PublicationExecutionRecord,
) -> Result<(), PublicationProofError> {
    if child.height.checked_sub(1) != Some(parent.height) || child.world_id != parent.world_id {
        return Err(PublicationProofError::new(
            PublicationProofErrorKind::ContinuityInvalid,
            format!("child_height={}", child.height),
        ));
    }
    if child.prev_node_block_hash.as_deref() != parent.node_block_hash.as_deref() {
        return Err(PublicationProofError::new(
            PublicationProofErrorKind::AncestryInvalid,
            format!("child_height={}", child.height),
        ));
    }
    if child.timestamp_ms < parent.timestamp_ms {
        return Err(PublicationProofError::new(
            PublicationProofErrorKind::ChronologyInvalid,
            format!("child_height={}", child.height),
        ));
    }
    Ok(())
}

pub(super) fn scan_contiguous_history(
    records_dir: &Path,
    world_id: &str,
    local: PublicationExecutionRecord,
    parent: PublicationExecutionRecord,
    target_height: Option<u64>,
) -> Result<Vec<PublicationExecutionRecord>, PublicationProofError> {
    validate_record(&local, local.height, world_id)?;
    validate_record(&parent, parent.height, world_id)?;
    validate_edge(&local, &parent)?;
    let mut records = vec![local, parent];
    loop {
        let oldest = records.last().expect("nonempty publication history");
        if target_height.is_some_and(|target| oldest.height <= target) || oldest.height == 0 {
            return Ok(records);
        }
        if records.len() == MAX_PUBLICATION_EPISODE_RECORD_SCAN {
            if record_path(records_dir, oldest.height - 1).exists() {
                return Err(PublicationProofError::new(
                    PublicationProofErrorKind::ScanLimitExceeded,
                    format!("records={}", records.len() + 1),
                ));
            }
            return Ok(records);
        }
        let previous_height = oldest.height - 1;
        let previous = load_record(records_dir, previous_height)?;
        validate_record(&previous, previous_height, world_id)?;
        validate_edge(oldest, &previous)?;
        records.push(previous);
    }
}

pub(super) fn record_path(records_dir: &Path, height: u64) -> PathBuf {
    records_dir.join(format!("{height:020}.json"))
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.trim().is_empty())
}
