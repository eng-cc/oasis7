use super::*;
use crate::node_engine_gap_sync_outcome::GapSyncHeightOutcome;

// The provider retains the stable v1 2 MiB admission limit. Request one half
// of that maximum so its legacy JSON byte-array response, when carried in the
// outer libp2p CBOR envelope, remains below the codec's 10 MiB response cap.
const REPLICATION_FETCH_BLOB_CHUNK_BYTES: usize =
    oasis7_proto::distributed_net::FETCH_BLOB_MAX_RAW_CHUNK_BYTES / 2;
const STORAGE_CHALLENGE_FETCH_BLOB_REQUEST_TIMEOUT_MS: u64 = 2_000;
const STORAGE_CHALLENGE_FETCH_BLOB_RETRY_BUDGET_MS: u64 = 3_000;

#[derive(Clone, Copy)]
struct FetchBlobRouteFallbackPolicy {
    allow_generic_route: bool,
    allow_connected_peer_fallback: bool,
    require_retryable_provider_route_before_fallback: bool,
}

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
        if Self::replication_gap_sync_local_state_blocked(
            self.last_replication_gap_sync_blocked_height,
            self.last_replication_gap_sync_blocked_reason.as_deref(),
            probe_height,
        ) {
            return Ok(true);
        }
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
            Ok(GapSyncHeightOutcome::Synced {
                message, payload, ..
            }) => {
                self.last_replication_successor_probe_height = None;
                self.last_replication_successor_probe_at_ms = None;
                self.last_replication_successor_probe_hold = None;
                let execution_result = with_execution_hook(&mut execution_hook, |hook| {
                    self.execute_synced_replication_commit(world_id, &payload, hook)
                });
                let (block_hash, committed_at_ms) = match execution_result {
                    Ok(result) => result,
                    Err(err) => {
                        self.record_replication_gap_sync_local_state_block(
                            probe_height,
                            self.network_committed_height.max(probe_height),
                            probe_height,
                            err.to_string(),
                        );
                        return Err(err);
                    }
                };
                if let Err(err) = self.persist_synced_replication_message(
                    endpoint,
                    node_id,
                    world_id,
                    replication_runtime,
                    &message,
                    probe_height,
                ) {
                    self.record_replication_gap_sync_local_state_block(
                        probe_height,
                        self.network_committed_height.max(probe_height),
                        probe_height,
                        err.to_string(),
                    );
                    return Err(err);
                }
                self.replication_persisted_height =
                    self.replication_persisted_height.max(probe_height);
                if let Err(err) =
                    self.record_synced_replication_height(probe_height, block_hash, committed_at_ms)
                {
                    self.record_replication_gap_sync_local_state_block(
                        probe_height,
                        self.network_committed_height.max(probe_height),
                        probe_height,
                        err.to_string(),
                    );
                    return Err(err);
                }
                Ok(true)
            }
            Ok(GapSyncHeightOutcome::NotFound { .. }) => {
                self.note_replication_successor_probe_attempt(probe_height, now_ms, false);
                Ok(false)
            }
            Err(err) if replication_request_waitable_connection_gap(&err) => {
                let hold_proposals = !(self.committed_height == 0
                    && self.replication_persisted_height == 0
                    && self.peer_heads.is_empty());
                self.note_replication_successor_probe_attempt(probe_height, now_ms, hold_proposals);
                Ok(hold_proposals)
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
    crate::network_bridge::replication_network_error_is_availability_gap(err)
        || crate::network_bridge::replication_network_error_is_route_unavailable(err)
        || crate::network_bridge::replication_network_error_is_rate_limited_protocol(
            err,
            super::replication::REPLICATION_FETCH_BLOB_PROTOCOL,
        )
        || reason.starts_with("blob fetch routes exhausted without response")
        || reason.starts_with("blob fetch provider routes exhausted without response")
        || crate::network_bridge::replication_network_error_is_unsupported_protocol(
            err,
            super::replication::REPLICATION_FETCH_BLOB_PROTOCOL,
        )
}

pub(super) fn replication_request_waitable_connection_gap(err: &NodeError) -> bool {
    crate::network_bridge::replication_network_error_is_availability_gap(err)
        || crate::network_bridge::replication_network_error_is_timeout_protocol(
            err,
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
        )
}

fn replication_successor_probe_fetch_commit_unavailable(err: &NodeError) -> bool {
    (crate::network_bridge::replication_network_error_is_route_unavailable(err)
        && crate::network_bridge::replication_network_error_mentions_protocol(
            err,
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
        ))
        || crate::network_bridge::replication_network_error_is_unsupported_protocol(
            err,
            super::replication::REPLICATION_FETCH_COMMIT_PROTOCOL,
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIBP2P_CBOR_DEFAULT_MAX_RESPONSE_BYTES: usize = 10 * 1024 * 1024;

    #[test]
    fn requested_fetch_blob_chunk_fits_libp2p_outer_cbor_response_limit() {
        let inner_json = serde_json::to_vec(&FetchBlobResponse {
            found: true,
            range_offset_bytes: Some(0),
            range_complete: Some(false),
            blob: Some(vec![u8::MAX; REPLICATION_FETCH_BLOB_CHUNK_BYTES]),
        })
        .expect("encode worst-case legacy fetch-blob response");
        let outer_cbor = serde_cbor::to_vec(&oasis7_proto::distributed_net::NetworkResponse {
            payload: inner_json,
        })
        .expect("encode libp2p response envelope");

        assert!(
            outer_cbor.len() <= LIBP2P_CBOR_DEFAULT_MAX_RESPONSE_BYTES,
            "requester chunk must fit the libp2p CBOR response cap after legacy JSON and outer envelope encoding: chunk_bytes={} encoded_bytes={} cap_bytes={}",
            REPLICATION_FETCH_BLOB_CHUNK_BYTES,
            outer_cbor.len(),
            LIBP2P_CBOR_DEFAULT_MAX_RESPONSE_BYTES,
        );
    }

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
    fn provider_aware_fallback_treats_provider_route_exhaustion_as_retryable() {
        let err = NodeError::Replication {
            reason: "blob fetch provider routes exhausted without response for world_id=w hash=abc"
                .to_string(),
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
    fn provider_aware_fallback_treats_fetch_blob_unsupported_as_retryable_without_debug_text() {
        let err = NodeError::Replication {
            reason: "replication network request failed: kind=unsupported protocol=/aw/node/replication/fetch-blob/1.0.0 detail=/aw/node/replication/fetch-blob/1.0.0 unsupported by remote"
                .to_string(),
        };

        assert!(
            should_fallback_provider_aware_replication_request(&err),
            "classification should use structured kind/protocol data, not the WorldError Debug text spelling"
        );
        assert!(!replication_request_waitable_connection_gap(&err));
    }

    #[test]
    fn provider_aware_fallback_does_not_hide_business_unsupported() {
        let err = NodeError::Replication {
            reason: "replication network request failed: kind=unsupported protocol=/aw/node/replication/fetch-blob/1.0.0 detail=remote peer declined request"
                .to_string(),
        };

        assert!(!should_fallback_provider_aware_replication_request(&err));
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

    #[test]
    fn successor_probe_treats_fetch_commit_unsupported_as_unavailable_without_debug_text() {
        let err = NodeError::Replication {
            reason: "replication network request failed: kind=unsupported protocol=/aw/node/replication/fetch-commit/1.0.0 detail=/aw/node/replication/fetch-commit/1.0.0 unsupported by remote"
                .to_string(),
        };

        assert!(
            replication_successor_probe_fetch_commit_unavailable(&err),
            "classification should survive display wording changes as long as kind/protocol are preserved"
        );
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
        FetchBlobRouteFallbackPolicy {
            allow_generic_route: true,
            allow_connected_peer_fallback: true,
            require_retryable_provider_route_before_fallback: false,
        },
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
        FetchBlobRouteFallbackPolicy {
            allow_generic_route: true,
            allow_connected_peer_fallback: false,
            require_retryable_provider_route_before_fallback: true,
        },
    )
}

fn request_fetch_blob_with_route_fallback_policy(
    endpoint: &ReplicationNetworkEndpoint,
    world_id: &str,
    content_hash: &str,
    request: &FetchBlobRequest,
    provider_ids: Option<&[String]>,
    policy: FetchBlobRouteFallbackPolicy,
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
            policy,
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
    policy: FetchBlobRouteFallbackPolicy,
) -> Result<FetchBlobResponse, NodeError> {
    let mut last_not_found: Option<FetchBlobResponse> = None;
    let mut last_retryable_error: Option<NodeError> = None;
    let mut attempted_provider_ids = std::collections::BTreeSet::new();
    let provider_lookup_supplied = provider_ids.is_some();
    let mut provider_route_attempted = false;

    if let Some(provider_ids) = provider_ids {
        for provider_id in provider_ids {
            let provider_id = provider_id.trim();
            if provider_id.is_empty() || !attempted_provider_ids.insert(provider_id.to_string()) {
                continue;
            }
            provider_route_attempted = true;
            let provider_route = [provider_id.to_string()];
            match endpoint
                .request_json_with_providers_budget::<FetchBlobRequest, FetchBlobResponse>(
                    REPLICATION_FETCH_BLOB_PROTOCOL,
                    request,
                    provider_route.as_slice(),
                    STORAGE_CHALLENGE_FETCH_BLOB_REQUEST_TIMEOUT_MS,
                    STORAGE_CHALLENGE_FETCH_BLOB_RETRY_BUDGET_MS,
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

    if provider_lookup_supplied && !policy.allow_connected_peer_fallback {
        if let Some(response) = last_not_found {
            return Ok(response);
        }
    }

    if policy.require_retryable_provider_route_before_fallback
        && last_retryable_error.is_none()
        && (provider_route_attempted || !provider_lookup_supplied)
    {
        return Err(NodeError::Replication {
            reason: format!(
                "blob fetch provider routes exhausted without response for world_id={} hash={} before retryable provider failure",
                world_id, content_hash
            ),
        });
    }

    if provider_lookup_supplied
        && !policy.allow_connected_peer_fallback
        && last_retryable_error.is_none()
        && provider_route_attempted
    {
        return Err(NodeError::Replication {
            reason: format!(
                "blob fetch provider routes exhausted without response for world_id={} hash={}",
                world_id, content_hash
            ),
        });
    }

    if provider_lookup_supplied
        && !policy.allow_connected_peer_fallback
        && !policy.require_retryable_provider_route_before_fallback
    {
        return Err(
            last_retryable_error.unwrap_or_else(|| NodeError::Replication {
                reason: format!(
                    "blob fetch provider routes exhausted without response for world_id={} hash={}",
                    world_id, content_hash
                ),
            }),
        );
    }

    // Provider throttling is an authoritative back-pressure signal. Do not
    // evade it through a connected or generic route after every advertised
    // provider has been exhausted; let the caller enter its bounded cooldown.
    if last_retryable_error
        .as_ref()
        .map(|err| {
            crate::network_bridge::replication_network_error_is_rate_limited_protocol(
                err,
                REPLICATION_FETCH_BLOB_PROTOCOL,
            )
        })
        .unwrap_or(false)
        && last_not_found.is_none()
    {
        return Err(last_retryable_error.expect("rate-limited error checked above"));
    }

    let mut generic_attempts = 0usize;
    // A generic request is the discovery fallback when provider lookup yielded no
    // concrete route. The storage-challenge policy deliberately uses generic as
    // its bounded fallback after a retryable provider failure. Normal checkpoint
    // fetches instead try distinct connected peers and must not silently route
    // back to the same incomplete advertised provider.
    let allow_generic_route = policy.allow_generic_route
        && (!provider_route_attempted || !policy.allow_connected_peer_fallback);
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

    if policy.allow_connected_peer_fallback {
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
