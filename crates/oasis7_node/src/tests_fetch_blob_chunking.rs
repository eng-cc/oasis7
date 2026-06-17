use super::*;
use oasis7_proto::distributed::DistributedErrorCode;
use oasis7_proto::distributed_net::NetworkSubscription;
use oasis7_proto::world_error::WorldError;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct LegacyExactChunkBlobNetwork {
    attempts: Arc<Mutex<usize>>,
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
        assert_eq!(request.limit_bytes, Some(2 * 1024 * 1024));
        serde_json::to_vec(&super::replication::FetchBlobResponse {
            found: true,
            range_offset_bytes: None,
            range_complete: None,
            blob: Some(vec![7; 2 * 1024 * 1024]),
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
        assert_eq!(request.limit_bytes, Some(2 * 1024 * 1024));
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
    assert_eq!(response.blob.as_ref().map(Vec::len), Some(2 * 1024 * 1024));
    assert_eq!(*attempts.lock().expect("lock attempts"), 1);
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
fn fetch_blob_provider_route_unavailable_does_not_probe_generic_or_connected_peers() {
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

    let result = super::request_fetch_blob_with_storage_challenge_routes(
        &endpoint,
        world_id,
        "checkpoint-payload",
        &request,
        Some(&["stale-provider".to_string()]),
    );

    assert!(
        result.is_err(),
        "provider-aware blob fetch should surface provider route unavailability without probing fallback peers: {result:?}"
    );
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
        "provider-aware blob fetch should not probe arbitrary connected peers after provider route unavailability"
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
