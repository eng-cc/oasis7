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
                || message.contains("NetworkProtocolUnavailable")
        }
        WorldError::NetworkProtocolUnavailable { protocol } => protocol.contains("handler missing"),
        _ => false,
    }
}

fn peer_error_indicates_retryable_connection_gap(err: &WorldError) -> bool {
    match err {
        WorldError::NetworkProtocolUnavailable { protocol } => {
            protocol.contains("is not connected for protocol")
                || protocol.contains("no connected peers for protocol")
        }
        _ => false,
    }
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
        .filter(|health| health.status == "blocked")
        .filter_map(|health| health.peer_id.parse::<PeerId>().ok())
        .collect()
}

fn active_transport_peers_from_healths(healths: &[ReplicationPeerHealthDebug]) -> Vec<PeerId> {
    let peers = healths
        .iter()
        .filter(|health| health.active_path_kind.is_some())
        .filter_map(|health| health.peer_id.parse::<PeerId>().ok())
        .collect();
    let peers = dedup_sorted_peers(peers);
    let blocked_peers = blocked_peers_from_healths(healths);
    let admissible = peers
        .iter()
        .copied()
        .filter(|peer_id| !blocked_peers.contains(peer_id))
        .collect::<Vec<_>>();
    if !admissible.is_empty() {
        admissible
    } else {
        peers
    }
}

fn connected_or_active_transport_peers(
    connected_peers: Vec<PeerId>,
    healths: &[ReplicationPeerHealthDebug],
) -> Vec<PeerId> {
    let blocked_peers = blocked_peers_from_healths(healths);
    let connected_peers = dedup_sorted_peers(connected_peers);
    let admissible_connected_peers = connected_peers
        .iter()
        .copied()
        .filter(|peer_id| !blocked_peers.contains(peer_id))
        .collect::<Vec<_>>();
    if !admissible_connected_peers.is_empty() {
        return admissible_connected_peers;
    }
    if !connected_peers.is_empty() {
        return connected_peers;
    }
    active_transport_peers_from_healths(healths)
}

#[cfg(test)]
mod peer_selection_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use oasis7_proto::distributed::DistributedErrorCode;
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    fn wait_until(what: &str, deadline: Instant, mut condition: impl FnMut() -> bool) {
        while Instant::now() < deadline {
            if condition() {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("timed out waiting for condition: {what}");
    }

    fn listening_addr_with_peer_id(network: &Libp2pReplicationNetwork) -> Multiaddr {
        network
            .listening_addrs()
            .into_iter()
            .find(|addr| addr.to_string().contains("127.0.0.1"))
            .expect("listener visible addr")
            .with(libp2p::multiaddr::Protocol::P2p(network.peer_id().into()))
    }

    #[test]
    fn libp2p_replication_network_generates_peer_id() {
        let network = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig::default());
        assert!(!network.peer_id().to_string().is_empty());
    }

    #[test]
    fn libp2p_replication_network_request_rejects_without_connected_peers_by_default() {
        let network = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig::default());
        let result = network.request("/aw/node/replication/ping", b"hello");
        match result {
            Err(WorldError::NetworkProtocolUnavailable { protocol }) => {
                assert!(protocol.contains("no connected peers"));
            }
            other => panic!("expected NetworkProtocolUnavailable, got {other:?}"),
        }
    }

    #[test]
    fn libp2p_replication_network_request_falls_back_to_local_handler_when_enabled() {
        let network = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            allow_local_handler_fallback_when_no_peers: true,
            ..Libp2pReplicationNetworkConfig::default()
        });
        network
            .register_handler(
                "/aw/node/replication/ping",
                Box::new(|payload| {
                    let mut out = payload.to_vec();
                    out.extend_from_slice(b"-ok");
                    Ok(out)
                }),
            )
            .expect("register local handler");

        let response = network
            .request("/aw/node/replication/ping", b"hello")
            .expect("local request");
        assert_eq!(response, b"hello-ok".to_vec());
    }

    #[test]
    fn libp2p_replication_network_request_response_between_peers() {
        let listener = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener addr")],
            ..Libp2pReplicationNetworkConfig::default()
        });
        let listen_deadline = Instant::now() + Duration::from_secs(10);
        wait_until("listener bind", listen_deadline, || {
            !listener.listening_addrs().is_empty()
        });

        let dial_addr = listening_addr_with_peer_id(&listener);
        listener
            .register_handler(
                "/aw/node/replication/ping",
                Box::new(|payload| {
                    let mut out = payload.to_vec();
                    out.extend_from_slice(b"-pong");
                    Ok(out)
                }),
            )
            .expect("register listener handler");

        let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
            bootstrap_peers: vec![dial_addr],
            ..Libp2pReplicationNetworkConfig::default()
        });
        let connect_deadline = Instant::now() + Duration::from_secs(10);
        wait_until("dialer connection", connect_deadline, || {
            !dialer.connected_peers().is_empty()
        });

        let request_deadline = Instant::now() + Duration::from_secs(10);
        wait_until("request response", request_deadline, || {
            match dialer.request("/aw/node/replication/ping", b"node") {
                Ok(payload) => payload == b"node-pong".to_vec(),
                Err(WorldError::NetworkProtocolUnavailable { .. }) => false,
                Err(WorldError::NetworkRequestFailed { .. }) => false,
                Err(err) => panic!(
                    "unexpected request error: {err:?}; dialer_errors={:?}; listener_errors={:?}",
                    dialer.debug_errors(),
                    listener.debug_errors(),
                ),
            }
        });
    }

    #[test]
    fn libp2p_replication_network_redials_bootstrap_peer_until_listener_is_ready() {
        let reserved_listener =
            TcpListener::bind("127.0.0.1:0").expect("reserve bootstrap listener port");
        let bootstrap_port = reserved_listener
            .local_addr()
            .expect("reserved bootstrap addr")
            .port();
        drop(reserved_listener);

        let bootstrap_addr = format!("/ip4/127.0.0.1/tcp/{bootstrap_port}")
            .parse()
            .expect("bootstrap addr");
        let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
            bootstrap_peers: vec![bootstrap_addr],
            ..Libp2pReplicationNetworkConfig::default()
        });

        std::thread::sleep(Duration::from_millis(200));

        let listener = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            listen_addrs: vec![format!("/ip4/127.0.0.1/tcp/{bootstrap_port}")
                .parse()
                .expect("listener addr")],
            ..Libp2pReplicationNetworkConfig::default()
        });
        listener
            .register_handler(
                "/aw/node/replication/ping",
                Box::new(|payload| {
                    let mut out = payload.to_vec();
                    out.extend_from_slice(b"-late");
                    Ok(out)
                }),
            )
            .expect("register listener handler");

        let connect_deadline = Instant::now() + Duration::from_secs(10);
        wait_until(
            "dialer reconnects to late listener",
            connect_deadline,
            || !dialer.connected_peers().is_empty(),
        );

        let listener_deadline = Instant::now() + Duration::from_secs(10);
        wait_until(
            "late listener sees connected peer",
            listener_deadline,
            || !listener.connected_peers().is_empty(),
        );
    }

    #[test]
    fn libp2p_replication_network_request_waits_for_delayed_bootstrap_connection() {
        let reserved_listener =
            TcpListener::bind("127.0.0.1:0").expect("reserve bootstrap listener port");
        let bootstrap_port = reserved_listener
            .local_addr()
            .expect("reserved bootstrap addr")
            .port();
        drop(reserved_listener);

        let bootstrap_addr = format!("/ip4/127.0.0.1/tcp/{bootstrap_port}")
            .parse()
            .expect("bootstrap addr");
        let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
            bootstrap_peers: vec![bootstrap_addr],
            ..Libp2pReplicationNetworkConfig::default()
        });

        let listener_thread = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(250));
            let listener = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
                listen_addrs: vec![format!("/ip4/127.0.0.1/tcp/{bootstrap_port}")
                    .parse()
                    .expect("listener addr")],
                ..Libp2pReplicationNetworkConfig::default()
            });
            listener
                .register_handler(
                    "/aw/node/replication/ping",
                    Box::new(|payload| {
                        let mut out = payload.to_vec();
                        out.extend_from_slice(b"-delayed");
                        Ok(out)
                    }),
                )
                .expect("register delayed listener handler");
            std::thread::sleep(Duration::from_secs(3));
        });

        let response = dialer
            .request("/aw/node/replication/ping", b"node")
            .expect("request should wait for delayed connection");
        assert_eq!(response, b"node-delayed".to_vec());

        listener_thread
            .join()
            .expect("join delayed listener thread");
    }

    #[test]
    fn filtered_request_peers_excludes_known_unsupported_peers_without_fallback() {
        let network = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig::default());
        let observer_peer = PeerId::random();
        let sequencer_peer = PeerId::random();
        network.mark_peer_unsupported_for_protocol(
            "/aw/node/replication/fetch-commit/1.0.0",
            observer_peer,
        );

        let filtered = network.filtered_request_peers(
            "/aw/node/replication/fetch-commit/1.0.0",
            vec![observer_peer, sequencer_peer],
        );
        assert_eq!(filtered, vec![sequencer_peer]);

        let filtered_only_unsupported = network.filtered_request_peers(
            "/aw/node/replication/fetch-commit/1.0.0",
            vec![observer_peer],
        );
        assert!(filtered_only_unsupported.is_empty());
    }

    #[test]
    fn filtered_request_peers_retries_unsupported_peer_after_retry_window() {
        let network = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            unsupported_protocol_retry_after: Duration::from_millis(5),
            ..Libp2pReplicationNetworkConfig::default()
        });
        let sequencer_peer = PeerId::random();
        network.mark_peer_unsupported_for_protocol(
            "/aw/node/replication/fetch-commit/1.0.0",
            sequencer_peer,
        );

        let filtered_initial = network.filtered_request_peers(
            "/aw/node/replication/fetch-commit/1.0.0",
            vec![sequencer_peer],
        );
        assert!(filtered_initial.is_empty());

        std::thread::sleep(Duration::from_millis(15));

        let filtered_after_retry_window = network.filtered_request_peers(
            "/aw/node/replication/fetch-commit/1.0.0",
            vec![sequencer_peer],
        );
        assert_eq!(filtered_after_retry_window, vec![sequencer_peer]);
    }

    #[test]
    fn libp2p_replication_network_request_round_robins_across_connected_peers() {
        let listener_a = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener a addr")],
            ..Libp2pReplicationNetworkConfig::default()
        });
        let listener_b = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener b addr")],
            ..Libp2pReplicationNetworkConfig::default()
        });
        let listen_deadline = Instant::now() + Duration::from_secs(10);
        wait_until("listener a bind", listen_deadline, || {
            !listener_a.listening_addrs().is_empty()
        });
        wait_until("listener b bind", listen_deadline, || {
            !listener_b.listening_addrs().is_empty()
        });

        listener_a
            .register_handler(
                "/aw/node/replication/ping",
                Box::new(|payload| {
                    let mut out = payload.to_vec();
                    out.extend_from_slice(b"-a");
                    Ok(out)
                }),
            )
            .expect("register listener a handler");
        listener_b
            .register_handler(
                "/aw/node/replication/ping",
                Box::new(|payload| {
                    let mut out = payload.to_vec();
                    out.extend_from_slice(b"-b");
                    Ok(out)
                }),
            )
            .expect("register listener b handler");

        let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
            bootstrap_peers: vec![
                listening_addr_with_peer_id(&listener_a),
                listening_addr_with_peer_id(&listener_b),
            ],
            ..Libp2pReplicationNetworkConfig::default()
        });
        let connect_deadline = Instant::now() + Duration::from_secs(10);
        wait_until("dialer connects to two peers", connect_deadline, || {
            dialer.connected_peers().len() >= 2
        });

        let first = dialer
            .request("/aw/node/replication/ping", b"node")
            .expect("first request");
        let second = dialer
            .request("/aw/node/replication/ping", b"node")
            .expect("second request");

        assert_ne!(
            first, second,
            "expected round-robin request targets to differ"
        );
        let mut responses = vec![first, second];
        responses.sort();
        assert_eq!(responses, vec![b"node-a".to_vec(), b"node-b".to_vec()]);
    }

    #[test]
    fn libp2p_replication_network_request_retries_next_peer_when_remote_handler_fails() {
        let listener_fail = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener fail addr")],
            ..Libp2pReplicationNetworkConfig::default()
        });
        let listener_ok = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener ok addr")],
            ..Libp2pReplicationNetworkConfig::default()
        });
        let listen_deadline = Instant::now() + Duration::from_secs(10);
        wait_until("listener fail bind", listen_deadline, || {
            !listener_fail.listening_addrs().is_empty()
        });
        wait_until("listener ok bind", listen_deadline, || {
            !listener_ok.listening_addrs().is_empty()
        });

        listener_fail
            .register_handler(
                "/aw/node/replication/ping",
                Box::new(|_payload| {
                    Err(WorldError::NetworkRequestFailed {
                        code: DistributedErrorCode::ErrUnsupported,
                        message: "forced failure".to_string(),
                        retryable: false,
                    })
                }),
            )
            .expect("register listener fail handler");
        listener_ok
            .register_handler(
                "/aw/node/replication/ping",
                Box::new(|payload| {
                    let mut out = payload.to_vec();
                    out.extend_from_slice(b"-ok");
                    Ok(out)
                }),
            )
            .expect("register listener ok handler");

        let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
            bootstrap_peers: vec![
                listening_addr_with_peer_id(&listener_fail),
                listening_addr_with_peer_id(&listener_ok),
            ],
            ..Libp2pReplicationNetworkConfig::default()
        });
        let connect_deadline = Instant::now() + Duration::from_secs(10);
        wait_until("dialer connects to two peers", connect_deadline, || {
            dialer.connected_peers().len() >= 2
        });

        let first = dialer
            .request("/aw/node/replication/ping", b"node")
            .expect("first request should succeed via retry");
        let second = dialer
            .request("/aw/node/replication/ping", b"node")
            .expect("second request should succeed via retry");

        assert_eq!(first, b"node-ok".to_vec());
        assert_eq!(second, b"node-ok".to_vec());
    }

    #[test]
    fn libp2p_replication_network_retries_previously_unsupported_single_peer_after_retry_window() {
        let listener = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener addr")],
            ..Libp2pReplicationNetworkConfig::default()
        });
        let listen_deadline = Instant::now() + Duration::from_secs(10);
        wait_until("listener bind", listen_deadline, || {
            !listener.listening_addrs().is_empty()
        });

        listener
            .register_handler(
                "/aw/node/replication/ping",
                Box::new(move |payload| {
                    let mut out = payload.to_vec();
                    out.extend_from_slice(b"-recovered");
                    Ok(out)
                }),
            )
            .expect("register listener handler");

        let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
            bootstrap_peers: vec![listening_addr_with_peer_id(&listener)],
            unsupported_protocol_retry_after: Duration::from_millis(25),
            ..Libp2pReplicationNetworkConfig::default()
        });
        let connect_deadline = Instant::now() + Duration::from_secs(10);
        wait_until("dialer connection", connect_deadline, || {
            !dialer.connected_peers().is_empty()
        });

        let listener_peer_id = listener.peer_id();
        dialer.mark_peer_unsupported_for_protocol("/aw/node/replication/ping", listener_peer_id);

        let immediate_retry = dialer.request("/aw/node/replication/ping", b"node");
        assert!(matches!(
            immediate_retry,
            Err(WorldError::NetworkProtocolUnavailable { protocol })
                if protocol.contains("no connected peers")
        ));

        std::thread::sleep(Duration::from_millis(60));

        let recovered = dialer
            .request("/aw/node/replication/ping", b"node")
            .expect("request after retry window");
        assert_eq!(recovered, b"node-recovered".to_vec());
    }

    #[test]
    fn libp2p_replication_network_does_not_quarantine_not_found_response_as_unsupported() {
        let listener = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener addr")],
            ..Libp2pReplicationNetworkConfig::default()
        });
        let listen_deadline = Instant::now() + Duration::from_secs(10);
        wait_until("listener bind", listen_deadline, || {
            !listener.listening_addrs().is_empty()
        });

        listener
            .register_handler(
                "/aw/node/replication/ping",
                Box::new(|_payload| {
                    Err(WorldError::NetworkRequestFailed {
                        code: DistributedErrorCode::ErrNotFound,
                        message: "missing content".to_string(),
                        retryable: false,
                    })
                }),
            )
            .expect("register listener handler");

        let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
            bootstrap_peers: vec![listening_addr_with_peer_id(&listener)],
            unsupported_protocol_retry_after: Duration::from_millis(250),
            ..Libp2pReplicationNetworkConfig::default()
        });
        let connect_deadline = Instant::now() + Duration::from_secs(10);
        wait_until("dialer connection", connect_deadline, || {
            !dialer.connected_peers().is_empty()
        });

        let first = dialer.request("/aw/node/replication/ping", b"node");
        assert!(matches!(
            first,
            Err(WorldError::NetworkRequestFailed {
                code: DistributedErrorCode::ErrNotFound,
                ..
            })
        ));

        let second = dialer.request("/aw/node/replication/ping", b"node");
        assert!(matches!(
            second,
            Err(WorldError::NetworkRequestFailed {
                code: DistributedErrorCode::ErrNotFound,
                ..
            })
        ));
    }

    #[test]
    fn retryable_connection_gap_detection_matches_request_to_peer_disconnects() {
        let err = WorldError::NetworkProtocolUnavailable {
            protocol: "libp2p-replication outbound request failed: NetworkProtocolUnavailable { protocol: \"peer 12D3KooW... is not connected for protocol /aw/node/replication/fetch-commit/1.0.0\" }".to_string(),
        };
        assert!(peer_error_indicates_retryable_connection_gap(&err));

        let not_retryable = WorldError::NetworkProtocolUnavailable {
            protocol: "libp2p-replication handler missing: /aw/node/replication/fetch-commit/1.0.0"
                .to_string(),
        };
        assert!(!peer_error_indicates_retryable_connection_gap(
            &not_retryable
        ));
    }

    #[test]
    fn libp2p_replication_network_preserves_remote_unsupported_error_code() {
        let listener = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("listener addr")],
            ..Libp2pReplicationNetworkConfig::default()
        });
        let listen_deadline = Instant::now() + Duration::from_secs(10);
        wait_until("listener bind", listen_deadline, || {
            !listener.listening_addrs().is_empty()
        });

        listener
            .register_handler(
                "/aw/node/replication/ping",
                Box::new(|_payload| {
                    Err(WorldError::NetworkRequestFailed {
                        code: DistributedErrorCode::ErrUnsupported,
                        message: "forced unsupported".to_string(),
                        retryable: false,
                    })
                }),
            )
            .expect("register listener handler");

        let dialer = Libp2pReplicationNetwork::new(Libp2pReplicationNetworkConfig {
            listen_addrs: vec!["/ip4/127.0.0.1/tcp/0".parse().expect("dialer addr")],
            bootstrap_peers: vec![listening_addr_with_peer_id(&listener)],
            unsupported_protocol_retry_after: Duration::from_millis(250),
            ..Libp2pReplicationNetworkConfig::default()
        });
        let connect_deadline = Instant::now() + Duration::from_secs(10);
        wait_until("dialer connection", connect_deadline, || {
            !dialer.connected_peers().is_empty()
        });

        let err = dialer
            .request("/aw/node/replication/ping", b"node")
            .expect_err("unsupported remote handler must bubble its code");
        assert!(matches!(
            err,
            WorldError::NetworkRequestFailed {
                code: DistributedErrorCode::ErrUnsupported,
                ..
            }
        ));
    }
}
