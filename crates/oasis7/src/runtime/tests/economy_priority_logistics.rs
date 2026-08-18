use super::pos;
use crate::runtime::{
    Action, DomainEvent, FactoryProfileV1, GovernanceProposalStatus, IndustryStage,
    MaterialDefaultPriority, MaterialLedgerId, MaterialProfileV1, MaterialTransitPriority,
    MaterialTransportLossClass, ProductProfileV1, ProposalDecision, RecipeProfileV1, RejectReason,
    World, WorldEventBody,
};
use crate::simulator::ResourceKind;
use oasis7_wasm_abi::{FactoryModuleSpec, MaterialStack, RecipeExecutionPlan};

#[path = "economy_priority_governance_tests.rs"]
mod governance_tests;

fn factory_spec(factory_id: &str, build_time_ticks: u32, recipe_slots: u16) -> FactoryModuleSpec {
    FactoryModuleSpec {
        factory_id: factory_id.to_string(),
        display_name: "Test Factory".to_string(),
        tier: 1,
        tags: vec!["assembly".to_string()],
        build_cost: vec![
            MaterialStack::new("steel_plate", 10),
            MaterialStack::new("circuit_board", 2),
        ],
        build_time_ticks,
        base_power_draw: 5,
        recipe_slots,
        throughput_bps: 10_000,
        maintenance_per_tick: 1,
    }
}

fn authorize_policy_update(world: &mut World, operator_agent_id: &str, proposal_key: &str) {
    world.submit_action(Action::OpenGovernanceProposal {
        proposer_agent_id: operator_agent_id.to_string(),
        proposal_key: proposal_key.to_string(),
        title: format!("title.{proposal_key}"),
        description: "authorize gameplay policy update".to_string(),
        options: vec!["approve".to_string(), "reject".to_string()],
        voting_window_ticks: 1,
        quorum_weight: 3,
        pass_threshold_bps: 5_000,
    });
    world.step().expect("open governance proposal");

    world.submit_action(Action::CastGovernanceVote {
        voter_agent_id: operator_agent_id.to_string(),
        proposal_key: proposal_key.to_string(),
        option: "approve".to_string(),
        weight: 3,
    });
    world.step().expect("cast governance vote");

    for _ in 0..2 {
        let Some(proposal) = world.state().governance_proposals.get(proposal_key) else {
            break;
        };
        if proposal.status != GovernanceProposalStatus::Open {
            break;
        }
        world.step().expect("advance governance proposal");
    }

    let proposal = world
        .state()
        .governance_proposals
        .get(proposal_key)
        .expect("proposal finalized");
    assert_eq!(proposal.status, GovernanceProposalStatus::Passed);
}

fn approved_manifest_proposal(world: &mut World, author: &str) -> u64 {
    let mut manifest = world.manifest().clone();
    manifest.version = manifest.version.saturating_add(1);
    let proposal_id = world
        .propose_manifest_update(manifest, author.to_string())
        .expect("propose manifest update");
    world
        .shadow_proposal(proposal_id)
        .expect("shadow manifest proposal");
    world
        .approve_proposal(proposal_id, author.to_string(), ProposalDecision::Approve)
        .expect("approve manifest proposal");
    proposal_id
}

fn latest_action_rejected_message(world: &World) -> String {
    world
        .journal()
        .events
        .iter()
        .rev()
        .find_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => {
                Some(format!("{reason:?}"))
            }
            _ => None,
        })
        .expect("action rejected")
}

fn latest_factory_production_block(world: &World) -> (String, String) {
    world
        .journal()
        .events
        .iter()
        .rev()
        .find_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::FactoryProductionBlocked {
                blocker_kind,
                blocker_detail,
                ..
            }) => Some((blocker_kind.clone(), blocker_detail.clone())),
            _ => None,
        })
        .expect("factory production blocked")
}

#[test]
fn due_recipe_jobs_prioritize_survival_over_expansion() {
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
        spec: factory_spec("factory.priority", 1, 2),
    });
    world.step().expect("start factory build");
    world.step().expect("factory ready");

    world
        .set_material_balance("iron_ingot", 4)
        .expect("seed recipe input");
    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-1"), "iron_ingot", 4)
        .expect("seed site recipe input");
    world.set_resource_balance(ResourceKind::Electricity, 20);

    let expansion_plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("iron_ingot", 2)],
        vec![MaterialStack::new("outpost_kit", 1)],
        Vec::new(),
        2,
        1,
    );
    let survival_plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("iron_ingot", 2)],
        vec![MaterialStack::new("oxygen_pack", 1)],
        Vec::new(),
        2,
        1,
    );

    // Submit expansion first to verify due-job completion still prioritizes survival.
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.priority".to_string(),
        recipe_id: "recipe.expand.outpost".to_string(),
        plan: expansion_plan,
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.priority".to_string(),
        recipe_id: "recipe.survival.oxygen".to_string(),
        plan: survival_plan,
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world.step().expect("start recipes");
    assert_eq!(world.pending_recipe_jobs_len(), 2);

    let before = world.journal().events.len();
    world.step().expect("complete recipes");

    let completed_recipe_ids: Vec<String> = world.journal().events[before..]
        .iter()
        .filter_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::RecipeCompleted { recipe_id, .. }) => {
                Some(recipe_id.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        completed_recipe_ids,
        vec![
            "recipe.survival.oxygen".to_string(),
            "recipe.expand.outpost".to_string(),
        ]
    );
}

#[test]
fn bottleneck_pressure_bumps_recipe_completion_priority() {
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
        spec: factory_spec("factory.bottleneck", 1, 2),
    });
    world.step().expect("start factory build");
    world.step().expect("factory ready");

    world
        .set_material_balance("gear", 4)
        .expect("seed non-bottleneck material");
    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-1"), "gear", 4)
        .expect("seed site non-bottleneck material");
    world
        .set_material_balance("control_chip", 2)
        .expect("seed bottleneck material");
    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-1"), "control_chip", 2)
        .expect("seed site bottleneck material");
    world.set_resource_balance(ResourceKind::Electricity, 20);

    let non_bottleneck_plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("gear", 2)],
        vec![MaterialStack::new("factory_frame", 1)],
        Vec::new(),
        2,
        1,
    );
    let bottleneck_plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("control_chip", 2)],
        vec![MaterialStack::new("factory_frame", 1)],
        Vec::new(),
        2,
        1,
    );

    // Submit non-bottleneck first. Bottleneck should still complete first under low-stock pressure.
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.bottleneck".to_string(),
        recipe_id: "recipe.production.frame.normal".to_string(),
        plan: non_bottleneck_plan,
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.bottleneck".to_string(),
        recipe_id: "recipe.production.frame.control_chip".to_string(),
        plan: bottleneck_plan,
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world.step().expect("start recipes");
    assert_eq!(world.pending_recipe_jobs_len(), 2);

    let before = world.journal().events.len();
    world.step().expect("complete recipes");

    let completed: Vec<(String, Vec<String>)> = world.journal().events[before..]
        .iter()
        .filter_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::RecipeCompleted {
                recipe_id,
                bottleneck_tags,
                ..
            }) => Some((recipe_id.clone(), bottleneck_tags.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(completed.len(), 2);
    assert_eq!(completed[0].0, "recipe.production.frame.control_chip");
    assert_eq!(completed[0].1, vec!["control_chip".to_string()]);
    assert!(completed[1].1.is_empty());
}

#[test]
fn logistics_sla_metrics_and_priority_are_observable_after_transit_completion() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "operator-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register operator");

    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-a"), "copper_wire", 100)
        .expect("seed source");
    world.submit_action(Action::TransferMaterial {
        requester_agent_id: "operator-a".to_string(),
        from_ledger: MaterialLedgerId::site("site-a"),
        to_ledger: MaterialLedgerId::site("site-b"),
        kind: "copper_wire".to_string(),
        amount: 50,
        distance_km: 100,
        priority: None,
        route_id: None,
        route_ids: Vec::new(),
        auto_reroute: false,
    });
    world.step().expect("start transit");

    let started_priority = world
        .journal()
        .events
        .last()
        .and_then(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::MaterialTransitStarted { priority, .. }) => {
                Some(*priority)
            }
            _ => None,
        })
        .expect("material transit started with priority");
    assert_eq!(started_priority, MaterialTransitPriority::Standard);

    world.step().expect("complete transit");

    let completed_priority = world
        .journal()
        .events
        .last()
        .and_then(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::MaterialTransitCompleted { priority, .. }) => {
                Some(*priority)
            }
            _ => None,
        })
        .expect("material transit completed with priority");
    assert_eq!(completed_priority, MaterialTransitPriority::Standard);

    let metrics = world.logistics_sla_metrics();
    assert_eq!(metrics.completed_transits, 1);
    assert_eq!(metrics.fulfilled_transits, 1);
    assert_eq!(metrics.breached_transits, 0);
    assert_eq!(metrics.total_delay_ticks, 0);
    assert_eq!(metrics.urgent_completed_transits, 0);
    assert_eq!(metrics.urgent_fulfilled_transits, 0);
    assert_eq!(metrics.urgent_breached_transits, 0);
    assert_eq!(metrics.urgent_total_delay_ticks, 0);
    assert_eq!(metrics.breach_rate(), 0.0);
    assert_eq!(metrics.fulfillment_rate(), 1.0);
}

#[test]
fn due_transits_prioritize_urgent_before_standard_with_same_ready_at() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "operator-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register operator");

    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-a"), "copper_wire", 50)
        .expect("seed standard source");
    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-c"), "oxygen_pack", 50)
        .expect("seed urgent source");

    world.submit_action(Action::TransferMaterial {
        requester_agent_id: "operator-a".to_string(),
        from_ledger: MaterialLedgerId::site("site-a"),
        to_ledger: MaterialLedgerId::site("site-b"),
        kind: "copper_wire".to_string(),
        amount: 20,
        distance_km: 100,
        priority: None,
        route_id: None,
        route_ids: Vec::new(),
        auto_reroute: false,
    });
    world.submit_action(Action::TransferMaterial {
        requester_agent_id: "operator-a".to_string(),
        from_ledger: MaterialLedgerId::site("site-c"),
        to_ledger: MaterialLedgerId::site("site-d"),
        kind: "oxygen_pack".to_string(),
        amount: 20,
        distance_km: 100,
        priority: None,
        route_id: None,
        route_ids: Vec::new(),
        auto_reroute: false,
    });
    world.step().expect("start transits");
    assert_eq!(world.pending_material_transits_len(), 2);

    let before = world.journal().events.len();
    world.step().expect("complete transits");

    let completion_priorities: Vec<MaterialTransitPriority> = world.journal().events[before..]
        .iter()
        .filter_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::MaterialTransitCompleted { priority, .. }) => {
                Some(*priority)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        completion_priorities,
        vec![
            MaterialTransitPriority::Urgent,
            MaterialTransitPriority::Standard,
        ]
    );

    let metrics = world.logistics_sla_metrics();
    assert_eq!(metrics.completed_transits, 2);
    assert_eq!(metrics.fulfilled_transits, 2);
    assert_eq!(metrics.urgent_completed_transits, 1);
    assert_eq!(metrics.urgent_fulfilled_transits, 1);
}

#[test]
fn due_transits_allow_explicit_priority_override_for_non_urgent_material() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "operator-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register operator");

    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-a"), "copper_wire", 60)
        .expect("seed source");

    world.submit_action(Action::TransferMaterial {
        requester_agent_id: "operator-a".to_string(),
        from_ledger: MaterialLedgerId::site("site-a"),
        to_ledger: MaterialLedgerId::site("site-b"),
        kind: "copper_wire".to_string(),
        amount: 20,
        distance_km: 100,
        priority: None,
        route_id: None,
        route_ids: Vec::new(),
        auto_reroute: false,
    });
    world.submit_action(Action::TransferMaterial {
        requester_agent_id: "operator-a".to_string(),
        from_ledger: MaterialLedgerId::site("site-a"),
        to_ledger: MaterialLedgerId::site("site-c"),
        kind: "copper_wire".to_string(),
        amount: 20,
        distance_km: 100,
        priority: Some(MaterialTransitPriority::Urgent),
        route_id: None,
        route_ids: Vec::new(),
        auto_reroute: false,
    });
    world.step().expect("start transits");
    assert_eq!(world.pending_material_transits_len(), 2);

    let before = world.journal().events.len();
    world.step().expect("complete transits");
    let completion_priorities: Vec<MaterialTransitPriority> = world.journal().events[before..]
        .iter()
        .filter_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::MaterialTransitCompleted { priority, .. }) => {
                Some(*priority)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        completion_priorities,
        vec![
            MaterialTransitPriority::Urgent,
            MaterialTransitPriority::Standard,
        ]
    );
}

#[test]
fn transfer_material_uses_profile_priority_and_loss_class() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "operator-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register operator");

    world
        .upsert_material_profile(MaterialProfileV1 {
            kind: "copper_wire".to_string(),
            tier: 2,
            category: "intermediate".to_string(),
            stack_limit: 500,
            transport_loss_class: MaterialTransportLossClass::High,
            decay_bps_per_tick: 0,
            default_priority: MaterialDefaultPriority::Urgent,
        })
        .expect("insert profile");
    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-a"), "copper_wire", 100)
        .expect("seed source");

    world.submit_action(Action::TransferMaterial {
        requester_agent_id: "operator-a".to_string(),
        from_ledger: MaterialLedgerId::site("site-a"),
        to_ledger: MaterialLedgerId::site("site-b"),
        kind: "copper_wire".to_string(),
        amount: 20,
        distance_km: 100,
        priority: None,
        route_id: None,
        route_ids: Vec::new(),
        auto_reroute: false,
    });
    world.step().expect("start transit");

    let (started_priority, started_loss_bps) = world
        .journal()
        .events
        .last()
        .and_then(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::MaterialTransitStarted {
                priority,
                loss_bps,
                ..
            }) => Some((*priority, *loss_bps)),
            _ => None,
        })
        .expect("material transit started");
    assert_eq!(started_priority, MaterialTransitPriority::Urgent);
    assert_eq!(started_loss_bps, 20);

    world.step().expect("complete transit");
    let completed_priority = world
        .journal()
        .events
        .last()
        .and_then(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::MaterialTransitCompleted { priority, .. }) => {
                Some(*priority)
            }
            _ => None,
        })
        .expect("material transit completed");
    assert_eq!(completed_priority, MaterialTransitPriority::Urgent);
}

#[test]
fn logistics_route_registration_persists_and_duplicate_tuple_is_rejected() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "operator-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register operator");

    let register = || Action::RegisterLogisticsRoute {
        requester_agent_id: "operator-a".to_string(),
        from_ledger: MaterialLedgerId::site("site-a"),
        to_ledger: MaterialLedgerId::site("site-b"),
        kind: "copper_wire".to_string(),
        distance_km: 100,
        priority: MaterialTransitPriority::Standard,
        capacity_units: 100,
        tariff_electricity_per_unit: 0,
    };
    world.submit_action(register());
    world.step().expect("register logistics route");

    let route_id = match &world.journal().events.last().expect("route event").body {
        WorldEventBody::Domain(DomainEvent::LogisticsRouteRegistered {
            requester_agent_id,
            route_id,
            from_ledger,
            to_ledger,
            kind,
            distance_km,
            priority,
            ..
        }) => {
            assert_eq!(requester_agent_id, "operator-a");
            assert_eq!(from_ledger, &MaterialLedgerId::site("site-a"));
            assert_eq!(to_ledger, &MaterialLedgerId::site("site-b"));
            assert_eq!(kind, "copper_wire");
            assert_eq!(*distance_km, 100);
            assert_eq!(*priority, MaterialTransitPriority::Standard);
            route_id.clone().expect("registered route id")
        }
        other => panic!("expected LogisticsRouteRegistered, got {other:?}"),
    };

    world.submit_action(register());
    world.step().expect("reject duplicate route");
    assert!(matches!(
        world.journal().events.last().map(|event| &event.body),
        Some(WorldEventBody::Domain(DomainEvent::ActionRejected {
            reason: RejectReason::RuleDenied { .. },
            ..
        }))
    ));
    let registrations = world
        .journal()
        .events
        .iter()
        .filter(|event| {
            matches!(
                event.body,
                WorldEventBody::Domain(DomainEvent::LogisticsRouteRegistered { .. })
            )
        })
        .count();
    assert_eq!(registrations, 1);
    assert!(!route_id.is_empty());
}

#[test]
fn route_bound_transit_carries_route_id_through_start_pending_and_completion() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "operator-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register operator");
    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-a"), "copper_wire", 100)
        .expect("seed route source");

    world.submit_action(Action::RegisterLogisticsRoute {
        requester_agent_id: "operator-a".to_string(),
        from_ledger: MaterialLedgerId::site("site-a"),
        to_ledger: MaterialLedgerId::site("site-b"),
        kind: "copper_wire".to_string(),
        distance_km: 100,
        priority: MaterialTransitPriority::Standard,
        capacity_units: 100,
        tariff_electricity_per_unit: 0,
    });
    world.step().expect("register route");
    let route_id = match &world.journal().events.last().expect("route event").body {
        WorldEventBody::Domain(DomainEvent::LogisticsRouteRegistered { route_id, .. }) => {
            route_id.clone().expect("route id")
        }
        other => panic!("expected LogisticsRouteRegistered, got {other:?}"),
    };

    world.submit_action(Action::TransferMaterial {
        requester_agent_id: "operator-a".to_string(),
        from_ledger: MaterialLedgerId::site("site-a"),
        to_ledger: MaterialLedgerId::site("site-b"),
        kind: "copper_wire".to_string(),
        amount: 20,
        distance_km: 100,
        priority: Some(MaterialTransitPriority::Standard),
        route_id: Some(route_id.clone()),
        route_ids: Vec::new(),
        auto_reroute: false,
    });
    world.step().expect("start route-bound transit");
    assert!(matches!(
        world.journal().events.last().map(|event| &event.body),
        Some(WorldEventBody::Domain(
            DomainEvent::MaterialTransitStarted {
                route_id: Some(started_route_id),
                ..
            }
        )) if started_route_id == &route_id
    ));
    assert_eq!(
        world
            .state()
            .pending_material_transits
            .values()
            .next()
            .and_then(|job| job.route_id.clone()),
        Some(route_id.clone())
    );

    world.step().expect("complete route-bound transit");
    assert!(matches!(
        world.journal().events.last().map(|event| &event.body),
        Some(WorldEventBody::Domain(
            DomainEvent::MaterialTransitCompleted {
                route_id: Some(completed_route_id),
                ..
            }
        )) if completed_route_id == &route_id
    ));
}

#[test]
fn incompatible_or_unreachable_route_registration_and_use_are_rejected_without_mutation() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "operator-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register operator");
    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-a"), "copper_wire", 500)
        .expect("seed route validation source");
    world.submit_action(Action::RegisterLogisticsRoute {
        requester_agent_id: "operator-a".to_string(),
        from_ledger: MaterialLedgerId::site("site-a"),
        to_ledger: MaterialLedgerId::site("site-b"),
        kind: "copper_wire".to_string(),
        distance_km: 100,
        priority: MaterialTransitPriority::Standard,
        capacity_units: 100,
        tariff_electricity_per_unit: 0,
    });
    world.step().expect("register route");
    let route_id = match &world.journal().events.last().expect("route event").body {
        WorldEventBody::Domain(DomainEvent::LogisticsRouteRegistered { route_id, .. }) => {
            route_id.clone().expect("route id")
        }
        other => panic!("expected LogisticsRouteRegistered, got {other:?}"),
    };

    let mismatches = [
        (
            MaterialLedgerId::site("site-other"),
            MaterialLedgerId::site("site-b"),
            "copper_wire",
            100,
            MaterialTransitPriority::Standard,
        ),
        (
            MaterialLedgerId::site("site-a"),
            MaterialLedgerId::site("site-b"),
            "iron_ingot",
            100,
            MaterialTransitPriority::Standard,
        ),
        (
            MaterialLedgerId::site("site-a"),
            MaterialLedgerId::site("site-b"),
            "copper_wire",
            200,
            MaterialTransitPriority::Standard,
        ),
        (
            MaterialLedgerId::site("site-a"),
            MaterialLedgerId::site("site-b"),
            "copper_wire",
            100,
            MaterialTransitPriority::Urgent,
        ),
    ];
    for (from_ledger, to_ledger, kind, distance_km, priority) in mismatches {
        let pending_before = world.pending_material_transits_len();
        world.submit_action(Action::TransferMaterial {
            requester_agent_id: "operator-a".to_string(),
            from_ledger,
            to_ledger,
            kind: kind.to_string(),
            amount: 10,
            distance_km,
            priority: Some(priority),
            route_id: Some(route_id.clone()),
            route_ids: Vec::new(),
            auto_reroute: false,
        });
        world.step().expect("reject mismatched route use");
        assert!(matches!(
            world.journal().events.last().map(|event| &event.body),
            Some(WorldEventBody::Domain(DomainEvent::ActionRejected { .. }))
        ));
        assert_eq!(world.pending_material_transits_len(), pending_before);
    }

    world.submit_action(Action::RegisterLogisticsRoute {
        requester_agent_id: "operator-a".to_string(),
        from_ledger: MaterialLedgerId::site("site-a"),
        to_ledger: MaterialLedgerId::site("site-far"),
        kind: "copper_wire".to_string(),
        distance_km: 20_001,
        priority: MaterialTransitPriority::Standard,
        capacity_units: 100,
        tariff_electricity_per_unit: 0,
    });
    world.step().expect("reject over-max route");
    assert!(matches!(
        world.journal().events.last().map(|event| &event.body),
        Some(WorldEventBody::Domain(DomainEvent::ActionRejected { .. }))
    ));

    let pending_before = world.pending_material_transits_len();
    let journal_before = world.journal().events.len();
    world.submit_action(Action::TransferMaterial {
        requester_agent_id: "operator-a".to_string(),
        from_ledger: MaterialLedgerId::site("site-a"),
        to_ledger: MaterialLedgerId::site("site-b"),
        kind: "copper_wire".to_string(),
        amount: 10,
        distance_km: 100,
        priority: Some(MaterialTransitPriority::Standard),
        route_id: Some("route-missing".to_string()),
        route_ids: Vec::new(),
        auto_reroute: false,
    });
    world.step().expect("reject unreachable route use");
    assert!(
        world.journal().events[journal_before..]
            .iter()
            .any(|event| {
                matches!(
                    &event.body,
                    WorldEventBody::Domain(DomainEvent::ActionRejected { .. })
                )
            })
    );
    assert!(
        !world.journal().events[journal_before..]
            .iter()
            .any(|event| {
                matches!(
                    &event.body,
                    WorldEventBody::Domain(DomainEvent::MaterialTransitStarted { .. })
                )
            })
    );
    assert_eq!(world.pending_material_transits_len(), pending_before);
}

fn recipe_route_fixture(factory_id: &str) -> World {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register recipe route builder");
    world
        .set_material_balance("steel_plate", 20)
        .expect("seed factory steel");
    world
        .set_material_balance("circuit_board", 4)
        .expect("seed factory circuits");
    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        spec: factory_spec(factory_id, 1, 1),
    });
    world.step().expect("start recipe route factory");
    world.step().expect("finish recipe route factory");
    authorize_policy_update(
        &mut world,
        "builder-a",
        &format!("proposal.route.recipe.{factory_id}"),
    );
    world.submit_action(Action::UpdateGameplayPolicy {
        operator_agent_id: "builder-a".to_string(),
        electricity_tax_bps: 0,
        data_tax_bps: 0,
        power_trade_fee_bps: 0,
        max_open_contracts_per_agent: 16,
        blocked_agents: Vec::new(),
        forbidden_location_ids: Vec::new(),
    });
    world.step().expect("disable recipe route tax");
    world.set_resource_balance(ResourceKind::Electricity, 100);
    world
}

fn register_recipe_route(
    world: &mut World,
    from_ledger: &str,
    to_ledger: &str,
    kind: &str,
) -> String {
    let journal_before = world.journal().events.len();
    world.submit_action(Action::RegisterLogisticsRoute {
        requester_agent_id: "builder-a".to_string(),
        from_ledger: MaterialLedgerId::site(from_ledger),
        to_ledger: MaterialLedgerId::site(to_ledger),
        kind: kind.to_string(),
        distance_km: 100,
        priority: MaterialTransitPriority::Standard,
        capacity_units: 100,
        tariff_electricity_per_unit: 0,
    });
    world.step().expect("register recipe route");
    world.journal().events[journal_before..]
        .iter()
        .rev()
        .find_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::LogisticsRouteRegistered {
                route_id: Some(route_id),
                from_ledger: event_from,
                to_ledger: event_to,
                kind: event_kind,
                distance_km,
                priority,
                ..
            }) if event_from == &MaterialLedgerId::site(from_ledger)
                && event_to == &MaterialLedgerId::site(to_ledger)
                && event_kind == kind
                && *distance_km == 100
                && *priority == MaterialTransitPriority::Standard =>
            {
                Some(route_id.clone())
            }
            _ => None,
        })
        .expect("recipe route registration event in registration step")
}

fn complete_recipe_route_transfer(
    world: &mut World,
    route_id: &str,
    from_ledger: &str,
    to_ledger: &str,
    kind: &str,
    amount: i64,
) {
    world
        .set_ledger_material_balance(MaterialLedgerId::site(from_ledger), kind, amount)
        .expect("seed recipe route source");
    world.submit_action(Action::TransferMaterial {
        requester_agent_id: "builder-a".to_string(),
        from_ledger: MaterialLedgerId::site(from_ledger),
        to_ledger: MaterialLedgerId::site(to_ledger),
        kind: kind.to_string(),
        amount,
        distance_km: 100,
        priority: Some(MaterialTransitPriority::Standard),
        route_id: Some(route_id.to_string()),
        route_ids: Vec::new(),
        auto_reroute: false,
    });
    world.step().expect("start recipe route transit");
    world.step().expect("complete recipe route transit");
    assert!(
        world
            .state()
            .completed_logistics_route_ids
            .contains(route_id),
        "completed route should be available for explicit recipe binding"
    );
}

fn assert_recipe_route_rejected(
    world: &mut World,
    factory_id: &str,
    recipe_id: &str,
    plan: &RecipeExecutionPlan,
    logistics_route_ids: Vec<String>,
) {
    let pending_before = world.state().pending_recipe_jobs.len();
    let completed_before = world.state().industry_progress.completed_recipe_jobs;
    let journal_before = world.journal().events.len();
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: factory_id.to_string(),
        recipe_id: recipe_id.to_string(),
        plan: plan.clone(),
        logistics_route_ids,
        logistics_path_ids: Vec::new(),
    });
    world.step().expect("reject recipe route binding");
    assert!(
        world.journal().events[journal_before..]
            .iter()
            .any(|event| {
                matches!(
                    &event.body,
                    WorldEventBody::Domain(DomainEvent::ActionRejected { .. })
                )
            })
    );
    assert_eq!(world.state().pending_recipe_jobs.len(), pending_before);
    assert_eq!(
        world.state().industry_progress.completed_recipe_jobs,
        completed_before
    );
    assert_eq!(
        world.state().industry_progress.stage,
        IndustryStage::Bootstrap
    );
}

fn complete_bound_recipe(
    world: &mut World,
    factory_id: &str,
    recipe_id: &str,
    plan: &RecipeExecutionPlan,
    logistics_route_ids: Vec<String>,
) {
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: factory_id.to_string(),
        recipe_id: recipe_id.to_string(),
        plan: plan.clone(),
        logistics_route_ids,
        logistics_path_ids: Vec::new(),
    });
    world.step().expect("start explicit route recipe");
    world.step().expect("complete explicit route recipe");
}

#[test]
fn completed_route_binds_recipe_start_job_completion_and_world_state() {
    let factory_id = "factory.recipe.route.binding";
    let recipe_id = "recipe.recipe.route.binding";
    let mut world = recipe_route_fixture(factory_id);
    let route_id = register_recipe_route(&mut world, "source-a", "site-1", "iron_ingot");
    complete_recipe_route_transfer(&mut world, &route_id, "source-a", "site-1", "iron_ingot", 2);
    let plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("iron_ingot", 2)],
        vec![MaterialStack::new("gear", 1)],
        Vec::new(),
        1,
        1,
    );

    let journal_start = world.journal().events.len();
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: factory_id.to_string(),
        recipe_id: recipe_id.to_string(),
        plan,
        logistics_route_ids: vec![route_id.clone()],
        logistics_path_ids: Vec::new(),
    });
    world.step().expect("start route-bound recipe");
    assert!(world.journal().events[journal_start..].iter().any(|event| {
        matches!(
            &event.body,
            WorldEventBody::Domain(DomainEvent::RecipeStarted {
                logistics_route_ids,
                ..
            }) if logistics_route_ids == &vec![route_id.clone()]
        )
    }));
    assert_eq!(
        world
            .state()
            .pending_recipe_jobs
            .values()
            .next()
            .map(|job| job.logistics_route_ids.clone()),
        Some(vec![route_id.clone()])
    );

    world.step().expect("complete route-bound recipe");
    assert!(world.journal().events.iter().rev().any(|event| {
        matches!(
            &event.body,
            WorldEventBody::Domain(DomainEvent::RecipeCompleted {
                logistics_route_ids,
                ..
            }) if logistics_route_ids == &vec![route_id.clone()]
        )
    }));
    assert!(
        world
            .state()
            .completed_logistics_route_ids
            .contains(&route_id)
    );
}

#[test]
fn incomplete_unknown_or_incompatible_recipe_routes_reject_without_progress() {
    let factory_id = "factory.recipe.route.reject";
    let recipe_id = "recipe.recipe.route.reject";
    let mut world = recipe_route_fixture(factory_id);
    let plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("iron_ingot", 2)],
        vec![MaterialStack::new("gear", 1)],
        Vec::new(),
        1,
        1,
    );

    let uncompleted =
        register_recipe_route(&mut world, "source-uncompleted", "site-1", "iron_ingot");
    assert_recipe_route_rejected(
        &mut world,
        factory_id,
        recipe_id,
        &plan,
        vec!["route-unknown".to_string()],
    );
    assert_recipe_route_rejected(&mut world, factory_id, recipe_id, &plan, vec![uncompleted]);

    let wrong_destination =
        register_recipe_route(&mut world, "source-destination", "site-wrong", "iron_ingot");
    complete_recipe_route_transfer(
        &mut world,
        &wrong_destination,
        "source-destination",
        "site-wrong",
        "iron_ingot",
        2,
    );
    assert_recipe_route_rejected(
        &mut world,
        factory_id,
        recipe_id,
        &plan,
        vec![wrong_destination],
    );

    let wrong_material =
        register_recipe_route(&mut world, "source-material", "site-1", "copper_wire");
    complete_recipe_route_transfer(
        &mut world,
        &wrong_material,
        "source-material",
        "site-1",
        "copper_wire",
        2,
    );
    assert_recipe_route_rejected(
        &mut world,
        factory_id,
        recipe_id,
        &plan,
        vec![wrong_material],
    );

    let route_a = register_recipe_route(&mut world, "source-a", "site-1", "iron_ingot");
    complete_recipe_route_transfer(&mut world, &route_a, "source-a", "site-1", "iron_ingot", 2);
    let route_b = register_recipe_route(&mut world, "source-b", "site-1", "iron_ingot");
    complete_recipe_route_transfer(&mut world, &route_b, "source-b", "site-1", "iron_ingot", 2);
    assert_recipe_route_rejected(
        &mut world,
        factory_id,
        recipe_id,
        &plan,
        vec![route_a.clone(), route_a.clone()],
    );
    assert_recipe_route_rejected(
        &mut world,
        factory_id,
        recipe_id,
        &plan,
        vec![route_b, route_a],
    );
}

#[test]
fn changing_completed_route_resets_stable_line_identity_but_retries_same_route() {
    let factory_id = "factory.recipe.route.identity";
    let recipe_id = "recipe.recipe.route.identity";
    let mut world = recipe_route_fixture(factory_id);
    let plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("iron_ingot", 2)],
        vec![MaterialStack::new("gear", 1)],
        Vec::new(),
        1,
        1,
    );
    let route_a = register_recipe_route(&mut world, "source-identity-a", "site-1", "iron_ingot");
    complete_recipe_route_transfer(
        &mut world,
        &route_a,
        "source-identity-a",
        "site-1",
        "iron_ingot",
        4,
    );
    let route_b = register_recipe_route(&mut world, "source-identity-b", "site-1", "iron_ingot");
    complete_recipe_route_transfer(
        &mut world,
        &route_b,
        "source-identity-b",
        "site-1",
        "iron_ingot",
        6,
    );

    complete_bound_recipe(
        &mut world,
        factory_id,
        recipe_id,
        &plan,
        vec![route_a.clone()],
    );
    let path_a = world
        .state()
        .factories
        .get(factory_id)
        .and_then(|factory| {
            factory
                .production
                .last_completed_canonical_snapshot
                .as_ref()
        })
        .map(|snapshot| snapshot.logistics_path_ids.clone())
        .expect("route A stable-line path identity");
    assert!(!path_a.is_empty());
    complete_bound_recipe(
        &mut world,
        factory_id,
        recipe_id,
        &plan,
        vec![route_a.clone()],
    );
    let factory = world
        .state()
        .factories
        .get(factory_id)
        .expect("factory after route A");
    assert_eq!(factory.production.same_recipe_repeat_count, 2);
    assert_eq!(
        world.state().industry_progress.stage,
        IndustryStage::Bootstrap
    );

    complete_bound_recipe(
        &mut world,
        factory_id,
        recipe_id,
        &plan,
        vec![route_b.clone()],
    );
    let factory = world
        .state()
        .factories
        .get(factory_id)
        .expect("factory after route B reset");
    assert_eq!(factory.production.same_recipe_repeat_count, 1);
    let path_b = factory
        .production
        .last_completed_canonical_snapshot
        .as_ref()
        .expect("route B stable-line path identity")
        .logistics_path_ids
        .clone();
    assert!(!path_b.is_empty());
    assert_ne!(path_a, path_b, "effective path change must reset identity");
    assert_eq!(
        world.state().industry_progress.stage,
        IndustryStage::Bootstrap
    );

    complete_bound_recipe(
        &mut world,
        factory_id,
        recipe_id,
        &plan,
        vec![route_b.clone()],
    );
    complete_bound_recipe(&mut world, factory_id, recipe_id, &plan, vec![route_b]);
    let factory = world
        .state()
        .factories
        .get(factory_id)
        .expect("factory after route B retries");
    assert_eq!(factory.production.same_recipe_repeat_count, 3);
    assert_eq!(
        world.state().industry_progress.stage,
        IndustryStage::ScaleOut
    );
}

#[test]
fn schedule_recipe_rejects_when_profile_stage_gate_exceeds_current_stage() {
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
        spec: factory_spec("factory.stage_gate", 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("factory ready");

    world
        .upsert_recipe_profile(RecipeProfileV1 {
            recipe_id: "recipe.profile.governance".to_string(),
            bottleneck_tags: vec!["gear".to_string()],
            stage_gate: "governance".to_string(),
            preferred_factory_tags: vec!["assembly".to_string()],
        })
        .expect("insert recipe profile");

    let plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("iron_ingot", 1)],
        vec![MaterialStack::new("module_rack", 1)],
        Vec::new(),
        1,
        1,
    );
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.stage_gate".to_string(),
        recipe_id: "recipe.profile.governance".to_string(),
        plan,
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world.step().expect("schedule blocked by stage gate");

    let message = latest_action_rejected_message(&world);
    assert!(
        message.contains("stage gate denied"),
        "expected stage gate reject, got {message}"
    );

    let (blocker_kind, blocker_detail) = latest_factory_production_block(&world);
    assert_eq!(blocker_kind, "governance_gate");
    assert!(blocker_detail.contains("stage gate denied"));
}

#[test]
fn schedule_recipe_rejects_when_product_unlock_stage_exceeds_current_stage() {
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
        spec: factory_spec("factory.unlock_stage", 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("factory ready");

    world
        .upsert_recipe_profile(RecipeProfileV1 {
            recipe_id: "recipe.profile.unlock_stage".to_string(),
            bottleneck_tags: Vec::new(),
            stage_gate: "bootstrap".to_string(),
            preferred_factory_tags: vec!["assembly".to_string()],
        })
        .expect("insert recipe profile");
    world
        .upsert_product_profile(ProductProfileV1 {
            product_id: "gear".to_string(),
            role_tag: "scale".to_string(),
            maintenance_sink: Vec::new(),
            tradable: true,
            unlock_stage: "governance".to_string(),
        })
        .expect("insert product profile");
    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-1"), "iron_ingot", 2)
        .expect("seed local material");
    world
        .set_material_balance("iron_ingot", 2)
        .expect("seed world material");
    world.set_resource_balance(ResourceKind::Electricity, 10);

    let plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("iron_ingot", 1)],
        vec![MaterialStack::new("gear", 1)],
        Vec::new(),
        1,
        1,
    );
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.unlock_stage".to_string(),
        recipe_id: "recipe.profile.unlock_stage".to_string(),
        plan,
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world
        .step()
        .expect("schedule blocked by product unlock stage");

    let message = latest_action_rejected_message(&world);
    assert!(
        message.contains("product unlock_stage denied"),
        "expected product unlock_stage reject, got {message}"
    );

    let (blocker_kind, blocker_detail) = latest_factory_production_block(&world);
    assert_eq!(blocker_kind, "governance_gate");
    assert!(blocker_detail.contains("product unlock_stage denied"));
}

#[test]
fn schedule_recipe_rejects_when_factory_tags_conflict_with_recipe_profile() {
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
        spec: factory_spec("factory.preferred_tag", 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("factory ready");

    world
        .upsert_recipe_profile(RecipeProfileV1 {
            recipe_id: "recipe.profile.tagged".to_string(),
            bottleneck_tags: Vec::new(),
            stage_gate: "bootstrap".to_string(),
            preferred_factory_tags: vec!["smelter".to_string()],
        })
        .expect("insert recipe profile");

    let plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("iron_ingot", 1)],
        vec![MaterialStack::new("gear", 1)],
        Vec::new(),
        1,
        1,
    );
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.preferred_tag".to_string(),
        recipe_id: "recipe.profile.tagged".to_string(),
        plan,
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world.step().expect("schedule blocked by preferred tag");

    let message = latest_action_rejected_message(&world);
    assert!(
        message.contains("preferred_factory_tags mismatch"),
        "expected preferred tag reject, got {message}"
    );

    let (blocker_kind, blocker_detail) = latest_factory_production_block(&world);
    assert_eq!(blocker_kind, "governance_gate");
    assert!(blocker_detail.contains("preferred_factory_tags mismatch"));
}

#[test]
fn schedule_recipe_uses_profile_bottleneck_tags_before_inference() {
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
        spec: factory_spec("factory.bottleneck.profile", 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("factory ready");

    world
        .upsert_recipe_profile(RecipeProfileV1 {
            recipe_id: "recipe.profile.bottleneck".to_string(),
            bottleneck_tags: vec!["Copper_Wire".to_string()],
            stage_gate: "bootstrap".to_string(),
            preferred_factory_tags: vec!["assembly".to_string()],
        })
        .expect("insert recipe profile");

    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-1"), "gear", 4)
        .expect("seed local material");
    world
        .set_material_balance("gear", 4)
        .expect("seed world material");
    world.set_resource_balance(ResourceKind::Electricity, 20);

    let plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("gear", 2)],
        vec![MaterialStack::new("factory_frame", 1)],
        Vec::new(),
        1,
        1,
    );
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.bottleneck.profile".to_string(),
        recipe_id: "recipe.profile.bottleneck".to_string(),
        plan,
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world.step().expect("start profile bottleneck recipe");

    let bottleneck_tags = world
        .journal()
        .events
        .last()
        .and_then(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::RecipeStarted {
                bottleneck_tags, ..
            }) => Some(bottleneck_tags.clone()),
            _ => None,
        })
        .expect("recipe started");
    assert_eq!(bottleneck_tags, vec!["copper_wire".to_string()]);
}

#[test]
fn due_recipe_jobs_prioritize_by_product_role_tag_before_keyword_fallback() {
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
        spec: factory_spec("factory.role_tag", 1, 2),
    });
    world.step().expect("start build");
    world.step().expect("factory ready");

    world
        .upsert_product_profile(ProductProfileV1 {
            product_id: "alpha_widget".to_string(),
            role_tag: "survival".to_string(),
            maintenance_sink: Vec::new(),
            tradable: true,
            unlock_stage: "bootstrap".to_string(),
        })
        .expect("insert survival profile");
    world
        .upsert_product_profile(ProductProfileV1 {
            product_id: "delta_widget".to_string(),
            role_tag: "scale".to_string(),
            maintenance_sink: Vec::new(),
            tradable: true,
            unlock_stage: "bootstrap".to_string(),
        })
        .expect("insert scale profile");

    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-1"), "gear", 6)
        .expect("seed local material");
    world
        .set_material_balance("gear", 6)
        .expect("seed world material");
    world.set_resource_balance(ResourceKind::Electricity, 20);

    let scale_plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("gear", 2)],
        vec![MaterialStack::new("delta_widget", 1)],
        Vec::new(),
        1,
        1,
    );
    let survival_plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("gear", 2)],
        vec![MaterialStack::new("alpha_widget", 1)],
        Vec::new(),
        1,
        1,
    );

    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.role_tag".to_string(),
        recipe_id: "recipe.misc.scale_a".to_string(),
        plan: scale_plan,
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.role_tag".to_string(),
        recipe_id: "recipe.misc.survival_b".to_string(),
        plan: survival_plan,
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world.step().expect("start recipes");
    assert_eq!(world.pending_recipe_jobs_len(), 2);

    let before = world.journal().events.len();
    world.step().expect("complete recipes");

    let completed_recipe_ids: Vec<String> = world.journal().events[before..]
        .iter()
        .filter_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::RecipeCompleted { recipe_id, .. }) => {
                Some(recipe_id.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        completed_recipe_ids,
        vec![
            "recipe.misc.survival_b".to_string(),
            "recipe.misc.scale_a".to_string(),
        ]
    );
}

#[test]
fn schedule_recipe_applies_product_maintenance_sink_to_consume() {
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
        spec: factory_spec("factory.maintenance_sink", 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("factory ready");

    world
        .upsert_recipe_profile(RecipeProfileV1 {
            recipe_id: "recipe.profile.maintenance_sink".to_string(),
            bottleneck_tags: vec!["iron_ingot".to_string()],
            stage_gate: "bootstrap".to_string(),
            preferred_factory_tags: vec!["assembly".to_string()],
        })
        .expect("insert recipe profile");
    world
        .upsert_product_profile(ProductProfileV1 {
            product_id: "durable_part".to_string(),
            role_tag: "scale".to_string(),
            maintenance_sink: vec![MaterialStack::new("hardware_part", 2)],
            tradable: true,
            unlock_stage: "bootstrap".to_string(),
        })
        .expect("insert product profile");
    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-1"), "iron_ingot", 2)
        .expect("seed local iron");
    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-1"), "hardware_part", 4)
        .expect("seed local hardware");
    world
        .set_material_balance("iron_ingot", 2)
        .expect("seed world iron");
    world
        .set_material_balance("hardware_part", 4)
        .expect("seed world hardware");
    world.set_resource_balance(ResourceKind::Electricity, 10);

    let plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("iron_ingot", 1)],
        vec![MaterialStack::new("durable_part", 2)],
        Vec::new(),
        1,
        1,
    );
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.maintenance_sink".to_string(),
        recipe_id: "recipe.profile.maintenance_sink".to_string(),
        plan,
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world.step().expect("schedule with maintenance sink");

    let consume = world
        .journal()
        .events
        .last()
        .and_then(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::RecipeStarted { consume, .. }) => {
                Some(consume.clone())
            }
            _ => None,
        })
        .expect("recipe started");
    let mut consume_map = std::collections::BTreeMap::new();
    for stack in consume {
        consume_map.insert(stack.kind, stack.amount);
    }
    assert_eq!(consume_map.get("iron_ingot"), Some(&1));
    assert_eq!(consume_map.get("hardware_part"), Some(&4));
}

#[test]
fn recipe_started_market_quote_reflects_governance_tax_change() {
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
        spec: factory_spec("factory.quote", 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("factory ready");

    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-1"), "iron_ingot", 12)
        .expect("seed local recipe input");
    world
        .set_material_balance("iron_ingot", 100)
        .expect("seed world recipe input");
    world.set_resource_balance(ResourceKind::Electricity, 50);

    authorize_policy_update(&mut world, "builder-a", "proposal.policy.zero-tax");
    world.submit_action(Action::UpdateGameplayPolicy {
        operator_agent_id: "builder-a".to_string(),
        electricity_tax_bps: 0,
        data_tax_bps: 0,
        power_trade_fee_bps: 0,
        max_open_contracts_per_agent: 16,
        blocked_agents: Vec::new(),
        forbidden_location_ids: Vec::new(),
    });
    world.step().expect("set zero tax policy");

    let plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("iron_ingot", 2)],
        vec![MaterialStack::new("gear", 1)],
        Vec::new(),
        1,
        1,
    );
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.quote".to_string(),
        recipe_id: "recipe.quote.low_tax".to_string(),
        plan: plan.clone(),
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world.step().expect("start low tax recipe");

    let low_tax_quote = match &world.journal().events.last().expect("recipe started").body {
        WorldEventBody::Domain(DomainEvent::RecipeStarted { market_quotes, .. }) => market_quotes
            .iter()
            .find(|quote| quote.kind == "iron_ingot")
            .expect("iron quote under low tax")
            .clone(),
        other => panic!("expected RecipeStarted, got {other:?}"),
    };
    assert_eq!(low_tax_quote.governance_tax_bps, 0);

    world.step().expect("complete low tax recipe");

    authorize_policy_update(&mut world, "builder-a", "proposal.policy.high-tax");
    world.submit_action(Action::UpdateGameplayPolicy {
        operator_agent_id: "builder-a".to_string(),
        electricity_tax_bps: 900,
        data_tax_bps: 700,
        power_trade_fee_bps: 0,
        max_open_contracts_per_agent: 16,
        blocked_agents: Vec::new(),
        forbidden_location_ids: Vec::new(),
    });
    world.step().expect("set high tax policy");

    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.quote".to_string(),
        recipe_id: "recipe.quote.high_tax".to_string(),
        plan,
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world.step().expect("start high tax recipe");

    let high_tax_quote = match &world.journal().events.last().expect("recipe started").body {
        WorldEventBody::Domain(DomainEvent::RecipeStarted { market_quotes, .. }) => market_quotes
            .iter()
            .find(|quote| quote.kind == "iron_ingot")
            .expect("iron quote under high tax")
            .clone(),
        other => panic!("expected RecipeStarted, got {other:?}"),
    };
    assert_eq!(high_tax_quote.governance_tax_bps, 1_600);
    assert!(
        high_tax_quote.effective_cost_index_ppm > low_tax_quote.effective_cost_index_ppm,
        "expected effective cost to increase with governance tax: low={:?} high={:?}",
        low_tax_quote,
        high_tax_quote
    );
}

#[test]
fn recipe_started_market_quote_uses_material_profile_transport_loss() {
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
        spec: factory_spec("factory.quote.profile_loss", 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("factory ready");

    world
        .upsert_material_profile(MaterialProfileV1 {
            kind: "iron_ingot".to_string(),
            tier: 2,
            category: "intermediate".to_string(),
            stack_limit: 500,
            transport_loss_class: MaterialTransportLossClass::High,
            decay_bps_per_tick: 0,
            default_priority: MaterialDefaultPriority::Standard,
        })
        .expect("insert iron profile");

    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-1"), "iron_ingot", 12)
        .expect("seed local recipe input");
    world
        .set_material_balance("iron_ingot", 100)
        .expect("seed world recipe input");
    world.set_resource_balance(ResourceKind::Electricity, 50);

    let plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("iron_ingot", 2)],
        vec![MaterialStack::new("gear", 1)],
        Vec::new(),
        1,
        1,
    );
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.quote.profile_loss".to_string(),
        recipe_id: "recipe.quote.profile_loss".to_string(),
        plan,
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world.step().expect("start recipe");

    let quote = match &world.journal().events.last().expect("recipe started").body {
        WorldEventBody::Domain(DomainEvent::RecipeStarted { market_quotes, .. }) => market_quotes
            .iter()
            .find(|quote| quote.kind == "iron_ingot")
            .expect("iron quote")
            .clone(),
        other => panic!("expected RecipeStarted, got {other:?}"),
    };
    assert_eq!(quote.transit_loss_bps, 20);
}

// The following tests are the RED contract for the explicit logistics-network
// authority selected in issue #3272.  They intentionally exercise public
// actions/events instead of implementation maps so the runtime can choose its
// reservation/index representation without weakening the behavioral gate.

fn logistics_network_fixture() -> World {
    let mut world = World::new();
    for agent_id in ["owner-a", "sender-a", "sender-b"] {
        world.submit_action(Action::RegisterAgent {
            agent_id: agent_id.to_string(),
            pos: pos(0, 0),
        });
        world.step().expect("register logistics-network agent");
    }
    world
        .set_agent_resource_balance("sender-a", ResourceKind::Electricity, 100)
        .expect("seed sender-a electricity");
    world
        .set_agent_resource_balance("sender-b", ResourceKind::Electricity, 100)
        .expect("seed sender-b electricity");
    world
        .set_ledger_material_balance(MaterialLedgerId::site("source"), "steel_plate", 20)
        .expect("seed logistics-network source");
    world
}

fn register_network_edge(
    world: &mut World,
    owner_agent_id: &str,
    from_ledger: &str,
    to_ledger: &str,
    distance_km: i64,
    capacity_units: i64,
    tariff_electricity_per_unit: i64,
) -> String {
    let journal_before = world.journal().events.len();
    world.submit_action(Action::RegisterLogisticsRoute {
        requester_agent_id: owner_agent_id.to_string(),
        from_ledger: MaterialLedgerId::site(from_ledger),
        to_ledger: MaterialLedgerId::site(to_ledger),
        kind: "steel_plate".to_string(),
        distance_km,
        priority: MaterialTransitPriority::Standard,
        capacity_units,
        tariff_electricity_per_unit,
    });
    world.step().expect("register logistics-network edge");
    world.journal().events[journal_before..]
        .iter()
        .rev()
        .find_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::LogisticsRouteRegistered {
                route_id: Some(route_id),
                from_ledger: event_from,
                to_ledger: event_to,
                kind,
                distance_km: event_distance,
                ..
            }) if event_from == &MaterialLedgerId::site(from_ledger)
                && event_to == &MaterialLedgerId::site(to_ledger)
                && kind == "steel_plate"
                && *event_distance == distance_km =>
            {
                Some(route_id.clone())
            }
            _ => None,
        })
        .expect("LogisticsRouteRegistered event in registration step")
}

fn transfer_over_path(
    requester_agent_id: &str,
    from_ledger: &str,
    to_ledger: &str,
    amount: i64,
    route_ids: Vec<String>,
    auto_reroute: bool,
) -> Action {
    Action::TransferMaterial {
        requester_agent_id: requester_agent_id.to_string(),
        from_ledger: MaterialLedgerId::site(from_ledger),
        to_ledger: MaterialLedgerId::site(to_ledger),
        kind: "steel_plate".to_string(),
        amount,
        // A non-empty explicit path derives distance from its edge snapshot;
        // zero keeps this fixture independent of any future aggregate-distance
        // balancing rule.
        distance_km: 0,
        priority: None,
        route_id: None,
        route_ids,
        auto_reroute,
    }
}

fn latest_started_path(world: &World) -> (Option<String>, Vec<String>, i64, u32) {
    world
        .journal()
        .events
        .iter()
        .rev()
        .find_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::MaterialTransitStarted {
                path_id,
                route_ids,
                tariff_electricity_total,
                reroute_count,
                ..
            }) => Some((
                path_id.clone(),
                route_ids.clone(),
                *tariff_electricity_total,
                *reroute_count,
            )),
            _ => None,
        })
        .expect("material transit started event")
}

#[test]
fn logistics_network_selects_deterministic_two_hop_path_and_rejects_cycle_or_unreachable() {
    let mut world = logistics_network_fixture();
    let ab = register_network_edge(&mut world, "owner-a", "source", "mid-a", 100, 10, 2);
    let bd = register_network_edge(&mut world, "owner-a", "mid-a", "destination", 100, 10, 2);
    let ac = register_network_edge(&mut world, "owner-a", "source", "mid-b", 100, 10, 2);
    let cd = register_network_edge(&mut world, "owner-a", "mid-b", "destination", 100, 10, 2);

    let expected = [vec![ab.clone(), bd.clone()], vec![ac.clone(), cd.clone()]]
        .into_iter()
        .min()
        .expect("two deterministic path candidates");
    world.submit_action(transfer_over_path(
        "sender-a",
        "source",
        "destination",
        1,
        Vec::new(),
        true,
    ));
    world.step().expect("select deterministic path");
    let (path_id, route_ids, tariff, reroute_count) = latest_started_path(&world);
    assert!(
        path_id.is_some(),
        "selected path must have persisted identity"
    );
    assert_eq!(route_ids, expected);
    assert_eq!(tariff, 4);
    assert_eq!(reroute_count, 0);

    let cycle_a = register_network_edge(&mut world, "owner-a", "cycle-a", "cycle-b", 100, 10, 1);
    let cycle_b = register_network_edge(&mut world, "owner-a", "cycle-b", "cycle-a", 100, 10, 1);
    let cycle_c =
        register_network_edge(&mut world, "owner-a", "cycle-a", "destination", 100, 10, 1);
    world
        .set_ledger_material_balance(MaterialLedgerId::site("cycle-a"), "steel_plate", 1)
        .expect("seed cycle source");
    let cycle_source_before =
        world.ledger_material_balance(&MaterialLedgerId::site("cycle-a"), "steel_plate");
    let journal_before_cycle = world.journal().events.len();
    world.submit_action(transfer_over_path(
        "sender-a",
        "cycle-a",
        "destination",
        1,
        vec![cycle_a, cycle_b, cycle_c],
        false,
    ));
    world.step().expect("reject cyclic path");
    assert!(
        world.journal().events[journal_before_cycle..]
            .iter()
            .any(|event| {
                matches!(
                    &event.body,
                    WorldEventBody::Domain(DomainEvent::ActionRejected { .. })
                )
            })
    );
    assert_eq!(
        world.ledger_material_balance(&MaterialLedgerId::site("cycle-a"), "steel_plate"),
        cycle_source_before,
        "cyclic path rejection must not debit source material"
    );
    assert_eq!(world.pending_material_transits_len(), 0);
    let cycle_rejection = latest_action_rejected_message(&world).to_ascii_lowercase();
    assert!(!cycle_rejection.contains("insufficient"));
    assert!(
        cycle_rejection.contains("cycle")
            || cycle_rejection.contains("loop")
            || cycle_rejection.contains("path")
            || cycle_rejection.contains("route")
    );

    let journal_before_unreachable = world.journal().events.len();
    world.submit_action(transfer_over_path(
        "sender-a",
        "source",
        "no-such-destination",
        1,
        Vec::new(),
        true,
    ));
    world.step().expect("reject unreachable destination");
    assert!(
        world.journal().events[journal_before_unreachable..]
            .iter()
            .any(|event| {
                matches!(
                    &event.body,
                    WorldEventBody::Domain(DomainEvent::ActionRejected { .. })
                )
            })
    );
    let unreachable_rejection = latest_action_rejected_message(&world).to_ascii_lowercase();
    assert!(!unreachable_rejection.contains("insufficient"));
    assert!(
        unreachable_rejection.contains("unreachable")
            || unreachable_rejection.contains("no route")
            || unreachable_rejection.contains("path")
            || unreachable_rejection.contains("route")
    );
}

#[test]
fn logistics_network_reserves_whole_path_atomically_and_releases_capacity_once() {
    let mut world = logistics_network_fixture();
    let ab = register_network_edge(&mut world, "owner-a", "source", "mid", 100, 1, 1);
    let bd = register_network_edge(&mut world, "owner-a", "mid", "destination", 100, 1, 1);
    let path = vec![ab, bd];

    world.submit_action(transfer_over_path(
        "sender-a",
        "source",
        "destination",
        1,
        path.clone(),
        false,
    ));
    world.step().expect("reserve first whole path");
    assert_eq!(world.pending_material_transits_len(), 1);
    let source_after_first =
        world.ledger_material_balance(&MaterialLedgerId::site("source"), "steel_plate");

    world.submit_action(transfer_over_path(
        "sender-b",
        "source",
        "destination",
        1,
        path.clone(),
        false,
    ));
    world
        .step()
        .expect("reject second path on capacity conflict");
    assert_eq!(world.pending_material_transits_len(), 1);
    assert_eq!(
        world.ledger_material_balance(&MaterialLedgerId::site("source"), "steel_plate"),
        source_after_first,
        "capacity rejection must not debit source or leave a partial reservation"
    );

    world
        .step()
        .expect("complete first path and release capacity");
    assert_eq!(world.pending_material_transits_len(), 0);
    world.submit_action(transfer_over_path(
        "sender-b",
        "source",
        "destination",
        1,
        path,
        false,
    ));
    world.step().expect("reserve path after release");
    assert_eq!(world.pending_material_transits_len(), 1);
}

#[test]
fn logistics_network_owner_only_availability_and_fixed_tariff_settle_once() {
    let mut world = logistics_network_fixture();
    let route = register_network_edge(&mut world, "owner-a", "source", "destination", 100, 10, 3);

    world.submit_action(Action::SetLogisticsRouteAvailability {
        requester_agent_id: "sender-a".to_string(),
        route_id: route.clone(),
        available: false,
    });
    world.step().expect("reject non-owner availability change");
    assert!(matches!(
        world.journal().events.last().map(|event| &event.body),
        Some(WorldEventBody::Domain(DomainEvent::ActionRejected { .. }))
    ));

    world.submit_action(Action::SetLogisticsRouteAvailability {
        requester_agent_id: "owner-a".to_string(),
        route_id: route.clone(),
        available: false,
    });
    world.step().expect("owner disables route");
    world.submit_action(transfer_over_path(
        "sender-a",
        "source",
        "destination",
        2,
        vec![route.clone()],
        false,
    ));
    world.step().expect("reject unavailable route");
    assert!(matches!(
        world.journal().events.last().map(|event| &event.body),
        Some(WorldEventBody::Domain(DomainEvent::ActionRejected { .. }))
    ));

    world.submit_action(Action::SetLogisticsRouteAvailability {
        requester_agent_id: "owner-a".to_string(),
        route_id: route.clone(),
        available: true,
    });
    world.step().expect("owner enables route");
    world
        .set_agent_resource_balance("sender-a", ResourceKind::Electricity, 20)
        .expect("seed sender electricity");
    world
        .set_agent_resource_balance("owner-a", ResourceKind::Electricity, 0)
        .expect("seed owner electricity");
    world.submit_action(transfer_over_path(
        "sender-a",
        "source",
        "destination",
        2,
        vec![route],
        false,
    ));
    world.step().expect("start tariff-bearing transit");
    world.step().expect("complete tariff-bearing transit");

    assert_eq!(
        world
            .agent_resource_balance("sender-a", ResourceKind::Electricity)
            .expect("sender balance"),
        14,
        "fixed tariff must debit sender once: 2 units * 3 Electricity"
    );
    assert_eq!(
        world
            .agent_resource_balance("owner-a", ResourceKind::Electricity)
            .expect("owner balance"),
        6,
        "fixed tariff must pay edge owner once"
    );
    let sender_after_settlement = world
        .agent_resource_balance("sender-a", ResourceKind::Electricity)
        .expect("sender balance after settlement");
    let owner_after_settlement = world
        .agent_resource_balance("owner-a", ResourceKind::Electricity)
        .expect("owner balance after settlement");
    world.step().expect("replay-free idle tick");
    assert_eq!(
        world
            .agent_resource_balance("sender-a", ResourceKind::Electricity)
            .expect("sender balance after idle tick"),
        sender_after_settlement
    );
    assert_eq!(
        world
            .agent_resource_balance("owner-a", ResourceKind::Electricity)
            .expect("owner balance after idle tick"),
        owner_after_settlement
    );
}

#[test]
fn logistics_network_insufficient_electricity_rejects_before_material_or_capacity_mutation() {
    let mut world = logistics_network_fixture();
    let route = register_network_edge(&mut world, "owner-a", "source", "destination", 100, 10, 3);
    world
        .set_agent_resource_balance("sender-a", ResourceKind::Electricity, 5)
        .expect("seed insufficient sender electricity");

    let source_ledger = MaterialLedgerId::site("source");
    let source_before = world.ledger_material_balance(&source_ledger, "steel_plate");
    let electricity_before = world
        .agent_resource_balance("sender-a", ResourceKind::Electricity)
        .expect("sender electricity before rejection");
    let pending_before = world.pending_material_transits_len();
    world.submit_action(transfer_over_path(
        "sender-a",
        "source",
        "destination",
        2,
        vec![route],
        false,
    ));
    world.step().expect("reject unaffordable tariff");

    let rejection = latest_action_rejected_message(&world).to_ascii_lowercase();
    assert!(
        rejection.contains("electricity") || rejection.contains("fund"),
        "expected insufficient Electricity rejection, got {rejection}"
    );
    assert_eq!(
        world.ledger_material_balance(&source_ledger, "steel_plate"),
        source_before,
        "insufficient tariff must not debit source material"
    );
    assert_eq!(
        world
            .agent_resource_balance("sender-a", ResourceKind::Electricity)
            .expect("sender electricity after rejection"),
        electricity_before,
        "insufficient tariff must not debit sender Electricity"
    );
    assert_eq!(
        world.pending_material_transits_len(),
        pending_before,
        "insufficient tariff must not reserve path capacity or enqueue transit"
    );
}

#[test]
fn logistics_network_opt_in_reroute_attempts_one_deterministic_alternate_without_double_charge() {
    let mut world = logistics_network_fixture();
    let primary_a = register_network_edge(&mut world, "owner-a", "source", "primary", 300, 1, 1);
    let primary_b =
        register_network_edge(&mut world, "owner-a", "primary", "destination", 300, 1, 1);
    let alternate_a =
        register_network_edge(&mut world, "owner-a", "source", "alternate", 300, 1, 5);
    let alternate_b =
        register_network_edge(&mut world, "owner-a", "alternate", "destination", 300, 1, 5);
    let primary = vec![primary_a, primary_b];
    let alternate = vec![alternate_a, alternate_b];

    world
        .set_agent_resource_balance("sender-b", ResourceKind::Electricity, 20)
        .expect("seed primary sender electricity");
    world.submit_action(transfer_over_path(
        "sender-b",
        "source",
        "destination",
        1,
        primary.clone(),
        false,
    ));
    world.step().expect("occupy primary path capacity");
    let started_before = world
        .journal()
        .events
        .iter()
        .filter(|event| {
            matches!(
                event.body,
                WorldEventBody::Domain(DomainEvent::MaterialTransitStarted { .. })
            )
        })
        .count();
    let source_before_no_reroute =
        world.ledger_material_balance(&MaterialLedgerId::site("source"), "steel_plate");
    world.submit_action(transfer_over_path(
        "sender-a",
        "source",
        "destination",
        1,
        primary.clone(),
        false,
    ));
    world.step().expect("reject without reroute opt-in");
    assert_eq!(
        world
            .journal()
            .events
            .iter()
            .filter(|event| {
                matches!(
                    event.body,
                    WorldEventBody::Domain(DomainEvent::MaterialTransitStarted { .. })
                )
            })
            .count(),
        started_before,
        "reroute is opt-in"
    );
    assert_eq!(
        world.ledger_material_balance(&MaterialLedgerId::site("source"), "steel_plate"),
        source_before_no_reroute,
        "failed primary attempt must not debit source"
    );

    world
        .set_agent_resource_balance("sender-a", ResourceKind::Electricity, 20)
        .expect("seed reroute sender electricity");
    world.submit_action(transfer_over_path(
        "sender-a",
        "source",
        "destination",
        1,
        primary,
        true,
    ));
    world.step().expect("perform one deterministic reroute");
    let (_path_id, route_ids, tariff, reroute_count) = latest_started_path(&world);
    assert_eq!(route_ids, alternate);
    assert_eq!(tariff, 10);
    assert_eq!(reroute_count, 1);
    assert_eq!(world.pending_material_transits_len(), 2);
    for _ in 0..8 {
        if world.pending_material_transits_len() == 0 {
            break;
        }
        world.step().expect("complete primary and rerouted paths");
    }
    assert_eq!(world.pending_material_transits_len(), 0);
    assert_eq!(
        world
            .agent_resource_balance("sender-a", ResourceKind::Electricity)
            .expect("reroute sender electricity"),
        10,
        "only the effective alternate path tariff is charged once"
    );
}

#[test]
fn logistics_network_reroute_persists_receipt_and_duplicate_completion_is_idempotent() {
    let mut world = logistics_network_fixture();
    let primary_a = register_network_edge(&mut world, "owner-a", "source", "primary", 300, 1, 1);
    let primary_b =
        register_network_edge(&mut world, "owner-a", "primary", "destination", 300, 1, 1);
    let alternate_a =
        register_network_edge(&mut world, "owner-a", "source", "alternate", 300, 1, 5);
    let alternate_b =
        register_network_edge(&mut world, "owner-a", "alternate", "destination", 300, 1, 5);
    let primary = vec![primary_a, primary_b];
    let alternate = vec![alternate_a, alternate_b];

    world
        .set_agent_resource_balance("sender-b", ResourceKind::Electricity, 20)
        .expect("seed primary sender electricity");
    world.submit_action(transfer_over_path(
        "sender-b",
        "source",
        "destination",
        1,
        primary.clone(),
        false,
    ));
    world.step().expect("occupy primary path capacity");

    world
        .set_agent_resource_balance("sender-a", ResourceKind::Electricity, 20)
        .expect("seed reroute sender electricity");
    world.submit_action(transfer_over_path(
        "sender-a",
        "source",
        "destination",
        1,
        primary,
        true,
    ));
    world.step().expect("perform one deterministic reroute");

    let rerouted_job_id = world
        .journal()
        .events
        .iter()
        .rev()
        .find_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::MaterialTransitStarted {
                job_id,
                route_ids,
                reroute_count,
                tariff_electricity_total,
                ..
            }) if *reroute_count == 1 => {
                assert_eq!(route_ids, &alternate);
                assert_eq!(*tariff_electricity_total, 10);
                Some(*job_id)
            }
            _ => None,
        })
        .expect("rerouted transit job");

    let receipt = world
        .journal()
        .events
        .iter()
        .rev()
        .find(|event| format!("{:?}", event.body).contains("LogisticsPathRerouted"))
        .expect("dedicated reroute receipt event");
    let receipt_debug = format!("{:?}", receipt.body);
    assert!(receipt_debug.contains("reroute"));
    assert!(receipt_debug.contains("tariff"));
    assert!(receipt_debug.contains("owner"));
    assert!(receipt_debug.contains("tax"));
    assert!(receipt_debug.contains(&alternate[0]) && receipt_debug.contains(&alternate[1]));

    for _ in 0..8 {
        if world.pending_material_transits_len() == 0 {
            break;
        }
        world.step().expect("complete rerouted transit");
    }
    let completed = world
        .journal()
        .events
        .iter()
        .find_map(|event| match &event.body {
            WorldEventBody::Domain(
                event @ DomainEvent::MaterialTransitCompleted { job_id, .. },
            ) if *job_id == rerouted_job_id => Some(event.clone()),
            _ => None,
        })
        .expect("rerouted transit completion");
    let mut replay = world.state().clone();
    let state_before_duplicate = serde_json::to_vec(&replay).expect("serialize settled state");
    replay
        .apply_domain_event(&completed, replay.time)
        .expect("duplicate completion replay");
    assert_eq!(
        serde_json::to_vec(&replay).expect("serialize replayed state"),
        state_before_duplicate,
        "duplicate completion must not double release capacity or settle tariff/payout"
    );
}

#[test]
fn logistics_network_legacy_empty_binding_keeps_direct_transfer_when_graph_edges_exist() {
    let mut world = logistics_network_fixture();
    let route = register_network_edge(&mut world, "owner-a", "source", "destination", 100, 10, 7);
    let sender_electricity_before = world
        .agent_resource_balance("sender-a", ResourceKind::Electricity)
        .expect("sender electricity before direct transfer");
    let source_before =
        world.ledger_material_balance(&MaterialLedgerId::site("source"), "steel_plate");
    let journal_before = world.journal().events.len();
    world.submit_action(Action::TransferMaterial {
        requester_agent_id: "sender-a".to_string(),
        from_ledger: MaterialLedgerId::site("source"),
        to_ledger: MaterialLedgerId::site("destination"),
        kind: "steel_plate".to_string(),
        amount: 1,
        distance_km: 0,
        priority: None,
        route_id: None,
        route_ids: Vec::new(),
        auto_reroute: false,
    });
    world
        .step()
        .expect("legacy direct transfer with graph edges");

    assert!(
        world.journal().events[journal_before..]
            .iter()
            .any(|event| {
                matches!(
                    &event.body,
                    WorldEventBody::Domain(DomainEvent::MaterialTransferred {
                        route_id: None,
                        distance_km: 0,
                        ..
                    })
                )
            })
    );
    assert!(
        !world.journal().events[journal_before..]
            .iter()
            .any(|event| {
                matches!(
                    &event.body,
                    WorldEventBody::Domain(DomainEvent::MaterialTransitStarted { .. })
                )
            })
    );
    assert_eq!(world.pending_material_transits_len(), 0);
    assert_eq!(
        world.ledger_material_balance(&MaterialLedgerId::site("source"), "steel_plate"),
        source_before - 1
    );
    assert_eq!(
        world
            .agent_resource_balance("sender-a", ResourceKind::Electricity)
            .expect("sender electricity after direct transfer"),
        sender_electricity_before,
        "legacy empty binding must not silently charge a graph tariff"
    );
    assert_eq!(
        world
            .state()
            .logistics_routes
            .get(&route)
            .expect("registered graph edge")
            .reserved_capacity_units,
        0,
        "legacy empty binding must not reserve graph edge capacity"
    );
}

#[test]
fn logistics_network_zero_distance_explicit_path_retains_path_authority_and_tariff() {
    let mut world = logistics_network_fixture();
    let route = register_network_edge(&mut world, "owner-a", "source", "destination", 0, 10, 4);
    let journal_before = world.journal().events.len();
    world.submit_action(transfer_over_path(
        "sender-a",
        "source",
        "destination",
        2,
        vec![route.clone()],
        false,
    ));
    world.step().expect("start explicit zero-distance path");
    let path_event_debug = world.journal().events[journal_before..]
        .iter()
        .map(|event| format!("{:?}", event.body))
        .find(|debug| {
            debug.contains("MaterialTransitStarted") || debug.contains("MaterialTransitSettled")
        })
        .expect("zero-distance explicit path settlement event");
    assert!(
        !path_event_debug.contains("MaterialTransferred"),
        "explicit path must not fall back to legacy immediate transfer"
    );
    assert!(path_event_debug.contains(&route));
    assert!(path_event_debug.contains("tariff") && path_event_debug.contains('8'));
}

#[test]
fn completed_multi_hop_path_is_the_only_recipe_path_binding_authority() {
    let factory_id = "factory.recipe.path.binding";
    let recipe_id = "recipe.recipe.path.binding";
    let mut world = recipe_route_fixture(factory_id);
    let route_a = register_recipe_route(&mut world, "source-path", "mid-path", "iron_ingot");
    let route_b = register_recipe_route(&mut world, "mid-path", "site-1", "iron_ingot");
    world
        .set_ledger_material_balance(MaterialLedgerId::site("source-path"), "iron_ingot", 2)
        .expect("seed multi-hop path source");
    world.submit_action(Action::TransferMaterial {
        requester_agent_id: "builder-a".to_string(),
        from_ledger: MaterialLedgerId::site("source-path"),
        to_ledger: MaterialLedgerId::site("site-1"),
        kind: "iron_ingot".to_string(),
        amount: 2,
        distance_km: 0,
        priority: Some(MaterialTransitPriority::Standard),
        route_id: None,
        route_ids: vec![route_a, route_b],
        auto_reroute: false,
    });
    world.step().expect("start multi-hop path");
    let started_path_id = world
        .journal()
        .events
        .iter()
        .rev()
        .find_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::MaterialTransitStarted {
                path_id: Some(path_id),
                ..
            }) => Some(path_id.clone()),
            _ => None,
        })
        .expect("multi-hop path id");
    while world.pending_material_transits_len() > 0 {
        world.step().expect("complete multi-hop path");
    }
    let completed_path_id = world
        .journal()
        .events
        .iter()
        .rev()
        .find_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::MaterialTransitCompleted {
                path_id: Some(path_id),
                ..
            }) => Some(path_id.clone()),
            _ => None,
        })
        .expect("completed multi-hop path id");
    assert_eq!(completed_path_id, started_path_id);

    let plan = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("iron_ingot", 2)],
        vec![MaterialStack::new("gear", 1)],
        Vec::new(),
        1,
        1,
    );
    let journal_before_unknown = world.journal().events.len();
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: factory_id.to_string(),
        recipe_id: recipe_id.to_string(),
        plan: plan.clone(),
        logistics_route_ids: Vec::new(),
        logistics_path_ids: vec!["path-unknown".to_string()],
    });
    world.step().expect("reject unknown recipe path binding");
    assert!(
        world.journal().events[journal_before_unknown..]
            .iter()
            .any(|event| {
                matches!(
                    &event.body,
                    WorldEventBody::Domain(DomainEvent::ActionRejected { .. })
                )
            })
    );
    assert_eq!(world.pending_recipe_jobs_len(), 0);

    let journal_before_valid = world.journal().events.len();
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: factory_id.to_string(),
        recipe_id: recipe_id.to_string(),
        plan,
        logistics_route_ids: Vec::new(),
        logistics_path_ids: vec![completed_path_id.clone()],
    });
    world.step().expect("start completed-path recipe");
    assert!(
        world.journal().events[journal_before_valid..]
            .iter()
            .any(|event| {
                matches!(
                    &event.body,
                    WorldEventBody::Domain(DomainEvent::RecipeStarted {
                        logistics_path_ids,
                        ..
                    }) if logistics_path_ids == &vec![completed_path_id.clone()]
                )
            })
    );
    assert_eq!(
        world
            .state()
            .pending_recipe_jobs
            .values()
            .next()
            .map(|job| job.logistics_path_ids.clone()),
        Some(vec![completed_path_id.clone()])
    );
    world.step().expect("complete completed-path recipe");
    assert!(world.journal().events.iter().rev().any(|event| {
        matches!(
            &event.body,
            WorldEventBody::Domain(DomainEvent::RecipeCompleted {
                logistics_path_ids,
                ..
            }) if logistics_path_ids == &vec![completed_path_id.clone()]
        )
    }));
    assert_eq!(
        world
            .state()
            .factories
            .get(factory_id)
            .and_then(|factory| factory
                .production
                .last_completed_canonical_snapshot
                .as_ref())
            .map(|snapshot| snapshot.logistics_path_ids.clone()),
        Some(vec![completed_path_id])
    );
}

#[test]
fn logistics_network_max_hops_is_inclusive_and_explicit_auto_plans_agree() {
    let mut world = logistics_network_fixture();
    let mut max_path = Vec::new();
    let mut from_ledger = "source".to_string();
    for hop in 0..8 {
        let to_ledger = format!("max-hop-{hop}");
        max_path.push(register_network_edge(
            &mut world,
            "owner-a",
            &from_ledger,
            &to_ledger,
            1,
            10,
            0,
        ));
        from_ledger = to_ledger;
    }
    let destination = from_ledger;
    world.submit_action(transfer_over_path(
        "sender-a",
        "source",
        &destination,
        1,
        max_path.clone(),
        false,
    ));
    world.step().expect("accept exactly configured max hops");
    let (_path_id, explicit_route_ids, _tariff, _reroute_count) = latest_started_path(&world);
    assert_eq!(explicit_route_ids, max_path);
    while world.pending_material_transits_len() > 0 {
        world.step().expect("complete max-hop explicit path");
    }

    let mut too_long_path = Vec::new();
    let mut from_ledger = "source".to_string();
    for hop in 0..9 {
        let to_ledger = format!("too-long-hop-{hop}");
        too_long_path.push(register_network_edge(
            &mut world,
            "owner-a",
            &from_ledger,
            &to_ledger,
            1,
            10,
            0,
        ));
        from_ledger = to_ledger;
    }
    let too_long_destination = from_ledger;
    let journal_before_too_long = world.journal().events.len();
    world.submit_action(transfer_over_path(
        "sender-a",
        "source",
        &too_long_destination,
        1,
        too_long_path,
        false,
    ));
    world.step().expect("reject max-hop overflow");
    assert!(
        world.journal().events[journal_before_too_long..]
            .iter()
            .any(|event| {
                matches!(
                    &event.body,
                    WorldEventBody::Domain(DomainEvent::ActionRejected { .. })
                )
            })
    );

    let journal_before_auto = world.journal().events.len();
    world.submit_action(transfer_over_path(
        "sender-a",
        "source",
        &destination,
        1,
        Vec::new(),
        true,
    ));
    world.step().expect("auto-plan exactly configured max hops");
    let (_path_id, auto_route_ids, _tariff, _reroute_count) = latest_started_path(&world);
    assert!(
        world.journal().events[journal_before_auto..]
            .iter()
            .any(|event| {
                matches!(
                    &event.body,
                    WorldEventBody::Domain(DomainEvent::MaterialTransitStarted { .. })
                )
            })
    );
    assert_eq!(auto_route_ids, max_path);
}

#[test]
fn logistics_network_path_and_reservation_identity_survive_serde_replay() {
    let mut world = logistics_network_fixture();
    let route = register_network_edge(&mut world, "owner-a", "source", "destination", 100, 3, 2);
    world.submit_action(transfer_over_path(
        "sender-a",
        "source",
        "destination",
        1,
        vec![route],
        false,
    ));
    world.step().expect("start persisted path");
    let (path_id, route_ids, tariff, reroute_count) = latest_started_path(&world);
    let job = world
        .state()
        .pending_material_transits
        .values()
        .next()
        .expect("pending path job");
    assert_eq!(job.path_id, path_id);
    assert_eq!(job.route_ids, route_ids);
    assert_eq!(job.tariff_electricity_total, tariff);
    assert_eq!(job.reroute_count, reroute_count);

    let state_json = serde_json::to_vec(world.state()).expect("serialize path state");
    let restored: crate::runtime::WorldState =
        serde_json::from_slice(&state_json).expect("deserialize path state");
    let restored_job = restored
        .pending_material_transits
        .values()
        .next()
        .expect("restored pending path job");
    assert_eq!(restored_job, job);
}
