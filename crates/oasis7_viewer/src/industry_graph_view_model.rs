use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::Resource;
use oasis7::geometry::GeoPos;
use oasis7::simulator::{
    chunk_coord_of, AssetKind, ChunkCoord, ModuleVisualAnchor, ModuleVisualEntity, PowerEvent,
    RejectReason, ResourceKind, ResourceOwner, WorldEvent, WorldEventKind, WorldSnapshot,
};

const INDUSTRY_EVENT_WINDOW: usize = 192;
const ROOT_CAUSE_CHAIN_LIMIT: usize = 8;
const WORLD_EDGE_LIMIT: usize = 18;
const WORLD_NODE_LIMIT: usize = 24;
const REGION_HOTSPOT_LIMIT: usize = 3;

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IndustrySemanticZoomState {
    pub level: IndustrySemanticZoomLevel,
}

impl Default for IndustrySemanticZoomState {
    fn default() -> Self {
        Self {
            level: IndustrySemanticZoomLevel::Node,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum IndustrySemanticZoomLevel {
    World,
    Region,
    Node,
}

impl IndustrySemanticZoomLevel {
    pub(crate) const ALL: [Self; 3] = [Self::World, Self::Region, Self::Node];

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::World => "world",
            Self::Region => "region",
            Self::Node => "node",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum IndustryNodeKind {
    Factory,
    Recipe,
    Product,
    LogisticsStation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum IndustryFlowKind {
    Material,
    Electricity,
    Data,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum IndustryTier {
    R1,
    R2,
    R3,
    R4,
    R5,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum IndustryStage {
    Bootstrap,
    Scale,
    Governance,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct IndustryNodeStatus {
    pub bottleneck: bool,
    pub congestion: bool,
    pub alert: bool,
    pub bottleneck_events: usize,
    pub congestion_events: usize,
    pub alert_events: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct IndustryGraphNode {
    pub id: String,
    pub label: String,
    pub kind: IndustryNodeKind,
    pub tier: IndustryTier,
    pub stage: IndustryStage,
    pub position: Option<GeoPos>,
    pub chunk: Option<ChunkCoord>,
    pub throughput: i64,
    pub stock_electricity: i64,
    pub stock_data: i64,
    pub status: IndustryNodeStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndustryGraphEdge {
    pub from: String,
    pub to: String,
    pub flow_kind: IndustryFlowKind,
    pub throughput: i64,
    pub transfer_events: usize,
    pub loss: i64,
    pub congested: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndustryRouteStats {
    pub from: String,
    pub to: String,
    pub transfer_events: usize,
    pub material: i64,
    pub electricity: i64,
    pub data: i64,
    pub power: i64,
    pub power_loss: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndustryRegionHotspot {
    pub coord: ChunkCoord,
    pub events: usize,
    pub alerts: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndustryNodeHotspot {
    pub node_id: String,
    pub score: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndustryRootCauseChain {
    pub chain_id: String,
    pub reject_event_id: u64,
    pub reject_label: String,
    pub shortage_label: String,
    pub congestion_label: String,
    pub stall_label: String,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct IndustryGraphRollup {
    pub factory_visuals: usize,
    pub recipe_visuals: usize,
    pub product_visuals: usize,
    pub logistics_visuals: usize,
    pub recent_refine_events: usize,
    pub recent_line_updates: usize,
    pub recent_hardware_output: i64,
    pub transfer_events: usize,
    pub total_power_moved: i64,
    pub total_power_loss: i64,
    pub power_trade_events: usize,
    pub insufficient_rejects: usize,
    pub power_trade_settlement: i64,
    pub refine_electricity_cost: i64,
    pub total_events: usize,
    pub alert_events: usize,
    pub flow_by_kind: BTreeMap<ResourceKind, i64>,
    pub shortfall_by_kind: BTreeMap<ResourceKind, i64>,
    pub stock_by_kind: BTreeMap<ResourceKind, i64>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct IndustryGraphViewModel {
    pub nodes: Vec<IndustryGraphNode>,
    pub edges: Vec<IndustryGraphEdge>,
    pub routes: Vec<IndustryRouteStats>,
    pub region_hotspots: Vec<IndustryRegionHotspot>,
    pub node_hotspots: Vec<IndustryNodeHotspot>,
    pub root_cause_chains: Vec<IndustryRootCauseChain>,
    pub rollup: IndustryGraphRollup,
    world_slice: IndustryGraphSlice,
    region_slice: IndustryGraphSlice,
    world_routes: Vec<IndustryRouteStats>,
    region_routes: Vec<IndustryRouteStats>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct IndustryGraphSlice {
    pub nodes: Vec<IndustryGraphNode>,
    pub edges: Vec<IndustryGraphEdge>,
}

#[derive(Debug, Clone)]
struct RejectEvidence {
    event_id: u64,
    reason_label: String,
    shortage_label: String,
    targets: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct RouteAccumulator {
    transfer_events: usize,
    material: i64,
    electricity: i64,
    data: i64,
    power: i64,
    power_loss: i64,
}

impl IndustryGraphViewModel {
    pub(crate) fn build(snapshot: Option<&WorldSnapshot>, events: &[WorldEvent]) -> Self {
        let mut rollup = IndustryGraphRollup::default();
        let mut nodes = BTreeMap::<String, IndustryGraphNode>::new();
        let mut owner_to_nodes = BTreeMap::<String, Vec<String>>::new();
        let mut edges = BTreeMap::<(String, String, IndustryFlowKind), IndustryGraphEdge>::new();
        let mut routes = BTreeMap::<(String, String), RouteAccumulator>::new();
        let mut region_hotspots = BTreeMap::<ChunkCoord, (usize, usize)>::new();
        let mut node_hotspots = BTreeMap::<String, usize>::new();
        let mut reject_evidence = Vec::<RejectEvidence>::new();

        if let Some(snapshot) = snapshot {
            for entity in snapshot.model.module_visual_entities.values() {
                let Some(kind) = classify_visual_node_kind(entity) else {
                    continue;
                };
                match kind {
                    IndustryNodeKind::Factory => rollup.factory_visuals += 1,
                    IndustryNodeKind::Recipe => rollup.recipe_visuals += 1,
                    IndustryNodeKind::Product => rollup.product_visuals += 1,
                    IndustryNodeKind::LogisticsStation => rollup.logistics_visuals += 1,
                }

                let position = resolve_module_position(snapshot, &entity.anchor);
                let chunk = position.and_then(|pos| chunk_coord_of(pos, &snapshot.config.space));
                let tier = infer_tier_from_text(&[
                    entity.entity_id.as_str(),
                    entity.module_id.as_str(),
                    entity.kind.as_str(),
                    entity.label.as_deref().unwrap_or_default(),
                ]);
                let stage = infer_stage_from_text(
                    &[
                        entity.module_id.as_str(),
                        entity.kind.as_str(),
                        entity.label.as_deref().unwrap_or_default(),
                    ],
                    tier,
                );
                let node_id = format!("module::{}", entity.entity_id);
                let node = IndustryGraphNode {
                    id: node_id.clone(),
                    label: entity
                        .label
                        .clone()
                        .filter(|label| !label.trim().is_empty())
                        .unwrap_or_else(|| entity.module_id.clone()),
                    kind,
                    tier,
                    stage,
                    position,
                    chunk,
                    throughput: 0,
                    stock_electricity: 0,
                    stock_data: 0,
                    status: IndustryNodeStatus::default(),
                };
                nodes.insert(node_id.clone(), node);

                if let Some(owner) = anchor_owner_label(&entity.anchor) {
                    owner_to_nodes.entry(owner).or_default().push(node_id);
                }
            }

            for (location_id, location) in &snapshot.model.locations {
                let owner = format!("location::{location_id}");
                upsert_owner_logistics_node(
                    &mut nodes,
                    Some(snapshot),
                    &owner,
                    Some(location.name.as_str()),
                    Some(location.pos),
                );
                owner_to_nodes
                    .entry(owner.clone())
                    .or_default()
                    .push(owner.clone());
            }

            for agent_id in snapshot.model.agents.keys() {
                let owner = format!("agent::{agent_id}");
                upsert_owner_logistics_node(&mut nodes, Some(snapshot), &owner, None, None);
                owner_to_nodes
                    .entry(owner.clone())
                    .or_default()
                    .push(owner.clone());
            }

            collect_resource_stocks(snapshot, &mut rollup.stock_by_kind);
            populate_owner_inventory(snapshot, &mut nodes);
        }

        for event in events.iter().rev().take(INDUSTRY_EVENT_WINDOW) {
            rollup.total_events += 1;
            let is_alert = matches!(event.kind, WorldEventKind::ActionRejected { .. });
            if is_alert {
                rollup.alert_events += 1;
            }

            if let Some(snapshot) = snapshot {
                if let Some(coord) = chunk_coord_for_event(snapshot, event) {
                    let entry = region_hotspots.entry(coord).or_insert((0, 0));
                    entry.0 += 1;
                    if is_alert {
                        entry.1 += 1;
                    }
                }
            }

            for target in node_targets_for_event(event) {
                let entry = node_hotspots.entry(target).or_insert(0);
                *entry += 1;
            }

            match &event.kind {
                WorldEventKind::ResourceTransferred {
                    from,
                    to,
                    kind,
                    amount,
                } => {
                    rollup.transfer_events += 1;
                    add_amount(&mut rollup.flow_by_kind, *kind, amount.abs());

                    let from_id = owner_label(from);
                    let to_id = owner_label(to);
                    ensure_owner_nodes(snapshot, &from_id, &to_id, &mut nodes, &mut owner_to_nodes);
                    add_edge(
                        &mut edges,
                        from_id.as_str(),
                        to_id.as_str(),
                        match kind {
                            ResourceKind::Electricity => IndustryFlowKind::Electricity,
                            ResourceKind::Data => IndustryFlowKind::Data,
                        },
                        amount.abs(),
                        0,
                    );
                    add_route_flow(
                        &mut routes,
                        from_id.as_str(),
                        to_id.as_str(),
                        match kind {
                            ResourceKind::Electricity => IndustryFlowKind::Electricity,
                            ResourceKind::Data => IndustryFlowKind::Data,
                        },
                        amount.abs(),
                        0,
                    );
                    bump_owner_throughput(
                        &mut nodes,
                        &owner_to_nodes,
                        from_id.as_str(),
                        amount.abs(),
                        false,
                        false,
                        false,
                    );
                    bump_owner_throughput(
                        &mut nodes,
                        &owner_to_nodes,
                        to_id.as_str(),
                        amount.abs(),
                        false,
                        false,
                        false,
                    );
                }
                WorldEventKind::Power(PowerEvent::PowerTransferred {
                    from,
                    to,
                    amount,
                    loss,
                    price_per_pu,
                    ..
                }) => {
                    rollup.transfer_events += 1;
                    rollup.power_trade_events += 1;
                    rollup.total_power_moved =
                        rollup.total_power_moved.saturating_add(amount.abs());
                    rollup.total_power_loss = rollup.total_power_loss.saturating_add(loss.abs());
                    add_amount(
                        &mut rollup.flow_by_kind,
                        ResourceKind::Electricity,
                        amount.abs(),
                    );

                    let settlement = amount.abs().saturating_mul((*price_per_pu).max(0));
                    rollup.power_trade_settlement =
                        rollup.power_trade_settlement.saturating_add(settlement);

                    let from_id = owner_label(from);
                    let to_id = owner_label(to);
                    ensure_owner_nodes(snapshot, &from_id, &to_id, &mut nodes, &mut owner_to_nodes);
                    let throughput = amount.abs().saturating_add(loss.abs());
                    add_edge(
                        &mut edges,
                        from_id.as_str(),
                        to_id.as_str(),
                        IndustryFlowKind::Electricity,
                        throughput,
                        loss.abs(),
                    );
                    add_route_flow(
                        &mut routes,
                        from_id.as_str(),
                        to_id.as_str(),
                        IndustryFlowKind::Electricity,
                        amount.abs(),
                        loss.abs(),
                    );
                    bump_owner_throughput(
                        &mut nodes,
                        &owner_to_nodes,
                        from_id.as_str(),
                        throughput,
                        false,
                        false,
                        false,
                    );
                    bump_owner_throughput(
                        &mut nodes,
                        &owner_to_nodes,
                        to_id.as_str(),
                        throughput,
                        false,
                        false,
                        false,
                    );
                }
                WorldEventKind::CompoundRefined {
                    owner,
                    electricity_cost,
                    hardware_output,
                    ..
                } => {
                    rollup.recent_refine_events += 1;
                    rollup.recent_hardware_output = rollup
                        .recent_hardware_output
                        .saturating_add(*hardware_output);
                    rollup.refine_electricity_cost = rollup
                        .refine_electricity_cost
                        .saturating_add((*electricity_cost).max(0));

                    let owner_id = owner_label(owner);
                    ensure_single_owner_node(snapshot, &owner_id, &mut nodes, &mut owner_to_nodes);
                    bump_owner_throughput(
                        &mut nodes,
                        &owner_to_nodes,
                        owner_id.as_str(),
                        hardware_output.abs(),
                        false,
                        false,
                        false,
                    );

                    // Derive a synthetic material edge for "product stream" readability.
                    let material_target = owner_to_nodes
                        .get(owner_id.as_str())
                        .and_then(|entries| {
                            entries.iter().find(|id| {
                                nodes.get(id.as_str()).is_some_and(|node| {
                                    node.kind == IndustryNodeKind::Product
                                        || node.kind == IndustryNodeKind::Recipe
                                })
                            })
                        })
                        .cloned()
                        .unwrap_or_else(|| owner_id.clone());
                    add_edge(
                        &mut edges,
                        owner_id.as_str(),
                        material_target.as_str(),
                        IndustryFlowKind::Material,
                        hardware_output.abs(),
                        0,
                    );
                    add_route_flow(
                        &mut routes,
                        owner_id.as_str(),
                        material_target.as_str(),
                        IndustryFlowKind::Material,
                        hardware_output.abs(),
                        0,
                    );
                }
                WorldEventKind::ModuleVisualEntityUpserted { entity } => {
                    if let Some(kind) = classify_visual_node_kind(entity) {
                        rollup.recent_line_updates += 1;
                        if snapshot.is_some() {
                            continue;
                        }

                        let owner = anchor_owner_label(&entity.anchor);
                        let node_id = format!("module::{}", entity.entity_id);
                        nodes
                            .entry(node_id.clone())
                            .or_insert_with(|| IndustryGraphNode {
                                id: node_id.clone(),
                                label: entity
                                    .label
                                    .clone()
                                    .filter(|label| !label.trim().is_empty())
                                    .unwrap_or_else(|| entity.module_id.clone()),
                                kind,
                                tier: infer_tier_from_text(&[
                                    entity.entity_id.as_str(),
                                    entity.module_id.as_str(),
                                    entity.kind.as_str(),
                                ]),
                                stage: infer_stage_from_text(
                                    &[entity.module_id.as_str(), entity.kind.as_str()],
                                    infer_tier_from_text(&[
                                        entity.entity_id.as_str(),
                                        entity.module_id.as_str(),
                                        entity.kind.as_str(),
                                    ]),
                                ),
                                position: None,
                                chunk: None,
                                throughput: 0,
                                stock_electricity: 0,
                                stock_data: 0,
                                status: IndustryNodeStatus::default(),
                            });
                        if let Some(owner) = owner {
                            owner_to_nodes.entry(owner).or_default().push(node_id);
                        }
                    }
                }
                WorldEventKind::ActionRejected { reason } => {
                    let reason_label = root_cause_key(reason);
                    let (targets, shortage_label, is_insufficient) = reject_targets(reason);

                    if is_insufficient {
                        rollup.insufficient_rejects += 1;
                    }

                    if let RejectReason::InsufficientResource {
                        kind,
                        requested,
                        available,
                        ..
                    } = reason
                    {
                        let shortfall = requested.saturating_sub(*available).max(0);
                        add_amount(&mut rollup.shortfall_by_kind, *kind, shortfall);
                    }

                    for target in &targets {
                        ensure_single_owner_node(snapshot, target, &mut nodes, &mut owner_to_nodes);
                        bump_owner_throughput(
                            &mut nodes,
                            &owner_to_nodes,
                            target,
                            0,
                            is_insufficient,
                            false,
                            true,
                        );
                    }

                    reject_evidence.push(RejectEvidence {
                        event_id: event.id,
                        reason_label,
                        shortage_label,
                        targets,
                    });
                }
                _ => {}
            }
        }

        // Route congestion and node status propagation.
        for edge in edges.values_mut() {
            if edge.transfer_events >= 3
                || (edge.loss > 0
                    && edge.throughput > 0
                    && edge.loss.saturating_mul(5) >= edge.throughput)
            {
                edge.congested = true;
                mark_node_congestion(&mut nodes, edge.from.as_str());
                mark_node_congestion(&mut nodes, edge.to.as_str());
            }
        }

        let mut root_cause_chains = Vec::new();
        for (idx, reject) in reject_evidence
            .into_iter()
            .take(ROOT_CAUSE_CHAIN_LIMIT)
            .enumerate()
        {
            let congestion = select_congestion_label(&edges, reject.targets.as_slice());
            let stall = select_stall_label(&nodes, &owner_to_nodes, reject.targets.as_slice());

            let mut targets = reject.targets;
            if let Some((from, to)) = parse_congestion_targets(congestion.as_str()) {
                targets.push(from);
                targets.push(to);
            }
            if !stall.is_empty() && stall != "none" {
                targets.push(stall.clone());
            }
            dedup_in_place(&mut targets);

            root_cause_chains.push(IndustryRootCauseChain {
                chain_id: format!("rc-{:02}", idx + 1),
                reject_event_id: reject.event_id,
                reject_label: reject.reason_label,
                shortage_label: reject.shortage_label,
                congestion_label: congestion,
                stall_label: stall,
                targets,
            });
        }

        let mut nodes: Vec<_> = nodes.into_values().collect();
        nodes.sort_by(|left, right| {
            right
                .throughput
                .cmp(&left.throughput)
                .then_with(|| right.status.alert_events.cmp(&left.status.alert_events))
                .then_with(|| left.id.cmp(&right.id))
        });

        let mut edges: Vec<_> = edges.into_values().collect();
        edges.sort_by(|left, right| {
            right
                .throughput
                .cmp(&left.throughput)
                .then_with(|| right.transfer_events.cmp(&left.transfer_events))
                .then_with(|| left.from.cmp(&right.from))
                .then_with(|| left.to.cmp(&right.to))
        });

        let mut routes: Vec<_> = routes
            .into_iter()
            .map(|((from, to), value)| IndustryRouteStats {
                from,
                to,
                transfer_events: value.transfer_events,
                material: value.material,
                electricity: value.electricity,
                data: value.data,
                power: value.power,
                power_loss: value.power_loss,
            })
            .collect();
        routes.sort_by(|left, right| {
            route_weight(right)
                .cmp(&route_weight(left))
                .then_with(|| right.transfer_events.cmp(&left.transfer_events))
                .then_with(|| left.from.cmp(&right.from))
                .then_with(|| left.to.cmp(&right.to))
        });

        let mut region_hotspots: Vec<_> = region_hotspots
            .into_iter()
            .map(|(coord, (events, alerts))| IndustryRegionHotspot {
                coord,
                events,
                alerts,
            })
            .collect();
        region_hotspots.sort_by(|left, right| {
            right
                .alerts
                .cmp(&left.alerts)
                .then_with(|| right.events.cmp(&left.events))
                .then_with(|| left.coord.cmp(&right.coord))
        });

        let mut node_hotspots: Vec<_> = node_hotspots
            .into_iter()
            .map(|(node_id, score)| IndustryNodeHotspot { node_id, score })
            .collect();
        node_hotspots.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.node_id.cmp(&right.node_id))
        });

        let world_slice = build_world_zoom_slice(&nodes, &edges);
        let region_slice = build_region_zoom_slice(&nodes, &edges, &region_hotspots, &world_slice);
        let world_routes = filter_routes_for_slice(&routes, &world_slice);
        let region_routes = filter_routes_for_slice(&routes, &region_slice);

        Self {
            nodes,
            edges,
            routes,
            region_hotspots,
            node_hotspots,
            root_cause_chains,
            rollup,
            world_slice,
            region_slice,
            world_routes,
            region_routes,
        }
    }

    pub(crate) fn has_industrial_signals(&self) -> bool {
        self.rollup.factory_visuals > 0
            || self.rollup.recipe_visuals > 0
            || self.rollup.product_visuals > 0
            || self.rollup.logistics_visuals > 0
            || self.rollup.recent_refine_events > 0
            || self.rollup.recent_line_updates > 0
            || !self.routes.is_empty()
            || self.rollup.transfer_events > 0
    }

    pub(crate) fn has_economy_signals(&self) -> bool {
        self.rollup.transfer_events > 0
            || self.rollup.power_trade_events > 0
            || self.rollup.recent_refine_events > 0
            || self.rollup.insufficient_rejects > 0
            || !self.rollup.flow_by_kind.is_empty()
    }

    pub(crate) fn has_ops_signals(&self) -> bool {
        self.rollup.total_events > 0
    }

    pub(crate) fn routes_for_zoom(
        &self,
        zoom: IndustrySemanticZoomLevel,
    ) -> Vec<IndustryRouteStats> {
        self.routes_for_zoom_ref(zoom).to_vec()
    }

    pub(crate) fn routes_for_zoom_ref(
        &self,
        zoom: IndustrySemanticZoomLevel,
    ) -> &[IndustryRouteStats] {
        match zoom {
            IndustrySemanticZoomLevel::Node => self.routes.as_slice(),
            IndustrySemanticZoomLevel::World => self.world_routes.as_slice(),
            IndustrySemanticZoomLevel::Region => self.region_routes.as_slice(),
        }
    }

    pub(crate) fn graph_slice_for_zoom(
        &self,
        zoom: IndustrySemanticZoomLevel,
    ) -> (&[IndustryGraphNode], &[IndustryGraphEdge]) {
        match zoom {
            IndustrySemanticZoomLevel::Node => (&self.nodes, &self.edges),
            IndustrySemanticZoomLevel::World => (&self.world_slice.nodes, &self.world_slice.edges),
            IndustrySemanticZoomLevel::Region => {
                (&self.region_slice.nodes, &self.region_slice.edges)
            }
        }
    }

    pub(crate) fn graph_for_zoom(&self, zoom: IndustrySemanticZoomLevel) -> IndustryGraphSlice {
        let (nodes, edges) = self.graph_slice_for_zoom(zoom);
        IndustryGraphSlice {
            nodes: nodes.to_vec(),
            edges: edges.to_vec(),
        }
    }
}

fn build_world_zoom_slice(
    nodes: &[IndustryGraphNode],
    edges: &[IndustryGraphEdge],
) -> IndustryGraphSlice {
    let edges = edges[..edges.len().min(WORLD_EDGE_LIMIT)].to_vec();

    let mut node_ids = BTreeSet::<String>::new();
    for edge in &edges {
        node_ids.insert(edge.from.clone());
        node_ids.insert(edge.to.clone());
    }

    let mut nodes: Vec<_> = nodes
        .iter()
        .filter(|node| node_ids.contains(node.id.as_str()))
        .cloned()
        .collect();
    nodes.sort_by(|left, right| {
        right
            .throughput
            .cmp(&left.throughput)
            .then_with(|| left.id.cmp(&right.id))
    });
    if nodes.len() > WORLD_NODE_LIMIT {
        nodes.truncate(WORLD_NODE_LIMIT);
    }

    IndustryGraphSlice { nodes, edges }
}

fn build_region_zoom_slice(
    nodes: &[IndustryGraphNode],
    edges: &[IndustryGraphEdge],
    region_hotspots: &[IndustryRegionHotspot],
    world_slice: &IndustryGraphSlice,
) -> IndustryGraphSlice {
    let allowed_chunks: BTreeSet<_> = region_hotspots
        .iter()
        .take(REGION_HOTSPOT_LIMIT)
        .map(|entry| entry.coord)
        .collect();

    let mut nodes: Vec<_> = nodes
        .iter()
        .filter(|node| {
            node.chunk
                .is_some_and(|coord| allowed_chunks.contains(&coord))
        })
        .cloned()
        .collect();

    if nodes.is_empty() {
        return world_slice.clone();
    }

    let node_ids: BTreeSet<_> = nodes.iter().map(|node| node.id.as_str()).collect();
    let mut edges: Vec<_> = edges
        .iter()
        .filter(|edge| node_ids.contains(edge.from.as_str()) || node_ids.contains(edge.to.as_str()))
        .take(WORLD_EDGE_LIMIT)
        .cloned()
        .collect();

    if edges.len() > WORLD_EDGE_LIMIT {
        edges.truncate(WORLD_EDGE_LIMIT);
    }

    nodes.sort_by(|left, right| {
        right
            .throughput
            .cmp(&left.throughput)
            .then_with(|| right.status.alert_events.cmp(&left.status.alert_events))
            .then_with(|| left.id.cmp(&right.id))
    });

    IndustryGraphSlice { nodes, edges }
}

fn filter_routes_for_slice(
    routes: &[IndustryRouteStats],
    slice: &IndustryGraphSlice,
) -> Vec<IndustryRouteStats> {
    let node_ids: BTreeSet<_> = slice.nodes.iter().map(|node| node.id.as_str()).collect();
    routes
        .iter()
        .filter(|route| {
            node_ids.is_empty()
                || node_ids.contains(route.from.as_str())
                || node_ids.contains(route.to.as_str())
        })
        .cloned()
        .collect()
}

#[path = "industry_graph_view_model_support.rs"]
mod industry_graph_view_model_support;

use industry_graph_view_model_support::*;

#[cfg(test)]
#[path = "industry_graph_view_model_tests.rs"]
mod tests;
