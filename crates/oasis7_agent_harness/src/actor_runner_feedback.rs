use super::*;

impl<A, Q, I, O, F> AsyncAgentRunner<A, Q, I, O, F>
where
    A: Serialize + Send + 'static,
    Q: Serialize + Send + 'static,
    I: Send + 'static,
    O: Clone + Send + 'static,
    F: Send + 'static,
{
    pub fn notify_runtime_feedback(
        &self,
        agent_id: &str,
        feedback: FeedbackEnvelopeV1,
    ) -> Result<(), AsyncAgentRunnerError> {
        let actor = self
            .actors
            .get(agent_id)
            .ok_or_else(|| AsyncAgentRunnerError::AgentNotRegistered(agent_id.to_string()))?;
        if !actor.supports_production
            || self.actor_lanes.get(agent_id) == Some(&ActorLane::Compatibility)
        {
            return Err(AsyncAgentRunnerError::Cognition(CognitionError::new(
                "feedback_lane_mismatch",
                "production Runtime feedback cannot be sent to a compatibility turn",
            )));
        }
        actor.try_send(
            ActorCommand::ProductionFeedback(Box::new(feedback)),
            agent_id,
        )
    }

    /// Deliver a host callback to the exact awaiting compatibility turn. The
    /// shared actor mailbox preserves ordering with provider work, while the
    /// lane check prevents production feedback from being routed through this
    /// reduced callback API.
    pub fn notify_compatibility_feedback(
        &self,
        turn_id: AsyncTurnId,
        feedback: F,
    ) -> Result<(), AsyncAgentRunnerError> {
        let record = self.compatibility_turns.get(&turn_id).ok_or_else(|| {
            AsyncAgentRunnerError::Cognition(CognitionError::new(
                "feedback_lane_mismatch",
                "turn is not an awaiting compatibility turn",
            ))
        })?;
        if self.awaiting_compatibility.get(&record.agent_id) != Some(&turn_id) {
            return Err(AsyncAgentRunnerError::Cognition(CognitionError::new(
                "feedback_lane_mismatch",
                "compatibility feedback does not match the active reduced turn",
            )));
        }
        let actor = self
            .actors
            .get(&record.agent_id)
            .ok_or_else(|| AsyncAgentRunnerError::AgentNotRegistered(record.agent_id.clone()))?;
        if !actor.supports_compatibility {
            return Err(AsyncAgentRunnerError::Cognition(CognitionError::new(
                "feedback_lane_mismatch",
                "actor has no compatibility callback lane",
            )));
        }
        actor.try_send(
            ActorCommand::CompatibilityCallback(feedback),
            &record.agent_id,
        )
    }

    pub fn notify_compatibility_callback(
        &self,
        agent_id: &str,
        callback: F,
    ) -> Result<(), AsyncAgentRunnerError> {
        let actor = self
            .actors
            .get(agent_id)
            .ok_or_else(|| AsyncAgentRunnerError::AgentNotRegistered(agent_id.to_string()))?;
        if !actor.supports_compatibility
            || self.actor_lanes.get(agent_id) == Some(&ActorLane::Production)
        {
            return Err(AsyncAgentRunnerError::Cognition(CognitionError::new(
                "feedback_lane_mismatch",
                "compatibility callback cannot be sent to a production turn",
            )));
        }
        actor.try_send(ActorCommand::CompatibilityCallback(callback), agent_id)
    }

    pub fn notify_production_callback(
        &self,
        agent_id: &str,
        callback: F,
    ) -> Result<(), AsyncAgentRunnerError> {
        let actor = self
            .actors
            .get(agent_id)
            .ok_or_else(|| AsyncAgentRunnerError::AgentNotRegistered(agent_id.to_string()))?;
        if !actor.supports_production
            || self.actor_lanes.get(agent_id) == Some(&ActorLane::Compatibility)
        {
            return Err(AsyncAgentRunnerError::Cognition(CognitionError::new(
                "feedback_lane_mismatch",
                "production callback cannot be sent to a compatibility turn",
            )));
        }
        actor.try_send(ActorCommand::ProductionCallback(callback), agent_id)
    }

    /// Route a typed behavior callback to the actor's currently admitted lane.
    /// With no active turn, compatibility is preferred when present because
    /// that preserves legacy observer/callback behavior without granting any
    /// production request identity.
    pub fn notify_behavior_callback(
        &self,
        agent_id: &str,
        callback: F,
    ) -> Result<(), AsyncAgentRunnerError> {
        let actor = self
            .actors
            .get(agent_id)
            .ok_or_else(|| AsyncAgentRunnerError::AgentNotRegistered(agent_id.to_string()))?;
        match self.actor_lanes.get(agent_id) {
            Some(ActorLane::Compatibility) if actor.supports_compatibility => {
                actor.try_send(ActorCommand::CompatibilityCallback(callback), agent_id)
            }
            Some(ActorLane::Production) if actor.supports_production => {
                actor.try_send(ActorCommand::ProductionCallback(callback), agent_id)
            }
            Some(_) => Err(AsyncAgentRunnerError::Cognition(CognitionError::new(
                "feedback_lane_mismatch",
                "registered provider cannot receive callbacks for the active lane",
            ))),
            None if actor.supports_compatibility => {
                actor.try_send(ActorCommand::CompatibilityCallback(callback), agent_id)
            }
            None if actor.supports_production => {
                actor.try_send(ActorCommand::ProductionCallback(callback), agent_id)
            }
            None => Err(AsyncAgentRunnerError::Cognition(CognitionError::new(
                "feedback_lane_mismatch",
                "registered provider has no behavior callback lane",
            ))),
        }
    }

    /// Record a Runtime feedback envelope for compatibility-turn replay
    /// fencing after Oasis has performed its own identity/receipt checks.
    /// This updates only local correlation metadata; it does not read or
    /// mutate Runtime state and does not authorize committed effects.
    pub fn accept_compatibility_feedback(
        &mut self,
        turn_id: AsyncTurnId,
        feedback: FeedbackEnvelopeV1,
    ) -> Result<(), AsyncAgentRunnerError> {
        let record = self.compatibility_turns.get(&turn_id).ok_or_else(|| {
            AsyncAgentRunnerError::Cognition(CognitionError::new(
                "feedback_lane_mismatch",
                "turn is not an awaiting compatibility turn",
            ))
        })?;
        let context = record.turn_context.as_ref().ok_or_else(|| {
            AsyncAgentRunnerError::Cognition(CognitionError::new(
                "feedback_lane_mismatch",
                "compatibility feedback requires a reduced turn identity",
            ))
        })?;
        if self.awaiting_compatibility.get(&record.agent_id) != Some(&turn_id)
            || feedback.agent_subject != record.agent_id
            || feedback.agent_session_id != context.agent_session_id
            || feedback.agent_turn_id != context.agent_turn_id
            || feedback.decision_request_id != context.decision_request_id
            || feedback.request_digest != context.request_digest
        {
            return Err(AsyncAgentRunnerError::Cognition(CognitionError::new(
                "feedback_correlation_mismatch",
                "feedback does not match an awaiting compatibility turn",
            )));
        }
        feedback.validate()?;
        self.cognition
            .accept_feedback(feedback)
            .map_err(AsyncAgentRunnerError::Cognition)
    }

    /// Read-only idempotency query over the recovered or locally accepted
    /// feedback ledger. Runtime remains the source of truth.
    pub fn contains_feedback(&self, agent: &str, session: &str, feedback_id: &str) -> bool {
        self.cognition
            .contains_feedback(agent, session, feedback_id)
    }

    /// Clear only an idle actor's local feedback/replay partition after the
    /// host has terminally expired or handed off its Runtime turn. This does
    /// not restore history or create production authority.
    pub fn clear_feedback_agent(&mut self, agent: &str) -> Result<(), AsyncAgentRunnerError> {
        if self.provider_is_still_in_flight(agent)
            || self.awaiting_runtime.contains_key(agent)
            || self.awaiting_compatibility.contains_key(agent)
        {
            return Err(AsyncAgentRunnerError::AgentBusy(agent.to_string()));
        }
        self.cognition.clear_agent(agent);
        Ok(())
    }

    #[doc(hidden)]
    pub fn cognition_state_digest_for_test(&self) -> String {
        let mut hash = blake3::Hasher::new();
        hash.update(b"oasis7.agent-harness.cognition-test-state.v1\0");
        hash.update(format!("{:?}", self.cognition).as_bytes());
        hash.finalize().to_hex().to_string()
    }

    /// Release a reduced compatibility turn only after the Oasis host has
    /// completed its own Runtime/continuation handling. This carries no
    /// receipt and cannot satisfy production feedback validation.
    pub fn release_compatibility_turn(
        &mut self,
        turn_id: AsyncTurnId,
    ) -> Result<(), AsyncAgentRunnerError> {
        let record = self.compatibility_turns.remove(&turn_id).ok_or_else(|| {
            AsyncAgentRunnerError::Cognition(CognitionError::new(
                "unknown_compatibility_turn",
                "compatibility turn is not awaiting host completion",
            ))
        })?;
        if self.awaiting_compatibility.get(&record.agent_id) != Some(&turn_id) {
            self.compatibility_turns.insert(turn_id, record);
            return Err(AsyncAgentRunnerError::Cognition(CognitionError::new(
                "feedback_lane_mismatch",
                "compatibility completion does not match the active reduced turn",
            )));
        }
        self.awaiting_compatibility.remove(&record.agent_id);
        self.actor_lanes.remove(&record.agent_id);
        self.cognition.clear_agent(&record.agent_id);
        Ok(())
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

    /// Stage the exact awaiting Runtime feedback, persist the host's matching
    /// effects, and only then publish the provider callback and release the
    /// turn. A failed persistence callback restores only this feedback
    /// partition and leaves the exact turn awaiting Runtime for retry.
    pub fn accept_feedback_with_persistence<P>(
        &mut self,
        turn_id: AsyncTurnId,
        feedback: FeedbackEnvelopeV1,
        persist: P,
    ) -> Result<(), AsyncAgentRunnerError>
    where
        P: FnOnce() -> Result<(), String>,
    {
        self.accept_feedback_with_persistence_observed(turn_id, feedback, |_| persist())
    }

    /// Same one-turn transaction as `accept_feedback_with_persistence`, with
    /// a read-only fingerprint available only inside the synchronous commit
    /// callback. Oasis uses this for its existing failure-injection evidence;
    /// it exposes no cognition records or restore authority.
    #[doc(hidden)]
    pub fn accept_feedback_with_persistence_observed<P>(
        &mut self,
        turn_id: AsyncTurnId,
        feedback: FeedbackEnvelopeV1,
        persist: P,
    ) -> Result<(), AsyncAgentRunnerError>
    where
        P: FnOnce(&str) -> Result<(), String>,
    {
        if !self.turns.contains_key(&turn_id) {
            // A terminal exact replay remains idempotent after the turn has
            // been released. Cognition still checks the full feedback digest,
            // so a reused id with changed content is rejected.
            return self
                .cognition
                .accept_feedback(feedback)
                .map_err(AsyncAgentRunnerError::Cognition);
        }
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
        let gate = Arc::new(FeedbackDeliveryGate::new());
        let reservation = AbortOnDrop::new(Arc::clone(&gate));
        self.actors
            .get(&agent_id)
            .ok_or_else(|| AsyncAgentRunnerError::AgentNotRegistered(agent_id.clone()))?
            .try_send(ActorCommand::ProductionFeedbackTransaction(gate), &agent_id)?;
        self.cognition
            .accept_feedback_with_persistence_observed(feedback.clone(), |cognition| {
                let mut hash = blake3::Hasher::new();
                hash.update(b"oasis7.agent-harness.cognition-test-state.v1\0");
                hash.update(format!("{cognition:?}").as_bytes());
                persist(&hash.finalize().to_hex().to_string())
            })
            .map_err(AsyncAgentRunnerError::Cognition)?;
        reservation.commit(feedback.clone());
        if matches!(
            feedback.status.as_str(),
            "committed" | "rejected" | "failed"
        ) {
            self.release_turn(turn_id);
        }
        Ok(())
    }
}
