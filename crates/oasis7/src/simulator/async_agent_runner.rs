//! Non-blocking simulator actor lifecycle for in-world Agents.
//!
//! `AgentRunner` is the legacy synchronous observe/decide/act adapter.  This
//! module owns the production boundary for provider-backed turns: each
//! registered behavior lives behind a bounded actor mailbox and the world
//! thread only performs non-blocking sends/receives.  A provider may take an
//! arbitrary amount of time (or fail); that work cannot hold a world step.
//!
//! The actor deliberately returns a decision outcome instead of applying a
//! world action.  Runtime owns the later MVCC/authority validation and effect
//! commit.  This keeps the async boundary useful for both Builtin and
//! ProviderBacked behaviors without granting an actor authority over the
//! world.

use std::collections::{BTreeMap, VecDeque};
use std::fmt;
use std::panic::{self, AssertUnwindSafe};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};
use std::thread::{self, JoinHandle};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::runtime::{AgentContinuation as RuntimeAgentContinuation, RuntimeReceiptLineageV1};

use super::Observation;
use super::agent::{ActionResult, AgentBehavior, AgentDecision, AgentDecisionTrace};
use super::cognition_policy::{
    ContinuationHandle, ContinuationProposalV1, MemoryWriteIntentPolicyV1,
    MemoryWritePolicyContextV1, MemoryWriteStore, RuntimeContinuationStatusV1,
};
use super::continuous_agent_harness::{
    ContinuousAgentTurnContextV1, FeedbackEnvelopeV1, MemoryWriteIntentV1, h_v1,
};
use super::decision_provider::{
    ActionCatalogEntry, MemoryWriteIntent, MockDecisionProvider, ProviderBackedAgentBehavior,
};
use super::kernel::{WorldEvent, WorldKernel};
use super::types::WorldTime;

const DEFAULT_MAILBOX_CAPACITY: usize = 16;

/// Stable identifier for an actor turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AsyncTurnId(u64);

impl AsyncTurnId {
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Lifecycle of a decision turn at the async simulator boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AsyncTurnLifecycle {
    Completed,
    Failed,
}

/// Feedback produced by the actor after deciding.  Runtime may append the
/// later action receipt without changing this initial provider outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AsyncTurnFeedback {
    Wait,
    WaitTicks(u64),
    ActionProposed,
    QueryProposed,
    ModuleCommandProposed,
    ProviderError { code: String },
    ActorPanicked,
}

/// The async actor's world-facing proposal.  `NoEffect` is intentional:
/// actual state mutation is outside this simulator actor and remains owned by
/// runtime validation/commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AsyncWorldEffect {
    NoEffect,
    ActionProposal,
    QueryProposal,
    ModuleCommandProposal,
}

/// A completed actor outcome.  The full decision and trace are retained so a
/// runtime adapter can bind them to an envelope and preserve diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AsyncAgentTurnOutcome {
    pub turn_id: AsyncTurnId,
    pub agent_id: String,
    pub lifecycle: AsyncTurnLifecycle,
    pub feedback: AsyncTurnFeedback,
    pub world_effect: AsyncWorldEffect,
    pub decision: Option<AgentDecision>,
    pub decision_trace: Option<AgentDecisionTrace>,
    pub prepared_context: Option<ContinuousAgentTurnContextV1>,
    pub memory_write_intents: Vec<MemoryWriteIntent>,
}

/// Non-blocking progress report returned by a world step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AsyncWorldProgress {
    pub logical_tick: WorldTime,
    pub completed_turns: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AsyncAgentRunnerError {
    InvalidCapacity,
    AgentAlreadyRegistered(String),
    AgentNotRegistered(String),
    AgentBusy(String),
    MailboxFull,
    AgentIdentityMismatch { expected: String, observed: String },
    ActorUnavailable(String),
    FeedbackUnavailable(String),
    Cognition(String),
}

impl AsyncAgentRunnerError {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidCapacity => "invalid_capacity",
            Self::AgentAlreadyRegistered(_) => "agent_already_registered",
            Self::AgentNotRegistered(_) => "agent_not_registered",
            Self::AgentBusy(_) => "agent_busy",
            Self::MailboxFull => "mailbox_full",
            Self::AgentIdentityMismatch { .. } => "agent_identity_mismatch",
            Self::ActorUnavailable(_) => "actor_unavailable",
            Self::FeedbackUnavailable(_) => "feedback_unavailable",
            Self::Cognition(_) => "cognition_error",
        }
    }
}

impl fmt::Display for AsyncAgentRunnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCapacity => write!(f, "actor mailbox capacity must be greater than zero"),
            Self::AgentAlreadyRegistered(agent_id) => {
                write!(f, "agent is already registered: {agent_id}")
            }
            Self::AgentNotRegistered(agent_id) => write!(f, "agent is not registered: {agent_id}"),
            Self::AgentBusy(agent_id) => write!(f, "agent already has an active turn: {agent_id}"),
            Self::MailboxFull => write!(f, "async agent mailbox is full"),
            Self::AgentIdentityMismatch { expected, observed } => {
                write!(
                    f,
                    "observation agent identity mismatch: expected {expected}, got {observed}"
                )
            }
            Self::ActorUnavailable(agent_id) => write!(f, "actor is unavailable: {agent_id}"),
            Self::FeedbackUnavailable(agent_id) => {
                write!(f, "actor feedback mailbox is unavailable: {agent_id}")
            }
            Self::Cognition(message) => write!(f, "cognition context error: {message}"),
        }
    }
}

impl std::error::Error for AsyncAgentRunnerError {}

enum ActorCommand {
    Decide {
        turn_id: AsyncTurnId,
        observation: Observation,
        context: Option<ContinuousAgentTurnContextV1>,
    },
    ActionResult(ActionResult),
    Event(WorldEvent),
    Shutdown,
}

struct ActorCompletion {
    turn_id: AsyncTurnId,
    agent_id: String,
    decision: Option<AgentDecision>,
    decision_trace: Option<AgentDecisionTrace>,
    prepared_context: Option<ContinuousAgentTurnContextV1>,
    memory_write_intents: Vec<MemoryWriteIntent>,
    panicked: bool,
}

struct AgentActor {
    sender: SyncSender<ActorCommand>,
    completions: Receiver<ActorCompletion>,
    active_turn: Arc<AtomicBool>,
    accepting_commands: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

impl AgentActor {
    fn new(
        agent_id: String,
        behavior: Box<dyn AgentBehavior + Send>,
        capacity: usize,
        shutdown: Arc<AtomicBool>,
    ) -> Result<Self, AsyncAgentRunnerError> {
        let (sender, receiver) = mpsc::sync_channel(capacity);
        let (completion_sender, completions) = mpsc::channel();
        let active_turn = Arc::new(AtomicBool::new(false));
        let accepting_commands = Arc::new(AtomicBool::new(true));
        let worker_accepting = Arc::clone(&accepting_commands);
        let worker_shutdown = Arc::clone(&shutdown);
        let worker_agent_id = agent_id.clone();
        let worker = thread::Builder::new()
            .name(format!("oasis7-agent-{agent_id}"))
            .spawn(move || {
                let mut behavior = behavior;
                while let Ok(command) = receiver.recv() {
                    match command {
                        ActorCommand::Decide {
                            turn_id,
                            observation,
                            context,
                        } => {
                            let result = panic::catch_unwind(AssertUnwindSafe(|| {
                                behavior.set_continuous_turn_context(context.as_ref());
                                let decision = behavior.decide(&observation);
                                let trace = behavior.take_decision_trace();
                                let memory_write_intents = behavior.take_memory_write_intents();
                                (decision, trace, memory_write_intents)
                            }));
                            let (decision, decision_trace, memory_write_intents, panicked) =
                                match result {
                                    Ok((decision, trace, memory_write_intents)) => {
                                        (Some(decision), trace, memory_write_intents, false)
                                    }
                                    Err(_) => (None, None, Vec::new(), true),
                                };
                            let completion = ActorCompletion {
                                turn_id,
                                agent_id: worker_agent_id.clone(),
                                decision,
                                decision_trace,
                                prepared_context: context,
                                memory_write_intents,
                                panicked,
                            };
                            if completion_sender.send(completion).is_err() {
                                break;
                            }
                        }
                        ActorCommand::ActionResult(result) => {
                            let _ = panic::catch_unwind(AssertUnwindSafe(|| {
                                behavior.on_action_result(&result);
                            }));
                        }
                        ActorCommand::Event(event) => {
                            let _ = panic::catch_unwind(AssertUnwindSafe(|| {
                                behavior.on_event(&event);
                            }));
                        }
                        ActorCommand::Shutdown => break,
                    }
                    if worker_shutdown.load(Ordering::Acquire) {
                        break;
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
            shutdown,
            join: Some(worker),
        })
    }

    fn try_send(&self, command: ActorCommand) -> Result<(), AsyncAgentRunnerError> {
        self.sender.try_send(command).map_err(|error| match error {
            TrySendError::Full(_) => AsyncAgentRunnerError::MailboxFull,
            TrySendError::Disconnected(_) => AsyncAgentRunnerError::ActorUnavailable(String::new()),
        })
    }

    fn try_completion(&self) -> Result<Option<ActorCompletion>, AsyncAgentRunnerError> {
        match self.completions.try_recv() {
            Ok(completion) => Ok(Some(completion)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                Err(AsyncAgentRunnerError::ActorUnavailable(String::new()))
            }
        }
    }
}

impl Drop for AgentActor {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        self.accepting_commands.store(false, Ordering::Release);
        let shutdown_queued = self.sender.try_send(ActorCommand::Shutdown).is_ok();
        // Never join a provider call from the world thread.  A cooperative
        // actor joins when idle; a provider that ignores shutdown is detached
        // so dropping the runner remains non-blocking as promised.
        if self.active_turn.load(Ordering::Acquire) || !shutdown_queued {
            let _ = self.join.take();
        } else if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

/// Bounded, non-blocking actor runner for simulator Agent behaviors.
pub struct AsyncAgentRunner {
    actors: BTreeMap<String, AgentActor>,
    mailbox_capacity: usize,
    active_turns: usize,
    next_turn_id: u64,
    logical_tick: WorldTime,
    completed: VecDeque<AsyncAgentTurnOutcome>,
    continuations: BTreeMap<String, ContinuationHandle>,
}

impl AsyncAgentRunner {
    pub fn new(mailbox_capacity: usize) -> Result<Self, AsyncAgentRunnerError> {
        if mailbox_capacity == 0 {
            return Err(AsyncAgentRunnerError::InvalidCapacity);
        }
        Ok(Self {
            actors: BTreeMap::new(),
            mailbox_capacity,
            active_turns: 0,
            next_turn_id: 0,
            logical_tick: 0,
            completed: VecDeque::new(),
            continuations: BTreeMap::new(),
        })
    }

    pub fn with_default_capacity() -> Self {
        Self::new(DEFAULT_MAILBOX_CAPACITY).expect("default actor mailbox capacity is valid")
    }

    pub fn mailbox_capacity(&self) -> usize {
        self.mailbox_capacity
    }

    pub fn register<B>(&mut self, behavior: B) -> Result<(), AsyncAgentRunnerError>
    where
        B: AgentBehavior + Send + 'static,
    {
        self.register_boxed(Box::new(behavior))
    }

    pub fn register_boxed(
        &mut self,
        behavior: Box<dyn AgentBehavior + Send>,
    ) -> Result<(), AsyncAgentRunnerError> {
        let agent_id = behavior.agent_id().to_string();
        if self.actors.contains_key(&agent_id) {
            return Err(AsyncAgentRunnerError::AgentAlreadyRegistered(agent_id));
        }
        let shutdown = Arc::new(AtomicBool::new(false));
        let actor = AgentActor::new(agent_id.clone(), behavior, self.mailbox_capacity, shutdown)?;
        self.actors.insert(agent_id, actor);
        Ok(())
    }

    fn register_boxed_with_shutdown(
        &mut self,
        behavior: Box<dyn AgentBehavior + Send>,
        shutdown: Arc<AtomicBool>,
    ) -> Result<(), AsyncAgentRunnerError> {
        let agent_id = behavior.agent_id().to_string();
        if self.actors.contains_key(&agent_id) {
            return Err(AsyncAgentRunnerError::AgentAlreadyRegistered(agent_id));
        }
        let actor = AgentActor::new(agent_id.clone(), behavior, self.mailbox_capacity, shutdown)?;
        self.actors.insert(agent_id, actor);
        Ok(())
    }

    pub fn agent_count(&self) -> usize {
        self.actors.len()
    }

    pub fn logical_tick(&self) -> WorldTime {
        self.logical_tick
    }

    pub fn active_turn_count(&self) -> usize {
        self.active_turns
    }

    pub fn provider_is_still_in_flight(&self, agent_id: &str) -> bool {
        self.actors
            .get(agent_id)
            .is_some_and(|actor| actor.active_turn.load(Ordering::Acquire))
    }

    pub fn start_turn(&mut self, agent_id: &str) -> Result<AsyncTurnId, AsyncAgentRunnerError> {
        let observation = default_observation(agent_id, self.logical_tick);
        self.start_turn_with_context_and_observation(agent_id, observation, None)
    }

    pub fn start_turn_with_observation(
        &mut self,
        agent_id: &str,
        observation: Observation,
    ) -> Result<AsyncTurnId, AsyncAgentRunnerError> {
        self.start_turn_with_context_and_observation(agent_id, observation, None)
    }

    pub fn start_turn_with_context(
        &mut self,
        agent_id: &str,
        context: ContinuousAgentTurnContextV1,
    ) -> Result<AsyncTurnId, AsyncAgentRunnerError> {
        context
            .validate_for_agent(agent_id)
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        let observation = default_observation(agent_id, self.logical_tick);
        self.start_turn_with_context_and_observation(agent_id, observation, Some(context))
    }

    fn start_turn_with_context_and_observation(
        &mut self,
        agent_id: &str,
        observation: Observation,
        context: Option<ContinuousAgentTurnContextV1>,
    ) -> Result<AsyncTurnId, AsyncAgentRunnerError> {
        if self
            .continuations
            .get(agent_id)
            .is_some_and(|continuation| continuation.active)
        {
            return Err(AsyncAgentRunnerError::AgentBusy(agent_id.to_string()));
        }
        let actor = self
            .actors
            .get_mut(agent_id)
            .ok_or_else(|| AsyncAgentRunnerError::AgentNotRegistered(agent_id.to_string()))?;
        if observation.agent_id != agent_id {
            return Err(AsyncAgentRunnerError::AgentIdentityMismatch {
                expected: agent_id.to_string(),
                observed: observation.agent_id,
            });
        }
        if actor.active_turn.load(Ordering::Acquire) {
            return Err(AsyncAgentRunnerError::AgentBusy(agent_id.to_string()));
        }
        if !actor.accepting_commands.load(Ordering::Acquire) {
            return Err(AsyncAgentRunnerError::ActorUnavailable(
                agent_id.to_string(),
            ));
        }
        let turn_id = AsyncTurnId(self.next_turn_id);
        self.next_turn_id = self.next_turn_id.saturating_add(1);
        actor.active_turn.store(true, Ordering::Release);
        if let Err(error) = actor.try_send(ActorCommand::Decide {
            turn_id,
            observation,
            context,
        }) {
            actor.active_turn.store(false, Ordering::Release);
            return Err(error.with_agent(agent_id));
        }
        self.active_turns = self.active_turns.saturating_add(1);
        Ok(turn_id)
    }

    /// Poll all actor completion mailboxes without blocking.
    pub fn poll_completed(&mut self) -> Result<Vec<AsyncAgentTurnOutcome>, AsyncAgentRunnerError> {
        let mut outcomes = Vec::new();
        for (agent_id, actor) in &mut self.actors {
            let Some(completion) = actor
                .try_completion()
                .map_err(|error| error.with_agent(agent_id))?
            else {
                continue;
            };
            actor.active_turn.store(false, Ordering::Release);
            self.active_turns = self.active_turns.saturating_sub(1);
            let outcome = outcome_from_completion(completion);
            self.completed.push_back(outcome.clone());
            outcomes.push(outcome);
        }
        Ok(outcomes)
    }

    pub fn take_completed(&mut self) -> Vec<AsyncAgentTurnOutcome> {
        self.completed.drain(..).collect()
    }

    /// Advance the simulator's logical clock and poll actor results.  There
    /// is no provider call, receive wait, or behavior invocation on this path.
    pub fn step_world_without_waiting_for_provider(
        &mut self,
    ) -> Result<AsyncWorldProgress, AsyncAgentRunnerError> {
        self.logical_tick = self.logical_tick.saturating_add(1);
        let completed_turns = self.poll_completed()?.len();
        Ok(AsyncWorldProgress {
            logical_tick: self.logical_tick,
            completed_turns,
        })
    }

    /// Runtime-facing adapter: execute one kernel step while keeping provider
    /// work outside the world thread.  Kernel action application is still
    /// synchronous and authoritative; only actor polling is async here.
    pub fn step_kernel_without_waiting_for_provider(
        &mut self,
        kernel: &mut WorldKernel,
    ) -> Result<(Option<WorldEvent>, AsyncWorldProgress), AsyncAgentRunnerError> {
        let event = kernel.step();
        self.logical_tick = kernel.time();
        let completed_turns = self.poll_completed()?.len();
        Ok((
            event,
            AsyncWorldProgress {
                logical_tick: self.logical_tick,
                completed_turns,
            },
        ))
    }

    /// Wait for one explicitly requested turn.  World steps must use
    /// `step_world_without_waiting_for_provider`; this convenience is for
    /// tests/CLI adapters that intentionally await an actor outcome.
    pub fn run_one_turn(&mut self) -> Result<AsyncAgentTurnOutcome, AsyncAgentRunnerError> {
        let agent_id = self
            .actors
            .keys()
            .next()
            .cloned()
            .ok_or_else(|| AsyncAgentRunnerError::AgentNotRegistered("<none>".to_string()))?;
        let turn_id = self.start_turn(&agent_id)?;
        loop {
            if let Some(outcome) = self
                .poll_completed()?
                .into_iter()
                .find(|outcome| outcome.turn_id == turn_id)
            {
                return Ok(outcome);
            }
            thread::yield_now();
        }
    }

    /// Send an action result to the actor without executing callback code on
    /// the world thread.  The callback is bounded by the actor mailbox.
    pub fn notify_action_result(
        &mut self,
        agent_id: &str,
        result: ActionResult,
    ) -> Result<(), AsyncAgentRunnerError> {
        let actor = self
            .actors
            .get(agent_id)
            .ok_or_else(|| AsyncAgentRunnerError::AgentNotRegistered(agent_id.to_string()))?;
        actor
            .try_send(ActorCommand::ActionResult(result))
            .map_err(|error| error.with_feedback_agent(agent_id))
    }

    pub fn notify_world_event(
        &mut self,
        agent_id: &str,
        event: WorldEvent,
    ) -> Result<(), AsyncAgentRunnerError> {
        let actor = self
            .actors
            .get(agent_id)
            .ok_or_else(|| AsyncAgentRunnerError::AgentNotRegistered(agent_id.to_string()))?;
        actor
            .try_send(ActorCommand::Event(event))
            .map_err(|error| error.with_feedback_agent(agent_id))
    }

    pub fn consume_runtime_feedback(
        &mut self,
        agent_id: &str,
        feedback: FeedbackEnvelopeV1,
        store: &mut MemoryWriteStore,
    ) -> Result<(), AsyncAgentRunnerError> {
        self.consume_runtime_feedback_with_lineage(agent_id, feedback, None, store)
    }

    /// Consume feedback accompanied by a Runtime-issued receipt lineage.
    /// Caller-signed feedback remains useful for diagnostics, but cannot
    /// promote provider memory intents without this projection.
    pub fn consume_runtime_feedback_with_lineage(
        &mut self,
        agent_id: &str,
        feedback: FeedbackEnvelopeV1,
        runtime_receipt: Option<&RuntimeReceiptLineageV1>,
        store: &mut MemoryWriteStore,
    ) -> Result<(), AsyncAgentRunnerError> {
        let outcome = self
            .completed
            .iter()
            .rev()
            .find(|outcome| outcome.agent_id == agent_id)
            .cloned()
            .ok_or_else(|| AsyncAgentRunnerError::Cognition("unknown actor outcome".to_string()))?;
        let context = outcome.prepared_context.as_ref().ok_or_else(|| {
            AsyncAgentRunnerError::Cognition("runtime feedback requires a host context".to_string())
        })?;
        validate_feedback(context, agent_id, &feedback)?;
        if feedback.status != "committed" {
            return Ok(());
        }
        let runtime_receipt = runtime_receipt.ok_or_else(|| {
            AsyncAgentRunnerError::Cognition(
                "committed feedback requires a Runtime-issued receipt lineage".to_string(),
            )
        })?;
        validate_runtime_receipt_lineage(context, &feedback, runtime_receipt)?;
        let policy_context = MemoryWritePolicyContextV1 {
            agent_id: context.agent_id.clone(),
            agent_session_id: context.agent_session_id.clone(),
            agent_turn_id: context.agent_turn_id.clone(),
            request_digest: context.request_digest.to_string(),
            source: "provider".to_string(),
            provenance: "provider_unverified".to_string(),
        };
        let policy = MemoryWriteIntentPolicyV1::default();
        let mut normalized = Vec::with_capacity(outcome.memory_write_intents.len());
        for intent in outcome.memory_write_intents {
            let intent = MemoryWriteIntentV1 {
                schema_version: 1,
                scope: intent.scope,
                summary: Some(intent.summary),
                tags: intent.tags,
                compatibility_reason: None,
            };
            let intent = policy
                .normalize(intent, &policy_context)
                .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
            let digest = policy
                .intent_digest(&intent, &policy_context)
                .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
            normalized.push((intent, digest));
        }
        for (intent, digest) in normalized {
            store
                .apply_runtime_receipt(intent, digest, runtime_receipt)
                .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        }
        Ok(())
    }

    pub fn submit_continuation_proposal(
        &mut self,
        agent_id: &str,
        proposal: ContinuationProposalV1,
    ) -> Result<ContinuationHandle, AsyncAgentRunnerError> {
        if !self.actors.contains_key(agent_id) {
            return Err(AsyncAgentRunnerError::AgentNotRegistered(
                agent_id.to_string(),
            ));
        }
        if proposal.agent_id != agent_id {
            return Err(AsyncAgentRunnerError::Cognition(
                "continuation proposal agent identity mismatch".to_string(),
            ));
        }
        proposal
            .validate()
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        if self
            .continuations
            .get(agent_id)
            .is_some_and(|continuation| continuation.active)
        {
            return Err(AsyncAgentRunnerError::AgentBusy(agent_id.to_string()));
        }
        let mut handle = ContinuationHandle {
            proposal,
            continuation_id: String::new(),
            wake_id: String::new(),
            wake_seq: 0,
            continuation_digest: String::new(),
            continuation_status_digest: String::new(),
            status: "scheduled".to_string(),
            terminal_disposition: None,
            active: true,
            provenance: "harness_policy".to_string(),
            world_effect: false,
            provider_invocation_count: 0,
        };
        self.continuations
            .insert(agent_id.to_string(), handle.clone());
        Ok(handle)
    }

    pub fn apply_runtime_continuation_status(
        &mut self,
        agent_id: &str,
        runtime: RuntimeContinuationStatusV1,
    ) -> Result<ContinuationHandle, AsyncAgentRunnerError> {
        let _ = runtime;
        Err(AsyncAgentRunnerError::Cognition(
            "legacy Runtime continuation status lacks an authoritative projection".to_string(),
        ))
    }

    /// Apply a complete Runtime-owned continuation projection.  Runtime, not
    /// this runner, allocates schedule IDs, sequence and status digests.
    pub fn apply_runtime_continuation_projection(
        &mut self,
        agent_id: &str,
        runtime: RuntimeAgentContinuation,
    ) -> Result<ContinuationHandle, AsyncAgentRunnerError> {
        let existing =
            self.continuations.get(agent_id).cloned().ok_or_else(|| {
                AsyncAgentRunnerError::Cognition("unknown continuation".to_string())
            })?;
        runtime
            .validate_authoritative()
            .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
        if runtime.continuation_proposal_id != existing.proposal.continuation_proposal_id
            || runtime.proposal_digest != existing.proposal.proposal_digest
            || runtime.agent_id != existing.proposal.agent_id
            || runtime.agent_session_id != existing.proposal.agent_session_id
            || runtime.agent_turn_id != existing.proposal.agent_turn_id
            || runtime.decision_request_id != existing.proposal.decision_request_id
            || runtime.origin_turn_id != existing.proposal.origin_turn_id
            || runtime.origin_request_digest != existing.proposal.origin_request_digest
        {
            return Err(AsyncAgentRunnerError::Cognition(
                "Runtime continuation projection correlation mismatch".to_string(),
            ));
        }
        let (status, active) = match runtime.status {
            crate::runtime::ContinuationStatusV1::Scheduled => ("scheduled", true),
            crate::runtime::ContinuationStatusV1::Pending => ("pending", true),
            crate::runtime::ContinuationStatusV1::Waking => ("waking", true),
            crate::runtime::ContinuationStatusV1::Consumed => ("consumed", true),
            crate::runtime::ContinuationStatusV1::Completed => ("completed", false),
            crate::runtime::ContinuationStatusV1::Cancelled => ("cancelled", false),
            crate::runtime::ContinuationStatusV1::Invalidated => ("invalidated", false),
            crate::runtime::ContinuationStatusV1::Expired => ("expired", false),
            crate::runtime::ContinuationStatusV1::Rejected => ("rejected", false),
        };
        let handle = ContinuationHandle {
            proposal: existing.proposal,
            continuation_id: runtime.continuation_id.clone(),
            wake_id: runtime.wake_id.clone(),
            wake_seq: runtime.wake_seq,
            continuation_digest: runtime.continuation_digest(),
            continuation_status_digest: runtime
                .continuation_status_digest
                .clone()
                .expect("validated Runtime continuation has a status digest"),
            status: status.to_string(),
            terminal_disposition: runtime.terminal_disposition.clone(),
            active,
            provenance: "runtime_authoritative".to_string(),
            world_effect: false,
            provider_invocation_count: 0,
        };
        if active {
            self.continuations
                .insert(agent_id.to_string(), handle.clone());
        } else {
            self.continuations.remove(agent_id);
        }
        Ok(handle)
    }

    /// Test helper: a provider that cooperatively waits until its actor is
    /// dropped.  It exercises the real actor path without leaking a permanent
    /// worker thread.
    pub fn blocking_provider_fixture(agent_id: &str) -> Self {
        let mut runner = Self::with_default_capacity();
        let shutdown = Arc::new(AtomicBool::new(false));
        let behavior = BlockingProviderBehavior::new(agent_id.to_string(), Arc::clone(&shutdown));
        runner
            .register_boxed_with_shutdown(Box::new(behavior), shutdown)
            .expect("fixture agent registration");
        runner
    }

    /// Test helper representing a built-in behavior.  It uses the same actor
    /// lifecycle and outcome projection as ProviderBacked.
    pub fn builtin_fixture(agent_id: &str) -> Self {
        let mut runner = Self::with_default_capacity();
        runner
            .register(BuiltinWaitBehavior::new(agent_id.to_string()))
            .expect("fixture agent registration");
        runner
    }

    /// Test helper using the real ProviderBackedAgentBehavior adapter and a
    /// deterministic mock provider response.
    pub fn provider_backed_fixture(agent_id: &str) -> Self {
        let mut runner = Self::with_default_capacity();
        let behavior = ProviderBackedAgentBehavior::new(
            agent_id.to_string(),
            MockDecisionProvider::new("async-fixture-provider"),
            vec![ActionCatalogEntry::new("wait", "wait")],
        );
        runner
            .register(behavior)
            .expect("fixture agent registration");
        runner
    }
}

impl Default for AsyncAgentRunner {
    fn default() -> Self {
        Self::with_default_capacity()
    }
}

impl AsyncAgentTurnOutcome {
    pub fn feedback_for_runtime_status(
        &self,
        status: &str,
        runtime_receipt_id: Option<&str>,
    ) -> Result<FeedbackEnvelopeV1, AsyncAgentRunnerError> {
        let context = self.prepared_context.as_ref().ok_or_else(|| {
            AsyncAgentRunnerError::Cognition("runtime feedback requires a host context".to_string())
        })?;
        if !matches!(
            status,
            "pending" | "committed" | "rejected" | "failed" | "cancelled" | "expired" | "stale"
        ) {
            return Err(AsyncAgentRunnerError::Cognition(
                "unknown Runtime feedback status".to_string(),
            ));
        }
        if status == "committed" && runtime_receipt_id.is_none() {
            return Err(AsyncAgentRunnerError::Cognition(
                "committed feedback requires a Runtime receipt".to_string(),
            ));
        }
        let runtime_receipt_id = runtime_receipt_id
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string);
        let feedback_id = h_v1(
            "oasis7.cognition.feedback.v1",
            &json!({
                "agent_id": context.agent_id,
                "agent_session_id": context.agent_session_id,
                "agent_turn_id": context.agent_turn_id,
                "decision_request_id": context.decision_request_id,
                "request_digest": context.request_digest,
                "status": status,
                "runtime_receipt_id": runtime_receipt_id,
            }),
        )
        .to_string();
        Ok(FeedbackEnvelopeV1 {
            feedback_id,
            feedback_seq: 0,
            agent_subject: context.agent_id.clone(),
            agent_session_id: context.agent_session_id.clone(),
            agent_turn_id: context.agent_turn_id.clone(),
            decision_request_id: context.decision_request_id.clone(),
            candidate_action_id: None,
            runtime_receipt_id,
            status: status.to_string(),
            request_digest: context.request_digest.clone(),
            reject_reason: (status != "committed").then(|| status.to_string()),
            // This helper is caller-side fixture plumbing.  A real Runtime
            // feedback projection must be supplied to
            // `consume_runtime_feedback_with_lineage`; never self-label this
            // envelope as authoritative.
            provenance: "harness_unverified".to_string(),
        })
    }
}

fn validate_feedback(
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
    {
        return Err(AsyncAgentRunnerError::Cognition(
            "Runtime feedback does not correlate to the actor turn".to_string(),
        ));
    }
    if feedback.feedback_id.trim().is_empty()
        || !matches!(
            feedback.status.as_str(),
            "pending" | "committed" | "rejected" | "failed" | "cancelled" | "expired" | "stale"
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

fn validate_runtime_receipt_lineage(
    context: &ContinuousAgentTurnContextV1,
    feedback: &FeedbackEnvelopeV1,
    receipt: &RuntimeReceiptLineageV1,
) -> Result<(), AsyncAgentRunnerError> {
    receipt
        .validate()
        .map_err(|error| AsyncAgentRunnerError::Cognition(error.to_string()))?;
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

/// Recover the structured provider error emitted by the provider-backed
/// behavior.  `AgentBehavior::decide` is intentionally a legacy infallible
/// interface, so provider failures are carried in the decision trace while
/// crossing the actor boundary.  Keep the machine-readable code from the
/// trace payload rather than treating the fallback `Wait` sentinel as a
/// successful turn.
fn provider_error_code(trace: &AgentDecisionTrace) -> Option<String> {
    let Some(error) = trace.llm_error.as_deref() else {
        return None;
    };

    let structured_code = trace
        .llm_output
        .as_deref()
        .and_then(|payload| serde_json::from_str::<serde_json::Value>(payload).ok())
        .and_then(|payload| payload.get("provider_error").cloned())
        .and_then(|provider_error| provider_error.get("code").cloned())
        .and_then(|code| code.as_str().map(str::to_owned));
    if structured_code.is_some() {
        return structured_code;
    }

    error
        .split_once(':')
        .map(|(code, _)| code.trim())
        .filter(|code| !code.is_empty())
        .map(str::to_owned)
}

impl AsyncAgentRunnerError {
    fn with_agent(self, agent_id: &str) -> Self {
        match self {
            Self::ActorUnavailable(_) => Self::ActorUnavailable(agent_id.to_string()),
            other => other,
        }
    }

    fn with_feedback_agent(self, agent_id: &str) -> Self {
        match self {
            Self::ActorUnavailable(_) | Self::MailboxFull => {
                Self::FeedbackUnavailable(agent_id.to_string())
            }
            other => other,
        }
    }
}

fn outcome_from_completion(completion: ActorCompletion) -> AsyncAgentTurnOutcome {
    if completion.panicked {
        return AsyncAgentTurnOutcome {
            turn_id: completion.turn_id,
            agent_id: completion.agent_id,
            lifecycle: AsyncTurnLifecycle::Failed,
            feedback: AsyncTurnFeedback::ActorPanicked,
            world_effect: AsyncWorldEffect::NoEffect,
            decision: None,
            decision_trace: None,
            prepared_context: completion.prepared_context,
            memory_write_intents: completion.memory_write_intents,
        };
    }
    if let Some(code) = completion
        .decision_trace
        .as_ref()
        .and_then(provider_error_code)
    {
        return AsyncAgentTurnOutcome {
            turn_id: completion.turn_id,
            agent_id: completion.agent_id,
            lifecycle: AsyncTurnLifecycle::Failed,
            feedback: AsyncTurnFeedback::ProviderError { code },
            world_effect: AsyncWorldEffect::NoEffect,
            decision: None,
            decision_trace: completion.decision_trace,
            prepared_context: completion.prepared_context,
            memory_write_intents: completion.memory_write_intents,
        };
    }
    let decision = completion.decision;
    let (feedback, world_effect) = match decision.as_ref() {
        Some(AgentDecision::Wait) => (AsyncTurnFeedback::Wait, AsyncWorldEffect::NoEffect),
        Some(AgentDecision::WaitTicks(ticks)) => (
            AsyncTurnFeedback::WaitTicks(*ticks),
            AsyncWorldEffect::NoEffect,
        ),
        Some(AgentDecision::Act(_)) => (
            AsyncTurnFeedback::ActionProposed,
            AsyncWorldEffect::ActionProposal,
        ),
        Some(AgentDecision::Query(_)) => (
            AsyncTurnFeedback::QueryProposed,
            AsyncWorldEffect::QueryProposal,
        ),
        Some(AgentDecision::ModuleCommand { .. }) => (
            AsyncTurnFeedback::ModuleCommandProposed,
            AsyncWorldEffect::ModuleCommandProposal,
        ),
        None => (
            AsyncTurnFeedback::ProviderError {
                code: "decision_missing".to_string(),
            },
            AsyncWorldEffect::NoEffect,
        ),
    };
    AsyncAgentTurnOutcome {
        turn_id: completion.turn_id,
        agent_id: completion.agent_id,
        lifecycle: AsyncTurnLifecycle::Completed,
        feedback,
        world_effect,
        decision,
        decision_trace: completion.decision_trace,
        prepared_context: completion.prepared_context,
        memory_write_intents: completion.memory_write_intents,
    }
}

fn default_observation(agent_id: &str, time: WorldTime) -> Observation {
    Observation {
        time,
        agent_id: agent_id.to_string(),
        pos: crate::geometry::GeoPos::new(0, 0, 0),
        self_resources: Default::default(),
        visibility_range_cm: 0,
        visible_agents: Vec::new(),
        visible_locations: Vec::new(),
        module_lifecycle: Default::default(),
        module_market: Default::default(),
        power_market: Default::default(),
        social_state: Default::default(),
    }
}

struct BuiltinWaitBehavior {
    agent_id: String,
}

impl BuiltinWaitBehavior {
    fn new(agent_id: String) -> Self {
        Self { agent_id }
    }
}

impl AgentBehavior for BuiltinWaitBehavior {
    fn agent_id(&self) -> &str {
        &self.agent_id
    }

    fn decide(&mut self, _observation: &Observation) -> AgentDecision {
        AgentDecision::Wait
    }
}

struct BlockingProviderBehavior {
    agent_id: String,
    cancelled: Arc<AtomicBool>,
}

impl BlockingProviderBehavior {
    fn new(agent_id: String, cancelled: Arc<AtomicBool>) -> Self {
        Self {
            agent_id,
            cancelled,
        }
    }
}

impl AgentBehavior for BlockingProviderBehavior {
    fn agent_id(&self) -> &str {
        &self.agent_id
    }

    fn decide(&mut self, _observation: &Observation) -> AgentDecision {
        while !self.cancelled.load(Ordering::Acquire) {
            thread::sleep(std::time::Duration::from_millis(1));
        }
        AgentDecision::Wait
    }
}

impl Drop for BlockingProviderBehavior {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Release);
    }
}
