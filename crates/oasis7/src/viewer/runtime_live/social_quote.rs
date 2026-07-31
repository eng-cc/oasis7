use super::control_plane::{
    ensure_agent_player_access_runtime, map_auth_verify_error_code, normalize_optional_public_key,
};
use super::session_policy::map_session_policy_error_code;
use super::support::send_response;
use super::*;
use crate::simulator::{ResourceOwner, WorldKernel};
use crate::viewer::auth::verify_declare_social_edge_quote_auth_proof;
use crate::viewer::protocol::{
    DeclareSocialEdgeQuotePreflight, DeclareSocialEdgeQuoteRequest, GameplayActionError,
};
use std::io::BufWriter;
use std::net::TcpStream;

impl ViewerRuntimeLiveServer {
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
}
