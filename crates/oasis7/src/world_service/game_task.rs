//! Durable owner task bootstrap derived exclusively from canonical intent results.
use super::*;
use crate::runtime::World;
use oasis7_agent_api::game::{DecimalU64, GAME_TASK_CREATE_DOMAIN, GameTaskStatusV1};
pub use oasis7_agent_api::game::{GameTaskControlV1, GameTaskCreateV1};

pub fn validate_owner(
    world: &World,
    signed: &SignedReadRequest<GameTaskCreateV1>,
) -> Result<(), String> {
    verify_read_request(GAME_TASK_CREATE_DOMAIN, signed)?;
    signed.request.validate()?;
    let request = &signed.request;
    if agent_authority::owner_public_key(world, &request.agent_id)? != signed.subject_public_key {
        return Err("Game task requires canonical owner signature".into());
    }
    let identity = &world.capability_revocation_state().agent_identities[&request.agent_id];
    let binding = world
        .current_cognition_runtime_binding()
        .map_err(|e| format!("{e:?}"))?;
    if request.owner_binding != identity.owner_binding
        || request.owner_generation.0 != identity.generation
        || request.world_id != binding.world_id
        || request.branch_id != binding.branch_id
        || request.reorg_epoch.0 != binding.reorg_epoch
        || world
            .chain_resource_manifest()
            .genesis_ref
            .as_ref()
            .is_some_and(|genesis| genesis != &request.genesis_digest)
    {
        return Err("Game task owner or history fence mismatch".into());
    }
    Ok(())
}

fn control(
    request: &SubmitIntentRequest<WorldServicePayloadV1>,
) -> Result<GameTaskControlV1, String> {
    let WorldServicePayloadV1::GameTaskCreate(signed) = &request.signed_payload else {
        return Err("not a Game task create".into());
    };
    let value = &signed.request;
    Ok(GameTaskControlV1 {
        schema_revision: 1,
        task_id: format!(
            "game-task-{}",
            request_digest("game-task-id", &request.correlation.key)?
        ),
        world_id: value.world_id.clone(),
        genesis_digest: value.genesis_digest.clone(),
        branch_id: value.branch_id.clone(),
        reorg_epoch: value.reorg_epoch,
        agent_id: value.agent_id.clone(),
        owner_binding: value.owner_binding.clone(),
        owner_generation: value.owner_generation,
        owner_public_key: signed.subject_public_key.clone(),
        task_revision: DecimalU64(1),
        task_status: GameTaskStatusV1::Suspended,
        latest_goal_revision: DecimalU64(0),
        active_goal_revision: DecimalU64(0),
        executor_epoch: DecimalU64(0),
        executor_id: None,
        created_operation_id: value.client_operation_id.clone(),
    })
}

pub(super) fn validate_result(result: &CanonicalIntentResultV1) -> Result<(), String> {
    if !matches!(
        result.request.signed_payload,
        WorldServicePayloadV1::GameTaskCreate(_)
    ) {
        return Ok(());
    }
    if result.rejected.is_some() {
        if !result.receipt.is_null() {
            return Err("rejected Game task has a success receipt".into());
        }
    } else if result.receipt
        != serde_json::to_value(control(&result.request)?).map_err(|e| e.to_string())?
    {
        return Err("invalid canonical Game task control receipt".into());
    }
    Ok(())
}

/// Returns current owner/history control only; malformed or ambiguous authority fails closed.
pub fn current_control(world: &World, agent_id: &str) -> Result<Option<GameTaskControlV1>, String> {
    let mut current = None;
    for (key, value) in &world.capability_revocation_state().world_service_results {
        correlation::validate_result(key, value)?;
        let result: CanonicalIntentResultV1 =
            serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        let WorldServicePayloadV1::GameTaskCreate(signed) = &result.request.signed_payload else {
            continue;
        };
        if result.rejected.is_some() || signed.request.agent_id != agent_id {
            continue;
        }
        let identity = world
            .capability_revocation_state()
            .agent_identities
            .get(agent_id)
            .ok_or("Game task current owner identity unavailable")?;
        let binding = world
            .current_cognition_runtime_binding()
            .map_err(|e| format!("{e:?}"))?;
        let owner_key = agent_authority::owner_public_key(world, agent_id)?;
        if signed.request.owner_binding != identity.owner_binding
            || signed.request.owner_generation.0 != identity.generation
            || signed.subject_public_key != owner_key
            || signed.request.world_id != binding.world_id
            || signed.request.branch_id != binding.branch_id
            || signed.request.reorg_epoch.0 != binding.reorg_epoch
            || world
                .chain_resource_manifest()
                .genesis_ref
                .as_ref()
                .is_some_and(|genesis| genesis != &signed.request.genesis_digest)
        {
            continue;
        }
        validate_owner(world, signed)?;
        if current.is_some() {
            return Err("ambiguous canonical Game task control".into());
        }
        current = Some(control(&result.request)?);
    }
    Ok(current)
}

/// Pure execution preparation. Staged results participate in uniqueness before journal publication.
pub fn prepare_create<'a>(
    world: &World,
    request: &SubmitIntentRequest<WorldServicePayloadV1>,
    staged: impl IntoIterator<Item = &'a CanonicalIntentResultV1>,
) -> Result<serde_json::Value, String> {
    let WorldServicePayloadV1::GameTaskCreate(signed) = &request.signed_payload else {
        return Err("not Game task create".into());
    };
    validate_owner(world, signed)?;
    if derive_correlation(
        request.correlation.key.world.clone(),
        &request.signed_payload,
    )? != request.correlation
    {
        return Err("Game task correlation mismatch".into());
    }
    if current_control(world, &signed.request.agent_id)?.is_some() {
        return Err("Game task already exists".into());
    }
    for result in staged {
        validate_result(result)?;
        if let WorldServicePayloadV1::GameTaskCreate(previous) = &result.request.signed_payload
            && result.rejected.is_none()
            && previous.request.agent_id == signed.request.agent_id
            && validate_owner(world, previous).is_ok()
        {
            return Err("Game task already exists within block".into());
        }
    }
    serde_json::to_value(control(request)?).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests;
