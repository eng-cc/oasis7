//! Native confidential task input; canonical prefix admission is separate.
use super::*;
use oasis7::world_service::private_goal_store::PrivateTaskContextError;
use oasis7::world_service::private_task_context::*;
use oasis7_agent_api::game::{GameTaskFenceV1, TaskContextIdentityV1};
fn pinned(
    service: &Service<'_>,
    fence: &GameTaskFenceV1,
    subject: &str,
    executor: bool,
) -> Result<PinnedWorld, PrivateError> {
    let pinned = service.pin(None)?;
    if fence.world_id != service.identity.world_id
        || fence.genesis_digest != service.identity.genesis_digest
    {
        return Err(PrivateError::Forbidden);
    }
    if executor {
        game_task::validate_task_executor(&pinned.world, fence, subject)
    } else {
        game_task::validate_task_reader(&pinned.world, fence, subject)
    }
    .map_err(|_| PrivateError::Forbidden)?;
    Ok(pinned)
}
fn snapshot(
    service: &Service<'_>,
    pinned: &PinnedWorld,
    fence: &GameTaskFenceV1,
    store: &PrivateGoalStore,
) -> Result<GoalExecutionSnapshotV1, PrivateError> {
    let control = game_task::current_control(&pinned.world, &fence.agent_id)?
        .ok_or(PrivateError::Forbidden)?;
    let publication = game_task::active_publish(&pinned.world, &fence.agent_id)?
        .ok_or(PrivateError::Unavailable)?;
    let WorldServicePayloadV1::GameGoalPublish(public) = &publication.request.signed_payload else {
        return Err(PrivateError::Recovery);
    };
    let historical = service.pin_at_height(publication.committed_height)?;
    let key = correlation::key_digest(&publication.request.correlation.key)?;
    if historical
        .world
        .capability_revocation_state()
        .world_service_results
        .get(&key)
        != Some(&serde_json::to_value(&publication).map_err(|_| PrivateError::Recovery)?)
    {
        return Err(PrivateError::Recovery);
    }
    let active_goal = store
        .get_prepared(&public.request)
        .map_err(|_| PrivateError::Recovery)?;
    let provider_authority =
        oasis7::viewer::ViewerRuntimeLiveServer::canonical_agent_service_context(
            &pinned.world,
            &fence.agent_id,
        )?;
    let actor = pinned
        .world
        .state()
        .agents
        .get(&fence.agent_id)
        .ok_or(PrivateError::Forbidden)?;
    let runtime_binding = pinned
        .world
        .current_cognition_runtime_binding()
        .map_err(|_| PrivateError::Unavailable)?;
    let output = GoalExecutionSnapshotV1 {
        schema_revision: 1,
        control,
        read_commit: pinned.commit.clone(),
        activation_commit: historical.commit,
        active_goal,
        actor_observation_schema: "world-agent-state/v1".into(),
        actor_observation: serde_json::to_value(actor).map_err(|_| PrivateError::Unavailable)?,
        provider_authority,
        runtime_binding,
        captured_base_binding: pinned
            .world
            .current_cognition_base_binding()
            .map_err(|_| PrivateError::Unavailable)?,
    };
    output.validate().map_err(|_| PrivateError::Recovery)?;
    Ok(output)
}
fn task_error(error: PrivateTaskContextError) -> PrivateError {
    match error {
        PrivateTaskContextError::Invalid => PrivateError::Invalid,
        PrivateTaskContextError::Conflict => PrivateError::Conflict,
        PrivateTaskContextError::Unavailable => PrivateError::Recovery,
    }
}
fn check_record_live(
    pinned: &PinnedWorld,
    record: &PreparedTaskContextV1,
) -> Result<(), PrivateError> {
    if pinned.world.state().time > record.valid_until_tick.0 {
        return Err(PrivateError::Conflict);
    }
    let current = oasis7::viewer::ViewerRuntimeLiveServer::canonical_agent_service_context(
        &pinned.world,
        &record.identity.fence.agent_id,
    )?;
    if current.capability_authorization_root
        != record
            .snapshot
            .provider_authority
            .capability_authorization_root
        || current.authority_context_hash
            != record.snapshot.provider_authority.authority_context_hash
        || current.capability_snapshot_hash
            != record.snapshot.provider_authority.capability_snapshot_hash
    {
        return Err(PrivateError::Conflict);
    }
    Ok(())
}
fn capture_result(
    pinned: &PinnedWorld,
    context: &TaskContextIdentityV1,
    subject: &str,
) -> Result<CanonicalIntentResultV1, PrivateError> {
    // Public canonical records bind the opaque identity; no private data consulted.
    let mut selected = None;
    for (key, value) in &pinned
        .world
        .capability_revocation_state()
        .world_service_results
    {
        correlation::validate_result(key, value)?;
        let result: CanonicalIntentResultV1 =
            serde_json::from_value(value.clone()).map_err(|_| PrivateError::Recovery)?;
        let WorldServicePayloadV1::Scheduler(signed) = &result.request.signed_payload else {
            continue;
        };
        let SchedulerOperationV1::TaskProviderPrefix { context: captured } =
            &signed.request.operation
        else {
            continue;
        };
        if result.rejected.is_none()
            && captured == context
            && signed.subject_public_key == subject
            && selected.replace(result).is_some()
        {
            return Err(PrivateError::Recovery);
        }
    }
    selected.ok_or(PrivateError::Conflict)
}
pub(super) fn process(
    service: &Service<'_>,
    store: &PrivateGoalStore,
    path: &str,
    body: &[u8],
    before_delivery: impl FnOnce(),
) -> Result<Value, PrivateError> {
    match path {
        TASK_SNAPSHOT_PATH => {
            let proof: SignedReadRequest<TaskGoalSnapshotRequestV1> =
                parse(body).map_err(|_| PrivateError::Invalid)?;
            verify_snapshot_request(&proof).map_err(|_| PrivateError::Invalid)?;
            let pinned = pinned(
                service,
                &proof.request.fence,
                &proof.subject_public_key,
                false,
            )?;
            let output = snapshot(service, &pinned, &proof.request.fence, store)?;
            before_delivery();
            let fresh = pinned_check(
                service,
                &proof.request.fence,
                &proof.subject_public_key,
                false,
            )?;
            if fresh.commit.binding != pinned.commit.binding {
                return Err(PrivateError::Conflict);
            }
            service
                .signed(path, &proof, output)
                .map_err(PrivateError::from)
        }
        TASK_CONTEXT_PREPARE_PATH => {
            let proof: SignedReadRequest<TaskContextPrepareV1> =
                parse(body).map_err(|_| PrivateError::Invalid)?;
            verify_prepare(&proof).map_err(|_| PrivateError::Invalid)?;
            let pinned = pinned(
                service,
                &proof.request.fence,
                &proof.subject_public_key,
                true,
            )?;
            let record = match store.lookup_task_context(&proof).map_err(task_error)? {
                Some(record) => record,
                None => store
                    .prepare_task_context(
                        &proof,
                        snapshot(service, &pinned, &proof.request.fence, store)?,
                    )
                    .map_err(task_error)?,
            };
            check_record_live(&pinned, &record)?;
            before_delivery();
            let fresh = pinned_check(
                service,
                &proof.request.fence,
                &proof.subject_public_key,
                true,
            )?;
            check_record_live(&fresh, &record)?;
            service
                .signed(path, &proof, record)
                .map_err(PrivateError::from)
        }
        TASK_CONTEXT_READ_PATH => {
            let proof: SignedReadRequest<TaskContextGetV1> =
                parse(body).map_err(|_| PrivateError::Invalid)?;
            verify_get(&proof).map_err(|_| PrivateError::Invalid)?;
            let pinned = pinned(
                service,
                &proof.request.context.fence,
                &proof.subject_public_key,
                true,
            )?;
            game_task::validate_task_context_capture(
                &pinned.world,
                &proof.request.context,
                &proof.subject_public_key,
            )
            .map_err(|_| PrivateError::Conflict)?;
            let result =
                capture_result(&pinned, &proof.request.context, &proof.subject_public_key)?;
            let historical = service.pin_at_height(result.committed_height)?;
            let key = correlation::key_digest(&result.request.correlation.key)?;
            if historical
                .world
                .capability_revocation_state()
                .world_service_results
                .get(&key)
                != Some(&serde_json::to_value(&result).map_err(|_| PrivateError::Recovery)?)
            {
                return Err(PrivateError::Recovery);
            }
            let record = store.get_task_context(&proof).map_err(task_error)?;
            check_record_live(&pinned, &record)?;
            let output = CapturedTaskContextV1 {
                schema_revision: 1,
                prepared: record,
                capture_commit: historical.commit,
                capture_proof: result,
                current_public_context: TaskPublicContextV1 {
                    read_commit: pinned.commit.clone(),
                    captured_base_binding: pinned
                        .world
                        .current_cognition_base_binding()
                        .map_err(|_| PrivateError::Unavailable)?,
                    provider_authority:
                        oasis7::viewer::ViewerRuntimeLiveServer::canonical_agent_service_context(
                            &pinned.world,
                            &proof.request.context.fence.agent_id,
                        )?,
                    actor_observation: serde_json::to_value(
                        pinned
                            .world
                            .state()
                            .agents
                            .get(&proof.request.context.fence.agent_id)
                            .ok_or(PrivateError::Forbidden)?,
                    )
                    .map_err(|_| PrivateError::Unavailable)?,
                },
            };
            output
                .validate_against(&proof)
                .map_err(|_| PrivateError::Recovery)?;
            before_delivery();
            let fresh = pinned_check(
                service,
                &proof.request.context.fence,
                &proof.subject_public_key,
                true,
            )?;
            if fresh.commit.binding != pinned.commit.binding {
                return Err(PrivateError::Conflict);
            }
            game_task::validate_task_context_capture(
                &fresh.world,
                &proof.request.context,
                &proof.subject_public_key,
            )
            .map_err(|_| PrivateError::Conflict)?;
            check_record_live(&fresh, &output.prepared)?;
            service
                .signed(path, &proof, output)
                .map_err(PrivateError::from)
        }
        _ => Err(PrivateError::Invalid),
    }
}
fn pinned_check(
    service: &Service<'_>,
    fence: &GameTaskFenceV1,
    subject: &str,
    executor: bool,
) -> Result<PinnedWorld, PrivateError> {
    pinned(service, fence, subject, executor)
}
