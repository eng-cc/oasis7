use super::*;
use crate::replication_state_reconcile::ReplicationCommitPayload;
use oasis7_proto::distributed::WorldHeadAnnounce;

pub(super) enum FreshObserverCheckpointBootstrap {
    Installed,
    PreflightUnavailable,
    RetryPending,
    HighCheckpointPending,
    LowHeadConfirmationPending,
    NotInstalled,
}

impl FreshObserverCheckpointBootstrap {
    pub(super) fn should_defer_height_one(&self) -> bool {
        matches!(
            self,
            Self::HighCheckpointPending | Self::LowHeadConfirmationPending
        )
    }
}

impl PosNodeEngine {
    // Mirrors release_default.execution_checkpoint_keep. Probe the advertised head first, then
    // the aligned retained-window boundaries, including the latest completed boundary.
    const HIGH_REPLICATION_CHECKPOINT_LOOKBACK_WINDOWS: u64 = 8;

    pub(super) fn retain_high_peer_checkpoint_retry_authority(&mut self) -> bool {
        let pending = self.checkpoint_bootstrap_enabled
            && self.committed_height == 0
            && self.replication_persisted_height == 0
            && self.last_execution_height == 0
            && self
                .peer_heads
                .values()
                .any(|head| head.height >= REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL);
        if pending {
            self.fresh_observer_checkpoint_bootstrap_retry_pending = true;
        }
        pending
    }

    pub(super) fn should_defer_fresh_observer_checkpoint_retry(&self) -> bool {
        let high_head_retry_pending = self.fresh_observer_checkpoint_bootstrap_retry_pending;
        self.checkpoint_bootstrap_enabled
            && self.committed_height == 0
            && self.replication_persisted_height == 0
            && self.last_execution_height == 0
            && (self
                .fresh_observer_checkpoint_low_head_confirmation
                .is_some()
                || high_head_retry_pending)
    }

    /// Keep a fresh observer at height zero while connected peers advertise a
    /// high, unresolved head that the world-head lookup may not expose yet.
    ///
    /// The world-head lookup is a bounded DHT/connected-provider probe and may
    /// transiently return a stale height-one candidate even though the
    /// validated peer-head cache already contains a current connected
    /// validator head.  Replaying that candidate would bypass checkpoint
    /// closure and can trigger an execution-peer mismatch without a height-zero
    /// rollback.  This gate is intentionally conservative: it is only
    /// used for a fresh observer with an unresolved high peer head, and callers
    /// still require a verified checkpoint receipt before advancing.
    pub(super) fn hold_fresh_observer_for_high_peer_heads(
        &mut self,
        advertised_network_height: u64,
    ) -> bool {
        if !self.checkpoint_bootstrap_enabled
            || self.committed_height != 0
            || self.replication_persisted_height != 0
            || self.last_execution_height != 0
            || advertised_network_height < REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL
        {
            return false;
        }
        let hold = self
            .peer_heads
            .values()
            .any(|head| head.height >= REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL);
        if hold {
            self.fresh_observer_checkpoint_bootstrap_retry_pending = true;
        }
        hold
    }

    pub(super) fn hold_fresh_observer_for_connected_validator_quorum(
        &mut self,
        endpoint: &ReplicationNetworkEndpoint,
        advertised_height: u64,
    ) -> bool {
        if !self.checkpoint_bootstrap_enabled
            || self.committed_height != 0
            || self.replication_persisted_height != 0
            || self.last_execution_height != 0
            || advertised_height >= REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL
        {
            return false;
        }
        let connected_validator_ids = endpoint
            .connected_peer_ids()
            .into_iter()
            .filter_map(|node_id| self.validator_id_for_peer_head(node_id.as_str()))
            .filter(|validator_id| !self.quarantined_validators.contains(validator_id))
            .collect::<BTreeSet<_>>();
        let connected_stake = connected_validator_ids
            .iter()
            .filter_map(|validator_id| self.validators.get(validator_id).copied())
            .sum::<u64>();
        if connected_stake < self.required_stake || connected_stake == 0 {
            return false;
        }
        self.fresh_observer_checkpoint_low_head_confirmation = None;
        self.fresh_observer_checkpoint_bootstrap_retry_pending = true;
        true
    }

    fn cached_high_checkpoint_head(&self, world_id: &str) -> Option<WorldHeadAnnounce> {
        self.peer_heads
            .iter()
            .filter_map(|(node_id, head)| {
                let validator_id = self.validator_id_for_peer_head(node_id)?;
                if self.quarantined_validators.contains(&validator_id)
                    || head.height < REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL
                    || head.block_hash.is_empty()
                {
                    return None;
                }
                let execution_block_hash = head.execution_block_hash.as_ref()?;
                let state_root = head.execution_state_root.as_ref()?;
                if execution_block_hash.is_empty() || state_root.is_empty() {
                    return None;
                }
                Some(WorldHeadAnnounce {
                    world_id: world_id.to_string(),
                    height: head.height,
                    block_hash: head.block_hash.clone(),
                    state_root: state_root.clone(),
                    timestamp_ms: head.committed_at_ms,
                    signature: String::new(),
                })
            })
            .max_by(|left, right| {
                left.height
                    .cmp(&right.height)
                    .then_with(|| left.block_hash.cmp(&right.block_hash))
                    .then_with(|| left.state_root.cmp(&right.state_root))
            })
    }

    pub(super) fn try_bootstrap_fresh_observer_from_advertised_checkpoint(
        &mut self,
        endpoint: &ReplicationNetworkEndpoint,
        node_id: &str,
        world_id: &str,
        replication_runtime: &mut ReplicationRuntime,
        execution_hook: &mut Option<&mut dyn NodeExecutionHook>,
        progress_callback: &mut Option<
            &mut dyn FnMut(NodeConsensusSnapshot) -> Result<(), NodeError>,
        >,
    ) -> Result<FreshObserverCheckpointBootstrap, NodeError> {
        if execution_hook.is_none()
            || !self.checkpoint_bootstrap_enabled
            || self.committed_height != 0
            || self.replication_persisted_height != 0
            || self.last_execution_height != 0
        {
            return Ok(FreshObserverCheckpointBootstrap::NotInstalled);
        }
        let Some(advertised_head) = endpoint.lookup_world_head(world_id)? else {
            self.fresh_observer_checkpoint_preflight_unavailable = true;
            self.fresh_observer_checkpoint_low_head_confirmation = None;
            return Ok(FreshObserverCheckpointBootstrap::PreflightUnavailable);
        };
        self.fresh_observer_checkpoint_preflight_unavailable = false;
        if advertised_head.height < REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL {
            // A stale world-head response must not revoke a high connected
            // peer-head observation.  The DHT/peer world-head route can still
            // report height one while a fresh observer has already observed
            // a retained high head from a validator.  Establish retry
            // authority before low-head confirmation so a later height-one
            // candidate cannot enter incremental execution without a verified
            // checkpoint closure.
            if let Some(cached_head) = self.cached_high_checkpoint_head(world_id) {
                self.fresh_observer_checkpoint_low_head_confirmation = None;
                self.fresh_observer_checkpoint_bootstrap_retry_pending = false;
                return match self.try_sync_high_replication_checkpoint_boundary(
                    endpoint,
                    node_id,
                    world_id,
                    replication_runtime,
                    cached_head.height,
                    0,
                    Some(&cached_head),
                    execution_hook,
                    progress_callback,
                ) {
                    Ok(true) => Ok(FreshObserverCheckpointBootstrap::Installed),
                    Ok(false) => {
                        self.fresh_observer_checkpoint_bootstrap_retry_pending = true;
                        Ok(FreshObserverCheckpointBootstrap::HighCheckpointPending)
                    }
                    Err(err) if Self::high_replication_checkpoint_probe_can_continue(&err) => {
                        self.fresh_observer_checkpoint_bootstrap_retry_pending = true;
                        Ok(FreshObserverCheckpointBootstrap::RetryPending)
                    }
                    Err(err) => Err(err),
                };
            }
            if self.hold_fresh_observer_for_connected_validator_quorum(
                endpoint,
                advertised_head.height,
            ) {
                return Ok(FreshObserverCheckpointBootstrap::HighCheckpointPending);
            }
            // Preserve the existing fail-closed hold for high heads that are
            // not trustworthy checkpoint candidates (for example, an unknown
            // or incomplete peer binding).  They must not be allowed to age
            // into low-head confirmation and height-one replay.
            if self
                .peer_heads
                .values()
                .any(|head| head.height >= REPLICATION_GAP_SYNC_MAX_HEIGHTS_PER_POLL)
            {
                self.fresh_observer_checkpoint_low_head_confirmation = None;
                self.fresh_observer_checkpoint_bootstrap_retry_pending = true;
                return Ok(FreshObserverCheckpointBootstrap::HighCheckpointPending);
            }
            // An already observed network height can require an immediate low-height
            // checkpoint to recover missing history. Only a completely unestablished
            // fresh observer needs to wait one poll for a stale peer head to advance.
            if self.network_committed_height != 0 {
                self.fresh_observer_checkpoint_low_head_confirmation = None;
                return Ok(if self.fresh_observer_checkpoint_bootstrap_retry_pending {
                    FreshObserverCheckpointBootstrap::HighCheckpointPending
                } else {
                    FreshObserverCheckpointBootstrap::NotInstalled
                });
            }
            let identity = (
                advertised_head.height,
                advertised_head.block_hash.clone(),
                advertised_head.state_root.clone(),
            );
            if self
                .fresh_observer_checkpoint_low_head_confirmation
                .as_ref()
                == Some(&identity)
            {
                self.fresh_observer_checkpoint_low_head_confirmation = None;
                return Ok(FreshObserverCheckpointBootstrap::NotInstalled);
            }
            self.fresh_observer_checkpoint_low_head_confirmation = Some(identity);
            return Ok(FreshObserverCheckpointBootstrap::LowHeadConfirmationPending);
        }
        self.fresh_observer_checkpoint_bootstrap_retry_pending = false;
        self.fresh_observer_checkpoint_low_head_confirmation = None;
        match self.try_sync_high_replication_checkpoint_boundary(
            endpoint,
            node_id,
            world_id,
            replication_runtime,
            advertised_head.height,
            0,
            Some(&advertised_head),
            execution_hook,
            progress_callback,
        ) {
            Ok(true) => Ok(FreshObserverCheckpointBootstrap::Installed),
            // A high advertised head is evidence that replaying the height-one
            // tail is unsafe until this observer has either installed the
            // matching verified checkpoint or observed that head change. In
            // particular, a bounded fetch probe can return `NotFound` while
            // the connected peer is still becoming request-ready.
            Ok(false) => {
                self.fresh_observer_checkpoint_bootstrap_retry_pending = true;
                Ok(FreshObserverCheckpointBootstrap::HighCheckpointPending)
            }
            Err(err) if Self::high_replication_checkpoint_probe_can_continue(&err) => {
                self.fresh_observer_checkpoint_bootstrap_retry_pending = true;
                Ok(FreshObserverCheckpointBootstrap::RetryPending)
            }
            Err(err) => Err(err),
        }
    }

    pub(super) fn validate_world_head_checkpoint_payload(
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

    pub(super) fn high_replication_checkpoint_candidates(
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
            for lookback in 0..=Self::HIGH_REPLICATION_CHECKPOINT_LOOKBACK_WINDOWS {
                push_candidate(aligned.saturating_sub(interval.saturating_mul(lookback)));
            }
        }
        candidates
    }

    pub(super) fn high_replication_checkpoint_probe_can_continue(err: &NodeError) -> bool {
        let NodeError::Replication { reason } = err else {
            return false;
        };
        let fetch_commit_route_error =
            crate::network_bridge::replication_network_error_is_protocol_unavailable(
                err,
                REPLICATION_FETCH_COMMIT_PROTOCOL,
            ) || crate::network_bridge::replication_network_error_is_timeout_protocol(
                err,
                REPLICATION_FETCH_COMMIT_PROTOCOL,
            );
        let checkpoint_blob_missing = reason.contains("execution checkpoint blob not found hash=")
            || (reason.contains("gap sync height ")
                && reason.contains(" blob not found for hash "));
        fetch_commit_route_error
            || checkpoint_blob_missing
            || crate::node_engine_replication_provider_route::replication_gap_sync_fetch_blob_rate_limited(err)
    }

    pub(super) fn high_replication_checkpoint_closure_missing_reason(
        err: &NodeError,
    ) -> Option<String> {
        let NodeError::Replication { reason } = err else {
            return None;
        };
        if reason.contains("execution checkpoint blob not found hash=") {
            return Some(reason.clone());
        }
        if reason.contains("gap sync height ") && reason.contains(" blob not found for hash ") {
            let content_hash = reason
                .rsplit_once(" hash ")
                .map(|(_, content_hash)| content_hash)
                .unwrap_or("unknown");
            return Some(format!(
                "execution checkpoint blob not found hash={content_hash}"
            ));
        }
        None
    }

    pub(super) fn replication_gap_sync_fetch_blob_rate_limited_in_cooldown(
        &self,
        next_height: u64,
        now_ms: i64,
    ) -> bool {
        crate::node_engine_replication_provider_route::replication_gap_sync_fetch_blob_rate_limited_in_cooldown(
            self.last_replication_gap_sync_blocked_height,
            self.last_replication_gap_sync_blocked_reason.as_deref(),
            self.last_replication_gap_sync_blocked_at_ms,
            next_height,
            now_ms,
        )
    }

    pub(super) fn record_high_replication_checkpoint_probe_failure(
        &mut self,
        next_height: u64,
        checkpoint_candidate: u64,
        now_ms: i64,
        err: &NodeError,
    ) -> bool {
        if crate::node_engine_replication_provider_route::replication_gap_sync_fetch_blob_rate_limited(err)
        {
            self.last_replication_gap_sync_blocked_height = Some(next_height);
            self.last_replication_gap_sync_blocked_at_ms = Some(now_ms);
            self.last_replication_gap_sync_blocked_reason = Some(format!(
                "replication checkpoint state sync rate limited at height {next_height}: {err}"
            ));
            return true;
        }
        self.last_replication_gap_sync_repair_attempt_height = Some(checkpoint_candidate);
        self.last_replication_gap_sync_repair_attempt_summary = Some(format!(
            "checkpoint_candidate={checkpoint_candidate} transient_error={err}"
        ));
        self.last_replication_gap_sync_repair_attempt_route_snapshot = None;
        false
    }

    pub(super) fn publish_execution_checkpoint_descriptor_providers(
        endpoint: &ReplicationNetworkEndpoint,
        world_id: &str,
        replication_runtime: &ReplicationRuntime,
        descriptor: &NodeExecutionCheckpointDescriptor,
    ) -> Result<(), NodeError> {
        let closure_refs = std::iter::once((
            descriptor.manifest_ref.as_str(),
            descriptor.manifest_size_bytes,
        ))
        .chain(
            descriptor
                .blobs
                .iter()
                .map(|blob_ref| (blob_ref.content_hash.as_str(), blob_ref.size_bytes)),
        )
        .collect::<Vec<_>>();
        for (content_hash, expected_size_bytes) in &closure_refs {
            let Some(bytes) = replication_runtime.load_blob_by_hash(content_hash)? else {
                return Err(NodeError::Replication {
                    reason: format!(
                        "checkpoint provider publication closure incomplete hash={content_hash}"
                    ),
                });
            };
            if bytes.len() as u64 != *expected_size_bytes {
                return Err(NodeError::Replication {
                    reason: format!(
                        "execution checkpoint provider publish local blob size mismatch hash={} expected={} actual={}",
                        content_hash,
                        expected_size_bytes,
                        bytes.len()
                    ),
                });
            }
        }
        for (content_hash, _) in closure_refs {
            endpoint.publish_local_content_provider_best_effort(world_id, content_hash);
        }
        Ok(())
    }
}
