use crate::runtime::{Action as RuntimeAction, RecipeExecutionPlan};
use crate::viewer::{GameplayActionError, GameplayActionRequest};
use oasis7_wasm_abi::{FactoryModuleSpec, MaterialStack};

pub const ACTION_BUILD_SMELTER_MK1: &str = "build_factory_smelter_mk1";
pub const ACTION_BUILD_ASSEMBLER_MK1: &str = "build_factory_assembler_mk1";
pub const ACTION_SCHEDULE_SMELTER_IRON_INGOT: &str = "schedule_recipe_smelter_iron_ingot";
pub const ACTION_SCHEDULE_SMELTER_COPPER_WIRE: &str = "schedule_recipe_smelter_copper_wire";
pub const ACTION_SCHEDULE_SMELTER_POLYMER_RESIN: &str = "schedule_recipe_smelter_polymer_resin";
pub const ACTION_SCHEDULE_SMELTER_ALLOY_PLATE: &str = "schedule_recipe_smelter_alloy_plate";
pub const ACTION_SCHEDULE_ASSEMBLER_GEAR: &str = "schedule_recipe_assembler_gear";
pub const ACTION_SCHEDULE_ASSEMBLER_CONTROL_CHIP: &str = "schedule_recipe_assembler_control_chip";
pub const ACTION_SCHEDULE_ASSEMBLER_MOTOR_MK1: &str = "schedule_recipe_assembler_motor_mk1";
pub const ACTION_SCHEDULE_ASSEMBLER_LOGISTICS_DRONE: &str =
    "schedule_recipe_assembler_logistics_drone";
pub const ACTION_SCHEDULE_ASSEMBLER_SENSOR_PACK: &str = "schedule_recipe_assembler_sensor_pack";
pub const ACTION_SCHEDULE_ASSEMBLER_MODULE_RACK: &str = "schedule_recipe_assembler_module_rack";
pub const ACTION_SCHEDULE_ASSEMBLER_FACTORY_CORE: &str = "schedule_recipe_assembler_factory_core";
pub const FACTORY_SMELTER_MK1: &str = "factory.smelter.mk1";
pub const FACTORY_ASSEMBLER_MK1: &str = "factory.assembler.mk1";

const RECIPE_SMELTER_IRON_INGOT: &str = "recipe.smelter.iron_ingot";
const RECIPE_SMELTER_COPPER_WIRE: &str = "recipe.smelter.copper_wire";
const RECIPE_SMELTER_POLYMER_RESIN: &str = "recipe.smelter.polymer_resin";
const RECIPE_SMELTER_ALLOY_PLATE: &str = "recipe.smelter.alloy_plate";
const RECIPE_ASSEMBLER_GEAR: &str = "recipe.assembler.gear";
const RECIPE_ASSEMBLER_CONTROL_CHIP: &str = "recipe.assembler.control_chip";
const RECIPE_ASSEMBLER_MOTOR_MK1: &str = "recipe.assembler.motor_mk1";
const RECIPE_ASSEMBLER_LOGISTICS_DRONE: &str = "recipe.assembler.logistics_drone";
const RECIPE_ASSEMBLER_SENSOR_PACK: &str = "recipe.assembler.sensor_pack";
const RECIPE_ASSEMBLER_MODULE_RACK: &str = "recipe.assembler.module_rack";
const RECIPE_ASSEMBLER_FACTORY_CORE: &str = "recipe.assembler.factory_core";

pub fn build_runtime_action_from_gameplay_request(
    request: &GameplayActionRequest,
) -> Result<RuntimeAction, GameplayActionError> {
    let target_agent_id = request.target_agent_id.trim();
    if target_agent_id.is_empty() {
        return Err(GameplayActionError {
            code: "target_agent_required".to_string(),
            message: "gameplay_action requires non-empty target_agent_id".to_string(),
            action_id: Some(request.action_id.clone()),
            target_agent_id: Some(request.target_agent_id.clone()),
        });
    }

    let action = match request.action_id.as_str() {
        ACTION_BUILD_SMELTER_MK1 => RuntimeAction::BuildFactory {
            builder_agent_id: target_agent_id.to_string(),
            site_id: "site-smelter".to_string(),
            spec: smelter_factory_spec(),
        },
        ACTION_BUILD_ASSEMBLER_MK1 => RuntimeAction::BuildFactory {
            builder_agent_id: target_agent_id.to_string(),
            site_id: "site-assembler".to_string(),
            spec: assembler_factory_spec(),
        },
        ACTION_SCHEDULE_SMELTER_IRON_INGOT => RuntimeAction::ScheduleRecipe {
            requester_agent_id: target_agent_id.to_string(),
            factory_id: FACTORY_SMELTER_MK1.to_string(),
            recipe_id: RECIPE_SMELTER_IRON_INGOT.to_string(),
            plan: plan_smelter_iron_ingot(),
        },
        ACTION_SCHEDULE_SMELTER_COPPER_WIRE => RuntimeAction::ScheduleRecipe {
            requester_agent_id: target_agent_id.to_string(),
            factory_id: FACTORY_SMELTER_MK1.to_string(),
            recipe_id: RECIPE_SMELTER_COPPER_WIRE.to_string(),
            plan: plan_smelter_copper_wire(),
        },
        ACTION_SCHEDULE_SMELTER_POLYMER_RESIN => RuntimeAction::ScheduleRecipe {
            requester_agent_id: target_agent_id.to_string(),
            factory_id: FACTORY_SMELTER_MK1.to_string(),
            recipe_id: RECIPE_SMELTER_POLYMER_RESIN.to_string(),
            plan: plan_smelter_polymer_resin(),
        },
        ACTION_SCHEDULE_SMELTER_ALLOY_PLATE => RuntimeAction::ScheduleRecipe {
            requester_agent_id: target_agent_id.to_string(),
            factory_id: FACTORY_SMELTER_MK1.to_string(),
            recipe_id: RECIPE_SMELTER_ALLOY_PLATE.to_string(),
            plan: plan_smelter_alloy_plate(),
        },
        ACTION_SCHEDULE_ASSEMBLER_GEAR => RuntimeAction::ScheduleRecipe {
            requester_agent_id: target_agent_id.to_string(),
            factory_id: FACTORY_ASSEMBLER_MK1.to_string(),
            recipe_id: RECIPE_ASSEMBLER_GEAR.to_string(),
            plan: plan_assembler_gear(),
        },
        ACTION_SCHEDULE_ASSEMBLER_CONTROL_CHIP => RuntimeAction::ScheduleRecipe {
            requester_agent_id: target_agent_id.to_string(),
            factory_id: FACTORY_ASSEMBLER_MK1.to_string(),
            recipe_id: RECIPE_ASSEMBLER_CONTROL_CHIP.to_string(),
            plan: plan_assembler_control_chip(),
        },
        ACTION_SCHEDULE_ASSEMBLER_MOTOR_MK1 => RuntimeAction::ScheduleRecipe {
            requester_agent_id: target_agent_id.to_string(),
            factory_id: FACTORY_ASSEMBLER_MK1.to_string(),
            recipe_id: RECIPE_ASSEMBLER_MOTOR_MK1.to_string(),
            plan: plan_assembler_motor_mk1(),
        },
        ACTION_SCHEDULE_ASSEMBLER_LOGISTICS_DRONE => RuntimeAction::ScheduleRecipe {
            requester_agent_id: target_agent_id.to_string(),
            factory_id: FACTORY_ASSEMBLER_MK1.to_string(),
            recipe_id: RECIPE_ASSEMBLER_LOGISTICS_DRONE.to_string(),
            plan: plan_assembler_logistics_drone(),
        },
        ACTION_SCHEDULE_ASSEMBLER_SENSOR_PACK => RuntimeAction::ScheduleRecipe {
            requester_agent_id: target_agent_id.to_string(),
            factory_id: FACTORY_ASSEMBLER_MK1.to_string(),
            recipe_id: RECIPE_ASSEMBLER_SENSOR_PACK.to_string(),
            plan: plan_assembler_sensor_pack(),
        },
        ACTION_SCHEDULE_ASSEMBLER_MODULE_RACK => RuntimeAction::ScheduleRecipe {
            requester_agent_id: target_agent_id.to_string(),
            factory_id: FACTORY_ASSEMBLER_MK1.to_string(),
            recipe_id: RECIPE_ASSEMBLER_MODULE_RACK.to_string(),
            plan: plan_assembler_module_rack(),
        },
        ACTION_SCHEDULE_ASSEMBLER_FACTORY_CORE => RuntimeAction::ScheduleRecipe {
            requester_agent_id: target_agent_id.to_string(),
            factory_id: FACTORY_ASSEMBLER_MK1.to_string(),
            recipe_id: RECIPE_ASSEMBLER_FACTORY_CORE.to_string(),
            plan: plan_assembler_factory_core(),
        },
        _ => {
            return Err(GameplayActionError {
                code: "unknown_gameplay_action".to_string(),
                message: format!(
                    "unknown gameplay action `{}` for target `{}`",
                    request.action_id, request.target_agent_id
                ),
                action_id: Some(request.action_id.clone()),
                target_agent_id: Some(request.target_agent_id.clone()),
            });
        }
    };
    Ok(action)
}

fn smelter_factory_spec() -> FactoryModuleSpec {
    FactoryModuleSpec {
        factory_id: FACTORY_SMELTER_MK1.to_string(),
        display_name: "Smelter MK1".to_string(),
        tier: 2,
        tags: vec!["smelter".to_string(), "thermal".to_string()],
        build_cost: vec![
            MaterialStack::new("structural_frame", 12),
            MaterialStack::new("heat_coil", 4),
            MaterialStack::new("refractory_brick", 6),
        ],
        build_time_ticks: 1,
        base_power_draw: 20,
        recipe_slots: 2,
        throughput_bps: 10_000,
        maintenance_per_tick: 1,
    }
}

fn assembler_factory_spec() -> FactoryModuleSpec {
    FactoryModuleSpec {
        factory_id: FACTORY_ASSEMBLER_MK1.to_string(),
        display_name: "Assembler MK1".to_string(),
        tier: 3,
        tags: vec!["assembler".to_string(), "precision".to_string()],
        build_cost: vec![
            MaterialStack::new("structural_frame", 8),
            MaterialStack::new("iron_ingot", 10),
            MaterialStack::new("copper_wire", 8),
        ],
        build_time_ticks: 1,
        base_power_draw: 20,
        recipe_slots: 2,
        throughput_bps: 10_000,
        maintenance_per_tick: 1,
    }
}

fn plan_smelter_iron_ingot() -> RecipeExecutionPlan {
    RecipeExecutionPlan::accepted(
        12,
        vec![
            MaterialStack::new("iron_ore", 48),
            MaterialStack::new("carbon_fuel", 12),
        ],
        vec![MaterialStack::new("iron_ingot", 36)],
        vec![MaterialStack::new("slag", 12)],
        96,
        1,
    )
}

fn plan_smelter_copper_wire() -> RecipeExecutionPlan {
    RecipeExecutionPlan::accepted(
        12,
        vec![MaterialStack::new("copper_ore", 36)],
        vec![MaterialStack::new("copper_wire", 48)],
        Vec::new(),
        72,
        1,
    )
}

fn plan_smelter_polymer_resin() -> RecipeExecutionPlan {
    RecipeExecutionPlan::accepted(
        4,
        vec![
            MaterialStack::new("carbon_fuel", 8),
            MaterialStack::new("silicate_ore", 8),
        ],
        vec![MaterialStack::new("polymer_resin", 8)],
        vec![MaterialStack::new("waste_resin", 4)],
        28,
        1,
    )
}

fn plan_smelter_alloy_plate() -> RecipeExecutionPlan {
    RecipeExecutionPlan::accepted(
        4,
        vec![
            MaterialStack::new("iron_ingot", 8),
            MaterialStack::new("copper_wire", 8),
        ],
        vec![MaterialStack::new("alloy_plate", 8)],
        vec![MaterialStack::new("slag", 4)],
        36,
        1,
    )
}

fn plan_assembler_gear() -> RecipeExecutionPlan {
    RecipeExecutionPlan::accepted(
        4,
        vec![MaterialStack::new("iron_ingot", 8)],
        vec![MaterialStack::new("gear", 4)],
        Vec::new(),
        16,
        1,
    )
}

fn plan_assembler_control_chip() -> RecipeExecutionPlan {
    RecipeExecutionPlan::accepted(
        4,
        vec![
            MaterialStack::new("copper_wire", 16),
            MaterialStack::new("polymer_resin", 8),
        ],
        vec![MaterialStack::new("control_chip", 4)],
        vec![MaterialStack::new("waste_resin", 4)],
        24,
        1,
    )
}

fn plan_assembler_motor_mk1() -> RecipeExecutionPlan {
    RecipeExecutionPlan::accepted(
        2,
        vec![
            MaterialStack::new("gear", 4),
            MaterialStack::new("copper_wire", 6),
        ],
        vec![MaterialStack::new("motor_mk1", 2)],
        Vec::new(),
        14,
        1,
    )
}

fn plan_assembler_logistics_drone() -> RecipeExecutionPlan {
    RecipeExecutionPlan::accepted(
        1,
        vec![
            MaterialStack::new("motor_mk1", 2),
            MaterialStack::new("control_chip", 1),
            MaterialStack::new("iron_ingot", 2),
        ],
        vec![MaterialStack::new("logistics_drone", 1)],
        vec![MaterialStack::new("assembly_scrap", 1)],
        12,
        1,
    )
}

fn plan_assembler_sensor_pack() -> RecipeExecutionPlan {
    RecipeExecutionPlan::accepted(
        2,
        vec![
            MaterialStack::new("control_chip", 2),
            MaterialStack::new("copper_wire", 4),
        ],
        vec![MaterialStack::new("sensor_pack", 2)],
        vec![MaterialStack::new("calibration_scrap", 2)],
        16,
        1,
    )
}

fn plan_assembler_module_rack() -> RecipeExecutionPlan {
    RecipeExecutionPlan::accepted(
        1,
        vec![
            MaterialStack::new("sensor_pack", 2),
            MaterialStack::new("control_chip", 1),
        ],
        vec![MaterialStack::new("module_rack", 1)],
        vec![MaterialStack::new("precision_scrap", 1)],
        10,
        1,
    )
}

fn plan_assembler_factory_core() -> RecipeExecutionPlan {
    RecipeExecutionPlan::accepted(
        1,
        vec![
            MaterialStack::new("module_rack", 1),
            MaterialStack::new("alloy_plate", 3),
        ],
        vec![MaterialStack::new("factory_core", 1)],
        vec![MaterialStack::new("structural_waste", 1)],
        14,
        1,
    )
}
