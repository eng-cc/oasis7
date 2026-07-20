use super::*;

pub(crate) const HOTSPOT_CORE_LAYER_Z_OFFSET: f32 = 0.01;
pub(crate) const HOTSPOT_CORE_SIZE_SCALE: f64 = 0.30;
pub(crate) const HOTSPOT_CORE_MIN_SIZE_PX: f64 = 2.0;
pub(crate) const HOTSPOT_CORE_MAX_SIZE_PX: f64 = 5.0;
pub(crate) const HOTSPOT_CORE_COLOR: Color = Color::srgba_u8(226, 232, 240, 230);

/// A non-interactive neutral inset that makes every hotspot read as a layered
/// signal without assigning an additional semantic color to its kind.
#[derive(Component)]
pub(crate) struct PixelWorldHotspotCoreVisual {
    pub(crate) id: String,
}

pub(crate) fn reconcile_hotspot_cores(
    commands: &mut Commands,
    runtime: &BevyRuntimeState,
    existing_cores: &Query<(Entity, &PixelWorldHotspotCoreVisual)>,
    width: f64,
    height: f64,
    animation_ms: f64,
) {
    let mut existing_by_id = HashMap::new();
    for (entity, core) in existing_cores.iter() {
        existing_by_id.insert(core.id.clone(), entity);
    }

    let Some(render_state) = runtime.render_state.as_ref() else {
        for entity in existing_by_id.into_values() {
            commands.entity(entity).despawn();
        }
        return;
    };
    let Some(world_bounds) = render_state.world_bounds.as_ref() else {
        for entity in existing_by_id.into_values() {
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
    }

    for (id, entity) in existing_by_id {
        if !active_ids.contains(&id) {
            commands.entity(entity).despawn();
        }
    }
}
