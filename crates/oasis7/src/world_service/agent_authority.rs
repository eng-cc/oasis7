//! Canonical owner/delegate authorization; no provider-local key registry.
use super::{
    AgentSignerDelegationChangeV1, CognitionIntentV1, SignedReadRequest, verify_read_request,
};
use crate::runtime::World;

pub fn owner_public_key(world: &World, agent_id: &str) -> Result<String, String> {
    let identity = world
        .capability_revocation_state()
        .agent_identities
        .get(agent_id)
        .ok_or("Agent identity unavailable")?;
    let mut claims =
        world.state().starter_oc_claims.values().filter(|claim| {
            claim.agent_id == agent_id && claim.player_id == identity.owner_binding
        });
    let claim = claims.next().ok_or("canonical owner claim unavailable")?;
    if claims.next().is_some() {
        return Err("ambiguous canonical owner claim".into());
    }
    claim
        .public_key
        .clone()
        .ok_or("canonical owner key unavailable".into())
}

pub fn validate_delegation(
    world: &World,
    signed: &SignedReadRequest<AgentSignerDelegationChangeV1>,
) -> Result<(), String> {
    verify_read_request("delegation", signed)?;
    let change = &signed.request;
    if owner_public_key(world, &change.agent_id)? != signed.subject_public_key {
        return Err("delegation requires canonical owner signature".into());
    }
    let identity = &world.capability_revocation_state().agent_identities[&change.agent_id];
    let binding = world
        .current_cognition_runtime_binding()
        .map_err(|e| format!("{e:?}"))?;
    if change.owner_binding != identity.owner_binding
        || change.agent_identity_generation != identity.generation
        || change.world_id != binding.world_id
        || change.branch_id != binding.branch_id
        || change.generation == 0
        || change.nonce == 0
        || hex::decode(&change.delegate_public_key)
            .map_err(|e| e.to_string())?
            .len()
            != 32
    {
        return Err("delegation identity, history or generation mismatch".into());
    }
    if let Some(previous) = world
        .capability_revocation_state()
        .agent_signer_delegations
        .get(&change.agent_id)
    {
        if previous == change {
            return Ok(());
        }
        if change.generation <= previous.generation || change.nonce <= previous.nonce {
            return Err("delegation generation or nonce did not advance".into());
        }
    }
    Ok(())
}

pub fn validate_cognition(
    world: &World,
    signed: &SignedReadRequest<CognitionIntentV1>,
) -> Result<(), String> {
    verify_read_request("cognition", signed)?;
    let intent = &signed.request;
    if matches!(
        intent.action,
        crate::runtime::Action::WorldServiceIntent { .. }
    ) || crate::consensus_action_payload::main_token_action_auth_required(&intent.action)
    {
        return Err(
            "cognition cannot authorize nested service or main-token controller actions".into(),
        );
    }
    intent.request.validate().map_err(|e| e.to_string())?;
    intent
        .response_artifact
        .validate_for_request(&intent.request)
        .map_err(|e| e.to_string())?;
    validate_agent_signer(
        world,
        &intent.request.agent_id,
        &signed.subject_public_key,
        intent.delegation_generation,
    )
}

pub fn validate_agent_signer(
    world: &World,
    agent_id: &str,
    public_key: &str,
    generation: u64,
) -> Result<(), String> {
    if public_key == owner_public_key(world, agent_id)? {
        if generation != 0 {
            return Err("owner proof has delegate generation".into());
        }
        return Ok(());
    }
    let delegation = world
        .capability_revocation_state()
        .agent_signer_delegations
        .get(agent_id)
        .ok_or("provider signer has no canonical delegation")?;
    let identity = &world.capability_revocation_state().agent_identities[agent_id];
    let binding = world
        .current_cognition_runtime_binding()
        .map_err(|e| format!("{e:?}"))?;
    if delegation.revoked
        || delegation.delegate_public_key != public_key
        || delegation.generation != generation
        || delegation.owner_binding != identity.owner_binding
        || delegation.agent_identity_generation != identity.generation
        || delegation.world_id != binding.world_id
        || delegation.branch_id != binding.branch_id
    {
        return Err("provider delegation revoked or fenced".into());
    }
    Ok(())
}
