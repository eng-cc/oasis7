#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
use std::collections::HashMap;
#[cfg(target_arch = "wasm32")]
use std::collections::VecDeque;
#[cfg(not(target_arch = "wasm32"))]
use std::io::{BufRead, Write};
#[cfg(not(target_arch = "wasm32"))]
use std::net::TcpStream;
use std::sync::mpsc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{Receiver, Sender};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use std::thread;

use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::view::{ColorGrading, ColorGradingGlobal};
#[cfg(target_arch = "wasm32")]
use gloo_timers::callback::Interval;
use oasis7::geometry::GeoPos;
use oasis7::simulator::{
    AgentDecisionTrace, AgentKinematics, RunnerMetrics, SpaceConfig, WorldEvent, WorldSnapshot,
};
use oasis7::viewer::{
    ViewerControl, ViewerControlProfile, ViewerRequest, ViewerResponse, ViewerStream,
    VIEWER_PROTOCOL_VERSION,
};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;
#[cfg(target_arch = "wasm32")]
use web_sys::{CloseEvent, ErrorEvent, Event, MessageEvent, UrlSearchParams, WebSocket};

#[cfg(not(target_arch = "wasm32"))]
const DEFAULT_ADDR: &str = "127.0.0.1:5010";
#[cfg(target_arch = "wasm32")]
const DEFAULT_WEB_WS_ADDR: &str = "ws://127.0.0.1:5011";
const DEFAULT_MAX_EVENTS: usize = 100;
const AGENT_BODY_MESH_RADIUS: f32 = 0.5;
const AGENT_BODY_MESH_LENGTH: f32 = 1.0;
const DEFAULT_2D_CAMERA_RADIUS: f32 = 90.0;
const DEFAULT_3D_CAMERA_RADIUS: f32 = 48.0;
const ORBIT_ROTATE_SENSITIVITY: f32 = 0.005;
const ORBIT_PAN_SENSITIVITY: f32 = 0.002;
const ORBIT_ZOOM_SENSITIVITY: f32 = 0.2;
const ORBIT_MIN_RADIUS: f32 = 4.0;
const ORBIT_MAX_RADIUS: f32 = 5_000.0;
const ORTHO_MIN_SCALE: f32 = 0.00005;
const ORTHO_MAX_SCALE: f32 = 128.0;
const PICK_MAX_DISTANCE: f32 = 1.0;
const LABEL_FONT_SIZE: f32 = 18.0;
const LOCATION_LABEL_OFFSET: f32 = 0.8;
const AGENT_LABEL_OFFSET: f32 = 0.6;
const LABEL_SCALE: f32 = 0.03;
const UI_PANEL_WIDTH: f32 = 380.0;
pub(crate) const EGUI_CHAT_INPUT_WIDGET_ID: &str = "viewer-chat-input-message";
mod app_bootstrap;
mod auto_degrade;
mod auto_focus;
mod button_feedback;
mod camera_controls;
mod copyable_text;
mod diagnosis;
mod egui_right_panel;
mod event_click_list;
mod event_window;
mod floating_origin;
mod headless;
mod i18n;
mod industry_graph_view_model;
mod internal_capture;
mod label_lod;
mod location_fragment_render;
mod main_connection;
mod main_ui_runtime;
mod material_library;
mod perf_probe;
mod render_perf_summary;
mod right_panel_module_visibility;
mod scene_dirty_refresh;
mod scene_helpers;
mod selection_emphasis;
mod selection_linking;
mod timeline_controls;
mod ui_locale_text;
mod ui_state_types;
mod ui_text;
mod ui_text_claims;
mod viewer_3d_config;
mod viewer_automation;
mod viewer_env;
mod viewer_render_profile;
#[cfg(target_arch = "wasm32")]
mod wasm_egui_input_bridge;
mod web_test_api;
mod world_overlay;

use app_bootstrap::run_ui;
#[cfg(not(target_arch = "wasm32"))]
use app_bootstrap::{resolve_addr, resolve_offline, run_headless};
use auto_degrade::{auto_degrade_config_from_env, update_auto_degrade_policy, AutoDegradeState};
use auto_focus::{
    apply_startup_auto_focus, auto_focus_config_from_env, handle_focus_selection_hotkey,
    AutoFocusState,
};
use button_feedback::{track_step_loading_state, StepControlLoadingState};
use camera_controls::{
    camera_orbit_preset, camera_projection_for_mode, orbit_camera_controls,
    sync_2d_zoom_projection, sync_camera_mode, sync_world_background_visibility,
    update_grid_line_lod_visibility, OrbitDragState, TwoDZoomTier,
};
use copyable_text::{load_runtime_cjk_font, CopyableTextPanelState};
use diagnosis::{update_diagnosis_panel, DiagnosisState};
use egui_right_panel::render_right_side_panel_egui;
use event_click_list::{handle_event_click_buttons, update_event_click_list_ui};
use event_window::{event_window_policy_from_env, push_event_with_window, EventWindowPolicy};
use floating_origin::update_floating_origin;
use headless::headless_auto_play_once;
#[cfg(not(target_arch = "wasm32"))]
use headless::headless_report;
use internal_capture::{
    internal_capture_config_from_env, trigger_internal_capture, InternalCaptureState,
};
use label_lod::{update_label_lod, LabelLodStats};
use main_connection::*;
use main_ui_runtime::*;
use material_library::{build_fragment_element_material_handles, FragmentElementMaterialHandles};
#[cfg(not(target_arch = "wasm32"))]
use render_perf_summary::sample_headless_perf_summary;
use render_perf_summary::{sample_render_perf_summary, RenderPerfHistory, RenderPerfSummary};
use right_panel_module_visibility::{
    persist_right_panel_module_visibility, resolve_right_panel_module_visibility_resources,
};
use scene_dirty_refresh::{refresh_scene_dirty_objects, scene_requires_full_rebuild};
use scene_helpers::*;
use selection_emphasis::{update_selection_emphasis, SelectionEmphasisState};
use selection_linking::{
    handle_jump_selection_events_button, handle_locate_focus_event_button, pick_3d_selection,
    update_event_object_link_text, EventObjectLinkState,
};
use timeline_controls::{
    handle_control_buttons, handle_timeline_adjust_buttons, handle_timeline_bar_drag,
    handle_timeline_mark_filter_buttons, handle_timeline_mark_jump_buttons,
    handle_timeline_seek_submit, sync_timeline_state_from_world, update_timeline_ui,
    TimelineMarkFilterState, TimelineUiState,
};
use ui_state_types::RightPanelLayoutState;
use ui_state_types::*;
use viewer_3d_config::{
    load_viewer_external_material_config_from, load_viewer_external_mesh_config_from,
    load_viewer_external_texture_config_from, resolve_viewer_3d_config,
    resolve_viewer_external_material_config, resolve_viewer_external_mesh_config,
    resolve_viewer_external_texture_config, Viewer3dConfig, ViewerExternalMaterialConfig,
    ViewerExternalMaterialSlotConfig, ViewerExternalMeshConfig, ViewerExternalTextureConfig,
    ViewerExternalTextureSlotConfig, ViewerGeometryTier, ViewerTonemappingMode,
};
use viewer_automation::{
    run_viewer_automation, viewer_automation_config_from_env, ViewerAutomationState,
};
pub(crate) use viewer_render_profile::{
    flow_render_profile, grid_line_thickness, grid_lod_distance_factor, label_lod_profile,
    FLOW_2D_PLANE_Y, FLOW_2D_THICKNESS_MAX, FLOW_THICKNESS_MAX, FLOW_THICKNESS_MIN,
};
#[cfg(target_arch = "wasm32")]
use wasm_egui_input_bridge::{
    pump_wasm_egui_input_bridge_events, setup_wasm_egui_input_bridge,
    sync_wasm_egui_input_bridge_focus,
};
use world_overlay::{
    handle_world_overlay_toggle_buttons, update_world_overlay_status_text,
    update_world_overlays_3d, world_overlay_config_from_env, OverlayRenderRuntime,
    WorldOverlayConfig, WorldOverlayUiState,
};

#[cfg(not(target_arch = "wasm32"))]
fn setup_wasm_egui_input_bridge() {}

#[cfg(not(target_arch = "wasm32"))]
fn sync_wasm_egui_input_bridge_focus() {}

#[cfg(not(target_arch = "wasm32"))]
fn pump_wasm_egui_input_bridge_events() {}

const WORLD_MIN_AXIS: f32 = 0.1;
const WORLD_FLOOR_THICKNESS: f32 = 0.03;
const WORLD_GRID_LINE_THICKNESS_2D: f32 = 0.008;
const WORLD_GRID_LINE_THICKNESS_3D: f32 = 0.014;
const CHUNK_GRID_LINE_THICKNESS_2D: f32 = 0.012;
const CHUNK_GRID_LINE_THICKNESS_3D: f32 = 0.022;
const MAX_EMISSIVE_COLOR_COMPONENT: f32 = 4.0;
const RECONNECT_BACKOFF_BASE_SECS: f64 = 0.8;
const RECONNECT_BACKOFF_MAX_SECS: f64 = 12.0;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let addr = resolve_addr();
    let headless = viewer_env::viewer_env_present("OASIS7_VIEWER_HEADLESS");
    let offline = resolve_offline(headless);

    if headless {
        run_headless(addr, offline);
    } else {
        run_ui(addr, offline);
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    run_ui(resolve_web_addr(), false);
}

#[cfg(target_arch = "wasm32")]
struct WasmWsRuntime {
    _socket: WebSocket,
    _sender_loop: Interval,
    _on_open: Closure<dyn FnMut(Event)>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
    _on_error: Closure<dyn FnMut(Event)>,
    _on_close: Closure<dyn FnMut(CloseEvent)>,
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static WASM_WS_RUNTIME: RefCell<Option<WasmWsRuntime>> = RefCell::new(None);
    static WASM_WS_REQUEST_QUEUE: RefCell<VecDeque<ViewerRequest>> = RefCell::new(VecDeque::new());
    static WASM_WS_RESPONSE_QUEUE: RefCell<VecDeque<ViewerResponse>> = RefCell::new(VecDeque::new());
}

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Copy)]
struct WasmQueueSendError;

#[cfg(target_arch = "wasm32")]
impl std::fmt::Display for WasmQueueSendError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("wasm queue unavailable")
    }
}

#[cfg(target_arch = "wasm32")]
impl std::error::Error for WasmQueueSendError {}

#[derive(Default)]
struct ViewerReconnectRuntime {
    attempt: u32,
    next_retry_at_secs: Option<f64>,
    last_error_signature: Option<String>,
}

impl ViewerReconnectRuntime {
    fn reset(&mut self) {
        self.attempt = 0;
        self.next_retry_at_secs = None;
        self.last_error_signature = None;
    }
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Default)]
struct WasmViewerRequestTx;

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Default)]
struct WasmViewerResponseTx;

#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy, Default)]
struct WasmViewerResponseRx;

#[cfg(target_arch = "wasm32")]
impl WasmViewerRequestTx {
    fn send(&self, request: ViewerRequest) -> Result<(), WasmQueueSendError> {
        WASM_WS_REQUEST_QUEUE.with(|queue| {
            queue.borrow_mut().push_back(request);
        });
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
impl WasmViewerResponseTx {
    fn send(&self, response: ViewerResponse) -> Result<(), WasmQueueSendError> {
        WASM_WS_RESPONSE_QUEUE.with(|queue| {
            queue.borrow_mut().push_back(response);
        });
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
impl WasmViewerResponseRx {
    fn try_recv(&self) -> Result<ViewerResponse, mpsc::TryRecvError> {
        WASM_WS_RESPONSE_QUEUE.with(|queue| match queue.borrow_mut().pop_front() {
            Some(value) => Ok(value),
            None => Err(mpsc::TryRecvError::Empty),
        })
    }
}

#[cfg(target_arch = "wasm32")]
fn wasm_reset_ws_queues() {
    WASM_WS_REQUEST_QUEUE.with(|queue| queue.borrow_mut().clear());
    WASM_WS_RESPONSE_QUEUE.with(|queue| queue.borrow_mut().clear());
}

#[cfg(target_arch = "wasm32")]
fn wasm_try_recv_request() -> Result<ViewerRequest, mpsc::TryRecvError> {
    WASM_WS_REQUEST_QUEUE.with(|queue| match queue.borrow_mut().pop_front() {
        Some(request) => Ok(request),
        None => Err(mpsc::TryRecvError::Empty),
    })
}

#[cfg(target_arch = "wasm32")]
fn resolve_web_addr() -> String {
    let default_addr = DEFAULT_WEB_WS_ADDR.to_string();
    let Some(window) = web_sys::window() else {
        return default_addr;
    };

    let search = match window.location().search() {
        Ok(search) => search,
        Err(_) => return default_addr,
    };

    let params = match UrlSearchParams::new_with_str(&search) {
        Ok(params) => params,
        Err(_) => return default_addr,
    };

    if let Some(ws) = params.get("ws") {
        return normalize_ws_addr(ws.trim());
    }
    if let Some(addr) = params.get("addr") {
        return normalize_ws_addr(addr.trim());
    }

    default_addr
}

#[cfg(target_arch = "wasm32")]
fn normalize_ws_addr(raw: &str) -> String {
    if raw.starts_with("ws://") || raw.starts_with("wss://") {
        return raw.to_string();
    }
    if let Some(stripped) = raw.strip_prefix("http://") {
        return format!("ws://{stripped}");
    }
    if let Some(stripped) = raw.strip_prefix("https://") {
        return format!("wss://{stripped}");
    }
    format!("ws://{raw}")
}

#[derive(Resource)]
struct ViewerConfig {
    addr: String,
    max_events: usize,
    event_window: EventWindowPolicy,
}

#[derive(Resource, Default)]
struct OfflineConfig {
    offline: bool,
}

#[derive(Resource, Clone, Copy, Debug, Default)]
struct ViewerControlProfileState {
    profile: Option<ViewerControlProfile>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource)]
struct ViewerClient {
    tx: Sender<ViewerRequest>,
    rx: Mutex<Receiver<ViewerResponse>>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Resource, Clone)]
struct ViewerClient {
    tx: WasmViewerRequestTx,
    rx: WasmViewerResponseRx,
}

#[derive(Resource)]
struct ViewerState {
    status: ConnectionStatus,
    snapshot: Option<WorldSnapshot>,
    events: Vec<WorldEvent>,
    decision_traces: Vec<AgentDecisionTrace>,
    metrics: Option<RunnerMetrics>,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            status: ConnectionStatus::Connecting,
            snapshot: None,
            events: Vec::new(),
            decision_traces: Vec::new(),
            metrics: None,
        }
    }
}

#[derive(Resource, Default)]
struct Viewer3dScene {
    root_entity: Option<Entity>,
    floating_origin_offset: Vec3,
    origin: Option<GeoPos>,
    space: Option<SpaceConfig>,
    last_snapshot_time: Option<u64>,
    last_event_id: Option<u64>,
    agent_entities: HashMap<String, Entity>,
    agent_positions: HashMap<String, GeoPos>,
    agent_heights_cm: HashMap<String, i64>,
    agent_location_ids: HashMap<String, String>,
    agent_module_counts: HashMap<String, usize>,
    agent_kinematics: HashMap<String, AgentKinematics>,
    location_entities: HashMap<String, Entity>,
    asset_entities: HashMap<String, Entity>,
    module_visual_entities: HashMap<String, Entity>,
    power_plant_entities: HashMap<String, Entity>,
    chunk_entities: HashMap<String, Entity>,
    chunk_line_entities: HashMap<String, Vec<Entity>>,
    location_positions: HashMap<String, GeoPos>,
    location_radii_cm: HashMap<String, i64>,
    background_entities: Vec<Entity>,
    heat_overlay_entities: Vec<Entity>,
    flow_overlay_entities: Vec<Entity>,
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
enum ViewerCameraMode {
    TwoD,
    ThreeD,
}

impl Default for ViewerCameraMode {
    fn default() -> Self {
        Self::TwoD
    }
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
enum ViewerPanelMode {
    Observe,
}

impl Default for ViewerPanelMode {
    fn default() -> Self {
        Self::Observe
    }
}

#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
enum ViewerExperienceMode {
    Player,
    Director,
}

impl Default for ViewerExperienceMode {
    fn default() -> Self {
        Self::Player
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ViewerMaterialVariantPreset {
    Default,
    Matte,
    Glossy,
}

impl Default for ViewerMaterialVariantPreset {
    fn default() -> Self {
        Self::Default
    }
}

impl ViewerMaterialVariantPreset {
    fn next(self) -> Self {
        match self {
            Self::Default => Self::Matte,
            Self::Matte => Self::Glossy,
            Self::Glossy => Self::Default,
        }
    }
}

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
struct MaterialVariantPreviewState {
    active: ViewerMaterialVariantPreset,
}

#[derive(Resource)]
struct Viewer3dAssets {
    agent_mesh: Handle<Mesh>,
    agent_material: Handle<StandardMaterial>,
    agent_module_marker_mesh: Handle<Mesh>,
    agent_module_marker_material: Handle<StandardMaterial>,
    location_mesh: Handle<Mesh>,
    fragment_element_material_library: FragmentElementMaterialHandles,
    asset_mesh: Handle<Mesh>,
    asset_material: Handle<StandardMaterial>,
    power_plant_mesh: Handle<Mesh>,
    power_plant_material: Handle<StandardMaterial>,
    location_core_silicate_material: Handle<StandardMaterial>,
    location_core_metal_material: Handle<StandardMaterial>,
    location_core_ice_material: Handle<StandardMaterial>,
    location_halo_material: Handle<StandardMaterial>,
    chunk_unexplored_material: Handle<StandardMaterial>,
    chunk_generated_material: Handle<StandardMaterial>,
    chunk_exhausted_material: Handle<StandardMaterial>,
    world_box_mesh: Handle<Mesh>,
    world_floor_material: Handle<StandardMaterial>,
    world_bounds_material: Handle<StandardMaterial>,
    world_grid_material: Handle<StandardMaterial>,
    heat_low_material: Handle<StandardMaterial>,
    heat_mid_material: Handle<StandardMaterial>,
    heat_high_material: Handle<StandardMaterial>,
    flow_power_material: Handle<StandardMaterial>,
    flow_trade_material: Handle<StandardMaterial>,
    label_font: Handle<Font>,
}

#[derive(Resource, Default)]
struct ViewerSelection {
    current: Option<SelectionInfo>,
}

#[derive(Resource, Default)]
struct ChatInputFocusSignal {
    wants_ime_focus: bool,
}

#[derive(Clone)]
struct SelectionInfo {
    entity: Entity,
    kind: SelectionKind,
    id: String,
    name: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SelectionKind {
    Agent,
    Location,
    Fragment,
    Asset,
    PowerPlant,
    Chunk,
}

impl ViewerSelection {
    fn clear(&mut self) {
        self.current = None;
    }
}

#[derive(Component)]
struct Viewer3dCamera;

#[derive(Component)]
struct Viewer3dSceneRoot;

#[derive(Component)]
struct WorldFloorSurface;

#[derive(Component)]
struct WorldBoundsSurface;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum ViewerLightRigRole {
    Key,
    Fill,
    Rim,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum GridLineKind {
    World,
    Chunk,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
enum GridLineAxis {
    AlongX,
    AlongZ,
}

#[derive(Component, Clone, Copy, Debug)]
struct GridLineVisual {
    kind: GridLineKind,
    axis: GridLineAxis,
    span: f32,
}

#[derive(Component)]
struct OrbitCamera {
    focus: Vec3,
    radius: f32,
    yaw: f32,
    pitch: f32,
}

impl OrbitCamera {
    fn apply_to_transform(&self, transform: &mut Transform) {
        let rotation =
            Quat::from_axis_angle(Vec3::Y, self.yaw) * Quat::from_axis_angle(Vec3::X, self.pitch);
        let offset = rotation * Vec3::new(0.0, 0.0, self.radius);
        transform.translation = self.focus + offset;
        transform.look_at(self.focus, Vec3::Y);
    }
}

fn grid_line_scale(axis: GridLineAxis, span: f32, thickness: f32) -> Vec3 {
    match axis {
        GridLineAxis::AlongX => Vec3::new(span.max(thickness), thickness, thickness),
        GridLineAxis::AlongZ => Vec3::new(thickness, thickness, span.max(thickness)),
    }
}

fn location_mesh_for_geometry_tier(tier: ViewerGeometryTier) -> Mesh {
    let subdivisions = match tier {
        ViewerGeometryTier::Debug => 2,
        ViewerGeometryTier::Balanced => 4,
        ViewerGeometryTier::Cinematic => 6,
    };
    Sphere::new(1.0)
        .mesh()
        .ico(subdivisions)
        .unwrap_or_else(|_| Sphere::new(1.0).into())
}

fn asset_mesh_for_geometry_tier(tier: ViewerGeometryTier) -> Mesh {
    match tier {
        ViewerGeometryTier::Debug => Cuboid::new(0.40, 0.40, 0.40).into(),
        ViewerGeometryTier::Balanced => Cuboid::new(0.45, 0.45, 0.45).into(),
        ViewerGeometryTier::Cinematic => Cuboid::new(0.50, 0.46, 0.50).into(),
    }
}

fn power_plant_mesh_for_geometry_tier(tier: ViewerGeometryTier) -> Mesh {
    match tier {
        ViewerGeometryTier::Debug => Cuboid::new(0.85, 0.62, 0.85).into(),
        ViewerGeometryTier::Balanced => Cuboid::new(0.95, 0.7, 0.95).into(),
        ViewerGeometryTier::Cinematic => Cuboid::new(1.05, 0.78, 1.05).into(),
    }
}

fn resolve_mesh_handle<F>(
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    override_path: Option<&str>,
    fallback: F,
) -> Handle<Mesh>
where
    F: FnOnce() -> Mesh,
{
    if let Some(path) = override_path {
        asset_server.load(path.to_string())
    } else {
        meshes.add(fallback())
    }
}

fn resolve_srgb_slot_color(default: [f32; 3], override_color: Option<[f32; 3]>) -> [f32; 3] {
    override_color.unwrap_or(default)
}

#[derive(Clone, Debug, Default)]
struct ResolvedTextureSlot {
    base_color_texture: Option<Handle<Image>>,
    normal_map_texture: Option<Handle<Image>>,
    metallic_roughness_texture: Option<Handle<Image>>,
    emissive_texture: Option<Handle<Image>>,
}

struct LocationOverrideMaterialTemplates {
    core_silicate: StandardMaterial,
    core_metal: StandardMaterial,
    core_ice: StandardMaterial,
    halo: StandardMaterial,
}

struct ResolvedThemeSceneAssets {
    agent_mesh: Handle<Mesh>,
    location_mesh: Handle<Mesh>,
    asset_mesh: Handle<Mesh>,
    power_plant_mesh: Handle<Mesh>,
    agent_material: StandardMaterial,
    asset_material: StandardMaterial,
    power_plant_material: StandardMaterial,
    location_override_materials: Option<LocationOverrideMaterialTemplates>,
}

fn resolve_texture_slot(
    asset_server: &AssetServer,
    slot: &ViewerExternalTextureSlotConfig,
) -> ResolvedTextureSlot {
    ResolvedTextureSlot {
        base_color_texture: slot
            .base_texture_asset
            .as_ref()
            .map(|path| asset_server.load(path.to_string())),
        normal_map_texture: slot
            .normal_texture_asset
            .as_ref()
            .map(|path| asset_server.load(path.to_string())),
        metallic_roughness_texture: slot
            .metallic_roughness_texture_asset
            .as_ref()
            .map(|path| asset_server.load(path.to_string())),
        emissive_texture: slot
            .emissive_texture_asset
            .as_ref()
            .map(|path| asset_server.load(path.to_string())),
    }
}

fn resolve_theme_scene_assets(
    config: &Viewer3dConfig,
    external_mesh: &ViewerExternalMeshConfig,
    external_material: &ViewerExternalMaterialConfig,
    external_texture: &ViewerExternalTextureConfig,
    variant_preview: &MaterialVariantPreviewState,
    meshes: &mut Assets<Mesh>,
    asset_server: &AssetServer,
) -> ResolvedThemeSceneAssets {
    let geometry_tier = config.assets.geometry_tier;
    let agent_mesh = resolve_mesh_handle(
        asset_server,
        meshes,
        external_mesh.agent_mesh_asset.as_deref(),
        || Capsule3d::new(AGENT_BODY_MESH_RADIUS, AGENT_BODY_MESH_LENGTH).into(),
    );
    let location_mesh = resolve_mesh_handle(
        asset_server,
        meshes,
        external_mesh.location_mesh_asset.as_deref(),
        || location_mesh_for_geometry_tier(geometry_tier),
    );
    let asset_mesh = resolve_mesh_handle(
        asset_server,
        meshes,
        external_mesh.asset_mesh_asset.as_deref(),
        || asset_mesh_for_geometry_tier(geometry_tier),
    );
    let power_plant_mesh = resolve_mesh_handle(
        asset_server,
        meshes,
        external_mesh.power_plant_mesh_asset.as_deref(),
        || power_plant_mesh_for_geometry_tier(geometry_tier),
    );

    let agent_texture = resolve_texture_slot(asset_server, &external_texture.agent);
    let location_texture = resolve_texture_slot(asset_server, &external_texture.location);
    let asset_texture = resolve_texture_slot(asset_server, &external_texture.asset);
    let power_plant_texture = resolve_texture_slot(asset_server, &external_texture.power_plant);

    let scalars = material_variant_scalars(variant_preview.active);
    let agent_roughness =
        apply_material_variant_scalar(config.materials.agent.roughness, scalars.roughness_scale);
    let agent_metallic =
        apply_material_variant_scalar(config.materials.agent.metallic, scalars.metallic_scale);
    let asset_roughness =
        apply_material_variant_scalar(config.materials.asset.roughness, scalars.roughness_scale);
    let asset_metallic =
        apply_material_variant_scalar(config.materials.asset.metallic, scalars.metallic_scale);
    let facility_roughness =
        apply_material_variant_scalar(config.materials.facility.roughness, scalars.roughness_scale);
    let facility_metallic =
        apply_material_variant_scalar(config.materials.facility.metallic, scalars.metallic_scale);
    let power_plant_roughness = apply_material_variant_scalar(
        config.materials.power_plant.roughness,
        scalars.roughness_scale,
    );
    let power_plant_metallic = apply_material_variant_scalar(
        config.materials.power_plant.metallic,
        scalars.metallic_scale,
    );

    let agent_base_color =
        resolve_srgb_slot_color([1.0, 0.42, 0.22], external_material.agent.base_color_srgb);
    let agent_emissive_color = resolve_srgb_slot_color(
        [0.90, 0.38, 0.20],
        external_material.agent.emissive_color_srgb,
    );
    let agent_material = StandardMaterial {
        base_color: color_from_srgb(agent_base_color),
        base_color_texture: agent_texture.base_color_texture,
        normal_map_texture: agent_texture.normal_map_texture,
        metallic_roughness_texture: agent_texture.metallic_roughness_texture,
        emissive_texture: agent_texture.emissive_texture,
        perceptual_roughness: agent_roughness,
        metallic: agent_metallic,
        emissive: emissive_from_srgb_with_boost(
            agent_emissive_color,
            config.materials.agent.emissive_boost,
        ),
        ..default()
    };

    let asset_base_color =
        resolve_srgb_slot_color([0.82, 0.76, 0.34], external_material.asset.base_color_srgb);
    let asset_emissive_color = resolve_srgb_slot_color(
        [0.82, 0.76, 0.34],
        external_material.asset.emissive_color_srgb,
    );
    let asset_material = StandardMaterial {
        base_color: color_from_srgb(asset_base_color),
        base_color_texture: asset_texture.base_color_texture,
        normal_map_texture: asset_texture.normal_map_texture,
        metallic_roughness_texture: asset_texture.metallic_roughness_texture,
        emissive_texture: asset_texture.emissive_texture,
        perceptual_roughness: asset_roughness,
        metallic: asset_metallic,
        emissive: emissive_from_srgb_with_boost(
            asset_emissive_color,
            config.materials.asset.emissive_boost,
        ),
        ..default()
    };

    let power_plant_base_color = resolve_srgb_slot_color(
        [0.95, 0.42, 0.20],
        external_material.power_plant.base_color_srgb,
    );
    let power_plant_emissive_color = resolve_srgb_slot_color(
        [0.95, 0.42, 0.20],
        external_material.power_plant.emissive_color_srgb,
    );
    let power_plant_material = StandardMaterial {
        base_color: color_from_srgb(power_plant_base_color),
        base_color_texture: power_plant_texture.base_color_texture,
        normal_map_texture: power_plant_texture.normal_map_texture,
        metallic_roughness_texture: power_plant_texture.metallic_roughness_texture,
        emissive_texture: power_plant_texture.emissive_texture,
        perceptual_roughness: power_plant_roughness,
        metallic: power_plant_metallic,
        emissive: emissive_from_srgb_with_boost(
            power_plant_emissive_color,
            config.materials.power_plant.emissive_boost,
        ),
        ..default()
    };

    let location_override_materials = if location_style_override_enabled(
        external_material.location,
        &external_texture.location,
    ) {
        let location_base_color = resolve_srgb_slot_color(
            [0.30, 0.42, 0.66],
            external_material.location.base_color_srgb,
        );
        let location_emissive_color = resolve_srgb_slot_color(
            location_base_color,
            external_material.location.emissive_color_srgb,
        );
        let core_textures = location_texture.clone();
        let core_template = |alpha: f32| StandardMaterial {
            base_color: color_from_srgb_with_alpha(location_base_color, alpha),
            base_color_texture: core_textures.base_color_texture.clone(),
            normal_map_texture: core_textures.normal_map_texture.clone(),
            metallic_roughness_texture: core_textures.metallic_roughness_texture.clone(),
            emissive_texture: core_textures.emissive_texture.clone(),
            perceptual_roughness: facility_roughness,
            metallic: facility_metallic,
            emissive: color_from_srgb(location_emissive_color).into(),
            alpha_mode: AlphaMode::Blend,
            ..default()
        };
        Some(LocationOverrideMaterialTemplates {
            core_silicate: core_template(0.22),
            core_metal: core_template(0.30),
            core_ice: core_template(0.30),
            halo: StandardMaterial {
                base_color: color_from_srgb_with_alpha(location_base_color, 0.10),
                base_color_texture: location_texture.base_color_texture,
                normal_map_texture: location_texture.normal_map_texture,
                metallic_roughness_texture: location_texture.metallic_roughness_texture,
                emissive_texture: location_texture.emissive_texture,
                emissive: color_from_srgb(location_emissive_color).into(),
                alpha_mode: AlphaMode::Blend,
                ..default()
            },
        })
    } else {
        None
    };

    ResolvedThemeSceneAssets {
        agent_mesh,
        location_mesh,
        asset_mesh,
        power_plant_mesh,
        agent_material,
        asset_material,
        power_plant_material,
        location_override_materials,
    }
}

fn texture_slot_override_enabled(slot: &ViewerExternalTextureSlotConfig) -> bool {
    slot.base_texture_asset.is_some()
        || slot.normal_texture_asset.is_some()
        || slot.metallic_roughness_texture_asset.is_some()
        || slot.emissive_texture_asset.is_some()
}

#[derive(Clone, Copy, Debug)]
struct MaterialVariantScalars {
    roughness_scale: f32,
    metallic_scale: f32,
}

fn parse_material_variant_preset(raw: &str) -> Option<ViewerMaterialVariantPreset> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "default" | "base" | "balanced" => Some(ViewerMaterialVariantPreset::Default),
        "matte" | "flat" => Some(ViewerMaterialVariantPreset::Matte),
        "glossy" | "shine" => Some(ViewerMaterialVariantPreset::Glossy),
        _ => None,
    }
}

fn resolve_material_variant_preview_state() -> MaterialVariantPreviewState {
    resolve_material_variant_preview_state_from(|key| std::env::var(key).ok())
}

fn resolve_material_variant_preview_state_from<F>(lookup: F) -> MaterialVariantPreviewState
where
    F: Fn(&str) -> Option<String>,
{
    let active =
        viewer_env::resolve_viewer_env_with(&lookup, "OASIS7_VIEWER_MATERIAL_VARIANT_PRESET")
            .as_deref()
            .and_then(parse_material_variant_preset)
            .unwrap_or_default();
    MaterialVariantPreviewState { active }
}

fn material_variant_scalars(preset: ViewerMaterialVariantPreset) -> MaterialVariantScalars {
    match preset {
        ViewerMaterialVariantPreset::Default => MaterialVariantScalars {
            roughness_scale: 1.0,
            metallic_scale: 1.0,
        },
        ViewerMaterialVariantPreset::Matte => MaterialVariantScalars {
            roughness_scale: 1.35,
            metallic_scale: 0.65,
        },
        ViewerMaterialVariantPreset::Glossy => MaterialVariantScalars {
            roughness_scale: 0.65,
            metallic_scale: 1.35,
        },
    }
}

fn apply_material_variant_scalar(base: f32, scale: f32) -> f32 {
    (base * scale).clamp(0.0, 1.0)
}

fn apply_material_variant_to_material(
    materials: &mut Assets<StandardMaterial>,
    handle: &Handle<StandardMaterial>,
    base_roughness: f32,
    base_metallic: f32,
    preset: ViewerMaterialVariantPreset,
) {
    let scalars = material_variant_scalars(preset);
    let Some(material) = materials.get_mut(handle) else {
        return;
    };
    material.perceptual_roughness =
        apply_material_variant_scalar(base_roughness, scalars.roughness_scale);
    material.metallic = apply_material_variant_scalar(base_metallic, scalars.metallic_scale);
}

fn apply_material_variant_to_scene_materials(
    materials: &mut Assets<StandardMaterial>,
    assets: &Viewer3dAssets,
    config: &Viewer3dConfig,
    preset: ViewerMaterialVariantPreset,
) {
    apply_material_variant_to_material(
        materials,
        &assets.agent_material,
        config.materials.agent.roughness,
        config.materials.agent.metallic,
        preset,
    );
    apply_material_variant_to_material(
        materials,
        &assets.asset_material,
        config.materials.asset.roughness,
        config.materials.asset.metallic,
        preset,
    );
    apply_material_variant_to_material(
        materials,
        &assets.power_plant_material,
        config.materials.power_plant.roughness,
        config.materials.power_plant.metallic,
        preset,
    );
}

fn color_from_srgb(rgb: [f32; 3]) -> Color {
    Color::srgb(rgb[0], rgb[1], rgb[2])
}

fn color_from_srgb_with_alpha(rgb: [f32; 3], alpha: f32) -> Color {
    Color::srgba(rgb[0], rgb[1], rgb[2], alpha)
}

fn emissive_from_srgb_with_boost(rgb: [f32; 3], boost: f32) -> LinearRgba {
    Color::srgb(
        (rgb[0] * boost).clamp(0.0, MAX_EMISSIVE_COLOR_COMPONENT),
        (rgb[1] * boost).clamp(0.0, MAX_EMISSIVE_COLOR_COMPONENT),
        (rgb[2] * boost).clamp(0.0, MAX_EMISSIVE_COLOR_COMPONENT),
    )
    .into()
}

fn location_material_override_enabled(slot: ViewerExternalMaterialSlotConfig) -> bool {
    slot.base_color_srgb.is_some() || slot.emissive_color_srgb.is_some()
}

fn location_style_override_enabled(
    material_slot: ViewerExternalMaterialSlotConfig,
    texture_slot: &ViewerExternalTextureSlotConfig,
) -> bool {
    location_material_override_enabled(material_slot) || texture_slot_override_enabled(texture_slot)
}

fn lighting_illuminance_triplet(config: &Viewer3dConfig) -> (f32, f32, f32) {
    let key = config.physical.exposed_illuminance_lux();
    let fill = (key * config.lighting.fill_light_ratio.max(0.0)).max(800.0);
    let rim = (key * config.lighting.rim_light_ratio.max(0.0)).max(450.0);
    (key, fill, rim)
}

fn resolve_tonemapping(mode: ViewerTonemappingMode) -> Tonemapping {
    match mode {
        ViewerTonemappingMode::None => Tonemapping::None,
        ViewerTonemappingMode::Reinhard => Tonemapping::Reinhard,
        ViewerTonemappingMode::ReinhardLuminance => Tonemapping::ReinhardLuminance,
        ViewerTonemappingMode::AcesFitted => Tonemapping::AcesFitted,
        ViewerTonemappingMode::AgX => Tonemapping::AgX,
        ViewerTonemappingMode::SomewhatBoringDisplayTransform => {
            Tonemapping::SomewhatBoringDisplayTransform
        }
        ViewerTonemappingMode::TonyMcMapface => Tonemapping::TonyMcMapface,
        ViewerTonemappingMode::BlenderFilmic => Tonemapping::BlenderFilmic,
    }
}

fn build_color_grading(config: &Viewer3dConfig) -> ColorGrading {
    let mut grading = ColorGrading::default();
    grading.global = ColorGradingGlobal {
        exposure: config.post_process.color_grading_exposure,
        post_saturation: config.post_process.color_grading_post_saturation,
        ..default()
    };
    grading
}

fn build_bloom(config: &Viewer3dConfig) -> Option<Bloom> {
    if !config.post_process.bloom_enabled {
        return None;
    }
    let mut bloom = Bloom::NATURAL;
    bloom.intensity = config.post_process.bloom_intensity.max(0.0);
    Some(bloom)
}

fn camera_post_process_components(
    config: &Viewer3dConfig,
) -> (Tonemapping, DebandDither, ColorGrading, Option<Bloom>) {
    let tonemapping = resolve_tonemapping(config.post_process.tonemapping);
    let deband_dither = if config.post_process.deband_dither_enabled {
        DebandDither::Enabled
    } else {
        DebandDither::Disabled
    };
    let color_grading = build_color_grading(config);
    let bloom = build_bloom(config);
    (tonemapping, deband_dither, color_grading, bloom)
}

#[derive(Component, Copy, Clone)]
struct BaseScale(Vec3);

#[cfg(test)]
#[path = "tests_camera_mode.rs"]
mod camera_mode_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_ui_text;
