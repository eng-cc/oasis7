use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::mem;

use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowPlugin};
use js_sys::Function;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

thread_local! {
    static BRIDGE_SHARED: RefCell<BridgeSharedState> = RefCell::new(BridgeSharedState::default());
}

const LOCATION_HIT_HALF_SIZE: f64 = 8.0;
const AGENT_HIT_HALF_SIZE: f64 = 8.0;

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
}

#[derive(Clone, Debug, Deserialize)]
struct Agent {
    id: String,
    #[allow(dead_code)]
    label: String,
    pos: Option<Position>,
}

#[derive(Clone, Debug, Deserialize)]
struct Selection {
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
    agents: Vec<Agent>,
    selection: Option<Selection>,
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

#[derive(Clone, Debug)]
struct HitRegion {
    kind: &'static str,
    id: String,
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

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
    input_events: Vec<InputEvent>,
    on_event: Option<Function>,
    on_fatal: Option<Function>,
}

#[derive(Clone, Debug, PartialEq)]
struct GridLayoutKey {
    width: i32,
    height: i32,
    step_milli: i32,
    offset_x_milli: i32,
    offset_y_milli: i32,
}

#[derive(Resource, Default)]
struct BevyRuntimeState {
    mounted: bool,
    render_state: Option<RenderState>,
    render_version: u64,
    camera: CameraState,
    drag_state: Option<DragState>,
    hit_regions: Vec<HitRegion>,
    hover_key: Option<String>,
    grid_layout: Option<GridLayoutKey>,
    location_entities: HashMap<String, Entity>,
    agent_entities: HashMap<String, Entity>,
}

#[derive(Component)]
struct PixelWorldGridVisual;

#[derive(Component)]
struct PixelWorldLocationVisual {
    id: String,
}

#[derive(Component)]
struct PixelWorldAgentVisual {
    id: String,
}

#[derive(Default)]
struct SharedSnapshot {
    mounted: bool,
    render_state: Option<RenderState>,
    render_version: u64,
    input_events: Vec<InputEvent>,
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
    to_value(value)
        .map_err(|error| JsValue::from_str(&format!("serialize js payload failed: {error}")))
}

fn status_value(status: &str) -> JsValue {
    js_value_from_serializable(&json!({ "status": status })).unwrap_or_else(|_| JsValue::NULL)
}

fn parse_render_state(raw: JsValue) -> Result<RenderState, JsValue> {
    from_value(raw)
        .map_err(|error| JsValue::from_str(&format!("render state parse failed: {error}")))
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

fn emit_event_value(value: &Value) -> Result<(), JsValue> {
    let payload = js_value_from_serializable(value)?;
    BRIDGE_SHARED.with(|shared| {
        shared
            .borrow()
            .on_event
            .as_ref()
            .ok_or_else(|| JsValue::from_str("event callback missing"))?
            .call1(&JsValue::NULL, &payload)
            .map(|_| ())
            .map_err(|_| JsValue::from_str("event callback failed"))
    })
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
        BRIDGE_SHARED.with(|shared| {
            if let Some(on_fatal) = shared.borrow().on_fatal.as_ref() {
                let _ = on_fatal.call1(&JsValue::NULL, &js_payload);
            }
        });
    }
    js_value_from_serializable(&json!({ "status": "fallback", "fatal": payload }))
        .unwrap_or_else(|_| status_value("fallback"))
}

fn shared_snapshot() -> SharedSnapshot {
    BRIDGE_SHARED.with(|shared| {
        let mut shared = shared.borrow_mut();
        SharedSnapshot {
            mounted: shared.mounted,
            render_state: shared.render_state.clone(),
            render_version: shared.render_version,
            input_events: mem::take(&mut shared.input_events),
        }
    })
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
    app.add_systems(Startup, setup_scene);
    app.add_systems(Update, (sync_external_state, render_scene));
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

fn sync_external_state(mut runtime: ResMut<BevyRuntimeState>) {
    let snapshot = shared_snapshot();
    runtime.mounted = snapshot.mounted;
    if snapshot.render_version != runtime.render_version {
        runtime.render_version = snapshot.render_version;
        runtime.render_state = snapshot.render_state;
    }

    for event in snapshot.input_events {
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
                    let _ = emit_camera_state(&runtime.camera);
                    continue;
                }

                if is_leave {
                    if runtime.hover_key.take().is_some() {
                        let _ = emit_event_value(
                            &json!({ "type": "hover_entity", "selection": Value::Null }),
                        );
                    }
                    continue;
                }

                let hit = hit_test(&runtime.hit_regions, x, y);
                let hover_key = hit.as_ref().map(|(kind, id)| format!("{kind}/{id}"));
                if hover_key == runtime.hover_key {
                    continue;
                }
                runtime.hover_key = hover_key;
                let selection = hit
                    .map(|(kind, id)| json!({ "kind": kind, "id": id }))
                    .unwrap_or(Value::Null);
                let _ =
                    emit_event_value(&json!({ "type": "hover_entity", "selection": selection }));
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
                let _ = emit_camera_state(&runtime.camera);
            }
            InputEvent::Click { x, y } => {
                if let Some((kind, id)) = hit_test(&runtime.hit_regions, x, y) {
                    let _ = emit_event_value(&json!({
                        "type": "select_entity",
                        "selection": { "kind": kind, "id": id }
                    }));
                }
            }
        }
    }
}

fn build_grid_layout(camera: &CameraState, width: f64, height: f64) -> GridLayoutKey {
    let grid_step = clamp(24.0 * camera.zoom.max(0.5), 12.0, 72.0);
    let offset_x = ((camera.pan_x_px % grid_step) + grid_step) % grid_step;
    let offset_y = ((camera.pan_y_px % grid_step) + grid_step) % grid_step;
    GridLayoutKey {
        width: width.round() as i32,
        height: height.round() as i32,
        step_milli: (grid_step * 1000.0).round() as i32,
        offset_x_milli: (offset_x * 1000.0).round() as i32,
        offset_y_milli: (offset_y * 1000.0).round() as i32,
    }
}

fn grid_geometry(layout: &GridLayoutKey) -> (f64, f64, f64, f64, Color) {
    (
        layout.step_milli as f64 / 1000.0,
        layout.offset_x_milli as f64 / 1000.0,
        layout.offset_y_milli as f64 / 1000.0,
        layout.width as f64,
        Color::srgba_u8(99, 179, 255, 26),
    )
}

fn reconcile_grid(
    commands: &mut Commands,
    runtime: &mut BevyRuntimeState,
    existing_grid: &Query<Entity, With<PixelWorldGridVisual>>,
    width: f64,
    height: f64,
) {
    let next_layout = build_grid_layout(&runtime.camera, width, height);
    if runtime.grid_layout.as_ref() == Some(&next_layout) {
        return;
    }

    for entity in existing_grid.iter() {
        commands.entity(entity).despawn();
    }

    let (grid_step, offset_x, offset_y, layout_width, grid_color) = grid_geometry(&next_layout);
    let layout_height = next_layout.height as f64;

    let mut x = offset_x;
    while x <= layout_width {
        commands.spawn((
            sprite_for_rect(grid_color, 1.0, layout_height as f32),
            Transform::from_translation(to_bevy_translation(
                x,
                layout_height / 2.0,
                layout_width,
                layout_height,
                0.0,
            )),
            PixelWorldGridVisual,
        ));
        x += grid_step;
    }

    let mut y = offset_y;
    while y <= layout_height {
        commands.spawn((
            sprite_for_rect(grid_color, layout_width as f32, 1.0),
            Transform::from_translation(to_bevy_translation(
                layout_width / 2.0,
                y,
                layout_width,
                layout_height,
                0.0,
            )),
            PixelWorldGridVisual,
        ));
        y += grid_step;
    }

    runtime.grid_layout = Some(next_layout);
}

fn reconcile_locations(
    commands: &mut Commands,
    runtime: &mut BevyRuntimeState,
    width: f64,
    height: f64,
    animation_ms: f64,
) {
    let mut active_ids = HashSet::new();
    let Some(render_state) = runtime.render_state.as_ref() else {
        return;
    };
    let Some(world_bounds) = render_state.world_bounds.as_ref() else {
        for (_, entity) in runtime.location_entities.drain() {
            commands.entity(entity).despawn();
        }
        return;
    };

    for location in &render_state.locations {
        let Some((canvas_x, canvas_y)) =
            to_canvas_point(&location.pos, world_bounds, width, height, &runtime.camera)
        else {
            continue;
        };
        active_ids.insert(location.id.clone());
        let pulse = 1.0 + (0.08 * ((animation_ms / 360.0) + location.id.len() as f64).sin());
        let size = 16.0 * pulse;
        let transform = Transform::from_translation(to_bevy_translation(
            canvas_x, canvas_y, width, height, 1.0,
        ));
        let sprite = sprite_for_square(Color::srgba_u8(110, 231, 183, 184), size as f32);

        if let Some(entity) = runtime.location_entities.get(&location.id).copied() {
            commands.entity(entity).insert((sprite, transform));
        } else {
            let entity = commands
                .spawn((
                    sprite,
                    transform,
                    PixelWorldLocationVisual {
                        id: location.id.clone(),
                    },
                ))
                .id();
            runtime
                .location_entities
                .insert(location.id.clone(), entity);
        }

        runtime.hit_regions.push(HitRegion {
            kind: "location",
            id: location.id.clone(),
            left: canvas_x - LOCATION_HIT_HALF_SIZE,
            top: canvas_y - LOCATION_HIT_HALF_SIZE,
            right: canvas_x + LOCATION_HIT_HALF_SIZE,
            bottom: canvas_y + LOCATION_HIT_HALF_SIZE,
        });
    }

    let stale_ids: Vec<String> = runtime
        .location_entities
        .keys()
        .filter(|id| !active_ids.contains(*id))
        .cloned()
        .collect();
    for id in stale_ids {
        if let Some(entity) = runtime.location_entities.remove(&id) {
            commands.entity(entity).despawn();
        }
    }
}

fn reconcile_agents(
    commands: &mut Commands,
    runtime: &mut BevyRuntimeState,
    width: f64,
    height: f64,
    animation_ms: f64,
) {
    let Some(render_state) = runtime.render_state.as_ref() else {
        for (_, entity) in runtime.agent_entities.drain() {
            commands.entity(entity).despawn();
        }
        return;
    };

    let mut active_ids = HashSet::new();
    for (index, agent) in render_state.agents.iter().enumerate() {
        active_ids.insert(agent.id.clone());
        let (canvas_x, canvas_y) = render_state
            .world_bounds
            .as_ref()
            .and_then(|world_bounds| {
                agent.pos.as_ref().and_then(|pos| {
                    to_canvas_point(pos, world_bounds, width, height, &runtime.camera)
                })
            })
            .unwrap_or_else(|| {
                fallback_point_for_entity(&agent.id, width, height, &runtime.camera)
            });
        let is_selected = render_state
            .selection
            .as_ref()
            .map(|selection| selection.kind == "agent" && selection.id == agent.id)
            .unwrap_or(false);
        let pulse = 1.0 + (0.12 * ((animation_ms / 240.0) + index as f64).sin());
        let size = if is_selected { 15.0 } else { 12.0 } * pulse;
        let color = if is_selected {
            Color::srgb_u8(251, 191, 36)
        } else {
            Color::srgb_u8(99, 179, 255)
        };
        let transform = Transform::from_translation(to_bevy_translation(
            canvas_x, canvas_y, width, height, 2.0,
        ));
        let sprite = sprite_for_square(color, size as f32);

        if let Some(entity) = runtime.agent_entities.get(&agent.id).copied() {
            commands.entity(entity).insert((sprite, transform));
        } else {
            let entity = commands
                .spawn((
                    sprite,
                    transform,
                    PixelWorldAgentVisual {
                        id: agent.id.clone(),
                    },
                ))
                .id();
            runtime.agent_entities.insert(agent.id.clone(), entity);
        }

        runtime.hit_regions.push(HitRegion {
            kind: "agent",
            id: agent.id.clone(),
            left: canvas_x - AGENT_HIT_HALF_SIZE,
            top: canvas_y - AGENT_HIT_HALF_SIZE,
            right: canvas_x + AGENT_HIT_HALF_SIZE,
            bottom: canvas_y + AGENT_HIT_HALF_SIZE,
        });
    }

    let stale_ids: Vec<String> = runtime
        .agent_entities
        .keys()
        .filter(|id| !active_ids.contains(*id))
        .cloned()
        .collect();
    for id in stale_ids {
        if let Some(entity) = runtime.agent_entities.remove(&id) {
            commands.entity(entity).despawn();
        }
    }
}

fn clear_runtime_visuals(commands: &mut Commands, runtime: &mut BevyRuntimeState) {
    for (_, entity) in runtime.location_entities.drain() {
        commands.entity(entity).despawn();
    }
    for (_, entity) in runtime.agent_entities.drain() {
        commands.entity(entity).despawn();
    }
    runtime.grid_layout = None;
    runtime.hit_regions.clear();
    runtime.hover_key = None;
}

fn render_scene(
    mut commands: Commands,
    mut runtime: ResMut<BevyRuntimeState>,
    windows: Query<&Window, With<PrimaryWindow>>,
    current_grid: Query<Entity, With<PixelWorldGridVisual>>,
    location_visuals: Query<(Entity, &PixelWorldLocationVisual)>,
    agent_visuals: Query<(Entity, &PixelWorldAgentVisual)>,
    time: Res<Time>,
) {
    if !runtime.mounted {
        clear_runtime_visuals(&mut commands, &mut runtime);
        for entity in current_grid.iter() {
            commands.entity(entity).despawn();
        }
        return;
    }

    for (entity, visual) in location_visuals.iter() {
        runtime
            .location_entities
            .entry(visual.id.clone())
            .or_insert(entity);
    }
    for (entity, visual) in agent_visuals.iter() {
        runtime
            .agent_entities
            .entry(visual.id.clone())
            .or_insert(entity);
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Some(_) = runtime.render_state.as_ref() else {
        clear_runtime_visuals(&mut commands, &mut runtime);
        for entity in current_grid.iter() {
            commands.entity(entity).despawn();
        }
        return;
    };

    let width = window.width() as f64;
    let height = window.height() as f64;
    let animation_ms = time.elapsed_secs_f64() * 1000.0;
    runtime.hit_regions.clear();

    reconcile_grid(&mut commands, &mut runtime, &current_grid, width, height);
    reconcile_locations(&mut commands, &mut runtime, width, height, animation_ms);
    reconcile_agents(&mut commands, &mut runtime, width, height, animation_ms);
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
            if let Some(existing_selector) = &shared.canvas_selector {
                if existing_selector != &canvas_selector {
                    return Err(format!(
                        "bevy runtime already bound to {existing_selector}, cannot rebind to {canvas_selector}"
                    ));
                }
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
            status_value("ready")
        } else {
            status_value("detached")
        }
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
        });
        status_value("detached")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::wasm_bindgen_test;

    fn assert_grid_layout_is_stable_for_same_camera_and_size() {
        let camera = CameraState::default();
        let left = build_grid_layout(&camera, 960.0, 540.0);
        let right = build_grid_layout(&camera, 960.0, 540.0);
        assert_eq!(left, right);
    }

    fn assert_grid_layout_changes_when_camera_pan_changes() {
        let mut camera = CameraState::default();
        let before = build_grid_layout(&camera, 960.0, 540.0);
        camera.pan_x_px = 10.0;
        let after = build_grid_layout(&camera, 960.0, 540.0);
        assert_ne!(before, after);
    }

    fn assert_fallback_point_stays_within_canvas() {
        let point = fallback_point_for_entity("agent-0", 960.0, 540.0, &CameraState::default());
        assert!(point.0 >= 0.0 && point.0 <= 960.0);
        assert!(point.1 >= 0.0 && point.1 <= 540.0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn grid_layout_is_stable_for_same_camera_and_size() {
        assert_grid_layout_is_stable_for_same_camera_and_size();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn grid_layout_changes_when_camera_pan_changes() {
        assert_grid_layout_changes_when_camera_pan_changes();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn fallback_point_stays_within_canvas() {
        assert_fallback_point_stays_within_canvas();
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn wasm_grid_layout_is_stable_for_same_camera_and_size() {
        assert_grid_layout_is_stable_for_same_camera_and_size();
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn wasm_grid_layout_changes_when_camera_pan_changes() {
        assert_grid_layout_changes_when_camera_pan_changes();
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    fn wasm_fallback_point_stays_within_canvas() {
        assert_fallback_point_stays_within_canvas();
    }
}
