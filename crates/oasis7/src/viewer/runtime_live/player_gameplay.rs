use super::*;

use super::super::auth::{verify_gameplay_action_auth_proof, VerifiedPlayerAuth};
use super::super::protocol::{GameplayActionAck, GameplayActionError, GameplayActionRequest};
use super::control_plane::{
    ensure_agent_player_access_runtime, map_auth_verify_error_code, normalize_optional_public_key,
};
use crate::runtime::{
    Action as RuntimeAction, IndustryStage, MaterialLedgerId, RecipeExecutionPlan, WorldState,
};
use crate::simulator::{PlayerGameplayAction, PlayerGameplayRecentFeedback};
use oasis7_wasm_abi::{FactoryModuleSpec, MaterialStack};
use std::collections::BTreeMap;

const GAMEPLAY_ACTION_PROTOCOL: &str = "gameplay_action.submit";
const ACTION_BUILD_SMELTER_MK1: &str = "build_factory_smelter_mk1";
const ACTION_BUILD_ASSEMBLER_MK1: &str = "build_factory_assembler_mk1";
const ACTION_SCHEDULE_SMELTER_IRON_INGOT: &str = "schedule_recipe_smelter_iron_ingot";
const ACTION_SCHEDULE_SMELTER_COPPER_WIRE: &str = "schedule_recipe_smelter_copper_wire";
const ACTION_SCHEDULE_SMELTER_POLYMER_RESIN: &str = "schedule_recipe_smelter_polymer_resin";
const ACTION_SCHEDULE_SMELTER_ALLOY_PLATE: &str = "schedule_recipe_smelter_alloy_plate";
const ACTION_SCHEDULE_ASSEMBLER_GEAR: &str = "schedule_recipe_assembler_gear";
const ACTION_SCHEDULE_ASSEMBLER_CONTROL_CHIP: &str = "schedule_recipe_assembler_control_chip";
const ACTION_SCHEDULE_ASSEMBLER_MOTOR_MK1: &str = "schedule_recipe_assembler_motor_mk1";
const ACTION_SCHEDULE_ASSEMBLER_LOGISTICS_DRONE: &str = "schedule_recipe_assembler_logistics_drone";
const ACTION_SCHEDULE_ASSEMBLER_SENSOR_PACK: &str = "schedule_recipe_assembler_sensor_pack";
const ACTION_SCHEDULE_ASSEMBLER_MODULE_RACK: &str = "schedule_recipe_assembler_module_rack";
const ACTION_SCHEDULE_ASSEMBLER_FACTORY_CORE: &str = "schedule_recipe_assembler_factory_core";
pub(super) const FACTORY_SMELTER_MK1: &str = "factory.smelter.mk1";
const FACTORY_ASSEMBLER_MK1: &str = "factory.assembler.mk1";
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
pub(super) fn supports_runtime_gameplay_actions() -> bool {
    true
}

pub(super) fn extend_available_actions(
    state: &WorldState,
    first_agent_id: Option<&str>,
    actions: &mut Vec<PlayerGameplayAction>,
) {
    if !supports_runtime_gameplay_actions() {
        return;
    }
    let Some(agent_id) = first_agent_id else {
        return;
    };

    let empty_materials = BTreeMap::new();
    let world_materials = state
        .material_ledgers
        .get(&MaterialLedgerId::world())
        .unwrap_or(&empty_materials);
    let agent_materials = state
        .material_ledgers
        .get(&MaterialLedgerId::agent(agent_id.to_string()))
        .unwrap_or(&empty_materials);
    let smelter_exists = state.factories.contains_key(FACTORY_SMELTER_MK1);
    let assembler_exists = state.factories.contains_key(FACTORY_ASSEMBLER_MK1);
    let industry_stage = state.industry_progress.stage;

    if !smelter_exists {
        actions.push(PlayerGameplayAction {
            action_id: ACTION_BUILD_SMELTER_MK1.to_string(),
            label: "Queue Smelter MK1 construction".to_string(),
            protocol_action: GAMEPLAY_ACTION_PROTOCOL.to_string(),
            target_agent_id: Some(agent_id.to_string()),
            disabled_reason: missing_materials_reason_with_world_fallback(
                &agent_materials,
                &world_materials,
                &[
                    ("structural_frame", 12),
                    ("heat_coil", 4),
                    ("refractory_brick", 6),
                ],
            ),
        });
        return;
    }

    actions.extend([
        PlayerGameplayAction {
            action_id: ACTION_SCHEDULE_SMELTER_IRON_INGOT.to_string(),
            label: "Queue iron ingot run".to_string(),
            protocol_action: GAMEPLAY_ACTION_PROTOCOL.to_string(),
            target_agent_id: Some(agent_id.to_string()),
            disabled_reason: None,
        },
        PlayerGameplayAction {
            action_id: ACTION_SCHEDULE_SMELTER_COPPER_WIRE.to_string(),
            label: "Queue copper wire run".to_string(),
            protocol_action: GAMEPLAY_ACTION_PROTOCOL.to_string(),
            target_agent_id: Some(agent_id.to_string()),
            disabled_reason: None,
        },
        PlayerGameplayAction {
            action_id: ACTION_SCHEDULE_SMELTER_POLYMER_RESIN.to_string(),
            label: "Queue polymer resin run".to_string(),
            protocol_action: GAMEPLAY_ACTION_PROTOCOL.to_string(),
            target_agent_id: Some(agent_id.to_string()),
            disabled_reason: None,
        },
        PlayerGameplayAction {
            action_id: ACTION_SCHEDULE_SMELTER_ALLOY_PLATE.to_string(),
            label: "Queue alloy plate run".to_string(),
            protocol_action: GAMEPLAY_ACTION_PROTOCOL.to_string(),
            target_agent_id: Some(agent_id.to_string()),
            disabled_reason: stage_gate_disabled_reason(industry_stage, IndustryStage::ScaleOut),
        },
    ]);

    if !assembler_exists {
        actions.push(PlayerGameplayAction {
            action_id: ACTION_BUILD_ASSEMBLER_MK1.to_string(),
            label: "Queue Assembler MK1 construction".to_string(),
            protocol_action: GAMEPLAY_ACTION_PROTOCOL.to_string(),
            target_agent_id: Some(agent_id.to_string()),
            disabled_reason: missing_materials_reason_with_world_fallback(
                &agent_materials,
                &world_materials,
                &[
                    ("structural_frame", 8),
                    ("iron_ingot", 10),
                    ("copper_wire", 8),
                ],
            ),
        });
        return;
    }

    actions.extend([
        PlayerGameplayAction {
            action_id: ACTION_SCHEDULE_ASSEMBLER_GEAR.to_string(),
            label: "Queue gear run".to_string(),
            protocol_action: GAMEPLAY_ACTION_PROTOCOL.to_string(),
            target_agent_id: Some(agent_id.to_string()),
            disabled_reason: None,
        },
        PlayerGameplayAction {
            action_id: ACTION_SCHEDULE_ASSEMBLER_CONTROL_CHIP.to_string(),
            label: "Queue control chip run".to_string(),
            protocol_action: GAMEPLAY_ACTION_PROTOCOL.to_string(),
            target_agent_id: Some(agent_id.to_string()),
            disabled_reason: None,
        },
        PlayerGameplayAction {
            action_id: ACTION_SCHEDULE_ASSEMBLER_MOTOR_MK1.to_string(),
            label: "Queue motor MK1 run".to_string(),
            protocol_action: GAMEPLAY_ACTION_PROTOCOL.to_string(),
            target_agent_id: Some(agent_id.to_string()),
            disabled_reason: None,
        },
        PlayerGameplayAction {
            action_id: ACTION_SCHEDULE_ASSEMBLER_LOGISTICS_DRONE.to_string(),
            label: "Queue logistics drone run".to_string(),
            protocol_action: GAMEPLAY_ACTION_PROTOCOL.to_string(),
            target_agent_id: Some(agent_id.to_string()),
            disabled_reason: None,
        },
        PlayerGameplayAction {
            action_id: ACTION_SCHEDULE_ASSEMBLER_SENSOR_PACK.to_string(),
            label: "Queue sensor pack run".to_string(),
            protocol_action: GAMEPLAY_ACTION_PROTOCOL.to_string(),
            target_agent_id: Some(agent_id.to_string()),
            disabled_reason: stage_gate_disabled_reason(industry_stage, IndustryStage::ScaleOut),
        },
        PlayerGameplayAction {
            action_id: ACTION_SCHEDULE_ASSEMBLER_MODULE_RACK.to_string(),
            label: "Queue module rack run".to_string(),
            protocol_action: GAMEPLAY_ACTION_PROTOCOL.to_string(),
            target_agent_id: Some(agent_id.to_string()),
            disabled_reason: stage_gate_disabled_reason(industry_stage, IndustryStage::Governance),
        },
        PlayerGameplayAction {
            action_id: ACTION_SCHEDULE_ASSEMBLER_FACTORY_CORE.to_string(),
            label: "Queue factory core run".to_string(),
            protocol_action: GAMEPLAY_ACTION_PROTOCOL.to_string(),
            target_agent_id: Some(agent_id.to_string()),
            disabled_reason: stage_gate_disabled_reason(industry_stage, IndustryStage::Governance),
        },
    ]);
}

impl ViewerRuntimeLiveServer {
    pub(super) fn handle_gameplay_action(
        &mut self,
        request: GameplayActionRequest,
    ) -> Result<GameplayActionAck, GameplayActionError> {
        self.ensure_gameplay_ready_for_action(
            "gameplay_action",
            Some(request.action_id.as_str()),
            Some(request.target_agent_id.as_str()),
        )
        .map_err(|(code, message)| GameplayActionError {
            code,
            message,
            action_id: Some(request.action_id.clone()),
            target_agent_id: Some(request.target_agent_id.clone()),
        })?;
        let verified = self.verify_gameplay_action_auth(&request)?;
        self.session_policy
            .validate_known_session_key(verified.player_id.as_str(), verified.public_key.as_str())
            .map_err(|message| GameplayActionError {
                code: map_session_policy_error_code(message.as_str()).to_string(),
                message,
                action_id: Some(request.action_id.clone()),
                target_agent_id: Some(request.target_agent_id.clone()),
            })?;
        self.llm_sidecar
            .consume_player_auth_nonce(verified.player_id.as_str(), verified.nonce)
            .map_err(|message| GameplayActionError {
                code: "auth_nonce_replay".to_string(),
                message,
                action_id: Some(request.action_id.clone()),
                target_agent_id: Some(request.target_agent_id.clone()),
            })?;

        let public_key = normalize_optional_public_key(request.public_key.as_deref());
        ensure_agent_player_access_runtime(
            &self.world,
            &self.llm_sidecar,
            request.target_agent_id.as_str(),
            verified.player_id.as_str(),
            public_key.as_deref(),
        )
        .map_err(|err| GameplayActionError {
            code: err.code,
            message: err.message,
            action_id: Some(request.action_id.clone()),
            target_agent_id: err.agent_id,
        })?;
        let events = self
            .llm_sidecar
            .bind_agent_player(
                request.target_agent_id.as_str(),
                verified.player_id.as_str(),
                public_key.as_deref(),
                false,
            )
            .map_err(|message| GameplayActionError {
                code: "player_bind_failed".to_string(),
                message,
                action_id: Some(request.action_id.clone()),
                target_agent_id: Some(request.target_agent_id.clone()),
            })?;
        for event in events {
            self.enqueue_virtual_event(event);
        }

        let runtime_action = runtime_action_from_request(&request)?;
        let runtime_action_id = self.world.submit_action(runtime_action);
        self.set_latest_player_gameplay_feedback(PlayerGameplayRecentFeedback {
            action: format!("gameplay_action:{}", request.action_id),
            stage: "accepted".to_string(),
            effect: format!(
                "queued industrial action {} for {} as runtime action {}",
                request.action_id, request.target_agent_id, runtime_action_id
            ),
            reason: None,
            hint: Some("advance 1-2 steps to apply the queued industrial action".to_string()),
            delta_logical_time: 0,
            delta_event_seq: 0,
        });

        Ok(GameplayActionAck {
            action_id: request.action_id,
            target_agent_id: request.target_agent_id,
            player_id: verified.player_id,
            runtime_action_id,
            accepted_at_tick: self.world.state().time,
            message: Some("advance 1-2 steps to apply the queued industrial action".to_string()),
        })
    }

    fn verify_gameplay_action_auth(
        &self,
        request: &GameplayActionRequest,
    ) -> Result<VerifiedPlayerAuth, GameplayActionError> {
        let Some(auth) = request.auth.as_ref() else {
            return Err(GameplayActionError {
                code: "auth_proof_required".to_string(),
                message: "gameplay_action requires auth proof".to_string(),
                action_id: Some(request.action_id.clone()),
                target_agent_id: Some(request.target_agent_id.clone()),
            });
        };
        verify_gameplay_action_auth_proof(request, auth).map_err(|message| GameplayActionError {
            code: map_auth_verify_error_code(message.as_str()).to_string(),
            message,
            action_id: Some(request.action_id.clone()),
            target_agent_id: Some(request.target_agent_id.clone()),
        })
    }
}

fn missing_materials_reason_with_world_fallback(
    agent_materials: &BTreeMap<String, i64>,
    world_materials: &BTreeMap<String, i64>,
    required: &[(&str, i64)],
) -> Option<String> {
    if has_required_materials(agent_materials, required)
        || has_required_materials(world_materials, required)
    {
        return None;
    }

    let details = required
        .iter()
        .map(|(kind, amount)| {
            let agent_current = material_balance(agent_materials, kind);
            let world_current = material_balance(world_materials, kind);
            format!("{kind}>={amount} (agent {agent_current}, world {world_current})")
        })
        .collect::<Vec<_>>();
    Some(format!("requires one ledger with {}", details.join(", ")))
}

fn has_required_materials(materials: &BTreeMap<String, i64>, required: &[(&str, i64)]) -> bool {
    required
        .iter()
        .all(|(kind, amount)| material_balance(materials, kind) >= *amount)
}

fn material_balance(materials: &BTreeMap<String, i64>, kind: &str) -> i64 {
    materials.get(kind).copied().unwrap_or_default()
}

fn stage_gate_disabled_reason(
    current_stage: IndustryStage,
    required_stage: IndustryStage,
) -> Option<String> {
    if industry_stage_rank(current_stage) >= industry_stage_rank(required_stage) {
        return None;
    }
    Some(format!(
        "requires industry stage {} (current: {})",
        industry_stage_label(required_stage),
        industry_stage_label(current_stage)
    ))
}

fn industry_stage_rank(stage: IndustryStage) -> u8 {
    match stage {
        IndustryStage::Bootstrap => 0,
        IndustryStage::ScaleOut => 1,
        IndustryStage::Governance => 2,
    }
}

fn industry_stage_label(stage: IndustryStage) -> &'static str {
    match stage {
        IndustryStage::Bootstrap => "bootstrap",
        IndustryStage::ScaleOut => "scale_out",
        IndustryStage::Governance => "governance",
    }
}

fn runtime_action_from_request(
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
