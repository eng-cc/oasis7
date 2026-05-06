use super::*;
use std::{hint::black_box, time::Instant};

use oasis7::simulator::{
    Agent, ChunkRuntimeConfig, Location, ModuleVisualAnchor, ModuleVisualEntity, WorldConfig,
    WorldModel, CHUNK_GENERATION_SCHEMA_VERSION, SNAPSHOT_VERSION,
};

fn sample_snapshot() -> WorldSnapshot {
    let mut model = WorldModel::default();
    model.locations.insert(
        "loc-a".to_string(),
        Location::new("loc-a", "Alpha", GeoPos::new(0, 0, 0)),
    );
    model.locations.insert(
        "loc-b".to_string(),
        Location::new("loc-b", "Beta", GeoPos::new(2_100_000, 0, 0)),
    );
    model.agents.insert(
        "agent-1".to_string(),
        Agent::new("agent-1", "loc-a", GeoPos::new(0, 0, 0)),
    );

    model.module_visual_entities.insert(
        "factory-1".to_string(),
        ModuleVisualEntity {
            entity_id: "factory-1".to_string(),
            module_id: "m4.factory.smelter.iron_ingot".to_string(),
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
            module_id: "m4.recipe.module_rack".to_string(),
            kind: "recipe".to_string(),
            label: Some("Rack Recipe".to_string()),
            anchor: ModuleVisualAnchor::Location {
                location_id: "loc-a".to_string(),
            },
        },
    );
    model.module_visual_entities.insert(
        "product-1".to_string(),
        ModuleVisualEntity {
            entity_id: "product-1".to_string(),
            module_id: "m4.product.module_rack".to_string(),
            kind: "product".to_string(),
            label: Some("Module Rack".to_string()),
            anchor: ModuleVisualAnchor::Location {
                location_id: "loc-b".to_string(),
            },
        },
    );

    WorldSnapshot {
        version: SNAPSHOT_VERSION,
        chunk_generation_schema_version: CHUNK_GENERATION_SCHEMA_VERSION,
        time: 77,
        config: WorldConfig::default(),
        model,
        chunk_runtime: ChunkRuntimeConfig::default(),
        next_event_id: 12,
        next_action_id: 4,
        pending_actions: Vec::new(),
        journal_len: 10,
        runtime_snapshot: None,
        player_gameplay: None,
    }
}

#[test]
fn build_graph_aggregates_nodes_edges_and_root_chains() {
    let snapshot = sample_snapshot();
    let events = vec![
        WorldEvent {
            id: 1,
            time: 70,
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
            id: 2,
            time: 71,
            kind: WorldEventKind::Power(PowerEvent::PowerTransferred {
                from: ResourceOwner::Location {
                    location_id: "loc-a".to_string(),
                },
                to: ResourceOwner::Agent {
                    agent_id: "agent-1".to_string(),
                },
                amount: 9,
                loss: 2,
                quoted_price_per_pu: 3,
                price_per_pu: 3,
                settlement_amount: 27,
            }),
            runtime_event: None,
        },
        WorldEvent {
            id: 3,
            time: 72,
            kind: WorldEventKind::CompoundRefined {
                owner: ResourceOwner::Location {
                    location_id: "loc-a".to_string(),
                },
                compound_mass_g: 20,
                electricity_cost: 4,
                hardware_output: 6,
            },
            runtime_event: None,
        },
        WorldEvent {
            id: 4,
            time: 73,
            kind: WorldEventKind::ActionRejected {
                reason: RejectReason::InsufficientResource {
                    owner: ResourceOwner::Location {
                        location_id: "loc-a".to_string(),
                    },
                    kind: ResourceKind::Data,
                    requested: 7,
                    available: 2,
                },
            },
            runtime_event: None,
        },
    ];

    let graph = IndustryGraphViewModel::build(Some(&snapshot), &events);
    assert!(graph.has_industrial_signals());
    assert!(graph.has_economy_signals());
    assert!(graph.has_ops_signals());

    assert!(graph
        .nodes
        .iter()
        .any(|node| node.kind == IndustryNodeKind::Factory));
    assert!(graph
        .edges
        .iter()
        .any(|edge| edge.flow_kind == IndustryFlowKind::Data));
    assert!(graph
        .edges
        .iter()
        .any(|edge| edge.flow_kind == IndustryFlowKind::Electricity));
    assert!(graph
        .edges
        .iter()
        .any(|edge| edge.flow_kind == IndustryFlowKind::Material));

    assert_eq!(graph.rollup.recent_refine_events, 1);
    assert_eq!(graph.rollup.recent_hardware_output, 6);
    assert_eq!(graph.rollup.insufficient_rejects, 1);
    assert_eq!(graph.rollup.power_trade_settlement, 27);

    assert!(!graph.root_cause_chains.is_empty());
    assert!(graph.root_cause_chains[0]
        .shortage_label
        .contains("shortage::Data:5"));
}

#[test]
fn graph_for_zoom_filters_world_and_region() {
    let snapshot = sample_snapshot();
    let events = vec![
        WorldEvent {
            id: 1,
            time: 70,
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
            id: 2,
            time: 71,
            kind: WorldEventKind::ActionRejected {
                reason: RejectReason::LocationNotFound {
                    location_id: "loc-b".to_string(),
                },
            },
            runtime_event: None,
        },
    ];

    let graph = IndustryGraphViewModel::build(Some(&snapshot), &events);
    let world = graph.graph_for_zoom(IndustrySemanticZoomLevel::World);
    let region = graph.graph_for_zoom(IndustrySemanticZoomLevel::Region);
    let node = graph.graph_for_zoom(IndustrySemanticZoomLevel::Node);

    assert!(!world.nodes.is_empty());
    assert!(!world.edges.is_empty());
    assert!(!region.nodes.is_empty());
    assert!(node.nodes.len() >= world.nodes.len());
    assert!(node.edges.len() >= world.edges.len());
}

#[test]
fn infer_tier_and_stage_follow_p3_keywords() {
    assert_eq!(
        infer_tier_from_text(&["m4.product.factory_core"]),
        IndustryTier::R5
    );
    assert_eq!(
        infer_stage_from_text(&["module governance"], IndustryTier::Unknown),
        IndustryStage::Governance
    );
    assert_eq!(
        infer_stage_from_text(&["module sensor_pack"], IndustryTier::R3),
        IndustryStage::Scale
    );
}

#[test]
fn semantic_zoom_state_defaults_to_node() {
    let state = IndustrySemanticZoomState::default();
    assert_eq!(state.level, IndustrySemanticZoomLevel::Node);
}

#[test]
fn zoom_cached_views_match_owned_views() {
    let snapshot = sample_snapshot();
    let events = vec![
        WorldEvent {
            id: 1,
            time: 70,
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
            id: 2,
            time: 71,
            kind: WorldEventKind::ActionRejected {
                reason: RejectReason::LocationNotFound {
                    location_id: "loc-b".to_string(),
                },
            },
            runtime_event: None,
        },
    ];
    let graph = IndustryGraphViewModel::build(Some(&snapshot), &events);
    let owned = graph.graph_for_zoom(IndustrySemanticZoomLevel::World);
    let (cached_nodes, cached_edges) = graph.graph_slice_for_zoom(IndustrySemanticZoomLevel::World);

    assert_eq!(cached_nodes.len(), owned.nodes.len());
    assert_eq!(cached_edges.len(), owned.edges.len());
    assert_eq!(
        graph
            .routes_for_zoom_ref(IndustrySemanticZoomLevel::World)
            .len(),
        graph
            .routes_for_zoom(IndustrySemanticZoomLevel::World)
            .len()
    );
}

fn sample_large_snapshot(
    location_count: usize,
    agent_count: usize,
    visuals_per_location: usize,
) -> WorldSnapshot {
    let mut model = WorldModel::default();
    for idx in 0..location_count {
        let location_id = format!("loc-{idx:03}");
        model.locations.insert(
            location_id.clone(),
            Location::new(
                location_id.as_str(),
                format!("Location {idx:03}"),
                GeoPos::new(idx as i64 * 120_000, 0, (idx % 8) as i64 * 60_000),
            ),
        );
    }

    for idx in 0..agent_count {
        let location_id = format!("loc-{:03}", idx % location_count.max(1));
        let agent_id = format!("agent-{idx:03}");
        model.agents.insert(
            agent_id.clone(),
            Agent::new(
                agent_id.as_str(),
                location_id.as_str(),
                GeoPos::new(idx as i64 * 1000, 0, 0),
            ),
        );
    }

    for location_idx in 0..location_count {
        let location_id = format!("loc-{location_idx:03}");
        for visual_idx in 0..visuals_per_location {
            let (kind, module_prefix, label) = match visual_idx % 3 {
                0 => ("factory", "factory", "Smelter"),
                1 => ("recipe", "recipe", "Recipe"),
                _ => ("product", "product", "Product"),
            };
            let entity_id = format!("{kind}-{location_idx:03}-{visual_idx:02}");
            model.module_visual_entities.insert(
                entity_id.clone(),
                ModuleVisualEntity {
                    entity_id,
                    module_id: format!("m4.{module_prefix}.{location_idx:03}.{visual_idx:02}"),
                    kind: kind.to_string(),
                    label: Some(format!("{label} {location_idx:03}-{visual_idx:02}")),
                    anchor: ModuleVisualAnchor::Location {
                        location_id: location_id.clone(),
                    },
                },
            );
        }
    }

    WorldSnapshot {
        version: SNAPSHOT_VERSION,
        chunk_generation_schema_version: CHUNK_GENERATION_SCHEMA_VERSION,
        time: 512,
        config: WorldConfig::default(),
        model,
        chunk_runtime: ChunkRuntimeConfig::default(),
        next_event_id: 10_000,
        next_action_id: 1_000,
        pending_actions: Vec::new(),
        journal_len: 8_000,
        runtime_snapshot: None,
        player_gameplay: None,
    }
}

fn sample_large_events(
    location_count: usize,
    agent_count: usize,
    event_count: usize,
) -> Vec<WorldEvent> {
    let mut events = Vec::with_capacity(event_count);
    for idx in 0..event_count {
        let location_idx = idx % location_count.max(1);
        let next_location_idx = (idx + 1) % location_count.max(1);
        let agent_idx = idx % agent_count.max(1);
        let time = 1_000 + idx as u64;
        let event = match idx % 4 {
            0 => WorldEvent {
                id: time,
                time,
                kind: WorldEventKind::ResourceTransferred {
                    from: ResourceOwner::Location {
                        location_id: format!("loc-{location_idx:03}"),
                    },
                    to: ResourceOwner::Agent {
                        agent_id: format!("agent-{agent_idx:03}"),
                    },
                    kind: if idx % 8 == 0 {
                        ResourceKind::Electricity
                    } else {
                        ResourceKind::Data
                    },
                    amount: (idx as i64 % 17) + 1,
                },
                runtime_event: None,
            },
            1 => WorldEvent {
                id: time,
                time,
                kind: WorldEventKind::Power(PowerEvent::PowerTransferred {
                    from: ResourceOwner::Location {
                        location_id: format!("loc-{location_idx:03}"),
                    },
                    to: ResourceOwner::Location {
                        location_id: format!("loc-{next_location_idx:03}"),
                    },
                    amount: 20 + (idx as i64 % 11),
                    loss: idx as i64 % 5,
                    quoted_price_per_pu: 3,
                    price_per_pu: 3,
                    settlement_amount: 60 + idx as i64,
                }),
                runtime_event: None,
            },
            2 => WorldEvent {
                id: time,
                time,
                kind: WorldEventKind::CompoundRefined {
                    owner: ResourceOwner::Location {
                        location_id: format!("loc-{location_idx:03}"),
                    },
                    compound_mass_g: 100 + idx as i64,
                    electricity_cost: 10 + (idx as i64 % 9),
                    hardware_output: 5 + (idx as i64 % 7),
                },
                runtime_event: None,
            },
            _ => WorldEvent {
                id: time,
                time,
                kind: WorldEventKind::ActionRejected {
                    reason: RejectReason::InsufficientResource {
                        owner: ResourceOwner::Location {
                            location_id: format!("loc-{location_idx:03}"),
                        },
                        kind: ResourceKind::Data,
                        requested: 50 + idx as i64,
                        available: idx as i64 % 9,
                    },
                },
                runtime_event: None,
            },
        };
        events.push(event);
    }
    events
}

#[test]
#[ignore = "perf harness"]
fn perf_industry_graph_build_and_zoom_paths() {
    let snapshot = sample_large_snapshot(64, 128, 6);
    let events = sample_large_events(64, 128, 6_000);
    let build_iterations = 20;
    let zoom_iterations = 300;

    let build_started_at = Instant::now();
    for _ in 0..build_iterations {
        black_box(IndustryGraphViewModel::build(Some(&snapshot), &events));
    }
    let build_elapsed = build_started_at.elapsed();

    let graph = IndustryGraphViewModel::build(Some(&snapshot), &events);
    let zoom_started_at = Instant::now();
    let mut checksum = 0usize;
    for _ in 0..zoom_iterations {
        checksum = checksum.saturating_add(black_box(
            graph
                .graph_for_zoom(IndustrySemanticZoomLevel::World)
                .edges
                .len(),
        ));
        checksum = checksum.saturating_add(black_box(
            graph
                .graph_for_zoom(IndustrySemanticZoomLevel::Region)
                .nodes
                .len(),
        ));
        checksum = checksum.saturating_add(black_box(
            graph
                .routes_for_zoom(IndustrySemanticZoomLevel::World)
                .len(),
        ));
        checksum = checksum.saturating_add(black_box(
            graph
                .routes_for_zoom(IndustrySemanticZoomLevel::Region)
                .len(),
        ));
    }
    let zoom_elapsed = zoom_started_at.elapsed();

    println!(
        "perf industry_graph build_total_ms={:.2} build_avg_ms={:.3} zoom_total_ms={:.2} zoom_avg_ms={:.3} checksum={} nodes={} edges={} routes={} events={}",
        build_elapsed.as_secs_f64() * 1000.0,
        build_elapsed.as_secs_f64() * 1000.0 / build_iterations as f64,
        zoom_elapsed.as_secs_f64() * 1000.0,
        zoom_elapsed.as_secs_f64() * 1000.0 / zoom_iterations as f64,
        checksum,
        graph.nodes.len(),
        graph.edges.len(),
        graph.routes.len(),
        events.len(),
    );

    assert!(checksum > 0);
}
