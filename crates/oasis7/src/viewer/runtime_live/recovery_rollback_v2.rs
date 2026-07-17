use super::recovery_receipt::recovery_error;
use super::*;
use sha2::{Digest, Sha256};

impl ViewerRuntimeLiveServer {
    pub(super) fn rollback_v2_to_replay_target(
        &mut self,
        request: AuthoritativeRollbackV2Request,
    ) -> Result<AuthoritativeRecoveryAck<u64>, AuthoritativeRecoveryError> {
        let intent = request.approval.intent;
        if intent.schema_version != 2 {
            return Err(recovery_error(
                "rollback_schema_unsupported",
                "rollback_v2 requires intent schema_version 2",
                Some(intent.replay_target.batch_id),
                None,
                None,
            ));
        }
        let canonical_v2_payload = intent.canonical_signing_payload().map_err(|err| {
            recovery_error(
                "rollback_intent_encode_failed",
                format!("canonical rollback intent encoding failed: {err}"),
                Some(intent.replay_target.batch_id.clone()),
                None,
                None,
            )
        })?;
        let canonical_intent_hash = hex::encode(Sha256::digest(&canonical_v2_payload));
        if let Some(outcome) = self.world.rollback_nonce_outcome(&intent.nonce) {
            if outcome.canonical_intent_hash == canonical_intent_hash {
                return self.get_rollback_receipt(intent.nonce);
            }
            return Err(recovery_error(
                "rollback_nonce_conflict",
                "authorization nonce is already bound to a different rollback intent",
                Some(intent.replay_target.batch_id),
                None,
                None,
            ));
        }
        if intent.expected_reorg_epoch != self.reorg_epoch {
            return Err(recovery_error(
                "rollback_reorg_epoch_mismatch",
                "rollback intent expected a different reorg epoch",
                Some(intent.replay_target.batch_id),
                None,
                None,
            ));
        }
        let checkpoint_index = self
            .stable_checkpoints
            .iter()
            .position(|checkpoint| checkpoint.batch_id == intent.rollback_checkpoint.batch_id);
        let target_index = self
            .stable_checkpoints
            .iter()
            .position(|checkpoint| checkpoint.batch_id == intent.replay_target.batch_id);
        if matches!((checkpoint_index, target_index), (Some(checkpoint), Some(target)) if checkpoint >= target)
        {
            return Err(recovery_error(
                "rollback_target_precedes_checkpoint",
                "replay target must be a finalized checkpoint strictly after rollback checkpoint",
                Some(intent.replay_target.batch_id),
                None,
                None,
            ));
        }
        if self.authoritative_recovery_dir().is_none() {
            return Err(recovery_error(
                "rollback_durable_sink_required",
                "governed rollback requires a durable authoritative recovery sink",
                Some(intent.replay_target.batch_id.clone()),
                None,
                None,
            ));
        }
        let _checkpoint_index = checkpoint_index.ok_or_else(|| {
            recovery_error(
                "rollback_checkpoint_not_found",
                "signed rollback checkpoint is not retained",
                Some(intent.rollback_checkpoint.batch_id.clone()),
                None,
                None,
            )
        })?;
        let _target_index = target_index.ok_or_else(|| {
            recovery_error(
                "stable_checkpoint_not_found",
                "signed replay target is not retained as finalized",
                Some(intent.replay_target.batch_id.clone()),
                None,
                None,
            )
        })?;
        let signatures = request.approval.signatures;
        let target_batch_id = intent.replay_target.batch_id.clone();
        let legacy = RollbackIntent {
            schema_version: intent.schema_version,
            rollback_ticket: intent.rollback_ticket,
            snapshot_hash: intent.rollback_checkpoint.snapshot_hash.clone(),
            snapshot_journal_len: intent.rollback_checkpoint.snapshot_journal_len,
            target_journal_len: intent.replay_target.target_journal_len,
            expected_target_state_root: intent.replay_target.expected_target_state_root.clone(),
            target_batch_id: Some(target_batch_id.clone()),
            reason: intent.reason,
            issued_at_ms: intent.issued_at_ms,
            expires_at_ms: intent.expires_at_ms,
            nonce: intent.nonce,
            rollback_checkpoint: Some(intent.rollback_checkpoint),
            replay_target: Some(intent.replay_target),
            expected_reorg_epoch: Some(intent.expected_reorg_epoch),
            max_replay_events: Some(intent.max_replay_events),
            max_replay_bytes: Some(intent.max_replay_bytes),
        };
        self.rollback_to_stable_checkpoint_v2(
            AuthoritativeRollbackRequest {
                target_batch_id: Some(target_batch_id),
                reason: request.reason,
                requested_by: None,
                approval: Some(RollbackAuthorizationEnvelope {
                    intent: legacy,
                    signatures,
                }),
            },
            canonical_v2_payload,
        )
    }
}
