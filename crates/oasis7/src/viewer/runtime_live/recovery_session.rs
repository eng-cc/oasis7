use super::authoritative::compute_runtime_snapshot_hash;
use super::session_policy::RuntimeRecoveryCursor;
use super::*;

impl ViewerRuntimeLiveServer {
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

    pub(super) fn apply_session_revoke_binding(&mut self, player_id: &str, _revoked_pubkey: &str) {
        if let Some(event) = self.llm_sidecar.clear_player_binding(player_id) {
            self.enqueue_virtual_event(event);
        }
    }

    pub(super) fn apply_session_rotate_binding(
        &mut self,
        player_id: &str,
        old_pubkey: &str,
        new_pubkey: &str,
    ) {
        let affected_agents = self
            .llm_sidecar
            .agent_player_bindings
            .iter()
            .filter(|(_, bound_player)| *bound_player == player_id)
            .map(|(agent_id, _)| agent_id.clone())
            .collect::<Vec<_>>();
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

    pub(super) fn record_session_revoke_metadata(
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

    pub(super) fn bind_player_session_agent(
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
