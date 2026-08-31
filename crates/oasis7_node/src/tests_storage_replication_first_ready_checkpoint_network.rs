#[derive(Clone)]
pub(super) struct FirstReadyHeadCheckpointNetwork {
    pub(super) inner: Arc<TestInMemoryNetwork>,
    pub(super) fetch_protocols: Arc<Mutex<Vec<String>>>,
}

#[derive(Clone)]
struct PeerHeadCheckpointNetwork {
    inner: Arc<TestInMemoryNetwork>,
    fetch_protocols: Arc<Mutex<Vec<String>>>,
    head: Arc<Mutex<super::replication::FetchHeadResponse>>,
    checkpoint_fetch_available: Arc<AtomicBool>,
    checkpoint_fetch_not_found: Arc<AtomicBool>,
    connected_peer_ids: Vec<String>,
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError>
    for FirstReadyHeadCheckpointNetwork
{
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        self.inner.publish(topic, payload)
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        self.inner.subscribe(topic)
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        if protocol == REPLICATION_GET_HEAD_PROTOCOL {
            return serde_json::to_vec(&super::replication::FetchHeadResponse {
                found: false,
                head: None,
            })
            .map_err(|err| WorldError::DistributedValidationFailed {
                reason: format!("encode absent world head response failed: {err}"),
            });
        }
        self.fetch_protocols
            .lock()
            .expect("lock checkpoint fetch protocols")
            .push(protocol.to_string());
        self.inner.request(protocol, payload)
    }

    fn connected_peer_ids(&self) -> Vec<String> {
        vec!["node-a".to_string()]
    }

    fn known_peer_ids(&self) -> Vec<String> {
        vec!["node-a".to_string()]
    }

    fn register_handler(
        &self,
        protocol: &str,
        handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        self.inner.register_handler(protocol, handler)
    }
}

impl oasis7_proto::distributed_net::DistributedNetwork<WorldError> for PeerHeadCheckpointNetwork {
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        self.inner.publish(topic, payload)
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        self.inner.subscribe(topic)
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        if protocol == REPLICATION_GET_HEAD_PROTOCOL {
            let head = self.head.lock().expect("lock peer checkpoint head").clone();
            return serde_json::to_vec(&head).map_err(|err| {
                WorldError::DistributedValidationFailed {
                    reason: format!("encode peer checkpoint head failed: {err}"),
                }
            });
        }
        if protocol == REPLICATION_FETCH_COMMIT_PROTOCOL
            && !self.checkpoint_fetch_available.load(Ordering::SeqCst)
        {
            if !self.checkpoint_fetch_not_found.load(Ordering::SeqCst) {
                return Err(WorldError::NetworkProtocolUnavailable {
                    protocol: protocol.to_string(),
                });
            }
            let request = serde_json::from_slice::<super::replication::FetchCommitRequest>(payload)
                .map_err(|err| WorldError::DistributedValidationFailed {
                    reason: format!("decode checkpoint fetch request failed: {err}"),
                })?;
            if request.height > 1 {
                return serde_json::to_vec(&super::replication::FetchCommitResponse {
                    found: false,
                    message: None,
                    lineage_envelope: None,
                })
                .map_err(|err| WorldError::DistributedValidationFailed {
                    reason: format!("encode unavailable checkpoint response failed: {err}"),
                });
            }
        }
        self.fetch_protocols
            .lock()
            .expect("lock checkpoint fetch protocols")
            .push(protocol.to_string());
        self.inner.request(protocol, payload)
    }

    fn connected_peer_ids(&self) -> Vec<String> {
        self.connected_peer_ids.clone()
    }

    fn known_peer_ids(&self) -> Vec<String> {
        self.connected_peer_ids.clone()
    }

    fn register_handler(
        &self,
        protocol: &str,
        handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        self.inner.register_handler(protocol, handler)
    }
}
