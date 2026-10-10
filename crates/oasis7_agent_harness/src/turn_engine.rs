//! Logical turn correlation above the non-blocking provider actor seam.
//!
//! This layer keeps the exact opaque lease passed by the application adapter
//! and requires Runtime callbacks for lease admission and committed receipt
//! readback. It never owns effect execution or settlement.

use std::collections::BTreeMap;
use std::fmt;

use oasis7_agent_api::{
    CognitionError, CognitionLeaseConsumptionViewV1, CognitionLeaseStatusV1,
    ContinuousAgentRequestContextV1, ContinuousAgentTurnContextV1, FeedbackEnvelopeV1,
    MemoryWriteIntentV1, ResponseArtifactIdentityV1, RuntimeReceiptLineageV1,
};
use serde::Serialize;
use serde_json::Value;

use crate::actor::{
    AsyncAgentRunner, AsyncAgentRunnerError, AsyncAgentTurnOutcome, AsyncTurnFeedback, AsyncTurnId,
    AsyncTurnLifecycle,
};
use crate::authority::RuntimeAuthority;
use crate::memory::{MemoryWriteIntentPolicyV1, MemoryWritePolicyContextV1, MemoryWriteStore};
use crate::provider::DecisionProvider;

#[derive(Debug)]
pub struct TurnRequest<L> {
    pub context: ContinuousAgentTurnContextV1,
    pub request: ContinuousAgentRequestContextV1,
    pub lease: Option<L>,
}

impl<L> TurnRequest<L> {
    pub fn new(
        context: ContinuousAgentTurnContextV1,
        request: ContinuousAgentRequestContextV1,
    ) -> Self {
        Self {
            context,
            request,
            lease: None,
        }
    }

    pub fn with_lease(mut self, lease: L) -> Self {
        self.lease = Some(lease);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnEnginePhase {
    Preparing,
    InFlight,
    Candidate,
    AwaitingReceipt,
    Pending,
    Completed,
    Rejected,
    Failed,
    Stale,
    Cancelled,
}

#[derive(Debug)]
pub struct TurnEngineEvent<A = Value, Q = Value> {
    pub turn_id: AsyncTurnId,
    pub phase: TurnEnginePhase,
    pub outcome: AsyncAgentTurnOutcome<A, Q>,
}

pub struct RuntimeReadbackProof<'a, R> {
    pub receipt: &'a R,
    pub response_identity: &'a ResponseArtifactIdentityV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnEngineError {
    Runner(AsyncAgentRunnerError),
    UnknownTurn(AsyncTurnId),
    RuntimeReadbackRequired(AsyncTurnId),
    InvalidLeaseProjection(String),
    InvalidReadback(AsyncTurnId, String),
    InvalidContinuationHandoff(AsyncTurnId, String),
    InvalidTransition {
        turn_id: AsyncTurnId,
        from: TurnEnginePhase,
        to: TurnEnginePhase,
    },
    FeedbackTurnMismatch(AsyncTurnId),
}

impl fmt::Display for TurnEngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Runner(error) => write!(formatter, "{error}"),
            Self::UnknownTurn(turn_id) => write!(formatter, "unknown turn {}", turn_id.get()),
            Self::RuntimeReadbackRequired(turn_id) => write!(
                formatter,
                "committed turn {} requires Runtime readback",
                turn_id.get()
            ),
            Self::InvalidLeaseProjection(agent) => write!(
                formatter,
                "Runtime lease projection does not correlate to agent {agent}"
            ),
            Self::InvalidReadback(turn_id, reason) => write!(
                formatter,
                "Runtime readback for turn {} was rejected: {reason}",
                turn_id.get()
            ),
            Self::InvalidContinuationHandoff(turn_id, reason) => write!(
                formatter,
                "Runtime continuation handoff for turn {} was rejected: {reason}",
                turn_id.get()
            ),
            Self::InvalidTransition { turn_id, from, to } => write!(
                formatter,
                "invalid turn {} transition {from:?} -> {to:?}",
                turn_id.get()
            ),
            Self::FeedbackTurnMismatch(turn_id) => {
                write!(formatter, "feedback does not match turn {}", turn_id.get())
            }
        }
    }
}

impl std::error::Error for TurnEngineError {}

impl From<AsyncAgentRunnerError> for TurnEngineError {
    fn from(error: AsyncAgentRunnerError) -> Self {
        Self::Runner(error)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MemoryProjectionReport {
    pub applied: usize,
    pub rejected: Vec<CognitionError>,
}

#[derive(Debug)]
struct TurnRecord<L> {
    agent_id: String,
    context: ContinuousAgentTurnContextV1,
    lease: Option<L>,
    phase: TurnEnginePhase,
}

/// Runtime-backed coordinator for provider turns. `A` and `Q` remain concrete
/// application payload types; the Harness does not serialize them into an
/// invented discriminator or erase them to JSON.
pub struct TurnEngine<A = Value, Q = Value, L = ()> {
    runner: AsyncAgentRunner<A, Q>,
    turns: BTreeMap<AsyncTurnId, TurnRecord<L>>,
}

impl<A, Q, L> TurnEngine<A, Q, L>
where
    A: Serialize + Send + 'static,
    Q: Serialize + Send + 'static,
{
    pub fn new(mailbox_capacity: usize) -> Result<Self, TurnEngineError> {
        Ok(Self {
            runner: AsyncAgentRunner::new(mailbox_capacity)?,
            turns: BTreeMap::new(),
        })
    }

    pub fn with_default_capacity() -> Self {
        Self {
            runner: AsyncAgentRunner::with_default_capacity(),
            turns: BTreeMap::new(),
        }
    }

    pub fn register<P>(
        &mut self,
        agent_id: impl Into<String>,
        provider: P,
    ) -> Result<(), TurnEngineError>
    where
        P: DecisionProvider<A, Q> + 'static,
    {
        self.runner.register(agent_id, provider).map_err(Into::into)
    }

    pub fn start_turn<H: RuntimeAuthority<Lease = L>>(
        &mut self,
        request: TurnRequest<L>,
        authority: &H,
    ) -> Result<AsyncTurnId, TurnEngineError> {
        let agent = request.request.agent_subject.as_str();
        request
            .context
            .validate_for_agent(agent)
            .map_err(|error| TurnEngineError::Runner(AsyncAgentRunnerError::Cognition(error)))?;
        request
            .request
            .validate_production_lane()
            .map_err(|error| TurnEngineError::Runner(AsyncAgentRunnerError::Cognition(error)))?;
        if request.context.agent_id != request.request.agent_subject
            || request.context.agent_session_id != request.request.agent_session_id
            || request.context.agent_turn_id != request.request.agent_turn_id
            || request.context.decision_request_id != request.request.decision_request_id
            || request.context.request_digest != request.request.request_digest
        {
            return Err(TurnEngineError::Runner(AsyncAgentRunnerError::Cognition(
                CognitionError::new(
                    "cognition_context_identity_mismatch",
                    "turn and request contexts do not correlate",
                ),
            )));
        }
        if let Some(lease) = request.lease.as_ref() {
            let view = authority
                .validate_lease(lease, &request.request)
                .map_err(|error| {
                    TurnEngineError::Runner(AsyncAgentRunnerError::Cognition(error))
                })?;
            if !lease_view_matches(&view, &request.request) {
                return Err(TurnEngineError::InvalidLeaseProjection(agent.to_string()));
            }
        }
        let turn_id = self.runner.start_turn(request.request.clone())?;
        self.turns.insert(
            turn_id,
            TurnRecord {
                agent_id: request.request.agent_subject.clone(),
                context: request.context,
                lease: request.lease,
                phase: TurnEnginePhase::InFlight,
            },
        );
        Ok(turn_id)
    }

    pub fn poll_completed(&mut self) -> Result<Vec<TurnEngineEvent<A, Q>>, TurnEngineError> {
        self.runner
            .poll_completed()?
            .into_iter()
            .map(|outcome| {
                let phase = phase_for_outcome(&outcome);
                let record = self
                    .turns
                    .get_mut(&outcome.turn_id)
                    .ok_or(TurnEngineError::UnknownTurn(outcome.turn_id))?;
                record.phase = phase;
                Ok(TurnEngineEvent {
                    turn_id: outcome.turn_id,
                    phase,
                    outcome,
                })
            })
            .collect()
    }

    pub fn retry_awaiting_turn(
        &mut self,
        turn_id: AsyncTurnId,
    ) -> Result<AsyncTurnId, TurnEngineError> {
        let request = self
            .runner
            .awaiting_request(turn_id)
            .ok_or(TurnEngineError::UnknownTurn(turn_id))?
            .clone();
        let result = self.runner.retry_awaiting_turn(turn_id, &request)?;
        let record = self
            .turns
            .get_mut(&turn_id)
            .ok_or(TurnEngineError::UnknownTurn(turn_id))?;
        record.phase = TurnEnginePhase::InFlight;
        Ok(result)
    }

    /// Borrow the exact opaque lease retained from admission. Settlement and
    /// release remain exclusively Runtime operations.
    pub fn lease_for_runtime(&self, turn_id: AsyncTurnId) -> Option<&L> {
        self.turns
            .get(&turn_id)
            .and_then(|record| record.lease.as_ref())
    }

    pub fn phase(&self, turn_id: AsyncTurnId) -> Option<TurnEnginePhase> {
        self.turns.get(&turn_id).map(|record| record.phase)
    }

    pub fn active_turn_count(&self) -> usize {
        self.runner.active_turn_count()
    }

    pub fn provider_is_still_in_flight(&self, agent_id: &str) -> bool {
        self.runner.provider_is_still_in_flight(agent_id)
    }

    pub fn accept_runtime_feedback<H: RuntimeAuthority<Lease = L>>(
        &mut self,
        turn_id: AsyncTurnId,
        feedback: FeedbackEnvelopeV1,
        readback: Option<RuntimeReadbackProof<'_, H::Receipt>>,
        authority: &H,
        memory_store: &mut MemoryWriteStore,
    ) -> Result<(TurnEnginePhase, MemoryProjectionReport), TurnEngineError> {
        let record = self
            .turns
            .get(&turn_id)
            .ok_or(TurnEngineError::UnknownTurn(turn_id))?;
        if feedback.agent_subject != record.agent_id
            || feedback.agent_session_id != record.context.agent_session_id
            || feedback.agent_turn_id != record.context.agent_turn_id
            || feedback.decision_request_id != record.context.decision_request_id
            || feedback.request_digest != record.context.request_digest
        {
            return Err(TurnEngineError::FeedbackTurnMismatch(turn_id));
        }
        feedback
            .validate()
            .map_err(|error| TurnEngineError::InvalidReadback(turn_id, error.to_string()))?;

        let mut memory_report = MemoryProjectionReport::default();
        if feedback.status == "committed" {
            let proof = readback.ok_or(TurnEngineError::RuntimeReadbackRequired(turn_id))?;
            let outcome = self
                .runner
                .awaiting_outcome(turn_id)
                .ok_or(TurnEngineError::RuntimeReadbackRequired(turn_id))?;
            let response = outcome
                .response_context
                .as_deref()
                .ok_or(TurnEngineError::RuntimeReadbackRequired(turn_id))?;
            let expected_identity = response.response_artifact_identity();
            expected_identity
                .validate()
                .map_err(|error| TurnEngineError::InvalidReadback(turn_id, error.to_string()))?;
            if proof.response_identity != &expected_identity {
                return Err(TurnEngineError::InvalidReadback(
                    turn_id,
                    "response artifact identity does not match the retained provider response"
                        .into(),
                ));
            }
            let lineage = authority
                .verify_receipt_readback(&feedback, proof.receipt, proof.response_identity)
                .map_err(|error| TurnEngineError::InvalidReadback(turn_id, error.to_string()))?;
            validate_committed_lineage(&lineage, &record.context, &feedback)
                .map_err(|error| TurnEngineError::InvalidReadback(turn_id, error.to_string()))?;
            memory_report = project_memory_intents(
                outcome,
                &record.context,
                proof.receipt,
                authority,
                memory_store,
            );
        }

        self.runner.accept_feedback(turn_id, feedback.clone())?;
        let next = phase_for_feedback(&feedback);
        if self.phase(turn_id) == Some(TurnEnginePhase::Candidate) {
            self.transition(turn_id, TurnEnginePhase::AwaitingReceipt)?;
        }
        self.transition(turn_id, next)?;
        if matches!(
            next,
            TurnEnginePhase::Completed
                | TurnEnginePhase::Rejected
                | TurnEnginePhase::Failed
                | TurnEnginePhase::Stale
        ) && let Some(record) = self.turns.get_mut(&turn_id)
        {
            record.lease = None;
        }
        Ok((next, memory_report))
    }

    /// Release the local provider single-flight state only after the host
    /// validates an authoritative continuation and the runner is awaiting the
    /// same request identity. Continuation accounting remains Runtime-owned.
    pub fn release_after_continuation_handoff<H: RuntimeAuthority>(
        &mut self,
        turn_id: AsyncTurnId,
        continuation: &H::Continuation,
        authority: &H,
    ) -> Result<(), TurnEngineError> {
        let record = self
            .turns
            .get(&turn_id)
            .ok_or(TurnEngineError::UnknownTurn(turn_id))?;
        let projection = authority
            .continuation_projection(continuation)
            .map_err(|error| {
                TurnEngineError::InvalidContinuationHandoff(turn_id, error.to_string())
            })?;
        let request = &record.context;
        // Match the old RuntimeDecisionRunner release contract: there must be
        // an outcome still awaiting Runtime, and its five request identity
        // fields must match before releasing the process-local actor slot.
        // Do not replace this with a Harness-owned continuation status policy.
        let awaiting = self
            .runner
            .awaiting_outcome(turn_id)
            .is_some_and(|outcome| {
                outcome.agent_id == record.agent_id
                    && outcome.request_context.agent_subject == record.agent_id
                    && outcome.request_context.agent_session_id == request.agent_session_id
                    && outcome.request_context.agent_turn_id == request.agent_turn_id
                    && outcome.request_context.decision_request_id == request.decision_request_id
                    && outcome.request_context.request_digest == request.request_digest
            });
        let correlated = awaiting
            && projection.agent_id == record.agent_id
            && projection.agent_session_id == request.agent_session_id
            && projection.agent_turn_id == request.agent_turn_id
            && projection.decision_request_id == request.decision_request_id
            && projection.origin_turn_id == request.agent_turn_id
            && projection.origin_request_digest == request.request_digest.to_string();
        if !correlated {
            return Err(TurnEngineError::InvalidContinuationHandoff(
                turn_id,
                "continuation does not correlate to the pending request".into(),
            ));
        }
        self.runner.release_turn(turn_id);
        if let Some(record) = self.turns.get_mut(&turn_id) {
            record.lease = None;
        }
        Ok(())
    }

    fn transition(
        &mut self,
        turn_id: AsyncTurnId,
        next: TurnEnginePhase,
    ) -> Result<(), TurnEngineError> {
        let record = self
            .turns
            .get_mut(&turn_id)
            .ok_or(TurnEngineError::UnknownTurn(turn_id))?;
        let current = record.phase;
        let valid = match (current, next) {
            (TurnEnginePhase::InFlight, TurnEnginePhase::Candidate)
            | (TurnEnginePhase::InFlight, TurnEnginePhase::Pending)
            | (TurnEnginePhase::InFlight, TurnEnginePhase::Failed)
            | (TurnEnginePhase::Candidate, TurnEnginePhase::AwaitingReceipt)
            | (TurnEnginePhase::AwaitingReceipt, TurnEnginePhase::Completed)
            | (TurnEnginePhase::AwaitingReceipt, TurnEnginePhase::Rejected)
            | (TurnEnginePhase::AwaitingReceipt, TurnEnginePhase::Stale)
            | (TurnEnginePhase::AwaitingReceipt, TurnEnginePhase::Failed)
            | (TurnEnginePhase::AwaitingReceipt, TurnEnginePhase::Pending)
            | (TurnEnginePhase::Pending, TurnEnginePhase::Completed)
            | (TurnEnginePhase::Pending, TurnEnginePhase::Rejected)
            | (TurnEnginePhase::Pending, TurnEnginePhase::Failed)
            | (TurnEnginePhase::Pending, TurnEnginePhase::Stale)
            | (TurnEnginePhase::Pending, TurnEnginePhase::Pending)
            | (_, TurnEnginePhase::Cancelled) => true,
            (from, to) if from == to => true,
            _ => false,
        };
        if !valid {
            return Err(TurnEngineError::InvalidTransition {
                turn_id,
                from: current,
                to: next,
            });
        }
        record.phase = next;
        Ok(())
    }
}

fn lease_view_matches(
    view: &CognitionLeaseConsumptionViewV1,
    request: &ContinuousAgentRequestContextV1,
) -> bool {
    view.schema_version == CognitionLeaseConsumptionViewV1::SCHEMA_VERSION
        && !view.lease_id.trim().is_empty()
        && view.idempotency_key == request.provider_invocation_key().to_string()
        && view.agent_id == request.agent_subject
        && view.agent_session_id == request.agent_session_id
        && view.agent_turn_id == request.agent_turn_id
        && view.decision_request_id == request.decision_request_id
        && view.request_digest == request.request_digest.to_string()
        && view.status == CognitionLeaseStatusV1::Reserved
        && view.reserved_amount == 1
        && view.reserved_at_tick <= request.runtime_binding.base_tick
}

fn validate_committed_lineage(
    lineage: &RuntimeReceiptLineageV1,
    context: &ContinuousAgentTurnContextV1,
    feedback: &FeedbackEnvelopeV1,
) -> Result<(), CognitionError> {
    lineage.validate()?;
    for digest in [
        &lineage.receipt_digest,
        &lineage.envelope_digest,
        &lineage.request_digest,
    ] {
        if !oasis7_agent_api::Digest32::from(digest.as_str()).is_canonical_blake3() {
            return Err(CognitionError::new(
                "runtime_receipt_lineage_invalid",
                "Runtime readback contains a non-canonical digest",
            ));
        }
    }
    if lineage.agent_id != context.agent_id
        || lineage.agent_session_id != context.agent_session_id
        || lineage.agent_turn_id != context.agent_turn_id
        || lineage.decision_request_id != context.decision_request_id
        || lineage.request_digest != context.request_digest.to_string()
        || lineage.feedback_id != feedback.feedback_id
        || feedback.runtime_receipt_id.as_deref() != Some(lineage.receipt_id.as_str())
    {
        return Err(CognitionError::new(
            "runtime_receipt_lineage_mismatch",
            "Runtime readback does not correlate to feedback",
        ));
    }
    let action_id = feedback.candidate_action_id.ok_or_else(|| {
        CognitionError::new(
            "runtime_receipt_action_missing",
            "committed feedback has no action identity",
        )
    })?;
    if lineage.action_id != action_id.to_string()
        && lineage.action_id != format!("action:{action_id}")
    {
        return Err(CognitionError::new(
            "runtime_receipt_action_mismatch",
            "Runtime action identity does not match feedback",
        ));
    }
    Ok(())
}

fn project_memory_intents<A, Q, H>(
    outcome: &AsyncAgentTurnOutcome<A, Q>,
    context: &ContinuousAgentTurnContextV1,
    receipt: &H::Receipt,
    authority: &H,
    store: &mut MemoryWriteStore,
) -> MemoryProjectionReport
where
    A: Serialize + Send + 'static,
    Q: Serialize + Send + 'static,
    H: RuntimeAuthority,
{
    let Some(response) = outcome.response_context.as_deref() else {
        return MemoryProjectionReport::default();
    };
    let policy_context = MemoryWritePolicyContextV1 {
        agent_id: context.agent_id.clone(),
        agent_session_id: context.agent_session_id.clone(),
        agent_turn_id: context.agent_turn_id.clone(),
        request_digest: context.request_digest.to_string(),
        source: "provider".into(),
        provenance: "provider_unverified".into(),
    };
    let policy = MemoryWriteIntentPolicyV1::default();
    let mut report = MemoryProjectionReport::default();
    for intent in &response.base_decision_response.memory_write_intents {
        let proposed = MemoryWriteIntentV1 {
            schema_version: 1,
            scope: intent.scope.clone(),
            summary: Some(intent.summary.clone()),
            tags: intent.tags.clone(),
            compatibility_reason: None,
        };
        let normalized = match policy.normalize(proposed, &policy_context) {
            Ok(intent) => intent,
            Err(error) => {
                report.rejected.push(error);
                continue;
            }
        };
        let digest = match policy.intent_digest(&normalized, &policy_context) {
            Ok(digest) => digest,
            Err(error) => {
                report.rejected.push(error);
                continue;
            }
        };
        match store.apply_runtime_receipt_with_context(
            normalized,
            digest,
            receipt,
            Some(&policy_context),
            authority,
        ) {
            Ok(()) => report.applied += 1,
            Err(error) => report.rejected.push(error),
        }
    }
    report
}

fn phase_for_outcome<A, Q>(outcome: &AsyncAgentTurnOutcome<A, Q>) -> TurnEnginePhase {
    if outcome.lifecycle == AsyncTurnLifecycle::Failed {
        return TurnEnginePhase::Failed;
    }
    match outcome.feedback {
        AsyncTurnFeedback::ActionProposed
        | AsyncTurnFeedback::QueryProposed
        | AsyncTurnFeedback::ModuleCommandProposed => TurnEnginePhase::Candidate,
        AsyncTurnFeedback::ProviderError | AsyncTurnFeedback::ActorPanicked => {
            TurnEnginePhase::Failed
        }
        AsyncTurnFeedback::Wait | AsyncTurnFeedback::WaitTicks(_) => TurnEnginePhase::Pending,
    }
}

fn phase_for_feedback(feedback: &FeedbackEnvelopeV1) -> TurnEnginePhase {
    match feedback.status.as_str() {
        "committed" => TurnEnginePhase::Completed,
        "rejected"
            if matches!(
                feedback.reject_reason.as_deref(),
                Some("stale" | "stale_base")
            ) =>
        {
            TurnEnginePhase::Stale
        }
        "rejected" => TurnEnginePhase::Rejected,
        "failed" => TurnEnginePhase::Failed,
        "pending" => TurnEnginePhase::Pending,
        _ => TurnEnginePhase::Pending,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oasis7_agent_api::{
        CognitionLeaseStatusV1, ContinuousAgentResponseContextV1, DecisionResponse, Digest32,
        GoalSnapshotV1, MemoryContextSnapshotV1, ResponseArtifactIdentityV1,
    };
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    struct FakeReceipt;
    struct FakeContinuation;

    struct FakeAuthority {
        mismatched_lease: bool,
        continuation: Option<crate::ContinuationHostProjection>,
        readback_calls: Arc<AtomicUsize>,
    }

    impl RuntimeAuthority for FakeAuthority {
        type Lease = ();
        type Receipt = FakeReceipt;
        type Continuation = FakeContinuation;

        fn validate_lease(
            &self,
            _lease: &Self::Lease,
            request: &ContinuousAgentRequestContextV1,
        ) -> Result<CognitionLeaseConsumptionViewV1, CognitionError> {
            Ok(CognitionLeaseConsumptionViewV1 {
                schema_version: CognitionLeaseConsumptionViewV1::SCHEMA_VERSION.into(),
                lease_id: "opaque-runtime-lease-1".into(),
                idempotency_key: request.provider_invocation_key().to_string(),
                agent_id: if self.mismatched_lease {
                    "different-agent"
                } else {
                    request.agent_subject.as_str()
                }
                .into(),
                agent_session_id: request.agent_session_id.clone(),
                agent_turn_id: request.agent_turn_id.clone(),
                decision_request_id: request.decision_request_id.clone(),
                request_digest: request.request_digest.to_string(),
                status: CognitionLeaseStatusV1::Reserved,
                reserved_amount: 1,
                reserved_at_tick: request.runtime_binding.base_tick,
            })
        }

        fn verify_receipt_readback(
            &self,
            _feedback: &FeedbackEnvelopeV1,
            _receipt: &Self::Receipt,
            _response_identity: &ResponseArtifactIdentityV1,
        ) -> Result<RuntimeReceiptLineageV1, CognitionError> {
            self.readback_calls.fetch_add(1, Ordering::SeqCst);
            Err(CognitionError::new(
                "fixture_no_readback",
                "fake host has no committed marker",
            ))
        }

        fn verify_memory_receipt(
            &self,
            _receipt: &Self::Receipt,
            _context: Option<&MemoryWritePolicyContextV1>,
        ) -> Result<RuntimeReceiptLineageV1, CognitionError> {
            Err(CognitionError::new(
                "fixture_no_readback",
                "fake host has no committed marker",
            ))
        }

        fn validated_feedback_history(
            &self,
        ) -> Result<Vec<crate::FeedbackHistoryProjection>, CognitionError> {
            Ok(Vec::new())
        }

        fn continuation_projection(
            &self,
            _continuation: &Self::Continuation,
        ) -> Result<crate::ContinuationHostProjection, CognitionError> {
            self.continuation.clone().ok_or_else(|| {
                CognitionError::new("fixture_no_continuation", "fake host has no continuation")
            })
        }
    }

    struct WaitProvider;

    impl DecisionProvider for WaitProvider {
        fn provider_id(&self) -> &str {
            "wait"
        }

        fn decide(
            &mut self,
            request: &ContinuousAgentRequestContextV1,
        ) -> Result<ContinuousAgentResponseContextV1, crate::DecisionProviderError> {
            let base = DecisionResponse::wait("wait");
            let mut response = ContinuousAgentResponseContextV1 {
                base_decision_response: base,
                context_discriminator: request.context_discriminator.clone(),
                context_version: request.context_version,
                agent_session_id: request.agent_session_id.clone(),
                agent_turn_id: request.agent_turn_id.clone(),
                decision_request_id: request.decision_request_id.clone(),
                retry_seq: request.retry_seq,
                transport_attempt: request.transport_attempt,
                request_digest: request.request_digest.clone(),
                response_digest: Digest32::default(),
            };
            response.response_digest =
                crate::provider::cognition_response_digest(&response.base_decision_response);
            Ok(response)
        }
    }

    fn context(request: &ContinuousAgentRequestContextV1) -> ContinuousAgentTurnContextV1 {
        ContinuousAgentTurnContextV1 {
            agent_id: request.agent_subject.clone(),
            agent_session_id: request.agent_session_id.clone(),
            agent_turn_id: request.agent_turn_id.clone(),
            decision_request_id: request.decision_request_id.clone(),
            request_digest: request.request_digest.clone(),
            memory_snapshot: MemoryContextSnapshotV1::empty("turn_private"),
            goal_snapshot: GoalSnapshotV1::empty(),
            continuation: None,
        }
    }

    fn authority(mismatched_lease: bool) -> FakeAuthority {
        FakeAuthority {
            mismatched_lease,
            continuation: None,
            readback_calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn continuation_authority(
        request: &ContinuousAgentRequestContextV1,
        mismatched_origin: bool,
    ) -> FakeAuthority {
        let mut projection = crate::ContinuationHostProjection {
            continuation_proposal_id: "proposal-1".into(),
            proposal_digest:
                "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            world_id: request.runtime_binding.world_id.clone(),
            agent_id: request.agent_subject.clone(),
            agent_session_id: request.agent_session_id.clone(),
            agent_turn_id: request.agent_turn_id.clone(),
            decision_request_id: request.decision_request_id.clone(),
            origin_turn_id: request.agent_turn_id.clone(),
            origin_request_digest: request.request_digest.to_string(),
            action_or_envelope_digest: None,
            precondition_digest:
                "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            valid_until_tick: None,
            continuation_id: "continuation-1".into(),
            wake_id: "wake-1".into(),
            wake_seq: 0,
            continuation_digest:
                "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into(),
            continuation_status_digest:
                "blake3:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd".into(),
            status: crate::ContinuationStatusProjectionV1::Scheduled,
            terminal_disposition: None,
            remaining_budget: oasis7_agent_api::ContinuationBudgetV1 {
                unit: "ticks".into(),
                value: 2,
            },
        };
        if mismatched_origin {
            projection.origin_request_digest =
                "blake3:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".into();
        }
        FakeAuthority {
            mismatched_lease: false,
            continuation: Some(projection),
            readback_calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    #[test]
    fn fabricated_lineage_dto_does_not_satisfy_readback_callback() {
        // The only committed-feedback path takes an opaque receipt and asks
        // the host callback to prove readback; a DTO cannot construct proof.
        let _lineage_type_is_not_a_proof: Option<RuntimeReceiptLineageV1> = None;
        let missing = TurnEngineError::RuntimeReadbackRequired(AsyncTurnId::from_test_only(1));
        assert!(matches!(
            missing,
            TurnEngineError::RuntimeReadbackRequired(_)
        ));
    }

    #[test]
    fn mismatched_read_only_lease_projection_is_rejected_before_actor_admission() {
        let request = crate::actor::tests::request("agent-a", "request-a", "turn-a");
        let mut engine = TurnEngine::<serde_json::Value, serde_json::Value, ()>::new(1).unwrap();
        let host = authority(true);
        let result = engine.start_turn(
            TurnRequest::new(context(&request), request).with_lease(()),
            &host,
        );
        assert!(
            matches!(result, Err(TurnEngineError::InvalidLeaseProjection(agent)) if agent == "agent-a")
        );
        assert_eq!(engine.active_turn_count(), 0);
    }

    #[test]
    fn committed_feedback_cannot_use_a_serialized_lineage_as_readback_proof() {
        let request = crate::actor::tests::request("agent-a", "request-a", "turn-a");
        let mut engine = TurnEngine::<serde_json::Value, serde_json::Value, ()>::new(1).unwrap();
        engine.register("agent-a", WaitProvider).unwrap();
        let host = authority(false);
        let turn_id = engine
            .start_turn(TurnRequest::new(context(&request), request.clone()), &host)
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while engine.poll_completed().unwrap().is_empty() {
            assert!(
                Instant::now() < deadline,
                "provider result must become pollable"
            );
            std::thread::yield_now();
        }
        let feedback = FeedbackEnvelopeV1 {
            feedback_id: "feedback-a".into(),
            feedback_seq: 1,
            agent_subject: request.agent_subject.clone(),
            agent_session_id: request.agent_session_id.clone(),
            agent_turn_id: request.agent_turn_id.clone(),
            decision_request_id: request.decision_request_id.clone(),
            candidate_action_id: Some(1),
            runtime_receipt_id: Some("receipt-a".into()),
            status: "committed".into(),
            request_digest: request.request_digest.clone(),
            reject_reason: None,
            provenance: "runtime_authoritative".into(),
        };
        let mut memory = MemoryWriteStore::default();
        let result = engine.accept_runtime_feedback(turn_id, feedback, None, &host, &mut memory);
        assert!(
            matches!(result, Err(TurnEngineError::RuntimeReadbackRequired(id)) if id == turn_id)
        );
        assert_eq!(host.readback_calls.load(Ordering::SeqCst), 0);
        assert_eq!(memory.entries().len(), 0);
    }

    #[test]
    fn continuation_handoff_requires_opaque_runtime_projection_correlated_to_pending_turn() {
        let request = crate::actor::tests::request("agent-a", "request-a", "turn-a");
        let mut engine = TurnEngine::<serde_json::Value, serde_json::Value, ()>::new(1).unwrap();
        engine.register("agent-a", WaitProvider).unwrap();
        let missing_authority = authority(false);
        let turn_id = engine
            .start_turn(
                TurnRequest::new(context(&request), request.clone()),
                &missing_authority,
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let events = loop {
            let events = engine.poll_completed().unwrap();
            if !events.is_empty() {
                break events;
            }
            assert!(
                Instant::now() < deadline,
                "provider Wait must become pollable"
            );
            std::thread::yield_now();
        };
        assert_eq!(events[0].phase, TurnEnginePhase::Pending);

        let mismatched = continuation_authority(&request, true);
        let result =
            engine.release_after_continuation_handoff(turn_id, &FakeContinuation, &mismatched);
        assert!(
            matches!(result, Err(TurnEngineError::InvalidContinuationHandoff(id, _)) if id == turn_id)
        );
        assert_eq!(engine.active_turn_count(), 1);

        let admitted = continuation_authority(&request, false);
        engine
            .release_after_continuation_handoff(turn_id, &FakeContinuation, &admitted)
            .unwrap();
        assert_eq!(engine.active_turn_count(), 0);
    }

    #[test]
    fn continuation_handoff_requires_the_old_runner_awaiting_outcome() {
        let request = crate::actor::tests::request("agent-a", "request-a", "turn-a");
        let mut engine = TurnEngine::<serde_json::Value, serde_json::Value, ()>::new(1).unwrap();
        engine.register("agent-a", WaitProvider).unwrap();
        let turn_id = engine
            .start_turn(
                TurnRequest::new(context(&request), request.clone()),
                &authority(false),
            )
            .unwrap();

        // The legacy release path searched only the actual Runtime-awaiting
        // outcome set. A matching continuation projection must not release an
        // in-flight actor before the runner has recorded that outcome.
        let host = continuation_authority(&request, false);
        let result = engine.release_after_continuation_handoff(turn_id, &FakeContinuation, &host);
        assert!(matches!(
            result,
            Err(TurnEngineError::InvalidContinuationHandoff(id, _)) if id == turn_id
        ));
        assert_eq!(engine.active_turn_count(), 1);
    }

    #[test]
    fn feedback_status_and_candidate_phase_are_mapped_without_world_effects() {
        let wait = FeedbackEnvelopeV1 {
            feedback_id: "f".into(),
            feedback_seq: 1,
            agent_subject: "a".into(),
            agent_session_id: "s".into(),
            agent_turn_id: "t".into(),
            decision_request_id: "r".into(),
            candidate_action_id: None,
            runtime_receipt_id: None,
            status: "pending".into(),
            request_digest:
                "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            reject_reason: None,
            provenance: "runtime_authoritative".into(),
        };
        assert_eq!(phase_for_feedback(&wait), TurnEnginePhase::Pending);
        let mut committed = wait;
        committed.status = "committed".into();
        assert_eq!(phase_for_feedback(&committed), TurnEnginePhase::Completed);
    }
}
