use std::collections::{HashMap, HashSet};

use super::*;

#[derive(Component)]
pub(crate) struct PixelWorldMicroDepotVisual {
    pub(crate) id: String,
}

#[derive(Component)]
pub(crate) struct PixelWorldMicroDepotDetailVisual {
    id: String,
}

fn facility_glyph(status: &str) -> Option<(Color, f32, f32, Vec2)> {
    match status {
        "active" => Some((Color::srgb_u8(52, 211, 153), 8.0, 0.0, Vec2::new(4.0, 1.5))),
        "suspended" => Some((
            Color::srgb_u8(251, 191, 36),
            8.0,
            std::f32::consts::FRAC_PI_4,
            Vec2::splat(3.0),
        )),
        "depleted" => Some((
            Color::srgb_u8(248, 113, 113),
            9.0,
            std::f32::consts::FRAC_PI_4,
            Vec2::new(9.0, 1.5),
        )),
        _ => None,
    }
}

pub(super) fn reconcile_micro_depot_facilities(
    commands: &mut Commands,
    runtime: &mut BevyRuntimeState,
    existing_details: &Query<(Entity, &PixelWorldMicroDepotDetailVisual)>,
    width: f64,
    height: f64,
) {
    let Some(render_state) = runtime.render_state.as_ref() else {
        for (_, entity) in runtime.micro_depot_entities.drain() {
            commands.entity(entity).despawn();
        }
        for (entity, _) in existing_details.iter() {
            commands.entity(entity).despawn();
        }
        return;
    };
    let Some(world_bounds) = render_state.world_bounds.as_ref() else {
        for (_, entity) in runtime.micro_depot_entities.drain() {
            commands.entity(entity).despawn();
        }
        for (entity, _) in existing_details.iter() {
            commands.entity(entity).despawn();
        }
        return;
    };
    let details_by_id = existing_details
        .iter()
        .map(|(entity, visual)| (visual.id.clone(), entity))
        .collect::<HashMap<_, _>>();
    let mut active_ids = HashSet::new();
    let mut active_detail_ids = HashSet::new();
    for facility in &render_state.micro_depot_facilities {
        let Some((color, size, rotation, detail_size)) = facility_glyph(&facility.status) else {
            continue;
        };
        let Some((canvas_x, canvas_y)) =
            to_canvas_point(&facility.pos, world_bounds, width, height, &runtime.camera)
        else {
            continue;
        };
        active_ids.insert(facility.id.clone());
        active_detail_ids.insert(facility.id.clone());
        let mut transform = Transform::from_translation(to_bevy_translation(
            canvas_x,
            canvas_y,
            width,
            height,
            MICRO_DEPOT_LAYER_Z,
        ));
        transform.rotation = Quat::from_rotation_z(rotation);
        let sprite = sprite_for_square(color, size);
        if let Some(entity) = runtime.micro_depot_entities.get(&facility.id).copied() {
            commands.entity(entity).insert((sprite, transform));
        } else {
            let entity = commands
                .spawn((
                    sprite,
                    transform,
                    PixelWorldMicroDepotVisual {
                        id: facility.id.clone(),
                    },
                ))
                .id();
            runtime
                .micro_depot_entities
                .insert(facility.id.clone(), entity);
        }

        let mut detail_transform = transform;
        detail_transform.translation.z += 0.01;
        if facility.status == "depleted" {
            detail_transform.rotation = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_4);
        } else {
            detail_transform.rotation = Quat::IDENTITY;
        }
        let detail = Sprite::from_color(Color::srgb_u8(15, 23, 42), detail_size);
        if let Some(entity) = details_by_id.get(&facility.id) {
            commands.entity(*entity).insert((detail, detail_transform));
        } else {
            commands.spawn((
                detail,
                detail_transform,
                PixelWorldMicroDepotDetailVisual {
                    id: facility.id.clone(),
                },
            ));
        }
    }
    despawn_stale_entities(commands, &mut runtime.micro_depot_entities, &active_ids);
    for (id, entity) in details_by_id {
        if !active_detail_ids.contains(&id) {
            commands.entity(entity).despawn();
        }
    }
}
