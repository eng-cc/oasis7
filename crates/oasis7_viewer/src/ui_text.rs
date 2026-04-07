use crate::industry_graph_view_model::{IndustryGraphViewModel, IndustrySemanticZoomLevel};
use oasis7::geometry::GeoPos;
use oasis7::simulator::{
    Action, AgentDecision, AgentDecisionTrace, Asset, AssetKind, ChunkCoord, ChunkState,
    FragmentElementKind, FragmentResourceBudget, ModuleVisualAnchor, PowerEvent, PowerPlant,
    ResourceKind, ResourceOwner, RunnerMetrics, WorldEvent, WorldEventKind, WorldSnapshot,
};

use super::viewer_3d_config::ViewerPhysicalRenderConfig;
#[cfg(not(target_arch = "wasm32"))]
use super::ConnectionStatus;
use super::{SelectionKind, ViewerSelection};

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn format_status(status: &ConnectionStatus) -> String {
    match status {
        ConnectionStatus::Connecting => "connecting".to_string(),
        ConnectionStatus::Connected => "connected".to_string(),
        ConnectionStatus::Error(message) => format!("error: {message}"),
    }
}

pub(super) fn world_summary(
    snapshot: Option<&WorldSnapshot>,
    metrics: Option<&RunnerMetrics>,
    physical: Option<&ViewerPhysicalRenderConfig>,
) -> String {
    let mut lines = Vec::new();
    if let Some(snapshot) = snapshot {
        let model = &snapshot.model;
        lines.push(format!("Time: {}", snapshot.time));
        lines.push(format!("Locations: {}", model.locations.len()));
        lines.push(format!("Agents: {}", model.agents.len()));
        lines.push(format!("Assets: {}", model.assets.len()));
        lines.push(format!(
            "Module Visuals: {}",
            model.module_visual_entities.len()
        ));
        lines.push(format!("Power Plants: {}", model.power_plants.len()));
        lines.push(format!("Chunks: {}", model.chunks.len()));
    } else {
        lines.push("World: (no snapshot)".to_string());
    }

    if let Some(metrics) = metrics {
        lines.push("".to_string());
        lines.push(format!("Ticks: {}", metrics.total_ticks));
        lines.push(format!("Actions: {}", metrics.total_actions));
        lines.push(format!("Decisions: {}", metrics.total_decisions));
    }

    if let Some(physical) = physical {
        lines.push("".to_string());
        lines.push(format!(
            "Render Physical: {}",
            if physical.enabled { "on" } else { "off" }
        ));
        if physical.enabled {
            lines.push(format!("Unit: 1u={:.2}m", physical.meters_per_unit));
            lines.push(format!(
                "Camera Clip(m): near={:.2} far={:.0}",
                physical.camera_near_m, physical.camera_far_m
            ));
            lines.push(format!(
                "Stellar Distance(AU): {:.2}",
                physical.stellar_distance_au
            ));
            lines.push(format!(
                "Irradiance(W/m²): {:.1}",
                physical.irradiance_w_m2()
            ));
            lines.push(format!(
                "Exposed Illuminance(lux): {:.0}",
                physical.exposed_illuminance_lux()
            ));
            lines.push(format!("Exposure(EV100): {:.2}", physical.exposure_ev100));
            lines.push(format!(
                "Radiation Ref Area(m²): {:.2}",
                physical.reference_radiation_area_m2
            ));
        }
    }

    lines.join("\n")
}

pub(super) fn events_summary(events: &[WorldEvent], focus_tick: Option<u64>) -> String {
    const WINDOW_SIZE: usize = 20;

    if events.is_empty() {
        return "Events:
(no events)"
            .to_string();
    }

    if focus_tick.is_none() {
        let mut lines = Vec::new();
        lines.push("Events:".to_string());
        for event in events.iter().rev().take(WINDOW_SIZE).rev() {
            lines.push(format!("#{} t{} {:?}", event.id, event.time, event.kind));
        }
        return lines.join("\n");
    }

    let requested_focus = focus_tick.unwrap_or(0);
    let mut nearest_idx = 0_usize;
    let mut nearest_dist = u64::MAX;

    for (idx, event) in events.iter().enumerate() {
        let dist = event.time.abs_diff(requested_focus);
        if dist < nearest_dist {
            nearest_dist = dist;
            nearest_idx = idx;
        }
    }

    let total = events.len();
    let half = WINDOW_SIZE / 2;
    let max_start = total.saturating_sub(WINDOW_SIZE);
    let window_start = nearest_idx.saturating_sub(half).min(max_start);
    let window_end = (window_start + WINDOW_SIZE).min(total);

    let focused = &events[nearest_idx];
    let mut lines = Vec::new();
    lines.push("Events (focused):".to_string());
    lines.push(format!(
        "Focus: requested t{} -> nearest t{} (#{}), Δt={}",
        requested_focus, focused.time, focused.id, nearest_dist
    ));
    for (idx, event) in events
        .iter()
        .enumerate()
        .skip(window_start)
        .take(window_end - window_start)
    {
        let prefix = if idx == nearest_idx { ">>" } else { "  " };
        lines.push(format!(
            "{} #{} t{} {:?}",
            prefix, event.id, event.time, event.kind
        ));
    }
    lines.join("\n")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(super) enum ProviderDebugFilter {
    #[default]
    All,
    LoopbackProviderOnly,
    ErrorsOnly,
}

impl ProviderDebugFilter {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::LoopbackProviderOnly => "loopback_provider_only",
            Self::ErrorsOnly => "errors_only",
        }
    }
}

pub(super) fn provider_debug_summary(
    traces: &[AgentDecisionTrace],
    filter: ProviderDebugFilter,
) -> String {
    let filtered: Vec<&AgentDecisionTrace> = traces
        .iter()
        .rev()
        .filter(|trace| provider_trace_matches(filter, trace))
        .take(4)
        .collect();

    let mut lines = Vec::new();
    lines.push(format!("Provider Debug: filter={}", filter.label()));
    lines.push(format!("Trace Count: {}", filtered.len()));

    let Some(latest) = filtered.first().copied() else {
        lines.push("(no matching decision trace yet)".to_string());
        return lines.join("\n");
    };

    lines.push(format!(
        "Latest: t{} provider={} decision={} latency_ms={}",
        latest.time,
        provider_label_for_trace(latest),
        decision_summary(&latest.decision),
        latest
            .llm_diagnostics
            .as_ref()
            .and_then(|value| value.latency_ms)
            .map(|value| value.to_string())
            .unwrap_or_else(|| "n/a".to_string())
    ));

    if let Some(err) = latest
        .llm_error
        .as_deref()
        .or_else(|| latest.parse_error.as_deref())
    {
        lines.push(format!("Last Error: {}", truncate_text(err, 160)));
    }

    let recent_latency = filtered
        .iter()
        .filter_map(|trace| {
            trace
                .llm_diagnostics
                .as_ref()
                .and_then(|value| value.latency_ms)
                .map(|latency| format!("t{}={}ms", trace.time, latency))
        })
        .collect::<Vec<_>>();
    lines.push(format!(
        "Recent Latency: {}",
        if recent_latency.is_empty() {
            "n/a".to_string()
        } else {
            recent_latency.join(", ")
        }
    ));

    lines.push("Recent Decisions:".to_string());
    for trace in filtered {
        lines.push(format!(
            "- t{} {} | provider={} | trace={}",
            trace.time,
            decision_summary(&trace.decision),
            provider_label_for_trace(trace),
            recent_trace_summary(trace)
        ));
    }

    lines.join("\n")
}

pub(super) fn agent_activity_summary(
    snapshot: Option<&WorldSnapshot>,
    events: &[WorldEvent],
) -> String {
    let Some(snapshot) = snapshot else {
        return "Agents Activity:\n(no snapshot)".to_string();
    };

    if snapshot.model.agents.is_empty() {
        return "Agents Activity:\n(none)".to_string();
    }

    let mut lines = Vec::new();
    lines.push("Agents Activity:".to_string());

    let mut agent_ids: Vec<_> = snapshot.model.agents.keys().cloned().collect();
    agent_ids.sort();

    for agent_id in agent_ids {
        if let Some(agent) = snapshot.model.agents.get(&agent_id) {
            let electricity = agent.resources.get(ResourceKind::Electricity);
            let activity =
                latest_agent_activity(&agent_id, events).unwrap_or_else(|| "idle".to_string());
            lines.push(format!(
                "{agent_id} @ {} | E={} | {}",
                agent.location_id, electricity, activity
            ));
        }
    }

    lines.join("\n")
}

#[path = "ui_text_details.rs"]
mod ui_text_details;

pub(super) fn selection_details_summary(
    selection: &ViewerSelection,
    snapshot: Option<&WorldSnapshot>,
    events: &[WorldEvent],
    decision_traces: &[AgentDecisionTrace],
    reference_radiation_area_m2: f32,
) -> String {
    ui_text_details::selection_details_summary(
        selection,
        snapshot,
        events,
        decision_traces,
        reference_radiation_area_m2,
    )
}

fn parse_chunk_coord(chunk_id: &str) -> Option<ChunkCoord> {
    let mut parts = chunk_id.split(',');
    let x = parts.next()?.trim().parse::<i32>().ok()?;
    let y = parts.next()?.trim().parse::<i32>().ok()?;
    let z = parts.next()?.trim().parse::<i32>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(ChunkCoord { x, y, z })
}

fn chunk_state_name(state: ChunkState) -> &'static str {
    match state {
        ChunkState::Unexplored => "unexplored",
        ChunkState::Generated => "generated",
        ChunkState::Exhausted => "exhausted",
    }
}

fn fragment_budget_totals_g(budget: &FragmentResourceBudget) -> Option<(i64, i64)> {
    let total = budget
        .total_by_element_g
        .values()
        .copied()
        .filter(|amount| *amount > 0)
        .fold(0_i64, |acc, amount| acc.saturating_add(amount));
    if total <= 0 {
        return None;
    }

    let remaining = budget
        .remaining_by_element_g
        .values()
        .copied()
        .filter(|amount| *amount > 0)
        .fold(0_i64, |acc, amount| acc.saturating_add(amount))
        .clamp(0, total);

    Some((remaining, total))
}

fn format_element_budget(
    budgets: &std::collections::BTreeMap<FragmentElementKind, i64>,
    limit: usize,
) -> Vec<String> {
    if budgets.is_empty() {
        return vec!["- (empty)".to_string()];
    }
    let mut entries: Vec<_> = budgets.iter().collect();
    entries.sort_by(|a, b| b.1.cmp(a.1));
    entries
        .into_iter()
        .take(limit)
        .map(|(kind, amount)| format!("- {:?}: {}g", kind, amount))
        .collect()
}

fn chunk_recent_events(coord: ChunkCoord, events: &[WorldEvent], limit: usize) -> Vec<String> {
    events
        .iter()
        .rev()
        .filter_map(|event| {
            event_activity_for_chunk(event, coord)
                .map(|activity| format!("- t{} #{} {}", event.time, event.id, activity))
        })
        .take(limit)
        .collect()
}

fn facility_details_lines(
    facility_id: &str,
    plant: &PowerPlant,
    snapshot: &WorldSnapshot,
    events: &[WorldEvent],
) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("Details: power_plant {facility_id}"));
    lines.push(format!("Location: {}", plant.location_id));
    lines.push(format!("Owner: {}", owner_label(&plant.owner)));
    lines.push(format!("Status: {:?}", plant.status));
    lines.push(format!(
        "Output: current={} capacity/tick={} effective={}",
        plant.current_output,
        plant.capacity_per_tick,
        plant.effective_output()
    ));
    lines.push(format!(
        "Costs: fuel_per_pu={} maintenance={} efficiency={:.2} degradation={:.2}",
        plant.fuel_cost_per_pu, plant.maintenance_cost, plant.efficiency, plant.degradation
    ));

    if let Some(location) = snapshot.model.locations.get(&plant.location_id) {
        lines.push(format!(
            "Location Pos(cm): {}",
            format_geo_pos(location.pos)
        ));
    }

    lines.push("".to_string());
    lines.push("Recent Events:".to_string());
    let mut related = power_plant_recent_events(facility_id, events, 6);
    if related.is_empty() {
        related.push("(none)".to_string());
    }
    lines.extend(related);

    lines
}

fn module_visual_details_summary(
    module_entity: &oasis7::simulator::ModuleVisualEntity,
    snapshot: &WorldSnapshot,
    events: &[WorldEvent],
) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "Details: module_visual {}",
        module_entity.entity_id
    ));
    lines.push(format!("Module: {}", module_entity.module_id));
    lines.push(format!("Kind: {}", module_entity.kind));
    lines.push(format!(
        "Label: {}",
        module_entity
            .label
            .as_deref()
            .filter(|label| !label.trim().is_empty())
            .unwrap_or("(none)")
    ));
    lines.push(format!(
        "Anchor: {}",
        module_visual_anchor_label(&module_entity.anchor)
    ));
    if let Some(pos) = module_visual_anchor_pos(snapshot, &module_entity.anchor) {
        lines.push(format!("Anchor Pos(cm): {}", format_geo_pos(pos)));
    }

    lines.push("".to_string());
    lines.push("Recent Events:".to_string());
    let mut related = module_visual_recent_events(module_entity.entity_id.as_str(), events, 6);
    if related.is_empty() {
        related.push("(none)".to_string());
    }
    lines.extend(related);

    lines.join("\n")
}

fn module_visual_anchor_label(anchor: &ModuleVisualAnchor) -> String {
    match anchor {
        ModuleVisualAnchor::Agent { agent_id } => format!("agent::{agent_id}"),
        ModuleVisualAnchor::Location { location_id } => format!("location::{location_id}"),
        ModuleVisualAnchor::Absolute { pos } => format!("absolute({})", format_geo_pos(*pos)),
    }
}

fn module_visual_anchor_pos(
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

fn asset_kind_name(asset: &Asset) -> String {
    match &asset.kind {
        AssetKind::Resource { kind } => format!("resource::{kind:?}"),
    }
}

fn owner_anchor_pos(snapshot: &WorldSnapshot, owner: &ResourceOwner) -> Option<GeoPos> {
    match owner {
        ResourceOwner::Agent { agent_id } => {
            snapshot.model.agents.get(agent_id).map(|agent| agent.pos)
        }
        ResourceOwner::Location { location_id } => snapshot
            .model
            .locations
            .get(location_id)
            .map(|location| location.pos),
    }
}

fn owner_label(owner: &ResourceOwner) -> String {
    match owner {
        ResourceOwner::Agent { agent_id } => format!("agent::{agent_id}"),
        ResourceOwner::Location { location_id } => format!("location::{location_id}"),
    }
}

fn format_geo_pos(pos: GeoPos) -> String {
    format!("x={:.0}, y={:.0}, z={:.0}", pos.x_cm, pos.y_cm, pos.z_cm)
}

fn thermal_ratio(heat: i64, capacity: i64) -> f64 {
    let heat = heat.max(0) as f64;
    let capacity = capacity.max(1) as f64;
    heat / capacity
}

fn thermal_ratio_color(thermal_ratio: f64) -> &'static str {
    if thermal_ratio <= 0.6 {
        "heat_low"
    } else if thermal_ratio <= 1.0 {
        "heat_mid"
    } else {
        "heat_high"
    }
}

fn radiation_visual_metrics(
    radiation_emission_per_tick: i64,
    power_unit_j: i64,
    time_step_s: i64,
    reference_radiation_area_m2: f32,
) -> (f64, f64, f64) {
    let emission = radiation_emission_per_tick.max(0) as f64;
    let joule_per_unit = power_unit_j.max(1) as f64;
    let seconds_per_tick = time_step_s.max(1) as f64;
    let area_m2 = if reference_radiation_area_m2.is_finite() && reference_radiation_area_m2 > 0.0 {
        reference_radiation_area_m2 as f64
    } else {
        1.0
    };
    let radiation_power_w = emission * joule_per_unit / seconds_per_tick;
    let radiation_flux_w_m2 = radiation_power_w / area_m2;
    (radiation_power_w, radiation_flux_w_m2, area_m2)
}

fn format_resource_stock(amounts: &std::collections::BTreeMap<ResourceKind, i64>) -> Vec<String> {
    if amounts.is_empty() {
        return vec!["- (empty)".to_string()];
    }
    amounts
        .iter()
        .map(|(kind, amount)| format!("- {:?}: {}", kind, amount))
        .collect()
}

fn agent_recent_events(agent_id: &str, events: &[WorldEvent], limit: usize) -> Vec<String> {
    events
        .iter()
        .rev()
        .filter_map(|event| {
            event_activity_for_agent(event, agent_id)
                .map(|activity| format!("- t{} #{} {}", event.time, event.id, activity))
        })
        .take(limit)
        .collect()
}

fn location_recent_events(location_id: &str, events: &[WorldEvent], limit: usize) -> Vec<String> {
    events
        .iter()
        .rev()
        .filter_map(|event| {
            event_activity_for_location(event, location_id)
                .map(|activity| format!("- t{} #{} {}", event.time, event.id, activity))
        })
        .take(limit)
        .collect()
}

fn owner_recent_events(owner: &ResourceOwner, events: &[WorldEvent], limit: usize) -> Vec<String> {
    events
        .iter()
        .rev()
        .filter_map(|event| {
            event_activity_for_owner(event, owner)
                .map(|activity| format!("- t{} #{} {}", event.time, event.id, activity))
        })
        .take(limit)
        .collect()
}

fn power_plant_recent_events(plant_id: &str, events: &[WorldEvent], limit: usize) -> Vec<String> {
    events
        .iter()
        .rev()
        .filter_map(|event| {
            event_activity_for_power_plant(event, plant_id)
                .map(|activity| format!("- t{} #{} {}", event.time, event.id, activity))
        })
        .take(limit)
        .collect()
}

fn module_visual_recent_events(
    entity_id: &str,
    events: &[WorldEvent],
    limit: usize,
) -> Vec<String> {
    events
        .iter()
        .rev()
        .filter_map(|event| {
            event_activity_for_module_visual(event, entity_id)
                .map(|activity| format!("- t{} #{} {}", event.time, event.id, activity))
        })
        .take(limit)
        .collect()
}

fn provider_trace_matches(filter: ProviderDebugFilter, trace: &AgentDecisionTrace) -> bool {
    match filter {
        ProviderDebugFilter::All => true,
        ProviderDebugFilter::LoopbackProviderOnly => is_loopback_provider_trace(trace),
        ProviderDebugFilter::ErrorsOnly => trace.llm_error.is_some() || trace.parse_error.is_some(),
    }
}

fn is_loopback_provider_trace(trace: &AgentDecisionTrace) -> bool {
    provider_label_for_trace(trace)
        .to_ascii_lowercase()
        .contains("provider")
}

fn provider_label_for_trace(trace: &AgentDecisionTrace) -> String {
    trace
        .llm_diagnostics
        .as_ref()
        .and_then(|value| value.model.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_string()
}

fn decision_summary(decision: &AgentDecision) -> String {
    match decision {
        AgentDecision::Wait => "wait".to_string(),
        AgentDecision::WaitTicks(ticks) => format!("wait_ticks {ticks}"),
        AgentDecision::Act(action) => match action {
            Action::MoveAgent { to, .. } => format!("move_agent -> {to}"),
            Action::SpeakToNearby {
                message,
                target_agent_id,
                ..
            } => format!(
                "speak_to_nearby target={} message={}",
                target_agent_id.as_deref().unwrap_or("nearby"),
                truncate_text(message, 48)
            ),
            Action::InspectTarget {
                target_id,
                target_kind,
                ..
            } => format!("inspect_target {target_kind} {target_id}"),
            Action::SimpleInteract {
                target_id,
                target_kind,
                interaction,
                ..
            } => format!("simple_interact {target_kind} {target_id} {interaction}"),
            other => format!("{:?}", other),
        },
    }
}

fn recent_trace_summary(trace: &AgentDecisionTrace) -> String {
    if let Some(err) = trace
        .llm_error
        .as_deref()
        .or_else(|| trace.parse_error.as_deref())
    {
        return format!("error:{}", truncate_text(err, 80));
    }
    if let Some(output) = trace.llm_output.as_deref() {
        return truncate_text(output, 96);
    }
    if let Some(input) = trace.llm_input.as_deref() {
        return truncate_text(input, 96);
    }
    if !trace.llm_step_trace.is_empty() {
        return format!("steps={}", trace.llm_step_trace.len());
    }
    "(no trace payload)".to_string()
}

fn agent_recent_traces(agent_id: &str, traces: &[AgentDecisionTrace], limit: usize) -> Vec<String> {
    traces
        .iter()
        .rev()
        .filter(|trace| trace.agent_id == agent_id)
        .flat_map(|trace| {
            let mut lines = Vec::new();
            lines.push(format!("- t{} decision {:?}", trace.time, trace.decision));
            if let Some(input) = trace.llm_input.as_ref() {
                lines.push(format!("  input: {}", truncate_text(input, 240)));
            }
            if let Some(output) = trace.llm_output.as_ref() {
                lines.push(format!("  output: {}", truncate_text(output, 240)));
            }
            if let Some(err) = trace.llm_error.as_ref() {
                lines.push(format!("  llm_error: {}", truncate_text(err, 160)));
            }
            if let Some(parse_error) = trace.parse_error.as_ref() {
                lines.push(format!(
                    "  parse_error: {}",
                    truncate_text(parse_error, 160)
                ));
            }
            if let Some(diagnostics) = trace.llm_diagnostics.as_ref() {
                lines.push(format!(
                    "  model: {}",
                    diagnostics.model.as_deref().unwrap_or("-")
                ));
                lines.push(format!(
                    "  latency_ms: {}",
                    diagnostics
                        .latency_ms
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "-".to_string())
                ));
                lines.push(format!(
                    "  tokens: prompt={} completion={} total={}",
                    diagnostics
                        .prompt_tokens
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    diagnostics
                        .completion_tokens
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    diagnostics
                        .total_tokens
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "-".to_string())
                ));
                lines.push(format!("  retries: {}", diagnostics.retry_count));
            }
            lines
        })
        .take(limit * 5)
        .collect()
}

fn truncate_text(text: &str, max_len: usize) -> String {
    let normalized = text.replace('\n', "\\n");
    if normalized.chars().count() <= max_len {
        return normalized;
    }
    normalized.chars().take(max_len).collect::<String>() + "..."
}

fn latest_agent_activity(agent_id: &str, events: &[WorldEvent]) -> Option<String> {
    for event in events.iter().rev() {
        if let Some(activity) = event_activity_for_agent(event, agent_id) {
            return Some(format!("t{} {}", event.time, activity));
        }
    }
    None
}

fn event_activity_for_agent(event: &WorldEvent, agent_id: &str) -> Option<String> {
    match &event.kind {
        WorldEventKind::AgentRegistered {
            agent_id: id,
            location_id,
            ..
        } if id == agent_id => Some(format!("register at {location_id}")),
        WorldEventKind::AgentMoved {
            agent_id: id,
            to,
            electricity_cost,
            ..
        } if id == agent_id => Some(format!("move -> {to} (cost {electricity_cost})")),
        WorldEventKind::RadiationHarvested {
            agent_id: id,
            amount,
            location_id,
            ..
        } if id == agent_id => Some(format!("harvest +{amount} at {location_id}")),
        WorldEventKind::ResourceTransferred {
            from,
            to,
            kind,
            amount,
        } => {
            let from_agent = owner_matches_agent(from, agent_id);
            let to_agent = owner_matches_agent(to, agent_id);
            match (from_agent, to_agent) {
                (true, true) => Some(format!("transfer {:?} {} (self)", kind, amount)),
                (true, false) => Some(format!("transfer out {:?} {}", kind, amount)),
                (false, true) => Some(format!("transfer in {:?} {}", kind, amount)),
                _ => None,
            }
        }
        WorldEventKind::CompoundRefined {
            owner,
            compound_mass_g,
            hardware_output,
            ..
        } if owner_matches_agent(owner, agent_id) => Some(format!(
            "refine {}g -> hw {}",
            compound_mass_g, hardware_output
        )),
        WorldEventKind::Power(power_event) => match power_event {
            PowerEvent::PowerConsumed {
                agent_id: id,
                amount,
                ..
            } if id == agent_id => Some(format!("power -{amount}")),
            PowerEvent::PowerStateChanged {
                agent_id: id, to, ..
            } if id == agent_id => Some(format!("power state -> {:?}", to)),
            PowerEvent::PowerCharged {
                agent_id: id,
                amount,
                ..
            } if id == agent_id => Some(format!("power +{amount}")),
            PowerEvent::PowerTransferred {
                from,
                to,
                amount,
                loss,
                ..
            } => {
                let from_agent = owner_matches_agent(from, agent_id);
                let to_agent = owner_matches_agent(to, agent_id);
                match (from_agent, to_agent) {
                    (true, true) => Some(format!("trade power {} (loss {})", amount, loss)),
                    (true, false) => Some(format!("sell power {} (loss {})", amount, loss)),
                    (false, true) => Some(format!("buy power {} (loss {})", amount, loss)),
                    _ => None,
                }
            }
            _ => None,
        },
        _ => None,
    }
}

fn event_activity_for_location(event: &WorldEvent, location_id: &str) -> Option<String> {
    match &event.kind {
        WorldEventKind::LocationRegistered {
            location_id: id,
            name,
            ..
        } if id == location_id => Some(format!("register {name}")),
        WorldEventKind::AgentRegistered {
            agent_id,
            location_id: id,
            ..
        } if id == location_id => Some(format!("agent {agent_id} spawn")),
        WorldEventKind::AgentMoved {
            agent_id, from, to, ..
        } if from == location_id => Some(format!("agent {agent_id} moved out -> {to}")),
        WorldEventKind::AgentMoved {
            agent_id, from, to, ..
        } if to == location_id => Some(format!("agent {agent_id} moved in <- {from}")),
        WorldEventKind::RadiationHarvested {
            agent_id,
            location_id: id,
            amount,
            ..
        } if id == location_id => Some(format!("agent {agent_id} harvest +{amount}")),
        WorldEventKind::ResourceTransferred {
            from,
            to,
            kind,
            amount,
        } => {
            let from_location = owner_matches_location(from, location_id);
            let to_location = owner_matches_location(to, location_id);
            match (from_location, to_location) {
                (true, true) => Some(format!("transfer {:?} {} (self)", kind, amount)),
                (true, false) => Some(format!("transfer out {:?} {}", kind, amount)),
                (false, true) => Some(format!("transfer in {:?} {}", kind, amount)),
                _ => None,
            }
        }
        WorldEventKind::Power(PowerEvent::PowerGenerated {
            location_id: id,
            amount,
            plant_id,
        }) if id == location_id => Some(format!("plant {plant_id} generated {amount}")),
        _ => None,
    }
}

fn event_activity_for_chunk(event: &WorldEvent, coord: ChunkCoord) -> Option<String> {
    match &event.kind {
        WorldEventKind::ChunkGenerated {
            coord: event_coord,
            fragment_count,
            block_count,
            cause,
            ..
        } if *event_coord == coord => Some(format!(
            "generated fragments={} blocks={} cause={:?}",
            fragment_count, block_count, cause
        )),
        _ => None,
    }
}

fn event_activity_for_module_visual(event: &WorldEvent, entity_id: &str) -> Option<String> {
    match &event.kind {
        WorldEventKind::ModuleVisualEntityUpserted { entity } if entity.entity_id == entity_id => {
            Some(format!(
                "upsert module={} kind={} anchor={}",
                entity.module_id,
                entity.kind,
                module_visual_anchor_label(&entity.anchor)
            ))
        }
        WorldEventKind::ModuleVisualEntityRemoved { entity_id: id } if id == entity_id => {
            Some("removed".to_string())
        }
        _ => None,
    }
}

#[path = "ui_text_activity.rs"]
mod ui_text_activity;
#[path = "ui_text_economy.rs"]
mod ui_text_economy;
#[path = "ui_text_industrial.rs"]
mod ui_text_industrial;
#[path = "ui_text_ops_navigation.rs"]
mod ui_text_ops_navigation;

use ui_text_activity::{
    event_activity_for_owner, event_activity_for_power_plant, owner_matches_agent,
    owner_matches_location,
};

#[allow(dead_code)]
pub(super) fn industrial_ops_summary(
    snapshot: Option<&WorldSnapshot>,
    events: &[WorldEvent],
) -> Option<String> {
    let graph = build_industry_graph_view_model(snapshot, events);
    industrial_ops_summary_with_zoom(&graph, snapshot, events, IndustrySemanticZoomLevel::Node)
}

pub(super) fn industrial_ops_summary_with_zoom(
    graph: &IndustryGraphViewModel,
    snapshot: Option<&WorldSnapshot>,
    events: &[WorldEvent],
    zoom: IndustrySemanticZoomLevel,
) -> Option<String> {
    ui_text_industrial::industrial_ops_summary_with_zoom(graph, snapshot, events, zoom)
}

#[allow(dead_code)]
pub(super) fn economy_dashboard_summary(
    snapshot: Option<&WorldSnapshot>,
    events: &[WorldEvent],
) -> Option<String> {
    let graph = build_industry_graph_view_model(snapshot, events);
    economy_dashboard_summary_with_zoom(&graph, IndustrySemanticZoomLevel::Node)
}

pub(super) fn economy_dashboard_summary_with_zoom(
    graph: &IndustryGraphViewModel,
    zoom: IndustrySemanticZoomLevel,
) -> Option<String> {
    ui_text_economy::economy_dashboard_summary_with_zoom(graph, zoom)
}

#[allow(dead_code)]
pub(super) fn ops_navigation_alert_summary(
    snapshot: Option<&WorldSnapshot>,
    events: &[WorldEvent],
) -> Option<String> {
    let graph = build_industry_graph_view_model(snapshot, events);
    ops_navigation_alert_summary_with_zoom(&graph, IndustrySemanticZoomLevel::Node)
}

pub(super) fn ops_navigation_alert_summary_with_zoom(
    graph: &IndustryGraphViewModel,
    zoom: IndustrySemanticZoomLevel,
) -> Option<String> {
    ui_text_ops_navigation::ops_navigation_alert_summary_with_zoom(graph, zoom)
}

pub(super) fn build_industry_graph_view_model(
    snapshot: Option<&WorldSnapshot>,
    events: &[WorldEvent],
) -> IndustryGraphViewModel {
    IndustryGraphViewModel::build(snapshot, events)
}

#[cfg(test)]
#[path = "ui_text_tests.rs"]
mod tests;
