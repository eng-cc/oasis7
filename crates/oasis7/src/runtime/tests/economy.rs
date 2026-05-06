use super::pos;
use crate::runtime::{
    util, Action, CapabilityGrant, DomainEvent, MaterialLedgerId, ModuleAbiContract,
    ModuleActivation, ModuleChangeSet, ModuleKind, ModuleLimits, ModuleManifest, ModuleRole,
    PolicySet, ProposalDecision, RejectReason, World, WorldEventBody,
};
use crate::simulator::ResourceKind;
use oasis7_wasm_abi::{
    FactoryBuildDecision, FactoryModuleSpec, MaterialStack, ModuleEmit, ModuleOutput,
    ProductValidationDecision, RecipeExecutionPlan,
};
use oasis7_wasm_executor::FixedSandbox;
use serde_json::json;

#[path = "economy_module_validation_tests.rs"]
mod module_validation_tests;

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

fn activate_pure_module(world: &mut World, module_id: &str, wasm_seed: &[u8]) {
    world.set_policy(PolicySet::allow_all());
    world.add_capability(CapabilityGrant::allow_all("cap.economy"));

    let wasm_hash = util::sha256_hex(wasm_seed);
    world
        .register_module_artifact(wasm_hash.clone(), wasm_seed)
        .expect("register module artifact");

    let manifest = ModuleManifest {
        module_id: module_id.to_string(),
        name: format!("module-{module_id}"),
        version: "0.1.0".to_string(),
        kind: ModuleKind::Pure,
        role: ModuleRole::Domain,
        wasm_hash: wasm_hash.clone(),
        interface_version: "wasm-1".to_string(),
        abi_contract: ModuleAbiContract::default(),
        exports: vec!["call".to_string()],
        subscriptions: Vec::new(),
        required_caps: Vec::new(),
        artifact_identity: Some(super::signed_test_artifact_identity(wasm_hash.as_str())),
        limits: ModuleLimits {
            max_mem_bytes: 1024 * 1024,
            max_gas: 1_000_000,
            max_call_rate: 1024,
            max_output_bytes: 1024 * 1024,
            max_effects: 0,
            max_emits: 8,
        },
    };

    let changes = ModuleChangeSet {
        register: vec![manifest.clone()],
        activate: vec![ModuleActivation {
            module_id: manifest.module_id.clone(),
            version: manifest.version.clone(),
        }],
        ..ModuleChangeSet::default()
    };

    let mut content = serde_json::Map::new();
    content.insert(
        "module_changes".to_string(),
        serde_json::to_value(changes).expect("serialize module changes"),
    );
    let proposal_id = world
        .propose_manifest_update(
            crate::runtime::Manifest {
                version: 2,
                content: serde_json::Value::Object(content),
            },
            "tester",
        )
        .expect("propose module activation");
    world
        .shadow_proposal(proposal_id)
        .expect("shadow module proposal");
    world
        .approve_proposal(proposal_id, "tester", ProposalDecision::Approve)
        .expect("approve module proposal");
    world
        .apply_proposal(proposal_id)
        .expect("apply module proposal");
}

#[test]
fn build_factory_consumes_materials_and_completes_after_delay() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");

    world
        .set_material_balance("steel_plate", 20)
        .expect("seed steel");
    world
        .set_material_balance("circuit_board", 4)
        .expect("seed circuits");

    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        spec: factory_spec("factory.alpha", 2, 1),
    });

    world.step().expect("start factory build");
    assert_eq!(world.pending_factory_builds_len(), 1);
    assert!(!world.has_factory("factory.alpha"));
    assert_eq!(world.material_balance("steel_plate"), 10);
    assert_eq!(world.material_balance("circuit_board"), 2);

    let started = world
        .journal()
        .events
        .last()
        .expect("factory build started event");
    match &started.body {
        WorldEventBody::Domain(DomainEvent::FactoryBuildStarted { spec, .. }) => {
            assert_eq!(spec.factory_id, "factory.alpha")
        }
        other => panic!("expected FactoryBuildStarted, got {other:?}"),
    }

    world.step().expect("tick without completion");
    assert_eq!(world.pending_factory_builds_len(), 1);
    assert!(!world.has_factory("factory.alpha"));

    world.step().expect("complete factory build");
    assert_eq!(world.pending_factory_builds_len(), 0);
    assert!(world.has_factory("factory.alpha"));

    let completed = world.journal().events.last().expect("factory built event");
    match &completed.body {
        WorldEventBody::Domain(DomainEvent::FactoryBuilt { spec, .. }) => {
            assert_eq!(spec.factory_id, "factory.alpha")
        }
        other => panic!("expected FactoryBuilt, got {other:?}"),
    }
}

#[test]
fn build_factory_prefers_builder_material_ledger_when_available() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");

    world
        .set_ledger_material_balance(MaterialLedgerId::agent("builder-a"), "steel_plate", 12)
        .expect("seed builder steel");
    world
        .set_ledger_material_balance(MaterialLedgerId::agent("builder-a"), "circuit_board", 3)
        .expect("seed builder circuits");
    world
        .set_material_balance("steel_plate", 100)
        .expect("seed world steel");
    world
        .set_material_balance("circuit_board", 100)
        .expect("seed world circuits");

    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        spec: factory_spec("factory.ledger", 1, 1),
    });
    world.step().expect("start build");

    assert_eq!(
        world.ledger_material_balance(&MaterialLedgerId::agent("builder-a"), "steel_plate"),
        2
    );
    assert_eq!(
        world.ledger_material_balance(&MaterialLedgerId::agent("builder-a"), "circuit_board"),
        1
    );
    assert_eq!(world.material_balance("steel_plate"), 100);
    assert_eq!(world.material_balance("circuit_board"), 100);
}

#[test]
fn schedule_recipe_consumes_inputs_and_power_then_produces_outputs() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");

    world
        .set_material_balance("steel_plate", 11)
        .expect("seed build steel");
    world
        .set_material_balance("circuit_board", 2)
        .expect("seed build circuits");

    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        spec: factory_spec("factory.recipe", 1, 1),
    });
    world.step().expect("start factory build");
    world.step().expect("factory ready");
    assert!(world.has_factory("factory.recipe"));

    world
        .set_material_balance("iron_ingot", 6)
        .expect("seed recipe input");
    world.set_resource_balance(ResourceKind::Electricity, 20);

    let plan = RecipeExecutionPlan::accepted(
        2,
        vec![MaterialStack::new("iron_ingot", 6)],
        vec![MaterialStack::new("motor_mk1", 2)],
        vec![MaterialStack::new("metal_scrap", 1)],
        7,
        1,
    );

    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.recipe".to_string(),
        recipe_id: "recipe.motor.mk1".to_string(),
        plan,
    });

    world.step().expect("start recipe");
    assert_eq!(world.pending_recipe_jobs_len(), 1);
    assert_eq!(world.material_balance("iron_ingot"), 0);
    assert_eq!(world.resource_balance(ResourceKind::Electricity), 13);

    let started = world.journal().events.last().expect("recipe started event");
    match &started.body {
        WorldEventBody::Domain(DomainEvent::RecipeStarted { recipe_id, .. }) => {
            assert_eq!(recipe_id, "recipe.motor.mk1")
        }
        other => panic!("expected RecipeStarted, got {other:?}"),
    }

    for _ in 0..4 {
        if world.pending_recipe_jobs_len() == 0 {
            break;
        }
        world.step().expect("advance recipe toward completion");
    }
    assert_eq!(world.pending_recipe_jobs_len(), 0);
    assert_eq!(world.material_balance("motor_mk1"), 2);
    assert_eq!(world.material_balance("metal_scrap"), 1);

    let completed = world
        .journal()
        .events
        .last()
        .expect("recipe completed event");
    match &completed.body {
        WorldEventBody::Domain(DomainEvent::RecipeCompleted { recipe_id, .. }) => {
            assert_eq!(recipe_id, "recipe.motor.mk1")
        }
        other => panic!("expected RecipeCompleted, got {other:?}"),
    }
}

#[test]
fn schedule_recipe_reads_and_writes_site_material_ledger() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");

    world
        .set_material_balance("steel_plate", 20)
        .expect("seed world steel");
    world
        .set_material_balance("circuit_board", 4)
        .expect("seed world circuits");

    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-ledger".to_string(),
        spec: factory_spec("factory.site.ledger", 1, 1),
    });
    world.step().expect("start factory build");
    world.step().expect("factory ready");
    assert!(world.has_factory("factory.site.ledger"));

    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-ledger"), "iron_ingot", 6)
        .expect("seed site iron");
    world.set_resource_balance(ResourceKind::Electricity, 20);

    let plan = RecipeExecutionPlan::accepted(
        2,
        vec![MaterialStack::new("iron_ingot", 6)],
        vec![MaterialStack::new("motor_mk1", 2)],
        vec![MaterialStack::new("metal_scrap", 1)],
        7,
        1,
    );
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.site.ledger".to_string(),
        recipe_id: "recipe.site.ledger".to_string(),
        plan,
    });

    world.step().expect("start recipe");
    assert_eq!(
        world.ledger_material_balance(&MaterialLedgerId::site("site-ledger"), "iron_ingot"),
        0
    );
    world.step().expect("complete recipe");
    assert_eq!(
        world.ledger_material_balance(&MaterialLedgerId::site("site-ledger"), "motor_mk1"),
        2
    );
    assert_eq!(
        world.ledger_material_balance(&MaterialLedgerId::site("site-ledger"), "metal_scrap"),
        1
    );
}

#[test]
fn transfer_material_distance_zero_moves_immediately() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "operator-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register operator");

    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-a"), "iron_ingot", 20)
        .expect("seed source");
    world.submit_action(Action::TransferMaterial {
        requester_agent_id: "operator-a".to_string(),
        from_ledger: MaterialLedgerId::site("site-a"),
        to_ledger: MaterialLedgerId::site("site-b"),
        kind: "iron_ingot".to_string(),
        amount: 8,
        distance_km: 0,
        priority: None,
    });
    world.step().expect("transfer material");

    assert_eq!(
        world.ledger_material_balance(&MaterialLedgerId::site("site-a"), "iron_ingot"),
        12
    );
    assert_eq!(
        world.ledger_material_balance(&MaterialLedgerId::site("site-b"), "iron_ingot"),
        8
    );
    assert_eq!(world.pending_material_transits_len(), 0);
    assert!(matches!(
        world
            .journal()
            .events
            .last()
            .expect("material transfer event")
            .body,
        WorldEventBody::Domain(DomainEvent::MaterialTransferred { .. })
    ));
}

#[test]
fn transfer_material_cross_site_creates_transit_and_applies_loss() {
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
        amount: 100,
        distance_km: 200,
        priority: None,
    });
    world.step().expect("start transit");

    assert_eq!(
        world.ledger_material_balance(&MaterialLedgerId::site("site-a"), "copper_wire"),
        0
    );
    assert_eq!(world.pending_material_transits_len(), 1);
    assert!(matches!(
        world
            .journal()
            .events
            .last()
            .expect("transit started event")
            .body,
        WorldEventBody::Domain(DomainEvent::MaterialTransitStarted { .. })
    ));

    world.step().expect("tick before completion");
    assert_eq!(world.pending_material_transits_len(), 1);
    world.step().expect("transit completion");
    assert_eq!(world.pending_material_transits_len(), 0);
    assert_eq!(
        world.ledger_material_balance(&MaterialLedgerId::site("site-b"), "copper_wire"),
        90
    );
    assert!(matches!(
        world
            .journal()
            .events
            .last()
            .expect("transit completed event")
            .body,
        WorldEventBody::Domain(DomainEvent::MaterialTransitCompleted {
            received_amount: 90,
            loss_amount: 10,
            ..
        })
    ));
}

#[test]
fn transfer_material_rejects_when_distance_exceeds_limit() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "operator-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register operator");

    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-a"), "iron_ingot", 20)
        .expect("seed source");
    world.submit_action(Action::TransferMaterial {
        requester_agent_id: "operator-a".to_string(),
        from_ledger: MaterialLedgerId::site("site-a"),
        to_ledger: MaterialLedgerId::site("site-b"),
        kind: "iron_ingot".to_string(),
        amount: 5,
        distance_km: 20_001,
        priority: None,
    });
    world.step().expect("reject out of range");

    match &world.journal().events.last().expect("reject event").body {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => {
            assert!(matches!(
                reason,
                RejectReason::MaterialTransferDistanceExceeded {
                    distance_km: 20_001,
                    max_distance_km: 10_000
                }
            ));
        }
        other => panic!("expected ActionRejected, got {other:?}"),
    }
}

#[test]
fn transfer_material_rejects_when_inflight_capacity_exceeded() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "operator-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register operator");

    world
        .set_ledger_material_balance(MaterialLedgerId::site("site-a"), "iron_ingot", 30)
        .expect("seed source");
    world.submit_action(Action::TransferMaterial {
        requester_agent_id: "operator-a".to_string(),
        from_ledger: MaterialLedgerId::site("site-a"),
        to_ledger: MaterialLedgerId::site("site-b"),
        kind: "iron_ingot".to_string(),
        amount: 10,
        distance_km: 100,
        priority: None,
    });
    world.submit_action(Action::TransferMaterial {
        requester_agent_id: "operator-a".to_string(),
        from_ledger: MaterialLedgerId::site("site-a"),
        to_ledger: MaterialLedgerId::site("site-c"),
        kind: "iron_ingot".to_string(),
        amount: 10,
        distance_km: 100,
        priority: None,
    });
    world.submit_action(Action::TransferMaterial {
        requester_agent_id: "operator-a".to_string(),
        from_ledger: MaterialLedgerId::site("site-a"),
        to_ledger: MaterialLedgerId::site("site-d"),
        kind: "iron_ingot".to_string(),
        amount: 10,
        distance_km: 100,
        priority: None,
    });

    world.step().expect("process transfer actions");
    assert_eq!(world.pending_material_transits_len(), 2);
    match &world
        .journal()
        .events
        .last()
        .expect("third transfer reject")
        .body
    {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => {
            assert!(matches!(
                reason,
                RejectReason::MaterialTransitCapacityExceeded {
                    in_flight: 2,
                    max_in_flight: 2
                }
            ));
        }
        other => panic!("expected ActionRejected, got {other:?}"),
    }
}

#[test]
fn schedule_recipe_rejects_when_factory_slots_are_full() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");

    world
        .set_material_balance("steel_plate", 10)
        .expect("seed build steel");
    world
        .set_material_balance("circuit_board", 2)
        .expect("seed build circuits");

    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        spec: factory_spec("factory.slot", 1, 1),
    });
    world.step().expect("start factory build");
    world.step().expect("factory ready");

    world
        .set_material_balance("gear", 8)
        .expect("seed recipe input");
    world.set_resource_balance(ResourceKind::Electricity, 50);

    let plan_a = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("gear", 2)],
        vec![MaterialStack::new("module_a", 1)],
        Vec::new(),
        2,
        3,
    );
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.slot".to_string(),
        recipe_id: "recipe.a".to_string(),
        plan: plan_a,
    });
    world.step().expect("start recipe a");
    assert_eq!(world.pending_recipe_jobs_len(), 1);

    let plan_b = RecipeExecutionPlan::accepted(
        1,
        vec![MaterialStack::new("gear", 2)],
        vec![MaterialStack::new("module_b", 1)],
        Vec::new(),
        2,
        1,
    );
    world.submit_action(Action::ScheduleRecipe {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.slot".to_string(),
        recipe_id: "recipe.b".to_string(),
        plan: plan_b,
    });
    world.step().expect("reject recipe b");

    let rejected = world.journal().events.last().expect("rejection event");
    match &rejected.body {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => match reason {
            RejectReason::FactoryBusy {
                factory_id,
                active_jobs,
                recipe_slots,
            } => {
                assert_eq!(factory_id, "factory.slot");
                assert_eq!(*active_jobs, 1);
                assert_eq!(*recipe_slots, 1);
            }
            other => panic!("expected FactoryBusy reject reason, got {other:?}"),
        },
        other => panic!("expected ActionRejected, got {other:?}"),
    }
}

#[test]
fn build_factory_rejects_when_materials_insufficient() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");

    world
        .set_material_balance("steel_plate", 3)
        .expect("seed limited steel");
    world
        .set_material_balance("circuit_board", 2)
        .expect("seed circuits");

    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        spec: factory_spec("factory.fail", 1, 1),
    });
    world.step().expect("build rejected");

    let rejected = world.journal().events.last().expect("rejection event");
    match &rejected.body {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => match reason {
            RejectReason::InsufficientMaterial {
                material_kind,
                requested,
                available,
            } => {
                assert_eq!(material_kind, "steel_plate");
                assert_eq!(*requested, 10);
                assert_eq!(*available, 3);
            }
            other => panic!("expected InsufficientMaterial reject reason, got {other:?}"),
        },
        other => panic!("expected ActionRejected, got {other:?}"),
    }
}

#[test]
fn build_factory_with_module_uses_module_decision() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");

    world
        .set_material_balance("steel_plate", 9)
        .expect("seed steel");
    world
        .set_material_balance("circuit_board", 2)
        .expect("seed circuits");

    activate_pure_module(&mut world, "m4.factory.basic", b"factory-module");

    world.submit_action(Action::BuildFactoryWithModule {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        module_id: "m4.factory.basic".to_string(),
        spec: factory_spec("factory.module", 5, 1),
    });

    let output = ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: vec![ModuleEmit {
            kind: "economy.factory_build_decision".to_string(),
            payload: serde_json::to_value(FactoryBuildDecision::accepted(
                vec![
                    MaterialStack::new("steel_plate", 8),
                    MaterialStack::new("circuit_board", 2),
                ],
                1,
            ))
            .expect("serialize factory build decision"),
        }],
        tick_lifecycle: None,
        output_bytes: 256,
    };
    let mut sandbox = FixedSandbox::succeed(output);
    world
        .step_with_modules(&mut sandbox)
        .expect("start factory build with module");

    assert_eq!(world.material_balance("steel_plate"), 1);
    assert_eq!(world.material_balance("circuit_board"), 0);
    assert_eq!(world.pending_factory_builds_len(), 1);

    let started = world
        .journal()
        .events
        .last()
        .expect("factory build started event");
    match &started.body {
        WorldEventBody::Domain(DomainEvent::FactoryBuildStarted { spec, .. }) => {
            assert_eq!(spec.build_time_ticks, 1);
            assert_eq!(spec.build_cost[0].amount, 8);
        }
        other => panic!("expected FactoryBuildStarted, got {other:?}"),
    }

    world
        .step_with_modules(&mut sandbox)
        .expect("complete factory build");
    assert!(world.has_factory("factory.module"));
}

#[test]
fn schedule_recipe_with_module_uses_module_plan() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");

    world
        .set_material_balance("steel_plate", 10)
        .expect("seed steel");
    world
        .set_material_balance("circuit_board", 2)
        .expect("seed circuits");
    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        spec: factory_spec("factory.recipe.module", 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("build complete");

    world
        .set_material_balance("iron_ingot", 7)
        .expect("seed ingot");
    world.set_resource_balance(ResourceKind::Electricity, 30);
    activate_pure_module(&mut world, "m4.recipe.motor", b"recipe-module");

    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.recipe.module".to_string(),
        recipe_id: "recipe.motor.mk1".to_string(),
        module_id: "m4.recipe.motor".to_string(),
        desired_batches: 2,
        deterministic_seed: 42,
    });

    let output = ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: vec![ModuleEmit {
            kind: "economy.recipe_execution_plan".to_string(),
            payload: serde_json::to_value(RecipeExecutionPlan::accepted(
                2,
                vec![MaterialStack::new("iron_ingot", 6)],
                vec![MaterialStack::new("motor_mk1", 2)],
                vec![MaterialStack::new("metal_scrap", 1)],
                9,
                1,
            ))
            .expect("serialize recipe execution plan"),
        }],
        tick_lifecycle: None,
        output_bytes: 256,
    };
    let mut sandbox = FixedSandbox::succeed(output);
    world
        .step_with_modules(&mut sandbox)
        .expect("start recipe with module");

    assert_eq!(world.material_balance("iron_ingot"), 1);
    assert_eq!(world.resource_balance(ResourceKind::Electricity), 21);
    assert_eq!(world.pending_recipe_jobs_len(), 1);

    for _ in 0..4 {
        if world.pending_recipe_jobs_len() == 0 {
            break;
        }
        world
            .step_with_modules(&mut sandbox)
            .expect("advance module recipe toward completion");
    }
    assert_eq!(world.pending_recipe_jobs_len(), 0);
    assert_eq!(world.material_balance("motor_mk1"), 2);
    assert_eq!(world.material_balance("metal_scrap"), 1);
}

#[test]
fn schedule_recipe_with_module_rejects_when_module_denies() {
    let mut world = World::new();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0, 0),
    });
    world.step().expect("register agent");

    world
        .set_material_balance("steel_plate", 10)
        .expect("seed steel");
    world
        .set_material_balance("circuit_board", 2)
        .expect("seed circuits");
    world.submit_action(Action::BuildFactory {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-1".to_string(),
        spec: factory_spec("factory.recipe.reject", 1, 1),
    });
    world.step().expect("start build");
    world.step().expect("build complete");
    activate_pure_module(&mut world, "m4.recipe.reject", b"recipe-reject-module");

    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.recipe.reject".to_string(),
        recipe_id: "recipe.fail".to_string(),
        module_id: "m4.recipe.reject".to_string(),
        desired_batches: 1,
        deterministic_seed: 7,
    });

    let output = ModuleOutput {
        new_state: None,
        effects: Vec::new(),
        emits: vec![ModuleEmit {
            kind: "economy.recipe_execution_plan".to_string(),
            payload: json!({
                "accepted_batches": 0,
                "consume": [],
                "produce": [],
                "byproducts": [],
                "power_required": 0,
                "duration_ticks": 0,
                "reject_reason": "insufficient pressure"
            }),
        }],
        tick_lifecycle: None,
        output_bytes: 256,
    };
    let mut sandbox = FixedSandbox::succeed(output);
    world
        .step_with_modules(&mut sandbox)
        .expect("module denial should turn into action rejected");

    let rejected = world.journal().events.last().expect("rejection event");
    match &rejected.body {
        WorldEventBody::Domain(DomainEvent::ActionRejected { reason, .. }) => match reason {
            RejectReason::RuleDenied { notes } => {
                assert!(notes
                    .iter()
                    .any(|note| note.contains("recipe module denied: insufficient pressure")));
            }
            other => panic!("expected RuleDenied, got {other:?}"),
        },
        other => panic!("expected ActionRejected, got {other:?}"),
    }
}
