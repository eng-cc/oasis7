use crate::NodeError;
use crate::replication::{FetchHeadRequest, FetchHeadResponse, REPLICATION_GET_HEAD_PROTOCOL};
use oasis7_proto::distributed::WorldHeadAnnounce;

use super::{
    GAP_SYNC_FETCH_HEAD_MAX_PROVIDER_ROUTES_PER_POLL, GAP_SYNC_FETCH_HEAD_REQUEST_TIMEOUT_MS,
    GAP_SYNC_FETCH_HEAD_RETRY_BUDGET_MS, ReplicationNetworkEndpoint,
    world_head_lookup_can_fallback,
};

impl ReplicationNetworkEndpoint {
    pub(super) fn request_world_head_from_peers(
        &self,
        world_id: &str,
    ) -> Result<Option<WorldHeadAnnounce>, NodeError> {
        let request = FetchHeadRequest {
            world_id: world_id.to_string(),
        };
        let mut connected_peer_ids = self.network.connected_peer_ids();
        connected_peer_ids.sort();
        connected_peer_ids.dedup();
        let mut peer_ids = connected_peer_ids.clone();
        let mut static_bootstrap_peer_ids = self.network.configured_static_bootstrap_peer_ids();
        static_bootstrap_peer_ids.sort();
        static_bootstrap_peer_ids.dedup();
        static_bootstrap_peer_ids
            .retain(|peer_id| !peer_id.trim().is_empty() && !connected_peer_ids.contains(peer_id));
        peer_ids.extend(static_bootstrap_peer_ids);
        let mut candidate_peer_ids = self.network.known_peer_ids();
        candidate_peer_ids.sort();
        candidate_peer_ids.dedup();
        candidate_peer_ids.retain(|peer_id| !peer_ids.contains(peer_id));
        peer_ids.extend(candidate_peer_ids);
        peer_ids.retain(|peer_id| !peer_id.trim().is_empty());
        let mut best_head = None;
        if peer_ids.is_empty() {
            self.maybe_update_best_peer_head(
                world_id,
                &mut best_head,
                self.request_json_budget(
                    REPLICATION_GET_HEAD_PROTOCOL,
                    &request,
                    GAP_SYNC_FETCH_HEAD_REQUEST_TIMEOUT_MS,
                    GAP_SYNC_FETCH_HEAD_RETRY_BUDGET_MS,
                ),
            )?;
        } else {
            for peer_id in peer_ids
                .into_iter()
                .take(GAP_SYNC_FETCH_HEAD_MAX_PROVIDER_ROUTES_PER_POLL)
            {
                self.maybe_update_best_peer_head(
                    world_id,
                    &mut best_head,
                    self.request_json_with_providers_budget(
                        REPLICATION_GET_HEAD_PROTOCOL,
                        &request,
                        std::slice::from_ref(&peer_id),
                        GAP_SYNC_FETCH_HEAD_REQUEST_TIMEOUT_MS,
                        GAP_SYNC_FETCH_HEAD_RETRY_BUDGET_MS,
                    ),
                )?;
            }
        }
        Ok(best_head)
    }

    pub(super) fn maybe_update_best_peer_head(
        &self,
        world_id: &str,
        best_head: &mut Option<WorldHeadAnnounce>,
        response: Result<FetchHeadResponse, NodeError>,
    ) -> Result<(), NodeError> {
        match response {
            Ok(FetchHeadResponse {
                found: true,
                head: Some(head),
            }) => {
                if head.world_id != world_id {
                    return Err(NodeError::Replication {
                        reason: format!(
                            "replication peer head mismatch: expected={} actual={}",
                            world_id, head.world_id
                        ),
                    });
                }
                let candidate = WorldHeadAnnounce {
                    world_id: head.world_id,
                    height: head.height,
                    block_hash: head.block_hash,
                    state_root: head.state_root,
                    timestamp_ms: head.timestamp_ms,
                    signature: String::new(),
                };
                if best_head
                    .as_ref()
                    .map(|current| candidate.height > current.height)
                    .unwrap_or(true)
                {
                    *best_head = Some(candidate);
                }
                Ok(())
            }
            Ok(_) => Ok(()),
            Err(err) if world_head_lookup_can_fallback(&err) => Ok(()),
            Err(err) => Err(err),
        }
    }
}
