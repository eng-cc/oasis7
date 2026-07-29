use super::control_plane::{
    ensure_agent_player_access_runtime, map_auth_verify_error_code, normalize_optional_public_key,
};
use super::session_policy::map_session_policy_error_code;
use super::*;
use crate::simulator::{ChunkCoord, FragmentRefillPreview, WorldKernel};
use crate::viewer::auth::verify_fragment_refill_preview_auth_proof;
use crate::viewer::protocol::{
    FragmentRefillElementRemaining, FragmentRefillPreviewChunk,
    FragmentRefillPreviewProtocolRequest, FragmentRefillPreviewRequest,
    FragmentRefillPreviewResponse, GameplayActionError,
};

const ACTION_ID: &str = "preview_fragment_replenishment";

impl ViewerRuntimeLiveServer {
    pub(in crate::viewer::runtime_live) fn preview_refill(
        &mut self,
        request: FragmentRefillPreviewProtocolRequest,
        writer: &mut BufWriter<TcpStream>,
    ) -> Result<(), ViewerRuntimeLiveServerError> {
        send_response(
            writer,
            &self
                .handle_fragment_refill_preview_protocol(request)
                .map(|quote| ViewerResponse::FragmentRefillPreviewPreflight { quote })
                .unwrap_or_else(|error| ViewerResponse::GameplayActionError { error }),
        )
    }

    pub(in crate::viewer::runtime_live) fn handle_fragment_refill_preview_protocol(
        &mut self,
        request: FragmentRefillPreviewProtocolRequest,
    ) -> Result<FragmentRefillPreviewResponse, GameplayActionError> {
        let chunk = request.chunk;
        self.handle_fragment_refill_preview(FragmentRefillPreviewRequest {
            chunk: ChunkCoord {
                x: chunk.x,
                y: chunk.y,
                z: chunk.z,
            },
            player_id: request.player_id,
            public_key: request.public_key,
            auth: request.auth,
        })
        .map(fragment_refill_preview_response)
    }

    /// Reads the kernel's fragment-replenishment forecast from a fresh runtime projection.
    /// It does not advance time, consume a nonce, or mutate a player/session binding.
    pub(in crate::viewer::runtime_live) fn handle_fragment_refill_preview(
        &mut self,
        request: FragmentRefillPreviewRequest,
    ) -> Result<FragmentRefillPreview, GameplayActionError> {
        let auth = request.auth.as_ref().ok_or_else(|| GameplayActionError {
            code: "auth_proof_required".to_string(),
            message: format!("{ACTION_ID} requires auth proof"),
            action_id: Some(ACTION_ID.to_string()),
            target_agent_id: None,
        })?;
        let verified =
            verify_fragment_refill_preview_auth_proof(&request, auth).map_err(|message| {
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
        let model = super::mapping::runtime_state_to_simulator_model(
            self.world.state(),
            &self.llm_sidecar,
            self.seed_model.as_ref(),
        );
        let mut kernel = WorldKernel::with_model_and_chunk_runtime(
            self.snapshot_config.clone(),
            model,
            self.llm_sidecar.chunk_runtime.clone(),
        );
        let mut snapshot = kernel.snapshot();
        snapshot.time = self.world.state().time;
        kernel =
            WorldKernel::from_snapshot(snapshot, kernel.journal_snapshot()).map_err(|err| {
                GameplayActionError {
                    code: "fragment_refill_preview_projection_failed".to_string(),
                    message: format!("{ACTION_ID} projection failed: {err:?}"),
                    action_id: Some(ACTION_ID.to_string()),
                    target_agent_id: Some(agent_id.to_string()),
                }
            })?;
        kernel
            .quote_fragment_refill_preview(request.chunk)
            .map_err(|reason| GameplayActionError {
                code: "fragment_refill_preview_rejected".to_string(),
                message: format!("{ACTION_ID} rejected: {reason}"),
                action_id: Some(ACTION_ID.to_string()),
                target_agent_id: Some(agent_id.to_string()),
            })
    }
}

fn fragment_refill_preview_response(quote: FragmentRefillPreview) -> FragmentRefillPreviewResponse {
    FragmentRefillPreviewResponse {
        chunk: FragmentRefillPreviewChunk {
            x: quote.chunk_coord.x,
            y: quote.chunk_coord.y,
            z: quote.chunk_coord.z,
        },
        target_frag_id: quote.target_frag_id,
        current_frag_remaining_summary: quote.current_frag_remaining_summary,
        chunk_remaining_summary: quote.chunk_remaining_summary,
        remaining_by_element_g: quote
            .remaining_by_element_g
            .into_iter()
            .map(|(element, remaining_g)| FragmentRefillElementRemaining {
                element: element.wire_label().to_string(),
                remaining_g,
            })
            .collect(),
        replenishment_enabled: quote.replenishment_enabled,
        replenishment_due: quote.replenishment_due,
        next_replenish_tick: quote.next_replenish_tick,
        ticks_until_replenish: quote.ticks_until_replenish,
        wait_cost_ticks: quote.wait_cost_ticks,
        estimated_replenished_frag_count: quote.estimated_replenished_frag_count,
        estimated_replenished_resource_hint: quote.estimated_replenished_resource_hint,
        next_industrial_goal_relevance: quote.next_industrial_goal_relevance,
        wait_cost_summary: quote.wait_cost_summary,
        recommended_resource_action: quote.recommended_resource_action,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulator::FragmentElementKind;
    use std::collections::BTreeMap;

    #[test]
    fn wire_response_uses_canonical_element_label_not_debug_formatting() {
        let mut remaining_by_element_g = BTreeMap::new();
        remaining_by_element_g.insert(FragmentElementKind::Iron, 17);
        let response = fragment_refill_preview_response(FragmentRefillPreview {
            chunk_coord: ChunkCoord { x: 1, y: 2, z: 3 },
            target_frag_id: None,
            current_frag_remaining_summary: "none".to_string(),
            chunk_remaining_summary: "17g".to_string(),
            remaining_by_element_g,
            replenishment_enabled: true,
            replenishment_due: false,
            next_replenish_tick: Some(10),
            ticks_until_replenish: Some(5),
            wait_cost_ticks: 5,
            estimated_replenished_frag_count: 0,
            estimated_replenished_resource_hint: "unknown".to_string(),
            next_industrial_goal_relevance: "current".to_string(),
            wait_cost_summary: "wait".to_string(),
            recommended_resource_action: "wait_current_chunk".to_string(),
        });

        assert_eq!(response.remaining_by_element_g[0].element, "iron");
    }
}
