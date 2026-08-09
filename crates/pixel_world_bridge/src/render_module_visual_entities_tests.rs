use super::*;
use crate::render::module_visual_entities::{
    MODULE_LABEL_LAYER_Z, MODULE_VISUAL_ENTITY_COLOR, MODULE_VISUAL_ENTITY_SIZE_PX,
    ModuleIdentityChipPart, PixelWorldModuleIdentityChipVisual, PixelWorldModuleVisualEntity,
    PixelWorldModuleVisualLabel,
};

fn module_visual(id: &str, pos: Position) -> ModuleVisualEntity {
    module_visual_with_kind(id, "opaque-kind", pos)
}

fn module_visual_with_kind(id: &str, kind: &str, pos: Position) -> ModuleVisualEntity {
    ModuleVisualEntity {
        id: id.to_string(),
        module_id: "opaque-module".to_string(),
        kind: kind.to_string(),
        label: None,
        pos,
    }
}

fn module_visual_with_label(
    id: &str,
    kind: &str,
    label: Option<&str>,
    pos: Position,
) -> ModuleVisualEntity {
    ModuleVisualEntity {
        id: id.to_string(),
        module_id: "opaque-module".to_string(),
        kind: kind.to_string(),
        label: label.map(ToString::to_string),
        pos,
    }
}

fn marker_raster_signature(app: &mut App) -> (usize, String) {
    let world = app.world_mut();
    let mut markers = world.query::<(&PixelWorldModuleVisualEntity, &Sprite, &Transform)>();
    let mut pixels = vec![[8_u8, 12, 20, 255]; 32 * 32];
    for (_, sprite, transform) in markers.iter(world) {
        let size = sprite
            .custom_size
            .expect("module marker uses an explicit pixel footprint");
        let color = sprite.color.to_srgba();
        let rotation = transform.rotation.to_euler(EulerRot::XYZ).2;
        let (sin, cos) = rotation.sin_cos();
        for y in 0..32 {
            for x in 0..32 {
                let relative_x = x as f32 + 0.5 - 16.0;
                let relative_y = y as f32 + 0.5 - 16.0;
                let local_x = (cos * relative_x) + (sin * relative_y);
                let local_y = (-sin * relative_x) + (cos * relative_y);
                if local_x.abs() <= size.x / 2.0 && local_y.abs() <= size.y / 2.0 {
                    pixels[y * 32 + x] = [
                        (color.red * 255.0).round() as u8,
                        (color.green * 255.0).round() as u8,
                        (color.blue * 255.0).round() as u8,
                        (color.alpha * 255.0).round() as u8,
                    ];
                }
            }
        }
    }
    let non_background = pixels
        .iter()
        .filter(|pixel| **pixel != [8, 12, 20, 255])
        .count();
    let signature = pixels
        .iter()
        .flatten()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    (non_background, format!("{signature:016x}"))
}

fn module_visual_raster_signature(kind: &str) -> (usize, String) {
    let mut state = sample_render_state(12_000.0);
    state.module_visual_entities = vec![module_visual_with_kind(
        "module-raster",
        kind,
        sample_position(1_530_000.0, 1_010_000.0),
    )];
    let mut app = render_test_app(state);
    let world = app.world_mut();
    let marker_position = {
        let mut markers = world.query::<(&PixelWorldModuleVisualEntity, &Transform)>();
        markers
            .single(world)
            .expect("module visual base is rendered")
            .1
            .translation
    };
    let mut sprites = world.query::<(
        &Sprite,
        &Transform,
        Option<&PixelWorldModuleVisualEntity>,
        Option<&PixelWorldModuleIdentityChipVisual>,
    )>();
    let mut sprites = sprites
        .iter(world)
        .filter(|(_, _, marker, chip)| marker.is_some() || chip.is_some())
        .collect::<Vec<_>>();
    sprites.sort_by(|(_, left, _, _), (_, right, _, _)| {
        left.translation
            .z
            .partial_cmp(&right.translation.z)
            .expect("sprite layer depth is finite")
    });
    let mut pixels = vec![[8_u8, 12, 20, 255]; 32 * 32];
    for (sprite, transform, _, _) in sprites {
        let Some(size) = sprite.custom_size else {
            continue;
        };
        let color = sprite.color.to_srgba();
        let rotation = transform.rotation.to_euler(EulerRot::XYZ).2;
        let (sin, cos) = rotation.sin_cos();
        for y in 0..32 {
            for x in 0..32 {
                let relative_x =
                    x as f32 + 0.5 - 16.0 - (transform.translation.x - marker_position.x);
                let relative_y =
                    y as f32 + 0.5 - 16.0 - (transform.translation.y - marker_position.y);
                let local_x = (cos * relative_x) + (sin * relative_y);
                let local_y = (-sin * relative_x) + (cos * relative_y);
                if local_x.abs() <= size.x / 2.0 && local_y.abs() <= size.y / 2.0 {
                    pixels[y * 32 + x] = [
                        (color.red * 255.0).round() as u8,
                        (color.green * 255.0).round() as u8,
                        (color.blue * 255.0).round() as u8,
                        (color.alpha * 255.0).round() as u8,
                    ];
                }
            }
        }
    }
    let non_background = pixels
        .iter()
        .filter(|pixel| **pixel != [8, 12, 20, 255])
        .count();
    let signature = pixels
        .iter()
        .flatten()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    (non_background, format!("{signature:016x}"))
}

#[test]
fn module_visual_entities_are_neutral_noninteractive_and_reconcile_stale_markers() {
    let mut state = sample_render_state(12_000.0);
    state.module_visual_entities = vec![
        module_visual_with_kind(
            "module-z",
            "beacon",
            sample_position(1_530_000.0, 1_010_000.0),
        ),
        module_visual_with_kind(
            "module-a",
            "relay",
            sample_position(1_530_000.0, 1_010_000.0),
        ),
    ];
    let mut app = render_test_app(state);
    let world = app.world_mut();
    let mut markers = world.query::<(&PixelWorldModuleVisualEntity, &Sprite, &Transform)>();
    let rendered = markers
        .iter(world)
        .map(|(visual, sprite, transform)| {
            assert_eq!(sprite.color, MODULE_VISUAL_ENTITY_COLOR);
            assert_eq!(
                sprite.custom_size,
                Some(Vec2::splat(MODULE_VISUAL_ENTITY_SIZE_PX))
            );
            (visual.id.clone(), transform.translation)
        })
        .collect::<Vec<_>>();
    assert_eq!(rendered.len(), 2);
    let mut chips = world.query::<&PixelWorldModuleIdentityChipVisual>();
    assert_eq!(
        chips.iter(world).count(),
        2,
        "known module kinds each render one noninteractive identity chip"
    );
    assert_ne!(
        rendered[0].1, rendered[1].1,
        "co-anchors must receive distinct stable offsets"
    );
    assert!(
        world
            .resource::<BevyRuntimeState>()
            .hit_regions
            .iter()
            .all(|region| region.kind != "module_visual"),
        "a module marker must not create an interaction region"
    );

    world
        .resource_mut::<BevyRuntimeState>()
        .render_state
        .as_mut()
        .expect("test render state")
        .module_visual_entities
        .truncate(1);
    app.update();
    let world = app.world_mut();
    let mut remaining = world.query::<&PixelWorldModuleVisualEntity>();
    assert_eq!(remaining.iter(world).count(), 1);
    let mut remaining_chips = world.query::<&PixelWorldModuleIdentityChipVisual>();
    assert_eq!(
        remaining_chips.iter(world).count(),
        1,
        "stale module identity chips must be removed with their base marker"
    );
    assert_eq!(
        world
            .resource::<BevyRuntimeState>()
            .module_visual_entities
            .len(),
        1
    );

    world
        .resource_mut::<BevyRuntimeState>()
        .render_state
        .as_mut()
        .expect("test render state")
        .module_visual_entities[0]
        .kind = "future_module_kind".to_string();
    app.update();
    let world = app.world_mut();
    let mut fallback_chips = world.query::<&PixelWorldModuleIdentityChipVisual>();
    assert_eq!(
        fallback_chips.iter(world).count(),
        0,
        "changing to an unknown kind must reconcile away the previous known-kind chip"
    );
}

#[test]
fn module_visual_marker_has_a_fixed_diamond_raster_signature() {
    let mut state = sample_render_state(12_000.0);
    state.module_visual_entities = vec![module_visual(
        "module-raster",
        sample_position(1_530_000.0, 1_010_000.0),
    )];
    let mut app = render_test_app(state);
    let (non_background, signature) = marker_raster_signature(&mut app);
    assert_eq!(non_background, 40);
    assert_eq!(signature, "8ccc88bd200daee5");
}

#[test]
fn co_anchored_module_identity_chips_follow_their_own_displaced_base_markers() {
    let anchor = sample_position(1_530_000.0, 1_010_000.0);
    let mut state = sample_render_state(12_000.0);
    state.module_visual_entities = vec![
        module_visual_with_kind("module-beacon", "beacon", anchor.clone()),
        module_visual_with_kind("module-relay", "relay", anchor),
    ];
    let mut app = render_test_app(state);
    let world = app.world_mut();
    let mut markers = world.query::<(&PixelWorldModuleVisualEntity, &Transform)>();
    let marker_positions = markers
        .iter(world)
        .map(|(marker, transform)| (marker.id.clone(), transform.translation))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut chips = world.query::<(&PixelWorldModuleIdentityChipVisual, &Transform)>();
    let chip_positions = chips
        .iter(world)
        .map(|(chip, transform)| ((chip.id.clone(), chip.part), transform.translation))
        .collect::<std::collections::BTreeMap<_, _>>();

    for ((id, part), chip_position) in chip_positions {
        let base_position = marker_positions[&id];
        let glyph_local_offset = match part {
            ModuleIdentityChipPart::BeaconStem => Vec2::new(0.0, -2.5),
            ModuleIdentityChipPart::RelayBar => Vec2::new(2.5, 0.0),
            other => panic!("unexpected co-anchor fixture chip part: {other:?}"),
        };
        assert_eq!(
            chip_position.truncate() - base_position.truncate(),
            glyph_local_offset,
            "{id} chip must retain its glyph-local offset from its own displaced base"
        );
    }
}

#[test]
fn known_module_kinds_have_distinct_identity_glyphs_while_unknown_kinds_stay_neutral() {
    let beacon = module_visual_raster_signature("beacon");
    let relay = module_visual_raster_signature("relay");
    let unknown = module_visual_raster_signature("future_module_kind");
    let neutral = module_visual_raster_signature("opaque-kind");

    assert_ne!(
        beacon, relay,
        "known module kinds need distinct, first-glance identity glyphs"
    );
    assert_eq!(
        unknown, neutral,
        "unknown module kinds must retain the neutral fallback raster"
    );
}

#[test]
fn module_visual_labels_are_zoom_gated_stably_suppressed_and_reconciled() {
    let anchor = sample_position(1_530_000.0, 1_010_000.0);
    let mut state = sample_render_state(12_000.0);
    state.module_visual_entities = vec![
        module_visual_with_label("module-z", "relay", Some("Relay marker"), anchor.clone()),
        module_visual_with_label("module-a", "beacon", None, anchor),
    ];
    let mut app = render_test_app(state);
    let world = app.world_mut();
    let mut labels = world.query::<(&PixelWorldModuleVisualLabel, &Text2d, &Transform)>();
    let rendered = labels
        .iter(world)
        .map(|(label, text, transform)| {
            assert_eq!(transform.translation.z, MODULE_LABEL_LAYER_Z);
            (label.id.clone(), text.0.clone())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        rendered,
        vec![("module-a".to_string(), "beacon:module-a".to_string())],
        "the stable ID-first label wins a co-anchor collision and uses the documented fallback"
    );

    world.resource_mut::<BevyRuntimeState>().camera.zoom = 1.0;
    app.update();
    let world = app.world_mut();
    let mut labels = world.query::<&PixelWorldModuleVisualLabel>();
    assert_eq!(
        labels.iter(world).count(),
        0,
        "overview zoom must return to glyph-only rendering"
    );

    world.resource_mut::<BevyRuntimeState>().camera.zoom = 3.0;
    world
        .resource_mut::<BevyRuntimeState>()
        .render_state
        .as_mut()
        .expect("test render state")
        .module_visual_entities
        .truncate(1);
    app.update();
    let world = app.world_mut();
    let mut labels = world.query::<(&PixelWorldModuleVisualLabel, &Text2d)>();
    let updated = labels
        .iter(world)
        .map(|(label, text)| (label.id.clone(), text.0.clone()))
        .collect::<Vec<_>>();
    assert_eq!(
        updated,
        vec![("module-z".to_string(), "Relay marker".to_string())]
    );

    world
        .resource_mut::<BevyRuntimeState>()
        .render_state
        .as_mut()
        .expect("test render state")
        .module_visual_entities
        .clear();
    app.update();
    let world = app.world_mut();
    let mut labels = world.query::<&PixelWorldModuleVisualLabel>();
    assert_eq!(
        labels.iter(world).count(),
        0,
        "removed markers leave no stale labels"
    );
    assert!(
        world
            .resource::<BevyRuntimeState>()
            .hit_regions
            .iter()
            .all(|region| region.kind != "module_visual"),
        "labels must not introduce a hit-test region"
    );
}
