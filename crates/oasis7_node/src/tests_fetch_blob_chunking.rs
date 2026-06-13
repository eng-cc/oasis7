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
fn fetch_blob_route_fallback_tries_connected_peers_after_not_found_routes() {
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
        Some(&["sequencer-peer".to_string()]),
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
        "expected stale provider miss, then each connected peer until storage returns the blob"
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
