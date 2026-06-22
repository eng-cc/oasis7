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
        pos: pos(0, 0),
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
                assert!(
                    notes
                        .iter()
                        .any(|note| note.contains("blocked by sandbox rule"))
                );
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
                assert!(
                    notes
                        .iter()
                        .any(|note| note.contains("wasm pre-action evaluator failed"))
                );
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

fn empty_micro_depot_input(action_id: ActionId) -> MicroDepotEvalInput {
    MicroDepotEvalInput {
        schema_version: "micro_depot.eval.v1".to_string(),
        module_id: "regional.micro_depot".to_string(),
        module_version: "0.1.0".to_string(),
        action: MicroDepotActionContext {
            action_id,
            kind: MicroDepotActionKind::Repair,
            target_id: "factory-7".to_string(),
            base_cost_class: MicroDepotPressureClass::High,
            base_risk_class: MicroDepotPressureClass::Medium,
            blocker_type: Some("local_parts_gap".to_string()),
        },
        player: MicroDepotPlayerContext {
            account_id: "player-1".to_string(),
            claim_id: "claim-1".to_string(),
        },
        depot: MicroDepotFacilityContext {
            facility_id: "depot-alpha".to_string(),
            status: MicroDepotStatus::Active,
            owner_claim_id: "claim-1".to_string(),
            capacity_class: MicroDepotPressureClass::Medium,
            supported_resource_kinds: vec!["hardware".to_string(), "data".to_string()],
            service_radius_cm: 250_000,
            upkeep_paid: true,
        },
        context: MicroDepotRegionContext {
            distance_to_target_cm: 125_000,
            resource_gap_class: MicroDepotPressureClass::High,
            route_pressure_class: MicroDepotPressureClass::Medium,
            region_pressure_class: MicroDepotPressureClass::Medium,
        },
    }
}

fn micro_depot_output_for(proposal: MicroDepotProposal) -> ModuleOutput {
    ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: vec![ModuleEmit {
            kind: MICRO_DEPOT_PROPOSAL_EMIT_KIND.to_string(),
            payload: serde_json::to_value(proposal).expect("serialize proposal"),
        }],
        tick_lifecycle: None,
        output_bytes: 256,
    }
}

#[derive(Clone, Default)]
struct DynamicMicroDepotSandbox {
    requests: Vec<ModuleCallRequest>,
}

impl ModuleSandbox for DynamicMicroDepotSandbox {
    fn call(&mut self, request: &ModuleCallRequest) -> Result<ModuleOutput, ModuleCallFailure> {
        self.requests.push(request.clone());
        let call_input: ModuleCallInput =
            serde_cbor::from_slice(&request.input).expect("decode module call input");
        let payload = call_input.action.expect("micro depot payload");
        let input: MicroDepotEvalInput =
            serde_cbor::from_slice(&payload).expect("decode micro depot eval input");
        let mut proposal = MicroDepotProposal {
            action_id: input.action.action_id,
            facility_id: input.depot.facility_id.clone(),
            decision: MicroDepotDecision::Applicable,
            cost_delta_class: MicroDepotDeltaClass::MinorDecrease,
            risk_delta_class: MicroDepotDeltaClass::None,
            wait_delta_class: MicroDepotDeltaClass::MinorDecrease,
            blocker_change: Some("repair_parts_gap_reduced".to_string()),
            consumed_resource_classes: vec![MicroDepotConsumedResourceClass {
                resource_kind: "data".to_string(),
                amount_class: MicroDepotPressureClass::Low,
            }],
            explanation_code: "depot_reserved_local_parts".to_string(),
            proposal_hash: String::new(),
        };
        proposal.proposal_hash = compute_micro_depot_proposal_hash(&input, &proposal)
            .expect("compute runtime-compatible proposal hash");
        Ok(micro_depot_output_for(proposal))
    }
}

#[test]
fn micro_depot_wasm_quote_captures_input_and_returns_runtime_validated_preview() {
    let input = empty_micro_depot_input(42);
    let mut proposal = MicroDepotProposal {
        action_id: input.action.action_id,
        facility_id: input.depot.facility_id.clone(),
        decision: MicroDepotDecision::Applicable,
        cost_delta_class: MicroDepotDeltaClass::MinorDecrease,
        risk_delta_class: MicroDepotDeltaClass::None,
        wait_delta_class: MicroDepotDeltaClass::MinorDecrease,
        blocker_change: Some("local_parts_gap_reduced".to_string()),
        consumed_resource_classes: vec![MicroDepotConsumedResourceClass {
            resource_kind: "hardware".to_string(),
            amount_class: MicroDepotPressureClass::Low,
        }],
        explanation_code: "depot_local_stock_available".to_string(),
        proposal_hash: String::new(),
    };
    proposal.proposal_hash = compute_micro_depot_proposal_hash(&input, &proposal)
        .expect("compute canonical proposal hash");
    let sandbox = Arc::new(Mutex::new(CapturingSandbox::new(Ok(
        micro_depot_output_for(proposal.clone()),
    ))));

    let preview = evaluate_micro_depot_quote_with_module(
        &input,
        "regional.micro_depot",
        "hash-micro-depot",
        "evaluate_quote",
        vec![0x00, 0x61, 0x73, 0x6d],
        ModuleLimits::unbounded(),
        Arc::clone(&sandbox),
    )
    .expect("valid module proposal should produce preview");

    assert_eq!(preview.proposal, proposal);
    assert_eq!(preview.module_id, "regional.micro_depot");
    assert_eq!(preview.wasm_hash, "hash-micro-depot");
    assert_eq!(preview.entrypoint, "evaluate_quote");
    assert_eq!(
        preview.effect.cost_delta_class,
        MicroDepotDeltaClass::MinorDecrease
    );
    assert_eq!(
        preview.effect.blocker_change.as_deref(),
        Some("local_parts_gap_reduced")
    );

    let requests = sandbox.lock().expect("lock sandbox").requests.clone();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(request.module_id, "regional.micro_depot");
    assert_eq!(request.wasm_hash, "hash-micro-depot");
    assert_eq!(request.entrypoint, "evaluate_quote");
    assert_eq!(request.wasm_bytes, vec![0x00, 0x61, 0x73, 0x6d].into());

    let call_input: ModuleCallInput = serde_cbor::from_slice(&request.input).expect("decode input");
    assert_eq!(call_input.ctx.origin.kind, "micro_depot_quote");
    assert_eq!(call_input.ctx.origin.id, "42");
    assert_eq!(
        call_input.ctx.stage.as_deref(),
        Some("micro_depot_evaluate_quote")
    );
    let payload = call_input.action.expect("micro depot payload");
    let decoded: MicroDepotEvalInput =
        serde_cbor::from_slice(&payload).expect("decode micro depot input");
    assert_eq!(decoded, input);
}

#[test]
fn micro_depot_wasm_quote_rejects_mismatched_proposal_hash() {
    let input = empty_micro_depot_input(7);
    let proposal = MicroDepotProposal {
        action_id: input.action.action_id,
        facility_id: input.depot.facility_id.clone(),
        decision: MicroDepotDecision::Applicable,
        cost_delta_class: MicroDepotDeltaClass::MajorDecrease,
        risk_delta_class: MicroDepotDeltaClass::None,
        wait_delta_class: MicroDepotDeltaClass::None,
        blocker_change: None,
        consumed_resource_classes: Vec::new(),
        explanation_code: "tampered".to_string(),
        proposal_hash: "sha256:not-the-runtime-hash".to_string(),
    };
    let sandbox = Arc::new(Mutex::new(CapturingSandbox::new(Ok(
        micro_depot_output_for(proposal),
    ))));

    let err = evaluate_micro_depot_quote_with_module(
        &input,
        "regional.micro_depot",
        "hash-micro-depot",
        "evaluate_quote",
        vec![1, 2, 3],
        ModuleLimits::unbounded(),
        Arc::clone(&sandbox),
    )
    .expect_err("mismatched proposal hash must be rejected");

    assert!(err.contains("micro_depot proposal_hash mismatch"));
}

#[test]
fn micro_depot_full_chain_installs_services_and_records_receipt() {
    let mut kernel = WorldKernel::new();
    kernel.submit_action(Action::RegisterLocation {
        location_id: "loc-hub".to_string(),
        name: "Hub".to_string(),
        pos: pos(0, 0),
        profile: LocationProfile::default(),
    });
    kernel.step().expect("register location");
    kernel.submit_action(Action::RegisterAgent {
        agent_id: "agent-builder".to_string(),
        location_id: "loc-hub".to_string(),
    });
    kernel.step().expect("register agent");
    seed_owner_resource(
        &mut kernel,
        ResourceOwner::Agent {
            agent_id: "agent-builder".to_string(),
        },
        ResourceKind::Data,
        5,
    );
    let replay_base_snapshot = kernel.snapshot();

    let sandbox = Arc::new(Mutex::new(DynamicMicroDepotSandbox::default()));
    kernel.set_micro_depot_wasm_module_evaluator(
        "regional.micro_depot",
        "hash-micro-depot",
        "evaluate_quote",
        vec![0x00, 0x61, 0x73, 0x6d],
        ModuleLimits::unbounded(),
        Arc::clone(&sandbox),
    );

    kernel.submit_action(Action::InstallMicroDepot {
        installer_agent_id: "agent-builder".to_string(),
        facility_id: "depot-alpha".to_string(),
        location_id: "loc-hub".to_string(),
        owner_claim_id: "claim-1".to_string(),
        module_id: "regional.micro_depot".to_string(),
        module_version: "0.1.0".to_string(),
        wasm_hash: "hash-micro-depot".to_string(),
        entrypoint: "evaluate_quote".to_string(),
        service_radius_cm: 250_000,
        supported_resource_kinds: vec!["data".to_string()],
    });
    let install_event = kernel.step().expect("install depot");
    match install_event.kind {
        WorldEventKind::MicroDepotInstalled {
            facility_id,
            installer_agent_id,
            location_id,
            ..
        } => {
            assert_eq!(facility_id, "depot-alpha");
            assert_eq!(installer_agent_id, "agent-builder");
            assert_eq!(location_id, "loc-hub");
        }
        other => panic!("unexpected install event: {other:?}"),
    }

    let service_action_id = kernel.submit_action(Action::ServiceMicroDepotRepair {
        agent_id: "agent-builder".to_string(),
        facility_id: "depot-alpha".to_string(),
        target_id: "loc-hub".to_string(),
        base_cost_class: MicroDepotPressureClass::High,
        base_risk_class: MicroDepotPressureClass::Medium,
        blocker_type: Some("repair_parts_gap".to_string()),
    });
    let service_event = kernel.step().expect("service depot repair");

    match service_event.kind {
        WorldEventKind::MicroDepotServiceApplied {
            facility_id,
            action_id,
            agent_id,
            target_id,
            proposal_hash,
            receipt_id,
            consumed_resources,
            explanation_code,
            ..
        } => {
            assert_eq!(facility_id, "depot-alpha");
            assert_eq!(action_id, service_action_id);
            assert_eq!(agent_id, "agent-builder");
            assert_eq!(target_id, "loc-hub");
            assert!(proposal_hash.starts_with("sha256:"));
            assert_eq!(
                receipt_id,
                format!("micro-depot:depot-alpha:{service_action_id}")
            );
            assert_eq!(
                consumed_resources,
                vec![MicroDepotResourceDebit {
                    kind: ResourceKind::Data,
                    amount: 1,
                }]
            );
            assert_eq!(explanation_code, "depot_reserved_local_parts");
        }
        other => panic!("unexpected service event: {other:?}"),
    }

    let depot = kernel
        .model()
        .regional_infrastructure
        .get("depot-alpha")
        .expect("depot state stored");
    assert_eq!(
        depot.last_receipt_id.as_deref(),
        Some(format!("micro-depot:depot-alpha:{service_action_id}").as_str())
    );
    assert!(
        depot
            .last_proposal_hash
            .as_deref()
            .is_some_and(|hash| hash.starts_with("sha256:"))
    );
    assert_eq!(
        kernel
            .model()
            .agents
            .get("agent-builder")
            .expect("agent")
            .resources
            .get(ResourceKind::Data),
        4
    );
    assert_eq!(sandbox.lock().expect("lock sandbox").requests.len(), 1);

    let replayed =
        WorldKernel::replay_from_snapshot(replay_base_snapshot, kernel.journal_snapshot())
            .expect("replay micro depot chain from seeded snapshot");
    assert_eq!(replayed.model(), kernel.model());
}

#[test]
fn micro_depot_registry_logistics_and_player_snapshot_close_loop() {
    let mut kernel = WorldKernel::new();
    kernel.submit_action(Action::RegisterLocation {
        location_id: "loc-hub".to_string(),
        name: "Hub".to_string(),
        pos: pos(0, 0),
        profile: LocationProfile::default(),
    });
    kernel.step().expect("register location");
    kernel.submit_action(Action::RegisterAgent {
        agent_id: "agent-builder".to_string(),
        location_id: "loc-hub".to_string(),
    });
    kernel.step().expect("register agent");
    seed_owner_resource(
        &mut kernel,
        ResourceOwner::Agent {
            agent_id: "agent-builder".to_string(),
        },
        ResourceKind::Data,
        5,
    );

    let wasm_bytes = vec![0x00, 0x61, 0x73, 0x6d, 0x6d, 0x64];
    kernel
        .register_micro_depot_wasm_artifact("hash-micro-depot", wasm_bytes.clone())
        .expect("register micro depot artifact");
    let sandbox = Arc::new(Mutex::new(DynamicMicroDepotSandbox::default()));
    kernel
        .set_micro_depot_wasm_module_from_registry(
            "regional.micro_depot",
            "hash-micro-depot",
            "evaluate_quote",
            ModuleLimits::unbounded(),
            Arc::clone(&sandbox),
        )
        .expect("activate micro depot artifact from registry");

    kernel.submit_action(Action::InstallMicroDepot {
        installer_agent_id: "agent-builder".to_string(),
        facility_id: "depot-alpha".to_string(),
        location_id: "loc-hub".to_string(),
        owner_claim_id: "claim-1".to_string(),
        module_id: "regional.micro_depot".to_string(),
        module_version: "0.1.0".to_string(),
        wasm_hash: "hash-micro-depot".to_string(),
        entrypoint: "evaluate_quote".to_string(),
        service_radius_cm: 250_000,
        supported_resource_kinds: vec!["data".to_string()],
    });
    kernel.step().expect("install depot");

    let logistics_action_id = kernel.submit_action(Action::ServiceMicroDepotLogistics {
        agent_id: "agent-builder".to_string(),
        facility_id: "depot-alpha".to_string(),
        target_id: "loc-hub".to_string(),
        base_cost_class: MicroDepotPressureClass::Medium,
        base_risk_class: MicroDepotPressureClass::Low,
        blocker_type: Some("route_pressure".to_string()),
    });
    let logistics_event = kernel.step().expect("service depot logistics");
    match logistics_event.kind {
        WorldEventKind::MicroDepotServiceApplied {
            action_id,
            action_kind,
            receipt_id,
            ..
        } => {
            assert_eq!(action_id, logistics_action_id);
            assert_eq!(action_kind, "logistics");
            assert_eq!(
                receipt_id,
                format!("micro-depot:depot-alpha:{logistics_action_id}")
            );
        }
        other => panic!("unexpected logistics event: {other:?}"),
    }

    let requests = sandbox.lock().expect("lock sandbox").requests.clone();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].wasm_bytes, wasm_bytes.into());
    let call_input: ModuleCallInput =
        serde_cbor::from_slice(&requests[0].input).expect("decode module call input");
    let input: MicroDepotEvalInput =
        serde_cbor::from_slice(&call_input.action.expect("action payload"))
            .expect("decode micro depot input");
    assert_eq!(input.action.kind, MicroDepotActionKind::Logistics);

    let snapshots = kernel.micro_depot_player_facility_snapshots();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].facility_id, "depot-alpha");
    assert_eq!(snapshots[0].status, "active");
    assert!(
        snapshots[0]
            .available_actions
            .contains(&"service_micro_depot_logistics".to_string())
    );
    assert!(
        snapshots[0]
            .last_receipt_id
            .as_deref()
            .is_some_and(|receipt| receipt.starts_with("micro-depot:depot-alpha:"))
    );
}

#[test]
fn micro_depot_lifecycle_suspend_pay_and_reclaim_replays() {
    let mut kernel = WorldKernel::new();
    kernel.submit_action(Action::RegisterLocation {
        location_id: "loc-hub".to_string(),
        name: "Hub".to_string(),
        pos: pos(0, 0),
        profile: LocationProfile::default(),
    });
    kernel.step().expect("register location");
    kernel.submit_action(Action::RegisterAgent {
        agent_id: "agent-builder".to_string(),
        location_id: "loc-hub".to_string(),
    });
    kernel.step().expect("register agent");
    let replay_base_snapshot = kernel.snapshot();

    kernel.submit_action(Action::InstallMicroDepot {
        installer_agent_id: "agent-builder".to_string(),
        facility_id: "depot-alpha".to_string(),
        location_id: "loc-hub".to_string(),
        owner_claim_id: "claim-1".to_string(),
        module_id: "regional.micro_depot".to_string(),
        module_version: "0.1.0".to_string(),
        wasm_hash: "hash-micro-depot".to_string(),
        entrypoint: "evaluate_quote".to_string(),
        service_radius_cm: 250_000,
        supported_resource_kinds: vec!["data".to_string()],
    });
    kernel.step().expect("install depot");

    kernel.submit_action(Action::SuspendMicroDepot {
        agent_id: "agent-builder".to_string(),
        facility_id: "depot-alpha".to_string(),
    });
    let suspend_event = kernel.step().expect("suspend depot");
    assert!(matches!(
        suspend_event.kind,
        WorldEventKind::MicroDepotSuspended { .. }
    ));
    assert_eq!(
        kernel
            .micro_depot_player_facility_snapshots()
            .pop()
            .expect("snapshot")
            .available_actions,
        vec![
            "pay_micro_depot_upkeep".to_string(),
            "reclaim_micro_depot".to_string()
        ]
    );

    kernel.submit_action(Action::PayMicroDepotUpkeep {
        agent_id: "agent-builder".to_string(),
        facility_id: "depot-alpha".to_string(),
    });
    let upkeep_event = kernel.step().expect("pay upkeep");
    assert!(matches!(
        upkeep_event.kind,
        WorldEventKind::MicroDepotUpkeepPaid { .. }
    ));
    assert_eq!(
        kernel
            .model()
            .regional_infrastructure
            .get("depot-alpha")
            .expect("depot")
            .status,
        "active"
    );

    kernel.submit_action(Action::ReclaimMicroDepot {
        agent_id: "agent-builder".to_string(),
        facility_id: "depot-alpha".to_string(),
    });
    let reclaim_event = kernel.step().expect("reclaim depot");
    assert!(matches!(
        reclaim_event.kind,
        WorldEventKind::MicroDepotReclaimed { .. }
    ));
    assert!(kernel.model().regional_infrastructure.is_empty());

    let replayed =
        WorldKernel::replay_from_snapshot(replay_base_snapshot, kernel.journal_snapshot())
            .expect("replay micro depot lifecycle");
    assert_eq!(replayed.model(), kernel.model());
}
