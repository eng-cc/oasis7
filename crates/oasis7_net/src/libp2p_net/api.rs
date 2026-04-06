use std::sync::Arc;

use futures::channel::oneshot;
use libp2p::{Multiaddr, PeerId};
use oasis7_proto::distributed_dht::DistributedDht as ProtoDistributedDht;
use oasis7_proto::distributed_net::DistributedNetwork as ProtoDistributedNetwork;

use crate::error::WorldError;
use crate::util::to_canonical_cbor;
use oasis7_proto::distributed::{
    dht_membership_key, dht_peer_record_key, dht_provider_key, dht_world_head_key,
    WorldHeadAnnounce,
};
use oasis7_proto::distributed_dht::{
    MembershipDirectorySnapshot, ProviderRecord, SignedPeerRecord,
};
use oasis7_proto::distributed_net::{NetworkMessage, NetworkSubscription};

use super::{snapshot_clone, Command, Libp2pNetwork, Libp2pReachabilitySnapshot, PeerManagerPeerHealth};

impl Libp2pNetwork {
    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    pub fn keypair(&self) -> &libp2p::identity::Keypair {
        &self.keypair
    }

    pub fn published(&self) -> Vec<NetworkMessage> {
        self.published.lock().expect("lock published").clone()
    }

    pub fn dial(&self, addr: Multiaddr) -> Result<(), WorldError> {
        self.enqueue_command(Command::Dial { addr })
    }

    pub fn listening_addrs(&self) -> Vec<Multiaddr> {
        self.listening_addrs
            .lock()
            .expect("lock listening addrs")
            .clone()
    }

    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.connected_peers
            .lock()
            .expect("lock connected peers")
            .iter()
            .cloned()
            .collect()
    }

    pub fn debug_errors(&self) -> Vec<String> {
        self.errors.lock().expect("lock errors").clone()
    }

    pub fn debug_peer_healths(&self) -> Vec<PeerManagerPeerHealth> {
        self.peer_healths
            .lock()
            .expect("lock peer healths")
            .values()
            .cloned()
            .collect()
    }

    pub fn reachability_snapshot(&self) -> Libp2pReachabilitySnapshot {
        snapshot_clone(&self.reachability)
    }

    pub(super) fn enqueue_command(&self, command: Command) -> Result<(), WorldError> {
        super::try_send_command(&self.command_tx, command)
    }
}

impl ProtoDistributedNetwork<WorldError> for Libp2pNetwork {
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        self.enqueue_command(Command::Publish {
            topic: topic.to_string(),
            payload: payload.to_vec(),
        })
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        self.enqueue_command(Command::Subscribe {
            topic: topic.to_string(),
        })?;
        Ok(NetworkSubscription::new(
            topic.to_string(),
            Arc::clone(&self.inbox),
        ))
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        self.request_with_providers(protocol, payload, &[])
    }

    fn request_with_providers(
        &self,
        protocol: &str,
        payload: &[u8],
        providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        let (sender, receiver) = oneshot::channel();
        self.enqueue_command(Command::Request {
            protocol: protocol.to_string(),
            payload: payload.to_vec(),
            providers: providers.to_vec(),
            response: sender,
        })?;
        futures::executor::block_on(receiver).map_err(|_| {
            WorldError::NetworkProtocolUnavailable {
                protocol: "libp2p".to_string(),
            }
        })?
    }

    fn register_handler(
        &self,
        protocol: &str,
        handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        self.enqueue_command(Command::RegisterHandler {
            protocol: protocol.to_string(),
            handler: Arc::from(handler),
        })
    }
}

impl ProtoDistributedDht<WorldError> for Libp2pNetwork {
    fn publish_provider(
        &self,
        world_id: &str,
        content_hash: &str,
        _provider_id: &str,
    ) -> Result<(), WorldError> {
        let key = dht_provider_key(world_id, content_hash);
        let (sender, receiver) = oneshot::channel();
        self.enqueue_command(Command::PublishProvider {
            key,
            response: sender,
        })?;
        futures::executor::block_on(receiver).map_err(|_| {
            WorldError::NetworkProtocolUnavailable {
                protocol: "libp2p".to_string(),
            }
        })?
    }

    fn get_providers(
        &self,
        world_id: &str,
        content_hash: &str,
    ) -> Result<Vec<ProviderRecord>, WorldError> {
        let key = dht_provider_key(world_id, content_hash);
        let (sender, receiver) = oneshot::channel();
        self.enqueue_command(Command::GetProviders {
            key,
            response: sender,
        })?;
        futures::executor::block_on(receiver).map_err(|_| {
            WorldError::NetworkProtocolUnavailable {
                protocol: "libp2p".to_string(),
            }
        })?
    }

    fn put_world_head(&self, world_id: &str, head: &WorldHeadAnnounce) -> Result<(), WorldError> {
        let key = dht_world_head_key(world_id);
        let payload = to_canonical_cbor(head)?;
        let (sender, receiver) = oneshot::channel();
        self.enqueue_command(Command::PutWorldHead {
            key,
            payload,
            response: sender,
        })?;
        futures::executor::block_on(receiver).map_err(|_| {
            WorldError::NetworkProtocolUnavailable {
                protocol: "libp2p".to_string(),
            }
        })?
    }

    fn get_world_head(&self, world_id: &str) -> Result<Option<WorldHeadAnnounce>, WorldError> {
        let key = dht_world_head_key(world_id);
        let (sender, receiver) = oneshot::channel();
        self.enqueue_command(Command::GetWorldHead {
            key,
            response: sender,
        })?;
        futures::executor::block_on(receiver).map_err(|_| {
            WorldError::NetworkProtocolUnavailable {
                protocol: "libp2p".to_string(),
            }
        })?
    }

    fn put_membership_directory(
        &self,
        world_id: &str,
        snapshot: &MembershipDirectorySnapshot,
    ) -> Result<(), WorldError> {
        let key = dht_membership_key(world_id);
        let payload = to_canonical_cbor(snapshot)?;
        let (sender, receiver) = oneshot::channel();
        self.enqueue_command(Command::PutMembershipDirectory {
            key,
            payload,
            response: sender,
        })?;
        futures::executor::block_on(receiver).map_err(|_| {
            WorldError::NetworkProtocolUnavailable {
                protocol: "libp2p".to_string(),
            }
        })?
    }

    fn get_membership_directory(
        &self,
        world_id: &str,
    ) -> Result<Option<MembershipDirectorySnapshot>, WorldError> {
        let key = dht_membership_key(world_id);
        let (sender, receiver) = oneshot::channel();
        self.enqueue_command(Command::GetMembershipDirectory {
            key,
            response: sender,
        })?;
        futures::executor::block_on(receiver).map_err(|_| {
            WorldError::NetworkProtocolUnavailable {
                protocol: "libp2p".to_string(),
            }
        })?
    }

    fn put_peer_record(&self, world_id: &str, record: &SignedPeerRecord) -> Result<(), WorldError> {
        let key = dht_peer_record_key(world_id, record.record.peer_id.as_str());
        let payload = to_canonical_cbor(record)?;
        let (sender, receiver) = oneshot::channel();
        self.enqueue_command(Command::PutPeerRecord {
            key,
            payload,
            response: sender,
        })?;
        futures::executor::block_on(receiver).map_err(|_| {
            WorldError::NetworkProtocolUnavailable {
                protocol: "libp2p".to_string(),
            }
        })?
    }

    fn get_peer_record(
        &self,
        world_id: &str,
        peer_id: &str,
    ) -> Result<Option<SignedPeerRecord>, WorldError> {
        let key = dht_peer_record_key(world_id, peer_id);
        let (sender, receiver) = oneshot::channel();
        self.enqueue_command(Command::GetPeerRecord {
            key,
            response: sender,
        })?;
        futures::executor::block_on(receiver).map_err(|_| {
            WorldError::NetworkProtocolUnavailable {
                protocol: "libp2p".to_string(),
            }
        })?
    }
}
