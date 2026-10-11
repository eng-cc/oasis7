//! Portable provider execution and cognition algorithms.
//!
//! This crate deliberately depends on the shared `oasis7_agent_api` contracts,
//! not on the Oasis7 application, Runtime, World, or kernel implementation.
//! Runtime-owned leases, receipts, durable continuation state, and any World
//! effects cross this boundary only through host-supplied opaque values and
//! callbacks.

mod actor;
mod authority;
mod cognition;
mod continuation;
mod memory;
mod model_provider;
mod provider;
mod turn_engine;

pub use actor::{
    AsyncAgentRunner, AsyncAgentRunnerError, AsyncAgentTurnOutcome, AsyncTurnFeedback, AsyncTurnId,
    AsyncTurnLifecycle, AsyncWorldEffect,
};
pub use authority::RuntimeAuthority;
pub use cognition::{AgentCognitionStore, FeedbackHistoryProjection};
pub use continuation::{
    ContinuationAuthorityContextV1, ContinuationBudgetProgressV1, ContinuationHandle,
    ContinuationHarness, ContinuationHostProjection, ContinuationInvalidationReason,
    ContinuationProjectionV1, ContinuationStatusProjectionV1,
};
pub use memory::{
    GoalSnapshotInputV1, GoalSnapshotProjector, MemoryCorrectionV1, MemoryWriteIntentPolicyV1,
    MemoryWritePolicyContextV1, MemoryWritePolicyOutcome, MemoryWriteStore,
    NormalizedMemoryWriteIntentV1,
};
pub use model_provider::{
    ModelProvider, ModelProviderError, ModelProviderRequest, ModelProviderResponse,
    ModelProviderTool, ModelProviderTrace, ModelProviderTraceEntry, ModelProviderTurn,
    ModelProviderUsage,
};
pub use provider::{DecisionProvider, DecisionProviderError};
pub use turn_engine::{
    MemoryProjectionReport, RuntimeReadbackProof, TurnEngine, TurnEngineError, TurnEngineEvent,
    TurnEnginePhase, TurnRequest,
};
