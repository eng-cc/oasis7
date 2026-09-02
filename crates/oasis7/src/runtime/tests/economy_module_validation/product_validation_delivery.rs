use super::*;

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
