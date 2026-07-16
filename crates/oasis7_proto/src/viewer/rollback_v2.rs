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
        #[derive(Serialize)]
        struct RuntimeCompatibleIntent<'a> {
            schema_version: u32,
            rollback_ticket: &'a str,
            snapshot_hash: &'a str,
            snapshot_journal_len: usize,
            target_journal_len: usize,
            target_journal_commitment: &'a str,
            expected_target_state_root: &'a str,
            target_batch_id: Option<&'a str>,
            reason: &'a str,
            issued_at_ms: u64,
            expires_at_ms: u64,
            nonce: &'a str,
        }
        let normalized = RuntimeCompatibleIntent {
            schema_version: self.schema_version,
            rollback_ticket: &self.rollback_ticket,
            snapshot_hash: &self.rollback_checkpoint.snapshot_hash,
            snapshot_journal_len: self.rollback_checkpoint.snapshot_journal_len,
            target_journal_len: self.replay_target.target_journal_len,
            target_journal_commitment: &self.replay_target.journal_commitment,
            expected_target_state_root: &self.replay_target.expected_target_state_root,
            target_batch_id: Some(&self.replay_target.batch_id),
            reason: &self.reason,
            issued_at_ms: self.issued_at_ms,
            expires_at_ms: self.expires_at_ms,
            nonce: &self.nonce,
        };
        let mut payload = b"oasis7:world-rollback-authorization:v2\0".to_vec();
        payload.extend(serde_json::to_vec(&normalized)?);
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerRollbackDisposition {
    pub player_id: String,
    pub action_id: String,
    pub disposition: PlayerActionDisposition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerActionDisposition {
    PreservedAtTarget,
    Replayed,
    RejectedFork,
    CompensationRequired,
}
