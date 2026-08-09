use std::collections::HashMap;

use super::*;

const LOCATION_CORNER_FRAME_COLOR: Color = Color::srgb_u8(167, 243, 208);
const LOCATION_CORNER_FRAME_ALPHA_SCALE: f64 = 0.85;
const LOCATION_CORNER_FRAME_LAYER_Z_OFFSET: f32 = 0.01;
const LOCATION_CORNER_FRAME_ARM_SCALE: f64 = 0.30;
const LOCATION_CORNER_FRAME_MIN_ARM_PX: f32 = 3.0;
const LOCATION_CORNER_FRAME_MAX_ARM_PX: f32 = 6.0;
const LOCATION_CORNER_FRAME_THICKNESS_PX: f32 = 2.0;
const LOCATION_CORNER_FRAME_PADDING_PX: f32 = 3.0;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum LocationCornerFramePart {
    UpperLeftHorizontal,
    UpperLeftVertical,
    LowerRightHorizontal,
    LowerRightVertical,
}

/// A display-only mint landmark frame for unselected, non-anchor locations.
#[derive(Component)]
pub(super) struct PixelWorldLocationCornerFrame {
    location_id: String,
    part: LocationCornerFramePart,
}

fn location_corner_frame_visuals(
    body_half_size: f32,
    arm_length: f32,
) -> [(LocationCornerFramePart, Vec2, Vec2); 4] {
    let outer_offset = body_half_size + LOCATION_CORNER_FRAME_PADDING_PX;
    let horizontal = Vec2::new(arm_length, LOCATION_CORNER_FRAME_THICKNESS_PX);
    let vertical = Vec2::new(LOCATION_CORNER_FRAME_THICKNESS_PX, arm_length);
    [
        (
            LocationCornerFramePart::UpperLeftHorizontal,
            Vec2::new(-outer_offset + (arm_length / 2.0), outer_offset),
            horizontal,
        ),
        (
            LocationCornerFramePart::UpperLeftVertical,
            Vec2::new(-outer_offset, outer_offset - (arm_length / 2.0)),
            vertical,
        ),
        (
            LocationCornerFramePart::LowerRightHorizontal,
            Vec2::new(outer_offset - (arm_length / 2.0), -outer_offset),
            horizontal,
        ),
        (
            LocationCornerFramePart::LowerRightVertical,
            Vec2::new(outer_offset, -outer_offset + (arm_length / 2.0)),
            vertical,
        ),
    ]
}

fn location_is_visible(location: &Location, world_bounds: &WorldBounds) -> bool {
    (0.0..=world_bounds.width_cm).contains(&location.pos.x_cm)
        && (0.0..=world_bounds.depth_cm).contains(&location.pos.y_cm)
}

pub(super) fn reconcile_location_corner_frames(
    commands: &mut Commands,
    runtime: &BevyRuntimeState,
    existing_frames: &Query<(Entity, &PixelWorldLocationCornerFrame)>,
    width: f64,
    height: f64,
    animation_ms: f64,
) {
    let mut existing_by_key = HashMap::new();
    for (entity, frame) in existing_frames.iter() {
        let key = (frame.location_id.clone(), frame.part);
        if let Some(duplicate) = existing_by_key.insert(key, entity) {
            commands.entity(duplicate).despawn();
        }
    }

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

    for location in &render_state.locations {
        let is_selected = render_state
            .selection
            .as_ref()
            .is_some_and(|selection| selection.kind == "location" && selection.id == location.id);
        if is_selected
            || location.marker_role.as_deref() == Some("logic_anchor")
            || !location_is_visible(location, world_bounds)
        {
            continue;
        }
        let Some((canvas_x, canvas_y)) =
            to_canvas_point(&location.pos, world_bounds, width, height, &runtime.camera)
        else {
            continue;
        };
        let style = location_visual_style(location, animation_ms);
        let arm_length = (style.size_px * LOCATION_CORNER_FRAME_ARM_SCALE).clamp(
            f64::from(LOCATION_CORNER_FRAME_MIN_ARM_PX),
            f64::from(LOCATION_CORNER_FRAME_MAX_ARM_PX),
        ) as f32;
        let color = LOCATION_CORNER_FRAME_COLOR
            .with_alpha((style.alpha * LOCATION_CORNER_FRAME_ALPHA_SCALE) as f32);

        for (part, offset, size) in
            location_corner_frame_visuals(style.size_px as f32 / 2.0, arm_length)
        {
            let key = (location.id.clone(), part);
            let sprite = sprite_for_rect(color, size.x, size.y);
            let transform = Transform::from_translation(to_bevy_translation(
                canvas_x + f64::from(offset.x),
                canvas_y - f64::from(offset.y),
                width,
                height,
                style.layer_z + LOCATION_CORNER_FRAME_LAYER_Z_OFFSET,
            ));
            if let Some(entity) = existing_by_key.remove(&key) {
                commands.entity(entity).insert((sprite, transform));
            } else {
                commands.spawn((
                    sprite,
                    transform,
                    PixelWorldLocationCornerFrame {
                        location_id: location.id.clone(),
                        part,
                    },
                ));
            }
        }
    }

    for entity in existing_by_key.into_values() {
        commands.entity(entity).despawn();
    }
}
