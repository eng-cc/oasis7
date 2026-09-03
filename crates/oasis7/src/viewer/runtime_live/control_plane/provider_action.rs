use super::super::decision_trace::is_trace_only_overflow;
use super::llm_sidecar::RuntimeProviderActionContext;
use super::*;
use crate::runtime::{
    CognitionCommitRejectReasonV1, RuntimeCognitionBaseBindingV1, RuntimeCognitionCommitRequestV1,
    RuntimeCognitionResponseArtifactV1, classify_cognition_commit_error,
};
use crate::simulator::{Action as SimulatorAction, AgentDecision, h_v1};

impl ViewerRuntimeLiveServer {
    pub(in crate::viewer::runtime_live) fn enqueue_llm_action_from_sidecar(
        &mut self,
    ) -> Result<Option<AgentDecisionTrace>, AgentDecisionTrace> {
        self.drain_provider_feedback_outbox();
        if let Some(agent_id) = self.llm_sidecar.provider_stale_replan_exhausted_agent() {
            return Err(stale_replan_exhausted_trace(&self.world, agent_id.as_str()));
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
                        Some(format!("provider requested wait for {ticks} tick(s)")),
                    );
                    self.deliver_provider_feedback_best_effort(feedback);
                }
            }
            AgentDecision::Query(_) => {
                if let Some(cognition) = decision.cognition {
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
                }
            }
            AgentDecision::ModuleCommand { .. } => {
                if let Some(cognition) = decision.cognition {
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
        trace
    }

    fn deliver_provider_feedback_best_effort(
        &mut self,
        feedback: crate::simulator::FeedbackEnvelopeV1,
    ) {
        if let Err(error) = self.world.enqueue_runtime_feedback(feedback) {
            tracing::warn!(
                error = ?error,
                "Runtime feedback outbox enqueue failed; provider feedback was not delivered"
            );
            return;
        }
        self.drain_provider_feedback_outbox();
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
            let feedback = match serde_json::from_value::<crate::simulator::FeedbackEnvelopeV1>(
                claimed.payload.clone(),
            ) {
                Ok(feedback) => feedback,
                Err(error) => {
                    self.retry_runtime_feedback_outbox(
                        feedback_id.as_str(),
                        format!("feedback payload decode failed: {error}"),
                    );
                    continue;
                }
            };
            match self
                .llm_sidecar
                .deliver_provider_cognition_feedback(&feedback)
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
        let feedback = self
            .llm_sidecar
            .finalize_provider_action(
                action_id,
                "committed",
                Some(committed.receipt_id),
                Some(lineage.feedback_id),
            )
            .ok_or_else(|| {
                "provider cognition single-flight did not close after Runtime receipt".to_string()
            })
            .map_err(ProviderRuntimeActionCommitError::Message)?;
        // Runtime has already committed and issued the receipt. Queueing and
        // delivery are separate so a provider transport failure cannot turn
        // that authoritative commit into a viewer-side ActionRejected event.
        self.deliver_provider_feedback_best_effort(feedback);
        Ok(())
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
            context_digest: h_v1("oasis7.cognition.context.v1", request).to_string(),
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
