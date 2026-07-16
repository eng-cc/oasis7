use super::authoritative::compute_runtime_snapshot_hash;
use super::session_policy::RuntimeRecoveryCursor;
use super::*;
use sha2::{Digest, Sha256};

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RuntimeRecoveryFaultInjection {
    ReconciliationDrift,
    CursorCompute,
    AckEncode,
}

impl ViewerRuntimeLiveServer {
    #[cfg(test)]
    pub(super) fn set_recovery_fault_injection(
        &mut self,
        fault: Option<RuntimeRecoveryFaultInjection>,
    ) {
        self.recovery_fault_injection = fault;
    }

    pub(super) fn handle_authoritative_recovery_for_protocol(
        &mut self,
        command: AuthoritativeRecoveryCommand,
        negotiated: &crate::viewer::protocol::NegotiatedViewerProtocol,
    ) -> Result<(AuthoritativeRecoveryAck<u64>, bool), AuthoritativeRecoveryError> {
        if matches!(
            command,
            AuthoritativeRecoveryCommand::Rollback { .. }
                | AuthoritativeRecoveryCommand::RollbackV2 { .. }
                | AuthoritativeRecoveryCommand::GetRollbackReceipt { .. }
        ) && !negotiated.supports_signed_rollback()
        {
            return Err(recovery_error(
                "protocol_upgrade_required",
                "signed authoritative rollback requires Viewer protocol v2 capability negotiation",
                None,
                None,
                None,
            ));
        }
        if matches!(command, AuthoritativeRecoveryCommand::Rollback { .. }) {
            return Err(recovery_error(
                "rollback_shape_unsupported",
                "legacy rollback shape is decode-only; use rollback_v2",
                None,
                None,
                None,
            ));
        }
        self.handle_authoritative_recovery(command)
    }

    pub(super) fn handle_authoritative_recovery(
        &mut self,
        command: AuthoritativeRecoveryCommand,
    ) -> Result<(AuthoritativeRecoveryAck<u64>, bool), AuthoritativeRecoveryError> {
        match command {
            AuthoritativeRecoveryCommand::RegisterSession { request } => {
                self.register_session_key(request).map(|ack| (ack, false))
            }
            AuthoritativeRecoveryCommand::Rollback { request } => self
                .rollback_to_stable_checkpoint(request)
                .map(|ack| (ack, true)),
            AuthoritativeRecoveryCommand::RollbackV2 { request } => self
                .rollback_v2_to_replay_target(request)
                .map(|ack| (ack, true)),
            AuthoritativeRecoveryCommand::GetRollbackReceipt {
                authorization_nonce,
            } => self
                .get_rollback_receipt(authorization_nonce)
                .map(|ack| (ack, false)),
            AuthoritativeRecoveryCommand::ReconnectSync { request } => {
                self.handle_reconnect_sync(request).map(|ack| (ack, false))
            }
            AuthoritativeRecoveryCommand::RevokeSession { request } => {
                self.revoke_session_key(request).map(|ack| (ack, false))
            }
            AuthoritativeRecoveryCommand::RotateSession { request } => {
                self.rotate_session_key(request).map(|ack| (ack, false))
            }
        }
    }

    fn rollback_v2_to_replay_target(
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
        if intent.expected_reorg_epoch != self.reorg_epoch {
            return Err(recovery_error(
                "rollback_reorg_epoch_mismatch",
                "rollback intent expected a different reorg epoch",
                Some(intent.replay_target.batch_id),
                None,
                None,
            ));
        }
        let canonical_intent_hash = hex::encode(Sha256::digest(
            intent.canonical_signing_payload().map_err(|err| {
                recovery_error(
                    "rollback_intent_encode_failed",
                    format!("canonical rollback intent encoding failed: {err}"),
                    Some(intent.replay_target.batch_id.clone()),
                    None,
                    None,
                )
            })?,
        ));
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
        self.rollback_to_stable_checkpoint(AuthoritativeRollbackRequest {
            target_batch_id: Some(target_batch_id),
            reason: request.reason,
            requested_by: None,
            approval: Some(RollbackAuthorizationEnvelope {
                intent: legacy,
                signatures,
            }),
        })
    }

    fn get_rollback_receipt(
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
        let cursor = self.current_recovery_cursor().map_err(|err| {
            recovery_error(
                "cursor_compute_failed",
                format!("{err:?}"),
                Some(target_batch_id.clone()),
                None,
                None,
            )
        })?;
        Ok(AuthoritativeRecoveryAck {
            status: AuthoritativeRecoveryStatus::RolledBack,
            reorg_epoch: self.reorg_epoch,
            snapshot_height: cursor.snapshot_height,
            snapshot_hash: cursor.snapshot_hash.clone(),
            log_cursor: cursor.log_cursor,
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
                receipt_id: format!(
                    "rollback:{}:{}",
                    outcome.rollback_ticket, authorization_nonce
                ),
                authorization_nonce,
                rollback_ticket: outcome.rollback_ticket.clone(),
                target_batch_id,
                invalidated_batch_ids: outcome.invalidated_batch_ids.clone(),
                prior_reorg_epoch: outcome.prior_reorg_epoch,
                committed_reorg_epoch: outcome.committed_reorg_epoch,
                replay_from_snapshot_height: cursor.snapshot_height,
                replay_from_log_cursor: outcome.rollback_event_id,
                snapshot_hash: cursor.snapshot_hash,
                snapshot_reload_required: true,
                player_dispositions: outcome
                    .dispositions
                    .iter()
                    .filter_map(|entry| {
                        Some(crate::viewer::protocol::PlayerRollbackDisposition {
                            player_id: entry.player_id.clone()?,
                            action_id: entry.action_id.clone()?,
                            disposition: match entry.status {
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
                        })
                    })
                    .collect(),
            }),
            acknowledged_at_tick: self.world.state().time,
        })
    }

    fn rollback_to_stable_checkpoint(
        &mut self,
        request: AuthoritativeRollbackRequest,
    ) -> Result<AuthoritativeRecoveryAck<u64>, AuthoritativeRecoveryError> {
        let approval = request.approval.ok_or_else(|| {
            recovery_error(
                "rollback_approval_required",
                "authoritative rollback requires approval evidence",
                request.target_batch_id.clone(),
                None,
                None,
            )
        })?;
        let target_batch_id = request
            .target_batch_id
            .clone()
            .or_else(|| {
                self.stable_checkpoints
                    .back()
                    .map(|entry| entry.batch_id.clone())
            })
            .ok_or_else(|| {
                recovery_error(
                    "stable_checkpoint_not_found",
                    "no stable checkpoint available for rollback",
                    None,
                    None,
                    None,
                )
            })?;
        if approval.intent.target_batch_id.as_deref() != Some(target_batch_id.as_str()) {
            return Err(recovery_error(
                "rollback_authorization_invalid",
                "rollback authorization is not bound to the selected batch",
                Some(target_batch_id),
                None,
                None,
            ));
        }
        let mut checkpoint = self
            .stable_checkpoints
            .iter()
            .find(|entry| entry.batch_id == target_batch_id)
            .cloned()
            .ok_or_else(|| {
                recovery_error(
                    "stable_checkpoint_not_found",
                    format!("stable checkpoint for batch {} not found", target_batch_id),
                    Some(target_batch_id.clone()),
                    None,
                    None,
                )
            })?;
        if let Some(checkpoint_ref) = approval.intent.rollback_checkpoint.as_ref() {
            let rollback_checkpoint = self
                .stable_checkpoints
                .iter()
                .find(|entry| entry.batch_id == checkpoint_ref.batch_id)
                .ok_or_else(|| {
                    recovery_error(
                        "rollback_checkpoint_not_found",
                        "signed rollback checkpoint is not retained",
                        Some(checkpoint_ref.batch_id.clone()),
                        None,
                        None,
                    )
                })?;
            if checkpoint_ref.snapshot_journal_len != rollback_checkpoint.snapshot.journal_len
                || checkpoint_ref.snapshot_hash != approval.intent.snapshot_hash
                || checkpoint_ref.snapshot_journal_len > checkpoint.journal.len()
            {
                return Err(recovery_error(
                    "rollback_checkpoint_mismatch",
                    "signed rollback checkpoint does not match retained checkpoint metadata",
                    Some(checkpoint_ref.batch_id.clone()),
                    None,
                    None,
                ));
            }
            let replay_events = checkpoint
                .journal
                .len()
                .saturating_sub(checkpoint_ref.snapshot_journal_len);
            let replay_bytes = serde_json::to_vec(
                &checkpoint.journal.events[checkpoint_ref.snapshot_journal_len..],
            )
            .map_err(|err| {
                recovery_error(
                    "rollback_replay_commitment_failed",
                    format!("serialize replay suffix failed: {err}"),
                    Some(target_batch_id.clone()),
                    None,
                    None,
                )
            })?;
            if replay_events > approval.intent.max_replay_events.unwrap_or(0)
                || replay_bytes.len() > approval.intent.max_replay_bytes.unwrap_or(0)
            {
                return Err(recovery_error(
                    "rollback_replay_bounds_exceeded",
                    "resolved replay suffix exceeds signed execution bounds",
                    Some(target_batch_id.clone()),
                    None,
                    None,
                ));
            }
            let actual_commitment = crate::runtime::rollback_journal_commitment(
                &rollback_checkpoint.snapshot,
                &checkpoint.journal,
                checkpoint.journal.len(),
            )
            .map_err(|err| rollback_runtime_error(err, target_batch_id.clone()))?;
            if approval
                .intent
                .replay_target
                .as_ref()
                .is_none_or(|target| target.journal_commitment != actual_commitment)
            {
                return Err(recovery_error(
                    "rollback_replay_commitment_mismatch",
                    "resolved replay suffix does not match signed journal commitment",
                    Some(target_batch_id.clone()),
                    None,
                    None,
                ));
            }
            checkpoint.snapshot = rollback_checkpoint.snapshot.clone();
        }
        let Some(batch_index) = self
            .authoritative_batches
            .iter()
            .position(|batch| batch.batch_id == target_batch_id)
        else {
            return Err(recovery_error(
                "batch_not_found",
                format!("authoritative batch {} not found", target_batch_id),
                Some(target_batch_id),
                None,
                None,
            ));
        };
        let invalidated_batch_ids = self
            .authoritative_batches
            .iter()
            .skip(batch_index.saturating_add(1))
            .map(|batch| batch.batch_id.clone())
            .collect::<Vec<_>>();
        let target_state_root = crate::runtime::World::from_snapshot(
            checkpoint.snapshot.clone(),
            checkpoint.journal.clone(),
        )
        .and_then(|world| world.current_state_root_hash())
        .map_err(|err| rollback_runtime_error(err, checkpoint.batch_id.clone()))?;
        if approval.intent.target_journal_len != checkpoint.journal.len()
            || approval.intent.expected_target_state_root != target_state_root
        {
            return Err(recovery_error(
                "rollback_authorization_invalid",
                "rollback authorization replay target does not match the selected checkpoint",
                Some(checkpoint.batch_id.clone()),
                None,
                None,
            ));
        }
        let rollback_ticket = approval.intent.rollback_ticket.clone();
        let authorization_nonce = approval.intent.nonce.clone();

        let reason = request.reason.trim();
        let rollback_reason = if reason.is_empty() {
            "authoritative_recovery_rollback".to_string()
        } else {
            reason.to_string()
        };
        let mut candidate_world = self.world.clone();
        let mut candidate_batches = self.authoritative_batches.clone();
        let mut candidate_challenges = self.authoritative_challenges.clone();
        let mut candidate_checkpoints = self.stable_checkpoints.clone();
        let candidate_reorg_epoch = self.reorg_epoch.saturating_add(1);

        candidate_world
            .rollback_to_snapshot_with_reconciliation(
                checkpoint.snapshot.clone(),
                checkpoint.journal.clone(),
                rollback_reason,
                Some(target_batch_id.as_str()),
                crate::runtime::RollbackAuthorizationEnvelope {
                    intent: crate::runtime::RollbackIntent {
                        schema_version: approval.intent.schema_version,
                        rollback_ticket: approval.intent.rollback_ticket,
                        snapshot_hash: approval.intent.snapshot_hash,
                        snapshot_journal_len: approval.intent.snapshot_journal_len,
                        target_journal_len: approval.intent.target_journal_len,
                        target_journal_commitment: approval
                            .intent
                            .replay_target
                            .as_ref()
                            .map(|target| target.journal_commitment.clone()),
                        expected_target_state_root: approval.intent.expected_target_state_root,
                        target_batch_id: approval.intent.target_batch_id,
                        reason: approval.intent.reason,
                        issued_at_ms: approval.intent.issued_at_ms,
                        expires_at_ms: approval.intent.expires_at_ms,
                        nonce: approval.intent.nonce,
                    },
                    signatures: approval
                        .signatures
                        .into_iter()
                        .map(|signature| crate::runtime::RollbackApprovalSignature {
                            authority_id: signature.authority_id,
                            role: match signature.role {
                                crate::viewer::protocol::RollbackAuthorityRole::OnCall => {
                                    crate::runtime::RollbackAuthorityRole::OnCall
                                }
                                crate::viewer::protocol::RollbackAuthorityRole::Governance => {
                                    crate::runtime::RollbackAuthorityRole::Governance
                                }
                            },
                            signature_scheme: signature.signature_scheme,
                            signature_hex: signature.signature_hex,
                        })
                        .collect(),
                },
                current_unix_time_ms(),
            )
            .map_err(|err| rollback_runtime_error(err, checkpoint.batch_id.clone()))?;

        #[cfg(test)]
        if self.recovery_fault_injection == Some(RuntimeRecoveryFaultInjection::ReconciliationDrift)
        {
            return Err(recovery_error(
                "rollback_reconciliation_drift",
                "injected rollback reconciliation drift",
                Some(checkpoint.batch_id.clone()),
                None,
                None,
            ));
        }

        candidate_batches.truncate(batch_index.saturating_add(1));
        candidate_challenges.retain(|challenge| {
            candidate_batches
                .iter()
                .any(|batch| batch.batch_id == challenge.batch_id)
        });
        if let Some(index) = candidate_checkpoints
            .iter()
            .position(|entry| entry.batch_id == checkpoint.batch_id)
        {
            candidate_checkpoints.truncate(index.saturating_add(1));
        }

        #[cfg(test)]
        if self.recovery_fault_injection == Some(RuntimeRecoveryFaultInjection::CursorCompute) {
            return Err(recovery_error(
                "cursor_compute_failed",
                "injected recovery cursor failure",
                Some(checkpoint.batch_id.clone()),
                None,
                None,
            ));
        }
        let snapshot_hash =
            compute_runtime_snapshot_hash(&candidate_world.snapshot()).map_err(|err| {
                recovery_error(
                    "cursor_compute_failed",
                    format!("{err:?}"),
                    Some(checkpoint.batch_id.clone()),
                    None,
                    None,
                )
            })?;
        candidate_world
            .record_rollback_outcome_recovery_metadata(
                authorization_nonce.as_str(),
                crate::runtime::RollbackOutcomeRecoveryMetadata {
                    target_batch_id: checkpoint.batch_id.clone(),
                    prior_reorg_epoch: self.reorg_epoch,
                    committed_reorg_epoch: candidate_reorg_epoch,
                    invalidated_batch_ids: invalidated_batch_ids.clone(),
                    dispositions: Vec::new(),
                },
            )
            .map_err(|err| rollback_runtime_error(err, checkpoint.batch_id.clone()))?;
        let ack = AuthoritativeRecoveryAck {
            status: AuthoritativeRecoveryStatus::RolledBack,
            reorg_epoch: candidate_reorg_epoch,
            snapshot_height: candidate_world.state().time,
            snapshot_hash: snapshot_hash.clone(),
            log_cursor: latest_runtime_event_seq(&candidate_world),
            stable_batch_id: Some(checkpoint.batch_id.clone()),
            player_id: None,
            agent_id: None,
            session_pubkey: None,
            replaced_by_pubkey: None,
            session_epoch: None,
            message: Some("rollback applied to stable checkpoint".to_string()),
            revoke_reason: None,
            revoked_by: None,
            rollback_receipt: Some(crate::viewer::protocol::AuthoritativeRollbackReceipt {
                receipt_id: format!("rollback:{rollback_ticket}:{authorization_nonce}"),
                authorization_nonce,
                rollback_ticket,
                target_batch_id: checkpoint.batch_id.clone(),
                invalidated_batch_ids,
                prior_reorg_epoch: self.reorg_epoch,
                committed_reorg_epoch: candidate_reorg_epoch,
                replay_from_snapshot_height: candidate_world.state().time,
                replay_from_log_cursor: latest_runtime_event_seq(&candidate_world),
                snapshot_hash,
                snapshot_reload_required: true,
                player_dispositions: Vec::new(),
            }),
            acknowledged_at_tick: candidate_world.state().time,
        };
        #[cfg(test)]
        if self.recovery_fault_injection == Some(RuntimeRecoveryFaultInjection::AckEncode) {
            return Err(recovery_error(
                "rollback_ack_encode_failed",
                "injected rollback acknowledgment encoding failure",
                Some(checkpoint.batch_id.clone()),
                None,
                None,
            ));
        }
        serde_json::to_vec(&ack).map_err(|err| {
            recovery_error(
                "rollback_ack_encode_failed",
                format!("{err:?}"),
                Some(checkpoint.batch_id.clone()),
                None,
                None,
            )
        })?;

        self.world = candidate_world;
        self.authoritative_batches = candidate_batches;
        self.authoritative_challenges = candidate_challenges;
        self.stable_checkpoints = candidate_checkpoints;
        self.reorg_epoch = candidate_reorg_epoch;
        self.rebuild_settlement_ranking_gate();
        Ok(ack)
    }

    fn register_session_key(
        &mut self,
        request: AuthoritativeSessionRegisterRequest,
    ) -> Result<AuthoritativeRecoveryAck<u64>, AuthoritativeRecoveryError> {
        let Some(auth) = request.auth.as_ref() else {
            return Err(recovery_error(
                "auth_proof_required",
                "session_register requires auth proof",
                None,
                Some(request.player_id.clone()),
                request.public_key.clone(),
            ));
        };
        let verified = verify_session_register_auth_proof(&request, auth).map_err(|message| {
            recovery_error(
                control_plane::map_auth_verify_error_code(message.as_str()),
                message,
                None,
                Some(request.player_id.clone()),
                request.public_key.clone(),
            )
        })?;
        let session_epoch = match self
            .session_policy
            .register_session(verified.player_id.as_str(), verified.public_key.as_str())
        {
            Ok(session_epoch) => session_epoch,
            Err(message) => {
                return Err(self.recovery_error_from_session_policy(
                    message,
                    verified.player_id.clone(),
                    Some(verified.public_key.clone()),
                ));
            }
        };
        self.llm_sidecar
            .consume_player_auth_nonce(verified.player_id.as_str(), verified.nonce)
            .map_err(|message| {
                recovery_error(
                    "auth_nonce_replay",
                    message,
                    None,
                    Some(verified.player_id.clone()),
                    Some(verified.public_key.clone()),
                )
            })?;

        let bound_agent_id = match request
            .requested_agent_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(agent_id) => {
                self.bind_player_session_agent(
                    agent_id,
                    verified.player_id.as_str(),
                    Some(verified.public_key.as_str()),
                    request.force_rebind,
                )
                .map_err(|message| {
                    recovery_error(
                        "player_bind_failed",
                        message,
                        None,
                        Some(verified.player_id.clone()),
                        Some(verified.public_key.clone()),
                    )
                })?;
                Some(agent_id.to_string())
            }
            None => self
                .llm_sidecar
                .bound_agent_for_player(verified.player_id.as_str())
                .map(ToOwned::to_owned),
        };

        let cursor = self.current_recovery_cursor().map_err(|err| {
            recovery_error(
                "cursor_compute_failed",
                format!("{err:?}"),
                None,
                Some(verified.player_id.clone()),
                Some(verified.public_key.clone()),
            )
        })?;
        Ok(AuthoritativeRecoveryAck {
            status: AuthoritativeRecoveryStatus::SessionRegistered,
            reorg_epoch: self.reorg_epoch,
            snapshot_height: cursor.snapshot_height,
            snapshot_hash: cursor.snapshot_hash,
            log_cursor: cursor.log_cursor,
            stable_batch_id: cursor.stable_batch_id,
            player_id: Some(verified.player_id),
            agent_id: bound_agent_id,
            session_pubkey: Some(verified.public_key),
            replaced_by_pubkey: None,
            session_epoch: Some(session_epoch),
            message: Some("session_registered".to_string()),
            revoke_reason: None,
            revoked_by: None,
            rollback_receipt: None,
            acknowledged_at_tick: self.world.state().time,
        })
    }

    fn handle_reconnect_sync(
        &mut self,
        request: AuthoritativeReconnectSyncRequest,
    ) -> Result<AuthoritativeRecoveryAck<u64>, AuthoritativeRecoveryError> {
        let player_id = request.player_id.trim().to_string();
        if player_id.is_empty() {
            return Err(recovery_error(
                "player_id_required",
                "reconnect_sync requires non-empty player_id",
                None,
                None,
                None,
            ));
        }

        let session_pubkey = request
            .session_pubkey
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let mut session_epoch = None;
        if let Some(pubkey) = session_pubkey.as_deref() {
            let epoch = match self
                .session_policy
                .validate_known_session_key(player_id.as_str(), pubkey)
            {
                Ok(epoch) => epoch,
                Err(message) => {
                    return Err(self.recovery_error_from_session_policy(
                        message,
                        player_id.clone(),
                        Some(pubkey.to_string()),
                    ));
                }
            };
            session_epoch = Some(epoch);
        }

        let cursor = self.current_recovery_cursor().map_err(|err| {
            recovery_error(
                "cursor_compute_failed",
                format!("{err:?}"),
                None,
                Some(player_id.clone()),
                session_pubkey.clone(),
            )
        })?;
        let stable_cursor = self
            .stable_checkpoints
            .back()
            .map(|entry| entry.log_cursor)
            .unwrap_or(0);

        let mut reasons = Vec::new();
        if request
            .expected_reorg_epoch
            .is_some_and(|epoch| epoch != self.reorg_epoch)
        {
            reasons.push(format!(
                "expected_reorg_epoch mismatch (client={}, server={})",
                request.expected_reorg_epoch.unwrap_or_default(),
                self.reorg_epoch
            ));
        }
        if let Some(last_known_cursor) = request.last_known_log_cursor {
            if last_known_cursor > cursor.log_cursor {
                reasons.push(format!(
                    "client cursor {} is ahead of server cursor {}",
                    last_known_cursor, cursor.log_cursor
                ));
            }
            if last_known_cursor < stable_cursor {
                reasons.push(format!(
                    "client cursor {} is behind stable cursor {}",
                    last_known_cursor, stable_cursor
                ));
            }
        }
        let message = if reasons.is_empty() {
            Some("delta_replay_allowed".to_string())
        } else {
            Some(format!("snapshot_reload_required: {}", reasons.join("; ")))
        };

        Ok(AuthoritativeRecoveryAck {
            status: AuthoritativeRecoveryStatus::CatchUpReady,
            reorg_epoch: self.reorg_epoch,
            snapshot_height: cursor.snapshot_height,
            snapshot_hash: cursor.snapshot_hash,
            log_cursor: cursor.log_cursor,
            stable_batch_id: cursor.stable_batch_id,
            player_id: Some(player_id),
            agent_id: self
                .llm_sidecar
                .bound_agent_for_player(request.player_id.as_str())
                .map(ToOwned::to_owned),
            session_pubkey,
            replaced_by_pubkey: None,
            session_epoch,
            message,
            revoke_reason: None,
            revoked_by: None,
            rollback_receipt: None,
            acknowledged_at_tick: self.world.state().time,
        })
    }

    fn revoke_session_key(
        &mut self,
        request: AuthoritativeSessionRevokeRequest,
    ) -> Result<AuthoritativeRecoveryAck<u64>, AuthoritativeRecoveryError> {
        let player_id = request.player_id.trim().to_string();
        if player_id.is_empty() {
            return Err(recovery_error(
                "player_id_required",
                "revoke_session requires non-empty player_id",
                None,
                None,
                None,
            ));
        }

        let (revoked_pubkey, session_epoch) = self
            .session_policy
            .revoke_session(player_id.as_str(), request.session_pubkey.as_deref())
            .map_err(|message| {
                recovery_error(
                    map_session_policy_error_code(message.as_str()),
                    message,
                    None,
                    Some(player_id.clone()),
                    request.session_pubkey.clone(),
                )
            })?;
        let revoke_metadata = RuntimeSessionRevokeMetadata {
            revoke_reason: normalize_optional_string(Some(request.revoke_reason.as_str())),
            revoked_by: normalize_optional_string(request.revoked_by.as_deref()),
        };
        self.record_session_revoke_metadata(
            player_id.as_str(),
            revoked_pubkey.as_str(),
            revoke_metadata.clone(),
        );
        self.clear_player_auth_runtime_state(player_id.as_str());
        self.apply_session_revoke_binding(player_id.as_str(), revoked_pubkey.as_str());

        let cursor = self.current_recovery_cursor().map_err(|err| {
            recovery_error(
                "cursor_compute_failed",
                format!("{err:?}"),
                None,
                Some(player_id.clone()),
                Some(revoked_pubkey.clone()),
            )
        })?;
        Ok(AuthoritativeRecoveryAck {
            status: AuthoritativeRecoveryStatus::SessionRevoked,
            reorg_epoch: self.reorg_epoch,
            snapshot_height: cursor.snapshot_height,
            snapshot_hash: cursor.snapshot_hash,
            log_cursor: cursor.log_cursor,
            stable_batch_id: cursor.stable_batch_id,
            player_id: Some(player_id),
            agent_id: None,
            session_pubkey: Some(revoked_pubkey),
            replaced_by_pubkey: None,
            session_epoch: Some(session_epoch),
            message: Some(request.revoke_reason.trim().to_string()),
            revoke_reason: revoke_metadata.revoke_reason,
            revoked_by: revoke_metadata.revoked_by,
            rollback_receipt: None,
            acknowledged_at_tick: self.world.state().time,
        })
    }

    fn rotate_session_key(
        &mut self,
        request: AuthoritativeSessionRotateRequest,
    ) -> Result<AuthoritativeRecoveryAck<u64>, AuthoritativeRecoveryError> {
        let player_id = request.player_id.trim().to_string();
        if player_id.is_empty() {
            return Err(recovery_error(
                "player_id_required",
                "rotate_session requires non-empty player_id",
                None,
                None,
                None,
            ));
        }

        let session_epoch = self
            .session_policy
            .rotate_session(
                player_id.as_str(),
                request.old_session_pubkey.as_str(),
                request.new_session_pubkey.as_str(),
            )
            .map_err(|message| {
                recovery_error(
                    map_session_policy_error_code(message.as_str()),
                    message,
                    None,
                    Some(player_id.clone()),
                    Some(request.old_session_pubkey.clone()),
                )
            })?;
        self.clear_player_auth_runtime_state(player_id.as_str());
        self.apply_session_rotate_binding(
            player_id.as_str(),
            request.old_session_pubkey.as_str(),
            request.new_session_pubkey.as_str(),
        );

        let cursor = self.current_recovery_cursor().map_err(|err| {
            recovery_error(
                "cursor_compute_failed",
                format!("{err:?}"),
                None,
                Some(player_id.clone()),
                Some(request.old_session_pubkey.clone()),
            )
        })?;
        Ok(AuthoritativeRecoveryAck {
            status: AuthoritativeRecoveryStatus::SessionRotated,
            reorg_epoch: self.reorg_epoch,
            snapshot_height: cursor.snapshot_height,
            snapshot_hash: cursor.snapshot_hash,
            log_cursor: cursor.log_cursor,
            stable_batch_id: cursor.stable_batch_id,
            player_id: Some(player_id),
            agent_id: self
                .llm_sidecar
                .bound_agent_for_player(request.player_id.as_str())
                .map(ToOwned::to_owned),
            session_pubkey: Some(request.old_session_pubkey),
            replaced_by_pubkey: Some(request.new_session_pubkey),
            session_epoch: Some(session_epoch),
            message: Some(request.rotate_reason.trim().to_string()),
            revoke_reason: None,
            revoked_by: None,
            rollback_receipt: None,
            acknowledged_at_tick: self.world.state().time,
        })
    }

    pub(super) fn current_recovery_cursor(
        &self,
    ) -> Result<RuntimeRecoveryCursor, ViewerRuntimeLiveServerError> {
        let snapshot_hash = compute_runtime_snapshot_hash(&self.world.snapshot())?;
        Ok(RuntimeRecoveryCursor {
            snapshot_hash,
            snapshot_height: self.world.state().time,
            log_cursor: latest_runtime_event_seq(&self.world),
            stable_batch_id: self
                .stable_checkpoints
                .back()
                .map(|entry| entry.batch_id.clone()),
        })
    }

    pub(super) fn clear_player_auth_runtime_state(&mut self, player_id: &str) {
        self.llm_sidecar
            .player_auth_last_nonce
            .remove(player_id.trim());
        self.llm_sidecar
            .clear_chat_intent_acks_for_player(player_id.trim());
    }

    fn apply_session_revoke_binding(&mut self, player_id: &str, _revoked_pubkey: &str) {
        if let Some(event) = self.llm_sidecar.clear_player_binding(player_id) {
            self.enqueue_virtual_event(event);
        }
    }

    fn apply_session_rotate_binding(
        &mut self,
        player_id: &str,
        old_pubkey: &str,
        new_pubkey: &str,
    ) {
        let mut affected_agents = Vec::new();
        for (agent_id, bound_player) in &self.llm_sidecar.agent_player_bindings {
            if bound_player == player_id {
                affected_agents.push(agent_id.clone());
            }
        }
        for agent_id in affected_agents {
            let should_replace = self
                .llm_sidecar
                .agent_public_key_bindings
                .get(agent_id.as_str())
                .map_or(true, |bound| bound == old_pubkey);
            if should_replace {
                self.llm_sidecar
                    .agent_public_key_bindings
                    .insert(agent_id, new_pubkey.to_string());
            }
        }
    }

    fn record_session_revoke_metadata(
        &mut self,
        player_id: &str,
        session_pubkey: &str,
        metadata: RuntimeSessionRevokeMetadata,
    ) {
        self.session_revoke_metadata.insert(
            session_revoke_metadata_key(player_id, session_pubkey),
            metadata,
        );
    }

    fn session_revoke_metadata(
        &self,
        player_id: &str,
        session_pubkey: &str,
    ) -> Option<&RuntimeSessionRevokeMetadata> {
        self.session_revoke_metadata
            .get(&session_revoke_metadata_key(player_id, session_pubkey))
    }

    fn recovery_error_from_session_policy(
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

    fn bind_player_session_agent(
        &mut self,
        agent_id: &str,
        player_id: &str,
        public_key: Option<&str>,
        allow_player_rebind: bool,
    ) -> Result<(), String> {
        if allow_player_rebind {
            if !self.world.state().agents.contains_key(agent_id) {
                return Err(format!("agent not found: {agent_id}"));
            }
        } else {
            control_plane::ensure_agent_player_binding_target_runtime(
                &self.world,
                &self.llm_sidecar,
                agent_id,
                player_id,
                public_key,
            )
            .map_err(|err| err.message)?;
        }
        for event in self.llm_sidecar.bind_agent_player(
            agent_id,
            player_id,
            public_key,
            allow_player_rebind,
        )? {
            self.enqueue_virtual_event(event);
        }
        Ok(())
    }
}

fn current_unix_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

fn rollback_runtime_error(
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
