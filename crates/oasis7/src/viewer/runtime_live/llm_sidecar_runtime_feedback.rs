use super::*;
use crate::runtime::{
    CausedBy as RuntimeCausedBy, DomainEvent as RuntimeDomainEvent,
    WorldEvent as RuntimeWorldEvent, WorldEventBody as RuntimeWorldEventBody,
};
use crate::simulator::{Action as SimulatorAction, ActionResult, FeedbackEnvelopeV1};

impl RuntimeLlmSidecar {
    pub(in crate::viewer::runtime_live) fn schedule_provider_stale_replan(
        &mut self,
        agent_id: &str,
        parent_agent_turn_id: &str,
        parent_decision_request_id: &str,
    ) -> bool {
        let state = self
            .provider_stale_replans
            .entry(agent_id.to_string())
            .or_insert_with(|| ProviderStaleReplanState {
                count: 0,
                pending_cause: None,
            });
        if state.count >= MAX_PROVIDER_STALE_REPLANS {
            return false;
        }
        state.count = state.count.saturating_add(1);
        state.pending_cause = Some(ProviderStaleReplanCause {
            parent_agent_turn_id: parent_agent_turn_id.to_string(),
            parent_decision_request_id: parent_decision_request_id.to_string(),
            count: state.count,
        });
        true
    }

    pub(in crate::viewer::runtime_live) fn provider_stale_replan_cause(
        &self,
        agent_id: &str,
    ) -> Option<ProviderStaleReplanCause> {
        self.provider_stale_replans
            .get(agent_id)
            .and_then(|state| state.pending_cause.clone())
    }

    pub(in crate::viewer::runtime_live) fn mark_provider_stale_replan_dispatched(
        &mut self,
        agent_id: &str,
    ) {
        if let Some(state) = self.provider_stale_replans.get_mut(agent_id) {
            state.pending_cause = None;
        }
    }

    pub(in crate::viewer::runtime_live) fn clear_provider_stale_replans(&mut self, agent_id: &str) {
        self.provider_stale_replans.remove(agent_id);
    }

    pub(in crate::viewer::runtime_live) fn provider_stale_replan_exhausted_agent(
        &self,
    ) -> Option<String> {
        self.provider_stale_replans
            .iter()
            .find(|(_, state)| state.count >= MAX_PROVIDER_STALE_REPLANS)
            .map(|(agent_id, _)| agent_id.clone())
    }

    pub(in crate::viewer::runtime_live) fn track_action(
        &mut self,
        action_id: u64,
        agent_id: String,
        action: SimulatorAction,
        cognition: Option<RuntimeProviderActionContext>,
    ) {
        self.pending_actions.insert(
            action_id,
            RuntimePendingAction {
                agent_id,
                action,
                cognition,
                feedback_emitted: false,
            },
        );
    }

    pub(in crate::viewer::runtime_live) fn notify_action_result(
        &mut self,
        action_id: u64,
        event: WorldEvent,
        rejected: bool,
    ) {
        let Some(mut pending) = self.pending_actions.remove(&action_id) else {
            return;
        };
        if pending.feedback_emitted {
            self.pending_actions.insert(action_id, pending);
            return;
        }
        let success = !rejected;
        let action_result = ActionResult {
            action: pending.action.clone(),
            action_id,
            success,
            event: event.clone(),
        };
        if let Some(runner) = self.runner.as_mut() {
            match runner {
                RuntimeDecisionRunner::Builtin(runner) => {
                    let _ = runner.notify_action_result(pending.agent_id.as_str(), &action_result);
                }
                RuntimeDecisionRunner::ProviderBacked(_) => {
                    // Production provider turns use the typed Runtime
                    // feedback seam below. Do not also send the legacy
                    // feedback envelope, which lacks turn/receipt identity.
                }
            }
        }
        pending.feedback_emitted = true;
        if rejected {
            self.release_provider_turn(pending.agent_id.as_str());
        } else {
            // ActionAccepted is an intermediate Runtime acknowledgement. The
            // pending action and provider turn remain occupied until a real
            // Runtime receipt/terminal callback is supplied.
            self.pending_actions.insert(action_id, pending);
        }
    }

    /// Runtime integration seam for the typed feedback path. A viewer event
    /// can establish `pending`, but only a Runtime receipt may close a
    /// successful provider turn as `committed`.
    pub(in crate::viewer::runtime_live) fn finalize_provider_action(
        &mut self,
        action_id: u64,
        status: &str,
        runtime_receipt_id: Option<String>,
        feedback_id: Option<String>,
    ) -> Option<FeedbackEnvelopeV1> {
        if status == "committed"
            && runtime_receipt_id
                .as_deref()
                .is_none_or(|receipt| receipt.trim().is_empty())
        {
            return None;
        }
        if !matches!(status, "committed" | "rejected" | "failed") {
            return None;
        }
        let Some(pending) = self.pending_actions.remove(&action_id) else {
            return None;
        };
        let feedback = pending.cognition.as_ref().map(|cognition| {
            self.provider_feedback(
                cognition,
                Some(action_id),
                status,
                runtime_receipt_id,
                feedback_id,
                None,
            )
        });
        self.release_provider_turn(pending.agent_id.as_str());
        feedback
    }

    /// Close a provider response that did not produce a Runtime action (for
    /// example an unmappable or non-retryable candidate).
    pub(in crate::viewer::runtime_live) fn fail_provider_turn(&mut self, agent_id: &str) {
        self.release_provider_turn(agent_id);
    }

    /// Create a typed terminal disposition for a provider request that never
    /// produced a Runtime action. The request context remains available until
    /// this method builds the envelope, so failures cannot silently fall back
    /// to legacy action feedback.
    pub(in crate::viewer::runtime_live) fn fail_provider_turn_with_feedback(
        &mut self,
        agent_id: &str,
        status: &str,
        reject_reason: impl Into<String>,
    ) -> Option<FeedbackEnvelopeV1> {
        if !matches!(status, "rejected" | "failed") {
            return None;
        }
        let context = self.provider_contexts.get(agent_id).cloned();
        let feedback = context.as_ref().map(|context| {
            self.provider_feedback_for_request(
                &context.request_context,
                None,
                status,
                None,
                None,
                Some(reject_reason.into()),
            )
        });
        self.release_provider_turn(agent_id);
        feedback
    }

    pub(in crate::viewer::runtime_live) fn provider_feedback(
        &mut self,
        cognition: &RuntimeProviderActionContext,
        candidate_action_id: Option<u64>,
        status: &str,
        runtime_receipt_id: Option<String>,
        feedback_id: Option<String>,
        reject_reason: Option<String>,
    ) -> FeedbackEnvelopeV1 {
        self.provider_feedback_for_request(
            &cognition.request.request_context,
            candidate_action_id,
            status,
            runtime_receipt_id,
            feedback_id,
            reject_reason,
        )
    }

    fn provider_feedback_for_request(
        &mut self,
        request: &crate::simulator::ContinuousAgentRequestContextV1,
        candidate_action_id: Option<u64>,
        status: &str,
        runtime_receipt_id: Option<String>,
        feedback_id: Option<String>,
        reject_reason: Option<String>,
    ) -> FeedbackEnvelopeV1 {
        let feedback_seq = self
            .provider_feedback_seq
            .entry(request.agent_subject.clone())
            .or_insert(1);
        let sequence = (*feedback_seq).max(1);
        *feedback_seq = sequence.saturating_add(1);
        let feedback_id = feedback_id
            .unwrap_or_else(|| format!("runtime-feedback:{}:{}", request.agent_subject, sequence));
        FeedbackEnvelopeV1 {
            feedback_id,
            feedback_seq: sequence,
            agent_subject: request.agent_subject.clone(),
            agent_session_id: request.agent_session_id.clone(),
            agent_turn_id: request.agent_turn_id.clone(),
            decision_request_id: request.decision_request_id.clone(),
            candidate_action_id,
            runtime_receipt_id,
            status: status.to_string(),
            request_digest: request.request_digest.clone(),
            reject_reason,
            provenance: "runtime_authoritative".to_string(),
        }
    }

    /// Deliver the Runtime-committed feedback through the same provider
    /// transport used for decisions. The receipt lineage has already been
    /// read back and verified by Runtime before this method is called.
    pub(in crate::viewer::runtime_live) fn deliver_provider_cognition_feedback(
        &self,
        feedback: &FeedbackEnvelopeV1,
    ) -> Result<(), String> {
        let settings = provider_settings_from_env()?
            .ok_or_else(|| "provider settings disappeared before feedback delivery".to_string())?;
        let client = ProviderLoopbackHttpClient::new_with_transport(
            settings.base_url.as_str(),
            settings.auth_token.as_deref(),
            settings.connect_timeout_ms,
            settings.provider_transport.as_str(),
        )
        .map_err(|error| format!("provider feedback client initialization failed: {error}"))?;
        let ack = client
            .submit_feedback_context(feedback)
            .map_err(|error| format!("provider cognition feedback delivery failed: {error}"))?;
        if !ack.ok {
            return Err(format!(
                "provider rejected Runtime cognition feedback: {}",
                ack.error
                    .or(ack.error_code)
                    .unwrap_or_else(|| "unknown provider feedback error".to_string())
            ));
        }
        Ok(())
    }

    fn release_provider_turn(&mut self, agent_id: &str) {
        if let Some(context) = self.provider_contexts.get(agent_id).cloned() {
            self.expire_async_runtime_turn(&context);
        }
        self.provider_active_turns.remove(agent_id);
        self.provider_wait_until.remove(agent_id);
        self.provider_contexts.remove(agent_id);
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::viewer::runtime_live) fn expire_async_runtime_turn(
        &mut self,
        context: &cognition_context::ProviderContextState,
    ) {
        if let Some(RuntimeDecisionRunner::ProviderBacked(runner)) = self.runner.as_mut() {
            let _ = runner.expire_runtime_turn(
                context.request_context.agent_subject.as_str(),
                context.request_context.agent_session_id.as_str(),
                context.request_context.agent_turn_id.as_str(),
                context.request_context.decision_request_id.as_str(),
            );
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(in crate::viewer::runtime_live) fn expire_async_runtime_turn(
        &mut self,
        _context: &cognition_context::ProviderContextState,
    ) {
    }

    pub(in crate::viewer::runtime_live) fn schedule_provider_wait(
        &mut self,
        agent_id: &str,
        now: u64,
        ticks: u64,
    ) {
        let wait_until = now.saturating_add(ticks.max(1));
        self.provider_wait_until
            .insert(agent_id.to_string(), wait_until);
    }

    pub(in crate::viewer::runtime_live) fn release_due_provider_waits(
        &mut self,
        world: &mut RuntimeWorld,
    ) -> Result<(), String> {
        let now = world.state().time;
        let due_agents: Vec<String> = self
            .provider_wait_until
            .iter()
            .filter_map(|(agent_id, wait_until)| (*wait_until <= now).then_some(agent_id.clone()))
            .collect();
        for agent_id in due_agents {
            if let Some(context) = self.provider_contexts.get(agent_id.as_str()).cloned() {
                let feedback = self.provider_feedback_for_request(
                    &context.request_context,
                    None,
                    "failed",
                    None,
                    None,
                    Some("provider wait elapsed without a Runtime action".to_string()),
                );
                world.enqueue_runtime_feedback(feedback).map_err(|error| {
                    format!("Runtime feedback outbox enqueue failed after provider wait: {error:?}")
                })?;
            }
            self.release_provider_turn(agent_id.as_str());
        }
        Ok(())
    }

    pub(in crate::viewer::runtime_live) fn notify_action_result_if_needed(
        &mut self,
        runtime_event: &RuntimeWorldEvent,
        mapped_event: WorldEvent,
    ) {
        let Some(caused_by) = runtime_event.caused_by.as_ref() else {
            return;
        };
        let RuntimeCausedBy::Action(action_id) = caused_by else {
            return;
        };
        let rejected = matches!(
            runtime_event.body,
            RuntimeWorldEventBody::Domain(RuntimeDomainEvent::ActionRejected { .. })
        );
        self.notify_action_result(*action_id, mapped_event, rejected);
    }
}
