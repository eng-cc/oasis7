use super::*;
use sha2::{Digest, Sha256};

pub(super) fn current_unix_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

pub(super) fn rollback_runtime_error(
    err: crate::runtime::WorldError,
    batch_id: String,
) -> AuthoritativeRecoveryError {
    let debug = format!("{err:?}");
    let code = match &err {
        crate::runtime::WorldError::JournalMismatch => "rollback_journal_mismatch",
        crate::runtime::WorldError::RollbackReplayJournalInvalid { .. } => {
            "rollback_replay_journal_invalid"
        }
        crate::runtime::WorldError::RollbackReplayTargetInvalid { .. } => {
            "rollback_replay_target_invalid"
        }
        crate::runtime::WorldError::RollbackTargetStateRootMismatch { .. } => {
            "rollback_target_state_root_mismatch"
        }
        crate::runtime::WorldError::RollbackReconciliationFailed { .. } => {
            "rollback_reconciliation_drift"
        }
        crate::runtime::WorldError::DistributedValidationFailed { reason }
            if reason.contains("reconciliation drift") =>
        {
            "rollback_reconciliation_drift"
        }
        crate::runtime::WorldError::DistributedValidationFailed { .. } => {
            "rollback_authorization_invalid"
        }
        _ => "internal_prepare_failed",
    };
    recovery_error(code, debug, Some(batch_id), None, None)
}

impl ViewerRuntimeLiveServer {
    pub(super) fn get_rollback_receipt(
        &self,
        authorization_nonce: String,
    ) -> Result<AuthoritativeRecoveryAck<u64>, AuthoritativeRecoveryError> {
        let outcome = self
            .world
            .rollback_nonce_outcome(&authorization_nonce)
            .ok_or_else(|| {
                recovery_error(
                    "rollback_receipt_not_found",
                    format!("no persisted rollback receipt for nonce {authorization_nonce}"),
                    None,
                    None,
                    None,
                )
            })?;
        let target_batch_id = outcome.target_batch_id.clone();
        let receipt = outcome.receipt.as_ref().ok_or_else(|| {
            recovery_error(
                "rollback_receipt_incomplete",
                "rollback outcome has no immutable receipt projection",
                Some(target_batch_id.clone()),
                None,
                None,
            )
        })?;
        let has_unresolved_compensation = outcome.dispositions.iter().any(|entry| {
            matches!(
                entry.status,
                crate::runtime::RollbackDispositionStatus::CompensationRequired
            )
        });
        Ok(AuthoritativeRecoveryAck {
            status: AuthoritativeRecoveryStatus::RolledBack,
            reorg_epoch: outcome.committed_reorg_epoch,
            snapshot_height: receipt.snapshot_height,
            snapshot_hash: receipt.snapshot_hash.clone(),
            log_cursor: receipt.log_cursor,
            stable_batch_id: Some(target_batch_id.clone()),
            player_id: None,
            agent_id: None,
            session_pubkey: None,
            replaced_by_pubkey: None,
            session_epoch: None,
            message: Some("persisted rollback receipt".to_string()),
            revoke_reason: None,
            revoked_by: None,
            rollback_receipt: Some(AuthoritativeRollbackReceipt {
                receipt_id: receipt.receipt_id.clone(),
                authorization_nonce: authorization_nonce.clone(),
                rollback_ticket: outcome.rollback_ticket.clone(),
                target_batch_id,
                invalidated_batch_ids: outcome.invalidated_batch_ids.clone(),
                prior_reorg_epoch: outcome.prior_reorg_epoch,
                committed_reorg_epoch: outcome.committed_reorg_epoch,
                replay_from_snapshot_height: receipt.snapshot_height,
                replay_from_log_cursor: receipt.log_cursor,
                snapshot_hash: receipt.snapshot_hash.clone(),
                snapshot_reload_required: true,
                player_dispositions: viewer_dispositions(
                    outcome.rollback_ticket.as_str(),
                    authorization_nonce.as_str(),
                    outcome.dispositions.as_slice(),
                ),
                ready_for_all_clear: !has_unresolved_compensation,
                readiness_blockers: has_unresolved_compensation
                    .then(|| "compensation_cases_unresolved".to_string())
                    .into_iter()
                    .collect(),
            }),
            acknowledged_at_tick: receipt.acknowledged_at_tick,
        })
    }
}

impl ViewerRuntimeLiveServer {
    fn session_revoke_metadata(
        &self,
        player_id: &str,
        session_pubkey: &str,
    ) -> Option<&RuntimeSessionRevokeMetadata> {
        self.session_revoke_metadata
            .get(&session_revoke_metadata_key(player_id, session_pubkey))
    }

    pub(super) fn recovery_error_from_session_policy(
        &self,
        message: String,
        player_id: String,
        session_pubkey: Option<String>,
    ) -> AuthoritativeRecoveryError {
        let code = map_session_policy_error_code(message.as_str());
        let (revoke_reason, revoked_by) = if code == "session_revoked" {
            session_pubkey
                .as_deref()
                .and_then(|pubkey| self.session_revoke_metadata(player_id.as_str(), pubkey))
                .map(|metadata| (metadata.revoke_reason.clone(), metadata.revoked_by.clone()))
                .unwrap_or((None, None))
        } else {
            (None, None)
        };
        recovery_error_with_revoke_metadata(
            code,
            message,
            None,
            Some(player_id),
            session_pubkey,
            revoke_reason,
            revoked_by,
        )
    }
}

pub(super) fn recovery_error(
    code: impl Into<String>,
    message: impl Into<String>,
    batch_id: Option<String>,
    player_id: Option<String>,
    session_pubkey: Option<String>,
) -> AuthoritativeRecoveryError {
    recovery_error_with_revoke_metadata(
        code,
        message,
        batch_id,
        player_id,
        session_pubkey,
        None,
        None,
    )
}

fn recovery_error_with_revoke_metadata(
    code: impl Into<String>,
    message: impl Into<String>,
    batch_id: Option<String>,
    player_id: Option<String>,
    session_pubkey: Option<String>,
    revoke_reason: Option<String>,
    revoked_by: Option<String>,
) -> AuthoritativeRecoveryError {
    AuthoritativeRecoveryError {
        code: code.into(),
        message: message.into(),
        batch_id,
        player_id,
        session_pubkey,
        revoke_reason,
        revoked_by,
    }
}

pub(super) fn viewer_dispositions(
    rollback_ticket: &str,
    authorization_nonce: &str,
    dispositions: &[crate::runtime::RollbackEventDisposition],
) -> Vec<crate::viewer::protocol::PlayerRollbackDisposition> {
    dispositions
        .iter()
        .map(|entry| {
            let stable_source = format!("{}:{}", entry.source_batch_id, entry.source_event_id);
            let disposition_id = hex::encode(Sha256::digest(format!(
                "oasis7:rollback-disposition:v1\0{stable_source}"
            )));
            let compensation_case_id = entry.compensation.as_ref().map(|case| {
                hex::encode(Sha256::digest(format!(
                    "oasis7:compensation-case:v1\0{rollback_ticket}:{authorization_nonce}:{disposition_id}:{}:{}",
                    case.owner_id, case.ticket_id
                )))
            });
            crate::viewer::protocol::PlayerRollbackDisposition {
                disposition_id,
                source_batch_id: entry.source_batch_id.clone(),
                source_event_id: entry.source_event_id,
                player_id: entry.player_id.clone(),
                action_id: entry.action_id.clone(),
                disposition: match entry.status {
                    crate::runtime::RollbackDispositionStatus::PreservedAtTarget => {
                        crate::viewer::protocol::PlayerActionDisposition::PreservedAtTarget
                    }
                    crate::runtime::RollbackDispositionStatus::Replayed => {
                        crate::viewer::protocol::PlayerActionDisposition::Replayed
                    }
                    crate::runtime::RollbackDispositionStatus::RejectedFork => {
                        crate::viewer::protocol::PlayerActionDisposition::RejectedFork
                    }
                    crate::runtime::RollbackDispositionStatus::CompensationRequired => {
                        crate::viewer::protocol::PlayerActionDisposition::CompensationRequired
                    }
                },
                compensation_case_id,
            }
        })
        .collect()
}
