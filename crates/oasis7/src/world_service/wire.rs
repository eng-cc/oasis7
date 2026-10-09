use serde::{Deserialize, Serialize};

pub const DESCRIBE_PATH: &str = "/v1/world/describe";
pub const SUBMIT_PATH: &str = "/v1/world/submit";
pub const LOOKUP_PATH: &str = "/v1/world/lookup";
pub const VIEW_PATH: &str = "/v1/world/view";
pub const CHANGES_PATH: &str = "/v1/world/changes";

/// The signature binds the operation domain and every request field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedReadRequest<T> {
    pub request: T,
    pub subject_public_key: String,
    pub signature_hex: String,
}

/// Verified against configured service trust, not a caller-supplied key.
/// This authenticates a service assertion; consensus proofs remain separate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedServiceResponse<T> {
    pub request_digest: String,
    pub payload: T,
    pub signature_hex: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthenticatedLookup {
    pub request: super::LookupIntentRequest,
    pub original: WorldServicePayloadV1,
}

/// Canonical execution result stored inside the journaled authorization state.
/// The enclosing execution record supplies the final CommitRef after hashing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonicalIntentResultV1 {
    pub request: super::SubmitIntentRequest<WorldServicePayloadV1>,
    pub action_id: u64,
    pub committed_height: u64,
    pub receipt: serde_json::Value,
    pub rejected: Option<String>,
}

/// Preserve legacy JSON payload bytes. Wrapping them adds no legacy authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload", rename_all = "snake_case")]
#[expect(
    clippy::large_enum_variant,
    reason = "Published signed payload variants preserve the existing typed API and canonical serialization boundary"
)]
pub enum WorldServicePayloadV1 {
    GameplayJson(Vec<u8>),
    Cognition(SignedReadRequest<CognitionIntentV1>),
    FeedbackAck(SignedReadRequest<FeedbackAckIntentV1>),
    Delegation(SignedReadRequest<AgentSignerDelegationChangeV1>),
    Scheduler(SignedReadRequest<SchedulerIntentV1>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchedulerIntentV1 {
    pub agent_id: String,
    pub request_id: String,
    pub delegation_generation: u64,
    pub captured_base_binding: crate::runtime::RuntimeCognitionBaseBindingV1,
    pub operation: SchedulerOperationV1,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", content = "data", rename_all = "snake_case")]
pub enum SchedulerOperationV1 {
    ProviderPrefix {
        request: crate::simulator::ContinuousAgentRequestContextV1,
        context_digest: String,
    },
    ProviderFailure {
        request: crate::simulator::ContinuousAgentRequestContextV1,
        reason: String,
    },
    ReserveLease(crate::runtime::CognitionLeaseRequestV1),
    SettleLease {
        lease_id: String,
        consumed_amount: u64,
    },
    ReleaseLease {
        lease_id: String,
    },
    AdmitContinuation(crate::runtime::CognitionContinuationProposalV1),
    TransitionContinuation {
        continuation_id: String,
        to: crate::runtime::ContinuationStatusV1,
        logical_tick: u64,
    },
    ConsumeContinuationBudget {
        continuation_id: String,
        budget_spent: u64,
        current_context: crate::runtime::CognitionContextDigestsV1,
    },
    ResumeWake {
        wake_id: String,
        proposal: crate::runtime::CognitionContinuationProposalV1,
        budget_spent: u64,
        resume: crate::runtime::CognitionContinuationResumeRequestV1,
        current_context: crate::runtime::CognitionContextDigestsV1,
    },
    HandoffWake {
        wake_id: String,
        disposition: crate::runtime::CognitionWakeDispositionV1,
        current_context: crate::runtime::CognitionContextDigestsV1,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CognitionIntentV1 {
    pub request: crate::runtime::RuntimeCognitionCommitRequestV1,
    pub action: crate::runtime::Action,
    pub response_artifact: crate::runtime::RuntimeCognitionResponseArtifactV1,
    pub delegation_generation: u64,
    // Omitting legacy absence preserves the original signed codec bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub causal_proposal: Option<CognitionCausalProposalV1>,
}

/// Public Agent explanation, signed as part of the cognition intent. Canonical
/// intent and owner-control authority are deliberately absent from this codec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CognitionCausalProposalV1 {
    pub expected_consequence: serde_json::Value,
    pub alternative: serde_json::Value,
    pub stakes: serde_json::Value,
    pub reason: Option<String>,
    pub evidence_refs: Vec<String>,
    pub correction_refs: Vec<String>,
    pub dissent: Option<String>,
}

/// Owner-signed, canonical registration/revocation. Generation is a fence,
/// never inferred from a provider-local key cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSignerDelegationChangeV1 {
    pub world_id: String,
    pub branch_id: String,
    pub agent_id: String,
    pub owner_binding: String,
    pub agent_identity_generation: u64,
    pub generation: u64,
    pub delegate_public_key: String,
    pub revoked: bool,
    pub nonce: u64,
}

/// Agent acknowledgement of an already published canonical feedback envelope.
/// The digest is of the original canonical payload, never a sanitized View.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeedbackAckIntentV1 {
    pub agent_id: String,
    pub agent_session_id: String,
    pub agent_turn_id: String,
    pub decision_request_id: String,
    pub request_digest: String,
    pub feedback_id: String,
    pub feedback_seq: u64,
    pub original_envelope_digest: String,
    /// Opaque signed Agent statement. Runtime validates its codec, not private
    /// memory contents or acceptance; the App persists acceptance before ACK.
    pub private_acceptance_digest: String,
    pub runtime_receipt_id: Option<String>,
    pub delegation_generation: u64,
}
impl FeedbackAckIntentV1 {
    pub fn validate(&self) -> Result<(), String> {
        let bounded = |value: &str| !value.trim().is_empty() && value.len() <= 256;
        let digest = |value: &str| crate::simulator::Digest32(value.into()).is_canonical_blake3();
        if ![
            &self.agent_id,
            &self.agent_session_id,
            &self.agent_turn_id,
            &self.decision_request_id,
            &self.feedback_id,
        ]
        .iter()
        .all(|v| bounded(v))
            || self.feedback_seq == 0
            || !digest(&self.request_digest)
            || !digest(&self.original_envelope_digest)
            || !digest(&self.private_acceptance_digest)
            || self
                .runtime_receipt_id
                .as_deref()
                .is_some_and(|v| !bounded(v))
        {
            return Err("feedback acknowledgement identity invalid".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod feedback_ack_codec_tests {
    use super::*;
    #[test]
    fn feedback_ack_preserves_canonical_digest_encoding_and_rejects_legacy_forms() {
        let digest = crate::simulator::h_v1("oasis7.feedback-ack.codec-test.v1", &"actual input")
            .to_string();
        let original = FeedbackAckIntentV1 {
            agent_id: "agent".into(),
            agent_session_id: "session".into(),
            agent_turn_id: "turn".into(),
            decision_request_id: "decision".into(),
            request_digest: digest.clone(),
            feedback_id: "feedback".into(),
            feedback_seq: 1,
            original_envelope_digest: digest.clone(),
            private_acceptance_digest: digest.clone(),
            runtime_receipt_id: Some("receipt".into()),
            delegation_generation: 1,
        };
        original.validate().unwrap();
        assert_eq!(
            serde_json::from_value::<FeedbackAckIntentV1>(serde_json::to_value(&original).unwrap())
                .unwrap(),
            original
        );
        for invalid in [
            digest.strip_prefix("blake3:").unwrap().to_string(),
            digest.to_uppercase(),
            format!("sha256:{}", "0".repeat(64)),
            "blake3:00".into(),
            format!("blake3:{}", "g".repeat(64)),
        ] {
            let mut value = original.clone();
            value.request_digest = invalid.clone();
            assert!(value.validate().is_err());
            let mut value = original.clone();
            value.original_envelope_digest = invalid.clone();
            assert!(value.validate().is_err());
            let mut value = original.clone();
            value.private_acceptance_digest = invalid;
            assert!(value.validate().is_err());
        }
    }
}
