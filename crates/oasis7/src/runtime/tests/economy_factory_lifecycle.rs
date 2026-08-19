use super::pos;
use crate::runtime::{
    Action, DomainEvent, MaterialLedgerId, RejectReason, World, WorldError, WorldEventBody,
};
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
    world
        .set_agent_resource_balance("builder-a", ResourceKind::Electricity, 20)
        .expect("seed builder electricity");
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
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
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
fn factory_depreciation_counts_only_jobs_for_each_factory() {
    let mut world = World::new();
    register_builder(&mut world, "builder-a");
    build_factory_ready(
        &mut world,
        "builder-a",
        "site-1",
        factory_spec("factory.target", 1, 2, 3),
    );
    build_factory_ready(
        &mut world,
        "builder-a",
        "site-2",
        factory_spec("factory.other", 1, 2, 3),
    );

    world
        .set_material_balance("iron_ingot", 4)
        .expect("seed recipe inputs");
    world
        .set_agent_resource_balance("builder-a", ResourceKind::Electricity, 40)
        .expect("seed builder electricity");
    world.set_resource_balance(ResourceKind::Electricity, 40);
    for factory_id in ["factory.target", "factory.other"] {
        world.submit_action(Action::ScheduleRecipe {
            requester_agent_id: "builder-a".to_string(),
            factory_id: factory_id.to_string(),
            recipe_id: format!("recipe.{factory_id}"),
            plan: RecipeExecutionPlan::accepted(
                1,
                vec![MaterialStack::new("iron_ingot", 1)],
                vec![MaterialStack::new("control_chip", 1)],
                Vec::new(),
                1,
                3,
            ),
            logistics_route_ids: Vec::new(),
            logistics_path_ids: Vec::new(),
        });
    }
    world.step().expect("start recipes");
    assert_eq!(world.pending_recipe_jobs_len(), 2);

    let durability_before_loaded_tick = world
        .snapshot()
        .state
        .factories
        .get("factory.target")
        .expect("target factory exists")
        .durability_ppm;

    world
        .step()
        .expect("depreciation under independent factory loads");

    let durability_after_loaded_tick = world
        .snapshot()
        .state
        .factories
        .get("factory.target")
        .expect("target factory exists")
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
fn industrial_integrity_factory_maintained_wrong_operator_rejects_before_debit() {
    let mut world = World::new();
    register_builder(&mut world, "builder-a");
    build_factory_ready(
        &mut world,
        "builder-a",
        "site-maintain-owner",
        factory_spec("factory.maintain-owner", 1, 1, 1),
    );
    world
        .set_material_balance("hardware_part", 2)
        .expect("seed maintenance parts");

    let mut replay = world.state().clone();
    let before = serde_json::to_vec(&replay).expect("serialize state before wrong operator");
    let event = DomainEvent::FactoryMaintained {
        operator_agent_id: "operator-not-builder".to_string(),
        factory_id: "factory.maintain-owner".to_string(),
        consume_ledger: MaterialLedgerId::world(),
        consumed_parts: 1,
        durability_ppm: 1_000_000,
    };

    let result = replay.apply_domain_event(&event, replay.time);
    assert!(
        matches!(result, Err(WorldError::ResourceBalanceInvalid { .. })),
        "maintenance by a non-builder must fail closed: {result:?}"
    );
    assert_eq!(
        serde_json::to_vec(&replay).expect("serialize state after wrong operator"),
        before,
        "wrong-operator maintenance must not debit hardware or mutate factory state"
    );
}

#[test]
fn industrial_integrity_factory_maintained_unknown_factory_rejects_before_debit() {
    let mut world = World::new();
    register_builder(&mut world, "builder-a");
    world
        .set_material_balance("hardware_part", 2)
        .expect("seed maintenance parts");

    let mut replay = world.state().clone();
    let before = serde_json::to_vec(&replay).expect("serialize state before unknown factory");
    let event = DomainEvent::FactoryMaintained {
        operator_agent_id: "builder-a".to_string(),
        factory_id: "factory.unknown-maintenance".to_string(),
        consume_ledger: MaterialLedgerId::world(),
        consumed_parts: 1,
        durability_ppm: 1_000_000,
    };

    let result = replay.apply_domain_event(&event, replay.time);
    assert!(
        matches!(result, Err(WorldError::ResourceBalanceInvalid { .. })),
        "maintenance for an unknown factory must fail closed: {result:?}"
    );
    assert_eq!(
        serde_json::to_vec(&replay).expect("serialize state after unknown factory"),
        before,
        "unknown-factory maintenance must not debit hardware or mutate state"
    );
}

#[test]
fn schedule_recipe_uses_factory_builder_electricity_and_debits_owner() {
    let mut world = World::new();
    register_builder(&mut world, "builder-a");
    build_factory_ready(
        &mut world,
        "builder-a",
        "site-owner-power",
        factory_spec("factory.owner-power", 1, 1, 1),
    );
    world
        .set_material_balance("iron_ingot", 1)
        .expect("seed recipe input");
    world
        .set_agent_resource_balance("builder-a", ResourceKind::Electricity, 5)
        .expect("seed builder electricity");
    world.set_resource_balance(ResourceKind::Electricity, 0);

    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.owner-power".to_string(),
        recipe_id: "recipe.owner-power".to_string(),
        plan: RecipeExecutionPlan::accepted(
            1,
            vec![MaterialStack::new("iron_ingot", 1)],
            vec![MaterialStack::new("control_chip", 1)],
            Vec::new(),
            5,
            2,
        ),
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world
        .step()
        .expect("schedule recipe with owner electricity");

    assert_eq!(world.pending_recipe_jobs_len(), 1);
    assert_eq!(
        world
            .agent_resource_balance("builder-a", ResourceKind::Electricity)
            .expect("builder electricity"),
        0,
        "recipe power must debit the factory builder ledger"
    );
    assert_eq!(
        world.resource_balance(ResourceKind::Electricity),
        0,
        "world electricity must not be used as an implicit payer"
    );
}

#[test]
fn schedule_recipe_rejects_when_builder_electricity_is_insufficient_even_if_world_has_power() {
    let mut world = World::new();
    register_builder(&mut world, "builder-a");
    build_factory_ready(
        &mut world,
        "builder-a",
        "site-owner-power-reject",
        factory_spec("factory.owner-power-reject", 1, 1, 1),
    );
    world
        .set_material_balance("iron_ingot", 1)
        .expect("seed recipe input");
    world
        .set_agent_resource_balance("builder-a", ResourceKind::Electricity, 0)
        .expect("clear builder electricity");
    world.set_resource_balance(ResourceKind::Electricity, 5);

    let material_before = world.material_balance("iron_ingot");
    let journal_start = world.journal().events.len();
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.owner-power-reject".to_string(),
        recipe_id: "recipe.owner-power-reject".to_string(),
        plan: RecipeExecutionPlan::accepted(
            1,
            vec![MaterialStack::new("iron_ingot", 1)],
            vec![MaterialStack::new("control_chip", 1)],
            Vec::new(),
            5,
            2,
        ),
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world
        .step()
        .expect("insufficient owner electricity should be an action rejection");

    assert_eq!(world.pending_recipe_jobs_len(), 0);
    assert_eq!(world.material_balance("iron_ingot"), material_before);
    assert_eq!(
        world
            .agent_resource_balance("builder-a", ResourceKind::Electricity)
            .expect("builder electricity"),
        0
    );
    assert_eq!(world.resource_balance(ResourceKind::Electricity), 5);
    let rejection = world.journal().events[journal_start..]
        .iter()
        .find_map(|event| match &event.body {
            WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => Some(reason),
            _ => None,
        })
        .expect("rejection event");
    match rejection {
        RejectReason::InsufficientResource {
            agent_id,
            kind: ResourceKind::Electricity,
            requested,
            available,
        } => {
            assert_eq!(agent_id, "builder-a");
            assert_eq!(requested, &5);
            assert_eq!(available, &0);
        }
        other => panic!("expected owner-power rejection, got {other:?}"),
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
fn industrial_integrity_factory_recycled_wrong_operator_is_byte_stable_rejection() {
    let mut world = World::new();
    register_builder(&mut world, "builder-a");
    build_factory_ready(
        &mut world,
        "builder-a",
        "site-recycle-owner",
        factory_spec("factory.recycle-owner", 1, 1, 1),
    );

    let mut replay = world.state().clone();
    let before = serde_json::to_vec(&replay).expect("serialize state before wrong recycle");
    let event = DomainEvent::FactoryRecycled {
        operator_agent_id: "operator-not-builder".to_string(),
        factory_id: "factory.recycle-owner".to_string(),
        recycle_ledger: MaterialLedgerId::site("site-recycle-owner"),
        recovered: vec![MaterialStack::new("steel_plate", 1)],
        durability_ppm: 1_000_000,
    };

    let result = replay.apply_domain_event(&event, replay.time);
    assert!(
        matches!(result, Err(WorldError::ResourceBalanceInvalid { .. })),
        "recycle by a non-builder must fail closed: {result:?}"
    );
    assert_eq!(
        serde_json::to_vec(&replay).expect("serialize state after wrong recycle"),
        before,
        "wrong-operator recycle must not remove the factory or recover materials"
    );
}

#[test]
fn industrial_integrity_duplicate_factory_recycle_is_noop_without_recovery_duplication() {
    let mut world = World::new();
    register_builder(&mut world, "builder-a");
    build_factory_ready(
        &mut world,
        "builder-a",
        "site-1",
        factory_spec("factory.recycle-replay", 1, 1, 1),
    );
    world.submit_action(Action::RecycleFactory {
        operator_agent_id: "builder-a".to_string(),
        factory_id: "factory.recycle-replay".to_string(),
    });
    world.step().expect("recycle factory for replay test");

    let recycled = world
        .journal()
        .events
        .iter()
        .rev()
        .find_map(|event| match &event.body {
            WorldEventBody::Domain(event @ DomainEvent::FactoryRecycled { .. }) => {
                Some(event.clone())
            }
            _ => None,
        })
        .expect("factory recycle event");
    let mut replay = world.state().clone();
    let before = serde_json::to_vec(&replay).expect("serialize state before recycle replay");

    replay
        .apply_domain_event(&recycled, replay.time)
        .expect("duplicate factory recycle should be idempotent");
    assert!(
        serde_json::to_vec(&replay).expect("serialize state after recycle replay") == before,
        "duplicate factory recycle must not duplicate recovered materials or progress"
    );
}

#[test]
fn industrial_integrity_retired_factory_id_rejects_same_id_rebuild_before_sink() {
    let mut world = World::new();
    register_builder(&mut world, "builder-a");
    let spec = factory_spec("factory.retired", 1, 1, 1);
    build_factory_ready(&mut world, "builder-a", "site-1", spec.clone());

    world.submit_action(Action::RecycleFactory {
        operator_agent_id: "builder-a".to_string(),
        factory_id: spec.factory_id.clone(),
    });
    world.step().expect("recycle factory");
    assert!(!world.has_factory(spec.factory_id.as_str()));

    let steel_before = world.material_balance("steel_plate");
    let circuits_before = world.material_balance("circuit_board");
    let journal_start = world.journal().events.len();
    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        spec,
    });
    world
        .step()
        .expect("same-id rebuild should produce an explicit rejection");

    assert_eq!(world.material_balance("steel_plate"), steel_before);
    assert_eq!(world.material_balance("circuit_board"), circuits_before);
    assert_eq!(world.pending_factory_builds_len(), 0);
    assert!(world.journal().events[journal_start..].iter().any(|event| {
        matches!(
            &event.body,
            WorldEventBody::Domain(DomainEvent::ActionRejected {
                reason: RejectReason::RuleDenied { notes },
                ..
            }) if notes.iter().any(|note| note.contains("retired") || note.contains("factory"))
        )
    }));
    assert!(
        !world.journal().events[journal_start..].iter().any(|event| {
            matches!(
                &event.body,
                WorldEventBody::Domain(DomainEvent::FactoryBuildStarted { .. })
            )
        })
    );
}

#[test]
fn industrial_integrity_retired_factory_id_rejects_stale_factory_built_replay() {
    let mut world = World::new();
    register_builder(&mut world, "builder-a");
    let spec = factory_spec("factory.retired.replay", 1, 1, 1);
    build_factory_ready(&mut world, "builder-a", "site-1", spec);
    let stale_built = world
        .journal()
        .events
        .iter()
        .find_map(|event| match &event.body {
            WorldEventBody::Domain(event @ DomainEvent::FactoryBuilt { .. }) => Some(event.clone()),
            _ => None,
        })
        .expect("original factory-built event");

    world.submit_action(Action::RecycleFactory {
        operator_agent_id: "builder-a".to_string(),
        factory_id: "factory.retired.replay".to_string(),
    });
    world.step().expect("recycle factory");
    assert!(!world.has_factory("factory.retired.replay"));

    let mut replay = world.state().clone();
    let before = serde_json::to_vec(&replay).expect("serialize state before stale build replay");
    let result = replay.apply_domain_event(&stale_built, replay.time);
    assert!(
        matches!(result, Err(WorldError::ResourceBalanceInvalid { .. })),
        "stale FactoryBuilt must fail closed: {result:?}"
    );
    assert!(
        serde_json::to_vec(&replay).expect("serialize state after stale build replay") == before,
        "stale FactoryBuilt must not mutate retired state"
    );
    assert!(!replay.factories.contains_key("factory.retired.replay"));
}

#[test]
fn industrial_integrity_unknown_factory_built_fails_without_state_mutation() {
    let mut world = World::new();
    register_builder(&mut world, "builder-a");

    let unknown_built = DomainEvent::FactoryBuilt {
        job_id: 9_999,
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-unknown".to_string(),
        spec: factory_spec("factory.unknown-built", 1, 1, 1),
    };
    let mut replay = world.state().clone();
    let before = serde_json::to_vec(&replay).expect("serialize state before unknown build");

    let result = replay.apply_domain_event(&unknown_built, replay.time);
    assert!(
        matches!(result, Err(WorldError::ResourceBalanceInvalid { .. })),
        "unknown FactoryBuilt must fail closed: {result:?}"
    );
    assert_eq!(
        serde_json::to_vec(&replay).expect("serialize state after unknown build"),
        before,
        "unknown FactoryBuilt must not mutate state"
    );
}

#[test]
fn industrial_integrity_retired_factory_id_keeps_old_recycle_replay_byte_stable() {
    let mut world = World::new();
    register_builder(&mut world, "builder-a");
    let spec = factory_spec("factory.retired.recycle-replay", 1, 1, 1);
    build_factory_ready(&mut world, "builder-a", "site-1", spec);
    let replacement = world
        .state()
        .factories
        .get("factory.retired.recycle-replay")
        .cloned()
        .expect("factory before recycle");

    world.submit_action(Action::RecycleFactory {
        operator_agent_id: "builder-a".to_string(),
        factory_id: "factory.retired.recycle-replay".to_string(),
    });
    world.step().expect("recycle factory");
    let recycled = world
        .journal()
        .events
        .iter()
        .rev()
        .find_map(|event| match &event.body {
            WorldEventBody::Domain(event @ DomainEvent::FactoryRecycled { .. }) => {
                Some(event.clone())
            }
            _ => None,
        })
        .expect("factory recycle event");

    // Simulate a stale same-ID replacement in the replay target.  A retired
    // identity must prevent the old recycle disposition from deleting it.
    let mut replay = world.state().clone();
    replay
        .factories
        .insert(replacement.factory_id.clone(), replacement);
    let before = serde_json::to_vec(&replay).expect("serialize state before old recycle replay");
    replay
        .apply_domain_event(&recycled, replay.time)
        .expect("old recycle replay should be idempotent");
    assert!(
        serde_json::to_vec(&replay).expect("serialize state after old recycle replay") == before,
        "old FactoryRecycled replay must not mutate a retired identity"
    );
    assert!(
        replay
            .factories
            .contains_key("factory.retired.recycle-replay")
    );
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
    world
        .set_agent_resource_balance("builder-a", ResourceKind::Electricity, 20)
        .expect("seed builder electricity");
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
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
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
    world
        .set_agent_resource_balance("builder-a", ResourceKind::Electricity, 20)
        .expect("seed builder electricity");
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
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
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
    world
        .set_agent_resource_balance("builder-a", ResourceKind::Electricity, 10)
        .expect("seed builder electricity");
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
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
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

#[test]
fn industrial_integrity_direct_factory_recycle_with_active_recipe_is_byte_stable_rejection() {
    let mut world = World::new();
    register_builder(&mut world, "builder-a");
    build_factory_ready(
        &mut world,
        "builder-a",
        "site-1",
        factory_spec("factory.active-wip", 1, 1, 1),
    );

    world
        .set_material_balance("iron_ingot", 1)
        .expect("seed recipe input");
    world
        .set_agent_resource_balance("builder-a", ResourceKind::Electricity, 10)
        .expect("seed builder electricity");
    world.set_resource_balance(ResourceKind::Electricity, 10);
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.active-wip".to_string(),
        recipe_id: "recipe.active-wip".to_string(),
        plan: RecipeExecutionPlan::accepted(
            1,
            vec![MaterialStack::new("iron_ingot", 1)],
            vec![MaterialStack::new("control_chip", 1)],
            Vec::new(),
            1,
            3,
        ),
        logistics_route_ids: Vec::new(),
        logistics_path_ids: Vec::new(),
    });
    world.step().expect("start recipe with active WIP");
    assert_eq!(world.pending_recipe_jobs_len(), 1);
    assert_eq!(world.material_balance("iron_ingot"), 0);
    assert_eq!(
        world
            .agent_resource_balance("builder-a", ResourceKind::Electricity)
            .expect("builder exists"),
        9
    );

    let mut replay = world.state().clone();
    let pending_before = replay.pending_recipe_jobs.clone();
    let material_ledgers_before = replay.material_ledgers.clone();
    let agent_before = replay
        .agents
        .get("builder-a")
        .cloned()
        .expect("builder state");
    let factory_before = replay
        .factories
        .get("factory.active-wip")
        .cloned()
        .expect("active factory state");
    let before = serde_json::to_vec(&replay).expect("serialize state before active-WIP recycle");
    let event = DomainEvent::FactoryRecycled {
        operator_agent_id: "builder-a".to_string(),
        factory_id: "factory.active-wip".to_string(),
        recycle_ledger: MaterialLedgerId::site("site-1"),
        recovered: vec![MaterialStack::new("steel_plate", 1)],
        durability_ppm: 1_000_000,
    };

    let result = replay.apply_domain_event(&event, replay.time);
    assert!(
        matches!(result, Err(WorldError::ResourceBalanceInvalid { .. })),
        "direct recycle reducer event must reject a factory with active WIP: {result:?}"
    );
    assert_eq!(
        serde_json::to_vec(&replay).expect("serialize state after active-WIP recycle"),
        before,
        "rejected active-WIP recycle must be byte-stable"
    );
    assert_eq!(
        replay.pending_recipe_jobs, pending_before,
        "rejected active-WIP recycle must preserve the pending recipe job"
    );
    assert_eq!(
        replay.material_ledgers, material_ledgers_before,
        "rejected active-WIP recycle must preserve consumed material balances"
    );
    assert_eq!(
        replay.agents.get("builder-a"),
        Some(&agent_before),
        "rejected active-WIP recycle must preserve consumed electricity and activity state"
    );
    assert_eq!(
        replay.factories.get("factory.active-wip"),
        Some(&factory_before),
        "rejected active-WIP recycle must preserve the factory state"
    );
}
