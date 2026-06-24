use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use oasis7_net::{world_error_is_publish_failure, world_error_is_retryable_connection_gap};
use oasis7_proto::distributed::WorldHeadAnnounce;
use oasis7_proto::distributed_dht as proto_dht;
use oasis7_proto::distributed_net::{
    DistributedNetwork, NetworkLane, NetworkLaneOperation, NetworkSubscription,
    classify_network_protocol,
};
use oasis7_proto::world_error::WorldError;
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::gossip_udp::{
    GossipAttestationMessage, GossipCommitMessage, GossipMessage, GossipProposalMessage,
};
use crate::network_bridge_gap_sync_budget::{
    gap_sync_fetch_commit_probe_route_budget, gap_sync_fetch_commit_route_budget,
    gap_sync_fetch_commit_route_budget_exhausted,
};
pub(crate) use crate::network_error_classification::{
    replication_network_error_is_availability_gap, replication_network_error_is_not_found,
    replication_network_error_is_protocol_unavailable,
    replication_network_error_is_route_unavailable, replication_network_error_is_timeout_protocol,
    replication_network_error_is_unsupported_protocol, replication_network_error_kind_label,
    replication_network_error_mentions_protocol,
};
use crate::replication::{
    FetchCommitRequest, FetchCommitResponse, FetchHeadRequest, FetchHeadResponse,
    GossipReplicationMessage, REPLICATION_FETCH_COMMIT_PROTOCOL, REPLICATION_GET_HEAD_PROTOCOL,
    load_blob_from_root,
};
use crate::{NodeError, NodeExecutionCheckpointDescriptor, NodeNetworkPolicy};

pub(crate) const DEFAULT_REPLICATION_TOPIC_PREFIX: &str = "aw";
pub(crate) const DEFAULT_CONSENSUS_PROPOSAL_TOPIC_SUFFIX: &str = "consensus.proposal";
pub(crate) const DEFAULT_CONSENSUS_ATTESTATION_TOPIC_SUFFIX: &str = "consensus.attestation";
pub(crate) const DEFAULT_CONSENSUS_COMMIT_TOPIC_SUFFIX: &str = "consensus.commit";
const FETCH_COMMIT_SUCCESS_CACHE_AFTER_MS: u64 = 5_000;
const FETCH_COMMIT_SUCCESS_CACHE_MAX_ENTRIES: usize = 64;
pub(crate) const REPLICATION_NETWORK_AVAILABILITY_GAP_PREFIX: &str =
    "replication network availability gap: ";
pub(crate) const REPLICATION_NETWORK_ROUTE_UNAVAILABLE_PREFIX: &str =
    "replication network route unavailable: ";
const FETCH_COMMIT_GENERIC_ROUTE_ATTEMPTS: usize = 4;
const GAP_SYNC_FETCH_COMMIT_MAX_PROVIDER_ROUTES_PER_POLL: usize = 8;
const GAP_SYNC_FETCH_COMMIT_MIN_PROVIDER_ROUTE_TIMEOUT_MS: u64 = 1_500;
const GAP_SYNC_FETCH_HEAD_REQUEST_TIMEOUT_MS: u64 = 1_500;
const GAP_SYNC_FETCH_HEAD_RETRY_BUDGET_MS: u64 = 3_000;
const GAP_SYNC_FETCH_HEAD_MAX_PROVIDER_ROUTES_PER_POLL: usize = 2;

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
    pub repair_summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GapSyncFetchCommitRetryPolicy {
    FullGapSync,
    SingleProbe,
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

    pub(crate) fn publish_local_content_provider(
        &self,
        network_policy: &NodeNetworkPolicy,
        world_id: &str,
        content_hash: &str,
    ) -> Result<(), NodeError> {
        let Some(dht) = self.dht.as_ref() else {
            return Ok(());
        };
        let Some(local_provider_id) = self.local_provider_id.as_deref() else {
            return Ok(());
        };
        if !network_policy
            .allows_lane_operation(NetworkLane::BlobState, NetworkLaneOperation::Serve)
        {
            return Ok(());
        }
        dht.publish_provider(world_id, content_hash, local_provider_id)
            .map_err(network_err)
    }

    pub(crate) fn publish_checkpoint_descriptor_providers_from_root_best_effort(
        &self,
        network_policy: &NodeNetworkPolicy,
        root_dir: &Path,
        world_id: &str,
        descriptor: Option<&NodeExecutionCheckpointDescriptor>,
    ) -> Result<(), NodeError> {
        let Some(descriptor) = descriptor else {
            return Ok(());
        };
        self.publish_checkpoint_blob_provider_from_root_best_effort(
            network_policy,
            root_dir,
            world_id,
            descriptor.manifest_ref.as_str(),
            descriptor.manifest_size_bytes,
        )?;
        for blob_ref in &descriptor.blobs {
            self.publish_checkpoint_blob_provider_from_root_best_effort(
                network_policy,
                root_dir,
                world_id,
                blob_ref.content_hash.as_str(),
                blob_ref.size_bytes,
            )?;
        }
        Ok(())
    }

    fn publish_checkpoint_blob_provider_from_root_best_effort(
        &self,
        network_policy: &NodeNetworkPolicy,
        root_dir: &Path,
        world_id: &str,
        content_hash: &str,
        expected_size_bytes: u64,
    ) -> Result<(), NodeError> {
        let Some(bytes) = load_blob_from_root(root_dir, content_hash)? else {
            return Ok(());
        };
        if bytes.len() as u64 != expected_size_bytes {
            return Err(NodeError::Replication {
                reason: format!(
                    "checkpoint provider publish local blob size mismatch hash={} expected={} actual={}",
                    content_hash,
                    expected_size_bytes,
                    bytes.len()
                ),
            });
        }
        self.publish_local_content_provider_best_effort(network_policy, world_id, content_hash);
        Ok(())
    }

    pub(crate) fn publish_local_content_provider_best_effort(
        &self,
        network_policy: &NodeNetworkPolicy,
        world_id: &str,
        content_hash: &str,
    ) {
        let Some(dht) = self.dht.as_ref() else {
            return;
        };
        let Some(local_provider_id) = self.local_provider_id.as_deref() else {
            return;
        };
        if !network_policy
            .allows_lane_operation(NetworkLane::BlobState, NetworkLaneOperation::Serve)
        {
            return;
        }
        let _ = dht.publish_provider_best_effort(world_id, content_hash, local_provider_id);
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
            .publish_best_effort(self.topic.as_str(), payload.as_slice())
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

    pub(crate) fn lookup_world_head(
        &self,
        world_id: &str,
    ) -> Result<Option<WorldHeadAnnounce>, NodeError> {
        if let Some(dht) = self.dht.as_ref() {
            match dht.get_world_head(world_id).map_err(network_err) {
                Ok(Some(head)) => {
                    validate_world_head_world_id(world_id, &head)?;
                    return Ok(Some(head));
                }
                Ok(None) => {}
                Err(err) if world_head_lookup_can_fallback(&err) => {}
                Err(err) => return Err(err),
            }
        }
        self.request_world_head_from_peers(world_id)
    }

    fn request_world_head_from_peers(
        &self,
        world_id: &str,
    ) -> Result<Option<WorldHeadAnnounce>, NodeError> {
        let request = FetchHeadRequest {
            world_id: world_id.to_string(),
        };
        let connected_peer_ids = self.network.connected_peer_ids();
        let mut best_head = None;
        if connected_peer_ids.is_empty() {
            self.maybe_update_best_peer_head(
                world_id,
                &mut best_head,
                self.request_json_budget(
                    REPLICATION_GET_HEAD_PROTOCOL,
                    &request,
                    GAP_SYNC_FETCH_HEAD_REQUEST_TIMEOUT_MS,
                    GAP_SYNC_FETCH_HEAD_RETRY_BUDGET_MS,
                ),
            )?;
        } else {
            for peer_id in connected_peer_ids
                .into_iter()
                .take(GAP_SYNC_FETCH_HEAD_MAX_PROVIDER_ROUTES_PER_POLL)
            {
                self.maybe_update_best_peer_head(
                    world_id,
                    &mut best_head,
                    self.request_json_with_providers_budget(
                        REPLICATION_GET_HEAD_PROTOCOL,
                        &request,
                        std::slice::from_ref(&peer_id),
                        GAP_SYNC_FETCH_HEAD_REQUEST_TIMEOUT_MS,
                        GAP_SYNC_FETCH_HEAD_RETRY_BUDGET_MS,
                    ),
                )?;
            }
        }
        Ok(best_head)
    }

    fn maybe_update_best_peer_head(
        &self,
        world_id: &str,
        best_head: &mut Option<WorldHeadAnnounce>,
        response: Result<FetchHeadResponse, NodeError>,
    ) -> Result<(), NodeError> {
        match response {
            Ok(FetchHeadResponse {
                found: true,
                head: Some(head),
            }) => {
                if head.world_id != world_id {
                    return Err(NodeError::Replication {
                        reason: format!(
                            "replication peer head mismatch: expected={} actual={}",
                            world_id, head.world_id
                        ),
                    });
                }
                let candidate = WorldHeadAnnounce {
                    world_id: head.world_id,
                    height: head.height,
                    block_hash: head.block_hash,
                    state_root: head.state_root,
                    timestamp_ms: head.timestamp_ms,
                    signature: String::new(),
                };
                if best_head
                    .as_ref()
                    .map(|current| candidate.height > current.height)
                    .unwrap_or(true)
                {
                    *best_head = Some(candidate);
                }
                Ok(())
            }
            Ok(_) => Ok(()),
            Err(err) if world_head_lookup_can_fallback(&err) => Ok(()),
            Err(err) => Err(err),
        }
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
            .map_err(|err| network_err_for_protocol(protocol, err))?;
        serde_json::from_slice::<Resp>(&response_bytes).map_err(|err| NodeError::Replication {
            reason: format!("decode replication response {} failed: {}", protocol, err),
        })
    }

    pub(crate) fn request_fetch_commit_for_gap_sync(
        &self,
        request: &FetchCommitRequest,
    ) -> Result<GapSyncFetchCommitResponse, NodeError> {
        self.request_fetch_commit_for_gap_sync_with_policy(
            request,
            GapSyncFetchCommitRetryPolicy::FullGapSync,
        )
    }

    pub(crate) fn request_fetch_commit_for_gap_sync_single_probe(
        &self,
        request: &FetchCommitRequest,
    ) -> Result<GapSyncFetchCommitResponse, NodeError> {
        self.request_fetch_commit_for_gap_sync_with_policy(
            request,
            GapSyncFetchCommitRetryPolicy::SingleProbe,
        )
    }

    fn request_fetch_commit_for_gap_sync_with_policy(
        &self,
        request: &FetchCommitRequest,
        retry_policy: GapSyncFetchCommitRetryPolicy,
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
                repair_summary: "cache=hit".to_string(),
            });
        }
        let mut last_err = None;
        let mut route_events = Vec::new();
        let route_sweep_started_at = Instant::now();
        let route_budget = || match retry_policy {
            GapSyncFetchCommitRetryPolicy::FullGapSync => {
                gap_sync_fetch_commit_route_budget(route_sweep_started_at)
            }
            GapSyncFetchCommitRetryPolicy::SingleProbe => {
                gap_sync_fetch_commit_probe_route_budget(route_sweep_started_at)
            }
        };
        let Some((request_timeout_ms, retry_budget_ms)) = route_budget() else {
            return Err(gap_sync_fetch_commit_route_budget_exhausted());
        };
        let mut response = match self
            .request_json_budget::<FetchCommitRequest, FetchCommitResponse>(
                REPLICATION_FETCH_COMMIT_PROTOCOL,
                request,
                request_timeout_ms,
                retry_budget_ms,
            ) {
            Ok(response) => {
                route_events.push(format!("generic:found={}", response.found));
                response
            }
            Err(err) => {
                route_events.push(format!("generic:error={}", short_node_error(&err)));
                last_err = Some(err);
                FetchCommitResponse {
                    found: false,
                    message: None,
                }
            }
        };
        if !response.found {
            let mut peer_ids = self.network.known_peer_ids();
            peer_ids.sort();
            peer_ids.dedup();
            peer_ids.retain(|peer_id| !peer_id.trim().is_empty());
            peer_ids.truncate(GAP_SYNC_FETCH_COMMIT_MAX_PROVIDER_ROUTES_PER_POLL);
            let peer_count = peer_ids.len();
            for (peer_index, peer_id) in peer_ids.into_iter().enumerate() {
                let provider_route = [peer_id.clone()];
                let Some((request_timeout_ms, retry_budget_ms)) = route_budget() else {
                    if last_err.is_none() {
                        last_err = Some(gap_sync_fetch_commit_route_budget_exhausted());
                    }
                    break;
                };
                let remaining_provider_routes = peer_count.saturating_sub(peer_index).max(1);
                let retry_budget_ms = split_provider_route_timeout_ms(
                    retry_budget_ms,
                    remaining_provider_routes,
                    GAP_SYNC_FETCH_COMMIT_MIN_PROVIDER_ROUTE_TIMEOUT_MS,
                );
                let request_timeout_ms = request_timeout_ms.min(retry_budget_ms);
                match self
                    .request_json_with_providers_budget::<FetchCommitRequest, FetchCommitResponse>(
                        REPLICATION_FETCH_COMMIT_PROTOCOL,
                        request,
                        provider_route.as_slice(),
                        request_timeout_ms,
                        retry_budget_ms,
                    ) {
                    Ok(candidate) => {
                        route_events.push(format!("peer:{}:found={}", peer_id, candidate.found));
                        if candidate.found {
                            response = candidate;
                            last_err = None;
                            break;
                        }
                        response = candidate;
                        last_err = None;
                    }
                    Err(err) => {
                        route_events.push(format!(
                            "peer:{}:error={}",
                            peer_id,
                            short_node_error(&err)
                        ));
                        last_err = Some(err);
                    }
                }
            }
        }
        if retry_policy == GapSyncFetchCommitRetryPolicy::FullGapSync {
            for _ in 1..FETCH_COMMIT_GENERIC_ROUTE_ATTEMPTS {
                if response.found {
                    break;
                }
                let Some((request_timeout_ms, retry_budget_ms)) = route_budget() else {
                    if last_err.is_none() {
                        last_err = Some(gap_sync_fetch_commit_route_budget_exhausted());
                    }
                    break;
                };
                match self.request_json_budget::<FetchCommitRequest, FetchCommitResponse>(
                    REPLICATION_FETCH_COMMIT_PROTOCOL,
                    request,
                    request_timeout_ms,
                    retry_budget_ms,
                ) {
                    Ok(candidate) => {
                        route_events.push(format!("generic_retry:found={}", candidate.found));
                        response = candidate;
                        last_err = None;
                    }
                    Err(err) => {
                        route_events
                            .push(format!("generic_retry:error={}", short_node_error(&err)));
                        last_err = Some(err);
                    }
                }
            }
        }
        if response.found || last_err.is_none() {
            return Ok(GapSyncFetchCommitResponse {
                response,
                repair_summary: summarize_fetch_commit_routes(&route_events),
            });
        }
        Err(last_err.expect("gap-sync fetch-commit last_err should exist"))
    }

    pub(crate) fn request_json_budget<Req, Resp>(
        &self,
        protocol: &str,
        request: &Req,
        request_timeout_ms: u64,
        retry_budget_ms: u64,
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
            .request_with_providers_budget(
                protocol,
                payload.as_slice(),
                &[],
                request_timeout_ms,
                retry_budget_ms,
            )
            .map_err(network_err)?;
        serde_json::from_slice::<Resp>(&response_bytes).map_err(|err| NodeError::Replication {
            reason: format!("decode replication response {} failed: {}", protocol, err),
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
            .map_err(|err| network_err_for_protocol(protocol, err))?;
        serde_json::from_slice::<Resp>(&response_bytes).map_err(|err| NodeError::Replication {
            reason: format!("decode replication response {} failed: {}", protocol, err),
        })
    }

    pub(crate) fn request_json_with_providers_budget<Req, Resp>(
        &self,
        protocol: &str,
        request: &Req,
        providers: &[String],
        request_timeout_ms: u64,
        retry_budget_ms: u64,
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
            .request_with_providers_budget(
                protocol,
                payload.as_slice(),
                providers,
                request_timeout_ms,
                retry_budget_ms,
            )
            .map_err(network_err)?;
        serde_json::from_slice::<Resp>(&response_bytes).map_err(|err| NodeError::Replication {
            reason: format!("decode replication response {} failed: {}", protocol, err),
        })
    }

    pub(crate) fn connected_peer_ids(&self) -> Vec<String> {
        self.network.connected_peer_ids()
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

    #[allow(dead_code)]
    pub(crate) fn publish_local_content_provider(
        &self,
        world_id: &str,
        content_hash: &str,
    ) -> Result<(), NodeError> {
        let handle = NodeReplicationNetworkHandle {
            network: Arc::clone(&self.network),
            dht: self.dht.clone(),
            local_provider_id: self.local_provider_id.clone(),
            topic: Some(self.topic.clone()),
        };
        handle.publish_local_content_provider(&self.network_policy, world_id, content_hash)
    }

    pub(crate) fn publish_local_content_provider_best_effort(
        &self,
        world_id: &str,
        content_hash: &str,
    ) {
        let Some(dht) = self.dht.as_ref() else {
            return;
        };
        let Some(local_provider_id) = self.local_provider_id.as_deref() else {
            return;
        };
        if !self
            .network_policy
            .allows_lane_operation(NetworkLane::BlobState, NetworkLaneOperation::Serve)
        {
            return;
        }
        let _ = dht.publish_provider_best_effort(world_id, content_hash, local_provider_id);
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

fn short_node_error(err: &NodeError) -> String {
    let raw = err.to_string();
    raw.chars().take(160).collect()
}

fn summarize_fetch_commit_routes(route_events: &[String]) -> String {
    if route_events.is_empty() {
        return "routes=none".to_string();
    }
    route_events.join(";")
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
#[path = "tests_network_bridge.rs"]
mod tests;

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
            .publish_best_effort(topic, payload.as_slice())
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
    network_err_with_request_protocol(None, err)
}

fn network_err_for_protocol(protocol: &str, err: WorldError) -> NodeError {
    network_err_with_request_protocol(Some(protocol), err)
}

fn network_err_with_request_protocol(protocol: Option<&str>, err: WorldError) -> NodeError {
    if world_error_is_retryable_connection_gap(&err) {
        return NodeError::Replication {
            reason: format!(
                "{REPLICATION_NETWORK_AVAILABILITY_GAP_PREFIX}{}",
                replication_network_error_detail(&err)
            ),
        };
    }
    if world_error_is_publish_failure(&err) {
        return NodeError::Replication {
            reason: format!("replication network error: {err:?}"),
        };
    }
    if let WorldError::NetworkRequestFailed { code, message, .. } = &err {
        let protocol = protocol.unwrap_or(message);
        return NodeError::Replication {
            reason: format!(
                "replication network request failed: kind={} protocol={} detail={}",
                replication_network_error_kind_label(*code),
                protocol,
                message
            ),
        };
    }
    if let WorldError::NetworkProtocolUnavailable { .. } = &err {
        return NodeError::Replication {
            reason: format!(
                "{REPLICATION_NETWORK_ROUTE_UNAVAILABLE_PREFIX}{}",
                replication_network_error_detail(&err)
            ),
        };
    }
    NodeError::Replication {
        reason: format!("replication network error: {err:?}"),
    }
}

fn world_head_lookup_can_fallback(err: &NodeError) -> bool {
    replication_network_error_is_availability_gap(err)
        || replication_network_error_is_route_unavailable(err)
        || replication_network_error_is_not_found(err)
        || replication_network_error_is_unsupported_protocol(err, REPLICATION_GET_HEAD_PROTOCOL)
        || replication_network_error_is_protocol_unavailable(err, REPLICATION_GET_HEAD_PROTOCOL)
        || replication_network_error_is_timeout_protocol(err, REPLICATION_GET_HEAD_PROTOCOL)
}

fn validate_world_head_world_id(world_id: &str, head: &WorldHeadAnnounce) -> Result<(), NodeError> {
    if head.world_id != world_id {
        return Err(NodeError::Replication {
            reason: format!(
                "world head mismatch: expected={} actual={}",
                world_id, head.world_id
            ),
        });
    }
    Ok(())
}

fn replication_network_error_detail(err: &WorldError) -> &str {
    match err {
        WorldError::NetworkProtocolUnavailable { protocol } => protocol.as_str(),
        WorldError::NetworkRequestFailed { message, .. } => message.as_str(),
        WorldError::DistributedValidationFailed { reason } => reason.as_str(),
        WorldError::BlobNotFound { content_hash } => content_hash.as_str(),
        WorldError::BlobHashMismatch { actual, .. } => actual.as_str(),
        WorldError::BlobHashInvalid { content_hash } => content_hash.as_str(),
        WorldError::Io(message) | WorldError::Serde(message) => message.as_str(),
        WorldError::SignatureKeyInvalid => "invalid signature key",
    }
}

fn split_provider_route_timeout_ms(
    retry_budget_ms: u64,
    remaining_provider_routes: usize,
    min_timeout_ms: u64,
) -> u64 {
    let remaining_provider_routes = remaining_provider_routes.max(1) as u64;
    let divided = retry_budget_ms / remaining_provider_routes;
    divided.max(min_timeout_ms).min(retry_budget_ms)
}
