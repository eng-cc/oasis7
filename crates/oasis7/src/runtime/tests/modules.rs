#[derive(Default)]
struct TickLifecycleSandbox {
    calls: Vec<oasis7_wasm_abi::ModuleCallRequest>,
    outputs: std::collections::VecDeque<oasis7_wasm_abi::ModuleOutput>,
}

impl TickLifecycleSandbox {
    fn with_outputs(outputs: Vec<oasis7_wasm_abi::ModuleOutput>) -> Self {
        Self {
            calls: Vec::new(),
            outputs: outputs.into(),
        }
    }
}

struct CaptureContextSandbox {
    requests: Vec<oasis7_wasm_abi::ModuleCallRequest>,
    outputs: std::collections::VecDeque<oasis7_wasm_abi::ModuleOutput>,
}

impl CaptureContextSandbox {
    fn with_outputs(outputs: Vec<oasis7_wasm_abi::ModuleOutput>) -> Self {
        Self {
            requests: Vec::new(),
            outputs: outputs.into(),
        }
    }
}

impl oasis7_wasm_abi::ModuleSandbox for CaptureContextSandbox {
    fn call(
        &mut self,
        request: &oasis7_wasm_abi::ModuleCallRequest,
    ) -> Result<oasis7_wasm_abi::ModuleOutput, oasis7_wasm_abi::ModuleCallFailure> {
        self.requests.push(request.clone());
        Ok(self
            .outputs
            .pop_front()
            .unwrap_or(oasis7_wasm_abi::ModuleOutput {
                new_state: None,
                effects: Vec::new(),
                emits: Vec::new(),
                tick_lifecycle: None,
                output_bytes: 0,
            }))
    }
}

include!("modules_split_part1.rs");
include!("modules_split_part2.rs");
