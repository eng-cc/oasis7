use super::*;
use crate::runtime::tests::signed_test_artifact_identity;
use crate::runtime::{
    Manifest, ModuleSubscription, ModuleSubscriptionStage, WorldError, WorldEvent,
};
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

fn logistics_drone_module_recipe_world(factory_id: &str) -> World {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");
    world.set_material_balance("steel_plate", 10).unwrap();
    world.set_material_balance("circuit_board", 2).unwrap();
    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        spec: factory_spec(factory_id, 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("build complete");
    for (kind, amount) in [("motor_mk1", 2), ("control_chip", 1), ("chassis_plate", 1)] {
        world.set_material_balance(kind, amount).unwrap();
    }
    world
        .set_agent_resource_balance("builder-a", ResourceKind::Electricity, 40)
        .unwrap();
    world.set_resource_balance(ResourceKind::Electricity, 40);
    activate_pure_module(&mut world, "m4.recipe.logistics_drone", b"recipe-module");
    activate_pure_module(&mut world, "m4.product.logistics_drone", b"product-module");
    world
}

#[test]
fn schedule_recipe_with_module_auto_validates_outputs_before_commit() {
    let mut world = logistics_drone_module_recipe_world("factory.recipe.auto_validate");

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
    let mut world = logistics_drone_module_recipe_world("factory.recipe.auto_reject");

    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.recipe.auto_reject".to_string(),
        recipe_id: "recipe.assembler.logistics_drone".to_string(),
        module_id: "m4.recipe.logistics_drone".to_string(),
        desired_batches: 1,
        deterministic_seed: 20260214,
    });

    let factory_completion_count_before = world
        .state()
        .factories
        .get("factory.recipe.auto_reject")
        .expect("factory state before rejected settlement")
        .production
        .completed_jobs;
    let stable_line_repeat_count_before = world
        .state()
        .factories
        .get("factory.recipe.auto_reject")
        .expect("factory state before rejected settlement")
        .production
        .same_recipe_repeat_count;
    let completed_recipe_jobs_before = world.state().industry_progress.completed_recipe_jobs;
    let industry_stage_before = world.state().industry_progress.stage;

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

    let factory = world
        .state()
        .factories
        .get("factory.recipe.auto_reject")
        .expect("factory state after rejected settlement");
    assert_eq!(
        factory.production.completed_jobs, factory_completion_count_before,
        "rejected product validation must not complete a factory job"
    );
    assert_eq!(
        factory.production.same_recipe_repeat_count, stable_line_repeat_count_before,
        "rejected product validation must not advance stable-line progress"
    );
    assert!(
        factory.production.last_completed_recipe_id.is_none(),
        "rejected product validation must not record a completed recipe"
    );
    assert!(
        factory
            .production
            .last_completed_canonical_snapshot
            .is_none(),
        "rejected product validation must not retain stable-line identity"
    );
    assert_eq!(
        world.state().industry_progress.completed_recipe_jobs,
        completed_recipe_jobs_before,
        "rejected product validation must not advance recipe completion progress"
    );
    assert_eq!(
        world.state().industry_progress.stage,
        industry_stage_before,
        "rejected product validation must leave the industry stage unchanged"
    );

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
fn schedule_recipe_with_module_blocks_atomic_commit_when_byproduct_validation_fails() {
    let mut world = logistics_drone_module_recipe_world("factory.recipe.byproduct-reject");
    activate_pure_module(&mut world, "m4.product.assembly_scrap", b"byproduct-module");

    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.recipe.byproduct-reject".to_string(),
        recipe_id: "recipe.assembler.logistics_drone".to_string(),
        module_id: "m4.recipe.logistics_drone".to_string(),
        desired_batches: 1,
        deterministic_seed: 20260819,
    });

    let mut sandbox = CaptureContextSandbox::with_outputs(vec![
        ModuleOutput {
            new_state: None,
            effects: Vec::new(),
            emits: vec![ModuleEmit {
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
            }],
            tick_lifecycle: None,
            output_bytes: 256,
        },
        ModuleOutput {
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
                .expect("serialize accepted main-product validation"),
            }],
            tick_lifecycle: None,
            output_bytes: 256,
        },
        ModuleOutput {
            new_state: None,
            effects: Vec::new(),
            emits: vec![ModuleEmit {
                kind: "economy.product_validation".to_string(),
                payload: serde_json::to_value(ProductValidationDecision::rejected(
                    "assembly_scrap",
                    0,
                    false,
                    vec!["recyclable".to_string()],
                    vec!["assembly_scrap is ineligible".to_string()],
                ))
                .expect("serialize rejected byproduct validation"),
            }],
            tick_lifecycle: None,
            output_bytes: 256,
        },
    ]);

    world
        .step_with_modules(&mut sandbox)
        .expect("start module recipe");
    assert_eq!(world.pending_recipe_jobs_len(), 1);
    let settlement_journal_start = world.journal().events.len();

    for _ in 0..4 {
        if world.pending_recipe_jobs_len() == 0 {
            break;
        }
        world
            .step_with_modules(&mut sandbox)
            .expect("advance module recipe toward byproduct rejection");
    }

    assert_eq!(world.pending_recipe_jobs_len(), 0);
    assert_eq!(
        world.material_balance("logistics_drone"),
        0,
        "a rejected byproduct must prevent main-product credit"
    );
    assert_eq!(
        world.material_balance("assembly_scrap"),
        0,
        "a rejected byproduct must not be credited"
    );
    assert!(
        !world.journal().events[settlement_journal_start..]
            .iter()
            .any(|event| matches!(
                event.body,
                WorldEventBody::Domain(DomainEvent::RecipeCompleted { .. })
            )),
        "a rejected byproduct must not emit RecipeCompleted"
    );

    let factory = world
        .state()
        .factories
        .get("factory.recipe.byproduct-reject")
        .expect("factory after rejected byproduct validation");
    assert_eq!(
        factory.production.status,
        crate::runtime::FactoryProductionStatus::Blocked
    );
    assert_eq!(
        factory.production.current_blocker_kind.as_deref(),
        Some("product_validation")
    );
    assert!(
        factory
            .production
            .current_blocker_detail
            .as_deref()
            .is_some_and(|detail| detail.contains("assembly_scrap")),
        "product-validation blocker must identify the rejected byproduct"
    );
    assert!(
        world.journal().events[settlement_journal_start..]
            .iter()
            .any(|event| matches!(
                &event.body,
                WorldEventBody::Domain(DomainEvent::ActionRejected {
                    reason: RejectReason::RuleDenied { notes },
                    ..
                }) if notes.iter().any(|note| note.contains("assembly_scrap is ineligible"))
            ))
    );

    let validation_modules: Vec<_> = sandbox.requests[1..]
        .iter()
        .map(|request| request.module_id.as_str())
        .collect();
    assert_eq!(
        validation_modules,
        vec!["m4.product.logistics_drone", "m4.product.assembly_scrap"],
        "validation order must be main outputs followed by byproducts"
    );
}

#[test]
fn industrial_integrity_schedule_recipe_with_module_rejects_plan_over_requested_batches_before_sink()
 {
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
    world
        .set_material_balance("iron_ingot", 1)
        .expect("seed recipe input");
    world.set_resource_balance(ResourceKind::Electricity, 40);
    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        spec: factory_spec("factory.recipe.batch-bound", 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("build complete");

    activate_pure_module(
        &mut world,
        "m4.recipe.batch-bound",
        b"batch-bound-recipe-module",
    );
    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.recipe.batch-bound".to_string(),
        recipe_id: "recipe.batch-bound".to_string(),
        module_id: "m4.recipe.batch-bound".to_string(),
        desired_batches: 1,
        deterministic_seed: 20260214,
    });

    let input_before = world.material_balance("iron_ingot");
    let power_before = world.resource_balance(ResourceKind::Electricity);
    let journal_start = world.journal().events.len();
    let output = ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: vec![ModuleEmit {
            kind: "economy.recipe_execution_plan".to_string(),
            payload: serde_json::to_value(RecipeExecutionPlan::accepted(
                2,
                vec![MaterialStack::new("iron_ingot", 1)],
                vec![MaterialStack::new("gear", 2)],
                Vec::new(),
                10,
                1,
            ))
            .expect("serialize over-accepted recipe plan"),
        }],
        tick_lifecycle: None,
        output_bytes: 256,
    };
    let mut sandbox = FixedSandbox::succeed(output);
    world
        .step_with_modules(&mut sandbox)
        .expect("over-accepted recipe plan should produce a rejection event");

    assert_eq!(world.pending_recipe_jobs_len(), 0);
    assert_eq!(world.material_balance("iron_ingot"), input_before);
    assert_eq!(
        world.resource_balance(ResourceKind::Electricity),
        power_before,
        "over-accepted plan must reject before power sink"
    );
    assert!(
        !world.journal().events[journal_start..].iter().any(|event| {
            matches!(
                &event.body,
                WorldEventBody::Domain(DomainEvent::RecipeStarted { .. })
            )
        })
    );
    assert!(world.journal().events[journal_start..].iter().any(|event| {
        matches!(
            &event.body,
            WorldEventBody::Domain(DomainEvent::ActionRejected {
                reason: RejectReason::RuleDenied { notes },
                ..
            }) if notes.iter().any(|note| {
                note.contains("accepted_batches") || note.contains("desired_batches")
            })
        )
    }));
}

#[test]
fn industrial_integrity_product_validation_blocker_rejects_tampered_identity_before_mutation() {
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
    world
        .set_material_balance("iron_ingot", 1)
        .expect("seed recipe input");
    world
        .set_agent_resource_balance("builder-a", ResourceKind::Electricity, 40)
        .expect("seed builder electricity");
    world.set_resource_balance(ResourceKind::Electricity, 40);
    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        spec: factory_spec("factory.recipe.identity-guard", 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("build complete");

    activate_pure_module(
        &mut world,
        "m4.recipe.identity-guard",
        b"identity-guard-recipe-module",
    );
    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.recipe.identity-guard".to_string(),
        recipe_id: "recipe.identity-guard".to_string(),
        module_id: "m4.recipe.identity-guard".to_string(),
        desired_batches: 1,
        deterministic_seed: 20260214,
    });

    let output = ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: vec![ModuleEmit {
            kind: "economy.recipe_execution_plan".to_string(),
            payload: serde_json::to_value(RecipeExecutionPlan::accepted(
                1,
                vec![MaterialStack::new("iron_ingot", 1)],
                vec![MaterialStack::new("gear", 1)],
                Vec::new(),
                10,
                1,
            ))
            .expect("serialize identity-guard recipe plan"),
        }],
        tick_lifecycle: None,
        output_bytes: 256,
    };
    let mut sandbox = FixedSandbox::succeed(output);
    world
        .step_with_modules(&mut sandbox)
        .expect("start module recipe");
    let pending = world
        .state()
        .pending_recipe_jobs
        .values()
        .next()
        .expect("pending module recipe")
        .clone();

    let mut replay = world.state().clone();
    let before = serde_json::to_vec(&replay).expect("serialize state before tampered blocker");
    let event = DomainEvent::FactoryProductionBlocked {
        action_id: pending.job_id,
        requester_agent_id: "forged-requester".to_string(),
        factory_id: "forged-factory".to_string(),
        recipe_id: "forged-recipe".to_string(),
        blocker_kind: "product_validation".to_string(),
        blocker_detail: "forged product validation disposition".to_string(),
    };

    let result = replay.apply_domain_event(&event, replay.time);
    assert!(
        matches!(result, Err(WorldError::ResourceBalanceInvalid { .. })),
        "tampered product-validation blocker must fail closed: {result:?}"
    );
    assert_eq!(
        serde_json::to_vec(&replay).expect("serialize state after tampered blocker"),
        before,
        "tampered blocker must not mutate serialized world state"
    );
    assert_eq!(
        replay.pending_recipe_jobs.get(&pending.job_id),
        Some(&pending),
        "tampered blocker must retain the pending recipe commitment"
    );
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
    let preview_source = world
        .state()
        .latest_product_validation
        .as_ref()
        .expect("accepted validation is retained for the player preview");
    assert_eq!(preview_source.product_id, "logistics_drone");
    assert!(preview_source.tradable);
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
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => {
            match reason {
                RejectReason::RuleDenied { notes } => {
                    assert!(notes
                    .iter()
                    .any(|note| note.contains("product module denied: stack exceeds limit")));
                }
                other => panic!("expected RuleDenied, got {other:?}"),
            }
        }
        other => panic!("expected ActionRejected, got {other:?}"),
    }
}

#[test]
fn product_validation_quote_previews_eligible_product_without_mutating_world() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");

    let journal_len_before_quote = world.journal().events.len();
    let stage_before_quote = world.state().industry_progress.stage;
    let quote = world
        .product_validation_quote(
            "builder-a",
            "m4.product.logistics_drone",
            &MaterialStack::new("logistics_drone", 1),
            20260214,
        )
        .expect("eligible product has a deterministic pre-submit quote");

    assert_eq!(world.journal().events.len(), journal_len_before_quote);
    assert_eq!(world.state().industry_progress.stage, stage_before_quote);
    assert_eq!(quote.product_id, "logistics_drone");
    assert_eq!(quote.product_role, "explore");
    assert!(quote.tradable);
    assert_eq!(quote.stage_before, "bootstrap");
    assert_eq!(quote.stage_after, "bootstrap");
    assert!(!quote.unlock_or_value_class.is_empty());
    assert!(!quote.recommended_action.is_empty());
    assert!(quote.submission_allowed);
    assert_eq!(
        quote,
        world
            .product_validation_quote(
                "builder-a",
                "m4.product.logistics_drone",
                &MaterialStack::new("logistics_drone", 1),
                20260214,
            )
            .expect("same input has the same quote"),
    );
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
    world
        .set_agent_resource_balance("builder-a", ResourceKind::Electricity, 5)
        .expect("seed builder electricity");
    world.set_resource_balance(ResourceKind::Electricity, 5);
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.blocked_resume".to_string(),
        recipe_id: "recipe.blocked_resume".to_string(),
        plan: plan.clone(),
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
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
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
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
    world
        .set_agent_resource_balance("builder-a", ResourceKind::Electricity, 5)
        .expect("seed builder electricity");
    world.set_resource_balance(ResourceKind::Electricity, 5);
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.blocked_resume.post_action".to_string(),
        recipe_id: "recipe.blocked_resume".to_string(),
        plan: plan.clone(),
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
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
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
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

#[test]
fn non_owner_schedule_recipe_with_module_rejects_before_module_plan_or_sink() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register owner");
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-b".to_string(),
        pos: pos(1, 0),
    });
    world.step().expect("register non-owner");
    world
        .set_material_balance("steel_plate", 10)
        .expect("seed steel");
    world
        .set_material_balance("circuit_board", 2)
        .expect("seed circuits");
    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        spec: factory_spec("factory.recipe.module-owner", 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("complete build");
    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-1"), "iron_ingot", 2)
        .expect("seed recipe input");
    world.set_resource_balance(ResourceKind::Electricity, 30);
    activate_pure_module(
        &mut world,
        "m4.recipe.module-owner",
        b"recipe-module-owner-guard",
    );
    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-b".to_string(),
        factory_id: "factory.recipe.module-owner".to_string(),
        recipe_id: "recipe.module-owner".to_string(),
        module_id: "m4.recipe.module-owner".to_string(),
        desired_batches: 1,
        deterministic_seed: 20260818,
    });

    let input_before =
        world.ledger_material_balance(&MaterialLedgerId::site("site-1"), "iron_ingot");
    let power_before = world.resource_balance(ResourceKind::Electricity);
    let journal_start = world.journal().events.len();
    let mut sandbox = CaptureContextSandbox::with_outputs(vec![ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: vec![ModuleEmit {
            kind: "economy.recipe_execution_plan".to_string(),
            payload: serde_json::to_value(RecipeExecutionPlan::accepted(
                1,
                vec![MaterialStack::new("iron_ingot", 2)],
                vec![MaterialStack::new("gear", 1)],
                Vec::new(),
                5,
                1,
            ))
            .expect("serialize owner-guard recipe plan"),
        }],
        tick_lifecycle: None,
        output_bytes: 256,
    }]);

    world
        .step_with_modules(&mut sandbox)
        .expect("reject non-owner module schedule");

    assert!(
        sandbox.requests.is_empty(),
        "non-owner schedule must be rejected before invoking the recipe module"
    );
    assert_eq!(world.pending_recipe_jobs_len(), 0);
    assert_eq!(
        world.ledger_material_balance(&MaterialLedgerId::site("site-1"), "iron_ingot"),
        input_before,
        "non-owner module schedule must not sink input"
    );
    assert_eq!(
        world.resource_balance(ResourceKind::Electricity),
        power_before,
        "non-owner module schedule must not sink power"
    );
    assert!(world.journal().events[journal_start..].iter().any(|event| {
        matches!(
            &event.body,
            WorldEventBody::Domain(DomainEvent::ActionRejected {
                reason: RejectReason::RuleDenied { .. },
                ..
            })
        )
    }));
}
