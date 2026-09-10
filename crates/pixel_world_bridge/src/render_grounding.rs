use super::*;

/// Passive pixel geometry; never contributes a hit region or world semantics.
#[derive(Component)]
pub(super) struct PixelWorldGroundingVisual {
    key: String,
}

pub(super) fn clear(commands: &mut Commands, query: &Query<(Entity, &PixelWorldGroundingVisual)>) {
    for (entity, _) in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub(super) fn reconcile(
    commands: &mut Commands,
    runtime: &BevyRuntimeState,
    query: &Query<(Entity, &PixelWorldGroundingVisual)>,
    width: f64,
    height: f64,
    animation_ms: f64,
) {
    let mut existing = query
        .iter()
        .map(|(entity, visual)| (visual.key.clone(), entity))
        .collect::<HashMap<_, _>>();
    let mut specs = Vec::new();
    if let Some(state) = runtime.render_state.as_ref() {
        for (index, agent) in state.agents.iter().enumerate() {
            let selected = state
                .selection
                .as_ref()
                .is_some_and(|s| s.kind == "agent" && s.id == agent.id);
            let style = agent_visual_style(agent, selected, animation_ms, index);
            let point = state
                .world_bounds
                .as_ref()
                .and_then(|bounds| {
                    agent.pos.as_ref().and_then(|pos| {
                        to_canvas_point(pos, bounds, width, height, &runtime.camera)
                    })
                })
                .unwrap_or_else(|| {
                    fallback_point_for_entity(&agent.id, width, height, &runtime.camera)
                });
            specs.push((
                format!("agent:{}", agent.id),
                point,
                style.size_px as f32,
                style.layer_z,
                true,
            ));
        }
        if let Some(bounds) = state.world_bounds.as_ref() {
            for location in &state.locations {
                if let Some(point) =
                    to_canvas_point(&location.pos, bounds, width, height, &runtime.camera)
                {
                    let selected = state
                        .selection
                        .as_ref()
                        .is_some_and(|s| s.kind == "location" && s.id == location.id);
                    let style = selected_location_visual_style(location, selected, animation_ms);
                    specs.push((
                        format!("location:{}", location.id),
                        point,
                        style.size_px as f32,
                        style.layer_z,
                        false,
                    ));
                }
            }
            for facility in &state.micro_depot_facilities {
                if !matches!(
                    facility.status.as_str(),
                    "active" | "suspended" | "depleted"
                ) {
                    continue;
                }
                if let Some(point) =
                    to_canvas_point(&facility.pos, bounds, width, height, &runtime.camera)
                {
                    specs.push((
                        format!("facility:{}", facility.id),
                        point,
                        9.0,
                        MICRO_DEPOT_LAYER_Z,
                        false,
                    ));
                }
            }
        }
    }
    for (id, (x, y), size, z, upright) in specs {
        let half_height = size * if upright { 0.5 } else { 0.36 };
        // Shared light direction: upper-left rim, lower-right contact shadow.
        for (part, dx, dy, w, h, color, layer) in [
            (
                "shadow",
                1.0,
                half_height + 1.0,
                size * 0.88,
                2.0,
                Color::srgba_u8(2, 6, 23, 180),
                z - 0.16,
            ),
            (
                "rim",
                -size * 0.12,
                -half_height + 1.0,
                size * if upright { 0.40 } else { 0.70 },
                1.0,
                Color::srgba_u8(226, 232, 240, 175),
                z + 0.006,
            ),
            (
                "profile",
                0.0,
                -half_height,
                size * if upright { 0.30 } else { 0.55 },
                if upright { 3.0 } else { 2.0 },
                Color::srgba_u8(148, 163, 184, 205),
                z - 0.01,
            ),
        ] {
            let key = format!("{id}:{part}");
            let sprite = sprite_for_rect(color, w, h);
            let transform = Transform::from_translation(to_bevy_translation(
                x + f64::from(dx),
                y + f64::from(dy),
                width,
                height,
                layer,
            ));
            if let Some(entity) = existing.remove(&key) {
                commands.entity(entity).insert((sprite, transform));
            } else {
                commands.spawn((sprite, transform, PixelWorldGroundingVisual { key }));
            }
        }
    }
    for entity in existing.into_values() {
        commands.entity(entity).despawn();
    }
}
