use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use oasis7_proto::distributed_dht as proto_dht;
use oasis7_proto::distributed_net::{
    classify_network_protocol, DistributedNetwork, NetworkLane, NetworkLaneOperation,
    NetworkSubscription,
};
use oasis7_proto::world_error::WorldError;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::gossip_udp::{
    GossipAttestationMessage, GossipCommitMessage, GossipMessage, GossipProposalMessage,
};
use crate::replication::{
    FetchCommitRequest, FetchCommitResponse, GossipReplicationMessage,
    REPLICATION_FETCH_COMMIT_PROTOCOL,
};
use crate::{NodeError, NodeNetworkPolicy};

pub(crate) const DEFAULT_REPLICATION_TOPIC_PREFIX: &str = "aw";
pub(crate) const DEFAULT_CONSENSUS_PROPOSAL_TOPIC_SUFFIX: &str = "consensus.proposal";
pub(crate) const DEFAULT_CONSENSUS_ATTESTATION_TOPIC_SUFFIX: &str = "consensus.attestation";
pub(crate) const DEFAULT_CONSENSUS_COMMIT_TOPIC_SUFFIX: &str = "consensus.commit";
const FETCH_COMMIT_SUCCESS_CACHE_AFTER_MS: u64 = 5_000;
const FETCH_COMMIT_SUCCESS_CACHE_MAX_ENTRIES: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct FetchCommitSuccessCacheKey {
    world_id: String,
    height: u64,
    requester_public_key_hex: Option<String>,
}

#[derive(Debug, Clone)]
struct CachedFetchCommitSuccess {
    response: FetchCommitResponse,
    cached_at: Instant,
    valid_until: Instant,
}

pub(crate) struct GapSyncFetchCommitResponse {
    pub response: FetchCommitResponse,
    pub from_cache: bool,
}

pub(crate) fn default_replication_topic(world_id: &str) -> String {
    format!("{DEFAULT_REPLICATION_TOPIC_PREFIX}.{world_id}.replication")
}

pub(crate) fn default_consensus_proposal_topic(world_id: &str) -> String {
    format!(
        "{DEFAULT_REPLICATION_TOPIC_PREFIX}.{world_id}.{}",
        DEFAULT_CONSENSUS_PROPOSAL_TOPIC_SUFFIX
    )
}

pub(crate) fn default_consensus_attestation_topic(world_id: &str) -> String {
    format!(
        "{DEFAULT_REPLICATION_TOPIC_PREFIX}.{world_id}.{}",
        DEFAULT_CONSENSUS_ATTESTATION_TOPIC_SUFFIX
    )
}

pub(crate) fn default_consensus_commit_topic(world_id: &str) -> String {
    format!(
        "{DEFAULT_REPLICATION_TOPIC_PREFIX}.{world_id}.{}",
        DEFAULT_CONSENSUS_COMMIT_TOPIC_SUFFIX
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrafficLaneRegistry {
    pub replication_topic: String,
    pub consensus_proposal_topic: String,
    pub consensus_attestation_topic: String,
    pub consensus_commit_topic: String,
}

impl TrafficLaneRegistry {
    fn for_handle(handle: &NodeReplicationNetworkHandle, world_id: &str) -> Self {
        Self {
            replication_topic: handle.resolved_topic(world_id),
            consensus_proposal_topic: default_consensus_proposal_topic(world_id),
            consensus_attestation_topic: default_consensus_attestation_topic(world_id),
            consensus_commit_topic: default_consensus_commit_topic(world_id),
        }
    }
}

fn validate_lane_access(
    network_policy: &NodeNetworkPolicy,
    lane: NetworkLane,
    operation: NetworkLaneOperation,
    label: &str,
) -> Result<(), NodeError> {
    if network_policy.allows_lane_operation(lane, operation) {
        return Ok(());
    }
    Err(NodeError::InvalidConfig {
        reason: format!(
            "node_role_claim={} cannot {:?} {} on lane={}",
            network_policy.node_role_claim, operation, label, lane
        ),
    })
}

#[derive(Clone)]
pub struct NodeReplicationNetworkHandle {
    network: Arc<dyn DistributedNetwork<WorldError> + Send + Sync>,
    dht: Option<Arc<dyn proto_dht::DistributedDht<WorldError> + Send + Sync>>,
    local_provider_id: Option<String>,
    topic: Option<String>,
}

impl fmt::Debug for NodeReplicationNetworkHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeReplicationNetworkHandle")
            .field("topic", &self.topic)
            .finish()
    }
}

impl NodeReplicationNetworkHandle {
    pub fn new(network: Arc<dyn DistributedNetwork<WorldError> + Send + Sync>) -> Self {
        Self {
            network,
            dht: None,
            local_provider_id: None,
            topic: None,
        }
    }

    pub fn with_dht(
        mut self,
        dht: Arc<dyn proto_dht::DistributedDht<WorldError> + Send + Sync>,
    ) -> Self {
        self.dht = Some(dht);
        self
    }

    pub fn with_local_provider_id(mut self, provider_id: impl Into<String>) -> Self {
        let provider_id = provider_id.into();
        let provider_id = provider_id.trim();
        if !provider_id.is_empty() {
            self.local_provider_id = Some(provider_id.to_string());
        }
        self
    }

    pub fn with_topic(mut self, topic: impl Into<String>) -> Result<Self, NodeError> {
        let topic = topic.into();
        if topic.trim().is_empty() {
            return Err(NodeError::InvalidConfig {
                reason: "replication network topic cannot be empty".to_string(),
            });
        }
        self.topic = Some(topic);
        Ok(self)
    }

    pub fn clone_network(&self) -> Arc<dyn DistributedNetwork<WorldError> + Send + Sync> {
        Arc::clone(&self.network)
    }

    fn resolved_topic(&self, world_id: &str) -> String {
        self.topic
            .clone()
            .unwrap_or_else(|| default_replication_topic(world_id))
    }

    fn resolved_lane_registry(&self, world_id: &str) -> TrafficLaneRegistry {
        TrafficLaneRegistry::for_handle(self, world_id)
    }
}

pub(crate) struct ReplicationNetworkEndpoint {
    network: Arc<dyn DistributedNetwork<WorldError> + Send + Sync>,
    dht: Option<Arc<dyn proto_dht::DistributedDht<WorldError> + Send + Sync>>,
    local_provider_id: Option<String>,
    network_policy: NodeNetworkPolicy,
    topic: String,
    subscription: Option<NetworkSubscription>,
    fetch_commit_success_cache_after: Duration,
    recent_fetch_commit_successes:
        Mutex<HashMap<FetchCommitSuccessCacheKey, CachedFetchCommitSuccess>>,
}

impl ReplicationNetworkEndpoint {
    pub(crate) fn new(
        handle: &NodeReplicationNetworkHandle,
        world_id: &str,
        subscribe: bool,
        network_policy: &NodeNetworkPolicy,
    ) -> Result<Self, NodeError> {
        let registry = handle.resolved_lane_registry(world_id);
        let topic = registry.replication_topic;
        let subscription = if subscribe {
            validate_lane_access(
                network_policy,
                NetworkLane::Sync,
                NetworkLaneOperation::Subscribe,
                topic.as_str(),
            )?;
            Some(
                handle
                    .network
                    .subscribe(topic.as_str())
                    .map_err(network_err)?,
            )
        } else {
            None
        };
        Ok(Self {
            network: Arc::clone(&handle.network),
            dht: handle.dht.clone(),
            local_provider_id: handle.local_provider_id.clone(),
            network_policy: network_policy.clone(),
            topic,
            subscription,
            fetch_commit_success_cache_after: Duration::from_millis(
                FETCH_COMMIT_SUCCESS_CACHE_AFTER_MS,
            ),
            recent_fetch_commit_successes: Mutex::new(HashMap::new()),
        })
    }

    #[cfg(test)]
    pub(crate) fn set_fetch_commit_success_cache_after_for_testing(&mut self, duration: Duration) {
        self.fetch_commit_success_cache_after = duration;
    }

    pub(crate) fn publish_replication(
        &self,
        message: &GossipReplicationMessage,
    ) -> Result<(), NodeError> {
        validate_lane_access(
            &self.network_policy,
            NetworkLane::Sync,
            NetworkLaneOperation::Publish,
            self.topic.as_str(),
        )?;
        let payload = serde_json::to_vec(message).map_err(|err| NodeError::Replication {
            reason: format!("serialize replication network message failed: {}", err),
        })?;
        self.network
            .publish(self.topic.as_str(), payload.as_slice())
            .map_err(network_err)
    }

    pub(crate) fn drain_replications(&self) -> Result<Vec<GossipReplicationMessage>, NodeError> {
        let Some(subscription) = &self.subscription else {
            return Ok(Vec::new());
        };

        let mut messages = Vec::new();
        for payload in subscription.drain() {
            if let Ok(message) = serde_json::from_slice::<GossipReplicationMessage>(&payload) {
                messages.push(message);
            }
        }
        Ok(messages)
    }

    pub(crate) fn request_json<Req, Resp>(
        &self,
        protocol: &str,
        request: &Req,
    ) -> Result<Resp, NodeError>
    where
        Req: Serialize,
        Resp: DeserializeOwned,
    {
        if let Some(lane) = classify_network_protocol(protocol) {
            validate_lane_access(
                &self.network_policy,
                lane,
                NetworkLaneOperation::Request,
                protocol,
            )?;
        }
        let payload = serde_json::to_vec(request).map_err(|err| NodeError::Replication {
            reason: format!("serialize replication request {} failed: {}", protocol, err),
        })?;
        let response_bytes = self
            .network
            .request(protocol, payload.as_slice())
            .map_err(network_err)?;
        serde_json::from_slice::<Resp>(&response_bytes).map_err(|err| NodeError::Replication {
            reason: format!("decode replication response {} failed: {}", protocol, err),
        })
    }

    pub(crate) fn request_fetch_commit_for_gap_sync(
        &self,
        request: &FetchCommitRequest,
    ) -> Result<GapSyncFetchCommitResponse, NodeError> {
        if let Some(lane) = classify_network_protocol(REPLICATION_FETCH_COMMIT_PROTOCOL) {
            validate_lane_access(
                &self.network_policy,
                lane,
                NetworkLaneOperation::Request,
                REPLICATION_FETCH_COMMIT_PROTOCOL,
            )?;
        }
        if let Some(response) = self.cached_fetch_commit_success_response(request) {
            return Ok(GapSyncFetchCommitResponse {
                response,
                from_cache: true,
            });
        }
        let response = self.request_json(REPLICATION_FETCH_COMMIT_PROTOCOL, request)?;
        Ok(GapSyncFetchCommitResponse {
            response,
            from_cache: false,
        })
    }

    pub(crate) fn remember_validated_fetch_commit_success(
        &self,
        request: &FetchCommitRequest,
        response: &FetchCommitResponse,
    ) {
        let Some(response) = cacheable_fetch_commit_success_response(response) else {
            return;
        };
        let now = Instant::now();
        let mut cache = self
            .recent_fetch_commit_successes
            .lock()
            .expect("lock fetch-commit success cache");
        cache.retain(|_, entry| entry.valid_until > now);
        if cache.len() >= FETCH_COMMIT_SUCCESS_CACHE_MAX_ENTRIES {
            if let Some(oldest_key) = cache
                .iter()
                .min_by_key(|(_, entry)| entry.cached_at)
                .map(|(key, _)| key.clone())
            {
                cache.remove(&oldest_key);
            }
        }
        cache.insert(
            fetch_commit_success_cache_key(request),
            CachedFetchCommitSuccess {
                response,
                cached_at: now,
                valid_until: now + self.fetch_commit_success_cache_after,
            },
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn request_json_with_providers<Req, Resp>(
        &self,
        protocol: &str,
        request: &Req,
        providers: &[String],
    ) -> Result<Resp, NodeError>
    where
        Req: Serialize,
        Resp: DeserializeOwned,
    {
        if let Some(lane) = classify_network_protocol(protocol) {
            validate_lane_access(
                &self.network_policy,
                lane,
                NetworkLaneOperation::Request,
                protocol,
            )?;
        }
        let payload = serde_json::to_vec(request).map_err(|err| NodeError::Replication {
            reason: format!("serialize replication request {} failed: {}", protocol, err),
        })?;
        let response_bytes = self
            .network
            .request_with_providers(protocol, payload.as_slice(), providers)
            .map_err(network_err)?;
        serde_json::from_slice::<Resp>(&response_bytes).map_err(|err| NodeError::Replication {
            reason: format!("decode replication response {} failed: {}", protocol, err),
        })
    }

    pub(crate) fn lookup_provider_ids_for_content_hash(
        &self,
        world_id: &str,
        content_hash: &str,
    ) -> Result<Option<Vec<String>>, NodeError> {
        let Some(dht) = self.dht.as_ref() else {
            return Ok(None);
        };
        let mut providers = dht
            .get_providers(world_id, content_hash)
            .map_err(network_err)?;
        providers.sort_by(|left, right| {
            right
                .last_seen_ms
                .cmp(&left.last_seen_ms)
                .then_with(|| left.provider_id.cmp(&right.provider_id))
        });
        let mut provider_ids = Vec::with_capacity(providers.len());
        for provider in providers {
            let provider_id = provider.provider_id.trim();
            if provider_id.is_empty() {
                continue;
            }
            if self.local_provider_id.as_deref() == Some(provider_id) {
                continue;
            }
            if provider_ids.iter().any(|existing| existing == provider_id) {
                continue;
            }
            provider_ids.push(provider_id.to_string());
        }
        Ok(Some(provider_ids))
    }

    pub(crate) fn publish_local_content_provider(
        &self,
        world_id: &str,
        content_hash: &str,
    ) -> Result<(), NodeError> {
        let Some(dht) = self.dht.as_ref() else {
            return Ok(());
        };
        let Some(local_provider_id) = self.local_provider_id.as_deref() else {
            return Ok(());
        };
        if !self
            .network_policy
            .allows_lane_operation(NetworkLane::BlobState, NetworkLaneOperation::Serve)
        {
            return Ok(());
        }
        dht.publish_provider(world_id, content_hash, local_provider_id)
            .map_err(network_err)
    }

    fn cached_fetch_commit_success_response(
        &self,
        request: &FetchCommitRequest,
    ) -> Option<FetchCommitResponse> {
        let now = Instant::now();
        let mut cache = self
            .recent_fetch_commit_successes
            .lock()
            .expect("lock fetch-commit success cache");
        cache.retain(|_, entry| entry.valid_until > now);
        cache
            .get(&fetch_commit_success_cache_key(request))
            .map(|entry| entry.response.clone())
    }
}

fn fetch_commit_success_cache_key(request: &FetchCommitRequest) -> FetchCommitSuccessCacheKey {
    FetchCommitSuccessCacheKey {
        world_id: request.world_id.clone(),
        height: request.height,
        requester_public_key_hex: request.requester_public_key_hex.clone(),
    }
}

fn cacheable_fetch_commit_success_response(
    response: &FetchCommitResponse,
) -> Option<FetchCommitResponse> {
    if !response.found {
        return None;
    }
    let mut cached = response.clone();
    let message = cached.message.as_mut()?;
    message.payload = Vec::new();
    Some(cached)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oasis7_distfs::FileReplicationRecord;

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
}

pub(crate) struct ConsensusNetworkEndpoint {
    network: Arc<dyn DistributedNetwork<WorldError> + Send + Sync>,
    network_policy: NodeNetworkPolicy,
    proposal_topic: String,
    attestation_topic: String,
    commit_topic: String,
    proposal_subscription: Option<NetworkSubscription>,
    attestation_subscription: Option<NetworkSubscription>,
    commit_subscription: Option<NetworkSubscription>,
}

impl ConsensusNetworkEndpoint {
    pub(crate) fn new(
        handle: &NodeReplicationNetworkHandle,
        world_id: &str,
        subscribe: bool,
        network_policy: &NodeNetworkPolicy,
    ) -> Result<Self, NodeError> {
        let registry = handle.resolved_lane_registry(world_id);
        let proposal_topic = registry.consensus_proposal_topic;
        let attestation_topic = registry.consensus_attestation_topic;
        let commit_topic = registry.consensus_commit_topic;
        let proposal_subscription = if subscribe {
            validate_lane_access(
                network_policy,
                NetworkLane::ConsensusGossip,
                NetworkLaneOperation::Subscribe,
                proposal_topic.as_str(),
            )?;
            Some(
                handle
                    .network
                    .subscribe(proposal_topic.as_str())
                    .map_err(network_err)?,
            )
        } else {
            None
        };
        let attestation_subscription = if subscribe {
            Some(
                handle
                    .network
                    .subscribe(attestation_topic.as_str())
                    .map_err(network_err)?,
            )
        } else {
            None
        };
        let commit_subscription = if subscribe {
            Some(
                handle
                    .network
                    .subscribe(commit_topic.as_str())
                    .map_err(network_err)?,
            )
        } else {
            None
        };
        Ok(Self {
            network: Arc::clone(&handle.network),
            network_policy: network_policy.clone(),
            proposal_topic,
            attestation_topic,
            commit_topic,
            proposal_subscription,
            attestation_subscription,
            commit_subscription,
        })
    }

    pub(crate) fn publish_proposal(
        &self,
        message: &GossipProposalMessage,
    ) -> Result<(), NodeError> {
        self.publish_json(self.proposal_topic.as_str(), message)
    }

    pub(crate) fn publish_attestation(
        &self,
        message: &GossipAttestationMessage,
    ) -> Result<(), NodeError> {
        self.publish_json(self.attestation_topic.as_str(), message)
    }

    pub(crate) fn publish_commit(&self, message: &GossipCommitMessage) -> Result<(), NodeError> {
        self.publish_json(self.commit_topic.as_str(), message)
    }

    pub(crate) fn drain_messages(&self) -> Result<Vec<GossipMessage>, NodeError> {
        let mut out = Vec::new();
        Self::drain_subscription(self.proposal_subscription.as_ref(), &mut out);
        Self::drain_subscription(self.attestation_subscription.as_ref(), &mut out);
        Self::drain_subscription(self.commit_subscription.as_ref(), &mut out);
        Ok(out)
    }

    pub(crate) fn allows_publish(&self) -> bool {
        self.network_policy
            .allows_lane_operation(NetworkLane::ConsensusGossip, NetworkLaneOperation::Publish)
    }

    fn publish_json<T: Serialize>(&self, topic: &str, message: &T) -> Result<(), NodeError> {
        validate_lane_access(
            &self.network_policy,
            NetworkLane::ConsensusGossip,
            NetworkLaneOperation::Publish,
            topic,
        )?;
        let payload = serde_json::to_vec(message).map_err(|err| NodeError::Replication {
            reason: format!("serialize consensus network message failed: {}", err),
        })?;
        self.network
            .publish(topic, payload.as_slice())
            .map_err(network_err)
    }

    fn drain_subscription(
        subscription: Option<&NetworkSubscription>,
        out: &mut Vec<GossipMessage>,
    ) {
        let Some(subscription) = subscription else {
            return;
        };
        for payload in subscription.drain() {
            if let Some(message) = decode_consensus_message(payload.as_slice()) {
                out.push(message);
            }
        }
    }
}

fn decode_consensus_message(payload: &[u8]) -> Option<GossipMessage> {
    if let Ok(message) = serde_json::from_slice::<GossipMessage>(payload) {
        match message {
            GossipMessage::Proposal(_)
            | GossipMessage::Attestation(_)
            | GossipMessage::Commit(_) => return Some(message),
            GossipMessage::Hello(_) => {}
            GossipMessage::Replication(_) => {}
        }
    }
    if let Ok(message) = serde_json::from_slice::<GossipProposalMessage>(payload) {
        return Some(GossipMessage::Proposal(message));
    }
    if let Ok(message) = serde_json::from_slice::<GossipAttestationMessage>(payload) {
        return Some(GossipMessage::Attestation(message));
    }
    if let Ok(message) = serde_json::from_slice::<GossipCommitMessage>(payload) {
        return Some(GossipMessage::Commit(message));
    }
    None
}

fn network_err(err: WorldError) -> NodeError {
    NodeError::Replication {
        reason: format!("replication network error: {err:?}"),
    }
}
