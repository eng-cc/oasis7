use super::*;

pub fn key_digest(key: &RequestKey) -> Result<String, String> {
    request_digest("request-key", key)
}

pub fn validate_result(key: &str, value: &serde_json::Value) -> Result<(), String> {
    let result: CanonicalIntentResultV1 =
        serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    result.request.validate().map_err(|e| e.to_string())?;
    if derive_correlation(
        result.request.correlation.key.world.clone(),
        &result.request.signed_payload,
    )? != result.request.correlation
        || key_digest(&result.request.correlation.key)? != key
        || result.committed_height == 0
    {
        return Err("invalid canonical intent result identity".into());
    }
    super::game_task::validate_result(&result)?;
    Ok(())
}

/// Derives identity from verified signed material; server additionally checks
/// canonical ownership/delegation and expected world before accepting it.
pub fn derive_correlation(
    world: WorldIdentity,
    payload: &WorldServicePayloadV1,
) -> Result<RequestCorrelation, String> {
    oasis7_client_api::world_service_correlation::derive_correlation_with_legacy(
        world,
        payload,
        |payload| {
            Ok(match payload {
                WorldServicePayloadV1::GameplayJson(bytes) => {
                    if let Ok(request) =
                        serde_json::from_slice::<crate::viewer::GameplayActionRequest>(bytes)
                    {
                        let proof = request.auth.as_ref().ok_or("missing gameplay proof")?;
                        let verified =
                            crate::viewer::verify_gameplay_action_auth_proof(&request, proof)?;
                        (
                            verified.public_key,
                            "gameplay",
                            "player_auth",
                            verified.nonce.to_string(),
                        )
                    } else {
                        let command: crate::viewer::CollectDataCommand =
                            serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
                        let crate::viewer::CollectDataCommand::Submit { request } = &command else {
                            return Err("unsupported collect_data operation".into());
                        };
                        let proof = request.auth.as_ref().ok_or("missing collect_data proof")?;
                        let verified =
                            crate::viewer::verify_collect_data_auth_proof(&command, proof)?;
                        (
                            verified.public_key,
                            "collect_data",
                            "collect_data_submit",
                            verified.nonce.to_string(),
                        )
                    }
                }
                WorldServicePayloadV1::AgentChat(request) => {
                    let proof = request
                        .auth
                        .as_ref()
                        .ok_or("missing canonical chat proof")?;
                    let verified = crate::viewer::verify_agent_chat_auth_proof(request, proof)?;
                    (
                        verified.public_key,
                        "agent_chat",
                        "canonical_owner_chat",
                        verified.nonce.to_string(),
                    )
                }
                _ => return Err("legacy verifier received non-browser payload".into()),
            })
        },
    )
}

/// Register the intent inside the existing opaque RuntimeAction node frame.
pub fn encode_consensus_intent(
    request: &super::SubmitIntentRequest<WorldServicePayloadV1>,
) -> Result<Vec<u8>, String> {
    use crate::consensus_action_payload::*;
    encode_consensus_action_payload(&ConsensusActionPayloadEnvelope {
        version: 1,
        auth: None,
        gameplay_submission_origin: None,
        body: ConsensusActionPayloadBody::RuntimeAction {
            action: crate::runtime::Action::WorldServiceIntent {
                request: serde_json::to_value(request).map_err(|error| error.to_string())?,
            },
        },
    })
}
