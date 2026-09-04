use super::*;
use crate::runtime::{
    CausedBy as RuntimeCausedBy, DomainEvent as RuntimeDomainEvent,
    WorldEvent as RuntimeWorldEvent, WorldEventBody as RuntimeWorldEventBody,
};
use crate::simulator::{Action as SimulatorAction, ActionResult, FeedbackEnvelopeV1};
use serde_json::Value as JsonValue;

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
        self.persist_provider_lineage_best_effort();
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
            self.persist_provider_lineage_best_effort();
        }
    }

    pub(in crate::viewer::runtime_live) fn clear_provider_stale_replans(&mut self, agent_id: &str) {
        if self.provider_stale_replans.remove(agent_id).is_some() {
            self.persist_provider_lineage_best_effort();
        }
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
        self.persist_provider_lineage_best_effort();
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
        // A provider-backed action is finalized from the Runtime receipt
        // before the corresponding world event reaches this adapter.  The
        // later ActionAccepted callback is an informational replay and must
        // not resurrect an already terminal single-flight action.
        if let Some(cognition) = pending.cognition.as_ref()
            && self
                .provider_terminal_states
                .get(pending.agent_id.as_str())
                .is_some_and(|terminal| {
                    terminal.agent_turn_id == cognition.request.request_context.agent_turn_id
                        && terminal.decision_request_id
                            == cognition.request.request_context.decision_request_id
                })
        {
            self.persist_provider_lineage_best_effort();
            return;
        }
        let success = !rejected;
        let action_result = ActionResult {
            action: pending.action.clone(),
            action_id,
            success,
            event: event.clone(),
        };
        #[cfg(target_arch = "wasm32")]
        if let Some(RuntimeDecisionRunner::Builtin(runner)) = self.runner.as_mut() {
            let _ = runner.notify_action_result(pending.agent_id.as_str(), &action_result);
        }
        // Native Builtin and ProviderBacked actors both use the typed Runtime
        // feedback seam. Sending the legacy action result here would bypass
        // receipt identity and make the two production lanes diverge.
        pending.feedback_emitted = true;
        if rejected {
            self.clear_provider_stale_replans(pending.agent_id.as_str());
            self.release_provider_turn(pending.agent_id.as_str());
        } else {
            // ActionAccepted is an intermediate Runtime acknowledgement. The
            // pending action and provider turn remain occupied until a real
            // Runtime receipt/terminal callback is supplied.
            self.pending_actions.insert(action_id, pending);
        }
        self.persist_provider_lineage_best_effort();
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
        let Some(pending) = self.pending_actions.get(&action_id).cloned() else {
            return None;
        };
        let Some(cognition) = pending.cognition.as_ref() else {
            // Builtin/legacy actions have no provider envelope to finalize.
            self.pending_actions.remove(&action_id);
            self.release_provider_turn(pending.agent_id.as_str());
            return None;
        };
        let feedback = self.provider_feedback(
            cognition,
            Some(action_id),
            status,
            runtime_receipt_id,
            feedback_id,
            None,
        );
        self.finalize_provider_action_with_feedback(action_id, feedback)
    }

    /// Finalize with a prebuilt Runtime feedback envelope.  This lets the
    /// production control plane consume provider memory intents against the
    /// exact Runtime receipt before the sidecar releases the awaiting actor
    /// outcome, without minting a second feedback sequence/id.
    pub(in crate::viewer::runtime_live) fn finalize_provider_action_with_feedback(
        &mut self,
        action_id: u64,
        feedback: FeedbackEnvelopeV1,
    ) -> Option<FeedbackEnvelopeV1> {
        let Some(pending) = self.pending_actions.remove(&action_id) else {
            return None;
        };
        if let Some(cognition) = pending.cognition.as_ref() {
            self.record_provider_terminal_state(
                pending.agent_id.as_str(),
                &cognition.request,
                feedback.status.as_str(),
                feedback.reject_reason.clone(),
                Some(feedback.feedback_id.clone()),
            );
        }
        self.provider_held_decisions
            .remove(pending.agent_id.as_str());
        self.release_provider_turn(pending.agent_id.as_str());
        Some(feedback)
    }

    /// Apply intents captured by an actor only after the Runtime has read back
    /// and verified the committed receipt lineage. Rejected or pending
    /// dispositions never enter the MemoryWriteStore. Native actors retain
    /// their awaiting outcome in AsyncAgentRunner, while the WASM synchronous
    /// lane projects the same bounded intent policy here.
    pub(in crate::viewer::runtime_live) fn consume_provider_memory_after_receipt(
        &mut self,
        agent_id: &str,
        feedback: FeedbackEnvelopeV1,
        receipt: &crate::runtime::RuntimeReceiptLineageV1,
        memory_write_intents: &[crate::simulator::MemoryWriteIntent],
    ) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(runner) = self
                .runner
                .as_mut()
                .and_then(RuntimeDecisionRunner::async_runner_mut)
            else {
                return Ok(());
            };
            runner
                .consume_runtime_feedback_with_lineage(
                    agent_id,
                    feedback,
                    Some(receipt),
                    &mut self.provider_memory_store,
                )
                .map_err(|error| format!("provider memory receipt gate rejected intents: {error}"))
        }
        #[cfg(target_arch = "wasm32")]
        {
            if feedback.status != "committed" {
                return Ok(());
            }
            let Some(context) = self.provider_contexts.get(agent_id) else {
                return Err("WASM memory receipt gate has no active cognition context".to_string());
            };
            if feedback.agent_subject != agent_id
                || feedback.agent_session_id != context.request_context.agent_session_id
                || feedback.agent_turn_id != context.request_context.agent_turn_id
                || feedback.decision_request_id != context.request_context.decision_request_id
                || feedback.request_digest != context.request_context.request_digest
                || feedback.runtime_receipt_id.as_deref() != Some(receipt.receipt_id.as_str())
            {
                return Err("WASM memory receipt gate context mismatch".to_string());
            }
            receipt
                .validate()
                .map_err(|error| format!("WASM memory receipt lineage invalid: {error}"))?;
            let source = match self.runner.as_ref() {
                Some(RuntimeDecisionRunner::Builtin(_)) => "builtin",
                Some(RuntimeDecisionRunner::ProviderBacked(_)) => "provider",
                None => return Err("WASM memory runner is unavailable".to_string()),
            };
            let policy_context = MemoryWritePolicyContextV1 {
                agent_id: context.request_context.agent_subject.clone(),
                agent_session_id: context.request_context.agent_session_id.clone(),
                agent_turn_id: context.request_context.agent_turn_id.clone(),
                request_digest: context.request_context.request_digest.to_string(),
                source: source.to_string(),
                provenance: if source == "provider" {
                    "provider_unverified".to_string()
                } else {
                    "builtin_unverified".to_string()
                },
            };
            let policy = MemoryWriteIntentPolicyV1::default();
            for intent in memory_write_intents {
                let intent = MemoryWriteIntentV1 {
                    schema_version: 1,
                    scope: intent.scope.clone(),
                    summary: Some(intent.summary.clone()),
                    tags: intent.tags.clone(),
                    compatibility_reason: None,
                };
                let normalized = match policy.normalize(intent, &policy_context) {
                    Ok(intent) => intent,
                    Err(error) => {
                        tracing::warn!(
                            agent_id,
                            code = error.code(),
                            error = %error,
                            "WASM memory intent rejected after Runtime commit"
                        );
                        continue;
                    }
                };
                let digest = match policy.intent_digest(&normalized, &policy_context) {
                    Ok(digest) => digest,
                    Err(error) => {
                        tracing::warn!(
                            agent_id,
                            code = error.code(),
                            error = %error,
                            "WASM memory intent digest rejected after Runtime commit"
                        );
                        continue;
                    }
                };
                if let Err(error) = self
                    .provider_memory_store
                    .apply_runtime_receipt_with_context(
                        normalized,
                        digest,
                        receipt,
                        Some(&policy_context),
                    )
                {
                    tracing::warn!(
                        agent_id,
                        code = error.code(),
                        error = %error,
                        "WASM memory projection rejected after Runtime commit"
                    );
                }
            }
            Ok(())
        }
    }

    /// Close a provider response that did not produce a Runtime action (for
    /// example an unmappable or non-retryable candidate).
    pub(in crate::viewer::runtime_live) fn fail_provider_turn(&mut self, agent_id: &str) {
        self.clear_provider_stale_replans(agent_id);
        self.provider_held_decisions.remove(agent_id);
        self.release_provider_turn(agent_id);
    }

    /// Close a successful provider response that cannot enter the typed
    /// Runtime action lane (for example a query or module command).  The
    /// Runtime failure is recorded before the Viewer exposes the rejected
    /// disposition, so these responses cannot leave a running turn behind.
    pub(in crate::viewer::runtime_live) fn fail_provider_cognition_turn(
        &self,
        world: &mut RuntimeWorld,
        agent_id: &str,
        reason: &str,
    ) -> Result<(), String> {
        let Some(context) = self.provider_contexts.get(agent_id) else {
            return Ok(());
        };
        super::async_support::runtime_provider_failure(world, context, reason)
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
        let reject_reason = reject_reason.into();
        let feedback = context.as_ref().map(|context| {
            self.provider_feedback_for_request(
                &context.request_context,
                None,
                status,
                None,
                None,
                Some(reject_reason.clone()),
            )
        });
        if let Some(context) = context.as_ref() {
            self.record_provider_terminal_state(
                agent_id,
                context,
                status,
                Some(reject_reason.clone()),
                feedback
                    .as_ref()
                    .map(|feedback| feedback.feedback_id.clone()),
            );
        }
        if reject_reason != "stale_base" {
            self.clear_provider_stale_replans(agent_id);
        }
        self.provider_held_decisions.remove(agent_id);
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

    pub(in crate::viewer::runtime_live) fn provider_feedback_for_request(
        &mut self,
        request: &crate::simulator::ContinuousAgentRequestContextV1,
        candidate_action_id: Option<u64>,
        status: &str,
        runtime_receipt_id: Option<String>,
        feedback_id: Option<String>,
        reject_reason: Option<String>,
    ) -> FeedbackEnvelopeV1 {
        let session_key = lineage::provider_feedback_session_key(
            request.agent_subject.as_str(),
            request.agent_session_id.as_str(),
        );
        let feedback_seq = self
            .provider_feedback_seq_by_session
            .entry(session_key)
            // Compatibility feedback cursors are intentionally scoped to the
            // Runtime Agent session. The legacy per-agent cursor remains a
            // high-water mark for migration/observability, but must never
            // seed a fresh session and leak sequence state across sessions.
            .or_insert(1);
        let sequence = (*feedback_seq).max(1);
        *feedback_seq = sequence.saturating_add(1);
        self.provider_feedback_seq
            .entry(request.agent_subject.clone())
            .and_modify(|current| *current = (*current).max(sequence.saturating_add(1)))
            .or_insert(sequence.saturating_add(1));
        let feedback_id = feedback_id.unwrap_or_else(|| {
            format!(
                "runtime-feedback:{}:{}:{}",
                request.agent_subject, request.agent_session_id, sequence
            )
        });
        let feedback = FeedbackEnvelopeV1 {
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
        };
        if matches!(status, "committed" | "rejected" | "failed") {
            if let Some(context) = self
                .provider_contexts
                .get(request.agent_subject.as_str())
                .cloned()
            {
                self.record_provider_terminal_state(
                    request.agent_subject.as_str(),
                    &context,
                    status,
                    feedback.reject_reason.clone(),
                    Some(feedback.feedback_id.clone()),
                );
            }
        }
        self.persist_provider_lineage_best_effort();
        feedback
    }

    /// Deliver the Runtime-committed feedback through the same provider
    /// transport used for decisions. The receipt lineage has already been
    /// read back and verified by Runtime before this method is called.
    pub(in crate::viewer::runtime_live) fn deliver_provider_cognition_feedback(
        &self,
        payload: &JsonValue,
    ) -> Result<(), String> {
        if matches!(self.runner, Some(RuntimeDecisionRunner::Builtin(_))) {
            // Builtin responses are already inside the host Harness. Runtime
            // feedback is still allocated/acked for one shared lifecycle, but
            // there is no external provider endpoint to call.
            return Ok(());
        }
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
            .submit_feedback_context_payload(payload)
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
        self.provider_held_decisions.remove(agent_id);
        self.persist_provider_lineage_best_effort();
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::viewer::runtime_live) fn expire_async_runtime_turn(
        &mut self,
        context: &cognition_context::ProviderContextState,
    ) {
        if let Some(runner) = self
            .runner
            .as_mut()
            .and_then(RuntimeDecisionRunner::async_runner_mut)
        {
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
        self.persist_provider_lineage_best_effort();
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
                    "rejected",
                    None,
                    None,
                    Some("no_effect".to_string()),
                );
                world.enqueue_runtime_feedback(feedback).map_err(|error| {
                    format!("Runtime feedback outbox enqueue failed after provider wait: {error:?}")
                })?;
                self.record_provider_terminal_state(
                    agent_id.as_str(),
                    &context,
                    "rejected",
                    Some("no_effect".to_string()),
                    None,
                );
                self.clear_provider_stale_replans(agent_id.as_str());
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
