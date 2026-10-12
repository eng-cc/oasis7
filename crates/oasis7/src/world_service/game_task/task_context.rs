//! Public task context capture and action metadata. Private input stays off-ledger.
use super::*;
/// Validates the public deadline against the live canonical capability catalog.
pub fn validate_task_context_identity(
    world: &World,
    context: &oasis7_agent_api::game::TaskContextIdentityV1,
    subject: &str,
    generation: u64,
    staged: &[CanonicalIntentResultV1],
) -> Result<(), String> {
    context.validate()?;
    validate_cognition_fence_staged(
        world,
        &context.fence.agent_id,
        Some(&context.fence),
        subject,
        generation,
        staged,
    )?;
    let authority = crate::viewer::ViewerRuntimeLiveServer::canonical_agent_service_context(
        world,
        &context.fence.agent_id,
    )?;
    if world.state().time > context.valid_until_tick.0
        || context.valid_until_tick.0 > authority.capability_catalog.valid_until_tick
    {
        return Err("task context expired or exceeds canonical grant expiry".into());
    }
    Ok(())
}

fn validate_context_capture_staged(
    world: &World,
    context: &oasis7_agent_api::game::TaskContextIdentityV1,
    subject: &str,
    staged: &[CanonicalIntentResultV1],
) -> Result<(), String> {
    validate_context_capture_inner(world, context, subject, staged, false)
}
fn validate_context_capture_inner(
    world: &World,
    context: &oasis7_agent_api::game::TaskContextIdentityV1,
    subject: &str,
    staged: &[CanonicalIntentResultV1],
    allow_expired: bool,
) -> Result<(), String> {
    let generation = context
        .fence
        .executor_binding
        .as_ref()
        .ok_or("task executor unavailable")?
        .delegation_generation
        .0;
    if allow_expired {
        context.validate()?;
        validate_cognition_fence_staged(
            world,
            &context.fence.agent_id,
            Some(&context.fence),
            subject,
            generation,
            staged,
        )?;
    } else {
        validate_task_context_identity(world, context, subject, generation, staged)?;
    }
    let mut matched = false;
    for (key, value) in &world.capability_revocation_state().world_service_results {
        correlation::validate_result(key, value)?;
        let result: CanonicalIntentResultV1 =
            serde_json::from_value(value.clone()).map_err(|_| "invalid task prefix result")?;
        matched |= matches_task_prefix(&result, context, subject);
    }
    for result in staged {
        correlation::validate_result(
            &correlation::key_digest(&result.request.correlation.key)?,
            &serde_json::to_value(result).map_err(|e| e.to_string())?,
        )?;
        matched |= matches_task_prefix(result, context, subject);
    }
    if !matched {
        return Err("task context has no canonical prefix".into());
    }
    world
        .validate_active_cognition_context(
            (
                &context.fence.agent_id,
                &context.agent_session_id,
                &context.agent_turn_id,
                &context.decision_request_id,
                &context.request_commitment,
            ),
            &context.context_commitment,
            &crate::runtime::AgentDecisionEnvelopeV1::provider_invocation_key_for_request(
                &context.request_commitment,
            ),
        )
        .map_err(|_| "task context is not active".to_string())
}
fn matches_task_prefix(
    result: &CanonicalIntentResultV1,
    context: &oasis7_agent_api::game::TaskContextIdentityV1,
    subject: &str,
) -> bool {
    if result.rejected.is_some() {
        return false;
    }
    matches!(&result.request.signed_payload, WorldServicePayloadV1::Scheduler(signed)
        if signed.subject_public_key == subject && matches!(&signed.request.operation,
            SchedulerOperationV1::TaskProviderPrefix { context: captured } if captured == context))
}
pub fn validate_task_context_capture(
    world: &World,
    context: &oasis7_agent_api::game::TaskContextIdentityV1,
    subject: &str,
) -> Result<(), String> {
    validate_context_capture_staged(world, context, subject, &[])
}
/// Pure public action envelope. No provider prompt, private goal digest or reasoning.
pub fn task_cognition_intent(
    world: &World,
    context: &oasis7_agent_api::game::TaskContextIdentityV1,
    action: crate::runtime::Action,
    subject: &str,
) -> Result<CognitionIntentV1, String> {
    validate_task_context_capture(world, context, subject)?;
    let authority = crate::viewer::ViewerRuntimeLiveServer::canonical_agent_service_context(
        world,
        &context.fence.agent_id,
    )?;
    let actor = world
        .state()
        .agents
        .get(&context.fence.agent_id)
        .ok_or("task actor unavailable")?;
    let actor = serde_json::to_value(actor).map_err(|e| e.to_string())?;
    task_cognition_intent_from_public_context(
        context,
        world
            .current_cognition_base_binding()
            .map_err(|e| format!("{e:?}"))?,
        &authority,
        &actor,
        action,
    )
}

pub fn task_cognition_intent_from_public_context(
    context: &oasis7_agent_api::game::TaskContextIdentityV1,
    binding: crate::runtime::RuntimeCognitionBaseBindingV1,
    authority: &projection::WorldServiceAgentContext,
    actor_observation: &serde_json::Value,
    action: crate::runtime::Action,
) -> Result<CognitionIntentV1, String> {
    context.validate()?;
    binding.validate().map_err(|e| e.to_string())?;
    if authority.agent_id != context.fence.agent_id
        || binding.world_id != context.fence.world_id
        || binding.branch_id != context.fence.branch_id
        || binding.reorg_epoch != context.fence.reorg_epoch.0
    {
        return Err("task action public authority mismatch".into());
    }
    let request = crate::runtime::RuntimeCognitionCommitRequestV1 {
        agent_id: context.fence.agent_id.clone(),
        agent_session_id: context.agent_session_id.clone(),
        agent_turn_id: context.agent_turn_id.clone(),
        decision_request_id: context.decision_request_id.clone(),
        retry_seq: context.retry_seq,
        transport_attempt: context.transport_attempt,
        request_digest: context.request_commitment.clone(),
        observation_digest: request_digest("game-task-public-observation/v1", actor_observation)?,
        context_digest: context.context_commitment.clone(),
        capability_snapshot_hash: authority.capability_snapshot_hash.clone(),
        authority_context_hash: authority.authority_context_hash.clone(),
        captured_base_binding: binding,
    };
    let mut response_artifact = crate::runtime::RuntimeCognitionResponseArtifactV1 {
        schema_version: 1,
        context_discriminator:
            crate::runtime::RuntimeCognitionResponseArtifactV1::CONTEXT_DISCRIMINATOR.into(),
        context_version: 1,
        agent_session_id: context.agent_session_id.clone(),
        agent_turn_id: context.agent_turn_id.clone(),
        decision_request_id: context.decision_request_id.clone(),
        retry_seq: context.retry_seq,
        transport_attempt: context.transport_attempt,
        request_digest: context.request_commitment.clone(),
        response_digest: request_digest(
            "game-task-public-action/v1",
            &(&context.request_commitment, &action),
        )?,
        artifact_digest: String::new(),
    };
    response_artifact.refresh_artifact_digest();
    request.validate().map_err(|e| e.to_string())?;
    response_artifact
        .validate_for_request(&request)
        .map_err(|e| e.to_string())?;
    Ok(CognitionIntentV1 {
        request,
        action,
        response_artifact,
        delegation_generation: context
            .fence
            .executor_binding
            .as_ref()
            .ok_or("task executor unavailable")?
            .delegation_generation
            .0,
        causal_proposal: None,
        task_fence: Some(context.fence.clone()),
    })
}
pub fn validate_task_commit_context(
    world: &World,
    request: &crate::runtime::RuntimeCognitionCommitRequestV1,
    fence: &oasis7_agent_api::game::GameTaskFenceV1,
    subject: &str,
    staged: &[CanonicalIntentResultV1],
) -> Result<(), String> {
    let mut prefixes = Vec::new();
    for (key, value) in &world.capability_revocation_state().world_service_results {
        correlation::validate_result(key, value)?;
        let result: CanonicalIntentResultV1 =
            serde_json::from_value(value.clone()).map_err(|_| "invalid task prefix")?;
        prefixes.push(result);
    }
    prefixes.extend_from_slice(staged);
    for result in prefixes {
        if result.rejected.is_some() {
            continue;
        }
        let WorldServicePayloadV1::Scheduler(signed) = &result.request.signed_payload else {
            continue;
        };
        let SchedulerOperationV1::TaskProviderPrefix { context } = &signed.request.operation else {
            continue;
        };
        if signed.subject_public_key == subject
            && context.fence == *fence
            && context.agent_session_id == request.agent_session_id
            && context.agent_turn_id == request.agent_turn_id
            && context.decision_request_id == request.decision_request_id
            && context.request_commitment == request.request_digest
            && context.context_commitment == request.context_digest
            && context.retry_seq == request.retry_seq
            && context.transport_attempt == request.transport_attempt
        {
            return validate_context_capture_staged(world, context, subject, staged);
        }
    }
    Err("task action has no matching canonical context".into())
}

/// Shared task gate for the core commit path, including legacy no-fence calls.
pub fn validate_task_cognition_admission(
    world: &World,
    request: &crate::runtime::RuntimeCognitionCommitRequestV1,
    authority: Option<(&oasis7_agent_api::game::GameTaskFenceV1, &str, u64)>,
    staged: &[CanonicalIntentResultV1],
) -> Result<(), String> {
    let (fence, subject, generation) = match authority {
        Some((fence, subject, generation)) => (Some(fence), subject, generation),
        None => (None, "", 0),
    };
    validate_cognition_fence_staged(world, &request.agent_id, fence, subject, generation, staged)?;
    if let Some(fence) = fence {
        validate_task_commit_context(world, request, fence, subject, staged)?;
    }
    Ok(())
}

pub fn validate_task_failure_context(
    world: &World,
    context: &oasis7_agent_api::game::TaskContextIdentityV1,
    subject: &str,
    generation: u64,
    reason: &str,
    staged: &[CanonicalIntentResultV1],
) -> Result<(), String> {
    if !matches!(
        reason,
        "provider_failed" | "transport_exhausted" | "context_expired" | "executor_aborted"
    ) {
        return Err("task failure requires bounded public code".into());
    }
    let expired = reason == "context_expired";
    if expired && world.state().time <= context.valid_until_tick.0 {
        return Err("task context has not expired".into());
    }
    if context
        .fence
        .executor_binding
        .as_ref()
        .is_none_or(|b| b.delegation_generation.0 != generation)
    {
        return Err("task failure delegation generation mismatch".into());
    }
    validate_context_capture_inner(world, context, subject, staged, expired)
}
