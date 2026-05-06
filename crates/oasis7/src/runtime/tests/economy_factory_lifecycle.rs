use super::pos;
use crate::runtime::{Action, DomainEvent, MaterialLedgerId, RejectReason, World, WorldEventBody};
use crate::simulator::ResourceKind;
use oasis7_wasm_abi::{FactoryModuleSpec, MaterialStack, RecipeExecutionPlan};

fn factory_spec(
    factory_id: &str,
    build_time_ticks: u32,
    recipe_slots: u16,
    maintenance_per_tick: i64,
) -> FactoryModuleSpec {
    FactoryModuleSpec {
        factory_id: factory_id.to_string(),
        display_name: "Lifecycle Factory".to_string(),
        tier: 1,
        tags: vec!["lifecycle".to_string()],
        build_cost: vec![
            MaterialStack::new("steel_plate", 10),
            MaterialStack::new("circuit_board", 2),
        ],
        build_time_ticks,
        base_power_draw: 5,
        recipe_slots,
        throughput_bps: 10_000,
        maintenance_per_tick,
    }
}

fn register_builder(world: &mut World, agent_id: &str) {
    world.submit_action(Action::RegisterAgent {
        agent_id: agent_id.to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register builder");
}

fn build_factory_ready(
    world: &mut World,
    builder_agent_id: &str,
    site_id: &str,
    spec: FactoryModuleSpec,
) {
    world
        .set_material_balance("steel_plate", 20)
        .expect("seed steel");
    world
        .set_material_balance("circuit_board", 4)
        .expect("seed circuits");
    world.submit_action(Action::BuildFactory {
        builder_agent_id: builder_agent_id.to_string(),
        site_id: site_id.to_string(),
        spec,
    });
    world.step().expect("start build");
    world.step().expect("complete build");
}

#[test]
fn factory_depreciation_reduces_durability_each_tick() {
    let mut world = World::new();
    register_builder(&mut world, "builder-a");
    build_factory_ready(
        &mut world,
        "builder-a",
        "site-1",
        factory_spec("factory.alpha", 1, 1, 3),
    );

    let snapshot = world.snapshot();
    let durability_before = snapshot
        .state
        .factories
        .get("factory.alpha")
        .expect("factory exists")
        .durability_ppm;
    assert_eq!(durability_before, 1_000_000);

    world.step().expect("idle tick for depreciation");
    let snapshot = world.snapshot();
    let durability_after = snapshot
        .state
        .factories
        .get("factory.alpha")
        .expect("factory exists")
        .durability_ppm;
    assert_eq!(durability_after, 997_000);

    let last = world.journal().events.last().expect("depreciation event");
    match &last.body {
        WorldEventBody::Domain(DomainEvent::FactoryDurabilityChanged {
            factory_id,
            previous_durability_ppm,
            durability_ppm,
            reason,
        }) => {
            assert_eq!(factory_id, "factory.alpha");
            assert_eq!(*previous_durability_ppm, 1_000_000);
            assert_eq!(*durability_ppm, 997_000);
            assert_eq!(reason, "depreciation_tick");
        }
        other => panic!("expected FactoryDurabilityChanged, got {other:?}"),
    }
}

#[test]
fn factory_depreciation_scales_with_active_recipe_load() {
    let mut world = World::new();
    register_builder(&mut world, "builder-a");
    build_factory_ready(
        &mut world,
        "builder-a",
        "site-1",
        factory_spec("factory.load", 1, 2, 3),
    );

    world
        .set_material_balance("iron_ingot", 2)
        .expect("seed recipe input");
    world.set_resource_balance(ResourceKind::Electricity, 20);
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.load".to_string(),
        recipe_id: "recipe.load".to_string(),
        plan: RecipeExecutionPlan::accepted(
            1,
            vec![MaterialStack::new("iron_ingot", 1)],
            vec![MaterialStack::new("control_chip", 1)],
            Vec::new(),
            1,
            3,
        ),
    });
    world.step().expect("start recipe");
    assert_eq!(world.pending_recipe_jobs_len(), 1);

    let durability_before_loaded_tick = world
        .snapshot()
        .state
        .factories
        .get("factory.load")
        .expect("factory exists")
        .durability_ppm;
    assert_eq!(durability_before_loaded_tick, 997_000);

    world.step().expect("depreciation under load");

    let durability_after_loaded_tick = world
        .snapshot()
        .state
        .factories
        .get("factory.load")
        .expect("factory exists")
        .durability_ppm;
    assert_eq!(
        durability_before_loaded_tick - durability_after_loaded_tick,
        4_500
    );
}

#[test]
fn maintain_factory_consumes_hardware_part_and_recovers_durability() {
    let mut world = World::new();
    register_builder(&mut world, "builder-a");
    build_factory_ready(
        &mut world,
        "builder-a",
        "site-1",
        factory_spec("factory.alpha", 1, 1, 4),
    );
    world.step().expect("depreciate once");
    world
        .set_material_balance("hardware_part", 10)
        .expect("seed hardware part");

    world.submit_action(Action::MaintainFactory {
        operator_agent_id: "builder-a".to_string(),
        factory_id: "factory.alpha".to_string(),
        parts: 2,
    });
    world.step().expect("maintain factory");

    let snapshot = world.snapshot();
    let durability_after = snapshot
        .state
        .factories
        .get("factory.alpha")
        .expect("factory exists")
        .durability_ppm;
    assert_eq!(durability_after, 1_000_000);
    assert_eq!(world.material_balance("hardware_part"), 9);

    let last = world.journal().events.last().expect("maintain event");
    match &last.body {
        WorldEventBody::Domain(DomainEvent::FactoryMaintained {
            factory_id,
            consumed_parts,
            durability_ppm,
            ..
        }) => {
            assert_eq!(factory_id, "factory.alpha");
            assert_eq!(*consumed_parts, 1);
            assert_eq!(*durability_ppm, 1_000_000);
        }
        other => panic!("expected FactoryMaintained, got {other:?}"),
    }
}

#[test]
fn recycle_factory_removes_factory_and_returns_materials() {
    let mut world = World::new();
    register_builder(&mut world, "builder-a");
    build_factory_ready(
        &mut world,
        "builder-a",
        "site-1",
        factory_spec("factory.alpha", 1, 1, 1),
    );
    world.step().expect("depreciate once");

    world.submit_action(Action::RecycleFactory {
        operator_agent_id: "builder-a".to_string(),
        factory_id: "factory.alpha".to_string(),
    });
    world.step().expect("recycle factory");

    assert!(!world.has_factory("factory.alpha"));
    let site_ledger = MaterialLedgerId::site("site-1");
    assert!(world.ledger_material_balance(&site_ledger, "steel_plate") > 0);
    assert!(world.ledger_material_balance(&site_ledger, "circuit_board") > 0);

    let last = world.journal().events.last().expect("recycle event");
    match &last.body {
        WorldEventBody::Domain(DomainEvent::FactoryRecycled {
            factory_id,
            recovered,
            ..
        }) => {
            assert_eq!(factory_id, "factory.alpha");
            assert!(!recovered.is_empty());
        }
        other => panic!("expected FactoryRecycled, got {other:?}"),
    }
}

#[test]
fn schedule_recipe_world_fallback_adds_one_tick_delay_for_moderate_bottleneck_deficit() {
    let mut world = World::new();
    register_builder(&mut world, "builder-a");
    world
        .set_ledger_material_balance(MaterialLedgerId::agent("builder-a"), "steel_plate", 20)
        .expect("seed agent steel");
    world
        .set_ledger_material_balance(MaterialLedgerId::agent("builder-a"), "circuit_board", 4)
        .expect("seed agent circuits");
    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        spec: factory_spec("factory.scarcity.moderate", 1, 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("complete build");

    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-1"), "iron_ingot", 6)
        .expect("seed partial local bottleneck");
    world
        .set_material_balance("iron_ingot", 20)
        .expect("seed world bottleneck");
    world.set_resource_balance(ResourceKind::Electricity, 20);

    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.scarcity.moderate".to_string(),
        recipe_id: "recipe.scarcity.moderate".to_string(),
        plan: RecipeExecutionPlan::accepted(
            1,
            vec![MaterialStack::new("iron_ingot", 10)],
            vec![MaterialStack::new("motor_mk1", 1)],
            Vec::new(),
            1,
            3,
        ),
    });
    world.step().expect("start delayed recipe");

    let now = world.snapshot().state.time;
    let started = world.journal().events.last().expect("recipe started");
    match &started.body {
        WorldEventBody::Domain(DomainEvent::RecipeStarted {
            consume_ledger,
            output_ledger,
            duration_ticks,
            ready_at,
            ..
        }) => {
            assert_eq!(consume_ledger, &MaterialLedgerId::world());
            assert_eq!(output_ledger, &MaterialLedgerId::world());
            assert_eq!(*duration_ticks, 4);
            assert_eq!(*ready_at, now.saturating_add(4));
        }
        other => panic!("expected RecipeStarted, got {other:?}"),
    }

    for _ in 0..3 {
        world.step().expect("wait delayed completion");
    }
    assert_eq!(world.pending_recipe_jobs_len(), 1);
    world.step().expect("complete delayed recipe");
    assert_eq!(world.pending_recipe_jobs_len(), 0);
}

#[test]
fn schedule_recipe_world_fallback_adds_two_tick_delay_for_severe_bottleneck_deficit() {
    let mut world = World::new();
    register_builder(&mut world, "builder-a");
    world
        .set_ledger_material_balance(MaterialLedgerId::agent("builder-a"), "steel_plate", 20)
        .expect("seed agent steel");
    world
        .set_ledger_material_balance(MaterialLedgerId::agent("builder-a"), "circuit_board", 4)
        .expect("seed agent circuits");
    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        spec: factory_spec("factory.scarcity.severe", 1, 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("complete build");

    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-1"), "iron_ingot", 2)
        .expect("seed severe local bottleneck");
    world
        .set_material_balance("iron_ingot", 20)
        .expect("seed world bottleneck");
    world.set_resource_balance(ResourceKind::Electricity, 20);

    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.scarcity.severe".to_string(),
        recipe_id: "recipe.scarcity.severe".to_string(),
        plan: RecipeExecutionPlan::accepted(
            1,
            vec![MaterialStack::new("iron_ingot", 10)],
            vec![MaterialStack::new("motor_mk1", 1)],
            Vec::new(),
            1,
            3,
        ),
    });
    world.step().expect("start delayed recipe");

    let now = world.snapshot().state.time;
    let started = world.journal().events.last().expect("recipe started");
    match &started.body {
        WorldEventBody::Domain(DomainEvent::RecipeStarted {
            duration_ticks,
            ready_at,
            ..
        }) => {
            assert_eq!(*duration_ticks, 5);
            assert_eq!(*ready_at, now.saturating_add(5));
        }
        other => panic!("expected RecipeStarted, got {other:?}"),
    }

    for _ in 0..4 {
        world.step().expect("wait severe delayed completion");
    }
    assert_eq!(world.pending_recipe_jobs_len(), 1);
    world.step().expect("complete severe delayed recipe");
    assert_eq!(world.pending_recipe_jobs_len(), 0);
}

#[test]
fn recycle_factory_rejects_when_recipe_job_is_active() {
    let mut world = World::new();
    register_builder(&mut world, "builder-a");
    build_factory_ready(
        &mut world,
        "builder-a",
        "site-1",
        factory_spec("factory.alpha", 1, 1, 1),
    );

    world
        .set_material_balance("iron_ingot", 1)
        .expect("seed recipe input");
    world.set_resource_balance(ResourceKind::Electricity, 10);
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.alpha".to_string(),
        recipe_id: "chip".to_string(),
        plan: RecipeExecutionPlan::accepted(
            1,
            vec![MaterialStack::new("iron_ingot", 1)],
            vec![MaterialStack::new("control_chip", 1)],
            Vec::new(),
            1,
            3,
        ),
    });
    world.step().expect("schedule recipe");
    assert_eq!(world.pending_recipe_jobs_len(), 1);

    world.submit_action(Action::RecycleFactory {
        operator_agent_id: "builder-a".to_string(),
        factory_id: "factory.alpha".to_string(),
    });
    world.step().expect("recycle attempt");

    assert!(world.has_factory("factory.alpha"));
    let last = world.journal().events.last().expect("recycle reject");
    match &last.body {
        WorldEventBody::Domain(DomainEvent::ActionRejected {
            reason: RejectReason::FactoryBusy { factory_id, .. },
            ..
        }) => assert_eq!(factory_id, "factory.alpha"),
        other => panic!("expected FactoryBusy rejection, got {other:?}"),
    }
}
