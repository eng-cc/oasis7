#![allow(improper_ctypes_definitions)]

use oasis7_wasm_sdk::{
    export_wasm_module,
    wire::{
        decode_action, decode_input, empty_output, encode_output, ModuleCallInput,
        ModuleEffectIntent, ModuleEmit, ModuleOutput,
    },
    LifecycleStage, WasmModuleLifecycle,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const MODULE_ID: &str = "m1.power.radiation_harvest";
const POWER_RADIATION_EMIT_KIND: &str = "power.radiation_harvest";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
struct GeoPos {
    x_cm: f64,
    y_cm: f64,
    z_cm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct AgentPowerState {
    pos: GeoPos,
    level: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
struct PowerState {
    #[serde(default)]
    agents: BTreeMap<String, AgentPowerState>,
}

fn decode_power_state(state_bytes: Option<&[u8]>) -> PowerState {
    let Some(state_bytes) = state_bytes else {
        return PowerState::default();
    };
    if state_bytes.is_empty() {
        return PowerState::default();
    }
    serde_cbor::from_slice(state_bytes).unwrap_or_default()
}

fn encode_power_state(state: &PowerState) -> Option<Vec<u8>> {
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
    Some(GeoPos {
        x_cm: value.get("x_cm")?.as_f64()?,
        y_cm: value.get("y_cm")?.as_f64()?,
        z_cm: value.get("z_cm")?.as_f64()?,
    })
}

fn decode_domain_event(event_bytes: &[u8]) -> Option<(String, Value)> {
    let event: Value = serde_cbor::from_slice(event_bytes).ok()?;
    if event
        .get("body")
        .and_then(|body| body.get("kind"))
        .and_then(Value::as_str)
        != Some("Domain")
    {
        return None;
    }
    let payload = event.get("body")?.get("payload")?;
    let event_type = payload.get("type")?.as_str()?.to_string();
    let data = payload.get("data")?.clone();
    Some((event_type, data))
}

fn radiation_harvest_per_tick(pos: GeoPos) -> i64 {
    if 1 <= 0 {
        return 0;
    }
    let axis_sum_cm = pos.x_cm.abs() + pos.y_cm.abs() + pos.z_cm.abs();
    let step = 800_000.max(1) as f64;
    let bonus = (axis_sum_cm / step).floor() as i64;
    let bounded_bonus = bonus.clamp(0, 1.max(0));
    1_i64.saturating_add(bounded_bonus)
}

fn update_radiation_positions(state: &mut PowerState, event_type: &str, data: &Value) -> bool {
    match event_type {
        "AgentRegistered" => {
            let Some(agent_id) = data.get("agent_id").and_then(Value::as_str) else {
                return false;
            };
            let Some(pos) = data.get("pos").and_then(parse_geo_pos) else {
                return false;
            };
            state
                .agents
                .entry(agent_id.to_string())
                .and_modify(|entry| entry.pos = pos)
                .or_insert(AgentPowerState { pos, level: 0 });
            true
        }
        "AgentMoved" => {
            let Some(agent_id) = data.get("agent_id").and_then(Value::as_str) else {
                return false;
            };
            let Some(to) = data.get("to").and_then(parse_geo_pos) else {
                return false;
            };
            let Some(entry) = state.agents.get_mut(agent_id) else {
                return false;
            };
            entry.pos = to;
            true
        }
        _ => false,
    }
}

fn build_radiation_action_output(input: &ModuleCallInput, mut state: PowerState) -> Vec<u8> {
    let Some((action_id, _)) = action_envelope(input) else {
        return encode_output(empty_output());
    };

    let mut changed = false;
    for agent_state in state.agents.values_mut() {
        let harvested = radiation_harvest_per_tick(agent_state.pos);
        if harvested <= 0 {
            continue;
        }
        agent_state.level = agent_state.level.saturating_add(harvested);
        changed = true;
    }

    let new_state = if changed {
        encode_power_state(&state)
    } else {
        None
    };
    let emit_payload = json!({
        "action_id": action_id,
        "agents": state
            .agents
            .iter()
            .map(|(agent_id, power)| {
                json!({
                    "agent_id": agent_id,
                    "level": power.level,
                })
            })
            .collect::<Vec<_>>()
    });

    encode_output(ModuleOutput {
        new_state,
        effects: Vec::new(),
        emits: vec![ModuleEmit {
            kind: POWER_RADIATION_EMIT_KIND.to_string(),
            payload: emit_payload,
        }],
        tick_lifecycle: None,
        output_bytes: 0,
    })
}

fn build_radiation_event_output(input: &ModuleCallInput, mut state: PowerState) -> Vec<u8> {
    let Some(event_bytes) = input.event.as_deref() else {
        return encode_output(empty_output());
    };
    let Some((event_type, data)) = decode_domain_event(event_bytes) else {
        return encode_output(empty_output());
    };
    let changed = update_radiation_positions(&mut state, &event_type, &data);
    let new_state = if changed {
        encode_power_state(&state)
    } else {
        None
    };

    encode_output(ModuleOutput {
        new_state,
        effects: Vec::new(),
        emits: Vec::new(),
        tick_lifecycle: None,
        output_bytes: 0,
    })
}

fn build_radiation_power_module_output(input: &ModuleCallInput) -> Vec<u8> {
    let state = decode_power_state(input.state.as_deref());

    if input.action.is_some() {
        return build_radiation_action_output(input, state);
    }
    if input.event.is_some() {
        return build_radiation_event_output(input, state);
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
    let output = build_radiation_power_module_output(&decoded);
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
