use super::authoritative::compute_runtime_snapshot_hash;
use super::recovery_receipt::{recovery_error, rollback_runtime_error};
use super::*;
use sha2::{Digest, Sha256};

#[cfg(test)]
pub(super) use super::recovery_persistence::RuntimeRecoveryFaultInjection;
use crate::viewer::{
    claim_registration_grant_nonce_for_recovery, consume_registration_grant_nonce,
};

impl ViewerRuntimeLiveServer {
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
                | AuthoritativeRecoveryCommand::ReevaluateRollbackReadiness { .. }
                | AuthoritativeRecoveryCommand::TransitionRollbackCompensation { .. }
                | AuthoritativeRecoveryCommand::ResolveRollbackAttribution { .. }
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
            AuthoritativeRecoveryCommand::ReevaluateRollbackReadiness {
                authorization_nonce,
                audit_evidence,
            } => self
                .reevaluate_rollback_readiness(authorization_nonce, audit_evidence)
                .map(|ack| (ack, false)),
            AuthoritativeRecoveryCommand::TransitionRollbackCompensation { request } => self
                .transition_rollback_compensation(request)
                .map(|ack| (ack, false)),
            AuthoritativeRecoveryCommand::ResolveRollbackAttribution { request } => self
                .resolve_rollback_attribution(request)
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

    pub(super) fn rollback_to_stable_checkpoint_v2(
        &mut self,
        request: AuthoritativeRollbackRequest,
        canonical_v2_payload: Vec<u8>,
    ) -> Result<AuthoritativeRecoveryAck<u64>, AuthoritativeRecoveryError> {
        self.rollback_to_stable_checkpoint_payload(request, Some(canonical_v2_payload))
    }

    fn rollback_to_stable_checkpoint(
        &mut self,
        request: AuthoritativeRollbackRequest,
    ) -> Result<AuthoritativeRecoveryAck<u64>, AuthoritativeRecoveryError> {
        self.rollback_to_stable_checkpoint_payload(request, None)
    }

    fn rollback_to_stable_checkpoint_payload(
        &mut self,
        request: AuthoritativeRollbackRequest,
        canonical_v2_payload: Option<Vec<u8>>,
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
        let checkpoint_batch_id = approval
            .intent
            .rollback_checkpoint
            .as_ref()
            .map(|entry| entry.batch_id.as_str())
            .unwrap_or(target_batch_id.as_str());
        let checkpoint_batch_index = self
            .authoritative_batches
            .iter()
            .position(|batch| batch.batch_id == checkpoint_batch_id)
            .ok_or_else(|| {
                recovery_error(
                    "rollback_checkpoint_not_found",
                    "rollback checkpoint is missing from authoritative batch history",
                    Some(checkpoint_batch_id.to_string()),
                    None,
                    None,
                )
            })?;
        if checkpoint_batch_index > batch_index
            || (approval.intent.rollback_checkpoint.is_some()
                && checkpoint_batch_index == batch_index)
        {
            return Err(recovery_error(
                "rollback_target_precedes_checkpoint",
                "replay target must be strictly after rollback checkpoint in authoritative history",
                Some(target_batch_id.clone()),
                None,
                None,
            ));
        }
        let mut seen_actions = std::collections::BTreeSet::new();
        let mut dispositions = Vec::new();
        let mut missing_player_attribution = false;
        for (index, batch) in self
            .authoritative_batches
            .iter()
            .enumerate()
            .skip(checkpoint_batch_index.saturating_add(1))
        {
            for event in &batch.events {
                let Some(action_id) = (match event
                    .runtime_event
                    .as_ref()
                    .and_then(|event| event.caused_by.as_ref())
                {
                    Some(crate::runtime::CausedBy::Action(action_id)) => Some(*action_id),
                    _ => None,
                }) else {
                    continue;
                };
                let Some(player_id) = self.runtime_action_players.get(&action_id).cloned() else {
                    // Runtime-authored automation remains covered by the journal commitment as
                    // separate system-event evidence. Because the action provenance cannot prove
                    // that this is automation rather than a player action, retain an explicit
                    // fail-closed census blocker instead of silently declaring coverage complete.
                    missing_player_attribution = true;
                    if seen_actions.insert(action_id) {
                        dispositions.push(crate::runtime::RollbackEventDisposition {
                            source_batch_id: batch.batch_id.clone(),
                            source_event_id: event.id,
                            status: if index <= batch_index {
                                crate::runtime::RollbackDispositionStatus::Replayed
                            } else {
                                crate::runtime::RollbackDispositionStatus::RejectedFork
                            },
                            compensation: None,
                            player_id: None,
                            action_id: Some(action_id.to_string()),
                            system_authored: false,
                        });
                    }
                    continue;
                };
                if !seen_actions.insert(action_id) {
                    continue;
                }
                let rejected_fork = index > batch_index;
                let compensation = rejected_fork.then(|| {
                    let public_case_digest = hex::encode(Sha256::digest(format!(
                        "oasis7:rollback-compensation-public:v1:{}:{}:{}",
                        batch.batch_id, event.id, action_id
                    )));
                    crate::runtime::RollbackCompensationCaseRef {
                        owner_id: "player_support".to_string(),
                        ticket_id: format!("rollback-case-{}", &public_case_digest[..16]),
                        state: crate::runtime::RollbackCompensationState::PendingAuthorization,
                    }
                });
                dispositions.push(crate::runtime::RollbackEventDisposition {
                    source_batch_id: batch.batch_id.clone(),
                    source_event_id: event.id,
                    status: if index <= checkpoint_batch_index {
                        crate::runtime::RollbackDispositionStatus::PreservedAtTarget
                    } else if index <= batch_index {
                        crate::runtime::RollbackDispositionStatus::Replayed
                    } else {
                        crate::runtime::RollbackDispositionStatus::CompensationRequired
                    },
                    compensation,
                    player_id: Some(player_id),
                    action_id: Some(action_id.to_string()),
                    system_authored: false,
                });
            }
        }
        let affected_events = dispositions
            .iter()
            .map(|entry| crate::runtime::RollbackSourceEventIdentity {
                source_batch_id: entry.source_batch_id.clone(),
                source_event_id: entry.source_event_id,
            })
            .collect::<Vec<_>>();
        let initial_readiness_blockers = if missing_player_attribution {
            vec![
                "readiness_evaluation_required".to_string(),
                "player_attribution_incomplete".to_string(),
            ]
        } else {
            vec!["readiness_evaluation_required".to_string()]
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
        let receipt_checkpoint = approval.intent.rollback_checkpoint.clone();
        let receipt_target = approval.intent.replay_target.clone();
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

        let rollback_result = if let Some(payload) = canonical_v2_payload.as_deref() {
            candidate_world.rollback_to_snapshot_with_reconciliation_v2(
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
                payload,
                super::recovery_receipt::current_unix_time_ms(),
            )
        } else {
            candidate_world.rollback_to_snapshot_with_reconciliation(
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
                super::recovery_receipt::current_unix_time_ms(),
            )
        };
        rollback_result.map_err(|err| rollback_runtime_error(err, checkpoint.batch_id.clone()))?;
        let immutable_outcome = candidate_world
            .rollback_nonce_outcome(authorization_nonce.as_str())
            .ok_or_else(|| {
                recovery_error(
                    "rollback_receipt_incomplete",
                    "rollback outcome is missing after the replay commit",
                    Some(checkpoint.batch_id.clone()),
                    None,
                    None,
                )
            })?;
        let canonical_intent_digest = immutable_outcome.canonical_intent_hash.clone();
        let receipt_checkpoint = Some(receipt_checkpoint.unwrap_or(
            crate::viewer::protocol::RollbackCheckpointRef {
                batch_id: checkpoint.batch_id.clone(),
                snapshot_hash: immutable_outcome.rollback_checkpoint_snapshot_hash.clone(),
                snapshot_journal_len: immutable_outcome.rollback_checkpoint_journal_len,
            },
        ));
        let receipt_target = Some(receipt_target.unwrap_or(
            crate::viewer::protocol::RollbackReplayTarget {
                batch_id: checkpoint.batch_id.clone(),
                target_journal_len: immutable_outcome.target_journal_len,
                expected_target_state_root: immutable_outcome.target_state_root.clone(),
                journal_commitment: immutable_outcome.target_journal_commitment.clone(),
            },
        ));

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
        let snapshot_height = candidate_world.state().time;
        let log_cursor = latest_runtime_event_seq(&candidate_world);
        let receipt_id = format!("rollback:{rollback_ticket}:{authorization_nonce}");
        let affected_census_digest =
            crate::runtime::rollback_affected_census_digest(affected_events.as_slice())
                .map_err(|err| rollback_runtime_error(err, checkpoint.batch_id.clone()))?;
        candidate_world
            .complete_rollback_outcome(
                authorization_nonce.as_str(),
                crate::runtime::RollbackOutcomeRecoveryMetadata {
                    target_batch_id: checkpoint.batch_id.clone(),
                    prior_reorg_epoch: self.reorg_epoch,
                    committed_reorg_epoch: candidate_reorg_epoch,
                    invalidated_batch_ids: invalidated_batch_ids.clone(),
                    dispositions: dispositions.clone(),
                },
                affected_events.as_slice(),
                crate::runtime::RollbackReceiptProjection {
                    receipt_id: receipt_id.clone(),
                    canonical_intent_digest: immutable_outcome.canonical_intent_hash.clone(),
                    rollback_checkpoint_batch_id: receipt_checkpoint
                        .as_ref()
                        .map(|checkpoint| checkpoint.batch_id.clone())
                        .unwrap_or_else(|| checkpoint.batch_id.clone()),
                    rollback_checkpoint_snapshot_hash: immutable_outcome
                        .rollback_checkpoint_snapshot_hash
                        .clone(),
                    rollback_checkpoint_journal_len: immutable_outcome
                        .rollback_checkpoint_journal_len,
                    replay_target_batch_id: checkpoint.batch_id.clone(),
                    replay_target_journal_len: immutable_outcome.target_journal_len,
                    replay_target_state_root: immutable_outcome.target_state_root.clone(),
                    replay_target_journal_commitment: immutable_outcome
                        .target_journal_commitment
                        .clone(),
                    affected_census_digest,
                    affected_census_count: affected_events.len(),
                    readiness_blockers: initial_readiness_blockers.clone(),
                    snapshot_height,
                    snapshot_hash: snapshot_hash.clone(),
                    log_cursor,
                    acknowledged_at_tick: snapshot_height,
                },
            )
            .map_err(|err| rollback_runtime_error(err, checkpoint.batch_id.clone()))?;
        let ack = AuthoritativeRecoveryAck {
            status: AuthoritativeRecoveryStatus::RolledBack,
            reorg_epoch: candidate_reorg_epoch,
            snapshot_height,
            snapshot_hash: snapshot_hash.clone(),
            log_cursor,
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
                receipt_id,
                authorization_nonce: authorization_nonce.clone(),
                rollback_ticket: rollback_ticket.clone(),
                canonical_intent_digest,
                rollback_checkpoint: receipt_checkpoint,
                replay_target: receipt_target.clone(),
                journal_commitment: immutable_outcome.target_journal_commitment.clone(),
                target_state_root: immutable_outcome.target_state_root.clone(),
                affected_event_census: affected_events
                    .iter()
                    .map(|entry| crate::viewer::protocol::RollbackSourceEventRef {
                        source_batch_id: entry.source_batch_id.clone(),
                        source_event_id: entry.source_event_id,
                    })
                    .collect(),
                target_batch_id: checkpoint.batch_id.clone(),
                invalidated_batch_ids,
                prior_reorg_epoch: self.reorg_epoch,
                committed_reorg_epoch: candidate_reorg_epoch,
                replay_from_snapshot_height: snapshot_height,
                replay_from_log_cursor: log_cursor,
                snapshot_hash,
                snapshot_reload_required: true,
                player_dispositions: super::recovery_receipt::viewer_dispositions(
                    rollback_ticket.as_str(),
                    authorization_nonce.as_str(),
                    dispositions.as_slice(),
                ),
                ready_for_all_clear: false,
                readiness_blockers: initial_readiness_blockers,
            }),
            acknowledged_at_tick: snapshot_height,
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
        let recovery_metadata = serde_json::to_vec(
            &super::recovery_receipt::RuntimeAuthoritativeRecoveryGeneration {
                schema_version: 1,
                ack: ack.clone(),
                authoritative_batches: candidate_batches.clone(),
                next_authoritative_batch_id: self.next_authoritative_batch_id,
                authoritative_challenges: candidate_challenges.clone(),
                next_authoritative_challenge_id: self.next_authoritative_challenge_id,
                stable_checkpoints: candidate_checkpoints.clone(),
                reorg_epoch: candidate_reorg_epoch,
                session_policy: self.session_policy.clone(),
                session_revoke_metadata: self
                    .session_revoke_metadata
                    .iter()
                    .map(|((player_id, session_pubkey), metadata)| {
                        super::recovery_receipt::RuntimePersistedSessionRevokeMetadata {
                            player_id: player_id.clone(),
                            session_pubkey: session_pubkey.clone(),
                            metadata: metadata.clone(),
                        }
                    })
                    .collect(),
                rollback_readiness: self.rollback_readiness.clone(),
                runtime_action_players: self.runtime_action_players.clone(),
                consumed_rollback_operator_nonces: self.consumed_rollback_operator_nonces.clone(),
                session_side_effects: self.persisted_session_side_effects(),
            },
        )
        .map_err(|err| {
            recovery_error(
                "rollback_ack_encode_failed",
                format!("{err:?}"),
                Some(checkpoint.batch_id.clone()),
                None,
                None,
            )
        })?;
        if self.authoritative_recovery_dir().is_some() {
            match self.commit_authoritative_recovery_envelope(&candidate_world, &recovery_metadata)
            {
                Ok(crate::runtime::AuthoritativeRecoveryCommitStatus::Committed(_)) => {
                    return Ok(ack);
                }
                Ok(crate::runtime::AuthoritativeRecoveryCommitStatus::NotCommitted {
                    current_generation_id,
                }) => {
                    return Err(recovery_error(
                        "rollback_persistence_commit_failed",
                        format!(
                            "authoritative recovery generation was not committed; current_generation_id={current_generation_id:?}"
                        ),
                        Some(checkpoint.batch_id.clone()),
                        None,
                        None,
                    ));
                }
                Err(crate::runtime::AuthoritativeRecoveryCommitError::StatusUnknown { reason }) => {
                    return Err(recovery_error(
                        "rollback_persistence_status_unknown",
                        reason,
                        Some(checkpoint.batch_id.clone()),
                        None,
                        None,
                    ));
                }
            }
        }

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
        let verified =
            verify_session_register_auth_proof_for_recovery(&request, auth).map_err(|message| {
                recovery_error(
                    control_plane::map_auth_verify_error_code(message.as_str()),
                    message,
                    None,
                    Some(request.player_id.clone()),
                    request.public_key.clone(),
                )
            })?;
        let mutation_snapshot = self.session_mutation_snapshot();
        let session_registration_plan = match self.session_policy.validate_session_registration(
            verified.player_id.as_str(),
            verified.public_key.as_str(),
            verified.hosted_registration_nonce.is_some(),
        ) {
            Ok(plan) => plan,
            Err(message) => {
                return Err(self.recovery_error_from_session_policy(
                    message,
                    verified.player_id.clone(),
                    Some(verified.public_key.clone()),
                ));
            }
        };
        let session_epoch = session_registration_plan.session_epoch();
        let rotates_session_key = session_registration_plan.rotates_key();
        self.llm_sidecar
            .validate_player_auth_nonce(verified.player_id.as_str(), verified.nonce)
            .map_err(|message| {
                recovery_error(
                    "auth_nonce_replay",
                    message,
                    None,
                    Some(verified.player_id.clone()),
                    Some(verified.public_key.clone()),
                )
            })?;

        let cursor = self.current_recovery_cursor().map_err(|err| {
            recovery_error(
                "cursor_compute_failed",
                format!("{err:?}"),
                None,
                Some(verified.player_id.clone()),
                Some(verified.public_key.clone()),
            )
        })?;
        let current_bound_agent_id = self
            .llm_sidecar
            .bound_agent_for_player(verified.player_id.as_str())
            .map(ToOwned::to_owned);
        let (bound_agent_id, binding_plan) = match request
            .requested_agent_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(agent_id) => {
                let rotates_current_binding =
                    rotates_session_key && current_bound_agent_id.as_deref() == Some(agent_id);
                let plan = self
                    .plan_player_session_agent_binding(
                        agent_id,
                        verified.player_id.as_str(),
                        Some(verified.public_key.as_str()),
                        request.force_rebind || rotates_current_binding,
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
                (Some(agent_id.to_string()), Some(plan))
            }
            None if rotates_session_key => {
                let bound_agent_id = current_bound_agent_id.clone();
                let plan = match bound_agent_id.as_deref() {
                    Some(agent_id) => Some(
                        self.plan_player_session_agent_binding(
                            agent_id,
                            verified.player_id.as_str(),
                            Some(verified.public_key.as_str()),
                            true,
                        )
                        .map_err(|message| {
                            recovery_error(
                                "player_bind_failed",
                                message,
                                None,
                                Some(verified.player_id.clone()),
                                Some(verified.public_key.clone()),
                            )
                        })?,
                    ),
                    None => None,
                };
                (bound_agent_id, plan)
            }
            None => (current_bound_agent_id, None),
        };

        if let Some(registration_nonce) = verified.hosted_registration_nonce.as_deref() {
            let claim_result = match self.authoritative_recovery_dir() {
                Some(recovery_dir) => claim_registration_grant_nonce_for_recovery(
                    registration_nonce,
                    recovery_dir.as_path(),
                ),
                None => consume_registration_grant_nonce(registration_nonce),
            };
            claim_result.map_err(|message| {
                recovery_error(
                    control_plane::map_auth_verify_error_code(message.as_str()),
                    message,
                    None,
                    Some(verified.player_id.clone()),
                    Some(verified.public_key.clone()),
                )
            })?;
        }

        self.session_policy
            .commit_session_registration(session_registration_plan);
        self.llm_sidecar
            .commit_player_auth_nonce(verified.player_id.as_str(), verified.nonce);
        if let Some(plan) = binding_plan {
            for event in self.llm_sidecar.apply_agent_player_binding_plan(plan) {
                self.enqueue_virtual_event(event);
            }
        }
        let ack = AuthoritativeRecoveryAck {
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
        };
        if let Err(error) = self.persist_current_recovery_generation(&ack) {
            self.restore_session_mutation_snapshot(mutation_snapshot);
            return Err(error);
        }
        Ok(ack)
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

        let mutation_snapshot = self.session_mutation_snapshot();
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
        let ack = AuthoritativeRecoveryAck {
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
        };
        if let Err(error) = self.persist_current_recovery_generation(&ack) {
            self.restore_session_mutation_snapshot(mutation_snapshot);
            return Err(error);
        }
        Ok(ack)
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

        let mutation_snapshot = self.session_mutation_snapshot();
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
        let rotation_metadata = RuntimeSessionRevokeMetadata {
            revoke_reason: normalize_optional_string(Some(request.rotate_reason.as_str())),
            revoked_by: normalize_optional_string(request.rotated_by.as_deref()),
        };
        self.record_session_revoke_metadata(
            player_id.as_str(),
            request.old_session_pubkey.as_str(),
            rotation_metadata.clone(),
        );
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
        let ack = AuthoritativeRecoveryAck {
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
            revoke_reason: rotation_metadata.revoke_reason,
            revoked_by: rotation_metadata.revoked_by,
            rollback_receipt: None,
            acknowledged_at_tick: self.world.state().time,
        };
        if let Err(error) = self.persist_current_recovery_generation(&ack) {
            self.restore_session_mutation_snapshot(mutation_snapshot);
            return Err(error);
        }
        Ok(ack)
    }
}
