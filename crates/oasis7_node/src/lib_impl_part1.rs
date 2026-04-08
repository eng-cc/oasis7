#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InboundSlotWindow {
    Accept,
    Future,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ObservedPosTick {
    slot: u64,
    tick_phase: u64,
}

impl PosNodeEngine {
    fn new(config: &NodeConfig) -> Result<Self, NodeError> {
        let (validators, validator_players, validator_signers, total_stake, required_stake) =
            validated_pos_state(&config.pos_config)?;
        let (consensus_signer, consensus_signer_public_key, enforce_consensus_signature) =
            if let Some(replication) = &config.replication {
                let signer_keypair = replication.consensus_signer()?;
                let signer_public_key = signer_keypair
                    .as_ref()
                    .map(|(_, public_key_hex)| public_key_hex.clone());
                let signer = signer_keypair
                    .map(|(signing_key, public_key_hex)| {
                        NodeConsensusMessageSigner::new(signing_key, public_key_hex)
                    })
                    .transpose()
                    .map_err(node_consensus_error)?;
                (
                    signer,
                    signer_public_key,
                    replication.enforce_consensus_signature(),
                )
            } else {
                (None, None, false)
            };
        if enforce_consensus_signature && validator_signers.len() != validators.len() {
            let missing_validator_signers = validators
                .keys()
                .filter(|validator_id| !validator_signers.contains_key(*validator_id))
                .cloned()
                .collect::<Vec<_>>();
            return Err(NodeError::InvalidConfig {
                reason: format!(
                    "consensus signature enforcement requires signer bindings for all validators; missing={}",
                    missing_validator_signers.join(",")
                ),
            });
        }
        if enforce_consensus_signature {
            if let Some(expected_public_key) = validator_signers.get(config.node_id.as_str()) {
                let Some(actual_public_key) = consensus_signer_public_key.as_deref() else {
                    return Err(NodeError::InvalidConfig {
                        reason: format!(
                            "consensus signer binding missing local signer keypair for validator {}",
                            config.node_id
                        ),
                    });
                };
                if actual_public_key != expected_public_key {
                    return Err(NodeError::InvalidConfig {
                        reason: format!(
                            "consensus signer binding mismatch for local validator {}: expected={} actual={}",
                            config.node_id, expected_public_key, actual_public_key
                        ),
                    });
                }
            }
        }
        if let Some(bound_player_id) = validator_players.get(config.node_id.as_str()) {
            if bound_player_id != &config.player_id {
                return Err(NodeError::InvalidConfig {
                    reason: format!(
                        "node_id {} is bound to validator player {}, but config player_id is {}",
                        config.node_id, bound_player_id, config.player_id
                    ),
                });
            }
        }
        Ok(Self {
            validators,
            validator_players,
            validator_signers,
            total_stake,
            required_stake,
            epoch_length_slots: config.pos_config.epoch_length_slots,
            slot_duration_ms: config.pos_config.slot_duration_ms,
            ticks_per_slot: config.pos_config.ticks_per_slot,
            proposal_tick_phase: config.pos_config.proposal_tick_phase,
            adaptive_tick_scheduler_enabled: config.pos_config.adaptive_tick_scheduler_enabled,
            slot_clock_genesis_unix_ms: config.pos_config.slot_clock_genesis_unix_ms,
            max_past_slot_lag: config.pos_config.max_past_slot_lag,
            last_observed_tick: 0,
            missed_tick_count: 0,
            last_observed_slot: 0,
            missed_slot_count: 0,
            local_validator_id: config.node_id.clone(),
            node_player_id: config.player_id.clone(),
            gossip_reverse_path_seeding_enabled: matches!(config.role, NodeRole::Observer),
            require_execution_on_commit: config.require_execution_on_commit,
            next_height: 1,
            next_slot: 0,
            committed_height: 0,
            network_committed_height: 0,
            pending: None,
            auto_attest_all_validators: config.auto_attest_all_validators,
            last_broadcast_proposal_height: 0,
            last_broadcast_local_attestation_height: 0,
            last_broadcast_committed_height: 0,
            replicate_local_commits: matches!(config.role, NodeRole::Sequencer)
                && config.replication.is_some(),
            require_peer_execution_hashes: config.require_peer_execution_hashes,
            consensus_signer,
            enforce_consensus_signature,
            peer_heads: BTreeMap::new(),
            last_committed_at_ms: None,
            last_committed_block_hash: None,
            inbound_rejected_proposal_future_slot: 0,
            inbound_rejected_proposal_stale_slot: 0,
            inbound_rejected_attestation_future_slot: 0,
            inbound_rejected_attestation_stale_slot: 0,
            inbound_rejected_attestation_epoch_mismatch: 0,
            last_inbound_timing_reject_reason: None,
            last_execution_height: 0,
            last_execution_block_hash: None,
            last_execution_state_root: None,
            execution_bindings: BTreeMap::new(),
            pending_consensus_actions: BTreeMap::new(),
            max_pending_consensus_actions: config.max_engine_pending_consensus_actions,
        })
    }

    fn tick(
        &mut self,
        node_id: &str,
        world_id: &str,
        now_ms: i64,
        gossip: Option<&mut GossipEndpoint>,
        mut replication: Option<&mut ReplicationRuntime>,
        replication_network: Option<&mut ReplicationNetworkEndpoint>,
        consensus_network: Option<&mut ConsensusNetworkEndpoint>,
        queued_actions: Vec<NodeConsensusAction>,
        execution_hook: Option<&mut dyn NodeExecutionHook>,
    ) -> Result<NodeEngineTickResult, NodeError> {
        merge_pending_consensus_actions(
            &mut self.pending_consensus_actions,
            queued_actions,
            self.max_pending_consensus_actions,
        )?;

        let observed_tick = self.observe_wall_clock_tick(now_ms)?;
        let current_slot = observed_tick.slot;

        if let Some(endpoint) = gossip.as_ref() {
            self.seed_reverse_gossip_path(endpoint, node_id, world_id, now_ms)?;
        }
        if let Some(endpoint) = gossip.as_ref() {
            self.ingest_peer_messages(
                endpoint,
                node_id,
                world_id,
                replication.as_deref_mut(),
                current_slot,
            )?;
        }
        if let Some(endpoint) = consensus_network.as_ref() {
            self.ingest_consensus_network_messages(endpoint, world_id, current_slot)?;
        }
        if let Some(endpoint) = replication_network.as_ref() {
            self.ingest_network_replications(
                endpoint,
                node_id,
                world_id,
                replication.as_deref_mut(),
            )?;
            self.sync_missing_replication_commits(
                endpoint,
                node_id,
                world_id,
                replication.as_deref_mut(),
            )?;
        }
        self.align_next_slot_to_wall_clock(current_slot)?;

        let mut decision = if self.pending.is_some() {
            self.advance_pending_attestations(now_ms)?
        } else if self.next_slot <= current_slot
            && observed_tick.tick_phase == self.proposal_tick_phase
        {
            self.propose_next_head(node_id, world_id, now_ms)?
        } else {
            self.idle_pending_decision()?
        };

        if matches!(decision.status, PosConsensusStatus::Pending) && self.pending.is_some() {
            decision = self.advance_pending_attestations(now_ms)?;
        }

        if let Some(endpoint) = consensus_network.as_ref() {
            self.broadcast_local_proposal_network(endpoint, node_id, world_id, now_ms)?;
            self.broadcast_local_attestation_network(endpoint, node_id, world_id, now_ms)?;
        } else if let Some(endpoint) = gossip.as_ref() {
            self.broadcast_local_proposal(endpoint, node_id, world_id, now_ms)?;
            self.broadcast_local_attestation(endpoint, node_id, world_id, now_ms)?;
        }

        let prev_committed_height = self.committed_height;
        self.apply_committed_execution(node_id, world_id, now_ms, &decision, execution_hook)?;
        self.apply_decision(&decision)?;
        if matches!(decision.status, PosConsensusStatus::Committed)
            && decision.height > prev_committed_height
        {
            self.last_committed_at_ms = Some(now_ms);
        }
        if let Some(endpoint) = consensus_network.as_ref() {
            self.broadcast_local_commit_network(endpoint, node_id, world_id, now_ms, &decision)?;
        } else if let Some(endpoint) = gossip.as_ref() {
            self.broadcast_local_commit(endpoint, node_id, world_id, now_ms, &decision)?;
        }
        self.broadcast_local_replication(
            gossip.as_deref(),
            replication_network.as_deref(),
            node_id,
            world_id,
            now_ms,
            &decision,
            replication.as_deref_mut(),
        )?;
        if let Some(endpoint) = gossip.as_ref() {
            self.ingest_peer_messages(
                endpoint,
                node_id,
                world_id,
                replication.as_deref_mut(),
                current_slot,
            )?;
        }
        if let Some(endpoint) = consensus_network.as_ref() {
            self.ingest_consensus_network_messages(endpoint, world_id, current_slot)?;
        }
        if let Some(endpoint) = replication_network.as_ref() {
            self.ingest_network_replications(
                endpoint,
                node_id,
                world_id,
                replication.as_deref_mut(),
            )?;
        }
        let committed_action_batch = if matches!(decision.status, PosConsensusStatus::Committed)
            && !decision.committed_actions.is_empty()
            && decision.height > prev_committed_height
        {
            Some(NodeCommittedActionBatch {
                height: decision.height,
                slot: decision.slot,
                epoch: decision.epoch,
                block_hash: decision.block_hash.clone(),
                action_root: decision.action_root.clone(),
                committed_at_unix_ms: now_ms,
                actions: decision.committed_actions.clone(),
            })
        } else {
            None
        };

        Ok(NodeEngineTickResult {
            consensus_snapshot: self.snapshot_from_decision(&decision),
            committed_action_batch,
        })
    }

    fn observe_wall_clock_tick(&mut self, now_ms: i64) -> Result<ObservedPosTick, NodeError> {
        if self.slot_clock_genesis_unix_ms.is_none() {
            let observed_tick_offset = ((self.last_observed_tick as u128)
                .saturating_mul(self.slot_duration_ms as u128))
                / self.ticks_per_slot as u128;
            let observed_offset_ms = observed_tick_offset.min(i64::MAX as u128) as i64;
            self.slot_clock_genesis_unix_ms = Some(now_ms.saturating_sub(observed_offset_ms));
        }
        let genesis = self.slot_clock_genesis_unix_ms.unwrap_or(now_ms);
        let elapsed_ms = if now_ms > genesis {
            (now_ms - genesis) as u64
        } else {
            0
        };
        let observed_tick = (((elapsed_ms as u128).saturating_mul(self.ticks_per_slot as u128))
            / self.slot_duration_ms as u128) as u64;
        if observed_tick > self.last_observed_tick {
            let delta_ticks = observed_tick - self.last_observed_tick;
            if delta_ticks > 1 {
                self.missed_tick_count = self
                    .missed_tick_count
                    .checked_add(delta_ticks - 1)
                    .ok_or_else(|| NodeError::Consensus {
                        reason: format!(
                            "missed_tick_count overflow while observing tick: current={} delta={}",
                            self.missed_tick_count,
                            delta_ticks - 1
                        ),
                    })?;
            }
            self.last_observed_tick = observed_tick;
        }
        let observed_slot = self.last_observed_tick / self.ticks_per_slot;
        self.last_observed_slot = self.last_observed_slot.max(observed_slot);
        let tick_phase = self.last_observed_tick % self.ticks_per_slot;
        Ok(ObservedPosTick {
            slot: self.last_observed_slot,
            tick_phase,
        })
    }

    fn align_next_slot_to_wall_clock(&mut self, current_slot: u64) -> Result<(), NodeError> {
        if self.next_slot >= current_slot {
            return Ok(());
        }
        let skipped_slots = current_slot - self.next_slot;
        self.missed_slot_count = self
            .missed_slot_count
            .checked_add(skipped_slots)
            .ok_or_else(|| NodeError::Consensus {
                reason: format!(
                    "missed_slot_count overflow while aligning next_slot: current={} delta={}",
                    self.missed_slot_count, skipped_slots
                ),
            })?;
        self.next_slot = current_slot;
        Ok(())
    }

    fn next_tick_wait_duration(&self, now_ms: i64, fallback: Duration) -> Duration {
        if !self.adaptive_tick_scheduler_enabled {
            return fallback;
        }
        let Some(genesis_unix_ms) = self.slot_clock_genesis_unix_ms else {
            return fallback;
        };
        let Some(wait_ms) = crate::runtime_util::millis_until_next_logical_tick(
            now_ms,
            genesis_unix_ms,
            self.slot_duration_ms,
            self.ticks_per_slot,
        ) else {
            return fallback;
        };
        Duration::from_millis(wait_ms.max(1))
    }

    fn classify_inbound_slot_window(
        &self,
        message_slot: u64,
        current_slot: u64,
    ) -> InboundSlotWindow {
        if message_slot > current_slot {
            return InboundSlotWindow::Future;
        }
        let latest_acceptable_slot = message_slot.saturating_add(self.max_past_slot_lag);
        if latest_acceptable_slot < current_slot {
            return InboundSlotWindow::Stale;
        }
        InboundSlotWindow::Accept
    }

    fn note_inbound_timing_reject(&mut self, reason: String) {
        self.last_inbound_timing_reject_reason = Some(reason);
    }

    fn idle_pending_decision(&self) -> Result<PosDecision, NodeError> {
        Ok(PosDecision {
            height: self.committed_height,
            slot: self.next_slot,
            epoch: self.slot_epoch(self.next_slot),
            status: PosConsensusStatus::Pending,
            block_hash: self
                .last_committed_block_hash
                .clone()
                .unwrap_or_else(|| "genesis".to_string()),
            action_root: compute_consensus_action_root(&[])?,
            committed_actions: Vec::new(),
            approved_stake: 0,
            rejected_stake: 0,
            required_stake: self.required_stake,
            total_stake: self.total_stake,
        })
    }

    fn propose_next_head(
        &mut self,
        node_id: &str,
        world_id: &str,
        now_ms: i64,
    ) -> Result<PosDecision, NodeError> {
        let slot = self.next_slot;
        let epoch = self.slot_epoch(slot);
        let committed_actions =
            drain_ordered_consensus_actions(&mut self.pending_consensus_actions);
        let action_root = compute_consensus_action_root(committed_actions.as_slice())?;
        let proposer_id = self
            .expected_proposer(slot)
            .ok_or_else(|| NodeError::Consensus {
                reason: "no proposer available".to_string(),
            })?;
        let parent_block_hash = self
            .last_committed_block_hash
            .as_deref()
            .unwrap_or("genesis");
        let block_hash = self.compute_block_hash(
            world_id,
            self.next_height,
            slot,
            epoch,
            proposer_id.as_str(),
            parent_block_hash,
            action_root.as_str(),
        )?;

        core_propose_next_head(
            &self.validators,
            self.total_stake,
            self.required_stake,
            self.epoch_length_slots,
            &mut self.next_height,
            &mut self.next_slot,
            &mut self.pending,
            proposer_id,
            block_hash,
            action_root,
            committed_actions,
            node_id,
            now_ms,
        )
        .map_err(node_pos_error)
    }

    fn advance_pending_attestations(&mut self, now_ms: i64) -> Result<PosDecision, NodeError> {
        core_advance_pending_attestations(
            &self.validators,
            self.total_stake,
            self.required_stake,
            self.local_validator_id.as_str(),
            self.auto_attest_all_validators,
            &mut self.pending,
            now_ms,
        )
        .map_err(node_pos_error)
    }

    fn insert_attestation(
        &self,
        proposal: &mut PendingProposal,
        validator_id: &str,
        approve: bool,
        voted_at_ms: i64,
        source_epoch: u64,
        target_epoch: u64,
        reason: Option<String>,
    ) -> Result<(), NodeError> {
        core_insert_attestation(
            &self.validators,
            self.total_stake,
            self.required_stake,
            proposal,
            validator_id,
            approve,
            voted_at_ms,
            source_epoch,
            target_epoch,
            reason,
        )
        .map_err(node_pos_error)
    }

    fn apply_decision(&mut self, decision: &PosDecision) -> Result<(), NodeError> {
        match decision.status {
            PosConsensusStatus::Pending => {}
            PosConsensusStatus::Committed => {
                let next_height = checked_consensus_successor(
                    decision.height,
                    "decision.height",
                    "applying committed decision",
                )?;
                self.committed_height = decision.height;
                self.network_committed_height = self.network_committed_height.max(decision.height);
                self.last_committed_block_hash = Some(decision.block_hash.clone());
                self.next_height = next_height;
                self.pending = None;
            }
            PosConsensusStatus::Rejected => {
                let next_height = checked_consensus_successor(
                    decision.height,
                    "decision.height",
                    "applying rejected decision",
                )?;
                merge_pending_consensus_actions(
                    &mut self.pending_consensus_actions,
                    decision.committed_actions.clone(),
                    self.max_pending_consensus_actions,
                )
                .map_err(|err| NodeError::Consensus {
                    reason: format!(
                        "requeue rejected consensus actions failed at height {}: {}",
                        decision.height, err
                    ),
                })?;
                self.next_height = next_height;
                self.pending = None;
            }
        }
        Ok(())
    }

    fn pending_consensus_action_capacity(&self) -> usize {
        let reserved_requeue_actions = self
            .pending
            .as_ref()
            .map(|proposal| proposal.committed_actions.len())
            .unwrap_or(0);
        let occupied_with_reserve = self
            .pending_consensus_actions
            .len()
            .saturating_add(reserved_requeue_actions);
        self.max_pending_consensus_actions
            .saturating_sub(occupied_with_reserve)
    }

    fn apply_committed_execution(
        &mut self,
        node_id: &str,
        world_id: &str,
        now_ms: i64,
        decision: &PosDecision,
        execution_hook: Option<&mut dyn NodeExecutionHook>,
    ) -> Result<(), NodeError> {
        if !matches!(decision.status, PosConsensusStatus::Committed) {
            return Ok(());
        }
        if decision.height <= self.last_execution_height {
            return Ok(());
        }
        let Some(execution_hook) = execution_hook else {
            if self.require_execution_on_commit {
                return Err(NodeError::Execution {
                    reason: format!(
                        "execution hook is required before committing height {}",
                        decision.height
                    ),
                });
            }
            return Ok(());
        };

        let result = execution_hook
            .on_commit(NodeExecutionCommitContext {
                world_id: world_id.to_string(),
                node_id: node_id.to_string(),
                height: decision.height,
                slot: decision.slot,
                epoch: decision.epoch,
                node_block_hash: decision.block_hash.clone(),
                action_root: decision.action_root.clone(),
                committed_actions: decision.committed_actions.clone(),
                committed_at_unix_ms: now_ms,
            })
            .map_err(|reason| NodeError::Execution { reason })?;

        if result.execution_height != decision.height {
            return Err(NodeError::Execution {
                reason: format!(
                    "execution hook returned mismatched height: expected {}, got {}",
                    decision.height, result.execution_height
                ),
            });
        }
        if result.execution_block_hash.trim().is_empty() {
            return Err(NodeError::Execution {
                reason: "execution hook returned empty execution_block_hash".to_string(),
            });
        }
        if result.execution_state_root.trim().is_empty() {
            return Err(NodeError::Execution {
                reason: "execution hook returned empty execution_state_root".to_string(),
            });
        }

        self.last_execution_height = result.execution_height;
        self.last_execution_block_hash = Some(result.execution_block_hash);
        self.last_execution_state_root = Some(result.execution_state_root);
        self.remember_execution_binding_for_height(decision.height);
        Ok(())
    }

    fn snapshot_from_decision(&self, decision: &PosDecision) -> NodeConsensusSnapshot {
        let peer_heads = self
            .peer_heads
            .iter()
            .map(|(node_id, head)| NodePeerCommittedHead {
                node_id: node_id.clone(),
                height: head.height,
                block_hash: head.block_hash.clone(),
                committed_at_ms: head.committed_at_ms,
                execution_block_hash: head.execution_block_hash.clone(),
                execution_state_root: head.execution_state_root.clone(),
            })
            .collect::<Vec<_>>();
        NodeConsensusSnapshot {
            mode: NodeConsensusMode::Pos,
            slot: self.next_slot,
            epoch: self.slot_epoch(self.next_slot),
            ticks_per_slot: self.ticks_per_slot,
            tick_phase: self.last_observed_tick % self.ticks_per_slot,
            proposal_tick_phase: self.proposal_tick_phase,
            last_observed_slot: self.last_observed_slot,
            missed_slot_count: self.missed_slot_count,
            last_observed_tick: self.last_observed_tick,
            missed_tick_count: self.missed_tick_count,
            adaptive_tick_scheduler_enabled: self.adaptive_tick_scheduler_enabled,
            latest_height: decision.height,
            committed_height: self.committed_height,
            last_committed_at_ms: self.last_committed_at_ms,
            network_committed_height: self.network_committed_height.max(self.committed_height),
            known_peer_heads: self.peer_heads.len(),
            peer_heads,
            inbound_rejected_proposal_future_slot: self.inbound_rejected_proposal_future_slot,
            inbound_rejected_proposal_stale_slot: self.inbound_rejected_proposal_stale_slot,
            inbound_rejected_attestation_future_slot: self.inbound_rejected_attestation_future_slot,
            inbound_rejected_attestation_stale_slot: self.inbound_rejected_attestation_stale_slot,
            inbound_rejected_attestation_epoch_mismatch: self
                .inbound_rejected_attestation_epoch_mismatch,
            last_inbound_timing_reject_reason: self.last_inbound_timing_reject_reason.clone(),
            last_status: Some(decision.status),
            last_block_hash: Some(decision.block_hash.clone()),
            last_execution_height: self.last_execution_height,
            last_execution_block_hash: self.last_execution_block_hash.clone(),
            last_execution_state_root: self.last_execution_state_root.clone(),
        }
    }

    fn seed_reverse_gossip_path(
        &self,
        endpoint: &GossipEndpoint,
        node_id: &str,
        world_id: &str,
        now_ms: i64,
    ) -> Result<(), NodeError> {
        if !self.gossip_reverse_path_seeding_enabled {
            return Ok(());
        }
        endpoint.broadcast_hello(&crate::gossip_udp::GossipHelloMessage {
            version: 1,
            world_id: world_id.to_string(),
            node_id: node_id.to_string(),
            sent_at_ms: now_ms,
        })
    }

    fn commit_execution_binding_for_height(
        &self,
        committed_height: u64,
    ) -> Result<(Option<&str>, Option<&str>), NodeError> {
        let (execution_block_hash, execution_state_root) = self
            .execution_binding_for_height(committed_height)
            .map(|(block_hash, state_root)| (Some(block_hash), Some(state_root)))
            .unwrap_or((None, None));
        if execution_block_hash.is_some() != execution_state_root.is_some() {
            return Err(NodeError::Consensus {
                reason:
                    "execution commit binding requires both execution_block_hash and execution_state_root"
                        .to_string(),
            });
        }
        Ok((execution_block_hash, execution_state_root))
    }

    fn execution_binding_for_height(&self, height: u64) -> Option<(&str, &str)> {
        if let Some((block_hash, state_root)) = self.execution_bindings.get(&height) {
            return Some((block_hash.as_str(), state_root.as_str()));
        }
        if self.last_execution_height != height {
            return None;
        }
        match (
            self.last_execution_block_hash.as_deref(),
            self.last_execution_state_root.as_deref(),
        ) {
            (Some(block_hash), Some(state_root)) => Some((block_hash, state_root)),
            _ => None,
        }
    }

    fn remember_execution_binding_for_height(&mut self, height: u64) {
        let (Some(block_hash), Some(state_root)) = (
            self.last_execution_block_hash.as_ref(),
            self.last_execution_state_root.as_ref(),
        ) else {
            return;
        };
        self.execution_bindings
            .insert(height, (block_hash.clone(), state_root.clone()));
        while self.execution_bindings.len() > EXECUTION_BINDING_HISTORY_LIMIT {
            let Some(first_height) = self.execution_bindings.keys().next().copied() else {
                break;
            };
            self.execution_bindings.remove(&first_height);
        }
    }

    fn validate_peer_commit_execution_binding(
        &self,
        height: u64,
        execution_block_hash: Option<&str>,
        execution_state_root: Option<&str>,
    ) -> Result<(), NodeError> {
        if execution_block_hash.is_some() != execution_state_root.is_some() {
            return Err(NodeError::Consensus {
                reason: format!(
                    "peer commit execution binding malformed at height {}: block/state pair mismatch",
                    height
                ),
            });
        }
        if self.require_peer_execution_hashes
            && (execution_block_hash.is_none() || execution_state_root.is_none())
        {
            return Err(NodeError::Consensus {
                reason: format!(
                    "peer commit missing required execution hashes at height {}",
                    height
                ),
            });
        }
        let Some((local_block_hash, local_state_root)) = self.execution_binding_for_height(height)
        else {
            return Ok(());
        };
        let (Some(peer_block_hash), Some(peer_state_root)) =
            (execution_block_hash, execution_state_root)
        else {
            return Err(NodeError::Consensus {
                reason: format!(
                    "peer commit missing execution hashes at locally executed height {}",
                    height
                ),
            });
        };
        if local_block_hash != peer_block_hash || local_state_root != peer_state_root {
            return Err(NodeError::Consensus {
                reason: format!(
                    "peer commit execution mismatch at height {}: local_block={} peer_block={} local_state={} peer_state={}",
                    height, local_block_hash, peer_block_hash, local_state_root, peer_state_root
                ),
            });
        }
        Ok(())
    }

    fn broadcast_local_replication(
        &mut self,
        gossip_endpoint: Option<&GossipEndpoint>,
        network_endpoint: Option<&ReplicationNetworkEndpoint>,
        node_id: &str,
        world_id: &str,
        now_ms: i64,
        decision: &PosDecision,
        replication: Option<&mut ReplicationRuntime>,
    ) -> Result<(), NodeError> {
        if !self.replicate_local_commits {
            return Ok(());
        }
        let Some(replication) = replication else {
            return Ok(());
        };
        self.enforce_storage_challenge_gate(
            replication,
            network_endpoint,
            node_id,
            world_id,
            now_ms,
        )?;
        let (execution_block_hash, execution_state_root) =
            self.commit_execution_binding_for_height(decision.height)?;
        if let Some(message) = replication.build_local_commit_message(
            node_id,
            world_id,
            now_ms,
            decision,
            execution_block_hash,
            execution_state_root,
        )? {
            if let Some(endpoint) = network_endpoint {
                endpoint.publish_local_content_provider(
                    world_id,
                    message.record.content_hash.as_str(),
                )?;
                endpoint.publish_replication(&message)?;
            } else if let Some(endpoint) = gossip_endpoint {
                endpoint.broadcast_replication(&message)?;
            }
        }
        Ok(())
    }

    fn enforce_storage_challenge_gate(
        &self,
        replication: &ReplicationRuntime,
        network_endpoint: Option<&ReplicationNetworkEndpoint>,
        node_id: &str,
        world_id: &str,
        now_ms: i64,
    ) -> Result<(), NodeError> {
        let report = replication.probe_storage_challenges(world_id, node_id, now_ms)?;
        if report.failed_checks > 0 {
            return Err(NodeError::Consensus {
                reason: format!(
                    "storage challenge gate failed: total_checks={} failed_checks={} reasons={:?}",
                    report.total_checks, report.failed_checks, report.failure_reasons
                ),
            });
        }

        let Some(endpoint) = network_endpoint else {
            return Ok(());
        };
        let content_hashes = replication
            .recent_replicated_content_hashes(world_id, STORAGE_GATE_NETWORK_SAMPLES_PER_CHECK)?;
        if content_hashes.is_empty() {
            return Ok(());
        }

        let mut successful_matches = 0usize;
        let mut attempted_samples = 0usize;
        let mut failure_reasons = Vec::new();
        for content_hash in content_hashes {
            attempted_samples = attempted_samples.saturating_add(1);

            let local_blob = match replication.load_blob_by_hash(content_hash.as_str())? {
                Some(blob) => blob,
                None => {
                    failure_reasons.push(format!(
                        "storage challenge gate local blob missing for hash {}",
                        content_hash
                    ));
                    continue;
                }
            };
            let fetch_blob_request = replication.build_fetch_blob_request(content_hash.as_str())?;
            let provider_lookup =
                match endpoint.lookup_provider_ids_for_content_hash(world_id, content_hash.as_str())
                {
                    Ok(provider_ids) => provider_ids,
                    Err(err) => {
                        failure_reasons.push(format!(
                            "storage challenge gate provider lookup failed for hash {}: {:?}",
                            content_hash, err
                        ));
                        continue;
                    }
                };
            let response = match if let Some(provider_ids) = provider_lookup.as_ref() {
                if provider_ids.is_empty() {
                    endpoint.request_json::<FetchBlobRequest, FetchBlobResponse>(
                        REPLICATION_FETCH_BLOB_PROTOCOL,
                        &fetch_blob_request,
                    )
                } else {
                    match endpoint.request_json_with_providers::<
                        FetchBlobRequest,
                        FetchBlobResponse,
                    >(
                        REPLICATION_FETCH_BLOB_PROTOCOL,
                        &fetch_blob_request,
                        provider_ids.as_slice(),
                    ) {
                        Ok(response) => Ok(response),
                        Err(err)
                            if should_fallback_provider_aware_replication_request(&err) =>
                        {
                            endpoint.request_json::<FetchBlobRequest, FetchBlobResponse>(
                                REPLICATION_FETCH_BLOB_PROTOCOL,
                                &fetch_blob_request,
                            )
                        }
                        Err(err) => Err(err),
                    }
                }
            } else {
                endpoint.request_json::<FetchBlobRequest, FetchBlobResponse>(
                    REPLICATION_FETCH_BLOB_PROTOCOL,
                    &fetch_blob_request,
                )
            } {
                Ok(response) => response,
                Err(err) => {
                    failure_reasons.push(format!(
                        "storage challenge gate network request failed for hash {}: {:?}",
                        content_hash, err
                    ));
                    continue;
                }
            };
            if !response.found {
                failure_reasons.push(format!(
                    "storage challenge gate network blob not found for hash {}",
                    content_hash
                ));
                continue;
            }
            let Some(network_blob) = response.blob else {
                failure_reasons.push(format!(
                    "storage challenge gate network blob payload missing for hash {}",
                    content_hash
                ));
                continue;
            };
            if blake3_hex(network_blob.as_slice()) != content_hash {
                failure_reasons.push(format!(
                    "storage challenge gate network blob hash mismatch for hash {}",
                    content_hash
                ));
                continue;
            }
            if network_blob != local_blob {
                failure_reasons.push(format!(
                    "storage challenge gate network blob bytes mismatch for hash {}",
                    content_hash
                ));
                continue;
            }
            successful_matches = successful_matches.saturating_add(1);
        }

        let required_matches = required_network_blob_matches(attempted_samples);
        if successful_matches < required_matches {
            return Err(NodeError::Consensus {
                reason: format!(
                    "storage challenge gate network threshold unmet: samples={} required_matches={} successful_matches={} reasons={:?}",
                    attempted_samples,
                    required_matches,
                    successful_matches,
                    failure_reasons
                ),
            });
        }
        Ok(())
    }

    fn ingest_network_replications(
        &mut self,
        endpoint: &ReplicationNetworkEndpoint,
        node_id: &str,
        world_id: &str,
        mut replication: Option<&mut ReplicationRuntime>,
    ) -> Result<(), NodeError> {
        let Some(replication_runtime) = replication.as_deref_mut() else {
            return Ok(());
        };
        let messages = endpoint.drain_replications()?;
        let mut rejected = Vec::new();
        for message in messages {
            let committed_successor = checked_replication_successor(
                self.committed_height,
                "committed_height",
                "ingesting replication message",
            )?;
            let payload_view = parse_replication_commit_payload_view(message.payload.as_slice());
            match replication_runtime
                .validate_remote_message_for_observe(node_id, world_id, &message)
            {
                Ok(true) => {}
                Ok(false) => continue,
                Err(err) => {
                    rejected.push(format!(
                        "node_id={} world_id={} err={}",
                        message.node_id, message.world_id, err
                    ));
                    continue;
                }
            }
            if let Some(payload) = payload_view.as_ref() {
                if self
                    .validate_peer_commit_execution_binding(
                        payload.height,
                        payload.execution_block_hash.as_deref(),
                        payload.execution_state_root.as_deref(),
                    )
                    .is_err()
                {
                    rejected.push(format!(
                        "node_id={} world_id={} err=peer execution hash validation failed for height {}",
                        message.node_id, message.world_id, payload.height
                    ));
                    continue;
                }
                self.observe_network_replication_commit(message.node_id.as_str(), payload);
            }
            let should_apply = payload_view
                .as_ref()
                .map(|payload| payload.height <= committed_successor)
                .unwrap_or(true);
            if !should_apply {
                continue;
            }
            match replication_runtime.apply_remote_message(node_id, world_id, &message) {
                Ok(()) => {
                    endpoint.publish_local_content_provider(
                        world_id,
                        message.record.content_hash.as_str(),
                    )?;
                    if let Some(payload) = payload_view {
                        if payload.height == committed_successor
                            && replication_runtime
                                .load_commit_message_by_height(world_id, payload.height)?
                                .is_some()
                        {
                            self.record_synced_replication_height(
                                payload.height,
                                payload.block_hash,
                                payload.committed_at_ms,
                            )?;
                        }
                    }
                }
                Err(err) => rejected.push(format!(
                    "node_id={} world_id={} err={}",
                    message.node_id, message.world_id, err
                )),
            }
        }
        if !rejected.is_empty() {
            let rejected_count = rejected.len();
            let sample = rejected.into_iter().take(3).collect::<Vec<_>>();
            return Err(NodeError::Replication {
                reason: format!(
                    "replication ingest rejected {rejected_count} message(s); sample={sample:?}"
                ),
            });
        }
        Ok(())
    }

    fn observe_network_replication_commit(
        &mut self,
        peer_node_id: &str,
        payload: &ReplicationCommitPayloadView,
    ) {
        if payload.height == 0 {
            return;
        }
        self.network_committed_height = self.network_committed_height.max(payload.height);
        self.peer_heads.insert(
            peer_node_id.to_string(),
            PeerCommittedHead {
                height: payload.height,
                block_hash: payload.block_hash.clone(),
                committed_at_ms: payload.committed_at_ms,
                execution_block_hash: payload.execution_block_hash.clone(),
                execution_state_root: payload.execution_state_root.clone(),
            },
        );
    }

    fn sync_missing_replication_commits(
        &mut self,
        endpoint: &ReplicationNetworkEndpoint,
        node_id: &str,
        world_id: &str,
        mut replication: Option<&mut ReplicationRuntime>,
    ) -> Result<(), NodeError> {
        let Some(replication_runtime) = replication.as_deref_mut() else {
            return Ok(());
        };
        if self.network_committed_height <= self.committed_height {
            return Ok(());
        }

        let mut next_height = checked_replication_successor(
            self.committed_height,
            "committed_height",
            "starting replication gap sync",
        )?;
        while next_height <= self.network_committed_height {
            let mut synced_commit: Option<(String, i64)> = None;
            let mut not_found = false;
            let mut last_error = None;
            for attempt in 1..=REPLICATION_GAP_SYNC_MAX_RETRIES_PER_HEIGHT {
                match self.sync_replication_height_once(
                    endpoint,
                    node_id,
                    world_id,
                    replication_runtime,
                    next_height,
                ) {
                    Ok(GapSyncHeightOutcome::Synced {
                        block_hash,
                        committed_at_ms,
                    }) => {
                        synced_commit = Some((block_hash, committed_at_ms));
                        break;
                    }
                    Ok(GapSyncHeightOutcome::NotFound) => {
                        not_found = true;
                        break;
                    }
                    Err(err) => {
                        last_error = Some(format!(
                            "attempt {attempt}/{} failed: {}",
                            REPLICATION_GAP_SYNC_MAX_RETRIES_PER_HEIGHT, err
                        ));
                    }
                }
            }
            if let Some((block_hash, committed_at_ms)) = synced_commit {
                self.record_synced_replication_height(next_height, block_hash, committed_at_ms)?;
                next_height = checked_replication_successor(
                    next_height,
                    "next_height",
                    "advancing replication gap sync cursor",
                )?;
                continue;
            }
            if not_found {
                break;
            }
            return Err(NodeError::Replication {
                reason: format!(
                    "gap sync height {} failed after {} attempts: {}",
                    next_height,
                    REPLICATION_GAP_SYNC_MAX_RETRIES_PER_HEIGHT,
                    last_error.unwrap_or_else(|| "unknown error".to_string())
                ),
            });
        }
        Ok(())
    }
}

fn should_fallback_provider_aware_replication_request(err: &NodeError) -> bool {
    let NodeError::Replication { reason } = err else {
        return false;
    };
    reason.contains("NetworkProtocolUnavailable")
        || reason.contains("libp2p-replication no connected providers for protocol")
        || reason.contains("libp2p-replication no connected peers for protocol")
        || (reason.contains("NetworkRequestFailed")
            && reason.contains("NetworkProtocolUnavailable"))
}
