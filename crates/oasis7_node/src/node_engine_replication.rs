use super::node_engine_storage_challenge::{
    evaluate_storage_challenge_sample, StorageChallengeSampleOutcome,
};
use super::*;
use crate::replication_state_reconcile::ReplicationCommitPayload;
use oasis7_proto::distributed::WorldHeadAnnounce;

impl PosNodeEngine {
    fn high_replication_checkpoint_candidates(
        advertised_network_height: u64,
        blocked_height: u64,
    ) -> Vec<u64> {
        let mut candidates = Vec::new();
        let mut push_candidate = |height: u64| {
            if height > blocked_height && height > 0 && !candidates.contains(&height) {
                candidates.push(height);
            }
        };
        push_candidate(advertised_network_height);
        for interval in [64_u64, 32_u64] {
            let aligned = advertised_network_height - (advertised_network_height % interval);
            push_candidate(aligned);
            push_candidate(aligned.saturating_sub(interval));
        }
        candidates.sort_unstable_by(|a, b| b.cmp(a));
        candidates
    }

    fn clear_replication_gap_sync_blocked_if_unblocked(&mut self) {
        if self
            .last_replication_gap_sync_blocked_height
            .map(|height| self.replication_persisted_height >= height)
            .unwrap_or(false)
        {
            self.last_replication_gap_sync_blocked_height = None;
            self.last_replication_gap_sync_blocked_reason = None;
            self.last_replication_gap_sync_repair_attempt_height = None;
            self.last_replication_gap_sync_repair_attempt_summary = None;
        }
    }

    fn advance_contiguous_replication_persisted_height(
        &mut self,
        replication_runtime: &ReplicationRuntime,
        world_id: &str,
        observed_latest_height: u64,
    ) -> Result<(), NodeError> {
        let mut next_height = checked_replication_successor(
            self.replication_persisted_height,
            "replication_persisted_height",
            "advancing contiguous replication persisted height",
        )?;
        while next_height <= observed_latest_height {
            if replication_runtime
                .load_commit_message_by_height(world_id, next_height)?
                .is_none()
            {
                break;
            }
            self.replication_persisted_height = next_height;
            next_height = checked_replication_successor(
                next_height,
                "next_height",
                "advancing contiguous replication persisted height cursor",
            )?;
        }
        self.clear_replication_gap_sync_blocked_if_unblocked();
        Ok(())
    }

    fn validate_world_head_checkpoint_payload(
        world_id: &str,
        payload: &ReplicationCommitPayload,
        expected_head: &WorldHeadAnnounce,
    ) -> Result<(), NodeError> {
        let payload_state_root = payload.execution_state_root.as_deref().unwrap_or_default();
        if expected_head.world_id != world_id
            || payload.world_id != world_id
            || expected_head.height != payload.height
            || expected_head.block_hash != payload.block_hash
            || expected_head.state_root != payload_state_root
        {
            return Err(NodeError::Replication {
                reason: format!(
                    "world head checkpoint mismatch: world_id={} expected_height={} payload_height={} expected_block_hash={} payload_block_hash={} expected_state_root={} payload_state_root={}",
                    world_id,
                    expected_head.height,
                    payload.height,
                    expected_head.block_hash,
                    payload.block_hash,
                    expected_head.state_root,
                    payload_state_root
                ),
            });
        }
        Ok(())
    }

    pub(super) fn broadcast_local_replication(
        &mut self,
        gossip_endpoint: Option<&GossipEndpoint>,
        network_endpoint: Option<&ReplicationNetworkEndpoint>,
        node_id: &str,
        world_id: &str,
        now_ms: i64,
        decision: &PosDecision,
        replication: Option<&mut ReplicationRuntime>,
        execution_hook: Option<&mut dyn NodeExecutionHook>,
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
        let execution_checkpoint = match (execution_hook, execution_block_hash) {
            (Some(hook), Some(_)) => hook
                .export_checkpoint_bundle(decision.height)
                .map_err(|reason| NodeError::Execution { reason })?,
            _ => None,
        };
        if let Some(message) = replication.build_local_commit_message_with_checkpoint(
            node_id,
            world_id,
            now_ms,
            decision,
            execution_block_hash,
            execution_state_root,
            execution_checkpoint,
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
            self.advance_contiguous_replication_persisted_height(
                replication,
                world_id,
                decision.height,
            )?;
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
        replication: Option<&mut ReplicationRuntime>,
        execution_hook: Option<&mut dyn NodeExecutionHook>,
    ) -> Result<(), NodeError> {
        self.ingest_network_replications_with_progress(
            endpoint,
            node_id,
            world_id,
            replication,
            execution_hook,
            None,
        )
    }

    pub(super) fn ingest_network_replications_with_progress(
        &mut self,
        endpoint: &ReplicationNetworkEndpoint,
        node_id: &str,
        world_id: &str,
        mut replication: Option<&mut ReplicationRuntime>,
        mut execution_hook: Option<&mut dyn NodeExecutionHook>,
        mut progress_callback: Option<&mut dyn FnMut(NodeConsensusSnapshot)>,
    ) -> Result<(), NodeError> {
        let Some(replication_runtime) = replication.as_deref_mut() else {
            return Ok(());
        };
        self.refresh_replication_persisted_height(replication_runtime, world_id)?;
        let messages = endpoint.drain_replications()?;
        let mut rejected = Vec::new();
        for message in messages {
            if message.node_id == node_id {
                continue;
            }
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
            match replication_runtime.validate_remote_message_for_apply(node_id, world_id, &message)
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
            let mut executed_commit = None;
            if let Some(payload) = payload_view.as_ref() {
                if payload.height == committed_successor {
                    let full_payload = parse_replication_commit_payload(message.payload.as_slice())
                        .ok_or_else(|| NodeError::Replication {
                            reason: format!(
                                "replication message payload decode failed at height {}",
                                payload.height
                            ),
                        })?;
                    let executed = with_execution_hook(&mut execution_hook, |hook| {
                        self.execute_synced_replication_commit(world_id, &full_payload, hook)
                    })?;
                    executed_commit = Some((full_payload.height, executed.0, executed.1));
                }
            }
            let mut persisted_commit = false;
            match replication_runtime.apply_remote_message(node_id, world_id, &message) {
                Ok(()) => {
                    persisted_commit = true;
                    endpoint.publish_local_content_provider(
                        world_id,
                        message.record.content_hash.as_str(),
                    )?;
                    if let Some(payload) = payload_view {
                        self.advance_contiguous_replication_persisted_height(
                            replication_runtime,
                            world_id,
                            payload.height,
                        )?;
                    }
                }
                Err(err) => rejected.push(format!(
                    "node_id={} world_id={} err={}",
                    message.node_id, message.world_id, err
                )),
            }
            if persisted_commit {
                if let Some((height, block_hash, committed_at_ms)) = executed_commit {
                    self.record_synced_replication_height(height, block_hash, committed_at_ms)?;
                    if let Some(callback) = progress_callback.as_deref_mut() {
                        let decision = self.idle_pending_decision()?;
                        callback(self.snapshot_from_decision(&decision));
                    }
                }
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
        if self
            .validator_id_for_peer_head(peer_node_id)
            .map(|validator_id| self.quarantined_validators.contains(&validator_id))
            .unwrap_or(false)
        {
            return;
        }
        self.network_committed_height = self.network_committed_height.max(payload.height);
        self.peer_heads.insert(
            peer_node_id.to_string(),
            PeerCommittedHead {
                height: payload.height,
                block_hash: payload.block_hash.clone(),
                committed_at_ms: payload.committed_at_ms,
                observed_at_ms: crate::runtime_util::now_unix_ms(),
                execution_block_hash: payload.execution_block_hash.clone(),
                execution_state_root: payload.execution_state_root.clone(),
                action_root: String::new(),
                public_key_hex: None,
                signature_hex: None,
            },
        );
    }

    pub(super) fn sync_missing_replication_commits(
        &mut self,
        endpoint: &ReplicationNetworkEndpoint,
        node_id: &str,
        world_id: &str,
        replication: Option<&mut ReplicationRuntime>,
        execution_hook: Option<&mut dyn NodeExecutionHook>,
    ) -> Result<(), NodeError> {
        self.sync_missing_replication_commits_with_progress(
            endpoint,
            node_id,
            world_id,
            replication,
            execution_hook,
            None,
        )
    }

    fn try_sync_high_replication_checkpoint_boundary(
        &mut self,
        endpoint: &ReplicationNetworkEndpoint,
        node_id: &str,
        world_id: &str,
        replication_runtime: &mut ReplicationRuntime,
        checkpoint_height: u64,
        blocked_height: u64,
        expected_checkpoint_head: Option<&WorldHeadAnnounce>,
        execution_hook: &mut Option<&mut dyn NodeExecutionHook>,
        progress_callback: &mut Option<&mut dyn FnMut(NodeConsensusSnapshot)>,
    ) -> Result<bool, NodeError> {
        if self.require_execution_on_commit
            || checkpoint_height <= blocked_height
            || checkpoint_height <= self.replication_persisted_height
        {
            return Ok(false);
        }
        let checkpoint = match self.sync_replication_height_once(
            endpoint,
            node_id,
            world_id,
            replication_runtime,
            checkpoint_height,
        )? {
            GapSyncHeightOutcome::Synced { message, payload } => (message, payload),
            GapSyncHeightOutcome::NotFound { .. } => return Ok(false),
        };
        let (message, payload) = checkpoint;
        if payload.execution_block_hash.is_none() || payload.execution_state_root.is_none() {
            return Ok(false);
        }
        if let Some(expected_head) = expected_checkpoint_head {
            if Self::validate_world_head_checkpoint_payload(world_id, &payload, expected_head)
                .is_err()
            {
                return Ok(false);
            }
        }

        let Some((block_hash, committed_at_ms)) =
            (if let Some(checkpoint_descriptor) = payload.execution_checkpoint.clone() {
                let Some(execution_block_hash) = payload.execution_block_hash.clone() else {
                    return Ok(false);
                };
                let Some(execution_state_root) = payload.execution_state_root.clone() else {
                    return Ok(false);
                };
                if checkpoint_descriptor.height != payload.height
                    || checkpoint_descriptor.execution_block_hash != execution_block_hash
                    || checkpoint_descriptor.execution_state_root != execution_state_root
                {
                    return Ok(false);
                }
                let checkpoint_bundle = self.fetch_execution_checkpoint_bundle(
                    endpoint,
                    world_id,
                    replication_runtime,
                    &checkpoint_descriptor,
                )?;
                with_execution_hook(execution_hook, |hook| {
                    let Some(hook) = hook else {
                        return Ok(None);
                    };
                    let result = hook
                        .install_checkpoint_bundle(
                            NodeExecutionCheckpointInstallContext {
                                world_id: world_id.to_string(),
                                node_id: node_id.to_string(),
                                height: payload.height,
                                node_block_hash: payload.block_hash.clone(),
                                execution_block_hash: execution_block_hash.clone(),
                                execution_state_root: execution_state_root.clone(),
                                committed_at_unix_ms: payload.committed_at_ms,
                            },
                            checkpoint_bundle,
                        )
                        .map_err(|reason| NodeError::Execution { reason })?;
                    if result.execution_height != payload.height
                        || result.execution_block_hash != execution_block_hash
                        || result.execution_state_root != execution_state_root
                    {
                        return Err(NodeError::Execution {
                            reason: format!(
                            "execution checkpoint install returned mismatched binding at height {}",
                            payload.height
                        ),
                        });
                    }
                    self.last_execution_height = result.execution_height;
                    self.last_execution_block_hash = Some(result.execution_block_hash);
                    self.last_execution_state_root = Some(result.execution_state_root);
                    self.remember_execution_binding_for_height(payload.height);
                    Ok(Some((payload.block_hash.clone(), payload.committed_at_ms)))
                })?
            } else {
                if execution_hook.is_some()
                    && checkpoint_height > self.last_execution_height.saturating_add(1)
                {
                    return Ok(false);
                }
                Some(with_execution_hook(execution_hook, |hook| {
                    self.execute_synced_replication_commit(world_id, &payload, hook)
                })?)
            })
        else {
            return Ok(false);
        };
        self.persist_synced_replication_message(
            endpoint,
            node_id,
            world_id,
            replication_runtime,
            &message,
            checkpoint_height,
        )?;
        self.replication_persisted_height =
            self.replication_persisted_height.max(checkpoint_height);
        self.record_synced_replication_height(checkpoint_height, block_hash, committed_at_ms)?;
        self.last_replication_gap_sync_blocked_height = None;
        self.last_replication_gap_sync_blocked_reason = None;
        self.last_replication_gap_sync_repair_attempt_height = None;
        self.last_replication_gap_sync_repair_attempt_summary = None;
        if let Some(callback) = progress_callback.as_deref_mut() {
            let decision = self.idle_pending_decision()?;
            callback(self.snapshot_from_decision(&decision));
        }
        Ok(true)
    }

    fn fetch_execution_checkpoint_bundle(
        &self,
        endpoint: &ReplicationNetworkEndpoint,
        world_id: &str,
        replication_runtime: &ReplicationRuntime,
        descriptor: &NodeExecutionCheckpointDescriptor,
    ) -> Result<NodeExecutionCheckpointBundle, NodeError> {
        self.ensure_execution_checkpoint_blob(
            endpoint,
            world_id,
            replication_runtime,
            descriptor.manifest_ref.as_str(),
            descriptor.manifest_size_bytes,
        )?;
        for blob_ref in &descriptor.blobs {
            self.ensure_execution_checkpoint_blob(
                endpoint,
                world_id,
                replication_runtime,
                blob_ref.content_hash.as_str(),
                blob_ref.size_bytes,
            )?;
        }
        replication_runtime.pin_execution_checkpoint_descriptor(descriptor)?;
        replication_runtime
            .load_execution_checkpoint_bundle(descriptor)?
            .ok_or_else(|| NodeError::Replication {
                reason: format!(
                    "execution checkpoint descriptor could not be materialized at height {}",
                    descriptor.height
                ),
            })
    }

    fn ensure_execution_checkpoint_blob(
        &self,
        endpoint: &ReplicationNetworkEndpoint,
        world_id: &str,
        replication_runtime: &ReplicationRuntime,
        content_hash: &str,
        expected_size_bytes: u64,
    ) -> Result<(), NodeError> {
        if let Some(bytes) = replication_runtime.load_blob_by_hash(content_hash)? {
            if bytes.len() as u64 != expected_size_bytes {
                return Err(NodeError::Replication {
                    reason: format!(
                        "execution checkpoint local blob size mismatch hash={} expected={} actual={}",
                        content_hash,
                        expected_size_bytes,
                        bytes.len()
                    ),
                });
            }
            return Ok(());
        }
        let request = replication_runtime.build_fetch_blob_request(content_hash)?;
        let response = request_fetch_blob_with_route_fallback(
            endpoint,
            world_id,
            content_hash,
            &request,
            None,
        )?;
        if !response.found {
            return Err(NodeError::Replication {
                reason: format!("execution checkpoint blob not found hash={content_hash}"),
            });
        }
        let blob = response.blob.ok_or_else(|| NodeError::Replication {
            reason: format!("execution checkpoint fetch missing blob hash={content_hash}"),
        })?;
        if blob.len() as u64 != expected_size_bytes {
            return Err(NodeError::Replication {
                reason: format!(
                    "execution checkpoint fetched blob size mismatch hash={} expected={} actual={}",
                    content_hash,
                    expected_size_bytes,
                    blob.len()
                ),
            });
        }
        let actual = blake3_hex(blob.as_slice());
        if actual != content_hash {
            return Err(NodeError::Replication {
                reason: format!(
                    "execution checkpoint fetched blob hash mismatch expected={} actual={}",
                    content_hash, actual
                ),
            });
        }
        replication_runtime.store_blob_by_hash(content_hash, blob.as_slice())
    }

    pub(super) fn sync_missing_replication_commits_with_progress(
        &mut self,
        endpoint: &ReplicationNetworkEndpoint,
        node_id: &str,
        world_id: &str,
        mut replication: Option<&mut ReplicationRuntime>,
        mut execution_hook: Option<&mut dyn NodeExecutionHook>,
        mut progress_callback: Option<&mut dyn FnMut(NodeConsensusSnapshot)>,
    ) -> Result<(), NodeError> {
        let Some(replication_runtime) = replication.as_deref_mut() else {
            return Ok(());
        };
        self.refresh_replication_persisted_height(replication_runtime, world_id)?;
        let starting_replication_persisted_height = self.replication_persisted_height;
        let advertised_world_head = endpoint.lookup_world_head(world_id)?;
        let advertised_network_height = self.network_committed_height.max(
            advertised_world_head
                .as_ref()
                .map(|head| head.height)
                .unwrap_or(0),
        );
        let expected_checkpoint_head = advertised_world_head
            .as_ref()
            .filter(|head| head.height == advertised_network_height);
        if advertised_network_height <= self.replication_persisted_height {
            self.last_replication_gap_sync_blocked_height = None;
            self.last_replication_gap_sync_blocked_reason = None;
            return Ok(());
        }

        let network_lag =
            advertised_network_height.saturating_sub(self.replication_persisted_height);
        if network_lag > REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL {
            for checkpoint_candidate in Self::high_replication_checkpoint_candidates(
                advertised_network_height,
                self.replication_persisted_height,
            ) {
                let expected_candidate_head =
                    expected_checkpoint_head.filter(|head| head.height == checkpoint_candidate);
                if self.try_sync_high_replication_checkpoint_boundary(
                    endpoint,
                    node_id,
                    world_id,
                    replication_runtime,
                    checkpoint_candidate,
                    self.replication_persisted_height,
                    expected_candidate_head,
                    &mut execution_hook,
                    &mut progress_callback,
                )? {
                    if self.replication_persisted_height > starting_replication_persisted_height {
                        self.network_committed_height =
                            self.network_committed_height.max(advertised_network_height);
                    }
                    return Ok(());
                }
            }
        }

        let mut next_height = checked_replication_successor(
            self.replication_persisted_height,
            "replication_persisted_height",
            "starting replication gap sync",
        )?;
        let gap_sync_target_height = advertised_network_height.min(
            self.replication_persisted_height
                .saturating_add(REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL),
        );
        while next_height <= gap_sync_target_height {
            let mut synced_commit: Option<(
                replication::GossipReplicationMessage,
                ReplicationCommitPayload,
            )> = None;
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
                    Ok(GapSyncHeightOutcome::Synced { message, payload }) => {
                        synced_commit = Some((message, payload));
                        break;
                    }
                    Ok(GapSyncHeightOutcome::NotFound { repair_summary }) => {
                        not_found = true;
                        self.last_replication_gap_sync_repair_attempt_height = Some(next_height);
                        self.last_replication_gap_sync_repair_attempt_summary =
                            Some(repair_summary);
                        break;
                    }
                    Err(err) if replication_request_waitable_connection_gap(&err) => {
                        if self.replication_persisted_height > starting_replication_persisted_height
                        {
                            self.network_committed_height =
                                self.network_committed_height.max(advertised_network_height);
                        }
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
            if let Some((message, payload)) = synced_commit {
                let (block_hash, committed_at_ms) =
                    with_execution_hook(&mut execution_hook, |hook| {
                        self.execute_synced_replication_commit(world_id, &payload, hook)
                    })?;
                self.persist_synced_replication_message(
                    endpoint,
                    node_id,
                    world_id,
                    replication_runtime,
                    &message,
                    next_height,
                )?;
                self.replication_persisted_height =
                    self.replication_persisted_height.max(next_height);
                self.record_synced_replication_height(next_height, block_hash, committed_at_ms)?;
                if let Some(callback) = progress_callback.as_deref_mut() {
                    let decision = self.idle_pending_decision()?;
                    callback(self.snapshot_from_decision(&decision));
                }
                next_height = checked_replication_successor(
                    next_height,
                    "next_height",
                    "advancing replication gap sync cursor",
                )?;
                continue;
            }
            if not_found {
                for checkpoint_candidate in Self::high_replication_checkpoint_candidates(
                    advertised_network_height,
                    next_height,
                ) {
                    let expected_candidate_head =
                        expected_checkpoint_head.filter(|head| head.height == checkpoint_candidate);
                    if self.try_sync_high_replication_checkpoint_boundary(
                        endpoint,
                        node_id,
                        world_id,
                        replication_runtime,
                        checkpoint_candidate,
                        next_height,
                        expected_candidate_head,
                        &mut execution_hook,
                        &mut progress_callback,
                    )? {
                        break;
                    }
                }
                if self.replication_persisted_height > next_height {
                    break;
                }
                self.last_replication_gap_sync_blocked_height = Some(next_height);
                self.last_replication_gap_sync_blocked_reason = Some(format!(
                    "replication gap sync blocked: missing commit height {next_height} while advertised_network_height={} network_committed_height={} gap_sync_target_height={} replication_persisted_height={} repair_attempt={}",
                    advertised_network_height,
                    self.network_committed_height,
                    gap_sync_target_height,
                    self.replication_persisted_height,
                    self.last_replication_gap_sync_repair_attempt_summary
                        .as_deref()
                        .unwrap_or("unavailable")
                ));
                break;
            }
            self.last_replication_gap_sync_blocked_height = Some(next_height);
            self.last_replication_gap_sync_blocked_reason = Some(format!(
                "replication gap sync failed at height {next_height}: {}",
                last_error
                    .clone()
                    .unwrap_or_else(|| "unknown error".to_string())
            ));
            return Err(NodeError::Replication {
                reason: format!(
                    "gap sync height {} failed after {} attempts: {}",
                    next_height,
                    REPLICATION_GAP_SYNC_MAX_RETRIES_PER_HEIGHT,
                    last_error.unwrap_or_else(|| "unknown error".to_string())
                ),
            });
        }
        if self.replication_persisted_height >= advertised_network_height {
            self.last_replication_gap_sync_blocked_height = None;
            self.last_replication_gap_sync_blocked_reason = None;
            self.last_replication_gap_sync_repair_attempt_height = None;
            self.last_replication_gap_sync_repair_attempt_summary = None;
        } else {
            self.clear_replication_gap_sync_blocked_if_unblocked();
        }
        if self.replication_persisted_height > starting_replication_persisted_height {
            self.network_committed_height =
                self.network_committed_height.max(advertised_network_height);
        }
        Ok(())
    }

    pub(super) fn refresh_replication_persisted_height(
        &mut self,
        replication_runtime: &ReplicationRuntime,
        world_id: &str,
    ) -> Result<(), NodeError> {
        if self.replication_persisted_height == 0 {
            let durable_writer_height = replication_runtime.writer_last_replicated_height();
            for durable_baseline_height in [self.committed_height, durable_writer_height] {
                if durable_baseline_height == 0 {
                    continue;
                }
                if replication_runtime
                    .load_commit_message_by_height(world_id, durable_baseline_height)?
                    .is_some()
                {
                    self.replication_persisted_height = durable_baseline_height;
                    break;
                }
            }
        }
        let observed_latest_height =
            replication_runtime.latest_persisted_commit_height(world_id)?;
        self.advance_contiguous_replication_persisted_height(
            replication_runtime,
            world_id,
            observed_latest_height,
        )?;
        Ok(())
    }
}
