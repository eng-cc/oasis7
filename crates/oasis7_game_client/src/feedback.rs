//! Local acceptance declarations and immutable ACK recovery. This module does
//! not execute private-memory effects or prove that the caller persisted them.
use crate::operation_store::{ClientStateKind, ClientStateV1};
use crate::{AuthorizedFeedbackRecord, VerifiedTaskWorldView, device_client::Client};
use oasis7_client_api::world_service::*;
use oasis7_client_api::world_service_correlation::derive_correlation;
use oasis7_client_api::world_service_wire::{FeedbackAckIntentV1, WorldServicePayloadV1};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Serialize, Deserialize)]
struct Acceptance {
    read_authority: String,
    feedback: AuthorizedFeedbackRecord,
    ack: SignedReadRequest<FeedbackAckIntentV1>,
}
type Journal = ClientStateV1<Acceptance>;
fn original(record: Journal) -> Result<SubmitIntentRequest<Acceptance>, String> {
    match record {
        ClientStateV1::PublicIntent(request) => Ok(*request),
        _ => Err("feedback acceptance recovery required".into()),
    }
}
fn validate(record: &Journal, kind: ClientStateKind) -> Result<(), String> {
    if kind != ClientStateKind::FeedbackAcceptance {
        return Err("feedback journal namespace mismatch".into());
    }
    let ClientStateV1::PublicIntent(request) = record else {
        return Err("feedback journal record invalid".into());
    };
    request
        .validate()
        .map_err(|_| "feedback journal envelope invalid")?;
    let accepted = &request.signed_payload;
    let f = &accepted.feedback.feedback;
    f.validate()
        .map_err(|_| "feedback journal envelope invalid")?;
    let ack = &accepted.ack.request;
    verify_read_request("feedback_ack", &accepted.ack)?;
    ack.validate()?;
    if accepted.read_authority.is_empty()
        || f.status != "committed"
        || ack.agent_id != f.agent_subject
        || ack.agent_session_id != f.agent_session_id
        || ack.agent_turn_id != f.agent_turn_id
        || ack.decision_request_id != f.decision_request_id
        || ack.request_digest != f.request_digest.to_string()
        || ack.feedback_id != f.feedback_id
        || ack.feedback_seq != f.feedback_seq
        || ack.original_envelope_digest != accepted.feedback.original_envelope_digest
        || ack.runtime_receipt_id != f.runtime_receipt_id
        || accepted.feedback.receipt_lineage.is_none()
    {
        return Err("feedback acceptance identity mismatch".into());
    }
    let expected = derive_correlation(
        request.correlation.key.world.clone(),
        &WorldServicePayloadV1::FeedbackAck(accepted.ack.clone()),
    )?;
    if expected != request.correlation {
        return Err("feedback acceptance correlation mismatch".into());
    }
    Ok(())
}
impl Client {
    fn feedback_index(&self, domain: &str, identity: impl Serialize) -> Result<String, String> {
        request_digest(domain, &(&self.world, &self.agent, identity))
    }
    /// Call only AFTER private effects and the acceptance digest are durably
    /// committed by the caller. This journal records that statement; it does
    /// not establish private-memory correctness or Runtime execution authority.
    /// `record_index` selects a record from an authenticated, scoped view.
    pub fn record_caller_persisted_feedback_acceptance(
        &self,
        view: &VerifiedTaskWorldView,
        record_index: usize,
        private_acceptance_digest: &str,
        delegation_generation: u64,
    ) -> Result<SubmitIntentRequest<WorldServicePayloadV1>, String> {
        if view.world != self.world
            || !view.read_authority_matches(&self.remote.read_authority_identity()?)
        {
            return Err("feedback read authority mismatch".into());
        }
        let mut feedback = view
            .projection()
            .feedback_history
            .get(record_index)
            .ok_or("authorized feedback record unavailable")?
            .clone();
        // Delivery progress is Runtime state, not part of local acceptance.
        feedback.delivery_state = "pending".into();
        let f = &feedback.feedback;
        if f.agent_subject != self.agent || f.status != "committed" {
            return Err("only committed scoped service feedback supports ACK".into());
        }
        let ack = sign_read_request(
            "feedback_ack",
            FeedbackAckIntentV1 {
                agent_id: f.agent_subject.clone(),
                agent_session_id: f.agent_session_id.clone(),
                agent_turn_id: f.agent_turn_id.clone(),
                decision_request_id: f.decision_request_id.clone(),
                request_digest: f.request_digest.to_string(),
                feedback_id: f.feedback_id.clone(),
                feedback_seq: f.feedback_seq,
                original_envelope_digest: feedback.original_envelope_digest.clone(),
                private_acceptance_digest: private_acceptance_digest.into(),
                runtime_receipt_id: f.runtime_receipt_id.clone(),
                delegation_generation,
            },
            &self.key_hex,
        )?;
        let correlation = derive_correlation(
            self.world.clone(),
            &WorldServicePayloadV1::FeedbackAck(ack.clone()),
        )?;
        let id = self.feedback_index("local-feedback-id", &f.feedback_id)?;
        let seq = self.feedback_index(
            "local-feedback-sequence",
            (&f.agent_session_id, f.feedback_seq),
        )?;
        let candidate = ClientStateV1::PublicIntent(Box::new(SubmitIntentRequest {
            contract_version: 1,
            correlation,
            deadline_unix_ms: None,
            signed_payload: Acceptance {
                read_authority: self.remote.read_authority_identity()?,
                feedback,
                ack,
            },
        }));
        validate(&candidate, ClientStateKind::FeedbackAcceptance)?;
        // Both immutable indexes must agree before any network send. A crash
        // between them is repaired by the same exact acceptance on retry.
        for index in [seq, id] {
            let winner = self
                .state
                .put_feedback_state(&index, &candidate, validate)?;
            if serde_cbor::to_vec(&winner).map_err(|_| "feedback encoding failed")?
                != serde_cbor::to_vec(&candidate).map_err(|_| "feedback encoding failed")?
            {
                return Err("feedback acceptance conflict; original retained".into());
            }
        }
        self.recover_caller_persisted_feedback_ack(
            &view.projection().feedback_history[record_index]
                .feedback
                .feedback_id,
        )
    }
    /// Recover the original signed ACK without requiring a still-visible view
    /// or reapplying private effects. Unknown results retain this identity.
    pub fn recover_caller_persisted_feedback_ack(
        &self,
        feedback_id: &str,
    ) -> Result<SubmitIntentRequest<WorldServicePayloadV1>, String> {
        let id = self.feedback_index("local-feedback-id", feedback_id)?;
        let record = self
            .state
            .read_feedback_state(&id, validate)?
            .ok_or("caller-persisted feedback acceptance unavailable")?;
        let request = original(record.clone())?;
        let f = &request.signed_payload.feedback.feedback;
        let seq = self.feedback_index(
            "local-feedback-sequence",
            (&f.agent_session_id, f.feedback_seq),
        )?;
        let secondary = self
            .state
            .read_feedback_state(&seq, validate)?
            .ok_or("feedback sequence acceptance unavailable")?;
        if serde_cbor::to_vec(&record).map_err(|_| "feedback encoding failed")?
            != serde_cbor::to_vec(&secondary).map_err(|_| "feedback encoding failed")?
        {
            return Err("feedback acceptance indexes conflict".into());
        }
        if request.signed_payload.read_authority != self.remote.read_authority_identity()?
            || request.correlation.key.world != self.world
            || request.correlation.key.verified_subject != self.public_key
            || request.signed_payload.ack.request.agent_id != self.agent
            || request.signed_payload.ack.request.feedback_id != feedback_id
        {
            return Err("feedback acceptance scope mismatch".into());
        }
        Ok(SubmitIntentRequest {
            contract_version: request.contract_version,
            correlation: request.correlation,
            deadline_unix_ms: request.deadline_unix_ms,
            signed_payload: WorldServicePayloadV1::FeedbackAck(request.signed_payload.ack),
        })
    }
    /// Send/recover the durable original ACK. A returned Unknown/Pending is
    /// unresolved, never proof of acknowledgement or private-memory recovery.
    pub fn acknowledge_caller_persisted_feedback(
        &self,
        feedback_id: &str,
    ) -> Result<IntentResponse<Value>, String> {
        let request = self.recover_caller_persisted_feedback_ack(feedback_id)?;
        self.submit_and_poll(&request)
    }
}

#[cfg(test)]
#[path = "feedback_tests.rs"]
mod tests;
