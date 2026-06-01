use crate::network_bridge::GapSyncFetchCommitResponse;
use crate::replication_state_reconcile::ReplicationCommitPayload;

use super::node_engine_core::InboundSlotWindow;
use super::*;

impl PosNodeEngine {
    fn observe_peer_commit_message(&mut self, commit: &GossipCommitMessage) {
        if commit.height == 0 {
            return;
        }
        let validator_id = self.validator_id_for_peer_head(commit.node_id.as_str());
        if validator_id
            .as_deref()
            .map(|id| self.quarantined_validators.contains(id))
            .unwrap_or(false)
        {
            return;
        }
        let now_ms = crate::runtime_util::now_unix_ms();
        let next_head = PeerCommittedHead {
            height: commit.height,
            block_hash: commit.block_hash.clone(),
            committed_at_ms: commit.committed_at_ms,
            observed_at_ms: now_ms,
            execution_block_hash: commit.execution_block_hash.clone(),
            execution_state_root: commit.execution_state_root.clone(),
            action_root: commit.action_root.clone(),
            public_key_hex: commit.public_key_hex.clone(),
            signature_hex: commit.signature_hex.clone(),
        };
        if let Some(previous) = self.peer_heads.get(commit.node_id.as_str()) {
            if commit.height < previous.height {
                return;
            }
            if commit.height == previous.height && peer_commit_heads_conflict(previous, &next_head)
            {
                if let Some(validator_id) = validator_id {
                    self.record_commit_equivocation_evidence(
                        validator_id,
                        commit.node_id.clone(),
                        previous.clone(),
                        next_head,
                        now_ms,
                    );
                    self.peer_heads.remove(commit.node_id.as_str());
                }
                return;
            }
        }
        self.network_committed_height = self.network_committed_height.max(commit.height);
        self.peer_heads.insert(commit.node_id.clone(), next_head);
    }

    fn record_commit_equivocation_evidence(
        &mut self,
        validator_id: String,
        node_id: String,
        first: PeerCommittedHead,
        second: PeerCommittedHead,
        observed_at_ms: i64,
    ) {
        let key = format!("commit_equivocation:{validator_id}:{}", first.height);
        self.quarantined_validators.insert(validator_id.clone());
        self.misbehavior_evidence
            .entry(key)
            .or_insert_with(|| ConsensusMisbehaviorEvidence {
                kind: "commit_equivocation".to_string(),
                validator_id: validator_id.clone(),
                node_id,
                height: first.height,
                observed_at_ms,
                first_block_hash: first.block_hash,
                second_block_hash: second.block_hash,
                first_execution_block_hash: first.execution_block_hash,
                second_execution_block_hash: second.execution_block_hash,
                first_execution_state_root: first.execution_state_root,
                second_execution_state_root: second.execution_state_root,
                first_action_root: first.action_root,
                second_action_root: second.action_root,
                first_public_key_hex: first.public_key_hex,
                second_public_key_hex: second.public_key_hex,
                first_signature_hex: first.signature_hex,
                second_signature_hex: second.signature_hex,
                slashable_stake: self
                    .validators
                    .get(validator_id.as_str())
                    .copied()
                    .unwrap_or(0),
                total_stake: self.total_stake,
                validator_stake_root: self.validator_stake_root.clone(),
                quarantined: true,
            });
    }

    pub(super) fn sync_replication_height_once(
        &self,
        endpoint: &ReplicationNetworkEndpoint,
        node_id: &str,
        world_id: &str,
        replication_runtime: &mut ReplicationRuntime,
        height: u64,
    ) -> Result<GapSyncHeightOutcome, NodeError> {
        self.sync_replication_height_once_with_fetch(
            endpoint,
            node_id,
            world_id,
            replication_runtime,
            height,
            |endpoint, request| endpoint.request_fetch_commit_for_gap_sync(request),
        )
    }

    pub(super) fn sync_replication_height_once_for_successor_probe(
        &self,
        endpoint: &ReplicationNetworkEndpoint,
        node_id: &str,
        world_id: &str,
        replication_runtime: &mut ReplicationRuntime,
        height: u64,
    ) -> Result<GapSyncHeightOutcome, NodeError> {
        self.sync_replication_height_once_with_fetch(
            endpoint,
            node_id,
            world_id,
            replication_runtime,
            height,
            |endpoint, request| endpoint.request_fetch_commit_for_gap_sync_single_probe(request),
        )
    }

    fn sync_replication_height_once_with_fetch(
        &self,
        endpoint: &ReplicationNetworkEndpoint,
        node_id: &str,
        world_id: &str,
        replication_runtime: &mut ReplicationRuntime,
        height: u64,
        fetch_commit: impl FnOnce(
            &ReplicationNetworkEndpoint,
            &FetchCommitRequest,
        ) -> Result<GapSyncFetchCommitResponse, NodeError>,
    ) -> Result<GapSyncHeightOutcome, NodeError> {
        let request = replication_runtime.build_fetch_commit_request(world_id, height)?;
        let fetch_commit = fetch_commit(endpoint, &request)?;
        if !fetch_commit.response.found {
            return Ok(GapSyncHeightOutcome::NotFound {
                repair_summary: fetch_commit.repair_summary,
            });
        }
        let mut message = fetch_commit
            .response
            .message
            .ok_or_else(|| NodeError::Replication {
                reason: format!(
                    "gap sync height {} commit response missing payload (found=true)",
                    height
                ),
            })?;
        if message.world_id != world_id || message.record.world_id != world_id {
            return Err(NodeError::Replication {
                reason: format!(
                    "gap sync height {} world mismatch: expected={} actual_message={} actual_record={}",
                    height, world_id, message.world_id, message.record.world_id
                ),
            });
        }

        let blob_request =
            replication_runtime.build_fetch_blob_request(message.record.content_hash.as_str())?;
        let provider_lookup = endpoint
            .lookup_provider_ids_for_content_hash(world_id, message.record.content_hash.as_str())?;
        let blob_response = request_fetch_blob_with_route_fallback(
            endpoint,
            world_id,
            message.record.content_hash.as_str(),
            &blob_request,
            provider_lookup.as_deref(),
        )?;
        if !blob_response.found {
            return Err(NodeError::Replication {
                reason: format!(
                    "gap sync height {} blob not found for hash {}",
                    height, message.record.content_hash
                ),
            });
        }
        let blob = blob_response.blob.ok_or_else(|| NodeError::Replication {
            reason: format!(
                "gap sync height {} blob payload missing for hash {}",
                height, message.record.content_hash
            ),
        })?;
        message.payload = blob;
        let payload =
            parse_replication_commit_payload(message.payload.as_slice()).ok_or_else(|| {
                NodeError::Replication {
                    reason: format!("gap sync height {} payload decode failed", height),
                }
            })?;
        if payload.world_id != world_id {
            return Err(NodeError::Replication {
                reason: format!(
                    "gap sync height {} payload world mismatch expected={} actual={}",
                    height, world_id, payload.world_id
                ),
            });
        }
        if payload.node_id != message.node_id {
            return Err(NodeError::Replication {
                reason: format!(
                    "gap sync height {} payload node mismatch expected={} actual={}",
                    height, message.node_id, payload.node_id
                ),
            });
        }
        if payload.height != height {
            return Err(NodeError::Replication {
                reason: format!(
                    "gap sync height {} payload mismatch actual={}",
                    height, payload.height
                ),
            });
        }
        if payload.block_hash.trim().is_empty() {
            return Err(NodeError::Replication {
                reason: format!("gap sync height {} payload block_hash is empty", height),
            });
        }
        match replication_runtime.validate_remote_message_for_apply(node_id, world_id, &message) {
            Ok(true) => {}
            Ok(false) => {
                return Ok(GapSyncHeightOutcome::NotFound {
                    repair_summary: "commit validation rejected apply".to_string(),
                })
            }
            Err(err) => return Err(err),
        }
        validate_consensus_action_root(payload.action_root.as_str(), payload.actions.as_slice())
            .map_err(|err| NodeError::Replication {
                reason: format!(
                    "gap sync height {} action_root validation failed: {:?}",
                    height, err
                ),
            })?;
        self.validate_peer_commit_execution_binding(
            payload.height,
            payload.execution_block_hash.as_deref(),
            payload.execution_state_root.as_deref(),
        )
        .map_err(|err| NodeError::Replication {
            reason: format!(
                "gap sync height {} execution hash validation failed: {}",
                height, err
            ),
        })?;
        endpoint.remember_validated_fetch_commit_success(
            &request,
            &FetchCommitResponse {
                found: true,
                message: Some(message.clone()),
            },
        );
        Ok(GapSyncHeightOutcome::Synced { message, payload })
    }

    pub(super) fn persist_synced_replication_message(
        &self,
        endpoint: &ReplicationNetworkEndpoint,
        node_id: &str,
        world_id: &str,
        replication_runtime: &mut ReplicationRuntime,
        message: &replication::GossipReplicationMessage,
        height: u64,
    ) -> Result<(), NodeError> {
        replication_runtime.apply_remote_message(node_id, world_id, message)?;
        let persisted = replication_runtime.load_commit_message_by_height(world_id, height)?;
        if persisted
            .as_ref()
            .map(|entry| entry.record.content_hash.as_str())
            != Some(message.record.content_hash.as_str())
        {
            return Err(NodeError::Replication {
                reason: format!(
                    "synced replication height {} persisted commit hash mismatch expected={}",
                    height, message.record.content_hash
                ),
            });
        }
        endpoint.publish_local_content_provider(world_id, message.record.content_hash.as_str())?;
        Ok(())
    }

    pub(super) fn execute_synced_replication_commit(
        &mut self,
        world_id: &str,
        payload: &ReplicationCommitPayload,
        execution_hook: Option<&mut dyn NodeExecutionHook>,
    ) -> Result<(String, i64), NodeError> {
        if payload.execution_block_hash.is_some() != payload.execution_state_root.is_some() {
            return Err(NodeError::Replication {
                reason: format!(
                    "synced replication height {} execution binding malformed",
                    payload.height
                ),
            });
        }
        let decision = PosDecision {
            height: payload.height,
            slot: payload.slot,
            epoch: payload.epoch,
            status: PosConsensusStatus::Committed,
            block_hash: payload.block_hash.clone(),
            action_root: payload.action_root.clone(),
            committed_actions: payload.actions.clone(),
            approved_stake: self.required_stake,
            rejected_stake: 0,
            required_stake: self.required_stake,
            total_stake: self.total_stake,
        };
        let local_node_id = self.local_validator_id.clone();
        self.apply_committed_execution(
            local_node_id.as_str(),
            world_id,
            payload.committed_at_ms,
            &decision,
            execution_hook,
        )?;
        self.validate_peer_commit_execution_binding(
            payload.height,
            payload.execution_block_hash.as_deref(),
            payload.execution_state_root.as_deref(),
        )
        .map_err(|err| NodeError::Replication {
            reason: format!(
                "synced replication height {} execution hash validation failed: {}",
                payload.height, err
            ),
        })?;
        Ok((payload.block_hash.clone(), payload.committed_at_ms))
    }

    pub(super) fn apply_synced_replication_commit(
        &mut self,
        world_id: &str,
        payload: &ReplicationCommitPayload,
        execution_hook: Option<&mut dyn NodeExecutionHook>,
    ) -> Result<(), NodeError> {
        let (block_hash, committed_at_ms) =
            self.execute_synced_replication_commit(world_id, payload, execution_hook)?;
        self.record_synced_replication_height(payload.height, block_hash, committed_at_ms)
    }

    pub(super) fn record_synced_replication_height(
        &mut self,
        height: u64,
        block_hash: String,
        committed_at_ms: i64,
    ) -> Result<(), NodeError> {
        self.replication_persisted_height = self.replication_persisted_height.max(height);
        if height <= self.committed_height {
            return Ok(());
        }
        let next_synced_height =
            checked_replication_successor(height, "height", "recording synced replication height")?;
        self.committed_height = height;
        self.network_committed_height = self.network_committed_height.max(height);
        self.last_committed_at_ms = Some(committed_at_ms);
        self.next_height = next_synced_height;
        self.last_committed_block_hash = Some(block_hash);
        self.pending = None;
        Ok(())
    }

    pub(super) fn rollback_to_replicated_commit_boundary(
        &mut self,
        height: u64,
        block_hash: String,
        committed_at_ms: i64,
        execution_block_hash: Option<String>,
        execution_state_root: Option<String>,
    ) -> Result<(), NodeError> {
        if execution_block_hash.is_some() != execution_state_root.is_some() {
            return Err(NodeError::Replication {
                reason: format!(
                    "replication boundary rollback height {} execution binding malformed",
                    height
                ),
            });
        }
        let next_height = checked_replication_successor(
            height,
            "height",
            "rolling back to replicated commit boundary",
        )?;
        self.committed_height = height;
        self.network_committed_height = self.network_committed_height.min(height);
        self.replication_persisted_height = self.replication_persisted_height.min(height);
        self.last_replication_gap_sync_blocked_height = None;
        self.last_replication_gap_sync_blocked_reason = None;
        self.last_replication_gap_sync_repair_attempt_height = None;
        self.last_replication_gap_sync_repair_attempt_summary = None;
        self.last_committed_at_ms = Some(committed_at_ms);
        self.next_height = next_height;
        self.last_committed_block_hash = Some(block_hash);
        self.last_execution_height = height;
        self.last_execution_block_hash = execution_block_hash;
        self.last_execution_state_root = execution_state_root;
        self.execution_bindings.clear();
        self.remember_execution_binding_for_height(height);
        self.pending = None;
        Ok(())
    }

    pub(super) fn ingest_proposal_message(
        &mut self,
        world_id: &str,
        message: &GossipProposalMessage,
        current_slot: u64,
    ) -> Result<(), NodeError> {
        if message.version != 1 || message.world_id != world_id {
            return Ok(());
        }
        if message.node_id != message.proposer_id {
            return Ok(());
        }
        if self
            .validate_message_player_binding(
                message.proposer_id.as_str(),
                message.player_id.as_str(),
                "proposal",
            )
            .is_err()
        {
            return Ok(());
        }
        if self
            .validate_message_signer_binding(
                message.proposer_id.as_str(),
                message.public_key_hex.as_deref(),
                "proposal",
            )
            .is_err()
        {
            return Ok(());
        }
        match self.classify_inbound_slot_window(message.slot, current_slot) {
            InboundSlotWindow::Accept => {}
            InboundSlotWindow::Future => {
                self.inbound_rejected_proposal_future_slot =
                    self.inbound_rejected_proposal_future_slot.saturating_add(1);
                self.note_inbound_timing_reject(format!(
                    "reject proposal timing: future slot={} current_slot={} height={} proposer_id={}",
                    message.slot, current_slot, message.height, message.proposer_id
                ));
                return Ok(());
            }
            InboundSlotWindow::Stale => {
                self.inbound_rejected_proposal_stale_slot =
                    self.inbound_rejected_proposal_stale_slot.saturating_add(1);
                self.note_inbound_timing_reject(format!(
                    "reject proposal timing: stale slot={} current_slot={} lag={} height={} proposer_id={}",
                    message.slot,
                    current_slot,
                    self.max_past_slot_lag,
                    message.height,
                    message.proposer_id
                ));
                return Ok(());
            }
        }
        if message.height < self.next_height {
            return Ok(());
        }
        if let Some(current) = self.pending.as_ref() {
            if current.height > message.height {
                return Ok(());
            }
            if current.height == message.height && current.block_hash == message.block_hash {
                return Ok(());
            }
        }
        if validate_consensus_action_root(message.action_root.as_str(), message.actions.as_slice())
            .is_err()
        {
            return Ok(());
        }

        let mut proposal = PendingProposal {
            height: message.height,
            slot: message.slot,
            epoch: message.epoch,
            opened_at_ms: message.proposed_at_ms,
            proposer_id: message.proposer_id.clone(),
            block_hash: message.block_hash.clone(),
            action_root: message.action_root.clone(),
            committed_actions: message.actions.clone(),
            attestations: BTreeMap::new(),
            approved_stake: 0,
            rejected_stake: 0,
            status: PosConsensusStatus::Pending,
        };
        // A higher proposal implies its predecessor chain must already exist remotely,
        // so replication gap sync should backfill up to height-1 before this proposal commits.
        self.network_committed_height = self
            .network_committed_height
            .max(proposal.height.saturating_sub(1));
        self.insert_attestation(
            &mut proposal,
            &message.proposer_id,
            true,
            message.proposed_at_ms,
            message.epoch.saturating_sub(1),
            message.epoch,
            Some(format!("proposal gossiped from {}", message.node_id)),
        )?;
        let next_height = self.next_height.max(proposal.height);
        let mut next_slot = self.next_slot;
        if proposal.slot >= self.next_slot {
            next_slot = checked_consensus_successor(
                proposal.slot,
                "proposal.slot",
                "ingesting proposal message",
            )?;
        }
        self.next_height = next_height;
        self.next_slot = next_slot;
        self.pending = Some(proposal);
        Ok(())
    }

    pub(super) fn ingest_attestation_message(
        &mut self,
        world_id: &str,
        message: &GossipAttestationMessage,
        current_slot: u64,
    ) -> Result<(), NodeError> {
        if message.version != 1 || message.world_id != world_id {
            return Ok(());
        }
        if message.node_id != message.validator_id {
            return Ok(());
        }
        if self
            .validate_message_player_binding(
                message.validator_id.as_str(),
                message.player_id.as_str(),
                "attestation",
            )
            .is_err()
        {
            return Ok(());
        }
        if self
            .validate_message_signer_binding(
                message.validator_id.as_str(),
                message.public_key_hex.as_deref(),
                "attestation",
            )
            .is_err()
        {
            return Ok(());
        }
        match self.classify_inbound_slot_window(message.slot, current_slot) {
            InboundSlotWindow::Accept => {}
            InboundSlotWindow::Future => {
                self.inbound_rejected_attestation_future_slot = self
                    .inbound_rejected_attestation_future_slot
                    .saturating_add(1);
                self.note_inbound_timing_reject(format!(
                    "reject attestation timing: future slot={} current_slot={} height={} validator_id={}",
                    message.slot, current_slot, message.height, message.validator_id
                ));
                return Ok(());
            }
            InboundSlotWindow::Stale => {
                self.inbound_rejected_attestation_stale_slot = self
                    .inbound_rejected_attestation_stale_slot
                    .saturating_add(1);
                self.note_inbound_timing_reject(format!(
                    "reject attestation timing: stale slot={} current_slot={} lag={} height={} validator_id={}",
                    message.slot,
                    current_slot,
                    self.max_past_slot_lag,
                    message.height,
                    message.validator_id
                ));
                return Ok(());
            }
        }
        let Some(mut proposal) = self.pending.clone() else {
            return Ok(());
        };
        if proposal.height != message.height || proposal.block_hash != message.block_hash {
            return Ok(());
        }
        if message.slot != proposal.slot || message.epoch != proposal.epoch {
            self.inbound_rejected_attestation_epoch_mismatch = self
                .inbound_rejected_attestation_epoch_mismatch
                .saturating_add(1);
            self.note_inbound_timing_reject(format!(
                "reject attestation timing: slot/epoch mismatch height={} validator_id={} message_slot={} message_epoch={} proposal_slot={} proposal_epoch={}",
                message.height,
                message.validator_id,
                message.slot,
                message.epoch,
                proposal.slot,
                proposal.epoch
            ));
            return Ok(());
        }
        let expected_target_epoch = self.slot_epoch(proposal.slot);
        if message.target_epoch != expected_target_epoch {
            self.inbound_rejected_attestation_epoch_mismatch = self
                .inbound_rejected_attestation_epoch_mismatch
                .saturating_add(1);
            self.note_inbound_timing_reject(format!(
                "reject attestation timing: target_epoch mismatch height={} validator_id={} expected_target_epoch={} actual_target_epoch={} slot={}",
                message.height,
                message.validator_id,
                expected_target_epoch,
                message.target_epoch,
                proposal.slot
            ));
            return Ok(());
        }

        self.insert_attestation(
            &mut proposal,
            &message.validator_id,
            message.approve,
            message.voted_at_ms,
            message.source_epoch,
            message.target_epoch,
            message.reason.clone(),
        )?;
        self.pending = Some(proposal);
        Ok(())
    }

    pub(super) fn ingest_consensus_network_messages(
        &mut self,
        endpoint: &ConsensusNetworkEndpoint,
        world_id: &str,
        current_slot: u64,
    ) -> Result<(), NodeError> {
        let messages = endpoint.drain_messages()?;
        for message in messages {
            match message {
                GossipMessage::Hello(_) => {}
                GossipMessage::Commit(commit) => {
                    if commit.version != 1 || commit.world_id != world_id {
                        continue;
                    }
                    if verify_commit_message_signature(&commit, self.enforce_consensus_signature)
                        .is_err()
                    {
                        continue;
                    }
                    if self
                        .validate_peer_commit_execution_binding(
                            commit.height,
                            commit.execution_block_hash.as_deref(),
                            commit.execution_state_root.as_deref(),
                        )
                        .is_err()
                    {
                        continue;
                    }
                    if validate_consensus_action_root(
                        commit.action_root.as_str(),
                        commit.actions.as_slice(),
                    )
                    .is_err()
                    {
                        continue;
                    }
                    if self
                        .validate_message_player_binding(
                            commit.node_id.as_str(),
                            commit.player_id.as_str(),
                            "commit",
                        )
                        .is_err()
                    {
                        continue;
                    }
                    if self
                        .validate_message_signer_binding(
                            commit.node_id.as_str(),
                            commit.public_key_hex.as_deref(),
                            "commit",
                        )
                        .is_err()
                    {
                        continue;
                    }
                    self.observe_peer_commit_message(&commit);
                }
                GossipMessage::Proposal(proposal) => {
                    if proposal.version != 1 || proposal.world_id != world_id {
                        continue;
                    }
                    if verify_proposal_message_signature(
                        &proposal,
                        self.enforce_consensus_signature,
                    )
                    .is_err()
                    {
                        continue;
                    }
                    self.ingest_proposal_message(world_id, &proposal, current_slot)?;
                }
                GossipMessage::Attestation(attestation) => {
                    if attestation.version != 1 || attestation.world_id != world_id {
                        continue;
                    }
                    if verify_attestation_message_signature(
                        &attestation,
                        self.enforce_consensus_signature,
                    )
                    .is_err()
                    {
                        continue;
                    }
                    self.ingest_attestation_message(world_id, &attestation, current_slot)?;
                }
                GossipMessage::Replication(_) => {}
            }
        }
        Ok(())
    }

    pub(super) fn ingest_peer_messages(
        &mut self,
        endpoint: &GossipEndpoint,
        node_id: &str,
        world_id: &str,
        mut replication: Option<&mut ReplicationRuntime>,
        current_slot: u64,
    ) -> Result<(), NodeError> {
        let messages = endpoint.drain_messages()?;
        for received in messages {
            let from = received.from;
            match received.message {
                GossipMessage::Hello(hello) => {
                    if hello.version != 1 || hello.world_id != world_id || hello.node_id == node_id
                    {
                        continue;
                    }
                    endpoint.remember_peer(from)?;
                }
                GossipMessage::Commit(commit) => {
                    if commit.version != 1 || commit.world_id != world_id {
                        continue;
                    }
                    if verify_commit_message_signature(&commit, self.enforce_consensus_signature)
                        .is_err()
                    {
                        continue;
                    }
                    if self
                        .validate_peer_commit_execution_binding(
                            commit.height,
                            commit.execution_block_hash.as_deref(),
                            commit.execution_state_root.as_deref(),
                        )
                        .is_err()
                    {
                        continue;
                    }
                    if validate_consensus_action_root(
                        commit.action_root.as_str(),
                        commit.actions.as_slice(),
                    )
                    .is_err()
                    {
                        continue;
                    }
                    if self
                        .validate_message_player_binding(
                            commit.node_id.as_str(),
                            commit.player_id.as_str(),
                            "commit",
                        )
                        .is_err()
                    {
                        continue;
                    }
                    if self
                        .validate_message_signer_binding(
                            commit.node_id.as_str(),
                            commit.public_key_hex.as_deref(),
                            "commit",
                        )
                        .is_err()
                    {
                        continue;
                    }
                    endpoint.remember_peer(from)?;
                    self.observe_peer_commit_message(&commit);
                }
                GossipMessage::Proposal(proposal) => {
                    if proposal.version != 1 || proposal.world_id != world_id {
                        continue;
                    }
                    if verify_proposal_message_signature(
                        &proposal,
                        self.enforce_consensus_signature,
                    )
                    .is_err()
                    {
                        continue;
                    }
                    self.ingest_proposal_message(world_id, &proposal, current_slot)?;
                    endpoint.remember_peer(from)?;
                }
                GossipMessage::Attestation(attestation) => {
                    if attestation.version != 1 || attestation.world_id != world_id {
                        continue;
                    }
                    if verify_attestation_message_signature(
                        &attestation,
                        self.enforce_consensus_signature,
                    )
                    .is_err()
                    {
                        continue;
                    }
                    self.ingest_attestation_message(world_id, &attestation, current_slot)?;
                    endpoint.remember_peer(from)?;
                }
                GossipMessage::Replication(replication_msg) => {
                    if replication_msg.version != 1
                        || replication_msg.world_id != world_id
                        || replication_msg.record.world_id != world_id
                    {
                        continue;
                    }
                    if let Some(replication_runtime) = replication.as_deref_mut() {
                        if replication_runtime
                            .apply_remote_message(node_id, world_id, &replication_msg)
                            .is_ok()
                        {
                            endpoint.remember_peer(from)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub(super) fn compute_block_hash(
        &self,
        world_id: &str,
        height: u64,
        slot: u64,
        epoch: u64,
        proposer_id: &str,
        parent_block_hash: &str,
        action_root: &str,
    ) -> Result<String, NodeError> {
        let payload = (
            1_u8,
            world_id,
            height,
            slot,
            epoch,
            proposer_id,
            parent_block_hash,
            action_root,
        );
        let bytes = serde_cbor::to_vec(&payload).map_err(|err| NodeError::Consensus {
            reason: format!("encode block hash payload failed: {err}"),
        })?;
        Ok(blake3_hex(bytes.as_slice()))
    }
}

fn peer_commit_heads_conflict(left: &PeerCommittedHead, right: &PeerCommittedHead) -> bool {
    left.block_hash != right.block_hash
        || left.execution_block_hash != right.execution_block_hash
        || left.execution_state_root != right.execution_state_root
        || (!left.action_root.is_empty()
            && !right.action_root.is_empty()
            && left.action_root != right.action_root)
}
