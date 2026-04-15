use crate::industry_graph_view_model::{
    IndustryGraphViewModel, IndustryNodeKind, IndustrySemanticZoomLevel,
};
use oasis7::simulator::{WorldEvent, WorldEventKind, WorldSnapshot};

const INDUSTRIAL_TOP_ROUTE_LIMIT: usize = 3;
const INDUSTRIAL_NODE_DETAIL_LIMIT: usize = 4;
const INDUSTRIAL_WORLD_HOTSPOT_LIMIT: usize = 3;
const INDUSTRIAL_BLOCKED_FACTORY_LIMIT: usize = 4;
const INDUSTRIAL_RECENT_FEEDBACK_LIMIT: usize = 5;

#[derive(Default)]
struct FactoryRuntimeRollup {
    running: usize,
    blocked: usize,
    idle: usize,
    active_jobs: u64,
    completed_jobs: u64,
    blocked_factories: Vec<String>,
}

#[allow(dead_code)]
pub(super) fn industrial_ops_summary(
    snapshot: Option<&WorldSnapshot>,
    events: &[WorldEvent],
) -> Option<String> {
    let graph = IndustryGraphViewModel::build(snapshot, events);
    industrial_ops_summary_with_zoom(&graph, snapshot, events, IndustrySemanticZoomLevel::Node)
}

pub(super) fn industrial_ops_summary_with_zoom(
    graph: &IndustryGraphViewModel,
    snapshot: Option<&WorldSnapshot>,
    events: &[WorldEvent],
    zoom: IndustrySemanticZoomLevel,
) -> Option<String> {
    if !graph.has_industrial_signals()
        && collect_factory_runtime_rollup(snapshot).is_none()
        && recent_runtime_feedback_lines(events).is_empty()
    {
        return None;
    }

    let mut lines = vec!["Industrial Ops:".to_string()];
    lines.push(format!("- Semantic Zoom: {}", zoom.key()));
    lines.push("Production Lines:".to_string());
    lines.push(format!(
        "- Factory Visuals: {}",
        graph.rollup.factory_visuals
    ));
    lines.push(format!("- Recipe Visuals: {}", graph.rollup.recipe_visuals));
    lines.push(format!(
        "- Product Visuals: {}",
        graph.rollup.product_visuals
    ));
    lines.push(format!(
        "- Logistics Visuals: {}",
        graph.rollup.logistics_visuals
    ));
    lines.push(format!(
        "- Recent Refine Events: {}",
        graph.rollup.recent_refine_events
    ));
    lines.push(format!(
        "- Recent Line Updates: {}",
        graph.rollup.recent_line_updates
    ));
    lines.push(format!(
        "- Refine Output(Recent): {}",
        graph.rollup.recent_hardware_output
    ));

    if let Some(rollup) = collect_factory_runtime_rollup(snapshot) {
        lines.push("".to_string());
        lines.push("Factory Runtime Status:".to_string());
        lines.push(format!(
            "- running={} blocked={} idle={} active_jobs={} completed_jobs={}",
            rollup.running, rollup.blocked, rollup.idle, rollup.active_jobs, rollup.completed_jobs,
        ));
        lines.push("Blocked Factories:".to_string());
        if rollup.blocked_factories.is_empty() {
            lines.push("- (none)".to_string());
        } else {
            for blocked in rollup
                .blocked_factories
                .into_iter()
                .take(INDUSTRIAL_BLOCKED_FACTORY_LIMIT)
            {
                lines.push(format!("- {blocked}"));
            }
        }
    }

    let recent_feedback = recent_runtime_feedback_lines(events);
    if !recent_feedback.is_empty() {
        lines.push("".to_string());
        lines.push("Recent Production Feedback:".to_string());
        for entry in recent_feedback {
            lines.push(format!("- {entry}"));
        }
    }

    lines.push("".to_string());
    lines.push("Logistics Routes:".to_string());
    let routes = graph.routes_for_zoom_ref(zoom);
    lines.push(format!("- Active Routes: {}", routes.len()));
    lines.push(format!(
        "- Transfer Events: {}",
        graph.rollup.transfer_events
    ));
    lines.push(format!(
        "- Power Moved: {} (loss={})",
        graph.rollup.total_power_moved, graph.rollup.total_power_loss
    ));

    for route in routes.iter().take(INDUSTRIAL_TOP_ROUTE_LIMIT) {
        lines.push(format!(
            "- Route {} -> {} moves={} material={} electricity={} data={} power={} loss={}",
            route.from,
            route.to,
            route.transfer_events,
            route.material,
            route.electricity,
            route.data,
            route.power,
            route.power_loss,
        ));
    }

    lines.push("".to_string());
    match zoom {
        IndustrySemanticZoomLevel::World => {
            lines.push("World Lens: Hotspots & Trunk Flow".to_string());
            if graph.region_hotspots.is_empty() {
                lines.push("- Hotspots: (none)".to_string());
            } else {
                for hotspot in graph
                    .region_hotspots
                    .iter()
                    .take(INDUSTRIAL_WORLD_HOTSPOT_LIMIT)
                {
                    lines.push(format!(
                        "- chunk({}, {}, {}): events={} alerts={}",
                        hotspot.coord.x,
                        hotspot.coord.y,
                        hotspot.coord.z,
                        hotspot.events,
                        hotspot.alerts,
                    ));
                }
            }
        }
        IndustrySemanticZoomLevel::Region => {
            lines.push("Region Lens: Cluster Nodes".to_string());
            let (nodes, edges) = graph.graph_slice_for_zoom(zoom);
            let factories = nodes
                .iter()
                .filter(|node| node.kind == IndustryNodeKind::Factory)
                .count();
            let recipes = nodes
                .iter()
                .filter(|node| node.kind == IndustryNodeKind::Recipe)
                .count();
            lines.push(format!("- Cluster Nodes: {}", nodes.len()));
            lines.push(format!("- Cluster Edges: {}", edges.len()));
            lines.push(format!("- Factory/Recipe: {factories}/{recipes}"));
        }
        IndustrySemanticZoomLevel::Node => {
            lines.push("Node Lens: Recipe & Inventory State".to_string());
            let mut detailed = graph.nodes.clone();
            detailed.sort_by(|left, right| {
                right
                    .throughput
                    .cmp(&left.throughput)
                    .then_with(|| left.id.cmp(&right.id))
            });
            for node in detailed
                .into_iter()
                .filter(|node| {
                    matches!(
                        node.kind,
                        IndustryNodeKind::Factory
                            | IndustryNodeKind::Recipe
                            | IndustryNodeKind::Product
                            | IndustryNodeKind::LogisticsStation
                    )
                })
                .take(INDUSTRIAL_NODE_DETAIL_LIMIT)
            {
                lines.push(format!(
                    "- {} kind={:?} tier={:?} stage={:?} throughput={} stock(E/D)={}/{} flags(b/c/a)={}/{}/{}",
                    node.id,
                    node.kind,
                    node.tier,
                    node.stage,
                    node.throughput,
                    node.stock_electricity,
                    node.stock_data,
                    yes_no(node.status.bottleneck),
                    yes_no(node.status.congestion),
                    yes_no(node.status.alert),
                ));
            }
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

fn recent_runtime_feedback_lines(events: &[WorldEvent]) -> Vec<String> {
    events
        .iter()
        .rev()
        .filter_map(|event| match &event.kind {
            WorldEventKind::RuntimeEvent { kind, domain_kind } => {
                let summary = domain_kind.as_deref()?;
                let label = match kind.as_str() {
                    "runtime.economy.recipe_started" => "accepted_and_executing",
                    "runtime.economy.recipe_completed" => "produced",
                    "runtime.economy.factory_production_blocked" => "blocked",
                    "runtime.economy.factory_production_resumed" => "resumed",
                    "runtime.economy.factory_built" => "factory_ready",
                    _ => return None,
                };
                Some(format!("{label} {summary}"))
            }
            _ => None,
        })
        .take(INDUSTRIAL_RECENT_FEEDBACK_LIMIT)
        .collect::<Vec<_>>()
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_factory_runtime_rollup(
    snapshot: Option<&WorldSnapshot>,
) -> Option<FactoryRuntimeRollup> {
    let runtime_snapshot = snapshot?.runtime_snapshot.as_ref()?;
    if runtime_snapshot.state.factories.is_empty() {
        return None;
    }

    let mut rollup = FactoryRuntimeRollup::default();
    for factory in runtime_snapshot.state.factories.values() {
        match factory.production.status {
            oasis7::runtime::FactoryProductionStatus::Running => rollup.running += 1,
            oasis7::runtime::FactoryProductionStatus::Blocked => rollup.blocked += 1,
            oasis7::runtime::FactoryProductionStatus::Idle => rollup.idle += 1,
        }
        rollup.active_jobs = rollup
            .active_jobs
            .saturating_add(factory.production.active_jobs as u64);
        rollup.completed_jobs = rollup
            .completed_jobs
            .saturating_add(factory.production.completed_jobs);
        if matches!(
            factory.production.status,
            oasis7::runtime::FactoryProductionStatus::Blocked
        ) {
            rollup.blocked_factories.push(format!(
                "factory={} reason={} detail={}",
                factory.factory_id,
                factory
                    .production
                    .current_blocker_kind
                    .as_deref()
                    .unwrap_or("unknown_reason"),
                factory
                    .production
                    .current_blocker_detail
                    .as_deref()
                    .unwrap_or("none"),
            ));
        }
    }
    Some(rollup)
}

#[cfg(target_arch = "wasm32")]
fn collect_factory_runtime_rollup(
    snapshot: Option<&WorldSnapshot>,
) -> Option<FactoryRuntimeRollup> {
    let runtime_snapshot = snapshot?.runtime_snapshot.as_ref()?;
    let factories = runtime_snapshot
        .get("state")
        .and_then(|value| value.get("factories"))
        .and_then(|value| value.as_object())?;
    if factories.is_empty() {
        return None;
    }

    let mut rollup = FactoryRuntimeRollup::default();
    for (factory_id, factory) in factories {
        let production = factory.get("production");
        let status = production
            .and_then(|value| value.get("status"))
            .and_then(|value| value.as_str())
            .unwrap_or("idle");
        match status {
            "running" => rollup.running += 1,
            "blocked" => rollup.blocked += 1,
            _ => rollup.idle += 1,
        }
        rollup.active_jobs = rollup.active_jobs.saturating_add(
            production
                .and_then(|value| value.get("active_jobs"))
                .and_then(|value| value.as_u64())
                .unwrap_or(0),
        );
        rollup.completed_jobs = rollup.completed_jobs.saturating_add(
            production
                .and_then(|value| value.get("completed_jobs"))
                .and_then(|value| value.as_u64())
                .unwrap_or(0),
        );
        if status == "blocked" {
            let reason = production
                .and_then(|value| value.get("current_blocker_kind"))
                .and_then(|value| value.as_str())
                .unwrap_or("unknown_reason");
            let detail = production
                .and_then(|value| value.get("current_blocker_detail"))
                .and_then(|value| value.as_str())
                .unwrap_or("none");
            rollup.blocked_factories.push(format!(
                "factory={factory_id} reason={reason} detail={detail}"
            ));
        }
    }
    Some(rollup)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oasis7::geometry::GeoPos;
    use oasis7::runtime::{
        FactoryModuleSpec, FactoryProductionState, FactoryProductionStatus, FactoryState,
        MaterialLedgerId, MaterialStack, World,
    };
    use oasis7::simulator::{
        ChunkRuntimeConfig, ModuleVisualAnchor, ModuleVisualEntity, PowerEvent, ResourceKind,
        ResourceOwner, WorldConfig, WorldModel, CHUNK_GENERATION_SCHEMA_VERSION, SNAPSHOT_VERSION,
    };

    fn sample_factory_spec(factory_id: &str) -> FactoryModuleSpec {
        FactoryModuleSpec {
            factory_id: factory_id.to_string(),
            display_name: "Test Factory".to_string(),
            tier: 1,
            tags: vec!["assembly".to_string()],
            build_cost: vec![MaterialStack::new("steel_plate", 10)],
            build_time_ticks: 4,
            base_power_draw: 8,
            recipe_slots: 1,
            throughput_bps: 10_000,
            maintenance_per_tick: 1,
        }
    }

    #[test]
    fn industrial_ops_summary_returns_none_without_industrial_signals() {
        assert!(industrial_ops_summary(None, &[]).is_none());
    }

    #[test]
    fn industrial_ops_summary_aggregates_production_and_routes() {
        let mut model = WorldModel::default();
        model.module_visual_entities.insert(
            "factory-1".to_string(),
            ModuleVisualEntity {
                entity_id: "factory-1".to_string(),
                module_id: "m4.factory.smelter.mk1".to_string(),
                kind: "factory".to_string(),
                label: Some("Smelter".to_string()),
                anchor: ModuleVisualAnchor::Location {
                    location_id: "loc-a".to_string(),
                },
            },
        );
        model.module_visual_entities.insert(
            "recipe-1".to_string(),
            ModuleVisualEntity {
                entity_id: "recipe-1".to_string(),
                module_id: "m4.recipe.smelter.iron_ingot".to_string(),
                kind: "recipe".to_string(),
                label: Some("Iron Ingot".to_string()),
                anchor: ModuleVisualAnchor::Location {
                    location_id: "loc-a".to_string(),
                },
            },
        );
        model.module_visual_entities.insert(
            "product-1".to_string(),
            ModuleVisualEntity {
                entity_id: "product-1".to_string(),
                module_id: "m4.product.component.motor_mk1".to_string(),
                kind: "product".to_string(),
                label: Some("Motor".to_string()),
                anchor: ModuleVisualAnchor::Location {
                    location_id: "loc-b".to_string(),
                },
            },
        );

        let snapshot = WorldSnapshot {
            version: SNAPSHOT_VERSION,
            chunk_generation_schema_version: CHUNK_GENERATION_SCHEMA_VERSION,
            time: 18,
            config: WorldConfig::default(),
            model,
            chunk_runtime: ChunkRuntimeConfig::default(),
            next_event_id: 5,
            next_action_id: 3,
            pending_actions: Vec::new(),
            journal_len: 4,
            runtime_snapshot: None,
            player_gameplay: None,
        };

        let events = vec![
            WorldEvent {
                id: 1,
                time: 12,
                kind: WorldEventKind::CompoundRefined {
                    owner: ResourceOwner::Location {
                        location_id: "loc-a".to_string(),
                    },
                    compound_mass_g: 14,
                    electricity_cost: 2,
                    hardware_output: 9,
                },
                runtime_event: None,
            },
            WorldEvent {
                id: 2,
                time: 13,
                kind: WorldEventKind::ModuleVisualEntityUpserted {
                    entity: ModuleVisualEntity {
                        entity_id: "factory-2".to_string(),
                        module_id: "m4.factory.assembler.mk1".to_string(),
                        kind: "factory".to_string(),
                        label: Some("Assembler".to_string()),
                        anchor: ModuleVisualAnchor::Absolute {
                            pos: GeoPos::new(10.0, 0.0, 10.0),
                        },
                    },
                },
                runtime_event: None,
            },
            WorldEvent {
                id: 3,
                time: 14,
                kind: WorldEventKind::ResourceTransferred {
                    from: ResourceOwner::Location {
                        location_id: "loc-a".to_string(),
                    },
                    to: ResourceOwner::Location {
                        location_id: "loc-b".to_string(),
                    },
                    kind: ResourceKind::Data,
                    amount: 5,
                },
                runtime_event: None,
            },
            WorldEvent {
                id: 4,
                time: 15,
                kind: WorldEventKind::Power(PowerEvent::PowerTransferred {
                    from: ResourceOwner::Location {
                        location_id: "loc-a".to_string(),
                    },
                    to: ResourceOwner::Location {
                        location_id: "loc-b".to_string(),
                    },
                    amount: 11,
                    loss: 1,
                    quoted_price_per_pu: 0,
                    price_per_pu: 0,
                    settlement_amount: 0,
                }),
                runtime_event: None,
            },
        ];

        let graph = IndustryGraphViewModel::build(Some(&snapshot), &events);
        let summary = industrial_ops_summary_with_zoom(
            &graph,
            Some(&snapshot),
            &events,
            IndustrySemanticZoomLevel::Node,
        )
        .expect("industrial summary exists");
        assert!(summary.contains("Production Lines:"));
        assert!(summary.contains("- Factory Visuals: 1"));
        assert!(summary.contains("- Recipe Visuals: 1"));
        assert!(summary.contains("- Product Visuals: 1"));
        assert!(summary.contains("- Recent Refine Events: 1"));
        assert!(summary.contains("- Refine Output(Recent): 9"));
        assert!(summary.contains("Logistics Routes:"));
        assert!(summary.contains("- Active Routes: 2"));
        assert!(summary.contains("- Power Moved: 11 (loss=1)"));
        assert!(summary.contains("location::loc-a -> location::loc-b"));
    }

    #[test]
    fn industrial_ops_summary_includes_runtime_factory_status_and_feedback() {
        let mut model = WorldModel::default();
        model.module_visual_entities.insert(
            "factory-1".to_string(),
            ModuleVisualEntity {
                entity_id: "factory-1".to_string(),
                module_id: "m4.factory.assembler.mk1".to_string(),
                kind: "factory".to_string(),
                label: Some("Assembler".to_string()),
                anchor: ModuleVisualAnchor::Location {
                    location_id: "loc-a".to_string(),
                },
            },
        );

        let mut runtime_snapshot = World::default().snapshot();
        runtime_snapshot.state.factories.insert(
            "factory.alpha".to_string(),
            FactoryState {
                factory_id: "factory.alpha".to_string(),
                site_id: "site.alpha".to_string(),
                builder_agent_id: "builder.alpha".to_string(),
                spec: sample_factory_spec("factory.alpha"),
                input_ledger: MaterialLedgerId::world(),
                output_ledger: MaterialLedgerId::world(),
                durability_ppm: 1_000_000,
                production: FactoryProductionState {
                    status: FactoryProductionStatus::Blocked,
                    active_jobs: 1,
                    current_job_id: Some(22),
                    current_recipe_id: Some("recipe.motor".to_string()),
                    last_started_at: Some(40),
                    last_completed_at: Some(39),
                    last_blocked_at: Some(41),
                    last_resumed_at: None,
                    current_blocker_kind: Some("material_shortage".to_string()),
                    current_blocker_detail: Some("material_shortage:iron_ingot".to_string()),
                    completed_jobs: 3,
                },
                built_at: 12,
            },
        );

        let snapshot = WorldSnapshot {
            version: SNAPSHOT_VERSION,
            chunk_generation_schema_version: CHUNK_GENERATION_SCHEMA_VERSION,
            time: 42,
            config: WorldConfig::default(),
            model,
            chunk_runtime: ChunkRuntimeConfig::default(),
            next_event_id: 8,
            next_action_id: 4,
            pending_actions: Vec::new(),
            journal_len: 7,
            runtime_snapshot: Some(runtime_snapshot),
            player_gameplay: None,
        };

        let events = vec![
            WorldEvent {
                id: 5,
                time: 40,
                kind: WorldEventKind::RuntimeEvent {
                    kind: "runtime.economy.recipe_started".to_string(),
                    domain_kind: Some(
                        "factory=factory.alpha recipe=recipe.motor requester=agent.alpha batches=1 outputs=motor_mk1x2"
                            .to_string(),
                    ),
                },
                runtime_event: None,
            },
            WorldEvent {
                id: 6,
                time: 41,
                kind: WorldEventKind::RuntimeEvent {
                    kind: "runtime.economy.factory_production_blocked".to_string(),
                    domain_kind: Some(
                        "factory=factory.alpha recipe=recipe.motor requester=agent.alpha reason=material_shortage detail=material_shortage:iron_ingot"
                            .to_string(),
                    ),
                },
                runtime_event: None,
            },
        ];

        let graph = IndustryGraphViewModel::build(Some(&snapshot), &events);
        let summary = industrial_ops_summary_with_zoom(
            &graph,
            Some(&snapshot),
            &events,
            IndustrySemanticZoomLevel::Node,
        )
        .expect("industrial summary exists");
        assert!(summary.contains("Factory Runtime Status:"));
        assert!(summary.contains("running=0 blocked=1 idle=0 active_jobs=1 completed_jobs=3"));
        assert!(summary.contains("Blocked Factories:"));
        assert!(summary.contains("factory=factory.alpha reason=material_shortage"));
        assert!(summary.contains("Recent Production Feedback:"));
        assert!(summary.contains("accepted_and_executing factory=factory.alpha"));
        assert!(summary.contains("blocked factory=factory.alpha"));
    }

    #[test]
    fn industrial_ops_summary_world_zoom_includes_hotspot_lens() {
        let graph = IndustryGraphViewModel::build(None, &[]);
        assert!(industrial_ops_summary_with_zoom(
            &graph,
            None,
            &[],
            IndustrySemanticZoomLevel::World
        )
        .is_none());
    }
}
