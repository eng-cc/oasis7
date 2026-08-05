use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum SelectedLocationCueEdge {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Component)]
pub(super) struct PixelWorldSelectedLocationCue {
    pub(super) location_id: String,
    pub(super) edge: SelectedLocationCueEdge,
}

pub(super) fn selected_location_cue_visuals(
    location: &Location,
    animation_ms: f64,
) -> [(SelectedLocationCueEdge, Vec2, Vec2); 4] {
    let location_style = selected_location_visual_style(location, true, animation_ms);
    let outer_size = location_style.size_px as f32
        + (SELECTED_LOCATION_CUE_PADDING_PX + SELECTED_LOCATION_CUE_THICKNESS_PX) * 2.0;
    let edge_offset = (outer_size - SELECTED_LOCATION_CUE_THICKNESS_PX) / 2.0;
    [
        (
            SelectedLocationCueEdge::Top,
            Vec2::new(0.0, edge_offset),
            Vec2::new(outer_size, SELECTED_LOCATION_CUE_THICKNESS_PX),
        ),
        (
            SelectedLocationCueEdge::Bottom,
            Vec2::new(0.0, -edge_offset),
            Vec2::new(outer_size, SELECTED_LOCATION_CUE_THICKNESS_PX),
        ),
        (
            SelectedLocationCueEdge::Left,
            Vec2::new(-edge_offset, 0.0),
            Vec2::new(SELECTED_LOCATION_CUE_THICKNESS_PX, outer_size),
        ),
        (
            SelectedLocationCueEdge::Right,
            Vec2::new(edge_offset, 0.0),
            Vec2::new(SELECTED_LOCATION_CUE_THICKNESS_PX, outer_size),
        ),
    ]
}

pub(super) fn reconcile_selected_location_cues(
    commands: &mut Commands,
    runtime: &BevyRuntimeState,
    existing_cues: &Query<(Entity, &PixelWorldSelectedLocationCue)>,
    width: f64,
    height: f64,
    animation_ms: f64,
) {
    let mut existing_by_key = HashMap::new();
    for (entity, cue) in existing_cues.iter() {
        existing_by_key.insert((cue.location_id.clone(), cue.edge), entity);
    }

    let mut active_keys = HashSet::new();
    let Some(render_state) = runtime.render_state.as_ref() else {
        for entity in existing_by_key.into_values() {
            commands.entity(entity).despawn();
        }
        return;
    };
    let Some(world_bounds) = render_state.world_bounds.as_ref() else {
        for entity in existing_by_key.into_values() {
            commands.entity(entity).despawn();
        }
        return;
    };

    let Some(selection) = render_state.selection.as_ref() else {
        for entity in existing_by_key.into_values() {
            commands.entity(entity).despawn();
        }
        return;
    };
    if selection.kind != "location" {
        for entity in existing_by_key.into_values() {
            commands.entity(entity).despawn();
        }
        return;
    }
    let Some(location) = render_state
        .locations
        .iter()
        .find(|location| location.id == selection.id)
    else {
        for entity in existing_by_key.into_values() {
            commands.entity(entity).despawn();
        }
        return;
    };
    let Some((canvas_x, canvas_y)) =
        to_canvas_point(&location.pos, world_bounds, width, height, &runtime.camera)
    else {
        for entity in existing_by_key.into_values() {
            commands.entity(entity).despawn();
        }
        return;
    };

    for (edge, offset, size) in selected_location_cue_visuals(location, animation_ms) {
        let key = (location.id.clone(), edge);
        active_keys.insert(key.clone());
        let sprite = sprite_for_rect(SELECTED_LOCATION_CUE_COLOR, size.x, size.y);
        let transform = Transform::from_translation(to_bevy_translation(
            canvas_x + f64::from(offset.x),
            canvas_y + f64::from(offset.y),
            width,
            height,
            SELECTED_LOCATION_CUE_LAYER_Z,
        ));
        if let Some(entity) = existing_by_key.get(&key).copied() {
            commands.entity(entity).insert((sprite, transform));
        } else {
            commands.spawn((
                sprite,
                transform,
                PixelWorldSelectedLocationCue {
                    location_id: location.id.clone(),
                    edge,
                },
            ));
        }
    }

    for (key, entity) in existing_by_key {
        if !active_keys.contains(&key) {
            commands.entity(entity).despawn();
        }
    }
}
