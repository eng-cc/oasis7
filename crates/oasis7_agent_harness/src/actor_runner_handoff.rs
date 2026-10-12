use super::*;

impl<A, Q, I, O, F> AsyncAgentRunner<A, Q, I, O, F>
where
    A: Serialize + Send + 'static,
    Q: Serialize + Send + 'static,
    I: Send + 'static,
    O: Clone + Send + 'static,
    F: Send + 'static,
{
    pub fn begin_turn_handoff(
        &self,
        turn_id: AsyncTurnId,
    ) -> Result<TurnHandoff, AsyncAgentRunnerError> {
        let (lane, agent_id, session_id, turn_identity, request_id, request_digest) =
            if let Some(turn) = self
                .turns
                .get(&turn_id)
                .filter(|turn| turn.awaiting_runtime)
            {
                if self.awaiting_runtime.get(&turn.agent_id) != Some(&turn_id) {
                    return Err(handoff_stale());
                }
                (
                    ActorLane::Production,
                    turn.agent_id.clone(),
                    turn.request.agent_session_id.clone(),
                    turn.request.agent_turn_id.clone(),
                    turn.request.decision_request_id.clone(),
                    turn.request.request_digest.clone(),
                )
            } else if let Some(turn) = self.compatibility_turns.get(&turn_id) {
                let context = turn.turn_context.as_ref().ok_or_else(handoff_stale)?;
                if self.awaiting_compatibility.get(&turn.agent_id) != Some(&turn_id) {
                    return Err(handoff_stale());
                }
                (
                    ActorLane::Compatibility,
                    turn.agent_id.clone(),
                    context.agent_session_id.clone(),
                    context.agent_turn_id.clone(),
                    context.decision_request_id.clone(),
                    context.request_digest.clone(),
                )
            } else {
                return Err(handoff_stale());
            };
        if self.provider_is_still_in_flight(&agent_id)
            || self.actor_lanes.get(&agent_id) != Some(&lane)
        {
            return Err(handoff_stale());
        }
        Ok(TurnHandoff {
            runner_identity: Arc::clone(&self.runner_identity),
            turn_id,
            lane,
            agent_id,
            session_id,
            turn_identity,
            request_id,
            request_digest,
        })
    }

    /// Temporarily release one exact awaiting turn while its host persists a
    /// terminal transition. The returned capability can commit that release
    /// once, or restore only this predecessor if the host write fails. It is
    /// bound to this runner and the complete request identity.
    pub fn prepare_turn_release(
        &mut self,
        turn_id: AsyncTurnId,
    ) -> Result<TurnHandoff, AsyncAgentRunnerError> {
        let handoff = self.begin_turn_handoff(turn_id)?;
        let predecessor = self.handoff_predecessor(&handoff)?;
        match handoff.lane {
            ActorLane::Production => {
                self.awaiting_runtime.remove(&handoff.agent_id);
            }
            ActorLane::Compatibility => {
                self.awaiting_compatibility.remove(&handoff.agent_id);
            }
        }
        self.actor_lanes.remove(&handoff.agent_id);
        self.cognition.clear_agent(&handoff.agent_id);
        // Keep the turn record until commit. This is the version check used by
        // both resolution paths and cannot overwrite a later turn.
        let still_same = match &predecessor {
            HandoffPredecessor::Production(request) => self
                .turns
                .get(&turn_id)
                .is_some_and(|turn| turn.awaiting_runtime && &turn.request == request.as_ref()),
            HandoffPredecessor::Compatibility(context) => self
                .compatibility_turns
                .get(&turn_id)
                .and_then(|turn| turn.turn_context.as_ref())
                .is_some_and(|current| current == context.as_ref()),
        };
        if !still_same {
            self.restore_handoff_predecessor(&handoff, predecessor)?;
            return Err(handoff_stale());
        }
        Ok(handoff)
    }

    /// Commit a prepared terminal release after the host persistence callback
    /// succeeds. A stale capability cannot retire a successor or another
    /// request.
    pub fn commit_turn_release(
        &mut self,
        handoff: TurnHandoff,
    ) -> Result<(), AsyncAgentRunnerError> {
        self.validate_turn_release(&handoff)?;
        match handoff.lane {
            ActorLane::Production => {
                self.turns.remove(&handoff.turn_id);
            }
            ActorLane::Compatibility => {
                self.compatibility_turns.remove(&handoff.turn_id);
            }
        }
        Ok(())
    }

    /// Restore a prepared release after a failed host write. The predecessor
    /// replay fence must still match, and no successor may have started.
    pub fn rollback_turn_release(
        &mut self,
        handoff: TurnHandoff,
    ) -> Result<(), AsyncAgentRunnerError> {
        self.validate_turn_release(&handoff)?;
        let predecessor = match handoff.lane {
            ActorLane::Production => self
                .turns
                .get(&handoff.turn_id)
                .filter(|turn| turn.awaiting_runtime)
                .map(|turn| HandoffPredecessor::Production(Box::new(turn.request.clone()))),
            ActorLane::Compatibility => self
                .compatibility_turns
                .get(&handoff.turn_id)
                .and_then(|turn| turn.turn_context.clone())
                .map(Box::new)
                .map(HandoffPredecessor::Compatibility),
        }
        .ok_or_else(handoff_stale)?;
        self.restore_handoff_predecessor(&handoff, predecessor)
    }

    fn validate_turn_release(&self, handoff: &TurnHandoff) -> Result<(), AsyncAgentRunnerError> {
        if !Arc::ptr_eq(&self.runner_identity, &handoff.runner_identity) {
            return Err(CognitionError::new(
                "handoff_runner_mismatch",
                "release capability belongs to another actor runner",
            )
            .into());
        }
        let released = match handoff.lane {
            ActorLane::Production => self.turns.get(&handoff.turn_id).is_some_and(|turn| {
                turn.awaiting_runtime
                    && turn.agent_id == handoff.agent_id
                    && turn.request.agent_session_id == handoff.session_id
                    && turn.request.agent_turn_id == handoff.turn_identity
                    && turn.request.decision_request_id == handoff.request_id
                    && turn.request.request_digest == handoff.request_digest
                    && !self.awaiting_runtime.contains_key(&handoff.agent_id)
                    && !self.actor_lanes.contains_key(&handoff.agent_id)
            }),
            ActorLane::Compatibility => self
                .compatibility_turns
                .get(&handoff.turn_id)
                .and_then(|turn| {
                    let context = turn.turn_context.as_ref()?;
                    Some(
                        turn.agent_id == handoff.agent_id
                            && context.agent_session_id == handoff.session_id
                            && context.agent_turn_id == handoff.turn_identity
                            && context.decision_request_id == handoff.request_id
                            && context.request_digest == handoff.request_digest
                            && !self.awaiting_compatibility.contains_key(&handoff.agent_id)
                            && !self.actor_lanes.contains_key(&handoff.agent_id),
                    )
                })
                .unwrap_or(false),
        };
        if !released
            || self.provider_is_still_in_flight(&handoff.agent_id)
            || self.agent_has_new_turn(&handoff.agent_id)
        {
            return Err(handoff_stale());
        }
        Ok(())
    }

    /// Release the exact predecessor only for the duration of one successor
    /// dispatch. A failed mailbox dispatch restores its request-scoped busy
    /// marker; it never restores a snapshot over a later successful turn.
    pub fn dispatch_turn_handoff(
        &mut self,
        handoff: TurnHandoff,
        next: TurnHandoffDispatch<I>,
    ) -> Result<AsyncTurnId, AsyncAgentRunnerError> {
        if !Arc::ptr_eq(&self.runner_identity, &handoff.runner_identity) {
            return Err(CognitionError::new(
                "handoff_runner_mismatch",
                "handoff capability belongs to another actor runner",
            )
            .into());
        }
        let predecessor = self.handoff_predecessor(&handoff)?;
        let next_agent = match &next {
            TurnHandoffDispatch::Production(request) => request.agent_subject.as_str(),
            TurnHandoffDispatch::Compatibility(request) => request.agent_subject.as_str(),
        };
        if next_agent != handoff.agent_id {
            return Err(CognitionError::new(
                "handoff_identity_mismatch",
                "successor dispatch must remain on the predecessor Agent",
            )
            .into());
        }

        match handoff.lane {
            ActorLane::Production => {
                self.awaiting_runtime.remove(&handoff.agent_id);
            }
            ActorLane::Compatibility => {
                self.awaiting_compatibility.remove(&handoff.agent_id);
            }
        }
        self.actor_lanes.remove(&handoff.agent_id);
        self.cognition.clear_agent(&handoff.agent_id);

        let dispatched = match next {
            TurnHandoffDispatch::Production(request) => self.start_turn(request),
            TurnHandoffDispatch::Compatibility(request) => self.start_compatibility_turn(request),
        };
        match dispatched {
            Ok(next_turn_id) => {
                match handoff.lane {
                    ActorLane::Production => {
                        self.turns.remove(&handoff.turn_id);
                    }
                    ActorLane::Compatibility => {
                        self.compatibility_turns.remove(&handoff.turn_id);
                    }
                }
                Ok(next_turn_id)
            }
            Err(dispatch_error) => {
                if self.agent_has_new_turn(&handoff.agent_id) {
                    return Err(CognitionError::new(
                        "handoff_rollback_conflict",
                        "successor state changed before the failed dispatch could roll back",
                    )
                    .into());
                }
                self.restore_handoff_predecessor(&handoff, predecessor)?;
                Err(dispatch_error)
            }
        }
    }

    fn handoff_predecessor(
        &self,
        handoff: &TurnHandoff,
    ) -> Result<HandoffPredecessor, AsyncAgentRunnerError> {
        let current = match handoff.lane {
            ActorLane::Production => self.turns.get(&handoff.turn_id).and_then(|turn| {
                (turn.awaiting_runtime
                    && turn.agent_id == handoff.agent_id
                    && turn.request.agent_session_id == handoff.session_id
                    && turn.request.agent_turn_id == handoff.turn_identity
                    && turn.request.decision_request_id == handoff.request_id
                    && turn.request.request_digest == handoff.request_digest
                    && self.awaiting_runtime.get(&handoff.agent_id) == Some(&handoff.turn_id)
                    && self.actor_lanes.get(&handoff.agent_id) == Some(&ActorLane::Production))
                .then(|| HandoffPredecessor::Production(Box::new(turn.request.clone())))
            }),
            ActorLane::Compatibility => {
                self.compatibility_turns
                    .get(&handoff.turn_id)
                    .and_then(|turn| {
                        let context = turn.turn_context.as_ref()?;
                        (turn.agent_id == handoff.agent_id
                            && context.agent_session_id == handoff.session_id
                            && context.agent_turn_id == handoff.turn_identity
                            && context.decision_request_id == handoff.request_id
                            && context.request_digest == handoff.request_digest
                            && self.awaiting_compatibility.get(&handoff.agent_id)
                                == Some(&handoff.turn_id)
                            && self.actor_lanes.get(&handoff.agent_id)
                                == Some(&ActorLane::Compatibility))
                        .then(|| HandoffPredecessor::Compatibility(Box::new(context.clone())))
                    })
            }
        };
        if self.provider_is_still_in_flight(&handoff.agent_id) {
            return Err(handoff_stale());
        }
        current.ok_or_else(handoff_stale)
    }

    fn restore_handoff_predecessor(
        &mut self,
        handoff: &TurnHandoff,
        predecessor: HandoffPredecessor,
    ) -> Result<(), AsyncAgentRunnerError> {
        match predecessor {
            HandoffPredecessor::Production(request) => {
                self.cognition.restore_handoff_request(&request)?;
                self.awaiting_runtime
                    .insert(handoff.agent_id.clone(), handoff.turn_id);
                self.actor_lanes
                    .insert(handoff.agent_id.clone(), ActorLane::Production);
            }
            HandoffPredecessor::Compatibility(context) => {
                self.cognition.restore_handoff_turn(&context)?;
                self.awaiting_compatibility
                    .insert(handoff.agent_id.clone(), handoff.turn_id);
                self.actor_lanes
                    .insert(handoff.agent_id.clone(), ActorLane::Compatibility);
            }
        }
        Ok(())
    }

    fn agent_has_new_turn(&self, agent_id: &str) -> bool {
        self.awaiting_runtime.contains_key(agent_id)
            || self.awaiting_compatibility.contains_key(agent_id)
            || self
                .actors
                .get(agent_id)
                .is_some_and(|actor| actor.active_turn.load(Ordering::Acquire))
            || self.cognition.has_active_request(agent_id)
    }
}
