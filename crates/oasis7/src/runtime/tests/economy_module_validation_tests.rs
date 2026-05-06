use super::*;
use crate::runtime::tests::signed_test_artifact_identity;
use crate::runtime::{Manifest, ModuleSubscription, ModuleSubscriptionStage, WorldEvent};
use oasis7_wasm_abi::{
    ModuleCallFailure, ModuleCallInput, ModuleCallRequest, ModuleOutput, ModuleSandbox,
};
use std::collections::VecDeque;

struct CaptureContextSandbox {
    requests: Vec<ModuleCallRequest>,
    outputs: VecDeque<ModuleOutput>,
}

impl CaptureContextSandbox {
    fn with_outputs(outputs: Vec<ModuleOutput>) -> Self {
        Self {
            requests: Vec::new(),
            outputs: outputs.into(),
        }
    }
}

impl ModuleSandbox for CaptureContextSandbox {
    fn call(&mut self, request: &ModuleCallRequest) -> Result<ModuleOutput, ModuleCallFailure> {
        self.requests.push(request.clone());
        Ok(self.outputs.pop_front().unwrap_or(ModuleOutput {
            new_state: None,
            effects: Vec::new(),
            emits: Vec::new(),
            tick_lifecycle: None,
            output_bytes: 0,
        }))
    }
}

fn activate_module_manifest_for_test(world: &mut World, manifest: ModuleManifest) {
    let changes = ModuleChangeSet {
        register: vec![manifest.clone()],
        activate: vec![ModuleActivation {
            module_id: manifest.module_id.clone(),
            version: manifest.version.clone(),
        }],
        ..ModuleChangeSet::default()
    };

    let mut content = serde_json::Map::new();
    content.insert(
        "module_changes".to_string(),
        serde_json::to_value(&changes).unwrap(),
    );
    let manifest_update = Manifest {
        version: 2,
        content: serde_json::Value::Object(content),
    };

    let proposal_id = world
        .propose_manifest_update(manifest_update, "alice")
        .unwrap();
    world.shadow_proposal(proposal_id).unwrap();
    world
        .approve_proposal(proposal_id, "bob", ProposalDecision::Approve)
        .unwrap();
    world.apply_proposal(proposal_id).unwrap();
}

#[test]
fn schedule_recipe_with_module_auto_validates_outputs_before_commit() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");

    world
        .set_material_balance("steel_plate", 10)
        .expect("seed steel");
    world
        .set_material_balance("circuit_board", 2)
        .expect("seed circuits");
    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        spec: factory_spec("factory.recipe.auto_validate", 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("build complete");

    world
        .set_material_balance("motor_mk1", 2)
        .expect("seed motor");
    world
        .set_material_balance("control_chip", 1)
        .expect("seed chip");
    world
        .set_material_balance("chassis_plate", 1)
        .expect("seed chassis");
    world.set_resource_balance(ResourceKind::Electricity, 40);

    activate_pure_module(&mut world, "m4.recipe.logistics_drone", b"recipe-module");
    activate_pure_module(&mut world, "m4.product.logistics_drone", b"product-module");

    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.recipe.auto_validate".to_string(),
        recipe_id: "recipe.assembler.logistics_drone".to_string(),
        module_id: "m4.recipe.logistics_drone".to_string(),
        desired_batches: 1,
        deterministic_seed: 20260214,
    });

    let output = ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: vec![
            ModuleEmit {
                kind: "economy.recipe_execution_plan".to_string(),
                payload: serde_json::to_value(RecipeExecutionPlan::accepted(
                    1,
                    vec![
                        MaterialStack::new("motor_mk1", 2),
                        MaterialStack::new("control_chip", 1),
                        MaterialStack::new("chassis_plate", 1),
                    ],
                    vec![MaterialStack::new("logistics_drone", 1)],
                    vec![MaterialStack::new("assembly_scrap", 1)],
                    10,
                    1,
                ))
                .expect("serialize recipe execution plan"),
            },
            ModuleEmit {
                kind: "economy.product_validation".to_string(),
                payload: serde_json::to_value(ProductValidationDecision::accepted(
                    "logistics_drone",
                    32,
                    true,
                    vec!["fleet_grade".to_string()],
                ))
                .expect("serialize product validation decision"),
            },
        ],
        tick_lifecycle: None,
        output_bytes: 512,
    };
    let mut sandbox = FixedSandbox::succeed(output);
    world
        .step_with_modules(&mut sandbox)
        .expect("start recipe with module");
    assert_eq!(world.pending_recipe_jobs_len(), 1);

    for _ in 0..4 {
        if world.pending_recipe_jobs_len() == 0 {
            break;
        }
        world
            .step_with_modules(&mut sandbox)
            .expect("advance module recipe toward validated completion");
    }
    assert_eq!(world.pending_recipe_jobs_len(), 0);
    assert_eq!(world.material_balance("logistics_drone"), 1);
    assert_eq!(world.material_balance("assembly_scrap"), 1);

    let has_product_validated = world.journal().events.iter().any(|event| {
        matches!(
            &event.body,
            WorldEventBody::Domain(DomainEvent::ProductValidated {
                module_id,
                stack,
                ..
            }) if module_id == "m4.product.logistics_drone" && stack.kind == "logistics_drone"
        )
    });
    assert!(has_product_validated);
}

#[test]
fn schedule_recipe_with_module_blocks_commit_when_product_validation_fails() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");

    world
        .set_material_balance("steel_plate", 10)
        .expect("seed steel");
    world
        .set_material_balance("circuit_board", 2)
        .expect("seed circuits");
    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        spec: factory_spec("factory.recipe.auto_reject", 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("build complete");

    world
        .set_material_balance("motor_mk1", 2)
        .expect("seed motor");
    world
        .set_material_balance("control_chip", 1)
        .expect("seed chip");
    world
        .set_material_balance("chassis_plate", 1)
        .expect("seed chassis");
    world.set_resource_balance(ResourceKind::Electricity, 40);

    activate_pure_module(&mut world, "m4.recipe.logistics_drone", b"recipe-module");
    activate_pure_module(&mut world, "m4.product.logistics_drone", b"product-module");

    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.recipe.auto_reject".to_string(),
        recipe_id: "recipe.assembler.logistics_drone".to_string(),
        module_id: "m4.recipe.logistics_drone".to_string(),
        desired_batches: 1,
        deterministic_seed: 20260214,
    });

    let output = ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: vec![
            ModuleEmit {
                kind: "economy.recipe_execution_plan".to_string(),
                payload: serde_json::to_value(RecipeExecutionPlan::accepted(
                    1,
                    vec![
                        MaterialStack::new("motor_mk1", 2),
                        MaterialStack::new("control_chip", 1),
                        MaterialStack::new("chassis_plate", 1),
                    ],
                    vec![MaterialStack::new("logistics_drone", 1)],
                    vec![MaterialStack::new("assembly_scrap", 1)],
                    10,
                    1,
                ))
                .expect("serialize recipe execution plan"),
            },
            ModuleEmit {
                kind: "economy.product_validation".to_string(),
                payload: serde_json::to_value(ProductValidationDecision::rejected(
                    "logistics_drone",
                    0,
                    true,
                    vec!["fleet_grade".to_string()],
                    vec!["stack exceeds limit".to_string()],
                ))
                .expect("serialize rejected product validation"),
            },
        ],
        tick_lifecycle: None,
        output_bytes: 512,
    };
    let mut sandbox = FixedSandbox::succeed(output);
    world
        .step_with_modules(&mut sandbox)
        .expect("start recipe with module");
    assert_eq!(world.pending_recipe_jobs_len(), 1);

    for _ in 0..4 {
        if world.pending_recipe_jobs_len() == 0 {
            break;
        }
        world
            .step_with_modules(&mut sandbox)
            .expect("advance module recipe toward rejection settlement");
    }
    assert_eq!(world.pending_recipe_jobs_len(), 0);
    assert_eq!(world.material_balance("logistics_drone"), 0);
    assert_eq!(world.material_balance("assembly_scrap"), 0);

    let has_rejected = world.journal().events.iter().any(|event| {
        matches!(
            &event.body,
            WorldEventBody::Domain(DomainEvent::ActionRejected {
                reason: RejectReason::RuleDenied { notes },
                ..
            }) if notes.iter().any(|note| note.contains("stack exceeds limit"))
        )
    });
    assert!(has_rejected);
}

#[test]
fn validate_product_with_module_uses_module_decision() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");
    activate_pure_module(&mut world, "m4.product.logistics_drone", b"product-module");

    world.submit_action(Action::ValidateProductWithModule {
        requester_agent_id: "builder-a".to_string(),
        module_id: "m4.product.logistics_drone".to_string(),
        stack: MaterialStack::new("logistics_drone", 1),
        deterministic_seed: 20260214,
    });

    let output = ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: vec![ModuleEmit {
            kind: "economy.product_validation".to_string(),
            payload: serde_json::to_value(ProductValidationDecision::accepted(
                "logistics_drone",
                32,
                true,
                vec!["fleet_grade".to_string()],
            ))
            .expect("serialize product validation decision"),
        }],
        tick_lifecycle: None,
        output_bytes: 256,
    };
    let mut sandbox = FixedSandbox::succeed(output);
    world
        .step_with_modules(&mut sandbox)
        .expect("validate product with module");

    let validated = world
        .journal()
        .events
        .last()
        .expect("product validated event");
    match &validated.body {
        WorldEventBody::Domain(DomainEvent::ProductValidated {
            requester_agent_id,
            module_id,
            stack,
            stack_limit,
            tradable,
            quality_levels,
            ..
        }) => {
            assert_eq!(requester_agent_id, "builder-a");
            assert_eq!(module_id, "m4.product.logistics_drone");
            assert_eq!(stack.kind, "logistics_drone");
            assert_eq!(stack.amount, 1);
            assert_eq!(*stack_limit, 32);
            assert!(*tradable);
            assert_eq!(quality_levels, &vec!["fleet_grade".to_string()]);
        }
        other => panic!("expected ProductValidated, got {other:?}"),
    }
}

#[test]
fn validate_product_with_module_rejects_when_module_denies() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");
    activate_pure_module(&mut world, "m4.product.logistics_drone", b"product-module");

    world.submit_action(Action::ValidateProductWithModule {
        requester_agent_id: "builder-a".to_string(),
        module_id: "m4.product.logistics_drone".to_string(),
        stack: MaterialStack::new("logistics_drone", 99),
        deterministic_seed: 20260214,
    });

    let output = ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: vec![ModuleEmit {
            kind: "economy.product_validation".to_string(),
            payload: serde_json::to_value(ProductValidationDecision::rejected(
                "logistics_drone",
                32,
                true,
                vec!["fleet_grade".to_string()],
                vec!["stack exceeds limit".to_string()],
            ))
            .expect("serialize rejected product validation"),
        }],
        tick_lifecycle: None,
        output_bytes: 256,
    };
    let mut sandbox = FixedSandbox::succeed(output);
    world
        .step_with_modules(&mut sandbox)
        .expect("module denial should turn into action rejected");

    let rejected = world.journal().events.last().expect("rejection event");
    match &rejected.body {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => match reason {
            RejectReason::RuleDenied { notes } => {
                assert!(notes
                    .iter()
                    .any(|note| note.contains("product module denied: stack exceeds limit")));
            }
            other => panic!("expected RuleDenied, got {other:?}"),
        },
        other => panic!("expected ActionRejected, got {other:?}"),
    }
}

#[test]
fn schedule_recipe_marks_factory_blocked_and_resumes_after_inputs_recover() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register builder");

    world
        .set_material_balance("steel_plate", 20)
        .expect("seed build steel");
    world
        .set_material_balance("circuit_board", 4)
        .expect("seed build circuits");
    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        spec: factory_spec("factory.blocked_resume", 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("finish build");

    let plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("iron_ingot", 2)],
        vec![MaterialStack::new("motor_mk1", 1)],
        Vec::new(),
        1,
        1,
    );
    world.set_resource_balance(ResourceKind::Electricity, 5);
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.blocked_resume".to_string(),
        recipe_id: "recipe.blocked_resume".to_string(),
        plan: plan.clone(),
    });
    world.step().expect("blocked schedule");

    let blocked_event = world.journal().events.last().expect("blocked event");
    match &blocked_event.body {
        WorldEventBody::Domain(DomainEvent::FactoryProductionBlocked {
            factory_id,
            recipe_id,
            blocker_kind,
            blocker_detail,
            ..
        }) => {
            assert_eq!(factory_id, "factory.blocked_resume");
            assert_eq!(recipe_id, "recipe.blocked_resume");
            assert_eq!(blocker_kind, "material_shortage");
            assert!(blocker_detail.contains("iron_ingot"));
        }
        other => panic!("expected FactoryProductionBlocked, got {other:?}"),
    }

    let factory = world
        .state()
        .factories
        .get("factory.blocked_resume")
        .expect("factory state");
    assert_eq!(
        factory.production.status,
        crate::runtime::FactoryProductionStatus::Blocked
    );
    assert_eq!(factory.production.active_jobs, 0);
    assert_eq!(
        factory.production.current_blocker_kind.as_deref(),
        Some("material_shortage")
    );

    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-1"), "iron_ingot", 2)
        .expect("seed recovery iron");
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.blocked_resume".to_string(),
        recipe_id: "recipe.blocked_resume".to_string(),
        plan,
    });
    world.step().expect("resume schedule");

    let resumed_event = world.journal().events.last().expect("resumed event");
    match &resumed_event.body {
        WorldEventBody::Domain(DomainEvent::FactoryProductionResumed {
            factory_id,
            recipe_id,
            previous_blocker_kind,
            ..
        }) => {
            assert_eq!(factory_id, "factory.blocked_resume");
            assert_eq!(recipe_id, "recipe.blocked_resume");
            assert_eq!(previous_blocker_kind.as_deref(), Some("material_shortage"));
        }
        other => panic!("expected FactoryProductionResumed, got {other:?}"),
    }

    let running_factory = world
        .state()
        .factories
        .get("factory.blocked_resume")
        .expect("factory state after resume");
    assert_eq!(
        running_factory.production.status,
        crate::runtime::FactoryProductionStatus::Running
    );
    assert_eq!(running_factory.production.active_jobs, 1);
    assert!(running_factory.production.current_blocker_kind.is_none());

    world.step().expect("complete resumed recipe");
    let completed_factory = world
        .state()
        .factories
        .get("factory.blocked_resume")
        .expect("factory state after completion");
    assert_eq!(
        completed_factory.production.status,
        crate::runtime::FactoryProductionStatus::Idle
    );
    assert_eq!(completed_factory.production.active_jobs, 0);
    assert_eq!(completed_factory.production.completed_jobs, 1);
}

#[test]
fn schedule_recipe_post_action_uses_primary_result_event_before_followup() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");

    world
        .set_material_balance("steel_plate", 20)
        .expect("seed build steel");
    world
        .set_material_balance("circuit_board", 4)
        .expect("seed build circuits");
    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        spec: factory_spec("factory.blocked_resume.post_action", 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("finish build");

    let plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("iron_ingot", 2)],
        vec![MaterialStack::new("motor_mk1", 1)],
        Vec::new(),
        1,
        1,
    );
    world.set_resource_balance(ResourceKind::Electricity, 5);
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.blocked_resume.post_action".to_string(),
        recipe_id: "recipe.blocked_resume".to_string(),
        plan: plan.clone(),
    });
    world.step().expect("block factory production");

    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-1"), "iron_ingot", 2)
        .expect("seed recovery iron");

    let observer_wasm_bytes = b"module-post-action-followup-observer";
    let observer_wasm_hash = util::sha256_hex(observer_wasm_bytes);
    world
        .register_module_artifact(observer_wasm_hash.clone(), observer_wasm_bytes)
        .unwrap();
    activate_module_manifest_for_test(
        &mut world,
        ModuleManifest {
            module_id: "m.post-action.followup-observer".to_string(),
            name: "PostActionFollowupObserver".to_string(),
            version: "0.1.0".to_string(),
            kind: ModuleKind::Pure,
            role: ModuleRole::Domain,
            wasm_hash: observer_wasm_hash.clone(),
            interface_version: "wasm-1".to_string(),
            abi_contract: ModuleAbiContract::default(),
            exports: vec!["call".to_string()],
            subscriptions: vec![ModuleSubscription {
                event_kinds: Vec::new(),
                action_kinds: vec!["action.economy.schedule_recipe".to_string()],
                stage: Some(ModuleSubscriptionStage::PostAction),
                filters: None,
            }],
            required_caps: Vec::new(),
            artifact_identity: Some(signed_test_artifact_identity(observer_wasm_hash.as_str())),
            limits: ModuleLimits {
                max_mem_bytes: 1024,
                max_gas: 10_000,
                max_call_rate: 1,
                max_output_bytes: 1024,
                max_effects: 0,
                max_emits: 0,
            },
        },
    );

    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.blocked_resume.post_action".to_string(),
        recipe_id: "recipe.blocked_resume".to_string(),
        plan,
    });
    let mut sandbox = CaptureContextSandbox::with_outputs(vec![ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: Vec::new(),
        tick_lifecycle: None,
        output_bytes: 0,
    }]);
    world
        .step_with_modules(&mut sandbox)
        .expect("resume schedule with post_action observer");

    assert_eq!(sandbox.requests.len(), 1);
    let observer_input: ModuleCallInput =
        serde_cbor::from_slice(&sandbox.requests[0].input).expect("decode observer input");
    let observed_event: WorldEvent = serde_cbor::from_slice(
        observer_input
            .event
            .as_deref()
            .expect("post_action result event bytes"),
    )
    .expect("decode post_action event");
    match observed_event.body {
        WorldEventBody::Domain(DomainEvent::RecipeStarted {
            factory_id,
            recipe_id,
            ..
        }) => {
            assert_eq!(factory_id, "factory.blocked_resume.post_action");
            assert_eq!(recipe_id, "recipe.blocked_resume");
        }
        other => panic!("expected RecipeStarted, got {other:?}"),
    }

    match &world.journal().events.last().expect("followup event").body {
        WorldEventBody::Domain(DomainEvent::FactoryProductionResumed {
            factory_id,
            recipe_id,
            ..
        }) => {
            assert_eq!(factory_id, "factory.blocked_resume.post_action");
            assert_eq!(recipe_id, "recipe.blocked_resume");
        }
        other => panic!("expected FactoryProductionResumed, got {other:?}"),
    }
}
