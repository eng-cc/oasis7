use super::*;

pub(super) fn parse_congestion_targets(label: &str) -> Option<(String, String)> {
    let payload = label.strip_prefix("route::")?;
    let mut parts = payload.split("->");
    let from = parts.next()?.trim().to_string();
    let to = parts.next()?.trim().to_string();
    Some((from, to))
}

pub(super) fn select_congestion_label(
    edges: &BTreeMap<(String, String, IndustryFlowKind), IndustryGraphEdge>,
    targets: &[String],
) -> String {
    let target_set: BTreeSet<_> = targets.iter().map(|value| value.as_str()).collect();

    let best = edges
        .values()
        .filter(|edge| edge.congested)
        .max_by(|left, right| {
            let left_hits = target_set.contains(left.from.as_str()) as i32
                + target_set.contains(left.to.as_str()) as i32;
            let right_hits = target_set.contains(right.from.as_str()) as i32
                + target_set.contains(right.to.as_str()) as i32;
            left_hits
                .cmp(&right_hits)
                .then_with(|| left.throughput.cmp(&right.throughput))
                .then_with(|| left.transfer_events.cmp(&right.transfer_events))
        });

    match best {
        Some(edge) => format!("route::{} -> {}", edge.from, edge.to),
        None => "none".to_string(),
    }
}

pub(super) fn select_stall_label(
    nodes: &BTreeMap<String, IndustryGraphNode>,
    owner_to_nodes: &BTreeMap<String, Vec<String>>,
    targets: &[String],
) -> String {
    let mut candidates = Vec::<IndustryGraphNode>::new();

    for target in targets {
        if let Some(entries) = owner_to_nodes.get(target.as_str()) {
            for entry in entries {
                if let Some(node) = nodes.get(entry.as_str()) {
                    if matches!(
                        node.kind,
                        IndustryNodeKind::Factory | IndustryNodeKind::Recipe
                    ) {
                        candidates.push(node.clone());
                    }
                }
            }
        }
    }

    candidates.sort_by(|left, right| {
        right
            .status
            .bottleneck_events
            .cmp(&left.status.bottleneck_events)
            .then_with(|| right.throughput.cmp(&left.throughput))
            .then_with(|| left.id.cmp(&right.id))
    });

    candidates
        .first()
        .map(|node| node.id.clone())
        .unwrap_or_else(|| "none".to_string())
}

pub(super) fn dedup_in_place(entries: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    entries.retain(|entry| seen.insert(entry.clone()));
}

pub(super) fn mark_node_congestion(nodes: &mut BTreeMap<String, IndustryGraphNode>, node_id: &str) {
    if let Some(node) = nodes.get_mut(node_id) {
        node.status.congestion = true;
        node.status.congestion_events += 1;
    }
}

pub(super) fn reject_targets(reason: &RejectReason) -> (Vec<String>, String, bool) {
    match reason {
        RejectReason::InsufficientResource {
            owner,
            kind,
            requested,
            available,
        } => {
            let owner = owner_label(owner);
            let shortfall = requested.saturating_sub(*available).max(0);
            (vec![owner], format!("shortage::{kind:?}:{shortfall}"), true)
        }
        RejectReason::AgentNotFound { agent_id } => (
            vec![format!("agent::{agent_id}")],
            "shortage::none".to_string(),
            false,
        ),
        RejectReason::AgentNotAtLocation {
            agent_id,
            location_id,
        }
        | RejectReason::AgentAlreadyAtLocation {
            agent_id,
            location_id,
        } => (
            vec![
                format!("agent::{agent_id}"),
                format!("location::{location_id}"),
            ],
            "shortage::none".to_string(),
            false,
        ),
        RejectReason::LocationNotFound { location_id } => (
            vec![format!("location::{location_id}")],
            "shortage::none".to_string(),
            false,
        ),
        _ => (Vec::new(), "shortage::none".to_string(), false),
    }
}

pub(super) fn chunk_coord_for_event(
    snapshot: &WorldSnapshot,
    event: &WorldEvent,
) -> Option<ChunkCoord> {
    if let WorldEventKind::ChunkGenerated { coord, .. } = event.kind {
        return Some(coord);
    }

    let location_id = location_target_for_event(event)?;
    let location = snapshot.model.locations.get(location_id.as_str())?;
    chunk_coord_of(location.pos, &snapshot.config.space)
}

pub(super) fn location_target_for_event(event: &WorldEvent) -> Option<String> {
    match &event.kind {
        WorldEventKind::LocationRegistered { location_id, .. } => Some(location_id.clone()),
        WorldEventKind::AgentRegistered { location_id, .. } => Some(location_id.clone()),
        WorldEventKind::AgentMoved { to, .. } => Some(to.clone()),
        WorldEventKind::RadiationHarvested { location_id, .. } => Some(location_id.clone()),
        WorldEventKind::Power(PowerEvent::PowerGenerated { location_id, .. }) => {
            Some(location_id.clone())
        }
        WorldEventKind::ResourceTransferred { from, to, .. } => {
            owner_location_id(from).or_else(|| owner_location_id(to))
        }
        WorldEventKind::Power(PowerEvent::PowerTransferred { from, to, .. }) => {
            owner_location_id(from).or_else(|| owner_location_id(to))
        }
        WorldEventKind::ActionRejected { reason } => match reason {
            RejectReason::LocationNotFound { location_id } => Some(location_id.clone()),
            RejectReason::AgentNotAtLocation { location_id, .. } => Some(location_id.clone()),
            RejectReason::AgentAlreadyAtLocation { location_id, .. } => Some(location_id.clone()),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn node_targets_for_event(event: &WorldEvent) -> Vec<String> {
    match &event.kind {
        WorldEventKind::AgentRegistered { agent_id, .. } => vec![format!("agent::{agent_id}")],
        WorldEventKind::AgentMoved {
            agent_id, from, to, ..
        } => vec![
            format!("agent::{agent_id}"),
            format!("location::{from}"),
            format!("location::{to}"),
        ],
        WorldEventKind::RadiationHarvested {
            agent_id,
            location_id,
            ..
        } => vec![
            format!("agent::{agent_id}"),
            format!("location::{location_id}"),
        ],
        WorldEventKind::ResourceTransferred { from, to, .. } => {
            vec![owner_label(from), owner_label(to)]
        }
        WorldEventKind::Power(PowerEvent::PowerTransferred { from, to, .. }) => {
            vec![owner_label(from), owner_label(to)]
        }
        WorldEventKind::ActionRejected { reason } => match reason {
            RejectReason::AgentNotFound { agent_id } => vec![format!("agent::{agent_id}")],
            RejectReason::AgentNotAtLocation {
                agent_id,
                location_id,
            } => vec![
                format!("agent::{agent_id}"),
                format!("location::{location_id}"),
            ],
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

pub(super) fn owner_location_id(owner: &ResourceOwner) -> Option<String> {
    match owner {
        ResourceOwner::Location { location_id } => Some(location_id.clone()),
        _ => None,
    }
}

pub(super) fn ensure_owner_nodes(
    snapshot: Option<&WorldSnapshot>,
    from: &str,
    to: &str,
    nodes: &mut BTreeMap<String, IndustryGraphNode>,
    owner_to_nodes: &mut BTreeMap<String, Vec<String>>,
) {
    ensure_single_owner_node(snapshot, from, nodes, owner_to_nodes);
    ensure_single_owner_node(snapshot, to, nodes, owner_to_nodes);
}

pub(super) fn ensure_single_owner_node(
    snapshot: Option<&WorldSnapshot>,
    owner: &str,
    nodes: &mut BTreeMap<String, IndustryGraphNode>,
    owner_to_nodes: &mut BTreeMap<String, Vec<String>>,
) {
    if nodes.contains_key(owner) {
        return;
    }

    upsert_owner_logistics_node(nodes, snapshot, owner, None, None);
    owner_to_nodes
        .entry(owner.to_string())
        .or_default()
        .push(owner.to_string());
}

pub(super) fn upsert_owner_logistics_node(
    nodes: &mut BTreeMap<String, IndustryGraphNode>,
    snapshot: Option<&WorldSnapshot>,
    owner: &str,
    label_hint: Option<&str>,
    pos_hint: Option<GeoPos>,
) {
    nodes.entry(owner.to_string()).or_insert_with(|| {
        let position =
            pos_hint.or_else(|| snapshot.and_then(|snapshot| owner_position(snapshot, owner)));
        let chunk = snapshot.and_then(|snapshot| {
            position.and_then(|pos| chunk_coord_of(pos, &snapshot.config.space))
        });
        let tier = infer_tier_from_text(&[owner]);
        let stage = infer_stage_from_text(&[owner], tier);

        IndustryGraphNode {
            id: owner.to_string(),
            label: label_hint
                .map(|value| value.to_string())
                .unwrap_or_else(|| owner.to_string()),
            kind: IndustryNodeKind::LogisticsStation,
            tier,
            stage,
            position,
            chunk,
            throughput: 0,
            stock_electricity: 0,
            stock_data: 0,
            status: IndustryNodeStatus::default(),
        }
    });
}

pub(super) fn resolve_module_position(
    snapshot: &WorldSnapshot,
    anchor: &ModuleVisualAnchor,
) -> Option<GeoPos> {
    match anchor {
        ModuleVisualAnchor::Agent { agent_id } => {
            snapshot.model.agents.get(agent_id).map(|agent| agent.pos)
        }
        ModuleVisualAnchor::Location { location_id } => snapshot
            .model
            .locations
            .get(location_id)
            .map(|location| location.pos),
        ModuleVisualAnchor::Absolute { pos } => Some(*pos),
    }
}

pub(super) fn anchor_owner_label(anchor: &ModuleVisualAnchor) -> Option<String> {
    match anchor {
        ModuleVisualAnchor::Agent { agent_id } => Some(format!("agent::{agent_id}")),
        ModuleVisualAnchor::Location { location_id } => Some(format!("location::{location_id}")),
        ModuleVisualAnchor::Absolute { .. } => None,
    }
}

pub(super) fn owner_position(snapshot: &WorldSnapshot, owner: &str) -> Option<GeoPos> {
    if let Some(agent_id) = owner.strip_prefix("agent::") {
        return snapshot.model.agents.get(agent_id).map(|agent| agent.pos);
    }
    if let Some(location_id) = owner.strip_prefix("location::") {
        return snapshot
            .model
            .locations
            .get(location_id)
            .map(|location| location.pos);
    }
    None
}

pub(super) fn populate_owner_inventory(
    snapshot: &WorldSnapshot,
    nodes: &mut BTreeMap<String, IndustryGraphNode>,
) {
    for (agent_id, agent) in &snapshot.model.agents {
        if let Some(node) = nodes.get_mut(format!("agent::{agent_id}").as_str()) {
            node.stock_electricity = agent.resources.get(ResourceKind::Electricity).max(0);
            node.stock_data = agent.resources.get(ResourceKind::Data).max(0);
        }
    }

    for (location_id, location) in &snapshot.model.locations {
        if let Some(node) = nodes.get_mut(format!("location::{location_id}").as_str()) {
            node.stock_electricity = location.resources.get(ResourceKind::Electricity).max(0);
            node.stock_data = location.resources.get(ResourceKind::Data).max(0);
        }
    }
}

pub(super) fn add_edge(
    edges: &mut BTreeMap<(String, String, IndustryFlowKind), IndustryGraphEdge>,
    from: &str,
    to: &str,
    flow_kind: IndustryFlowKind,
    throughput: i64,
    loss: i64,
) {
    let key = (from.to_string(), to.to_string(), flow_kind);
    edges
        .entry(key)
        .and_modify(|edge| {
            edge.transfer_events += 1;
            edge.throughput = edge.throughput.saturating_add(throughput.max(0));
            edge.loss = edge.loss.saturating_add(loss.max(0));
        })
        .or_insert_with(|| IndustryGraphEdge {
            from: from.to_string(),
            to: to.to_string(),
            flow_kind,
            throughput: throughput.max(0),
            transfer_events: 1,
            loss: loss.max(0),
            congested: false,
        });
}

pub(super) fn add_route_flow(
    routes: &mut BTreeMap<(String, String), RouteAccumulator>,
    from: &str,
    to: &str,
    flow_kind: IndustryFlowKind,
    amount: i64,
    loss: i64,
) {
    let entry = routes
        .entry((from.to_string(), to.to_string()))
        .or_default();
    entry.transfer_events += 1;
    match flow_kind {
        IndustryFlowKind::Material => {
            entry.material = entry.material.saturating_add(amount.max(0));
        }
        IndustryFlowKind::Electricity => {
            entry.electricity = entry.electricity.saturating_add(amount.max(0));
            entry.power = entry.power.saturating_add(amount.max(0));
            entry.power_loss = entry.power_loss.saturating_add(loss.max(0));
        }
        IndustryFlowKind::Data => {
            entry.data = entry.data.saturating_add(amount.max(0));
        }
    }
}

pub(super) fn bump_owner_throughput(
    nodes: &mut BTreeMap<String, IndustryGraphNode>,
    owner_to_nodes: &BTreeMap<String, Vec<String>>,
    owner: &str,
    throughput: i64,
    bottleneck: bool,
    congestion: bool,
    alert: bool,
) {
    if let Some(entries) = owner_to_nodes.get(owner) {
        for node_id in entries {
            if let Some(node) = nodes.get_mut(node_id.as_str()) {
                node.throughput = node.throughput.saturating_add(throughput.max(0));
                if bottleneck {
                    node.status.bottleneck = true;
                    node.status.bottleneck_events += 1;
                }
                if congestion {
                    node.status.congestion = true;
                    node.status.congestion_events += 1;
                }
                if alert {
                    node.status.alert = true;
                    node.status.alert_events += 1;
                }
            }
        }
    }

    if let Some(node) = nodes.get_mut(owner) {
        node.throughput = node.throughput.saturating_add(throughput.max(0));
        if bottleneck {
            node.status.bottleneck = true;
            node.status.bottleneck_events += 1;
        }
        if congestion {
            node.status.congestion = true;
            node.status.congestion_events += 1;
        }
        if alert {
            node.status.alert = true;
            node.status.alert_events += 1;
        }
    }
}

pub(super) fn classify_visual_node_kind(entity: &ModuleVisualEntity) -> Option<IndustryNodeKind> {
    let module_id = entity.module_id.to_ascii_lowercase();
    let kind = entity.kind.to_ascii_lowercase();

    if module_id.contains("recipe") || kind.contains("recipe") {
        return Some(IndustryNodeKind::Recipe);
    }

    if module_id.contains("product") || kind.contains("product") {
        return Some(IndustryNodeKind::Product);
    }

    if module_id.contains("logistics")
        || kind.contains("logistics")
        || module_id.contains("transport")
        || kind.contains("relay")
        || module_id.contains("station")
        || kind.contains("station")
    {
        return Some(IndustryNodeKind::LogisticsStation);
    }

    if module_id.contains("factory")
        || kind.contains("factory")
        || module_id.contains("miner")
        || module_id.contains("smelter")
        || module_id.contains("assembler")
    {
        return Some(IndustryNodeKind::Factory);
    }

    None
}

pub(super) fn infer_tier_from_text(parts: &[&str]) -> IndustryTier {
    let raw = parts.join(" ").to_ascii_lowercase();

    if raw.contains("r5")
        || raw.contains("factory_core")
        || raw.contains("relay_tower")
        || raw.contains("grid_buffer")
        || raw.contains("governance")
    {
        return IndustryTier::R5;
    }
    if raw.contains("r4")
        || raw.contains("drone")
        || raw.contains("repair_kit")
        || raw.contains("survey_probe")
        || raw.contains("module_rack")
    {
        return IndustryTier::R4;
    }
    if raw.contains("r3")
        || raw.contains("gear")
        || raw.contains("chip")
        || raw.contains("motor")
        || raw.contains("sensor")
        || raw.contains("power_core")
    {
        return IndustryTier::R3;
    }
    if raw.contains("r2")
        || raw.contains("ingot")
        || raw.contains("wire")
        || raw.contains("alloy")
        || raw.contains("resin")
        || raw.contains("substrate")
        || raw.contains("smelter")
    {
        return IndustryTier::R2;
    }
    if raw.contains("r1")
        || raw.contains("ore")
        || raw.contains("raw")
        || raw.contains("fuel")
        || raw.contains("radiation")
        || raw.contains("mine")
    {
        return IndustryTier::R1;
    }

    IndustryTier::Unknown
}

pub(super) fn infer_stage_from_text(parts: &[&str], tier: IndustryTier) -> IndustryStage {
    let raw = parts.join(" ").to_ascii_lowercase();

    if raw.contains("governance") {
        return IndustryStage::Governance;
    }
    if raw.contains("scale") || raw.contains("scale_out") {
        return IndustryStage::Scale;
    }
    if raw.contains("bootstrap") {
        return IndustryStage::Bootstrap;
    }

    match tier {
        IndustryTier::R1 | IndustryTier::R2 => IndustryStage::Bootstrap,
        IndustryTier::R3 | IndustryTier::R4 => IndustryStage::Scale,
        IndustryTier::R5 => IndustryStage::Governance,
        IndustryTier::Unknown => IndustryStage::Unknown,
    }
}

pub(super) fn root_cause_key(reason: &RejectReason) -> String {
    let raw = format!("{reason:?}");
    raw.split('{')
        .next()
        .unwrap_or(raw.as_str())
        .trim()
        .to_string()
}

pub(super) fn add_amount(map: &mut BTreeMap<ResourceKind, i64>, kind: ResourceKind, amount: i64) {
    let entry = map.entry(kind).or_insert(0);
    *entry = entry.saturating_add(amount.max(0));
}

pub(super) fn collect_resource_stocks(
    snapshot: &WorldSnapshot,
    out: &mut BTreeMap<ResourceKind, i64>,
) {
    for agent in snapshot.model.agents.values() {
        for (kind, amount) in &agent.resources.amounts {
            add_amount(out, *kind, *amount);
        }
    }

    for location in snapshot.model.locations.values() {
        for (kind, amount) in &location.resources.amounts {
            add_amount(out, *kind, *amount);
        }
    }

    for asset in snapshot.model.assets.values() {
        let AssetKind::Resource { kind } = asset.kind;
        add_amount(out, kind, asset.quantity);
    }
}

pub(super) fn owner_label(owner: &ResourceOwner) -> String {
    match owner {
        ResourceOwner::Agent { agent_id } => format!("agent::{agent_id}"),
        ResourceOwner::Location { location_id } => format!("location::{location_id}"),
    }
}

pub(super) fn route_weight(route: &IndustryRouteStats) -> i64 {
    route
        .material
        .abs()
        .saturating_add(route.electricity.abs())
        .saturating_add(route.data.abs())
        .saturating_add(route.power.abs())
}
