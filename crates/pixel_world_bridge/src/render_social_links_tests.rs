use super::*;
use crate::render::social_links::SocialLinkPart;
use std::collections::HashMap;

fn social_entities(app: &mut App) -> HashMap<(String, SocialLinkPart), (Entity, Transform)> {
    let world = app.world_mut();
    let mut query = world.query::<(Entity, &PixelWorldSocialLinkVisual, &Transform)>();
    query
        .iter(world)
        .map(|(entity, visual, transform)| {
            ((visual.link_id.clone(), visual.part), (entity, *transform))
        })
        .collect()
}

fn social_state() -> RenderState {
    let mut state = sample_render_state(12_000.0);
    state.selection = None;
    state.social_links = vec![SocialLink {
        id: "social_edge:7".to_string(),
        from: sample_position(1_250_000.0, 750_000.0),
        to: sample_position(1_750_000.0, 1_250_000.0),
        relation_kind: "ally".to_string(),
        lifecycle: "active".to_string(),
    }];
    state
}

#[test]
fn social_links_are_noninteractive_stable_and_cleaned_up() {
    let mut app = render_test_app(social_state());
    let initial = social_entities(&mut app);
    assert_eq!(initial.len(), 3, "line plus two endpoint glyphs");
    assert!(initial.keys().all(|(id, _)| id == "social_edge:7"));
    let hits = hit_regions(&mut app);

    app.update();
    let repeated = social_entities(&mut app);
    assert_eq!(
        repeated.keys().collect::<std::collections::HashSet<_>>(),
        initial.keys().collect::<std::collections::HashSet<_>>()
    );
    assert_eq!(
        repeated
            .values()
            .map(|(entity, _)| *entity)
            .collect::<std::collections::HashSet<_>>(),
        initial
            .values()
            .map(|(entity, _)| *entity)
            .collect::<std::collections::HashSet<_>>()
    );
    assert_eq!(
        hit_regions(&mut app),
        hits,
        "social visuals do not add hit regions"
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        let mut state = social_state();
        state.social_links.clear();
        runtime.render_state = Some(state);
        runtime.render_version += 1;
    }
    app.update();
    assert!(social_entities(&mut app).is_empty());

    let mut app = render_test_app(social_state());
    assert_eq!(social_entities(&mut app).len(), 3);
    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime.render_state = None;
        runtime.render_version += 1;
    }
    app.update();
    assert!(
        social_entities(&mut app).is_empty(),
        "clearing the render state must despawn social visuals"
    );
    assert!(hit_regions(&mut app).is_empty());
}

#[test]
fn social_links_have_a_stable_nonzero_raster_delta_without_hits() {
    let mut baseline_state = social_state();
    baseline_state.social_links.clear();
    let mut baseline = render_test_app(baseline_state);
    let (baseline_image, baseline_summary) = rasterize_pixel_regression(&mut baseline);
    let baseline_hits = hit_regions(&mut baseline);

    let mut visible = render_test_app(social_state());
    let (visible_image, visible_summary) = rasterize_pixel_regression(&mut visible);
    let mut repeated = render_test_app(social_state());
    let (repeated_image, repeated_summary) = rasterize_pixel_regression(&mut repeated);
    let pixel_delta = baseline_image
        .pixels()
        .zip(visible_image.pixels())
        .filter(|(baseline, visible)| baseline != visible)
        .count();

    assert!(
        visible_summary.non_background_pixels > baseline_summary.non_background_pixels,
        "social line and endpoint glyphs must add visible raster pixels"
    );
    assert!(
        pixel_delta > 0,
        "social visuals must create a nonzero pixel delta"
    );
    assert_ne!(
        visible_summary.raw_rgba_fnv1a64, baseline_summary.raw_rgba_fnv1a64,
        "social visuals must change the deterministic raster signature"
    );
    assert_eq!(
        visible_summary.raw_rgba_fnv1a64, repeated_summary.raw_rgba_fnv1a64,
        "social visuals must produce a stable raster hash"
    );
    assert_eq!(visible_image.as_raw(), repeated_image.as_raw());
    assert_eq!(hit_regions(&mut visible), baseline_hits);
}
