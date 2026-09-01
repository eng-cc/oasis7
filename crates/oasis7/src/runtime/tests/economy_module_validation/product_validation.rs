use super::*;
use crate::runtime::ProductValidationReceiptV1;

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
    let site_ledger = MaterialLedgerId::site("site-1");
    assert_eq!(
        world.ledger_material_balance(&site_ledger, "logistics_drone"),
        1
    );
    assert_eq!(
        world.ledger_material_balance(&site_ledger, "assembly_scrap"),
        1
    );

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
fn product_validation_receipt_reuses_decision_after_crash_window_without_module_call() {
    let mut world = logistics_drone_module_recipe_world("factory.recipe.validation-retry");
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.recipe.validation-retry".to_string(),
        recipe_id: "recipe.assembler.logistics_drone".to_string(),
        plan: RecipeExecutionPlan::accepted(
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
        ),
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world.step().expect("start recipe");
    let pending = world
        .state()
        .pending_recipe_jobs
        .values()
        .next()
        .expect("pending recipe");
    let validation = DomainEvent::ProductValidationRecorded {
        receipt: ProductValidationReceiptV1 {
            job_id: pending.job_id,
            validation_index: Some(0),
            requester_agent_id: pending.requester_agent_id.clone(),
            module_id: "m4.product.logistics_drone".to_string(),
            stack: pending.produce[0].clone(),
            decision: ProductValidationDecision::accepted(
                "logistics_drone",
                32,
                true,
                vec!["fleet_grade".to_string()],
            ),
        },
    };
    // Simulate a crash after the validation event committed but before the
    // due-job loop could append RecipeCompleted.
    let mut journal = world.journal().clone();
    let event_id = journal
        .events
        .last()
        .map_or(1, |event| event.id.saturating_add(1));
    journal.append(WorldEvent {
        id: event_id,
        time: world.state().time,
        caused_by: None,
        body: WorldEventBody::Domain(validation),
    });
    world = World::from_snapshot(world.snapshot(), journal)
        .expect("recover after committed validation receipt");

    let mut sandbox = CaptureContextSandbox::with_outputs(Vec::new());
    world
        .step_with_modules(&mut sandbox)
        .expect("retry due recipe from persisted receipt");
    assert!(
        !sandbox
            .requests
            .iter()
            .any(|request| request.module_id == "m4.product.logistics_drone")
    );
    assert_eq!(world.pending_recipe_jobs_len(), 0);
    assert_eq!(
        world.ledger_material_balance(&MaterialLedgerId::site("site-1"), "logistics_drone"),
        1
    );
}

#[test]
fn rejected_product_validation_receipt_reuses_decision_after_crash_window() {
    let mut world = logistics_drone_module_recipe_world("factory.recipe.validation-reject-retry");
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.recipe.validation-reject-retry".to_string(),
        recipe_id: "recipe.assembler.logistics_drone".to_string(),
        plan: RecipeExecutionPlan::accepted(
            1,
            vec![
                MaterialStack::new("motor_mk1", 2),
                MaterialStack::new("control_chip", 1),
                MaterialStack::new("chassis_plate", 1),
            ],
            vec![MaterialStack::new("logistics_drone", 1)],
            Vec::new(),
            10,
            1,
        ),
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world.step().expect("start recipe");
    let pending = world
        .state()
        .pending_recipe_jobs
        .values()
        .next()
        .expect("pending recipe")
        .clone();
    let validation = DomainEvent::ProductValidationRecorded {
        receipt: ProductValidationReceiptV1 {
            job_id: pending.job_id,
            validation_index: Some(0),
            requester_agent_id: pending.requester_agent_id.clone(),
            module_id: "m4.product.logistics_drone".to_string(),
            stack: pending.produce[0].clone(),
            decision: ProductValidationDecision::rejected(
                "logistics_drone",
                0,
                true,
                vec!["fleet_grade".to_string()],
                vec!["stack exceeds limit".to_string()],
            ),
        },
    };
    let mut journal = world.journal().clone();
    let event_id = journal
        .events
        .last()
        .map_or(1, |event| event.id.saturating_add(1));
    journal.append(WorldEvent {
        id: event_id,
        time: world.state().time,
        caused_by: None,
        body: WorldEventBody::Domain(validation),
    });
    world = World::from_snapshot(world.snapshot(), journal)
        .expect("recover after committed rejected validation receipt");
    let mut sandbox = CaptureContextSandbox::with_outputs(Vec::new());
    world
        .step_with_modules(&mut sandbox)
        .expect("retry rejected due recipe from receipt");
    assert!(
        !sandbox
            .requests
            .iter()
            .any(|request| request.module_id == "m4.product.logistics_drone")
    );
    assert_eq!(world.pending_recipe_jobs_len(), 0);
    assert!(
        world
            .state()
            .factory_production_failure_dispositions
            .contains_key(&pending.job_id)
    );
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
    let pending = world
        .state()
        .pending_recipe_jobs
        .values()
        .next()
        .expect("pending recipe commitment")
        .clone();

    for _ in 0..4 {
        if world.pending_recipe_jobs_len() == 0 {
            break;
        }
        world
            .step_with_modules(&mut sandbox)
            .expect("advance module recipe toward rejection settlement");
    }
    assert_eq!(world.pending_recipe_jobs_len(), 0);
    let site_ledger = MaterialLedgerId::site("site-1");
    assert_eq!(
        world.ledger_material_balance(&site_ledger, "logistics_drone"),
        0
    );
    assert_eq!(
        world.ledger_material_balance(&site_ledger, "assembly_scrap"),
        0
    );

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

    assert_eq!(
        world.state().factory_production_failure_dispositions.len(),
        1,
        "a product-validation rejection must persist exactly one disposition"
    );
    let disposition = world
        .state()
        .factory_production_failure_dispositions
        .get(&pending.job_id)
        .expect("product-validation failure disposition");
    assert_eq!(disposition.action_id, pending.job_id);
    assert_eq!(disposition.requester_agent_id, pending.requester_agent_id);
    assert_eq!(disposition.factory_id, pending.factory_id);
    assert_eq!(disposition.recipe_id, pending.recipe_id);
    assert_eq!(disposition.blocker_kind, "product_validation");
    assert_eq!(
        disposition.blocker_detail,
        "product validation rejected for logistics_drone before production settlement"
    );
    assert_eq!(disposition.disposition_kind, "consumed_lost");
    assert_eq!(disposition.consumed_inputs, pending.consume);
    assert_eq!(disposition.lost_inputs, pending.consume);
    assert_eq!(disposition.consumed_power, pending.power_required);
    assert_eq!(disposition.lost_power, pending.power_required);
    assert_eq!(
        disposition.next_action,
        "inspect_product_validation_and_reschedule"
    );
    assert_eq!(disposition.next_recheck, None);

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
fn product_validation_failure_disposition_survives_non_empty_snapshot_roundtrip() {
    let world = settled_product_validation_rejection_world("factory.receipt.snapshot");
    let expected = world
        .state()
        .factory_production_failure_dispositions
        .clone();
    let expected_validation_receipts = world.state().product_validation_receipts.clone();
    assert!(!expected.is_empty());
    assert!(!expected_validation_receipts.is_empty());

    let restored = World::from_snapshot(world.snapshot(), world.journal().clone())
        .expect("restore product-validation disposition snapshot");
    assert_eq!(
        restored.state().factory_production_failure_dispositions,
        expected,
        "non-empty failure disposition must survive snapshot replay"
    );
    assert_eq!(
        restored.state().product_validation_receipts,
        expected_validation_receipts,
        "product validation receipt must survive snapshot replay"
    );
}

#[test]
fn product_validation_failure_disposition_exact_replay_is_idempotent_and_byte_stable() {
    let world = settled_product_validation_rejection_world("factory.receipt.replay");
    let blocker = product_validation_blocker_event(&world);
    let mut replay = world.state().clone();
    let before = serde_json::to_vec(&replay).expect("serialize state before exact replay");

    replay
        .apply_domain_event(&blocker, replay.time)
        .expect("exact product-validation blocker replay");

    assert_eq!(
        serde_json::to_vec(&replay).expect("serialize state after exact replay"),
        before,
        "exact same-ID disposition replay must be byte-stable"
    );
}

#[test]
fn product_validation_failure_disposition_conflicting_same_id_rejects_without_mutation() {
    let world = settled_product_validation_rejection_world("factory.receipt.conflict");
    let mut blocker = product_validation_blocker_event(&world);
    if let DomainEvent::FactoryProductionBlocked { blocker_detail, .. } = &mut blocker {
        blocker_detail.push_str(" (conflict)");
    }

    let mut replay = world.state().clone();
    let before = serde_json::to_vec(&replay).expect("serialize state before conflicting replay");
    let result = replay.apply_domain_event(&blocker, replay.time);
    assert!(
        matches!(result, Err(WorldError::ResourceBalanceInvalid { .. })),
        "conflicting same-ID disposition must fail closed: {result:?}"
    );
    assert_eq!(
        serde_json::to_vec(&replay).expect("serialize state after conflicting replay"),
        before,
        "conflicting same-ID disposition must not mutate state"
    );
}
