use super::authoritative::compute_runtime_snapshot_hash;
use super::session_policy::RuntimeRecoveryCursor;
use super::*;

impl ViewerRuntimeLiveServer {
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

    fn rollback_to_stable_checkpoint(
        &mut self,
        request: AuthoritativeRollbackRequest,
    ) -> Result<AuthoritativeRecoveryAck<u64>, AuthoritativeRecoveryError> {
        let target_batch_id = request
            .target_batch_id
            .clone()
            .or_else(|| self.stable_checkpoints.back().map(|entry| entry.batch_id.clone()))
            .ok_or_else(|| {
                recovery_error(
                    "stable_checkpoint_not_found",
                    "no stable checkpoint available for rollback",
                    None,
                    None,
                    None,
                )
            })?;
        let checkpoint = self
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

        let reason = request.reason.trim();
        let rollback_reason = if reason.is_empty() {
            "authoritative_recovery_rollback".to_string()
        } else {
            reason.to_string()
        };
        self.world
            .rollback_to_snapshot_with_reconciliation(
                checkpoint.snapshot.clone(),
                checkpoint.journal.clone(),
                rollback_reason,
            )
            .map_err(|err| {
                recovery_error(
                    "rollback_failed",
                    format!("{err:?}"),
                    Some(checkpoint.batch_id.clone()),
                    None,
                    None,
                )
            })?;

        self.authoritative_batches
            .truncate(batch_index.saturating_add(1));
        self.authoritative_challenges.retain(|challenge| {
            self.authoritative_batches
                .iter()
                .any(|batch| batch.batch_id == challenge.batch_id)
        });
        self.prune_stable_checkpoints_after_batch(checkpoint.batch_id.as_str());
        self.rebuild_settlement_ranking_gate();
        self.reorg_epoch = self.reorg_epoch.saturating_add(1);

        let cursor = self.current_recovery_cursor().map_err(|err| {
            recovery_error(
                "cursor_compute_failed",
                format!("{err:?}"),
                Some(checkpoint.batch_id.clone()),
                None,
                None,
            )
        })?;
        Ok(AuthoritativeRecoveryAck {
            status: AuthoritativeRecoveryStatus::RolledBack,
            reorg_epoch: self.reorg_epoch,
            snapshot_height: cursor.snapshot_height,
            snapshot_hash: cursor.snapshot_hash,
            log_cursor: cursor.log_cursor,
            stable_batch_id: Some(checkpoint.batch_id),
            player_id: None,
            agent_id: None,
            session_pubkey: None,
            replaced_by_pubkey: None,
            session_epoch: None,
            message: Some("rollback applied to stable checkpoint".to_string()),
            revoke_reason: None,
            revoked_by: None,
            acknowledged_at_tick: self.world.state().time,
        })
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
        control_plane::ensure_agent_player_access_runtime(
            &self.world,
            &self.llm_sidecar,
            agent_id,
            player_id,
            public_key,
        )
        .map_err(|err| err.message)?;
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
