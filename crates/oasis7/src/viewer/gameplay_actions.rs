#[cfg(not(target_arch = "wasm32"))]
use crate::{
    runtime::{Action as RuntimeAction, RecipeExecutionPlan},
    viewer::{GameplayActionError, GameplayActionRequest},
};
#[cfg(not(target_arch = "wasm32"))]
use oasis7_wasm_abi::{FactoryModuleSpec, MaterialStack};

pub const ACTION_BUILD_SMELTER_MK1: &str = "build_factory_smelter_mk1";
pub const ACTION_BUILD_ASSEMBLER_MK1: &str = "build_factory_assembler_mk1";
pub const ACTION_CLAIM_AGENT: &str = "claim_agent";
pub const ACTION_RELEASE_AGENT_CLAIM: &str = "release_agent_claim";
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

#[cfg(not(target_arch = "wasm32"))]
const RECIPE_SMELTER_IRON_INGOT: &str = "recipe.smelter.iron_ingot";
#[cfg(not(target_arch = "wasm32"))]
const RECIPE_SMELTER_COPPER_WIRE: &str = "recipe.smelter.copper_wire";
#[cfg(not(target_arch = "wasm32"))]
const RECIPE_SMELTER_POLYMER_RESIN: &str = "recipe.smelter.polymer_resin";
#[cfg(not(target_arch = "wasm32"))]
const RECIPE_SMELTER_ALLOY_PLATE: &str = "recipe.smelter.alloy_plate";
#[cfg(not(target_arch = "wasm32"))]
const RECIPE_ASSEMBLER_GEAR: &str = "recipe.assembler.gear";
#[cfg(not(target_arch = "wasm32"))]
const RECIPE_ASSEMBLER_CONTROL_CHIP: &str = "recipe.assembler.control_chip";
#[cfg(not(target_arch = "wasm32"))]
const RECIPE_ASSEMBLER_MOTOR_MK1: &str = "recipe.assembler.motor_mk1";
#[cfg(not(target_arch = "wasm32"))]
const RECIPE_ASSEMBLER_LOGISTICS_DRONE: &str = "recipe.assembler.logistics_drone";
#[cfg(not(target_arch = "wasm32"))]
const RECIPE_ASSEMBLER_SENSOR_PACK: &str = "recipe.assembler.sensor_pack";
#[cfg(not(target_arch = "wasm32"))]
const RECIPE_ASSEMBLER_MODULE_RACK: &str = "recipe.assembler.module_rack";
#[cfg(not(target_arch = "wasm32"))]
const RECIPE_ASSEMBLER_FACTORY_CORE: &str = "recipe.assembler.factory_core";

#[cfg(not(target_arch = "wasm32"))]
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
        ACTION_CLAIM_AGENT => Some(RuntimeAction::ClaimAgent {
            claimer_agent_id: required_actor_agent_id(request)?,
            target_agent_id: target_agent_id.to_string(),
        }),
        ACTION_RELEASE_AGENT_CLAIM => Some(RuntimeAction::ReleaseAgentClaim {
            claimer_agent_id: required_actor_agent_id(request)?,
            target_agent_id: target_agent_id.to_string(),
        }),
        ACTION_BUILD_SMELTER_MK1 => runtime_factory_build_action(
            target_agent_id,
            "site-smelter",
            FACTORY_SMELTER_MK1,
            FACTORY_SMELTER_MK1,
        ),
        ACTION_BUILD_ASSEMBLER_MK1 => runtime_factory_build_action(
            target_agent_id,
            "site-assembler",
            FACTORY_ASSEMBLER_MK1,
            FACTORY_ASSEMBLER_MK1,
        ),
        ACTION_SCHEDULE_SMELTER_IRON_INGOT => runtime_schedule_recipe_action(
            target_agent_id,
            FACTORY_SMELTER_MK1,
            RECIPE_SMELTER_IRON_INGOT,
            12,
        ),
        ACTION_SCHEDULE_SMELTER_COPPER_WIRE => runtime_schedule_recipe_action(
            target_agent_id,
            FACTORY_SMELTER_MK1,
            RECIPE_SMELTER_COPPER_WIRE,
            12,
        ),
        ACTION_SCHEDULE_SMELTER_POLYMER_RESIN => runtime_schedule_recipe_action(
            target_agent_id,
            FACTORY_SMELTER_MK1,
            RECIPE_SMELTER_POLYMER_RESIN,
            4,
        ),
        ACTION_SCHEDULE_SMELTER_ALLOY_PLATE => runtime_schedule_recipe_action(
            target_agent_id,
            FACTORY_SMELTER_MK1,
            RECIPE_SMELTER_ALLOY_PLATE,
            4,
        ),
        ACTION_SCHEDULE_ASSEMBLER_GEAR => runtime_schedule_recipe_action(
            target_agent_id,
            FACTORY_ASSEMBLER_MK1,
            RECIPE_ASSEMBLER_GEAR,
            4,
        ),
        ACTION_SCHEDULE_ASSEMBLER_CONTROL_CHIP => runtime_schedule_recipe_action(
            target_agent_id,
            FACTORY_ASSEMBLER_MK1,
            RECIPE_ASSEMBLER_CONTROL_CHIP,
            4,
        ),
        ACTION_SCHEDULE_ASSEMBLER_MOTOR_MK1 => runtime_schedule_recipe_action(
            target_agent_id,
            FACTORY_ASSEMBLER_MK1,
            RECIPE_ASSEMBLER_MOTOR_MK1,
            2,
        ),
        ACTION_SCHEDULE_ASSEMBLER_LOGISTICS_DRONE => runtime_schedule_recipe_action(
            target_agent_id,
            FACTORY_ASSEMBLER_MK1,
            RECIPE_ASSEMBLER_LOGISTICS_DRONE,
            1,
        ),
        ACTION_SCHEDULE_ASSEMBLER_SENSOR_PACK => runtime_schedule_recipe_action(
            target_agent_id,
            FACTORY_ASSEMBLER_MK1,
            RECIPE_ASSEMBLER_SENSOR_PACK,
            2,
        ),
        ACTION_SCHEDULE_ASSEMBLER_MODULE_RACK => runtime_schedule_recipe_action(
            target_agent_id,
            FACTORY_ASSEMBLER_MK1,
            RECIPE_ASSEMBLER_MODULE_RACK,
            1,
        ),
        ACTION_SCHEDULE_ASSEMBLER_FACTORY_CORE => runtime_schedule_recipe_action(
            target_agent_id,
            FACTORY_ASSEMBLER_MK1,
            RECIPE_ASSEMBLER_FACTORY_CORE,
            1,
        ),
        _ => None,
    }
    .ok_or_else(|| GameplayActionError {
        code: "unknown_gameplay_action".to_string(),
        message: format!(
            "unknown gameplay action `{}` for target `{}`",
            request.action_id, request.target_agent_id
        ),
        action_id: Some(request.action_id.clone()),
        target_agent_id: Some(request.target_agent_id.clone()),
    })?;
    Ok(action)
}

pub fn gameplay_action_requires_actor_agent(action_id: &str) -> bool {
    matches!(action_id, ACTION_CLAIM_AGENT | ACTION_RELEASE_AGENT_CLAIM)
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::viewer) fn runtime_factory_build_action(
    builder_agent_id: &str,
    site_id: &str,
    factory_id: &str,
    factory_kind: &str,
) -> Option<RuntimeAction> {
    Some(RuntimeAction::BuildFactory {
        builder_agent_id: builder_agent_id.to_string(),
        site_id: site_id.to_string(),
        spec: factory_spec_for_kind(factory_kind, factory_id)?,
    })
}

#[cfg(not(target_arch = "wasm32"))]
pub(in crate::viewer) fn runtime_schedule_recipe_action(
    requester_agent_id: &str,
    factory_id: &str,
    recipe_id: &str,
    accepted_batches: u32,
) -> Option<RuntimeAction> {
    Some(RuntimeAction::ScheduleRecipe {
        requester_agent_id: requester_agent_id.to_string(),
        factory_id: factory_id.to_string(),
        recipe_id: recipe_id.to_string(),
        plan: recipe_plan_for_id(recipe_id, accepted_batches)?,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn required_actor_agent_id(request: &GameplayActionRequest) -> Result<String, GameplayActionError> {
    let actor_agent_id = request
        .actor_agent_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| GameplayActionError {
            code: "actor_agent_required".to_string(),
            message: format!(
                "gameplay_action `{}` requires non-empty actor_agent_id",
                request.action_id
            ),
            action_id: Some(request.action_id.clone()),
            target_agent_id: Some(request.target_agent_id.clone()),
        })?;
    Ok(actor_agent_id.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn factory_spec_for_kind(factory_kind: &str, factory_id: &str) -> Option<FactoryModuleSpec> {
    let mut spec = match factory_kind {
        FACTORY_SMELTER_MK1 => FactoryModuleSpec {
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
        },
        FACTORY_ASSEMBLER_MK1 => FactoryModuleSpec {
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
        },
        _ => return None,
    };
    spec.factory_id = factory_id.to_string();
    Some(spec)
}

#[cfg(not(target_arch = "wasm32"))]
fn scale_material_stacks(stacks: &[MaterialStack], scale: i64) -> Vec<MaterialStack> {
    stacks
        .iter()
        .map(|stack| MaterialStack::new(stack.kind.clone(), stack.amount.saturating_mul(scale)))
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn recipe_plan_for_id(recipe_id: &str, accepted_batches: u32) -> Option<RecipeExecutionPlan> {
    let accepted_batches_i64 = i64::from(accepted_batches);
    let (consume, produce, byproducts, power_per_batch, duration_ticks): (
        Vec<MaterialStack>,
        Vec<MaterialStack>,
        Vec<MaterialStack>,
        i64,
        u32,
    ) = match recipe_id {
        RECIPE_SMELTER_IRON_INGOT => (
            vec![
                MaterialStack::new("iron_ore", 4),
                MaterialStack::new("carbon_fuel", 1),
            ],
            vec![MaterialStack::new("iron_ingot", 3)],
            vec![MaterialStack::new("slag", 1)],
            8,
            1,
        ),
        RECIPE_SMELTER_COPPER_WIRE => (
            vec![MaterialStack::new("copper_ore", 3)],
            vec![MaterialStack::new("copper_wire", 4)],
            Vec::new(),
            6,
            1,
        ),
        RECIPE_SMELTER_POLYMER_RESIN => (
            vec![
                MaterialStack::new("carbon_fuel", 2),
                MaterialStack::new("silicate_ore", 2),
            ],
            vec![MaterialStack::new("polymer_resin", 2)],
            vec![MaterialStack::new("waste_resin", 1)],
            7,
            1,
        ),
        RECIPE_SMELTER_ALLOY_PLATE => (
            vec![
                MaterialStack::new("iron_ingot", 2),
                MaterialStack::new("copper_wire", 2),
            ],
            vec![MaterialStack::new("alloy_plate", 2)],
            vec![MaterialStack::new("slag", 1)],
            9,
            1,
        ),
        RECIPE_ASSEMBLER_GEAR => (
            vec![MaterialStack::new("iron_ingot", 2)],
            vec![MaterialStack::new("gear", 1)],
            Vec::new(),
            4,
            1,
        ),
        RECIPE_ASSEMBLER_CONTROL_CHIP => (
            vec![
                MaterialStack::new("copper_wire", 4),
                MaterialStack::new("polymer_resin", 2),
            ],
            vec![MaterialStack::new("control_chip", 1)],
            vec![MaterialStack::new("waste_resin", 1)],
            6,
            1,
        ),
        RECIPE_ASSEMBLER_MOTOR_MK1 => (
            vec![
                MaterialStack::new("gear", 2),
                MaterialStack::new("copper_wire", 3),
            ],
            vec![MaterialStack::new("motor_mk1", 1)],
            Vec::new(),
            7,
            1,
        ),
        RECIPE_ASSEMBLER_LOGISTICS_DRONE => (
            vec![
                MaterialStack::new("motor_mk1", 2),
                MaterialStack::new("control_chip", 1),
                MaterialStack::new("iron_ingot", 2),
            ],
            vec![MaterialStack::new("logistics_drone", 1)],
            vec![MaterialStack::new("assembly_scrap", 1)],
            12,
            1,
        ),
        RECIPE_ASSEMBLER_SENSOR_PACK => (
            vec![
                MaterialStack::new("control_chip", 1),
                MaterialStack::new("copper_wire", 2),
            ],
            vec![MaterialStack::new("sensor_pack", 1)],
            vec![MaterialStack::new("calibration_scrap", 1)],
            8,
            1,
        ),
        RECIPE_ASSEMBLER_MODULE_RACK => (
            vec![
                MaterialStack::new("sensor_pack", 2),
                MaterialStack::new("control_chip", 1),
            ],
            vec![MaterialStack::new("module_rack", 1)],
            vec![MaterialStack::new("precision_scrap", 1)],
            10,
            1,
        ),
        RECIPE_ASSEMBLER_FACTORY_CORE => (
            vec![
                MaterialStack::new("module_rack", 1),
                MaterialStack::new("alloy_plate", 3),
            ],
            vec![MaterialStack::new("factory_core", 1)],
            vec![MaterialStack::new("structural_waste", 1)],
            14,
            1,
        ),
        _ => return None,
    };

    Some(RecipeExecutionPlan::accepted(
        accepted_batches,
        scale_material_stacks(&consume, accepted_batches_i64),
        scale_material_stacks(&produce, accepted_batches_i64),
        scale_material_stacks(&byproducts, accepted_batches_i64),
        power_per_batch.saturating_mul(accepted_batches_i64),
        duration_ticks,
    ))
}
