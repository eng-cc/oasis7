//! Stable, serializable contracts shared by the Runtime-facing adapter and an
//! agent Harness. These types carry proposals and read-only projections only;
//! they do not implement a World, execute provider calls, or grant state-write
//! authority.
//!
//! Runtime remains authoritative for request admission, capability checks,
//! resource reservation/settlement, world effects, durable receipts, and
//! continuation lifecycle. A receipt-lineage value is correlation data, not
//! proof of a durable world readback.

mod cognition;
mod provider;
mod runtime;

pub use cognition::{
    BudgetContractV1, CognitionError, ContinuationBudgetV1, ContinuationProposalV1,
    ContinuousAgentRequestContextV1, ContinuousAgentResponseContextV1,
    ContinuousAgentTurnContextV1, Digest32, FeedbackEnvelopeV1, GoalSnapshotV1,
    MemoryContextEntryV1, MemoryContextSnapshotV1, MemoryWriteIntentV1, ResponseArtifactIdentityV1,
    RuntimeBindingV1, WakeConditionSubjectV1, WakeConditionV1, feedback_digest, h_v1,
};
pub use provider::{
    ActionCatalogEntry, CapabilityInvocationContext, DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION,
    DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION, DecisionRequest, DecisionRequestContractError,
    DecisionResponse, MemoryWriteIntent, ObservationEnvelope, ProviderDecision,
    ProviderDiagnostics, ProviderErrorEnvelope, ProviderExecutionMode, ProviderInteractionTarget,
    ProviderMissionContext, ProviderModuleCommand, ProviderNavigationNode, ProviderNearbyEntity,
    ProviderObservation, ProviderRecentEvent, ProviderSelfState, ProviderTokenUsage,
    ProviderTraceEnvelope, ProviderTranscriptEntry,
};
pub use runtime::{
    CognitionLeaseConsumptionViewV1, CognitionLeaseStatusV1, RuntimeReceiptLineageV1,
};
