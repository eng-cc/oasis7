use super::*;
use std::collections::BTreeMap;

fn depot(id: &str, status: &str) -> MicroDepotFacility {
    MicroDepotFacility {
        id: format!("micro_depot:{id}"),
        facility_id: id.to_string(),
        location_id: "loc-0".to_string(),
        status: status.to_string(),
        pos: sample_position(1_530_000.0, 1_010_000.0),
    }
}

fn fnv1a64(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn glyph_raster_signature(app: &mut App) -> (Vec<[u8; 4]>, String) {
    let world = app.world_mut();
    let mut layers = Vec::new();
    let mut glyphs = world.query::<(&PixelWorldMicroDepotVisual, &Sprite, &Transform)>();
    layers.extend(glyphs.iter(world).map(|(_, sprite, transform)| {
        (
            sprite
                .custom_size
                .expect("glyph sprites have explicit dimensions"),
            sprite.color.to_srgba(),
            transform.rotation.to_euler(EulerRot::XYZ).2,
            transform.translation.z,
        )
    }));
    let mut details = world.query::<(&PixelWorldMicroDepotDetailVisual, &Sprite, &Transform)>();
    layers.extend(details.iter(world).map(|(_, sprite, transform)| {
        (
            sprite
                .custom_size
                .expect("glyph sprites have explicit dimensions"),
            sprite.color.to_srgba(),
            transform.rotation.to_euler(EulerRot::XYZ).2,
            transform.translation.z,
        )
    }));
    layers.sort_by(|left, right| {
        left.3
            .partial_cmp(&right.3)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut pixels = vec![[0_u8; 4]; 32 * 32];
    for (size, color, rotation, _) in layers {
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
    let bytes = pixels.iter().flatten().copied().collect::<Vec<_>>();
    let signature = fnv1a64(&bytes);
    (pixels, signature)
}

#[test]
fn micro_depot_glyphs_are_status_distinct_non_interactive_and_cleanup_stale_entities() {
    let mut state = sample_render_state(12_000.0);
    state.micro_depot_facilities = vec![
        depot("active", "active"),
        depot("suspended", "suspended"),
        depot("depleted", "depleted"),
    ];
    let mut app = render_test_app(state);
    let world = app.world_mut();
    let mut glyphs = world.query::<(&PixelWorldMicroDepotVisual, &Sprite, &Transform)>();
    let rendered = glyphs
        .iter(world)
        .map(|(visual, sprite, transform)| {
            (
                visual.id.clone(),
                (
                    sprite.color.to_srgba(),
                    transform.rotation.to_scaled_axis().z,
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(rendered.len(), 3);
    assert_ne!(
        rendered["micro_depot:active"].0,
        rendered["micro_depot:suspended"].0
    );
    assert_ne!(
        rendered["micro_depot:active"].1,
        rendered["micro_depot:suspended"].1
    );
    assert!(
        world
            .resource::<BevyRuntimeState>()
            .hit_regions
            .iter()
            .all(|region| region.kind != "micro_depot"),
        "facility glyphs must not add an interaction region"
    );

    world
        .resource_mut::<BevyRuntimeState>()
        .render_state
        .as_mut()
        .expect("test render state")
        .micro_depot_facilities
        .truncate(1);
    app.update();
    let world = app.world_mut();
    let mut remaining = world.query::<&PixelWorldMicroDepotVisual>();
    let mut details = world.query::<&PixelWorldMicroDepotDetailVisual>();
    assert_eq!(remaining.iter(world).count(), 1);
    assert_eq!(details.iter(world).count(), 1);
}

#[test]
fn micro_depot_statuses_have_shape_sensitive_raster_signatures() {
    let raster_for = |status: &str| {
        let mut state = sample_render_state(12_000.0);
        state.selection = None;
        state.micro_depot_facilities = vec![depot(status, status)];
        let mut app = render_test_app(state);
        let (pixels, signature) = glyph_raster_signature(&mut app);
        assert!(
            pixels.iter().any(|pixel| match status {
                "active" => *pixel == [52, 211, 153, 255],
                "suspended" => *pixel == [251, 191, 36, 255],
                "depleted" => *pixel == [248, 113, 113, 255],
                _ => false,
            }),
            "{status} must contribute its published status color to the raster"
        );
        signature
    };
    let active = raster_for("active");
    let suspended = raster_for("suspended");
    let depleted = raster_for("depleted");
    assert_ne!(active, suspended);
    assert_ne!(suspended, depleted);
    assert_ne!(active, depleted);
}
