use super::{
    agent_recent_events, agent_recent_traces, asset_kind_name, chunk_recent_events,
    chunk_state_name, facility_details_lines, format_element_budget, format_geo_pos,
    format_resource_stock, fragment_budget_totals_g, module_visual_details_summary,
    owner_anchor_pos, owner_label, owner_recent_events, parse_chunk_coord,
    radiation_visual_metrics, thermal_ratio, thermal_ratio_color, SelectionKind, ViewerSelection,
};
use crate::ui_text_claims::extend_agent_details_with_claim_lines;
use oasis7::simulator::{chunk_bounds, AgentDecisionTrace, WorldEvent, WorldSnapshot};

pub(super) fn selection_details_summary(
    selection: &ViewerSelection,
    snapshot: Option<&WorldSnapshot>,
    events: &[WorldEvent],
    decision_traces: &[AgentDecisionTrace],
    reference_radiation_area_m2: f32,
) -> String {
    let Some(selected) = selection.current.as_ref() else {
        return "Details:\n(click object to inspect)".to_string();
    };

    match selected.kind {
        SelectionKind::Agent => {
            agent_details_summary(selected.id.as_str(), snapshot, events, decision_traces)
        }
        SelectionKind::Location => location_details_summary(
            selected.id.as_str(),
            selected.name.as_deref(),
            snapshot,
            events,
            reference_radiation_area_m2,
        ),
        SelectionKind::Fragment => {
            fragment_details_summary(selected.id.as_str(), selected.name.as_deref(), snapshot)
        }
        SelectionKind::Asset => asset_details_summary(selected.id.as_str(), snapshot, events),
        SelectionKind::PowerPlant => {
            power_plant_details_summary(selected.id.as_str(), snapshot, events)
        }
        SelectionKind::Chunk => chunk_details_summary(
            selected.id.as_str(),
            selected.name.as_deref(),
            snapshot,
            events,
        ),
    }
}

fn fragment_details_summary(
    fragment_id: &str,
    owner_location_id: Option<&str>,
    snapshot: Option<&WorldSnapshot>,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!("Details: fragment {fragment_id}"));
    let location_id = owner_location_id.unwrap_or("(unknown)");
    lines.push(format!("Location: {location_id}"));

    if let Some(snapshot) = snapshot {
        if let Some(location) = snapshot.model.locations.get(location_id) {
            lines.push(format!("Location Name: {}", location.name));
        }
    }

    lines.join("\n")
}

fn agent_details_summary(
    agent_id: &str,
    snapshot: Option<&WorldSnapshot>,
    events: &[WorldEvent],
    decision_traces: &[AgentDecisionTrace],
) -> String {
    let Some(snapshot) = snapshot else {
        return format!("Details: agent {agent_id}\n(no snapshot)");
    };

    let Some(agent) = snapshot.model.agents.get(agent_id) else {
        return format!("Details: agent {agent_id}\n(not found in snapshot)");
    };

    let mut lines = Vec::new();
    lines.push(format!("Details: agent {agent_id}"));
    lines.push(format!("Location: {}", agent.location_id));
    lines.push(format!("Pos(cm): {}", format_geo_pos(agent.pos)));
    lines.push(format!(
        "Body: kind={} height={}cm",
        agent.body.kind, agent.body.height_cm
    ));
    let body_height_cm = agent.body.height_cm.max(1);
    lines.push(format!(
        "Body Size: data_height={:.2}m ({}cm)",
        body_height_cm as f64 / 100.0,
        body_height_cm
    ));
    if let Some(location) = snapshot.model.locations.get(&agent.location_id) {
        let location_radius_cm = location.profile.radius_cm.max(1);
        let scale_ratio = body_height_cm as f64 / location_radius_cm as f64;
        lines.push(format!(
            "Location Radius: {}cm ({:.2}m)",
            location_radius_cm,
            location_radius_cm as f64 / 100.0
        ));
        lines.push(format!(
            "Scale Ratio: height/location_radius={scale_ratio:.3}"
        ));
    }
    lines.push(format!(
        "Power: {}/{} ({:?})",
        agent.power.level, agent.power.capacity, agent.power.state
    ));
    lines.push(format!("Thermal: heat={}", agent.thermal.heat));
    let thermal_ratio = thermal_ratio(agent.thermal.heat, snapshot.config.physics.thermal_capacity);
    lines.push(format!(
        "Thermal Visual: ratio={:.2} color={}",
        thermal_ratio,
        thermal_ratio_color(thermal_ratio)
    ));

    lines.push("Resources:".to_string());
    lines.extend(format_resource_stock(&agent.resources.amounts));

    extend_agent_details_with_claim_lines(agent_id, snapshot, &mut lines);

    lines.push("".to_string());
    lines.push("Recent Events:".to_string());
    let mut recent_events = agent_recent_events(agent_id, events, 6);
    if recent_events.is_empty() {
        recent_events.push("(none)".to_string());
    }
    lines.extend(recent_events);

    lines.push("".to_string());
    lines.push("Recent LLM I/O:".to_string());
    let mut recent_traces = agent_recent_traces(agent_id, decision_traces, 3);
    if recent_traces.is_empty() {
        recent_traces.push("(no llm trace yet)".to_string());
    }
    lines.extend(recent_traces);

    lines.join("\n")
}

fn location_details_summary(
    location_id: &str,
    selected_name: Option<&str>,
    snapshot: Option<&WorldSnapshot>,
    events: &[WorldEvent],
    reference_radiation_area_m2: f32,
) -> String {
    let Some(snapshot) = snapshot else {
        return format!("Details: location {location_id}\n(no snapshot)");
    };

    let Some(location) = snapshot.model.locations.get(location_id) else {
        return format!("Details: location {location_id}\n(not found in snapshot)");
    };

    let plant_count = snapshot
        .model
        .power_plants
        .values()
        .filter(|plant| plant.location_id == location_id)
        .count();
    let asset_count = snapshot
        .model
        .assets
        .values()
        .filter(|asset| super::owner_matches_location(&asset.owner, location_id))
        .count();

    let mut lines = Vec::new();
    lines.push(format!("Details: location {location_id}"));
    lines.push(format!(
        "Name: {}",
        selected_name.unwrap_or(location.name.as_str())
    ));
    lines.push(format!("Pos(cm): {}", format_geo_pos(location.pos)));
    lines.push(format!(
        "Profile: material={:?} radius={}cm radiation/tick={}",
        location.profile.material,
        location.profile.radius_cm,
        location.profile.radiation_emission_per_tick
    ));
    let (radiation_power_w, radiation_flux_w_m2, area_m2) = radiation_visual_metrics(
        location.profile.radiation_emission_per_tick,
        snapshot.config.physics.power_unit_j,
        snapshot.config.physics.time_step_s,
        reference_radiation_area_m2,
    );
    lines.push(format!(
        "Radiation Visual: power={radiation_power_w:.2}W flux={radiation_flux_w_m2:.2}W/m2 area={area_m2:.2}m2"
    ));
    lines.push(format!(
        "Facilities: plants={} assets_owned={}",
        plant_count, asset_count
    ));

    lines.push("Resources:".to_string());
    lines.extend(format_resource_stock(&location.resources.amounts));

    if let Some(fragment) = location.fragment_profile.as_ref() {
        lines.push("".to_string());
        lines.push(format!(
            "Fragment: blocks={} mass={}g density={}kg/m3",
            fragment.blocks.blocks.len(),
            fragment.total_mass_g,
            fragment.bulk_density_kg_per_m3
        ));
    }

    if let Some(budget) = location.fragment_budget.as_ref() {
        if let Some((remaining_total, total_total)) = fragment_budget_totals_g(budget) {
            let mined_pct =
                (1.0 - remaining_total as f64 / total_total as f64).clamp(0.0, 1.0) * 100.0;
            lines.push(format!(
                "Fragment Depletion: mined={mined_pct:.1}% remaining={remaining_total}g/{total_total}g"
            ));
        }

        lines.push("Fragment Budget (remaining top):".to_string());
        let mut remaining: Vec<_> = budget.remaining_by_element_g.iter().collect();
        remaining.sort_by(|a, b| b.1.cmp(a.1));
        for (kind, amount) in remaining.into_iter().take(6) {
            lines.push(format!("- {:?}: {}g", kind, amount));
        }
    }

    lines.push("".to_string());
    lines.push("Recent Events:".to_string());
    let mut related = super::location_recent_events(location_id, events, 6);
    if related.is_empty() {
        related.push("(none)".to_string());
    }
    lines.extend(related);

    lines.join("\n")
}

fn asset_details_summary(
    asset_id: &str,
    snapshot: Option<&WorldSnapshot>,
    events: &[WorldEvent],
) -> String {
    let Some(snapshot) = snapshot else {
        return format!("Details: asset {asset_id}\n(no snapshot)");
    };

    if let Some(asset) = snapshot.model.assets.get(asset_id) {
        let mut lines = Vec::new();
        lines.push(format!("Details: asset {asset_id}"));
        lines.push(format!("Kind: {}", asset_kind_name(asset)));
        lines.push(format!("Quantity: {}", asset.quantity));
        lines.push(format!("Owner: {}", owner_label(&asset.owner)));
        if let Some(anchor) = owner_anchor_pos(snapshot, &asset.owner) {
            lines.push(format!("Owner Pos(cm): {}", format_geo_pos(anchor)));
        }

        lines.push("".to_string());
        lines.push("Recent Owner Events:".to_string());
        let mut related = owner_recent_events(&asset.owner, events, 6);
        if related.is_empty() {
            related.push("(none)".to_string());
        }
        lines.extend(related);

        return lines.join("\n");
    }

    if let Some(module_entity) = snapshot.model.module_visual_entities.get(asset_id) {
        return module_visual_details_summary(module_entity, snapshot, events);
    }

    format!("Details: asset {asset_id}\n(not found in snapshot)")
}

fn power_plant_details_summary(
    facility_id: &str,
    snapshot: Option<&WorldSnapshot>,
    events: &[WorldEvent],
) -> String {
    let Some(snapshot) = snapshot else {
        return format!("Details: power_plant {facility_id}\n(no snapshot)");
    };

    let Some(plant) = snapshot.model.power_plants.get(facility_id) else {
        return format!("Details: power_plant {facility_id}\n(not found in snapshot)");
    };

    facility_details_lines(facility_id, plant, snapshot, events).join("\n")
}

fn chunk_details_summary(
    chunk_id: &str,
    selected_state: Option<&str>,
    snapshot: Option<&WorldSnapshot>,
    events: &[WorldEvent],
) -> String {
    let Some(snapshot) = snapshot else {
        return format!("Details: chunk {chunk_id}\n(no snapshot)");
    };

    let Some(coord) = parse_chunk_coord(chunk_id) else {
        return format!("Details: chunk {chunk_id}\n(invalid chunk id)");
    };

    let Some(state) = snapshot.model.chunks.get(&coord) else {
        return format!("Details: chunk {chunk_id}\n(not found in snapshot)");
    };

    let mut lines = Vec::new();
    lines.push(format!("Details: chunk {chunk_id}"));
    lines.push(format!(
        "State: {}",
        selected_state.unwrap_or(chunk_state_name(*state))
    ));

    if let Some(bounds) = chunk_bounds(coord, &snapshot.config.space) {
        lines.push(format!(
            "Bounds(cm): x[{:.0},{:.0}] y[{:.0},{:.0}] z[{:.0},{:.0}]",
            bounds.min.x_cm,
            bounds.max.x_cm,
            bounds.min.y_cm,
            bounds.max.y_cm,
            bounds.min.z_cm,
            bounds.max.z_cm
        ));
    }

    let reservation_count = snapshot
        .model
        .chunk_boundary_reservations
        .get(&coord)
        .map(|items| items.len())
        .unwrap_or(0);
    lines.push(format!("Boundary Reservations: {}", reservation_count));

    lines.push("".to_string());
    lines.push("Budget (remaining top):".to_string());
    if let Some(budget) = snapshot.model.chunk_resource_budgets.get(&coord) {
        lines.extend(format_element_budget(&budget.remaining_by_element_g, 6));

        lines.push("Budget (total top):".to_string());
        lines.extend(format_element_budget(&budget.total_by_element_g, 6));
    } else {
        lines.push("- (none)".to_string());
    }

    lines.push("".to_string());
    lines.push("Recent Events:".to_string());
    let mut related = chunk_recent_events(coord, events, 6);
    if related.is_empty() {
        related.push("(none)".to_string());
    }
    lines.extend(related);

    lines.join("\n")
}
