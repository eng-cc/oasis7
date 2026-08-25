use super::*;

impl PosNodeEngine {
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
