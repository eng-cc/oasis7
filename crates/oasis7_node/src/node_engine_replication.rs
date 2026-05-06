use super::node_engine_storage_challenge::{
    evaluate_storage_challenge_sample, StorageChallengeSampleOutcome,
};
use super::*;
use crate::replication_state_reconcile::ReplicationCommitPayload;

impl PosNodeEngine {
    pub(super) fn broadcast_local_replication(
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
        if !matches!(decision.status, PosConsensusStatus::Committed) {
            return Ok(());
        }
        if self
            .expected_proposer(decision.slot)
            .as_deref()
            .map(|proposer_id| proposer_id != node_id)
            .unwrap_or(false)
        {
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

    pub(super) fn enforce_storage_challenge_gate(
        &mut self,
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
        if self.committed_height < STORAGE_GATE_NETWORK_WARMUP_HEIGHT && self.peer_heads.is_empty()
        {
            return Ok(());
        }
        self.prune_storage_challenge_success_cache();
        let primary_samples = replication
            .recent_replicated_content_refs(world_id, STORAGE_GATE_NETWORK_SAMPLES_PER_CHECK)?;
        if primary_samples.is_empty() {
            return Ok(());
        }

        let mut successful_matches = 0usize;
        let mut attempted_probes = 0usize;
        let mut total_samples = 0usize;
        let mut failure_reasons = Vec::new();
        let mut hard_failure = false;
        for (_, content_hash) in primary_samples.iter() {
            total_samples = total_samples.saturating_add(1);
            if self.storage_challenge_success_cache_hit(replication, content_hash.as_str())? {
                successful_matches = successful_matches.saturating_add(1);
                continue;
            }
            attempted_probes = attempted_probes.saturating_add(1);
            match evaluate_storage_challenge_sample(
                replication,
                endpoint,
                world_id,
                content_hash.as_str(),
            )? {
                StorageChallengeSampleOutcome::Matched => {
                    successful_matches = successful_matches.saturating_add(1);
                    self.mark_storage_challenge_success(content_hash.as_str());
                }
                StorageChallengeSampleOutcome::Unavailable { reason } => {
                    failure_reasons.push(reason);
                }
                StorageChallengeSampleOutcome::HardFailure { reason } => {
                    hard_failure = true;
                    failure_reasons.push(reason);
                }
            }
        }

        let mut required_matches = required_network_blob_matches(primary_samples.len());
        if self.committed_height < STORAGE_GATE_NETWORK_WARMUP_HEIGHT
            || (self.require_peer_execution_hashes && self.peer_heads.is_empty())
        {
            required_matches = required_matches.min(1);
        }
        if successful_matches >= required_matches {
            return Ok(());
        }

        if !hard_failure {
            let fallback_samples = replication.replicated_content_refs_from_height(
                world_id,
                self.storage_challenge_fallback_height,
                STORAGE_GATE_FALLBACK_SAMPLES_PER_CHECK,
            )?;
            for (height, content_hash) in fallback_samples {
                total_samples = total_samples.saturating_add(1);
                if self.storage_challenge_success_cache_hit(replication, content_hash.as_str())? {
                    successful_matches = successful_matches.saturating_add(1);
                    if successful_matches >= required_matches {
                        self.storage_challenge_fallback_height = height.saturating_add(1);
                        return Ok(());
                    }
                    continue;
                }
                attempted_probes = attempted_probes.saturating_add(1);
                match evaluate_storage_challenge_sample(
                    replication,
                    endpoint,
                    world_id,
                    content_hash.as_str(),
                )? {
                    StorageChallengeSampleOutcome::Matched => {
                        successful_matches = successful_matches.saturating_add(1);
                        self.mark_storage_challenge_success(content_hash.as_str());
                    }
                    StorageChallengeSampleOutcome::Unavailable { reason } => {
                        failure_reasons.push(reason);
                    }
                    StorageChallengeSampleOutcome::HardFailure { reason } => {
                        failure_reasons.push(reason);
                        break;
                    }
                }
                if successful_matches >= required_matches {
                    self.storage_challenge_fallback_height = height.saturating_add(1);
                    return Ok(());
                }
            }
        }

        if successful_matches < required_matches {
            return Err(NodeError::Consensus {
                reason: format!(
                    "storage challenge gate network threshold unmet: total_samples={} attempted_probes={} required_matches={} successful_matches={} reasons={:?}",
                    total_samples,
                    attempted_probes,
                    required_matches,
                    successful_matches,
                    failure_reasons
                ),
            });
        }
        Ok(())
    }

    pub(super) fn ingest_network_replications(
        &mut self,
        endpoint: &ReplicationNetworkEndpoint,
        node_id: &str,
        world_id: &str,
        mut replication: Option<&mut ReplicationRuntime>,
        mut execution_hook: Option<&mut dyn NodeExecutionHook>,
    ) -> Result<(), NodeError> {
        let Some(replication_runtime) = replication.as_deref_mut() else {
            return Ok(());
        };
        self.refresh_replication_persisted_height(replication_runtime, world_id)?;
        let messages = endpoint.drain_replications()?;
        let mut rejected = Vec::new();
        for message in messages {
            let committed_successor = checked_replication_successor(
                self.committed_height,
                "committed_height",
                "ingesting replication message",
            )?;
            let persisted_successor = checked_replication_successor(
                self.replication_persisted_height,
                "replication_persisted_height",
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
                .map(|payload| payload.height <= persisted_successor)
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
                        if replication_runtime
                            .load_commit_message_by_height(world_id, payload.height)?
                            .is_some()
                        {
                            self.replication_persisted_height =
                                self.replication_persisted_height.max(payload.height);
                        }
                        if payload.height == committed_successor
                            && self.replication_persisted_height >= payload.height
                        {
                            let full_payload =
                                parse_replication_commit_payload(message.payload.as_slice())
                                    .ok_or_else(|| NodeError::Replication {
                                        reason: format!(
                                    "replication message payload decode failed at height {}",
                                    payload.height
                                ),
                                    })?;
                            with_execution_hook(&mut execution_hook, |hook| {
                                self.apply_synced_replication_commit(world_id, &full_payload, hook)
                            })?;
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

    pub(super) fn sync_missing_replication_commits(
        &mut self,
        endpoint: &ReplicationNetworkEndpoint,
        node_id: &str,
        world_id: &str,
        mut replication: Option<&mut ReplicationRuntime>,
        mut execution_hook: Option<&mut dyn NodeExecutionHook>,
    ) -> Result<(), NodeError> {
        let Some(replication_runtime) = replication.as_deref_mut() else {
            return Ok(());
        };
        self.refresh_replication_persisted_height(replication_runtime, world_id)?;
        if self.network_committed_height <= self.replication_persisted_height {
            return Ok(());
        }

        let mut next_height = checked_replication_successor(
            self.replication_persisted_height,
            "replication_persisted_height",
            "starting replication gap sync",
        )?;
        while next_height <= self.network_committed_height {
            let mut synced_commit: Option<ReplicationCommitPayload> = None;
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
                    Ok(GapSyncHeightOutcome::Synced { payload }) => {
                        synced_commit = Some(payload);
                        break;
                    }
                    Ok(GapSyncHeightOutcome::NotFound) => {
                        not_found = true;
                        break;
                    }
                    Err(err) if replication_request_waitable_connection_gap(&err) => {
                        return Ok(());
                    }
                    Err(err) => {
                        last_error = Some(format!(
                            "attempt {attempt}/{} failed: {}",
                            REPLICATION_GAP_SYNC_MAX_RETRIES_PER_HEIGHT, err
                        ));
                    }
                }
            }
            if let Some(payload) = synced_commit {
                self.replication_persisted_height =
                    self.replication_persisted_height.max(next_height);
                with_execution_hook(&mut execution_hook, |hook| {
                    self.apply_synced_replication_commit(world_id, &payload, hook)
                })?;
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

    pub(super) fn refresh_replication_persisted_height(
        &mut self,
        replication_runtime: &ReplicationRuntime,
        world_id: &str,
    ) -> Result<(), NodeError> {
        self.replication_persisted_height = self
            .replication_persisted_height
            .max(replication_runtime.latest_persisted_commit_height(world_id)?);
        Ok(())
    }
}
