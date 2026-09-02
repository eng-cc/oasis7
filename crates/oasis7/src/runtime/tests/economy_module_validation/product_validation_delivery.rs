use super::*;
use crate::runtime::ProductValidationAttemptV1;

#[test]
fn product_validation_recovery_replays_existing_delivery_before_next_output_intent() {
    let mut world = logistics_drone_module_recipe_world("factory.recipe.delivery-checkpoint");
    let observer_wasm = b"product-validation-delivery-observer";
    let observer_hash = crate::runtime::util::sha256_hex(observer_wasm);
    world
        .register_module_artifact(observer_hash.clone(), observer_wasm)
        .expect("register observer artifact");
    activate_module_manifest_for_test(
        &mut world,
        ModuleManifest {
            module_id: "m4.product.validation.observer".to_string(),
            name: "ProductValidationDeliveryObserver".to_string(),
            version: "0.1.0".to_string(),
            kind: ModuleKind::Pure,
            role: ModuleRole::Domain,
            wasm_hash: observer_hash.clone(),
            interface_version: "wasm-1".to_string(),
            abi_contract: ModuleAbiContract::default(),
            exports: vec!["call".to_string()],
            subscriptions: vec![ModuleSubscription {
                event_kinds: vec!["domain.economy.product_validated".to_string()],
                action_kinds: Vec::new(),
                stage: Some(ModuleSubscriptionStage::PostEvent),
                filters: None,
            }],
            required_caps: Vec::new(),
            artifact_identity: Some(signed_test_artifact_identity(observer_hash.as_str())),
            limits: ModuleLimits {
                max_mem_bytes: 1024 * 1024,
                max_gas: 1_000_000,
                max_call_rate: 1024,
                max_output_bytes: 1024 * 1024,
                max_effects: 0,
                max_emits: 0,
            },
        },
    );
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.recipe.delivery-checkpoint".to_string(),
        recipe_id: "recipe.assembler.logistics_drone".to_string(),
        plan: RecipeExecutionPlan::accepted(
            1,
            vec![
                MaterialStack::new("motor_mk1", 2),
                MaterialStack::new("control_chip", 1),
                MaterialStack::new("chassis_plate", 1),
            ],
            vec![
                MaterialStack::new("logistics_drone", 1),
                MaterialStack::new("logistics_drone", 1),
            ],
            Vec::new(),
            10,
            1,
        ),
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world.step().expect("start multi-output recipe");
    let pending = world
        .state()
        .pending_recipe_jobs
        .values()
        .next()
        .expect("pending multi-output recipe")
        .clone();
    let first_stack = pending.produce[0].clone();
    let second_stack = pending.produce[1].clone();
    let first_receipt = ProductValidationReceiptV1 {
        job_id: pending.job_id,
        validation_index: Some(0),
        requester_agent_id: pending.requester_agent_id.clone(),
        module_id: "m4.product.logistics_drone".to_string(),
        stack: first_stack.clone(),
        decision: ProductValidationDecision::accepted(
            "logistics_drone",
            32,
            true,
            vec!["fleet_grade".to_string()],
        ),
        failure_detail: None,
    };
    let mut journal = world.journal().clone();
    let mut next_id = journal.events.last().map_or(1, |event| event.id + 1);
    journal.append(WorldEvent {
        id: next_id,
        time: world.state().time,
        caused_by: None,
        body: WorldEventBody::Domain(DomainEvent::ProductValidationRecorded {
            receipt: first_receipt.clone(),
        }),
    });
    next_id += 1;
    journal.append(WorldEvent {
        id: next_id,
        time: world.state().time,
        caused_by: None,
        body: WorldEventBody::Domain(DomainEvent::ProductValidated {
            requester_agent_id: first_receipt.requester_agent_id.clone(),
            module_id: first_receipt.module_id.clone(),
            stack: first_stack,
            stack_limit: first_receipt.decision.stack_limit,
            tradable: first_receipt.decision.tradable,
            quality_levels: first_receipt.decision.quality_levels.clone(),
            notes: first_receipt.decision.notes.clone(),
        }),
    });
    next_id += 1;
    journal.append(WorldEvent {
        id: next_id,
        time: world.state().time,
        caused_by: None,
        body: WorldEventBody::Domain(DomainEvent::ProductValidationAttemptStarted {
            attempt: ProductValidationAttemptV1 {
                job_id: pending.job_id,
                validation_index: Some(1),
                requester_agent_id: pending.requester_agent_id.clone(),
                module_id: "m4.product.logistics_drone".to_string(),
                stack: second_stack,
            },
        }),
    });
    world =
        World::from_snapshot(world.snapshot(), journal).expect("recover next-output checkpoint");
    world
        .register_module_artifact(observer_hash, observer_wasm)
        .expect("restore observer artifact after recovery");

    let product_validated_before = world
        .journal()
        .events
        .iter()
        .filter(|event| {
            matches!(
                &event.body,
                WorldEventBody::Domain(DomainEvent::ProductValidated { .. })
            )
        })
        .count();
    let mut sandbox = CaptureContextSandbox::with_outputs(Vec::new());
    world
        .step_with_modules(&mut sandbox)
        .expect("recover and route first output before second intent");

    assert_eq!(
        sandbox
            .requests
            .iter()
            .filter(|request| request.module_id == "m4.product.validation.observer")
            .count(),
        1,
        "an existing journaled delivery must be replayed to subscribers"
    );
    assert_eq!(
        sandbox
            .requests
            .iter()
            .filter(|request| request.module_id == "m4.product.logistics_drone")
            .count(),
        0,
        "receipt recovery must not reinvoke the validator module"
    );
    assert_eq!(
        world
            .journal()
            .events
            .iter()
            .filter(|event| {
                matches!(
                    &event.body,
                    WorldEventBody::Domain(DomainEvent::ProductValidated { .. })
                )
            })
            .count(),
        product_validated_before,
        "replaying a journaled delivery must not append a duplicate"
    );
}

#[test]
fn product_validation_receipt_with_existing_delivery_is_not_replayed() {
    let mut world = logistics_drone_module_recipe_world("factory.recipe.delivery-replay");
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.recipe.delivery-replay".to_string(),
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
    let receipt = ProductValidationReceiptV1 {
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
        failure_detail: None,
    };
    let mut journal = world.journal().clone();
    let next_id = journal.events.last().map_or(1, |event| event.id + 1);
    journal.append(WorldEvent {
        id: next_id,
        time: world.state().time,
        caused_by: None,
        body: WorldEventBody::Domain(DomainEvent::ProductValidationRecorded {
            receipt: receipt.clone(),
        }),
    });
    journal.append(WorldEvent {
        id: next_id + 1,
        time: world.state().time,
        caused_by: None,
        body: WorldEventBody::Domain(DomainEvent::ProductValidated {
            requester_agent_id: receipt.requester_agent_id.clone(),
            module_id: receipt.module_id.clone(),
            stack: receipt.stack.clone(),
            stack_limit: receipt.decision.stack_limit,
            tradable: receipt.decision.tradable,
            quality_levels: receipt.decision.quality_levels.clone(),
            notes: receipt.decision.notes.clone(),
        }),
    });
    world = World::from_snapshot(world.snapshot(), journal).expect("recover delivered receipt");
    let before = world
        .journal()
        .events
        .iter()
        .filter(|event| {
            matches!(
                &event.body,
                WorldEventBody::Domain(DomainEvent::ProductValidated { .. })
            )
        })
        .count();
    let mut sandbox = CaptureContextSandbox::with_outputs(Vec::new());
    world
        .step_with_modules(&mut sandbox)
        .expect("settle receipt");
    assert!(sandbox.requests.is_empty());
    assert_eq!(
        world
            .journal()
            .events
            .iter()
            .filter(|event| matches!(
                &event.body,
                WorldEventBody::Domain(DomainEvent::ProductValidated { .. })
            ))
            .count(),
        before,
        "an already delivered validation must not be replayed"
    );
}
