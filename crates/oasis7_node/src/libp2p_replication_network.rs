use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use libp2p::identity::Keypair;
use libp2p::{Multiaddr, PeerId};
use oasis7_net::{Libp2pNetwork, Libp2pNetworkConfig, Libp2pReachabilitySnapshot};
use oasis7_proto::distributed::WorldHeadAnnounce;
use oasis7_proto::distributed::{DistributedErrorCode, ErrorResponse};
use oasis7_proto::distributed_dht::{
    DistributedDht, MembershipDirectorySnapshot, PeerDiscoverySource, PeerRecord, ProviderRecord,
    SignedPeerRecord,
};
use oasis7_proto::distributed_net::{
    DistributedNetwork as ProtoDistributedNetwork, NetworkSubscription,
};
use oasis7_proto::world_error::WorldError;

use crate::NodeError;

type Handler = Arc<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicationPeerHealthDebug {
    pub peer_id: String,
    pub status: String,
    pub issues: Vec<String>,
    pub discovery_sources: Vec<String>,
    pub active_path_kind: Option<String>,
    pub source_operator: Option<String>,
    pub source_asn: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicationNetworkDebugSnapshot {
    pub local_peer_id: String,
    pub connected_peers: Vec<String>,
    pub peer_healths: Vec<ReplicationPeerHealthDebug>,
    pub registered_protocols: Vec<String>,
    pub unsupported_protocol_peers: HashMap<String, Vec<String>>,
    pub recent_errors: Vec<String>,
}

const REQUEST_CONNECTED_PEER_WAIT_RETRIES: usize = 12;
const REQUEST_CONNECTED_PEER_WAIT_INTERVAL_MS: u64 = 150;
const REQUEST_CONNECTION_REFRESH_RETRIES: usize = 12;
const UNSUPPORTED_PROTOCOL_RETRY_AFTER_MS: u64 = 5_000;

#[derive(Debug, Clone)]
pub struct Libp2pReplicationNetworkConfig {
    pub keypair: Option<Keypair>,
    pub peer_record: Option<PeerRecord>,
    pub listen_addrs: Vec<Multiaddr>,
    pub bootstrap_peers: Vec<Multiaddr>,
    pub allow_local_handler_fallback_when_no_peers: bool,
    pub unsupported_protocol_retry_after: Duration,
}

impl Default for Libp2pReplicationNetworkConfig {
    fn default() -> Self {
        Self {
            keypair: None,
            peer_record: None,
            listen_addrs: Vec::new(),
            bootstrap_peers: Vec::new(),
            allow_local_handler_fallback_when_no_peers: false,
            unsupported_protocol_retry_after: Duration::from_millis(
                UNSUPPORTED_PROTOCOL_RETRY_AFTER_MS,
            ),
        }
    }
}

#[derive(Clone)]
pub struct Libp2pReplicationNetwork {
    inner: Libp2pNetwork,
    allow_local_handler_fallback_when_no_peers: bool,
    handlers: Arc<Mutex<HashMap<String, Handler>>>,
    request_peer_cursor: Arc<AtomicUsize>,
    unsupported_protocol_retry_after: Duration,
    unsupported_protocol_peers: Arc<Mutex<HashMap<String, HashMap<PeerId, Instant>>>>,
}

impl Libp2pReplicationNetwork {
    pub fn new(config: Libp2pReplicationNetworkConfig) -> Self {
        let inner = Libp2pNetwork::new(Libp2pNetworkConfig {
            keypair: config.keypair,
            peer_record: config.peer_record,
            listen_addrs: config.listen_addrs,
            bootstrap_peers: config.bootstrap_peers,
            ..Libp2pNetworkConfig::default()
        });

        Self {
            inner,
            allow_local_handler_fallback_when_no_peers: config
                .allow_local_handler_fallback_when_no_peers,
            handlers: Arc::new(Mutex::new(HashMap::new())),
            request_peer_cursor: Arc::new(AtomicUsize::new(0)),
            unsupported_protocol_retry_after: config.unsupported_protocol_retry_after,
            unsupported_protocol_peers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn peer_id(&self) -> PeerId {
        self.inner.peer_id()
    }

    #[cfg(test)]
    pub fn listening_addrs(&self) -> Vec<Multiaddr> {
        self.inner.listening_addrs()
    }

    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.inner.connected_peers()
    }

    pub fn debug_errors(&self) -> Vec<String> {
        self.inner.debug_errors()
    }

    pub fn debug_snapshot(&self) -> ReplicationNetworkDebugSnapshot {
        let local_peer_id = self.peer_id().to_string();
        let mut connected_peers: Vec<String> = self
            .inner
            .connected_peers()
            .into_iter()
            .map(|peer_id| peer_id.to_string())
            .collect();
        connected_peers.sort();
        connected_peers.dedup();

        let mut peer_healths: Vec<ReplicationPeerHealthDebug> = self
            .inner
            .debug_peer_healths()
            .into_iter()
            .map(|health| {
                let mut discovery_sources: Vec<String> = health
                    .discovery_sources
                    .into_iter()
                    .map(replication_discovery_source_label)
                    .collect();
                discovery_sources.sort();
                discovery_sources.dedup();
                ReplicationPeerHealthDebug {
                    peer_id: health.peer_id,
                    status: replication_peer_health_status_label(health.status),
                    issues: health
                        .issues
                        .into_iter()
                        .map(replication_peer_health_issue_label)
                        .collect(),
                    discovery_sources,
                    active_path_kind: health.active_path_kind,
                    source_operator: health.source_operator,
                    source_asn: health.source_asn,
                }
            })
            .collect();
        peer_healths.sort_by(|left, right| left.peer_id.cmp(&right.peer_id));

        let mut unsupported_protocol_peers: HashMap<String, Vec<String>> = self
            .unsupported_protocol_peers
            .lock()
            .expect("lock unsupported protocol peers")
            .iter()
            .map(|(protocol, peers)| {
                let mut peer_ids: Vec<String> = peers.keys().map(PeerId::to_string).collect();
                peer_ids.sort();
                peer_ids.dedup();
                (protocol.clone(), peer_ids)
            })
            .collect();
        for peer_ids in unsupported_protocol_peers.values_mut() {
            peer_ids.sort();
            peer_ids.dedup();
        }

        let mut recent_errors = self.inner.debug_errors();
        recent_errors.sort();
        recent_errors.dedup();

        let mut registered_protocols: Vec<String> = self
            .handlers
            .lock()
            .expect("lock libp2p replication handlers")
            .keys()
            .cloned()
            .collect();
        registered_protocols.sort();
        registered_protocols.dedup();

        ReplicationNetworkDebugSnapshot {
            local_peer_id,
            connected_peers,
            peer_healths,
            registered_protocols,
            unsupported_protocol_peers,
            recent_errors,
        }
    }

    pub fn reachability_snapshot(&self) -> Libp2pReachabilitySnapshot {
        self.inner.reachability_snapshot()
    }

    fn local_handler(&self, protocol: &str) -> Option<Handler> {
        self.handlers
            .lock()
            .expect("lock libp2p replication handlers")
            .get(protocol)
            .cloned()
    }

    fn request_via_peer(
        &self,
        protocol: &str,
        payload: &[u8],
        peer: PeerId,
    ) -> Result<Vec<u8>, WorldError> {
        let response = self
            .inner
            .request_to_peer(protocol, payload, peer)
            .map_err(|err| WorldError::NetworkProtocolUnavailable {
                protocol: format!("libp2p-replication outbound request failed: {err:?}"),
            })?;

        if let Some(remote_error) = decode_error_response(response.as_slice()) {
            return Err(WorldError::NetworkRequestFailed {
                code: remote_error.code,
                message: remote_error.message,
                retryable: remote_error.retryable,
            });
        }

        Ok(response)
    }

    fn filtered_request_peers(&self, protocol: &str, ordered_peers: Vec<PeerId>) -> Vec<PeerId> {
        let now = Instant::now();
        let mut unsupported = self
            .unsupported_protocol_peers
            .lock()
            .expect("lock unsupported protocol peers");
        let Some(peers) = unsupported.get_mut(protocol) else {
            return ordered_peers;
        };
        peers.retain(|_, unsupported_until| *unsupported_until > now);
        let keep_protocol_entry = !peers.is_empty();
        let filtered: Vec<PeerId> = ordered_peers
            .iter()
            .copied()
            .filter(|peer| !peers.contains_key(peer))
            .collect();
        if !keep_protocol_entry {
            unsupported.remove(protocol);
        }
        filtered
    }

    fn mark_peer_unsupported_for_protocol(&self, protocol: &str, peer: PeerId) {
        let unsupported_until = Instant::now() + self.unsupported_protocol_retry_after;
        self.unsupported_protocol_peers
            .lock()
            .expect("lock unsupported protocol peers")
            .entry(protocol.to_string())
            .or_default()
            .insert(peer, unsupported_until);
    }

    fn call_local_handler(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        match self.local_handler(protocol) {
            Some(handler) => handler(payload),
            None => Err(WorldError::NetworkProtocolUnavailable {
                protocol: format!("libp2p-replication handler missing: {protocol}"),
            }),
        }
    }

    fn wait_for_connected_peers(&self) -> Vec<PeerId> {
        let mut peers = self.connected_peers_sorted();
        for _ in 0..REQUEST_CONNECTED_PEER_WAIT_RETRIES {
            if !peers.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(
                REQUEST_CONNECTED_PEER_WAIT_INTERVAL_MS,
            ));
            peers = self.connected_peers_sorted();
        }
        peers
    }

    fn connected_peers_sorted(&self) -> Vec<PeerId> {
        connected_or_active_transport_peers(
            self.inner.connected_peers(),
            self.debug_snapshot().peer_healths.as_slice(),
        )
    }

    fn collect_connected_provider_peers(&self, providers: &[String]) -> Vec<PeerId> {
        let connected_peers: HashSet<PeerId> = self.connected_peers_sorted().into_iter().collect();
        let mut ordered_provider_peers = Vec::new();
        let mut seen = HashSet::new();
        for provider in providers {
            let Ok(peer_id) = provider.parse::<PeerId>() else {
                continue;
            };
            if !connected_peers.contains(&peer_id) || !seen.insert(peer_id) {
                continue;
            }
            ordered_provider_peers.push(peer_id);
        }
        ordered_provider_peers
    }

    fn wait_for_connected_provider_peers(&self, providers: &[String]) -> Vec<PeerId> {
        for attempt in 0..=REQUEST_CONNECTED_PEER_WAIT_RETRIES {
            let ordered_provider_peers = self.collect_connected_provider_peers(providers);
            if !ordered_provider_peers.is_empty() || attempt == REQUEST_CONNECTED_PEER_WAIT_RETRIES
            {
                return ordered_provider_peers;
            }
            std::thread::sleep(Duration::from_millis(
                REQUEST_CONNECTED_PEER_WAIT_INTERVAL_MS,
            ));
        }
        Vec::new()
    }

    fn request_over_refreshed_peers<F, G>(
        &self,
        protocol: &str,
        payload: &[u8],
        initial_peers: Vec<PeerId>,
        mut refresh_peers: F,
        no_connected_error: G,
    ) -> Result<Vec<u8>, WorldError>
    where
        F: FnMut() -> Vec<PeerId>,
        G: Fn() -> WorldError,
    {
        let cursor = self.request_peer_cursor.fetch_add(1, Ordering::Relaxed);
        let mut last_error = None;

        for attempt in 0..=REQUEST_CONNECTION_REFRESH_RETRIES {
            let candidate_source = if attempt == 0 {
                initial_peers.clone()
            } else {
                refresh_peers()
            };
            let ordered_peers = self.filtered_request_peers(
                protocol,
                rotated_peers(candidate_source.as_slice(), cursor.saturating_add(attempt)),
            );
            if ordered_peers.is_empty() {
                let should_retry = attempt < REQUEST_CONNECTION_REFRESH_RETRIES
                    && last_error
                        .as_ref()
                        .map(peer_error_indicates_retryable_connection_gap)
                        .unwrap_or(false);
                if should_retry {
                    std::thread::sleep(Duration::from_millis(
                        REQUEST_CONNECTED_PEER_WAIT_INTERVAL_MS,
                    ));
                    continue;
                }
                return Err(last_error.unwrap_or_else(&no_connected_error));
            }

            let mut retryable_connection_gap = false;
            for peer in ordered_peers {
                match self.request_via_peer(protocol, payload, peer) {
                    Ok(reply) => return Ok(reply),
                    Err(err) => {
                        if peer_error_indicates_unsupported_protocol(&err) {
                            self.mark_peer_unsupported_for_protocol(protocol, peer);
                        }
                        retryable_connection_gap |=
                            peer_error_indicates_retryable_connection_gap(&err);
                        last_error = Some(err);
                    }
                }
            }

            if retryable_connection_gap && attempt < REQUEST_CONNECTION_REFRESH_RETRIES {
                std::thread::sleep(Duration::from_millis(
                    REQUEST_CONNECTED_PEER_WAIT_INTERVAL_MS,
                ));
                continue;
            }

            return Err(last_error.unwrap_or_else(&no_connected_error));
        }

        Err(last_error.unwrap_or_else(no_connected_error))
    }
}

pub fn derive_libp2p_identity_keypair(private_key_hex: &str) -> Result<Keypair, NodeError> {
    let private_key_bytes = hex::decode(private_key_hex).map_err(|_| NodeError::InvalidConfig {
        reason: "node.private_key must be valid hex for libp2p identity derivation".to_string(),
    })?;
    Keypair::ed25519_from_bytes(private_key_bytes).map_err(|err| NodeError::InvalidConfig {
        reason: format!("failed to derive libp2p identity keypair: {err}"),
    })
}

impl ProtoDistributedNetwork<WorldError> for Libp2pReplicationNetwork {
    fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), WorldError> {
        self.inner.publish(topic, payload)
    }

    fn subscribe(&self, topic: &str) -> Result<NetworkSubscription, WorldError> {
        self.inner.subscribe(topic)
    }

    fn request(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        let peers = self.wait_for_connected_peers();

        if peers.is_empty() {
            if self.allow_local_handler_fallback_when_no_peers {
                return self.call_local_handler(protocol, payload);
            }
            if !self.inner.connected_peers().is_empty() {
                return Err(WorldError::NetworkProtocolUnavailable {
                    protocol: format!(
                        "libp2p-replication no admissible connected peers for protocol {protocol}"
                    ),
                });
            }
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: format!("libp2p-replication no connected peers for protocol {protocol}"),
            });
        }

        self.request_over_refreshed_peers(
            protocol,
            payload,
            peers,
            || self.connected_peers_sorted(),
            || WorldError::NetworkProtocolUnavailable {
                protocol: format!("libp2p-replication no connected peers for protocol {protocol}"),
            },
        )
    }

    fn request_with_providers(
        &self,
        protocol: &str,
        payload: &[u8],
        providers: &[String],
    ) -> Result<Vec<u8>, WorldError> {
        if providers.is_empty() {
            return self.request(protocol, payload);
        }

        let ordered_provider_peers = self.wait_for_connected_provider_peers(providers);
        if ordered_provider_peers.is_empty() {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: format!(
                    "libp2p-replication no connected providers for protocol {protocol}"
                ),
            });
        }

        self.request_over_refreshed_peers(
            protocol,
            payload,
            ordered_provider_peers,
            || self.collect_connected_provider_peers(providers),
            || WorldError::NetworkProtocolUnavailable {
                protocol: format!(
                    "libp2p-replication no connected providers for protocol {protocol}"
                ),
            },
        )
    }

    fn register_handler(
        &self,
        protocol: &str,
        handler: Box<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>,
    ) -> Result<(), WorldError> {
        let handler: Handler = Arc::from(handler);
        self.inner.register_handler(
            protocol,
            Box::new({
                let handler = Arc::clone(&handler);
                move |payload| handler(payload)
            }),
        )?;

        self.handlers
            .lock()
            .expect("lock libp2p replication handlers")
            .insert(protocol.to_string(), handler);
        Ok(())
    }
}

impl DistributedDht<WorldError> for Libp2pReplicationNetwork {
    fn publish_provider(
        &self,
        world_id: &str,
        content_hash: &str,
        provider_id: &str,
    ) -> Result<(), WorldError> {
        self.inner
            .publish_provider(world_id, content_hash, provider_id)
    }

    fn get_providers(
        &self,
        world_id: &str,
        content_hash: &str,
    ) -> Result<Vec<ProviderRecord>, WorldError> {
        self.inner.get_providers(world_id, content_hash)
    }

    fn put_world_head(&self, world_id: &str, head: &WorldHeadAnnounce) -> Result<(), WorldError> {
        self.inner.put_world_head(world_id, head)
    }

    fn get_world_head(&self, world_id: &str) -> Result<Option<WorldHeadAnnounce>, WorldError> {
        self.inner.get_world_head(world_id)
    }

    fn put_membership_directory(
        &self,
        world_id: &str,
        snapshot: &MembershipDirectorySnapshot,
    ) -> Result<(), WorldError> {
        self.inner.put_membership_directory(world_id, snapshot)
    }

    fn get_membership_directory(
        &self,
        world_id: &str,
    ) -> Result<Option<MembershipDirectorySnapshot>, WorldError> {
        self.inner.get_membership_directory(world_id)
    }

    fn put_peer_record(&self, world_id: &str, record: &SignedPeerRecord) -> Result<(), WorldError> {
        self.inner.put_peer_record(world_id, record)
    }

    fn get_peer_record(
        &self,
        world_id: &str,
        peer_id: &str,
    ) -> Result<Option<SignedPeerRecord>, WorldError> {
        self.inner.get_peer_record(world_id, peer_id)
    }
}

fn decode_error_response(payload: &[u8]) -> Option<ErrorResponse> {
    serde_cbor::from_slice(payload).ok()
}

fn replication_discovery_source_label(source: PeerDiscoverySource) -> String {
    match source {
        PeerDiscoverySource::StaticBootstrap => "static_bootstrap".to_string(),
        PeerDiscoverySource::Dht => "dht".to_string(),
        PeerDiscoverySource::Rendezvous => "rendezvous".to_string(),
        PeerDiscoverySource::PeerExchange => "peer_exchange".to_string(),
        PeerDiscoverySource::Manual => "manual".to_string(),
    }
}

fn replication_peer_health_status_label(status: oasis7_net::PeerManagerHealthStatus) -> String {
    match status {
        oasis7_net::PeerManagerHealthStatus::Active => "active".to_string(),
        oasis7_net::PeerManagerHealthStatus::Candidate => "candidate".to_string(),
        oasis7_net::PeerManagerHealthStatus::Suspect => "suspect".to_string(),
        oasis7_net::PeerManagerHealthStatus::Blocked => "blocked".to_string(),
    }
}

fn replication_peer_health_issue_label(issue: oasis7_net::PeerManagerHealthIssue) -> String {
    match issue {
        oasis7_net::PeerManagerHealthIssue::MissingPeerRecord => "missing_peer_record".to_string(),
        oasis7_net::PeerManagerHealthIssue::SingleSourceDiscovery {
            observed_sources,
            required_sources,
        } => format!(
            "single_source_discovery observed={observed_sources} required={required_sources}"
        ),
        oasis7_net::PeerManagerHealthIssue::InsufficientActiveDiscoverySources {
            observed_sources,
            required_sources,
        } => format!(
            "insufficient_active_discovery_sources observed={observed_sources} required={required_sources}"
        ),
        oasis7_net::PeerManagerHealthIssue::Ipv4SubnetConcentration {
            subnet,
            peers_in_bucket,
            active_peer_count,
            limit_per_mille,
        } => format!(
            "ipv4_subnet_concentration subnet={subnet} peers_in_bucket={peers_in_bucket} active_peer_count={active_peer_count} limit_per_mille={limit_per_mille}"
        ),
        oasis7_net::PeerManagerHealthIssue::RelayDomainConcentration {
            relay_domain,
            peers_in_bucket,
            active_peer_count,
            limit_per_mille,
        } => format!(
            "relay_domain_concentration relay_domain={relay_domain} peers_in_bucket={peers_in_bucket} active_peer_count={active_peer_count} limit_per_mille={limit_per_mille}"
        ),
        oasis7_net::PeerManagerHealthIssue::OperatorConcentration {
            source_operator,
            peers_in_bucket,
            active_peer_count,
            limit_per_mille,
        } => format!(
            "operator_concentration source_operator={source_operator} peers_in_bucket={peers_in_bucket} active_peer_count={active_peer_count} limit_per_mille={limit_per_mille}"
        ),
        oasis7_net::PeerManagerHealthIssue::AsnConcentration {
            source_asn,
            peers_in_bucket,
            active_peer_count,
            limit_per_mille,
        } => format!(
            "asn_concentration source_asn={source_asn} peers_in_bucket={peers_in_bucket} active_peer_count={active_peer_count} limit_per_mille={limit_per_mille}"
        ),
        oasis7_net::PeerManagerHealthIssue::RelayBudgetExceeded {
            relayed_active_peers,
            active_peer_count,
            limit_per_mille,
        } => format!(
            "relay_budget_exceeded relayed_active_peers={relayed_active_peers} active_peer_count={active_peer_count} limit_per_mille={limit_per_mille}"
        ),
    }
}

fn peer_error_indicates_unsupported_protocol(err: &WorldError) -> bool {
    match err {
        WorldError::NetworkRequestFailed { code, message, .. } => {
            matches!(code, DistributedErrorCode::ErrUnsupported)
                && (message.contains("handler missing") || message.starts_with('/'))
        }
        WorldError::NetworkProtocolUnavailable { protocol } => protocol.contains("handler missing"),
        _ => false,
    }
}

fn peer_error_indicates_retryable_connection_gap(err: &WorldError) -> bool {
    match err {
        WorldError::NetworkProtocolUnavailable { protocol } => {
            peer_error_message_indicates_retryable_connection_gap(protocol)
        }
        WorldError::NetworkRequestFailed { message, .. } => {
            peer_error_message_indicates_retryable_connection_gap(message)
        }
        _ => false,
    }
}

fn peer_error_message_indicates_retryable_connection_gap(message: &str) -> bool {
    message.contains("is not connected for protocol")
        || message.contains("no connected peers for protocol")
        || message.contains("no admissible connected peers for protocol")
        || message.contains("no connected providers for protocol")
        || message.contains("no healthy provider for protocol")
        || message.contains("no healthy connected providers for protocol")
        || message.contains("request failed: ConnectionClosed")
        || message.contains("request failed: DialFailure")
        || message.contains("request failed: Timeout")
}

fn rotated_peers(peers: &[PeerId], cursor: usize) -> Vec<PeerId> {
    if peers.is_empty() {
        return Vec::new();
    }
    let start = cursor % peers.len();
    peers[start..]
        .iter()
        .chain(peers[..start].iter())
        .copied()
        .collect()
}

fn dedup_sorted_peers(mut peers: Vec<PeerId>) -> Vec<PeerId> {
    peers.sort_by_key(|peer| peer.to_string());
    peers.dedup();
    peers
}

fn blocked_peers_from_healths(healths: &[ReplicationPeerHealthDebug]) -> HashSet<PeerId> {
    healths
        .iter()
        .filter(|health| peer_is_request_blocked(health))
        .filter_map(|health| health.peer_id.parse::<PeerId>().ok())
        .collect()
}

fn soft_deprioritized_peers_from_healths(
    healths: &[ReplicationPeerHealthDebug],
) -> HashSet<PeerId> {
    healths
        .iter()
        .filter(|health| peer_is_soft_deprioritized_for_requests(health))
        .filter_map(|health| health.peer_id.parse::<PeerId>().ok())
        .collect()
}

fn peer_is_request_blocked(health: &ReplicationPeerHealthDebug) -> bool {
    health.status == "blocked"
        && !health.issues.is_empty()
        && !health
            .issues
            .iter()
            .all(|issue| issue_is_soft_bootstrap_constraint(issue))
}

fn peer_is_soft_deprioritized_for_requests(health: &ReplicationPeerHealthDebug) -> bool {
    health.status == "blocked"
        && !health.issues.is_empty()
        && health
            .issues
            .iter()
            .all(|issue| issue_is_soft_bootstrap_constraint(issue))
        && health
            .issues
            .iter()
            .any(|issue| issue == "missing_peer_record")
}

fn issue_is_soft_bootstrap_constraint(issue: &str) -> bool {
    issue == "missing_peer_record"
        || issue.starts_with("insufficient_active_discovery_sources ")
        || issue.starts_with("single_source_discovery ")
}

fn request_candidate_peers(
    peers: Vec<PeerId>,
    healths: &[ReplicationPeerHealthDebug],
) -> Vec<PeerId> {
    let blocked_peers = blocked_peers_from_healths(healths);
    let soft_deprioritized_peers = soft_deprioritized_peers_from_healths(healths);
    let preferred = peers
        .iter()
        .copied()
        .filter(|peer_id| {
            !blocked_peers.contains(peer_id) && !soft_deprioritized_peers.contains(peer_id)
        })
        .collect::<Vec<_>>();
    if !preferred.is_empty() {
        return preferred;
    }
    peers
        .into_iter()
        .filter(|peer_id| !blocked_peers.contains(peer_id))
        .collect()
}

fn active_transport_peers_from_healths(healths: &[ReplicationPeerHealthDebug]) -> Vec<PeerId> {
    let peers = healths
        .iter()
        .filter(|health| health.active_path_kind.is_some())
        .filter_map(|health| health.peer_id.parse::<PeerId>().ok())
        .collect();
    let peers = dedup_sorted_peers(peers);
    request_candidate_peers(peers, healths)
}

fn connected_or_active_transport_peers(
    connected_peers: Vec<PeerId>,
    healths: &[ReplicationPeerHealthDebug],
) -> Vec<PeerId> {
    let connected_peers = dedup_sorted_peers(connected_peers);
    let admissible_connected_peers = request_candidate_peers(connected_peers.clone(), healths);
    if !admissible_connected_peers.is_empty() {
        return admissible_connected_peers;
    }
    if !connected_peers.is_empty() {
        return Vec::new();
    }
    active_transport_peers_from_healths(healths)
}

#[cfg(test)]
mod peer_selection_tests;

#[cfg(test)]
mod tests;
