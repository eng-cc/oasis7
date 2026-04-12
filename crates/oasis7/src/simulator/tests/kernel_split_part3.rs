#[test]
fn schedule_recipe_accepts_smelter_recipe_on_smelter_factory() {
    let mut config = WorldConfig::default();
    config.economy.factory_build_electricity_cost = 0;
    config.economy.factory_build_hardware_cost = 0;
    config.economy.recipe_electricity_cost_per_batch = 0;
    config.economy.recipe_hardware_cost_per_batch = 0;
    let mut kernel = WorldKernel::with_config(config);
    kernel.submit_action(Action::RegisterLocation {
        location_id: "loc-smelter".to_string(),
        name: "smelter-site".to_string(),
        pos: pos(0.0, 0.0),
        profile: LocationProfile::default(),
    });
    kernel.submit_action(Action::RegisterAgent {
        agent_id: "agent-smelter".to_string(),
        location_id: "loc-smelter".to_string(),
    });
    kernel.step_until_empty();

    kernel.submit_action(Action::BuildFactory {
        owner: ResourceOwner::Agent {
            agent_id: "agent-smelter".to_string(),
        },
        location_id: "loc-smelter".to_string(),
        factory_id: "factory.smelter.alpha".to_string(),
        factory_kind: "factory.smelter.mk1".to_string(),
    });
    kernel.step().expect("build smelter factory");

    kernel.submit_action(Action::ScheduleRecipe {
        owner: ResourceOwner::Agent {
            agent_id: "agent-smelter".to_string(),
        },
        factory_id: "factory.smelter.alpha".to_string(),
        recipe_id: "recipe.smelter.iron_ingot".to_string(),
        batches: 1,
    });
    let event = kernel.step().expect("schedule smelter recipe");
    match event.kind {
        WorldEventKind::RecipeScheduled {
            recipe_id,
            finished_product_id,
            ..
        } => {
            assert_eq!(recipe_id, "recipe.smelter.iron_ingot");
            assert_eq!(finished_product_id, "iron_ingot");
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn schedule_recipe_rejects_incompatible_factory_kind() {
    let mut config = WorldConfig::default();
    config.economy.factory_build_electricity_cost = 0;
    config.economy.factory_build_hardware_cost = 0;
    let mut kernel = WorldKernel::with_config(config);
    kernel.submit_action(Action::RegisterLocation {
        location_id: "loc-factory".to_string(),
        name: "factory-site".to_string(),
        pos: pos(0.0, 0.0),
        profile: LocationProfile::default(),
    });
    kernel.submit_action(Action::RegisterAgent {
        agent_id: "agent-builder".to_string(),
        location_id: "loc-factory".to_string(),
    });
    kernel.step_until_empty();

    kernel.submit_action(Action::BuildFactory {
        owner: ResourceOwner::Agent {
            agent_id: "agent-builder".to_string(),
        },
        location_id: "loc-factory".to_string(),
        factory_id: "factory.power.alpha".to_string(),
        factory_kind: "factory.power.radiation.mk1".to_string(),
    });
    kernel.step().expect("build power factory");

    kernel.submit_action(Action::ScheduleRecipe {
        owner: ResourceOwner::Agent {
            agent_id: "agent-builder".to_string(),
        },
        factory_id: "factory.power.alpha".to_string(),
        recipe_id: "recipe.assembler.control_chip".to_string(),
        batches: 1,
    });
    let event = kernel.step().expect("schedule incompatible recipe");
    match event.kind {
        WorldEventKind::ActionRejected { reason } => match reason {
            RejectReason::RuleDenied { notes } => {
                assert_eq!(notes.len(), 1);
                assert!(
                    notes[0].contains("requires factory kind factory.assembler.mk1"),
                    "unexpected note: {}",
                    notes[0]
                );
                assert!(
                    notes[0].contains("factory.power.radiation.mk1"),
                    "unexpected note: {}",
                    notes[0]
                );
            }
            other => panic!("unexpected reject reason: {other:?}"),
        },
        other => panic!("unexpected event: {other:?}"),
    }
}

fn collect_basic_action_sequence(kernel: &mut WorldKernel) -> Vec<WorldEventKind> {
    let mut kinds = Vec::new();

    kernel.submit_action(Action::RegisterLocation {
        location_id: "loc-seq".to_string(),
        name: "seq".to_string(),
        pos: pos(0.0, 0.0),
        profile: LocationProfile::default(),
    });
    kinds.push(kernel.step().expect("register location").kind);

    kernel.submit_action(Action::RegisterAgent {
        agent_id: "agent-seq".to_string(),
        location_id: "loc-seq".to_string(),
    });
    kinds.push(kernel.step().expect("register agent").kind);

    kernel.submit_action(Action::RegisterAgent {
        agent_id: "agent-seq".to_string(),
        location_id: "loc-seq".to_string(),
    });
    kinds.push(kernel.step().expect("reject duplicate agent").kind);

    kinds
}

#[test]
fn kernel_rule_hooks_default_path_keeps_action_behavior() {
    let mut baseline = WorldKernel::new();
    let baseline_kinds = collect_basic_action_sequence(&mut baseline);

    let mut with_noop_hooks = WorldKernel::new();
    with_noop_hooks
        .add_pre_action_rule_hook(|action_id, _, _| KernelRuleDecision::allow(action_id));
    with_noop_hooks.add_post_action_rule_hook(|_, _, _| {});
    let hook_kinds = collect_basic_action_sequence(&mut with_noop_hooks);

    assert_eq!(baseline_kinds, hook_kinds);
}

#[test]
fn kernel_rule_hooks_run_in_registration_order() {
    let mut kernel = WorldKernel::new();
    let trace = Arc::new(Mutex::new(Vec::new()));

    let trace_pre_1 = Arc::clone(&trace);
    kernel.add_pre_action_rule_hook(move |action_id, _, _| {
        trace_pre_1.lock().expect("lock trace").push("pre-1");
        KernelRuleDecision::allow(action_id)
    });

    let trace_pre_2 = Arc::clone(&trace);
    kernel.add_pre_action_rule_hook(move |action_id, _, _| {
        trace_pre_2.lock().expect("lock trace").push("pre-2");
        KernelRuleDecision::allow(action_id)
    });

    let trace_post_1 = Arc::clone(&trace);
    kernel.add_post_action_rule_hook(move |_, _, _| {
        trace_post_1.lock().expect("lock trace").push("post-1");
    });

    let trace_post_2 = Arc::clone(&trace);
    kernel.add_post_action_rule_hook(move |_, _, _| {
        trace_post_2.lock().expect("lock trace").push("post-2");
    });

    kernel.submit_action(Action::RegisterLocation {
        location_id: "loc-hook-order".to_string(),
        name: "hook-order".to_string(),
        pos: pos(0.0, 0.0),
        profile: LocationProfile::default(),
    });
    kernel.step().expect("step with hooks");

    let trace = trace.lock().expect("lock trace");
    assert_eq!(*trace, vec!["pre-1", "pre-2", "post-1", "post-2"]);
}

#[test]
fn kernel_post_action_hook_receives_emitted_event() {
    let mut kernel = WorldKernel::new();
    let captured = Arc::new(Mutex::new(None::<(ActionId, Action, WorldEvent)>));
    let captured_hook = Arc::clone(&captured);

    kernel.add_post_action_rule_hook(move |action_id, action, event| {
        *captured_hook.lock().expect("lock captured") =
            Some((action_id, action.clone(), event.clone()));
    });

    let action = Action::RegisterLocation {
        location_id: "loc-hook-post".to_string(),
        name: "hook-post".to_string(),
        pos: pos(0.0, 0.0),
        profile: LocationProfile::default(),
    };
    let submitted_action_id = kernel.submit_action(action.clone());
    let emitted_event = kernel.step().expect("step with post hook");

    let captured = captured.lock().expect("lock captured");
    let (hook_action_id, hook_action, hook_event) = captured.clone().expect("captured event");
    assert_eq!(hook_action_id, submitted_action_id);
    assert_eq!(hook_action, action);
    assert_eq!(hook_event, emitted_event);
}
