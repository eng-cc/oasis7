use std::collections::BTreeMap;

use oasis7::simulator::{WorldEvent, WorldSnapshot};

use crate::industry_graph_view_model::{IndustryGraphViewModel, IndustrySemanticZoomLevel};

#[allow(dead_code)]
pub(super) fn economy_dashboard_summary(
    snapshot: Option<&WorldSnapshot>,
    events: &[WorldEvent],
) -> Option<String> {
    let graph = IndustryGraphViewModel::build(snapshot, events);
    economy_dashboard_summary_with_zoom(&graph, IndustrySemanticZoomLevel::Node)
}

pub(super) fn economy_dashboard_summary_with_zoom(
    graph: &IndustryGraphViewModel,
    zoom: IndustrySemanticZoomLevel,
) -> Option<String> {
    if !graph.has_economy_signals() {
        return None;
    }

    let mut lines = vec!["Economy Dashboard:".to_string()];
    lines.push(format!("- Semantic Zoom: {}", zoom.key()));

    lines.push("Supply & Demand:".to_string());
    for kind in [
        oasis7::simulator::ResourceKind::Electricity,
        oasis7::simulator::ResourceKind::Data,
    ] {
        let stock = amount_or_zero(&graph.rollup.stock_by_kind, kind);
        let flow = amount_or_zero(&graph.rollup.flow_by_kind, kind);
        let shortfall = amount_or_zero(&graph.rollup.shortfall_by_kind, kind);
        lines.push(format!(
            "- {:?}: stock={} flow={} shortfall={} health={}",
            kind,
            stock,
            flow,
            shortfall,
            inventory_health_label(stock, flow, shortfall)
        ));
    }
    lines.push(format!(
        "- Insufficient Rejects(Recent): {}",
        graph.rollup.insufficient_rejects
    ));

    lines.push("".to_string());
    lines.push("Cost & Revenue Proxy:".to_string());
    lines.push(format!(
        "- Transfer Events(Recent): {}",
        graph.rollup.transfer_events
    ));
    lines.push(format!(
        "- Power Trades(Recent): {}",
        graph.rollup.power_trade_events
    ));
    lines.push(format!(
        "- Power Trade Settlement(Recent): {}",
        graph.rollup.power_trade_settlement
    ));
    lines.push(format!(
        "- Refine Electricity Cost(Recent): {}",
        graph.rollup.refine_electricity_cost
    ));
    lines.push(format!(
        "- Power Loss(Recent): {}",
        graph.rollup.total_power_loss
    ));

    let electricity_flow = amount_or_zero(
        &graph.rollup.flow_by_kind,
        oasis7::simulator::ResourceKind::Electricity,
    );
    let data_flow = amount_or_zero(
        &graph.rollup.flow_by_kind,
        oasis7::simulator::ResourceKind::Data,
    );
    let outbound_value_proxy = electricity_flow.saturating_add(data_flow.saturating_mul(2));
    let total_cost_proxy = graph
        .rollup
        .power_trade_settlement
        .saturating_add(graph.rollup.refine_electricity_cost)
        .saturating_add(graph.rollup.total_power_loss);
    let margin_proxy = outbound_value_proxy.saturating_sub(total_cost_proxy);

    lines.push(format!(
        "- Outbound Value Proxy(Recent): {outbound_value_proxy}"
    ));
    lines.push(format!("- Margin Proxy(Recent): {margin_proxy}"));

    if zoom != IndustrySemanticZoomLevel::World {
        let (nodes, _) = graph.graph_slice_for_zoom(zoom);
        lines.push("".to_string());
        lines.push("Inventory Focus:".to_string());
        for node in nodes.iter().take(4) {
            lines.push(format!(
                "- {}: stock(E/D)={}/{} throughput={} flags(b/c/a)={}/{}/{}",
                node.id,
                node.stock_electricity,
                node.stock_data,
                node.throughput,
                yes_no(node.status.bottleneck),
                yes_no(node.status.congestion),
                yes_no(node.status.alert),
            ));
        }
    }

    Some(lines.join("\n"))
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "Y"
    } else {
        "N"
    }
}

fn amount_or_zero(
    map: &BTreeMap<oasis7::simulator::ResourceKind, i64>,
    kind: oasis7::simulator::ResourceKind,
) -> i64 {
    *map.get(&kind).unwrap_or(&0)
}

fn inventory_health_label(stock: i64, flow: i64, shortfall: i64) -> &'static str {
    if stock <= 0 {
        return "critical";
    }
    let pressure = flow.saturating_add(shortfall).max(1);
    let ratio = stock as f64 / pressure as f64;
    if ratio < 0.5 {
        "critical"
    } else if ratio < 2.0 {
        "warn"
    } else {
        "stable"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oasis7::geometry::GeoPos;
    use oasis7::simulator::{
        Agent, ChunkRuntimeConfig, Location, PowerEvent, RejectReason, ResourceKind, ResourceOwner,
        WorldConfig, WorldEvent, WorldEventKind, WorldModel, WorldSnapshot,
        CHUNK_GENERATION_SCHEMA_VERSION, SNAPSHOT_VERSION,
    };

    #[test]
    fn economy_dashboard_summary_returns_none_without_economy_signals() {
        assert!(economy_dashboard_summary(None, &[]).is_none());
    }

    #[test]
    fn economy_dashboard_summary_reports_supply_demand_and_cost_proxy() {
        let mut model = WorldModel::default();
        let mut agent = Agent::new("agent-1", "loc-a", GeoPos::new(0, 0, 0));
        agent.resources.set(ResourceKind::Electricity, 25).ok();
        model.agents.insert("agent-1".to_string(), agent);

        let mut location = Location::new("loc-a", "Alpha", GeoPos::new(0, 0, 0));
        location.resources.set(ResourceKind::Data, 9).ok();
        model.locations.insert("loc-a".to_string(), location);

        let snapshot = WorldSnapshot {
            version: SNAPSHOT_VERSION,
            chunk_generation_schema_version: CHUNK_GENERATION_SCHEMA_VERSION,
            time: 30,
            config: WorldConfig::default(),
            model,
            chunk_runtime: ChunkRuntimeConfig::default(),
            next_event_id: 8,
            next_action_id: 2,
            pending_actions: Vec::new(),
            journal_len: 4,
            runtime_snapshot: None,
            player_gameplay: None,
        };

        let events = vec![
            WorldEvent {
                id: 1,
                time: 21,
                kind: WorldEventKind::ResourceTransferred {
                    from: ResourceOwner::Location {
                        location_id: "loc-a".to_string(),
                    },
                    to: ResourceOwner::Agent {
                        agent_id: "agent-1".to_string(),
                    },
                    kind: ResourceKind::Data,
                    amount: 6,
                },
                runtime_event: None,
            },
            WorldEvent {
                id: 2,
                time: 22,
                kind: WorldEventKind::Power(PowerEvent::PowerTransferred {
                    from: ResourceOwner::Location {
                        location_id: "loc-a".to_string(),
                    },
                    to: ResourceOwner::Agent {
                        agent_id: "agent-1".to_string(),
                    },
                    amount: 10,
                    loss: 2,
                    quoted_price_per_pu: 3,
                    price_per_pu: 3,
                    settlement_amount: 30,
                }),
                runtime_event: None,
            },
            WorldEvent {
                id: 3,
                time: 23,
                kind: WorldEventKind::CompoundRefined {
                    owner: ResourceOwner::Location {
                        location_id: "loc-a".to_string(),
                    },
                    compound_mass_g: 12,
                    electricity_cost: 4,
                    hardware_output: 3,
                },
                runtime_event: None,
            },
            WorldEvent {
                id: 4,
                time: 24,
                kind: WorldEventKind::ActionRejected {
                    reason: RejectReason::InsufficientResource {
                        owner: ResourceOwner::Agent {
                            agent_id: "agent-1".to_string(),
                        },
                        kind: ResourceKind::Data,
                        requested: 8,
                        available: 3,
                    },
                },
                runtime_event: None,
            },
        ];

        let summary =
            economy_dashboard_summary(Some(&snapshot), &events).expect("economy summary exists");
        assert!(summary.contains("Economy Dashboard:"));
        assert!(summary.contains("Supply & Demand:"));
        assert!(summary.contains("Electricity: stock=25"));
        assert!(summary.contains("Data: stock=9"));
        assert!(summary.contains("Insufficient Rejects(Recent): 1"));
        assert!(summary.contains("Cost & Revenue Proxy:"));
        assert!(summary.contains("Power Trade Settlement(Recent): 30"));
        assert!(summary.contains("Refine Electricity Cost(Recent): 4"));
        assert!(summary.contains("Power Loss(Recent): 2"));
    }
}
