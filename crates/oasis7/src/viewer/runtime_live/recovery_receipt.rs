use super::*;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct RuntimeAuthoritativeRecoveryGeneration {
    pub(super) schema_version: u32,
    pub(super) ack: AuthoritativeRecoveryAck<u64>,
    pub(super) authoritative_batches: VecDeque<RuntimeAuthoritativeBatchRecord>,
    pub(super) next_authoritative_batch_id: u64,
    pub(super) authoritative_challenges: VecDeque<RuntimeAuthoritativeChallengeRecord>,
    pub(super) next_authoritative_challenge_id: u64,
    pub(super) stable_checkpoints: VecDeque<RuntimeStableCheckpoint>,
    pub(super) reorg_epoch: u64,
    #[serde(default)]
    pub(super) session_policy: RuntimeSessionPolicy,
    #[serde(default)]
    pub(super) session_revoke_metadata: Vec<RuntimePersistedSessionRevokeMetadata>,
    #[serde(default)]
    pub(super) rollback_readiness: BTreeMap<String, RuntimeRollbackReadinessRecord>,
    #[serde(default)]
    pub(super) runtime_action_players: BTreeMap<u64, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct RuntimePersistedSessionRevokeMetadata {
    pub(super) player_id: String,
    pub(super) session_pubkey: String,
    pub(super) metadata: RuntimeSessionRevokeMetadata,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct RuntimeRollbackReadinessRecord {
    pub(super) canonical_intent_digest: String,
    pub(super) affected_census_digest: String,
    pub(super) ready: bool,
    pub(super) blockers: Vec<String>,
    pub(super) evaluated_at_tick: u64,
}

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
    pub(super) fn persist_current_recovery_generation(
        &mut self,
        ack: &AuthoritativeRecoveryAck<u64>,
    ) -> Result<(), AuthoritativeRecoveryError> {
        let Some(recovery_dir) = self.authoritative_recovery_dir() else {
            return Ok(());
        };
        let metadata = serde_json::to_vec(&RuntimeAuthoritativeRecoveryGeneration {
            schema_version: 2,
            ack: ack.clone(),
            authoritative_batches: self.authoritative_batches.clone(),
            next_authoritative_batch_id: self.next_authoritative_batch_id,
            authoritative_challenges: self.authoritative_challenges.clone(),
            next_authoritative_challenge_id: self.next_authoritative_challenge_id,
            stable_checkpoints: self.stable_checkpoints.clone(),
            reorg_epoch: self.reorg_epoch,
            session_policy: self.session_policy.clone(),
            session_revoke_metadata: self
                .session_revoke_metadata
                .iter()
                .map(|((player_id, session_pubkey), metadata)| {
                    RuntimePersistedSessionRevokeMetadata {
                        player_id: player_id.clone(),
                        session_pubkey: session_pubkey.clone(),
                        metadata: metadata.clone(),
                    }
                })
                .collect(),
            rollback_readiness: self.rollback_readiness.clone(),
            runtime_action_players: self.runtime_action_players.clone(),
        })
        .map_err(|err| {
            recovery_error(
                "recovery_generation_encode_failed",
                format!("{err:?}"),
                ack.stable_batch_id.clone(),
                ack.player_id.clone(),
                ack.session_pubkey.clone(),
            )
        })?;
        match self
            .world
            .commit_authoritative_recovery_generation(&recovery_dir, &metadata)
        {
            Ok(crate::runtime::AuthoritativeRecoveryCommitStatus::Committed(committed)) => {
                self.world = committed.world;
                Ok(())
            }
            Ok(crate::runtime::AuthoritativeRecoveryCommitStatus::NotCommitted {
                current_generation_id,
            }) => Err(recovery_error(
                "recovery_persistence_commit_failed",
                format!(
                    "Viewer recovery generation was not committed; current_generation_id={current_generation_id:?}"
                ),
                ack.stable_batch_id.clone(),
                ack.player_id.clone(),
                ack.session_pubkey.clone(),
            )),
            Err(crate::runtime::AuthoritativeRecoveryCommitError::StatusUnknown { reason }) => {
                Err(recovery_error(
                    "recovery_persistence_status_unknown",
                    reason,
                    ack.stable_batch_id.clone(),
                    ack.player_id.clone(),
                    ack.session_pubkey.clone(),
                ))
            }
        }
    }

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
        let readiness = self
            .rollback_readiness
            .get(&authorization_nonce)
            .filter(|record| {
                record.canonical_intent_digest == outcome.canonical_intent_hash
                    && record.affected_census_digest == outcome.affected_census_digest
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
                canonical_intent_digest: outcome.canonical_intent_hash.clone(),
                rollback_checkpoint: Some(crate::viewer::protocol::RollbackCheckpointRef {
                    batch_id: receipt.rollback_checkpoint_batch_id.clone(),
                    snapshot_hash: receipt.rollback_checkpoint_snapshot_hash.clone(),
                    snapshot_journal_len: receipt.rollback_checkpoint_journal_len,
                }),
                replay_target: Some(crate::viewer::protocol::RollbackReplayTarget {
                    batch_id: receipt.replay_target_batch_id.clone(),
                    target_journal_len: receipt.replay_target_journal_len,
                    expected_target_state_root: receipt.replay_target_state_root.clone(),
                    journal_commitment: receipt.replay_target_journal_commitment.clone(),
                }),
                journal_commitment: outcome.target_journal_commitment.clone(),
                target_state_root: outcome.target_state_root.clone(),
                affected_event_census: outcome
                    .dispositions
                    .iter()
                    .map(|entry| crate::viewer::protocol::RollbackSourceEventRef {
                        source_batch_id: entry.source_batch_id.clone(),
                        source_event_id: entry.source_event_id,
                    })
                    .collect(),
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
                ready_for_all_clear: readiness.is_some_and(|record| record.ready),
                readiness_blockers: readiness
                    .map(|record| record.blockers.clone())
                    .unwrap_or_else(|| {
                        if receipt.readiness_blockers.is_empty() {
                            vec!["readiness_evaluation_required".to_string()]
                        } else {
                            receipt.readiness_blockers.clone()
                        }
                    }),
            }),
            acknowledged_at_tick: receipt.acknowledged_at_tick,
        })
    }

    pub(super) fn reevaluate_rollback_readiness(
        &mut self,
        authorization_nonce: String,
    ) -> Result<AuthoritativeRecoveryAck<u64>, AuthoritativeRecoveryError> {
        let recovery_dir = self.authoritative_recovery_dir().ok_or_else(|| {
            recovery_error(
                "rollback_durable_sink_required",
                "readiness reevaluation requires a durable authoritative recovery sink",
                None,
                None,
                None,
            )
        })?;
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
        let receipt = outcome.receipt.as_ref().ok_or_else(|| {
            recovery_error(
                "rollback_receipt_incomplete",
                "rollback outcome has no immutable receipt projection",
                Some(outcome.target_batch_id.clone()),
                None,
                None,
            )
        })?;
        let affected_events = outcome
            .dispositions
            .iter()
            .map(|entry| crate::runtime::RollbackSourceEventIdentity {
                source_batch_id: entry.source_batch_id.clone(),
                source_event_id: entry.source_event_id,
            })
            .collect::<Vec<_>>();
        let evidence = crate::runtime::RollbackReadinessEvidence {
            target_root_matches: receipt.replay_target_state_root == outcome.target_state_root,
            epoch_matches: self.reorg_epoch == outcome.committed_reorg_epoch,
            drift_free: self.world.first_tick_consensus_drift().is_none(),
            consensus_chain_valid: self.world.verify_tick_consensus_chain().is_ok(),
            receipt_retrievable: true,
        };
        let mut blockers = match self.world.rollback_readiness(
            authorization_nonce.as_str(),
            affected_events.as_slice(),
            &evidence,
        ) {
            crate::runtime::RollbackReadiness::Ready => Vec::new(),
            crate::runtime::RollbackReadiness::Blocked { reasons } => reasons,
        };
        if outcome.dispositions.iter().any(|entry| {
            entry.compensation.as_ref().is_some_and(|case| {
                case.state != crate::runtime::RollbackCompensationState::Completed
            })
        }) {
            blockers.push("compensation_cases_unresolved".to_string());
        }
        blockers.sort();
        blockers.dedup();
        let record = RuntimeRollbackReadinessRecord {
            canonical_intent_digest: outcome.canonical_intent_hash.clone(),
            affected_census_digest: outcome.affected_census_digest.clone(),
            ready: blockers.is_empty(),
            blockers: blockers.clone(),
            evaluated_at_tick: self.world.state().time,
        };
        let mut candidate_readiness = self.rollback_readiness.clone();
        candidate_readiness.insert(authorization_nonce.clone(), record);
        let mut ack = self.get_rollback_receipt(authorization_nonce.clone())?;
        if let Some(public_receipt) = ack.rollback_receipt.as_mut() {
            public_receipt.ready_for_all_clear = blockers.is_empty();
            public_receipt.readiness_blockers = blockers;
        }
        let metadata = serde_json::to_vec(&RuntimeAuthoritativeRecoveryGeneration {
            schema_version: 2,
            ack: ack.clone(),
            authoritative_batches: self.authoritative_batches.clone(),
            next_authoritative_batch_id: self.next_authoritative_batch_id,
            authoritative_challenges: self.authoritative_challenges.clone(),
            next_authoritative_challenge_id: self.next_authoritative_challenge_id,
            stable_checkpoints: self.stable_checkpoints.clone(),
            reorg_epoch: self.reorg_epoch,
            session_policy: self.session_policy.clone(),
            session_revoke_metadata: self
                .session_revoke_metadata
                .iter()
                .map(|((player_id, session_pubkey), metadata)| {
                    RuntimePersistedSessionRevokeMetadata {
                        player_id: player_id.clone(),
                        session_pubkey: session_pubkey.clone(),
                        metadata: metadata.clone(),
                    }
                })
                .collect(),
            rollback_readiness: candidate_readiness.clone(),
            runtime_action_players: self.runtime_action_players.clone(),
        })
        .map_err(|err| {
            recovery_error(
                "rollback_ack_encode_failed",
                format!("{err:?}"),
                Some(outcome.target_batch_id.clone()),
                None,
                None,
            )
        })?;
        match self
            .world
            .commit_authoritative_recovery_generation(&recovery_dir, &metadata)
        {
            Ok(crate::runtime::AuthoritativeRecoveryCommitStatus::Committed(committed)) => {
                self.world = committed.world;
                self.rollback_readiness = candidate_readiness;
                Ok(ack)
            }
            Ok(crate::runtime::AuthoritativeRecoveryCommitStatus::NotCommitted {
                current_generation_id,
            }) => Err(recovery_error(
                "rollback_persistence_commit_failed",
                format!(
                    "readiness generation was not committed; current_generation_id={current_generation_id:?}"
                ),
                Some(outcome.target_batch_id),
                None,
                None,
            )),
            Err(crate::runtime::AuthoritativeRecoveryCommitError::StatusUnknown { reason }) => {
                Err(recovery_error(
                    "rollback_persistence_status_unknown",
                    reason,
                    Some(outcome.target_batch_id),
                    None,
                    None,
                ))
            }
        }
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
            let compensation = entry.compensation.as_ref().zip(compensation_case_id.as_ref()).map(
                |(case, case_id)| crate::viewer::protocol::PlayerCompensationStatus {
                    case_id: case_id.clone(),
                    responsible_party: case.owner_id.clone(),
                    ticket_reference: case.ticket_id.clone(),
                    state: match case.state {
                        crate::runtime::RollbackCompensationState::PendingAuthorization => crate::viewer::protocol::PlayerCompensationState::PendingAuthorization,
                        crate::runtime::RollbackCompensationState::Authorized => crate::viewer::protocol::PlayerCompensationState::Authorized,
                        crate::runtime::RollbackCompensationState::InProgress => crate::viewer::protocol::PlayerCompensationState::InProgress,
                        crate::runtime::RollbackCompensationState::Completed => crate::viewer::protocol::PlayerCompensationState::Completed,
                        crate::runtime::RollbackCompensationState::Rejected => crate::viewer::protocol::PlayerCompensationState::Rejected,
                    },
                },
            );
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
                compensation,
            }
        })
        .collect()
}
