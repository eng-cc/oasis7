use super::*;

const GOSSIP_REVERSE_PATH_SEED_INTERVAL_MS: i64 = 5_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InboundSlotWindow {
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
    pub(super) fn new(config: &NodeConfig) -> Result<Self, NodeError> {
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
                (None::<NodeConsensusMessageSigner>, None::<String>, false)
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
            last_gossip_reverse_path_seed_at_ms: None,
            allow_local_proposals: config.allow_local_proposals,
            require_execution_on_commit: config.require_execution_on_commit,
            next_height: 1,
            next_slot: 0,
            committed_height: 0,
            network_committed_height: 0,
            replication_persisted_height: 0,
            last_replication_successor_probe_height: None,
            last_replication_successor_probe_at_ms: None,
            last_replication_successor_probe_hold: None,
            storage_challenge_fallback_height: 1,
            recent_storage_challenge_successes: BTreeMap::new(),
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
            recent_finality_latency_ms: VecDeque::new(),
            execution_bindings: BTreeMap::new(),
            pending_consensus_actions: BTreeMap::new(),
            max_pending_consensus_actions: config.max_engine_pending_consensus_actions,
        })
    }

    pub(super) fn tick(
        &mut self,
        node_id: &str,
        world_id: &str,
        now_ms: i64,
        gossip: Option<&GossipEndpoint>,
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
        let hold_for_replication_probe = if let Some(endpoint) = replication_network.as_ref() {
            self.maybe_hold_proposal_for_replication_successor_probe(
                endpoint,
                node_id,
                world_id,
                now_ms,
                replication.as_deref_mut(),
            )?
        } else {
            false
        };
        self.align_next_slot_to_wall_clock(current_slot)?;

        let mut decision = if self.pending.is_some() {
            self.advance_pending_attestations(now_ms)?
        } else if hold_for_replication_probe {
            self.idle_pending_decision()?
        } else if !self.allow_local_proposals {
            self.idle_pending_decision()?
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
        if matches!(decision.status, PosConsensusStatus::Committed)
            && decision.height > prev_committed_height
        {
            if let Some(latency_ms) = self.pending.as_ref().and_then(|proposal| {
                (proposal.height == decision.height)
                    .then(|| now_ms.saturating_sub(proposal.opened_at_ms))
            }) {
                self.record_finality_latency(latency_ms);
            }
        }
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

    pub(super) fn next_tick_wait_duration(&self, now_ms: i64, fallback: Duration) -> Duration {
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

    pub(super) fn classify_inbound_slot_window(
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

    pub(super) fn note_inbound_timing_reject(&mut self, reason: String) {
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

    pub(super) fn insert_attestation(
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

    pub(super) fn apply_decision(&mut self, decision: &PosDecision) -> Result<(), NodeError> {
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

    pub(super) fn pending_consensus_action_capacity(&self) -> usize {
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

    fn record_finality_latency(&mut self, latency_ms: i64) {
        if self.recent_finality_latency_ms.len() >= FINALITY_LATENCY_HISTORY_LIMIT {
            self.recent_finality_latency_ms.pop_front();
        }
        self.recent_finality_latency_ms.push_back(latency_ms.max(0));
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

    pub(super) fn snapshot_from_decision(&self, decision: &PosDecision) -> NodeConsensusSnapshot {
        let pending_proposal = self.pending.as_ref().map(|proposal| {
            let action_payload_bytes =
                total_action_payload_bytes(proposal.committed_actions.iter());
            NodePendingProposalSnapshot {
                height: proposal.height,
                slot: proposal.slot,
                epoch: proposal.epoch,
                proposer_id: proposal.proposer_id.clone(),
                opened_at_ms: proposal.opened_at_ms,
                action_count: proposal.committed_actions.len(),
                action_payload_bytes,
                attestation_count: proposal.attestations.len(),
                approved_stake: proposal.approved_stake,
                rejected_stake: proposal.rejected_stake,
                required_stake: self.required_stake,
                total_stake: self.total_stake,
                approval_progress_bps: progress_bps(proposal.approved_stake, self.required_stake),
                rejection_progress_bps: progress_bps(proposal.rejected_stake, self.required_stake),
                remaining_approval_stake: self
                    .required_stake
                    .saturating_sub(proposal.approved_stake),
                status: proposal.status,
            }
        });
        let reserved_requeue_action_count = self
            .pending
            .as_ref()
            .map(|proposal| proposal.committed_actions.len())
            .unwrap_or(0);
        let reserved_requeue_payload_bytes = self
            .pending
            .as_ref()
            .map(|proposal| total_action_payload_bytes(proposal.committed_actions.iter()))
            .unwrap_or(0);
        let queued_payload_bytes =
            total_action_payload_bytes(self.pending_consensus_actions.values());
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
            pending_proposal,
            pending_consensus_actions: NodePendingConsensusActionsSnapshot {
                queued_action_count: self.pending_consensus_actions.len(),
                queued_payload_bytes,
                reserved_requeue_action_count,
                reserved_requeue_payload_bytes,
                available_capacity: self.pending_consensus_action_capacity(),
                max_capacity: self.max_pending_consensus_actions,
                submit_buffer_action_count: 0,
                submit_buffer_payload_bytes: 0,
                submit_buffer_max_capacity: 0,
            },
            recent_finality_latency: summarize_finality_latency(
                self.recent_finality_latency_ms.iter().copied(),
            ),
            last_status: Some(decision.status),
            last_block_hash: self
                .last_committed_block_hash
                .clone()
                .or_else(|| Some(decision.block_hash.clone())),
            last_execution_height: self.last_execution_height,
            last_execution_block_hash: self.last_execution_block_hash.clone(),
            last_execution_state_root: self.last_execution_state_root.clone(),
        }
    }

    fn seed_reverse_gossip_path(
        &mut self,
        endpoint: &GossipEndpoint,
        node_id: &str,
        world_id: &str,
        now_ms: i64,
    ) -> Result<(), NodeError> {
        if !self.gossip_reverse_path_seeding_enabled {
            return Ok(());
        }
        if self
            .last_gossip_reverse_path_seed_at_ms
            .map(|last_ms| now_ms.saturating_sub(last_ms) < GOSSIP_REVERSE_PATH_SEED_INTERVAL_MS)
            .unwrap_or(false)
        {
            return Ok(());
        }
        endpoint.broadcast_hello(&crate::gossip_udp::GossipHelloMessage {
            version: 1,
            world_id: world_id.to_string(),
            node_id: node_id.to_string(),
            sent_at_ms: now_ms,
        })?;
        self.last_gossip_reverse_path_seed_at_ms = Some(now_ms);
        Ok(())
    }

    pub(super) fn commit_execution_binding_for_height(
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

    pub(super) fn execution_binding_for_height(&self, height: u64) -> Option<(&str, &str)> {
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

    pub(super) fn remember_execution_binding_for_height(&mut self, height: u64) {
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

    pub(super) fn validate_peer_commit_execution_binding(
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
}

fn total_action_payload_bytes<'a>(actions: impl Iterator<Item = &'a NodeConsensusAction>) -> usize {
    actions.map(|action| action.payload_cbor.len()).sum()
}

fn progress_bps(numerator: u64, denominator: u64) -> u16 {
    if denominator == 0 {
        return 0;
    }
    let ratio = numerator.saturating_mul(10_000).saturating_div(denominator);
    ratio.min(10_000) as u16
}

fn rounded_percentile_index(sample_count: usize, pct: usize) -> usize {
    sample_count
        .saturating_sub(1)
        .saturating_mul(pct)
        .saturating_add(50)
        / 100
}

fn summarize_finality_latency(latencies: impl Iterator<Item = i64>) -> NodeFinalityLatencySnapshot {
    let mut samples = latencies
        .filter(|latency| *latency >= 0)
        .collect::<Vec<_>>();
    if samples.is_empty() {
        return NodeFinalityLatencySnapshot::default();
    }
    samples.sort_unstable();
    let sample_count = samples.len();
    let total = samples
        .iter()
        .fold(0_i128, |acc, value| acc.saturating_add(*value as i128));
    let percentile = |pct: usize| -> Option<i64> {
        let idx = rounded_percentile_index(sample_count, pct);
        samples.get(idx).copied()
    };
    NodeFinalityLatencySnapshot {
        sample_count,
        avg_latency_ms: Some((total / sample_count as i128) as i64),
        max_latency_ms: samples.last().copied(),
        p50_latency_ms: percentile(50),
        p95_latency_ms: percentile(95),
    }
}
