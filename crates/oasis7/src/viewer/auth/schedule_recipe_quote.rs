use super::*;
use crate::viewer::protocol::ScheduleRecipeQuoteRequest;

fn signing_request(request: &ScheduleRecipeQuoteRequest) -> GameplayActionRequest {
    GameplayActionRequest {
        action_id: "quote_schedule_recipe".to_string(),
        // Bind the complete requested production operation; this remains a preflight only.
        target_agent_id: format!(
            "factory_id:{}|recipe_id:{}|batches:{}",
            request.factory_id, request.recipe_id, request.batches
        ),
        actor_agent_id: None,
        player_id: request.player_id.clone(),
        public_key: request.public_key.clone(),
        auth: None,
    }
}

pub fn sign_schedule_recipe_quote_auth_proof(
    request: &ScheduleRecipeQuoteRequest,
    nonce: u64,
    signer_public_key_hex: &str,
    signer_private_key_hex: &str,
) -> Result<PlayerAuthProof, String> {
    sign_gameplay_action_auth_proof(
        &signing_request(request),
        nonce,
        signer_public_key_hex,
        signer_private_key_hex,
    )
}

pub fn verify_schedule_recipe_quote_auth_proof(
    request: &ScheduleRecipeQuoteRequest,
    proof: &PlayerAuthProof,
) -> Result<VerifiedPlayerAuth, String> {
    verify_gameplay_action_auth_proof(&signing_request(request), proof)
}
