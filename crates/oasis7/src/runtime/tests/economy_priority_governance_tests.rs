use super::*;

#[test]
fn industry_stage_progresses_from_bootstrap_to_scale_out_and_governance() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0.0, 0.0),
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
        spec: factory_spec("factory.stage", 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("factory ready");

    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-1"), "iron_ingot", 30)
        .expect("seed local recipe material");
    world.set_resource_balance(ResourceKind::Electricity, 100);

    authorize_policy_update(&mut world, "builder-a", "proposal.policy.disable-tax");
    world.submit_action(Action::UpdateGameplayPolicy {
        operator_agent_id: "builder-a".to_string(),
        electricity_tax_bps: 0,
        data_tax_bps: 0,
        power_trade_fee_bps: 0,
        max_open_contracts_per_agent: 16,
        blocked_agents: Vec::new(),
        forbidden_location_ids: Vec::new(),
    });
    world.step().expect("disable tax policy");

    let recipe_plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("iron_ingot", 2)],
        vec![MaterialStack::new("gear", 1)],
        Vec::new(),
        1,
        1,
    );
    for index in 0..3 {
        world.submit_action(Action::ScheduleRecipe {
            requester_agent_id: "builder-a".to_string(),
            factory_id: "factory.stage".to_string(),
            recipe_id: format!("recipe.stage.{index}"),
            plan: recipe_plan.clone(),
        });
        world.step().expect("start recipe");
        world.step().expect("complete recipe");
    }

    assert_eq!(
        world.state().industry_progress.stage,
        IndustryStage::ScaleOut
    );
    assert_eq!(world.state().industry_progress.completed_recipe_jobs, 3);
    assert_eq!(
        world.state().industry_progress.completed_material_transits,
        0
    );

    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-a"), "copper_wire", 60)
        .expect("seed transit material");
    for _ in 0..3 {
        world.submit_action(Action::TransferMaterial {
            requester_agent_id: "builder-a".to_string(),
            from_ledger: MaterialLedgerId::site("site-a"),
            to_ledger: MaterialLedgerId::site("site-b"),
            kind: "copper_wire".to_string(),
            amount: 10,
            distance_km: 100,
            priority: None,
        });
        world.step().expect("start transit");
        world.step().expect("complete transit");
    }
    assert_eq!(
        world.state().industry_progress.stage,
        IndustryStage::ScaleOut
    );
    assert_eq!(
        world.state().industry_progress.completed_material_transits,
        3
    );

    authorize_policy_update(&mut world, "builder-a", "proposal.policy.enable-tax");
    world.submit_action(Action::UpdateGameplayPolicy {
        operator_agent_id: "builder-a".to_string(),
        electricity_tax_bps: 500,
        data_tax_bps: 0,
        power_trade_fee_bps: 0,
        max_open_contracts_per_agent: 16,
        blocked_agents: Vec::new(),
        forbidden_location_ids: Vec::new(),
    });
    world.step().expect("enable tax policy");

    assert_eq!(
        world.state().industry_progress.stage,
        IndustryStage::Governance
    );
}

fn assert_rejected_note_contains(world: &World, action_id: u64, expected: &str) {
    let reason = world
        .journal()
        .events
        .iter()
        .rev()
        .find_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::ActionRejected {
                action_id: rejected_action_id,
                reason,
            }) if *rejected_action_id == action_id => Some(format!("{reason:?}")),
            _ => None,
        })
        .expect("action rejected event");
    assert!(
        reason.contains(expected),
        "expected `{expected}` in rejection reason: {reason}"
    );
}

#[test]
fn govern_profile_requires_existing_approved_or_applied_manifest_proposal() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "operator-a".to_string(),
        pos: pos(0.0, 0.0),
    });
    world.step().expect("register operator");

    let missing_action_id = world.submit_action(Action::GovernMaterialProfile {
        operator_agent_id: "operator-a".to_string(),
        proposal_id: 9_999,
        profile: MaterialProfileV1 {
            kind: "copper_wire".to_string(),
            tier: 2,
            category: "intermediate".to_string(),
            stack_limit: 500,
            transport_loss_class: MaterialTransportLossClass::High,
            decay_bps_per_tick: 0,
            default_priority: MaterialDefaultPriority::Urgent,
        },
    });
    world.step().expect("missing proposal rejection");
    assert_rejected_note_contains(&world, missing_action_id, "governance proposal not found");

    let mut manifest = world.manifest().clone();
    manifest.version = manifest.version.saturating_add(1);
    let proposed_only_id = world
        .propose_manifest_update(manifest, "operator-a".to_string())
        .expect("propose manifest update");
    let proposed_action_id = world.submit_action(Action::GovernProductProfile {
        operator_agent_id: "operator-a".to_string(),
        proposal_id: proposed_only_id,
        profile: ProductProfileV1 {
            product_id: "governed_product".to_string(),
            role_tag: "scale".to_string(),
            maintenance_sink: Vec::new(),
            tradable: true,
            unlock_stage: "bootstrap".to_string(),
        },
    });
    world.step().expect("proposed-only rejection");
    assert_rejected_note_contains(
        &world,
        proposed_action_id,
        "governance proposal must be approved or applied",
    );
}

#[test]
fn govern_profile_actions_emit_events_and_update_profile_state() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "operator-a".to_string(),
        pos: pos(0.0, 0.0),
    });
    world.step().expect("register operator");
    let proposal_id = approved_manifest_proposal(&mut world, "operator-a");

    world.submit_action(Action::GovernMaterialProfile {
        operator_agent_id: "operator-a".to_string(),
        proposal_id,
        profile: MaterialProfileV1 {
            kind: "governed_wire".to_string(),
            tier: 3,
            category: "intermediate".to_string(),
            stack_limit: 700,
            transport_loss_class: MaterialTransportLossClass::Low,
            decay_bps_per_tick: 5,
            default_priority: MaterialDefaultPriority::Standard,
        },
    });
    world.step().expect("govern material profile");
    assert!(matches!(
        world.journal().events.last().map(|event| &event.body),
        Some(WorldEventBody::Domain(DomainEvent::MaterialProfileGoverned {
            operator_agent_id,
            proposal_id: event_proposal_id,
            profile,
        })) if operator_agent_id == "operator-a"
            && *event_proposal_id == proposal_id
            && profile.kind == "governed_wire"
    ));

    world.submit_action(Action::GovernProductProfile {
        operator_agent_id: "operator-a".to_string(),
        proposal_id,
        profile: ProductProfileV1 {
            product_id: "governed_product".to_string(),
            role_tag: "survival".to_string(),
            maintenance_sink: vec![MaterialStack::new("hardware_part", 1)],
            tradable: true,
            unlock_stage: "scale_out".to_string(),
        },
    });
    world.step().expect("govern product profile");
    assert!(matches!(
        world.journal().events.last().map(|event| &event.body),
        Some(WorldEventBody::Domain(DomainEvent::ProductProfileGoverned {
            operator_agent_id,
            proposal_id: event_proposal_id,
            profile,
        })) if operator_agent_id == "operator-a"
            && *event_proposal_id == proposal_id
            && profile.product_id == "governed_product"
    ));

    world.submit_action(Action::GovernRecipeProfile {
        operator_agent_id: "operator-a".to_string(),
        proposal_id,
        profile: RecipeProfileV1 {
            recipe_id: "governed_recipe".to_string(),
            bottleneck_tags: vec!["control_chip".to_string()],
            stage_gate: "governance".to_string(),
            preferred_factory_tags: vec!["assembly".to_string()],
        },
    });
    world.step().expect("govern recipe profile");
    assert!(matches!(
        world.journal().events.last().map(|event| &event.body),
        Some(WorldEventBody::Domain(DomainEvent::RecipeProfileGoverned {
            operator_agent_id,
            proposal_id: event_proposal_id,
            profile,
        })) if operator_agent_id == "operator-a"
            && *event_proposal_id == proposal_id
            && profile.recipe_id == "governed_recipe"
    ));

    world.submit_action(Action::GovernFactoryProfile {
        operator_agent_id: "operator-a".to_string(),
        proposal_id,
        profile: FactoryProfileV1 {
            factory_id: "governed_factory".to_string(),
            tier: 2,
            recipe_slots: 3,
            tags: vec!["assembly".to_string()],
        },
    });
    world.step().expect("govern factory profile");
    assert!(matches!(
        world.journal().events.last().map(|event| &event.body),
        Some(WorldEventBody::Domain(DomainEvent::FactoryProfileGoverned {
            operator_agent_id,
            proposal_id: event_proposal_id,
            profile,
        })) if operator_agent_id == "operator-a"
            && *event_proposal_id == proposal_id
            && profile.factory_id == "governed_factory"
    ));

    assert_eq!(
        world
            .state()
            .material_profiles
            .get("governed_wire")
            .map(|profile| profile.stack_limit),
        Some(700)
    );
    assert_eq!(
        world
            .state()
            .product_profiles
            .get("governed_product")
            .map(|profile| profile.role_tag.as_str()),
        Some("survival")
    );
    assert_eq!(
        world
            .state()
            .recipe_profiles
            .get("governed_recipe")
            .map(|profile| profile.stage_gate.as_str()),
        Some("governance")
    );
    assert_eq!(
        world
            .state()
            .factory_profiles
            .get("governed_factory")
            .map(|profile| profile.recipe_slots),
        Some(3)
    );
}

#[test]
fn govern_profile_actions_reject_invalid_profile_payloads() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "operator-a".to_string(),
        pos: pos(0.0, 0.0),
    });
    world.step().expect("register operator");
    let proposal_id = approved_manifest_proposal(&mut world, "operator-a");

    let material_action_id = world.submit_action(Action::GovernMaterialProfile {
        operator_agent_id: "operator-a".to_string(),
        proposal_id,
        profile: MaterialProfileV1 {
            kind: "broken_material".to_string(),
            tier: 0,
            category: "intermediate".to_string(),
            stack_limit: 100,
            transport_loss_class: MaterialTransportLossClass::Medium,
            decay_bps_per_tick: 0,
            default_priority: MaterialDefaultPriority::Standard,
        },
    });
    world.step().expect("reject invalid material profile");
    assert_rejected_note_contains(&world, material_action_id, "tier must be >= 1");

    let product_action_id = world.submit_action(Action::GovernProductProfile {
        operator_agent_id: "operator-a".to_string(),
        proposal_id,
        profile: ProductProfileV1 {
            product_id: "broken_product".to_string(),
            role_tag: "".to_string(),
            maintenance_sink: Vec::new(),
            tradable: true,
            unlock_stage: "bootstrap".to_string(),
        },
    });
    world.step().expect("reject invalid product profile");
    assert_rejected_note_contains(&world, product_action_id, "role_tag cannot be empty");

    let recipe_action_id = world.submit_action(Action::GovernRecipeProfile {
        operator_agent_id: "operator-a".to_string(),
        proposal_id,
        profile: RecipeProfileV1 {
            recipe_id: "".to_string(),
            bottleneck_tags: vec!["gear".to_string()],
            stage_gate: "bootstrap".to_string(),
            preferred_factory_tags: vec!["assembly".to_string()],
        },
    });
    world.step().expect("reject invalid recipe profile");
    assert_rejected_note_contains(&world, recipe_action_id, "recipe_id cannot be empty");

    let factory_action_id = world.submit_action(Action::GovernFactoryProfile {
        operator_agent_id: "operator-a".to_string(),
        proposal_id,
        profile: FactoryProfileV1 {
            factory_id: "broken_factory".to_string(),
            tier: 1,
            recipe_slots: 0,
            tags: vec!["assembly".to_string()],
        },
    });
    world.step().expect("reject invalid factory profile");
    assert_rejected_note_contains(&world, factory_action_id, "recipe_slots must be > 0");
}
