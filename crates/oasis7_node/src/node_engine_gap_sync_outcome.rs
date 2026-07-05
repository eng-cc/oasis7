use crate::{NodeReplicationGapSyncRouteSnapshot, replication, replication_state_reconcile};

#[derive(Debug, Clone)]
pub(super) enum GapSyncHeightOutcome {
    Synced {
        message: replication::GossipReplicationMessage,
        payload: replication_state_reconcile::ReplicationCommitPayload,
        repair_summary: String,
        route_snapshot: NodeReplicationGapSyncRouteSnapshot,
    },
    NotFound {
        repair_summary: String,
        route_snapshot: NodeReplicationGapSyncRouteSnapshot,
    },
}
