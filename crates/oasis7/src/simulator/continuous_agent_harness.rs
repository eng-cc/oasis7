//! Versioned cognition wire types and the small amount of host-side state
//! required by the continuous-agent provider contract.
//!
//! The existing [`DecisionRequest`] and [`DecisionResponse`] types remain the
//! provider's inner payload.  This module deliberately adds an outer context
//! instead of teaching the legacy DTOs about cognition identity.  Runtime
//! persistence, action receipts, and continuation scheduling consume these
//! values but remain runtime-owned.

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::cognition_policy::{ContinuationProposalV1, GoalSnapshotV1, MemoryContextSnapshotV1};
use super::{DecisionRequest, DecisionResponse, FeedbackEnvelope};

pub const CONTINUOUS_AGENT_CONTEXT_DISCRIMINATOR: &str = "oasis7.continuous-agent-context";
pub const CONTINUOUS_AGENT_CONTEXT_VERSION: u16 = 1;
pub const COGNITION_REQUEST_DIGEST_DOMAIN: &str = "oasis7.cognition.request.v1";
pub const COGNITION_PROVIDER_INVOCATION_DOMAIN: &str = "oasis7.cognition.provider-invocation.v1";

/// A wire digest.  The newtype keeps the wire representation as the familiar
/// `blake3:<64 lowercase hex>` string while preventing accidental mixing with
/// ordinary provider identifiers in typed APIs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Digest32(pub String);

impl Digest32 {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// Compatibility convenience for early callers that treated digest
    /// construction as fallible.  Digest construction now validates the
    /// protocol value before returning and is therefore infallible.
    pub fn expect(self, _message: &str) -> Self {
        self
    }
}

impl fmt::Display for Digest32 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0.as_str())
    }
}

impl From<String> for Digest32 {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for Digest32 {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// Stable, domain-separated BLAKE3-256 over canonical CBOR `[domain,payload]`.
///
/// The supported payloads are all serde values owned by the protocol.  The
/// canonical encoder can only fail for a value that is not representable by
/// the ABI's CBOR value model; keeping this function infallible makes it safe
/// to use in identity construction while retaining one implementation of the
/// encoding rules.
pub fn h_v1<T: Serialize>(domain: &str, payload: &T) -> Digest32 {
    let bytes = oasis7_wasm_abi::encode_canonical_cbor(&(domain, payload))
        .expect("cognition payload must be encodable as canonical CBOR");
    let hash = blake3::hash(bytes.as_slice());
    Digest32(format!("blake3:{hash}"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CognitionError {
    code: String,
    message: String,
}

impl CognitionError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn code(&self) -> &str {
        self.code.as_str()
    }

    pub fn message(&self) -> &str {
        self.message.as_str()
    }
}

impl fmt::Display for CognitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for CognitionError {}

/// Shared branch/finality binding.  Runtime is authoritative for its values;
/// the Harness only carries and hashes the verified projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalityBindingV1 {
    #[serde(default = "default_schema_version")]
    pub schema_version: u16,
    pub branch_id: String,
    pub finality_epoch: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finality_block_hash: Option<Digest32>,
    pub finality_status: String,
    pub reorg_epoch: u64,
}

/// Runtime snapshot binding consumed by cognition identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeBindingV1 {
    pub world_id: String,
    pub branch_id: String,
    pub finality_epoch: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finality_block_hash: Option<Digest32>,
    pub finality_status: String,
    pub base_tick: u64,
    pub base_world_hash: Digest32,
    pub reorg_epoch: u64,
    pub runtime_manifest_hash: Digest32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetContractV1 {
    pub max_latency_ms: u64,
    pub max_repair_attempts: u32,
}

/// The additive outer request wrapper.  `transport_attempt` is intentionally
/// retained for observability but excluded from identity bytes.  The legacy
/// inner timeout is likewise a transport budget, not a provider invocation
/// identity; the normalized `budget_contract` remains the policy input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContinuousAgentRequestContextV1 {
    pub base_decision_request: DecisionRequest,
    pub context_discriminator: String,
    pub context_version: u16,
    pub protocol_version: String,
    pub agent_session_id: String,
    pub agent_turn_id: String,
    pub decision_request_id: String,
    pub retry_seq: u64,
    pub transport_attempt: u64,
    pub agent_subject: String,
    pub runtime_binding: RuntimeBindingV1,
    pub observation_digest: Digest32,
    pub capability_catalog_digest: Digest32,
    pub capability_invocation_context_digest: Digest32,
    pub memory_snapshot_digest: Digest32,
    pub goal_snapshot_digest: Digest32,
    pub continuation_digest: Digest32,
    pub adapter_protocol_version: String,
    pub budget_contract: BudgetContractV1,
    pub request_digest: Digest32,
}

impl ContinuousAgentRequestContextV1 {
    pub fn validate(&self) -> Result<(), CognitionError> {
        if self.context_discriminator != CONTINUOUS_AGENT_CONTEXT_DISCRIMINATOR {
            return Err(CognitionError::new(
                "unsupported_context_discriminator",
                "continuous-agent request discriminator is not recognized",
            ));
        }
        if self.context_version != CONTINUOUS_AGENT_CONTEXT_VERSION {
            return Err(CognitionError::new(
                "unsupported_context_version",
                format!(
                    "unsupported continuous-agent context version {}",
                    self.context_version
                ),
            ));
        }
        if self.agent_session_id.trim().is_empty()
            || self.agent_turn_id.trim().is_empty()
            || self.decision_request_id.trim().is_empty()
            || self.agent_subject.trim().is_empty()
        {
            return Err(CognitionError::new(
                "missing_cognition_identity",
                "session, turn, request, and subject identities are required",
            ));
        }
        Ok(())
    }

    pub fn validate_value(value: &Value) -> Result<(), CognitionError> {
        let version = value
            .get("context_version")
            .and_then(Value::as_u64)
            .ok_or_else(|| {
                CognitionError::new(
                    "unsupported_context_version",
                    "continuous-agent context_version is missing or invalid",
                )
            })?;
        if version != u64::from(CONTINUOUS_AGENT_CONTEXT_VERSION) {
            return Err(CognitionError::new(
                "unsupported_context_version",
                format!("unsupported continuous-agent context version {version}"),
            ));
        }
        let discriminator = value
            .get("context_discriminator")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                CognitionError::new(
                    "unsupported_context_discriminator",
                    "continuous-agent context discriminator is missing or invalid",
                )
            })?;
        if discriminator != CONTINUOUS_AGENT_CONTEXT_DISCRIMINATOR {
            return Err(CognitionError::new(
                "unsupported_context_discriminator",
                "continuous-agent context discriminator is not recognized",
            ));
        }
        Ok(())
    }

    /// Return the canonical request payload bytes, excluding output identity,
    /// transport attempt, and the legacy timeout-only transport field.
    pub fn canonical_request_bytes(&self) -> Result<Vec<u8>, CognitionError> {
        self.validate()?;
        let mut value = serde_json::to_value(self)
            .map_err(|error| CognitionError::new("canonical_encoding_failed", error.to_string()))?;
        let object = value.as_object_mut().ok_or_else(|| {
            CognitionError::new(
                "canonical_encoding_failed",
                "request context is not an object",
            )
        })?;
        object.remove("request_digest");
        object.remove("transport_attempt");
        if let Some(base) = object
            .get_mut("base_decision_request")
            .and_then(Value::as_object_mut)
        {
            base.remove("timeout_budget_ms");
        }
        oasis7_wasm_abi::encode_canonical_cbor(&value)
            .map_err(|error| CognitionError::new("canonical_encoding_failed", error.to_string()))
    }

    pub fn request_digest(&self) -> Digest32 {
        h_v1(
            COGNITION_REQUEST_DIGEST_DOMAIN,
            &self
                .canonical_request_bytes()
                .expect("valid cognition request must have canonical bytes"),
        )
    }

    pub fn provider_invocation_key(&self) -> Digest32 {
        h_v1(COGNITION_PROVIDER_INVOCATION_DOMAIN, &self.request_digest())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContinuousAgentResponseContextV1 {
    pub base_decision_response: DecisionResponse,
    pub context_discriminator: String,
    pub context_version: u16,
    pub agent_session_id: String,
    pub agent_turn_id: String,
    pub decision_request_id: String,
    pub retry_seq: u64,
    pub transport_attempt: u64,
    pub request_digest: Digest32,
}

/// Host projection shared by Builtin and ProviderBacked simulator actors for
/// one turn.  Memory is retrieval context only; GoalSnapshot is the sole
/// mission projection.  Runtime owns continuation status and receipts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContinuousAgentTurnContextV1 {
    pub agent_id: String,
    pub agent_session_id: String,
    pub agent_turn_id: String,
    pub decision_request_id: String,
    pub request_digest: Digest32,
    pub memory_snapshot: MemoryContextSnapshotV1,
    pub goal_snapshot: GoalSnapshotV1,
    #[serde(default)]
    pub continuation: Option<ContinuationProposalV1>,
}

impl ContinuousAgentTurnContextV1 {
    pub fn validate_for_agent(&self, agent_id: &str) -> Result<(), CognitionError> {
        if self.agent_id != agent_id
            || self.agent_session_id.trim().is_empty()
            || self.agent_turn_id.trim().is_empty()
            || self.decision_request_id.trim().is_empty()
            || self.request_digest.as_str().trim().is_empty()
        {
            return Err(CognitionError::new(
                "cognition_context_identity_mismatch",
                "turn context identity does not match the actor",
            ));
        }
        if self.memory_snapshot.digest != self.memory_snapshot.computed_digest()
            || self.goal_snapshot.digest != self.goal_snapshot.computed_digest()
        {
            return Err(CognitionError::new(
                "cognition_context_digest_mismatch",
                "host cognition context contains an invalid projection digest",
            ));
        }
        if let Some(continuation) = &self.continuation {
            continuation.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryWriteIntentV1 {
    #[serde(default = "default_schema_version")]
    pub schema_version: u16,
    pub scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedbackEnvelopeV1 {
    pub feedback_id: String,
    pub feedback_seq: u64,
    pub agent_subject: String,
    pub agent_session_id: String,
    pub agent_turn_id: String,
    pub decision_request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_action_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_receipt_id: Option<String>,
    pub status: String,
    pub request_digest: Digest32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reject_reason: Option<String>,
    pub provenance: String,
}

impl FeedbackEnvelopeV1 {
    /// Legacy feedback has no runtime disposition/receipt and therefore cannot
    /// be promoted to authoritative cognition feedback.
    pub fn from_legacy_value(value: Value) -> Result<Self, CognitionError> {
        let legacy: FeedbackEnvelope = serde_json::from_value(value)
            .map_err(|error| CognitionError::new("legacy_feedback_invalid", error.to_string()))?;
        let _ = legacy;
        Err(CognitionError::new(
            "legacy_feedback_ambiguous",
            "legacy feedback lacks a Runtime disposition or committed receipt",
        ))
    }
}

#[derive(Debug, Clone)]
struct ActiveCognitionRequest {
    session_id: String,
    turn_id: String,
    request_id: String,
    request_digest: Digest32,
}

/// In-memory host-side correlation and single-flight guard for P0.1.
/// Runtime owns durable recovery and receipt truth; this store only prevents
/// concurrent cognition contamination before a request reaches the provider.
#[derive(Debug, Clone, Default)]
pub struct AgentCognitionStore {
    active_by_agent: BTreeMap<String, ActiveCognitionRequest>,
    digest_by_request_id: BTreeMap<String, Digest32>,
}

impl AgentCognitionStore {
    pub fn begin_request(
        &mut self,
        request: ContinuousAgentRequestContextV1,
    ) -> Result<(), CognitionError> {
        request.validate()?;
        let digest = request.request_digest();
        if let Some(previous) = self.digest_by_request_id.get(&request.decision_request_id) {
            if previous != &digest {
                return Err(CognitionError::new(
                    "request_identity_collision",
                    "decision_request_id was reused with different canonical inputs",
                ));
            }
            return Ok(());
        }
        if self.active_by_agent.contains_key(&request.agent_subject) {
            return Err(CognitionError::new(
                "agent_busy",
                "an Agent already has an in-flight cognition request",
            ));
        }
        self.digest_by_request_id
            .insert(request.decision_request_id.clone(), digest.clone());
        self.active_by_agent.insert(
            request.agent_subject,
            ActiveCognitionRequest {
                session_id: request.agent_session_id,
                turn_id: request.agent_turn_id,
                request_id: request.decision_request_id,
                request_digest: digest,
            },
        );
        Ok(())
    }

    pub fn accept_feedback(&mut self, feedback: FeedbackEnvelopeV1) -> Result<(), CognitionError> {
        let Some(active) = self.active_by_agent.get(&feedback.agent_subject) else {
            if self
                .active_by_agent
                .keys()
                .any(|agent| agent != &feedback.agent_subject)
            {
                return Err(CognitionError::new(
                    "cross_agent_feedback",
                    "feedback does not belong to the active Agent subject",
                ));
            }
            return Err(CognitionError::new(
                "unknown_feedback",
                "feedback does not match an active cognition request",
            ));
        };
        if active.session_id != feedback.agent_session_id
            || active.turn_id != feedback.agent_turn_id
            || active.request_id != feedback.decision_request_id
        {
            return Err(CognitionError::new(
                "feedback_correlation_mismatch",
                "feedback cognition lineage does not match the active request",
            ));
        }
        if active.request_digest != feedback.request_digest {
            return Err(CognitionError::new(
                "feedback_digest_mismatch",
                "feedback request digest does not match the active request",
            ));
        }
        if matches!(
            feedback.status.as_str(),
            "committed" | "rejected" | "failed" | "cancelled" | "expired" | "stale"
        ) {
            self.active_by_agent.remove(&feedback.agent_subject);
        }
        Ok(())
    }

    pub fn clear_agent(&mut self, agent_subject: &str) {
        self.active_by_agent.remove(agent_subject);
    }
}

fn default_schema_version() -> u16 {
    1
}
