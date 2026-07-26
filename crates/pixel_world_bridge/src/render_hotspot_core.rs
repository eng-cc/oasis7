use super::*;
use bevy::ecs::system::SystemParam;

pub(crate) const HOTSPOT_CORE_LAYER_Z_OFFSET: f32 = 0.01;
pub(crate) const HOTSPOT_CORE_SIZE_SCALE: f64 = 0.30;
pub(crate) const HOTSPOT_CORE_MIN_SIZE_PX: f64 = 2.0;
pub(crate) const HOTSPOT_CORE_MAX_SIZE_PX: f64 = 5.0;
pub(crate) const HOTSPOT_CORE_COLOR: Color = Color::srgba_u8(226, 232, 240, 230);
pub(crate) const HOTSPOT_CORE_HIGHLIGHT_COLOR: Color = Color::srgba_u8(248, 250, 252, 230);
pub(crate) const HOTSPOT_CORE_SHADOW_COLOR: Color = Color::srgba_u8(148, 163, 184, 230);

/// A non-interactive neutral inset that makes every hotspot read as a layered
/// signal without assigning an additional semantic color to its kind.
#[derive(Component)]
pub(crate) struct PixelWorldHotspotCoreVisual {
    pub(crate) id: String,
}

#[derive(Component)]
pub(crate) struct PixelWorldHotspotCoreHighlightVisual {
    id: String,
}

#[derive(Component)]
pub(crate) struct PixelWorldHotspotCoreShadowVisual {
    id: String,
}

#[derive(SystemParam)]
pub(crate) struct HotspotCoreQueries<'w, 's> {
    cores: Query<'w, 's, (Entity, &'static PixelWorldHotspotCoreVisual)>,
    highlights: Query<'w, 's, (Entity, &'static PixelWorldHotspotCoreHighlightVisual)>,
    shadows: Query<'w, 's, (Entity, &'static PixelWorldHotspotCoreShadowVisual)>,
}

pub(crate) fn despawn_hotspot_core_treatments(
    commands: &mut Commands,
    queries: &HotspotCoreQueries,
) {
    for (entity, _) in queries.cores.iter() {
        commands.entity(entity).despawn();
    }
    for (entity, _) in queries.highlights.iter() {
        commands.entity(entity).despawn();
    }
    for (entity, _) in queries.shadows.iter() {
        commands.entity(entity).despawn();
    }
}

pub(crate) fn reconcile_hotspot_cores(
    commands: &mut Commands,
    runtime: &BevyRuntimeState,
    queries: &HotspotCoreQueries,
    width: f64,
    height: f64,
    animation_ms: f64,
) {
    let mut existing_by_id = HashMap::new();
    for (entity, core) in queries.cores.iter() {
        existing_by_id.insert(core.id.clone(), entity);
    }
    let existing_highlights_by_id = queries
        .highlights
        .iter()
        .map(|(entity, highlight)| (highlight.id.clone(), entity))
        .collect::<HashMap<_, _>>();
    let existing_shadows_by_id = queries
        .shadows
        .iter()
        .map(|(entity, shadow)| (shadow.id.clone(), entity))
        .collect::<HashMap<_, _>>();

    let Some(render_state) = runtime.render_state.as_ref() else {
        for entity in existing_by_id.into_values() {
            commands.entity(entity).despawn();
        }
        for entity in existing_highlights_by_id.into_values() {
            commands.entity(entity).despawn();
        }
        for entity in existing_shadows_by_id.into_values() {
            commands.entity(entity).despawn();
        }
        return;
    };
    let Some(world_bounds) = render_state.world_bounds.as_ref() else {
        for entity in existing_by_id.into_values() {
            commands.entity(entity).despawn();
        }
        for entity in existing_highlights_by_id.into_values() {
            commands.entity(entity).despawn();
        }
        for entity in existing_shadows_by_id.into_values() {
            commands.entity(entity).despawn();
        }
        return;
    };

    let mut active_ids = HashSet::new();
    for (index, hotspot) in render_state.visual_hotspots.iter().enumerate() {
        let Some((canvas_x, canvas_y)) =
            to_canvas_point(&hotspot.pos, world_bounds, width, height, &runtime.camera)
        else {
            continue;
        };
        active_ids.insert(hotspot.id.clone());
        let pulse = 1.0 + (0.1 * ((animation_ms / 280.0) + index as f64).sin());
        let base_size = hotspot.size_hint_px.unwrap_or(10.0) * pulse;
        let core_size = (base_size * HOTSPOT_CORE_SIZE_SCALE)
            .clamp(HOTSPOT_CORE_MIN_SIZE_PX, HOTSPOT_CORE_MAX_SIZE_PX)
            as f32;
        let sprite = sprite_for_square(HOTSPOT_CORE_COLOR, core_size);
        let transform = Transform::from_translation(to_bevy_translation(
            canvas_x,
            canvas_y,
            width,
            height,
            1.5 + HOTSPOT_CORE_LAYER_Z_OFFSET,
        ));

        if let Some(entity) = existing_by_id.get(&hotspot.id).copied() {
            commands.entity(entity).insert((sprite, transform));
        } else {
            commands.spawn((
                sprite,
                transform,
                PixelWorldHotspotCoreVisual {
                    id: hotspot.id.clone(),
                },
            ));
        }

        let decoration_offset = ((core_size - 1.0) / 2.0).min(1.0);
        let highlight_transform = Transform::from_translation(
            transform.translation + Vec3::new(-decoration_offset, decoration_offset, 0.001),
        );
        let shadow_transform = Transform::from_translation(
            transform.translation + Vec3::new(decoration_offset, -decoration_offset, 0.002),
        );
        let highlight_sprite = sprite_for_square(HOTSPOT_CORE_HIGHLIGHT_COLOR, 1.0);
        let shadow_sprite = sprite_for_square(HOTSPOT_CORE_SHADOW_COLOR, 1.0);
        if let Some(entity) = existing_highlights_by_id.get(&hotspot.id) {
            commands
                .entity(*entity)
                .insert((highlight_sprite, highlight_transform));
        } else {
            commands.spawn((
                highlight_sprite,
                highlight_transform,
                PixelWorldHotspotCoreHighlightVisual {
                    id: hotspot.id.clone(),
                },
            ));
        }
        if let Some(entity) = existing_shadows_by_id.get(&hotspot.id) {
            commands
                .entity(*entity)
                .insert((shadow_sprite, shadow_transform));
        } else {
            commands.spawn((
                shadow_sprite,
                shadow_transform,
                PixelWorldHotspotCoreShadowVisual {
                    id: hotspot.id.clone(),
                },
            ));
        }
    }

    for (id, entity) in existing_by_id {
        if !active_ids.contains(&id) {
            commands.entity(entity).despawn();
        }
    }
    for (id, entity) in existing_highlights_by_id {
        if !active_ids.contains(&id) {
            commands.entity(entity).despawn();
        }
    }
    for (id, entity) in existing_shadows_by_id {
        if !active_ids.contains(&id) {
            commands.entity(entity).despawn();
        }
    }
}
