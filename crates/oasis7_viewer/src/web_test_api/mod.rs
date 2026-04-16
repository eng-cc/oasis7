mod wasm;

#[cfg(target_arch = "wasm32")]
use crate::viewer_automation::{
    enqueue_runtime_steps, parse_automation_mode, parse_automation_steps, parse_automation_target,
};
#[cfg(target_arch = "wasm32")]
use crate::{
    dispatch_viewer_control, viewer_control_profile_name, viewer_control_supported_for_profile,
    ViewerAutomationState, ViewerClient, ViewerControlDispatchResult, ViewerControlProfileState,
    ViewerSelection, ViewerState,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::{
    viewer_control_supported_for_profile, ViewerAutomationState, ViewerClient,
    ViewerControlProfileState, ViewerSelection, ViewerState,
};
#[cfg(target_arch = "wasm32")]
use crate::{ConnectionStatus, SelectionKind};
use crate::{OrbitCamera, Viewer3dCamera, ViewerCameraMode};
use bevy::prelude::*;
#[cfg(target_arch = "wasm32")]
use oasis7::viewer::ControlCompletionStatus;
use oasis7::viewer::{ViewerControl, ViewerControlProfile};
#[cfg(target_arch = "wasm32")]
use std::cell::RefCell;
#[cfg(target_arch = "wasm32")]
use std::collections::{HashMap, VecDeque};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::{JsCast, JsValue};
#[cfg(target_arch = "wasm32")]
use web_sys::js_sys::{Array, Function, Object, Reflect as JsReflect};
#[cfg(target_arch = "wasm32")]
use web_sys::UrlSearchParams;
#[cfg(target_arch = "wasm32")]
const TEST_API_QUERY_KEY: &str = "test_api";
#[cfg(target_arch = "wasm32")]
const TEST_API_GLOBAL_NAME: &str = "__AW_TEST__";
#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
const WEB_TEST_API_CONTROL_ACTIONS: [&str; 4] = ["play", "pause", "step", "seek"];
#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
const WEB_TEST_API_CONTROL_ACTIONS_LIVE: [&str; 3] = ["play", "pause", "step"];
#[cfg(target_arch = "wasm32")]
const CONTROL_STALL_FRAME_RATE_FALLBACK: f64 = 60.0;
#[cfg(target_arch = "wasm32")]
const CONTROL_STALL_FRAME_THRESHOLD_STEP: u32 = 150;
#[cfg(target_arch = "wasm32")]
const CONTROL_STALL_FRAME_THRESHOLD_PLAY_STEADY: u32 = 180;
#[cfg(target_arch = "wasm32")]
const CONTROL_STALL_FRAME_THRESHOLD_PLAY_COLD_START: u32 = 420;
#[cfg(target_arch = "wasm32")]
const CONTROL_STAGE_RECEIVED: &str = "received";
#[cfg(target_arch = "wasm32")]
const CONTROL_STAGE_EXECUTING: &str = "executing";
#[cfg(target_arch = "wasm32")]
const CONTROL_STAGE_COMPLETED_ADVANCED: &str = "completed_advanced";
#[cfg(target_arch = "wasm32")]
const CONTROL_STAGE_COMPLETED_NO_PROGRESS: &str = "completed_no_progress";
#[cfg(target_arch = "wasm32")]
const CONTROL_STAGE_BLOCKED: &str = "blocked";
#[cfg(target_arch = "wasm32")]
enum WebTestApiCommand {
    EnqueueSteps(Vec<crate::viewer_automation::ViewerAutomationStep>),
    SendControl {
        control: ViewerControl,
        feedback_id: u64,
        request_id: Option<u64>,
    },
}
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug)]
struct WebTestApiControlFeedback {
    id: u64,
    request_id: Option<u64>,
    action: String,
    accepted: bool,
    enqueued: bool,
    stage: String,
    parsed_control: Option<String>,
    reason: Option<String>,
    hint: Option<String>,
    effect: String,
    baseline_logical_time: u64,
    baseline_event_seq: u64,
    baseline_trace_count: usize,
    delta_logical_time: u64,
    delta_event_seq: u64,
    delta_trace_count: usize,
    awaiting_effect: bool,
    no_progress_frames: u32,
}
#[derive(Clone, Debug)]
pub(super) struct WebTestApiControlFeedbackSnapshot {
    pub(super) action: String,
    pub(super) stage: String,
    pub(super) reason: Option<String>,
    pub(super) hint: Option<String>,
    pub(super) effect: String,
    pub(super) delta_logical_time: u64,
    pub(super) delta_event_seq: u64,
    pub(super) delta_trace_count: usize,
}
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Debug)]
struct WebTestApiStateSnapshot {
    connection_status: &'static str,
    control_profile: Option<ViewerControlProfile>,
    logical_time: u64,
    event_seq: u64,
    selected_kind: Option<String>,
    selected_id: Option<String>,
    error_count: u64,
    last_error: Option<String>,
    event_count: usize,
    trace_count: usize,
    camera_mode: &'static str,
    camera_radius: f64,
    camera_ortho_scale: f64,
    last_control_feedback: Option<WebTestApiControlFeedback>,
}
#[cfg(target_arch = "wasm32")]
impl Default for WebTestApiStateSnapshot {
    fn default() -> Self {
        Self {
            connection_status: "connecting",
            control_profile: None,
            logical_time: 0,
            event_seq: 0,
            selected_kind: None,
            selected_id: None,
            error_count: 0,
            last_error: None,
            event_count: 0,
            trace_count: 0,
            camera_mode: "3d",
            camera_radius: 0.0,
            camera_ortho_scale: 0.0,
            last_control_feedback: None,
        }
    }
}
#[cfg(target_arch = "wasm32")]
thread_local! {
    static WEB_TEST_API_COMMAND_QUEUE: RefCell<VecDeque<WebTestApiCommand>> = RefCell::new(VecDeque::new());
    static WEB_TEST_API_STATE_SNAPSHOT: RefCell<WebTestApiStateSnapshot> = RefCell::new(WebTestApiStateSnapshot::default());
    static WEB_TEST_API_COMPLETION_ACKS: RefCell<HashMap<u64, oasis7::viewer::ControlCompletionAck>> = RefCell::new(HashMap::new());
    static WEB_TEST_API_CONTROL_FEEDBACK_ID: RefCell<u64> = const { RefCell::new(0) };
    static WEB_TEST_API_RUNTIME_FATAL_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn probe_control_for_action(action: &str) -> Option<ViewerControl> {
    match action {
        "play" => Some(ViewerControl::Play),
        "pause" => Some(ViewerControl::Pause),
        "step" => Some(ViewerControl::Step { count: 1 }),
        "seek" => Some(ViewerControl::Seek { tick: 0 }),
        _ => None,
    }
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn supported_control_actions_for_profile(
    profile: Option<ViewerControlProfile>,
) -> &'static [&'static str] {
    match profile {
        Some(ViewerControlProfile::Live) => &WEB_TEST_API_CONTROL_ACTIONS_LIVE,
        Some(ViewerControlProfile::Playback) | None => &WEB_TEST_API_CONTROL_ACTIONS,
    }
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn control_action_supported_for_profile(
    action: &str,
    profile: Option<ViewerControlProfile>,
) -> bool {
    probe_control_for_action(action)
        .is_some_and(|control| viewer_control_supported_for_profile(profile, &control))
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn supported_action_list(profile: Option<ViewerControlProfile>) -> String {
    supported_control_actions_for_profile(profile).join(", ")
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn unsupported_control_reason(
    profile: ViewerControlProfile,
    control: &ViewerControl,
    locale_zh: bool,
) -> Option<String> {
    match (profile, control, locale_zh) {
        (ViewerControlProfile::Live, ViewerControl::Seek { .. }, true) => {
            Some("live 控制模式不支持 seek".to_string())
        }
        (ViewerControlProfile::Live, ViewerControl::Seek { .. }, false) => {
            Some("seek is not supported in live control mode".to_string())
        }
        _ => None,
    }
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn unsupported_control_hint(
    profile: ViewerControlProfile,
    control: &ViewerControl,
    locale_zh: bool,
) -> Option<String> {
    match (profile, control, locale_zh) {
        (ViewerControlProfile::Live, ViewerControl::Seek { .. }, true) => {
            Some("live 模式请使用 play / pause / step".to_string())
        }
        (ViewerControlProfile::Live, ViewerControl::Seek { .. }, false) => {
            Some("use play/pause/step in live mode".to_string())
        }
        _ => None,
    }
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn unsupported_action_reason(
    action: &str,
    profile: Option<ViewerControlProfile>,
    locale_zh: bool,
) -> Option<String> {
    let control = probe_control_for_action(action)?;
    let profile = profile?;
    unsupported_control_reason(profile, &control, locale_zh)
}

#[cfg_attr(not(any(test, target_arch = "wasm32")), allow(dead_code))]
fn unsupported_action_hint(
    action: &str,
    profile: Option<ViewerControlProfile>,
    locale_zh: bool,
) -> Option<String> {
    let control = probe_control_for_action(action)?;
    let profile = profile?;
    unsupported_control_hint(profile, &control, locale_zh)
}
#[cfg(target_arch = "wasm32")]
pub(super) struct WebTestApiBindings {
    _api: Object,
    _run_steps: Closure<dyn FnMut(JsValue)>,
    _set_mode: Closure<dyn FnMut(JsValue)>,
    _focus: Closure<dyn FnMut(JsValue)>,
    _select: Closure<dyn FnMut(JsValue)>,
    _describe_controls: Closure<dyn FnMut() -> JsValue>,
    _fill_control_example: Closure<dyn FnMut(JsValue) -> JsValue>,
    _send_control: Closure<dyn FnMut(JsValue, JsValue) -> JsValue>,
    _report_fatal_error: Closure<dyn FnMut(JsValue, JsValue)>,
    _get_state: Closure<dyn FnMut() -> JsValue>,
    _runtime_diag_installer: Function,
}

#[cfg(target_arch = "wasm32")]
fn log_api_warning(message: &str) {
    web_sys::console::warn_1(&JsValue::from_str(message));
}

#[cfg(target_arch = "wasm32")]
fn web_test_api_enabled(window: &web_sys::Window) -> bool {
    if cfg!(debug_assertions) {
        return true;
    }
    let Ok(search) = window.location().search() else {
        return false;
    };
    let Ok(params) = UrlSearchParams::new_with_str(&search) else {
        return false;
    };
    params
        .get(TEST_API_QUERY_KEY)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

#[cfg(target_arch = "wasm32")]
fn push_command(command: WebTestApiCommand) {
    WEB_TEST_API_COMMAND_QUEUE.with(|queue| {
        queue.borrow_mut().push_back(command);
    });
}

#[cfg(target_arch = "wasm32")]
fn next_control_feedback_id() -> u64 {
    WEB_TEST_API_CONTROL_FEEDBACK_ID.with(|counter| {
        let mut counter = counter.borrow_mut();
        *counter = counter.saturating_add(1);
        *counter
    })
}

#[cfg(target_arch = "wasm32")]
fn latest_progress_baseline() -> (u64, u64, usize) {
    WEB_TEST_API_STATE_SNAPSHOT.with(|slot| {
        let snapshot = slot.borrow();
        (
            snapshot.logical_time,
            snapshot.event_seq,
            snapshot.trace_count,
        )
    })
}

#[cfg(target_arch = "wasm32")]
fn control_feedback_no_progress_threshold(feedback: &WebTestApiControlFeedback) -> u32 {
    match feedback.action.as_str() {
        "play" => {
            if feedback.baseline_logical_time == 0
                && feedback.baseline_event_seq == 0
                && feedback.baseline_trace_count == 0
            {
                CONTROL_STALL_FRAME_THRESHOLD_PLAY_COLD_START
            } else {
                CONTROL_STALL_FRAME_THRESHOLD_PLAY_STEADY
            }
        }
        "step" => CONTROL_STALL_FRAME_THRESHOLD_STEP,
        _ => CONTROL_STALL_FRAME_THRESHOLD_STEP,
    }
}

#[cfg(target_arch = "wasm32")]
fn stall_hint_secs(frame_threshold: u32) -> f64 {
    frame_threshold as f64 / CONTROL_STALL_FRAME_RATE_FALLBACK
}

#[cfg(target_arch = "wasm32")]
fn control_payload_example(action: &str) -> JsValue {
    match action {
        "play" | "pause" => JsValue::NULL,
        "step" => {
            let payload = Object::new();
            let _ = JsReflect::set(
                &payload,
                &JsValue::from_str("count"),
                &JsValue::from_f64(5.0),
            );
            JsValue::from(payload)
        }
        "seek" => {
            let payload = Object::new();
            let _ = JsReflect::set(
                &payload,
                &JsValue::from_str("tick"),
                &JsValue::from_f64(120.0),
            );
            JsValue::from(payload)
        }
        _ => JsValue::NULL,
    }
}

#[cfg(target_arch = "wasm32")]
fn control_description(action: &str, is_zh: bool) -> &'static str {
    match (action, is_zh) {
        ("play", true) => "开始连续推进世界",
        ("play", false) => "Start continuous world advancement",
        ("pause", true) => "暂停连续推进",
        ("pause", false) => "Pause continuous advancement",
        ("step", true) => "推进固定步数（payload.count）",
        ("step", false) => "Advance fixed steps (payload.count)",
        ("seek", true) => "跳转到目标 tick（payload.tick）",
        ("seek", false) => "Seek timeline to target tick (payload.tick)",
        (_, true) => "未知动作",
        (_, false) => "Unknown action",
    }
}
#[cfg(target_arch = "wasm32")]
fn build_control_catalog_js_value(profile: Option<ViewerControlProfile>) -> JsValue {
    let object = Object::new();
    let controls = Array::new();
    for action in WEB_TEST_API_CONTROL_ACTIONS {
        let entry = Object::new();
        let supported = control_action_supported_for_profile(action, profile);
        let _ = JsReflect::set(
            &entry,
            &JsValue::from_str("action"),
            &JsValue::from_str(action),
        );
        let _ = JsReflect::set(
            &entry,
            &JsValue::from_str("description"),
            &JsValue::from_str(control_description(action, false)),
        );
        let _ = JsReflect::set(
            &entry,
            &JsValue::from_str("descriptionZh"),
            &JsValue::from_str(control_description(action, true)),
        );
        let _ = JsReflect::set(
            &entry,
            &JsValue::from_str("examplePayload"),
            &control_payload_example(action),
        );
        let _ = JsReflect::set(
            &entry,
            &JsValue::from_str("supported"),
            &JsValue::from_bool(supported),
        );
        let _ = JsReflect::set(
            &entry,
            &JsValue::from_str("reason"),
            &unsupported_action_reason(action, profile, false)
                .map(|value| JsValue::from_str(value.as_str()))
                .unwrap_or(JsValue::NULL),
        );
        let _ = JsReflect::set(
            &entry,
            &JsValue::from_str("hint"),
            &unsupported_action_hint(action, profile, false)
                .map(|value| JsValue::from_str(value.as_str()))
                .unwrap_or(JsValue::NULL),
        );
        controls.push(&entry);
    }
    let _ = JsReflect::set(&object, &JsValue::from_str("controls"), &controls);
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("controlProfile"),
        &viewer_control_profile_name(profile)
            .map(JsValue::from_str)
            .unwrap_or(JsValue::NULL),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("usage"),
        &JsValue::from_str(
            "Use getState().controlProfile to inspect current mode, then fillControlExample(action) and sendControl(action, payload).",
        ),
    );
    JsValue::from(object)
}
#[cfg(target_arch = "wasm32")]
fn parse_control_example_action(payload: &JsValue) -> Option<String> {
    let action = parse_string_payload(payload)?;
    let action = action.trim().to_ascii_lowercase();
    WEB_TEST_API_CONTROL_ACTIONS
        .iter()
        .find(|candidate| **candidate == action)
        .map(|_| action)
}
#[cfg(target_arch = "wasm32")]
fn parse_control_action_label(control: &ViewerControl) -> String {
    match control {
        ViewerControl::Play => "play".to_string(),
        ViewerControl::Pause => "pause".to_string(),
        ViewerControl::Step { count } => format!("step(count={count})"),
        ViewerControl::Seek { tick } => format!("seek(tick={tick})"),
    }
}
#[cfg(target_arch = "wasm32")]
fn control_action_hint(
    action: &str,
    locale_zh: bool,
    profile: Option<ViewerControlProfile>,
) -> String {
    match (action, locale_zh) {
        ("step", true) => "示例 payload: {\"count\": 5}".to_string(),
        ("step", false) => "Example payload: {\"count\": 5}".to_string(),
        ("seek", true) => "示例 payload: {\"tick\": 120}".to_string(),
        ("seek", false) => "Example payload: {\"tick\": 120}".to_string(),
        (_, true) => format!("可用动作: {}", supported_action_list(profile)),
        (_, false) => format!("Valid actions: {}", supported_action_list(profile)),
    }
}

#[cfg(target_arch = "wasm32")]
fn blocked_completion_ack_hint(error_code: Option<&str>) -> String {
    match error_code {
        Some("llm_mode_required" | "llm_init_failed") => {
            "Next: restore active LLM access, then retry step/play".to_string()
        }
        _ => {
            "Next: inspect the runtime failure, repair the broken world/module state, then retry control"
                .to_string()
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn current_web_test_api_control_profile() -> Option<ViewerControlProfile> {
    WEB_TEST_API_STATE_SNAPSHOT.with(|slot| slot.borrow().control_profile)
}
#[cfg(target_arch = "wasm32")]
fn update_last_control_feedback(feedback: WebTestApiControlFeedback) {
    WEB_TEST_API_STATE_SNAPSHOT.with(|slot| {
        slot.borrow_mut().last_control_feedback = Some(feedback);
    });
}

#[cfg(target_arch = "wasm32")]
fn normalize_runtime_error_text(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string()
}

#[cfg(target_arch = "wasm32")]
fn record_runtime_fatal_error(source: &str, message: &str) {
    let message = normalize_runtime_error_text(message);
    if message.is_empty() {
        return;
    }
    let source = normalize_runtime_error_text(source);
    let formatted = if source.is_empty() {
        message
    } else {
        format!("{source}: {message}")
    };
    WEB_TEST_API_RUNTIME_FATAL_ERROR.with(|slot| {
        *slot.borrow_mut() = Some(formatted.clone());
    });
    WEB_TEST_API_STATE_SNAPSHOT.with(|slot| {
        let mut snapshot = slot.borrow_mut();
        if snapshot.last_error.as_deref() != Some(formatted.as_str()) {
            snapshot.error_count = snapshot.error_count.saturating_add(1);
        }
        snapshot.connection_status = "error";
        snapshot.last_error = Some(formatted.clone());
    });
}

#[cfg(target_arch = "wasm32")]
fn current_runtime_fatal_error() -> Option<String> {
    WEB_TEST_API_RUNTIME_FATAL_ERROR.with(|slot| slot.borrow().clone())
}

#[cfg(target_arch = "wasm32")]
fn install_runtime_diagnostic_hooks() -> Function {
    let installer = Function::new_no_args(
        r#"
if (!window.__AW_TEST__ || window.__AW_RUNTIME_DIAG_INSTALLED) {
  return;
}
window.__AW_RUNTIME_DIAG_INSTALLED = true;
const stringify = (value) => {
  if (typeof value === 'string') {
    return value;
  }
  try {
    return JSON.stringify(value);
  } catch (_) {
    return String(value);
  }
};
const reloadKey = '__AW_RUNTIME_FATAL_RELOAD_ONCE__';
const knownFatal = /copy_deferred_lighting_id_pipeline|Shader compilation failed|wgpu error|Validation Error|CONTEXT_LOST_WEBGL|context lost/i;
const maybeReloadOnce = (message) => {
  try {
    const text = String(message || '');
    if (!knownFatal.test(text) || !window.sessionStorage) {
      return;
    }
    const marker = `${window.location.href}::fatal-reload`;
    if (window.sessionStorage.getItem(reloadKey) === marker) {
      return;
    }
    window.sessionStorage.setItem(reloadKey, marker);
    window.setTimeout(() => window.location.reload(), 0);
  } catch (_) {}
};
const report = (source, message) => {
  try {
    const text = String(message || source || 'unknown runtime error');
    maybeReloadOnce(text);
    const currentAw = window.__AW_TEST__;
    if (!currentAw || typeof currentAw.reportFatalError !== 'function') {
      return;
    }
    currentAw.reportFatalError(text, String(source || 'runtime'));
  } catch (_) {}
};
window.addEventListener('error', (event) => {
  const message = event?.message || event?.error?.message || stringify(event?.error || 'window error');
  report('window.error', message);
});
window.addEventListener('unhandledrejection', (event) => {
  const reason = event?.reason?.message || stringify(event?.reason || 'unhandled rejection');
  report('window.unhandledrejection', reason);
});
const originalError = console.error.bind(console);
console.error = function(...args) {
  try {
    const text = args.map(stringify).join(' ');
    if (/copy_deferred_lighting_id_pipeline|Shader compilation failed|wgpu error|Validation Error|CONTEXT_LOST_WEBGL|context lost/i.test(text)) {
      report('console.error', text);
    }
  } catch (_) {}
  return originalError(...args);
};
const canvas = typeof document !== 'undefined' && document.querySelector ? document.querySelector('canvas') : null;
if (canvas && canvas.addEventListener) {
  canvas.addEventListener('webglcontextlost', () => {
    report('webglcontextlost', 'WebGL context lost');
  });
}
"#,
    );
    let _ = installer.call0(&JsValue::NULL);
    installer
}

#[cfg(target_arch = "wasm32")]
fn mutate_last_control_feedback(
    feedback_id: u64,
    mutator: impl FnOnce(&mut WebTestApiControlFeedback),
) {
    WEB_TEST_API_STATE_SNAPSHOT.with(|slot| {
        let mut snapshot = slot.borrow_mut();
        let Some(current) = snapshot.last_control_feedback.as_mut() else {
            return;
        };
        if current.id == feedback_id {
            mutator(current);
        }
    });
}

#[cfg(target_arch = "wasm32")]
fn take_control_completion_ack(request_id: u64) -> Option<oasis7::viewer::ControlCompletionAck> {
    WEB_TEST_API_COMPLETION_ACKS.with(|acks| acks.borrow_mut().remove(&request_id))
}

#[cfg(target_arch = "wasm32")]
pub(super) fn record_control_completion_ack(ack: oasis7::viewer::ControlCompletionAck) {
    WEB_TEST_API_COMPLETION_ACKS.with(|acks| {
        acks.borrow_mut().insert(ack.request_id, ack);
    });
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn record_control_completion_ack(_ack: oasis7::viewer::ControlCompletionAck) {}

#[cfg(target_arch = "wasm32")]
fn parse_step_count(payload: &JsValue) -> Option<usize> {
    if payload.is_undefined() || payload.is_null() {
        return Some(1);
    }

    if let Some(number) = payload.as_f64() {
        if number.is_finite() && number >= 1.0 {
            return Some(number as usize);
        }
    }

    let count = JsReflect::get(payload, &JsValue::from_str("count")).ok()?;
    let number = count.as_f64()?;
    if number.is_finite() && number >= 1.0 {
        return Some(number as usize);
    }
    None
}

#[cfg(target_arch = "wasm32")]
fn parse_seek_tick(payload: &JsValue) -> Option<u64> {
    if payload.is_undefined() || payload.is_null() {
        return None;
    }

    if let Some(number) = payload.as_f64() {
        if number.is_finite() && number >= 0.0 {
            return Some(number as u64);
        }
    }

    let tick = JsReflect::get(payload, &JsValue::from_str("tick")).ok()?;
    let number = tick.as_f64()?;
    if number.is_finite() && number >= 0.0 {
        return Some(number as u64);
    }
    None
}

#[cfg(target_arch = "wasm32")]
fn parse_control_action(action: &str, payload: &JsValue) -> Option<ViewerControl> {
    match action.trim().to_ascii_lowercase().as_str() {
        "play" => Some(ViewerControl::Play),
        "pause" => Some(ViewerControl::Pause),
        "step" => parse_step_count(payload).map(|count| ViewerControl::Step { count }),
        "seek" => parse_seek_tick(payload).map(|tick| ViewerControl::Seek { tick }),
        _ => None,
    }
}

#[cfg(target_arch = "wasm32")]
fn normalize_control_action(action: &str) -> String {
    action.trim().to_ascii_lowercase()
}

#[cfg(target_arch = "wasm32")]
fn parse_string_payload(payload: &JsValue) -> Option<String> {
    payload
        .as_string()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(target_arch = "wasm32")]
fn parse_run_steps_command(payload: &JsValue) -> Option<WebTestApiCommand> {
    if let Some(raw_steps) = parse_string_payload(payload) {
        let steps = parse_automation_steps(raw_steps.as_str());
        if steps.is_empty() {
            return None;
        }
        return Some(WebTestApiCommand::EnqueueSteps(steps));
    }

    parse_step_count(payload).map(|count| WebTestApiCommand::SendControl {
        control: ViewerControl::Step { count },
        feedback_id: 0,
        request_id: None,
    })
}

#[cfg(target_arch = "wasm32")]
fn build_control_feedback_js_value(feedback: &WebTestApiControlFeedback) -> JsValue {
    let object = Object::new();
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("id"),
        &JsValue::from_f64(feedback.id as f64),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("requestId"),
        &feedback
            .request_id
            .map(|value| JsValue::from_f64(value as f64))
            .unwrap_or(JsValue::NULL),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("action"),
        &JsValue::from_str(feedback.action.as_str()),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("accepted"),
        &JsValue::from_bool(feedback.accepted),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("enqueued"),
        &JsValue::from_bool(feedback.enqueued),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("stage"),
        &JsValue::from_str(feedback.stage.as_str()),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("parsedControl"),
        &feedback
            .parsed_control
            .as_ref()
            .map(|value| JsValue::from_str(value))
            .unwrap_or(JsValue::NULL),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("reason"),
        &feedback
            .reason
            .as_ref()
            .map(|value| JsValue::from_str(value))
            .unwrap_or(JsValue::NULL),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("hint"),
        &feedback
            .hint
            .as_ref()
            .map(|value| JsValue::from_str(value))
            .unwrap_or(JsValue::NULL),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("effect"),
        &JsValue::from_str(feedback.effect.as_str()),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("deltaLogicalTime"),
        &JsValue::from_f64(feedback.delta_logical_time as f64),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("deltaEventSeq"),
        &JsValue::from_f64(feedback.delta_event_seq as f64),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("deltaTraceCount"),
        &JsValue::from_f64(feedback.delta_trace_count as f64),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("awaitingEffect"),
        &JsValue::from_bool(feedback.awaiting_effect),
    );
    JsValue::from(object)
}

#[cfg(target_arch = "wasm32")]
fn build_control_feedback(
    action: String,
    accepted: bool,
    parsed_control: Option<String>,
    reason: Option<String>,
    hint: Option<String>,
    effect: String,
    awaiting_effect: bool,
) -> WebTestApiControlFeedback {
    let (baseline_logical_time, baseline_event_seq, baseline_trace_count) =
        latest_progress_baseline();
    let stage = if !accepted {
        CONTROL_STAGE_BLOCKED
    } else if awaiting_effect {
        CONTROL_STAGE_RECEIVED
    } else {
        CONTROL_STAGE_COMPLETED_ADVANCED
    };
    WebTestApiControlFeedback {
        id: next_control_feedback_id(),
        request_id: None,
        action,
        accepted,
        enqueued: accepted && awaiting_effect,
        stage: stage.to_string(),
        parsed_control,
        reason,
        hint,
        effect,
        baseline_logical_time,
        baseline_event_seq,
        baseline_trace_count,
        delta_logical_time: 0,
        delta_event_seq: 0,
        delta_trace_count: 0,
        awaiting_effect,
        no_progress_frames: 0,
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) fn latest_web_test_api_control_feedback() -> Option<WebTestApiControlFeedbackSnapshot> {
    WEB_TEST_API_STATE_SNAPSHOT.with(|slot| {
        let snapshot = slot.borrow();
        snapshot
            .last_control_feedback
            .as_ref()
            .map(|feedback| WebTestApiControlFeedbackSnapshot {
                action: feedback.action.clone(),
                stage: feedback.stage.clone(),
                reason: feedback.reason.clone(),
                hint: feedback.hint.clone(),
                effect: feedback.effect.clone(),
                delta_logical_time: feedback.delta_logical_time,
                delta_event_seq: feedback.delta_event_seq,
                delta_trace_count: feedback.delta_trace_count,
            })
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn latest_web_test_api_control_feedback() -> Option<WebTestApiControlFeedbackSnapshot> {
    None
}

#[cfg(target_arch = "wasm32")]
fn build_state_js_value(snapshot: &WebTestApiStateSnapshot) -> JsValue {
    let object = Object::new();
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("connectionStatus"),
        &JsValue::from_str(snapshot.connection_status),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("controlProfile"),
        &viewer_control_profile_name(snapshot.control_profile)
            .map(JsValue::from_str)
            .unwrap_or(JsValue::NULL),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("logicalTime"),
        &JsValue::from_f64(snapshot.logical_time as f64),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("eventSeq"),
        &JsValue::from_f64(snapshot.event_seq as f64),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("tick"),
        &JsValue::from_f64(snapshot.logical_time as f64),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("selectedKind"),
        &snapshot
            .selected_kind
            .as_ref()
            .map(|value| JsValue::from_str(value))
            .unwrap_or(JsValue::NULL),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("selectedId"),
        &snapshot
            .selected_id
            .as_ref()
            .map(|value| JsValue::from_str(value))
            .unwrap_or(JsValue::NULL),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("errorCount"),
        &JsValue::from_f64(snapshot.error_count as f64),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("lastError"),
        &snapshot
            .last_error
            .as_ref()
            .map(|value| JsValue::from_str(value))
            .unwrap_or(JsValue::NULL),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("eventCount"),
        &JsValue::from_f64(snapshot.event_count as f64),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("traceCount"),
        &JsValue::from_f64(snapshot.trace_count as f64),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("cameraMode"),
        &JsValue::from_str(snapshot.camera_mode),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("cameraRadius"),
        &JsValue::from_f64(snapshot.camera_radius),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("cameraOrthoScale"),
        &JsValue::from_f64(snapshot.camera_ortho_scale),
    );
    let _ = JsReflect::set(
        &object,
        &JsValue::from_str("lastControlFeedback"),
        &snapshot
            .last_control_feedback
            .as_ref()
            .map(build_control_feedback_js_value)
            .unwrap_or(JsValue::NULL),
    );
    JsValue::from(object)
}

pub(super) fn setup_web_test_api(world: &mut World) {
    wasm::setup_web_test_api(world);
}

pub(super) fn consume_web_test_api_commands(
    automation_state: ResMut<ViewerAutomationState>,
    state: Option<Res<ViewerState>>,
    client: Option<Res<ViewerClient>>,
    control_profile: Option<Res<ViewerControlProfileState>>,
) {
    wasm::consume_web_test_api_commands(automation_state, state, client, control_profile);
}

pub(super) fn publish_web_test_api_state(
    state: Res<ViewerState>,
    selection: Res<ViewerSelection>,
    camera_mode: Res<ViewerCameraMode>,
    cameras: Query<(&OrbitCamera, &Projection), With<Viewer3dCamera>>,
    client: Option<Res<ViewerClient>>,
    control_profile: Option<Res<ViewerControlProfileState>>,
) {
    wasm::publish_web_test_api_state(
        state,
        selection,
        camera_mode,
        cameras,
        client,
        control_profile,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_profile_catalog_hides_seek_support() {
        assert!(control_action_supported_for_profile("seek", None));
        assert!(control_action_supported_for_profile(
            "seek",
            Some(ViewerControlProfile::Playback),
        ));
        assert!(!control_action_supported_for_profile(
            "seek",
            Some(ViewerControlProfile::Live),
        ));
        assert_eq!(
            supported_control_actions_for_profile(Some(ViewerControlProfile::Live)),
            &WEB_TEST_API_CONTROL_ACTIONS_LIVE
        );
    }

    #[test]
    fn live_seek_rejection_reason_is_explicit() {
        assert_eq!(
            unsupported_action_reason("seek", Some(ViewerControlProfile::Live), false),
            Some("seek is not supported in live control mode".to_string())
        );
        assert_eq!(
            unsupported_action_hint("seek", Some(ViewerControlProfile::Live), false),
            Some("use play/pause/step in live mode".to_string())
        );
    }

    #[test]
    fn supported_action_list_tracks_profile() {
        assert_eq!(
            supported_action_list(Some(ViewerControlProfile::Live)),
            "play, pause, step"
        );
        assert_eq!(
            supported_action_list(Some(ViewerControlProfile::Playback)),
            "play, pause, step, seek"
        );
    }
}
