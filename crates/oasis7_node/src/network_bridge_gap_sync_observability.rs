use std::time::Instant;

use crate::network_bridge_gap_sync_budget::{short_node_error, summarize_fetch_commit_routes};
use crate::replication::{FetchCommitRequest, FetchCommitResponse};
use crate::{NodeError, NodeReplicationGapSyncRouteSnapshot};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct FetchCommitSuccessCacheKey {
    pub(super) world_id: String,
    pub(super) height: u64,
    pub(super) requester_public_key_hex: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct CachedFetchCommitSuccess {
    pub(super) response: FetchCommitResponse,
    pub(super) cached_at: Instant,
    pub(super) valid_until: Instant,
}

pub(crate) struct GapSyncFetchCommitResponse {
    pub response: FetchCommitResponse,
    pub repair_summary: String,
    pub route_snapshot: NodeReplicationGapSyncRouteSnapshot,
}

pub(super) fn fetch_commit_success_cache_key(
    request: &FetchCommitRequest,
) -> FetchCommitSuccessCacheKey {
    FetchCommitSuccessCacheKey {
        world_id: request.world_id.clone(),
        height: request.height,
        requester_public_key_hex: request.requester_public_key_hex.clone(),
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum GapSyncFetchCommitRouteKind {
    Generic,
    Provider,
    GenericRetry,
}

pub(super) struct GapSyncFetchCommitRouteObserver {
    started_at: Instant,
    snapshot: NodeReplicationGapSyncRouteSnapshot,
}

impl GapSyncFetchCommitRouteObserver {
    pub(super) fn new(started_at: Instant) -> Self {
        Self {
            started_at,
            snapshot: NodeReplicationGapSyncRouteSnapshot::default(),
        }
    }

    pub(super) fn observe_found(&mut self, kind: GapSyncFetchCommitRouteKind, found: bool) {
        self.observe_attempt(kind);
        if found {
            self.snapshot.synced_route_count += 1;
        } else {
            self.snapshot.not_found_route_count += 1;
        }
    }

    pub(super) fn observe_error(&mut self, kind: GapSyncFetchCommitRouteKind, err: &NodeError) {
        self.observe_attempt(kind);
        self.snapshot.error_route_count += 1;
        self.snapshot.last_slow_route_reason = Some(short_node_error(err));
    }

    pub(super) fn observe_budget_exhausted(&mut self, err: &NodeError) {
        self.snapshot.budget_exhausted_count += 1;
        self.snapshot.last_slow_route_reason = Some(short_node_error(err));
    }

    pub(super) fn finish(mut self) -> NodeReplicationGapSyncRouteSnapshot {
        self.snapshot.elapsed_ms = self.started_at.elapsed().as_millis() as u64;
        self.snapshot
    }

    fn observe_attempt(&mut self, kind: GapSyncFetchCommitRouteKind) {
        self.snapshot.route_attempt_count += 1;
        match kind {
            GapSyncFetchCommitRouteKind::Generic => self.snapshot.generic_route_count += 1,
            GapSyncFetchCommitRouteKind::Provider => self.snapshot.provider_route_count += 1,
            GapSyncFetchCommitRouteKind::GenericRetry => {
                self.snapshot.generic_retry_route_count += 1
            }
        }
    }
}

pub(super) fn summarize_gap_sync_fetch_commit_routes(
    route_events: &[String],
    snapshot: &NodeReplicationGapSyncRouteSnapshot,
) -> String {
    let route_summary = summarize_fetch_commit_routes(route_events);
    let slow_reason = snapshot.last_slow_route_reason.as_deref().unwrap_or("none");
    format!(
        "{route_summary};elapsed_ms={};routes_attempted={};routes_synced={};routes_not_found={};routes_error={};generic_routes={};provider_routes={};generic_retry_routes={};budget_exhausted={};last_slow_route_reason={}",
        snapshot.elapsed_ms,
        snapshot.route_attempt_count,
        snapshot.synced_route_count,
        snapshot.not_found_route_count,
        snapshot.error_route_count,
        snapshot.generic_route_count,
        snapshot.provider_route_count,
        snapshot.generic_retry_route_count,
        snapshot.budget_exhausted_count,
        slow_reason
    )
}

pub(super) fn split_provider_route_timeout_ms(
    retry_budget_ms: u64,
    remaining_provider_routes: usize,
    min_timeout_ms: u64,
) -> u64 {
    let remaining_provider_routes = remaining_provider_routes.max(1) as u64;
    let divided = retry_budget_ms / remaining_provider_routes;
    divided.max(min_timeout_ms).min(retry_budget_ms)
}
