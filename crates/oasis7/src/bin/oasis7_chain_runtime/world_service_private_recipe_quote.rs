//! Pure quote over the same authenticated canonical pin as the captured task.
use super::*;
use oasis7::world_service::task_recipe_quote::*;

pub(super) fn process(
    service: &Service<'_>,
    store: &PrivateGoalStore,
    body: &[u8],
    before_delivery: impl FnOnce(),
) -> Result<Value, PrivateError> {
    let proof: SignedReadRequest<TaskRecipeQuoteRequestV1> =
        parse(body).map_err(|_| PrivateError::Invalid)?;
    verify_recipe_quote_request(&proof).map_err(|_| PrivateError::Invalid)?;
    let context_proof = &proof.request.context_proof;
    let fence = &context_proof.request.context.fence;
    let pinned = service.pin(None)?;
    let task = validate_pinned_task(service, &pinned, fence, &proof.subject_public_key, true)?;
    let pinned = task.pinned;
    if pinned
        .world
        .current_cognition_base_binding()
        .map_err(|_| PrivateError::Unavailable)?
        != proof.request.expected_base_binding
    {
        return Err(PrivateError::Conflict);
    }
    let captured = load_captured(service, store, context_proof, &task)?;
    let quote = pinned
        .world
        .builtin_recipe_quote(
            &fence.agent_id,
            &proof.request.factory_id,
            &proof.request.recipe_id,
            proof.request.batches,
        )
        .map_err(|_| PrivateError::Forbidden)?;
    let output = TaskRecipeQuoteV1 {
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
    let fresh = service.pin(None)?;
    let fresh_task = validate_pinned_task(service, &fresh, fence, &proof.subject_public_key, true)?;
    if fresh_task.pinned.commit != pinned.commit {
        return Err(PrivateError::Conflict);
    }
    fresh_task
        .history
        .validate_prefix_capture(&context_proof.request.context, &proof.subject_public_key)
        .map_err(|_| PrivateError::Conflict)?;
    check_record_live(fresh_task.pinned, &captured.prepared, None)?;
    service
        .signed(TASK_RECIPE_QUOTE_PATH, &proof, output)
        .map_err(PrivateError::from)
}
