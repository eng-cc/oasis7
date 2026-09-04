use super::*;

/// Admit a provider Wait through the production Harness and Runtime seams.
/// The local Viewer timer is deliberately not a fallback: a wait is a durable
/// continuation only after both authorities accept the exact current
/// observation/goal/policy/precondition snapshot.
#[cfg(not(target_arch = "wasm32"))]
pub(in crate::viewer::runtime_live::control_plane::llm_sidecar) fn admit_provider_wait_continuation(
    sidecar: &mut RuntimeLlmSidecar,
    world: &mut RuntimeWorld,
    kernel: &mut WorldKernel,
    cognition: &RuntimeProviderActionContext,
) -> Result<(), String> {
    let request = &cognition.request.request_context;
    let observation = kernel
        .observe(request.agent_subject.as_str())
        .map_err(|error| format!("provider Wait observation failed: {error:?}"))?;
    let precondition_digest = provider_wait_precondition_digest(&observation);
    let current = crate::simulator::ContinuationCurrentContextV1::from_observation(
        observation,
        &cognition.request.turn_context.goal_snapshot,
        provider_policy_context_digest(request),
        precondition_digest,
    );
    current
        .validate_for_agent(request.agent_subject.as_str())
        .map_err(|error| format!("provider Wait current context invalid: {error}"))?;

    let wake_tick = world.state().time.saturating_add(1);
    let mut simulator = crate::simulator::ContinuationProposalV1 {
        schema_version: 1,
        continuation_proposal_id: format!(
            "provider-wait:{}:{}:{}",
            request.agent_subject, request.agent_turn_id, request.decision_request_id
        ),
        world_id: request.runtime_binding.world_id.clone(),
        agent_id: request.agent_subject.clone(),
        agent_session_id: request.agent_session_id.clone(),
        agent_turn_id: request.agent_turn_id.clone(),
        decision_request_id: request.decision_request_id.clone(),
        origin_turn_id: request.agent_turn_id.clone(),
        origin_request_digest: request.request_digest.to_string(),
        action_or_plan_kind: "wait".to_string(),
        action_or_envelope_digest: None,
        remaining_budget: crate::simulator::ContinuationBudgetV1 {
            unit: "ticks".to_string(),
            value: 2,
        },
        baseline_observation_digest: current.authority.baseline_observation_digest.clone(),
        goal_digest: current.authority.goal_digest.clone(),
        policy_digest: current.authority.policy_digest.clone(),
        policy_revision: cognition.request.turn_context.goal_snapshot.revision.max(1),
        precondition_summary: "provider wait until the next Runtime tick".to_string(),
        precondition_digest: current.authority.precondition_digest.clone(),
        wake_conditions: vec![crate::simulator::WakeConditionV1 {
            schema_version: "wake-condition.v1".to_string(),
            kind: "at_or_after_tick".to_string(),
            logical_tick: Some(wake_tick),
            event_digest: None,
            receipt_id: None,
            subject: None,
            path_or_rule: None,
            operator: None,
            expected_value_bytes: None,
        }],
        valid_until_tick: Some(wake_tick.saturating_add(16)),
        source: "provider_wait".to_string(),
        proposal_digest: String::new(),
    };
    simulator.proposal_digest = simulator
        .proposal_digest()
        .map_err(|error| format!("provider Wait Harness proposal invalid: {error}"))?
        .to_string();
    let runtime: crate::runtime::CognitionContinuationProposalV1 = serde_json::from_value(
        serde_json::to_value(&simulator)
            .map_err(|error| format!("provider Wait Runtime proposal encoding failed: {error}"))?,
    )
    .map_err(|error| format!("provider Wait Runtime proposal decoding failed: {error}"))?;
    // The paired schema uses the same canonical proposal digest. Runtime
    // fills branch/finality/manifest fields, which are intentionally excluded
    // from the digest domain.
    let mut runtime = runtime;
    runtime.proposal_digest = runtime.proposal_digest();

    let Some(runner) = sidecar
        .runner
        .as_mut()
        .and_then(RuntimeDecisionRunner::async_runner_mut)
    else {
        return Err("provider Wait runner is unavailable".to_string());
    };
    let handle = runner
        .submit_continuation_proposal_with_current_context(
            request.agent_subject.as_str(),
            simulator,
            &current,
        )
        .map_err(|error| format!("provider Wait Harness admission failed: {error}"))?;
    let admitted = match world.admit_cognition_continuation(runtime) {
        Ok(admitted) => admitted,
        Err(error) => {
            if let Some(runner) = sidecar
                .runner
                .as_mut()
                .and_then(RuntimeDecisionRunner::async_runner_mut)
            {
                let _ = runner.invalidate_continuation_for_agent(
                    request.agent_subject.as_str(),
                    crate::simulator::ContinuationInvalidationReason::Rejected,
                );
            }
            return Err(format!("provider Wait Runtime admission failed: {error:?}"));
        }
    };
    if let Some(runner) = sidecar
        .runner
        .as_mut()
        .and_then(RuntimeDecisionRunner::async_runner_mut)
    {
        runner
            .apply_runtime_continuation_projection_with_current_context(
                request.agent_subject.as_str(),
                admitted,
                &current,
            )
            .map_err(|error| {
                format!(
                    "provider Wait Runtime projection failed after admission: {error} (Harness handle {})",
                    handle.chain_id
                )
            })?;
        runner
            .release_runtime_turn_for_continuation(
                request.agent_subject.as_str(),
                request.agent_session_id.as_str(),
                request.agent_turn_id.as_str(),
                request.decision_request_id.as_str(),
            )
            .map_err(|error| format!("provider Wait actor turn release failed: {error}"))?;
    }
    sidecar
        .provider_active_turns
        .remove(request.agent_subject.as_str());
    sidecar
        .provider_contexts
        .remove(request.agent_subject.as_str());
    sidecar
        .provider_wait_until
        .remove(request.agent_subject.as_str());
    sidecar.persist_provider_lineage_best_effort();
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
impl RuntimeLlmSidecar {
    pub(in crate::viewer::runtime_live::control_plane::llm_sidecar) fn admit_provider_wait_continuation(
        &mut self,
        world: &mut RuntimeWorld,
        kernel: &mut WorldKernel,
        cognition: &RuntimeProviderActionContext,
    ) -> Result<(), String> {
        admit_provider_wait_continuation(self, world, kernel, cognition)
    }
}
