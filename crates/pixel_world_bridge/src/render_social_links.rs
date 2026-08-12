use std::collections::{HashMap, HashSet};

use super::*;

const SOCIAL_LINK_LAYER_Z: f32 = 0.42;
const SOCIAL_LINK_THICKNESS_PX: f32 = 1.5;
const SOCIAL_LINK_COLOR: Color = Color::srgba_u8(196, 181, 253, 118);
const SOCIAL_ENDPOINT_COLOR: Color = Color::srgba_u8(221, 214, 254, 180);
const SOCIAL_MIN_LINK_LENGTH_PX: f64 = 8.0;

#[derive(Component)]
pub(super) struct PixelWorldSocialLinkVisual {
    pub(super) link_id: String,
    pub(super) part: SocialLinkPart,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum SocialLinkPart {
    Line,
    FromGlyph,
    ToGlyph,
}

pub(super) fn despawn_social_links(
    commands: &mut Commands,
    existing: &Query<(Entity, &PixelWorldSocialLinkVisual)>,
) {
    for (entity, _) in existing.iter() {
        commands.entity(entity).despawn();
    }
}

pub(super) fn reconcile_social_links(
    commands: &mut Commands,
    runtime: &BevyRuntimeState,
    existing: &Query<(Entity, &PixelWorldSocialLinkVisual)>,
    width: f64,
    height: f64,
) {
    let mut by_key = HashMap::new();
    for (entity, visual) in existing.iter() {
        let key = (visual.link_id.clone(), visual.part);
        if let Some(duplicate) = by_key.insert(key, entity) {
            commands.entity(duplicate).despawn();
        }
    }
    let Some(render_state) = runtime.render_state.as_ref() else {
        despawn_existing(commands, by_key);
        return;
    };
    let Some(bounds) = render_state.world_bounds.as_ref() else {
        despawn_existing(commands, by_key);
        return;
    };
    let mut active = HashSet::new();
    for link in &render_state.social_links {
        let Some((from_x, from_y)) =
            to_canvas_point(&link.from, bounds, width, height, &runtime.camera)
        else {
            continue;
        };
        let Some((to_x, to_y)) = to_canvas_point(&link.to, bounds, width, height, &runtime.camera)
        else {
            continue;
        };
        let dx = to_x - from_x;
        let dy = to_y - from_y;
        let length = (dx * dx + dy * dy).sqrt();
        if !length.is_finite() || length < SOCIAL_MIN_LINK_LENGTH_PX {
            continue;
        }
        upsert_line(
            commands,
            &mut by_key,
            &mut active,
            &link.id,
            SocialLinkPart::Line,
            from_x,
            from_y,
            to_x,
            to_y,
            SOCIAL_LINK_COLOR,
            width,
            height,
        );
        upsert_glyph(
            commands,
            &mut by_key,
            &mut active,
            &link.id,
            SocialLinkPart::FromGlyph,
            from_x,
            from_y,
            SOCIAL_ENDPOINT_COLOR,
            width,
            height,
        );
        upsert_glyph(
            commands,
            &mut by_key,
            &mut active,
            &link.id,
            SocialLinkPart::ToGlyph,
            to_x,
            to_y,
            SOCIAL_ENDPOINT_COLOR,
            width,
            height,
        );
    }
    for (key, entity) in by_key {
        if !active.contains(&key) {
            commands.entity(entity).despawn();
        }
    }
}

fn despawn_existing(commands: &mut Commands, entities: HashMap<(String, SocialLinkPart), Entity>) {
    for entity in entities.into_values() {
        commands.entity(entity).despawn();
    }
}

#[allow(clippy::too_many_arguments)]
fn upsert_line(
    commands: &mut Commands,
    by_key: &mut HashMap<(String, SocialLinkPart), Entity>,
    active: &mut HashSet<(String, SocialLinkPart)>,
    link_id: &str,
    part: SocialLinkPart,
    from_x: f64,
    from_y: f64,
    to_x: f64,
    to_y: f64,
    color: Color,
    width: f64,
    height: f64,
) {
    let key = (link_id.to_string(), part);
    active.insert(key.clone());
    let length = ((to_x - from_x).powi(2) + (to_y - from_y).powi(2)).sqrt();
    let sprite = sprite_for_rect(color, length as f32, SOCIAL_LINK_THICKNESS_PX);
    let transform = transform_for_line(
        from_x,
        from_y,
        to_x,
        to_y,
        width,
        height,
        SOCIAL_LINK_LAYER_Z,
    );
    if let Some(entity) = by_key.remove(&key) {
        commands.entity(entity).insert((sprite, transform));
        by_key.insert(key, entity);
    } else {
        let entity = commands
            .spawn((
                sprite,
                transform,
                PixelWorldSocialLinkVisual {
                    link_id: link_id.to_string(),
                    part,
                },
            ))
            .id();
        by_key.insert(key, entity);
    }
}

fn upsert_glyph(
    commands: &mut Commands,
    by_key: &mut HashMap<(String, SocialLinkPart), Entity>,
    active: &mut HashSet<(String, SocialLinkPart)>,
    link_id: &str,
    part: SocialLinkPart,
    x: f64,
    y: f64,
    color: Color,
    width: f64,
    height: f64,
) {
    let key = (link_id.to_string(), part);
    active.insert(key.clone());
    let mut transform = Transform::from_translation(to_bevy_translation(
        x,
        y,
        width,
        height,
        SOCIAL_LINK_LAYER_Z + 0.01,
    ));
    transform.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_4);
    let sprite = sprite_for_square(color, 5.0);
    if let Some(entity) = by_key.remove(&key) {
        commands.entity(entity).insert((sprite, transform));
        by_key.insert(key, entity);
    } else {
        let entity = commands
            .spawn((
                sprite,
                transform,
                PixelWorldSocialLinkVisual {
                    link_id: link_id.to_string(),
                    part,
                },
            ))
            .id();
        by_key.insert(key, entity);
    }
}
