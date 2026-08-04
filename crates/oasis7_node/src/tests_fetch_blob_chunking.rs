use super::*;
use crate::replication_probe_gate::should_fallback_provider_aware_replication_request;
use oasis7_proto::distributed::DistributedErrorCode;
use oasis7_proto::distributed_net::{
    FETCH_BLOB_MAX_RAW_CHUNK_BYTES, NetworkSubscription, fetch_blob_legacy_json_encoded_upper_bound,
};
use oasis7_proto::world_error::WorldError;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const REQUEST_CHUNK_BYTES: usize = FETCH_BLOB_MAX_RAW_CHUNK_BYTES / 2;

#[derive(Clone)]
struct LegacyExactChunkBlobNetwork {
    attempts: Arc<Mutex<usize>>,
}

#[derive(Clone)]
struct WorstCaseLegacyChunkBlobNetwork {
    attempts: Arc<Mutex<Vec<(u64, u64)>>>,
}

#[derive(Clone)]
struct ChunkProgressRateLimitNetwork {
    attempts: Arc<Mutex<Vec<(String, u64)>>>,
    connected_peer_serves_final_chunk: bool,
    rate_limit_active: Arc<Mutex<bool>>,
    provider_a_not_found_at_or_after: Arc<Mutex<Option<u64>>>,
    oversized_chunk_at_offset: Option<u64>,
}

#[derive(Clone)]
struct ConnectedPeerBlobFallbackNetwork {
    blob_provider_id: String,
    blob: Vec<u8>,
    generic_attempts: Arc<Mutex<usize>>,
    provider_attempts: Arc<Mutex<Vec<Vec<String>>>>,
    connected_peer_ids: Vec<String>,
    unsupported_provider_ids: Vec<String>,
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError> for LegacyExactChunkBlobNetwork {
    fn publish(&self, _topic: &str, _payload: &[u8]) -> Result<(), WorldError> {
        Ok(())
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        Ok(NetworkSubscription::new(
            topic.to_string(),
            Arc::new(Mutex::new(HashMap::new())),
        ))
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        *self.attempts.lock().expect("lock attempts") += 1;
        assert_eq!(
            protocol,
            super::replication::REPLICATION_FETCH_BLOB_PROTOCOL
        );
        let request: super::replication::FetchBlobRequest =
            serde_json::from_slice(payload).expect("decode fetch blob request");
        assert_eq!(request.offset_bytes, Some(0));
        assert_eq!(request.limit_bytes, Some(REQUEST_CHUNK_BYTES as u64));
        serde_json::to_vec(&super::replication::FetchBlobResponse {
            found: true,
            range_offset_bytes: None,
            range_complete: None,
            blob: Some(vec![u8::MAX; FETCH_BLOB_MAX_RAW_CHUNK_BYTES]),
        })
        .map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode legacy blob response failed: {err}"),
        })
    }

    fn register_handler(
        &self,
        _protocol: &str,
        _handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        Ok(())
    }
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError>
    for WorstCaseLegacyChunkBlobNetwork
{
    fn publish(&self, _topic: &str, _payload: &[u8]) -> Result<(), WorldError> {
        Ok(())
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        Ok(NetworkSubscription::new(
            topic.to_string(),
            Arc::new(Mutex::new(HashMap::new())),
        ))
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        assert_eq!(
            protocol,
            super::replication::REPLICATION_FETCH_BLOB_PROTOCOL
        );
        let request: super::replication::FetchBlobRequest =
            serde_json::from_slice(payload).expect("decode fetch blob request");
        let offset = request.offset_bytes.expect("range offset");
        let limit = request.limit_bytes.expect("range limit");
        self.attempts
            .lock()
            .expect("lock attempts")
            .push((offset, limit));
        assert_eq!(limit, REQUEST_CHUNK_BYTES as u64);
        let total = FETCH_BLOB_MAX_RAW_CHUNK_BYTES.saturating_add(17) as u64;
        let length = (total.saturating_sub(offset)).min(limit) as usize;
        let response = super::replication::FetchBlobResponse {
            found: true,
            range_offset_bytes: Some(offset),
            range_complete: Some(offset.saturating_add(length as u64) == total),
            blob: Some(vec![u8::MAX; length]),
        };
        let encoded = serde_json::to_vec(&response).expect("encode legacy response");
        assert!(
            encoded.len() <= fetch_blob_legacy_json_encoded_upper_bound(length),
            "legacy JSON response must fit its centralized budget bound"
        );
        Ok(encoded)
    }

    fn register_handler(
        &self,
        _protocol: &str,
        _handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        Ok(())
    }
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError>
    for ChunkProgressRateLimitNetwork
{
    fn publish(&self, _topic: &str, _payload: &[u8]) -> Result<(), WorldError> {
        Ok(())
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        Ok(NetworkSubscription::new(
            topic.to_string(),
            Arc::new(Mutex::new(HashMap::new())),
        ))
    }

    fn request(&self, _protocol: &str, _payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        serde_json::to_vec(&super::replication::FetchBlobResponse {
            found: false,
            range_offset_bytes: None,
            range_complete: None,
            blob: None,
        })
        .map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode generic blob miss failed: {err}"),
        })
    }

    fn connected_peer_ids(&self) -> Vec<String> {
        vec!["provider-b".to_string()]
    }

    fn request_with_providers(
        &self,
        protocol: &str,
        payload: &[u8],
        providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        assert_eq!(
            protocol,
            super::replication::REPLICATION_FETCH_BLOB_PROTOCOL
        );
        let request: super::replication::FetchBlobRequest =
            serde_json::from_slice(payload).expect("decode fetch blob request");
        let offset = request.offset_bytes.expect("range offset");
        assert_eq!(request.limit_bytes, Some(REQUEST_CHUNK_BYTES as u64));
        let provider = providers
            .first()
            .expect("single provider route")
            .to_string();
        self.attempts
            .lock()
            .expect("lock attempts")
            .push((provider.clone(), offset));

        if provider == "provider-a"
            && self
                .provider_a_not_found_at_or_after
                .lock()
                .expect("lock provider-a not-found phase")
                .is_some_and(|threshold| offset >= threshold)
        {
            return serde_json::to_vec(&super::replication::FetchBlobResponse {
                found: false,
                range_offset_bytes: None,
                range_complete: None,
                blob: None,
            })
            .map_err(|err| WorldError::DistributedValidationFailed {
                reason: format!("encode provider-a blob miss failed: {err}"),
            });
        }
        if provider == "provider-a"
            && *self
                .rate_limit_active
                .lock()
                .expect("lock rate-limit phase")
            && offset >= (REQUEST_CHUNK_BYTES * 2) as u64
        {
            return Err(WorldError::NetworkRequestFailed {
                code: DistributedErrorCode::ErrRateLimited,
                message: super::replication::REPLICATION_FETCH_BLOB_PROTOCOL.to_string(),
                retryable: true,
            });
        }
        if provider == "provider-b" && !self.connected_peer_serves_final_chunk {
            return serde_json::to_vec(&super::replication::FetchBlobResponse {
                found: false,
                range_offset_bytes: None,
                range_complete: None,
                blob: None,
            })
            .map_err(|err| WorldError::DistributedValidationFailed {
                reason: format!("encode connected blob miss failed: {err}"),
            });
        }

        let total = FETCH_BLOB_MAX_RAW_CHUNK_BYTES.saturating_add(17) as u64;
        let length = if self.oversized_chunk_at_offset == Some(offset) {
            REQUEST_CHUNK_BYTES
        } else {
            (total.saturating_sub(offset)).min(REQUEST_CHUNK_BYTES as u64) as usize
        };
        serde_json::to_vec(&super::replication::FetchBlobResponse {
            found: true,
            range_offset_bytes: Some(offset),
            range_complete: Some(offset.saturating_add(length as u64) == total),
            blob: Some(vec![u8::MAX; length]),
        })
        .map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode chunk response failed: {err}"),
        })
    }

    fn register_handler(
        &self,
        _protocol: &str,
        _handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        Ok(())
    }
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError>
    for ConnectedPeerBlobFallbackNetwork
{
    fn publish(&self, _topic: &str, _payload: &[u8]) -> Result<(), WorldError> {
        Ok(())
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        Ok(NetworkSubscription::new(
            topic.to_string(),
            Arc::new(Mutex::new(HashMap::new())),
        ))
    }

    fn request(&self, protocol: &str, _payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        *self.generic_attempts.lock().expect("lock generic attempts") += 1;
        assert_eq!(
            protocol,
            super::replication::REPLICATION_FETCH_BLOB_PROTOCOL
        );
        serde_json::to_vec(&super::replication::FetchBlobResponse {
            found: false,
            range_offset_bytes: None,
            range_complete: None,
            blob: None,
        })
        .map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode generic blob response failed: {err}"),
        })
    }

    fn connected_peer_ids(&self) -> Vec<String> {
        self.connected_peer_ids.clone()
    }

    fn request_with_providers(
        &self,
        protocol: &str,
        payload: &[u8],
        providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        self.provider_attempts
            .lock()
            .expect("lock provider attempts")
            .push(providers.to_vec());
        assert_eq!(
            protocol,
            super::replication::REPLICATION_FETCH_BLOB_PROTOCOL
        );
        let request: super::replication::FetchBlobRequest =
            serde_json::from_slice(payload).expect("decode fetch blob request");
        assert_eq!(request.offset_bytes, Some(0));
        assert_eq!(request.limit_bytes, Some(REQUEST_CHUNK_BYTES as u64));
        if providers.iter().any(|provider_id| {
            self.unsupported_provider_ids
                .iter()
                .any(|unsupported| unsupported == provider_id)
        }) {
            return Err(WorldError::NetworkRequestFailed {
                code: DistributedErrorCode::ErrUnsupported,
                message: super::replication::REPLICATION_FETCH_BLOB_PROTOCOL.to_string(),
                retryable: false,
            });
        }
        if providers
            .iter()
            .any(|provider_id| provider_id == &self.blob_provider_id)
        {
            return serde_json::to_vec(&super::replication::FetchBlobResponse {
                found: true,
                range_offset_bytes: Some(0),
                range_complete: Some(true),
                blob: Some(self.blob.clone()),
            })
            .map_err(|err| WorldError::DistributedValidationFailed {
                reason: format!("encode provider blob response failed: {err}"),
            });
        }
        serde_json::to_vec(&super::replication::FetchBlobResponse {
            found: false,
            range_offset_bytes: None,
            range_complete: None,
            blob: None,
        })
        .map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode provider blob miss failed: {err}"),
        })
    }

    fn register_handler(
        &self,
        _protocol: &str,
        _handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        Ok(())
    }
}

#[test]
fn fetch_blob_chunking_accepts_exact_chunk_legacy_full_response_without_looping() {
    let world_id = "world-legacy-exact-chunk";
    let dir = temp_dir("legacy-exact-chunk-endpoint");
    let attempts = Arc::new(Mutex::new(0usize));
    let network = Arc::new(LegacyExactChunkBlobNetwork {
        attempts: Arc::clone(&attempts),
    });
    let config = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config")
        .with_replication(signed_replication_config(dir, 41));
    let handle = NodeReplicationNetworkHandle::new(network);
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
            .expect("endpoint");
    let request = super::replication::FetchBlobRequest {
        content_hash: "legacy-exact-chunk".to_string(),
        offset_bytes: None,
        limit_bytes: None,
        requester_public_key_hex: None,
        requester_signature_hex: None,
    };

    let response = super::request_fetch_blob_with_route_fallback(
        &endpoint,
        world_id,
        "legacy-exact-chunk",
        &request,
        None,
    )
    .expect("fetch blob");

    assert!(response.found);
    assert_eq!(
        response.blob.as_ref().map(Vec::len),
        Some(FETCH_BLOB_MAX_RAW_CHUNK_BYTES)
    );
    assert_eq!(*attempts.lock().expect("lock attempts"), 1);
}

#[test]
fn fetch_blob_worst_case_legacy_json_multichunk_response_stays_within_budget() {
    let world_id = "world-worst-case-legacy-chunks";
    let dir = temp_dir("worst-case-legacy-chunks");
    let attempts = Arc::new(Mutex::new(Vec::new()));
    let network = Arc::new(WorstCaseLegacyChunkBlobNetwork {
        attempts: Arc::clone(&attempts),
    });
    let config = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config")
        .with_replication(signed_replication_config(dir, 43));
    let endpoint = ReplicationNetworkEndpoint::new(
        &NodeReplicationNetworkHandle::new(network),
        world_id,
        false,
        &config.network_policy,
    )
    .expect("endpoint");
    let response = super::request_fetch_blob_with_route_fallback(
        &endpoint,
        world_id,
        "worst-case-legacy",
        &super::replication::FetchBlobRequest {
            content_hash: "worst-case-legacy".to_string(),
            offset_bytes: None,
            limit_bytes: None,
            requester_public_key_hex: None,
            requester_signature_hex: None,
        },
        None,
    )
    .expect("chunked fetch");
    assert_eq!(
        response.blob.as_ref().map(Vec::len),
        Some(FETCH_BLOB_MAX_RAW_CHUNK_BYTES + 17)
    );
    assert_eq!(
        attempts.lock().expect("lock attempts").as_slice(),
        &[
            (0, REQUEST_CHUNK_BYTES as u64),
            (REQUEST_CHUNK_BYTES as u64, REQUEST_CHUNK_BYTES as u64),
            ((REQUEST_CHUNK_BYTES * 2) as u64, REQUEST_CHUNK_BYTES as u64),
        ]
    );
}

#[test]
fn fetch_blob_chunking_preserves_nonzero_progress_when_connected_peer_finishes_after_rate_limit() {
    let world_id = "world-chunk-progress-rate-limit-connected-recovery";
    let dir = temp_dir("chunk-progress-rate-limit-connected-recovery");
    let attempts = Arc::new(Mutex::new(Vec::new()));
    let network = Arc::new(ChunkProgressRateLimitNetwork {
        attempts: Arc::clone(&attempts),
        connected_peer_serves_final_chunk: true,
        rate_limit_active: Arc::new(Mutex::new(true)),
        provider_a_not_found_at_or_after: Arc::new(Mutex::new(None)),
        oversized_chunk_at_offset: None,
    });
    let config = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config")
        .with_replication(signed_replication_config(dir, 48));
    let endpoint = ReplicationNetworkEndpoint::new(
        &NodeReplicationNetworkHandle::new(network),
        world_id,
        false,
        &config.network_policy,
    )
    .expect("endpoint");
    let response = super::request_fetch_blob_with_route_fallback(
        &endpoint,
        world_id,
        "checkpoint-payload",
        &super::replication::FetchBlobRequest {
            content_hash: "checkpoint-payload".to_string(),
            offset_bytes: None,
            limit_bytes: None,
            requester_public_key_hex: None,
            requester_signature_hex: None,
        },
        Some(&["provider-a".to_string()]),
    )
    .expect("connected peer should finish the in-progress chunked blob");

    assert_eq!(
        response.blob.as_ref().map(Vec::len),
        Some(FETCH_BLOB_MAX_RAW_CHUNK_BYTES + 17)
    );
    assert_eq!(
        attempts.lock().expect("lock attempts").as_slice(),
        &[
            ("provider-a".to_string(), 0),
            ("provider-a".to_string(), REQUEST_CHUNK_BYTES as u64),
            ("provider-a".to_string(), (REQUEST_CHUNK_BYTES * 2) as u64),
            ("provider-b".to_string(), (REQUEST_CHUNK_BYTES * 2) as u64),
        ],
        "fallback must retain the assembled offset instead of restarting from zero",
    );
}

#[test]
fn fetch_blob_chunk_rate_limit_after_progress_wins_over_connected_not_found() {
    let world_id = "world-chunk-progress-rate-limit-error-precedence";
    let dir = temp_dir("chunk-progress-rate-limit-error-precedence");
    let attempts = Arc::new(Mutex::new(Vec::new()));
    let network = Arc::new(ChunkProgressRateLimitNetwork {
        attempts: Arc::clone(&attempts),
        connected_peer_serves_final_chunk: false,
        rate_limit_active: Arc::new(Mutex::new(true)),
        provider_a_not_found_at_or_after: Arc::new(Mutex::new(None)),
        oversized_chunk_at_offset: None,
    });
    let config = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config")
        .with_replication(signed_replication_config(dir, 49));
    let endpoint = ReplicationNetworkEndpoint::new(
        &NodeReplicationNetworkHandle::new(network),
        world_id,
        false,
        &config.network_policy,
    )
    .expect("endpoint");
    let err = super::request_fetch_blob_with_route_fallback(
        &endpoint,
        world_id,
        "checkpoint-payload",
        &super::replication::FetchBlobRequest {
            content_hash: "checkpoint-payload".to_string(),
            offset_bytes: None,
            limit_bytes: None,
            requester_public_key_hex: None,
            requester_signature_hex: None,
        },
        Some(&["provider-a".to_string()]),
    )
    .expect_err(
        "a rate limit after prior chunks must remain retryable despite connected not-found",
    );

    assert!(
        crate::network_bridge::replication_network_error_is_rate_limited_protocol(
            &err,
            super::replication::REPLICATION_FETCH_BLOB_PROTOCOL,
        ),
        "rate-limit error must retain precedence over connected not-found: {err:?}",
    );
    assert_eq!(
        attempts.lock().expect("lock attempts").as_slice(),
        &[
            ("provider-a".to_string(), 0),
            ("provider-a".to_string(), REQUEST_CHUNK_BYTES as u64),
            ("provider-a".to_string(), (REQUEST_CHUNK_BYTES * 2) as u64),
            ("provider-b".to_string(), (REQUEST_CHUNK_BYTES * 2) as u64),
        ],
        "the retryable terminal chunk must retain its nonzero offset",
    );
}

#[test]
fn fetch_blob_chunk_progress_resumes_after_rate_limit_window_without_committing_partial_blob() {
    let world_id = "world-chunk-progress-two-invocation-resume";
    let dir = temp_dir("chunk-progress-two-invocation-resume");
    let attempts = Arc::new(Mutex::new(Vec::new()));
    let rate_limit_active = Arc::new(Mutex::new(true));
    let network = Arc::new(ChunkProgressRateLimitNetwork {
        attempts: Arc::clone(&attempts),
        connected_peer_serves_final_chunk: false,
        rate_limit_active: Arc::clone(&rate_limit_active),
        provider_a_not_found_at_or_after: Arc::new(Mutex::new(None)),
        oversized_chunk_at_offset: None,
    });
    let config = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config")
        .with_replication(signed_replication_config(dir, 50));
    let endpoint = ReplicationNetworkEndpoint::new(
        &NodeReplicationNetworkHandle::new(network),
        world_id,
        false,
        &config.network_policy,
    )
    .expect("endpoint");
    let request = super::replication::FetchBlobRequest {
        content_hash: "checkpoint-payload".to_string(),
        offset_bytes: None,
        limit_bytes: None,
        requester_public_key_hex: None,
        requester_signature_hex: None,
    };
    let expected_size_bytes = FETCH_BLOB_MAX_RAW_CHUNK_BYTES + 17;
    let mut progress = super::FetchBlobChunkProgress::default();

    let first_err = super::request_fetch_blob_with_route_fallback_resuming(
        &endpoint,
        world_id,
        "checkpoint-payload",
        &request,
        Some(&["provider-a".to_string()]),
        &mut progress,
        expected_size_bytes as u64,
    )
    .expect_err("the third response-window chunk is rate-limited");
    assert!(
        crate::network_bridge::replication_network_error_is_rate_limited_protocol(
            &first_err,
            super::replication::REPLICATION_FETCH_BLOB_PROTOCOL,
        ),
        "first invocation must return structured fetch-blob rate-limit: {first_err:?}",
    );
    assert_eq!(progress.next_offset(), (REQUEST_CHUNK_BYTES * 2) as u64);
    assert_eq!(
        attempts.lock().expect("lock attempts").as_slice(),
        &[
            ("provider-a".to_string(), 0),
            ("provider-a".to_string(), REQUEST_CHUNK_BYTES as u64),
            ("provider-a".to_string(), (REQUEST_CHUNK_BYTES * 2) as u64),
            ("provider-b".to_string(), (REQUEST_CHUNK_BYTES * 2) as u64),
        ],
        "partial bytes are progress only; they are not a complete content-addressed blob",
    );

    *rate_limit_active.lock().expect("lock rate-limit phase") = false;
    let response = super::request_fetch_blob_with_route_fallback_resuming(
        &endpoint,
        world_id,
        "checkpoint-payload",
        &request,
        Some(&["provider-a".to_string()]),
        &mut progress,
        expected_size_bytes as u64,
    )
    .expect("second invocation resumes after the response-budget window reset");
    assert_eq!(
        response.blob.as_ref().map(Vec::len),
        Some(expected_size_bytes)
    );
    assert_eq!(
        progress.next_offset(),
        0,
        "complete blob clears partial progress"
    );
    assert_eq!(
        attempts.lock().expect("lock attempts").as_slice(),
        &[
            ("provider-a".to_string(), 0),
            ("provider-a".to_string(), REQUEST_CHUNK_BYTES as u64),
            ("provider-a".to_string(), (REQUEST_CHUNK_BYTES * 2) as u64),
            ("provider-b".to_string(), (REQUEST_CHUNK_BYTES * 2) as u64),
            ("provider-a".to_string(), (REQUEST_CHUNK_BYTES * 2) as u64),
        ],
        "second invocation must resume at 2MiB rather than restart at zero",
    );
}

#[test]
fn fetch_blob_chunk_progress_clears_after_non_rate_terminal_response() {
    let world_id = "world-chunk-progress-non-rate-clear";
    let dir = temp_dir("chunk-progress-non-rate-clear");
    let attempts = Arc::new(Mutex::new(Vec::new()));
    let rate_limit_active = Arc::new(Mutex::new(true));
    let provider_a_not_found_at_or_after = Arc::new(Mutex::new(None));
    let network = Arc::new(ChunkProgressRateLimitNetwork {
        attempts: Arc::clone(&attempts),
        connected_peer_serves_final_chunk: false,
        rate_limit_active: Arc::clone(&rate_limit_active),
        provider_a_not_found_at_or_after: Arc::clone(&provider_a_not_found_at_or_after),
        oversized_chunk_at_offset: None,
    });
    let config = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config")
        .with_replication(signed_replication_config(dir, 51));
    let endpoint = ReplicationNetworkEndpoint::new(
        &NodeReplicationNetworkHandle::new(network),
        world_id,
        false,
        &config.network_policy,
    )
    .expect("endpoint");
    let request = super::replication::FetchBlobRequest {
        content_hash: "checkpoint-payload".to_string(),
        offset_bytes: None,
        limit_bytes: None,
        requester_public_key_hex: None,
        requester_signature_hex: None,
    };
    let expected_size_bytes = (FETCH_BLOB_MAX_RAW_CHUNK_BYTES + 17) as u64;
    let mut progress = super::FetchBlobChunkProgress::default();

    super::request_fetch_blob_with_route_fallback_resuming(
        &endpoint,
        world_id,
        "checkpoint-payload",
        &request,
        Some(&["provider-a".to_string()]),
        &mut progress,
        expected_size_bytes,
    )
    .expect_err("first window is rate-limited after two chunks");
    assert_eq!(progress.next_offset(), (REQUEST_CHUNK_BYTES * 2) as u64);

    *rate_limit_active.lock().expect("lock rate-limit phase") = false;
    *provider_a_not_found_at_or_after
        .lock()
        .expect("lock provider-a not-found phase") = Some((REQUEST_CHUNK_BYTES * 2) as u64);
    let not_found = super::request_fetch_blob_with_route_fallback_resuming(
        &endpoint,
        world_id,
        "checkpoint-payload",
        &request,
        Some(&["provider-a".to_string()]),
        &mut progress,
        expected_size_bytes,
    )
    .expect("not-found is terminal but not an engine error");
    assert!(!not_found.found);
    assert_eq!(
        progress.next_offset(),
        0,
        "terminal non-rate response clears progress"
    );
    let attempts_before_restart = attempts.lock().expect("lock attempts").len();

    *provider_a_not_found_at_or_after
        .lock()
        .expect("lock provider-a not-found phase") = None;
    super::request_fetch_blob_with_route_fallback_resuming(
        &endpoint,
        world_id,
        "checkpoint-payload",
        &request,
        Some(&["provider-a".to_string()]),
        &mut progress,
        expected_size_bytes,
    )
    .expect("cleared progress restarts from zero and completes");
    assert_eq!(
        attempts.lock().expect("lock attempts")[attempts_before_restart],
        ("provider-a".to_string(), 0),
        "the call after terminal cleanup must restart from offset zero",
    );
    assert_eq!(progress.next_offset(), 0);
}

#[test]
fn fetch_blob_chunk_progress_overflow_clears_before_partial_blob_can_complete() {
    let world_id = "world-chunk-progress-overflow-clear";
    let dir = temp_dir("chunk-progress-overflow-clear");
    let attempts = Arc::new(Mutex::new(Vec::new()));
    let network = Arc::new(ChunkProgressRateLimitNetwork {
        attempts: Arc::clone(&attempts),
        connected_peer_serves_final_chunk: true,
        rate_limit_active: Arc::new(Mutex::new(false)),
        provider_a_not_found_at_or_after: Arc::new(Mutex::new(None)),
        oversized_chunk_at_offset: Some((REQUEST_CHUNK_BYTES * 2) as u64),
    });
    let config = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config")
        .with_replication(signed_replication_config(dir, 52));
    let endpoint = ReplicationNetworkEndpoint::new(
        &NodeReplicationNetworkHandle::new(network),
        world_id,
        false,
        &config.network_policy,
    )
    .expect("endpoint");
    let mut progress = super::FetchBlobChunkProgress::default();
    let expected_size_bytes = (FETCH_BLOB_MAX_RAW_CHUNK_BYTES + 17) as u64;
    let err = super::request_fetch_blob_with_route_fallback_resuming(
        &endpoint,
        world_id,
        "checkpoint-payload",
        &super::replication::FetchBlobRequest {
            content_hash: "checkpoint-payload".to_string(),
            offset_bytes: None,
            limit_bytes: None,
            requester_public_key_hex: None,
            requester_signature_hex: None,
        },
        Some(&["provider-a".to_string()]),
        &mut progress,
        expected_size_bytes,
    )
    .expect_err("oversized range chunk must fail before it can become a complete blob");
    assert!(
        err.to_string()
            .contains("blob fetch progress exceeds expected size"),
        "expected bounded overflow signature: {err}",
    );
    assert_eq!(
        progress.next_offset(),
        0,
        "overflow clears partial progress"
    );
    assert_eq!(
        attempts.lock().expect("lock attempts").as_slice(),
        &[
            ("provider-a".to_string(), 0),
            ("provider-a".to_string(), REQUEST_CHUNK_BYTES as u64),
            ("provider-a".to_string(), (REQUEST_CHUNK_BYTES * 2) as u64),
        ],
    );
}

#[test]
fn fetch_blob_route_fallback_tries_connected_peers_without_provider_routes() {
    let world_id = "world-blob-connected-peer-fallback";
    let dir = temp_dir("blob-connected-peer-fallback-endpoint");
    let generic_attempts = Arc::new(Mutex::new(0usize));
    let provider_attempts = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let network = Arc::new(ConnectedPeerBlobFallbackNetwork {
        blob_provider_id: "storage-peer".to_string(),
        blob: b"checkpoint-payload-from-storage".to_vec(),
        generic_attempts: Arc::clone(&generic_attempts),
        provider_attempts: Arc::clone(&provider_attempts),
        connected_peer_ids: vec![
            "sequencer-peer".to_string(),
            "storage-peer".to_string(),
            "sequencer-peer".to_string(),
        ],
        unsupported_provider_ids: Vec::new(),
    });
    let config = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config")
        .with_replication(signed_replication_config(dir, 42));
    let handle = NodeReplicationNetworkHandle::new(network);
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
            .expect("endpoint");
    let request = super::replication::FetchBlobRequest {
        content_hash: "checkpoint-payload".to_string(),
        offset_bytes: None,
        limit_bytes: None,
        requester_public_key_hex: None,
        requester_signature_hex: None,
    };

    let response = super::request_fetch_blob_with_route_fallback(
        &endpoint,
        world_id,
        "checkpoint-payload",
        &request,
        None,
    )
    .expect("fetch blob");

    assert!(response.found);
    assert_eq!(
        response.blob.as_deref(),
        Some(b"checkpoint-payload-from-storage".as_slice())
    );
    assert_eq!(
        *generic_attempts.lock().expect("lock generic attempts"),
        1,
        "expected one generic miss before connected-peer fallback"
    );
    assert_eq!(
        provider_attempts
            .lock()
            .expect("lock provider attempts")
            .as_slice(),
        &[
            vec!["sequencer-peer".to_string()],
            vec!["storage-peer".to_string()],
        ],
        "expected generic miss, then each connected peer until storage returns the blob"
    );
}

#[test]
fn fetch_blob_provider_route_miss_does_not_probe_generic_or_connected_peers() {
    let world_id = "world-blob-provider-miss-strict";
    let dir = temp_dir("blob-provider-miss-strict-endpoint");
    let generic_attempts = Arc::new(Mutex::new(0usize));
    let provider_attempts = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let network = Arc::new(ConnectedPeerBlobFallbackNetwork {
        blob_provider_id: "storage-peer".to_string(),
        blob: b"checkpoint-payload-from-storage".to_vec(),
        generic_attempts: Arc::clone(&generic_attempts),
        provider_attempts: Arc::clone(&provider_attempts),
        connected_peer_ids: vec![
            "observer-light-peer".to_string(),
            "storage-peer".to_string(),
        ],
        unsupported_provider_ids: Vec::new(),
    });
    let config = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config")
        .with_replication(signed_replication_config(dir, 44));
    let handle = NodeReplicationNetworkHandle::new(network);
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
            .expect("endpoint");
    let request = super::replication::FetchBlobRequest {
        content_hash: "checkpoint-payload".to_string(),
        offset_bytes: None,
        limit_bytes: None,
        requester_public_key_hex: None,
        requester_signature_hex: None,
    };

    let response = super::request_fetch_blob_with_storage_challenge_routes(
        &endpoint,
        world_id,
        "checkpoint-payload",
        &request,
        Some(&["stale-provider".to_string()]),
    )
    .expect("provider miss should return not-found");

    assert!(!response.found);
    assert_eq!(
        *generic_attempts.lock().expect("lock generic attempts"),
        0,
        "provider-aware blob fetch should not spend budget on generic non-provider routing"
    );
    assert_eq!(
        provider_attempts
            .lock()
            .expect("lock provider attempts")
            .as_slice(),
        &[vec!["stale-provider".to_string()]],
        "provider-aware blob fetch should not probe arbitrary connected observer/storage peers after a provider miss"
    );
}

#[test]
fn fetch_blob_provider_route_unavailable_falls_back_to_generic_route() {
    let world_id = "world-blob-provider-unavailable-strict";
    let dir = temp_dir("blob-provider-unavailable-strict-endpoint");
    let generic_attempts = Arc::new(Mutex::new(0usize));
    let provider_attempts = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let network = Arc::new(ConnectedPeerBlobFallbackNetwork {
        blob_provider_id: "storage-peer".to_string(),
        blob: b"checkpoint-payload-from-storage".to_vec(),
        generic_attempts: Arc::clone(&generic_attempts),
        provider_attempts: Arc::clone(&provider_attempts),
        connected_peer_ids: vec![
            "observer-light-peer".to_string(),
            "storage-peer".to_string(),
        ],
        unsupported_provider_ids: vec!["stale-provider".to_string()],
    });
    let config = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config")
        .with_replication(signed_replication_config(dir, 45));
    let handle = NodeReplicationNetworkHandle::new(network);
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
            .expect("endpoint");
    let request = super::replication::FetchBlobRequest {
        content_hash: "checkpoint-payload".to_string(),
        offset_bytes: None,
        limit_bytes: None,
        requester_public_key_hex: None,
        requester_signature_hex: None,
    };

    let response = super::request_fetch_blob_with_storage_challenge_routes(
        &endpoint,
        world_id,
        "checkpoint-payload",
        &request,
        Some(&["stale-provider".to_string()]),
    )
    .expect("retryable provider route unavailability should fall back");

    assert!(
        !response.found,
        "generic fallback should be attempted but still surface not-found when no fallback peer has the blob"
    );
    assert!(
        *generic_attempts.lock().expect("lock generic attempts") > 0,
        "provider-aware blob fetch should spend bounded generic attempts after retryable provider route unavailability"
    );
    assert_eq!(
        provider_attempts
            .lock()
            .expect("lock provider attempts")
            .as_slice(),
        &[vec!["stale-provider".to_string()]],
        "provider-aware blob fetch should try the stale provider before bounded generic fallback"
    );
}

#[test]
fn fetch_blob_storage_challenge_without_provider_lookup_does_not_probe_generic_route() {
    let world_id = "world-blob-provider-lookup-missing";
    let dir = temp_dir("blob-provider-lookup-missing-endpoint");
    let generic_attempts = Arc::new(Mutex::new(0usize));
    let provider_attempts = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let network = Arc::new(ConnectedPeerBlobFallbackNetwork {
        blob_provider_id: "storage-peer".to_string(),
        blob: b"checkpoint-payload-from-storage".to_vec(),
        generic_attempts: Arc::clone(&generic_attempts),
        provider_attempts: Arc::clone(&provider_attempts),
        connected_peer_ids: vec!["storage-peer".to_string()],
        unsupported_provider_ids: Vec::new(),
    });
    let config = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config")
        .with_replication(signed_replication_config(dir, 46));
    let handle = NodeReplicationNetworkHandle::new(network);
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
            .expect("endpoint");
    let request = super::replication::FetchBlobRequest {
        content_hash: "checkpoint-payload".to_string(),
        offset_bytes: None,
        limit_bytes: None,
        requester_public_key_hex: None,
        requester_signature_hex: None,
    };

    let err = super::request_fetch_blob_with_storage_challenge_routes(
        &endpoint,
        world_id,
        "checkpoint-payload",
        &request,
        None,
    )
    .expect_err("storage challenge fallback requires a retryable provider route failure");

    assert!(
        should_fallback_provider_aware_replication_request(&err),
        "missing provider lookup should remain a retryable/degraded storage challenge condition: {err:?}"
    );
    assert_eq!(
        *generic_attempts.lock().expect("lock generic attempts"),
        0,
        "storage challenge should not probe generic lane when provider lookup is unavailable"
    );
    assert!(
        provider_attempts
            .lock()
            .expect("lock provider attempts")
            .is_empty(),
        "storage challenge should not invent provider attempts without a provider lookup"
    );
}

#[test]
fn fetch_blob_storage_challenge_empty_provider_routes_probe_bounded_generic_route() {
    let world_id = "world-blob-provider-empty-routes";
    let dir = temp_dir("blob-provider-empty-routes-endpoint");
    let generic_attempts = Arc::new(Mutex::new(0usize));
    let provider_attempts = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let network = Arc::new(ConnectedPeerBlobFallbackNetwork {
        blob_provider_id: "storage-peer".to_string(),
        blob: b"checkpoint-payload-from-storage".to_vec(),
        generic_attempts: Arc::clone(&generic_attempts),
        provider_attempts: Arc::clone(&provider_attempts),
        connected_peer_ids: vec!["storage-peer".to_string()],
        unsupported_provider_ids: Vec::new(),
    });
    let config = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config")
        .with_replication(signed_replication_config(dir, 47));
    let handle = NodeReplicationNetworkHandle::new(network);
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
            .expect("endpoint");
    let request = super::replication::FetchBlobRequest {
        content_hash: "checkpoint-payload".to_string(),
        offset_bytes: None,
        limit_bytes: None,
        requester_public_key_hex: None,
        requester_signature_hex: None,
    };

    let provider_ids: Vec<String> = Vec::new();
    let response = super::request_fetch_blob_with_storage_challenge_routes(
        &endpoint,
        world_id,
        "checkpoint-payload",
        &request,
        Some(provider_ids.as_slice()),
    )
    .expect("empty DHT provider lookup should allow bounded generic recovery");

    assert!(
        !response.found,
        "generic fallback miss should remain observable as blob-not-found"
    );
    assert_eq!(
        *generic_attempts.lock().expect("lock generic attempts"),
        3,
        "storage challenge should spend only the bounded generic attempts when DHT has no non-local providers"
    );
    assert!(
        provider_attempts
            .lock()
            .expect("lock provider attempts")
            .is_empty(),
        "storage challenge should not try provider routes when DHT returns no non-local providers"
    );
}

#[test]
fn fetch_blob_route_fallback_skips_unsupported_connected_peers() {
    let world_id = "world-blob-connected-peer-unsupported-fallback";
    let dir = temp_dir("blob-connected-peer-unsupported-fallback-endpoint");
    let generic_attempts = Arc::new(Mutex::new(0usize));
    let provider_attempts = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let network = Arc::new(ConnectedPeerBlobFallbackNetwork {
        blob_provider_id: "storage-peer".to_string(),
        blob: b"checkpoint-payload-from-storage".to_vec(),
        generic_attempts: Arc::clone(&generic_attempts),
        provider_attempts: Arc::clone(&provider_attempts),
        connected_peer_ids: vec!["legacy-peer".to_string(), "storage-peer".to_string()],
        unsupported_provider_ids: vec!["legacy-peer".to_string()],
    });
    let config = NodeConfig::new("node-b", world_id, NodeRole::Observer)
        .expect("config")
        .with_replication(signed_replication_config(dir, 43));
    let handle = NodeReplicationNetworkHandle::new(network);
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, world_id, false, &config.network_policy)
            .expect("endpoint");
    let request = super::replication::FetchBlobRequest {
        content_hash: "checkpoint-payload".to_string(),
        offset_bytes: None,
        limit_bytes: None,
        requester_public_key_hex: None,
        requester_signature_hex: None,
    };

    let response = super::request_fetch_blob_with_route_fallback(
        &endpoint,
        world_id,
        "checkpoint-payload",
        &request,
        None,
    )
    .expect("fetch blob");

    assert!(response.found);
    assert_eq!(
        response.blob.as_deref(),
        Some(b"checkpoint-payload-from-storage".as_slice())
    );
    assert_eq!(
        provider_attempts
            .lock()
            .expect("lock provider attempts")
            .as_slice(),
        &[
            vec!["legacy-peer".to_string()],
            vec!["storage-peer".to_string()],
        ],
        "expected unsupported legacy peer to be skipped before trying storage"
    );
}
