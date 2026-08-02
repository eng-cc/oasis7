use super::*;
use bevy::ecs::system::SystemParam;
use std::collections::{HashMap, HashSet};

const HOTSPOT_CUE_LAYER_Z_OFFSET: f32 = 0.005;
const HOTSPOT_CUE_COLOR: Color = Color::srgba_u8(226, 232, 240, 220);
const HOTSPOT_CUE_THICKNESS_PX: f32 = 1.5;

/// Non-interactive geometry that supplements the shared hotspot diamond without
/// assigning additional semantic color to its kind.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum HotspotCuePart {
    BlockerCrossAscending,
    BlockerCrossDescending,
    GoalCornerTop,
    GoalCornerRight,
}

#[derive(Component)]
pub(crate) struct PixelWorldHotspotCueVisual {
    pub(crate) id: String,
    pub(crate) part: HotspotCuePart,
}

#[derive(SystemParam)]
pub(crate) struct HotspotCueQueries<'w, 's> {
    cues: Query<'w, 's, (Entity, &'static PixelWorldHotspotCueVisual)>,
}

pub(crate) fn despawn_hotspot_cues(commands: &mut Commands, queries: &HotspotCueQueries) {
    for (entity, _) in queries.cues.iter() {
        commands.entity(entity).despawn();
    }
}

pub(crate) fn reconcile_hotspot_cues(
    commands: &mut Commands,
    runtime: &BevyRuntimeState,
    queries: &HotspotCueQueries,
    width: f64,
    height: f64,
    animation_ms: f64,
) {
    let existing = queries
        .cues
        .iter()
        .map(|(entity, cue)| ((cue.id.clone(), cue.part), entity))
        .collect::<HashMap<_, _>>();
    let Some(render_state) = runtime.render_state.as_ref() else {
        for entity in existing.into_values() {
            commands.entity(entity).despawn();
        }
        return;
    };
    let Some(world_bounds) = render_state.world_bounds.as_ref() else {
        for entity in existing.into_values() {
            commands.entity(entity).despawn();
        }
        return;
    };

    let mut active = HashSet::new();
    for (index, hotspot) in render_state.visual_hotspots.iter().enumerate() {
        let Some((canvas_x, canvas_y)) =
            to_canvas_point(&hotspot.pos, world_bounds, width, height, &runtime.camera)
        else {
            continue;
        };
        let pulse = 1.0 + (0.1 * ((animation_ms / 280.0) + index as f64).sin());
        let size = (hotspot.size_hint_px.unwrap_or(10.0) * pulse) as f32;
        let cue_specs: &[(HotspotCuePart, f32, f32, f32, f32, f32)] = match hotspot.kind.as_str() {
            "blocker" => &[
                (
                    HotspotCuePart::BlockerCrossAscending,
                    0.0,
                    0.0,
                    0.92,
                    1.0,
                    std::f32::consts::FRAC_PI_4,
                ),
                (
                    HotspotCuePart::BlockerCrossDescending,
                    0.0,
                    0.0,
                    0.92,
                    1.0,
                    -std::f32::consts::FRAC_PI_4,
                ),
            ],
            "goal" => &[
                (HotspotCuePart::GoalCornerTop, 0.22, 0.22, 0.44, 1.0, 0.0),
                (HotspotCuePart::GoalCornerRight, 0.22, 0.22, 1.0, 0.44, 0.0),
            ],
            _ => &[],
        };
        for (part, offset_x, offset_y, width_scale, height_scale, rotation) in cue_specs {
            let key = (hotspot.id.clone(), *part);
            active.insert(key.clone());
            let sprite = sprite_for_rect(
                HOTSPOT_CUE_COLOR,
                size * *width_scale,
                HOTSPOT_CUE_THICKNESS_PX * *height_scale,
            );
            let mut transform = Transform::from_translation(to_bevy_translation(
                canvas_x + f64::from(size * *offset_x),
                canvas_y + f64::from(size * *offset_y),
                width,
                height,
                1.5 + HOTSPOT_CUE_LAYER_Z_OFFSET,
            ));
            transform.rotation = Quat::from_rotation_z(*rotation);
            if let Some(entity) = existing.get(&key) {
                commands.entity(*entity).insert((sprite, transform));
            } else {
                commands.spawn((
                    sprite,
                    transform,
                    PixelWorldHotspotCueVisual {
                        id: hotspot.id.clone(),
                        part: *part,
                    },
                ));
            }
        }
    }
    for (key, entity) in existing {
        if !active.contains(&key) {
            commands.entity(entity).despawn();
        }
    }
}
