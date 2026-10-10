//! Bounded asynchronous provider actors.
//!
//! Actor workers receive only shared request DTOs and return typed proposals.
//! They never receive a World, Runtime, lease, receipt, or mutable application
//! state. Polling the runner is non-blocking; a slow provider cannot stall the
//! host loop.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::thread::{self, JoinHandle};

use oasis7_agent_api::{
    CognitionError, ContinuousAgentRequestContextV1, ContinuousAgentResponseContextV1,
    FeedbackEnvelopeV1, ProviderDecision,
};
use serde::Serialize;
use serde_json::Value;

use crate::cognition::AgentCognitionStore;
use crate::provider::{DecisionProvider, DecisionProviderError};

const DEFAULT_MAILBOX_CAPACITY: usize = 16;

/// Stable process-local identity for one logical actor turn. This is not a
/// wire identifier and has no effect on Runtime request identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AsyncTurnId(u64);

impl AsyncTurnId {
    pub const fn get(self) -> u64 {
        self.0
    }

    #[cfg(test)]
    pub(crate) const fn from_test_only(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncTurnLifecycle {
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncTurnFeedback {
    Wait,
    WaitTicks(u64),
    ActionProposed,
    QueryProposed,
    ModuleCommandProposed,
    ProviderError,
    ActorPanicked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsyncWorldEffect {
    NoEffect,
    Proposal,
}

/// A provider result retained for the host's Runtime decision and readback
/// boundary. The response remains typed; no action/query discriminator or
/// JSON-erasure protocol is introduced here.
#[derive(Debug)]
pub struct AsyncAgentTurnOutcome<A = Value, Q = Value> {
    pub turn_id: AsyncTurnId,
    pub agent_id: String,
    pub lifecycle: AsyncTurnLifecycle,
    pub feedback: AsyncTurnFeedback,
    pub world_effect: AsyncWorldEffect,
    pub request_context: ContinuousAgentRequestContextV1,
    pub response_context: Option<Arc<ContinuousAgentResponseContextV1<A, Q>>>,
    pub provider_error: Option<DecisionProviderError>,
}

impl<A, Q> Clone for AsyncAgentTurnOutcome<A, Q> {
    fn clone(&self) -> Self {
        Self {
            turn_id: self.turn_id,
            agent_id: self.agent_id.clone(),
            lifecycle: self.lifecycle,
            feedback: self.feedback,
            world_effect: self.world_effect,
            request_context: self.request_context.clone(),
            response_context: self.response_context.clone(),
            provider_error: self.provider_error.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsyncAgentRunnerError {
    InvalidCapacity,
    AgentAlreadyRegistered(String),
    AgentNotRegistered(String),
    AgentBusy(String),
    MailboxFull,
    ActorUnavailable(String),
    Cognition(CognitionError),
}

impl AsyncAgentRunnerError {
    pub fn code(&self) -> &str {
        match self {
            Self::InvalidCapacity => "invalid_capacity",
            Self::AgentAlreadyRegistered(_) => "agent_already_registered",
            Self::AgentNotRegistered(_) => "agent_not_registered",
            Self::AgentBusy(_) => "agent_busy",
            Self::MailboxFull => "mailbox_full",
            Self::ActorUnavailable(_) => "actor_unavailable",
            Self::Cognition(error) => error.code(),
        }
    }
}

impl fmt::Display for AsyncAgentRunnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCapacity => formatter.write_str("actor mailbox capacity must be positive"),
            Self::AgentAlreadyRegistered(agent) => {
                write!(formatter, "actor already registered: {agent}")
            }
            Self::AgentNotRegistered(agent) => {
                write!(formatter, "actor is not registered: {agent}")
            }
            Self::AgentBusy(agent) => write!(
                formatter,
                "actor already has an active or awaiting turn: {agent}"
            ),
            Self::MailboxFull => formatter.write_str("actor mailbox is full"),
            Self::ActorUnavailable(agent) => write!(formatter, "actor is unavailable: {agent}"),
            Self::Cognition(error) => write!(formatter, "cognition request rejected: {error}"),
        }
    }
}

impl std::error::Error for AsyncAgentRunnerError {}

impl From<CognitionError> for AsyncAgentRunnerError {
    fn from(error: CognitionError) -> Self {
        Self::Cognition(error)
    }
}

enum ActorCommand {
    Decide {
        turn_id: AsyncTurnId,
        request: Box<ContinuousAgentRequestContextV1>,
    },
    Feedback(Box<FeedbackEnvelopeV1>),
    Shutdown,
}

struct ActorCompletion<A, Q> {
    turn_id: AsyncTurnId,
    result: Result<ContinuousAgentResponseContextV1<A, Q>, DecisionProviderError>,
    panicked: bool,
}

struct AgentActor<A, Q> {
    sender: SyncSender<ActorCommand>,
    completions: Receiver<ActorCompletion<A, Q>>,
    active_turn: Arc<AtomicBool>,
    accepting_commands: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl<A, Q> AgentActor<A, Q>
where
    A: Serialize + Send + 'static,
    Q: Serialize + Send + 'static,
{
    fn new(
        agent_id: String,
        provider: Box<dyn DecisionProvider<A, Q> + 'static>,
        capacity: usize,
    ) -> Result<Self, AsyncAgentRunnerError> {
        let (sender, receiver) = mpsc::sync_channel(capacity);
        let (completion_sender, completions) = mpsc::channel();
        let active_turn = Arc::new(AtomicBool::new(false));
        let accepting_commands = Arc::new(AtomicBool::new(true));
        let worker_accepting = Arc::clone(&accepting_commands);
        let worker = thread::Builder::new()
            .name(format!("agent-harness-{agent_id}"))
            .spawn(move || {
                let mut provider = provider;
                while let Ok(command) = receiver.recv() {
                    match command {
                        ActorCommand::Decide { turn_id, request } => {
                            let result = panic::catch_unwind(AssertUnwindSafe(|| {
                                provider.decide_checked(&request)
                            }));
                            let (result, panicked) = match result {
                                Ok(result) => (result, false),
                                Err(_) => (
                                    Err(DecisionProviderError::new(
                                        "actor_panicked",
                                        "provider panicked while handling a decision request",
                                        false,
                                    )),
                                    true,
                                ),
                            };
                            if completion_sender
                                .send(ActorCompletion {
                                    turn_id,
                                    result,
                                    panicked,
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                        ActorCommand::Feedback(feedback) => {
                            let _ = panic::catch_unwind(AssertUnwindSafe(|| {
                                let _ = provider.push_feedback(&feedback);
                            }));
                        }
                        ActorCommand::Shutdown => break,
                    }
                }
                worker_accepting.store(false, Ordering::Release);
            })
            .map_err(|_| AsyncAgentRunnerError::ActorUnavailable(agent_id.clone()))?;
        Ok(Self {
            sender,
            completions,
            active_turn,
            accepting_commands,
            join: Some(worker),
        })
    }

    fn try_send(&self, command: ActorCommand, agent_id: &str) -> Result<(), AsyncAgentRunnerError> {
        self.sender.try_send(command).map_err(|error| match error {
            TrySendError::Full(_) => AsyncAgentRunnerError::MailboxFull,
            TrySendError::Disconnected(_) => {
                AsyncAgentRunnerError::ActorUnavailable(agent_id.to_string())
            }
        })
    }

    fn try_completion(
        &self,
        agent_id: &str,
    ) -> Result<Option<ActorCompletion<A, Q>>, AsyncAgentRunnerError> {
        match self.completions.try_recv() {
            Ok(completion) => Ok(Some(completion)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(AsyncAgentRunnerError::ActorUnavailable(
                agent_id.to_string(),
            )),
        }
    }
}

impl<A, Q> Drop for AgentActor<A, Q> {
    fn drop(&mut self) {
        self.accepting_commands.store(false, Ordering::Release);
        let shutdown_queued = self.sender.try_send(ActorCommand::Shutdown).is_ok();
        // Never join a provider call from the caller's loop. A busy provider
        // may be detached; idle workers are joined after receiving shutdown.
        if self.active_turn.load(Ordering::Acquire) || !shutdown_queued {
            let _ = self.join.take();
        } else if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

#[derive(Debug)]
struct RunnerTurn<A, Q> {
    agent_id: String,
    request: ContinuousAgentRequestContextV1,
    awaiting_runtime: bool,
    last_outcome: Option<AsyncAgentTurnOutcome<A, Q>>,
}

/// Bounded per-provider actor runner. It tracks request single-flight and
/// transport retries but delegates durable admission, lease settlement,
/// world effects, and receipt readback to the host.
pub struct AsyncAgentRunner<A = Value, Q = Value> {
    actors: BTreeMap<String, AgentActor<A, Q>>,
    mailbox_capacity: usize,
    active_turns: usize,
    next_turn_id: u64,
    turns: BTreeMap<AsyncTurnId, RunnerTurn<A, Q>>,
    awaiting_runtime: BTreeMap<String, AsyncTurnId>,
    completed: VecDeque<AsyncAgentTurnOutcome<A, Q>>,
    cognition: AgentCognitionStore,
}

impl<A, Q> AsyncAgentRunner<A, Q>
where
    A: Serialize + Send + 'static,
    Q: Serialize + Send + 'static,
{
    pub fn new(mailbox_capacity: usize) -> Result<Self, AsyncAgentRunnerError> {
        if mailbox_capacity == 0 {
            return Err(AsyncAgentRunnerError::InvalidCapacity);
        }
        Ok(Self {
            actors: BTreeMap::new(),
            mailbox_capacity,
            active_turns: 0,
            next_turn_id: 1,
            turns: BTreeMap::new(),
            awaiting_runtime: BTreeMap::new(),
            completed: VecDeque::new(),
            cognition: AgentCognitionStore::default(),
        })
    }

    pub fn with_default_capacity() -> Self {
        Self::new(DEFAULT_MAILBOX_CAPACITY).expect("default actor capacity is positive")
    }

    pub fn mailbox_capacity(&self) -> usize {
        self.mailbox_capacity
    }

    pub fn register<P>(
        &mut self,
        agent_id: impl Into<String>,
        provider: P,
    ) -> Result<(), AsyncAgentRunnerError>
    where
        P: DecisionProvider<A, Q> + 'static,
    {
        self.register_boxed(agent_id, Box::new(provider))
    }

    pub fn register_boxed(
        &mut self,
        agent_id: impl Into<String>,
        provider: Box<dyn DecisionProvider<A, Q> + 'static>,
    ) -> Result<(), AsyncAgentRunnerError> {
        let agent_id = agent_id.into();
        if self.actors.contains_key(&agent_id) {
            return Err(AsyncAgentRunnerError::AgentAlreadyRegistered(agent_id));
        }
        let actor = AgentActor::new(agent_id.clone(), provider, self.mailbox_capacity)?;
        self.actors.insert(agent_id, actor);
        Ok(())
    }

    pub fn agent_count(&self) -> usize {
        self.actors.len()
    }

    pub fn active_turn_count(&self) -> usize {
        self.active_turns + self.awaiting_runtime.len()
    }

    pub fn provider_is_still_in_flight(&self, agent_id: &str) -> bool {
        self.actors
            .get(agent_id)
            .is_some_and(|actor| actor.active_turn.load(Ordering::Acquire))
    }

    pub fn start_turn(
        &mut self,
        request: ContinuousAgentRequestContextV1,
    ) -> Result<AsyncTurnId, AsyncAgentRunnerError> {
        request.validate_production_lane()?;
        let agent_id = request.agent_subject.clone();
        if self.awaiting_runtime.contains_key(&agent_id)
            || self
                .actors
                .get(&agent_id)
                .is_some_and(|actor| actor.active_turn.load(Ordering::Acquire))
        {
            return Err(AsyncAgentRunnerError::AgentBusy(agent_id));
        }
        let actor = self
            .actors
            .get(&agent_id)
            .ok_or_else(|| AsyncAgentRunnerError::AgentNotRegistered(agent_id.clone()))?;
        if !actor.accepting_commands.load(Ordering::Acquire) {
            return Err(AsyncAgentRunnerError::ActorUnavailable(agent_id));
        }
        self.cognition.begin_request(request.clone())?;
        let turn_id = AsyncTurnId(self.next_turn_id);
        self.next_turn_id = self.next_turn_id.checked_add(1).ok_or_else(|| {
            AsyncAgentRunnerError::Cognition(CognitionError::new(
                "turn_id_exhausted",
                "process-local turn identity space is exhausted",
            ))
        })?;
        if let Err(error) = actor.try_send(
            ActorCommand::Decide {
                turn_id,
                request: Box::new(request.clone()),
            },
            &agent_id,
        ) {
            self.cognition.clear_agent(&agent_id);
            return Err(error);
        }
        actor.active_turn.store(true, Ordering::Release);
        self.active_turns += 1;
        self.turns.insert(
            turn_id,
            RunnerTurn {
                agent_id,
                request,
                awaiting_runtime: false,
                last_outcome: None,
            },
        );
        Ok(turn_id)
    }

    /// Poll every actor without blocking for provider work.
    pub fn poll_completed(
        &mut self,
    ) -> Result<Vec<AsyncAgentTurnOutcome<A, Q>>, AsyncAgentRunnerError> {
        let mut outcomes = Vec::new();
        for (agent_id, actor) in &mut self.actors {
            let Some(completion) = actor.try_completion(agent_id)? else {
                continue;
            };
            actor.active_turn.store(false, Ordering::Release);
            self.active_turns = self.active_turns.saturating_sub(1);
            let turn = self.turns.get_mut(&completion.turn_id).ok_or_else(|| {
                AsyncAgentRunnerError::Cognition(CognitionError::new(
                    "unknown_actor_completion",
                    "actor completed an unknown process-local turn",
                ))
            })?;
            let (lifecycle, feedback, world_effect, response_context, provider_error) =
                match completion.result {
                    Ok(response) => {
                        let (feedback, effect) =
                            classify_decision(&response.base_decision_response.decision);
                        (
                            AsyncTurnLifecycle::Completed,
                            feedback,
                            effect,
                            Some(Arc::new(response)),
                            None,
                        )
                    }
                    Err(error) if completion.panicked => (
                        AsyncTurnLifecycle::Failed,
                        AsyncTurnFeedback::ActorPanicked,
                        AsyncWorldEffect::NoEffect,
                        None,
                        Some(error),
                    ),
                    Err(error) => (
                        AsyncTurnLifecycle::Failed,
                        AsyncTurnFeedback::ProviderError,
                        AsyncWorldEffect::NoEffect,
                        None,
                        Some(error),
                    ),
                };
            let outcome = AsyncAgentTurnOutcome {
                turn_id: completion.turn_id,
                agent_id: turn.agent_id.clone(),
                lifecycle,
                feedback,
                world_effect,
                request_context: turn.request.clone(),
                response_context,
                provider_error,
            };
            turn.awaiting_runtime = true;
            turn.last_outcome = Some(outcome.clone());
            self.awaiting_runtime
                .insert(turn.agent_id.clone(), completion.turn_id);
            self.completed.push_back(outcome.clone());
            outcomes.push(outcome);
        }
        Ok(outcomes)
    }

    pub fn take_completed(&mut self) -> Vec<AsyncAgentTurnOutcome<A, Q>> {
        self.completed.drain(..).collect()
    }

    /// Retry only a Runtime-awaiting turn and only from its exact last
    /// accepted request. The request digest and retry sequence remain stable;
    /// the transport attempt is the sole incremented field.
    pub fn retry_awaiting_turn(
        &mut self,
        turn_id: AsyncTurnId,
        expected_request: &ContinuousAgentRequestContextV1,
    ) -> Result<AsyncTurnId, AsyncAgentRunnerError> {
        let turn = self.turns.get(&turn_id).ok_or_else(|| {
            AsyncAgentRunnerError::Cognition(CognitionError::new("unknown_turn", "turn is unknown"))
        })?;
        if !turn.awaiting_runtime || &turn.request != expected_request {
            return Err(AsyncAgentRunnerError::Cognition(CognitionError::new(
                "retry_identity_mismatch",
                "retry must use the exact last Runtime-awaiting request",
            )));
        }
        let mut retry_request = expected_request.clone();
        retry_request.transport_attempt = retry_request
            .transport_attempt
            .checked_add(1)
            .ok_or_else(|| {
                AsyncAgentRunnerError::Cognition(CognitionError::new(
                    "transport_attempt_exhausted",
                    "transport attempt identity cannot be incremented",
                ))
            })?;
        retry_request.validate_production_lane()?;
        let agent_id = turn.agent_id.clone();
        let actor = self
            .actors
            .get(&agent_id)
            .ok_or_else(|| AsyncAgentRunnerError::AgentNotRegistered(agent_id.clone()))?;
        if actor.active_turn.load(Ordering::Acquire) {
            return Err(AsyncAgentRunnerError::AgentBusy(agent_id));
        }
        if !actor.accepting_commands.load(Ordering::Acquire) {
            return Err(AsyncAgentRunnerError::ActorUnavailable(agent_id));
        }
        actor.try_send(
            ActorCommand::Decide {
                turn_id,
                request: Box::new(retry_request.clone()),
            },
            &agent_id,
        )?;
        actor.active_turn.store(true, Ordering::Release);
        self.active_turns += 1;
        let turn = self.turns.get_mut(&turn_id).expect("turn checked above");
        turn.request = retry_request;
        turn.awaiting_runtime = false;
        turn.last_outcome = None;
        self.awaiting_runtime.remove(&agent_id);
        self.completed.retain(|outcome| outcome.turn_id != turn_id);
        Ok(turn_id)
    }

    pub fn awaiting_request(
        &self,
        turn_id: AsyncTurnId,
    ) -> Option<&ContinuousAgentRequestContextV1> {
        self.turns
            .get(&turn_id)
            .filter(|turn| turn.awaiting_runtime)
            .map(|turn| &turn.request)
    }

    pub fn awaiting_outcome(&self, turn_id: AsyncTurnId) -> Option<&AsyncAgentTurnOutcome<A, Q>> {
        self.turns
            .get(&turn_id)
            .filter(|turn| turn.awaiting_runtime)
            .and_then(|turn| turn.last_outcome.as_ref())
    }

    pub fn notify_runtime_feedback(
        &self,
        agent_id: &str,
        feedback: FeedbackEnvelopeV1,
    ) -> Result<(), AsyncAgentRunnerError> {
        let actor = self
            .actors
            .get(agent_id)
            .ok_or_else(|| AsyncAgentRunnerError::AgentNotRegistered(agent_id.to_string()))?;
        actor.try_send(ActorCommand::Feedback(Box::new(feedback)), agent_id)
    }

    pub(crate) fn accept_feedback(
        &mut self,
        turn_id: AsyncTurnId,
        feedback: FeedbackEnvelopeV1,
    ) -> Result<(), AsyncAgentRunnerError> {
        let turn = self.turns.get(&turn_id).ok_or_else(|| {
            AsyncAgentRunnerError::Cognition(CognitionError::new("unknown_turn", "turn is unknown"))
        })?;
        let request = &turn.request;
        if !turn.awaiting_runtime
            || feedback.agent_subject != request.agent_subject
            || feedback.agent_session_id != request.agent_session_id
            || feedback.agent_turn_id != request.agent_turn_id
            || feedback.decision_request_id != request.decision_request_id
            || feedback.request_digest != request.request_digest
        {
            return Err(AsyncAgentRunnerError::Cognition(CognitionError::new(
                "feedback_correlation_mismatch",
                "feedback does not match an awaiting turn",
            )));
        }
        feedback.validate()?;
        let agent_id = request.agent_subject.clone();
        self.cognition.accept_feedback(feedback.clone())?;
        if matches!(
            feedback.status.as_str(),
            "committed" | "rejected" | "failed"
        ) {
            self.release_turn(turn_id);
        }
        self.notify_runtime_feedback(&agent_id, feedback)?;
        Ok(())
    }

    /// Release local single-flight correlation only after the host has
    /// accepted authoritative terminal feedback or a continuation handoff.
    pub(crate) fn release_turn(&mut self, turn_id: AsyncTurnId) {
        if let Some(turn) = self.turns.remove(&turn_id) {
            self.awaiting_runtime.remove(&turn.agent_id);
            self.cognition.clear_agent(&turn.agent_id);
        }
    }
}

fn classify_decision<A, Q>(
    decision: &ProviderDecision<A, Q>,
) -> (AsyncTurnFeedback, AsyncWorldEffect) {
    match decision {
        ProviderDecision::Wait => (AsyncTurnFeedback::Wait, AsyncWorldEffect::NoEffect),
        ProviderDecision::WaitTicks { ticks } => (
            AsyncTurnFeedback::WaitTicks(*ticks),
            AsyncWorldEffect::NoEffect,
        ),
        ProviderDecision::Act { .. } => (
            AsyncTurnFeedback::ActionProposed,
            AsyncWorldEffect::Proposal,
        ),
        ProviderDecision::Query { .. } => {
            (AsyncTurnFeedback::QueryProposed, AsyncWorldEffect::Proposal)
        }
        ProviderDecision::ModuleCommand { .. } | ProviderDecision::ModuleCommandResponse { .. } => {
            (
                AsyncTurnFeedback::ModuleCommandProposed,
                AsyncWorldEffect::Proposal,
            )
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use oasis7_agent_api::{
        BudgetContractV1, CapabilityInvocationContext, DecisionRequest, DecisionResponse, Digest32,
        ObservationEnvelope, ProviderObservation, RuntimeBindingV1, h_v1,
    };
    use oasis7_wasm_abi::{
        CapabilityAudience, CapabilityCatalogSnapshot, CapabilityPresenter, CapabilitySubject,
    };
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    const HASH_A: &str = "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const HASH_B: &str = "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    pub(crate) fn request(
        agent: &str,
        request_id: &str,
        turn_id: &str,
    ) -> ContinuousAgentRequestContextV1 {
        let subject = CapabilitySubject::Agent {
            agent_id: agent.into(),
            owner_binding: "owner-1".into(),
            generation: 1,
        };
        let presenter = CapabilityPresenter {
            presenter_id: "provider-1".into(),
            presenter_kind: "provider".into(),
            session_id: Some(format!("session-{agent}")),
            attestation_ref: None,
        };
        let audience = CapabilityAudience {
            world_id: "world-1".into(),
            branch_id: "main".into(),
            finality_epoch: 1,
            target_kind: "world".into(),
            target_id: None,
        };
        let catalog = CapabilityCatalogSnapshot {
            snapshot_id: format!("catalog-{agent}"),
            world_id: "world-1".into(),
            world_head: 1,
            branch_id: "main".into(),
            finality_epoch: 1,
            logical_tick: 1,
            module_registry_hash: HASH_A.into(),
            policy_hash: HASH_B.into(),
            revocation_epoch: 0,
            subject: subject.clone(),
            presenter: presenter.clone(),
            audience: audience.clone(),
            entries: Vec::new(),
            valid_until_tick: 10,
        };
        let invocation = CapabilityInvocationContext {
            grant_id: "grant-1".into(),
            subject,
            presenter,
            audience,
            catalog_snapshot_id: format!("catalog-{agent}"),
            module_id: String::new(),
            module_version: String::new(),
            response_nonce: "nonce-1".into(),
        };
        let base_decision_request = DecisionRequest {
            observation: ObservationEnvelope {
                agent_id: agent.into(),
                world_time: 1,
                mode: Default::default(),
                observation_schema_version: "oc_dual_obs_v1".into(),
                action_schema_version: "oc_dual_act_v1".into(),
                environment_class: None,
                fallback_reason: None,
                observation: ProviderObservation::default(),
                recent_event_summary: Vec::new(),
                memory_summary: None,
                action_catalog: Vec::new(),
                module_command_catalog: Vec::new(),
                timeout_budget_ms: 1000,
            },
            provider_config_ref: None,
            agent_profile: None,
            fixture_id: None,
            replay_id: None,
            capability_catalog: Some(catalog.clone()),
            capability_invocation_context: Some(invocation.clone()),
            timeout_budget_ms: 1000,
        };
        let mut request = ContinuousAgentRequestContextV1 {
            base_decision_request,
            context_discriminator: "oasis7.continuous-agent-context".into(),
            context_version: 1,
            protocol_version: "continuous-agent-v1".into(),
            agent_session_id: format!("session-{agent}"),
            agent_turn_id: turn_id.into(),
            decision_request_id: request_id.into(),
            retry_seq: 1,
            transport_attempt: 1,
            agent_subject: agent.into(),
            runtime_binding: RuntimeBindingV1 {
                world_id: "world-1".into(),
                branch_id: "main".into(),
                finality_epoch: 1,
                finality_block_hash: Some(HASH_A.into()),
                finality_status: "verified".into(),
                base_tick: 1,
                base_world_hash: HASH_B.into(),
                reorg_epoch: 0,
                runtime_manifest_hash: HASH_A.into(),
            },
            observation_digest: HASH_A.into(),
            capability_catalog_digest: h_v1("oasis7.cognition.capability-catalog.v1", &catalog),
            capability_invocation_context_digest: h_v1(
                "oasis7.cognition.capability-invocation-context.v1",
                &invocation,
            ),
            memory_snapshot_digest: HASH_A.into(),
            goal_snapshot_digest: HASH_B.into(),
            continuation_digest: HASH_A.into(),
            adapter_protocol_version: "test-adapter-v1".into(),
            budget_contract: BudgetContractV1 {
                max_latency_ms: 1000,
                max_repair_attempts: 0,
                max_model_calls: 1,
                max_tool_calls: 0,
            },
            request_digest: Digest32::default(),
        };
        request.request_digest = request.request_digest();
        request
            .validate_production_lane()
            .expect("test request is valid");
        request
    }

    struct WaitProvider {
        calls: Arc<AtomicUsize>,
        delay: Duration,
    }

    impl DecisionProvider for WaitProvider {
        fn provider_id(&self) -> &str {
            "wait"
        }
        fn decide(
            &mut self,
            request: &ContinuousAgentRequestContextV1,
        ) -> Result<ContinuousAgentResponseContextV1, DecisionProviderError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            thread::sleep(self.delay);
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
                response_digest: Default::default(),
            };
            response.response_digest =
                crate::provider::cognition_response_digest(&response.base_decision_response);
            Ok(response)
        }
    }

    #[test]
    fn slow_provider_does_not_block_other_actor_progress_and_retry_changes_only_transport() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut runner = AsyncAgentRunner::<Value, Value>::new(1).unwrap();
        runner
            .register(
                "agent-a",
                WaitProvider {
                    calls: Arc::clone(&calls),
                    delay: Duration::from_millis(150),
                },
            )
            .unwrap();
        runner
            .register(
                "agent-b",
                WaitProvider {
                    calls: Arc::clone(&calls),
                    delay: Duration::from_millis(150),
                },
            )
            .unwrap();
        let a = request("agent-a", "request-a", "turn-a");
        let b = request("agent-b", "request-b", "turn-b");
        let turn_a = runner.start_turn(a.clone()).unwrap();
        let turn_b = runner.start_turn(b.clone()).unwrap();
        let start = Instant::now();
        let _ = runner.poll_completed().unwrap();
        assert!(start.elapsed() < Duration::from_millis(100));
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut outcomes = Vec::new();
        while outcomes.len() < 2 {
            outcomes.extend(runner.poll_completed().unwrap());
            assert!(Instant::now() < deadline, "provider outcomes should arrive");
            if outcomes.len() < 2 {
                thread::yield_now();
            }
        }
        let outcome_a = outcomes
            .into_iter()
            .find(|outcome| outcome.turn_id == turn_a)
            .unwrap();
        let outcome_b = runner.awaiting_outcome(turn_b).unwrap();
        assert_eq!(outcome_a.feedback, AsyncTurnFeedback::Wait);
        assert_eq!(outcome_b.feedback, AsyncTurnFeedback::Wait);
        let retry = runner.retry_awaiting_turn(turn_a, &a).unwrap();
        assert_eq!(retry, turn_a);
        let retry_request = runner.awaiting_request(turn_a);
        assert!(
            retry_request.is_none(),
            "retry is in flight until provider completion"
        );
        let result = loop {
            if let Some(outcome) = runner
                .poll_completed()
                .unwrap()
                .into_iter()
                .find(|outcome| outcome.turn_id == turn_a)
            {
                break outcome;
            }
            thread::yield_now();
        };
        assert_eq!(
            result.request_context.transport_attempt,
            a.transport_attempt + 1
        );
        assert_eq!(result.request_context.request_digest, a.request_digest);
        assert_eq!(result.request_context.retry_seq, a.retry_seq);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn request_identity_mismatch_is_rejected_before_provider_dispatch() {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut runner = AsyncAgentRunner::<Value, Value>::new(1).unwrap();
        runner
            .register(
                "agent-a",
                WaitProvider {
                    calls: Arc::clone(&calls),
                    delay: Duration::ZERO,
                },
            )
            .unwrap();
        let mut invalid = request("agent-a", "request-a", "turn-a");
        invalid.agent_subject = "agent-b".into();
        assert!(runner.start_turn(invalid).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}
