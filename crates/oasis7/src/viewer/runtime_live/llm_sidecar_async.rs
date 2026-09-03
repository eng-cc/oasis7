use super::*;

#[cfg(not(target_arch = "wasm32"))]
use crate::simulator::{AsyncAgentTurnOutcome, AsyncTurnLifecycle};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(in crate::viewer::runtime_live) struct RuntimeLlmDecision {
    pub(in crate::viewer::runtime_live) agent_id: String,
    pub(in crate::viewer::runtime_live) decision: AgentDecision,
    pub(in crate::viewer::runtime_live) decision_trace: Option<AgentDecisionTrace>,
    pub(in crate::viewer::runtime_live) cognition: Option<RuntimeProviderActionContext>,
    /// Provider memory intents remain attached to the response until the
    /// Runtime receipt/readback gate accepts them.
    #[serde(default)]
    pub(in crate::viewer::runtime_live) memory_write_intents:
        Vec<crate::simulator::MemoryWriteIntent>,
}

pub(super) fn provider_trace_retryable(trace: &AgentDecisionTrace) -> bool {
    trace
        .llm_output
        .as_ref()
        .and_then(|output| serde_json::from_str::<serde_json::Value>(output).ok())
        .and_then(|output| {
            output
                .get("provider_error")
                .or_else(|| output.get("trace_payload")?.get("provider_error"))
                .and_then(|error| error.get("retryable"))
                .and_then(serde_json::Value::as_bool)
        })
        .unwrap_or(false)
}

/// Context capture identifies the logical request, not one transport attempt.
/// Normalize the attempt before hashing so a retry can append only a new
/// `RequestDispatched` event while reusing the already captured context.
pub(crate) fn runtime_provider_context_digest(
    request: &crate::simulator::ContinuousAgentRequestContextV1,
) -> String {
    let mut logical_request = request.clone();
    logical_request.transport_attempt = 1;
    crate::simulator::h_v1("oasis7.cognition.context.v1", &logical_request).to_string()
}

impl RuntimeLlmDecision {
    pub(super) fn from_error(world: &RuntimeWorld, message: String) -> Self {
        let agent_id = world
            .state()
            .agents
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| "runtime-agent-0".to_string());
        Self::from_agent_error(world, agent_id, message)
    }

    fn from_agent_error(world: &RuntimeWorld, agent_id: String, message: String) -> Self {
        let trace = AgentDecisionTrace {
            agent_id: agent_id.clone(),
            time: world.state().time,
            decision: AgentDecision::Wait,
            llm_input: None,
            llm_output: None,
            llm_error: Some(message),
            parse_error: None,
            llm_diagnostics: None,
            llm_effect_intents: Vec::new(),
            llm_effect_receipts: Vec::new(),
            llm_step_trace: Vec::new(),
            llm_prompt_section_trace: Vec::new(),
            llm_chat_messages: Vec::new(),
        };
        Self {
            agent_id,
            decision: AgentDecision::Wait,
            decision_trace: Some(trace),
            cognition: None,
            memory_write_intents: Vec::new(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl RuntimeLlmSidecar {
    pub(in crate::viewer::runtime_live) fn next_async_provider_decision(
        &mut self,
        world: &mut RuntimeWorld,
        kernel: &mut WorldKernel,
        world_id: &str,
    ) -> Option<RuntimeLlmDecision> {
        // Retry notifications that survived a previous chain-link tick or
        // process restart before admitting another provider turn.
        self.flush_pending_provider_world_events();
        let completed = {
            let Some(RuntimeDecisionRunner::ProviderBacked(runner)) = self.runner.as_mut() else {
                return Some(RuntimeLlmDecision::from_error(
                    world,
                    "provider runner not initialized".to_string(),
                ));
            };
            match runner.poll_completed() {
                Ok(outcomes) => outcomes,
                Err(error) => {
                    return Some(RuntimeLlmDecision::from_error(
                        world,
                        format!("async provider runner poll failed: {error}"),
                    ));
                }
            }
        };
        if let Some(RuntimeDecisionRunner::ProviderBacked(runner)) = self.runner.as_mut() {
            // `poll_completed` also records outcomes for compatibility users;
            // this production adapter consumes them into its own decision
            // queue so an outcome cannot accumulate across world ticks.
            let _ = runner.take_completed();
        }
        for outcome in completed {
            if self
                .provider_terminal_states
                .get(outcome.agent_id.as_str())
                .is_some_and(|terminal| {
                    outcome
                        .prepared_request_context
                        .as_ref()
                        .is_some_and(|request| {
                            request.agent_turn_id == terminal.agent_turn_id
                                && request.decision_request_id == terminal.decision_request_id
                        })
                })
            {
                self.record_late_provider_response(&outcome);
                tracing::warn!(
                    agent_id = outcome.agent_id,
                    "discarded late provider response after terminal feedback"
                );
                continue;
            }
            let decision = self.provider_decision_from_async_outcome(world, kernel, outcome);
            self.provider_completed_decisions.push_back(decision);
        }
        if !self.provider_completed_decisions.is_empty() {
            self.persist_provider_lineage_best_effort();
        }
        if let Some(decision) = self.provider_completed_decisions.pop_front() {
            self.provider_held_decisions
                .insert(decision.agent_id.clone(), decision.clone());
            self.persist_provider_lineage_best_effort();
            return Some(decision);
        }

        if let Err(message) = self.prepare_provider_request_contexts(world, kernel, world_id) {
            return Some(RuntimeLlmDecision::from_error(world, message));
        }
        let now = world.state().time;
        let candidates = self.provider_agent_ids.iter().cloned().collect::<Vec<_>>();
        for agent_id in candidates {
            if self.provider_recovery_pending.contains_key(&agent_id)
                || self.provider_active_turns.contains_key(&agent_id)
                || self
                    .provider_wait_until
                    .get(&agent_id)
                    .is_some_and(|wait_until| *wait_until > now)
            {
                continue;
            }
            let in_flight = matches!(
                self.runner.as_ref(),
                Some(RuntimeDecisionRunner::ProviderBacked(runner))
                    if runner.provider_is_still_in_flight(agent_id.as_str())
            );
            if in_flight {
                continue;
            }
            let Some(context) = self.provider_contexts.get(&agent_id).cloned() else {
                continue;
            };
            let observation = match kernel.observe(agent_id.as_str()) {
                Ok(observation) => observation,
                Err(error) => {
                    return Some(RuntimeLlmDecision::from_agent_error(
                        world,
                        agent_id,
                        format!("provider observation failed: {error:?}"),
                    ));
                }
            };
            if let Err(error) = runtime_provider_prefix(world, &context) {
                let _ = runtime_provider_failure(world, &context, "persistence_failure");
                return Some(RuntimeLlmDecision::from_agent_error(
                    world,
                    agent_id,
                    format!("Runtime cognition prefix rejected provider I/O: {error}"),
                ));
            }
            let start_result = match self.runner.as_mut() {
                Some(RuntimeDecisionRunner::ProviderBacked(runner)) => runner
                    .start_turn_with_request_context_and_observation(
                        agent_id.as_str(),
                        observation,
                        context.turn_context.clone(),
                        context.request_context.clone(),
                    ),
                _ => {
                    return Some(RuntimeLlmDecision::from_error(
                        world,
                        "provider runner disappeared while starting an async turn".to_string(),
                    ));
                }
            };
            if let Err(error) = start_result {
                let _ = runtime_provider_failure(world, &context, "provider_failure");
                return Some(RuntimeLlmDecision::from_agent_error(
                    world,
                    agent_id,
                    format!("async provider turn start failed: {error}"),
                ));
            }
            // The actor now owns the provider call. Returning without a
            // decision lets the caller advance the Runtime world immediately;
            // a later poll will surface the completed outcome.
            return None;
        }
        None
    }

    fn provider_decision_from_async_outcome(
        &mut self,
        world: &mut RuntimeWorld,
        kernel: &mut WorldKernel,
        outcome: AsyncAgentTurnOutcome,
    ) -> RuntimeLlmDecision {
        let agent_id = outcome.agent_id.clone();
        let context = outcome
            .prepared_context
            .clone()
            .zip(outcome.prepared_request_context.clone())
            .map(
                |(turn_context, request_context)| cognition_context::ProviderContextState {
                    turn_context,
                    request_context,
                },
            )
            .or_else(|| self.provider_contexts.get(&agent_id).cloned());
        if let Some(context) = context.as_ref() {
            self.provider_contexts
                .insert(agent_id.clone(), context.clone());
        }
        let cognition = context
            .clone()
            .zip(outcome.prepared_response_context.clone())
            .map(|(request, response)| RuntimeProviderActionContext {
                request,
                response,
                memory_write_intents: outcome.memory_write_intents.clone(),
            });
        let decision = outcome.decision.unwrap_or(AgentDecision::Wait);
        let decision_trace = outcome.decision_trace.or_else(|| {
            (outcome.lifecycle == AsyncTurnLifecycle::Failed).then(|| AgentDecisionTrace {
                agent_id: agent_id.clone(),
                time: world.state().time,
                decision: AgentDecision::Wait,
                llm_input: None,
                llm_output: None,
                llm_error: Some(format!(
                    "async provider actor failed: {:?}",
                    outcome.feedback
                )),
                parse_error: None,
                llm_diagnostics: None,
                llm_effect_intents: Vec::new(),
                llm_effect_receipts: Vec::new(),
                llm_step_trace: Vec::new(),
                llm_prompt_section_trace: Vec::new(),
                llm_chat_messages: Vec::new(),
            })
        });
        if let Some(cognition) = cognition.as_ref() {
            if decision_trace
                .as_ref()
                .is_none_or(|trace| trace.llm_error.is_none() && trace.parse_error.is_none())
            {
                self.provider_active_turns
                    .insert(agent_id.clone(), cognition.request.clone());
                match &decision {
                    AgentDecision::Wait => {
                        self.schedule_provider_wait(agent_id.as_str(), world.state().time, 1);
                    }
                    AgentDecision::WaitTicks(ticks) => {
                        self.schedule_provider_wait(agent_id.as_str(), world.state().time, *ticks);
                    }
                    AgentDecision::Act(_)
                    | AgentDecision::Query(_)
                    | AgentDecision::ModuleCommand { .. } => {}
                }
            }
        } else if decision_trace
            .as_ref()
            .is_some_and(provider_trace_retryable)
        {
            if let Some(context) = context {
                if context.request_context.transport_attempt < MAX_PROVIDER_TRANSPORT_ATTEMPTS {
                    let mut retry_context = context.request_context.clone();
                    retry_context.transport_attempt =
                        retry_context.transport_attempt.saturating_add(1);
                    if let Err(error) = runtime_provider_dispatch(world, &retry_context) {
                        tracing::warn!(
                            agent_id,
                            error,
                            "Runtime cognition retry dispatch rejected"
                        );
                        let _ = runtime_provider_failure(world, &context, "persistence_failure");
                        self.mark_provider_transport_exhausted(agent_id.clone());
                        self.persist_provider_lineage_best_effort();
                        return RuntimeLlmDecision {
                            agent_id,
                            decision,
                            decision_trace,
                            cognition,
                            memory_write_intents: outcome.memory_write_intents,
                        };
                    }
                    let retry_result = kernel
                        .observe(agent_id.as_str())
                        .map_err(|error| format!("provider retry observation failed: {error:?}"))
                        .and_then(|observation| match self.runner.as_mut() {
                            Some(RuntimeDecisionRunner::ProviderBacked(runner)) => runner
                                .retry_awaiting_turn_with_request_context_and_observation(
                                    agent_id.as_str(),
                                    observation,
                                    context.turn_context.clone(),
                                    context.request_context.clone(),
                                )
                                .map(|_| ())
                                .map_err(|error| error.to_string()),
                            _ => Err("provider runner disappeared while retrying an async turn"
                                .to_string()),
                        });
                    if let Err(error) = retry_result {
                        tracing::warn!(
                            agent_id,
                            error,
                            "async provider transport retry could not be dispatched"
                        );
                        self.mark_provider_transport_exhausted(agent_id.clone());
                    }
                } else {
                    // Keep the last response correlated long enough for the
                    // Viewer control plane to emit one typed terminal
                    // `failed_provider` disposition. Without this marker the
                    // context remains reusable and every later world tick can
                    // redispatch the same exhausted transport attempt.
                    self.mark_provider_transport_exhausted(agent_id.clone());
                    let _ = runtime_provider_failure(world, &context, "failed_provider");
                }
            }
        } else if let Some(context) = context {
            // A provider/actor failure that is not retryable must be closed
            // durably before the control plane observes the typed error.
            let _ = runtime_provider_failure(world, &context, "provider_failure");
        }
        self.persist_provider_lineage_best_effort();
        RuntimeLlmDecision {
            agent_id,
            decision,
            decision_trace,
            cognition,
            memory_write_intents: outcome.memory_write_intents,
        }
    }
}

fn cognition_event_matches(
    world: &RuntimeWorld,
    kind: &str,
    request: &crate::simulator::ContinuousAgentRequestContextV1,
) -> bool {
    world
        .cognition()
        .get("cognition_journal")
        .and_then(|journal| journal.get("events"))
        .and_then(serde_json::Value::as_array)
        .is_some_and(|events| {
            events.iter().any(|event| {
                event.get("event_kind").and_then(serde_json::Value::as_str) == Some(kind)
                    && event.get("agent_id").and_then(serde_json::Value::as_str)
                        == Some(request.agent_subject.as_str())
                    && event
                        .get("agent_session_id")
                        .and_then(serde_json::Value::as_str)
                        == Some(request.agent_session_id.as_str())
                    && event
                        .get("agent_turn_id")
                        .and_then(serde_json::Value::as_str)
                        == Some(request.agent_turn_id.as_str())
                    && event
                        .get("decision_request_id")
                        .and_then(serde_json::Value::as_str)
                        == Some(request.decision_request_id.as_str())
                    && event
                        .get("request_digest")
                        .and_then(serde_json::Value::as_str)
                        == Some(request.request_digest.to_string().as_str())
            })
        })
}

pub(super) fn runtime_provider_prefix(
    world: &mut RuntimeWorld,
    context: &cognition_context::ProviderContextState,
) -> Result<(), String> {
    let request = &context.request_context;
    let request_digest = request.request_digest.to_string();
    if !cognition_event_matches(world, "TurnStarted", request) {
        world
            .start_cognition_turn(
                request.agent_subject.as_str(),
                request.agent_session_id.as_str(),
                request.agent_turn_id.as_str(),
                request.decision_request_id.as_str(),
                request_digest.as_str(),
            )
            .map_err(|error| format!("TurnStarted failed: {error:?}"))?;
    }
    if !cognition_event_matches(world, "ContextCaptured", request) {
        let context_digest = runtime_provider_context_digest(request);
        world
            .capture_cognition_context(
                request.agent_subject.as_str(),
                request.agent_session_id.as_str(),
                request.agent_turn_id.as_str(),
                request.decision_request_id.as_str(),
                request_digest.as_str(),
                context_digest.as_str(),
            )
            .map_err(|error| format!("ContextCaptured failed: {error:?}"))?;
    }
    runtime_provider_dispatch(world, request)
}

pub(super) fn runtime_provider_dispatch(
    world: &mut RuntimeWorld,
    request: &crate::simulator::ContinuousAgentRequestContextV1,
) -> Result<(), String> {
    world
        .dispatch_cognition_request(
            request.agent_subject.as_str(),
            request.agent_session_id.as_str(),
            request.agent_turn_id.as_str(),
            request.decision_request_id.as_str(),
            request.request_digest.to_string().as_str(),
            request.provider_invocation_key().to_string().as_str(),
            request.retry_seq,
            request.transport_attempt,
        )
        .map_err(|error| format!("RequestDispatched failed: {error:?}"))
}

pub(super) fn runtime_provider_failure(
    world: &mut RuntimeWorld,
    context: &cognition_context::ProviderContextState,
    reason: &str,
) -> Result<(), String> {
    let request = &context.request_context;
    world
        .fail_cognition_turn(
            request.agent_subject.as_str(),
            request.agent_session_id.as_str(),
            request.agent_turn_id.as_str(),
            request.decision_request_id.as_str(),
            request.request_digest.to_string().as_str(),
            reason,
            request.retry_seq,
            request.transport_attempt,
        )
        .map_err(|error| format!("CognitionTurnFailed failed: {error:?}"))
}
