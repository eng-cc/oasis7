use super::*;
use oasis7_distfs::FileReplicationRecord;
use oasis7_proto::distributed_dht::{
    MembershipDirectorySnapshot, ProviderRecord, SignedPeerRecord,
};
use std::sync::mpsc;
use std::time::Duration;

use crate::{NodeConfig, NodeRole};

struct NoopDistributedNetwork;

impl DistributedNetwork<WorldError> for NoopDistributedNetwork {
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

    fn register_handler(
        &self,
        _protocol: &str,
        _handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        Ok(())
    }
}

struct BlockingProviderDht {
    entered: Mutex<mpsc::Sender<()>>,
    release: Mutex<mpsc::Receiver<()>>,
}

impl proto_dht::DistributedDht<WorldError> for BlockingProviderDht {
    fn publish_provider(
        &self,
        _world_id: &str,
        _content_hash: &str,
        _provider_id: &str,
    ) -> Result<(), WorldError> {
        let _ = self.entered.lock().expect("lock entered").send(());
        let _ = self
            .release
            .lock()
            .expect("lock release")
            .recv_timeout(Duration::from_secs(2));
        Ok(())
    }

    fn publish_provider_best_effort(
        &self,
        _world_id: &str,
        _content_hash: &str,
        _provider_id: &str,
    ) -> Result<(), WorldError> {
        let _ = self.entered.lock().expect("lock entered").send(());
        Ok(())
    }

    fn get_providers(
        &self,
        _world_id: &str,
        _content_hash: &str,
    ) -> Result<Vec<ProviderRecord>, WorldError> {
        Ok(Vec::new())
    }

    fn put_world_head(&self, _world_id: &str, _head: &WorldHeadAnnounce) -> Result<(), WorldError> {
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

#[test]
fn publish_failure_stays_generic_replication_error() {
    let err = network_err(WorldError::NetworkProtocolUnavailable {
        protocol: "libp2p publish failed topic=aw.publish.fail: InsufficientPeers".to_string(),
    });
    assert_eq!(
        err,
        NodeError::Replication {
            reason: "replication network error: NetworkProtocolUnavailable { protocol: \"libp2p publish failed topic=aw.publish.fail: InsufficientPeers\" }".to_string(),
        }
    );
}

#[test]
fn cacheable_fetch_commit_success_response_drops_payload_allocation() {
    let mut payload = Vec::with_capacity(4096);
    payload.extend_from_slice(b"replicated-commit-payload");
    let response = FetchCommitResponse {
        found: true,
        message: Some(GossipReplicationMessage {
            version: 1,
            world_id: "world-cache".to_string(),
            node_id: "node-a".to_string(),
            record: FileReplicationRecord {
                world_id: "world-cache".to_string(),
                writer_id: "writer-a".to_string(),
                writer_epoch: 1,
                sequence: 1,
                path: "consensus/commits/00000000000000000001.json".to_string(),
                content_hash: "hash-1".to_string(),
                size_bytes: payload.len() as u64,
                updated_at_ms: 1,
            },
            payload,
            public_key_hex: None,
            signature_hex: None,
        }),
    };

    let cached = cacheable_fetch_commit_success_response(&response).expect("cached response");
    let payload = &cached.message.expect("cached message").payload;
    assert!(payload.is_empty());
    assert_eq!(payload.capacity(), 0);
}

#[test]
fn best_effort_provider_publish_does_not_block_on_slow_dht() {
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let dht = Arc::new(BlockingProviderDht {
        entered: Mutex::new(entered_tx),
        release: Mutex::new(release_rx),
    });
    let handle = NodeReplicationNetworkHandle::new(Arc::new(NoopDistributedNetwork))
        .with_dht(dht)
        .with_local_provider_id("storage-provider");
    let config =
        NodeConfig::new("storage-provider", "world-provider", NodeRole::Storage).expect("config");
    let endpoint =
        ReplicationNetworkEndpoint::new(&handle, "world-provider", false, &config.network_policy)
            .expect("endpoint");

    endpoint.publish_local_content_provider_best_effort("world-provider", "hash-1");

    entered_rx
        .recv_timeout(Duration::from_millis(250))
        .expect("best-effort provider publish should start promptly");
    drop(release_tx);
}
