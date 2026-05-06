use oasis7::simulator::{WorldEvent, WorldSnapshot};

use crate::industry_graph_view_model::{IndustryGraphViewModel, IndustrySemanticZoomLevel};

const OPS_NAV_TOP_LIMIT: usize = 3;

#[allow(dead_code)]
pub(super) fn ops_navigation_alert_summary(
    snapshot: Option<&WorldSnapshot>,
    events: &[WorldEvent],
) -> Option<String> {
    let graph = IndustryGraphViewModel::build(snapshot, events);
    ops_navigation_alert_summary_with_zoom(&graph, IndustrySemanticZoomLevel::Node)
}

pub(super) fn ops_navigation_alert_summary_with_zoom(
    graph: &IndustryGraphViewModel,
    zoom: IndustrySemanticZoomLevel,
) -> Option<String> {
    if !graph.has_ops_signals() {
        return None;
    }

    let mut lines = vec!["Ops Navigator:".to_string()];
    lines.push(format!("- Semantic Zoom: {}", zoom.key()));

    lines.push("World:".to_string());
    lines.push(format!(
        "- Activity Events(Recent): {}",
        graph.rollup.total_events
    ));
    lines.push(format!(
        "- Alert Events(Recent): {}",
        graph.rollup.alert_events
    ));

    lines.push("".to_string());
    lines.push("Region Hotspots:".to_string());
    if graph.region_hotspots.is_empty() {
        lines.push("- (none)".to_string());
    } else {
        for hotspot in graph.region_hotspots.iter().take(OPS_NAV_TOP_LIMIT) {
            lines.push(format!(
                "- chunk({}, {}, {}): events={} alerts={}",
                hotspot.coord.x, hotspot.coord.y, hotspot.coord.z, hotspot.events, hotspot.alerts,
            ));
        }
    }

    lines.push("".to_string());
    lines.push("Node Hotspots:".to_string());
    if graph.node_hotspots.is_empty() {
        lines.push("- (none)".to_string());
    } else {
        for node in graph.node_hotspots.iter().take(OPS_NAV_TOP_LIMIT) {
            lines.push(format!("- {}: score={}", node.node_id, node.score));
        }
    }

    lines.push("".to_string());
    lines.push("Alert Root Causes:".to_string());
    if graph.root_cause_chains.is_empty() {
        lines.push("- (none)".to_string());
    } else {
        for chain in graph.root_cause_chains.iter().take(OPS_NAV_TOP_LIMIT) {
            lines.push(format!(
                "- {} #{}: Reject({}) -> Shortage({}) -> Congestion({}) -> Stall({})",
                chain.chain_id,
                chain.reject_event_id,
                chain.reject_label,
                chain.shortage_label,
                chain.congestion_label,
                chain.stall_label,
            ));
            if !chain.targets.is_empty() {
                lines.push(format!("  target={}", chain.targets.join(",")));
            }
        }
    }

    Some(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oasis7::geometry::GeoPos;
    use oasis7::simulator::{
        ChunkRuntimeConfig, Location, RejectReason, ResourceKind, ResourceOwner, WorldConfig,
        WorldEvent, WorldEventKind, WorldModel, WorldSnapshot, CHUNK_GENERATION_SCHEMA_VERSION,
        SNAPSHOT_VERSION,
    };

    #[test]
    fn ops_navigation_alert_summary_returns_none_without_snapshot() {
        assert!(ops_navigation_alert_summary(None, &[]).is_none());
    }

    #[test]
    fn ops_navigation_alert_summary_reports_regions_nodes_and_causes() {
        let mut model = WorldModel::default();
        model.locations.insert(
            "loc-a".to_string(),
            Location::new("loc-a", "Alpha", GeoPos::new(1, 1, 1)),
        );
        model.locations.insert(
            "loc-b".to_string(),
            Location::new("loc-b", "Beta", GeoPos::new(2_100_000, 1, 1)),
        );

        let snapshot = WorldSnapshot {
            version: SNAPSHOT_VERSION,
            chunk_generation_schema_version: CHUNK_GENERATION_SCHEMA_VERSION,
            time: 42,
            config: WorldConfig::default(),
            model,
            chunk_runtime: ChunkRuntimeConfig::default(),
            next_event_id: 6,
            next_action_id: 3,
            pending_actions: Vec::new(),
            journal_len: 5,
            runtime_snapshot: None,
            player_gameplay: None,
        };

        let events = vec![
            WorldEvent {
                id: 1,
                time: 31,
                kind: WorldEventKind::ResourceTransferred {
                    from: ResourceOwner::Location {
                        location_id: "loc-a".to_string(),
                    },
                    to: ResourceOwner::Location {
                        location_id: "loc-b".to_string(),
                    },
                    kind: ResourceKind::Data,
                    amount: 2,
                },
                runtime_event: None,
            },
            WorldEvent {
                id: 2,
                time: 32,
                kind: WorldEventKind::AgentMoved {
                    agent_id: "agent-1".to_string(),
                    from: "loc-a".to_string(),
                    to: "loc-b".to_string(),
                    distance_cm: 100,
                    electricity_cost: 1,
                },
                runtime_event: None,
            },
            WorldEvent {
                id: 3,
                time: 33,
                kind: WorldEventKind::ActionRejected {
                    reason: RejectReason::InsufficientResource {
                        owner: ResourceOwner::Agent {
                            agent_id: "agent-1".to_string(),
                        },
                        kind: ResourceKind::Data,
                        requested: 7,
                        available: 2,
                    },
                },
                runtime_event: None,
            },
        ];

        let summary =
            ops_navigation_alert_summary(Some(&snapshot), &events).expect("summary should exist");
        assert!(summary.contains("Ops Navigator:"));
        assert!(summary.contains("World:"));
        assert!(summary.contains("Activity Events(Recent): 3"));
        assert!(summary.contains("Alert Events(Recent): 1"));
        assert!(summary.contains("Region Hotspots:"));
        assert!(summary.contains("Node Hotspots:"));
        assert!(summary.contains("Alert Root Causes:"));
        assert!(summary.contains("InsufficientResource"));
        assert!(summary.contains("target="));
    }
}
