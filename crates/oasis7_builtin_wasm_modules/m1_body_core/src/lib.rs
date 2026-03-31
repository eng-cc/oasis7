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

const MODULE_ID: &str = "m1.body.core";
const RULE_DECISION_EMIT_KIND: &str = "rule.decision";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct BodyKernelView {
    mass_kg: u64,
    radius_cm: u64,
    thrust_limit: u64,
    cross_section_cm2: u64,
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

fn build_body_module_action_output(input: &ModuleCallInput) -> Vec<u8> {
    let Some((action_id, action)) = action_envelope(input) else {
        return encode_output(empty_output());
    };
    if action.get("type").and_then(Value::as_str) != Some("BodyAction") {
        return encode_output(empty_output());
    }
    let Some(data) = action.get("data") else {
        return encode_output(empty_output());
    };
    let Some(agent_id) = data.get("agent_id").and_then(Value::as_str) else {
        return encode_output(empty_output());
    };
    let Some(kind) = data.get("kind").and_then(Value::as_str) else {
        return encode_output(empty_output());
    };
    let Some(payload) = data.get("payload") else {
        return encode_output(empty_output());
    };

    let mut decision = json!({
        "action_id": action_id,
        "verdict": "allow",
        "cost": { "entries": {} },
        "notes": [],
    });

    let view: BodyKernelView = match serde_json::from_value(payload.clone()) {
        Ok(view) => view,
        Err(err) => {
            decision["verdict"] = json!("deny");
            decision["notes"] = json!([format!("body action payload decode failed: {err}")]);
            return rule_emit_output(decision);
        }
    };

    decision["verdict"] = json!("modify");
    decision["override_action"] = json!({
        "type": "EmitBodyAttributes",
        "data": {
            "agent_id": agent_id,
            "view": view,
            "reason": format!("body.{kind}"),
        }
    });
    if 10 > 0 {
        decision["cost"] = json!({
            "entries": {
                "electricity": -10
            }
        });
    }
    rule_emit_output(decision)
}

fn build_body_module_output(input: &ModuleCallInput) -> Vec<u8> {
    if input.action.is_some() {
        return build_body_module_action_output(input);
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
    let output = build_body_module_output(&decoded);
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
