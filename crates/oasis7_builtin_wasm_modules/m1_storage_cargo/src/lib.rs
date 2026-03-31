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
use serde_json::Value;
use std::collections::BTreeMap;

const MODULE_ID: &str = "m1.storage.cargo";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
struct CargoLedgerState {
    #[serde(default)]
    consumed_interface_items: BTreeMap<String, u64>,
    #[serde(default)]
    agent_expansion_levels: BTreeMap<String, u16>,
    #[serde(default)]
    reject_count: u64,
}

fn decode_storage_cargo_state(state_bytes: Option<&[u8]>) -> CargoLedgerState {
    let Some(state_bytes) = state_bytes else {
        return CargoLedgerState::default();
    };
    if state_bytes.is_empty() {
        return CargoLedgerState::default();
    }
    serde_cbor::from_slice(state_bytes).unwrap_or_default()
}

fn encode_storage_cargo_state(state: &CargoLedgerState) -> Option<Vec<u8>> {
    serde_cbor::to_vec(state).ok()
}

fn build_storage_cargo_module_output(input: &ModuleCallInput) -> Vec<u8> {
    let Some(event_bytes) = input.event.as_deref() else {
        return encode_output(empty_output());
    };
    let event: Value = match serde_cbor::from_slice(event_bytes) {
        Ok(value) => value,
        Err(_) => return encode_output(empty_output()),
    };
    if event
        .get("body")
        .and_then(|body| body.get("kind"))
        .and_then(Value::as_str)
        != Some("Domain")
    {
        return encode_output(empty_output());
    }
    let Some(payload) = event.get("body").and_then(|body| body.get("payload")) else {
        return encode_output(empty_output());
    };
    let Some(event_type) = payload.get("type").and_then(Value::as_str) else {
        return encode_output(empty_output());
    };
    let Some(data) = payload.get("data") else {
        return encode_output(empty_output());
    };

    let mut state = decode_storage_cargo_state(input.state.as_deref());
    let changed = match event_type {
        "BodyInterfaceExpanded" => {
            let Some(agent_id) = data.get("agent_id").and_then(Value::as_str) else {
                return encode_output(empty_output());
            };
            let Some(consumed_item_id) = data.get("consumed_item_id").and_then(Value::as_str)
            else {
                return encode_output(empty_output());
            };
            let Some(expansion_level_raw) = data.get("expansion_level").and_then(Value::as_u64)
            else {
                return encode_output(empty_output());
            };
            let Ok(expansion_level) = u16::try_from(expansion_level_raw) else {
                return encode_output(empty_output());
            };

            state
                .agent_expansion_levels
                .insert(agent_id.to_string(), expansion_level);
            let consumed = state
                .consumed_interface_items
                .entry(consumed_item_id.to_string())
                .or_insert(0);
            *consumed = consumed.saturating_add(1);
            true
        }
        "BodyInterfaceExpandRejected" => {
            state.reject_count = state.reject_count.saturating_add(1);
            true
        }
        _ => false,
    };

    if !changed {
        return encode_output(empty_output());
    }

    encode_output(ModuleOutput {
        new_state: encode_storage_cargo_state(&state),
        effects: Vec::new(),
        emits: Vec::new(),
        tick_lifecycle: None,
        output_bytes: 0,
    })
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
    let output = build_storage_cargo_module_output(&decoded);
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
