use super::RollbackApprovalSignature;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoritativeRollbackV2Request {
    pub reason: String,
    pub approval: RollbackAuthorizationEnvelopeV2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RollbackAuthorizationEnvelopeV2 {
    pub intent: RollbackIntentV2,
    pub signatures: Vec<RollbackApprovalSignature>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RollbackIntentV2 {
    pub schema_version: u32,
    pub rollback_ticket: String,
    pub rollback_checkpoint: RollbackCheckpointRef,
    pub replay_target: RollbackReplayTarget,
    pub expected_reorg_epoch: u64,
    pub max_replay_events: usize,
    pub max_replay_bytes: usize,
    pub reason: String,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub nonce: String,
}

impl RollbackIntentV2 {
    pub fn canonical_signing_payload(&self) -> Result<Vec<u8>, serde_json::Error> {
        let mut payload = b"oasis7:governed-rollback-replay:v2\0".to_vec();
        payload.extend(serde_json::to_vec(self)?);
        Ok(payload)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RollbackCheckpointRef {
    pub batch_id: String,
    pub snapshot_hash: String,
    #[serde(rename = "journal_len")]
    pub snapshot_journal_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RollbackReplayTarget {
    pub batch_id: String,
    #[serde(rename = "journal_len")]
    pub target_journal_len: usize,
    #[serde(rename = "state_root")]
    pub expected_target_state_root: String,
    pub journal_commitment: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoritativeRollbackReceipt {
    pub receipt_id: String,
    pub authorization_nonce: String,
    pub rollback_ticket: String,
    #[serde(default)]
    pub canonical_intent_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollback_checkpoint: Option<RollbackCheckpointRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replay_target: Option<RollbackReplayTarget>,
    #[serde(default)]
    pub journal_commitment: String,
    #[serde(default)]
    pub target_state_root: String,
    #[serde(default)]
    pub affected_event_census: Vec<RollbackSourceEventRef>,
    pub target_batch_id: String,
    pub invalidated_batch_ids: Vec<String>,
    pub prior_reorg_epoch: u64,
    pub committed_reorg_epoch: u64,
    pub replay_from_snapshot_height: u64,
    pub replay_from_log_cursor: u64,
    pub snapshot_hash: String,
    pub snapshot_reload_required: bool,
    #[serde(default)]
    pub player_dispositions: Vec<PlayerRollbackDisposition>,
    #[serde(default)]
    pub ready_for_all_clear: bool,
    #[serde(default)]
    pub readiness_blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackSourceEventRef {
    pub source_batch_id: String,
    pub source_event_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerRollbackDisposition {
    pub disposition_id: String,
    pub source_batch_id: String,
    pub source_event_id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_id: Option<String>,
    pub disposition: PlayerActionDisposition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compensation_case_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compensation: Option<PlayerCompensationStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCompensationStatus {
    pub case_id: String,
    pub responsible_party: String,
    pub ticket_reference: String,
    pub state: PlayerCompensationState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackStrictAuditEvidence {
    pub rollback_ticket: String,
    pub receipt_id: String,
    pub canonical_intent_digest: String,
    pub recovery_snapshot_hash: String,
    pub reorg_epoch: u64,
    pub candidate_state_root: String,
    pub strict_registry_audit_passed: bool,
    pub strict_manifest_audit_passed: bool,
    pub evidence_digest: String,
    pub observed_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerCompensationState {
    PendingAuthorization,
    Authorized,
    InProgress,
    Completed,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerActionDisposition {
    PreservedAtTarget,
    Replayed,
    RejectedFork,
    CompensationRequired,
}
