use serde::{Deserialize, Serialize};
use ts_rs::TS;
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SourceStatus {
    pub kind: String,
    pub state: String,
    pub last_attempt: Option<String>,
    pub last_success: Option<String>,
    pub source_revision: Option<String>,
    pub generation: String,
    pub coverage: String,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    pub path: String,
    pub git_common_dir: String,
    pub branch: Option<String>,
    pub head: String,
    pub environment: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DocumentSection {
    pub id: String,
    pub path: String,
    pub title: String,
    pub content: String,
    pub line: usize,
    pub revision: String,
    pub committed_revision: String,
    pub dirty: bool,
    pub fact_kind: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub project_path: Option<String>,
    pub view: String,
    pub query: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DecisionDraft {
    pub id: Option<String>,
    pub content: String,
    pub reason: String,
    pub scope: String,
    pub related_section: Option<String>,
    pub confirmed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct DecisionRecord {
    pub id: String,
    pub content: String,
    pub reason: String,
    pub scope: String,
    pub related_section: Option<String>,
    pub state: String,
    pub confirmed_at: Option<String>,
    pub formal_update: String,
    pub implementation_sync: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ReadingPosition {
    pub section_id: String,
    pub revision: String,
    pub line: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[serde(rename_all = "camelCase")]
pub struct GitSnapshot {
    pub status: String,
    pub worktrees: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    #[serde(default)]
    pub associations: Vec<WorkAssociation>,
    pub project: Option<Project>,
    pub sources: Vec<SourceStatus>,
    pub sections: Vec<DocumentSection>,
    pub decisions: Vec<DecisionRecord>,
    pub preferences: Preferences,
    pub reading_positions: Vec<ReadingPosition>,
    pub git: GitSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct WorkAssociation {
    pub work_url: String,
    pub goal_section_ids: Vec<String>,
    pub confirmed_at: String,
}
