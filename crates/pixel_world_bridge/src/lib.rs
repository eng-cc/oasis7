use js_sys::{Function, JSON};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement};

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

#[wasm_bindgen]
pub struct PixelWorldBridge {
    mounted: bool,
    canvas: Option<HtmlCanvasElement>,
    context: Option<CanvasRenderingContext2d>,
    render_state: Option<RenderState>,
    camera: CameraState,
    drag_state: Option<DragState>,
    hit_regions: Vec<HitRegion>,
    last_hover_key: Option<String>,
    last_animation_ms: f64,
    on_event: Function,
    on_fatal: Function,
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.min(max).max(min)
}

fn js_value_from_json_value(value: &Value) -> Result<JsValue, JsValue> {
    let encoded = serde_json::to_string(value)
        .map_err(|error| JsValue::from_str(&format!("serialize json failed: {error}")))?;
    JSON::parse(&encoded)
}

fn status_value(status: &str) -> JsValue {
    js_value_from_json_value(&json!({ "status": status })).unwrap_or_else(|_| JsValue::NULL)
}

fn parse_render_state(raw: JsValue) -> Result<RenderState, JsValue> {
    let encoded = JSON::stringify(&raw)
        .map_err(|_| JsValue::from_str("render state stringify failed"))?
        .as_string()
        .ok_or_else(|| JsValue::from_str("render state stringify produced non-string"))?;
    serde_json::from_str::<RenderState>(&encoded)
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

#[wasm_bindgen]
impl PixelWorldBridge {
    #[wasm_bindgen(constructor)]
    pub fn new(on_event: Function, on_fatal: Function) -> Self {
        Self {
            mounted: false,
            canvas: None,
            context: None,
            render_state: None,
            camera: CameraState {
                zoom: 1.0,
                pan_x_px: 0.0,
                pan_y_px: 0.0,
            },
            drag_state: None,
            hit_regions: Vec::new(),
            last_hover_key: None,
            last_animation_ms: 0.0,
            on_event,
            on_fatal,
        }
    }

    #[wasm_bindgen]
    pub fn mount(&mut self, canvas: HtmlCanvasElement, initial_render_state: JsValue) -> JsValue {
        let context = match canvas
            .get_context("2d")
            .ok()
            .flatten()
            .and_then(|value| value.dyn_into::<CanvasRenderingContext2d>().ok())
        {
            Some(context) => context,
            None => return self.emit_fatal("2d canvas context unavailable"),
        };

        let parsed_state = match parse_render_state(initial_render_state) {
            Ok(state) => state,
            Err(error) => return self.emit_fatal_js(error),
        };

        self.canvas = Some(canvas);
        self.context = Some(context);
        self.render_state = Some(parsed_state);
        self.camera = CameraState {
            zoom: 1.0,
            pan_x_px: 0.0,
            pan_y_px: 0.0,
        };
        self.drag_state = None;
        self.hit_regions.clear();
        self.last_hover_key = None;
        self.last_animation_ms = 0.0;
        self.mounted = true;

        if let Err(error) = self.render_current_frame() {
            return self.emit_fatal_js(error);
        }

        let _ = self.emit_event_value(&json!({ "type": "canvas_ready" }));
        let _ = self.emit_camera_state();
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn update(&mut self, next_render_state: JsValue) -> JsValue {
        if !self.mounted {
            return status_value("detached");
        }
        self.render_state = match parse_render_state(next_render_state) {
            Ok(state) => Some(state),
            Err(error) => return self.emit_fatal_js(error),
        };
        if let Err(error) = self.render_current_frame() {
            return self.emit_fatal_js(error);
        }
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn tick(&mut self, animation_ms: f64) -> JsValue {
        if !self.mounted {
            return status_value("detached");
        }
        self.last_animation_ms = animation_ms;
        if let Err(error) = self.render_current_frame() {
            return self.emit_fatal_js(error);
        }
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn pointer_down(&mut self, x: f64, y: f64, pointer_id: i32) -> JsValue {
        self.drag_state = Some(DragState {
            pointer_id,
            start_x: x,
            start_y: y,
            start_pan_x: self.camera.pan_x_px,
            start_pan_y: self.camera.pan_y_px,
        });
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn pointer_move(&mut self, x: f64, y: f64, is_leave: bool, pointer_id: i32) -> JsValue {
        if let Some(drag_state) = &self.drag_state {
            if drag_state.pointer_id == pointer_id {
                self.camera.pan_x_px = drag_state.start_pan_x + (x - drag_state.start_x);
                self.camera.pan_y_px = drag_state.start_pan_y + (y - drag_state.start_y);
                if let Err(error) = self.render_current_frame() {
                    return self.emit_fatal_js(error);
                }
                let _ = self.emit_camera_state();
                return status_value("ready");
            }
        }

        if is_leave {
            if self.last_hover_key.take().is_some() {
                let _ = self
                    .emit_event_value(&json!({ "type": "hover_entity", "selection": Value::Null }));
            }
            return status_value("ready");
        }

        let hit = self.hit_test(x, y);
        let hover_key = hit.as_ref().map(|(kind, id)| format!("{kind}/{id}"));
        if hover_key == self.last_hover_key {
            return status_value("ready");
        }
        self.last_hover_key = hover_key;
        let selection = hit
            .map(|(kind, id)| json!({ "kind": kind, "id": id }))
            .unwrap_or(Value::Null);
        let _ = self.emit_event_value(&json!({ "type": "hover_entity", "selection": selection }));
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn pointer_up(&mut self, pointer_id: i32) -> JsValue {
        if self
            .drag_state
            .as_ref()
            .map(|drag_state| drag_state.pointer_id == pointer_id)
            .unwrap_or(false)
        {
            self.drag_state = None;
        }
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn wheel(&mut self, delta_y: f64) -> JsValue {
        let factor = if delta_y < 0.0 { 1.12 } else { 0.89 };
        self.camera.zoom = clamp(self.camera.zoom * factor, 0.6, 3.5);
        if let Err(error) = self.render_current_frame() {
            return self.emit_fatal_js(error);
        }
        let _ = self.emit_camera_state();
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn click(&mut self, x: f64, y: f64) -> JsValue {
        if let Some((kind, id)) = self.hit_test(x, y) {
            let _ = self.emit_event_value(&json!({
                "type": "select_entity",
                "selection": { "kind": kind, "id": id }
            }));
        }
        status_value("ready")
    }

    #[wasm_bindgen]
    pub fn unmount(&mut self) -> JsValue {
        self.mounted = false;
        self.canvas = None;
        self.context = None;
        self.render_state = None;
        self.hit_regions.clear();
        self.drag_state = None;
        self.last_hover_key = None;
        status_value("detached")
    }
}

impl PixelWorldBridge {
    fn emit_fatal(&self, message: &str) -> JsValue {
        self.emit_fatal_js(JsValue::from_str(message))
    }

    fn emit_fatal_js(&self, error: JsValue) -> JsValue {
        let message = error
            .as_string()
            .unwrap_or_else(|| "pixel world renderer fatal".to_string());
        let payload = json!({
            "code": "pixel_world_renderer_fatal",
            "message": message,
        });
        if let Ok(js_payload) = js_value_from_json_value(&payload) {
            let _ = self.on_fatal.call1(&JsValue::NULL, &js_payload);
            return json!({ "status": "fallback", "fatal": payload })
                .to_string()
                .parse::<String>()
                .ok()
                .and_then(|encoded| JSON::parse(&encoded).ok())
                .unwrap_or_else(|| status_value("fallback"));
        }
        status_value("fallback")
    }

    fn emit_event_value(&self, value: &Value) -> Result<(), JsValue> {
        let payload = js_value_from_json_value(value)?;
        self.on_event
            .call1(&JsValue::NULL, &payload)
            .map(|_| ())
            .map_err(|_| JsValue::from_str("event callback failed"))
    }

    fn emit_camera_state(&self) -> Result<(), JsValue> {
        self.emit_event_value(&json!({
            "type": "camera_state_changed",
            "camera": CameraStatePayload {
                zoom: (self.camera.zoom * 1000.0).round() / 1000.0,
                pan_x_px: self.camera.pan_x_px.round() as i32,
                pan_y_px: self.camera.pan_y_px.round() as i32,
            }
        }))
    }

    fn hit_test(&self, x: f64, y: f64) -> Option<(String, String)> {
        for region in self.hit_regions.iter().rev() {
            if x >= region.left && x <= region.right && y >= region.top && y <= region.bottom {
                return Some((region.kind.to_string(), region.id.clone()));
            }
        }
        None
    }

    #[allow(deprecated)]
    fn render_current_frame(&mut self) -> Result<(), JsValue> {
        let canvas = self
            .canvas
            .as_ref()
            .ok_or_else(|| JsValue::from_str("canvas missing during render"))?;
        let context = self
            .context
            .as_ref()
            .ok_or_else(|| JsValue::from_str("2d context missing during render"))?;
        let render_state = self
            .render_state
            .as_ref()
            .ok_or_else(|| JsValue::from_str("render state missing during render"))?;
        let width = canvas.width() as f64;
        let height = canvas.height() as f64;

        context.clear_rect(0.0, 0.0, width, height);
        context.set_fill_style(&JsValue::from_str("#0a121a"));
        context.fill_rect(0.0, 0.0, width, height);

        self.draw_grid(context, width, height);
        self.hit_regions.clear();

        if let Some(world_bounds) = &render_state.world_bounds {
            for location in &render_state.locations {
                if let Some((x, y)) =
                    to_canvas_point(&location.pos, world_bounds, width, height, &self.camera)
                {
                    let pulse = 1.0
                        + (0.08
                            * ((self.last_animation_ms / 360.0) + location.id.len() as f64).sin());
                    let size = 16.0 * pulse;
                    context.set_fill_style(&JsValue::from_str("rgba(110, 231, 183, 0.72)"));
                    context.fill_rect(x - (size / 2.0), y - (size / 2.0), size, size);
                    context.set_stroke_style(&JsValue::from_str("rgba(110, 231, 183, 0.95)"));
                    context.stroke_rect(x - (size / 2.0), y - (size / 2.0), size, size);
                    self.hit_regions.push(HitRegion {
                        kind: "location",
                        id: location.id.clone(),
                        left: x - 8.0,
                        top: y - 8.0,
                        right: x + 8.0,
                        bottom: y + 8.0,
                    });
                }
            }
        }

        for (index, agent) in render_state.agents.iter().enumerate() {
            let point = render_state
                .world_bounds
                .as_ref()
                .and_then(|world_bounds| {
                    agent.pos.as_ref().and_then(|pos| {
                        to_canvas_point(pos, world_bounds, width, height, &self.camera)
                    })
                })
                .unwrap_or_else(|| {
                    fallback_point_for_entity(&agent.id, width, height, &self.camera)
                });
            let is_selected = render_state
                .selection
                .as_ref()
                .map(|selection| selection.kind == "agent" && selection.id == agent.id)
                .unwrap_or(false);
            let pulse = 1.0 + (0.12 * ((self.last_animation_ms / 240.0) + index as f64).sin());
            let size = if is_selected { 15.0 } else { 12.0 } * pulse;
            context.set_fill_style(&JsValue::from_str(if is_selected {
                "#fbbf24"
            } else {
                "#63b3ff"
            }));
            context.fill_rect(point.0 - (size / 2.0), point.1 - (size / 2.0), size, size);
            context.set_stroke_style(&JsValue::from_str(if is_selected {
                "#fde68a"
            } else {
                "#c6e4ff"
            }));
            context.set_line_width(2.0);
            context.stroke_rect(point.0 - (size / 2.0), point.1 - (size / 2.0), size, size);
            self.hit_regions.push(HitRegion {
                kind: "agent",
                id: agent.id.clone(),
                left: point.0 - 8.0,
                top: point.1 - 8.0,
                right: point.0 + 8.0,
                bottom: point.1 + 8.0,
            });
        }

        Ok(())
    }

    #[allow(deprecated)]
    fn draw_grid(&self, context: &CanvasRenderingContext2d, width: f64, height: f64) {
        let grid_step = clamp(24.0 * self.camera.zoom.max(0.5), 12.0, 72.0);
        let offset_x = ((self.camera.pan_x_px % grid_step) + grid_step) % grid_step;
        let offset_y = ((self.camera.pan_y_px % grid_step) + grid_step) % grid_step;
        context.set_stroke_style(&JsValue::from_str("rgba(99, 179, 255, 0.10)"));
        context.set_line_width(1.0);
        let mut x = offset_x;
        while x <= width {
            context.begin_path();
            context.move_to(x + 0.5, 0.0);
            context.line_to(x + 0.5, height);
            context.stroke();
            x += grid_step;
        }
        let mut y = offset_y;
        while y <= height {
            context.begin_path();
            context.move_to(0.0, y + 0.5);
            context.line_to(width, y + 0.5);
            context.stroke();
            y += grid_step;
        }
    }
}
