#![cfg(all(feature = "wasmtime", feature = "test_tier_full"))]

use super::super::*;
use super::pos;
use crate::simulator::ResourceKind;
use oasis7_wasm_abi::{FactoryModuleSpec, MaterialStack};
use oasis7_wasm_executor::{WasmExecutor, WasmExecutorConfig};

fn has_active(world: &World, module_id: &str) -> bool {
    world.module_registry().active.contains_key(module_id)
}

fn sandbox() -> WasmExecutor {
    WasmExecutor::new(WasmExecutorConfig::default()).expect("initialize wasm executor")
}

fn apply_module_changes(world: &mut World, actor: &str, changes: ModuleChangeSet) {
    let mut content = serde_json::Map::new();
    content.insert(
        "module_changes".to_string(),
        serde_json::to_value(&changes).expect("serialize module change set"),
    );
    let manifest = Manifest {
        version: world.manifest().version.saturating_add(1),
        content: serde_json::Value::Object(content),
    };

    let proposal_id = world
        .propose_manifest_update(manifest, actor.to_string())
        .expect("propose changes");
    world.shadow_proposal(proposal_id).expect("shadow proposal");
    world
        .approve_proposal(proposal_id, actor.to_string(), ProposalDecision::Approve)
        .expect("approve proposal");
    world.apply_proposal(proposal_id).expect("apply proposal");
}

fn factory_spec(
    factory_id: &str,
    display_name: &str,
    tier: u8,
    tags: &[&str],
    build_cost: &[(&str, i64)],
) -> FactoryModuleSpec {
    FactoryModuleSpec {
        factory_id: factory_id.to_string(),
        display_name: display_name.to_string(),
        tier,
        tags: tags.iter().map(|item| item.to_string()).collect(),
        build_cost: build_cost
            .iter()
            .map(|(kind, amount)| MaterialStack::new(*kind, *amount))
            .collect(),
        build_time_ticks: 3,
        base_power_draw: 20,
        recipe_slots: 2,
        throughput_bps: 10_000,
        maintenance_per_tick: 1,
    }
}

fn step_twice(world: &mut World, sandbox: &mut WasmExecutor) {
    world
        .step_with_modules(sandbox)
        .expect("start module-backed action");
    world
        .step_with_modules(sandbox)
        .expect("settle module-backed action");
}

fn start_and_settle_recipe(world: &mut World, sandbox: &mut WasmExecutor) {
    world
        .step_with_modules(sandbox)
        .expect("start module-backed recipe action");
    for _ in 0..8 {
        if world.pending_recipe_jobs_len() == 0 {
            break;
        }
        world
            .step_with_modules(sandbox)
            .expect("settle module-backed recipe action");
    }
}

#[test]
fn m4_builtin_module_ids_manifest_matches_runtime_constants() {
    let expected = vec![
        M4_FACTORY_MINER_MODULE_ID,
        M4_FACTORY_SMELTER_MODULE_ID,
        M4_FACTORY_ASSEMBLER_MODULE_ID,
        M4_RECIPE_SMELT_IRON_MODULE_ID,
        M4_RECIPE_SMELT_COPPER_WIRE_MODULE_ID,
        M4_RECIPE_SMELT_POLYMER_RESIN_MODULE_ID,
        M4_RECIPE_SMELT_ALLOY_PLATE_MODULE_ID,
        M4_RECIPE_ASSEMBLE_GEAR_MODULE_ID,
        M4_RECIPE_ASSEMBLE_CONTROL_CHIP_MODULE_ID,
        M4_RECIPE_ASSEMBLE_MOTOR_MODULE_ID,
        M4_RECIPE_ASSEMBLE_DRONE_MODULE_ID,
        M4_RECIPE_ASSEMBLE_SENSOR_PACK_MODULE_ID,
        M4_RECIPE_ASSEMBLE_MODULE_RACK_MODULE_ID,
        M4_RECIPE_ASSEMBLE_FACTORY_CORE_MODULE_ID,
        M4_PRODUCT_IRON_INGOT_MODULE_ID,
        M4_PRODUCT_ALLOY_PLATE_MODULE_ID,
        M4_PRODUCT_CONTROL_CHIP_MODULE_ID,
        M4_PRODUCT_MOTOR_MODULE_ID,
        M4_PRODUCT_LOGISTICS_DRONE_MODULE_ID,
        M4_PRODUCT_SENSOR_PACK_MODULE_ID,
        M4_PRODUCT_MODULE_RACK_MODULE_ID,
        M4_PRODUCT_FACTORY_CORE_MODULE_ID,
    ];
    assert_eq!(m4_bootstrap_module_ids(), expected);
    assert_eq!(m4_builtin_module_ids_manifest(), expected);
}

#[test]
fn install_m4_economy_bootstrap_modules_registers_and_activates() {
    let mut world = World::new();
    world
        .install_m4_economy_bootstrap_modules("bootstrap")
        .expect("install m4 economy modules");

    for module_id in m4_builtin_module_ids_manifest() {
        assert!(has_active(&world, module_id));
        let key = ModuleRegistry::record_key(module_id, M4_ECONOMY_MODULE_VERSION);
        assert!(world.module_registry().records.contains_key(&key));
    }
}

#[test]
fn install_m4_economy_bootstrap_modules_injects_layered_profiles() {
    let mut world = World::new();
    world
        .install_m4_economy_bootstrap_modules("bootstrap")
        .expect("install m4 economy modules");

    let iron_ore = world
        .state()
        .material_profiles
        .get("iron_ore")
        .expect("material profile iron_ore");
    assert_eq!(iron_ore.tier, 1);
    assert_eq!(iron_ore.category, "ore");

    let factory_core = world
        .state()
        .material_profiles
        .get("factory_core")
        .expect("material profile factory_core");
    assert_eq!(factory_core.tier, 5);
    assert_eq!(factory_core.category, "infrastructure");

    let module_rack = world
        .state()
        .product_profiles
        .get("module_rack")
        .expect("product profile module_rack");
    assert_eq!(module_rack.role_tag, "governance");
    assert_eq!(module_rack.unlock_stage, "governance");

    let drone_recipe = world
        .state()
        .recipe_profiles
        .get("recipe.assembler.logistics_drone")
        .expect("recipe profile logistics_drone");
    assert_eq!(drone_recipe.stage_gate, "bootstrap");
    assert!(drone_recipe
        .preferred_factory_tags
        .iter()
        .any(|tag| tag == "assembler"));
}

#[test]
fn install_m4_economy_bootstrap_modules_is_idempotent() {
    let mut world = World::new();
    world
        .install_m4_economy_bootstrap_modules("bootstrap")
        .expect("first install");
    let event_len = world.journal().len();

    world
        .install_m4_economy_bootstrap_modules("bootstrap")
        .expect("second install");

    assert_eq!(world.journal().len(), event_len);
}

#[test]
fn install_m4_economy_bootstrap_modules_reactivates_registered_version() {
    let mut world = World::new();
    world
        .install_m4_economy_bootstrap_modules("bootstrap")
        .expect("initial install");

    let registered_count = world.module_registry().records.len();

    apply_module_changes(
        &mut world,
        "bootstrap",
        ModuleChangeSet {
            deactivate: vec![ModuleDeactivation {
                module_id: M4_RECIPE_ASSEMBLE_DRONE_MODULE_ID.to_string(),
                reason: "test deactivate".to_string(),
            }],
            ..ModuleChangeSet::default()
        },
    );
    assert!(!has_active(&world, M4_RECIPE_ASSEMBLE_DRONE_MODULE_ID));

    world
        .install_m4_economy_bootstrap_modules("bootstrap")
        .expect("reactivate install");

    assert!(has_active(&world, M4_RECIPE_ASSEMBLE_DRONE_MODULE_ID));
    assert_eq!(world.module_registry().records.len(), registered_count);
}

#[test]
fn m4_economy_modules_drive_resource_to_product_chain() {
    let mut world = World::new();
    world
        .install_m4_economy_bootstrap_modules("bootstrap")
        .expect("install m4 economy package");

    let mut wasm = sandbox();
    world.submit_action(Action::RegisterAgent {
        agent_id: "builder-a".to_string(),
        pos: pos(0.0, 0.0),
    });
    world
        .step_with_modules(&mut wasm)
        .expect("register builder");

    world.set_resource_balance(ResourceKind::Electricity, 400);
    world
        .set_material_balance("structural_frame", 40)
        .expect("seed structural frames");
    world
        .set_material_balance("circuit_board", 4)
        .expect("seed circuit boards");
    world
        .set_material_balance("servo_motor", 2)
        .expect("seed servo motors");
    world
        .set_material_balance("heat_coil", 6)
        .expect("seed heat coils");
    world
        .set_material_balance("refractory_brick", 8)
        .expect("seed refractory bricks");
    world
        .set_material_balance("iron_ore", 60)
        .expect("seed iron ore");
    world
        .set_material_balance("carbon_fuel", 20)
        .expect("seed carbon fuel");
    world
        .set_material_balance("copper_ore", 60)
        .expect("seed copper ore");
    world
        .set_material_balance("silicate_ore", 20)
        .expect("seed silicate ore");
    world
        .set_material_balance("hardware_part", 40)
        .expect("seed hardware parts");

    world.submit_action(Action::BuildFactoryWithModule {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-smelter".to_string(),
        module_id: M4_FACTORY_SMELTER_MODULE_ID.to_string(),
        spec: factory_spec(
            "factory.smelter.mk1",
            "Smelter MK1",
            2,
            &["smelter", "thermal"],
            &[
                ("structural_frame", 12),
                ("heat_coil", 4),
                ("refractory_brick", 6),
            ],
        ),
    });
    step_twice(&mut world, &mut wasm);
    assert!(world.has_factory("factory.smelter.mk1"));

    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.smelter.mk1".to_string(),
        recipe_id: "recipe.smelter.iron_ingot".to_string(),
        module_id: M4_RECIPE_SMELT_IRON_MODULE_ID.to_string(),
        desired_batches: 12,
        deterministic_seed: 20260214,
    });
    start_and_settle_recipe(&mut world, &mut wasm);

    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.smelter.mk1".to_string(),
        recipe_id: "recipe.smelter.copper_wire".to_string(),
        module_id: M4_RECIPE_SMELT_COPPER_WIRE_MODULE_ID.to_string(),
        desired_batches: 12,
        deterministic_seed: 20260214,
    });
    start_and_settle_recipe(&mut world, &mut wasm);

    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.smelter.mk1".to_string(),
        recipe_id: "recipe.smelter.polymer_resin".to_string(),
        module_id: M4_RECIPE_SMELT_POLYMER_RESIN_MODULE_ID.to_string(),
        desired_batches: 4,
        deterministic_seed: 20260214,
    });
    start_and_settle_recipe(&mut world, &mut wasm);

    world.submit_action(Action::BuildFactoryWithModule {
        builder_agent_id: "builder-a".to_string(),
        site_id: "site-assembler".to_string(),
        module_id: M4_FACTORY_ASSEMBLER_MODULE_ID.to_string(),
        spec: factory_spec(
            "factory.assembler.mk1",
            "Assembler MK1",
            3,
            &["assembler", "precision"],
            &[
                ("structural_frame", 8),
                ("iron_ingot", 10),
                ("copper_wire", 8),
            ],
        ),
    });
    step_twice(&mut world, &mut wasm);
    assert!(world.has_factory("factory.assembler.mk1"));

    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.assembler.mk1".to_string(),
        recipe_id: "recipe.assembler.gear".to_string(),
        module_id: M4_RECIPE_ASSEMBLE_GEAR_MODULE_ID.to_string(),
        desired_batches: 4,
        deterministic_seed: 20260214,
    });
    start_and_settle_recipe(&mut world, &mut wasm);

    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.assembler.mk1".to_string(),
        recipe_id: "recipe.assembler.control_chip".to_string(),
        module_id: M4_RECIPE_ASSEMBLE_CONTROL_CHIP_MODULE_ID.to_string(),
        desired_batches: 4,
        deterministic_seed: 20260214,
    });
    start_and_settle_recipe(&mut world, &mut wasm);

    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.assembler.mk1".to_string(),
        recipe_id: "recipe.assembler.motor_mk1".to_string(),
        module_id: M4_RECIPE_ASSEMBLE_MOTOR_MODULE_ID.to_string(),
        desired_batches: 2,
        deterministic_seed: 20260214,
    });
    start_and_settle_recipe(&mut world, &mut wasm);

    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.assembler.mk1".to_string(),
        recipe_id: "recipe.assembler.logistics_drone".to_string(),
        module_id: M4_RECIPE_ASSEMBLE_DRONE_MODULE_ID.to_string(),
        desired_batches: 1,
        deterministic_seed: 20260214,
    });
    start_and_settle_recipe(&mut world, &mut wasm);

    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.smelter.mk1".to_string(),
        recipe_id: "recipe.smelter.alloy_plate".to_string(),
        module_id: M4_RECIPE_SMELT_ALLOY_PLATE_MODULE_ID.to_string(),
        desired_batches: 3,
        deterministic_seed: 20260214,
    });
    start_and_settle_recipe(&mut world, &mut wasm);

    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.assembler.mk1".to_string(),
        recipe_id: "recipe.assembler.sensor_pack".to_string(),
        module_id: M4_RECIPE_ASSEMBLE_SENSOR_PACK_MODULE_ID.to_string(),
        desired_batches: 2,
        deterministic_seed: 20260214,
    });
    start_and_settle_recipe(&mut world, &mut wasm);

    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.assembler.mk1".to_string(),
        recipe_id: "recipe.assembler.module_rack".to_string(),
        module_id: M4_RECIPE_ASSEMBLE_MODULE_RACK_MODULE_ID.to_string(),
        desired_batches: 1,
        deterministic_seed: 20260214,
    });
    start_and_settle_recipe(&mut world, &mut wasm);

    world.submit_action(Action::ScheduleRecipeWithModule {
        requester_agent_id: "builder-a".to_string(),
        factory_id: "factory.assembler.mk1".to_string(),
        recipe_id: "recipe.assembler.factory_core".to_string(),
        module_id: M4_RECIPE_ASSEMBLE_FACTORY_CORE_MODULE_ID.to_string(),
        desired_batches: 1,
        deterministic_seed: 20260214,
    });
    start_and_settle_recipe(&mut world, &mut wasm);

    assert_eq!(world.material_balance("factory_core"), 1);
    assert_eq!(world.material_balance("module_rack"), 0);
    assert_eq!(world.material_balance("sensor_pack"), 0);
    assert_eq!(world.material_balance("logistics_drone"), 1);
    assert_eq!(world.material_balance("motor_mk1"), 0);
    assert_eq!(world.material_balance("control_chip"), 0);
    assert_eq!(world.material_balance("gear"), 0);
    assert_eq!(world.material_balance("alloy_plate"), 2);
    assert_eq!(world.material_balance("hardware_part"), 24);
    assert_eq!(world.material_balance("iron_ingot"), 10);
    assert_eq!(world.material_balance("copper_wire"), 8);
    assert_eq!(world.material_balance("slag"), 15);
    assert_eq!(world.material_balance("waste_resin"), 8);
    assert_eq!(world.material_balance("assembly_scrap"), 1);
    assert_eq!(world.material_balance("calibration_scrap"), 2);
    assert_eq!(world.material_balance("precision_scrap"), 1);
    assert_eq!(world.material_balance("structural_waste"), 1);
    assert_eq!(world.resource_balance(ResourceKind::Electricity), 71);

    let rejected_events = world
        .journal()
        .events
        .iter()
        .filter(|event| {
            matches!(
                event.body,
                WorldEventBody::Domain(DomainEvent::ActionRejected { .. })
            )
        })
        .count();
    assert_eq!(rejected_events, 0);
}
