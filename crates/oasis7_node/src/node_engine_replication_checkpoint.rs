use super::*;

impl PosNodeEngine {
    // Mirrors release_default.execution_checkpoint_keep. Probe the advertised head first, then
    // the older retained-window boundaries. The newest aligned boundary can still be in-flight
    // or unretained on live testnet nodes, so retained-window probing starts one interval back.
    const HIGH_REPLICATION_CHECKPOINT_LOOKBACK_WINDOWS: u64 = 8;

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
            let first_lookback = if aligned.saturating_sub(interval) > blocked_height
                && aligned != advertised_network_height
            {
                1
            } else {
                0
            };
            for lookback in first_lookback..=Self::HIGH_REPLICATION_CHECKPOINT_LOOKBACK_WINDOWS {
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
        fetch_commit_route_error || checkpoint_blob_missing
    }
}
