use std::collections::{HashMap, HashSet};

use super::*;

pub(super) fn fragment_color(fragment: &FragmentTerrainPatch, lod: FragmentTerrainLod) -> Color {
    let alpha = fragment_alpha(fragment, lod);
    Color::srgba_u8(
        fragment.color[0],
        fragment.color[1],
        fragment.color[2],
        (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

pub(super) fn fragment_inset_color(fragment: &FragmentTerrainPatch) -> Color {
    let alpha = fragment_alpha(fragment, FragmentTerrainLod::Detail);
    Color::srgba_u8(
        fragment.color[0].saturating_mul(3) / 5,
        fragment.color[1].saturating_mul(3) / 5,
        fragment.color[2].saturating_mul(3) / 5,
        (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

pub(super) fn fragment_fleck_color(fragment: &FragmentTerrainPatch) -> Color {
    let alpha = fragment_alpha(fragment, FragmentTerrainLod::Detail);
    let lighten = |channel: u8| channel.saturating_add((u8::MAX - channel) * 2 / 5);
    Color::srgba_u8(
        lighten(fragment.color[0]),
        lighten(fragment.color[1]),
        lighten(fragment.color[2]),
        (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

pub(super) fn fragment_shadow_color(
    fragment: &FragmentTerrainPatch,
    lod: FragmentTerrainLod,
) -> Color {
    let alpha = fragment_alpha(fragment, lod) * f64::from(FRAGMENT_SHADOW_ALPHA_CAP);
    Color::srgba_u8(
        fragment.color[0].saturating_mul(2) / 5,
        fragment.color[1].saturating_mul(2) / 5,
        fragment.color[2].saturating_mul(2) / 5,
        (alpha.clamp(0.0, 1.0) * 255.0).round() as u8,
    )
}

pub(super) fn reconcile_fragments(
    commands: &mut Commands,
    runtime: &mut BevyRuntimeState,
    existing_shadows: &Query<(Entity, &PixelWorldFragmentShadowVisual)>,
    existing_insets: &Query<(Entity, &PixelWorldFragmentInsetVisual)>,
    existing_flecks: &Query<(Entity, &PixelWorldFragmentFleckVisual)>,
    width: f64,
    height: f64,
) {
    let Some(render_state) = runtime.render_state.as_ref() else {
        for (_, entity) in runtime.fragment_entities.drain() {
            commands.entity(entity).despawn();
        }
        for (entity, _) in existing_shadows.iter() {
            commands.entity(entity).despawn();
        }
        for (entity, _) in existing_insets.iter() {
            commands.entity(entity).despawn();
        }
        for (entity, _) in existing_flecks.iter() {
            commands.entity(entity).despawn();
        }
        return;
    };
    let Some(world_bounds) = render_state.world_bounds.as_ref() else {
        for (_, entity) in runtime.fragment_entities.drain() {
            commands.entity(entity).despawn();
        }
        for (entity, _) in existing_shadows.iter() {
            commands.entity(entity).despawn();
        }
        for (entity, _) in existing_insets.iter() {
            commands.entity(entity).despawn();
        }
        for (entity, _) in existing_flecks.iter() {
            commands.entity(entity).despawn();
        }
        return;
    };

    let mut active_ids = HashSet::new();
    let mut active_shadow_ids = HashSet::new();
    let mut active_inset_ids = HashSet::new();
    let mut active_fleck_ids = HashSet::new();
    let existing_shadows_by_id = existing_shadows
        .iter()
        .map(|(entity, shadow)| (shadow.id.clone(), entity))
        .collect::<HashMap<_, _>>();
    let existing_insets_by_id = existing_insets
        .iter()
        .map(|(entity, inset)| (inset.id.clone(), entity))
        .collect::<HashMap<_, _>>();
    let existing_flecks_by_id = existing_flecks
        .iter()
        .map(|(entity, fleck)| (fleck.id.clone(), entity))
        .collect::<HashMap<_, _>>();
    for fragment in &render_state.fragment_terrain {
        let Some(style) =
            fragment_visual_style(fragment, world_bounds, width, height, &runtime.camera)
        else {
            continue;
        };
        let Some((canvas_x, canvas_y)) =
            to_canvas_point(&fragment.pos, world_bounds, width, height, &runtime.camera)
        else {
            continue;
        };
        active_ids.insert(fragment.id.clone());
        let sprite = sprite_for_square(fragment_color(fragment, style.lod), style.size_px as f32);
        let transform = Transform::from_translation(to_bevy_translation(
            canvas_x,
            canvas_y,
            width,
            height,
            style.layer_z,
        ));

        active_shadow_ids.insert(fragment.id.clone());
        let shadow_offset = (style.size_px as f32 * FRAGMENT_SHADOW_OFFSET_CAP).min(1.0);
        let mut shadow_transform = transform;
        shadow_transform.translation += Vec3::new(
            shadow_offset,
            shadow_offset,
            FRAGMENT_SHADOW_LAYER_Z - style.layer_z,
        );
        let shadow_sprite = sprite_for_square(
            fragment_shadow_color(fragment, style.lod),
            style.size_px as f32,
        );
        if let Some(entity) = existing_shadows_by_id.get(&fragment.id) {
            commands
                .entity(*entity)
                .insert((shadow_sprite, shadow_transform));
        } else {
            commands.spawn((
                shadow_sprite,
                shadow_transform,
                PixelWorldFragmentShadowVisual {
                    id: fragment.id.clone(),
                },
            ));
        }

        if let Some(entity) = runtime.fragment_entities.get(&fragment.id).copied() {
            commands.entity(entity).insert((sprite, transform));
        } else {
            let entity = commands
                .spawn((
                    sprite,
                    transform,
                    PixelWorldFragmentVisual {
                        id: fragment.id.clone(),
                    },
                ))
                .id();
            runtime
                .fragment_entities
                .insert(fragment.id.clone(), entity);
        }

        if style.lod == FragmentTerrainLod::Detail {
            active_inset_ids.insert(fragment.id.clone());
            let inset_size = (style.size_px as f32 * FRAGMENT_INSET_SIZE_SCALE).max(1.0);
            let inset_offset = style.size_px as f32 * FRAGMENT_INSET_OFFSET_SCALE;
            let mut inset_transform = transform;
            inset_transform.translation += Vec3::new(
                inset_offset,
                inset_offset,
                FRAGMENT_INSET_LAYER_Z - style.layer_z,
            );
            let inset_sprite = sprite_for_square(fragment_inset_color(fragment), inset_size);
            if let Some(entity) = existing_insets_by_id.get(&fragment.id) {
                commands
                    .entity(*entity)
                    .insert((inset_sprite, inset_transform));
            } else {
                commands.spawn((
                    inset_sprite,
                    inset_transform,
                    PixelWorldFragmentInsetVisual {
                        id: fragment.id.clone(),
                    },
                ));
            }

            active_fleck_ids.insert(fragment.id.clone());
            let fleck_size = (style.size_px as f32 * FRAGMENT_FLECK_SIZE_SCALE).clamp(1.0, 4.0);
            let fleck_offset = style.size_px as f32 * FRAGMENT_FLECK_OFFSET_SCALE;
            let mut fleck_transform = transform;
            fleck_transform.translation += Vec3::new(
                -fleck_offset,
                fleck_offset,
                FRAGMENT_FLECK_LAYER_Z - style.layer_z,
            );
            let fleck_sprite = sprite_for_square(fragment_fleck_color(fragment), fleck_size);
            if let Some(entity) = existing_flecks_by_id.get(&fragment.id) {
                commands
                    .entity(*entity)
                    .insert((fleck_sprite, fleck_transform));
            } else {
                commands.spawn((
                    fleck_sprite,
                    fleck_transform,
                    PixelWorldFragmentFleckVisual {
                        id: fragment.id.clone(),
                    },
                ));
            }
        }
    }

    super::despawn_stale_entities(commands, &mut runtime.fragment_entities, &active_ids);
    for (id, entity) in existing_shadows_by_id {
        if !active_shadow_ids.contains(&id) {
            commands.entity(entity).despawn();
        }
    }
    for (id, entity) in existing_insets_by_id {
        if !active_inset_ids.contains(&id) {
            commands.entity(entity).despawn();
        }
    }
    for (id, entity) in existing_flecks_by_id {
        if !active_fleck_ids.contains(&id) {
            commands.entity(entity).despawn();
        }
    }
}
