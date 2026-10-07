use crate::{runtime::{Action, World}, viewer::*};

pub fn authenticated_action(world: &World, bytes: &[u8]) -> Result<Action, String> {
    if let Ok(request) = serde_json::from_slice::<GameplayActionRequest>(bytes) {
        let proof = request.auth.as_ref().ok_or("missing gameplay signature")?;
        verify_gameplay_action_auth_proof(&request, proof)?;
        return build_runtime_action_from_gameplay_request(&request).map_err(|e| e.message);
    }
    let command: CollectDataCommand = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let CollectDataCommand::Submit { request } = &command else { return Err("unsupported collect_data".into()); };
    let proof = request.auth.as_ref().ok_or("missing collect_data signature")?;
    let verified = verify_collect_data_auth_proof(&command, proof)?;
    let mut claims = world.state().starter_oc_claims.values().filter(|claim|
        claim.player_id == verified.player_id && claim.public_key.as_deref() == Some(&verified.public_key));
    let claim = claims.next().ok_or("canonical Agent owner claim unavailable")?;
    if claims.next().is_some() { return Err("ambiguous Agent owner claim".into()); }
    Ok(Action::CollectDataAuthenticated { collector_agent_id: claim.agent_id.clone(), electricity_cost: request.electricity_cost,
        data_amount: request.data_amount, player_id: verified.player_id, public_key: verified.public_key,
        nonce: verified.nonce, signature: proof.signature.clone() })
}
