use super::World;
use crate::world_service::{
    AgentSignerDelegationChangeV1, CognitionIntentV1, SignedReadRequest, agent_authority,
};
#[path = "world_service_scheduler.rs"]
mod scheduler;

pub(super) fn apply_service_authorization_event(
    world: &mut World,
    event: &crate::runtime::CapabilityAuthorizationEvent,
) -> Result<(), crate::runtime::WorldError> {
    use crate::runtime::CapabilityAuthorizationEvent as Event;
    let deny = |reason: String| crate::runtime::WorldError::DistributedValidationFailed { reason };
    match event {
        Event::AgentSignerDelegationInstalled { signed } => {
            agent_authority::validate_delegation(world, signed).map_err(deny)?;
            world
                .capability_revocation_state
                .agent_signer_delegations
                .insert(signed.request.agent_id.clone(), signed.request.clone());
        }
        Event::WorldServiceIntentRecorded { key, record } => {
            crate::world_service::correlation::validate_result(key, record).map_err(deny)?;
            if world
                .capability_revocation_state
                .world_service_results
                .get(key)
                .is_some_and(|previous| previous != record)
            {
                return Err(deny("canonical intent result conflict".into()));
            }
            world
                .capability_revocation_state
                .world_service_results
                .insert(key.clone(), record.clone());
        }
        _ => return Err(deny("unsupported service authorization event".into())),
    }
    Ok(())
}

impl World {
    pub fn service_continuation_context(&self, continuation_id: &str) -> Option<serde_json::Value> {
        self.cognition
            .get("continuation_contexts")?
            .get(continuation_id)
            .cloned()
    }
    pub fn record_world_service_result(
        &mut self,
        result: crate::world_service::CanonicalIntentResultV1,
    ) -> Result<(), String> {
        let key = crate::world_service::correlation::key_digest(&result.request.correlation.key)?;
        let record = serde_json::to_value(result).map_err(|e| e.to_string())?;
        self.append_capability_authorization_event_batch(vec![
            crate::runtime::CapabilityAuthorizationEvent::WorldServiceIntentRecorded {
                key,
                record,
            },
        ])
        .map(|_| ())
        .map_err(|e| format!("{e:?}"))
    }
    /// Called only by registered canonical execution; HTTP admission uses the
    /// pure verifier and never invokes this mutation.
    pub fn apply_agent_signer_delegation(
        &mut self,
        signed: &SignedReadRequest<AgentSignerDelegationChangeV1>,
    ) -> Result<(), String> {
        agent_authority::validate_delegation(self, signed)?;
        if self
            .capability_revocation_state
            .agent_signer_delegations
            .get(&signed.request.agent_id)
            == Some(&signed.request)
        {
            return Ok(());
        }
        self.append_capability_authorization_event_batch(vec![
            crate::runtime::CapabilityAuthorizationEvent::AgentSignerDelegationInstalled {
                signed: signed.clone(),
            },
        ])
        .map_err(|e| format!("{e:?}"))?;
        Ok(())
    }

    pub fn commit_authenticated_cognition(
        &mut self,
        signed: SignedReadRequest<CognitionIntentV1>,
    ) -> Result<serde_json::Value, String> {
        agent_authority::validate_cognition(self, &signed)?;
        let intent = signed.request;
        let (record, lineage) = self
            .commit_cognition_action(intent.request, intent.action, intent.response_artifact)
            .map_err(|e| format!("{e:?}"))?;
        self.verify_runtime_receipt_lineage(&lineage)
            .map_err(|e| format!("{e:?}"))?;
        let action_id = lineage
            .action_id
            .strip_prefix("action:")
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or("canonical receipt action identity invalid")?;
        // Match the local provider path: the World allocator verifies this
        // committed lineage and durably assigns the feedback sequence. Exact
        // replays return the same outbox entry, never a caller-minted receipt.
        let feedback = self
            .allocate_runtime_feedback_with_projection(
                crate::runtime::RuntimeFeedbackRequestV1 {
                    feedback_id: Some(lineage.feedback_id.clone()),
                    agent_subject: lineage.agent_id.clone(),
                    agent_session_id: lineage.agent_session_id.clone(),
                    agent_turn_id: lineage.agent_turn_id.clone(),
                    decision_request_id: lineage.decision_request_id.clone(),
                    candidate_action_id: Some(action_id),
                    runtime_receipt_id: Some(lineage.receipt_id.clone()),
                    status: lineage.status.clone(),
                    request_digest: lineage.request_digest.clone(),
                    reject_reason: None,
                },
                crate::runtime::RuntimeFeedbackProjectionV1 {
                    envelope_digest: Some(lineage.envelope_digest.clone()),
                    ..Default::default()
                },
            )
            .map_err(|e| format!("{e:?}"))?;
        feedback.validate().map_err(|e| e.to_string())?;
        Ok(
            serde_json::json!({"commit_record": record, "lineage": lineage, "feedback": feedback.payload}),
        )
    }
}
