//! Personal reading DTOs; source facts and numeric occurrences are never persisted.
use serde::{Deserialize, Serialize};
use ts_rs::TS;
#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTreeRouteStep {
    pub path: Option<String>,
    pub source_path: Option<String>,
    pub destination: Option<String>,
    pub group: Option<String>,
    pub duplicate_index: usize,
    pub duplicate_count: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTreeReadingScope {
    pub schema_version: u32,
    pub project_id: String,
    pub baseline_mode: String,
    /// Current source provenance, not personal record identity.
    pub baseline_oid: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTreeReadingKey {
    pub schema_version: u32,
    pub project_id: String,
    pub baseline_mode: String,
    pub baseline_oid: String,
    pub view: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTreeReadingFilters {
    pub query: String,
    pub read_filter: String,
    pub lifecycle_filter: String,
    pub kind_filter: String,
    pub pairing_filter: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTreeReadingSelection {
    pub route: Vec<DocumentTreeRouteStep>,
    pub path: String,
    pub section_path: Option<String>,
    pub section_id: Option<String>,
    pub section_revision: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTreeReadingState {
    pub filters: DocumentTreeReadingFilters,
    pub expanded_routes: Vec<Vec<DocumentTreeRouteStep>>,
    pub parent_route: Vec<DocumentTreeRouteStep>,
    pub selection: Option<DocumentTreeReadingSelection>,
    pub tree_scroll_top: f64,
    pub detail_scroll_top: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTreeReadingSaveRequest {
    pub key: DocumentTreeReadingKey,
    pub state: DocumentTreeReadingState,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTreeReadingLoadResult {
    pub schema_version: u32,
    pub key: DocumentTreeReadingKey,
    /// missing / ready / corrupt / unsupported
    pub status: String,
    pub state: Option<DocumentTreeReadingState>,
    pub notice: Option<String>,
    pub source_baseline_oid: Option<String>,
    pub resolved_section_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTreeLastView {
    pub schema_version: u32,
    pub status: String,
    pub view: Option<String>,
    pub notice: Option<String>,
}
