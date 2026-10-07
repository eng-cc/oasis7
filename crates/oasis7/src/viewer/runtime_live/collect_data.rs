use super::super::auth::{VerifiedPlayerAuth, verify_collect_data_auth_proof};
use super::super::protocol::{CollectDataCommand, GameplayActionAck, GameplayActionError};
use super::control_plane::{
    ensure_agent_player_access_runtime, map_auth_verify_error_code, normalize_optional_public_key,
};
use super::player_gameplay::{CollectDataResult, data_collection_preflight};
use super::*;
use crate::runtime::Action as RuntimeAction;
use crate::simulator::ResourceKind;

impl ViewerRuntimeLiveServer {
    pub(super) fn handle_collect_data_protocol_request(
        &mut self,
        command: CollectDataCommand,
        session: &mut RuntimeLiveSession,
        writer: &mut BufWriter<TcpStream>,
    ) -> Result<(), ViewerRuntimeLiveServerError> {
        match self.handle_collect_data(command) {
            Ok(CollectDataResult::Preflight(quote)) => {
                send_response(writer, &ViewerResponse::CollectDataPreflight { quote })?;
            }
            Ok(CollectDataResult::Submit(ack)) => {
                let ack_player_id = ack.player_id.clone();
                send_response(writer, &ViewerResponse::GameplayActionAck { ack })?;
                if !ack_player_id.trim().is_empty() {
                    session.current_player_id = Some(ack_player_id);
                }
                if session.explicitly_subscribed_to(ViewerStream::Snapshot) {
                    let snapshot = self.compat_snapshot(session.current_player_id.as_deref());
                    send_response(writer, &ViewerResponse::Snapshot { snapshot })?;
                }
            }
            Err(error) => {
                self.record_gameplay_action_rejection(&error);
                send_response(writer, &ViewerResponse::GameplayActionError { error })?;
                if session.explicitly_subscribed_to(ViewerStream::Snapshot) {
                    let snapshot = self.compat_snapshot(session.current_player_id.as_deref());
                    send_response(writer, &ViewerResponse::Snapshot { snapshot })?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn handle_collect_data(
        &mut self,
        command: CollectDataCommand,
    ) -> Result<CollectDataResult, GameplayActionError> {
        if self.config.world_service.is_some()
            && let CollectDataCommand::Submit { request } = &command
        {
            let auth = request.auth.as_ref().ok_or_else(|| GameplayActionError {
                code: "auth_proof_required".into(),
                message: "collect_data requires auth proof".into(),
                action_id: Some("collect_data".into()),
                target_agent_id: None,
            })?;
            let verified = verify_collect_data_auth_proof(&command, auth).map_err(|message| {
                GameplayActionError {
                    code: map_auth_verify_error_code(&message).into(),
                    message,
                    action_id: Some("collect_data".into()),
                    target_agent_id: None,
                }
            })?;
            self.session_policy
                .validate_known_session_key(&verified.player_id, &verified.public_key)
                .map_err(|message| GameplayActionError {
                    code: map_session_policy_error_code(&message).into(),
                    message,
                    action_id: Some("collect_data".into()),
                    target_agent_id: None,
                })?;
            let runtime_action_id = self.submit_world_service_gameplay(&command)?;
            if runtime_action_id != 0 {
                self.runtime_action_players
                    .insert(runtime_action_id, verified.player_id.clone());
            }
            return Ok(CollectDataResult::Submit(GameplayActionAck {
                action_id: "collect_data".into(),
                target_agent_id: self
                    .llm_sidecar
                    .bound_agent_for_player(&verified.player_id)
                    .unwrap_or_default()
                    .into(),
                player_id: verified.player_id,
                runtime_action_id,
                accepted_at_tick: self.world.state().time,
                message: Some("received by world service; await committed outcome".into()),
            }));
        }
        let (verified, collector_agent_id) = self.authorize_collect_data(&command)?;
        let request = match &command {
            CollectDataCommand::Preflight { request } | CollectDataCommand::Submit { request } => {
                request
            }
        };
        let available_electricity = self
            .world
            .state()
            .agents
            .get(collector_agent_id.as_str())
            .map(|agent| agent.state.resources.get(ResourceKind::Electricity))
            .ok_or_else(|| GameplayActionError {
                code: "collector_agent_missing".to_string(),
                message: format!("bound collector Agent {collector_agent_id} is not in the world"),
                action_id: Some("collect_data".to_string()),
                target_agent_id: Some(collector_agent_id.clone()),
            })?;
        let quote = data_collection_preflight(
            collector_agent_id.clone(),
            available_electricity,
            request.electricity_cost,
            request.data_amount,
        );
        match &command {
            CollectDataCommand::Preflight { .. } => Ok(CollectDataResult::Preflight(quote)),
            CollectDataCommand::Submit { .. } => {
                if !quote.can_execute {
                    return Err(GameplayActionError {
                        code: "collect_data_preflight_blocked".to_string(),
                        message: quote
                            .blocked_reason
                            .clone()
                            .unwrap_or_else(|| "data collection is blocked".to_string()),
                        action_id: Some("collect_data".to_string()),
                        target_agent_id: Some(collector_agent_id),
                    });
                }
                if self.config.world_service.is_some() {
                    let runtime_action_id = self.submit_world_service_gameplay(&command)?;
                    if runtime_action_id != 0 {
                        self.runtime_action_players
                            .insert(runtime_action_id, verified.player_id.clone());
                    }
                    return Ok(CollectDataResult::Submit(GameplayActionAck {
                        action_id: "collect_data".into(),
                        target_agent_id: collector_agent_id,
                        player_id: verified.player_id,
                        runtime_action_id,
                        accepted_at_tick: self.world.state().time,
                        message: Some("received by world service; await committed outcome".into()),
                    }));
                }
                if let Some(chain_status_bind) = self
                    .config
                    .chain_status_bind
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    let chain_submit_bind = self
                        .config
                        .chain_submit_bind
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .unwrap_or(chain_status_bind)
                        .to_string();
                    let submitted = chain_link::submit_chain_linked_collect_data(
                        chain_submit_bind.as_str(),
                        &command,
                    )?;
                    let runtime_action_id = submitted.action_id.expect(
                        "chain collect_data submit must include action_id after ok=true validation",
                    );
                    self.runtime_action_players
                        .insert(runtime_action_id, verified.player_id.clone());
                    return Ok(CollectDataResult::Submit(GameplayActionAck {
                        action_id: "collect_data".to_string(),
                        target_agent_id: collector_agent_id,
                        player_id: verified.player_id,
                        runtime_action_id,
                        accepted_at_tick: self.world.state().time,
                        message: Some(
                            "submitted data collection to chain runtime; wait for committed world sync"
                                .to_string(),
                        ),
                    }));
                }
                let runtime_action_id = self.world.submit_action(RuntimeAction::CollectData {
                    collector_agent_id: collector_agent_id.clone(),
                    electricity_cost: request.electricity_cost,
                    data_amount: request.data_amount,
                });
                self.runtime_action_players
                    .insert(runtime_action_id, verified.player_id.clone());
                Ok(CollectDataResult::Submit(GameplayActionAck {
                    action_id: "collect_data".to_string(),
                    target_agent_id: collector_agent_id,
                    player_id: verified.player_id,
                    runtime_action_id,
                    accepted_at_tick: self.world.state().time,
                    message: Some(
                        "advance 1-2 steps to apply the queued data collection".to_string(),
                    ),
                }))
            }
        }
    }

    fn authorize_collect_data(
        &mut self,
        command: &CollectDataCommand,
    ) -> Result<(VerifiedPlayerAuth, String), GameplayActionError> {
        self.ensure_gameplay_ready_for_action("collect_data", Some("collect_data"), None)
            .map_err(|(code, message)| GameplayActionError {
                code,
                message,
                action_id: Some("collect_data".to_string()),
                target_agent_id: None,
            })?;
        let request = match command {
            CollectDataCommand::Preflight { request } | CollectDataCommand::Submit { request } => {
                request
            }
        };
        let auth = request.auth.as_ref().ok_or_else(|| GameplayActionError {
            code: "auth_proof_required".to_string(),
            message: "collect_data requires auth proof".to_string(),
            action_id: Some("collect_data".to_string()),
            target_agent_id: None,
        })?;
        let verified = verify_collect_data_auth_proof(command, auth).map_err(|message| {
            GameplayActionError {
                code: map_auth_verify_error_code(message.as_str()).to_string(),
                message,
                action_id: Some("collect_data".to_string()),
                target_agent_id: None,
            }
        })?;
        self.session_policy
            .validate_known_session_key(verified.player_id.as_str(), verified.public_key.as_str())
            .map_err(|message| GameplayActionError {
                code: map_session_policy_error_code(message.as_str()).to_string(),
                message,
                action_id: Some("collect_data".to_string()),
                target_agent_id: None,
            })?;
        self.llm_sidecar
            .consume_player_auth_nonce(verified.player_id.as_str(), verified.nonce)
            .map_err(|message| GameplayActionError {
                code: "auth_nonce_replay".to_string(),
                message,
                action_id: Some("collect_data".to_string()),
                target_agent_id: None,
            })?;
        let collector_agent_id = self
            .llm_sidecar
            .bound_agent_for_player(verified.player_id.as_str())
            .ok_or_else(|| GameplayActionError {
                code: "player_agent_binding_required".to_string(),
                message: "collect_data requires a bound player Agent session".to_string(),
                action_id: Some("collect_data".to_string()),
                target_agent_id: None,
            })?
            .to_string();
        let public_key = normalize_optional_public_key(request.public_key.as_deref());
        ensure_agent_player_access_runtime(
            &self.world,
            &self.llm_sidecar,
            collector_agent_id.as_str(),
            verified.player_id.as_str(),
            public_key.as_deref(),
        )
        .map_err(|err| GameplayActionError {
            code: err.code,
            message: err.message,
            action_id: Some("collect_data".to_string()),
            target_agent_id: err.agent_id,
        })?;
        Ok((verified, collector_agent_id))
    }
}
