use super::*;

use super::super::auth::{verify_gameplay_action_auth_proof, VerifiedPlayerAuth};
use super::super::gameplay_actions::{
    build_runtime_action_from_gameplay_request, gameplay_action_requires_actor_agent,
    ACTION_BUILD_ASSEMBLER_MK1, ACTION_BUILD_SMELTER_MK1, ACTION_RELEASE_AGENT_CLAIM,
    ACTION_SCHEDULE_ASSEMBLER_CONTROL_CHIP, ACTION_SCHEDULE_ASSEMBLER_FACTORY_CORE,
    ACTION_SCHEDULE_ASSEMBLER_GEAR, ACTION_SCHEDULE_ASSEMBLER_LOGISTICS_DRONE,
    ACTION_SCHEDULE_ASSEMBLER_MODULE_RACK, ACTION_SCHEDULE_ASSEMBLER_MOTOR_MK1,
    ACTION_SCHEDULE_ASSEMBLER_SENSOR_PACK, ACTION_SCHEDULE_SMELTER_ALLOY_PLATE,
    ACTION_SCHEDULE_SMELTER_COPPER_WIRE, ACTION_SCHEDULE_SMELTER_IRON_INGOT,
    ACTION_SCHEDULE_SMELTER_POLYMER_RESIN, FACTORY_ASSEMBLER_MK1, FACTORY_SMELTER_MK1,
};
use super::super::protocol::{GameplayActionAck, GameplayActionError, GameplayActionRequest};
use super::control_plane::{
    ensure_agent_player_access_runtime, map_auth_verify_error_code, normalize_optional_public_key,
};
use crate::runtime::{IndustryStage, MaterialLedgerId, WorldState};
use crate::simulator::{PlayerGameplayAction, PlayerGameplayRecentFeedback};
use std::collections::BTreeMap;

const GAMEPLAY_ACTION_PROTOCOL: &str = "gameplay_action.submit";
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
        if gameplay_action_requires_actor_agent(request.action_id.as_str()) {
            let bound_agent_id = self
                .llm_sidecar
                .bound_agent_for_player(verified.player_id.as_str())
                .ok_or_else(|| GameplayActionError {
                    code: "player_agent_binding_required".to_string(),
                    message: format!(
                        "gameplay_action `{}` requires a bound player agent session",
                        request.action_id
                    ),
                    action_id: Some(request.action_id.clone()),
                    target_agent_id: Some(request.target_agent_id.clone()),
                })?;
            ensure_agent_player_access_runtime(
                &self.world,
                &self.llm_sidecar,
                bound_agent_id,
                verified.player_id.as_str(),
                public_key.as_deref(),
            )
            .map_err(|err| GameplayActionError {
                code: err.code,
                message: err.message,
                action_id: Some(request.action_id.clone()),
                target_agent_id: err.agent_id,
            })?;
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
            if actor_agent_id != bound_agent_id {
                return Err(GameplayActionError {
                    code: "actor_agent_mismatch".to_string(),
                    message: format!(
                        "gameplay_action `{}` actor_agent_id {} does not match bound player agent {}",
                        request.action_id, actor_agent_id, bound_agent_id
                    ),
                    action_id: Some(request.action_id.clone()),
                    target_agent_id: Some(request.target_agent_id.clone()),
                });
            }
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
        } else {
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
        }

        let accepted_at_tick = self.world.state().time;
        let chain_status_bind = self
            .config
            .chain_status_bind
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if let Some(chain_status_bind) = chain_status_bind {
            let _ = build_runtime_action_from_gameplay_request(&request)?;
            let submitted =
                chain_link::submit_chain_linked_gameplay_action(chain_status_bind, &request)?;
            let submitted_action_id = submitted
                .action_id
                .expect("chain gameplay submit must include action_id after ok=true validation");
            self.set_latest_player_gameplay_feedback(PlayerGameplayRecentFeedback {
                action: format!("gameplay_action:{}", request.action_id),
                stage: "submitted".to_string(),
                effect: format!(
                    "submitted gameplay action {} for {} to chain runtime as consensus action {}",
                    request.action_id, request.target_agent_id, submitted_action_id
                ),
                reason: None,
                hint: Some(
                    "wait for committed world sync to observe the gameplay action outcome"
                        .to_string(),
                ),
                delta_logical_time: 0,
                delta_event_seq: 0,
            });

            return Ok(GameplayActionAck {
                action_id: request.action_id,
                target_agent_id: request.target_agent_id,
                player_id: verified.player_id,
                runtime_action_id: submitted_action_id,
                accepted_at_tick,
                message: Some(
                    "submitted to chain runtime; wait for committed world sync to observe the gameplay action"
                        .to_string(),
                ),
            });
        }

        let runtime_action = build_runtime_action_from_gameplay_request(&request)?;
        let runtime_action_id = self.world.submit_action(runtime_action);
        self.set_latest_player_gameplay_feedback(PlayerGameplayRecentFeedback {
            action: format!("gameplay_action:{}", request.action_id),
            stage: "accepted".to_string(),
            effect: format!(
                "queued gameplay action {} for {} as runtime action {}",
                request.action_id, request.target_agent_id, runtime_action_id
            ),
            reason: None,
            hint: Some(match request.action_id.as_str() {
                ACTION_RELEASE_AGENT_CLAIM => {
                    "advance 1-2 steps to queue the release cooldown for this claim".to_string()
                }
                _ => "advance 1-2 steps to apply the queued gameplay action".to_string(),
            }),
            delta_logical_time: 0,
            delta_event_seq: 0,
        });

        Ok(GameplayActionAck {
            action_id: request.action_id,
            target_agent_id: request.target_agent_id,
            player_id: verified.player_id,
            runtime_action_id,
            accepted_at_tick,
            message: Some("advance 1-2 steps to apply the queued gameplay action".to_string()),
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
