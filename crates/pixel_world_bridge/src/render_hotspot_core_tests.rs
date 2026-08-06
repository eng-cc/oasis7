use super::*;
use crate::render::hotspot_core::{
    HOTSPOT_CORE_COLOR, HOTSPOT_CORE_LAYER_Z_OFFSET, HOTSPOT_CORE_MAX_SIZE_PX,
    HOTSPOT_CORE_MIN_SIZE_PX, HOTSPOT_CORE_SIZE_SCALE, PixelWorldHotspotCoreHighlightVisual,
    PixelWorldHotspotCoreShadowVisual,
};
use crate::render::hotspot_cues::{HotspotCuePart, PixelWorldHotspotCueVisual};
use std::time::Duration;

const HOTSPOT_CORE_HIGHLIGHT_COLOR: Color = Color::srgba_u8(248, 250, 252, 230);
const HOTSPOT_CORE_SHADOW_COLOR: Color = Color::srgba_u8(148, 163, 184, 230);

#[test]
fn bevy_ecs_reconciles_neutral_hotspot_cores_with_stable_hover_hit_regions() {
    let mut app = render_test_app(sample_render_state_with_hotspot_candidates());
    let first = visual_probe_summary(&mut app);
    let first_highlight_entities =
        hotspot_core_decoration_entities::<PixelWorldHotspotCoreHighlightVisual>(&mut app);
    let first_shadow_entities =
        hotspot_core_decoration_entities::<PixelWorldHotspotCoreShadowVisual>(&mut app);
    let first_cue_entities = hotspot_cue_entities(&mut app);

    assert_eq!(first.hotspots.len(), 3);
    assert_eq!(first.hotspot_cores.len(), 3);
    assert_eq!(first.hotspot_entity_cache_size, 3);
    assert_eq!(first.hotspot_core_entity_count, 3);
    assert_eq!(first_highlight_entities.len(), 3);
    assert_eq!(first_shadow_entities.len(), 3);
    assert_eq!(first_cue_entities.len(), 6);
    assert_eq!(
        first.hit_regions, 5,
        "each visible hotspot needs one hover-only hit region alongside the existing agent and location regions"
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
        3,
        "a consecutive visible reconcile must reuse each hotspot core"
    );
    assert_eq!(
        hotspot_core_decoration_entities::<PixelWorldHotspotCoreHighlightVisual>(&mut app),
        first_highlight_entities,
        "a consecutive visible reconcile must reuse each hotspot core highlight"
    );
    assert_eq!(
        hotspot_core_decoration_entities::<PixelWorldHotspotCoreShadowVisual>(&mut app),
        first_shadow_entities,
        "a consecutive visible reconcile must reuse each hotspot core shadow"
    );
    assert_eq!(
        hotspot_cue_entities(&mut app),
        first_cue_entities,
        "a consecutive visible reconcile must reuse each hotspot outline cue"
    );
    let initial_blocker_core_size = first
        .hotspot_cores
        .iter()
        .find(|core| core.id == "hotspot-blocker")
        .expect("blocker hotspot core")
        .size_px;
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(70));
    app.update();
    let pulsed_blocker_core_size = visual_probe_summary(&mut app)
        .hotspot_cores
        .iter()
        .find(|core| core.id == "hotspot-blocker")
        .expect("pulsed blocker hotspot core")
        .size_px;
    assert_ne!(
        pulsed_blocker_core_size, initial_blocker_core_size,
        "the existing hotspot core pulse remains intact"
    );
    assert_eq!(
        hotspot_core_decoration_entities::<PixelWorldHotspotCoreHighlightVisual>(&mut app),
        first_highlight_entities,
        "pulse updates must reuse each hotspot core highlight"
    );
    assert_eq!(
        hotspot_core_decoration_entities::<PixelWorldHotspotCoreShadowVisual>(&mut app),
        first_shadow_entities,
        "pulse updates must reuse each hotspot core shadow"
    );
    assert_eq!(
        hotspot_cue_entities(&mut app),
        first_cue_entities,
        "pulse updates must reuse each hotspot outline cue"
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        let mut without_recent_event = sample_render_state_with_hotspot_candidates();
        without_recent_event
            .visual_hotspots
            .retain(|hotspot| hotspot.id != "hotspot-recent-event");
        runtime.render_state = Some(without_recent_event);
        runtime.render_version += 1;
    }
    app.update();
    let without_recent_event = visual_probe_summary(&mut app);
    assert!(
        !hotspot_cue_parts(&mut app).contains_key("hotspot-recent-event"),
        "removing a recent event must despawn its non-interactive cue"
    );
    assert_eq!(without_recent_event.hotspots.len(), 2);
    assert_eq!(
        without_recent_event.hit_regions, 4,
        "removing a hotspot must remove only that hotspot's hover hit region"
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
    assert!(
        hotspot_core_decoration_entities::<PixelWorldHotspotCoreHighlightVisual>(&mut app)
            .is_empty()
    );
    assert!(
        hotspot_core_decoration_entities::<PixelWorldHotspotCoreShadowVisual>(&mut app).is_empty()
    );
    assert!(hotspot_cue_entities(&mut app).is_empty());
    assert_eq!(removed.hotspot_entity_cache_size, 0);
    assert_eq!(
        removed.hit_regions, 2,
        "removing all hotspots must retain only the existing agent and location hit regions"
    );

    let mut no_render_state_app = render_test_app(sample_render_state_with_hotspot_candidates());
    no_render_state_app
        .world_mut()
        .resource_mut::<BevyRuntimeState>()
        .render_state = None;
    no_render_state_app.update();
    assert_eq!(
        visual_probe_summary(&mut no_render_state_app).hotspot_core_entity_count,
        0
    );
    assert!(
        hotspot_core_decoration_entities::<PixelWorldHotspotCoreHighlightVisual>(
            &mut no_render_state_app
        )
        .is_empty()
    );
    assert!(
        hotspot_core_decoration_entities::<PixelWorldHotspotCoreShadowVisual>(
            &mut no_render_state_app
        )
        .is_empty()
    );
    assert!(hotspot_cue_entities(&mut no_render_state_app).is_empty());
}

#[test]
fn hotspot_hover_hit_regions_keep_identity_and_agent_location_precedence() {
    let mut app = render_test_app(sample_render_state_with_hotspot_candidates());
    let blocker_region = hit_regions(&mut app)
        .into_iter()
        .find(|region| region.kind == "hotspot" && region.id == "hotspot-blocker")
        .expect("visible blocker hotspot needs a stable hover hit region");
    let blocker_center = (
        (blocker_region.left + blocker_region.right) / 2.0,
        (blocker_region.top + blocker_region.bottom) / 2.0,
    );

    assert_eq!(
        hit_test(&hit_regions(&mut app), blocker_center.0, blocker_center.1),
        Some(("hotspot".to_string(), "hotspot-blocker".to_string())),
        "a visible hotspot must resolve to its stable hover identity"
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        let mut overlapping = sample_render_state_with_hotspot_candidates();
        overlapping.visual_hotspots[0].pos = overlapping.agents[0]
            .pos
            .clone()
            .expect("fixture agent position");
        runtime.render_state = Some(overlapping);
        runtime.render_version += 1;
    }
    app.update();

    let overlapping_regions = hit_regions(&mut app);
    let agent_region = overlapping_regions
        .iter()
        .find(|region| region.kind == "agent" && region.id == "agent-0")
        .expect("existing agent hit region");
    assert_eq!(
        hit_test(
            &overlapping_regions,
            (agent_region.left + agent_region.right) / 2.0,
            (agent_region.top + agent_region.bottom) / 2.0,
        ),
        Some(("agent".to_string(), "agent-0".to_string())),
        "an overlapping hotspot must not displace existing agent/location hit precedence"
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        let mut without_hotspots = sample_render_state_with_hotspot_candidates();
        without_hotspots.visual_hotspots.clear();
        runtime.render_state = Some(without_hotspots);
        runtime.render_version += 1;
    }
    app.update();
    assert!(
        hit_regions(&mut app)
            .iter()
            .all(|region| region.kind != "hotspot"),
        "hotspot removal clears its hover hit region"
    );
}

#[test]
fn hotspot_test_readback_returns_only_reconciled_stable_centers_and_clears_with_removal() {
    let mut app = render_test_app(sample_render_state_with_hotspot_candidates());
    let targets = crate::hotspot_test_hit_targets(&hit_regions(&mut app));
    let blocker = targets
        .iter()
        .find(|target| target.id == "hotspot-blocker")
        .expect("reconciled blocker hotspot exposes a test-only pointer target");
    assert_eq!(blocker.kind, "hotspot");
    assert!(
        hit_test(&hit_regions(&mut app), blocker.canvas_x, blocker.canvas_y)
            .is_some_and(|(kind, id)| kind == "hotspot" && id == "hotspot-blocker"),
        "reported center must stay inside the matching live hit region"
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        let mut without_hotspots = sample_render_state_with_hotspot_candidates();
        without_hotspots.visual_hotspots.clear();
        runtime.render_state = Some(without_hotspots);
        runtime.render_version += 1;
    }
    app.update();
    assert!(
        crate::hotspot_test_hit_targets(&hit_regions(&mut app)).is_empty(),
        "removed hotspots must not remain observable through the test-only readback"
    );
}

#[test]
fn hotspot_kinds_keep_the_same_base_footprint_and_add_distinct_non_color_outline_cues() {
    let mut app = render_test_app(sample_render_state_with_hotspot_candidates());
    let footprints = hotspot_base_footprints(&mut app);
    for (id, (width, height, rotation)) in footprints {
        assert_eq!(width, height, "{id} must retain its square base footprint");
        assert_eq!(
            rotation,
            (std::f32::consts::FRAC_PI_4 * 100.0).round() as i16,
            "{id} must retain the legacy diamond base orientation"
        );
    }

    let cues = hotspot_cue_parts(&mut app);
    assert_eq!(
        cues["hotspot-blocker"],
        vec![
            HotspotCuePart::BlockerCrossAscending,
            HotspotCuePart::BlockerCrossDescending,
        ],
        "blockers need a non-color warning cross above the unchanged base"
    );
    assert_eq!(
        cues["hotspot-goal"],
        vec![
            HotspotCuePart::GoalCornerTop,
            HotspotCuePart::GoalCornerRight
        ],
        "goals need a non-color corner outline above the unchanged base"
    );
    assert_eq!(
        cues.get("hotspot-recent-event").map(Vec::len),
        Some(2),
        "resource transfer needs two directional, non-color glyph parts above its unchanged base"
    );
}

#[test]
fn recent_event_kinds_have_distinct_low_density_glyph_parts_without_changing_hotspot_bases_or_hits()
{
    let mut render_state = sample_render_state_with_hotspot_candidates();
    render_state.visual_hotspots.extend([
        VisualHotspot {
            id: "hotspot-build-queue-event".to_string(),
            label: "Build queue update".to_string(),
            kind: "build_queue".to_string(),
            pos: sample_position(2_000_000.0, 1_000_000.0),
            emphasis: Some(0.6),
            size_hint_px: Some(12.0),
        },
        VisualHotspot {
            id: "hotspot-unknown-event".to_string(),
            label: "Future event kind".to_string(),
            kind: "future_event_kind".to_string(),
            pos: sample_position(2_200_000.0, 1_000_000.0),
            emphasis: Some(0.6),
            size_hint_px: Some(12.0),
        },
    ]);
    let mut app = render_test_app(render_state);

    for id in [
        "hotspot-recent-event",
        "hotspot-build-queue-event",
        "hotspot-unknown-event",
    ] {
        let (width, height, rotation) = hotspot_base_footprints(&mut app)[id];
        assert_eq!(
            width, height,
            "{id} must retain the shared square hotspot base"
        );
        assert_eq!(
            rotation,
            (std::f32::consts::FRAC_PI_4 * 100.0).round() as i16,
            "{id} must retain the legacy diamond base orientation"
        );
    }
    assert_eq!(
        visual_probe_summary(&mut app).hit_regions,
        7,
        "kind glyphs must not add hover hit regions"
    );

    let cues = hotspot_cue_parts(&mut app);
    let transfer_parts = &cues["hotspot-recent-event"];
    let build_queue_parts = &cues["hotspot-build-queue-event"];
    assert_eq!(
        transfer_parts.len(),
        2,
        "resource flow must use two short directional glyph parts rather than the neutral fallback tick"
    );
    assert_eq!(
        build_queue_parts.len(),
        2,
        "build queue must use two offset stacked glyph parts rather than the neutral fallback tick"
    );
    assert_ne!(
        transfer_parts, build_queue_parts,
        "resource flow and build queue must have deterministic, distinct non-color glyph part sets"
    );
    assert_eq!(
        cues["hotspot-unknown-event"],
        vec![HotspotCuePart::RecentEventTick],
        "an unknown recent-event kind must retain exactly the existing fallback tick"
    );
}

fn hotspot_base_footprints(app: &mut App) -> std::collections::BTreeMap<String, (i16, i16, i16)> {
    let world = app.world_mut();
    let mut hotspots = world.query::<(&PixelWorldHotspotVisual, &Sprite, &Transform)>();
    hotspots
        .iter(world)
        .map(|(hotspot, sprite, transform)| {
            let size = sprite.custom_size.expect("hotspot base size");
            let (_, _, rotation) = transform.rotation.to_euler(EulerRot::XYZ);
            let orientation = (rotation.rem_euclid(std::f32::consts::PI) * 100.0).round() as i16;
            (
                hotspot.id.clone(),
                (size.x.round() as i16, size.y.round() as i16, orientation),
            )
        })
        .collect()
}

fn hotspot_cue_parts(app: &mut App) -> std::collections::BTreeMap<String, Vec<HotspotCuePart>> {
    let world = app.world_mut();
    let mut cues = world.query::<&PixelWorldHotspotCueVisual>();
    let mut by_id = std::collections::BTreeMap::<String, Vec<HotspotCuePart>>::new();
    for cue in cues.iter(world) {
        by_id.entry(cue.id.clone()).or_default().push(cue.part);
    }
    for parts in by_id.values_mut() {
        parts.sort();
    }
    by_id
}

fn hotspot_cue_entities(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();
    let mut cues = world.query_filtered::<Entity, With<PixelWorldHotspotCueVisual>>();
    let mut entities = cues.iter(world).collect::<Vec<_>>();
    entities.sort_by_key(|entity| entity.to_bits());
    entities
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

#[test]
fn bevy_ecs_layers_static_asymmetric_one_pixel_hotspot_core_treatment_inside_base_footprint() {
    let mut render_state = sample_render_state_with_hotspot_candidates();
    render_state.visual_hotspots[0].size_hint_px = Some(1.0);
    let mut app = render_test_app(render_state);
    let world = app.world_mut();
    let mut sprite_query = world.query::<(&Sprite, &Transform)>();
    let sprites = sprite_query.iter(world).collect::<Vec<_>>();
    let core_transforms = sprites
        .iter()
        .filter(|(sprite, _)| sprite.color == HOTSPOT_CORE_COLOR)
        .map(|(sprite, transform)| {
            (
                sprite.custom_size.expect("hotspot core size").x,
                transform.translation,
            )
        })
        .collect::<Vec<_>>();

    assert!(
        core_transforms
            .iter()
            .any(|(size, _)| *size == HOTSPOT_CORE_MIN_SIZE_PX as f32),
        "the 1px treatment must remain inside the clamped 2px core"
    );
    assert!(
        core_transforms
            .iter()
            .any(|(size, _)| *size == HOTSPOT_CORE_MAX_SIZE_PX as f32),
        "the 1px treatment must remain inside the clamped 5px core"
    );
    let highlights = sprites
        .iter()
        .filter(|(sprite, _)| sprite.color == HOTSPOT_CORE_HIGHLIGHT_COLOR)
        .map(|(sprite, transform)| (sprite, transform.translation))
        .collect::<Vec<_>>();
    let shadows = sprites
        .iter()
        .filter(|(sprite, _)| sprite.color == HOTSPOT_CORE_SHADOW_COLOR)
        .map(|(sprite, transform)| (sprite, transform.translation))
        .collect::<Vec<_>>();

    assert_eq!(highlights.len(), core_transforms.len());
    assert_eq!(shadows.len(), core_transforms.len());
    for (core_size, core_translation) in core_transforms {
        let footprint_half_extent = (core_size - 1.0) / 2.0;
        assert!(highlights.iter().any(|(sprite, translation)| {
            sprite.custom_size == Some(Vec2::ONE)
                && translation.x < core_translation.x
                && translation.y > core_translation.y
                && (translation.x - core_translation.x).abs() <= footprint_half_extent
                && (translation.y - core_translation.y).abs() <= footprint_half_extent
        }));
        assert!(shadows.iter().any(|(sprite, translation)| {
            sprite.custom_size == Some(Vec2::ONE)
                && translation.x > core_translation.x
                && translation.y < core_translation.y
                && (translation.x - core_translation.x).abs() <= footprint_half_extent
                && (translation.y - core_translation.y).abs() <= footprint_half_extent
        }));
    }
}

fn hotspot_core_decoration_entities<T: Component>(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();
    let mut query = world.query_filtered::<Entity, With<T>>();
    let mut entities = query.iter(world).collect::<Vec<_>>();
    entities.sort_by_key(|entity| entity.to_bits());
    entities
}
