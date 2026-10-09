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

    pub fn apply_authenticated_feedback_ack(
        &mut self,
        signed: SignedReadRequest<crate::world_service::FeedbackAckIntentV1>,
    ) -> Result<serde_json::Value, String> {
        agent_authority::validate_feedback_ack(self, &signed)?;
        let ack = signed.request;
        let record = self
            .runtime_feedback_outbox()
            .map_err(|e| format!("{e:?}"))?
            .into_iter()
            .find(|r| r.feedback_id == ack.feedback_id)
            .ok_or("canonical feedback acknowledgement record missing")?;
        // Native claim/ack events are published in one candidate transaction;
        // never assign delivery state on a caller's sanitized projection.
        let mut transaction = self.clone();
        if record.state != "acked" {
            transaction
                .claim_runtime_feedback(&ack.feedback_id)
                .map_err(|e| format!("{e:?}"))?;
            transaction
                .ack_runtime_feedback(&ack.feedback_id)
                .map_err(|e| format!("{e:?}"))?;
        }
        *self = transaction;
        Ok(serde_json::json!({"acknowledged": ack, "delivery_state": "acked"}))
    }

    pub fn commit_authenticated_cognition(
        &mut self,
        signed: SignedReadRequest<CognitionIntentV1>,
    ) -> Result<serde_json::Value, String> {
        agent_authority::validate_cognition(self, &signed)?;
        let intent = signed.request;
        // The outer service correlation already binds the signed payload. This
        // additional canonical fence prevents a differently signed explanation
        // from reusing a cognition decision whose request digest predates it.
        let identity = serde_json::json!([
            intent.request.agent_id,
            intent.request.agent_session_id,
            intent.request.agent_turn_id,
            intent.request.decision_request_id,
            intent.request.request_digest,
        ]);
        let key = identity.to_string();
        let proposal = serde_json::to_value(&intent.causal_proposal).map_err(|e| e.to_string())?;
        let stored = self
            .cognition
            .get("service_causal_proposals")
            .and_then(|map| map.get(&key));
        if stored.is_some_and(|old| old != &proposal) {
            return Err("canonical cognition causal proposal idempotency conflict".into());
        }
        let committed = self
            .cognition
            .get("commit_records")
            .and_then(serde_json::Value::as_array)
            .is_some_and(|records| {
                records.iter().any(|record| {
                    record.get("agent_id") == Some(&identity[0])
                        && record.get("agent_session_id") == Some(&identity[1])
                        && record.get("agent_turn_id") == Some(&identity[2])
                        && record.get("decision_request_id") == Some(&identity[3])
                        && record.get("request_digest") == Some(&identity[4])
                        && record.get("status").and_then(serde_json::Value::as_str)
                            == Some("committed")
                })
            });
        // A legacy commit has no signed causal proposal. It cannot be upgraded
        // retroactively by a replay that introduces a new explanation.
        if committed && stored.is_none() && intent.causal_proposal.is_some() {
            return Err("canonical cognition causal proposal missing for prior commit".into());
        }
        if !committed {
            let mut context = crate::runtime::AgentDecisionCausalContextV1::default();
            if let Some(public) = &intent.causal_proposal {
                context.expected_consequence = public.expected_consequence.clone();
                context.stakes = public.stakes.clone();
                context.alternative = public.alternative.clone();
                context.reason = public.reason.clone();
                context.evidence_refs = public.evidence_refs.clone();
                context.correction_refs = public.correction_refs.clone();
                context.dissent = public.dissent.clone();
            }
            context.intent_id = self
                .state()
                .agents
                .get(&intent.request.agent_id)
                .and_then(|agent| agent.intent.as_ref())
                .filter(|intent| matches!(intent.status.as_str(), "accepted" | "blocked"))
                .map(|intent| intent.intent_id.clone());
            self.bind_agent_causal_decision(
                &intent.request.decision_request_id,
                &intent.request.agent_id,
                &intent.action,
                context,
            )
            .map_err(|e| format!("{e:?}"))?;
        }
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
        // This is persisted in the same canonical candidate as the delegation
        // decision, cognition receipt and feedback; rejected candidates vanish.
        let object = self
            .cognition
            .as_object_mut()
            .ok_or("canonical cognition projection invalid")?;
        let proposals = object
            .entry("service_causal_proposals")
            .or_insert_with(|| serde_json::json!({}))
            .as_object_mut()
            .ok_or("canonical causal proposal index invalid")?;
        proposals.insert(key, proposal);
        Ok(
            serde_json::json!({"commit_record": record, "lineage": lineage, "feedback": feedback.payload}),
        )
    }
}
