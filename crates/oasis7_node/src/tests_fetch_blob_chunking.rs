use super::*;
use oasis7_proto::distributed_net::NetworkSubscription;
use oasis7_proto::world_error::WorldError;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct LegacyExactChunkBlobNetwork {
    attempts: Arc<Mutex<usize>>,
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
