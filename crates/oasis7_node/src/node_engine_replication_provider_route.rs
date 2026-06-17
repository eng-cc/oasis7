use crate::NodeError;

pub(super) const REPLICATION_GAP_SYNC_PROVIDER_ROUTE_BLOCK_RETRY_COOLDOWN_MS: i64 = 30_000;

pub(super) fn replication_gap_sync_provider_blob_route_blocked(err: &NodeError) -> bool {
    let NodeError::Replication { reason } = err else {
        return false;
    };
    replication_gap_sync_provider_blob_route_blocked_reason(reason)
}

pub(super) fn replication_gap_sync_provider_blob_route_blocked_reason(reason: &str) -> bool {
    reason.contains("gap sync height ")
        && (reason.contains(" blob not found for hash ")
            || reason.contains("blob fetch provider routes exhausted")
            || reason.contains(crate::network_bridge::REPLICATION_NETWORK_ROUTE_UNAVAILABLE_PREFIX))
}

pub(super) fn replication_gap_sync_provider_blob_route_blocked_in_cooldown(
    blocked_height: Option<u64>,
    blocked_reason: Option<&str>,
    blocked_at_ms: Option<i64>,
    next_height: u64,
    now_ms: i64,
) -> bool {
    if blocked_height != Some(next_height)
        || !blocked_reason
            .map(replication_gap_sync_provider_blob_route_blocked_reason)
            .unwrap_or(false)
    {
        return false;
    }
    let Some(blocked_at_ms) = blocked_at_ms else {
        return false;
    };
    now_ms.saturating_sub(blocked_at_ms)
        < REPLICATION_GAP_SYNC_PROVIDER_ROUTE_BLOCK_RETRY_COOLDOWN_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_route_blocked_cooldown_expires_for_retry() {
        let reason = "replication gap sync provider route blocked: gap sync height 1 blob not found for hash abc";
        let blocked_at_ms = 1_000;

        assert!(
            replication_gap_sync_provider_blob_route_blocked_in_cooldown(
                Some(1),
                Some(reason),
                Some(blocked_at_ms),
                1,
                blocked_at_ms + REPLICATION_GAP_SYNC_PROVIDER_ROUTE_BLOCK_RETRY_COOLDOWN_MS - 1
            )
        );
        assert!(
            !replication_gap_sync_provider_blob_route_blocked_in_cooldown(
                Some(1),
                Some(reason),
                Some(blocked_at_ms),
                1,
                blocked_at_ms + REPLICATION_GAP_SYNC_PROVIDER_ROUTE_BLOCK_RETRY_COOLDOWN_MS
            )
        );
        assert!(
            !replication_gap_sync_provider_blob_route_blocked_in_cooldown(
                Some(1),
                Some(reason),
                None,
                1,
                blocked_at_ms
            )
        );
    }
}
