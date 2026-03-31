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

impl MaterialStackData {
    fn new(kind: impl Into<String>, amount: i64) -> Self {
        Self {
            kind: kind.into(),
            amount,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct RecipeExecutionRequestData {
    recipe_id: String,
    factory_id: String,
    desired_batches: u32,
    #[serde(default)]
    available_inputs: Vec<MaterialStackData>,
    available_power: i64,
    #[allow(dead_code)]
    deterministic_seed: u64,
}

#[derive(Debug, Clone, Serialize)]
struct RecipeExecutionPlanData {
    accepted_batches: u32,
    #[serde(default)]
    consume: Vec<MaterialStackData>,
    #[serde(default)]
    produce: Vec<MaterialStackData>,
    #[serde(default)]
    byproducts: Vec<MaterialStackData>,
    power_required: i64,
    duration_ticks: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    reject_reason: Option<String>,
}

impl RecipeExecutionPlanData {
    fn accepted(
        accepted_batches: u32,
        consume: Vec<MaterialStackData>,
        produce: Vec<MaterialStackData>,
        byproducts: Vec<MaterialStackData>,
        power_required: i64,
        duration_ticks: u32,
    ) -> Self {
        Self {
            accepted_batches,
            consume,
            produce,
            byproducts,
            power_required,
            duration_ticks,
            reject_reason: None,
        }
    }

    fn rejected(reason: impl Into<String>) -> Self {
        Self {
            accepted_batches: 0,
            consume: Vec::new(),
            produce: Vec::new(),
            byproducts: Vec::new(),
            power_required: 0,
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

fn first_recipe_bottleneck(
    request: &RecipeExecutionRequestData,
    available: &std::collections::BTreeMap<&str, i64>,
) -> String {
    for (kind, amount) in CONSUME_PER_BATCH {
        let available_amount = available.get(*kind).copied().unwrap_or(0);
        if available_amount < *amount {
            return format!(
                "insufficient material kind={} required_per_batch={} available={}",
                kind, amount, available_amount
            );
        }
    }
    if POWER_PER_BATCH > request.available_power {
        return format!(
            "insufficient power required_per_batch={} available={}",
            POWER_PER_BATCH, request.available_power
        );
    }
    "requested batches reduced to zero by constraints".to_string()
}

fn scale_stacks(items: &[(&str, i64)], factor: u32) -> Vec<MaterialStackData> {
    items
        .iter()
        .map(|(kind, amount)| MaterialStackData::new(*kind, amount.saturating_mul(factor as i64)))
        .collect()
}

fn emit_recipe_plan(plan: RecipeExecutionPlanData) -> Vec<u8> {
    let payload = serde_json::to_value(plan).unwrap_or_else(|_| {
        serde_json::json!({
            "accepted_batches": 0,
            "consume": [],
            "produce": [],
            "byproducts": [],
            "power_required": 0,
            "duration_ticks": 0,
            "reject_reason": "serialize plan failed"
        })
    });
    encode_output(ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: vec![ModuleEmit {
            kind: RECIPE_EXECUTION_PLAN_EMIT_KIND.to_string(),
            payload,
        }],
        tick_lifecycle: None,
        output_bytes: 512,
    })
}

fn build_recipe_output(input: &ModuleCallInput) -> Vec<u8> {
    let Some(request) = decode_action::<RecipeExecutionRequestData>(input) else {
        return encode_output(empty_output());
    };

    if request.recipe_id != RECIPE_ID {
        return emit_recipe_plan(RecipeExecutionPlanData::rejected(format!(
            "recipe_id mismatch expected={} got={}",
            RECIPE_ID, request.recipe_id
        )));
    }

    if !request
        .factory_id
        .to_ascii_lowercase()
        .contains(REQUIRED_FACTORY_MARKER)
    {
        return emit_recipe_plan(RecipeExecutionPlanData::rejected(format!(
            "factory {} incompatible with {}",
            request.factory_id, RECIPE_ID
        )));
    }

    if request.desired_batches == 0 {
        return emit_recipe_plan(RecipeExecutionPlanData::rejected(
            "desired_batches must be > 0",
        ));
    }

    let available = as_inventory(&request.available_inputs);
    let mut accepted_batches = request.desired_batches;

    for (kind, amount) in CONSUME_PER_BATCH {
        if *amount <= 0 {
            return emit_recipe_plan(RecipeExecutionPlanData::rejected(format!(
                "invalid recipe consume amount kind={} amount={}",
                kind, amount
            )));
        }

        let available_amount = available.get(*kind).copied().unwrap_or(0).max(0);
        let material_max = (available_amount / *amount).max(0) as u32;
        accepted_batches = accepted_batches.min(material_max);
    }

    if POWER_PER_BATCH > 0 {
        let power_max = (request.available_power.max(0) / POWER_PER_BATCH) as u32;
        accepted_batches = accepted_batches.min(power_max);
    }

    if accepted_batches == 0 {
        let reason = first_recipe_bottleneck(&request, &available);
        return emit_recipe_plan(RecipeExecutionPlanData::rejected(reason));
    }

    let consume = scale_stacks(CONSUME_PER_BATCH, accepted_batches);
    let produce = scale_stacks(PRODUCE_PER_BATCH, accepted_batches);
    let byproducts = scale_stacks(BYPRODUCTS_PER_BATCH, accepted_batches);
    let power_required = POWER_PER_BATCH.saturating_mul(accepted_batches as i64);

    emit_recipe_plan(RecipeExecutionPlanData::accepted(
        accepted_batches,
        consume,
        produce,
        byproducts,
        power_required,
        DURATION_TICKS,
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
    let output = build_recipe_output(&decoded);
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
