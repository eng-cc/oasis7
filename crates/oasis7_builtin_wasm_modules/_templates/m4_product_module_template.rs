use oasis7_wasm_sdk::{
    export_wasm_module,
    wire::{
        decode_action, decode_input, empty_output, encode_output, ModuleCallInput,
        ModuleEffectIntent, ModuleEmit, ModuleOutput,
    },
    LifecycleStage, WasmModuleLifecycle,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct MaterialStackData {
    kind: String,
    amount: i64,
}

#[derive(Debug, Clone, Serialize)]
struct ProductValidationData {
    product_id: String,
    accepted: bool,
    #[serde(default)]
    notes: Vec<String>,
    stack_limit: u32,
    tradable: bool,
    #[serde(default)]
    quality_levels: Vec<String>,
}

fn build_product_output(input: &ModuleCallInput) -> Vec<u8> {
    let stack = decode_action::<MaterialStackData>(input).unwrap_or_else(|| MaterialStackData {
        kind: PRODUCT_ID.to_string(),
        amount: 0,
    });

    let mut notes = Vec::new();
    let mut accepted = true;

    if stack.kind != PRODUCT_ID {
        accepted = false;
        notes.push(format!(
            "product kind mismatch expected={} got={}",
            PRODUCT_ID, stack.kind
        ));
    }
    if stack.amount <= 0 {
        accepted = false;
        notes.push("stack amount must be > 0".to_string());
    }
    if stack.amount > STACK_LIMIT as i64 {
        accepted = false;
        notes.push(format!(
            "stack exceeds limit amount={} limit={}",
            stack.amount, STACK_LIMIT
        ));
    }

    let payload = serde_json::to_value(ProductValidationData {
        product_id: PRODUCT_ID.to_string(),
        accepted,
        notes,
        stack_limit: STACK_LIMIT,
        tradable: TRADABLE,
        quality_levels: QUALITY_LEVELS.iter().map(|item| item.to_string()).collect(),
    })
    .unwrap_or_else(|_| serde_json::json!({ "accepted": false }));

    encode_output(ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: vec![ModuleEmit {
            kind: PRODUCT_VALIDATION_EMIT_KIND.to_string(),
            payload,
        }],
        tick_lifecycle: None,
        output_bytes: 512,
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
    let output = build_product_output(&decoded);
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
