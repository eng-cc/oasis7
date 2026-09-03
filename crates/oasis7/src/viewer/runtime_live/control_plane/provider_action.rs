use super::super::decision_trace::is_trace_only_overflow;
use super::llm_sidecar::RuntimeProviderActionContext;
use super::*;
use crate::runtime::{
    CognitionCommitRejectReasonV1, RuntimeCognitionBaseBindingV1, RuntimeCognitionCommitRequestV1,
    RuntimeCognitionResponseArtifactV1, RuntimeFeedbackProjectionV1, RuntimeFeedbackRequestV1,
    classify_cognition_commit_error,
};
use crate::simulator::{Action as SimulatorAction, AgentDecision, h_v1};

impl ViewerRuntimeLiveServer {
    pub(in crate::viewer::runtime_live) fn enqueue_llm_action_from_sidecar(
        &mut self,
    ) -> Result<Option<AgentDecisionTrace>, AgentDecisionTrace> {
        self.drain_provider_feedback_outbox();
        if let Some(agent_id) = self.llm_sidecar.provider_stale_replan_exhausted_agent() {
            let mut trace = stale_replan_exhausted_trace(&self.world, agent_id.as_str());
            if let Err(error) = self.handoff_runtime_wake_for_agent(
                agent_id.as_str(),
                crate::runtime::ContinuationStatusV1::Rejected,
                "provider_stale_replan_exhausted",
            ) {
                trace.llm_error = Some(format!(
                    "{}; Runtime wake handoff failed: {error}",
                    trace.llm_error.take().unwrap_or_default()
                ));
            }
            return Err(trace);
        }
        if let Some(agent_id) = self.llm_sidecar.take_provider_transport_exhausted_agent() {
            return Err(self.finish_provider_transport_exhaustion(agent_id, None));
        }
        let decision = self.llm_sidecar.next_llm_decision(
            &mut self.world,
            &self.snapshot_config,
            self.config.world_id.as_str(),
        );
        // `next_llm_decision` may close a due provider Wait in the Runtime
        // outbox before returning a new decision. Drain that newly queued
        // terminal feedback in the same control pass so the provider sees
        // canonical `rejected/no_effect` rather than only the earlier
        // `pending` disposition.
        self.drain_provider_feedback_outbox();
        let Some(decision) = decision else {
            return Ok(None);
        };
        let decision_trace = decision.decision_trace.clone();
        if let Some(agent_id) = self.llm_sidecar.take_provider_transport_exhausted_agent() {
            return Err(self.finish_provider_transport_exhaustion(agent_id, decision_trace));
        }
        if let Some(trace) = decision_trace.as_ref() {
            if trace.llm_error.is_some() && !is_trace_only_overflow(trace) {
                if !decision_trace_provider_error_retryable(trace).unwrap_or(false) {
                    if let Some(feedback) = self.llm_sidecar.fail_provider_turn_with_feedback(
                        decision.agent_id.as_str(),
                        "failed",
                        trace
                            .llm_error
                            .clone()
                            .unwrap_or_else(|| "provider decision failed".to_string()),
                    ) {
                        self.deliver_provider_feedback_best_effort(feedback);
                    }
                    if let Err(error) = self.handoff_runtime_wake_for_agent(
                        decision.agent_id.as_str(),
                        crate::runtime::ContinuationStatusV1::Rejected,
                        "provider_decision_failed",
                    ) {
                        let mut terminal_trace = trace.clone();
                        terminal_trace.llm_error = Some(format!(
                            "{}; Runtime wake handoff failed: {error}",
                            terminal_trace.llm_error.take().unwrap_or_default()
                        ));
                        return Err(terminal_trace);
                    }
                }
                return Err(trace.clone());
            }
            if let Some(message) = trace.parse_error.as_ref() {
                if let Some(feedback) = self.llm_sidecar.fail_provider_turn_with_feedback(
                    decision.agent_id.as_str(),
                    "rejected",
                    message.clone(),
                ) {
                    self.deliver_provider_feedback_best_effort(feedback);
                }
                self.enqueue_virtual_event(WorldEventKind::ActionRejected {
                    reason: SimulatorRejectReason::RuleDenied {
                        notes: vec![format!("llm_failed: {}", message)],
                    },
                });
                if let Err(error) = self.handoff_runtime_wake_for_agent(
                    decision.agent_id.as_str(),
                    crate::runtime::ContinuationStatusV1::Rejected,
                    "provider_response_parse_failed",
                ) {
                    let mut terminal_trace = trace.clone();
                    terminal_trace.llm_error =
                        Some(format!("Runtime wake handoff failed: {error}"));
                    return Err(terminal_trace);
                }
                return Ok(decision_trace);
            }
        }

        match decision.decision.clone() {
            AgentDecision::Act(action) => match simulator_action_to_runtime(&action, &self.world) {
                Some(runtime_action) => {
                    if let Some(cognition) = decision.cognition {
                        match self.commit_provider_runtime_action(
                            &runtime_action,
                            &cognition,
                            action,
                        ) {
                            Ok(()) => {}
                            Err(error) => {
                                let stale_base = error.is_stale_base();
                                let reason = error.reason();
                                if stale_base {
                                    let request = &cognition.request.request_context;
                                    self.llm_sidecar.schedule_provider_stale_replan(
                                        request.agent_subject.as_str(),
                                        request.agent_turn_id.as_str(),
                                        request.decision_request_id.as_str(),
                                    );
                                }
                                if let Some(feedback) =
                                    self.llm_sidecar.fail_provider_turn_with_feedback(
                                        cognition.request.request_context.agent_subject.as_str(),
                                        "rejected",
                                        if stale_base {
                                            CognitionCommitRejectReasonV1::StaleBase
                                                .code()
                                                .to_string()
                                        } else {
                                            reason.clone()
                                        },
                                    )
                                {
                                    self.deliver_provider_feedback_best_effort(feedback);
                                }
                                self.enqueue_virtual_event(WorldEventKind::ActionRejected {
                                    reason: SimulatorRejectReason::RuleDenied {
                                        notes: vec![if stale_base {
                                            "stale_base".to_string()
                                        } else {
                                            reason
                                        }],
                                    },
                                });
                                let handoff_status = if stale_base {
                                    crate::runtime::ContinuationStatusV1::Invalidated
                                } else {
                                    crate::runtime::ContinuationStatusV1::Rejected
                                };
                                let handoff_reason = if stale_base {
                                    "provider_stale_base_replan"
                                } else {
                                    "provider_action_commit_rejected"
                                };
                                if let Err(error) = self.handoff_runtime_wake_for_agent(
                                    cognition.request.request_context.agent_subject.as_str(),
                                    handoff_status,
                                    handoff_reason,
                                ) {
                                    return Err(wake_handoff_error_trace(
                                        cognition.request.request_context.agent_subject.as_str(),
                                        self.world.state().time,
                                        error,
                                    ));
                                }
                            }
                        }
                    } else {
                        let action_id = self.world.submit_action(runtime_action);
                        self.llm_sidecar.track_action(
                            action_id,
                            decision.agent_id,
                            action.clone(),
                            None,
                        );
                    }
                }
                None => {
                    let reason = format!(
                        "runtime llm bridge cannot map action: {}",
                        simulator_action_label(&action)
                    );
                    self.llm_sidecar
                        .fail_provider_cognition_turn(
                            &mut self.world,
                            decision.agent_id.as_str(),
                            "cognition_failed",
                        )
                        .map_err(|error| {
                            wake_handoff_error_trace(
                                decision.agent_id.as_str(),
                                self.world.state().time,
                                error,
                            )
                        })?;
                    if let Some(feedback) = self.llm_sidecar.fail_provider_turn_with_feedback(
                        decision.agent_id.as_str(),
                        "rejected",
                        reason.clone(),
                    ) {
                        self.deliver_provider_feedback_best_effort(feedback);
                    }
                    self.enqueue_virtual_event(WorldEventKind::ActionRejected {
                        reason: SimulatorRejectReason::RuleDenied {
                            notes: vec![reason],
                        },
                    });
                    self.handoff_runtime_wake_for_agent(
                        decision.agent_id.as_str(),
                        crate::runtime::ContinuationStatusV1::Rejected,
                        "provider_action_unmappable",
                    )
                    .map_err(|error| AgentDecisionTrace {
                        agent_id: decision.agent_id.clone(),
                        time: self.world.state().time,
                        decision: AgentDecision::Wait,
                        llm_input: None,
                        llm_output: None,
                        llm_error: Some(error),
                        parse_error: None,
                        llm_diagnostics: None,
                        llm_effect_intents: Vec::new(),
                        llm_effect_receipts: Vec::new(),
                        llm_step_trace: Vec::new(),
                        llm_prompt_section_trace: Vec::new(),
                        llm_chat_messages: Vec::new(),
                    })?;
                }
            },
            AgentDecision::Wait | AgentDecision::WaitTicks(_) => {
                if let Some(cognition) = decision.cognition {
                    let ticks = match &decision.decision {
                        AgentDecision::Wait => 1,
                        AgentDecision::WaitTicks(ticks) => (*ticks).max(1),
                        _ => unreachable!("wait branch is exhaustive"),
                    };
                    self.llm_sidecar.schedule_provider_wait(
                        decision.agent_id.as_str(),
                        self.world.state().time,
                        ticks,
                    );
                    let feedback = self.llm_sidecar.provider_feedback(
                        &cognition,
                        None,
                        "pending",
                        None,
                        None,
                        Some("retry_scheduled".to_string()),
                    );
                    self.deliver_provider_feedback_best_effort(feedback);
                    if cognition.request.turn_context.continuation.is_some() {
                        // A Runtime-resumed request already consumed the
                        // selected wake and admitted its next continuation.
                        // Keep that continuation under the normal scheduler;
                        // local Viewer wait timers must not terminalize it.
                        self.llm_sidecar
                            .fail_provider_turn(decision.agent_id.as_str());
                    } else {
                        self.handoff_runtime_wake_for_agent(
                            decision.agent_id.as_str(),
                            crate::runtime::ContinuationStatusV1::Rejected,
                            "provider_wait_compatibility_terminal",
                        )
                        .map_err(|error| AgentDecisionTrace {
                            agent_id: decision.agent_id.clone(),
                            time: self.world.state().time,
                            decision: AgentDecision::Wait,
                            llm_input: None,
                            llm_output: None,
                            llm_error: Some(error),
                            parse_error: None,
                            llm_diagnostics: None,
                            llm_effect_intents: Vec::new(),
                            llm_effect_receipts: Vec::new(),
                            llm_step_trace: Vec::new(),
                            llm_prompt_section_trace: Vec::new(),
                            llm_chat_messages: Vec::new(),
                        })?;
                    }
                }
            }
            AgentDecision::Query(_) => {
                if let Some(cognition) = decision.cognition {
                    self.llm_sidecar
                        .fail_provider_cognition_turn(
                            &mut self.world,
                            decision.agent_id.as_str(),
                            "cognition_failed",
                        )
                        .map_err(|error| {
                            wake_handoff_error_trace(
                                decision.agent_id.as_str(),
                                self.world.state().time,
                                error,
                            )
                        })?;
                    let feedback = self.llm_sidecar.provider_feedback(
                        &cognition,
                        None,
                        "rejected",
                        None,
                        None,
                        Some(
                            "provider query is not executable in the runtime live action lane"
                                .to_string(),
                        ),
                    );
                    self.llm_sidecar
                        .fail_provider_turn(decision.agent_id.as_str());
                    self.deliver_provider_feedback_best_effort(feedback);
                    self.handoff_runtime_wake_for_agent(
                        decision.agent_id.as_str(),
                        crate::runtime::ContinuationStatusV1::Rejected,
                        "provider_query_not_executable",
                    )
                    .map_err(|error| AgentDecisionTrace {
                        agent_id: decision.agent_id.clone(),
                        time: self.world.state().time,
                        decision: AgentDecision::Wait,
                        llm_input: None,
                        llm_output: None,
                        llm_error: Some(error),
                        parse_error: None,
                        llm_diagnostics: None,
                        llm_effect_intents: Vec::new(),
                        llm_effect_receipts: Vec::new(),
                        llm_step_trace: Vec::new(),
                        llm_prompt_section_trace: Vec::new(),
                        llm_chat_messages: Vec::new(),
                    })?;
                }
            }
            AgentDecision::ModuleCommand { .. } => {
                if let Some(cognition) = decision.cognition {
                    self.llm_sidecar
                        .fail_provider_cognition_turn(
                            &mut self.world,
                            decision.agent_id.as_str(),
                            "cognition_failed",
                        )
                        .map_err(|error| {
                            wake_handoff_error_trace(
                                decision.agent_id.as_str(),
                                self.world.state().time,
                                error,
                            )
                        })?;
                    let feedback = self.llm_sidecar.provider_feedback(
                        &cognition,
                        None,
                        "rejected",
                        None,
                        None,
                        Some("module commands require the Runtime typed command lane".to_string()),
                    );
                    self.llm_sidecar
                        .fail_provider_turn(decision.agent_id.as_str());
                    self.deliver_provider_feedback_best_effort(feedback);
                    self.handoff_runtime_wake_for_agent(
                        decision.agent_id.as_str(),
                        crate::runtime::ContinuationStatusV1::Rejected,
                        "provider_module_command_not_executable",
                    )
                    .map_err(|error| AgentDecisionTrace {
                        agent_id: decision.agent_id.clone(),
                        time: self.world.state().time,
                        decision: AgentDecision::Wait,
                        llm_input: None,
                        llm_output: None,
                        llm_diagnostics: None,
                        llm_error: Some(error),
                        parse_error: None,
                        llm_effect_intents: Vec::new(),
                        llm_effect_receipts: Vec::new(),
                        llm_step_trace: Vec::new(),
                        llm_prompt_section_trace: Vec::new(),
                        llm_chat_messages: Vec::new(),
                    })?;
                }
            }
        }
        Ok(decision_trace)
    }

    fn finish_provider_transport_exhaustion(
        &mut self,
        agent_id: String,
        prior_trace: Option<AgentDecisionTrace>,
    ) -> AgentDecisionTrace {
        let reason = "failed_provider: provider transport retry budget exhausted";
        if let Some(feedback) = self.llm_sidecar.fail_provider_turn_with_feedback(
            agent_id.as_str(),
            "failed",
            "failed_provider",
        ) {
            self.deliver_provider_feedback_best_effort(feedback);
        }
        self.llm_sidecar
            .clear_provider_transport_exhausted(agent_id.as_str());
        let mut trace = prior_trace.unwrap_or_else(|| AgentDecisionTrace {
            agent_id: agent_id.clone(),
            time: self.world.state().time,
            decision: AgentDecision::Wait,
            llm_input: None,
            llm_output: None,
            llm_error: None,
            parse_error: None,
            llm_diagnostics: None,
            llm_effect_intents: Vec::new(),
            llm_effect_receipts: Vec::new(),
            llm_step_trace: Vec::new(),
            llm_prompt_section_trace: Vec::new(),
            llm_chat_messages: Vec::new(),
        });
        trace.agent_id = agent_id;
        trace.decision = AgentDecision::Wait;
        trace.llm_error = Some(reason.to_string());
        trace.llm_output = Some(
            serde_json::json!({
                "provider_error": {
                    "code": "failed_provider",
                    "retryable": false,
                }
            })
            .to_string(),
        );
        if let Err(error) = self.handoff_runtime_wake_for_agent(
            trace.agent_id.as_str(),
            crate::runtime::ContinuationStatusV1::Rejected,
            "provider_transport_exhausted",
        ) {
            trace.llm_error = Some(format!(
                "{}; Runtime wake handoff failed: {error}",
                trace.llm_error.take().unwrap_or_default()
            ));
        }
        trace
    }

    fn deliver_provider_feedback_best_effort(
        &mut self,
        feedback: crate::simulator::FeedbackEnvelopeV1,
    ) {
        self.deliver_provider_feedback_with_projection(
            feedback,
            RuntimeFeedbackProjectionV1::default(),
        );
    }

    fn deliver_provider_feedback_with_projection(
        &mut self,
        feedback: crate::simulator::FeedbackEnvelopeV1,
        projection: RuntimeFeedbackProjectionV1,
    ) {
        if let Err(error) = self.allocate_runtime_feedback(feedback, projection) {
            tracing::warn!(
                error = ?error,
                "Runtime feedback allocation failed; provider feedback was not delivered"
            );
            return;
        }
        self.drain_provider_feedback_outbox();
    }

    fn allocate_runtime_feedback(
        &mut self,
        feedback: crate::simulator::FeedbackEnvelopeV1,
        projection: RuntimeFeedbackProjectionV1,
    ) -> Result<crate::runtime::RuntimeFeedbackOutboxRecordV1, String> {
        let status = feedback.status.clone();
        let request = RuntimeFeedbackRequestV1 {
            // Runtime receipts already carry a Runtime-issued feedback id.
            // All other dispositions receive their id and sequence from the
            // Runtime allocator, never from the adapter-local compatibility
            // counter.
            feedback_id: feedback
                .runtime_receipt_id
                .as_ref()
                .map(|_| feedback.feedback_id.clone()),
            agent_subject: feedback.agent_subject,
            agent_session_id: feedback.agent_session_id,
            agent_turn_id: feedback.agent_turn_id,
            decision_request_id: feedback.decision_request_id,
            candidate_action_id: feedback.candidate_action_id,
            runtime_receipt_id: feedback.runtime_receipt_id,
            status: feedback.status,
            request_digest: feedback.request_digest.to_string(),
            reject_reason: canonical_feedback_reason(
                status.as_str(),
                feedback.reject_reason.as_deref(),
            ),
        };
        self.world
            .allocate_runtime_feedback_with_projection(request, projection)
            .map_err(|error| format!("Runtime feedback allocation failed: {error:?}"))
    }

    /// Drive the Runtime-owned feedback outbox without making the provider
    /// transport part of the World transaction. Claimed records remain
    /// durable and are returned to `pending` on transport failure, including
    /// across a viewer restart.
    fn drain_provider_feedback_outbox(&mut self) {
        let pending = match self.world.pending_runtime_feedback() {
            Ok(pending) => pending,
            Err(error) => {
                tracing::warn!(
                    error = ?error,
                    "Runtime feedback outbox read failed"
                );
                return;
            }
        };
        for record in pending {
            let feedback_id = record.feedback_id.clone();
            let claimed = match self.world.claim_runtime_feedback(feedback_id.as_str()) {
                Ok(record) => record,
                Err(error) => {
                    tracing::warn!(
                        feedback_id = feedback_id.as_str(),
                        error = ?error,
                        "Runtime feedback outbox claim failed"
                    );
                    continue;
                }
            };
            let payload = match claimed.transport_payload() {
                Ok(payload) => payload,
                Err(error) => {
                    self.retry_runtime_feedback_outbox(
                        feedback_id.as_str(),
                        format!("feedback transport payload validation failed: {error}"),
                    );
                    continue;
                }
            };
            if let Err(error) =
                serde_json::from_value::<crate::simulator::FeedbackEnvelopeV1>(payload.clone())
            {
                self.retry_runtime_feedback_outbox(
                    feedback_id.as_str(),
                    format!("feedback transport payload decode failed: {error}"),
                );
                continue;
            }
            match self
                .llm_sidecar
                .deliver_provider_cognition_feedback(&payload)
            {
                Ok(()) => {
                    if let Err(error) = self.world.ack_runtime_feedback(feedback_id.as_str()) {
                        tracing::warn!(
                            feedback_id = feedback_id.as_str(),
                            error = ?error,
                            "Runtime feedback outbox acknowledgement failed"
                        );
                        self.retry_runtime_feedback_outbox(
                            feedback_id.as_str(),
                            format!("feedback acknowledgement failed: {error:?}"),
                        );
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        feedback_id = feedback_id.as_str(),
                        error = %error,
                        "provider cognition feedback delivery failed; returning to Runtime outbox"
                    );
                    self.retry_runtime_feedback_outbox(feedback_id.as_str(), error);
                }
            }
        }
    }

    fn retry_runtime_feedback_outbox(&mut self, feedback_id: &str, reason: impl Into<String>) {
        if let Err(error) = self
            .world
            .retry_runtime_feedback(feedback_id, reason.into())
        {
            tracing::warn!(
                feedback_id,
                error = ?error,
                "Runtime feedback outbox retry transition failed"
            );
        }
    }

    fn commit_provider_runtime_action(
        &mut self,
        runtime_action: &crate::runtime::Action,
        cognition: &RuntimeProviderActionContext,
        simulator_action: SimulatorAction,
    ) -> Result<(), ProviderRuntimeActionCommitError> {
        if self.world.pending_actions_len() != 0 {
            return Err(ProviderRuntimeActionCommitError::Message(
                "Runtime has pending actions; provider cognition action rejected".to_string(),
            ));
        }

        let (request, response_artifact) = provider_cognition_commit_inputs(&self.world, cognition)
            .map_err(ProviderRuntimeActionCommitError::Message)?;
        let (committed, returned_lineage) = self
            .world
            .commit_cognition_action(request, runtime_action.clone(), response_artifact)
            .map_err(|error| {
                if matches!(
                    classify_cognition_commit_error(&error),
                    Some(CognitionCommitRejectReasonV1::StaleBase)
                ) {
                    ProviderRuntimeActionCommitError::StaleBase
                } else {
                    ProviderRuntimeActionCommitError::Message(format!(
                        "Runtime cognition action commit rejected provider action: {error:?}"
                    ))
                }
            })?;
        let lineage = self
            .world
            .read_runtime_receipt_lineage(returned_lineage.receipt_id.as_str())
            .map_err(|error| {
                ProviderRuntimeActionCommitError::Message(format!(
                    "Runtime cognition receipt readback failed: {error:?}"
                ))
            })?;
        self.world
            .verify_runtime_receipt_lineage(&lineage)
            .map_err(|error| {
                ProviderRuntimeActionCommitError::Message(format!(
                    "Runtime cognition receipt verification failed: {error:?}"
                ))
            })?;
        if lineage != returned_lineage || lineage.receipt_id != committed.receipt_id {
            return Err(ProviderRuntimeActionCommitError::Message(
                "Runtime cognition receipt readback identity mismatch".to_string(),
            ));
        }
        let action_id = committed
            .action_id
            .strip_prefix("action:")
            .ok_or_else(|| {
                ProviderRuntimeActionCommitError::Message(
                    "Runtime cognition commit returned an invalid action id".to_string(),
                )
            })?
            .parse::<u64>()
            .map_err(|error| {
                ProviderRuntimeActionCommitError::Message(format!(
                    "Runtime cognition action id is not numeric: {error}"
                ))
            })?;
        self.llm_sidecar
            .clear_provider_stale_replans(cognition.request.request_context.agent_subject.as_str());
        self.llm_sidecar.track_action(
            action_id,
            cognition.request.request_context.agent_subject.clone(),
            simulator_action,
            Some(cognition.clone()),
        );
        let feedback = self.llm_sidecar.provider_feedback(
            cognition,
            Some(action_id),
            "committed",
            Some(committed.receipt_id.clone()),
            Some(lineage.feedback_id.clone()),
            None,
        );
        let feedback_projection = RuntimeFeedbackProjectionV1 {
            envelope_digest: Some(lineage.envelope_digest.clone()),
            emitted_events: Vec::new(),
            committed_event_summary: Some(format!(
                "runtime_receipt_id={} action_id={}",
                lineage.receipt_id, lineage.action_id
            )),
            world_delta_summary: None,
        };
        let queued_feedback = self
            .allocate_runtime_feedback(feedback.clone(), feedback_projection)
            .map_err(|error| {
                tracing::warn!(
                    error,
                    "Runtime receipt committed but feedback outbox allocation failed"
                );
                error
            })
            .ok();
        let feedback = queued_feedback
            .as_ref()
            .and_then(|record| record.transport_payload().ok())
            .and_then(|payload| serde_json::from_value(payload).ok())
            .unwrap_or(feedback);
        self.llm_sidecar
            .consume_provider_memory_after_receipt(
                cognition.request.request_context.agent_subject.as_str(),
                feedback.clone(),
                &lineage,
            )
            .map_err(ProviderRuntimeActionCommitError::Message)?;
        let finalized = self
            .llm_sidecar
            .finalize_provider_action_with_feedback(action_id, feedback)
            .ok_or_else(|| {
                "provider cognition single-flight did not close after Runtime receipt".to_string()
            });
        finalized.map_err(ProviderRuntimeActionCommitError::Message)?;
        // Runtime has already committed and issued the receipt. Queueing and
        // delivery are separate so a provider transport failure cannot turn
        // that authoritative commit into a viewer-side ActionRejected event.
        if queued_feedback.is_some() {
            self.drain_provider_feedback_outbox();
        }
        self.handoff_runtime_wake_for_agent(
            cognition.request.request_context.agent_subject.as_str(),
            crate::runtime::ContinuationStatusV1::Completed,
            "provider_action_committed",
        )
        .map_err(ProviderRuntimeActionCommitError::Message)?;
        Ok(())
    }
}

fn canonical_feedback_reason(status: &str, reason: Option<&str>) -> Option<String> {
    match status {
        "committed" => None,
        "pending" => Some("retry_scheduled".to_string()),
        "failed" => Some(
            matches!(
                reason,
                Some("failed_provider")
                    | Some("failed_persist")
                    | Some("cognition_failed")
                    | Some("provider_unavailable")
            )
            .then_some(reason.unwrap_or("provider_unavailable"))
            .unwrap_or("provider_unavailable")
            .to_string(),
        ),
        "rejected" => Some(
            matches!(
                reason,
                Some("stale_base")
                    | Some("expired")
                    | Some("stale_capability_snapshot")
                    | Some("authority_denied")
                    | Some("intent_conflict")
                    | Some("reorg_invalidated")
                    | Some("finality_anchor_mismatch")
                    | Some("precondition_failed")
                    | Some("action_rejected")
                    | Some("idempotency_conflict")
                    | Some("no_effect")
                    | Some("cancelled")
                    | Some("late_response_after_cancel")
                    | Some("legacy_no_cognition_proof")
                    | Some("cognition_context_mismatch")
            )
            .then_some(reason.unwrap_or("action_rejected"))
            .unwrap_or("action_rejected")
            .to_string(),
        ),
        _ => reason.map(ToOwned::to_owned),
    }
}

fn wake_handoff_error_trace(agent_id: &str, time: u64, error: String) -> AgentDecisionTrace {
    AgentDecisionTrace {
        agent_id: agent_id.to_string(),
        time,
        decision: AgentDecision::Wait,
        llm_input: None,
        llm_output: None,
        llm_error: Some(error),
        parse_error: None,
        llm_diagnostics: None,
        llm_effect_intents: Vec::new(),
        llm_effect_receipts: Vec::new(),
        llm_step_trace: Vec::new(),
        llm_prompt_section_trace: Vec::new(),
        llm_chat_messages: Vec::new(),
    }
}

fn provider_cognition_commit_inputs(
    world: &RuntimeWorld,
    cognition: &RuntimeProviderActionContext,
) -> Result<
    (
        RuntimeCognitionCommitRequestV1,
        RuntimeCognitionResponseArtifactV1,
    ),
    String,
> {
    let response = &cognition.response;
    let request = &cognition.request.request_context;
    let response_identity = response.response_artifact_identity();
    response
        .validate_response_artifact_identity(&response_identity)
        .map_err(|error| format!("provider response identity rejected: {error}"))?;
    let capability_root = world.capability_authorization_root().to_string();
    let capability_snapshot_hash = h_v1("oasis7.runtime.manifest.v1", &capability_root).to_string();
    let authority_context_hash =
        h_v1("oasis7.runtime.authority-context.v1", &capability_root).to_string();
    Ok((
        RuntimeCognitionCommitRequestV1 {
            agent_id: request.agent_subject.clone(),
            agent_session_id: request.agent_session_id.clone(),
            agent_turn_id: request.agent_turn_id.clone(),
            decision_request_id: request.decision_request_id.clone(),
            retry_seq: request.retry_seq,
            transport_attempt: request.transport_attempt,
            request_digest: request.request_digest.to_string(),
            observation_digest: request.observation_digest.to_string(),
            context_digest: super::llm_sidecar::runtime_provider_context_digest(request),
            capability_snapshot_hash,
            authority_context_hash,
            captured_base_binding: RuntimeCognitionBaseBindingV1 {
                world_id: request.runtime_binding.world_id.clone(),
                branch_id: request.runtime_binding.branch_id.clone(),
                finality_epoch: request.runtime_binding.finality_epoch,
                finality_block_hash: request
                    .runtime_binding
                    .finality_block_hash
                    .as_ref()
                    .map(ToString::to_string),
                finality_status: request.runtime_binding.finality_status.clone(),
                base_tick: request.runtime_binding.base_tick,
                base_world_hash: request.runtime_binding.base_world_hash.to_string(),
                reorg_epoch: request.runtime_binding.reorg_epoch,
                runtime_manifest_hash: request.runtime_binding.runtime_manifest_hash.to_string(),
            },
        },
        RuntimeCognitionResponseArtifactV1 {
            schema_version: response.context_version,
            context_discriminator: response.context_discriminator.clone(),
            context_version: response.context_version,
            agent_session_id: response.agent_session_id.clone(),
            agent_turn_id: response.agent_turn_id.clone(),
            decision_request_id: response.decision_request_id.clone(),
            retry_seq: response.retry_seq,
            transport_attempt: response.transport_attempt,
            request_digest: response.request_digest.to_string(),
            response_digest: response.response_digest.to_string(),
            artifact_digest: response_identity.artifact_digest.to_string(),
        },
    ))
}

enum ProviderRuntimeActionCommitError {
    StaleBase,
    Message(String),
}

impl ProviderRuntimeActionCommitError {
    fn is_stale_base(&self) -> bool {
        matches!(self, Self::StaleBase)
    }

    fn reason(&self) -> String {
        match self {
            Self::StaleBase => CognitionCommitRejectReasonV1::StaleBase.code().to_string(),
            Self::Message(reason) => reason.clone(),
        }
    }
}

fn stale_replan_exhausted_trace(world: &RuntimeWorld, agent_id: &str) -> AgentDecisionTrace {
    AgentDecisionTrace {
        agent_id: agent_id.to_string(),
        time: world.state().time,
        decision: AgentDecision::Wait,
        llm_input: None,
        llm_output: None,
        llm_error: Some("stale_base replan budget exhausted".to_string()),
        parse_error: None,
        llm_diagnostics: None,
        llm_effect_intents: Vec::new(),
        llm_effect_receipts: Vec::new(),
        llm_step_trace: Vec::new(),
        llm_prompt_section_trace: Vec::new(),
        llm_chat_messages: Vec::new(),
    }
}
