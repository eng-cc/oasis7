use super::fixtures::sample_render_state_with_location_resource_summary;
use super::*;
use crate::render::PixelWorldLocationResourceCue;

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResourceCueGeometry {
    entity: Entity,
    relative_x_milli: i32,
    relative_y_milli: i32,
    width_milli: i32,
    height_milli: i32,
    rotation_milli: i32,
    z_milli: i32,
}

fn location_resource_cue_geometry(app: &mut App) -> Vec<ResourceCueGeometry> {
    let location = visual_probe_summary(app)
        .locations
        .into_iter()
        .find(|location| location.id == "loc-0")
        .expect("resource cue fixture location");
    let world = app.world_mut();
    let mut sprites =
        world.query::<(Entity, &PixelWorldLocationResourceCue, &Sprite, &Transform)>();
    let mut geometry = sprites
        .iter(world)
        .filter_map(|(entity, _, sprite, transform)| {
            let size = sprite.custom_size?;
            Some(ResourceCueGeometry {
                entity,
                relative_x_milli: ((transform.translation.x - location.x) * 1_000.0).round() as i32,
                relative_y_milli: ((transform.translation.y - location.y) * 1_000.0).round() as i32,
                width_milli: (size.x * 1_000.0).round() as i32,
                height_milli: (size.y * 1_000.0).round() as i32,
                rotation_milli: (transform.rotation.to_euler(EulerRot::XYZ).2 * 1_000.0).round()
                    as i32,
                z_milli: (transform.translation.z * 1_000.0).round() as i32,
            })
        })
        .collect::<Vec<_>>();
    geometry.sort_by_key(|shape| {
        (
            shape.relative_x_milli,
            shape.relative_y_milli,
            shape.width_milli,
            shape.height_milli,
            shape.rotation_milli,
            shape.z_milli,
            shape.entity.to_bits(),
        )
    });
    geometry
}

fn cue_shape(geometry: &[ResourceCueGeometry]) -> Vec<ResourceCueGeometry> {
    geometry
        .iter()
        .cloned()
        .map(|mut shape| {
            shape.entity = Entity::PLACEHOLDER;
            shape
        })
        .collect()
}

fn location_hit_regions(app: &mut App) -> Vec<HitRegion> {
    hit_regions(app)
        .into_iter()
        .filter(|region| region.kind == "location")
        .collect()
}

fn selected_resource_readout(app: &mut App) -> Option<(String, String, String)> {
    let world = app.world_mut();
    let mut query = world.query::<&PixelWorldSelectedResourceReadout>();
    query.iter(world).next().map(|readout| {
        (
            readout.target_kind.clone(),
            readout.target_id.clone(),
            readout.display.clone(),
        )
    })
}

#[test]
fn location_resource_report_cue_is_binary_and_ignores_key_count_or_amount() {
    for empty_summary in ["", "-", "amounts:{}"] {
        let mut app = render_test_app(sample_render_state_with_location_resource_summary(
            empty_summary,
        ));
        assert!(
            location_resource_cue_geometry(&mut app).is_empty(),
            "empty or missing resource report {empty_summary:?} must not create a cue"
        );
    }

    let mut one_kind = render_test_app(sample_render_state_with_location_resource_summary(
        "water:1",
    ));
    let one_kind_geometry = location_resource_cue_geometry(&mut one_kind);
    assert!(
        !one_kind_geometry.is_empty(),
        "a non-empty Location resource report must create a visible non-color shape cue"
    );

    let mut many_kinds = render_test_app(sample_render_state_with_location_resource_summary(
        "water:999 · iron:1 · rare-earth:0.0001",
    ));
    let many_kinds_geometry = location_resource_cue_geometry(&mut many_kinds);
    assert_eq!(
        cue_shape(&one_kind_geometry),
        cue_shape(&many_kinds_geometry),
        "the cue shape must stay binary and independent of resource key count or amount"
    );
}

#[test]
fn location_resource_report_cue_reconciles_deterministically_without_hit_or_readout_changes() {
    let mut state = sample_render_state_with_location_resource_summary("water:12");
    state.selection = Some(Selection {
        kind: "location".to_string(),
        id: "loc-0".to_string(),
    });
    let mut app = render_test_app(state);
    let initial_geometry = location_resource_cue_geometry(&mut app);
    assert!(
        !initial_geometry.is_empty(),
        "resource report cue should be rendered"
    );
    let initial_hit_regions = location_hit_regions(&mut app);
    assert_eq!(initial_hit_regions.len(), 1);
    assert_eq!(initial_hit_regions[0].id, "loc-0");
    assert_eq!(
        selected_resource_readout(&mut app),
        Some((
            "location".to_string(),
            "loc-0".to_string(),
            "water:12".to_string(),
        )),
        "the cue must not replace the existing selected resource readout"
    );

    app.update();
    let reconciled_geometry = location_resource_cue_geometry(&mut app);
    assert_eq!(
        cue_shape(&initial_geometry),
        cue_shape(&reconciled_geometry),
        "an unchanged report must retain deterministic cue geometry"
    );
    assert_eq!(
        initial_geometry
            .iter()
            .map(|shape| shape.entity)
            .collect::<Vec<_>>(),
        reconciled_geometry
            .iter()
            .map(|shape| shape.entity)
            .collect::<Vec<_>>(),
        "an unchanged report must reuse cue entities"
    );
    assert_eq!(location_hit_regions(&mut app), initial_hit_regions);

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        let render_state = runtime.render_state.as_mut().expect("render state");
        render_state.locations[0].resource_summary = "-".to_string();
        runtime.render_version += 1;
    }
    app.update();
    assert!(
        location_resource_cue_geometry(&mut app).is_empty(),
        "a stale resource report cue must be removed when the report becomes empty"
    );
    assert_eq!(location_hit_regions(&mut app), initial_hit_regions);
    assert_eq!(
        selected_resource_readout(&mut app),
        Some((
            "location".to_string(),
            "loc-0".to_string(),
            "No resources reported".to_string(),
        )),
        "cue removal must leave the selected empty-state readout authoritative"
    );
}

#[test]
fn location_resource_report_cue_exports_deterministic_raster_delta() {
    let raster_state = |resource_summary: &str| {
        let mut state = sample_render_state(12_000.0);
        state.links.clear();
        state.visual_hotspots.clear();
        state.selection = None;
        state.locations[0].resource_summary = resource_summary.to_string();
        state.locations[0].marker_role = Some("logic_anchor".to_string());
        state.locations[0].marker_alpha = Some(0.32);
        state
    };

    let mut absent = render_test_app(raster_state("-"));
    let (absent_image, absent_summary) = rasterize_pixel_regression(&mut absent);

    let mut visible = render_test_app(raster_state("water:1"));
    let (visible_image, visible_summary) = rasterize_pixel_regression(&mut visible);
    let mut repeated = render_test_app(raster_state("water:1"));
    let (repeated_image, repeated_summary) = rasterize_pixel_regression(&mut repeated);

    let pixel_delta = absent_image
        .pixels()
        .zip(visible_image.pixels())
        .filter(|(absent, visible)| absent != visible)
        .count();
    assert!(
        visible_summary.location_resource_cue_pixels > 0,
        "a published resource report must contribute dedicated raster pixels"
    );
    assert_eq!(
        pixel_delta, visible_summary.location_resource_cue_pixels,
        "the binary resource cue must add exactly its dedicated pixels to the empty-report raster"
    );
    assert_eq!(
        visible_summary.raw_rgba_fnv1a64, repeated_summary.raw_rgba_fnv1a64,
        "the same resource report must produce a deterministic raster hash"
    );
    assert_eq!(
        visible_image.as_raw(),
        repeated_image.as_raw(),
        "the same resource report must reproduce every raster pixel"
    );
    assert_eq!(
        visible_summary.location_resource_cue_pixels, repeated_summary.location_resource_cue_pixels,
        "the same resource report must produce a deterministic cue pixel count"
    );
    assert_eq!(
        absent_summary.location_resource_cue_pixels, 0,
        "an empty resource report must not contribute cue pixels"
    );

    println!(
        "location_resource_report_cue_raster delta={} cue_pixels={} hash={}",
        pixel_delta, visible_summary.location_resource_cue_pixels, visible_summary.raw_rgba_fnv1a64,
    );
    write_pixel_probe_if_requested(&visible_image, &visible_summary);
}
