use super::*;
use crate::render::hotspot_core::{
    HOTSPOT_CORE_COLOR, HOTSPOT_CORE_LAYER_Z_OFFSET, HOTSPOT_CORE_MAX_SIZE_PX,
    HOTSPOT_CORE_MIN_SIZE_PX, HOTSPOT_CORE_SIZE_SCALE,
};

#[test]
fn bevy_ecs_reconciles_neutral_hotspot_cores_without_hit_region_changes() {
    let mut app = render_test_app(sample_render_state_with_hotspot_candidates());
    let first = visual_probe_summary(&mut app);

    assert_eq!(first.hotspots.len(), 2);
    assert_eq!(first.hotspot_cores.len(), 2);
    assert_eq!(first.hotspot_entity_cache_size, 2);
    assert_eq!(first.hotspot_core_entity_count, 2);
    assert_eq!(
        first.hit_regions, 2,
        "hotspot cores must not add hit regions"
    );
    for core in &first.hotspot_cores {
        let base = first
            .hotspots
            .iter()
            .find(|hotspot| hotspot.id == core.id)
            .expect("hotspot base for core");
        assert_eq!(core.x, base.x);
        assert_eq!(core.y, base.y);
        assert_eq!(core.z, base.z + HOTSPOT_CORE_LAYER_Z_OFFSET);
        assert!(
            (HOTSPOT_CORE_MIN_SIZE_PX as f32..=HOTSPOT_CORE_MAX_SIZE_PX as f32)
                .contains(&core.size_px)
        );
        let expected_size = (base.size_px as f64 * HOTSPOT_CORE_SIZE_SCALE)
            .clamp(HOTSPOT_CORE_MIN_SIZE_PX, HOTSPOT_CORE_MAX_SIZE_PX)
            as f32;
        assert_eq!(core.size_px, expected_size);
    }
    let world = app.world_mut();
    let mut core_query = world.query::<(&PixelWorldHotspotCoreVisual, &Sprite)>();
    for (_, sprite) in core_query.iter(world) {
        assert_eq!(sprite.color, HOTSPOT_CORE_COLOR);
    }

    app.update();
    assert_eq!(
        visual_probe_summary(&mut app).hotspot_core_entity_count,
        2,
        "a consecutive visible reconcile must reuse each hotspot core"
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        let mut removed = sample_render_state_with_hotspot_candidates();
        removed.visual_hotspots.clear();
        runtime.render_state = Some(removed);
        runtime.render_version += 1;
    }
    app.update();
    let removed = visual_probe_summary(&mut app);
    assert!(removed.hotspot_cores.is_empty());
    assert_eq!(removed.hotspot_core_entity_count, 0);
    assert_eq!(removed.hotspot_entity_cache_size, 0);
    assert_eq!(
        removed.hit_regions, 2,
        "hotspot removal must not alter hit regions"
    );
}

#[test]
fn bevy_pixel_regression_exports_visible_neutral_hotspot_cores() {
    let mut app = render_test_app(sample_render_state_with_hotspot_candidates());
    let (image, summary) = rasterize_pixel_regression(&mut app);

    assert!(summary.hotspot_pixels > 0);
    assert!(summary.hotspot_core_pixels > 0);
    assert_ne!(summary.hotspot_core_sample_rgba, PIXEL_BACKGROUND);
    assert!(
        image
            .pixels()
            .any(|pixel| pixel.0 == summary.hotspot_core_sample_rgba),
        "neutral hotspot cores must contribute visible raster pixels"
    );

    write_pixel_probe_if_requested(&image, &summary);
}
