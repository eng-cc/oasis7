#![allow(improper_ctypes_definitions)]

use oasis7_wasm_sdk::{
    export_wasm_module,
    wire::{
        decode_action, decode_input, empty_output, encode_output, parse_json_geo_pos_cm, GeoPosCm,
        ModuleCallInput, ModuleEffectIntent, ModuleEmit, ModuleOutput,
    },
    LifecycleStage, WasmModuleLifecycle,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const MODULE_ID: &str = "m1.rule.visibility";
const DEFAULT_VISIBILITY_RANGE_CM: i64 = 10_000_000;
const RULE_DECISION_EMIT_KIND: &str = "rule.decision";

type GeoPos = GeoPosCm;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
struct PositionState {
    #[serde(default)]
    agents: BTreeMap<String, GeoPos>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Observation {
    time: u64,
    agent_id: String,
    pos: GeoPos,
    visibility_range_cm: i64,
    visible_agents: Vec<ObservedAgent>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
struct ObservedAgent {
    agent_id: String,
    pos: GeoPos,
    distance_cm: i64,
}

fn decode_state(state_bytes: Option<&[u8]>) -> PositionState {
    let Some(state_bytes) = state_bytes else {
        return PositionState::default();
    };
    if state_bytes.is_empty() {
        return PositionState::default();
    }
    serde_cbor::from_slice(state_bytes).unwrap_or_default()
}

fn encode_state(state: &PositionState) -> Option<Vec<u8>> {
    serde_cbor::to_vec(state).ok()
}

fn action_envelope(input: &ModuleCallInput) -> Option<(u64, Value)> {
    let action_bytes = input.action.as_deref()?;
    if action_bytes.is_empty() {
        return None;
    }
    let action: Value = serde_cbor::from_slice(action_bytes).ok()?;
    let action_id = action.get("id")?.as_u64()?;
    let action_payload = action.get("action")?.clone();
    Some((action_id, action_payload))
}

fn parse_geo_pos(value: &Value) -> Option<GeoPos> {
    parse_json_geo_pos_cm(value)
}

fn space_distance_cm(a: GeoPos, b: GeoPos) -> i64 {
    let dx_m = (a.x_cm - b.x_cm) as f64 / 100.0;
    let dy_m = (a.y_cm - b.y_cm) as f64 / 100.0;
    let dz_m = (a.z_cm - b.z_cm) as f64 / 100.0;
    ((dx_m * dx_m + dy_m * dy_m + dz_m * dz_m).sqrt() * 100.0)
        .round()
        .max(0.0) as i64
}

fn update_position_state_from_event(state: &mut PositionState, event_bytes: &[u8]) -> bool {
    let event: Value = match serde_cbor::from_slice(event_bytes) {
        Ok(value) => value,
        Err(_) => return false,
    };
    let body = match event.get("body") {
        Some(body) => body,
        None => return false,
    };
    if body.get("kind").and_then(Value::as_str) != Some("Domain") {
        return false;
    }
    let payload = match body.get("payload") {
        Some(payload) => payload,
        None => return false,
    };
    let event_type = match payload.get("type").and_then(Value::as_str) {
        Some(event_type) => event_type,
        None => return false,
    };
    let data = match payload.get("data") {
        Some(data) => data,
        None => return false,
    };

    match event_type {
        "AgentRegistered" => {
            let Some(agent_id) = data.get("agent_id").and_then(Value::as_str) else {
                return false;
            };
            let Some(pos) = data.get("pos").and_then(parse_geo_pos) else {
                return false;
            };
            state.agents.insert(agent_id.to_string(), pos);
            true
        }
        "AgentMoved" => {
            let Some(agent_id) = data.get("agent_id").and_then(Value::as_str) else {
                return false;
            };
            let Some(pos) = data.get("to").and_then(parse_geo_pos) else {
                return false;
            };
            state.agents.insert(agent_id.to_string(), pos);
            true
        }
        _ => false,
    }
}

fn rule_emit_output(decision_payload: Value) -> Vec<u8> {
    let output = ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: vec![ModuleEmit {
            kind: RULE_DECISION_EMIT_KIND.to_string(),
            payload: decision_payload,
        }],
        tick_lifecycle: None,
        output_bytes: 0,
    };
    encode_output(output)
}

fn build_state_tracking_event_output(
    event_bytes: Option<&[u8]>,
    mut state: PositionState,
) -> Vec<u8> {
    let Some(event_bytes) = event_bytes else {
        return encode_output(empty_output());
    };
    let changed = update_position_state_from_event(&mut state, event_bytes);
    let new_state = if changed { encode_state(&state) } else { None };
    encode_output(ModuleOutput {
        new_state,
        effects: Vec::new(),
        emits: Vec::new(),
        tick_lifecycle: None,
        output_bytes: 0,
    })
}

fn build_visibility_rule_action_output(input: &ModuleCallInput, state: &PositionState) -> Vec<u8> {
    let Some((action_id, action)) = action_envelope(input) else {
        return encode_output(empty_output());
    };
    if action.get("type").and_then(Value::as_str) != Some("QueryObservation") {
        return encode_output(empty_output());
    }
    let Some(agent_id) = action
        .get("data")
        .and_then(|data| data.get("agent_id"))
        .and_then(Value::as_str)
    else {
        return encode_output(empty_output());
    };

    let mut decision = json!({
        "action_id": action_id,
        "verdict": "modify",
        "cost": { "entries": {} },
        "notes": [],
    });

    let Some(origin) = state.agents.get(agent_id).copied() else {
        decision["verdict"] = json!("deny");
        decision["notes"] = json!(["agent position missing for visibility rule"]);
        return rule_emit_output(decision);
    };

    let mut visible_agents = Vec::new();
    for (other_id, other_pos) in &state.agents {
        if other_id == agent_id {
            continue;
        }
        let distance_cm = space_distance_cm(origin, *other_pos);
        if distance_cm <= DEFAULT_VISIBILITY_RANGE_CM {
            visible_agents.push(ObservedAgent {
                agent_id: other_id.clone(),
                pos: *other_pos,
                distance_cm,
            });
        }
    }

    let observation = Observation {
        time: input.ctx.time,
        agent_id: agent_id.to_string(),
        pos: origin,
        visibility_range_cm: DEFAULT_VISIBILITY_RANGE_CM,
        visible_agents,
    };

    decision["override_action"] = json!({
        "type": "EmitObservation",
        "data": {
            "observation": observation
        }
    });
    rule_emit_output(decision)
}

fn build_visibility_rule_output(input: &ModuleCallInput) -> Vec<u8> {
    let state = decode_state(input.state.as_deref());
    if input.action.is_some() {
        return build_visibility_rule_action_output(input, &state);
    }
    if input.event.is_some() {
        return build_state_tracking_event_output(input.event.as_deref(), state);
    }
    encode_output(empty_output())
}

fn read_input_bytes(input_ptr: i32, input_len: i32) -> Vec<u8> {
    if input_ptr > 0 && input_len > 0 {
        let ptr = input_ptr as *const u8;
        let len = input_len as usize;
        // SAFETY: host guarantees valid wasm linear memory pointer/len for the call.
        return unsafe { std::slice::from_raw_parts(ptr, len).to_vec() };
    }
    Vec::new()
}

fn write_bytes_to_memory(bytes: &[u8]) -> (i32, i32) {
    let len = i32::try_from(bytes.len()).unwrap_or(0);
    if len <= 0 {
        return (0, 0);
    }
    let ptr = oasis7_wasm_sdk::default_alloc(len);
    if ptr <= 0 {
        return (0, 0);
    }
    // SAFETY: alloc returns a writable wasm linear memory region with at least len bytes.
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, len as usize);
    }
    (ptr, len)
}

fn reduce_impl(input_ptr: i32, input_len: i32) -> (i32, i32) {
    let input = read_input_bytes(input_ptr, input_len);
    let Ok(mut decoded) = decode_input(&input) else {
        return write_bytes_to_memory(&encode_output(empty_output()));
    };
    decoded.ctx.module_id = MODULE_ID.to_string();
    let output = build_visibility_rule_output(&decoded);
    write_bytes_to_memory(&output)
}

#[derive(Default)]
struct BuiltinWasmModule;

impl WasmModuleLifecycle for BuiltinWasmModule {
    fn module_id(&self) -> &'static str {
        MODULE_ID
    }

    fn alloc(&mut self, len: i32) -> i32 {
        oasis7_wasm_sdk::default_alloc(len)
    }

    fn on_init(&mut self, _stage: LifecycleStage) {}

    fn on_teardown(&mut self, _stage: LifecycleStage) {}

    fn on_reduce(&mut self, input_ptr: i32, input_len: i32) -> (i32, i32) {
        reduce_impl(input_ptr, input_len)
    }

    fn on_call(&mut self, input_ptr: i32, input_len: i32) -> (i32, i32) {
        reduce_impl(input_ptr, input_len)
    }
}

export_wasm_module!(BuiltinWasmModule);
