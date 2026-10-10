//! Durable private feedback consumption precedes canonical acknowledgement.
use super::lineage_persistence::PendingProviderServiceIntent;
use super::*;
use crate::world_service::{FeedbackAckIntentV1, WorldServicePayloadV1};
use oasis7_client_api::world_service::RequestCorrelation;
use serde::{Deserialize, Serialize};
#[path = "llm_sidecar_service_feedback_ack_checkpoint.rs"]
mod checkpoint_validation;
#[cfg(any(test, feature = "test_tier_required"))]
#[path = "llm_sidecar_service_feedback_ack_probe.rs"]
mod feedback_ack_probe;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(in crate::viewer::runtime_live) struct PendingFeedbackAck {
    pub correlation: RequestCorrelation,
    pub payload: WorldServicePayloadV1,
    pub original_feedback: serde_json::Value,
    pub receipt_lineage: crate::runtime::RuntimeReceiptLineageV1,
    pub delivered: bool,
    pub issued: bool,
}

fn private_acceptance_digest(
    pending: &PendingProviderServiceIntent,
    raw: &serde_json::Value,
    receipt: &crate::runtime::RuntimeReceiptLineageV1,
    memory: &MemoryWriteStore,
) -> Result<String, String> {
    let state = serde_json::to_value(memory).map_err(|e| e.to_string())?;
    let mappings: serde_json::Map<String, serde_json::Value> = state["committed_by_digest"]
        .as_object()
        .ok_or("private memory mappings missing")?
        .iter()
        .filter(|(_, value)| value.as_str() == Some(receipt.receipt_id.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    // Stored receipt entries are immutable originals; owner corrections are
    // separate records. Bind full normalized content, not merely copied ids.
    let identities: Vec<serde_json::Value> = memory
        .entries()
        .iter()
        .filter(|entry| entry["receipt_id"] == receipt.receipt_id)
        .cloned()
        .collect();
    Ok(crate::simulator::h_v1("oasis7.app.private-feedback-acceptance.v1", &serde_json::json!({
        "original_cognition":pending.payload, "original_feedback":raw, "receipt_lineage":receipt,
        "captured_intents":pending.cognition.memory_write_intents, "committed_by_digest":mappings, "receipt_memory_identity":identities
    })).to_string())
}
impl RuntimeLlmSidecar {
    pub(in crate::viewer::runtime_live) fn validate_pending_feedback_consumption(
        &self,
        pending: &PendingProviderServiceIntent,
    ) -> Result<(), String> {
        pending
            .feedback_ack
            .as_ref()
            .ok_or("feedback consumption missing")?
            .validate_memory(pending, &self.provider_memory_store)
    }
    pub(in crate::viewer::runtime_live) fn checkpoint_service_feedback_consumption(
        &mut self,
        world: &RuntimeWorld,
        pending: &PendingProviderServiceIntent,
        feedback: &crate::simulator::FeedbackEnvelopeV1,
        receipt: &crate::runtime::RuntimeReceiptLineageV1,
        raw: serde_json::Value,
    ) -> Result<(), String> {
        let agent = &pending.cognition.request.request_context.agent_subject;
        if let Some(existing) = self
            .provider_service_pending
            .get(agent)
            .and_then(|p| p.feedback_ack.as_ref())
        {
            existing.validate_memory(pending, &self.provider_memory_store)?;
            if existing.original_feedback != raw {
                return Err("canonical feedback changed after consumption".into());
            }
            return Ok(());
        }
        let view = self
            .provider_service_projection
            .as_ref()
            .ok_or("feedback consumption authenticated View missing")?;
        let record = view
            .feedback_history
            .as_ref()
            .ok_or("feedback consumption history missing")?
            .records
            .iter()
            .find(|record| record.feedback.feedback_id == feedback.feedback_id)
            .ok_or("canonical original feedback missing")?;
        if serde_json::to_value(&record.feedback).map_err(|e| e.to_string())?
            != serde_json::to_value(feedback).map_err(|e| e.to_string())?
            || record.receipt_lineage.as_ref() != Some(receipt)
        {
            return Err("canonical feedback/receipt history mismatch".into());
        }
        let signer = self
            .provider_service_signer
            .as_ref()
            .ok_or("feedback ACK signer missing")?;
        let signing_key = signer.private_key_hex.clone();
        let intent = FeedbackAckIntentV1 {
            agent_id: agent.clone(),
            agent_session_id: feedback.agent_session_id.clone(),
            agent_turn_id: feedback.agent_turn_id.clone(),
            decision_request_id: feedback.decision_request_id.clone(),
            request_digest: feedback.request_digest.to_string(),
            feedback_id: feedback.feedback_id.clone(),
            feedback_seq: feedback.feedback_seq,
            original_envelope_digest: record.original_envelope_digest.clone(),
            runtime_receipt_id: feedback.runtime_receipt_id.clone(),
            delegation_generation: signer.delegation_generation,
            private_acceptance_digest: format!("blake3:{}", "0".repeat(64)),
        };
        let payload =
            WorldServicePayloadV1::FeedbackAck(crate::world_service::authority::sign_read_request(
                "feedback_ack",
                intent,
                &signer.private_key_hex,
            )?);
        let checkpoint = PendingFeedbackAck {
            correlation: crate::world_service::derive_correlation(
                pending.correlation.key.world.clone(),
                &payload,
            )?,
            payload,
            original_feedback: raw,
            receipt_lineage: receipt.clone(),
            delivered: false,
            issued: false,
        };
        checkpoint.validate(pending)?;
        if self.runner.is_none()
            && self
                .provider_restored_service_checkpoints
                .contains_key(agent)
        {
            // Authenticate the unchanged original before creating inactive actors.
            // Registration starts no model, Reserve, Prefix or new turn under Pause.
            self.validate_hosted_restored_original(world, pending)?;
            self.ensure_runner_initialized()?;
        }
        let previous_memory = self.provider_memory_store.clone();
        let previous_pending = self.provider_service_pending.clone();
        let mut memory = previous_memory.clone();
        #[cfg(any(test, feature = "test_tier_required"))]
        self.feedback_ack_missing_runner_probe()?;
        let actual_runner_result = self
            .runner
            .take()
            .ok_or_else(|| "feedback consumption native runner missing".to_string());
        let mut decision_runner = match actual_runner_result {
            Ok(runner) => runner,
            Err(error) => {
                #[cfg(any(test, feature = "test_tier_required"))]
                self.feedback_ack_missing_runner_rejected_probe(&error)?;
                return Err(error);
            }
        };
        let result = match decision_runner.async_runner_mut() {
            Some(_runner)
                if self
                    .provider_restored_service_checkpoints
                    .contains_key(agent) =>
            {
                let restored = self
                    .provider_restored_service_checkpoints
                    .get(agent)
                    .cloned()
                    .unwrap();

                (|| {
                    if serde_json::to_value(&restored).map_err(|e| e.to_string())?
                        != serde_json::to_value(pending).map_err(|e| e.to_string())?
                    {
                        return Err("restored feedback memory original changed".into());
                    }
                    super::restored_memory::validate_original_memory_checkpoint(world, pending)?;
                    crate::simulator::project_receipt_memory(
                        &pending.cognition.request.turn_context,
                        feedback,
                        receipt,
                        &pending.cognition.memory_write_intents,
                        &mut memory,
                    )
                    .map_err(|e| e.to_string())?;
                    memory
                        .finalize_corrections(receipt)
                        .map_err(|e| e.to_string())?;
                    let mut checkpoint = checkpoint;
                    let WorldServicePayloadV1::FeedbackAck(mut signed) = checkpoint.payload else {
                        unreachable!()
                    };
                    signed.request.private_acceptance_digest = private_acceptance_digest(
                        pending,
                        &checkpoint.original_feedback,
                        receipt,
                        &memory,
                    )?;
                    checkpoint.payload = WorldServicePayloadV1::FeedbackAck(
                        crate::world_service::authority::sign_read_request(
                            "feedback_ack",
                            signed.request,
                            &signing_key,
                        )?,
                    );
                    checkpoint.correlation = crate::world_service::derive_correlation(
                        pending.correlation.key.world.clone(),
                        &checkpoint.payload,
                    )?;
                    checkpoint.validate_memory(pending, &memory)?;
                    self.provider_memory_store = memory.clone();
                    self.provider_service_pending
                        .get_mut(agent)
                        .ok_or("feedback original pending missing")?
                        .feedback_ack = Some(checkpoint);
                    #[cfg(any(test, feature = "test_tier_required"))]
                    self.feedback_ack_persist_probe(&previous_memory, &previous_pending)?;
                    self.persist_provider_lineage()
                })()
            }
            Some(runner) => runner
                .with_committed_feedback_memory_transaction(
                    agent,
                    feedback.clone(),
                    receipt,
                    &mut memory,
                    |staged| {
                        staged
                            .finalize_corrections(receipt)
                            .map_err(|e| e.to_string())?;
                        let mut checkpoint = checkpoint;
                        let WorldServicePayloadV1::FeedbackAck(mut signed) = checkpoint.payload
                        else {
                            unreachable!()
                        };
                        signed.request.private_acceptance_digest = private_acceptance_digest(
                            pending,
                            &checkpoint.original_feedback,
                            receipt,
                            staged,
                        )?;
                        checkpoint.payload = WorldServicePayloadV1::FeedbackAck(
                            crate::world_service::authority::sign_read_request(
                                "feedback_ack",
                                signed.request,
                                &signing_key,
                            )?,
                        );
                        checkpoint.correlation = crate::world_service::derive_correlation(
                            pending.correlation.key.world.clone(),
                            &checkpoint.payload,
                        )?;
                        checkpoint.validate_memory(pending, staged)?;
                        self.provider_memory_store = staged.clone();
                        self.provider_service_pending
                            .get_mut(agent)
                            .ok_or("feedback original pending missing")?
                            .feedback_ack = Some(checkpoint);
                        #[cfg(any(test, feature = "test_tier_required"))]
                        self.feedback_ack_persist_probe(&previous_memory, &previous_pending)?;
                        self.persist_provider_lineage()
                    },
                )
                .map_err(|e| e.to_string()),
            None => Err("feedback consumption native runner missing".into()),
        };
        self.runner = Some(decision_runner);
        if let Err(_error) = &result {
            self.provider_memory_store = previous_memory;
            self.provider_service_pending = previous_pending;
            #[cfg(any(test, feature = "test_tier_required"))]
            self.feedback_ack_rollback_probe(_error)?;
        }
        result
    }
}
