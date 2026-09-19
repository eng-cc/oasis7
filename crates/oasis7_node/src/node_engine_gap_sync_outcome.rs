use crate::{NodeReplicationGapSyncRouteSnapshot, replication, replication_state_reconcile};

#[derive(Debug, Clone)]
#[expect(
    clippy::large_enum_variant,
    reason = "Internal gap-sync state-machine outcome retains the complete validated synced payload for immediate installation"
)]
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
