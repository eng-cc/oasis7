use std::collections::{HashMap, HashSet};

use super::*;

pub(crate) const MODULE_VISUAL_ENTITY_COLOR: Color = Color::srgb_u8(129, 140, 248);
pub(crate) const MODULE_VISUAL_ENTITY_SIZE_PX: f32 = 6.0;
const MODULE_IDENTITY_CHIP_COLOR: Color = Color::srgba_u8(226, 232, 240, 230);
const MODULE_IDENTITY_CHIP_LAYER_Z_OFFSET: f32 = 0.01;

#[derive(Component)]
pub(crate) struct PixelWorldModuleVisualEntity {
    pub(crate) id: String,
}

/// A small neutral glyph that distinguishes known module visual kinds without
/// changing the base marker's semantic color or interaction footprint.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) enum ModuleIdentityChipPart {
    BeaconStem,
    RelayBar,
    ArtifactDot,
    SensorLead,
    SensorTrail,
}

#[derive(Component)]
pub(crate) struct PixelWorldModuleIdentityChipVisual {
    pub(crate) id: String,
    pub(crate) part: ModuleIdentityChipPart,
}

#[derive(SystemParam)]
pub(crate) struct ModuleIdentityChipQueries<'w, 's> {
    chips: Query<'w, 's, (Entity, &'static PixelWorldModuleIdentityChipVisual)>,
}

pub(crate) fn despawn_module_identity_chips(
    commands: &mut Commands,
    queries: &ModuleIdentityChipQueries,
) {
    for (entity, _) in queries.chips.iter() {
        commands.entity(entity).despawn();
    }
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
    chip_queries: &ModuleIdentityChipQueries,
    width: f64,
    height: f64,
) {
    let existing_chips = chip_queries
        .chips
        .iter()
        .map(|(entity, chip)| ((chip.id.clone(), chip.part), entity))
        .collect::<HashMap<_, _>>();
    let Some(render_state) = runtime.render_state.as_ref() else {
        for (_, entity) in runtime.module_visual_entities.drain() {
            commands.entity(entity).despawn();
        }
        despawn_module_identity_chips(commands, chip_queries);
        return;
    };
    let Some(world_bounds) = render_state.world_bounds.as_ref() else {
        for (_, entity) in runtime.module_visual_entities.drain() {
            commands.entity(entity).despawn();
        }
        despawn_module_identity_chips(commands, chip_queries);
        return;
    };

    let mut entities = render_state
        .module_visual_entities
        .iter()
        .collect::<Vec<_>>();
    entities.sort_by(|left, right| left.id.cmp(&right.id));
    let mut active_ids = HashSet::new();
    let mut active_chips = HashSet::new();
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
        let co_anchor_offset = CO_ANCHOR_OFFSETS[co_anchor_index % CO_ANCHOR_OFFSETS.len()];
        active_ids.insert(entity.id.clone());
        let mut transform = Transform::from_translation(to_bevy_translation(
            canvas_x + f64::from(co_anchor_offset.x),
            canvas_y + f64::from(co_anchor_offset.y),
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
        for (part, glyph_offset, size) in module_identity_chip_specs(&entity.kind) {
            let key = (entity.id.clone(), part);
            active_chips.insert(key.clone());
            let chip_transform = Transform::from_translation(to_bevy_translation(
                canvas_x + f64::from(co_anchor_offset.x + glyph_offset.x),
                canvas_y + f64::from(co_anchor_offset.y + glyph_offset.y),
                width,
                height,
                MODULE_VISUAL_ENTITY_LAYER_Z + MODULE_IDENTITY_CHIP_LAYER_Z_OFFSET,
            ));
            let chip_sprite = sprite_for_rect(MODULE_IDENTITY_CHIP_COLOR, size.x, size.y);
            if let Some(existing) = existing_chips.get(&key) {
                commands
                    .entity(*existing)
                    .insert((chip_sprite, chip_transform));
            } else {
                commands.spawn((
                    chip_sprite,
                    chip_transform,
                    PixelWorldModuleIdentityChipVisual {
                        id: entity.id.clone(),
                        part,
                    },
                ));
            }
        }
    }
    despawn_stale_entities(commands, &mut runtime.module_visual_entities, &active_ids);
    for (key, entity) in existing_chips {
        if !active_chips.contains(&key) {
            commands.entity(entity).despawn();
        }
    }
}

fn module_identity_chip_specs(kind: &str) -> Vec<(ModuleIdentityChipPart, Vec2, Vec2)> {
    match kind {
        "beacon" => vec![(
            ModuleIdentityChipPart::BeaconStem,
            Vec2::new(0.0, 2.5),
            Vec2::new(1.0, 2.0),
        )],
        "relay" => vec![(
            ModuleIdentityChipPart::RelayBar,
            Vec2::new(2.5, 0.0),
            Vec2::new(2.0, 1.0),
        )],
        "artifact" => vec![(
            ModuleIdentityChipPart::ArtifactDot,
            Vec2::new(0.0, 0.0),
            Vec2::ONE,
        )],
        "sensor" => vec![
            (
                ModuleIdentityChipPart::SensorLead,
                Vec2::new(-2.0, 1.5),
                Vec2::ONE,
            ),
            (
                ModuleIdentityChipPart::SensorTrail,
                Vec2::new(2.0, -1.5),
                Vec2::ONE,
            ),
        ],
        _ => Vec::new(),
    }
}
