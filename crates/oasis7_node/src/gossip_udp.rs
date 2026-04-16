use std::collections::{BTreeMap, BTreeSet};
use std::net::{SocketAddr, UdpSocket};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) use oasis7_consensus::node_consensus_message::{
    NodeGossipAttestationMessage as GossipAttestationMessage,
    NodeGossipCommitMessage as GossipCommitMessage,
    NodeGossipProposalMessage as GossipProposalMessage,
};
use serde::{Deserialize, Serialize};

use crate::replication::GossipReplicationMessage;
use crate::{NodeError, NodeGossipConfig};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum GossipMessage {
    Hello(GossipHelloMessage),
    Commit(GossipCommitMessage),
    Proposal(GossipProposalMessage),
    Attestation(GossipAttestationMessage),
    Replication(GossipReplicationMessage),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GossipHelloMessage {
    pub version: u8,
    pub world_id: String,
    pub node_id: String,
    pub sent_at_ms: i64,
}

#[derive(Debug)]
pub(crate) struct GossipEndpoint {
    socket: UdpSocket,
    bind_addr: SocketAddr,
    peers: Mutex<GossipPeerBook>,
    traffic_metrics: Mutex<GossipTrafficMetricsSnapshot>,
    max_dynamic_peers: usize,
    dynamic_peer_ttl_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReceivedGossipMessage {
    pub from: SocketAddr,
    pub message: GossipMessage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GossipTrafficDirectionMetricsSnapshot {
    pub datagrams: u64,
    pub payload_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GossipTrafficLaneMetricsSnapshot {
    pub inbound: GossipTrafficDirectionMetricsSnapshot,
    pub outbound: GossipTrafficDirectionMetricsSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GossipTrafficMetricsSnapshot {
    pub observed_since_unix_ms: i64,
    pub scope: String,
    pub excludes_transport_headers: bool,
    pub totals: GossipTrafficLaneMetricsSnapshot,
    pub by_kind: BTreeMap<String, GossipTrafficLaneMetricsSnapshot>,
}

impl Default for GossipTrafficMetricsSnapshot {
    fn default() -> Self {
        Self {
            observed_since_unix_ms: now_unix_ms(),
            scope: "udp_payload_bytes".to_string(),
            excludes_transport_headers: true,
            totals: GossipTrafficLaneMetricsSnapshot::default(),
            by_kind: BTreeMap::new(),
        }
    }
}

impl GossipEndpoint {
    pub(crate) fn broadcast_hello(&self, message: &GossipHelloMessage) -> Result<(), NodeError> {
        self.broadcast_message(GossipMessage::Hello(message.clone()))
    }

    pub(crate) fn bind(config: &NodeGossipConfig) -> Result<Self, NodeError> {
        let socket = UdpSocket::bind(config.bind_addr).map_err(|err| NodeError::Gossip {
            reason: format!("bind {} failed: {}", config.bind_addr, err),
        })?;
        socket
            .set_nonblocking(true)
            .map_err(|err| NodeError::Gossip {
                reason: format!("set_nonblocking failed: {}", err),
            })?;
        let peers = config
            .peers
            .iter()
            .copied()
            .filter(|peer| *peer != config.bind_addr)
            .collect::<BTreeSet<_>>();
        Ok(Self {
            socket,
            bind_addr: config.bind_addr,
            peers: Mutex::new(GossipPeerBook::new(peers)),
            traffic_metrics: Mutex::new(GossipTrafficMetricsSnapshot::default()),
            max_dynamic_peers: config.max_dynamic_peers.max(1),
            dynamic_peer_ttl_ms: config.dynamic_peer_ttl_ms.max(1),
        })
    }

    pub(crate) fn broadcast_commit(&self, message: &GossipCommitMessage) -> Result<(), NodeError> {
        self.broadcast_message(GossipMessage::Commit(message.clone()))
    }

    pub(crate) fn broadcast_proposal(
        &self,
        message: &GossipProposalMessage,
    ) -> Result<(), NodeError> {
        self.broadcast_message(GossipMessage::Proposal(message.clone()))
    }

    pub(crate) fn broadcast_attestation(
        &self,
        message: &GossipAttestationMessage,
    ) -> Result<(), NodeError> {
        self.broadcast_message(GossipMessage::Attestation(message.clone()))
    }

    pub(crate) fn broadcast_replication(
        &self,
        message: &GossipReplicationMessage,
    ) -> Result<(), NodeError> {
        self.broadcast_message(GossipMessage::Replication(message.clone()))
    }

    fn broadcast_message(&self, envelope: GossipMessage) -> Result<(), NodeError> {
        let kind = gossip_message_kind_label(&envelope);
        let bytes = serde_json::to_vec(&envelope).map_err(|err| NodeError::Gossip {
            reason: format!("serialize gossip message failed: {}", err),
        })?;
        self.broadcast_bytes(kind, &bytes)
    }

    fn broadcast_bytes(&self, kind: &str, bytes: &[u8]) -> Result<(), NodeError> {
        let peers = self.snapshot_peers()?;
        let mut sent_datagrams = 0u64;
        for peer in &peers {
            match self.socket.send_to(bytes, peer) {
                Ok(_) => sent_datagrams = sent_datagrams.saturating_add(1),
                Err(err) => {
                    if sent_datagrams > 0 {
                        self.record_outbound(kind, bytes.len(), sent_datagrams);
                    }
                    return Err(NodeError::Gossip {
                        reason: format!("send_to {} failed: {}", peer, err),
                    });
                }
            }
        }
        self.record_outbound(kind, bytes.len(), sent_datagrams);
        Ok(())
    }

    pub(crate) fn remember_peer(&self, peer: SocketAddr) -> Result<(), NodeError> {
        if peer == self.bind_addr || peer.port() == 0 {
            return Ok(());
        }
        let mut peers = self.peers.lock().map_err(|_| NodeError::Gossip {
            reason: "peers mutex poisoned".to_string(),
        })?;
        peers.remember_dynamic_peer(
            peer,
            now_unix_ms(),
            self.max_dynamic_peers,
            self.dynamic_peer_ttl_ms,
        );
        Ok(())
    }

    pub(crate) fn drain_messages(&self) -> Result<Vec<ReceivedGossipMessage>, NodeError> {
        let mut buf = [0u8; 4096];
        let mut messages = Vec::new();
        loop {
            match self.socket.recv_from(&mut buf) {
                Ok((size, from)) => {
                    let payload = &buf[..size];
                    if let Ok(message) = serde_json::from_slice::<GossipMessage>(payload) {
                        self.record_inbound(gossip_message_kind_label(&message), size);
                        messages.push(ReceivedGossipMessage { from, message });
                        continue;
                    }
                    if let Ok(commit) = serde_json::from_slice::<GossipCommitMessage>(payload) {
                        self.record_inbound("commit", size);
                        messages.push(ReceivedGossipMessage {
                            from,
                            message: GossipMessage::Commit(commit),
                        });
                    }
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(err) => {
                    return Err(NodeError::Gossip {
                        reason: format!("recv_from failed: {}", err),
                    });
                }
            }
        }
        Ok(messages)
    }

    fn snapshot_peers(&self) -> Result<Vec<SocketAddr>, NodeError> {
        let mut peers = self.peers.lock().map_err(|_| NodeError::Gossip {
            reason: "peers mutex poisoned".to_string(),
        })?;
        Ok(peers.snapshot(now_unix_ms(), self.dynamic_peer_ttl_ms))
    }

    pub(crate) fn traffic_metrics_snapshot(&self) -> GossipTrafficMetricsSnapshot {
        self.traffic_metrics
            .lock()
            .map(|locked| locked.clone())
            .unwrap_or_else(|_| GossipTrafficMetricsSnapshot {
                scope: "udp_payload_bytes_degraded".to_string(),
                ..GossipTrafficMetricsSnapshot::default()
            })
    }

    fn record_outbound(&self, kind: &str, payload_bytes: usize, datagrams: u64) {
        let Ok(mut snapshot) = self.traffic_metrics.lock() else {
            return;
        };
        let payload_bytes = (payload_bytes as u64).saturating_mul(datagrams);
        bump_gossip_lane(&mut snapshot.totals.outbound, payload_bytes, datagrams);
        bump_gossip_lane(
            &mut snapshot
                .by_kind
                .entry(kind.to_string())
                .or_default()
                .outbound,
            payload_bytes,
            datagrams,
        );
    }

    fn record_inbound(&self, kind: &str, payload_bytes: usize) {
        let Ok(mut snapshot) = self.traffic_metrics.lock() else {
            return;
        };
        let payload_bytes = payload_bytes as u64;
        bump_gossip_lane(&mut snapshot.totals.inbound, payload_bytes, 1);
        bump_gossip_lane(
            &mut snapshot
                .by_kind
                .entry(kind.to_string())
                .or_default()
                .inbound,
            payload_bytes,
            1,
        );
    }
}

fn bump_gossip_lane(
    lane: &mut GossipTrafficDirectionMetricsSnapshot,
    payload_bytes: u64,
    datagrams: u64,
) {
    lane.datagrams = lane.datagrams.saturating_add(datagrams);
    lane.payload_bytes = lane.payload_bytes.saturating_add(payload_bytes);
}

fn gossip_message_kind_label(message: &GossipMessage) -> &'static str {
    match message {
        GossipMessage::Hello(_) => "hello",
        GossipMessage::Commit(_) => "commit",
        GossipMessage::Proposal(_) => "proposal",
        GossipMessage::Attestation(_) => "attestation",
        GossipMessage::Replication(_) => "replication",
    }
}

#[derive(Debug, Clone)]
struct GossipPeerBook {
    static_peers: BTreeSet<SocketAddr>,
    dynamic_peers: BTreeMap<SocketAddr, i64>,
}

impl GossipPeerBook {
    fn new(static_peers: BTreeSet<SocketAddr>) -> Self {
        Self {
            static_peers,
            dynamic_peers: BTreeMap::new(),
        }
    }

    fn remember_dynamic_peer(
        &mut self,
        peer: SocketAddr,
        now_ms: i64,
        max_dynamic_peers: usize,
        dynamic_peer_ttl_ms: i64,
    ) {
        self.prune_expired_dynamic_peers(now_ms, dynamic_peer_ttl_ms);
        if self.static_peers.contains(&peer) {
            return;
        }
        if let Some(last_seen) = self.dynamic_peers.get_mut(&peer) {
            *last_seen = now_ms;
            return;
        }
        while self.dynamic_peers.len() >= max_dynamic_peers {
            let Some(evicted_peer) = self.oldest_dynamic_peer() else {
                break;
            };
            self.dynamic_peers.remove(&evicted_peer);
        }
        self.dynamic_peers.insert(peer, now_ms);
    }

    fn snapshot(&mut self, now_ms: i64, dynamic_peer_ttl_ms: i64) -> Vec<SocketAddr> {
        self.prune_expired_dynamic_peers(now_ms, dynamic_peer_ttl_ms);
        let mut peers = self.static_peers.iter().copied().collect::<Vec<_>>();
        peers.extend(
            self.dynamic_peers
                .keys()
                .filter(|peer| !self.static_peers.contains(peer))
                .copied(),
        );
        peers
    }

    fn oldest_dynamic_peer(&self) -> Option<SocketAddr> {
        self.dynamic_peers
            .iter()
            .min_by(|(left_addr, left_seen), (right_addr, right_seen)| {
                left_seen
                    .cmp(right_seen)
                    .then_with(|| left_addr.cmp(right_addr))
            })
            .map(|(peer, _)| *peer)
    }

    fn prune_expired_dynamic_peers(&mut self, now_ms: i64, dynamic_peer_ttl_ms: i64) {
        self.dynamic_peers
            .retain(|_, last_seen_ms| now_ms.saturating_sub(*last_seen_ms) <= dynamic_peer_ttl_ms);
    }
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use super::*;
    use crate::NodeGossipConfig;

    fn gossip_config(bind_addr: SocketAddr, peers: Vec<SocketAddr>) -> NodeGossipConfig {
        NodeGossipConfig {
            bind_addr,
            peers,
            max_dynamic_peers: 8,
            dynamic_peer_ttl_ms: 60_000,
        }
    }

    #[test]
    fn gossip_endpoint_tracks_inbound_and_outbound_payload_bytes() {
        let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
        let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
        let addr_a = socket_a.local_addr().expect("addr a");
        let addr_b = socket_b.local_addr().expect("addr b");
        drop(socket_a);
        drop(socket_b);

        let endpoint_a = GossipEndpoint::bind(&gossip_config(addr_a, vec![addr_b])).expect("a");
        let endpoint_b = GossipEndpoint::bind(&gossip_config(addr_b, vec![addr_a])).expect("b");
        endpoint_a
            .broadcast_commit(&GossipCommitMessage {
                version: 1,
                world_id: "w".to_string(),
                node_id: "node-a".to_string(),
                player_id: "player-a".to_string(),
                height: 1,
                slot: 1,
                epoch: 0,
                block_hash: "block-1".to_string(),
                action_root: "root".to_string(),
                actions: Vec::new(),
                committed_at_ms: 1_000,
                execution_block_hash: None,
                execution_state_root: None,
                public_key_hex: None,
                signature_hex: None,
            })
            .expect("broadcast");
        thread::sleep(Duration::from_millis(20));

        let received = endpoint_b.drain_messages().expect("drain");
        assert_eq!(received.len(), 1);

        let outbound = endpoint_a.traffic_metrics_snapshot();
        assert_eq!(outbound.totals.outbound.datagrams, 1);
        assert!(outbound.totals.outbound.payload_bytes > 0);
        assert_eq!(
            outbound
                .by_kind
                .get("commit")
                .map(|lane| lane.outbound.datagrams),
            Some(1)
        );

        let inbound = endpoint_b.traffic_metrics_snapshot();
        assert_eq!(inbound.totals.inbound.datagrams, 1);
        assert!(inbound.totals.inbound.payload_bytes > 0);
        assert_eq!(
            inbound
                .by_kind
                .get("commit")
                .map(|lane| lane.inbound.datagrams),
            Some(1)
        );
    }

    #[test]
    fn gossip_endpoint_records_partial_outbound_success_before_error() {
        let socket_a = UdpSocket::bind("127.0.0.1:0").expect("bind a");
        let socket_b = UdpSocket::bind("127.0.0.1:0").expect("bind b");
        let addr_a = socket_a.local_addr().expect("addr a");
        let addr_b = socket_b.local_addr().expect("addr b");
        drop(socket_a);
        drop(socket_b);

        let invalid_peer: SocketAddr = "255.255.255.255:1".parse().expect("invalid peer");
        let endpoint_a =
            GossipEndpoint::bind(&gossip_config(addr_a, vec![addr_b, invalid_peer])).expect("a");
        let endpoint_b = GossipEndpoint::bind(&gossip_config(addr_b, vec![addr_a])).expect("b");

        let err = endpoint_a
            .broadcast_commit(&GossipCommitMessage {
                version: 1,
                world_id: "w".to_string(),
                node_id: "node-a".to_string(),
                player_id: "player-a".to_string(),
                height: 1,
                slot: 1,
                epoch: 0,
                block_hash: "block-1".to_string(),
                action_root: "root".to_string(),
                actions: Vec::new(),
                committed_at_ms: 1_000,
                execution_block_hash: None,
                execution_state_root: None,
                public_key_hex: None,
                signature_hex: None,
            })
            .expect_err("partial send should surface send_to error");
        assert!(matches!(err, NodeError::Gossip { .. }));

        thread::sleep(Duration::from_millis(20));

        let received = endpoint_b.drain_messages().expect("drain");
        assert_eq!(received.len(), 1);

        let outbound = endpoint_a.traffic_metrics_snapshot();
        assert_eq!(outbound.totals.outbound.datagrams, 1);
        assert!(outbound.totals.outbound.payload_bytes > 0);
        assert_eq!(
            outbound
                .by_kind
                .get("commit")
                .map(|lane| lane.outbound.datagrams),
            Some(1)
        );
    }
}
