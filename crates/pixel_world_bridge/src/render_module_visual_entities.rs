use std::collections::HashSet;

use super::*;

pub(crate) const MODULE_VISUAL_ENTITY_COLOR: Color = Color::srgb_u8(129, 140, 248);
pub(crate) const MODULE_VISUAL_ENTITY_SIZE_PX: f32 = 6.0;

#[derive(Component)]
pub(crate) struct PixelWorldModuleVisualEntity {
    pub(crate) id: String,
}

const CO_ANCHOR_OFFSETS: [Vec2; 9] = [
    Vec2::ZERO,
    Vec2::new(-3.0, -3.0),
    Vec2::new(3.0, -3.0),
    Vec2::new(-3.0, 3.0),
    Vec2::new(3.0, 3.0),
    Vec2::new(-6.0, 0.0),
    Vec2::new(6.0, 0.0),
    Vec2::new(0.0, -6.0),
    Vec2::new(0.0, 6.0),
];

pub(super) fn reconcile_module_visual_entities(
    commands: &mut Commands,
    runtime: &mut BevyRuntimeState,
    width: f64,
    height: f64,
) {
    let Some(render_state) = runtime.render_state.as_ref() else {
        for (_, entity) in runtime.module_visual_entities.drain() {
            commands.entity(entity).despawn();
        }
        return;
    };
    let Some(world_bounds) = render_state.world_bounds.as_ref() else {
        for (_, entity) in runtime.module_visual_entities.drain() {
            commands.entity(entity).despawn();
        }
        return;
    };

    let mut entities = render_state
        .module_visual_entities
        .iter()
        .collect::<Vec<_>>();
    entities.sort_by(|left, right| left.id.cmp(&right.id));
    let mut active_ids = HashSet::new();
    for (index, entity) in entities.iter().enumerate() {
        let Some((canvas_x, canvas_y)) =
            to_canvas_point(&entity.pos, world_bounds, width, height, &runtime.camera)
        else {
            continue;
        };
        let co_anchor_index = entities[..index]
            .iter()
            .filter(|other| {
                other.pos.x_cm == entity.pos.x_cm
                    && other.pos.y_cm == entity.pos.y_cm
                    && other.pos.z_cm == entity.pos.z_cm
            })
            .count();
        let offset = CO_ANCHOR_OFFSETS[co_anchor_index % CO_ANCHOR_OFFSETS.len()];
        active_ids.insert(entity.id.clone());
        let mut transform = Transform::from_translation(to_bevy_translation(
            canvas_x + f64::from(offset.x),
            canvas_y + f64::from(offset.y),
            width,
            height,
            MODULE_VISUAL_ENTITY_LAYER_Z,
        ));
        transform.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_4);
        let sprite = sprite_for_square(MODULE_VISUAL_ENTITY_COLOR, MODULE_VISUAL_ENTITY_SIZE_PX);
        if let Some(existing) = runtime.module_visual_entities.get(&entity.id).copied() {
            commands.entity(existing).insert((sprite, transform));
        } else {
            let spawned = commands
                .spawn((
                    sprite,
                    transform,
                    PixelWorldModuleVisualEntity {
                        id: entity.id.clone(),
                    },
                ))
                .id();
            runtime
                .module_visual_entities
                .insert(entity.id.clone(), spawned);
        }
    }
    despawn_stale_entities(commands, &mut runtime.module_visual_entities, &active_ids);
}
