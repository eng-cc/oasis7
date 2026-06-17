use super::*;

pub(super) struct ProviderLookupFailureDht;

impl proto_dht::DistributedDht<WorldError> for ProviderLookupFailureDht {
    fn publish_provider(
        &self,
        _world_id: &str,
        _content_hash: &str,
        _provider_id: &str,
    ) -> Result<(), WorldError> {
        Ok(())
    }

    fn get_providers(
        &self,
        _world_id: &str,
        _content_hash: &str,
    ) -> Result<Vec<ProviderRecord>, WorldError> {
        Err(WorldError::NetworkRequestFailed {
            code: oasis7_proto::distributed::DistributedErrorCode::ErrTimeout,
            message: "simulated provider lookup timeout".to_string(),
            retryable: true,
        })
    }

    fn put_world_head(
        &self,
        _world_id: &str,
        _head: &WorldHeadAnnounce,
    ) -> Result<(), WorldError> {
        Ok(())
    }

    fn get_world_head(&self, _world_id: &str) -> Result<Option<WorldHeadAnnounce>, WorldError> {
        Ok(None)
    }

    fn put_membership_directory(
        &self,
        _world_id: &str,
        _snapshot: &MembershipDirectorySnapshot,
    ) -> Result<(), WorldError> {
        Ok(())
    }

    fn get_membership_directory(
        &self,
        _world_id: &str,
    ) -> Result<Option<MembershipDirectorySnapshot>, WorldError> {
        Ok(None)
    }

    fn put_peer_record(
        &self,
        _world_id: &str,
        _record: &SignedPeerRecord,
    ) -> Result<(), WorldError> {
        Ok(())
    }

    fn get_peer_record(
        &self,
        _world_id: &str,
        _peer_id: &str,
    ) -> Result<Option<SignedPeerRecord>, WorldError> {
        Ok(None)
    }
}

pub(super) struct EmptyProviderLookupDht;

impl proto_dht::DistributedDht<WorldError> for EmptyProviderLookupDht {
    fn publish_provider(
        &self,
        _world_id: &str,
        _content_hash: &str,
        _provider_id: &str,
    ) -> Result<(), WorldError> {
        Ok(())
    }

    fn get_providers(
        &self,
        _world_id: &str,
        _content_hash: &str,
    ) -> Result<Vec<ProviderRecord>, WorldError> {
        Ok(Vec::new())
    }

    fn put_world_head(
        &self,
        _world_id: &str,
        _head: &WorldHeadAnnounce,
    ) -> Result<(), WorldError> {
        Ok(())
    }

    fn get_world_head(&self, _world_id: &str) -> Result<Option<WorldHeadAnnounce>, WorldError> {
        Ok(None)
    }

    fn put_membership_directory(
        &self,
        _world_id: &str,
        _snapshot: &MembershipDirectorySnapshot,
    ) -> Result<(), WorldError> {
        Ok(())
    }

    fn get_membership_directory(
        &self,
        _world_id: &str,
    ) -> Result<Option<MembershipDirectorySnapshot>, WorldError> {
        Ok(None)
    }

    fn put_peer_record(
        &self,
        _world_id: &str,
        _record: &SignedPeerRecord,
    ) -> Result<(), WorldError> {
        Ok(())
    }

    fn get_peer_record(
        &self,
        _world_id: &str,
        _peer_id: &str,
    ) -> Result<Option<SignedPeerRecord>, WorldError> {
        Ok(None)
    }
}

#[derive(Clone, Default)]
pub(super) struct ProviderLookupFailureGenericBlobNetwork {
    blobs: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    generic_attempts: Arc<Mutex<usize>>,
}

impl ProviderLookupFailureGenericBlobNetwork {
    pub(super) fn new(blobs: HashMap<String, Vec<u8>>) -> Self {
        Self {
            blobs: Arc::new(Mutex::new(blobs)),
            generic_attempts: Arc::new(Mutex::new(0)),
        }
    }

    pub(super) fn generic_attempts(&self) -> usize {
        *self.generic_attempts.lock().expect("lock generic attempts")
    }

    fn fetch_blob_response(&self, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        let request =
            serde_json::from_slice::<crate::replication::FetchBlobRequest>(payload).map_err(
                |err| WorldError::DistributedValidationFailed {
                    reason: format!("decode fetch blob request failed: {err}"),
                },
            )?;
        let blob = self
            .blobs
            .lock()
            .expect("lock blobs")
            .get(request.content_hash.as_str())
            .cloned();
        let response = crate::replication::FetchBlobResponse {
            found: blob.is_some(),
            range_offset_bytes: None,
            range_complete: None,
            blob,
        };
        serde_json::to_vec(&response).map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode fetch blob response failed: {err}"),
        })
    }
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError>
    for ProviderLookupFailureGenericBlobNetwork
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

    fn request(&self, _protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        *self.generic_attempts.lock().expect("lock generic attempts") += 1;
        self.fetch_blob_response(payload)
    }

    fn connected_peer_ids(&self) -> Vec<String> {
        vec!["observer-fallback-peer".to_string()]
    }

    fn request_with_providers(
        &self,
        _protocol: &str,
        payload: &[u8],
        _providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        self.fetch_blob_response(payload)
    }

    fn register_handler(
        &self,
        _protocol: &str,
        _handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        Ok(())
    }
}

#[derive(Clone, Default)]
pub(super) struct ProviderLookupFailureConnectedPeerTrapNetwork {
    connected_peer_provider_attempts: Arc<Mutex<usize>>,
}

impl ProviderLookupFailureConnectedPeerTrapNetwork {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn connected_peer_provider_attempts(&self) -> usize {
        *self
            .connected_peer_provider_attempts
            .lock()
            .expect("lock connected peer provider attempts")
    }
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError>
    for ProviderLookupFailureConnectedPeerTrapNetwork
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
        Err(WorldError::NetworkProtocolUnavailable {
            protocol: protocol.to_string(),
        })
    }

    fn connected_peer_ids(&self) -> Vec<String> {
        vec!["observer-light-peer".to_string()]
    }

    fn request_with_providers(
        &self,
        protocol: &str,
        _payload: &[u8],
        providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        if providers
            .iter()
            .any(|provider_id| provider_id == "observer-light-peer")
        {
            *self
                .connected_peer_provider_attempts
                .lock()
                .expect("lock connected peer provider attempts") += 1;
            return serde_json::to_vec(&crate::replication::FetchBlobResponse {
                found: true,
                range_offset_bytes: None,
                range_complete: None,
                blob: Some(b"observer-light-wrong-blob".to_vec()),
            })
            .map_err(|err| WorldError::DistributedValidationFailed {
                reason: format!("encode trap blob response failed: {err}"),
            });
        }
        Err(WorldError::NetworkProtocolUnavailable {
            protocol: protocol.to_string(),
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

#[derive(Clone, Default)]
pub(super) struct ProviderRouteFailureGenericTrapNetwork {
    generic_attempts: Arc<Mutex<usize>>,
}

impl ProviderRouteFailureGenericTrapNetwork {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn generic_attempts(&self) -> usize {
        *self.generic_attempts.lock().expect("lock generic attempts")
    }
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError>
    for ProviderRouteFailureGenericTrapNetwork
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
        *self.generic_attempts.lock().expect("lock generic attempts") += 1;
        serde_json::to_vec(&crate::replication::FetchBlobResponse {
            found: true,
            range_offset_bytes: None,
            range_complete: None,
            blob: Some(b"generic-route-wrong-blob".to_vec()),
        })
        .map_err(|err| WorldError::DistributedValidationFailed {
            reason: format!("encode generic trap blob response failed: {err}"),
        })
    }

    fn connected_peer_ids(&self) -> Vec<String> {
        vec!["observer-light-peer".to_string()]
    }

    fn request_with_providers(
        &self,
        protocol: &str,
        _payload: &[u8],
        _providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        Err(WorldError::NetworkProtocolUnavailable {
            protocol: protocol.to_string(),
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

pub(super) struct MalformedProviderResponseNetwork;

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError>
    for MalformedProviderResponseNetwork
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
        Err(WorldError::NetworkProtocolUnavailable {
            protocol: protocol.to_string(),
        })
    }

    fn connected_peer_ids(&self) -> Vec<String> {
        vec!["storage-provider-1".to_string()]
    }

    fn request_with_providers(
        &self,
        _protocol: &str,
        _payload: &[u8],
        _providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        Ok(b"not-json".to_vec())
    }

    fn register_handler(
        &self,
        _protocol: &str,
        _handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        Ok(())
    }
}
