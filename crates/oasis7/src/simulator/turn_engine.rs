//! Provider-neutral logical turn lifecycle over the native async actor seam.
//!
//! TurnEngine owns correlation and candidate lifecycle. Runtime still owns
//! action semantics, MVCC, journal, lease settlement, and authoritative
//! receipt production.

use std::collections::BTreeMap;
use std::fmt;

use super::agent::AgentBehavior;
use super::async_agent_runner::RuntimeReceiptReadbackVerifier;
use super::async_agent_runner::{
    AsyncAgentRunner, AsyncAgentRunnerError, AsyncAgentTurnOutcome, AsyncTurnFeedback, AsyncTurnId,
    AsyncTurnLifecycle, AsyncWorldEffect,
};
use super::cognition_policy::MemoryWriteStore;
use super::continuous_agent_harness::{
    ContinuousAgentRequestContextV1, ContinuousAgentTurnContextV1, FeedbackEnvelopeV1,
    ResponseArtifactIdentityV1,
};
use super::kernel::Observation;
use crate::runtime::{CognitionLeaseV1, RuntimeReceiptLineageV1};

#[derive(Debug, Clone)]
pub struct TurnRequest {
    pub agent_id: String,
    pub observation: Observation,
    pub turn_context: ContinuousAgentTurnContextV1,
    pub request_context: ContinuousAgentRequestContextV1,
    pub cognition_lease: Option<CognitionLeaseV1>,
}

impl TurnRequest {
    pub fn new(
        observation: Observation,
        turn_context: ContinuousAgentTurnContextV1,
        request_context: ContinuousAgentRequestContextV1,
    ) -> Self {
        Self {
            agent_id: turn_context.agent_id.clone(),
            observation,
            turn_context,
            request_context,
            cognition_lease: None,
        }
    }

    pub fn with_cognition_lease(mut self, cognition_lease: CognitionLeaseV1) -> Self {
        self.cognition_lease = Some(cognition_lease);
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

#[derive(Debug, Clone, PartialEq)]
pub struct TurnEngineEvent {
    pub turn_id: AsyncTurnId,
    pub phase: TurnEnginePhase,
    pub outcome: AsyncAgentTurnOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TurnEngineError {
    Runner(AsyncAgentRunnerError),
    UnknownTurn(AsyncTurnId),
    RuntimeReadbackRequired(AsyncTurnId),
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
            Self::UnknownTurn(turn_id) => write!(formatter, "unknown turn: {}", turn_id.get()),
            Self::RuntimeReadbackRequired(turn_id) => write!(
                formatter,
                "committed turn {} requires Runtime world readback proof",
                turn_id.get()
            ),
            Self::InvalidTransition { turn_id, from, to } => write!(
                formatter,
                "invalid turn {} transition from {from:?} to {to:?}",
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

/// Runtime-owned proof used to admit committed feedback into the cognition
/// lane. A receipt value alone is only a correlation DTO; the verifier must
/// establish the durable World readback before the runner can project memory.
pub struct RuntimeReadbackProof<'a> {
    pub receipt: &'a RuntimeReceiptLineageV1,
    pub response_identity: &'a ResponseArtifactIdentityV1,
    pub verifier: &'a dyn RuntimeReceiptReadbackVerifier,
}

#[derive(Debug, Clone)]
struct TurnRecord {
    agent_id: String,
    session_id: String,
    turn_id: String,
    request_id: String,
    request_digest: super::Digest32,
    phase: TurnEnginePhase,
}

pub struct TurnEngine {
    runner: AsyncAgentRunner,
    turns: BTreeMap<AsyncTurnId, TurnRecord>,
}

impl TurnEngine {
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

    pub fn register<B>(&mut self, behavior: B) -> Result<(), TurnEngineError>
    where
        B: AgentBehavior + Send + 'static,
    {
        self.runner.register(behavior).map_err(Into::into)
    }

    pub fn start_turn(&mut self, request: TurnRequest) -> Result<AsyncTurnId, TurnEngineError> {
        let TurnRequest {
            agent_id,
            observation,
            turn_context,
            request_context,
            cognition_lease,
        } = request;
        let session_id = request_context.agent_session_id.clone();
        let request_turn_id = request_context.agent_turn_id.clone();
        let request_id = request_context.decision_request_id.clone();
        let request_digest = request_context.request_digest.clone();
        let turn_id = match cognition_lease {
            Some(lease) => self
                .runner
                .start_turn_with_request_context_and_observation_and_lease(
                    agent_id.as_str(),
                    observation,
                    turn_context,
                    request_context,
                    lease,
                )?,
            None => self
                .runner
                .start_turn_with_request_context_and_observation(
                    agent_id.as_str(),
                    observation,
                    turn_context,
                    request_context,
                )?,
        };
        self.turns.insert(
            turn_id,
            TurnRecord {
                agent_id,
                session_id,
                turn_id: request_turn_id,
                request_id,
                request_digest,
                phase: TurnEnginePhase::InFlight,
            },
        );
        Ok(turn_id)
    }

    pub fn poll_completed(&mut self) -> Result<Vec<TurnEngineEvent>, TurnEngineError> {
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

    pub fn accept_runtime_feedback(
        &mut self,
        turn_id: AsyncTurnId,
        feedback: FeedbackEnvelopeV1,
        runtime_readback: Option<RuntimeReadbackProof<'_>>,
        memory_store: &mut MemoryWriteStore,
    ) -> Result<TurnEnginePhase, TurnEngineError> {
        let record = self
            .turns
            .get(&turn_id)
            .ok_or(TurnEngineError::UnknownTurn(turn_id))?;
        if record.session_id != feedback.agent_session_id
            || record.turn_id != feedback.agent_turn_id
            || record.request_id != feedback.decision_request_id
            || record.request_digest != feedback.request_digest
        {
            return Err(TurnEngineError::FeedbackTurnMismatch(turn_id));
        }
        // Runtime-authoritative feedback itself is the submission/disposition
        // event. The old public mark_candidate_submitted(turn_id) seam was
        // intentionally removed: a caller must present Runtime feedback and,
        // for committed responses, the strict durable readback proof below.
        if feedback.status == "committed" {
            let Some(readback) = runtime_readback else {
                return Err(TurnEngineError::RuntimeReadbackRequired(turn_id));
            };
            self.runner.consume_runtime_feedback_with_world_readback(
                record.agent_id.as_str(),
                feedback.clone(),
                readback.receipt,
                readback.response_identity,
                readback.verifier,
                memory_store,
            )?;
        } else {
            self.runner.consume_runtime_feedback_with_lineage(
                record.agent_id.as_str(),
                feedback.clone(),
                None,
                memory_store,
            )?;
        }
        if self.phase(turn_id) == Some(TurnEnginePhase::Candidate) {
            self.transition(turn_id, TurnEnginePhase::AwaitingReceipt)?;
        }
        let phase = match feedback.status.as_str() {
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
            _ => self
                .phase(turn_id)
                .ok_or(TurnEngineError::UnknownTurn(turn_id))?,
        };
        self.transition(turn_id, phase)?;
        Ok(phase)
    }

    pub fn phase(&self, turn_id: AsyncTurnId) -> Option<TurnEnginePhase> {
        self.turns.get(&turn_id).map(|record| record.phase)
    }

    pub fn active_turn_count(&self) -> usize {
        self.turns
            .values()
            .filter(|record| {
                matches!(
                    record.phase,
                    TurnEnginePhase::Preparing
                        | TurnEnginePhase::InFlight
                        | TurnEnginePhase::Candidate
                        | TurnEnginePhase::AwaitingReceipt
                        | TurnEnginePhase::Pending
                )
            })
            .count()
    }

    pub fn step_world_without_waiting_for_provider(
        &mut self,
    ) -> Result<super::AsyncWorldProgress, TurnEngineError> {
        Ok(self.runner.step_world_without_waiting_for_provider()?)
    }

    pub fn provider_is_still_in_flight(&self, agent_id: &str) -> bool {
        self.runner.provider_is_still_in_flight(agent_id)
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
            (a, b) if a == b => true,
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

fn phase_for_outcome(outcome: &AsyncAgentTurnOutcome) -> TurnEnginePhase {
    if outcome.lifecycle == AsyncTurnLifecycle::Failed {
        return TurnEnginePhase::Failed;
    }
    match outcome.feedback {
        AsyncTurnFeedback::ActionProposed
        | AsyncTurnFeedback::QueryProposed
        | AsyncTurnFeedback::ModuleCommandProposed => TurnEnginePhase::Candidate,
        AsyncTurnFeedback::ProviderError { .. } | AsyncTurnFeedback::ActorPanicked => {
            TurnEnginePhase::Failed
        }
        AsyncTurnFeedback::Wait | AsyncTurnFeedback::WaitTicks(_) => TurnEnginePhase::Pending,
    }
}

#[allow(dead_code)]
fn _effect_is_candidate_only(effect: AsyncWorldEffect) -> bool {
    !matches!(effect, AsyncWorldEffect::NoEffect)
}
