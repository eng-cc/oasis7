use std::collections::{HashMap, HashSet};

use super::*;
use bevy::ecs::system::SystemParam;

#[derive(Component)]
pub(crate) struct PixelWorldMicroDepotVisual {
    pub(crate) id: String,
}

#[derive(Component)]
pub(crate) struct PixelWorldMicroDepotDetailVisual {
    id: String,
}

/// One segment of the passive world-scale service-range outline. It deliberately
/// has no hit region: the existing depot glyph remains the only facility visual
/// identity and this range is observation-only.
#[derive(Component)]
pub(crate) struct PixelWorldMicroDepotServiceRadiusVisual {
    pub(crate) id: String,
}

#[derive(SystemParam)]
pub(super) struct MicroDepotFacilityOverlayQueries<'w, 's> {
    details: Query<'w, 's, (Entity, &'static PixelWorldMicroDepotDetailVisual)>,
    service_radius_outlines:
        Query<'w, 's, (Entity, &'static PixelWorldMicroDepotServiceRadiusVisual)>,
}

impl MicroDepotFacilityOverlayQueries<'_, '_> {
    pub(super) fn despawn(&self, commands: &mut Commands) {
        for (entity, _) in self.details.iter() {
            commands.entity(entity).despawn();
        }
        for (entity, _) in self.service_radius_outlines.iter() {
            commands.entity(entity).despawn();
        }
    }
}

const SERVICE_RADIUS_LAYER_Z: f32 = MICRO_DEPOT_LAYER_Z - 0.03;
const SERVICE_RADIUS_ALPHA: f32 = 0.22;
const SERVICE_RADIUS_THICKNESS_PX: f32 = 1.25;

fn service_radius_outline_points(
    canvas_x: f64,
    canvas_y: f64,
    radius_x: f64,
    radius_y: f64,
) -> [(f64, f64); 8] {
    std::array::from_fn(|index| {
        let angle = (index as f64) * std::f64::consts::FRAC_PI_4;
        (
            canvas_x + (angle.cos() * radius_x),
            canvas_y + (angle.sin() * radius_y),
        )
    })
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
    overlays: &MicroDepotFacilityOverlayQueries,
    width: f64,
    height: f64,
) {
    let Some(render_state) = runtime.render_state.as_ref() else {
        for (_, entity) in runtime.micro_depot_entities.drain() {
            commands.entity(entity).despawn();
        }
        overlays.despawn(commands);
        return;
    };
    let Some(world_bounds) = render_state.world_bounds.as_ref() else {
        for (_, entity) in runtime.micro_depot_entities.drain() {
            commands.entity(entity).despawn();
        }
        overlays.despawn(commands);
        return;
    };
    let details_by_id = overlays
        .details
        .iter()
        .map(|(entity, visual)| (visual.id.clone(), entity))
        .collect::<HashMap<_, _>>();
    let mut active_ids = HashSet::new();
    let mut active_detail_ids = HashSet::new();
    let mut active_service_radius_ids = HashSet::new();
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

        if facility.service_radius_cm > 0.0 {
            let world_width_px = (width - 40.0).max(1.0) * runtime.camera.zoom.max(0.5);
            let world_depth_px = (height - 40.0).max(1.0) * runtime.camera.zoom.max(0.5);
            let radius_x =
                facility.service_radius_cm / world_bounds.width_cm.max(1.0) * world_width_px;
            let radius_y =
                facility.service_radius_cm / world_bounds.depth_cm.max(1.0) * world_depth_px;
            let points = service_radius_outline_points(canvas_x, canvas_y, radius_x, radius_y);
            let outline_color = Color::srgba(0.35, 0.88, 0.76, SERVICE_RADIUS_ALPHA);
            active_service_radius_ids.insert(facility.id.clone());
            // The outline has multiple segments, so it is intentionally not
            // stored in the one-entity-per-id glyph map. Rebuild its tiny
            // passive treatment when the camera or published radius changes.
            for (entity, visual) in overlays.service_radius_outlines.iter() {
                if visual.id == facility.id {
                    commands.entity(entity).despawn();
                }
            }
            for index in 0..points.len() {
                let (from_x, from_y) = points[index];
                let (to_x, to_y) = points[(index + 1) % points.len()];
                let length = ((to_x - from_x).powi(2) + (to_y - from_y).powi(2))
                    .sqrt()
                    .max(SERVICE_RADIUS_THICKNESS_PX as f64);
                commands.spawn((
                    sprite_for_rect(outline_color, length as f32, SERVICE_RADIUS_THICKNESS_PX),
                    transform_for_line(
                        from_x,
                        from_y,
                        to_x,
                        to_y,
                        width,
                        height,
                        SERVICE_RADIUS_LAYER_Z,
                    ),
                    PixelWorldMicroDepotServiceRadiusVisual {
                        id: facility.id.clone(),
                    },
                ));
            }
        }
    }
    despawn_stale_entities(commands, &mut runtime.micro_depot_entities, &active_ids);
    for (id, entity) in details_by_id {
        if !active_detail_ids.contains(&id) {
            commands.entity(entity).despawn();
        }
    }
    for (entity, visual) in overlays.service_radius_outlines.iter() {
        if !active_service_radius_ids.contains(&visual.id) {
            commands.entity(entity).despawn();
        }
    }
}
