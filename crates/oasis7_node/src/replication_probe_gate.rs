use super::*;

impl PosNodeEngine {
    pub(super) fn maybe_hold_proposal_for_replication_successor_probe(
        &mut self,
        endpoint: &ReplicationNetworkEndpoint,
        node_id: &str,
        world_id: &str,
        now_ms: i64,
        mut replication: Option<&mut ReplicationRuntime>,
        mut execution_hook: Option<&mut dyn NodeExecutionHook>,
    ) -> Result<bool, NodeError> {
        let Some(replication_runtime) = replication.as_deref_mut() else {
            return Ok(false);
        };
        if !self.peer_heads.is_empty() {
            return Ok(false);
        }

        self.refresh_replication_persisted_height(replication_runtime, world_id)?;
        let probe_from_height = self.replication_persisted_height.max(self.committed_height);
        let probe_height = checked_replication_successor(
            probe_from_height,
            "probe_from_height",
            "probing replication successor commit",
        )?;
        if let Some(last_hold_decision) =
            self.replication_successor_probe_cooldown_decision(probe_height, now_ms)
        {
            return Ok(last_hold_decision);
        }

        match self.sync_replication_height_once(
            endpoint,
            node_id,
            world_id,
            replication_runtime,
            probe_height,
        ) {
            Ok(GapSyncHeightOutcome::Synced { message, payload }) => {
                self.last_replication_successor_probe_height = None;
                self.last_replication_successor_probe_at_ms = None;
                self.last_replication_successor_probe_hold = None;
                let (block_hash, committed_at_ms) =
                    with_execution_hook(&mut execution_hook, |hook| {
                        self.execute_synced_replication_commit(world_id, &payload, hook)
                    })?;
                self.persist_synced_replication_message(
                    endpoint,
                    node_id,
                    world_id,
                    replication_runtime,
                    &message,
                    probe_height,
                )?;
                self.replication_persisted_height =
                    self.replication_persisted_height.max(probe_height);
                self.record_synced_replication_height(probe_height, block_hash, committed_at_ms)?;
                Ok(true)
            }
            Ok(GapSyncHeightOutcome::NotFound) => {
                self.note_replication_successor_probe_attempt(probe_height, now_ms, false);
                Ok(false)
            }
            Err(err) if replication_request_waitable_connection_gap(&err) => {
                self.note_replication_successor_probe_attempt(probe_height, now_ms, true);
                Ok(true)
            }
            Err(err) => Err(err),
        }
    }

    fn replication_successor_probe_cooldown_decision(
        &self,
        probe_height: u64,
        now_ms: i64,
    ) -> Option<bool> {
        match (
            self.last_replication_successor_probe_height,
            self.last_replication_successor_probe_at_ms,
            self.last_replication_successor_probe_hold,
        ) {
            (Some(last_height), Some(last_at_ms), Some(last_hold_decision))
                if last_height == probe_height
                    && now_ms.saturating_sub(last_at_ms)
                        < REPLICATION_SUCCESSOR_PROBE_COOLDOWN_MS =>
            {
                Some(last_hold_decision)
            }
            _ => None,
        }
    }

    fn note_replication_successor_probe_attempt(
        &mut self,
        probe_height: u64,
        now_ms: i64,
        hold_proposals: bool,
    ) {
        self.last_replication_successor_probe_height = Some(probe_height);
        self.last_replication_successor_probe_at_ms = Some(now_ms);
        self.last_replication_successor_probe_hold = Some(hold_proposals);
    }
}

fn should_fallback_provider_aware_replication_request(err: &NodeError) -> bool {
    let NodeError::Replication { reason } = err else {
        return false;
    };
    reason.contains("NetworkProtocolUnavailable")
        || reason.contains("libp2p-replication no admissible connected peers for protocol")
        || reason.contains("libp2p-replication no connected providers for protocol")
        || reason.contains("libp2p-replication no connected peers for protocol")
        || (reason.contains("NetworkRequestFailed")
            && reason.contains("NetworkProtocolUnavailable"))
}

pub(super) fn replication_request_waitable_connection_gap(err: &NodeError) -> bool {
    let NodeError::Replication { reason } = err else {
        return false;
    };
    reason.contains("no admissible connected peers for protocol")
        || reason.contains("no connected peers for protocol")
        || reason.contains("no connected providers for protocol")
        || reason.contains("no healthy provider for protocol")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_aware_fallback_treats_no_admissible_peers_as_retryable() {
        let err = NodeError::Replication {
            reason:
                "NetworkProtocolUnavailable { protocol: \"libp2p-replication no admissible connected peers for protocol /aw/node/replication/fetch-blob/1.0.0\" }"
                    .to_string(),
        };
        assert!(should_fallback_provider_aware_replication_request(&err));
        assert!(replication_request_waitable_connection_gap(&err));
    }
}

pub(super) fn request_fetch_blob_with_route_fallback(
    endpoint: &ReplicationNetworkEndpoint,
    world_id: &str,
    content_hash: &str,
    request: &FetchBlobRequest,
    provider_ids: Option<&[String]>,
) -> Result<FetchBlobResponse, NodeError> {
    let mut last_not_found: Option<FetchBlobResponse> = None;
    let mut last_retryable_error: Option<NodeError> = None;

    if let Some(provider_ids) = provider_ids {
        for provider_id in provider_ids {
            let provider_route = [provider_id.clone()];
            match endpoint.request_json_with_providers::<FetchBlobRequest, FetchBlobResponse>(
                REPLICATION_FETCH_BLOB_PROTOCOL,
                request,
                provider_route.as_slice(),
            ) {
                Ok(response) => {
                    if response.found {
                        return Ok(response);
                    }
                    last_not_found = Some(response);
                }
                Err(err) if should_fallback_provider_aware_replication_request(&err) => {
                    last_retryable_error = Some(err);
                }
                Err(err) => return Err(err),
            }
        }
    }

    for _ in 0..REPLICATION_FETCH_BLOB_GENERIC_ROUTE_ATTEMPTS {
        match endpoint.request_json::<FetchBlobRequest, FetchBlobResponse>(
            REPLICATION_FETCH_BLOB_PROTOCOL,
            request,
        ) {
            Ok(response) => {
                if response.found {
                    return Ok(response);
                }
                last_not_found = Some(response);
            }
            Err(err) if should_fallback_provider_aware_replication_request(&err) => {
                last_retryable_error = Some(err);
            }
            Err(err) => return Err(err),
        }
    }

    if let Some(response) = last_not_found {
        return Ok(response);
    }

    Err(
        last_retryable_error.unwrap_or_else(|| NodeError::Replication {
            reason: format!(
                "blob fetch routes exhausted without response for world_id={} hash={}",
                world_id, content_hash
            ),
        }),
    )
}
