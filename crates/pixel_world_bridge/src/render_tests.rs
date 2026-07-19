use super::*;
use bevy::app::{App, Update};
use bevy::color::Alpha;
use bevy::window::Window;
use image::{Rgba, RgbaImage};
use serde::Serialize;
use std::fs;
use std::path::Path;

const VIEWPORT_WIDTH: u32 = 960;
const VIEWPORT_HEIGHT: u32 = 540;
const PIXEL_BACKGROUND: [u8; 4] = [8, 12, 20, 255];

#[derive(Clone, Debug, Serialize)]
struct VisualProbeRow {
    id: String,
    size_px: f32,
    alpha: f32,
    z: f32,
    x: f32,
    y: f32,
}

#[derive(Clone, Debug, Serialize)]
struct VisualProbeSummary {
    fragments: Vec<VisualProbeRow>,
    locations: Vec<VisualProbeRow>,
    agents: Vec<VisualProbeRow>,
    selected_location_cues: Vec<VisualProbeRow>,
    hit_regions: usize,
    fragment_entity_cache_size: usize,
    location_entity_cache_size: usize,
    agent_entity_cache_size: usize,
}

#[derive(Clone, Debug)]
struct PixelLayer {
    kind: &'static str,
    center_x: f32,
    center_y: f32,
    size: Vec2,
    z: f32,
    rgba: [f32; 4],
}

#[derive(Clone, Debug, Serialize)]
struct PixelRegressionSummary {
    width: u32,
    height: u32,
    raw_rgba_fnv1a64: String,
    non_background_pixels: usize,
    fragment_pixels: usize,
    grid_pixels: usize,
    location_pixels: usize,
    selected_location_cue_pixels: usize,
    agent_pixels: usize,
    fragment_sample_rgba: [u8; 4],
    location_sample_rgba: [u8; 4],
    agent_sample_rgba: [u8; 4],
}

fn sample_position(x_cm: f64, y_cm: f64) -> Position {
    Position {
        x_cm,
        y_cm,
        z_cm: 0.0,
    }
}

fn sample_render_state(fragment_footprint_cm: f64) -> RenderState {
    RenderState {
        world_bounds: Some(WorldBounds {
            width_cm: 3_000_000.0,
            depth_cm: 2_000_000.0,
            height_cm: 500_000.0,
        }),
        locations: vec![Location {
            id: "loc-0".to_string(),
            label: "Fragment Field Anchor".to_string(),
            pos: sample_position(1_500_000.0, 1_000_000.0),
            radius_cm: 30_000.0,
            resource_summary: "-".to_string(),
            size_hint_px: Some(10.0),
            marker_role: Some("logic_anchor".to_string()),
            marker_alpha: Some(0.32),
        }],
        fragment_terrain: vec![FragmentTerrainPatch {
            id: "fragment:loc-0:0".to_string(),
            location_id: "loc-0".to_string(),
            pos: sample_position(1_503_000.0, 1_006_000.0),
            footprint_cm: fragment_footprint_cm,
            dominant_compound: "silicate_matrix".to_string(),
            color: [141, 199, 170],
            emphasis: Some(0.58),
        }],
        agents: vec![Agent {
            id: "agent-0".to_string(),
            label: "Survey Agent".to_string(),
            pos: Some(sample_position(1_520_000.0, 1_015_000.0)),
            location_id: Some("loc-0".to_string()),
            resource_summary: "-".to_string(),
            status_badges: vec!["position=location_derived".to_string()],
            size_hint_px: Some(16.0),
        }],
        links: vec![],
        visual_hotspots: vec![],
        selection: Some(Selection {
            kind: "agent".to_string(),
            id: "agent-0".to_string(),
        }),
    }
}

fn sample_render_state_with_selection(
    fragment_footprint_cm: f64,
    kind: &str,
    id: &str,
) -> RenderState {
    let mut render_state = sample_render_state(fragment_footprint_cm);
    render_state.selection = Some(Selection {
        kind: kind.to_string(),
        id: id.to_string(),
    });
    render_state
}

fn sample_render_state_with_beacon_candidates(kind: &str, id: &str) -> RenderState {
    let mut render_state = sample_render_state_with_selection(12_000.0, kind, id);
    render_state.locations.push(Location {
        id: "loc-1".to_string(),
        label: "Unselected Location".to_string(),
        pos: sample_position(1_620_000.0, 1_100_000.0),
        radius_cm: 30_000.0,
        resource_summary: "-".to_string(),
        size_hint_px: Some(10.0),
        marker_role: Some("logic_anchor".to_string()),
        marker_alpha: Some(0.32),
    });
    render_state.agents.push(Agent {
        id: "agent-1".to_string(),
        label: "Unselected Agent".to_string(),
        pos: Some(sample_position(1_640_000.0, 1_115_000.0)),
        location_id: Some("loc-1".to_string()),
        resource_summary: "-".to_string(),
        status_badges: vec![],
        size_hint_px: Some(16.0),
    });
    render_state
}

fn test_runtime(render_state: RenderState) -> BevyRuntimeState {
    BevyRuntimeState {
        mounted: true,
        render_state: Some(render_state),
        render_version: 1,
        camera: CameraState {
            zoom: 3.0,
            pan_x_px: 0.0,
            pan_y_px: 0.0,
        },
        camera_fit_version: 1,
        camera_user_override: true,
        ..Default::default()
    }
}

fn render_test_app(render_state: RenderState) -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_resource(test_runtime(render_state));
    app.world_mut().spawn((
        Window {
            resolution: (VIEWPORT_WIDTH, VIEWPORT_HEIGHT).into(),
            ..Default::default()
        },
        PrimaryWindow,
    ));
    app.add_systems(Update, render_scene);
    app.update();
    app
}

fn row(id: &str, sprite: &Sprite, transform: &Transform) -> VisualProbeRow {
    let size = sprite
        .custom_size
        .map(|custom_size| custom_size.x.max(custom_size.y))
        .unwrap_or(0.0);
    VisualProbeRow {
        id: id.to_string(),
        size_px: size,
        alpha: sprite.color.alpha(),
        z: transform.translation.z,
        x: transform.translation.x,
        y: transform.translation.y,
    }
}

fn visual_probe_summary(app: &mut App) -> VisualProbeSummary {
    let world = app.world_mut();

    let mut fragment_query = world.query::<(&PixelWorldFragmentVisual, &Sprite, &Transform)>();
    let fragments = fragment_query
        .iter(world)
        .map(|(visual, sprite, transform)| row(&visual.id, sprite, transform))
        .collect();

    let mut location_query = world.query::<(&PixelWorldLocationVisual, &Sprite, &Transform)>();
    let locations = location_query
        .iter(world)
        .map(|(visual, sprite, transform)| row(&visual.id, sprite, transform))
        .collect();

    let mut agent_query = world.query::<(&PixelWorldAgentVisual, &Sprite, &Transform)>();
    let agents = agent_query
        .iter(world)
        .map(|(visual, sprite, transform)| row(&visual.id, sprite, transform))
        .collect();

    let mut selected_location_cue_query =
        world.query::<(&PixelWorldSelectedLocationCue, &Sprite, &Transform)>();
    let selected_location_cues = selected_location_cue_query
        .iter(world)
        .map(|(cue, sprite, transform)| row(&cue.location_id, sprite, transform))
        .collect();

    let runtime = world.resource::<BevyRuntimeState>();
    VisualProbeSummary {
        fragments,
        locations,
        agents,
        selected_location_cues,
        hit_regions: runtime.hit_regions.len(),
        fragment_entity_cache_size: runtime.fragment_entities.len(),
        location_entity_cache_size: runtime.location_entities.len(),
        agent_entity_cache_size: runtime.agent_entities.len(),
    }
}

fn hit_regions(app: &mut App) -> Vec<HitRegion> {
    app.world_mut()
        .resource::<BevyRuntimeState>()
        .hit_regions
        .clone()
}

fn pixel_layer(kind: &'static str, sprite: &Sprite, transform: &Transform) -> PixelLayer {
    let size_px = sprite
        .custom_size
        .map(|custom_size| custom_size.x.max(custom_size.y))
        .unwrap_or(0.0);
    let color = sprite.color.to_srgba();
    PixelLayer {
        kind,
        center_x: (VIEWPORT_WIDTH as f32 / 2.0) + transform.translation.x,
        center_y: (VIEWPORT_HEIGHT as f32 / 2.0) - transform.translation.y,
        size: sprite.custom_size.unwrap_or(Vec2::splat(size_px)),
        z: transform.translation.z,
        rgba: [color.red, color.green, color.blue, color.alpha],
    }
}

fn collect_pixel_layers(app: &mut App) -> Vec<PixelLayer> {
    let world = app.world_mut();
    let mut layers = Vec::new();

    let mut grid_query = world.query::<(&PixelWorldGridVisual, &Sprite, &Transform)>();
    layers.extend(
        grid_query
            .iter(world)
            .map(|(_, sprite, transform)| pixel_layer("grid", sprite, transform)),
    );

    let mut fragment_query = world.query::<(&PixelWorldFragmentVisual, &Sprite, &Transform)>();
    layers.extend(
        fragment_query
            .iter(world)
            .map(|(_, sprite, transform)| pixel_layer("fragment", sprite, transform)),
    );

    let mut selected_location_cue_query =
        world.query::<(&PixelWorldSelectedLocationCue, &Sprite, &Transform)>();
    layers.extend(
        selected_location_cue_query
            .iter(world)
            .map(|(_, sprite, transform)| pixel_layer("selected_location_cue", sprite, transform)),
    );

    let mut location_query = world.query::<(&PixelWorldLocationVisual, &Sprite, &Transform)>();
    layers.extend(
        location_query
            .iter(world)
            .map(|(_, sprite, transform)| pixel_layer("location", sprite, transform)),
    );

    let mut agent_query = world.query::<(&PixelWorldAgentVisual, &Sprite, &Transform)>();
    layers.extend(
        agent_query
            .iter(world)
            .map(|(_, sprite, transform)| pixel_layer("agent", sprite, transform)),
    );

    layers.sort_by(|left, right| {
        left.z
            .partial_cmp(&right.z)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    layers
}

fn blend_src_over(src_rgba: [f32; 4], dst: [u8; 4]) -> [u8; 4] {
    let src_alpha = src_rgba[3].clamp(0.0, 1.0);
    let dst_alpha = (dst[3] as f32 / 255.0).clamp(0.0, 1.0);
    let out_alpha = src_alpha + (dst_alpha * (1.0 - src_alpha));
    let mut out = [0u8; 4];

    for channel in 0..3 {
        let src = src_rgba[channel].clamp(0.0, 1.0);
        let dst = (dst[channel] as f32 / 255.0).clamp(0.0, 1.0);
        let blended = if out_alpha <= f32::EPSILON {
            0.0
        } else {
            ((src * src_alpha) + (dst * dst_alpha * (1.0 - src_alpha))) / out_alpha
        };
        out[channel] = (blended * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    out[3] = (out_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
    out
}

fn layer_kind_id(kind: &str) -> u8 {
    match kind {
        "grid" => 1,
        "fragment" => 2,
        "location" => 3,
        "selected_location_cue" => 4,
        "agent" => 5,
        _ => 0,
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

fn sample_pixel(image: &RgbaImage, layer: &PixelLayer) -> [u8; 4] {
    let x = layer
        .center_x
        .round()
        .clamp(0.0, (VIEWPORT_WIDTH - 1) as f32) as u32;
    let y = layer
        .center_y
        .round()
        .clamp(0.0, (VIEWPORT_HEIGHT - 1) as f32) as u32;
    image.get_pixel(x, y).0
}

fn rasterize_pixel_regression(app: &mut App) -> (RgbaImage, PixelRegressionSummary) {
    let layers = collect_pixel_layers(app);
    let mut image = RgbaImage::from_pixel(VIEWPORT_WIDTH, VIEWPORT_HEIGHT, Rgba(PIXEL_BACKGROUND));
    let mut kind_buffer = vec![0u8; (VIEWPORT_WIDTH * VIEWPORT_HEIGHT) as usize];

    for layer in &layers {
        let width = layer.size.x.round().max(1.0) as i32;
        let height = layer.size.y.round().max(1.0) as i32;
        let left = layer.center_x.round() as i32 - (width / 2);
        let top = layer.center_y.round() as i32 - (height / 2);
        let right = left + width;
        let bottom = top + height;
        for y in top.max(0)..bottom.min(VIEWPORT_HEIGHT as i32) {
            for x in left.max(0)..right.min(VIEWPORT_WIDTH as i32) {
                let pixel = image.get_pixel_mut(x as u32, y as u32);
                let next = blend_src_over(layer.rgba, pixel.0);
                pixel.0 = next;
                kind_buffer[(y as u32 * VIEWPORT_WIDTH + x as u32) as usize] =
                    layer_kind_id(layer.kind);
            }
        }
    }

    let non_background_pixels = image
        .pixels()
        .filter(|pixel| pixel.0 != PIXEL_BACKGROUND)
        .count();
    let grid_pixels = kind_buffer.iter().filter(|kind| **kind == 1).count();
    let fragment_pixels = kind_buffer.iter().filter(|kind| **kind == 2).count();
    let location_pixels = kind_buffer.iter().filter(|kind| **kind == 3).count();
    let selected_location_cue_pixels = kind_buffer.iter().filter(|kind| **kind == 4).count();
    let agent_pixels = kind_buffer.iter().filter(|kind| **kind == 5).count();

    let fragment_layer = layers
        .iter()
        .find(|layer| layer.kind == "fragment")
        .expect("pixel regression fragment layer");
    let location_layer = layers
        .iter()
        .find(|layer| layer.kind == "location")
        .expect("pixel regression location layer");
    let agent_layer = layers
        .iter()
        .find(|layer| layer.kind == "agent")
        .expect("pixel regression agent layer");

    let summary = PixelRegressionSummary {
        width: VIEWPORT_WIDTH,
        height: VIEWPORT_HEIGHT,
        raw_rgba_fnv1a64: fnv1a64(image.as_raw()),
        non_background_pixels,
        grid_pixels,
        fragment_pixels,
        location_pixels,
        selected_location_cue_pixels,
        agent_pixels,
        fragment_sample_rgba: sample_pixel(&image, fragment_layer),
        location_sample_rgba: sample_pixel(&image, location_layer),
        agent_sample_rgba: sample_pixel(&image, agent_layer),
    };

    (image, summary)
}

fn write_probe_summary_if_requested(summary: &VisualProbeSummary) {
    let Ok(out_dir) = std::env::var("PIXEL_WORLD_BEVY_RENDER_PROBE_OUT_DIR") else {
        return;
    };
    let out_dir = Path::new(&out_dir);
    fs::create_dir_all(out_dir).expect("create bevy render probe output directory");
    let summary_json =
        serde_json::to_string_pretty(summary).expect("serialize bevy render probe summary");
    fs::write(out_dir.join("summary.json"), summary_json).expect("write bevy render probe summary");
}

fn write_pixel_probe_if_requested(image: &RgbaImage, summary: &PixelRegressionSummary) {
    let Ok(out_dir) = std::env::var("PIXEL_WORLD_BEVY_PIXEL_PROBE_OUT_DIR") else {
        return;
    };
    let out_dir = Path::new(&out_dir);
    fs::create_dir_all(out_dir).expect("create bevy pixel probe output directory");
    let summary_json =
        serde_json::to_string_pretty(summary).expect("serialize bevy pixel probe summary");
    fs::write(out_dir.join("pixel-summary.json"), summary_json)
        .expect("write bevy pixel probe summary");
    image
        .save(out_dir.join("pixel-regression.png"))
        .expect("write bevy pixel probe png");
    zoom_non_background_pixels(image, 12)
        .save(out_dir.join("pixel-regression-crop.png"))
        .expect("write bevy pixel probe crop png");
}

fn zoom_non_background_pixels(image: &RgbaImage, scale: u32) -> RgbaImage {
    let mut min_x = VIEWPORT_WIDTH;
    let mut min_y = VIEWPORT_HEIGHT;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut found = false;

    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel.0 == PIXEL_BACKGROUND {
            continue;
        }
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
        found = true;
    }

    if !found {
        return image.clone();
    }

    let padding = 12u32;
    min_x = min_x.saturating_sub(padding);
    min_y = min_y.saturating_sub(padding);
    max_x = (max_x + padding).min(VIEWPORT_WIDTH - 1);
    max_y = (max_y + padding).min(VIEWPORT_HEIGHT - 1);

    let crop_width = max_x - min_x + 1;
    let crop_height = max_y - min_y + 1;
    let scale = scale.min((2048 / crop_width.max(crop_height)).max(1));
    let mut zoomed = RgbaImage::from_pixel(
        crop_width * scale,
        crop_height * scale,
        Rgba(PIXEL_BACKGROUND),
    );

    for y in 0..crop_height {
        for x in 0..crop_width {
            let source = *image.get_pixel(min_x + x, min_y + y);
            for yy in 0..scale {
                for xx in 0..scale {
                    zoomed.put_pixel((x * scale) + xx, (y * scale) + yy, source);
                }
            }
        }
    }
    zoomed
}

#[test]
fn bevy_ecs_reconciles_fragment_location_agent_visuals_from_render_state() {
    let mut app = render_test_app(sample_render_state(12_000.0));
    let summary = visual_probe_summary(&mut app);

    assert_eq!(summary.fragments.len(), 1);
    assert_eq!(summary.locations.len(), 1);
    assert_eq!(summary.agents.len(), 1);
    assert_eq!(summary.fragments[0].id, "fragment:loc-0:0");
    assert_eq!(summary.locations[0].id, "loc-0");
    assert_eq!(summary.agents[0].id, "agent-0");

    assert!(summary.fragments[0].size_px < summary.locations[0].size_px);
    assert!(summary.locations[0].size_px < summary.agents[0].size_px);
    assert!(summary.fragments[0].alpha < summary.locations[0].alpha);
    assert!(summary.locations[0].alpha < 0.5);
    assert!(summary.fragments[0].z < summary.locations[0].z);
    assert!(summary.locations[0].z < summary.agents[0].z);
    assert_eq!(summary.hit_regions, 2);
}

#[test]
fn bevy_ecs_reuses_hit_regions_on_unchanged_animation_frames() {
    let mut app = render_test_app(sample_render_state(12_000.0));
    let first_regions = hit_regions(&mut app);
    assert_eq!(first_regions.len(), 2);
    {
        let runtime = app.world_mut().resource::<BevyRuntimeState>();
        assert!(!runtime.hit_regions_dirty);
        assert!(runtime.hit_region_cache_key.is_some());
    }

    app.update();

    let second_regions = hit_regions(&mut app);
    assert_eq!(second_regions, first_regions);
    let runtime = app.world_mut().resource::<BevyRuntimeState>();
    assert!(!runtime.hit_regions_dirty);
    assert!(runtime.hit_region_cache_key.is_some());
}

#[test]
fn bevy_ecs_refreshes_hit_regions_after_render_state_update() {
    let mut app = render_test_app(sample_render_state(12_000.0));
    let first_regions = hit_regions(&mut app);
    let mut updated_state = sample_render_state(12_000.0);
    updated_state.agents[0].id = "agent-updated".to_string();
    updated_state.agents[0].pos = Some(sample_position(2_500_000.0, 1_500_000.0));

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime.render_state = Some(updated_state);
        runtime.render_version += 1;
        runtime.hit_regions_dirty = true;
    }
    app.update();

    let second_regions = hit_regions(&mut app);
    assert_ne!(second_regions, first_regions);
    assert!(
        second_regions
            .iter()
            .any(|region| region.id == "agent-updated")
    );
    assert!(!second_regions.iter().any(|region| region.id == "agent-0"));
}

#[test]
fn bevy_ecs_refreshes_hit_regions_after_camera_change() {
    let mut app = render_test_app(sample_render_state(12_000.0));
    let first_regions = hit_regions(&mut app);

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime.camera.pan_x_px += 40.0;
    }
    app.update();

    let second_regions = hit_regions(&mut app);
    assert_ne!(second_regions, first_regions);
    assert_eq!(second_regions.len(), first_regions.len());
}

#[test]
fn bevy_ecs_removes_hidden_fragment_visuals_and_stale_cache_entries() {
    let mut app = render_test_app(sample_render_state(12_000.0));
    let before = visual_probe_summary(&mut app);
    assert_eq!(before.fragments.len(), 1);
    assert_eq!(before.fragment_entity_cache_size, 1);

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime.render_state = Some(sample_render_state(100.0));
        runtime.render_version += 1;
    }
    app.update();

    let after = visual_probe_summary(&mut app);
    assert_eq!(after.fragments.len(), 0);
    assert_eq!(after.fragment_entity_cache_size, 0);
    assert_eq!(after.locations.len(), 1);
    assert_eq!(after.agents.len(), 1);
}

#[test]
fn bevy_render_probe_contract_captures_visual_hierarchy() {
    let mut app = render_test_app(sample_render_state(12_000.0));
    let summary = visual_probe_summary(&mut app);

    assert_eq!(summary.fragment_entity_cache_size, 1);
    assert_eq!(summary.location_entity_cache_size, 1);
    assert_eq!(summary.agent_entity_cache_size, 1);
    assert!(summary.fragments[0].size_px > 0.0);
    assert!(summary.agents[0].size_px >= 15.0);
    assert!(summary.fragments[0].z < summary.agents[0].z);

    write_probe_summary_if_requested(&summary);
}

#[test]
fn selected_location_has_opaque_amber_two_pixel_ring_above_location_and_below_agents() {
    let mut app = render_test_app(sample_render_state_with_selection(
        12_000.0, "location", "loc-0",
    ));
    let summary = visual_probe_summary(&mut app);
    let selected_location = summary
        .locations
        .iter()
        .find(|location| location.id == "loc-0")
        .expect("selected location visual");

    assert_eq!(summary.selected_location_cues.len(), 4);
    for cue in &summary.selected_location_cues {
        assert_eq!(cue.id, "loc-0");
        assert_eq!(cue.alpha, 1.0);
        assert_eq!(cue.z, SELECTED_LOCATION_CUE_LAYER_Z);
        assert!(cue.size_px >= SELECTED_LOCATION_CUE_THICKNESS_PX);
        assert!(cue.z > selected_location.z);
        assert!(cue.z < summary.agents[0].z);
    }

    let world = app.world_mut();
    let mut cue_query = world.query::<(&Sprite, &PixelWorldSelectedLocationCue)>();
    for (sprite, _) in cue_query.iter(world) {
        let color = sprite.color.to_srgba();
        assert_eq!(
            [color.red, color.green, color.blue],
            [251.0 / 255.0, 191.0 / 255.0, 36.0 / 255.0]
        );
        assert_eq!(color.alpha, 1.0);
        assert!(
            sprite.custom_size.unwrap().x == SELECTED_LOCATION_CUE_THICKNESS_PX
                || sprite.custom_size.unwrap().y == SELECTED_LOCATION_CUE_THICKNESS_PX
        );
    }
}

#[test]
fn selected_location_ring_tracks_selection_and_leaves_no_stale_entities() {
    let mut app = render_test_app(sample_render_state_with_selection(
        12_000.0, "location", "loc-0",
    ));
    assert_eq!(
        visual_probe_summary(&mut app).selected_location_cues.len(),
        4
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        runtime.render_state = Some(sample_render_state_with_selection(
            12_000.0, "agent", "agent-0",
        ));
        runtime.render_version += 1;
    }
    app.update();
    assert!(
        visual_probe_summary(&mut app)
            .selected_location_cues
            .is_empty()
    );

    {
        let mut runtime = app.world_mut().resource_mut::<BevyRuntimeState>();
        let mut render_state = sample_render_state(12_000.0);
        render_state.selection = None;
        runtime.render_state = Some(render_state);
        runtime.render_version += 1;
    }
    app.update();
    assert!(
        visual_probe_summary(&mut app)
            .selected_location_cues
            .is_empty()
    );
}

#[test]
fn bevy_pixel_regression_rasterizes_fragment_location_agent_hierarchy() {
    let mut app = render_test_app(sample_render_state(12_000.0));
    let (image, summary) = rasterize_pixel_regression(&mut app);

    assert!(summary.non_background_pixels >= 350);
    assert!(summary.fragment_pixels > 0);
    assert!(summary.location_pixels > 0);
    assert!(summary.agent_pixels > summary.location_pixels);
    assert!(summary.agent_pixels > summary.fragment_pixels);
    assert_eq!(summary.raw_rgba_fnv1a64, "f2c7f7e9757b32de");
    assert_eq!(summary.agent_sample_rgba, [251, 191, 36, 255]);
    assert!(summary.fragment_sample_rgba[1] > summary.fragment_sample_rgba[0]);
    assert!(summary.location_sample_rgba[1] > summary.location_sample_rgba[0]);

    write_pixel_probe_if_requested(&image, &summary);
}

#[test]
fn bevy_pixel_regression_exports_selected_location_ring_with_world_layers() {
    let mut app = render_test_app(sample_render_state_with_beacon_candidates(
        "location", "loc-0",
    ));
    let (image, summary) = rasterize_pixel_regression(&mut app);

    assert!(summary.grid_pixels > 0);
    assert!(summary.fragment_pixels > 0);
    assert!(summary.location_pixels > 0);
    assert!(summary.selected_location_cue_pixels > 0);
    assert!(summary.agent_pixels > 0);
    assert!(summary.agent_pixels > summary.selected_location_cue_pixels);
    assert!(
        image.pixels().any(|pixel| pixel.0 == [251, 191, 36, 255]),
        "selected location ring must contribute opaque amber pixels"
    );

    write_pixel_probe_if_requested(&image, &summary);
}

#[test]
fn bevy_pixel_regression_keeps_canvas_visible_for_agent_and_location_selection() {
    for (kind, id) in [("agent", "agent-0"), ("location", "loc-0")] {
        let mut app = render_test_app(sample_render_state_with_selection(12_000.0, kind, id));
        let (_, summary) = rasterize_pixel_regression(&mut app);

        assert!(
            summary.non_background_pixels >= 350,
            "{kind} selection should keep the pixel-world canvas nonblank"
        );
        assert!(
            summary.fragment_pixels > 0,
            "{kind} selection should keep fragment pixels"
        );
        assert!(
            summary.location_pixels > 0,
            "{kind} selection should keep location pixels"
        );
        assert!(
            summary.agent_pixels > 0,
            "{kind} selection should keep agent pixels"
        );
        assert_ne!(
            summary.raw_rgba_fnv1a64, "c3c91248ad807f7b",
            "{kind} selection should not rasterize as the all-background frame"
        );
    }
}

#[test]
fn bevy_pixel_regression_gives_selected_agent_and_location_the_same_non_color_beacon() {
    for (kind, selected_id, unselected_id) in [
        ("agent", "agent-0", "agent-1"),
        ("location", "loc-0", "loc-1"),
    ] {
        let mut app = render_test_app(sample_render_state_with_beacon_candidates(
            kind,
            selected_id,
        ));
        let summary = visual_probe_summary(&mut app);

        let (selected, unselected) = match kind {
            "agent" => (
                summary
                    .agents
                    .iter()
                    .find(|row| row.id == selected_id)
                    .unwrap(),
                summary
                    .agents
                    .iter()
                    .find(|row| row.id == unselected_id)
                    .unwrap(),
            ),
            "location" => (
                summary
                    .locations
                    .iter()
                    .find(|row| row.id == selected_id)
                    .unwrap(),
                summary
                    .locations
                    .iter()
                    .find(|row| row.id == unselected_id)
                    .unwrap(),
            ),
            _ => unreachable!("test cases use known render entity kinds"),
        };

        assert!(
            selected.size_px > unselected.size_px,
            "selected {kind} must have a geometry beacon beyond its color"
        );
        assert!(
            selected.z > unselected.z,
            "selected {kind} must have a draw-priority beacon beyond its color"
        );
    }
}
