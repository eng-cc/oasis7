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
    max_dynamic_peers: usize,
    dynamic_peer_ttl_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReceivedGossipMessage {
    pub from: SocketAddr,
    pub message: GossipMessage,
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
        let bytes = serde_json::to_vec(&envelope).map_err(|err| NodeError::Gossip {
            reason: format!("serialize gossip message failed: {}", err),
        })?;
        self.broadcast_bytes(&bytes)
    }

    fn broadcast_bytes(&self, bytes: &[u8]) -> Result<(), NodeError> {
        for peer in self.snapshot_peers()? {
            self.socket
                .send_to(bytes, &peer)
                .map_err(|err| NodeError::Gossip {
                    reason: format!("send_to {} failed: {}", peer, err),
                })?;
        }
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
                        messages.push(ReceivedGossipMessage { from, message });
                        continue;
                    }
                    if let Ok(commit) = serde_json::from_slice::<GossipCommitMessage>(payload) {
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
