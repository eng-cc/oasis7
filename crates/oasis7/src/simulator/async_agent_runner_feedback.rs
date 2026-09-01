use super::{AsyncAgentRunnerError, ContinuousAgentTurnContextV1, Digest32, FeedbackEnvelopeV1};

use crate::runtime::RuntimeReceiptLineageV1;

pub(super) fn validate_feedback(
    context: &ContinuousAgentTurnContextV1,
    agent_id: &str,
    feedback: &FeedbackEnvelopeV1,
) -> Result<(), AsyncAgentRunnerError> {
    if feedback.agent_subject != agent_id
        || feedback.agent_subject != context.agent_id
        || feedback.agent_session_id != context.agent_session_id
        || feedback.agent_turn_id != context.agent_turn_id
        || feedback.decision_request_id != context.decision_request_id
        || feedback.request_digest != context.request_digest
        || feedback.provenance != "runtime_authoritative"
        || feedback.feedback_seq == 0
        || !feedback.request_digest.is_canonical_blake3()
    {
        return Err(AsyncAgentRunnerError::Cognition(
            "Runtime feedback does not correlate to the actor turn".to_string(),
        ));
    }
    if feedback.feedback_id.trim().is_empty()
        || !matches!(
            feedback.status.as_str(),
            "pending" | "committed" | "rejected" | "failed"
        )
    {
        return Err(AsyncAgentRunnerError::Cognition(
            "Runtime feedback identity or status is invalid".to_string(),
        ));
    }
    if feedback.status == "committed"
        && feedback
            .runtime_receipt_id
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        return Err(AsyncAgentRunnerError::Cognition(
            "committed feedback requires a Runtime receipt".to_string(),
        ));
    }
    Ok(())
}

pub(super) fn validate_runtime_receipt_lineage(
    context: &ContinuousAgentTurnContextV1,
    feedback: &FeedbackEnvelopeV1,
    receipt: &RuntimeReceiptLineageV1,
) -> Result<(), AsyncAgentRunnerError> {
    receipt
        .validate()
        .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
    if !receipt.receipt_digest.starts_with("blake3:")
        || !receipt.envelope_digest.starts_with("blake3:")
        || !receipt.request_digest.starts_with("blake3:")
        || !Digest32::from(receipt.receipt_digest.clone()).is_canonical_blake3()
        || !Digest32::from(receipt.envelope_digest.clone()).is_canonical_blake3()
        || !Digest32::from(receipt.request_digest.clone()).is_canonical_blake3()
    {
        return Err(AsyncAgentRunnerError::Cognition(
            "Runtime receipt proof digests must be canonical BLAKE3-256 values".to_string(),
        ));
    }
    if receipt.agent_id != context.agent_id
        || receipt.agent_session_id != context.agent_session_id
        || receipt.agent_turn_id != context.agent_turn_id
        || receipt.decision_request_id != context.decision_request_id
        || receipt.request_digest != context.request_digest.to_string()
        || receipt.feedback_id != feedback.feedback_id
        || feedback.runtime_receipt_id.as_deref() != Some(receipt.receipt_id.as_str())
    {
        return Err(AsyncAgentRunnerError::Cognition(
            "Runtime receipt does not correlate to the actor turn and feedback".to_string(),
        ));
    }
    let Some(candidate_action_id) = feedback.candidate_action_id else {
        return Err(AsyncAgentRunnerError::Cognition(
            "committed feedback requires a Runtime action lineage".to_string(),
        ));
    };
    if receipt.action_id != candidate_action_id.to_string() {
        return Err(AsyncAgentRunnerError::Cognition(
            "Runtime receipt action lineage does not match feedback".to_string(),
        ));
    }
    Ok(())
}
