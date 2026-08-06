use super::control_plane::{
    ensure_agent_player_access_runtime, map_auth_verify_error_code, normalize_optional_public_key,
};
use super::*;
use crate::simulator::{ResourceOwner, WorldKernel};
use crate::viewer::{
    GameplayActionError, ScheduleRecipeQuotePreflight, ScheduleRecipeQuoteRequest,
    VerifiedPlayerAuth, verify_schedule_recipe_quote_auth_proof,
};

impl ViewerRuntimeLiveServer {
    /// Computes the canonical ScheduleRecipe quote for the authenticated player's bound Agent.
    /// This is a read-only mapping: it neither consumes the auth proof nor schedules production.
    pub(super) fn handle_schedule_recipe_quote(
        &mut self,
        request: ScheduleRecipeQuoteRequest,
    ) -> Result<ScheduleRecipeQuotePreflight, GameplayActionError> {
        let verified = self.verify_schedule_recipe_quote_auth(&request)?;
        self.session_policy
            .validate_known_session_key(verified.player_id.as_str(), verified.public_key.as_str())
            .map_err(|message| GameplayActionError {
                code: map_session_policy_error_code(message.as_str()).to_string(),
                message,
                action_id: Some("quote_schedule_recipe".to_string()),
                target_agent_id: None,
            })?;
        let agent_id = self
            .llm_sidecar
            .bound_agent_for_player(verified.player_id.as_str())
            .ok_or_else(|| GameplayActionError {
                code: "player_agent_binding_required".to_string(),
                message: "quote_schedule_recipe requires a bound player Agent session".to_string(),
                action_id: Some("quote_schedule_recipe".to_string()),
                target_agent_id: None,
            })?;
        let public_key = normalize_optional_public_key(request.public_key.as_deref());
        ensure_agent_player_access_runtime(
            &self.world,
            &self.llm_sidecar,
            agent_id,
            verified.player_id.as_str(),
            public_key.as_deref(),
        )
        .map_err(|err| GameplayActionError {
            code: err.code,
            message: err.message,
            action_id: Some("quote_schedule_recipe".to_string()),
            target_agent_id: err.agent_id,
        })?;

        let model = mapping::runtime_state_to_simulator_model(
            self.world.state(),
            &self.llm_sidecar,
            self.seed_model.as_ref(),
        );
        let quote = WorldKernel::with_model(self.snapshot_config.clone(), model)
            .quote_schedule_recipe(
                &ResourceOwner::Agent {
                    agent_id: agent_id.to_string(),
                },
                request.factory_id.as_str(),
                request.recipe_id.as_str(),
                request.batches,
            )
            .map_err(|reason| GameplayActionError {
                code: "schedule_recipe_quote_rejected".to_string(),
                message: format!("quote_schedule_recipe rejected: {reason:?}"),
                action_id: Some("quote_schedule_recipe".to_string()),
                target_agent_id: Some(agent_id.to_string()),
            })?;
        Ok(ScheduleRecipeQuotePreflight {
            owner_agent_id: agent_id.to_string(),
            factory_id: quote.factory_id,
            recipe_id: quote.recipe_id,
            batches: quote.batches,
            base_duration_ticks: quote.base_duration_ticks,
            electricity_cost: quote.electricity_cost,
            electricity_after: quote.electricity_after,
            hardware_cost: quote.hardware_cost,
            data_output: quote.data_output,
            finished_product_id: quote.finished_product_id,
            finished_product_units: quote.finished_product_units,
            local_shortage_delay_ticks: quote.local_shortage_delay_ticks,
            shortage_reason: quote.shortage_reason,
            recommended_pre_step: quote.recommended_pre_step,
            runway_before_ticks: quote.runway_before_ticks,
            runway_after_ticks: quote.runway_after_ticks,
            downtime_threshold_ppm: quote.downtime_threshold_ppm,
            continue_production_risk: quote.continue_production_risk,
            maintenance_pressure_delta: quote.maintenance_pressure_delta,
            recommended_maintenance_action: quote.recommended_maintenance_action,
        })
    }

    fn verify_schedule_recipe_quote_auth(
        &self,
        request: &ScheduleRecipeQuoteRequest,
    ) -> Result<VerifiedPlayerAuth, GameplayActionError> {
        let auth = request.auth.as_ref().ok_or_else(|| GameplayActionError {
            code: "auth_proof_required".to_string(),
            message: "quote_schedule_recipe requires auth proof".to_string(),
            action_id: Some("quote_schedule_recipe".to_string()),
            target_agent_id: None,
        })?;
        verify_schedule_recipe_quote_auth_proof(request, auth).map_err(|message| {
            GameplayActionError {
                code: map_auth_verify_error_code(message.as_str()).to_string(),
                message,
                action_id: Some("quote_schedule_recipe".to_string()),
                target_agent_id: None,
            }
        })
    }
}
