use super::control_plane::{
    ensure_agent_player_access_runtime, map_auth_verify_error_code, normalize_optional_public_key,
};
use super::session_policy::map_session_policy_error_code;
use super::support::send_response;
use super::*;
use crate::simulator::{ResourceKind, ResourceOwner, SocialStake, WorldJournal, WorldKernel};
use crate::viewer::auth::{
    verify_declare_social_edge_quote_auth_proof, verify_publish_social_fact_quote_auth_proof,
    verify_social_contact_quote_auth_proof,
};
use crate::viewer::protocol::{
    DeclareSocialEdgeQuotePreflight, DeclareSocialEdgeQuoteRequest, FirstContactClass,
    GameplayActionError, PublishSocialFactQuotePreflight, PublishSocialFactQuoteRequest,
    SocialContactQuotePreflight, SocialContactQuoteRequest,
};
use std::collections::HashMap;
use std::io::BufWriter;
use std::net::TcpStream;

impl ViewerRuntimeLiveServer {
    /// Computes an authenticated, non-mutating first-contact preview from runtime session state.
    pub(in crate::viewer::runtime_live) fn handle_social_contact_quote(
        &mut self,
        request: SocialContactQuoteRequest,
    ) -> Result<SocialContactQuotePreflight, GameplayActionError> {
        const ACTION_ID: &str = "quote_social_contact";
        let auth = request.auth.as_ref().ok_or_else(|| GameplayActionError {
            code: "auth_proof_required".to_string(),
            message: format!("{ACTION_ID} requires auth proof"),
            action_id: Some(ACTION_ID.to_string()),
            target_agent_id: None,
        })?;
        let verified =
            verify_social_contact_quote_auth_proof(&request, auth).map_err(|message| {
                GameplayActionError {
                    code: map_auth_verify_error_code(message.as_str()).to_string(),
                    message,
                    action_id: Some(ACTION_ID.to_string()),
                    target_agent_id: None,
                }
            })?;
        self.session_policy
            .validate_known_session_key(verified.player_id.as_str(), verified.public_key.as_str())
            .map_err(|message| GameplayActionError {
                code: map_session_policy_error_code(message.as_str()).to_string(),
                message,
                action_id: Some(ACTION_ID.to_string()),
                target_agent_id: None,
            })?;
        let agent_id = self
            .llm_sidecar
            .bound_agent_for_player(verified.player_id.as_str())
            .ok_or_else(|| GameplayActionError {
                code: "player_agent_binding_required".to_string(),
                message: format!("{ACTION_ID} requires a bound player Agent session"),
                action_id: Some(ACTION_ID.to_string()),
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
            action_id: Some(ACTION_ID.to_string()),
            target_agent_id: err.agent_id,
        })?;

        let purpose = request.contact_purpose.trim();
        let (
            first_contact_class,
            expected_mutual_value,
            risk_or_commitment,
            recommended_contact_action,
            defer_reason,
        ) = if purpose.is_empty() {
            (
                    FirstContactClass::DeferContact,
                    "Keep the current local goal visible until a concrete exchange is available."
                        .to_string(),
                    "No resources, time, reputation, or membership are exposed.".to_string(),
                    "Continue independent local work".to_string(),
                    "No current local purpose makes contact worthwhile; revisit when a specific trade, aid, or information need appears.".to_string(),
                )
        } else if matches!(
            request.first_contact_class,
            FirstContactClass::OrganizationEscalation
        ) {
            (
                    FirstContactClass::DeferContact,
                    "Keep the current local goal visible while evaluating later collaboration."
                        .to_string(),
                    "Organization membership, governance, long-term supply, and exclusivity require a separate later confirmation.".to_string(),
                    "Continue independent local work".to_string(),
                    "Organization escalation is not a default first contact; revisit it only as a separate later decision.".to_string(),
                )
        } else if matches!(request.first_contact_class, FirstContactClass::DeferContact) {
            (
                    FirstContactClass::DeferContact,
                    "Retain the local objective without creating a social obligation.".to_string(),
                    "No resources, time, reputation, or membership are exposed.".to_string(),
                    "Continue independent local work".to_string(),
                    "Independent progress is currently the safer choice; revisit when the stated purpose becomes urgent.".to_string(),
                )
        } else {
            let (value, risk, action) = match request.first_contact_class {
                    FirstContactClass::TradeOrService => (
                        "Both sides can satisfy a concrete, limited exchange.".to_string(),
                        "Only the stated one-time trade or service is exposed; no membership or governance commitment is created.".to_string(),
                        "Propose the limited trade or service".to_string(),
                    ),
                    FirstContactClass::MutualAid => (
                        "Both sides can remove a current blocker through recoverable aid.".to_string(),
                        "Aid remains scoped to the current blocker and creates no continuing membership obligation.".to_string(),
                        "Request scoped mutual aid".to_string(),
                    ),
                    FirstContactClass::InformationExchange => (
                        "Both sides gain route, price, risk, or opportunity information.".to_string(),
                        "Information exchange commits no resources, votes, or organization identity.".to_string(),
                        "Exchange scoped route or price information".to_string(),
                    ),
                    FirstContactClass::DeferContact | FirstContactClass::OrganizationEscalation => unreachable!("handled above"),
                };
            (
                request.first_contact_class,
                value,
                risk,
                action,
                String::new(),
            )
        };

        Ok(SocialContactQuotePreflight {
            first_contact_class,
            contact_purpose: purpose.to_string(),
            expected_mutual_value,
            risk_or_commitment,
            solo_lane_preserved: true,
            recommended_contact_action,
            defer_reason,
        })
    }

    /// Computes an authenticated, non-mutating social-edge impact preflight from runtime state.
    pub(in crate::viewer::runtime_live) fn handle_declare_social_edge_quote(
        &mut self,
        request: DeclareSocialEdgeQuoteRequest,
    ) -> Result<DeclareSocialEdgeQuotePreflight, GameplayActionError> {
        let auth = request.auth.as_ref().ok_or_else(|| GameplayActionError {
            code: "auth_proof_required".to_string(),
            message: "quote_declare_social_edge requires auth proof".to_string(),
            action_id: Some("quote_declare_social_edge".to_string()),
            target_agent_id: None,
        })?;
        let verified =
            verify_declare_social_edge_quote_auth_proof(&request, auth).map_err(|message| {
                GameplayActionError {
                    code: map_auth_verify_error_code(message.as_str()).to_string(),
                    message,
                    action_id: Some("quote_declare_social_edge".to_string()),
                    target_agent_id: None,
                }
            })?;
        self.session_policy
            .validate_known_session_key(verified.player_id.as_str(), verified.public_key.as_str())
            .map_err(|message| GameplayActionError {
                code: map_session_policy_error_code(message.as_str()).to_string(),
                message,
                action_id: Some("quote_declare_social_edge".to_string()),
                target_agent_id: None,
            })?;
        let declarer_agent_id = self
            .llm_sidecar
            .bound_agent_for_player(verified.player_id.as_str())
            .ok_or_else(|| GameplayActionError {
                code: "player_agent_binding_required".to_string(),
                message: "quote_declare_social_edge requires a bound player Agent session"
                    .to_string(),
                action_id: Some("quote_declare_social_edge".to_string()),
                target_agent_id: None,
            })?;
        let public_key = normalize_optional_public_key(request.public_key.as_deref());
        ensure_agent_player_access_runtime(
            &self.world,
            &self.llm_sidecar,
            declarer_agent_id,
            verified.player_id.as_str(),
            public_key.as_deref(),
        )
        .map_err(|err| GameplayActionError {
            code: err.code,
            message: err.message,
            action_id: Some("quote_declare_social_edge".to_string()),
            target_agent_id: err.agent_id,
        })?;
        let model = super::mapping::runtime_state_to_simulator_model(
            self.world.state(),
            &self.llm_sidecar,
            self.seed_model.as_ref(),
        );
        let quote = WorldKernel::with_model(self.snapshot_config.clone(), model)
            .quote_declare_social_edge(
                &ResourceOwner::Agent {
                    agent_id: declarer_agent_id.to_string(),
                },
                request.schema_id.as_str(),
                request.relation_kind.as_str(),
                &ResourceOwner::Agent {
                    agent_id: request.from_agent_id,
                },
                &ResourceOwner::Agent {
                    agent_id: request.to_agent_id,
                },
                request.weight_bps,
                request.backing_fact_ids.as_slice(),
                request.ttl_ticks,
            )
            .map_err(|reason| GameplayActionError {
                code: "declare_social_edge_quote_rejected".to_string(),
                message: format!("quote_declare_social_edge rejected: {reason:?}"),
                action_id: Some("quote_declare_social_edge".to_string()),
                target_agent_id: Some(declarer_agent_id.to_string()),
            })?;
        Ok(DeclareSocialEdgeQuotePreflight {
            actor_id: quote.actor_id,
            action_kind: quote.action_kind,
            schema_id: quote.schema_id,
            subject_id: quote.subject_id,
            object_id: quote.object_id,
            claim_summary: quote.claim_summary,
            confidence_ppm: quote.confidence_ppm,
            stake_at_risk: quote.stake_at_risk,
            ttl_ticks: quote.ttl_ticks,
            affected_relationships: quote.affected_relationships,
            affected_social_surfaces: quote.affected_social_surfaces,
            cooperation_opportunity_delta: quote.cooperation_opportunity_delta,
            blacklist_or_dispute_risk: quote.blacklist_or_dispute_risk,
            governance_or_claim_relevance: quote.governance_or_claim_relevance,
            recommended_social_action: quote.recommended_social_action,
            why_this_action_matters: quote.why_this_action_matters,
        })
    }

    /// Computes an authenticated, non-mutating social-fact impact preflight from runtime state.
    pub(in crate::viewer::runtime_live) fn handle_publish_social_fact_quote(
        &mut self,
        request: PublishSocialFactQuoteRequest,
    ) -> Result<PublishSocialFactQuotePreflight, GameplayActionError> {
        const ACTION_ID: &str = "quote_publish_social_fact";
        let auth = request.auth.as_ref().ok_or_else(|| GameplayActionError {
            code: "auth_proof_required".to_string(),
            message: format!("{ACTION_ID} requires auth proof"),
            action_id: Some(ACTION_ID.to_string()),
            target_agent_id: None,
        })?;
        let verified =
            verify_publish_social_fact_quote_auth_proof(&request, auth).map_err(|message| {
                GameplayActionError {
                    code: map_auth_verify_error_code(message.as_str()).to_string(),
                    message,
                    action_id: Some(ACTION_ID.to_string()),
                    target_agent_id: None,
                }
            })?;
        self.session_policy
            .validate_known_session_key(verified.player_id.as_str(), verified.public_key.as_str())
            .map_err(|message| GameplayActionError {
                code: map_session_policy_error_code(message.as_str()).to_string(),
                message,
                action_id: Some(ACTION_ID.to_string()),
                target_agent_id: None,
            })?;
        let publisher_agent_id = self
            .llm_sidecar
            .bound_agent_for_player(verified.player_id.as_str())
            .ok_or_else(|| GameplayActionError {
                code: "player_agent_binding_required".to_string(),
                message: format!("{ACTION_ID} requires a bound player Agent session"),
                action_id: Some(ACTION_ID.to_string()),
                target_agent_id: None,
            })?;
        let public_key = normalize_optional_public_key(request.public_key.as_deref());
        ensure_agent_player_access_runtime(
            &self.world,
            &self.llm_sidecar,
            publisher_agent_id,
            verified.player_id.as_str(),
            public_key.as_deref(),
        )
        .map_err(|err| GameplayActionError {
            code: err.code,
            message: err.message,
            action_id: Some(ACTION_ID.to_string()),
            target_agent_id: err.agent_id,
        })?;
        let stake = request
            .stake
            .as_ref()
            .map(|stake| {
                let kind = match stake.kind.trim().to_ascii_lowercase().as_str() {
                    "electricity" => ResourceKind::Electricity,
                    "data" => ResourceKind::Data,
                    _ => {
                        return Err(GameplayActionError {
                            code: "publish_social_fact_quote_rejected".to_string(),
                            message: format!("{ACTION_ID} rejected: invalid stake kind"),
                            action_id: Some(ACTION_ID.to_string()),
                            target_agent_id: Some(publisher_agent_id.to_string()),
                        });
                    }
                };
                Ok(SocialStake {
                    kind,
                    amount: stake.amount,
                })
            })
            .transpose()?;
        let object = request
            .object_agent_id
            .map(|agent_id| ResourceOwner::Agent { agent_id });
        let model = super::mapping::runtime_state_to_simulator_model(
            self.world.state(),
            &self.llm_sidecar,
            self.seed_model.as_ref(),
        );
        let mut simulator_event_ids = HashMap::new();
        let journal = WorldJournal {
            version: crate::simulator::JOURNAL_VERSION,
            events: self
                .world
                .journal()
                .events
                .iter()
                .enumerate()
                .map(|(index, event)| {
                    simulator_event_ids.insert(event.id, index as u64);
                    let mut mapped = super::mapping::map_runtime_event(
                        event,
                        &self.snapshot_config,
                        self.seed_model.as_ref(),
                    );
                    mapped.id = index as u64;
                    mapped
                })
                .collect(),
        };
        let evidence_event_ids = request
            .evidence_event_ids
            .iter()
            .map(|runtime_event_id| {
                simulator_event_ids
                    .get(runtime_event_id)
                    .copied()
                    .ok_or_else(|| GameplayActionError {
                        code: "publish_social_fact_quote_rejected".to_string(),
                        message: format!(
                            "{ACTION_ID} rejected: social evidence event missing: {runtime_event_id}"
                        ),
                        action_id: Some(ACTION_ID.to_string()),
                        target_agent_id: Some(publisher_agent_id.to_string()),
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let mut kernel = WorldKernel::with_model(self.snapshot_config.clone(), model);
        let mut snapshot = kernel.snapshot();
        snapshot.time = self.world.state().time;
        snapshot.journal_len = journal.events.len();
        snapshot.next_event_id = journal.events.len() as u64;
        kernel =
            WorldKernel::from_snapshot(snapshot, journal).map_err(|err| GameplayActionError {
                code: "publish_social_fact_quote_projection_failed".to_string(),
                message: format!("{ACTION_ID} projection failed: {err:?}"),
                action_id: Some(ACTION_ID.to_string()),
                target_agent_id: Some(publisher_agent_id.to_string()),
            })?;
        let quote = kernel
            .quote_publish_social_fact(
                &ResourceOwner::Agent {
                    agent_id: publisher_agent_id.to_string(),
                },
                request.schema_id.as_str(),
                &ResourceOwner::Agent {
                    agent_id: request.subject_agent_id,
                },
                object.as_ref(),
                request.claim.as_str(),
                request.confidence_ppm,
                evidence_event_ids.as_slice(),
                request.ttl_ticks,
                stake.as_ref(),
            )
            .map_err(|reason| GameplayActionError {
                code: "publish_social_fact_quote_rejected".to_string(),
                message: format!("{ACTION_ID} rejected: {reason:?}"),
                action_id: Some(ACTION_ID.to_string()),
                target_agent_id: Some(publisher_agent_id.to_string()),
            })?;
        Ok(PublishSocialFactQuotePreflight {
            actor_id: quote.actor_id,
            action_kind: "publish_fact".to_string(),
            schema_id: quote.schema_id,
            subject_id: quote.subject_id,
            object_id: quote.object_id,
            claim_summary: quote.claim_summary,
            confidence_ppm: quote.confidence_ppm,
            stake_at_risk: quote.stake_at_risk,
            ttl_ticks: quote.ttl_ticks,
            affected_relationships: quote.affected_relationships,
            affected_social_surfaces: quote.affected_social_surfaces,
            cooperation_opportunity_delta: quote.cooperation_opportunity_delta,
            blacklist_or_dispute_risk: quote.blacklist_or_dispute_risk,
            governance_or_claim_relevance: quote.governance_or_claim_relevance,
            recommended_social_action: quote.recommended_social_action,
            why_this_action_matters: quote.why_this_action_matters,
        })
    }

    pub(in crate::viewer::runtime_live) fn quote_declare_social_edge(
        &mut self,
        request: DeclareSocialEdgeQuoteRequest,
        writer: &mut BufWriter<TcpStream>,
    ) -> Result<(), ViewerRuntimeLiveServerError> {
        send_response(
            writer,
            &self
                .handle_declare_social_edge_quote(request)
                .map(|quote| ViewerResponse::DeclareSocialEdgeQuotePreflight { quote })
                .unwrap_or_else(|error| ViewerResponse::GameplayActionError { error }),
        )
    }

    pub(in crate::viewer::runtime_live) fn quote_publish_social_fact(
        &mut self,
        request: PublishSocialFactQuoteRequest,
        writer: &mut BufWriter<TcpStream>,
    ) -> Result<(), ViewerRuntimeLiveServerError> {
        send_response(
            writer,
            &self
                .handle_publish_social_fact_quote(request)
                .map(|quote| ViewerResponse::PublishSocialFactQuotePreflight { quote })
                .unwrap_or_else(|error| ViewerResponse::GameplayActionError { error }),
        )
    }

    pub(in crate::viewer::runtime_live) fn quote_social_contact(
        &mut self,
        request: SocialContactQuoteRequest,
        writer: &mut BufWriter<TcpStream>,
    ) -> Result<(), ViewerRuntimeLiveServerError> {
        send_response(
            writer,
            &self
                .handle_social_contact_quote(request)
                .map(|quote| ViewerResponse::SocialContactQuotePreflight { quote })
                .unwrap_or_else(|error| ViewerResponse::GameplayActionError { error }),
        )
    }
}
