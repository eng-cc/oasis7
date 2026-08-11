use super::*;
use std::collections::HashSet;

const LOCATION_RESOURCE_CUE_SIZE_PX: f32 = 6.0;
const LOCATION_RESOURCE_CUE_LAYER_Z: f32 = 1.08;
const LOCATION_RESOURCE_CUE_COLOR: Color = Color::srgba_u8(226, 232, 240, 220);

#[derive(Component)]
pub(crate) struct PixelWorldLocationResourceCue {
    pub(crate) location_id: String,
}

fn has_published_resource_report(summary: &str) -> bool {
    let compact = summary.split_whitespace().collect::<String>();
    !compact.is_empty() && compact != "-" && compact != "amounts:{}"
}

pub(crate) fn reconcile(
    commands: &mut Commands,
    runtime: &BevyRuntimeState,
    existing: &Query<(Entity, &PixelWorldLocationResourceCue)>,
    width: f64,
    height: f64,
) {
    let mut active_ids = HashSet::new();
    let Some(render_state) = runtime.render_state.as_ref() else {
        despawn(commands, existing);
        return;
    };
    let Some(world_bounds) = render_state.world_bounds.as_ref() else {
        despawn(commands, existing);
        return;
    };

    for location in &render_state.locations {
        if !has_published_resource_report(&location.resource_summary) {
            continue;
        }
        let Some((canvas_x, canvas_y)) =
            to_canvas_point(&location.pos, world_bounds, width, height, &runtime.camera)
        else {
            continue;
        };
        active_ids.insert(location.id.as_str());
        let transform = Transform {
            translation: to_bevy_translation(
                canvas_x,
                canvas_y,
                width,
                height,
                LOCATION_RESOURCE_CUE_LAYER_Z,
            ),
            rotation: Quat::from_rotation_z(std::f32::consts::FRAC_PI_4),
            ..Default::default()
        };
        let sprite = sprite_for_square(LOCATION_RESOURCE_CUE_COLOR, LOCATION_RESOURCE_CUE_SIZE_PX);
        if let Some((entity, _)) = existing
            .iter()
            .find(|(_, cue)| cue.location_id == location.id)
        {
            commands.entity(entity).insert((sprite, transform));
        } else {
            commands.spawn((
                sprite,
                transform,
                PixelWorldLocationResourceCue {
                    location_id: location.id.clone(),
                },
            ));
        }
    }

    for (entity, cue) in existing.iter() {
        if !active_ids.contains(cue.location_id.as_str()) {
            commands.entity(entity).despawn();
        }
    }
}

pub(crate) fn despawn(
    commands: &mut Commands,
    existing: &Query<(Entity, &PixelWorldLocationResourceCue)>,
) {
    for (entity, _) in existing.iter() {
        commands.entity(entity).despawn();
    }
}
