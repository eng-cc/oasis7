use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use libp2p::identity::Keypair;
use libp2p::{Multiaddr, PeerId};
use oasis7_net::{Libp2pNetwork, Libp2pNetworkConfig, Libp2pReachabilitySnapshot};
use oasis7_proto::distributed::WorldHeadAnnounce;
use oasis7_proto::distributed::{DistributedErrorCode, ErrorResponse};
use oasis7_proto::distributed_dht::{
    DistributedDht, MembershipDirectorySnapshot, PeerRecord, ProviderRecord, SignedPeerRecord,
};
use oasis7_proto::distributed_net::{
    DistributedNetwork as ProtoDistributedNetwork, NetworkSubscription,
};
use oasis7_proto::world_error::WorldError;

use crate::NodeError;

type Handler = Arc<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>;

const REQUEST_CONNECTED_PEER_WAIT_RETRIES: usize = 12;
const REQUEST_CONNECTED_PEER_WAIT_INTERVAL_MS: u64 = 150;

#[derive(Debug, Clone)]
pub struct Libp2pReplicationNetworkConfig {
    pub keypair: Option<Keypair>,
    pub peer_record: Option<PeerRecord>,
    pub listen_addrs: Vec<Multiaddr>,
    pub bootstrap_peers: Vec<Multiaddr>,
    pub allow_local_handler_fallback_when_no_peers: bool,
}

impl Default for Libp2pReplicationNetworkConfig {
    fn default() -> Self {
        Self {
            keypair: None,
            peer_record: None,
            listen_addrs: Vec::new(),
            bootstrap_peers: Vec::new(),
            allow_local_handler_fallback_when_no_peers: false,
        }
    }
}

#[derive(Clone)]
pub struct Libp2pReplicationNetwork {
    inner: Libp2pNetwork,
    allow_local_handler_fallback_when_no_peers: bool,
    handlers: Arc<Mutex<HashMap<String, Handler>>>,
    request_peer_cursor: Arc<AtomicUsize>,
    unsupported_protocol_peers: Arc<Mutex<HashMap<String, HashSet<PeerId>>>>,
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

    #[cfg(test)]
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.inner.connected_peers()
    }

    #[cfg(test)]
    pub fn debug_errors(&self) -> Vec<String> {
        self.inner.debug_errors()
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
        let unsupported = self
            .unsupported_protocol_peers
            .lock()
            .expect("lock unsupported protocol peers");
        let Some(peers) = unsupported.get(protocol) else {
            return ordered_peers;
        };
        let filtered: Vec<PeerId> = ordered_peers
            .iter()
            .copied()
            .filter(|peer| !peers.contains(peer))
            .collect();
        if filtered.is_empty() {
            ordered_peers
        } else {
            filtered
        }
    }

    fn mark_peer_unsupported_for_protocol(&self, protocol: &str, peer: PeerId) {
        self.unsupported_protocol_peers
            .lock()
            .expect("lock unsupported protocol peers")
            .entry(protocol.to_string())
            .or_default()
            .insert(peer);
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
        let mut peers = self.inner.connected_peers();
        for _ in 0..REQUEST_CONNECTED_PEER_WAIT_RETRIES {
            if !peers.is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(
                REQUEST_CONNECTED_PEER_WAIT_INTERVAL_MS,
            ));
            peers = self.inner.connected_peers();
        }
        peers.sort_by_key(|peer| peer.to_string());
        peers
    }

    fn wait_for_connected_provider_peers(&self, providers: &[String]) -> Vec<PeerId> {
        for attempt in 0..=REQUEST_CONNECTED_PEER_WAIT_RETRIES {
            let connected_peers: HashSet<PeerId> =
                self.inner.connected_peers().into_iter().collect();
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

        let cursor = self.request_peer_cursor.fetch_add(1, Ordering::Relaxed);
        let ordered_peers =
            self.filtered_request_peers(protocol, rotated_peers(peers.as_slice(), cursor));
        let mut last_error = None;
        for peer in ordered_peers {
            match self.request_via_peer(protocol, payload, peer) {
                Ok(reply) => return Ok(reply),
                Err(err) => {
                    if peer_error_indicates_unsupported_protocol(&err) {
                        self.mark_peer_unsupported_for_protocol(protocol, peer);
                    }
                    last_error = Some(err);
                }
            }
        }

        Err(
            last_error.unwrap_or_else(|| WorldError::NetworkProtocolUnavailable {
                protocol: format!("libp2p-replication no connected peers for protocol {protocol}"),
            }),
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
        let ordered_provider_peers = self.filtered_request_peers(protocol, ordered_provider_peers);
        if ordered_provider_peers.is_empty() {
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: format!(
                    "libp2p-replication no connected providers for protocol {protocol}"
                ),
            });
        }

        let mut last_error = None;
        for peer in ordered_provider_peers {
            match self.request_via_peer(protocol, payload, peer) {
                Ok(reply) => return Ok(reply),
                Err(err) => {
                    if peer_error_indicates_unsupported_protocol(&err) {
                        self.mark_peer_unsupported_for_protocol(protocol, peer);
                    }
                    last_error = Some(err);
                }
            }
        }

        Err(
            last_error.unwrap_or_else(|| WorldError::NetworkProtocolUnavailable {
                protocol: format!(
                    "libp2p-replication no connected providers for protocol {protocol}"
                ),
            }),
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

fn peer_error_indicates_unsupported_protocol(err: &WorldError) -> bool {
    match err {
        WorldError::NetworkRequestFailed { code, message, .. } => {
            matches!(
                code,
                DistributedErrorCode::ErrNotFound | DistributedErrorCode::ErrUnsupported
            ) || message.contains("NetworkProtocolUnavailable")
        }
        WorldError::NetworkProtocolUnavailable { protocol } => protocol.contains("handler missing"),
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
    fn libp2p_replication_network_request_with_providers_honors_provider_subset() {
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

        let fail_only = dialer.request_with_providers(
            "/aw/node/replication/ping",
            b"node",
            &[listener_fail.peer_id().to_string()],
        );
        assert!(
            matches!(
                fail_only,
                Err(WorldError::NetworkRequestFailed { .. })
                    | Err(WorldError::NetworkProtocolUnavailable { .. })
            ),
            "expected provider-restricted request to stay on failing peer, got {fail_only:?}"
        );

        let ok_only = dialer
            .request_with_providers(
                "/aw/node/replication/ping",
                b"node",
                &[listener_ok.peer_id().to_string()],
            )
            .expect("provider-restricted request should reach ok peer");
        assert_eq!(ok_only, b"node-ok".to_vec());
    }
}
