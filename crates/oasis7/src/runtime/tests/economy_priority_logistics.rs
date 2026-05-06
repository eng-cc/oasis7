use super::pos;
use crate::runtime::{
    Action, DomainEvent, FactoryProfileV1, GovernanceProposalStatus, IndustryStage,
    MaterialDefaultPriority, MaterialLedgerId, MaterialProfileV1, MaterialTransitPriority,
    MaterialTransportLossClass, ProductProfileV1, ProposalDecision, RecipeProfileV1, World,
    WorldEventBody,
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
    });
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.priority".to_string(),
        recipe_id: "recipe.survival.oxygen".to_string(),
        plan: survival_plan,
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
    });
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.bottleneck".to_string(),
        recipe_id: "recipe.production.frame.control_chip".to_string(),
        plan: bottleneck_plan,
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
    });
    world.submit_action(Action::TransferMaterial {
        requester_agent_id: "operator-a".to_string(),
        from_ledger: MaterialLedgerId::site("site-c"),
        to_ledger: MaterialLedgerId::site("site-d"),
        kind: "oxygen_pack".to_string(),
        amount: 20,
        distance_km: 100,
        priority: None,
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
    });
    world.submit_action(Action::TransferMaterial {
        requester_agent_id: "operator-a".to_string(),
        from_ledger: MaterialLedgerId::site("site-a"),
        to_ledger: MaterialLedgerId::site("site-c"),
        kind: "copper_wire".to_string(),
        amount: 20,
        distance_km: 100,
        priority: Some(MaterialTransitPriority::Urgent),
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
    });
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.role_tag".to_string(),
        recipe_id: "recipe.misc.survival_b".to_string(),
        plan: survival_plan,
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
