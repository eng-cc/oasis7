use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use libp2p::kad;
use libp2p::relay;
use libp2p::rendezvous;
use libp2p::request_response;
use libp2p::swarm::SwarmEvent;
use serde::{Deserialize, Serialize};

use super::swarm_behaviour::BehaviourEvent;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TrafficDirectionMetricsSnapshot {
    pub messages: u64,
    pub payload_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TrafficLaneMetricsSnapshot {
    pub inbound: TrafficDirectionMetricsSnapshot,
    pub outbound: TrafficDirectionMetricsSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Libp2pControlPlaneMetricsSnapshot {
    pub units: String,
    pub total_events: u64,
    pub by_kind: BTreeMap<String, u64>,
}

impl Default for Libp2pControlPlaneMetricsSnapshot {
    fn default() -> Self {
        Self {
            units: "events".to_string(),
            total_events: 0,
            by_kind: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Libp2pTrafficMetricsSnapshot {
    pub observed_since_unix_ms: i64,
    pub scope: String,
    pub excludes_transport_overhead: bool,
    pub excludes_kademlia_control_plane: bool,
    pub excludes_gossipsub_mesh_fanout: bool,
    pub totals: TrafficLaneMetricsSnapshot,
    pub gossip: TrafficLaneMetricsSnapshot,
    pub request: TrafficLaneMetricsSnapshot,
    pub response: TrafficLaneMetricsSnapshot,
    pub control_plane: Libp2pControlPlaneMetricsSnapshot,
    pub by_topic: BTreeMap<String, TrafficLaneMetricsSnapshot>,
    pub by_protocol: BTreeMap<String, TrafficLaneMetricsSnapshot>,
}

impl Default for Libp2pTrafficMetricsSnapshot {
    fn default() -> Self {
        Self {
            observed_since_unix_ms: now_unix_ms(),
            scope: "application_payload_only".to_string(),
            excludes_transport_overhead: true,
            excludes_kademlia_control_plane: true,
            excludes_gossipsub_mesh_fanout: true,
            totals: TrafficLaneMetricsSnapshot::default(),
            gossip: TrafficLaneMetricsSnapshot::default(),
            request: TrafficLaneMetricsSnapshot::default(),
            response: TrafficLaneMetricsSnapshot::default(),
            control_plane: Libp2pControlPlaneMetricsSnapshot::default(),
            by_topic: BTreeMap::new(),
            by_protocol: BTreeMap::new(),
        }
    }
}

pub(crate) type SharedLibp2pTrafficMetrics = Arc<Mutex<Libp2pTrafficMetricsSnapshot>>;

pub(crate) fn init_shared_traffic_metrics() -> SharedLibp2pTrafficMetrics {
    Arc::new(Mutex::new(Libp2pTrafficMetricsSnapshot::default()))
}

pub(crate) fn snapshot_traffic_metrics(
    metrics: &SharedLibp2pTrafficMetrics,
) -> Libp2pTrafficMetricsSnapshot {
    metrics
        .lock()
        .map(|locked| locked.clone())
        .unwrap_or_else(|_| Libp2pTrafficMetricsSnapshot {
            scope: "application_payload_only_degraded".to_string(),
            ..Libp2pTrafficMetricsSnapshot::default()
        })
}

pub(crate) fn record_gossip_outbound(
    metrics: &SharedLibp2pTrafficMetrics,
    topic: &str,
    payload_bytes: usize,
) {
    record_traffic(
        metrics,
        TrafficFamily::Gossip,
        TrafficDirection::Outbound,
        Some(topic),
        None,
        payload_bytes,
    );
}

pub(crate) fn record_gossip_inbound(
    metrics: &SharedLibp2pTrafficMetrics,
    topic: &str,
    payload_bytes: usize,
) {
    record_traffic(
        metrics,
        TrafficFamily::Gossip,
        TrafficDirection::Inbound,
        Some(topic),
        None,
        payload_bytes,
    );
}

pub(crate) fn record_request_outbound(
    metrics: &SharedLibp2pTrafficMetrics,
    protocol: &str,
    payload_bytes: usize,
) {
    record_traffic(
        metrics,
        TrafficFamily::Request,
        TrafficDirection::Outbound,
        None,
        Some(protocol),
        payload_bytes,
    );
}

pub(crate) fn record_request_inbound(
    metrics: &SharedLibp2pTrafficMetrics,
    protocol: &str,
    payload_bytes: usize,
) {
    record_traffic(
        metrics,
        TrafficFamily::Request,
        TrafficDirection::Inbound,
        None,
        Some(protocol),
        payload_bytes,
    );
}

pub(crate) fn record_response_outbound(
    metrics: &SharedLibp2pTrafficMetrics,
    protocol: &str,
    payload_bytes: usize,
) {
    record_traffic(
        metrics,
        TrafficFamily::Response,
        TrafficDirection::Outbound,
        None,
        Some(protocol),
        payload_bytes,
    );
}

pub(crate) fn record_response_inbound(
    metrics: &SharedLibp2pTrafficMetrics,
    protocol: &str,
    payload_bytes: usize,
) {
    record_traffic(
        metrics,
        TrafficFamily::Response,
        TrafficDirection::Inbound,
        None,
        Some(protocol),
        payload_bytes,
    );
}

pub(crate) fn record_control_plane_event(metrics: &SharedLibp2pTrafficMetrics, kind: &str) {
    let Ok(mut snapshot) = metrics.lock() else {
        return;
    };
    snapshot.control_plane.total_events = snapshot.control_plane.total_events.saturating_add(1);
    let entry = snapshot
        .control_plane
        .by_kind
        .entry(kind.to_string())
        .or_insert(0);
    *entry = entry.saturating_add(1);
}

pub(crate) fn classify_control_plane_event(
    event: &SwarmEvent<BehaviourEvent>,
) -> Option<&'static str> {
    match event {
        SwarmEvent::Behaviour(BehaviourEvent::RequestResponse(
            request_response::Event::OutboundFailure { .. },
        )) => Some("request_response.outbound_failure"),
        SwarmEvent::Behaviour(BehaviourEvent::RequestResponse(
            request_response::Event::InboundFailure { .. },
        )) => Some("request_response.inbound_failure"),
        SwarmEvent::Behaviour(BehaviourEvent::Kademlia(kad::Event::OutboundQueryProgressed {
            ..
        })) => Some("kademlia.outbound_query_progressed"),
        SwarmEvent::Behaviour(BehaviourEvent::Kademlia(kad::Event::RoutingUpdated { .. })) => {
            Some("kademlia.routing_updated")
        }
        SwarmEvent::Behaviour(BehaviourEvent::Autonat(_)) => Some("autonat.event"),
        SwarmEvent::Behaviour(BehaviourEvent::RelayClient(
            relay::client::Event::ReservationReqAccepted { .. },
        )) => Some("relay_client.reservation_accepted"),
        SwarmEvent::Behaviour(BehaviourEvent::RelayClient(
            relay::client::Event::OutboundCircuitEstablished { .. },
        )) => Some("relay_client.outbound_circuit_established"),
        SwarmEvent::Behaviour(BehaviourEvent::RelayClient(
            relay::client::Event::InboundCircuitEstablished { .. },
        )) => Some("relay_client.inbound_circuit_established"),
        SwarmEvent::Behaviour(BehaviourEvent::Dcutr(_)) => Some("dcutr.event"),
        SwarmEvent::Behaviour(BehaviourEvent::RendezvousClient(
            rendezvous::client::Event::Registered { .. },
        )) => Some("rendezvous_client.registered"),
        SwarmEvent::Behaviour(BehaviourEvent::RendezvousClient(
            rendezvous::client::Event::RegisterFailed { .. },
        )) => Some("rendezvous_client.register_failed"),
        SwarmEvent::Behaviour(BehaviourEvent::RendezvousClient(
            rendezvous::client::Event::Discovered { .. },
        )) => Some("rendezvous_client.discovered"),
        SwarmEvent::Behaviour(BehaviourEvent::RendezvousClient(
            rendezvous::client::Event::DiscoverFailed { .. },
        )) => Some("rendezvous_client.discover_failed"),
        SwarmEvent::Behaviour(BehaviourEvent::RendezvousClient(
            rendezvous::client::Event::Expired { .. },
        )) => Some("rendezvous_client.expired"),
        SwarmEvent::Behaviour(BehaviourEvent::RendezvousServer(
            rendezvous::server::Event::PeerNotRegistered { .. },
        )) => Some("rendezvous_server.peer_not_registered"),
        SwarmEvent::NewExternalAddrCandidate { .. } => {
            Some("transport.new_external_addr_candidate")
        }
        SwarmEvent::ExternalAddrConfirmed { .. } => Some("transport.external_addr_confirmed"),
        SwarmEvent::ExternalAddrExpired { .. } => Some("transport.external_addr_expired"),
        SwarmEvent::NewListenAddr { .. } => Some("transport.new_listen_addr"),
        SwarmEvent::ExpiredListenAddr { .. } => Some("transport.expired_listen_addr"),
        SwarmEvent::ListenerClosed { .. } => Some("transport.listener_closed"),
        SwarmEvent::ConnectionEstablished { .. } => Some("transport.connection_established"),
        SwarmEvent::ConnectionClosed { .. } => Some("transport.connection_closed"),
        SwarmEvent::OutgoingConnectionError { .. } => Some("transport.outgoing_connection_error"),
        SwarmEvent::IncomingConnectionError { .. } => Some("transport.incoming_connection_error"),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum TrafficFamily {
    Gossip,
    Request,
    Response,
}

#[derive(Clone, Copy)]
enum TrafficDirection {
    Inbound,
    Outbound,
}

fn record_traffic(
    metrics: &SharedLibp2pTrafficMetrics,
    family: TrafficFamily,
    direction: TrafficDirection,
    topic: Option<&str>,
    protocol: Option<&str>,
    payload_bytes: usize,
) {
    let Ok(mut snapshot) = metrics.lock() else {
        return;
    };
    let payload_bytes = payload_bytes as u64;
    bump_lane(&mut snapshot.totals, direction, payload_bytes);
    match family {
        TrafficFamily::Gossip => bump_lane(&mut snapshot.gossip, direction, payload_bytes),
        TrafficFamily::Request => bump_lane(&mut snapshot.request, direction, payload_bytes),
        TrafficFamily::Response => bump_lane(&mut snapshot.response, direction, payload_bytes),
    }
    if let Some(topic) = topic {
        bump_lane(
            snapshot.by_topic.entry(topic.to_string()).or_default(),
            direction,
            payload_bytes,
        );
    }
    if let Some(protocol) = protocol {
        bump_lane(
            snapshot
                .by_protocol
                .entry(protocol.to_string())
                .or_default(),
            direction,
            payload_bytes,
        );
    }
}

fn bump_lane(
    lane: &mut TrafficLaneMetricsSnapshot,
    direction: TrafficDirection,
    payload_bytes: u64,
) {
    let counters = match direction {
        TrafficDirection::Inbound => &mut lane.inbound,
        TrafficDirection::Outbound => &mut lane.outbound,
    };
    counters.messages = counters.messages.saturating_add(1);
    counters.payload_bytes = counters.payload_bytes.saturating_add(payload_bytes);
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
    use super::*;

    #[test]
    fn recorders_accumulate_totals_and_breakdowns() {
        let metrics = init_shared_traffic_metrics();
        record_gossip_outbound(&metrics, "aw.smoke", 5);
        record_gossip_inbound(&metrics, "aw.smoke", 7);
        record_request_outbound(&metrics, "/aw/rr/1.0.0/ping", 11);
        record_response_inbound(&metrics, "/aw/rr/1.0.0/ping", 13);
        record_control_plane_event(&metrics, "transport.connection_established");
        record_control_plane_event(&metrics, "transport.connection_established");
        record_control_plane_event(&metrics, "kademlia.routing_updated");

        let snapshot = snapshot_traffic_metrics(&metrics);
        assert_eq!(snapshot.scope, "application_payload_only");
        assert!(snapshot.excludes_transport_overhead);
        assert!(snapshot.excludes_kademlia_control_plane);
        assert!(snapshot.excludes_gossipsub_mesh_fanout);
        assert_eq!(snapshot.totals.outbound.messages, 2);
        assert_eq!(snapshot.totals.outbound.payload_bytes, 16);
        assert_eq!(snapshot.totals.inbound.messages, 2);
        assert_eq!(snapshot.totals.inbound.payload_bytes, 20);
        assert_eq!(
            snapshot
                .by_topic
                .get("aw.smoke")
                .map(|lane| lane.outbound.payload_bytes),
            Some(5)
        );
        assert_eq!(
            snapshot
                .by_protocol
                .get("/aw/rr/1.0.0/ping")
                .map(|lane| lane.inbound.payload_bytes),
            Some(13)
        );
        assert_eq!(snapshot.control_plane.units, "events");
        assert_eq!(snapshot.control_plane.total_events, 3);
        assert_eq!(
            snapshot
                .control_plane
                .by_kind
                .get("transport.connection_established"),
            Some(&2)
        );
        assert_eq!(
            snapshot
                .control_plane
                .by_kind
                .get("kademlia.routing_updated"),
            Some(&1)
        );
    }
}
