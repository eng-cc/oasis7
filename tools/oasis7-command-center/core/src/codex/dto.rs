use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ts_rs::TS;
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeTarget {
    pub instance_id: String,
    pub thread_id: String,
    pub turn_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSession {
    pub purpose: String,
    pub scope: String,
    pub model: Option<String>,
    pub basis_revision: String,
    pub thread_id: String,
    pub instance_id: String,
    pub turn_id: Option<String>,
    pub state: String,
    pub owned: bool,
    pub background_state: String,
    #[serde(default)]
    pub restorable_history: bool,
    #[serde(default)]
    pub private_sources: Vec<PrivateInputRef>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeOperation {
    pub id: String,
    pub instance_id: String,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    pub method: String,
    pub state: String,
    pub fingerprint: String,
    pub created_at: String,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeQuestion {
    pub id: String,
    pub header: String,
    pub question: String,
    pub options: Vec<String>,
    pub is_secret: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeRequest {
    pub request_id: String,
    pub instance_id: String,
    pub thread_id: String,
    pub turn_id: String,
    pub method: String,
    pub state: String,
    pub summary: String,
    pub questions: Vec<RuntimeQuestion>,
    pub allowed_decisions: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RequestAnswer {
    pub request_id: String,
    pub target: RuntimeTarget,
    pub decision: Option<String>,
    pub answers: BTreeMap<String, Vec<String>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSnapshot {
    pub project_id: String,
    pub environment: String,
    pub enabled: bool,
    pub experimental: bool,
    pub state: String,
    pub version: Option<String>,
    pub instance_id: Option<String>,
    pub capabilities: Vec<String>,
    pub coverage: String,
    pub error: Option<String>,
    pub sessions: Vec<RuntimeSession>,
    pub requests: Vec<RuntimeRequest>,
    pub operations: Vec<RuntimeOperation>,
    pub logs: Vec<String>,
    pub logs_truncated: bool,
    pub exit_waiting: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct QuitStatus {
    pub safe: bool,
    pub active: usize,
    pub unknown: usize,
    pub pending_requests: usize,
    pub waiting: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct SessionConfig {
    pub purpose: String,
    pub scope: String,
    pub model: Option<String>,
    pub basis_revision: String,
}

/// References only: private text remains in the original store and explicit runtime input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PrivateInputRef {
    pub binding_id: String,
    #[ts(type = "number")]
    pub binding_version: u64,
    pub entry_id: String,
    pub revision: String,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PrivateInputPreview {
    pub source: PrivateInputRef,
    pub text: String,
    pub attachment_bytes: usize,
    pub max_input_bytes: usize,
}
