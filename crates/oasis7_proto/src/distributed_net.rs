//! Distributed network adapter abstractions (libp2p-ready).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::distributed_dht::PeerNodeRole;

pub const DEFAULT_SUBSCRIPTION_INBOX_MAX_MESSAGES: usize = 1024;
pub const CONSENSUS_LANE_SUBSCRIPTION_INBOX_MAX_MESSAGES: usize = 256;
pub const SYNC_LANE_SUBSCRIPTION_INBOX_MAX_MESSAGES: usize = 1024;
pub const BLOB_STATE_LANE_SUBSCRIPTION_INBOX_MAX_MESSAGES: usize = 128;
pub const CONTROL_LANE_SUBSCRIPTION_INBOX_MAX_MESSAGES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkLane {
    ConsensusGossip,
    Sync,
    BlobState,
    Control,
}

impl NetworkLane {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ConsensusGossip => "consensus_gossip",
            Self::Sync => "sync",
            Self::BlobState => "blob_state",
            Self::Control => "control",
        }
    }

    pub fn qos_class(self) -> NetworkLaneQosClass {
        match self {
            Self::ConsensusGossip => NetworkLaneQosClass::LowJitter,
            Self::Sync => NetworkLaneQosClass::RecoverableBulk,
            Self::BlobState => NetworkLaneQosClass::RateLimitedBulk,
            Self::Control => NetworkLaneQosClass::HighPriorityControl,
        }
    }

    pub fn default_subscription_inbox_messages(self) -> usize {
        match self {
            Self::ConsensusGossip => CONSENSUS_LANE_SUBSCRIPTION_INBOX_MAX_MESSAGES,
            Self::Sync => SYNC_LANE_SUBSCRIPTION_INBOX_MAX_MESSAGES,
            Self::BlobState => BLOB_STATE_LANE_SUBSCRIPTION_INBOX_MAX_MESSAGES,
            Self::Control => CONTROL_LANE_SUBSCRIPTION_INBOX_MAX_MESSAGES,
        }
    }

    pub fn allows_role(self, role: PeerNodeRole, operation: NetworkLaneOperation) -> bool {
        match self {
            Self::ConsensusGossip => match operation {
                NetworkLaneOperation::Publish => {
                    matches!(role, PeerNodeRole::ValidatorCore | PeerNodeRole::Sentry)
                }
                NetworkLaneOperation::Subscribe => !matches!(role, PeerNodeRole::Relay),
                NetworkLaneOperation::Request | NetworkLaneOperation::Serve => false,
            },
            Self::Sync => match operation {
                NetworkLaneOperation::Publish
                | NetworkLaneOperation::Subscribe
                | NetworkLaneOperation::Request => !matches!(role, PeerNodeRole::Relay),
                NetworkLaneOperation::Serve => matches!(
                    role,
                    PeerNodeRole::ValidatorCore
                        | PeerNodeRole::Sentry
                        | PeerNodeRole::FullStorage
                ),
            },
            Self::BlobState => match operation {
                NetworkLaneOperation::Request => !matches!(role, PeerNodeRole::Relay),
                NetworkLaneOperation::Publish
                | NetworkLaneOperation::Subscribe
                | NetworkLaneOperation::Serve => matches!(
                    role,
                    PeerNodeRole::ValidatorCore
                        | PeerNodeRole::Sentry
                        | PeerNodeRole::FullStorage
                ),
            },
            Self::Control => true,
        }
    }
}

impl std::fmt::Display for NetworkLane {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkLaneQosClass {
    LowJitter,
    RecoverableBulk,
    RateLimitedBulk,
    HighPriorityControl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NetworkLaneOperation {
    Publish,
    Subscribe,
    Request,
    Serve,
}

pub fn classify_network_topic(topic: &str) -> Option<NetworkLane> {
    let topic = topic.trim();
    if topic.is_empty() {
        return None;
    }
    if topic.ends_with(".consensus.proposal")
        || topic.ends_with(".consensus.attestation")
        || topic.ends_with(".consensus.commit")
    {
        return Some(NetworkLane::ConsensusGossip);
    }
    if topic.ends_with(".replication") {
        return Some(NetworkLane::Sync);
    }
    if topic.ends_with(".feedback.announce") {
        return Some(NetworkLane::BlobState);
    }
    None
}

pub fn classify_network_protocol(protocol: &str) -> Option<NetworkLane> {
    let protocol = protocol.trim();
    if protocol.is_empty() {
        return None;
    }
    if protocol.starts_with("/aw/node/replication/fetch-commit/") {
        return Some(NetworkLane::Sync);
    }
    if protocol.starts_with("/aw/node/replication/fetch-blob/") {
        return Some(NetworkLane::BlobState);
    }
    if protocol.starts_with("/aw/rr/1.0.0/") {
        return Some(NetworkLane::Control);
    }
    None
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkMessage {
    pub topic: String,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkRequest {
    pub protocol: String,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkResponse {
    pub payload: Vec<u8>,
}

pub trait DistributedNetwork<E> {
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), E>;
    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, E>;
    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, E>;
    fn request_with_providers(
        &self,
        protocol: &str,
        payload: &[u8],
        _providers: &[String],
    ) -> Result<Vec<u8>, E> {
        self.request(protocol, payload)
    }
    fn register_handler(
        &self,
        protocol: &str,
        handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, E> + Send + Sync>,
    ) -> Result<(), E>;
}

#[derive(Debug, Clone)]
pub struct NetworkSubscription {
    topic: String,
    inbox: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
    max_inbox_messages: usize,
}

impl NetworkSubscription {
    pub fn new(topic: String, inbox: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>) -> Self {
        Self::with_max_inbox_messages(topic, inbox, DEFAULT_SUBSCRIPTION_INBOX_MAX_MESSAGES)
    }

    pub fn with_max_inbox_messages(
        topic: String,
        inbox: Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
        max_inbox_messages: usize,
    ) -> Self {
        Self {
            topic,
            inbox,
            max_inbox_messages: max_inbox_messages.max(1),
        }
    }

    pub fn topic(&self) -> &str {
        &self.topic
    }

    pub fn max_inbox_messages(&self) -> usize {
        self.max_inbox_messages
    }

    pub fn drain(&self) -> Vec<Vec<u8>> {
        let mut inbox = self.inbox.lock().expect("lock inbox");
        inbox.remove(&self.topic).unwrap_or_default()
    }
}

pub fn push_bounded_inbox_message(
    inbox: &Arc<Mutex<HashMap<String, Vec<Vec<u8>>>>>,
    topic: &str,
    payload: Vec<u8>,
    max_inbox_messages: usize,
) {
    let max_inbox_messages = max_inbox_messages.max(1);
    let mut inbox = inbox.lock().expect("lock inbox");
    let entries = inbox.entry(topic.to_string()).or_default();
    entries.push(payload);
    let overflow = entries.len().saturating_sub(max_inbox_messages);
    if overflow > 0 {
        entries.drain(0..overflow);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distributed_dht::PeerNodeRole;

    #[test]
    fn push_bounded_inbox_message_evicts_oldest_messages() {
        let inbox = Arc::new(Mutex::new(HashMap::<String, Vec<Vec<u8>>>::new()));
        push_bounded_inbox_message(&inbox, "topic-a", b"m1".to_vec(), 2);
        push_bounded_inbox_message(&inbox, "topic-a", b"m2".to_vec(), 2);
        push_bounded_inbox_message(&inbox, "topic-a", b"m3".to_vec(), 2);

        let mut guard = inbox.lock().expect("lock inbox");
        let queued = guard.remove("topic-a").expect("topic queue");
        assert_eq!(queued, vec![b"m2".to_vec(), b"m3".to_vec()]);
    }

    #[test]
    fn network_subscription_new_uses_default_bounded_limit() {
        let inbox = Arc::new(Mutex::new(HashMap::<String, Vec<Vec<u8>>>::new()));
        let subscription = NetworkSubscription::new("topic-a".to_string(), Arc::clone(&inbox));
        assert_eq!(
            subscription.max_inbox_messages(),
            DEFAULT_SUBSCRIPTION_INBOX_MAX_MESSAGES
        );
    }

    #[test]
    fn classify_network_bindings_maps_topics_and_protocols_to_lanes() {
        assert_eq!(
            classify_network_topic("aw.world.consensus.proposal"),
            Some(NetworkLane::ConsensusGossip)
        );
        assert_eq!(
            classify_network_topic("aw.world.replication"),
            Some(NetworkLane::Sync)
        );
        assert_eq!(
            classify_network_topic("aw.world.feedback.announce"),
            Some(NetworkLane::BlobState)
        );
        assert_eq!(
            classify_network_protocol("/aw/node/replication/fetch-commit/1.0.0"),
            Some(NetworkLane::Sync)
        );
        assert_eq!(
            classify_network_protocol("/aw/node/replication/fetch-blob/1.0.0"),
            Some(NetworkLane::BlobState)
        );
        assert_eq!(
            classify_network_protocol("/aw/rr/1.0.0/get_cached_peer_record"),
            Some(NetworkLane::Control)
        );
    }

    #[test]
    fn lane_role_policy_blocks_obvious_role_mismatches() {
        assert!(NetworkLane::ConsensusGossip
            .allows_role(PeerNodeRole::ValidatorCore, NetworkLaneOperation::Publish));
        assert!(!NetworkLane::ConsensusGossip
            .allows_role(PeerNodeRole::ObserverLight, NetworkLaneOperation::Publish));
        assert!(NetworkLane::Sync.allows_role(
            PeerNodeRole::ObserverLight,
            NetworkLaneOperation::Request
        ));
        assert!(!NetworkLane::Sync.allows_role(
            PeerNodeRole::ObserverLight,
            NetworkLaneOperation::Serve
        ));
        assert!(NetworkLane::BlobState.allows_role(
            PeerNodeRole::ObserverLight,
            NetworkLaneOperation::Request
        ));
        assert!(!NetworkLane::BlobState.allows_role(
            PeerNodeRole::ObserverLight,
            NetworkLaneOperation::Serve
        ));
        assert!(NetworkLane::BlobState
            .allows_role(PeerNodeRole::FullStorage, NetworkLaneOperation::Serve));
        assert!(
            !NetworkLane::BlobState.allows_role(PeerNodeRole::Relay, NetworkLaneOperation::Serve)
        );
        assert!(NetworkLane::Control.allows_role(PeerNodeRole::Relay, NetworkLaneOperation::Serve));
    }
}
