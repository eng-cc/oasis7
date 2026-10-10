//! Oasis-owned Runtime authority callbacks for the extracted agent harness.
//!
//! The Harness consumes only projections returned here after Oasis has
//! validated the corresponding durable Runtime record. This adapter never
//! creates leases, receipts, or continuation authority.

use oasis7_agent_api::{
    CognitionError, CognitionLeaseConsumptionViewV1, ContinuousAgentRequestContextV1,
    FeedbackEnvelopeV1, ResponseArtifactIdentityV1, RuntimeReceiptLineageV1,
};
use oasis7_agent_harness::{
    ContinuationHostProjection, ContinuationStatusProjectionV1, FeedbackHistoryLineage,
    FeedbackHistoryProjection, MemoryWritePolicyContextV1, RuntimeAuthority,
};

use crate::runtime::{
    AgentContinuation, CognitionLeaseStatusV1, CognitionLeaseV1,
    RuntimeCognitionResponseArtifactV1, RuntimeReceiptLineageV1 as RuntimeOwnedReceiptLineageV1,
    World as RuntimeWorld,
};
use crate::simulator::{
    FeedbackEnvelopeV1 as OasisFeedbackEnvelopeV1, agent_api_compat,
    validated_cognition_lease_consumption_view,
};

/// Read-only adapter over one actual Runtime World. Runtime owns this value's
/// lifetime; the Harness receives no reference to it and cannot mutate World.
pub(super) struct RuntimeWorldAuthority<'a> {
    world: &'a RuntimeWorld,
}

impl<'a> RuntimeWorldAuthority<'a> {
    pub(super) fn new(world: &'a RuntimeWorld) -> Self {
        Self { world }
    }

    fn receipt_lineage(
        &self,
        receipt: &RuntimeOwnedReceiptLineageV1,
    ) -> Result<RuntimeOwnedReceiptLineageV1, CognitionError> {
        receipt
            .validate()
            .map_err(|error| cognition_error("runtime_receipt_invalid", error))?;
        let actual = self
            .world
            .read_runtime_receipt_lineage(&receipt.receipt_id)
            .map_err(|error| {
                cognition_error("runtime_receipt_readback_failed", format!("{error:?}"))
            })?;
        if actual != *receipt {
            return Err(CognitionError::new(
                "runtime_receipt_readback_mismatch",
                "supplied receipt lineage differs from Runtime durable readback",
            ));
        }
        Ok(actual)
    }

    fn committed_feedback_lineage(
        &self,
        feedback: &OasisFeedbackEnvelopeV1,
    ) -> Result<RuntimeOwnedReceiptLineageV1, CognitionError> {
        let api_feedback = agent_api_compat::feedback_to_api(feedback);
        api_feedback
            .validate()
            .map_err(|error| cognition_error("runtime_feedback_invalid", error))?;
        if api_feedback.status != "committed" {
            return Err(CognitionError::new(
                "runtime_feedback_not_committed",
                "receipt lineage is required only for committed feedback",
            ));
        }
        let receipt_id = api_feedback.runtime_receipt_id.as_deref().ok_or_else(|| {
            CognitionError::new(
                "runtime_feedback_receipt_missing",
                "committed feedback has no Runtime receipt identity",
            )
        })?;
        let lineage = self
            .world
            .read_runtime_receipt_lineage(receipt_id)
            .map_err(|error| {
                cognition_error("runtime_receipt_readback_failed", format!("{error:?}"))
            })?;
        lineage
            .validate()
            .map_err(|error| cognition_error("runtime_receipt_invalid", error))?;
        if lineage.status != "committed"
            || lineage.receipt_id != receipt_id
            || lineage.agent_id != api_feedback.agent_subject
            || lineage.agent_session_id != api_feedback.agent_session_id
            || lineage.agent_turn_id != api_feedback.agent_turn_id
            || lineage.decision_request_id != api_feedback.decision_request_id
            || lineage.request_digest != api_feedback.request_digest.to_string()
            || lineage.feedback_id != api_feedback.feedback_id
            || api_feedback
                .candidate_action_id
                .is_none_or(|action_id| lineage.action_id != format!("action:{action_id}"))
        {
            return Err(CognitionError::new(
                "runtime_feedback_receipt_mismatch",
                "committed feedback does not match its Runtime receipt lineage",
            ));
        }
        Ok(lineage)
    }

    fn verify_response_identity(
        &self,
        lineage: &RuntimeOwnedReceiptLineageV1,
        identity: &ResponseArtifactIdentityV1,
    ) -> Result<(), CognitionError> {
        identity
            .validate()
            .map_err(|error| cognition_error("runtime_response_identity_invalid", error))?;
        let record = self
            .world
            .replay_cognition_response(&lineage.envelope_digest)
            .map_err(|error| {
                cognition_error("runtime_response_readback_failed", format!("{error:?}"))
            })?;
        let artifact: RuntimeCognitionResponseArtifactV1 =
            serde_json::from_value(record.response_artifact.ok_or_else(|| {
                CognitionError::new(
                    "runtime_response_artifact_missing",
                    "committed Runtime response has no durable artifact identity",
                )
            })?)
            .map_err(|error| cognition_error("runtime_response_artifact_invalid", error))?;
        artifact
            .validate()
            .map_err(|error| cognition_error("runtime_response_artifact_invalid", error))?;
        if artifact.schema_version != identity.schema_version
            || artifact.context_discriminator != identity.context_discriminator
            || artifact.context_version != identity.context_version
            || artifact.agent_session_id != identity.agent_session_id
            || artifact.agent_turn_id != identity.agent_turn_id
            || artifact.decision_request_id != identity.decision_request_id
            || artifact.retry_seq != identity.retry_seq
            || artifact.transport_attempt != identity.transport_attempt
            || artifact.request_digest != identity.request_digest.to_string()
            || artifact.response_digest != identity.response_digest.to_string()
            || artifact.artifact_digest != identity.artifact_digest.to_string()
        {
            return Err(CognitionError::new(
                "runtime_response_identity_readback_mismatch",
                "response artifact identity differs from durable Runtime readback",
            ));
        }
        Ok(())
    }
}

impl RuntimeAuthority for RuntimeWorldAuthority<'_> {
    type Lease = CognitionLeaseV1;
    type Receipt = RuntimeOwnedReceiptLineageV1;
    type Continuation = AgentContinuation;

    fn validate_lease(
        &self,
        lease: &Self::Lease,
        request: &ContinuousAgentRequestContextV1,
    ) -> Result<CognitionLeaseConsumptionViewV1, CognitionError> {
        request.validate_production_lane()?;
        let current_binding = self
            .world
            .current_cognition_runtime_binding()
            .map_err(|error| {
                cognition_error("runtime_binding_readback_failed", format!("{error:?}"))
            })?;
        if current_binding != request.runtime_binding {
            return Err(CognitionError::new(
                "runtime_binding_changed",
                "request Runtime binding no longer matches the current World",
            ));
        }
        let economy = self.world.cognition_economy().map_err(|error| {
            cognition_error("runtime_lease_readback_failed", format!("{error:?}"))
        })?;
        if economy.leases.get(&lease.lease_id) != Some(lease)
            || lease.status != CognitionLeaseStatusV1::Reserved
        {
            return Err(CognitionError::new(
                "runtime_lease_readback_mismatch",
                "lease is not the current reserved Runtime record",
            ));
        }
        validated_cognition_lease_consumption_view(
            request.agent_subject.as_str(),
            &agent_api_compat::request_context_from_api(request.clone()),
            lease,
            self.world.state().time,
        )
        .map_err(|error| cognition_error("runtime_lease_invalid", error))
    }

    fn verify_receipt_readback(
        &self,
        feedback: &FeedbackEnvelopeV1,
        receipt: &Self::Receipt,
        response_identity: &ResponseArtifactIdentityV1,
    ) -> Result<RuntimeReceiptLineageV1, CognitionError> {
        feedback
            .validate()
            .map_err(|error| cognition_error("runtime_feedback_invalid", error))?;
        let actual = self.receipt_lineage(receipt)?;
        let oasis_feedback = agent_api_compat::feedback_from_api(feedback.clone());
        let expected = self.committed_feedback_lineage(&oasis_feedback)?;
        if actual != expected {
            return Err(CognitionError::new(
                "runtime_receipt_feedback_mismatch",
                "receipt argument differs from the feedback's Runtime receipt",
            ));
        }
        self.verify_response_identity(&actual, response_identity)?;
        Ok(actual.agent_api_projection())
    }

    fn verify_memory_receipt(
        &self,
        receipt: &Self::Receipt,
        context: Option<&MemoryWritePolicyContextV1>,
    ) -> Result<RuntimeReceiptLineageV1, CognitionError> {
        let context = context.ok_or_else(|| {
            CognitionError::new(
                "runtime_memory_context_missing",
                "Runtime memory readback requires its provider turn context",
            )
        })?;
        if context.source != "provider" || context.provenance != "provider_unverified" {
            return Err(CognitionError::new(
                "runtime_memory_context_invalid",
                "memory intent is not from the unverified provider lane",
            ));
        }
        let actual = self.receipt_lineage(receipt)?;
        if actual.status != "committed"
            || actual.agent_id != context.agent_id
            || actual.agent_session_id != context.agent_session_id
            || actual.agent_turn_id != context.agent_turn_id
            || actual.request_digest != context.request_digest
        {
            return Err(CognitionError::new(
                "runtime_memory_receipt_mismatch",
                "committed receipt does not match the provider memory intent context",
            ));
        }
        Ok(actual.agent_api_projection())
    }

    fn validated_feedback_history(&self) -> Result<Vec<FeedbackHistoryProjection>, CognitionError> {
        let records = self.world.runtime_feedback_outbox().map_err(|error| {
            cognition_error(
                "runtime_feedback_history_readback_failed",
                format!("{error:?}"),
            )
        })?;
        records
            .into_iter()
            .map(|record| {
                record
                    .validate()
                    .map_err(|error| cognition_error("runtime_feedback_record_invalid", error))?;
                let feedback =
                    serde_json::from_value::<OasisFeedbackEnvelopeV1>(record.payload.clone())
                        .map_err(|error| {
                            cognition_error("runtime_feedback_payload_invalid", error)
                        })?;
                let receipt_lineage = if feedback.status == "committed" {
                    Some(
                        self.committed_feedback_lineage(&feedback)?
                            .agent_api_projection(),
                    )
                } else {
                    None
                };
                Ok(FeedbackHistoryProjection {
                    feedback: agent_api_compat::feedback_to_api(&feedback),
                    acknowledged: record.state == "acked",
                    lineage: Some(FeedbackHistoryLineage {
                        original_envelope_digest: record.envelope_digest,
                        delivery_state: record.state,
                        receipt_lineage,
                    }),
                })
            })
            .collect()
    }

    fn continuation_projection(
        &self,
        continuation: &Self::Continuation,
    ) -> Result<ContinuationHostProjection, CognitionError> {
        continuation
            .validate_authoritative()
            .map_err(|error| cognition_error("runtime_continuation_invalid", error))?;
        Ok(ContinuationHostProjection {
            continuation_proposal_id: continuation.continuation_proposal_id.clone(),
            proposal_digest: continuation.proposal_digest.clone(),
            world_id: continuation.world_id.clone(),
            agent_id: continuation.agent_id.clone(),
            agent_session_id: continuation.agent_session_id.clone(),
            agent_turn_id: continuation.agent_turn_id.clone(),
            decision_request_id: continuation.decision_request_id.clone(),
            origin_turn_id: continuation.origin_turn_id.clone(),
            origin_request_digest: continuation.origin_request_digest.clone(),
            action_or_envelope_digest: continuation.action_or_envelope_digest.clone(),
            precondition_digest: continuation.precondition_digest.clone(),
            valid_until_tick: continuation.valid_until_tick,
            continuation_id: continuation.continuation_id.clone(),
            wake_id: continuation.wake_id.clone(),
            wake_seq: continuation.wake_seq,
            continuation_digest: continuation.continuation_digest(),
            continuation_status_digest: continuation
                .continuation_status_digest
                .clone()
                .expect("validate_authoritative requires a Runtime-issued status digest"),
            status: match continuation.status {
                crate::runtime::ContinuationStatusV1::Scheduled => {
                    ContinuationStatusProjectionV1::Scheduled
                }
                crate::runtime::ContinuationStatusV1::Pending => {
                    ContinuationStatusProjectionV1::Pending
                }
                crate::runtime::ContinuationStatusV1::Waking => {
                    ContinuationStatusProjectionV1::Waking
                }
                crate::runtime::ContinuationStatusV1::Consumed => {
                    ContinuationStatusProjectionV1::Consumed
                }
                crate::runtime::ContinuationStatusV1::Completed => {
                    ContinuationStatusProjectionV1::Completed
                }
                crate::runtime::ContinuationStatusV1::Cancelled => {
                    ContinuationStatusProjectionV1::Cancelled
                }
                crate::runtime::ContinuationStatusV1::Invalidated => {
                    ContinuationStatusProjectionV1::Invalidated
                }
                crate::runtime::ContinuationStatusV1::Expired => {
                    ContinuationStatusProjectionV1::Expired
                }
                crate::runtime::ContinuationStatusV1::Rejected => {
                    ContinuationStatusProjectionV1::Rejected
                }
            },
            terminal_disposition: continuation.terminal_disposition.clone(),
            remaining_budget: oasis7_agent_api::ContinuationBudgetV1 {
                unit: continuation.remaining_budget.unit.clone(),
                value: continuation.remaining_budget.value,
            },
        })
    }
}

fn cognition_error(code: &str, error: impl std::fmt::Display) -> CognitionError {
    CognitionError::new(code, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{RuntimeFeedbackOutboxRecordV1, RuntimeFeedbackProjectionV1};

    fn rejected_feedback(agent: &str, session: &str, seq: u64) -> OasisFeedbackEnvelopeV1 {
        OasisFeedbackEnvelopeV1 {
            feedback_id: format!("{agent}-{session}-{seq}"),
            feedback_seq: seq,
            agent_subject: agent.to_string(),
            agent_session_id: session.to_string(),
            agent_turn_id: format!("turn-{seq}"),
            decision_request_id: format!("request-{seq}"),
            candidate_action_id: None,
            runtime_receipt_id: None,
            status: "rejected".to_string(),
            request_digest: crate::simulator::Digest32(format!("blake3:{}", "a".repeat(64))),
            reject_reason: Some("stale_base".to_string()),
            provenance: "runtime_authoritative".to_string(),
        }
    }

    fn record(feedback: &OasisFeedbackEnvelopeV1, state: &str) -> RuntimeFeedbackOutboxRecordV1 {
        let mut record = RuntimeFeedbackOutboxRecordV1::from_feedback_with_projection(
            feedback,
            RuntimeFeedbackProjectionV1::default(),
        )
        .expect("feedback creates a valid outbox record");
        record.state = state.to_string();
        record
            .validate()
            .expect("record validates after state transition");
        record
    }

    #[test]
    fn runtime_history_projects_ack_state_from_durable_outbox() {
        let mut world = RuntimeWorld::new();
        let acked = rejected_feedback("agent-a", "session-a", 1);
        let pending = rejected_feedback("agent-a", "session-a", 2);
        world.enqueue_runtime_feedback(acked.clone()).unwrap();
        world.claim_runtime_feedback(&acked.feedback_id).unwrap();
        world.ack_runtime_feedback(&acked.feedback_id).unwrap();
        world.enqueue_runtime_feedback(pending.clone()).unwrap();

        let authority = RuntimeWorldAuthority::new(&world);
        let history = authority.validated_feedback_history().unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(
            history[0].feedback,
            agent_api_compat::feedback_to_api(&acked)
        );
        assert!(history[0].acknowledged);
        assert_eq!(
            history[1].feedback,
            agent_api_compat::feedback_to_api(&pending)
        );
        assert!(!history[1].acknowledged);
    }

    #[test]
    fn history_rejects_a_forged_durable_envelope_digest() {
        let mut world = RuntimeWorld::new();
        let feedback = rejected_feedback("agent-a", "session-a", 1);
        let mut forged = record(&feedback, "acked");
        forged.envelope_digest = format!("blake3:{}", "b".repeat(64));
        world
            .enqueue_runtime_feedback(feedback)
            .expect("Runtime accepts the original payload");
        // The authority reads only the actual World record; a detached record
        // cannot be substituted for that durable history.
        assert!(forged.validate().is_err());
        let history = RuntimeWorldAuthority::new(&world)
            .validated_feedback_history()
            .unwrap();
        assert_eq!(history.len(), 1);
    }
}
