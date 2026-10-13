//! Pure quote over the same authenticated canonical pin as the captured task.
use super::*;
use oasis7::world_service::task_factory_quote::*;

pub(super) fn process(
    service: &Service<'_>,
    store: &PrivateGoalStore,
    body: &[u8],
    before_delivery: impl FnOnce(),
) -> Result<Value, PrivateError> {
    let proof: SignedReadRequest<TaskFactoryQuoteRequestV1> =
        parse(body).map_err(|_| PrivateError::Invalid)?;
    verify_factory_quote_request(&proof).map_err(|_| PrivateError::Invalid)?;
    let context_proof = &proof.request.context_proof;
    let fence = &context_proof.request.context.fence;
    let pinned = pinned(service, fence, &proof.subject_public_key, true)?;
    if pinned
        .world
        .current_cognition_base_binding()
        .map_err(|_| PrivateError::Unavailable)?
        != proof.request.expected_base_binding
    {
        return Err(PrivateError::Conflict);
    }
    let captured = load_captured(service, store, context_proof, &pinned)?;
    let quote = pinned
        .world
        .builtin_factory_quote(
            &fence.agent_id,
            &proof.request.site_id,
            &proof.request.factory_kind,
            &proof.request.factory_id,
        )
        .map_err(|_| PrivateError::Forbidden)?;
    let output = TaskFactoryQuoteV1 {
        schema_revision: 1,
        context: context_proof.request.context.clone(),
        read_commit: pinned.commit.clone(),
        current_public_context: captured.current_public_context.clone(),
        quote,
    };
    output
        .validate_against(&proof)
        .map_err(|_| PrivateError::Recovery)?;
    before_delivery();
    let fresh = pinned_check(service, fence, &proof.subject_public_key, true)?;
    if fresh.commit != pinned.commit {
        return Err(PrivateError::Conflict);
    }
    game_task::validate_task_context_capture(
        &fresh.world,
        &context_proof.request.context,
        &proof.subject_public_key,
    )
    .map_err(|_| PrivateError::Conflict)?;
    check_record_live(&fresh, &captured.prepared)?;
    service
        .signed(TASK_FACTORY_QUOTE_PATH, &proof, output)
        .map_err(PrivateError::from)
}
