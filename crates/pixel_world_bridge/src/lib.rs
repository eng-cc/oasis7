use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::mem;
use std::time::Duration;

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowPlugin};
use bevy::winit::{UpdateMode, WinitSettings};
use js_sys::{Function, Object, Reflect};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use serde_wasm_bindgen::{Serializer, from_value};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

mod host_state;
mod render;

thread_local! {
    static BRIDGE_SHARED: RefCell<BridgeSharedState> = RefCell::new(BridgeSharedState::default());
}

#[derive(Clone, Debug, Deserialize)]
struct Position {
    x_cm: f64,
    y_cm: f64,
    #[allow(dead_code)]
    z_cm: f64,
}

#[derive(Clone, Debug, Deserialize)]
struct Location {
    id: String,
    #[allow(dead_code)]
    label: String,
    pos: Position,
    #[allow(dead_code)]
    radius_cm: f64,
    #[allow(dead_code)]
    resource_summary: String,
    size_hint_px: Option<f64>,
    marker_role: Option<String>,
    marker_alpha: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
struct Agent {
    id: String,
    #[allow(dead_code)]
    label: String,
    pos: Option<Position>,
    #[allow(dead_code)]
    location_id: Option<String>,
    #[allow(dead_code)]
    resource_summary: String,
    #[allow(dead_code)]
    status_badges: Vec<String>,
    #[serde(default)]
    position_source: AgentPositionSource,
    size_hint_px: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_optional_string")]
    power_state: Option<String>,
}

fn deserialize_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<Value>::deserialize(deserializer)?;
    Ok(value.and_then(|value| value.as_str().map(ToOwned::to_owned)))
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
enum AgentPositionSource {
    Snapshot,
    LocationDerived,
    #[default]
    Missing,
}

/// Authoritative simulator power state, projected only when the source
/// snapshot contains one of the published enum values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum AgentPowerState {
    Normal,
    LowPower,
    Critical,
    Shutdown,
}

#[derive(Clone, Debug, Deserialize)]
struct Link {
    id: String,
    #[allow(dead_code)]
    kind: String,
    from: Position,
    to: Position,
    emphasis: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
struct SocialLink {
    id: String,
    from: Position,
    to: Position,
    #[serde(default)]
    #[allow(dead_code)]
    relation_kind: String,
    #[serde(default)]
    #[allow(dead_code)]
    lifecycle: String,
}

#[derive(Clone, Debug, Deserialize)]
struct FragmentTerrainPatch {
    id: String,
    #[allow(dead_code)]
    location_id: String,
    pos: Position,
    footprint_cm: f64,
    #[allow(dead_code)]
    dominant_compound: String,
    color: [u8; 3],
    emphasis: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
struct MicroDepotFacility {
    id: String,
    #[allow(dead_code)]
    facility_id: String,
    #[allow(dead_code)]
    location_id: String,
    status: String,
    pos: Position,
    #[serde(default)]
    service_radius_cm: f64,
}

#[derive(Clone, Debug, Deserialize)]
struct ModuleVisualEntity {
    id: String,
    #[allow(dead_code)]
    module_id: String,
    #[allow(dead_code)]
    kind: String,
    #[allow(dead_code)]
    label: Option<String>,
    pos: Position,
}

#[derive(Clone, Debug, Deserialize)]
struct VisualHotspot {
    id: String,
    #[allow(dead_code)]
    label: String,
    #[allow(dead_code)]
    kind: String,
    pos: Position,
    emphasis: Option<f64>,
    size_hint_px: Option<f64>,
}

#[derive(Clone, Debug, Deserialize)]
struct Selection {
    kind: String,
    id: String,
}

#[derive(Clone, Debug, Deserialize)]
struct ReceiptTarget {
    agent_id: String,
    state: String,
}

#[derive(Clone, Debug, Deserialize)]
struct RecommendedTarget {
    agent_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FocusTarget {
    kind: String,
    id: String,
}

#[derive(Clone, Debug, Deserialize)]
struct WorldBounds {
    width_cm: f64,
    depth_cm: f64,
    #[allow(dead_code)]
    height_cm: f64,
}

#[derive(Clone, Debug, Deserialize)]
struct RenderState {
    world_bounds: Option<WorldBounds>,
    locations: Vec<Location>,
    #[serde(default)]
    fragment_terrain: Vec<FragmentTerrainPatch>,
    #[serde(default)]
    micro_depot_facilities: Vec<MicroDepotFacility>,
    #[serde(default)]
    module_visual_entities: Vec<ModuleVisualEntity>,
    agents: Vec<Agent>,
    links: Vec<Link>,
    #[serde(default)]
    social_links: Vec<SocialLink>,
    visual_hotspots: Vec<VisualHotspot>,
    selection: Option<Selection>,
    #[serde(default)]
    receipt_target: Option<ReceiptTarget>,
    #[serde(default)]
    recommended_target: Option<RecommendedTarget>,
}

#[derive(Clone, Debug, Serialize)]
struct CameraStatePayload {
    zoom: f64,
    pan_x_px: i32,
    pan_y_px: i32,
}

#[derive(Clone, Debug)]
struct CameraState {
    zoom: f64,
    pan_x_px: f64,
    pan_y_px: f64,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan_x_px: 0.0,
            pan_y_px: 0.0,
        }
    }
}

#[derive(Clone, Debug)]
struct DragState {
    pointer_id: i32,
    start_x: f64,
    start_y: f64,
    start_pan_x: f64,
    start_pan_y: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct HitRegion {
    kind: &'static str,
    id: String,
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct HotspotTestHitTarget {
    kind: &'static str,
    id: String,
    canvas_x: f64,
    canvas_y: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
struct LocationTestHitTarget {
    id: String,
    canvas_x: f64,
    canvas_y: f64,
}

const HOTSPOT_TEST_READBACK_CONTRACT: &str = "oasis7_hotspot_pointer_evidence_v1";
const LOCATION_TEST_READBACK_CONTRACT: &str = "oasis7_location_frame_evidence_v1";

#[derive(Clone, Debug)]
enum InputEvent {
    PointerDown {
        x: f64,
        y: f64,
        pointer_id: i32,
    },
    PointerMove {
        x: f64,
        y: f64,
        is_leave: bool,
        pointer_id: i32,
    },
    PointerUp {
        pointer_id: i32,
    },
    Wheel {
        delta_y: f64,
    },
    Click {
        x: f64,
        y: f64,
    },
}

#[derive(Default)]
struct BridgeSharedState {
    booted: bool,
    mounted: bool,
    canvas_selector: Option<String>,
    render_state: Option<RenderState>,
    render_version: u64,
    animation_version: u64,
    input_events: Vec<InputEvent>,
    on_event: Option<Function>,
    on_fatal: Option<Function>,
    hotspot_test_targets: Vec<HotspotTestHitTarget>,
    location_test_targets: Vec<LocationTestHitTarget>,
}

#[derive(Resource, Default)]
struct BevyRuntimeState {
    mounted: bool,
    render_state: Option<RenderState>,
    render_version: u64,
    animation_version: u64,
    render_content_signature: u64,
    needs_reconcile: bool,
    animation_dirty: bool,
    reactive_scheduling: bool,
    camera: CameraState,
    camera_fit_version: u64,
    last_canvas_size: Option<(u32, u32)>,
    camera_user_override: bool,
    pending_focus_target: Option<FocusTarget>,
    active_follow_target: Option<FocusTarget>,
    drag_state: Option<DragState>,
    hit_regions: Vec<HitRegion>,
    hit_region_cache_key: Option<HitRegionCacheKey>,
    hit_regions_dirty: bool,
    hover_key: Option<String>,
    grid_layout: Option<render::GridLayoutKey>,
    fragment_entities: HashMap<String, Entity>,
    micro_depot_entities: HashMap<String, Entity>,
    module_visual_entities: HashMap<String, Entity>,
    location_entities: HashMap<String, Entity>,
    agent_entities: HashMap<String, Entity>,
    link_entities: HashMap<String, Entity>,
    hotspot_entities: HashMap<String, Entity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HitRegionCacheKey {
    render_version: u64,
    width_px: u32,
    height_px: u32,
    camera_zoom_bits: u64,
    camera_pan_x_bits: u64,
    camera_pan_y_bits: u64,
}

impl HitRegionCacheKey {
    fn new(runtime: &BevyRuntimeState, width: f64, height: f64) -> Self {
        Self {
            render_version: runtime.render_version,
            width_px: width.round().max(0.0) as u32,
            height_px: height.round().max(0.0) as u32,
            camera_zoom_bits: runtime.camera.zoom.to_bits(),
            camera_pan_x_bits: runtime.camera.pan_x_px.to_bits(),
            camera_pan_y_bits: runtime.camera.pan_y_px.to_bits(),
        }
    }
}

enum RenderSnapshot {
    Unchanged,
    Changed {
        version: u64,
        state: Option<RenderState>,
    },
}

#[derive(Default)]
struct SharedSnapshot {
    mounted: bool,
    render: RenderSnapshot,
    input_events: Vec<InputEvent>,
}

impl Default for RenderSnapshot {
    fn default() -> Self {
        Self::Unchanged
    }
}

#[wasm_bindgen]
pub struct PixelWorldBridge {
    mounted: bool,
    on_event: Function,
    on_fatal: Function,
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.min(max).max(min)
}

fn js_value_from_serializable<T: Serialize>(value: &T) -> Result<JsValue, JsValue> {
    value
        .serialize(&Serializer::json_compatible())
        .map_err(|error| JsValue::from_str(&format!("serialize js payload failed: {error}")))
}

fn render_state_fatal_value(code: &str, message: &str) -> JsValue {
    js_value_from_serializable(&json!({
        "fatal": {
            "code": code,
            "message": message,
        }
    }))
    .unwrap_or_else(|_| {
        let root = Object::new();
        let fatal = Object::new();
        let _ = Reflect::set(&fatal, &JsValue::from_str("code"), &JsValue::from_str(code));
        let _ = Reflect::set(
            &fatal,
            &JsValue::from_str("message"),
            &JsValue::from_str(message),
        );
        let _ = Reflect::set(&root, &JsValue::from_str("fatal"), &fatal);
        root.into()
    })
}

fn status_value(status: &str) -> JsValue {
    js_value_from_serializable(&json!({ "status": status })).unwrap_or(JsValue::NULL)
}

fn parse_render_state(raw: JsValue) -> Result<RenderState, JsValue> {
    from_value(raw)
        .map_err(|error| JsValue::from_str(&format!("render state parse failed: {error}")))
}

#[wasm_bindgen]
pub fn build_pixel_world_render_state(raw_input: JsValue) -> JsValue {
    let input: Value = match from_value(raw_input) {
        Ok(value) => value,
        Err(error) => {
            return render_state_fatal_value(
                "pixel_world_render_state_parse_failed",
                &format!("render state input parse failed: {error}"),
            );
        }
    };
    let render_state = host_state::build_render_state(&input);
    js_value_from_serializable(&render_state).unwrap_or_else(|error| {
        render_state_fatal_value(
            "pixel_world_render_state_serialize_failed",
            &error
                .as_string()
                .unwrap_or_else(|| "render state serialization failed".to_string()),
        )
    })
}

fn fallback_point_for_entity(
    id: &str,
    width: f64,
    height: f64,
    camera: &CameraState,
) -> (f64, f64) {
    let hash_x = ((id.len() * 29) as f64) % (width - 72.0).max(40.0);
    let hash_y = ((id.len() * 17) as f64) % (height - 88.0).max(48.0);
    to_canvas_point(
        &Position {
            x_cm: 36.0 + hash_x,
            y_cm: 44.0 + hash_y,
            z_cm: 0.0,
        },
        &WorldBounds {
            width_cm: width.max(1.0),
            depth_cm: height.max(1.0),
            height_cm: 0.0,
        },
        width,
        height,
        camera,
    )
    .unwrap_or((width / 2.0, height / 2.0))
}

fn to_canvas_point(
    position: &Position,
    world_bounds: &WorldBounds,
    width: f64,
    height: f64,
    camera: &CameraState,
) -> Option<(f64, f64)> {
    let safe_width = world_bounds.width_cm.max(1.0);
    let safe_depth = world_bounds.depth_cm.max(1.0);
    let normalized_x = clamp(position.x_cm / safe_width, 0.0, 1.0);
    let normalized_y = clamp(position.y_cm / safe_depth, 0.0, 1.0);
    let base_x = 20.0 + (normalized_x * (width - 40.0).max(1.0));
    let base_y = 20.0 + (normalized_y * (height - 40.0).max(1.0));
    let centered_x = base_x - (width / 2.0);
    let centered_y = base_y - (height / 2.0);
    Some((
        (width / 2.0) + (centered_x * camera.zoom.max(0.5)) + camera.pan_x_px,
        (height / 2.0) + (centered_y * camera.zoom.max(0.5)) + camera.pan_y_px,
    ))
}

fn to_bevy_translation(canvas_x: f64, canvas_y: f64, width: f64, height: f64, z: f32) -> Vec3 {
    Vec3::new(
        (canvas_x - (width / 2.0)) as f32,
        ((height / 2.0) - canvas_y) as f32,
        z,
    )
}

fn sprite_for_square(color: Color, size: f32) -> Sprite {
    Sprite::from_color(color, Vec2::splat(size))
}

fn sprite_for_rect(color: Color, width: f32, height: f32) -> Sprite {
    Sprite::from_color(color, Vec2::new(width, height))
}

fn transform_for_line(
    from_x: f64,
    from_y: f64,
    to_x: f64,
    to_y: f64,
    width: f64,
    height: f64,
    z: f32,
) -> Transform {
    let mid_x = (from_x + to_x) / 2.0;
    let mid_y = (from_y + to_y) / 2.0;
    let angle = (-(to_y - from_y)).atan2(to_x - from_x) as f32;
    let mut transform =
        Transform::from_translation(to_bevy_translation(mid_x, mid_y, width, height, z));
    transform.rotation = Quat::from_rotation_z(angle);
    transform
}

fn emit_event_value(value: &Value) -> Result<(), JsValue> {
    let payload = js_value_from_serializable(value)?;
    let callback = BRIDGE_SHARED.with(|shared| {
        shared
            .borrow()
            .on_event
            .clone()
            .ok_or_else(|| JsValue::from_str("event callback missing"))
    })?;
    callback
        .call1(&JsValue::NULL, &payload)
        .map(|_| ())
        .map_err(|_| JsValue::from_str("event callback failed"))
}

fn emit_camera_state(camera: &CameraState) -> Result<(), JsValue> {
    emit_event_value(&json!({
        "type": "camera_state_changed",
        "camera": CameraStatePayload {
            zoom: (camera.zoom * 1000.0).round() / 1000.0,
            pan_x_px: camera.pan_x_px.round() as i32,
            pan_y_px: camera.pan_y_px.round() as i32,
        }
    }))
}

fn emit_fatal_payload(message: &str) -> JsValue {
    let payload = json!({
        "code": "pixel_world_renderer_fatal",
        "message": message,
    });
    if let Ok(js_payload) = js_value_from_serializable(&payload) {
        let on_fatal = BRIDGE_SHARED.with(|shared| shared.borrow().on_fatal.clone());
        if let Some(on_fatal) = on_fatal {
            let _ = on_fatal.call1(&JsValue::NULL, &js_payload);
        }
    }
    js_value_from_serializable(&json!({ "status": "unavailable", "fatal": payload }))
        .unwrap_or_else(|_| status_value("unavailable"))
}

fn shared_snapshot(current_render_version: u64) -> SharedSnapshot {
    BRIDGE_SHARED.with(|shared| {
        let mut shared = shared.borrow_mut();
        let render = if shared.render_version == current_render_version {
            RenderSnapshot::Unchanged
        } else {
            RenderSnapshot::Changed {
                version: shared.render_version,
                state: shared.render_state.clone(),
            }
        };
        SharedSnapshot {
            mounted: shared.mounted,
            render,
            input_events: mem::take(&mut shared.input_events),
        }
    })
}

fn shared_animation_version() -> u64 {
    BRIDGE_SHARED.with(|shared| shared.borrow().animation_version)
}

fn hash_f64(hasher: &mut DefaultHasher, value: f64) {
    value.to_bits().hash(hasher);
}

fn hash_position(hasher: &mut DefaultHasher, position: &Position) {
    hash_f64(hasher, position.x_cm);
    hash_f64(hasher, position.y_cm);
    hash_f64(hasher, position.z_cm);
}

fn hash_optional_position(hasher: &mut DefaultHasher, position: Option<&Position>) {
    position.is_some().hash(hasher);
    if let Some(position) = position {
        hash_position(hasher, position);
    }
}

fn social_link_sort_key(link: &SocialLink) -> (&str, u64, u64, u64, u64, u64, u64, &str, &str) {
    (
        link.id.as_str(),
        link.from.x_cm.to_bits(),
        link.from.y_cm.to_bits(),
        link.from.z_cm.to_bits(),
        link.to.x_cm.to_bits(),
        link.to.y_cm.to_bits(),
        link.to.z_cm.to_bits(),
        link.relation_kind.as_str(),
        link.lifecycle.as_str(),
    )
}

fn hash_social_links(hasher: &mut DefaultHasher, links: &[SocialLink]) {
    let mut ordered = links.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|link| social_link_sort_key(link));
    ordered.len().hash(hasher);
    for link in ordered {
        link.id.hash(hasher);
        hash_position(hasher, &link.from);
        hash_position(hasher, &link.to);
        link.relation_kind.hash(hasher);
        link.lifecycle.hash(hasher);
    }
}

fn render_content_signature(render_state: Option<&RenderState>) -> u64 {
    render_signature(render_state, RenderSignatureMode::Content)
}

fn camera_content_signature(render_state: Option<&RenderState>) -> u64 {
    render_signature(render_state, RenderSignatureMode::Camera)
}

#[derive(Clone, Copy)]
enum RenderSignatureMode {
    Content,
    Camera,
}

fn render_signature(render_state: Option<&RenderState>, mode: RenderSignatureMode) -> u64 {
    let include_agent_labels = matches!(mode, RenderSignatureMode::Content);
    let include_resource_reports = matches!(mode, RenderSignatureMode::Content);
    let mut hasher = DefaultHasher::new();
    let Some(render_state) = render_state else {
        return hasher.finish();
    };

    render_state.world_bounds.is_some().hash(&mut hasher);
    if let Some(bounds) = render_state.world_bounds.as_ref() {
        hash_f64(&mut hasher, bounds.width_cm);
        hash_f64(&mut hasher, bounds.depth_cm);
        hash_f64(&mut hasher, bounds.height_cm);
    }

    render_state.locations.len().hash(&mut hasher);
    for location in &render_state.locations {
        location.id.hash(&mut hasher);
        hash_position(&mut hasher, &location.pos);
        hash_f64(&mut hasher, location.size_hint_px.unwrap_or(0.0));
        location.marker_role.hash(&mut hasher);
        hash_f64(&mut hasher, location.marker_alpha.unwrap_or(0.0));
        if include_resource_reports {
            location.resource_summary.hash(&mut hasher);
        }
    }

    render_state.fragment_terrain.len().hash(&mut hasher);
    for fragment in &render_state.fragment_terrain {
        fragment.id.hash(&mut hasher);
        hash_position(&mut hasher, &fragment.pos);
        hash_f64(&mut hasher, fragment.footprint_cm);
        fragment.color.hash(&mut hasher);
        hash_f64(&mut hasher, fragment.emphasis.unwrap_or(0.0));
    }

    render_state.module_visual_entities.len().hash(&mut hasher);
    for entity in &render_state.module_visual_entities {
        entity.id.hash(&mut hasher);
        hash_position(&mut hasher, &entity.pos);
    }

    render_state.agents.len().hash(&mut hasher);
    for agent in &render_state.agents {
        agent.id.hash(&mut hasher);
        if include_agent_labels {
            agent.label.hash(&mut hasher);
        }
        hash_optional_position(&mut hasher, agent.pos.as_ref());
        hash_f64(&mut hasher, agent.size_hint_px.unwrap_or(0.0));
        if matches!(mode, RenderSignatureMode::Content) {
            agent.position_source.hash(&mut hasher);
            agent.power_state.hash(&mut hasher);
        }
    }

    render_state.links.len().hash(&mut hasher);
    for link in &render_state.links {
        link.id.hash(&mut hasher);
        hash_position(&mut hasher, &link.from);
        hash_position(&mut hasher, &link.to);
        hash_f64(&mut hasher, link.emphasis.unwrap_or(0.0));
    }

    if matches!(mode, RenderSignatureMode::Content) {
        hash_social_links(&mut hasher, &render_state.social_links);
    }

    render_state.visual_hotspots.len().hash(&mut hasher);
    for hotspot in &render_state.visual_hotspots {
        hotspot.id.hash(&mut hasher);
        hash_position(&mut hasher, &hotspot.pos);
        hash_f64(&mut hasher, hotspot.emphasis.unwrap_or(0.0));
        hash_f64(&mut hasher, hotspot.size_hint_px.unwrap_or(0.0));
    }

    render_state.receipt_target.is_some().hash(&mut hasher);
    if let Some(receipt_target) = render_state.receipt_target.as_ref() {
        receipt_target.agent_id.hash(&mut hasher);
        receipt_target.state.hash(&mut hasher);
    }

    render_state.recommended_target.is_some().hash(&mut hasher);
    if let Some(recommended_target) = render_state.recommended_target.as_ref() {
        recommended_target.agent_id.hash(&mut hasher);
    }

    hasher.finish()
}

fn focus_target_from_render_state(render_state: Option<&RenderState>) -> Option<FocusTarget> {
    let selection = render_state?.selection.as_ref()?;
    Some(FocusTarget {
        kind: selection.kind.clone(),
        id: selection.id.clone(),
    })
}

fn apply_external_render_snapshot(
    runtime: &mut BevyRuntimeState,
    mounted: bool,
    render: RenderSnapshot,
) {
    runtime.mounted = mounted;
    let RenderSnapshot::Changed {
        version: render_version,
        state: render_state,
    } = render
    else {
        return;
    };

    let previous_focus_target = focus_target_from_render_state(runtime.render_state.as_ref());
    let next_focus_target = focus_target_from_render_state(render_state.as_ref());
    let next_signature = render_content_signature(render_state.as_ref());
    let content_changed = next_signature != runtime.render_content_signature;
    let camera_content_changed = camera_content_signature(render_state.as_ref())
        != camera_content_signature(runtime.render_state.as_ref());
    if camera_content_changed {
        runtime.camera_fit_version = 0;
        runtime.last_canvas_size = None;
        runtime.camera_user_override = false;
        runtime.hit_regions_dirty = true;
    }
    runtime.render_content_signature = next_signature;
    if next_focus_target != previous_focus_target {
        runtime.pending_focus_target = next_focus_target.clone();
        runtime.active_follow_target = next_focus_target
            .as_ref()
            .filter(|target| target.kind == "agent")
            .cloned();
    } else if camera_content_changed
        && let Some(follow_target) = runtime.active_follow_target.clone()
    {
        runtime.pending_focus_target = Some(follow_target);
    }
    runtime.render_version = render_version;
    runtime.render_state = render_state;
    runtime.needs_reconcile = content_changed || next_focus_target != previous_focus_target;
    runtime.hit_regions_dirty |=
        camera_content_changed || next_focus_target != previous_focus_target;
}

fn push_input_event(event: InputEvent) {
    BRIDGE_SHARED.with(|shared| {
        shared.borrow_mut().input_events.push(event);
    });
}

fn boot_bevy_app(canvas_selector: String) {
    let mut app = App::new();
    app.insert_resource(ClearColor(Color::srgb_u8(10, 18, 26)));
    app.insert_resource(BevyRuntimeState::default());
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Pixel World Embedded Runtime".to_string(),
            resolution: (960u32, 540u32).into(),
            canvas: Some(canvas_selector),
            fit_canvas_to_parent: true,
            prevent_default_event_handling: false,
            ..default()
        }),
        ..default()
    }));
    app.insert_resource(WinitSettings {
        // Pointer/window events wake immediately; the timeout is only the
        // low-power ambient animation heartbeat and stays below the 12 Hz cap.
        focused_mode: UpdateMode::reactive(Duration::from_millis(500)),
        unfocused_mode: UpdateMode::reactive_low_power(Duration::from_millis(500)),
    });
    app.add_systems(Startup, setup_scene);
    app.add_systems(Update, (sync_external_state, render::render_scene));
    app.run();
}

fn setup_scene(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn hit_test(hit_regions: &[HitRegion], x: f64, y: f64) -> Option<(String, String)> {
    for region in hit_regions.iter().rev() {
        if x >= region.left && x <= region.right && y >= region.top && y <= region.bottom {
            return Some((region.kind.to_string(), region.id.clone()));
        }
    }
    None
}

fn hotspot_test_hit_targets(hit_regions: &[HitRegion]) -> Vec<HotspotTestHitTarget> {
    hit_regions
        .iter()
        .filter(|region| region.kind == "hotspot")
        .map(|region| HotspotTestHitTarget {
            kind: region.kind,
            id: region.id.clone(),
            canvas_x: (region.left + region.right) / 2.0,
            canvas_y: (region.top + region.bottom) / 2.0,
        })
        .collect()
}

fn publish_hotspot_test_hit_targets(hit_regions: &[HitRegion]) {
    let targets = hotspot_test_hit_targets(hit_regions);
    BRIDGE_SHARED.with(|shared| shared.borrow_mut().hotspot_test_targets = targets);
}

fn publish_location_test_hit_targets(hit_regions: &[HitRegion]) {
    let targets = hit_regions
        .iter()
        .filter(|region| region.kind == "location")
        .map(|region| LocationTestHitTarget {
            id: region.id.clone(),
            canvas_x: (region.left + region.right) / 2.0,
            canvas_y: (region.top + region.bottom) / 2.0,
        })
        .collect();
    BRIDGE_SHARED.with(|shared| shared.borrow_mut().location_test_targets = targets);
}

fn click_selection_from_hit(hit: Option<(String, String)>) -> Option<(String, String)> {
    hit.filter(|(kind, _)| kind != "hotspot")
}

fn process_input_event(runtime: &mut BevyRuntimeState, event: InputEvent) {
    match event {
        InputEvent::PointerDown { x, y, pointer_id } => {
            runtime.drag_state = Some(DragState {
                pointer_id,
                start_x: x,
                start_y: y,
                start_pan_x: runtime.camera.pan_x_px,
                start_pan_y: runtime.camera.pan_y_px,
            });
        }
        InputEvent::PointerMove {
            x,
            y,
            is_leave,
            pointer_id,
        } => {
            if let Some((start_pan_x, start_pan_y, start_x, start_y)) = runtime
                .drag_state
                .as_ref()
                .filter(|drag_state| drag_state.pointer_id == pointer_id)
                .map(|drag_state| {
                    (
                        drag_state.start_pan_x,
                        drag_state.start_pan_y,
                        drag_state.start_x,
                        drag_state.start_y,
                    )
                })
            {
                runtime.camera.pan_x_px = start_pan_x + (x - start_x);
                runtime.camera.pan_y_px = start_pan_y + (y - start_y);
                runtime.camera_user_override = true;
                runtime.active_follow_target = None;
                runtime.pending_focus_target = None;
                runtime.hit_regions_dirty = true;
                let _ = emit_camera_state(&runtime.camera);
                return;
            }

            if is_leave {
                if runtime.hover_key.take().is_some() {
                    let _ = emit_event_value(
                        &json!({ "type": "hover_entity", "selection": Value::Null }),
                    );
                }
                return;
            }

            let hit = hit_test(&runtime.hit_regions, x, y);
            let hover_key = hit.as_ref().map(|(kind, id)| format!("{kind}/{id}"));
            if hover_key == runtime.hover_key {
                return;
            }
            runtime.hover_key = hover_key;
            let selection = hit
                .map(|(kind, id)| json!({ "kind": kind, "id": id }))
                .unwrap_or(Value::Null);
            let _ = emit_event_value(&json!({ "type": "hover_entity", "selection": selection }));
        }
        InputEvent::PointerUp { pointer_id } => {
            if runtime
                .drag_state
                .as_ref()
                .map(|drag_state| drag_state.pointer_id == pointer_id)
                .unwrap_or(false)
            {
                runtime.drag_state = None;
            }
        }
        InputEvent::Wheel { delta_y } => {
            let factor = if delta_y < 0.0 { 1.12 } else { 0.89 };
            runtime.camera.zoom = clamp(runtime.camera.zoom * factor, 0.6, 3.5);
            runtime.camera_user_override = true;
            runtime.active_follow_target = None;
            runtime.pending_focus_target = None;
            runtime.hit_regions_dirty = true;
            let _ = emit_camera_state(&runtime.camera);
        }
        InputEvent::Click { x, y } => {
            if let Some((kind, id)) = click_selection_from_hit(hit_test(&runtime.hit_regions, x, y))
            {
                let _ = emit_event_value(&json!({
                    "type": "select_entity",
                    "selection": { "kind": kind, "id": id }
                }));
            }
        }
    }
}

fn sync_external_state(mut runtime: ResMut<BevyRuntimeState>) {
    runtime.reactive_scheduling = true;
    let snapshot = shared_snapshot(runtime.render_version);
    let SharedSnapshot {
        mounted,
        render,
        input_events,
    } = snapshot;
    apply_external_render_snapshot(&mut runtime, mounted, render);

    let has_input = !input_events.is_empty();
    for event in input_events {
        process_input_event(&mut runtime, event);
    }
    runtime.needs_reconcile |= has_input;
    let animation_version = shared_animation_version();
    runtime.animation_dirty |= animation_version != runtime.animation_version;
    if runtime.animation_dirty {
        runtime.animation_version = animation_version;
    }
}

#[wasm_bindgen]
impl PixelWorldBridge {
    #[wasm_bindgen(constructor)]
    pub fn new(on_event: Function, on_fatal: Function) -> Self {
        Self {
            mounted: false,
            on_event,
            on_fatal,
        }
    }

    #[wasm_bindgen]
    pub fn mount(&mut self, canvas: HtmlCanvasElement, initial_render_state: JsValue) -> JsValue {
        let parsed_state = match parse_render_state(initial_render_state) {
            Ok(state) => state,
            Err(error) => return emit_fatal_payload(&error.as_string().unwrap_or_default()),
        };
        let canvas_id = if canvas.id().is_empty() {
            let generated = "pixel-world-embedded-runtime-canvas".to_string();
            canvas.set_id(&generated);
            generated
        } else {
            canvas.id()
        };
        let canvas_selector = format!("#{canvas_id}");

        let mount_result = BRIDGE_SHARED.with(|shared| {
            let mut shared = shared.borrow_mut();
            if let Some(existing_selector) = &shared.canvas_selector
                && existing_selector != &canvas_selector
            {
                return Err(format!(
                    "bevy runtime already bound to {existing_selector}, cannot rebind to {canvas_selector}"
                ));
            }
            shared.canvas_selector = Some(canvas_selector.clone());
            shared.render_state = Some(parsed_state);
            shared.render_version += 1;
            shared.mounted = true;
            shared.on_event = Some(self.on_event.clone());
            shared.on_fatal = Some(self.on_fatal.clone());
            let should_boot = !shared.booted;
            if should_boot {
                shared.booted = true;
            }
            Ok(should_boot)
        });

        let should_boot = match mount_result {
            Ok(should_boot) => should_boot,
            Err(message) => return emit_fatal_payload(&message),
        };

        self.mounted = true;

        if should_boot {
            boot_bevy_app(canvas_selector);
        }

        let _ = emit_event_value(&json!({ "type": "canvas_ready" }));
        let _ = emit_camera_state(&CameraState::default());
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn update(&mut self, next_render_state: JsValue) -> JsValue {
        if !self.mounted {
            return status_value("detached");
        }
        let parsed_state = match parse_render_state(next_render_state) {
            Ok(state) => state,
            Err(error) => return emit_fatal_payload(&error.as_string().unwrap_or_default()),
        };
        BRIDGE_SHARED.with(|shared| {
            let mut shared = shared.borrow_mut();
            shared.render_state = Some(parsed_state);
            shared.render_version += 1;
        });
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn tick(&mut self, _animation_ms: f64) -> JsValue {
        if self.mounted {
            BRIDGE_SHARED.with(|shared| {
                let mut shared = shared.borrow_mut();
                shared.animation_version = shared.animation_version.wrapping_add(1);
            });
            status_value("ready")
        } else {
            status_value("detached")
        }
    }

    #[wasm_bindgen]
    pub fn hotspot_test_hit_targets(&self, contract: String) -> JsValue {
        if !self.mounted || contract != HOTSPOT_TEST_READBACK_CONTRACT {
            return JsValue::NULL;
        }
        BRIDGE_SHARED.with(|shared| {
            js_value_from_serializable(&shared.borrow().hotspot_test_targets)
                .unwrap_or(JsValue::NULL)
        })
    }

    #[wasm_bindgen]
    pub fn location_test_hit_targets(&self, contract: String) -> JsValue {
        if !self.mounted || contract != LOCATION_TEST_READBACK_CONTRACT {
            return JsValue::NULL;
        }
        BRIDGE_SHARED.with(|shared| {
            js_value_from_serializable(&shared.borrow().location_test_targets)
                .unwrap_or(JsValue::NULL)
        })
    }

    #[wasm_bindgen]
    pub fn pointer_down(&mut self, x: f64, y: f64, pointer_id: i32) -> JsValue {
        push_input_event(InputEvent::PointerDown { x, y, pointer_id });
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn pointer_move(&mut self, x: f64, y: f64, is_leave: bool, pointer_id: i32) -> JsValue {
        push_input_event(InputEvent::PointerMove {
            x,
            y,
            is_leave,
            pointer_id,
        });
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn pointer_up(&mut self, pointer_id: i32) -> JsValue {
        push_input_event(InputEvent::PointerUp { pointer_id });
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn wheel(&mut self, delta_y: f64) -> JsValue {
        push_input_event(InputEvent::Wheel { delta_y });
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn click(&mut self, x: f64, y: f64) -> JsValue {
        push_input_event(InputEvent::Click { x, y });
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn unmount(&mut self) -> JsValue {
        self.mounted = false;
        BRIDGE_SHARED.with(|shared| {
            let mut shared = shared.borrow_mut();
            shared.mounted = false;
            shared.render_state = None;
            shared.render_version += 1;
            shared.input_events.clear();
            shared.hotspot_test_targets.clear();
            shared.location_test_targets.clear();
        });
        status_value("detached")
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod lib_tests;
