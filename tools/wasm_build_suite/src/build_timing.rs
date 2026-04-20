use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildTimingSnapshot {
    pub total_build_wall_ms: u64,
    pub cargo_build_ms: u64,
    pub canonicalize_ms: u64,
    pub hash_ms: u64,
    pub receipt_write_ms: u64,
    pub metadata_write_ms: u64,
}
