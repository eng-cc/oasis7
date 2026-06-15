use super::*;

const REPLICATION_FETCH_BLOB_CHUNK_BYTES: usize = 2 * 1024 * 1024;

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

        match self.sync_replication_height_once_for_successor_probe(
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
            Ok(GapSyncHeightOutcome::NotFound { .. }) => {
                self.note_replication_successor_probe_attempt(probe_height, now_ms, false);
                Ok(false)
            }
            Err(err) if replication_request_waitable_connection_gap(&err) => {
                self.note_replication_successor_probe_attempt(probe_height, now_ms, true);
                Ok(true)
            }
            Err(err) if replication_successor_probe_fetch_commit_unavailable(&err) => {
                self.note_replication_successor_probe_attempt(probe_height, now_ms, false);
                Ok(false)
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

pub(super) fn should_fallback_provider_aware_replication_request(err: &NodeError) -> bool {
    let NodeError::Replication { reason } = err else {
        return false;
    };
    reason.starts_with(crate::network_bridge::REPLICATION_NETWORK_AVAILABILITY_GAP_PREFIX)
        || reason.starts_with(crate::network_bridge::REPLICATION_NETWORK_ROUTE_UNAVAILABLE_PREFIX)
        || reason.starts_with("blob fetch routes exhausted without response")
        || (reason.contains("ErrUnsupported")
            && reason.contains(super::replication::REPLICATION_FETCH_BLOB_PROTOCOL))
}

pub(super) fn replication_request_waitable_connection_gap(err: &NodeError) -> bool {
    let NodeError::Replication { reason } = err else {
        return false;
    };
    reason.starts_with(crate::network_bridge::REPLICATION_NETWORK_AVAILABILITY_GAP_PREFIX)
}

fn replication_successor_probe_fetch_commit_unavailable(err: &NodeError) -> bool {
    let NodeError::Replication { reason } = err else {
        return false;
    };
    reason.contains(super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL)
        && (reason.starts_with(crate::network_bridge::REPLICATION_NETWORK_ROUTE_UNAVAILABLE_PREFIX)
            || reason.contains("ErrUnsupported"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_aware_fallback_treats_no_admissible_peers_as_retryable() {
        let err = NodeError::Replication {
            reason: format!(
                "{}libp2p-replication no admissible connected peers for protocol /aw/node/replication/fetch-blob/1.0.0",
                crate::network_bridge::REPLICATION_NETWORK_AVAILABILITY_GAP_PREFIX
            ),
        };
        assert!(should_fallback_provider_aware_replication_request(&err));
        assert!(replication_request_waitable_connection_gap(&err));
    }

    #[test]
    fn provider_aware_fallback_treats_route_unavailable_as_retryable() {
        let err = NodeError::Replication {
            reason: format!(
                "{}simulated provider route unavailable",
                crate::network_bridge::REPLICATION_NETWORK_ROUTE_UNAVAILABLE_PREFIX
            ),
        };
        assert!(should_fallback_provider_aware_replication_request(&err));
        assert!(!replication_request_waitable_connection_gap(&err));
    }

    #[test]
    fn provider_aware_fallback_treats_fetch_blob_unsupported_as_retryable() {
        let err = NodeError::Replication {
            reason: "replication network error: NetworkRequestFailed { code: ErrUnsupported, message: \"/aw/node/replication/fetch-blob/1.0.0\", retryable: false }"
                .to_string(),
        };
        assert!(should_fallback_provider_aware_replication_request(&err));
        assert!(!replication_request_waitable_connection_gap(&err));
    }

    #[test]
    fn successor_probe_treats_fetch_commit_unsupported_as_unavailable() {
        let err = NodeError::Replication {
            reason: "replication network error: NetworkRequestFailed { code: ErrUnsupported, message: \"/aw/node/replication/fetch-commit/1.0.0\", retryable: false }"
                .to_string(),
        };
        assert!(replication_successor_probe_fetch_commit_unavailable(&err));
        assert!(!replication_request_waitable_connection_gap(&err));
    }
}

pub(super) fn request_fetch_blob_with_route_fallback(
    endpoint: &ReplicationNetworkEndpoint,
    world_id: &str,
    content_hash: &str,
    request: &FetchBlobRequest,
    provider_ids: Option<&[String]>,
) -> Result<FetchBlobResponse, NodeError> {
    request_fetch_blob_with_route_fallback_policy(
        endpoint,
        world_id,
        content_hash,
        request,
        provider_ids,
        true,
        true,
    )
}

pub(super) fn request_fetch_blob_with_storage_challenge_routes(
    endpoint: &ReplicationNetworkEndpoint,
    world_id: &str,
    content_hash: &str,
    request: &FetchBlobRequest,
    provider_ids: Option<&[String]>,
) -> Result<FetchBlobResponse, NodeError> {
    request_fetch_blob_with_route_fallback_policy(
        endpoint,
        world_id,
        content_hash,
        request,
        provider_ids,
        false,
        false,
    )
}

fn request_fetch_blob_with_route_fallback_policy(
    endpoint: &ReplicationNetworkEndpoint,
    world_id: &str,
    content_hash: &str,
    request: &FetchBlobRequest,
    provider_ids: Option<&[String]>,
    allow_generic_route: bool,
    allow_connected_peer_fallback: bool,
) -> Result<FetchBlobResponse, NodeError> {
    let mut offset = 0usize;
    let mut assembled = Vec::new();

    loop {
        let mut chunk_request = request.clone();
        chunk_request.offset_bytes = Some(offset as u64);
        chunk_request.limit_bytes = Some(REPLICATION_FETCH_BLOB_CHUNK_BYTES as u64);
        let response = request_fetch_blob_chunk_with_route_fallback(
            endpoint,
            world_id,
            content_hash,
            &chunk_request,
            provider_ids,
            allow_generic_route,
            allow_connected_peer_fallback,
        )?;
        if !response.found {
            return Ok(response);
        }
        let range_aware = response.range_offset_bytes == Some(offset as u64);
        let Some(chunk) = response.blob else {
            return Ok(response);
        };
        if !range_aware {
            return Ok(FetchBlobResponse {
                found: true,
                range_offset_bytes: response.range_offset_bytes,
                range_complete: response.range_complete,
                blob: Some(chunk),
            });
        }
        if chunk.is_empty() {
            return Ok(FetchBlobResponse {
                found: true,
                range_offset_bytes: None,
                range_complete: None,
                blob: Some(assembled),
            });
        }
        let is_final_chunk = response
            .range_complete
            .unwrap_or(chunk.len() < REPLICATION_FETCH_BLOB_CHUNK_BYTES);
        offset = offset.saturating_add(chunk.len());
        assembled.extend_from_slice(chunk.as_slice());
        if is_final_chunk {
            return Ok(FetchBlobResponse {
                found: true,
                range_offset_bytes: None,
                range_complete: None,
                blob: Some(assembled),
            });
        }
    }
}

fn request_fetch_blob_chunk_with_route_fallback(
    endpoint: &ReplicationNetworkEndpoint,
    world_id: &str,
    content_hash: &str,
    request: &FetchBlobRequest,
    provider_ids: Option<&[String]>,
    allow_generic_route: bool,
    allow_connected_peer_fallback: bool,
) -> Result<FetchBlobResponse, NodeError> {
    let mut last_not_found: Option<FetchBlobResponse> = None;
    let mut last_retryable_error: Option<NodeError> = None;
    let mut attempted_provider_ids = std::collections::BTreeSet::new();

    if let Some(provider_ids) = provider_ids {
        for provider_id in provider_ids {
            let provider_id = provider_id.trim();
            if provider_id.is_empty() || !attempted_provider_ids.insert(provider_id.to_string()) {
                continue;
            }
            let provider_route = [provider_id.to_string()];
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

    let mut generic_attempts = 0usize;
    if allow_generic_route && generic_attempts < REPLICATION_FETCH_BLOB_GENERIC_ROUTE_ATTEMPTS {
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
        generic_attempts += 1;
    }

    if allow_connected_peer_fallback {
        let mut connected_peer_ids = endpoint.connected_peer_ids();
        connected_peer_ids.sort();
        connected_peer_ids.dedup();
        for peer_id in connected_peer_ids {
            let peer_id = peer_id.trim();
            if peer_id.is_empty() || !attempted_provider_ids.insert(peer_id.to_string()) {
                continue;
            }
            let provider_route = [peer_id.to_string()];
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

    while allow_generic_route && generic_attempts < REPLICATION_FETCH_BLOB_GENERIC_ROUTE_ATTEMPTS {
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
        generic_attempts += 1;
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
