use super::recovery_receipt::recovery_error;
use super::*;

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
        let retry_approval = crate::runtime::RollbackAuthorizationEnvelope {
            intent: crate::runtime::RollbackIntent {
                schema_version: intent.schema_version,
                rollback_ticket: intent.rollback_ticket.clone(),
                snapshot_hash: intent.rollback_checkpoint.snapshot_hash.clone(),
                snapshot_journal_len: intent.rollback_checkpoint.snapshot_journal_len,
                target_journal_len: intent.replay_target.target_journal_len,
                target_journal_commitment: Some(intent.replay_target.journal_commitment.clone()),
                expected_target_state_root: intent.replay_target.expected_target_state_root.clone(),
                target_batch_id: Some(intent.replay_target.batch_id.clone()),
                reason: intent.reason.clone(),
                issued_at_ms: intent.issued_at_ms,
                expires_at_ms: intent.expires_at_ms,
                nonce: intent.nonce.clone(),
            },
            signatures: request
                .approval
                .signatures
                .iter()
                .map(|signature| crate::runtime::RollbackApprovalSignature {
                    authority_id: signature.authority_id.clone(),
                    role: match signature.role {
                        crate::viewer::protocol::RollbackAuthorityRole::OnCall => {
                            crate::runtime::RollbackAuthorityRole::OnCall
                        }
                        crate::viewer::protocol::RollbackAuthorityRole::Governance => {
                            crate::runtime::RollbackAuthorityRole::Governance
                        }
                    },
                    signature_scheme: signature.signature_scheme.clone(),
                    signature_hex: signature.signature_hex.clone(),
                })
                .collect(),
        };
        if self
            .world
            .authenticate_committed_rollback_retry(
                &retry_approval,
                &canonical_v2_payload,
                super::recovery_receipt::current_unix_time_ms(),
            )
            .map_err(|err| {
                super::recovery_receipt::rollback_runtime_error(
                    err,
                    intent.replay_target.batch_id.clone(),
                )
            })?
        {
            return self.get_rollback_receipt(intent.nonce);
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
