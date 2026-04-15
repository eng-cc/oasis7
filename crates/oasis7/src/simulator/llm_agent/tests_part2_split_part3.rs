#[test]
fn llm_agent_segments_move_agent_when_target_distance_exceeds_limit() {
    let client = MockClient {
        output: Some(r#"{"decision":"move_agent","to":"loc-factory"}"#.to_string()),
        err: None,
    };
    let mut behavior = LlmAgentBehavior::new("agent-1", base_config(), client);

    let mut observation = make_observation();
    observation.visible_locations = vec![
        ObservedLocation {
            location_id: "loc-home".to_string(),
            name: "home".to_string(),
            pos: GeoPos {
                x_cm: 0.0,
                y_cm: 0.0,
                z_cm: 0.0,
            },
            profile: Default::default(),
            distance_cm: 0,
        },
        ObservedLocation {
            location_id: "loc-relay".to_string(),
            name: "relay".to_string(),
            pos: GeoPos {
                x_cm: 900_000.0,
                y_cm: 0.0,
                z_cm: 0.0,
            },
            profile: Default::default(),
            distance_cm: 900_000,
        },
        ObservedLocation {
            location_id: "loc-factory".to_string(),
            name: "factory".to_string(),
            pos: GeoPos {
                x_cm: 2_500_000.0,
                y_cm: 0.0,
                z_cm: 0.0,
            },
            profile: Default::default(),
            distance_cm: 2_500_000,
        },
    ];

    let decision = behavior.decide(&observation);
    assert_eq!(
        decision,
        AgentDecision::Act(Action::MoveAgent {
            agent_id: "agent-1".to_string(),
            to: "loc-relay".to_string(),
        })
    );

    let trace = behavior.take_decision_trace().expect("trace");
    assert!(trace.llm_step_trace.iter().any(|step| step
        .output_summary
        .contains("segmented by distance guardrail")));
}

#[test]
fn llm_agent_uses_relay_fallback_after_move_distance_exceeded_history() {
    let client = SequenceMockClient::new(vec![
        r#"{"decision":"move_agent","to":"loc-far"}"#.to_string(),
        r#"{"decision":"move_agent","to":"loc-far"}"#.to_string(),
    ]);
    let mut behavior = LlmAgentBehavior::new("agent-1", base_config(), client);

    let mut observation = make_observation();
    observation.visible_locations = vec![
        ObservedLocation {
            location_id: "loc-home".to_string(),
            name: "home".to_string(),
            pos: GeoPos {
                x_cm: 0.0,
                y_cm: 0.0,
                z_cm: 0.0,
            },
            profile: Default::default(),
            distance_cm: 0,
        },
        ObservedLocation {
            location_id: "loc-relay".to_string(),
            name: "relay".to_string(),
            pos: GeoPos {
                x_cm: 900_000.0,
                y_cm: 0.0,
                z_cm: 0.0,
            },
            profile: Default::default(),
            distance_cm: 900_000,
        },
    ];

    let first = behavior.decide(&observation);
    assert_eq!(
        first,
        AgentDecision::Act(Action::MoveAgent {
            agent_id: "agent-1".to_string(),
            to: "loc-far".to_string(),
        })
    );
    let _ = behavior.take_decision_trace();

    behavior.on_action_result(&ActionResult {
        action: Action::MoveAgent {
            agent_id: "agent-1".to_string(),
            to: "loc-far".to_string(),
        },
        action_id: 611,
        success: false,
        event: WorldEvent {
            id: 711,
            time: 150,
            kind: WorldEventKind::ActionRejected {
                reason: RejectReason::MoveDistanceExceeded {
                    distance_cm: 1_800_000,
                    max_distance_cm: 1_000_000,
                },
            },
            runtime_event: None,
        },
    });

    observation.time = 151;
    let second = behavior.decide(&observation);
    assert_eq!(
        second,
        AgentDecision::Act(Action::MoveAgent {
            agent_id: "agent-1".to_string(),
            to: "loc-relay".to_string(),
        })
    );

    let trace = behavior.take_decision_trace().expect("trace");
    assert!(trace.llm_step_trace.iter().any(|step| step
        .output_summary
        .contains("fallback relay after move_distance_exceeded")));
}

#[test]
fn llm_agent_hard_switches_schedule_recipe_to_next_uncovered_recipe() {
    let client = MockClient {
        output: Some(
            r#"{"decision":"schedule_recipe","owner":"self","factory_id":"factory.alpha","recipe_id":"recipe.assembler.control_chip","batches":1}"#.to_string(),
        ),
        err: None,
    };
    let mut behavior = LlmAgentBehavior::new("agent-1", base_config(), client);
    seed_known_factory(
        &mut behavior,
        "factory.alpha",
        "factory.smelter.mk1",
        "loc-home",
    );

    behavior.on_action_result(&ActionResult {
        action: Action::ScheduleRecipe {
            owner: ResourceOwner::Agent {
                agent_id: "agent-1".to_string(),
            },
            factory_id: "factory.alpha".to_string(),
            recipe_id: "recipe.assembler.control_chip".to_string(),
            batches: 1,
        },
        action_id: 520,
        success: true,
        event: WorldEvent {
            id: 620,
            time: 120,
            kind: WorldEventKind::RecipeScheduled {
                owner: ResourceOwner::Agent {
                    agent_id: "agent-1".to_string(),
                },
                factory_id: "factory.alpha".to_string(),
                recipe_id: "recipe.assembler.control_chip".to_string(),
                batches: 1,
                electricity_cost: 6,
                hardware_cost: 2,
                data_output: 1,
                finished_product_id: "product.component.control_chip".to_string(),
                finished_product_units: 1,
            },
            runtime_event: None,
        },
    });

    let mut observation = make_observation();
    observation
        .self_resources
        .add(ResourceKind::Data, 24)
        .expect("add test hardware");
    observation
        .self_resources
        .add(ResourceKind::Electricity, 100)
        .expect("add test electricity");
    observation
        .self_resources
        .add(ResourceKind::Electricity, 100)
        .expect("add test electricity");

    let decision = behavior.decide(&observation);
    assert_eq!(
        decision,
        AgentDecision::Act(Action::ScheduleRecipe {
            owner: ResourceOwner::Agent {
                agent_id: "agent-1".to_string(),
            },
            factory_id: "factory.alpha".to_string(),
            recipe_id: "recipe.smelter.iron_ingot".to_string(),
            batches: 1,
        })
    );

    let trace = behavior.take_decision_trace().expect("trace");
    assert!(!trace.llm_step_trace.is_empty());
}

#[test]
fn llm_agent_keeps_hard_switch_within_current_factory_kind() {
    let client = MockClient {
        output: Some(
            r#"{"decision":"schedule_recipe","owner":"self","factory_id":"factory.alpha","recipe_id":"recipe.assembler.control_chip","batches":1}"#.to_string(),
        ),
        err: None,
    };
    let mut behavior = LlmAgentBehavior::new("agent-1", base_config(), client);
    seed_known_factory(
        &mut behavior,
        "factory.alpha",
        "factory.assembler.mk1",
        "loc-home",
    );

    behavior.on_action_result(&ActionResult {
        action: Action::ScheduleRecipe {
            owner: ResourceOwner::Agent {
                agent_id: "agent-1".to_string(),
            },
            factory_id: "factory.alpha".to_string(),
            recipe_id: "recipe.assembler.control_chip".to_string(),
            batches: 1,
        },
        action_id: 560,
        success: true,
        event: WorldEvent {
            id: 660,
            time: 130,
            kind: WorldEventKind::RecipeScheduled {
                owner: ResourceOwner::Agent {
                    agent_id: "agent-1".to_string(),
                },
                factory_id: "factory.alpha".to_string(),
                recipe_id: "recipe.assembler.control_chip".to_string(),
                batches: 1,
                electricity_cost: 6,
                hardware_cost: 2,
                data_output: 1,
                finished_product_id: "product.component.control_chip".to_string(),
                finished_product_units: 1,
            },
            runtime_event: None,
        },
    });

    let mut observation = make_observation();
    observation
        .self_resources
        .add(ResourceKind::Data, 24)
        .expect("add test hardware");
    observation
        .self_resources
        .add(ResourceKind::Electricity, 100)
        .expect("add test electricity");

    let decision = behavior.decide(&observation);
    assert_eq!(
        decision,
        AgentDecision::Act(Action::ScheduleRecipe {
            owner: ResourceOwner::Agent {
                agent_id: "agent-1".to_string(),
            },
            factory_id: "factory.alpha".to_string(),
            recipe_id: "recipe.assembler.gear".to_string(),
            batches: 1,
        })
    );
}

#[test]
fn llm_agent_hands_off_coverage_to_known_assembler_after_last_smelter_recipe() {
    let client = MockClient {
        output: Some(
            r#"{"decision":"schedule_recipe","owner":"self","factory_id":"factory.smelter.1","recipe_id":"recipe.smelter.polymer_resin","batches":1}"#.to_string(),
        ),
        err: None,
    };
    let mut behavior = LlmAgentBehavior::new("agent-1", base_config(), client);
    seed_known_factory(
        &mut behavior,
        "factory.smelter.1",
        "factory.smelter.mk1",
        "loc-home",
    );
    seed_known_factory(
        &mut behavior,
        "factory.assembler.1",
        "factory.assembler.mk1",
        "loc-home",
    );

    for (offset, recipe_id, finished_product_id) in [
        (0_u64, "recipe.smelter.iron_ingot", "iron_ingot"),
        (1_u64, "recipe.smelter.copper_wire", "copper_wire"),
        (2_u64, "recipe.smelter.polymer_resin", "polymer_resin"),
        (3_u64, "recipe.smelter.alloy_plate", "alloy_plate"),
    ] {
        behavior.on_action_result(&ActionResult {
            action: Action::ScheduleRecipe {
                owner: ResourceOwner::Agent {
                    agent_id: "agent-1".to_string(),
                },
                factory_id: "factory.smelter.1".to_string(),
                recipe_id: recipe_id.to_string(),
                batches: 1,
            },
            action_id: 700 + offset,
            success: true,
            event: WorldEvent {
                id: 800 + offset,
                time: 200 + offset,
                kind: WorldEventKind::RecipeScheduled {
                    owner: ResourceOwner::Agent {
                        agent_id: "agent-1".to_string(),
                    },
                    factory_id: "factory.smelter.1".to_string(),
                    recipe_id: recipe_id.to_string(),
                    batches: 1,
                    electricity_cost: 6,
                    hardware_cost: 2,
                    data_output: 1,
                    finished_product_id: finished_product_id.to_string(),
                    finished_product_units: 1,
                },
                runtime_event: None,
            },
        });
    }

    let mut observation = make_observation();
    observation
        .self_resources
        .add(ResourceKind::Data, 24)
        .expect("add test hardware");
    observation
        .self_resources
        .add(ResourceKind::Electricity, 100)
        .expect("add test electricity");

    let decision = behavior.decide(&observation);
    assert_eq!(
        decision,
        AgentDecision::Act(Action::ScheduleRecipe {
            owner: ResourceOwner::Agent {
                agent_id: "agent-1".to_string(),
            },
            factory_id: "factory.assembler.1".to_string(),
            recipe_id: "recipe.assembler.gear".to_string(),
            batches: 1,
        })
    );
}

#[test]
fn llm_agent_moves_before_cross_factory_coverage_handoff() {
    let client = MockClient {
        output: Some(
            r#"{"decision":"schedule_recipe","owner":"self","factory_id":"factory.smelter.1","recipe_id":"recipe.smelter.polymer_resin","batches":1}"#.to_string(),
        ),
        err: None,
    };
    let mut behavior = LlmAgentBehavior::new("agent-1", base_config(), client);
    seed_known_factory(
        &mut behavior,
        "factory.smelter.1",
        "factory.smelter.mk1",
        "loc-smelter",
    );
    seed_known_factory(
        &mut behavior,
        "factory.assembler.1",
        "factory.assembler.mk1",
        "loc-assembler",
    );

    for (offset, recipe_id, finished_product_id) in [
        (0_u64, "recipe.smelter.iron_ingot", "iron_ingot"),
        (1_u64, "recipe.smelter.copper_wire", "copper_wire"),
        (2_u64, "recipe.smelter.polymer_resin", "polymer_resin"),
    ] {
        behavior.on_action_result(&ActionResult {
            action: Action::ScheduleRecipe {
                owner: ResourceOwner::Agent {
                    agent_id: "agent-1".to_string(),
                },
                factory_id: "factory.smelter.1".to_string(),
                recipe_id: recipe_id.to_string(),
                batches: 1,
            },
            action_id: 900 + offset,
            success: true,
            event: WorldEvent {
                id: 1_000 + offset,
                time: 230 + offset,
                kind: WorldEventKind::RecipeScheduled {
                    owner: ResourceOwner::Agent {
                        agent_id: "agent-1".to_string(),
                    },
                    factory_id: "factory.smelter.1".to_string(),
                    recipe_id: recipe_id.to_string(),
                    batches: 1,
                    electricity_cost: 6,
                    hardware_cost: 2,
                    data_output: 1,
                    finished_product_id: finished_product_id.to_string(),
                    finished_product_units: 1,
                },
                runtime_event: None,
            },
        });
    }

    let mut observation = make_observation();
    observation.visible_locations = vec![
        ObservedLocation {
            location_id: "loc-smelter".to_string(),
            name: "smelter".to_string(),
            pos: GeoPos {
                x_cm: 0.0,
                y_cm: 0.0,
                z_cm: 0.0,
            },
            profile: Default::default(),
            distance_cm: 0,
        },
        ObservedLocation {
            location_id: "loc-assembler".to_string(),
            name: "assembler".to_string(),
            pos: GeoPos {
                x_cm: 500.0,
                y_cm: 0.0,
                z_cm: 0.0,
            },
            profile: Default::default(),
            distance_cm: 500,
        },
    ];
    observation
        .self_resources
        .add(ResourceKind::Data, 24)
        .expect("add test hardware");
    observation
        .self_resources
        .add(ResourceKind::Electricity, 100)
        .expect("add test electricity");

    let decision = behavior.decide(&observation);
    assert_eq!(
        decision,
        AgentDecision::Act(Action::ScheduleRecipe {
            owner: ResourceOwner::Agent {
                agent_id: "agent-1".to_string(),
            },
            factory_id: "factory.smelter.1".to_string(),
            recipe_id: "recipe.smelter.alloy_plate".to_string(),
            batches: 1,
        })
    );
}

#[test]
fn llm_agent_hard_switches_with_factory_kind_inferred_from_recipe_hint() {
    let client = MockClient {
        output: Some(
            r#"{"decision":"schedule_recipe","owner":"self","factory_id":"factory.alpha","recipe_id":"recipe.assembler.control_chip","batches":1}"#.to_string(),
        ),
        err: None,
    };
    let mut behavior = LlmAgentBehavior::new("agent-1", base_config(), client);
    behavior.remember_factory_location_hint("factory.alpha", "loc-home", None);

    behavior.on_action_result(&ActionResult {
        action: Action::ScheduleRecipe {
            owner: ResourceOwner::Agent {
                agent_id: "agent-1".to_string(),
            },
            factory_id: "factory.alpha".to_string(),
            recipe_id: "recipe.assembler.control_chip".to_string(),
            batches: 1,
        },
        action_id: 1_060,
        success: true,
        event: WorldEvent {
            id: 1_160,
            time: 240,
            kind: WorldEventKind::RecipeScheduled {
                owner: ResourceOwner::Agent {
                    agent_id: "agent-1".to_string(),
                },
                factory_id: "factory.alpha".to_string(),
                recipe_id: "recipe.assembler.control_chip".to_string(),
                batches: 1,
                electricity_cost: 6,
                hardware_cost: 2,
                data_output: 1,
                finished_product_id: "control_chip".to_string(),
                finished_product_units: 1,
            },
            runtime_event: None,
        },
    });

    let mut observation = make_observation();
    observation.visible_locations = vec![ObservedLocation {
        location_id: "loc-home".to_string(),
        name: "home".to_string(),
        pos: GeoPos {
            x_cm: 0.0,
            y_cm: 0.0,
            z_cm: 0.0,
        },
        profile: Default::default(),
        distance_cm: 0,
    }];
    observation
        .self_resources
        .add(ResourceKind::Data, 24)
        .expect("add test hardware");
    observation
        .self_resources
        .add(ResourceKind::Electricity, 100)
        .expect("add test electricity");

    let decision = behavior.decide(&observation);
    assert_eq!(
        decision,
        AgentDecision::Act(Action::ScheduleRecipe {
            owner: ResourceOwner::Agent {
                agent_id: "agent-1".to_string(),
            },
            factory_id: "factory.alpha".to_string(),
            recipe_id: "recipe.assembler.gear".to_string(),
            batches: 1,
        })
    );
}

#[test]
fn llm_agent_builds_missing_factory_before_cross_stage_coverage_handoff() {
    let client = MockClient {
        output: Some(
            r#"{"decision":"schedule_recipe","owner":"self","factory_id":"factory.smelter.1","recipe_id":"recipe.smelter.polymer_resin","batches":1}"#.to_string(),
        ),
        err: None,
    };
    let mut behavior = LlmAgentBehavior::new("agent-1", base_config(), client);
    seed_known_factory(
        &mut behavior,
        "factory.smelter.1",
        "factory.smelter.mk1",
        "loc-home",
    );

    for (offset, recipe_id, finished_product_id) in [
        (0_u64, "recipe.smelter.iron_ingot", "iron_ingot"),
        (1_u64, "recipe.smelter.copper_wire", "copper_wire"),
        (2_u64, "recipe.smelter.polymer_resin", "polymer_resin"),
    ] {
        behavior.on_action_result(&ActionResult {
            action: Action::ScheduleRecipe {
                owner: ResourceOwner::Agent {
                    agent_id: "agent-1".to_string(),
                },
                factory_id: "factory.smelter.1".to_string(),
                recipe_id: recipe_id.to_string(),
                batches: 1,
            },
            action_id: 1_200 + offset,
            success: true,
            event: WorldEvent {
                id: 1_300 + offset,
                time: 260 + offset,
                kind: WorldEventKind::RecipeScheduled {
                    owner: ResourceOwner::Agent {
                        agent_id: "agent-1".to_string(),
                    },
                    factory_id: "factory.smelter.1".to_string(),
                    recipe_id: recipe_id.to_string(),
                    batches: 1,
                    electricity_cost: 6,
                    hardware_cost: 2,
                    data_output: 1,
                    finished_product_id: finished_product_id.to_string(),
                    finished_product_units: 1,
                },
                runtime_event: None,
            },
        });
    }

    let mut observation = make_observation();
    observation.visible_locations = vec![ObservedLocation {
        location_id: "loc-home".to_string(),
        name: "home".to_string(),
        pos: GeoPos {
            x_cm: 0.0,
            y_cm: 0.0,
            z_cm: 0.0,
        },
        profile: Default::default(),
        distance_cm: 0,
    }];
    observation
        .self_resources
        .add(ResourceKind::Data, 24)
        .expect("add test hardware");
    observation
        .self_resources
        .add(ResourceKind::Electricity, 100)
        .expect("add test electricity");

    let decision = behavior.decide(&observation);
    assert_eq!(
        decision,
        AgentDecision::Act(Action::ScheduleRecipe {
            owner: ResourceOwner::Agent {
                agent_id: "agent-1".to_string(),
            },
            factory_id: "factory.smelter.1".to_string(),
            recipe_id: "recipe.smelter.alloy_plate".to_string(),
            batches: 1,
        })
    );
}

#[test]
fn llm_agent_preserves_hard_switch_on_noncanonical_same_kind_factory() {
    let client = MockClient {
        output: Some(
            r#"{"decision":"schedule_recipe","owner":"self","factory_id":"factory.assembler.legacy","recipe_id":"recipe.assembler.control_chip","batches":1}"#.to_string(),
        ),
        err: None,
    };
    let mut behavior = LlmAgentBehavior::new("agent-1", base_config(), client);
    seed_known_factory(
        &mut behavior,
        "factory.assembler.legacy",
        "factory.assembler.mk1",
        "loc-home",
    );
    seed_known_factory(
        &mut behavior,
        "factory.assembler.current",
        "factory.assembler.mk1",
        "loc-home",
    );

    behavior.on_action_result(&ActionResult {
        action: Action::ScheduleRecipe {
            owner: ResourceOwner::Agent {
                agent_id: "agent-1".to_string(),
            },
            factory_id: "factory.assembler.legacy".to_string(),
            recipe_id: "recipe.assembler.control_chip".to_string(),
            batches: 1,
        },
        action_id: 860,
        success: true,
        event: WorldEvent {
            id: 960,
            time: 220,
            kind: WorldEventKind::RecipeScheduled {
                owner: ResourceOwner::Agent {
                    agent_id: "agent-1".to_string(),
                },
                factory_id: "factory.assembler.legacy".to_string(),
                recipe_id: "recipe.assembler.control_chip".to_string(),
                batches: 1,
                electricity_cost: 6,
                hardware_cost: 2,
                data_output: 1,
                finished_product_id: "control_chip".to_string(),
                finished_product_units: 1,
            },
            runtime_event: None,
        },
    });

    let mut observation = make_observation();
    observation
        .self_resources
        .add(ResourceKind::Data, 24)
        .expect("add test hardware");
    observation
        .self_resources
        .add(ResourceKind::Electricity, 100)
        .expect("add test electricity");

    let decision = behavior.decide(&observation);
    assert_eq!(
        decision,
        AgentDecision::Act(Action::ScheduleRecipe {
            owner: ResourceOwner::Agent {
                agent_id: "agent-1".to_string(),
            },
            factory_id: "factory.assembler.legacy".to_string(),
            recipe_id: "recipe.assembler.gear".to_string(),
            batches: 1,
        })
    );
}

#[test]
fn llm_agent_rewrites_wait_ticks_to_sustained_schedule_after_full_recipe_coverage() {
    let client = MockClient {
        output: Some(r#"{"decision":"wait_ticks","ticks":3}"#.to_string()),
        err: None,
    };
    let mut behavior = LlmAgentBehavior::new("agent-1", base_config(), client);

    behavior.on_action_result(&ActionResult {
        action: Action::BuildFactory {
            owner: ResourceOwner::Agent {
                agent_id: "agent-1".to_string(),
            },
            location_id: "loc-home".to_string(),
            factory_id: "factory.alpha".to_string(),
            factory_kind: "factory.assembler.mk1".to_string(),
        },
        action_id: 900,
        success: true,
        event: WorldEvent {
            id: 901,
            time: 180,
            kind: WorldEventKind::FactoryBuilt {
                owner: ResourceOwner::Agent {
                    agent_id: "agent-1".to_string(),
                },
                location_id: "loc-home".to_string(),
                factory_id: "factory.alpha".to_string(),
                factory_kind: "factory.assembler.mk1".to_string(),
                electricity_cost: 10,
                hardware_cost: 5,
            },
            runtime_event: None,
        },
    });

    let coverage_events = [
        (
            "recipe.smelter.iron_ingot",
            6_i64,
            2_i64,
            "product.material.iron_ingot",
        ),
        (
            "recipe.smelter.copper_wire",
            6_i64,
            2_i64,
            "product.material.copper_wire",
        ),
        (
            "recipe.smelter.polymer_resin",
            6_i64,
            2_i64,
            "product.material.polymer_resin",
        ),
        (
            "recipe.smelter.alloy_plate",
            9_i64,
            4_i64,
            "product.material.alloy_plate",
        ),
        (
            "recipe.assembler.gear",
            4_i64,
            2_i64,
            "product.component.gear",
        ),
        (
            "recipe.assembler.control_chip",
            6_i64,
            4_i64,
            "product.component.control_chip",
        ),
        (
            "recipe.assembler.motor_mk1",
            7_i64,
            4_i64,
            "product.component.motor_mk1",
        ),
        (
            "recipe.assembler.logistics_drone",
            24_i64,
            8_i64,
            "product.component.logistics_drone",
        ),
        (
            "recipe.assembler.sensor_pack",
            8_i64,
            4_i64,
            "product.component.sensor_pack",
        ),
        (
            "recipe.assembler.module_rack",
            10_i64,
            6_i64,
            "product.finished.module_rack",
        ),
        (
            "recipe.assembler.factory_core",
            14_i64,
            8_i64,
            "product.infrastructure.factory_core",
        ),
    ];

    for (offset, (recipe_id, electricity_cost, hardware_cost, finished_product_id)) in
        coverage_events.into_iter().enumerate()
    {
        behavior.on_action_result(&ActionResult {
            action: Action::ScheduleRecipe {
                owner: ResourceOwner::Agent {
                    agent_id: "agent-1".to_string(),
                },
                factory_id: "factory.alpha".to_string(),
                recipe_id: recipe_id.to_string(),
                batches: 1,
            },
            action_id: 910 + offset as u64,
            success: true,
            event: WorldEvent {
                id: 920 + offset as u64,
                time: 181 + offset as u64,
                kind: WorldEventKind::RecipeScheduled {
                    owner: ResourceOwner::Agent {
                        agent_id: "agent-1".to_string(),
                    },
                    factory_id: "factory.alpha".to_string(),
                    recipe_id: recipe_id.to_string(),
                    batches: 1,
                    electricity_cost,
                    hardware_cost,
                    data_output: 1,
                    finished_product_id: finished_product_id.to_string(),
                    finished_product_units: 1,
                },
                runtime_event: None,
            },
        });
    }

    let mut observation = make_observation();
    observation.time = 190;
    observation.visible_locations = vec![ObservedLocation {
        location_id: "loc-home".to_string(),
        name: "home".to_string(),
        pos: GeoPos {
            x_cm: 0.0,
            y_cm: 0.0,
            z_cm: 0.0,
        },
        profile: Default::default(),
        distance_cm: 0,
    }];
    observation
        .self_resources
        .add(ResourceKind::Data, 16)
        .expect("add test hardware");
    observation
        .self_resources
        .add(ResourceKind::Electricity, 90)
        .expect("add test electricity");

    let decision = behavior.decide(&observation);
    assert_eq!(
        decision,
        AgentDecision::Act(Action::ScheduleRecipe {
            owner: ResourceOwner::Agent {
                agent_id: "agent-1".to_string(),
            },
            factory_id: "factory.alpha".to_string(),
            recipe_id: "recipe.assembler.gear".to_string(),
            batches: 1,
        })
    );

    let trace = behavior.take_decision_trace().expect("trace");
    assert!(trace.llm_step_trace.iter().any(|step| step
        .output_summary
        .contains("wait_ticks(3) rewritten to sustained production")));
    assert!(trace
        .llm_step_trace
        .iter()
        .any(|step| step.output_summary.contains("decision_rewrite={")));
}

#[test]
fn llm_agent_rewrites_wait_to_recovery_action_after_full_recipe_coverage() {
    let client = MockClient {
        output: Some(r#"{"decision":"wait"}"#.to_string()),
        err: None,
    };
    let mut behavior = LlmAgentBehavior::new("agent-1", base_config(), client);

    behavior.on_action_result(&ActionResult {
        action: Action::BuildFactory {
            owner: ResourceOwner::Agent {
                agent_id: "agent-1".to_string(),
            },
            location_id: "loc-home".to_string(),
            factory_id: "factory.alpha".to_string(),
            factory_kind: "factory.assembler.mk1".to_string(),
        },
        action_id: 930,
        success: true,
        event: WorldEvent {
            id: 931,
            time: 200,
            kind: WorldEventKind::FactoryBuilt {
                owner: ResourceOwner::Agent {
                    agent_id: "agent-1".to_string(),
                },
                location_id: "loc-home".to_string(),
                factory_id: "factory.alpha".to_string(),
                factory_kind: "factory.assembler.mk1".to_string(),
                electricity_cost: 10,
                hardware_cost: 5,
            },
            runtime_event: None,
        },
    });

    let covered_recipe_ids = [
        "recipe.smelter.iron_ingot",
        "recipe.smelter.copper_wire",
        "recipe.smelter.polymer_resin",
        "recipe.smelter.alloy_plate",
        "recipe.assembler.gear",
        "recipe.assembler.control_chip",
        "recipe.assembler.motor_mk1",
        "recipe.assembler.logistics_drone",
        "recipe.assembler.sensor_pack",
        "recipe.assembler.module_rack",
        "recipe.assembler.factory_core",
    ];
    for (offset, recipe_id) in covered_recipe_ids.into_iter().enumerate() {
        behavior.on_action_result(&ActionResult {
            action: Action::ScheduleRecipe {
                owner: ResourceOwner::Agent {
                    agent_id: "agent-1".to_string(),
                },
                factory_id: "factory.alpha".to_string(),
                recipe_id: recipe_id.to_string(),
                batches: 1,
            },
            action_id: 940 + offset as u64,
            success: true,
            event: WorldEvent {
                id: 950 + offset as u64,
                time: 201 + offset as u64,
                kind: WorldEventKind::RecipeScheduled {
                    owner: ResourceOwner::Agent {
                        agent_id: "agent-1".to_string(),
                    },
                    factory_id: "factory.alpha".to_string(),
                    recipe_id: recipe_id.to_string(),
                    batches: 1,
                    electricity_cost: 6,
                    hardware_cost: 2,
                    data_output: 1,
                    finished_product_id: format!("product.{recipe_id}"),
                    finished_product_units: 1,
                },
                runtime_event: None,
            },
        });
    }

    let mut observation = make_observation();
    observation.time = 210;
    observation.visible_locations = vec![ObservedLocation {
        location_id: "loc-home".to_string(),
        name: "home".to_string(),
        pos: GeoPos {
            x_cm: 0.0,
            y_cm: 0.0,
            z_cm: 0.0,
        },
        profile: Default::default(),
        distance_cm: 0,
    }];
    observation
        .self_resources
        .add(ResourceKind::Data, 4)
        .expect("add hardware");
    observation
        .self_resources
        .remove(ResourceKind::Electricity, 28)
        .expect("drain electricity");

    let decision = behavior.decide(&observation);
    assert_eq!(
        decision,
        AgentDecision::Act(Action::HarvestRadiation {
            agent_id: "agent-1".to_string(),
            max_amount: DEFAULT_LLM_HARVEST_MAX_AMOUNT_CAP,
        })
    );

    let trace = behavior.take_decision_trace().expect("trace");
    assert!(trace.llm_step_trace.iter().any(|step| step
        .output_summary
        .contains("wait rewritten to sustained production")));
    assert!(trace
        .llm_step_trace
        .iter()
        .any(|step| step.output_summary.contains("decision_rewrite={")));
}

#[test]
fn llm_agent_reroutes_schedule_recipe_when_hardware_cannot_cover_one_batch() {
    let client = MockClient {
        output: Some(
            r#"{"decision":"schedule_recipe","owner":"self","factory_id":"factory.alpha","recipe_id":"recipe.assembler.logistics_drone","batches":5}"#.to_string(),
        ),
        err: None,
    };
    let mut behavior = LlmAgentBehavior::new("agent-1", base_config(), client);
    seed_known_factory(
        &mut behavior,
        "factory.alpha",
        "factory.assembler.mk1",
        "loc-home",
    );
    let mut observation = make_observation();
    observation
        .self_resources
        .add(ResourceKind::Data, 7)
        .expect("add test hardware");
    observation
        .self_resources
        .add(ResourceKind::Data, 1_000)
        .expect("add test compound");

    let decision = behavior.decide(&observation);
    assert_eq!(
        decision,
        AgentDecision::Act(Action::ScheduleRecipe {
            owner: ResourceOwner::Agent {
                agent_id: "agent-1".to_string(),
            },
            factory_id: "factory.alpha".to_string(),
            recipe_id: "recipe.assembler.logistics_drone".to_string(),
            batches: 2,
        })
    );

    let trace = behavior.take_decision_trace().expect("trace");
    assert!(trace
        .llm_step_trace
        .iter()
        .any(|step| step.output_summary.contains("batches clamped")));
}

#[test]
fn llm_agent_reroutes_schedule_recipe_to_mine_when_compound_missing_and_caps_mass() {
    let client = MockClient {
        output: Some(
            r#"{"decision":"schedule_recipe","owner":"self","factory_id":"factory.alpha","recipe_id":"recipe.assembler.logistics_drone","batches":1}"#.to_string(),
        ),
        err: None,
    };
    let mut behavior = LlmAgentBehavior::new("agent-1", base_config(), client);
    seed_known_factory(
        &mut behavior,
        "factory.alpha",
        "factory.assembler.mk1",
        "loc-home",
    );
    let mut observation = make_observation();
    observation.visible_locations = vec![ObservedLocation {
        location_id: "loc-home".to_string(),
        name: "home".to_string(),
        pos: GeoPos {
            x_cm: 0.0,
            y_cm: 0.0,
            z_cm: 0.0,
        },
        profile: Default::default(),
        distance_cm: 0,
    }];

    let decision = behavior.decide(&observation);
    assert_eq!(
        decision,
        AgentDecision::Act(Action::ScheduleRecipe {
            owner: ResourceOwner::Agent {
                agent_id: "agent-1".to_string(),
            },
            factory_id: "factory.alpha".to_string(),
            recipe_id: "recipe.assembler.logistics_drone".to_string(),
            batches: 1,
        })
    );
}

#[test]
fn llm_agent_clamps_mine_compound_mass_by_known_location_availability() {
    let client = MockClient {
        output: Some(
            r#"{"decision":"mine_compound","owner":"self","location_id":"loc-home","compound_mass_g":4000}"#
                .to_string(),
        ),
        err: None,
    };
    let mut behavior = LlmAgentBehavior::new("agent-1", base_config(), client);
    behavior.on_action_result(&ActionResult {
        action: Action::MineCompound {
            owner: ResourceOwner::Agent {
                agent_id: "agent-1".to_string(),
            },
            location_id: "loc-home".to_string(),
            compound_mass_g: 4_000,
        },
        action_id: 612,
        success: false,
        event: WorldEvent {
            id: 712,
            time: 160,
            kind: WorldEventKind::ActionRejected {
                reason: RejectReason::InsufficientResource {
                    owner: ResourceOwner::Location {
                        location_id: "loc-home".to_string(),
                    },
                    kind: ResourceKind::Data,
                    requested: 4_000,
                    available: 1_000,
                },
            },
            runtime_event: None,
        },
    });

    let mut observation = make_observation();
    observation.visible_locations = vec![ObservedLocation {
        location_id: "loc-home".to_string(),
        name: "home".to_string(),
        pos: GeoPos {
            x_cm: 0.0,
            y_cm: 0.0,
            z_cm: 0.0,
        },
        profile: Default::default(),
        distance_cm: 0,
    }];
    observation.time = 161;

    let decision = behavior.decide(&observation);
    assert_eq!(
        decision,
        AgentDecision::Act(Action::MineCompound {
            owner: ResourceOwner::Agent {
                agent_id: "agent-1".to_string(),
            },
            location_id: "loc-home".to_string(),
            compound_mass_g: 1_000,
        })
    );

    let trace = behavior.take_decision_trace().expect("trace");
    assert!(trace.llm_step_trace.iter().any(|step| step
        .output_summary
        .contains("clamped by known_location_compound_available")));
}
