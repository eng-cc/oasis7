use super::*;

impl<A, Q, I, O, F> AsyncAgentRunner<A, Q, I, O, F>
where
    A: Serialize + Send + 'static,
    Q: Serialize + Send + 'static,
    I: Send + 'static,
    O: Clone + Send + 'static,
    F: Send + 'static,
{
    pub fn start_turn(
        &mut self,
        request: ContinuousAgentRequestContextV1,
    ) -> Result<AsyncTurnId, AsyncAgentRunnerError> {
        request.validate_production_lane()?;
        if !self.cognition.feedback_recovery_loaded() {
            return Err(AsyncAgentRunnerError::Cognition(CognitionError::new(
                "feedback_recovery_required",
                "restore host-validated feedback history before dispatching a provider turn",
            )));
        }
        let agent_id = request.agent_subject.clone();
        if self.awaiting_runtime.contains_key(&agent_id)
            || self.awaiting_compatibility.contains_key(&agent_id)
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
        if !actor.supports_production {
            return Err(AsyncAgentRunnerError::Cognition(CognitionError::new(
                "production_lane_unavailable",
                "actor was registered for compatibility requests only",
            )));
        }
        if !actor.accepting_commands.load(Ordering::Acquire) {
            return Err(AsyncAgentRunnerError::ActorUnavailable(agent_id));
        }
        let request_was_fenced = self
            .cognition
            .has_request_fence(&request.decision_request_id);
        self.cognition.begin_request(request.clone())?;
        let turn_id = AsyncTurnId(self.next_turn_id);
        self.next_turn_id = self.next_turn_id.checked_add(1).ok_or_else(|| {
            AsyncAgentRunnerError::Cognition(CognitionError::new(
                "turn_id_exhausted",
                "process-local turn identity space is exhausted",
            ))
        })?;
        if let Err(error) = actor.try_send(
            ActorCommand::DecideProduction {
                turn_id,
                request: Box::new(request.clone()),
            },
            &agent_id,
        ) {
            if request_was_fenced {
                self.cognition.clear_agent(&agent_id);
            } else {
                self.cognition.rollback_unstarted_request(&request);
            }
            return Err(error);
        }
        self.actor_lanes
            .insert(agent_id.clone(), ActorLane::Production);
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

    /// Admit an explicitly reduced compatibility turn. This lane preserves
    /// the historic ability to run before global host-history recovery while
    /// still applying local identity, replay, and collision checks whenever a
    /// reduced turn context is supplied. It never sets the production restore
    /// flag or creates a trusted request binding.
    pub fn start_compatibility_turn(
        &mut self,
        request: CompatibilityTurnRequest<I>,
    ) -> Result<AsyncTurnId, AsyncAgentRunnerError> {
        let agent_id = request.agent_subject.clone();
        if self.awaiting_runtime.contains_key(&agent_id)
            || self.awaiting_compatibility.contains_key(&agent_id)
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
        if !actor.supports_compatibility {
            return Err(AsyncAgentRunnerError::Cognition(CognitionError::new(
                "compatibility_lane_unavailable",
                "actor was registered for production requests only",
            )));
        }
        if !actor.accepting_commands.load(Ordering::Acquire) {
            return Err(AsyncAgentRunnerError::ActorUnavailable(agent_id));
        }
        let turn_was_fenced = request.turn_context.as_ref().is_some_and(|context| {
            self.cognition
                .has_request_fence(&context.decision_request_id)
        });
        if let Some(context) = request.turn_context.as_ref() {
            context.validate_for_agent(&agent_id)?;
            self.cognition.begin_turn(context)?;
        }
        let turn_id = AsyncTurnId(self.next_turn_id);
        self.next_turn_id = self.next_turn_id.checked_add(1).ok_or_else(|| {
            AsyncAgentRunnerError::Cognition(CognitionError::new(
                "turn_id_exhausted",
                "process-local turn identity space is exhausted",
            ))
        })?;
        let turn_context = request.turn_context.clone();
        if let Err(error) = actor.try_send(
            ActorCommand::DecideCompatibility { turn_id, request },
            &agent_id,
        ) {
            if turn_context.is_some() {
                if turn_was_fenced {
                    self.cognition.clear_agent(&agent_id);
                } else if let Some(context) = turn_context.as_ref() {
                    self.cognition.rollback_unstarted_turn(context);
                }
            }
            return Err(error);
        }
        self.actor_lanes
            .insert(agent_id.clone(), ActorLane::Compatibility);
        actor.active_turn.store(true, Ordering::Release);
        self.active_turns += 1;
        self.compatibility_turns.insert(
            turn_id,
            CompatibilityTurnRecord {
                agent_id,
                turn_context,
            },
        );
        Ok(turn_id)
    }

    /// Poll every actor without blocking for provider work.
    pub fn poll_completed(
        &mut self,
    ) -> Result<Vec<AsyncAgentTurnOutcome<A, Q>>, AsyncAgentRunnerError> {
        let previous_len = self.completed.len();
        self.pump_completed()?;
        Ok(self.completed.iter().skip(previous_len).cloned().collect())
    }

    /// Polls the same actor mailboxes as `poll_completed`, returning only
    /// reduced compatibility outcomes. Production outcomes remain in their
    /// own queue and cannot be promoted by this API.
    pub fn poll_compatibility_completed(
        &mut self,
    ) -> Result<Vec<CompatibilityTurnOutcome<O>>, AsyncAgentRunnerError> {
        let previous_len = self.compatibility_completed.len();
        self.pump_completed()?;
        Ok(self
            .compatibility_completed
            .iter()
            .skip(previous_len)
            .cloned()
            .collect())
    }

    fn pump_completed(&mut self) -> Result<(), AsyncAgentRunnerError> {
        let mut ready = Vec::new();
        for (agent_id, actor) in &mut self.actors {
            if let Some(completion) = actor.try_completion(agent_id)? {
                actor.active_turn.store(false, Ordering::Release);
                ready.push((agent_id.clone(), completion));
            }
        }
        for (_agent_id, completion) in ready {
            self.active_turns = self.active_turns.saturating_sub(1);
            match completion.result {
                ActorCompletionResult::Production(result) => {
                    let turn = self.turns.get_mut(&completion.turn_id).ok_or_else(|| {
                        AsyncAgentRunnerError::Cognition(CognitionError::new(
                            "unknown_actor_completion",
                            "actor completed an unknown process-local turn",
                        ))
                    })?;
                    let (lifecycle, feedback, world_effect, response_context, provider_error) =
                        match result {
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
                            Err(error) if error.code == "actor_panicked" => (
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
                    self.completed.push_back(outcome);
                }
                ActorCompletionResult::Compatibility(result) => {
                    let (record_agent_id, turn_context) = self
                        .compatibility_turns
                        .get(&completion.turn_id)
                        .map(|record| (record.agent_id.clone(), record.turn_context.clone()))
                        .ok_or_else(|| {
                            AsyncAgentRunnerError::Cognition(CognitionError::new(
                                "unknown_compatibility_completion",
                                "actor completed an unknown compatibility turn",
                            ))
                        })?;
                    let outcome = CompatibilityTurnOutcome {
                        turn_id: completion.turn_id,
                        agent_id: record_agent_id.clone(),
                        lifecycle: if result.is_ok() {
                            AsyncTurnLifecycle::Completed
                        } else {
                            AsyncTurnLifecycle::Failed
                        },
                        turn_context: turn_context.clone(),
                        result,
                    };
                    if turn_context.is_some() {
                        self.awaiting_compatibility
                            .insert(record_agent_id.clone(), completion.turn_id);
                    } else {
                        self.compatibility_turns.remove(&completion.turn_id);
                        self.actor_lanes.remove(&record_agent_id);
                    }
                    self.compatibility_completed.push_back(outcome);
                }
            }
        }
        Ok(())
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
            ActorCommand::DecideProduction {
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
}
