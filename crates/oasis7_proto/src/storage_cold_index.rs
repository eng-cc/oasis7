use serde::{Deserialize, Serialize};

pub const STORAGE_COLD_INDEX_SCHEMA_V1: u32 = 1;
pub const STORAGE_COLD_INDEX_DIR_SUFFIX: &str = ".cold-index";
pub const STORAGE_COLD_INDEX_MANIFEST_FILE: &str = "index.json";
pub const STORAGE_COLD_INDEX_SEGMENTS_DIR: &str = "segments";
pub const STORAGE_COLD_INDEX_KEY_KIND_HEIGHT: &str = "height";
pub const STORAGE_COLD_INDEX_VALUE_KIND_CONTENT_HASH: &str = "content_hash";
pub const STORAGE_COLD_INDEX_VALUE_KIND_COMMIT_PACK_REF: &str = "commit_pack_ref";

fn storage_cold_index_schema_v1() -> u32 {
    STORAGE_COLD_INDEX_SCHEMA_V1
}

pub fn storage_cold_index_dir_name(namespace: &str) -> String {
    format!("{namespace}{STORAGE_COLD_INDEX_DIR_SUFFIX}")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StorageColdIndexRange {
    pub from_key: u64,
    pub to_key: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StorageColdIndexRangeAnchor {
    pub from_key: u64,
    pub to_key: u64,
    pub first_content_hash: String,
    pub last_content_hash: String,
    pub entry_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageColdIndexManifest {
    #[serde(default = "storage_cold_index_schema_v1")]
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub namespace: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub key_kind: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub value_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hot_range: Option<StorageColdIndexRange>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cold_range_anchor: Option<StorageColdIndexRangeAnchor>,
}

impl StorageColdIndexManifest {
    pub fn new(
        namespace: impl Into<String>,
        key_kind: impl Into<String>,
        value_kind: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: STORAGE_COLD_INDEX_SCHEMA_V1,
            namespace: namespace.into(),
            key_kind: key_kind.into(),
            value_kind: value_kind.into(),
            hot_range: None,
            cold_range_anchor: None,
        }
    }
}

impl Default for StorageColdIndexManifest {
    fn default() -> Self {
        Self {
            schema_version: STORAGE_COLD_INDEX_SCHEMA_V1,
            namespace: String::new(),
            key_kind: String::new(),
            value_kind: String::new(),
            hot_range: None,
            cold_range_anchor: None,
        }
    }
}
