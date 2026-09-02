//! Versioned cognition wire types and the small amount of host-side state
//! required by the continuous-agent provider contract.
//!
//! The existing [`DecisionRequest`] and [`DecisionResponse`] types remain the
//! provider's inner payload.  This module deliberately adds an outer context
//! instead of teaching the legacy DTOs about cognition identity.  Runtime
//! persistence, action receipts, and continuation scheduling consume these
//! values but remain runtime-owned.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::cognition_policy::{ContinuationProposalV1, GoalSnapshotV1, MemoryContextSnapshotV1};
use super::{DecisionRequest, DecisionResponse, FeedbackEnvelope};

pub const CONTINUOUS_AGENT_CONTEXT_DISCRIMINATOR: &str = "oasis7.continuous-agent-context";
pub const CONTINUOUS_AGENT_CONTEXT_VERSION: u16 = 1;
pub const COGNITION_REQUEST_DIGEST_DOMAIN: &str = "oasis7.cognition.request.v1";
pub const COGNITION_PROVIDER_INVOCATION_DOMAIN: &str = "oasis7.cognition.provider-invocation.v1";
pub const COGNITION_RESPONSE_DIGEST_DOMAIN: &str = "oasis7.cognition.response.v1";
pub const COGNITION_RESPONSE_ARTIFACT_IDENTITY_DOMAIN: &str =
    "oasis7.cognition.response-artifact-identity.v1";
const COGNITION_FEEDBACK_DIGEST_DOMAIN: &str = "oasis7.cognition.feedback.v1";
const MAX_FEEDBACK_REPLAY_ENTRIES: usize = 8;

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

    /// Return whether this value is the canonical wire rendering of a
    /// BLAKE3-256 digest.  Callers at replay and receipt seams must validate
    /// the shape instead of treating an arbitrary provider string as proof.
    pub fn is_canonical_blake3(&self) -> bool {
        valid_blake3_digest(self.as_str())
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

impl Default for Digest32 {
    fn default() -> Self {
        Self(String::new())
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

impl FinalityBindingV1 {
    /// Validate the Runtime-owned branch/finality projection.  A block hash
    /// is optional for non-verified states, but every supplied hash must use
    /// the shared typed BLAKE3-256 rendering.
    pub fn validate(&self) -> Result<(), CognitionError> {
        if self.branch_id.trim().is_empty() || self.branch_id.len() > 128 {
            return Err(CognitionError::new(
                "recovery_pending",
                "finality binding branch identity is invalid",
            ));
        }
        if !matches!(
            self.finality_status.as_str(),
            "pending" | "verified" | "reorged" | "suspended"
        ) {
            return Err(CognitionError::new(
                "recovery_pending",
                "finality binding status is not in the v1 registry",
            ));
        }
        match self.finality_block_hash.as_ref() {
            Some(hash) if !valid_blake3_digest(hash.as_str()) => Err(CognitionError::new(
                "recovery_pending",
                "finality block hash is not a canonical BLAKE3-256 digest",
            )),
            None if self.finality_status == "verified" => Err(CognitionError::new(
                "recovery_pending",
                "verified finality requires a block hash",
            )),
            _ => Ok(()),
        }
    }

    pub fn digest(&self) -> Digest32 {
        h_v1("oasis7.runtime.finality-binding.v1", self)
    }
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

impl RuntimeBindingV1 {
    pub fn validate(&self) -> Result<(), CognitionError> {
        FinalityBindingV1 {
            schema_version: 1,
            branch_id: self.branch_id.clone(),
            finality_epoch: self.finality_epoch,
            finality_block_hash: self.finality_block_hash.clone(),
            finality_status: self.finality_status.clone(),
            reorg_epoch: self.reorg_epoch,
        }
        .validate()
        .and_then(|_| {
            if self.base_world_hash.is_canonical_blake3()
                && self.runtime_manifest_hash.is_canonical_blake3()
            {
                Ok(())
            } else {
                Err(CognitionError::new(
                    "invalid_runtime_binding_digest",
                    "Runtime binding hashes must be canonical BLAKE3-256 values",
                ))
            }
        })
    }
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
        self.validate_structure()?;
        self.base_decision_request
            .validate_contract()
            .map_err(|error| CognitionError::new(error.code, error.message))?;
        let derived_digest = self.request_digest();
        if self.request_digest != derived_digest {
            return Err(CognitionError::new(
                "request_digest_mismatch",
                "declared request digest does not match canonical request inputs",
            ));
        }
        Ok(())
    }

    /// Validate the production-target provider lane. Legacy fixtures may use
    /// zero transport fields, but an actual async provider invocation must
    /// carry both retry and transport attempt lineage explicitly.
    pub fn validate_production_lane(&self) -> Result<(), CognitionError> {
        self.validate()?;
        if self.retry_seq == 0 || self.transport_attempt == 0 {
            return Err(CognitionError::new(
                "missing_retry_lineage",
                "production provider requests require nonzero retry_seq and transport_attempt",
            ));
        }
        Ok(())
    }

    fn validate_structure(&self) -> Result<(), CognitionError> {
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
        if self.base_decision_request.observation.agent_id != self.agent_subject {
            return Err(CognitionError::new(
                "cognition_context_identity_mismatch",
                "outer request subject does not match the inner observation subject",
            ));
        }
        for (name, digest) in [
            ("observation_digest", &self.observation_digest),
            ("capability_catalog_digest", &self.capability_catalog_digest),
            (
                "capability_invocation_context_digest",
                &self.capability_invocation_context_digest,
            ),
            ("memory_snapshot_digest", &self.memory_snapshot_digest),
            ("goal_snapshot_digest", &self.goal_snapshot_digest),
            ("continuation_digest", &self.continuation_digest),
        ] {
            if !digest.is_canonical_blake3() {
                return Err(CognitionError::new(
                    "invalid_context_digest",
                    format!("{name} must be a canonical BLAKE3-256 digest"),
                ));
            }
        }
        self.runtime_binding.validate()?;
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

    /// Return canonical request payload bytes, excluding output identity,
    /// transport attempt, and both legacy timeout-only transport fields.
    pub fn canonical_request_bytes(&self) -> Result<Vec<u8>, CognitionError> {
        self.validate_structure()?;
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
            if let Some(observation) = base.get_mut("observation").and_then(Value::as_object_mut) {
                observation.remove("timeout_budget_ms");
            }
        }
        // Keep the optional finality hash's presence explicit in the identity
        // payload.  The wire serializer omits `None`, while the canonical
        // contract hashes optional values with an explicit null/presence tag.
        if let Some(runtime_binding) = object
            .get_mut("runtime_binding")
            .and_then(Value::as_object_mut)
        {
            runtime_binding
                .entry("finality_block_hash")
                .or_insert(Value::Null);
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
    /// Content identity for the provider response/artifact retained at the
    /// replay seam. Runtime may bind this digest to its durable envelope.
    #[serde(default)]
    pub response_digest: Digest32,
}

/// Identity payload for the response/artifact replay seam. The response
/// digest proves provider content; this additional digest binds that content
/// to the complete outer turn lineage before Runtime stores an artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseArtifactIdentityV1 {
    pub schema_version: u16,
    pub context_discriminator: String,
    pub context_version: u16,
    pub agent_session_id: String,
    pub agent_turn_id: String,
    pub decision_request_id: String,
    pub retry_seq: u64,
    pub transport_attempt: u64,
    pub request_digest: Digest32,
    pub response_digest: Digest32,
    pub artifact_digest: Digest32,
}

impl ResponseArtifactIdentityV1 {
    fn canonical_payload(&self) -> Value {
        let mut payload = serde_json::to_value(self).expect("response identity is serializable");
        payload
            .as_object_mut()
            .expect("response identity is an object")
            .remove("artifact_digest");
        payload
    }

    pub fn validate(&self) -> Result<(), CognitionError> {
        if self.schema_version != 1
            || self.context_discriminator != CONTINUOUS_AGENT_CONTEXT_DISCRIMINATOR
            || self.context_version != CONTINUOUS_AGENT_CONTEXT_VERSION
            || self.agent_session_id.trim().is_empty()
            || self.agent_turn_id.trim().is_empty()
            || self.decision_request_id.trim().is_empty()
            || !self.request_digest.is_canonical_blake3()
            || !self.response_digest.is_canonical_blake3()
            || !self.artifact_digest.is_canonical_blake3()
        {
            return Err(CognitionError::new(
                "response_artifact_identity_invalid",
                "response artifact identity is incomplete or not canonical",
            ));
        }
        if self.artifact_digest
            != h_v1(
                COGNITION_RESPONSE_ARTIFACT_IDENTITY_DOMAIN,
                &self.canonical_payload(),
            )
        {
            return Err(CognitionError::new(
                "response_artifact_identity_mismatch",
                "response artifact identity digest does not match its lineage",
            ));
        }
        Ok(())
    }
}

impl ContinuousAgentResponseContextV1 {
    pub fn response_artifact_identity(&self) -> ResponseArtifactIdentityV1 {
        let mut identity = ResponseArtifactIdentityV1 {
            schema_version: 1,
            context_discriminator: self.context_discriminator.clone(),
            context_version: self.context_version,
            agent_session_id: self.agent_session_id.clone(),
            agent_turn_id: self.agent_turn_id.clone(),
            decision_request_id: self.decision_request_id.clone(),
            retry_seq: self.retry_seq,
            transport_attempt: self.transport_attempt,
            request_digest: self.request_digest.clone(),
            response_digest: self.response_digest.clone(),
            artifact_digest: Digest32::default(),
        };
        identity.artifact_digest = h_v1(
            COGNITION_RESPONSE_ARTIFACT_IDENTITY_DOMAIN,
            &identity.canonical_payload(),
        );
        identity
    }

    pub fn response_artifact_identity_payload(&self) -> Result<Value, CognitionError> {
        serde_json::to_value(self.response_artifact_identity()).map_err(|e| {
            CognitionError::new("response_artifact_identity_encoding_failed", e.to_string())
        })
    }

    pub fn validate_response_artifact_identity(
        &self,
        identity: &ResponseArtifactIdentityV1,
    ) -> Result<(), CognitionError> {
        if self.response_digest
            != h_v1(
                COGNITION_RESPONSE_DIGEST_DOMAIN,
                &self.base_decision_response,
            )
        {
            return Err(CognitionError::new(
                "response_digest_mismatch",
                "provider response digest does not match its content",
            ));
        }
        identity.validate()?;
        if identity != &self.response_artifact_identity() {
            return Err(CognitionError::new(
                "response_artifact_identity_mismatch",
                "response artifact identity does not match the response context",
            ));
        }
        Ok(())
    }
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
    subject_by_request_id: BTreeMap<String, String>,
    /// Feedback replay state is partitioned by `(agent_subject, session)`;
    /// no Agent can observe another Agent's feedback history.
    feedback_partitions: BTreeMap<(String, String), FeedbackPartition>,
}

#[derive(Debug, Clone, Default)]
struct FeedbackPartition {
    next_seq: u64,
    digest_by_id: BTreeMap<String, Digest32>,
    digest_by_seq: BTreeMap<u64, (String, Digest32)>,
    replay_order: VecDeque<String>,
    held: BTreeMap<u64, FeedbackEnvelopeV1>,
}

impl AgentCognitionStore {
    /// Admit the reduced compatibility lane with the same per-Agent
    /// single-flight semantics. Production-target callers should prefer
    /// `begin_request`, which also verifies the trusted outer context.
    pub fn begin_turn(
        &mut self,
        turn: &ContinuousAgentTurnContextV1,
    ) -> Result<(), CognitionError> {
        turn.validate_for_agent(turn.agent_id.as_str())?;
        if let Some(active) = self.active_by_agent.get(&turn.agent_id) {
            if active.session_id == turn.agent_session_id
                && active.turn_id == turn.agent_turn_id
                && active.request_id == turn.decision_request_id
                && active.request_digest == turn.request_digest
            {
                return Ok(());
            }
            return Err(CognitionError::new(
                "agent_busy",
                "an Agent already has an in-flight cognition request",
            ));
        }
        if self
            .digest_by_request_id
            .contains_key(&turn.decision_request_id)
        {
            if self
                .subject_by_request_id
                .get(&turn.decision_request_id)
                .is_some_and(|subject| subject != &turn.agent_id)
            {
                return Err(CognitionError::new(
                    "request_identity_collision",
                    "decision_request_id was reused by another Agent subject",
                ));
            }
            return Err(CognitionError::new(
                "request_replay",
                "decision_request_id was already admitted and is no longer active",
            ));
        }
        self.digest_by_request_id.insert(
            turn.decision_request_id.clone(),
            turn.request_digest.clone(),
        );
        self.subject_by_request_id
            .insert(turn.decision_request_id.clone(), turn.agent_id.clone());
        self.active_by_agent.insert(
            turn.agent_id.clone(),
            ActiveCognitionRequest {
                session_id: turn.agent_session_id.clone(),
                turn_id: turn.agent_turn_id.clone(),
                request_id: turn.decision_request_id.clone(),
                request_digest: turn.request_digest.clone(),
            },
        );
        Ok(())
    }

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
            if self
                .subject_by_request_id
                .get(&request.decision_request_id)
                .is_some_and(|subject| subject != &request.agent_subject)
            {
                return Err(CognitionError::new(
                    "request_identity_collision",
                    "decision_request_id was reused by another Agent subject",
                ));
            }
            if self.active_by_agent.values().any(|active| {
                active.request_id == request.decision_request_id && active.request_digest == digest
            }) {
                return Ok(());
            }
            return Err(CognitionError::new(
                "request_replay",
                "decision_request_id was already admitted and is no longer active",
            ));
        }
        if self.active_by_agent.contains_key(&request.agent_subject) {
            return Err(CognitionError::new(
                "agent_busy",
                "an Agent already has an in-flight cognition request",
            ));
        }
        self.digest_by_request_id
            .insert(request.decision_request_id.clone(), digest.clone());
        self.subject_by_request_id.insert(
            request.decision_request_id.clone(),
            request.agent_subject.clone(),
        );
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
        if !self.active_by_agent.contains_key(&feedback.agent_subject)
            && self
                .active_by_agent
                .keys()
                .any(|agent| agent != &feedback.agent_subject)
        {
            return Err(CognitionError::new(
                "cross_agent_feedback",
                "feedback does not belong to the active Agent subject",
            ));
        }
        validate_feedback_contract(&feedback)?;
        let partition_key = (
            feedback.agent_subject.clone(),
            feedback.agent_session_id.clone(),
        );
        let feedback_digest_value = feedback_digest(&feedback);
        let Some(active) = self.active_by_agent.get(&feedback.agent_subject) else {
            if let Some(partition) = self.feedback_partitions.get(&partition_key) {
                if let Some(previous) = partition.digest_by_id.get(&feedback.feedback_id) {
                    return Self::replay_or_reject_feedback(previous, feedback_digest_value);
                }
                if let Some((previous_id, previous_digest)) =
                    partition.digest_by_seq.get(&feedback.feedback_seq)
                {
                    if previous_id != &feedback.feedback_id
                        || previous_digest != &feedback_digest_value
                    {
                        return Err(CognitionError::new(
                            "feedback_identity_collision",
                            "feedback_seq was reused with a different envelope",
                        ));
                    }
                }
            }
            return Err(CognitionError::new(
                "unknown_feedback",
                "feedback does not match an active cognition request",
            ));
        };
        let partition = self.feedback_partitions.entry(partition_key).or_default();
        if let Some(previous) = partition.digest_by_id.get(&feedback.feedback_id) {
            return Self::replay_or_reject_feedback(previous, feedback_digest_value);
        }
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
        let expected = partition.next_seq.saturating_add(1);
        if feedback.feedback_seq > expected {
            if partition.held.len() >= MAX_FEEDBACK_REPLAY_ENTRIES {
                return Err(CognitionError::new(
                    "feedback_sequence_overflow",
                    "feedback sequence gap exceeds the bounded replay window",
                ));
            }
            partition.held.insert(feedback.feedback_seq, feedback);
            return Err(CognitionError::new(
                "feedback_sequence_gap",
                format!("expected feedback sequence {expected}"),
            ));
        }
        if feedback.feedback_seq < expected {
            if let Some((previous_id, previous)) =
                partition.digest_by_seq.get(&feedback.feedback_seq)
            {
                if previous_id == &feedback.feedback_id {
                    return Self::replay_or_reject_feedback(previous, feedback_digest_value);
                }
                return Err(CognitionError::new(
                    "feedback_identity_collision",
                    "feedback_seq was reused with a different envelope",
                ));
            }
            return Err(CognitionError::new(
                "feedback_sequence_gap",
                format!(
                    "feedback sequence {} is behind {expected}",
                    feedback.feedback_seq
                ),
            ));
        }
        Self::remember_feedback(
            partition,
            feedback.feedback_id.clone(),
            feedback_digest_value,
        );
        partition.next_seq = feedback.feedback_seq;
        if matches!(
            feedback.status.as_str(),
            "committed" | "rejected" | "failed"
        ) {
            self.active_by_agent.remove(&feedback.agent_subject);
        }
        while let Some(next_feedback) = partition.held.remove(&partition.next_seq.saturating_add(1))
        {
            let next_digest = feedback_digest(&next_feedback);
            Self::remember_feedback(partition, next_feedback.feedback_id.clone(), next_digest);
            partition.next_seq = next_feedback.feedback_seq;
            if matches!(
                next_feedback.status.as_str(),
                "committed" | "rejected" | "failed"
            ) {
                self.active_by_agent.remove(&next_feedback.agent_subject);
            }
        }
        Ok(())
    }

    fn replay_or_reject_feedback(
        previous: &Digest32,
        current: Digest32,
    ) -> Result<(), CognitionError> {
        if previous == &current {
            return Ok(());
        }
        Err(CognitionError::new(
            "feedback_id_conflict",
            "feedback_id was reused with a different envelope",
        ))
    }

    fn remember_feedback(partition: &mut FeedbackPartition, feedback_id: String, digest: Digest32) {
        let sequence = partition.next_seq.saturating_add(1);
        partition
            .digest_by_seq
            .insert(sequence, (feedback_id.clone(), digest.clone()));
        partition.digest_by_id.insert(feedback_id.clone(), digest);
        partition.replay_order.push_back(feedback_id);
        while partition.replay_order.len() > MAX_FEEDBACK_REPLAY_ENTRIES {
            let Some(expired_id) = partition.replay_order.pop_front() else {
                break;
            };
            partition.digest_by_id.remove(&expired_id);
            if let Some(sequence) = partition
                .digest_by_seq
                .iter()
                .find_map(|(sequence, (id, _))| (id == &expired_id).then_some(*sequence))
            {
                partition.digest_by_seq.remove(&sequence);
            }
        }
    }

    pub fn clear_agent(&mut self, agent_subject: &str) {
        self.active_by_agent.remove(agent_subject);
    }

    pub fn contains_feedback(
        &self,
        agent_subject: &str,
        agent_session_id: &str,
        feedback_id: &str,
    ) -> bool {
        self.feedback_partitions
            .iter()
            .any(|((subject, session), partition)| {
                subject == agent_subject
                    && session == agent_session_id
                    && partition.digest_by_id.contains_key(feedback_id)
            })
    }
}

fn default_schema_version() -> u16 {
    1
}

fn feedback_digest(feedback: &FeedbackEnvelopeV1) -> Digest32 {
    h_v1(COGNITION_FEEDBACK_DIGEST_DOMAIN, feedback)
}

fn validate_feedback_contract(feedback: &FeedbackEnvelopeV1) -> Result<(), CognitionError> {
    if feedback.feedback_id.trim().is_empty()
        || feedback.feedback_seq == 0
        || feedback.agent_subject.trim().is_empty()
        || feedback.agent_session_id.trim().is_empty()
        || feedback.agent_turn_id.trim().is_empty()
        || feedback.decision_request_id.trim().is_empty()
        || !feedback.request_digest.is_canonical_blake3()
        || feedback.provenance != "runtime_authoritative"
        || !matches!(
            feedback.status.as_str(),
            "pending" | "committed" | "rejected" | "failed"
        )
    {
        return Err(CognitionError::new(
            "feedback_contract_invalid",
            "feedback requires canonical identity, Runtime provenance, sequence, and status",
        ));
    }
    if feedback.status == "committed"
        && (feedback.candidate_action_id.is_none()
            || feedback
                .runtime_receipt_id
                .as_deref()
                .is_none_or(|value| value.trim().is_empty()))
    {
        return Err(CognitionError::new(
            "feedback_contract_invalid",
            "committed feedback requires action and Runtime receipt identity",
        ));
    }
    Ok(())
}

fn valid_blake3_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("blake3:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
