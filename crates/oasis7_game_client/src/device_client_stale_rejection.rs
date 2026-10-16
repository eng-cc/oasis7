//! Read-only recovery of one rejected action's service-authenticated feedback.
use super::*;
use crate::{AuthorizedFeedbackRecord, VerifiedTaskWorldView};

fn terminal_original_rejection(outcome: &IntentOutcome<Value>) -> bool {
    matches!(
        outcome,
        IntentOutcome::Rejected {
            reason: WorldServiceErrorKind::Conflict
        }
    )
}

impl Client {
    /// A generic conflict is insufficient: require the exact terminal original
    /// and its scoped Runtime stale-base feedback before offering a new decision.
    /// This never signs, submits, ACKs, or replaces the original action.
    pub fn known_stale_rejection(
        &self,
        operation: &str,
    ) -> Result<Option<AuthorizedFeedbackRecord>, String> {
        let Some(original) = self.cached_intent(ClientStateKind::Action, operation)? else {
            return Ok(None);
        };
        let result = self.poll_original(&original)?;
        if !terminal_original_rejection(&result.outcome) {
            return Ok(None);
        }
        // The canonical lookup API maps a rejected canonical result to Rejected
        // Conflict. A Committed receipt is never interpreted as arbitrary JSON
        // containing a rejection flag.
        let view = self.public_view()?;
        self.match_stale_rejection(&original, &view)
    }
    fn match_stale_rejection(
        &self,
        original: &SubmitIntentRequest<WorldServicePayloadV1>,
        view: &VerifiedTaskWorldView,
    ) -> Result<Option<AuthorizedFeedbackRecord>, String> {
        if view.world != self.world
            || !view.read_authority_matches(&self.remote.read_authority_identity()?)
            || original.correlation.key.world != self.world
            || original.correlation
                != derive_correlation(self.world.clone(), &original.signed_payload)?
        {
            return Err("rejected feedback read authority or original correlation mismatch".into());
        }
        let WorldServicePayloadV1::Cognition(signed) = &original.signed_payload else {
            return Err("rejected feedback requires an original task cognition action".into());
        };
        let fence = signed
            .request
            .task_fence
            .as_ref()
            .ok_or("original task fence unavailable")?;
        if signed.subject_public_key != self.public_key
            || signed.request.request.agent_id != self.agent
            || fence.agent_id != self.agent
            || view.task_fence() != Some(fence)
        {
            return Err("rejected feedback task fence or signer scope mismatch".into());
        }
        let request = &signed.request.request;
        let mut found = None;
        for record in &view.projection().feedback_history {
            let f = &record.feedback;
            if f.agent_subject != request.agent_id
                || f.agent_session_id != request.agent_session_id
                || f.agent_turn_id != request.agent_turn_id
                || f.decision_request_id != request.decision_request_id
                || f.request_digest.to_string() != request.request_digest
                || f.status != "rejected"
                || f.reject_reason.as_deref() != Some("stale_base")
            {
                continue;
            }
            if f.provenance != "runtime_authoritative"
                || f.candidate_action_id.is_some()
                || f.runtime_receipt_id.is_some()
                || record.receipt_lineage.is_some()
            {
                return Err("rejected feedback disposition or lineage mismatch".into());
            }
            // original_envelope_digest is the Runtime outbox digest of the full
            // payload including private projection. The verified signed view
            // authenticates it; never recompute it from the sanitized envelope.
            if found.is_some() {
                return Err("ambiguous rejected feedback for original action".into());
            }
            found = Some(record.clone());
        }
        Ok(found)
    }
}
#[cfg(all(test, unix))]
#[path = "device_client_stale_rejection_tests.rs"]
mod tests;
