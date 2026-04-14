use super::*;
use oasis7_wasm_abi::{
    ModuleCallErrorCode, ModuleCallFailure, ModuleCallInput, ModuleCallRequest, ModuleEmit,
    ModuleLimits, ModuleOutput, ModuleSandbox,
};
use std::sync::{Arc, Mutex};

fn register_location_action(location_id: &str) -> Action {
    Action::RegisterLocation {
        location_id: location_id.to_string(),
        name: format!("name-{location_id}"),
        pos: pos(0.0, 0.0),
        profile: LocationProfile::default(),
    }
}

fn empty_output() -> ModuleOutput {
    ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: Vec::new(),
        tick_lifecycle: None,
        output_bytes: 0,
    }
}

fn decision_output(decision: KernelRuleDecision) -> ModuleOutput {
    ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: vec![ModuleEmit {
            kind: "rule.decision".to_string(),
            payload: serde_json::to_value(decision).expect("serialize decision"),
        }],
        tick_lifecycle: None,
        output_bytes: 128,
    }
}

#[derive(Clone)]
struct CapturingSandbox {
    response: Result<ModuleOutput, ModuleCallFailure>,
    requests: Vec<ModuleCallRequest>,
}

impl CapturingSandbox {
    fn new(response: Result<ModuleOutput, ModuleCallFailure>) -> Self {
        Self {
            response,
            requests: Vec::new(),
        }
    }

    fn set_response(&mut self, response: Result<ModuleOutput, ModuleCallFailure>) {
        self.response = response;
    }
}

impl ModuleSandbox for CapturingSandbox {
    fn call(&mut self, request: &ModuleCallRequest) -> Result<ModuleOutput, ModuleCallFailure> {
        self.requests.push(request.clone());
        self.response.clone()
    }
}

#[test]
fn kernel_wasm_sandbox_bridge_allow_path_captures_encoded_request() {
    let mut kernel = WorldKernel::new();
    kernel.submit_action(register_location_action("loc-ready"));
    kernel.step().expect("seed location");

    let sandbox = Arc::new(Mutex::new(CapturingSandbox::new(Ok(empty_output()))));
    kernel.set_pre_action_wasm_rule_module_evaluator(
        "rule.module",
        "hash-allow",
        "call",
        vec![0x00, 0x61, 0x73, 0x6d],
        ModuleLimits::unbounded(),
        Arc::clone(&sandbox),
    );

    let action = Action::RegisterAgent {
        agent_id: "agent-allow".to_string(),
        location_id: "loc-ready".to_string(),
    };
    let action_id = kernel.submit_action(action.clone());
    let event = kernel.step().expect("allow path should execute action");

    match event.kind {
        WorldEventKind::AgentRegistered {
            agent_id,
            location_id,
            ..
        } => {
            assert_eq!(agent_id, "agent-allow");
            assert_eq!(location_id, "loc-ready");
        }
        other => panic!("unexpected event kind: {other:?}"),
    }

    let requests = sandbox.lock().expect("lock sandbox").requests.clone();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(request.module_id, "rule.module");
    assert_eq!(request.wasm_hash, "hash-allow");
    assert_eq!(request.entrypoint, "call");
    assert_eq!(request.wasm_bytes, vec![0x00, 0x61, 0x73, 0x6d].into());

    let call_input: ModuleCallInput = serde_cbor::from_slice(&request.input).expect("decode input");
    assert_eq!(call_input.ctx.module_id, "rule.module");
    assert_eq!(call_input.ctx.origin.kind, "simulator_action");
    assert_eq!(call_input.ctx.origin.id, action_id.to_string());

    let payload = call_input.action.expect("action payload");
    let decoded: KernelRuleModuleInput = serde_cbor::from_slice(&payload).expect("decode payload");
    assert_eq!(decoded.action_id, action_id);
    assert_eq!(decoded.action, action);
    assert_eq!(decoded.context.time, 1);
    assert_eq!(decoded.context.location_ids, vec!["loc-ready".to_string()]);
    assert!(decoded.context.agent_ids.is_empty());
}

#[test]
fn kernel_wasm_sandbox_bridge_modify_overrides_action() {
    let mut kernel = WorldKernel::new();
    let sandbox = Arc::new(Mutex::new(CapturingSandbox::new(Ok(empty_output()))));
    kernel.set_pre_action_wasm_rule_module_evaluator(
        "rule.module",
        "hash-modify",
        "call",
        vec![1, 2, 3],
        ModuleLimits::unbounded(),
        Arc::clone(&sandbox),
    );

    let action_id = kernel.submit_action(register_location_action("loc-original"));
    sandbox
        .lock()
        .expect("lock sandbox")
        .set_response(Ok(decision_output(KernelRuleDecision::modify(
            action_id,
            register_location_action("loc-overridden"),
        ))));

    let event = kernel.step().expect("modify path emits event");
    match event.kind {
        WorldEventKind::LocationRegistered { location_id, .. } => {
            assert_eq!(location_id, "loc-overridden");
        }
        other => panic!("unexpected event kind: {other:?}"),
    }
    assert!(!kernel.model().locations.contains_key("loc-original"));
    assert!(kernel.model().locations.contains_key("loc-overridden"));
}

#[test]
fn kernel_wasm_sandbox_bridge_deny_rejects_action() {
    let mut kernel = WorldKernel::new();
    let sandbox = Arc::new(Mutex::new(CapturingSandbox::new(Ok(empty_output()))));
    kernel.set_pre_action_wasm_rule_module_evaluator(
        "rule.module",
        "hash-deny",
        "call",
        vec![7, 8, 9],
        ModuleLimits::unbounded(),
        Arc::clone(&sandbox),
    );

    let action_id = kernel.submit_action(register_location_action("loc-denied"));
    sandbox
        .lock()
        .expect("lock sandbox")
        .set_response(Ok(decision_output(KernelRuleDecision::deny(
            action_id,
            vec!["blocked by sandbox rule".to_string()],
        ))));

    let event = kernel.step().expect("deny path emits reject");
    match event.kind {
        WorldEventKind::ActionRejected { reason } => match reason {
            RejectReason::RuleDenied { notes } => {
                assert!(notes
                    .iter()
                    .any(|note| note.contains("blocked by sandbox rule")));
            }
            other => panic!("unexpected reject reason: {other:?}"),
        },
        other => panic!("unexpected event kind: {other:?}"),
    }
    assert!(!kernel.model().locations.contains_key("loc-denied"));
}

#[test]
fn kernel_wasm_sandbox_bridge_failure_is_rejected_with_rule_denied_note() {
    let mut kernel = WorldKernel::new();
    let failure = ModuleCallFailure {
        module_id: "rule.module".to_string(),
        trace_id: "trace-1".to_string(),
        code: ModuleCallErrorCode::Timeout,
        detail: "sandbox timeout".to_string(),
    };
    let sandbox = Arc::new(Mutex::new(CapturingSandbox::new(Err(failure))));
    kernel.set_pre_action_wasm_rule_module_evaluator(
        "rule.module",
        "hash-fail",
        "call",
        vec![5, 5, 5],
        ModuleLimits::unbounded(),
        Arc::clone(&sandbox),
    );

    kernel.submit_action(register_location_action("loc-fail"));
    let event = kernel.step().expect("failure path emits reject");

    match event.kind {
        WorldEventKind::ActionRejected { reason } => match reason {
            RejectReason::RuleDenied { notes } => {
                assert!(notes
                    .iter()
                    .any(|note| note.contains("wasm pre-action evaluator failed")));
                assert!(notes.iter().any(|note| note.contains("module call failed")));
                assert!(notes.iter().any(|note| note.contains("Timeout")));
            }
            other => panic!("unexpected reject reason: {other:?}"),
        },
        other => panic!("unexpected event kind: {other:?}"),
    }
}

#[test]
fn kernel_wasm_artifact_registry_activation_fails_when_hash_missing() {
    let mut kernel = WorldKernel::new();
    let sandbox = Arc::new(Mutex::new(CapturingSandbox::new(Ok(empty_output()))));
    let error = kernel
        .set_pre_action_wasm_rule_module_from_registry(
            "rule.module",
            "hash-missing",
            "call",
            ModuleLimits::unbounded(),
            Arc::clone(&sandbox),
        )
        .expect_err("missing hash should fail activation");
    assert!(error.contains("pre-action wasm artifact missing"));
    assert!(error.contains("hash-missing"));
}

#[test]
fn kernel_wasm_artifact_registry_rejects_conflicting_duplicate_hash() {
    let mut kernel = WorldKernel::new();
    kernel
        .register_pre_action_wasm_rule_artifact("hash-dup", vec![0x00, 0x61, 0x73, 0x6d])
        .expect("first registration should pass");
    kernel
        .register_pre_action_wasm_rule_artifact("hash-dup", vec![0x00, 0x61, 0x73, 0x6d])
        .expect("same bytes idempotent registration should pass");

    let error = kernel
        .register_pre_action_wasm_rule_artifact("hash-dup", vec![0x01, 0x02, 0x03, 0x04])
        .expect_err("same hash with different bytes should fail");
    assert!(error.contains("already registered with different bytes"));
}

#[test]
fn kernel_wasm_artifact_registry_activation_uses_registered_bytes() {
    let mut kernel = WorldKernel::new();
    let wasm_bytes = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00];
    kernel
        .register_pre_action_wasm_rule_artifact("hash-registered", wasm_bytes.clone())
        .expect("register artifact");

    let sandbox = Arc::new(Mutex::new(CapturingSandbox::new(Ok(empty_output()))));
    kernel
        .set_pre_action_wasm_rule_module_from_registry(
            "rule.module",
            "hash-registered",
            "call",
            ModuleLimits::unbounded(),
            Arc::clone(&sandbox),
        )
        .expect("activate module from registry");

    kernel.submit_action(register_location_action("loc-from-registry"));
    let event = kernel.step().expect("activation from registry should run");
    match event.kind {
        WorldEventKind::LocationRegistered { location_id, .. } => {
            assert_eq!(location_id, "loc-from-registry");
        }
        other => panic!("unexpected event kind: {other:?}"),
    }

    let requests = sandbox.lock().expect("lock sandbox").requests.clone();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(request.module_id, "rule.module");
    assert_eq!(request.wasm_hash, "hash-registered");
    assert_eq!(request.entrypoint, "call");
    assert_eq!(request.wasm_bytes, wasm_bytes.into());
}
