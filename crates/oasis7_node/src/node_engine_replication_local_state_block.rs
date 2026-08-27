use super::*;

impl PosNodeEngine {
    pub(super) fn record_replication_gap_sync_repair_attempt(
        &mut self,
        height: u64,
        repair_summary: String,
        route_snapshot: NodeReplicationGapSyncRouteSnapshot,
    ) {
        self.last_replication_gap_sync_repair_attempt_height = Some(height);
        self.last_replication_gap_sync_repair_attempt_summary = Some(repair_summary);
        self.last_replication_gap_sync_repair_attempt_route_snapshot = Some(route_snapshot);
    }

    pub(super) fn clear_replication_gap_sync_blocked_if_unblocked(&mut self) {
        if self
            .last_replication_gap_sync_blocked_height
            .map(|height| self.replication_persisted_height >= height)
            .unwrap_or(false)
        {
            self.last_replication_gap_sync_blocked_height = None;
            self.last_replication_gap_sync_blocked_reason = None;
            self.last_replication_gap_sync_blocked_at_ms = None;
            self.last_replication_gap_sync_repair_attempt_height = None;
            self.last_replication_gap_sync_repair_attempt_summary = None;
            self.last_replication_gap_sync_repair_attempt_route_snapshot = None;
        }
    }

    pub(super) fn advance_contiguous_replication_persisted_height(
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
            if self.replication_persisted_height == 0 {
                self.refresh_replication_persisted_height_from_local_execution_baseline();
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

    fn refresh_replication_persisted_height_from_local_execution_baseline(&mut self) {
        if !self.require_execution_on_commit
            || self.committed_height == 0
            || self.last_execution_height < self.committed_height
            || self.last_committed_block_hash.is_none()
            || self.last_execution_block_hash.is_none()
            || self.last_execution_state_root.is_none()
        {
            return;
        }
        self.replication_persisted_height = self.committed_height;
        self.clear_replication_gap_sync_blocked_if_unblocked();
    }

    pub(super) fn replication_gap_sync_local_state_blocked_reason(reason: &str) -> bool {
        // A rendered rollback-unavailable diagnostic is not a checkpoint
        // authority signal.  The fresh-observer fallback consumes the
        // structured NodeError variant directly; keeping this generic state
        // marker from recognizing the legacy phrase prevents callers from
        // turning arbitrary Display text into recovery authority.
        if reason.contains("rollback record for height") {
            return false;
        }
        reason.contains("execution hash validation failed")
            || reason.contains("execution driver peer mismatch")
            || reason.contains("forced execution failure")
            || reason.contains("rollback to height")
            || reason.contains("No space left on device")
    }

    pub(super) fn replication_gap_sync_local_state_blocked(
        blocked_height: Option<u64>,
        blocked_reason: Option<&str>,
        next_height: u64,
    ) -> bool {
        blocked_height == Some(next_height)
            && blocked_reason
                .map(Self::replication_gap_sync_local_state_blocked_reason)
                .unwrap_or(false)
    }

    pub(super) fn record_replication_gap_sync_local_state_block(
        &mut self,
        height: u64,
        advertised_network_height: u64,
        gap_sync_target_height: u64,
        reason: String,
    ) {
        self.last_replication_gap_sync_blocked_height = Some(height);
        self.last_replication_gap_sync_blocked_at_ms = None;
        self.last_replication_gap_sync_blocked_reason = Some(format!(
            "replication gap sync local state blocked: deterministic execution/state failure at height {height} while advertised_network_height={} network_committed_height={} gap_sync_target_height={} replication_persisted_height={}: {}",
            advertised_network_height,
            self.network_committed_height,
            gap_sync_target_height,
            self.replication_persisted_height,
            reason
        ));
        self.last_replication_gap_sync_repair_attempt_height = Some(height);
        self.last_replication_gap_sync_repair_attempt_summary = Some(reason);
        self.last_replication_gap_sync_repair_attempt_route_snapshot = None;
    }
}
