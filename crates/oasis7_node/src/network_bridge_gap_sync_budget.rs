use std::time::{Duration, Instant};

use crate::NodeError;

#[cfg(all(feature = "libp2p", not(target_arch = "wasm32")))]
use crate::libp2p_replication_network::{
    FETCH_COMMIT_REQUEST_RETRY_BUDGET_MS, FETCH_COMMIT_REQUEST_TO_PEER_TIMEOUT_MS,
};
#[cfg(all(feature = "libp2p", target_arch = "wasm32"))]
use crate::libp2p_replication_network_wasm::{
    FETCH_COMMIT_REQUEST_RETRY_BUDGET_MS, FETCH_COMMIT_REQUEST_TO_PEER_TIMEOUT_MS,
};
#[cfg(not(feature = "libp2p"))]
const FETCH_COMMIT_REQUEST_RETRY_BUDGET_MS: u64 = 120_000;
#[cfg(not(feature = "libp2p"))]
const FETCH_COMMIT_REQUEST_TO_PEER_TIMEOUT_MS: u64 = 30_000;

pub(super) fn gap_sync_fetch_commit_route_budget(started_at: Instant) -> Option<(u64, u64)> {
    gap_sync_fetch_commit_route_budget_after(
        started_at.elapsed(),
        FETCH_COMMIT_REQUEST_RETRY_BUDGET_MS,
        FETCH_COMMIT_REQUEST_TO_PEER_TIMEOUT_MS,
        FETCH_COMMIT_REQUEST_TO_PEER_TIMEOUT_MS,
    )
}

pub(super) fn gap_sync_fetch_commit_probe_route_budget(started_at: Instant) -> Option<(u64, u64)> {
    gap_sync_fetch_commit_route_budget_after(started_at.elapsed(), 3_000, 1_500, 3_000)
}

fn gap_sync_fetch_commit_route_budget_after(
    elapsed: Duration,
    sweep_budget_ms: u64,
    request_timeout_ms: u64,
    route_budget_cap_ms: u64,
) -> Option<(u64, u64)> {
    let elapsed_ms = elapsed.as_millis();
    let sweep_budget_ms = sweep_budget_ms as u128;
    if elapsed_ms >= sweep_budget_ms {
        return None;
    }
    let remaining_budget_ms = (sweep_budget_ms - elapsed_ms) as u64;
    let route_budget_ms = remaining_budget_ms.min(route_budget_cap_ms);
    Some((request_timeout_ms.min(route_budget_ms), route_budget_ms))
}

pub(super) fn gap_sync_fetch_commit_route_budget_exhausted() -> NodeError {
    NodeError::Replication {
        reason: format!(
            "gap-sync fetch-commit request budget exhausted budget_ms={}",
            FETCH_COMMIT_REQUEST_RETRY_BUDGET_MS
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gap_sync_fetch_commit_route_budget_is_shared_across_route_sweep() {
        assert_eq!(
            gap_sync_fetch_commit_route_budget_after(
                Duration::from_millis(0),
                120_000,
                30_000,
                30_000
            ),
            Some((30_000, 30_000))
        );
        assert_eq!(
            gap_sync_fetch_commit_route_budget_after(
                Duration::from_millis(110_000),
                120_000,
                30_000,
                30_000,
            ),
            Some((10_000, 10_000))
        );
        assert_eq!(
            gap_sync_fetch_commit_route_budget_after(
                Duration::from_millis(120_000),
                120_000,
                30_000,
                30_000,
            ),
            None
        );
    }

    #[test]
    fn gap_sync_fetch_commit_probe_budget_stays_short() {
        assert_eq!(
            gap_sync_fetch_commit_route_budget_after(Duration::from_millis(0), 3_000, 1_500, 3_000),
            Some((1_500, 3_000))
        );
    }
}
