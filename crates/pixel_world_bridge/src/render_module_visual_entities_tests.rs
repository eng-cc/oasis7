use super::*;
use crate::render::module_visual_entities::{
    MODULE_VISUAL_ENTITY_COLOR, MODULE_VISUAL_ENTITY_SIZE_PX, PixelWorldModuleVisualEntity,
};

fn module_visual(id: &str, pos: Position) -> ModuleVisualEntity {
    ModuleVisualEntity {
        id: id.to_string(),
        module_id: "opaque-module".to_string(),
        kind: "opaque-kind".to_string(),
        label: None,
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

#[test]
fn module_visual_entities_are_neutral_noninteractive_and_reconcile_stale_markers() {
    let mut state = sample_render_state(12_000.0);
    state.module_visual_entities = vec![
        module_visual("module-z", sample_position(1_530_000.0, 1_010_000.0)),
        module_visual("module-a", sample_position(1_530_000.0, 1_010_000.0)),
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
    assert_eq!(
        world
            .resource::<BevyRuntimeState>()
            .module_visual_entities
            .len(),
        1
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
