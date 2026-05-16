use super::*;

use super::super::auth::{
    verify_agent_chat_auth_proof, verify_hosted_prompt_control_apply_strong_auth_grant,
    verify_hosted_prompt_control_rollback_strong_auth_grant,
    verify_prompt_control_apply_auth_proof, verify_prompt_control_rollback_auth_proof,
    PromptControlAuthIntent, VerifiedPlayerAuth,
};
use super::super::protocol::{
    AgentChatAck, AgentChatError, AgentChatRequest, PromptControlAck, PromptControlApplyRequest,
    PromptControlCommand, PromptControlError, PromptControlOperation, PromptControlRollbackRequest,
};
use crate::runtime::World as RuntimeWorld;
use crate::simulator::{
    AgentDecision, AgentDecisionTrace, AgentPromptProfile, PromptUpdateOperation, WorldEventKind,
};
use sha2::{Digest, Sha256};

mod llm_sidecar;
pub(super) use llm_sidecar::{
    simulator_action_label, simulator_action_to_runtime, RuntimeLlmSidecar,
};

const RUNTIME_AGENT_CHAT_ECHO_ENV: &str = "OASIS7_RUNTIME_AGENT_CHAT_ECHO";
const RUNTIME_AGENT_CHAT_ECHO_PREFIX: &str = "[qa-echo]";
const HOSTED_STRONG_AUTH_GRANT_PUBLIC_KEY_ENV: &str = "OASIS7_HOSTED_STRONG_AUTH_PUBLIC_KEY";

#[allow(dead_code)]
pub(in crate::viewer::runtime_live) fn runtime_provider_settings_from_env(
) -> Result<Option<llm_sidecar::ProviderDecisionSettings>, String> {
    llm_sidecar::provider_settings_from_env()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ResolvedAgentChatIntent {
    intent_tick: Option<u64>,
    intent_seq: u64,
}

pub(super) fn runtime_agent_chat_echo_enabled_from_env() -> bool {
    std::env::var(RUNTIME_AGENT_CHAT_ECHO_ENV)
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn hosted_strong_auth_grant_public_key_from_env() -> Result<String, String> {
    std::env::var(HOSTED_STRONG_AUTH_GRANT_PUBLIC_KEY_ENV)
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "hosted strong auth backend grant signer is not configured on this runtime".to_string()
        })
}

fn hosted_strong_auth_now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

impl ViewerRuntimeLiveServer {
    pub(super) fn handle_prompt_control(
        &mut self,
        command: PromptControlCommand,
    ) -> Result<PromptControlAck, PromptControlError> {
        if self.hosted_public_join_mode() {
            match &command {
                PromptControlCommand::Preview { request } => {
                    self.verify_hosted_prompt_control_apply_strong_auth(
                        PromptControlAuthIntent::Preview,
                        request,
                    )?;
                }
                PromptControlCommand::Apply { request } => {
                    self.verify_hosted_prompt_control_apply_strong_auth(
                        PromptControlAuthIntent::Apply,
                        request,
                    )?;
                }
                PromptControlCommand::Rollback { request } => {
                    self.verify_hosted_prompt_control_rollback_strong_auth(request)?;
                }
            }
        }
        if !self.llm_sidecar.is_llm_mode() {
            let (agent_id, message) = match command {
                PromptControlCommand::Preview { request }
                | PromptControlCommand::Apply { request } => (
                    request.agent_id,
                    "prompt_control requires runtime live server running with --llm".to_string(),
                ),
                PromptControlCommand::Rollback { request } => (
                    request.agent_id,
                    "prompt_control rollback requires runtime live server running with --llm"
                        .to_string(),
                ),
            };
            return Err(PromptControlError {
                code: "llm_mode_required".to_string(),
                message,
                agent_id: Some(agent_id.clone()),
                current_version: self.current_prompt_version(agent_id.as_str()),
            });
        }
        if !self.llm_sidecar.supports_prompt_control() {
            let (agent_id, current_version) = match &command {
                PromptControlCommand::Preview { request }
                | PromptControlCommand::Apply { request } => (
                    request.agent_id.clone(),
                    self.current_prompt_version(request.agent_id.as_str()),
                ),
                PromptControlCommand::Rollback { request } => (
                    request.agent_id.clone(),
                    self.current_prompt_version(request.agent_id.as_str()),
                ),
            };
            return Err(PromptControlError {
                code: "agent_provider_prompt_control_unsupported".to_string(),
                message:
                    "prompt_control is not yet supported when runtime live uses ProviderBacked(Local HTTP)"
                        .to_string(),
                agent_id: Some(agent_id),
                current_version,
            });
        }

        match command {
            PromptControlCommand::Preview { request } => self.prompt_control_preview(request),
            PromptControlCommand::Apply { request } => self.prompt_control_apply(request),
            PromptControlCommand::Rollback { request } => self.prompt_control_rollback(request),
        }
    }

    fn prompt_control_preview(
        &mut self,
        request: PromptControlApplyRequest,
    ) -> Result<PromptControlAck, PromptControlError> {
        let player_id =
            normalize_required_player_id(request.player_id.as_str(), request.agent_id.as_str())?;
        let public_key = normalize_optional_public_key(request.public_key.as_deref());
        self.verify_and_consume_prompt_control_apply_auth(
            PromptControlAuthIntent::Preview,
            &request,
        )?;
        ensure_agent_player_access_runtime(
            &self.world,
            &self.llm_sidecar,
            request.agent_id.as_str(),
            player_id.as_str(),
            public_key.as_deref(),
        )?;
        let current = self.current_prompt_profile(request.agent_id.as_str())?;
        ensure_expected_prompt_version_runtime(
            request.agent_id.as_str(),
            current.version,
            request.expected_version,
        )?;

        let mut candidate = current.clone();
        apply_prompt_patch_runtime(&mut candidate, &request);
        let applied_fields = changed_prompt_fields_runtime(&current, &candidate);
        let preview_version = if applied_fields.is_empty() {
            current.version
        } else {
            current.version.saturating_add(1)
        };

        Ok(PromptControlAck {
            agent_id: request.agent_id,
            operation: PromptControlOperation::Apply,
            preview: true,
            version: preview_version,
            updated_at_tick: self.world.state().time,
            applied_fields,
            digest: prompt_profile_digest_runtime(&candidate),
            rolled_back_to_version: None,
        })
    }

    fn prompt_control_apply(
        &mut self,
        request: PromptControlApplyRequest,
    ) -> Result<PromptControlAck, PromptControlError> {
        let player_id =
            normalize_required_player_id(request.player_id.as_str(), request.agent_id.as_str())?;
        let public_key = normalize_optional_public_key(request.public_key.as_deref());
        self.verify_and_consume_prompt_control_apply_auth(
            PromptControlAuthIntent::Apply,
            &request,
        )?;
        ensure_agent_player_access_runtime(
            &self.world,
            &self.llm_sidecar,
            request.agent_id.as_str(),
            player_id.as_str(),
            public_key.as_deref(),
        )?;
        let current = self.current_prompt_profile(request.agent_id.as_str())?;
        ensure_expected_prompt_version_runtime(
            request.agent_id.as_str(),
            current.version,
            request.expected_version,
        )?;
        ensure_updated_by_matches_player_runtime(
            request.updated_by.as_deref(),
            player_id.as_str(),
            request.agent_id.as_str(),
        )?;

        let mut candidate = current.clone();
        apply_prompt_patch_runtime(&mut candidate, &request);
        let applied_fields = changed_prompt_fields_runtime(&current, &candidate);
        let digest = prompt_profile_digest_runtime(&candidate);
        if applied_fields.is_empty() {
            return Ok(PromptControlAck {
                agent_id: request.agent_id,
                operation: PromptControlOperation::Apply,
                preview: false,
                version: current.version,
                updated_at_tick: current.updated_at_tick,
                applied_fields,
                digest,
                rolled_back_to_version: None,
            });
        }

        candidate.version = current.version.saturating_add(1);
        candidate.updated_at_tick = self.world.state().time;
        candidate.updated_by = player_id.clone();
        self.llm_sidecar.upsert_prompt_profile(candidate.clone());
        self.llm_sidecar.apply_prompt_profile_to_driver(&candidate);
        self.bind_agent_player_access(
            request.agent_id.as_str(),
            player_id.as_str(),
            public_key.as_deref(),
        )?;
        let digest = prompt_profile_digest_runtime(&candidate);
        self.enqueue_virtual_event(WorldEventKind::AgentPromptUpdated {
            profile: candidate.clone(),
            operation: PromptUpdateOperation::Apply,
            applied_fields: applied_fields.clone(),
            digest: digest.clone(),
            rolled_back_to_version: None,
        });
        self.llm_sidecar.request_decision();
        self.set_latest_player_gameplay_feedback(PlayerGameplayRecentFeedback {
            action: "prompt_control.apply".to_string(),
            stage: "completed_advanced".to_string(),
            effect: format!(
                "updated prompt guidance for {} to version {}",
                request.agent_id, candidate.version
            ),
            intent_summary: Some(format!("apply updated prompt guidance for {}", request.agent_id)),
            target_agent_id: Some(request.agent_id.clone()),
            reason: None,
            hint: Some(
                "continue the world and watch whether the new prompt guidance changes the agent's next decision"
                    .to_string(),
            ),
            delta_logical_time: 0,
            delta_event_seq: 0,
        });

        Ok(PromptControlAck {
            agent_id: request.agent_id,
            operation: PromptControlOperation::Apply,
            preview: false,
            version: candidate.version,
            updated_at_tick: candidate.updated_at_tick,
            applied_fields,
            digest,
            rolled_back_to_version: None,
        })
    }

    fn prompt_control_rollback(
        &mut self,
        request: PromptControlRollbackRequest,
    ) -> Result<PromptControlAck, PromptControlError> {
        let player_id =
            normalize_required_player_id(request.player_id.as_str(), request.agent_id.as_str())?;
        let public_key = normalize_optional_public_key(request.public_key.as_deref());
        self.verify_and_consume_prompt_control_rollback_auth(&request)?;
        ensure_agent_player_access_runtime(
            &self.world,
            &self.llm_sidecar,
            request.agent_id.as_str(),
            player_id.as_str(),
            public_key.as_deref(),
        )?;
        let current = self.current_prompt_profile(request.agent_id.as_str())?;
        ensure_expected_prompt_version_runtime(
            request.agent_id.as_str(),
            current.version,
            request.expected_version,
        )?;
        ensure_updated_by_matches_player_runtime(
            request.updated_by.as_deref(),
            player_id.as_str(),
            request.agent_id.as_str(),
        )?;

        let target = if request.to_version == 0 {
            AgentPromptProfile::for_agent(request.agent_id.clone())
        } else {
            self.lookup_prompt_profile_version(request.agent_id.as_str(), request.to_version)
                .ok_or_else(|| PromptControlError {
                    code: "target_version_not_found".to_string(),
                    message: format!(
                        "prompt profile version {} not found for {}",
                        request.to_version, request.agent_id
                    ),
                    agent_id: Some(request.agent_id.clone()),
                    current_version: Some(current.version),
                })?
        };

        let mut candidate = current.clone();
        candidate.system_prompt_override = target.system_prompt_override;
        candidate.short_term_goal_override = target.short_term_goal_override;
        candidate.long_term_goal_override = target.long_term_goal_override;
        let applied_fields = changed_prompt_fields_runtime(&current, &candidate);
        if applied_fields.is_empty() {
            return Err(PromptControlError {
                code: "rollback_noop".to_string(),
                message: format!(
                    "rollback target version {} yields no prompt changes for {}",
                    request.to_version, request.agent_id
                ),
                agent_id: Some(request.agent_id),
                current_version: Some(current.version),
            });
        }

        candidate.version = current.version.saturating_add(1);
        candidate.updated_at_tick = self.world.state().time;
        candidate.updated_by = player_id.clone();
        self.llm_sidecar.upsert_prompt_profile(candidate.clone());
        self.llm_sidecar.apply_prompt_profile_to_driver(&candidate);
        self.bind_agent_player_access(
            request.agent_id.as_str(),
            player_id.as_str(),
            public_key.as_deref(),
        )?;
        let digest = prompt_profile_digest_runtime(&candidate);
        self.enqueue_virtual_event(WorldEventKind::AgentPromptUpdated {
            profile: candidate.clone(),
            operation: PromptUpdateOperation::Rollback,
            applied_fields: applied_fields.clone(),
            digest: digest.clone(),
            rolled_back_to_version: Some(request.to_version),
        });
        self.llm_sidecar.request_decision();
        self.set_latest_player_gameplay_feedback(PlayerGameplayRecentFeedback {
            action: "prompt_control.rollback".to_string(),
            stage: "completed_advanced".to_string(),
            effect: format!(
                "rolled back prompt guidance for {} to base version {} via version {}",
                request.agent_id, request.to_version, candidate.version
            ),
            intent_summary: Some(format!(
                "roll back prompt guidance for {}",
                request.agent_id
            )),
            target_agent_id: Some(request.agent_id.clone()),
            reason: None,
            hint: Some(
                "continue the world and confirm the agent now follows the restored guidance"
                    .to_string(),
            ),
            delta_logical_time: 0,
            delta_event_seq: 0,
        });

        Ok(PromptControlAck {
            agent_id: request.agent_id,
            operation: PromptControlOperation::Rollback,
            preview: false,
            version: candidate.version,
            updated_at_tick: candidate.updated_at_tick,
            applied_fields,
            digest,
            rolled_back_to_version: Some(request.to_version),
        })
    }

    pub(super) fn handle_agent_chat(
        &mut self,
        request: AgentChatRequest,
    ) -> Result<AgentChatAck, AgentChatError> {
        let agent_id = request.agent_id.clone();
        if !self.llm_sidecar.is_llm_mode() {
            return Err(AgentChatError {
                code: "llm_mode_required".to_string(),
                message: "agent chat requires runtime live server running with --llm".to_string(),
                agent_id: Some(agent_id),
            });
        }
        if !self.llm_sidecar.supports_agent_chat() {
            return Err(AgentChatError {
                code: "agent_provider_chat_unsupported".to_string(),
                message:
                    "agent chat is not yet supported when runtime live uses ProviderBacked(Local HTTP)"
                        .to_string(),
                agent_id: Some(agent_id),
            });
        }

        let player_id = request
            .player_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let Some(player_id) = player_id else {
            return Err(AgentChatError {
                code: "player_id_required".to_string(),
                message: "agent_chat requires non-empty player_id".to_string(),
                agent_id: Some(agent_id),
            });
        };
        let public_key = normalize_optional_public_key(request.public_key.as_deref());
        let message = request.message.trim().to_string();
        if message.is_empty() {
            return Err(AgentChatError {
                code: "empty_message".to_string(),
                message: "chat message cannot be empty".to_string(),
                agent_id: Some(agent_id),
            });
        }
        let verified = self.verify_agent_chat_auth(&request)?;
        self.session_policy
            .validate_known_session_key(verified.player_id.as_str(), verified.public_key.as_str())
            .map_err(|message| AgentChatError {
                code: map_session_policy_error_code(message.as_str()).to_string(),
                message,
                agent_id: Some(agent_id.clone()),
            })?;
        let intent = resolve_agent_chat_intent(&request, verified.nonce).map_err(|message| {
            AgentChatError {
                code: "intent_seq_invalid".to_string(),
                message,
                agent_id: Some(agent_id.clone()),
            }
        })?;
        if let Some(replay_ack) = self
            .llm_sidecar
            .find_chat_intent_replay(
                verified.player_id.as_str(),
                agent_id.as_str(),
                intent.intent_seq,
                intent.intent_tick,
                message.as_str(),
                public_key.as_deref(),
            )
            .map_err(|message| AgentChatError {
                code: "intent_seq_conflict".to_string(),
                message,
                agent_id: Some(agent_id.clone()),
            })?
        {
            return Ok(replay_ack);
        }
        self.llm_sidecar
            .consume_player_auth_nonce(verified.player_id.as_str(), verified.nonce)
            .map_err(|message| AgentChatError {
                code: "auth_nonce_replay".to_string(),
                message,
                agent_id: Some(agent_id.clone()),
            })?;
        self.bind_agent_player_access_for_chat(
            agent_id.as_str(),
            player_id.as_str(),
            public_key.as_deref(),
        )?;
        let chat_echo_enabled = runtime_agent_chat_echo_enabled_from_env();
        match self.llm_sidecar.push_chat_message(
            &self.world,
            &self.snapshot_config,
            agent_id.as_str(),
            message.as_str(),
        ) {
            Ok(()) => {
                self.llm_sidecar.request_decision();
            }
            Err(error) if chat_echo_enabled && error.code == "llm_init_failed" => {}
            Err(error) => return Err(error),
        }
        self.enqueue_agent_chat_echo_event_if_enabled(agent_id.as_str(), message.as_str());
        self.set_latest_player_gameplay_feedback(PlayerGameplayRecentFeedback {
            action: "agent_chat".to_string(),
            stage: "accepted".to_string(),
            effect: format!("queued direct agent instruction for {}", agent_id),
            intent_summary: Some(format!("send direct instruction to {}", agent_id)),
            target_agent_id: Some(agent_id.clone()),
            reason: None,
            hint: Some(
                "continue the world and watch for the agent's reply or the next visible world consequence"
                    .to_string(),
            ),
            delta_logical_time: 0,
            delta_event_seq: 0,
        });
        let ack = AgentChatAck {
            agent_id: agent_id.clone(),
            accepted_at_tick: self.world.state().time,
            message_len: message.chars().count(),
            player_id: Some(player_id),
            intent_tick: intent.intent_tick,
            intent_seq: Some(intent.intent_seq),
            idempotent_replay: false,
        };
        self.llm_sidecar.record_chat_intent_ack(
            verified.player_id.as_str(),
            agent_id.as_str(),
            intent.intent_seq,
            intent.intent_tick,
            message.as_str(),
            public_key.as_deref(),
            &ack,
        );
        Ok(ack)
    }

    fn verify_and_consume_prompt_control_apply_auth(
        &mut self,
        intent: PromptControlAuthIntent,
        request: &PromptControlApplyRequest,
    ) -> Result<(), PromptControlError> {
        let Some(auth) = request.auth.as_ref() else {
            return Err(PromptControlError {
                code: "auth_proof_required".to_string(),
                message: "prompt_control requires auth proof".to_string(),
                agent_id: Some(request.agent_id.clone()),
                current_version: self.current_prompt_version(request.agent_id.as_str()),
            });
        };
        let verified =
            verify_prompt_control_apply_auth_proof(intent, request, auth).map_err(|message| {
                PromptControlError {
                    code: map_auth_verify_error_code(message.as_str()).to_string(),
                    message,
                    agent_id: Some(request.agent_id.clone()),
                    current_version: self.current_prompt_version(request.agent_id.as_str()),
                }
            })?;
        self.session_policy
            .validate_known_session_key(verified.player_id.as_str(), verified.public_key.as_str())
            .map_err(|message| PromptControlError {
                code: map_session_policy_error_code(message.as_str()).to_string(),
                message,
                agent_id: Some(request.agent_id.clone()),
                current_version: self.current_prompt_version(request.agent_id.as_str()),
            })?;
        self.llm_sidecar
            .consume_player_auth_nonce(verified.player_id.as_str(), verified.nonce)
            .map_err(|message| PromptControlError {
                code: "auth_nonce_replay".to_string(),
                message,
                agent_id: Some(request.agent_id.clone()),
                current_version: self.current_prompt_version(request.agent_id.as_str()),
            })?;
        Ok(())
    }

    fn verify_hosted_prompt_control_apply_strong_auth(
        &self,
        intent: PromptControlAuthIntent,
        request: &PromptControlApplyRequest,
    ) -> Result<(), PromptControlError> {
        let Some(grant) = request.strong_auth_grant.as_ref() else {
            return Err(self.hosted_prompt_control_strong_auth_error(
                "strong_auth_required",
                request.agent_id.as_str(),
                "prompt_control requires hosted strong auth grant on hosted_public_join",
            ));
        };
        let signer_public_key =
            hosted_strong_auth_grant_public_key_from_env().map_err(|message| {
                self.hosted_prompt_control_strong_auth_error(
                    "strong_auth_required",
                    request.agent_id.as_str(),
                    message.as_str(),
                )
            })?;
        verify_hosted_prompt_control_apply_strong_auth_grant(
            intent,
            request,
            grant,
            signer_public_key.as_str(),
            hosted_strong_auth_now_unix_ms(),
        )
        .map_err(|message| {
            self.hosted_prompt_control_strong_auth_error(
                "strong_auth_grant_invalid",
                request.agent_id.as_str(),
                message.as_str(),
            )
        })
    }

    fn verify_and_consume_prompt_control_rollback_auth(
        &mut self,
        request: &PromptControlRollbackRequest,
    ) -> Result<(), PromptControlError> {
        let Some(auth) = request.auth.as_ref() else {
            return Err(PromptControlError {
                code: "auth_proof_required".to_string(),
                message: "prompt_control rollback requires auth proof".to_string(),
                agent_id: Some(request.agent_id.clone()),
                current_version: self.current_prompt_version(request.agent_id.as_str()),
            });
        };
        let verified =
            verify_prompt_control_rollback_auth_proof(request, auth).map_err(|message| {
                PromptControlError {
                    code: map_auth_verify_error_code(message.as_str()).to_string(),
                    message,
                    agent_id: Some(request.agent_id.clone()),
                    current_version: self.current_prompt_version(request.agent_id.as_str()),
                }
            })?;
        self.session_policy
            .validate_known_session_key(verified.player_id.as_str(), verified.public_key.as_str())
            .map_err(|message| PromptControlError {
                code: map_session_policy_error_code(message.as_str()).to_string(),
                message,
                agent_id: Some(request.agent_id.clone()),
                current_version: self.current_prompt_version(request.agent_id.as_str()),
            })?;
        self.llm_sidecar
            .consume_player_auth_nonce(verified.player_id.as_str(), verified.nonce)
            .map_err(|message| PromptControlError {
                code: "auth_nonce_replay".to_string(),
                message,
                agent_id: Some(request.agent_id.clone()),
                current_version: self.current_prompt_version(request.agent_id.as_str()),
            })?;
        Ok(())
    }

    fn verify_hosted_prompt_control_rollback_strong_auth(
        &self,
        request: &PromptControlRollbackRequest,
    ) -> Result<(), PromptControlError> {
        let Some(grant) = request.strong_auth_grant.as_ref() else {
            return Err(self.hosted_prompt_control_strong_auth_error(
                "strong_auth_required",
                request.agent_id.as_str(),
                "prompt_control rollback requires hosted strong auth grant on hosted_public_join",
            ));
        };
        let signer_public_key =
            hosted_strong_auth_grant_public_key_from_env().map_err(|message| {
                self.hosted_prompt_control_strong_auth_error(
                    "strong_auth_required",
                    request.agent_id.as_str(),
                    message.as_str(),
                )
            })?;
        verify_hosted_prompt_control_rollback_strong_auth_grant(
            request,
            grant,
            signer_public_key.as_str(),
            hosted_strong_auth_now_unix_ms(),
        )
        .map_err(|message| {
            self.hosted_prompt_control_strong_auth_error(
                "strong_auth_grant_invalid",
                request.agent_id.as_str(),
                message.as_str(),
            )
        })
    }

    fn hosted_prompt_control_strong_auth_error(
        &self,
        code: &str,
        agent_id: &str,
        message: &str,
    ) -> PromptControlError {
        PromptControlError {
            code: code.to_string(),
            message: message.to_string(),
            agent_id: Some(agent_id.to_string()),
            current_version: self.current_prompt_version(agent_id),
        }
    }

    fn verify_agent_chat_auth(
        &mut self,
        request: &AgentChatRequest,
    ) -> Result<VerifiedPlayerAuth, AgentChatError> {
        let Some(auth) = request.auth.as_ref() else {
            return Err(AgentChatError {
                code: "auth_proof_required".to_string(),
                message: "agent_chat requires auth proof".to_string(),
                agent_id: Some(request.agent_id.clone()),
            });
        };
        let verified =
            verify_agent_chat_auth_proof(request, auth).map_err(|message| AgentChatError {
                code: map_auth_verify_error_code(message.as_str()).to_string(),
                message,
                agent_id: Some(request.agent_id.clone()),
            })?;
        Ok(verified)
    }

    fn current_prompt_version(&self, agent_id: &str) -> Option<u64> {
        self.llm_sidecar
            .prompt_profiles
            .get(agent_id)
            .map(|profile| profile.version)
    }

    fn current_prompt_profile(
        &self,
        agent_id: &str,
    ) -> Result<AgentPromptProfile, PromptControlError> {
        if !self.world.state().agents.contains_key(agent_id) {
            return Err(PromptControlError {
                code: "agent_not_found".to_string(),
                message: format!("agent not found: {agent_id}"),
                agent_id: Some(agent_id.to_string()),
                current_version: None,
            });
        }
        Ok(self
            .llm_sidecar
            .prompt_profiles
            .get(agent_id)
            .cloned()
            .unwrap_or_else(|| AgentPromptProfile::for_agent(agent_id.to_string())))
    }

    fn lookup_prompt_profile_version(
        &self,
        agent_id: &str,
        version: u64,
    ) -> Option<AgentPromptProfile> {
        self.llm_sidecar
            .prompt_profile_history
            .get(agent_id)
            .and_then(
                |versions: &std::collections::BTreeMap<u64, AgentPromptProfile>| {
                    versions.get(&version).cloned()
                },
            )
            .or_else(|| {
                let profile = self.llm_sidecar.prompt_profiles.get(agent_id)?;
                if profile.version == version {
                    Some(profile.clone())
                } else {
                    None
                }
            })
    }

    fn bind_agent_player_access(
        &mut self,
        agent_id: &str,
        player_id: &str,
        public_key: Option<&str>,
    ) -> Result<(), PromptControlError> {
        ensure_agent_player_access_runtime(
            &self.world,
            &self.llm_sidecar,
            agent_id,
            player_id,
            public_key,
        )?;
        let events = self
            .llm_sidecar
            .bind_agent_player(agent_id, player_id, public_key, false)
            .map_err(|message| PromptControlError {
                code: "player_bind_failed".to_string(),
                message,
                agent_id: Some(agent_id.to_string()),
                current_version: self.current_prompt_version(agent_id),
            })?;
        for event in events {
            self.enqueue_virtual_event(event);
        }
        Ok(())
    }

    fn bind_agent_player_access_for_chat(
        &mut self,
        agent_id: &str,
        player_id: &str,
        public_key: Option<&str>,
    ) -> Result<(), AgentChatError> {
        let mapped = ensure_agent_player_access_runtime(
            &self.world,
            &self.llm_sidecar,
            agent_id,
            player_id,
            public_key,
        )
        .map_err(|err| AgentChatError {
            code: "agent_control_forbidden".to_string(),
            message: err.message,
            agent_id: err.agent_id,
        });
        mapped?;
        let events = self
            .llm_sidecar
            .bind_agent_player(agent_id, player_id, public_key, false)
            .map_err(|message| AgentChatError {
                code: "player_bind_failed".to_string(),
                message,
                agent_id: Some(agent_id.to_string()),
            })?;
        for event in events {
            self.enqueue_virtual_event(event);
        }
        Ok(())
    }

    pub(super) fn enqueue_virtual_event(&mut self, kind: WorldEventKind) {
        let id = self.next_virtual_event_id();
        self.pending_virtual_events.push_back(WorldEvent {
            id,
            time: self.world.state().time,
            kind,
            runtime_event: None,
        });
    }

    fn enqueue_agent_chat_echo_event_if_enabled(&mut self, agent_id: &str, message: &str) {
        if !runtime_agent_chat_echo_enabled_from_env() {
            return;
        }
        let Some(agent) = self.world.state().agents.get(agent_id) else {
            return;
        };
        self.enqueue_virtual_event(WorldEventKind::AgentSpoke {
            agent_id: agent_id.to_string(),
            location_id: location_id_for_pos(agent.state.pos),
            message: format!("{RUNTIME_AGENT_CHAT_ECHO_PREFIX} {message}"),
            target_agent_id: None,
        });
    }

    fn next_virtual_event_id(&mut self) -> u64 {
        let floor = latest_runtime_event_seq(&self.world)
            .saturating_add(1)
            .max(1);
        if self.next_virtual_event_id < floor {
            self.next_virtual_event_id = floor;
        }
        let id = self.next_virtual_event_id;
        self.next_virtual_event_id = self.next_virtual_event_id.saturating_add(1);
        id
    }

    pub(super) fn enqueue_llm_action_from_sidecar(
        &mut self,
    ) -> Result<Option<AgentDecisionTrace>, AgentDecisionTrace> {
        let Some(decision) = self
            .llm_sidecar
            .next_llm_decision(&self.world, &self.snapshot_config)
        else {
            return Ok(None);
        };
        let decision_trace = decision.decision_trace.clone();
        if let Some(trace) = decision_trace.as_ref() {
            if trace.llm_error.is_some() {
                return Err(trace.clone());
            }
            if let Some(message) = trace.parse_error.as_ref() {
                self.enqueue_virtual_event(WorldEventKind::ActionRejected {
                    reason: SimulatorRejectReason::RuleDenied {
                        notes: vec![format!("llm_failed: {}", message)],
                    },
                });
                return Ok(decision_trace);
            }
        }

        if let AgentDecision::Act(action) = decision.decision {
            match simulator_action_to_runtime(&action, &self.world) {
                Some(runtime_action) => {
                    let action_id = self.world.submit_action(runtime_action);
                    self.llm_sidecar
                        .track_action(action_id, decision.agent_id, action.clone());
                }
                None => {
                    self.enqueue_virtual_event(WorldEventKind::ActionRejected {
                        reason: SimulatorRejectReason::RuleDenied {
                            notes: vec![format!(
                                "runtime llm bridge cannot map action: {}",
                                simulator_action_label(&action)
                            )],
                        },
                    });
                }
            }
        }
        Ok(decision_trace)
    }
}

pub(super) fn normalize_required_player_id(
    player_id: &str,
    agent_id: &str,
) -> Result<String, PromptControlError> {
    let normalized = player_id.trim();
    if normalized.is_empty() {
        return Err(PromptControlError {
            code: "player_id_required".to_string(),
            message: format!(
                "prompt_control for {} requires non-empty player_id",
                agent_id
            ),
            agent_id: Some(agent_id.to_string()),
            current_version: None,
        });
    }
    Ok(normalized.to_string())
}

pub(super) fn normalize_optional_public_key(public_key: Option<&str>) -> Option<String> {
    public_key
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(super) fn ensure_updated_by_matches_player_runtime(
    updated_by: Option<&str>,
    player_id: &str,
    agent_id: &str,
) -> Result<(), PromptControlError> {
    let Some(updated_by) = updated_by.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    if updated_by == player_id {
        return Ok(());
    }
    Err(PromptControlError {
        code: "updated_by_mismatch".to_string(),
        message: format!(
            "updated_by ({}) must match player_id ({}) for {}",
            updated_by, player_id, agent_id
        ),
        agent_id: Some(agent_id.to_string()),
        current_version: None,
    })
}

pub(super) fn ensure_agent_player_access_runtime(
    world: &RuntimeWorld,
    sidecar: &RuntimeLlmSidecar,
    agent_id: &str,
    player_id: &str,
    public_key: Option<&str>,
) -> Result<(), PromptControlError> {
    if !world.state().agents.contains_key(agent_id) {
        return Err(PromptControlError {
            code: "agent_not_found".to_string(),
            message: format!("agent not found: {agent_id}"),
            agent_id: Some(agent_id.to_string()),
            current_version: None,
        });
    }
    let Some(bound_player_id) = sidecar.agent_player_bindings.get(agent_id) else {
        return Ok(());
    };
    if bound_player_id == player_id {
        let Some(bound_public_key) = sidecar.agent_public_key_bindings.get(agent_id) else {
            return Ok(());
        };
        let requested_public_key = normalize_optional_public_key(public_key);
        if requested_public_key.as_deref() == Some(bound_public_key.as_str()) {
            return Ok(());
        }
        let message = if requested_public_key.is_none() {
            format!(
                "agent {} is bound to player {} with public_key {}, public_key is required",
                agent_id, bound_player_id, bound_public_key
            )
        } else {
            format!(
                "agent {} is bound to player {} with different public_key",
                agent_id, bound_player_id
            )
        };
        return Err(PromptControlError {
            code: "agent_control_forbidden".to_string(),
            message,
            agent_id: Some(agent_id.to_string()),
            current_version: sidecar
                .prompt_profiles
                .get(agent_id)
                .map(|entry| entry.version),
        });
    }
    Err(PromptControlError {
        code: "agent_control_forbidden".to_string(),
        message: format!(
            "agent {} is bound to player {}, not {}",
            agent_id, bound_player_id, player_id
        ),
        agent_id: Some(agent_id.to_string()),
        current_version: sidecar
            .prompt_profiles
            .get(agent_id)
            .map(|entry| entry.version),
    })
}

pub(super) fn apply_prompt_patch_runtime(
    profile: &mut AgentPromptProfile,
    request: &PromptControlApplyRequest,
) {
    if let Some(next) = &request.system_prompt_override {
        profile.system_prompt_override = sanitize_patch_string(next.clone());
    }
    if let Some(next) = &request.short_term_goal_override {
        profile.short_term_goal_override = sanitize_patch_string(next.clone());
    }
    if let Some(next) = &request.long_term_goal_override {
        profile.long_term_goal_override = sanitize_patch_string(next.clone());
    }
}

fn sanitize_patch_string(value: Option<String>) -> Option<String> {
    value
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
}

pub(super) fn changed_prompt_fields_runtime(
    current: &AgentPromptProfile,
    candidate: &AgentPromptProfile,
) -> Vec<String> {
    let mut fields = Vec::new();
    if current.system_prompt_override != candidate.system_prompt_override {
        fields.push("system_prompt_override".to_string());
    }
    if current.short_term_goal_override != candidate.short_term_goal_override {
        fields.push("short_term_goal_override".to_string());
    }
    if current.long_term_goal_override != candidate.long_term_goal_override {
        fields.push("long_term_goal_override".to_string());
    }
    fields
}

pub(super) fn prompt_profile_digest_runtime(profile: &AgentPromptProfile) -> String {
    let payload = serde_json::json!({
        "agent_id": profile.agent_id,
        "system_prompt_override": profile.system_prompt_override,
        "short_term_goal_override": profile.short_term_goal_override,
        "long_term_goal_override": profile.long_term_goal_override,
    });
    let bytes = serde_json::to_vec(&payload).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub(super) fn ensure_expected_prompt_version_runtime(
    agent_id: &str,
    current_version: u64,
    expected_version: Option<u64>,
) -> Result<(), PromptControlError> {
    if let Some(expected) = expected_version {
        if expected != current_version {
            return Err(PromptControlError {
                code: "version_conflict".to_string(),
                message: format!(
                    "prompt profile version conflict for {}: expected {}, current {}",
                    agent_id, expected, current_version
                ),
                agent_id: Some(agent_id.to_string()),
                current_version: Some(current_version),
            });
        }
    }
    Ok(())
}

pub(super) fn map_auth_verify_error_code(message: &str) -> &'static str {
    if message.contains("nonce") {
        return "auth_nonce_invalid";
    }
    if message.contains("signature") || message.contains("awviewauth:v1") {
        return "auth_signature_invalid";
    }
    if message.contains("player_id") || message.contains("public_key") {
        return "auth_claim_mismatch";
    }
    if message.contains("required") || message.contains("empty") {
        return "auth_claim_invalid";
    }
    "auth_invalid"
}

fn resolve_agent_chat_intent(
    request: &AgentChatRequest,
    verified_nonce: u64,
) -> Result<ResolvedAgentChatIntent, String> {
    let intent_seq = match request.intent_seq {
        Some(0) => {
            return Err("intent_seq must be greater than zero".to_string());
        }
        Some(seq) if seq != verified_nonce => {
            return Err(format!(
                "intent_seq {} must match auth nonce {}",
                seq, verified_nonce
            ));
        }
        Some(seq) => seq,
        None => verified_nonce,
    };
    Ok(ResolvedAgentChatIntent {
        intent_tick: request.intent_tick,
        intent_seq,
    })
}
