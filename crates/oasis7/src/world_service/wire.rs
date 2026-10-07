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
pub enum WorldServicePayloadV1 {
    GameplayJson(Vec<u8>),
    Cognition(SignedReadRequest<CognitionIntentV1>),
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
