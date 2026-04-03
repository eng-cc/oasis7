use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use libp2p::identity::Keypair;
use libp2p::{Multiaddr, PeerId};
use oasis7_net::{Libp2pNetwork, Libp2pNetworkConfig};
use oasis7_proto::distributed::ErrorResponse;
use oasis7_proto::distributed_dht::PeerRecord;
use oasis7_proto::distributed_net::{
    DistributedNetwork as ProtoDistributedNetwork, NetworkSubscription,
};
use oasis7_proto::world_error::WorldError;

use crate::NodeError;

type Handler = Arc<dyn Fn(&[u8]) -> Result<Vec<u8>, WorldError> + Send + Sync>;

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

    fn local_handler(&self, protocol: &str) -> Option<Handler> {
        self.handlers
            .lock()
            .expect("lock libp2p replication handlers")
            .get(protocol)
            .cloned()
    }

    fn call_local_handler(&self, protocol: &str, payload: &[u8]) -> Result<Vec<u8>, WorldError> {
        match self.local_handler(protocol) {
            Some(handler) => handler(payload),
            None => Err(WorldError::NetworkProtocolUnavailable {
                protocol: format!("libp2p-replication handler missing: {protocol}"),
            }),
        }
    }

    fn request_via_peer(
        &self,
        protocol: &str,
        payload: &[u8],
        peer: PeerId,
    ) -> Result<Vec<u8>, WorldError> {
        let provider = peer.to_string();
        let response = ProtoDistributedNetwork::request_with_providers(
            &self.inner,
            protocol,
            payload,
            &[provider],
        )
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
        let mut peers = self.inner.connected_peers();
        peers.sort_by_key(|peer| peer.to_string());

        if peers.is_empty() {
            if self.allow_local_handler_fallback_when_no_peers {
                return self.call_local_handler(protocol, payload);
            }
            return Err(WorldError::NetworkProtocolUnavailable {
                protocol: format!("libp2p-replication no connected peers for protocol {protocol}"),
            });
        }

        let cursor = self.request_peer_cursor.fetch_add(1, Ordering::Relaxed);
        let ordered_peers = rotated_peers(peers.as_slice(), cursor);
        let mut last_error = None;
        for peer in ordered_peers {
            match self.request_via_peer(protocol, payload, peer) {
                Ok(reply) => return Ok(reply),
                Err(err) => last_error = Some(err),
            }
        }

        Err(
            last_error.unwrap_or_else(|| WorldError::NetworkProtocolUnavailable {
                protocol: format!("libp2p-replication no connected peers for protocol {protocol}"),
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

fn decode_error_response(payload: &[u8]) -> Option<ErrorResponse> {
    serde_cbor::from_slice(payload).ok()
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
}
