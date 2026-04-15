use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

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
    }
}
