use oasis7_wasm_sdk::{
    export_wasm_module,
    wire::{
        decode_action, decode_input, empty_output, encode_output, ModuleCallInput, ModuleEmit,
        ModuleOutput,
    },
    LifecycleStage, WasmModuleLifecycle,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct MaterialStackData {
    kind: String,
    amount: i64,
}

impl MaterialStackData {
    fn new(kind: impl Into<String>, amount: i64) -> Self {
        Self {
            kind: kind.into(),
            amount,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct FactoryBuildRequestData {
    factory_id: String,
    #[allow(dead_code)]
    site_id: String,
    #[allow(dead_code)]
    builder: String,
    #[serde(default)]
    available_inputs: Vec<MaterialStackData>,
    available_power: i64,
}

#[derive(Debug, Clone, Serialize)]
struct FactoryBuildDecisionData {
    accepted: bool,
    #[serde(default)]
    consume: Vec<MaterialStackData>,
    duration_ticks: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reject_reason: Option<String>,
}

impl FactoryBuildDecisionData {
    fn accepted(consume: Vec<MaterialStackData>, duration_ticks: u32) -> Self {
        Self {
            accepted: true,
            consume,
            duration_ticks,
            reject_reason: None,
        }
    }

    fn rejected(reason: impl Into<String>) -> Self {
        Self {
            accepted: false,
            consume: Vec::new(),
            duration_ticks: 0,
            reject_reason: Some(reason.into()),
        }
    }
}

fn as_inventory(stacks: &[MaterialStackData]) -> std::collections::BTreeMap<&str, i64> {
    let mut map = std::collections::BTreeMap::new();
    for stack in stacks {
        if stack.kind.trim().is_empty() {
            continue;
        }
        *map.entry(stack.kind.as_str()).or_insert(0) += stack.amount;
    }
    map
}

fn first_missing_material(
    required: &[(&str, i64)],
    available: &std::collections::BTreeMap<&str, i64>,
) -> Option<String> {
    for (kind, amount) in required {
        let available_amount = available.get(*kind).copied().unwrap_or(0);
        if available_amount < *amount {
            return Some(format!(
                "insufficient material kind={} required={} available={}",
                kind, amount, available_amount
            ));
        }
    }
    None
}

fn stacks_from_spec(items: &[(&str, i64)]) -> Vec<MaterialStackData> {
    items
        .iter()
        .map(|(kind, amount)| MaterialStackData::new(*kind, *amount))
        .collect()
}

fn emit_factory_decision(decision: FactoryBuildDecisionData) -> Vec<u8> {
    let payload = serde_json::to_value(decision).unwrap_or_else(|_| {
        serde_json::json!({
            "accepted": false,
            "consume": [],
            "duration_ticks": 0,
            "reject_reason": "serialize decision failed"
        })
    });
    encode_output(ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: vec![ModuleEmit {
            kind: FACTORY_BUILD_DECISION_EMIT_KIND.to_string(),
            payload,
        }],
        tick_lifecycle: None,
        output_bytes: 512,
    })
}

fn build_factory_output(input: &ModuleCallInput) -> Vec<u8> {
    let Ok(request) = decode_action::<FactoryBuildRequestData>(input) else {
        return encode_output(empty_output());
    };

    if request.factory_id != FACTORY_ID {
        return emit_factory_decision(FactoryBuildDecisionData::rejected(format!(
            "factory_id mismatch expected={} got={}",
            FACTORY_ID, request.factory_id
        )));
    }

    if request.available_power < FACTORY_MIN_POWER {
        return emit_factory_decision(FactoryBuildDecisionData::rejected(format!(
            "insufficient power required={} available={}",
            FACTORY_MIN_POWER, request.available_power
        )));
    }

    let available = as_inventory(&request.available_inputs);
    if let Some(reason) = first_missing_material(FACTORY_CONSUME, &available) {
        return emit_factory_decision(FactoryBuildDecisionData::rejected(reason));
    }

    emit_factory_decision(FactoryBuildDecisionData::accepted(
        stacks_from_spec(FACTORY_CONSUME),
        FACTORY_DURATION_TICKS,
    ))
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
    let output = build_factory_output(&decoded);
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
