use serde::{Deserialize, Serialize};

use crate::distributed::{
    BlobRef, BlockAnnounce, SnapshotManifest, WIRE_ENCODING_CBOR, WorldBlock, WorldHeadAnnounce,
};

pub const DEFAULT_SNAPSHOT_CHUNK_BYTES: usize = 256 * 1024;
pub const DEFAULT_JOURNAL_EVENTS_PER_SEGMENT: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SegmentConfig {
    pub snapshot_chunk_bytes: usize,
    pub journal_events_per_segment: usize,
}

impl Default for SegmentConfig {
    fn default() -> Self {
        Self {
            snapshot_chunk_bytes: DEFAULT_SNAPSHOT_CHUNK_BYTES,
            journal_events_per_segment: DEFAULT_JOURNAL_EVENTS_PER_SEGMENT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalSegmentRef {
    pub from_event_id: u64,
    pub to_event_id: u64,
    pub content_hash: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionWriteConfig {
    pub segment: SegmentConfig,
    pub codec: String,
}

impl Default for ExecutionWriteConfig {
    fn default() -> Self {
        Self {
            segment: SegmentConfig::default(),
            codec: WIRE_ENCODING_CBOR.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionWriteResult {
    pub block: WorldBlock,
    pub block_hash: String,
    pub block_ref: BlobRef,
    pub block_announce: BlockAnnounce,
    pub head_announce: WorldHeadAnnounce,
    pub snapshot_manifest: SnapshotManifest,
    pub snapshot_manifest_ref: BlobRef,
    pub journal_segments: Vec<JournalSegmentRef>,
    pub journal_segments_ref: BlobRef,
}
