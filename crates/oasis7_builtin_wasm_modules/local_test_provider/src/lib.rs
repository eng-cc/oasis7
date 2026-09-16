#![allow(improper_ctypes_definitions)]

//! Small pure command module used by the explicit DevLocal W3 initializer.
//!
//! The module deliberately has no effects, state, or provider access. Its
//! purpose is to prove that a real SDK-built `call` artifact can pass the
//! governed module lifecycle and be selected by the Runtime command catalog.

use oasis7_wasm_sdk::{
    LifecycleStage, WasmModuleLifecycle, export_wasm_module,
    wire::{ModuleCallInput, decode_input, empty_output, encode_output},
};

const MODULE_ID: &str = "module.runtime.local-test-provider";

fn read_input_bytes(input_ptr: i32, input_len: i32) -> Vec<u8> {
    if input_ptr <= 0 || input_len <= 0 {
        return Vec::new();
    }
    let ptr = input_ptr as *const u8;
    let len = input_len as usize;
    // SAFETY: the host supplies a valid pointer and length for each call.
    unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
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
    // SAFETY: `default_alloc` returns a writable region of at least `len`.
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, len as usize) };
    (ptr, len)
}

fn call_impl(input_ptr: i32, input_len: i32) -> (i32, i32) {
    let input = read_input_bytes(input_ptr, input_len);
    let Ok(decoded) = decode_input(&input) else {
        return write_bytes_to_memory(&encode_output(empty_output()));
    };
    if decoded.ctx.module_id != MODULE_ID {
        return write_bytes_to_memory(&encode_output(empty_output()));
    }
    let _input: ModuleCallInput = decoded;
    write_bytes_to_memory(&encode_output(empty_output()))
}

#[derive(Default)]
struct LocalTestProviderModule;

impl WasmModuleLifecycle for LocalTestProviderModule {
    fn module_id(&self) -> &'static str {
        MODULE_ID
    }

    fn alloc(&mut self, len: i32) -> i32 {
        oasis7_wasm_sdk::default_alloc(len)
    }

    fn on_init(&mut self, _stage: LifecycleStage) {}

    fn on_teardown(&mut self, _stage: LifecycleStage) {}

    fn on_reduce(&mut self, input_ptr: i32, input_len: i32) -> (i32, i32) {
        call_impl(input_ptr, input_len)
    }

    fn on_call(&mut self, input_ptr: i32, input_len: i32) -> (i32, i32) {
        call_impl(input_ptr, input_len)
    }
}

export_wasm_module!(LocalTestProviderModule);
