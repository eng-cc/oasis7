use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ControlCompletionStatus {
    Advanced,
    TimeoutNoProgress,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlCompletionAck<Time> {
    pub request_id: u64,
    pub status: ControlCompletionStatus,
    pub delta_logical_time: Time,
    pub delta_event_seq: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptControlOperation {
    Apply,
    Rollback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptControlAck<Time> {
    pub agent_id: String,
    pub operation: PromptControlOperation,
    pub preview: bool,
    pub version: u64,
    pub updated_at_tick: Time,
    pub applied_fields: Vec<String>,
    pub digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rolled_back_to_version: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PromptControlError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_version: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentChatAck<Time> {
    pub agent_id: String,
    pub accepted_at_tick: Time,
    pub message_len: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent_tick: Option<Time>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent_seq: Option<u64>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub idempotent_replay: bool,
    /// Durable runtime identity. Older clients may ignore these additive
    /// fields; retries use them instead of reconstructing acceptance from
    /// response arrival time. `accepted_event_seq` is the canonical position.
    /// The fields remain optional so older clients can continue decoding the
    /// acknowledgement while newer clients preserve durable retry identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_event_seq: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaced_by: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentChatError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameplayActionAck<Time> {
    pub action_id: String,
    pub target_agent_id: String,
    pub player_id: String,
    pub runtime_action_id: u64,
    pub accepted_at_tick: Time,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameplayActionError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_agent_id: Option<String>,
}
