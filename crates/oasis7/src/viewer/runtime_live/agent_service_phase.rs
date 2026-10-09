//! Hosted canonical Act transitions. Original durable maps survive worker loss.
use super::agent_service_io::*;
use super::control_plane::llm_sidecar::lineage_persistence::PendingProviderServiceIntent;
use super::control_plane::provider_action_commit::{
    ProviderServiceCognitionReceipt, decode_provider_service_receipt,
    validate_provider_service_receipt,
};
use super::control_plane::simulator_action_to_runtime;
use crate::world_service::{verified_view::VerifiedWorldView, wire::SchedulerOperationV1};
use oasis7_client_api::world_service::*;

#[derive(Clone)]
pub(super) enum HostedServicePhase {
    Act {
        pending: PendingProviderServiceIntent,
        submit: bool,
    },
    ActView {
        pending: PendingProviderServiceIntent,
        receipt: ProviderServiceCognitionReceipt,
        commit: CommitRef,
        settled: bool,
    },
    ActSettle {
        pending: PendingProviderServiceIntent,
        receipt: ProviderServiceCognitionReceipt,
        submit: bool,
    },
    Feedback {
        pending: PendingProviderServiceIntent,
        receipt: ProviderServiceCognitionReceipt,
    },
    FeedbackAck {
        pending: PendingProviderServiceIntent,
        receipt: ProviderServiceCognitionReceipt,
        submit: bool,
    },
    FeedbackAckView {
        pending: PendingProviderServiceIntent,
        receipt: ProviderServiceCognitionReceipt,
        commit: CommitRef,
    },
    Finalize {
        pending: PendingProviderServiceIntent,
        receipt: ProviderServiceCognitionReceipt,
        feedback_acked: bool,
    },
}
impl HostedServicePhase {
    #[cfg(any(test, feature = "test_tier_required"))]
    pub(super) fn test_phase_label(&self) -> &'static str {
        match self {
            Self::Act { .. } => "act",
            Self::ActView { settled: false, .. } => "act_view",
            Self::ActView { settled: true, .. } => "act_settled_view",
            Self::ActSettle { .. } => "act_settle",
            Self::Feedback { .. } => "feedback",
            Self::FeedbackAck { .. } => "feedback_ack",
            Self::FeedbackAckView { .. } => "feedback_ack_view",
            Self::Finalize { .. } => "finalize",
        }
    }
    fn identity(&self) -> String {
        let pending = match self {
            Self::Act { pending, .. }
            | Self::ActView { pending, .. }
            | Self::ActSettle { pending, .. }
            | Self::Feedback { pending, .. }
            | Self::FeedbackAck { pending, .. }
            | Self::FeedbackAckView { pending, .. }
            | Self::Finalize { pending, .. } => pending,
        };
        let phase = match self {
            Self::Act { .. } => "act",
            Self::ActView { settled: true, .. } => "act_settled_view",
            Self::ActView { .. } => "act_view",
            Self::ActSettle { .. } => "act_settle",
            Self::Feedback { .. } => "feedback",
            Self::FeedbackAck { .. } => "feedback_ack",
            Self::FeedbackAckView { .. } => "feedback_ack_view",
            Self::Finalize { .. } => "finalize",
        };
        format!(
            "{phase}:{}",
            pending
                .cognition
                .request
                .request_context
                .provider_invocation_key()
        )
    }
}
impl crate::viewer::ViewerRuntimeLiveServer {
    pub(super) fn hosted_service_config_digest(&self) -> Result<String, String> {
        let config = self
            .config
            .world_service
            .as_ref()
            .ok_or("canonical service configuration missing")?;
        let signer = self
            .config
            .world_service_agent_signer
            .as_ref()
            .ok_or("canonical Agent signer missing")?;
        // Private fence: never expose the credential-bearing source or this token.
        let material = format!(
            "{}|{}|{:?}|{}|{}|{:?}|{}|{}|{}",
            config.endpoint,
            config.trusted_service_public_key,
            config.expected_world,
            config.scope_id,
            config.read_private_key_hex,
            config.timeout,
            config.max_response_bytes,
            signer.private_key_hex,
            signer.delegation_generation
        );
        Ok(blake3::hash(material.as_bytes()).to_hex().to_string())
    }
    pub(super) fn prepare_hosted_service_io(
        &mut self,
        _eligible: bool,
    ) -> Result<AgentServiceProgress, String> {
        if self.config.world_service.is_none() {
            return Ok(AgentServiceProgress::Idle);
        }
        self.configure_service_provider();
        if self.config.world_service_agent_signer.is_none()
            && self.llm_sidecar.provider_service_pending.is_empty()
            && self.llm_sidecar.hosted_service_phase.is_none()
        {
            return Ok(AgentServiceProgress::Idle);
        }
        let current_config = self.hosted_service_config_digest()?;
        if self
            .llm_sidecar
            .hosted_service_config_binding
            .as_ref()
            .is_some_and(|original| original != &current_config)
        {
            return Err("hosted service original configuration changed; checkpoint fenced".into());
        }
        if self.llm_sidecar.hosted_wait.is_some() {
            return self.prepare_hosted_wait_io();
        }
        if self.llm_sidecar.hosted_resume.is_some() {
            return self.prepare_hosted_resume_io(_eligible);
        }
        if self.llm_sidecar.hosted_service_phase.is_none()
            && self.llm_sidecar.provider_service_pending.is_empty()
            && self.llm_sidecar.hosted_admission.is_none()
        {
            let resume = self.prepare_hosted_resume_io(_eligible)?;
            if !matches!(resume, AgentServiceProgress::Idle) {
                return Ok(resume);
            }
        }
        if self.llm_sidecar.hosted_service_inflight.is_some() {
            return Ok(AgentServiceProgress::Idle);
        }
        if self.llm_sidecar.hosted_service_phase.is_none() {
            if let Some(pending) = self
                .llm_sidecar
                .provider_service_pending
                .values()
                .next()
                .cloned()
            {
                self.llm_sidecar
                    .validate_hosted_restored_original(&self.world, &pending)?;
                let crate::world_service::wire::WorldServicePayloadV1::Cognition(signed) =
                    &pending.payload
                else {
                    return Err("hosted recovery operation mismatch".into());
                };
                self.prepare_provider_service_action_checkpoint(
                    &signed.request.action,
                    &pending.cognition,
                    pending.action.clone(),
                )
                .map_err(|error| format!("{error:?}"))?;
                // Existing durable intent is always recovered by original Lookup.
                self.llm_sidecar.hosted_service_phase = Some(HostedServicePhase::Act {
                    pending,
                    submit: false,
                });
            } else {
                if let Some(decision) = self
                    .llm_sidecar
                    .poll_hosted_provider_decision(&self.world)?
                {
                    if matches!(
                        decision.decision,
                        crate::simulator::AgentDecision::Wait
                            | crate::simulator::AgentDecision::WaitTicks(_)
                    ) {
                        self.llm_sidecar.capture_hosted_wait(decision)?;
                        return Ok(AgentServiceProgress::Advanced);
                    }
                    let crate::simulator::AgentDecision::Act(action) = decision.decision else {
                        return Err("hosted provider decision has no supported phase".into());
                    };
                    let cognition = decision.cognition.ok_or("hosted Act cognition missing")?;
                    let runtime_action = simulator_action_to_runtime(&action, &self.world)
                        .ok_or("hosted Agent action has no registered codec")?;
                    let (pending, existed) = self
                        .prepare_provider_service_action_checkpoint(
                            &runtime_action,
                            &cognition,
                            action,
                        )
                        .map_err(|error| format!("{error:?}"))?;
                    self.llm_sidecar
                        .retire_hosted_provider_outcome(&cognition.request.request_context)?;
                    self.llm_sidecar.hosted_service_phase = Some(HostedServicePhase::Act {
                        pending,
                        submit: !existed,
                    });
                }
            }
        }
        let Some(phase) = self.llm_sidecar.hosted_service_phase.clone() else {
            return Ok(AgentServiceProgress::Idle);
        };
        self.llm_sidecar
            .hosted_service_config_binding
            .get_or_insert(current_config);
        if let HostedServicePhase::Finalize {
            pending,
            receipt,
            feedback_acked,
        } = phase
        {
            if pending.feedback_ack.is_none() {
                self.llm_sidecar.checkpoint_service_feedback_consumption(
                    &self.world,
                    &pending,
                    &receipt.feedback,
                    &receipt.lineage,
                    receipt.raw_feedback.clone(),
                )?;
            }
            let pending = self
                .llm_sidecar
                .provider_service_pending
                .get(&pending.cognition.request.request_context.agent_subject)
                .cloned()
                .ok_or("feedback pending missing")?;
            self.llm_sidecar
                .validate_pending_feedback_consumption(&pending)?;
            if feedback_acked {
                self.finalize_hosted_service_act(pending, receipt)?;
                self.llm_sidecar.hosted_service_phase = None;
                self.llm_sidecar.hosted_service_config_binding = None;
            } else {
                // Delivery is repeated idempotently after recovery; a persisted
                // unsigned phase bit is not proof of provider acceptance.
                self.llm_sidecar.hosted_service_phase =
                    Some(HostedServicePhase::Feedback { pending, receipt });
            }
            return Ok(AgentServiceProgress::Advanced);
        }
        let mut client = self
            .world_service_client()
            .map_err(|error| format!("{error:?}"))?
            .ok_or("canonical service transport missing")?;
        if let HostedServicePhase::FeedbackAckView { pending, .. } = &phase {
            self.llm_sidecar
                .validate_hosted_restored_original(&self.world, pending)?;
            let crate::world_service::WorldServicePayloadV1::Cognition(signed) = &pending.payload
            else {
                return Err("ACK View original Cognition missing".into());
            };
            let agent = &signed.request.request.agent_id;
            if agent != &pending.cognition.request.request_context.agent_subject {
                return Err("ACK View original Agent mismatch".into());
            }
            let mut config = client.config().clone();
            config.scope_id = format!("agent:{agent}");
            client = crate::world_service::client::RemoteWorldServiceClient::new(config)
                .map_err(|error| error.to_string())?
                .with_query_state(self.world_service_query_state.clone());
        }
        let operation = match &phase {
            HostedServicePhase::Act { pending, submit } => {
                original_intent_io(&pending.correlation, &pending.payload, *submit)
            }
            HostedServicePhase::ActView {
                pending, commit, ..
            } => AgentServiceIoOperation::View(ReadWorldViewRequest {
                contract_version: WORLD_SERVICE_CONTRACT_VERSION,
                world: pending.correlation.key.world.clone(),
                scope_id: client.config().scope_id.clone(),
                min_commit: Some(commit.clone()),
                fixed_commit: None,
                deadline_unix_ms: None,
            }),
            HostedServicePhase::ActSettle {
                pending, submit, ..
            } => {
                let request = &pending.cognition.request.request_context;
                let lease = pending
                    .cognition
                    .cognition_lease
                    .as_ref()
                    .ok_or("hosted settled Act missing lease")?;
                let (checkpoint, existed) = self.llm_sidecar.prepare_service_scheduler_checkpoint(
                    request,
                    "settle",
                    SchedulerOperationV1::SettleLease {
                        lease_id: lease.lease_id.clone(),
                        consumed_amount: lease.reserved_amount,
                    },
                    None,
                )?;
                original_intent_io(
                    &checkpoint.correlation,
                    &checkpoint.payload,
                    *submit && !existed,
                )
            }
            HostedServicePhase::Feedback {
                pending,
                receipt: _,
            } => {
                let ack = pending
                    .feedback_ack
                    .as_ref()
                    .ok_or("feedback consumption missing")?;
                if self.llm_sidecar.service_feedback_is_builtin() {
                    self.mark_hosted_feedback_delivered(pending.clone())?;
                    return Ok(AgentServiceProgress::Advanced);
                }
                AgentServiceIoOperation::Feedback {
                    client: self.llm_sidecar.service_feedback_transport()?,
                    payload: ack.original_feedback.clone(),
                }
            }
            HostedServicePhase::FeedbackAck {
                pending, submit, ..
            } => {
                let ack = pending
                    .feedback_ack
                    .as_ref()
                    .ok_or("feedback ACK checkpoint missing")?;
                original_intent_io(&ack.correlation, &ack.payload, *submit)
            }
            HostedServicePhase::FeedbackAckView {
                pending, commit, ..
            } => AgentServiceIoOperation::View(ReadWorldViewRequest {
                contract_version: WORLD_SERVICE_CONTRACT_VERSION,
                world: pending.correlation.key.world.clone(),
                scope_id: client.config().scope_id.clone(),
                min_commit: Some(commit.clone()),
                fixed_commit: None,
                deadline_unix_ms: None,
            }),
            HostedServicePhase::Finalize { .. } => unreachable!(),
        };
        if let HostedServicePhase::FeedbackAck {
            pending,
            submit: true,
            ..
        } = &phase
        {
            let agent = &pending.cognition.request.request_context.agent_subject;
            let previous = self.llm_sidecar.provider_service_pending.clone();
            self.llm_sidecar
                .provider_service_pending
                .get_mut(agent)
                .ok_or("ACK original missing")?
                .feedback_ack
                .as_mut()
                .ok_or("ACK checkpoint missing")?
                .issued = true;
            if let Err(error) = self.llm_sidecar.persist_provider_lineage() {
                self.llm_sidecar.provider_service_pending = previous;
                return Err(error);
            }
        }
        self.llm_sidecar.hosted_service_generation =
            self.llm_sidecar.hosted_service_generation.saturating_add(1);
        let token = AgentServiceIoToken {
            generation: self.llm_sidecar.hosted_service_generation,
            config_digest: self.hosted_service_config_digest()?,
            phase_digest: phase.identity(),
        };
        self.llm_sidecar.hosted_service_inflight = Some(token.clone());
        // After dispatch reservation any worker failure must recover by Lookup.
        match self.llm_sidecar.hosted_service_phase.as_mut() {
            Some(HostedServicePhase::Act { submit, .. })
            | Some(HostedServicePhase::ActSettle { submit, .. })
            | Some(HostedServicePhase::FeedbackAck { submit, .. }) => *submit = false,
            _ => {}
        }
        Ok(AgentServiceProgress::NeedsIo(Box::new(AgentServiceIoJob {
            token,
            client: Some(client),
            operation,
        })))
    }
    pub(super) fn apply_hosted_service_io(
        &mut self,
        result: AgentServiceIoResult,
    ) -> Result<AgentServiceProgress, String> {
        if result.token.phase_digest.starts_with("wait:") {
            return self.apply_hosted_wait_io(result);
        }
        if result.token.phase_digest.starts_with("resume:") {
            return self.apply_hosted_resume_io(result);
        }
        if self.llm_sidecar.hosted_service_inflight.as_ref() != Some(&result.token) {
            return Err(
                "hosted service completion token mismatch; original intent retained".into(),
            );
        }
        self.llm_sidecar.hosted_service_inflight = None;
        if result.token.config_digest != self.hosted_service_config_digest()? {
            return Err("hosted service configuration changed; original intent fenced".into());
        }
        let phase = self
            .llm_sidecar
            .hosted_service_phase
            .clone()
            .ok_or("hosted service completion lacks original phase")?;
        if result.token.phase_digest != phase.identity() {
            return Err("hosted service phase identity mismatch".into());
        }
        let response = result.response?;
        match phase {
            HostedServicePhase::Act { pending, .. } => {
                let Some(response) = original_response(response)? else {
                    return Ok(AgentServiceProgress::Advanced);
                };
                response
                    .validate(&pending.correlation)
                    .map_err(|error| error.to_string())?;
                match response.outcome {
                    IntentOutcome::Committed { commit, receipt } => {
                        let receipt: ProviderServiceCognitionReceipt =
                            decode_provider_service_receipt(receipt)?;
                        validate_provider_service_receipt(&pending, &receipt)?;
                        self.llm_sidecar.hosted_service_phase = Some(HostedServicePhase::ActView {
                            pending,
                            receipt,
                            commit,
                            settled: false,
                        });
                    }
                    IntentOutcome::Received { .. } | IntentOutcome::Pending => {}
                    IntentOutcome::Rejected { reason } => {
                        return Err(format!("canonical Agent rejected: {reason:?}"));
                    }
                    _ => {
                        return Err(
                            "canonical Agent outcome cannot complete original intent".into()
                        );
                    }
                }
            }
            HostedServicePhase::ActView {
                pending,
                receipt,
                commit,
                settled,
            } => {
                let AgentServiceIoResponse::View(view) = response else {
                    return Err("hosted service expected verified minimum view".into());
                };
                if !view
                    .version()
                    .commit
                    .satisfies_minimum(&commit)
                    .map_err(|error| error.to_string())?
                {
                    return Err("hosted service view precedes committed Act".into());
                }
                self.apply_hosted_verified_view(*view)?;
                self.llm_sidecar.hosted_service_phase = Some(if settled {
                    HostedServicePhase::Finalize {
                        pending,
                        receipt,
                        feedback_acked: false,
                    }
                } else {
                    HostedServicePhase::ActSettle {
                        pending,
                        receipt,
                        submit: true,
                    }
                });
            }
            HostedServicePhase::ActSettle {
                pending, receipt, ..
            } => {
                let request = &pending.cognition.request.request_context;
                let checkpoint = self
                    .llm_sidecar
                    .provider_scheduler_pending
                    .get(&format!("{}:settle", request.provider_invocation_key()))
                    .ok_or("hosted settlement checkpoint missing")?;
                let Some(response) = original_response(response)? else {
                    return Ok(AgentServiceProgress::Advanced);
                };
                response
                    .validate(&checkpoint.correlation)
                    .map_err(|error| error.to_string())?;
                match response.outcome {
                    IntentOutcome::Committed {
                        receipt: settled,
                        commit,
                    } => {
                        super::control_plane::RuntimeLlmSidecar::validate_hosted_settlement_receipt(request,pending.cognition.cognition_lease.as_ref().ok_or("hosted settlement lease missing")?,settled)?;
                        self.llm_sidecar.hosted_service_phase = Some(HostedServicePhase::ActView {
                            pending,
                            receipt,
                            commit,
                            settled: true,
                        });
                    }
                    IntentOutcome::Received { .. } | IntentOutcome::Pending => {}
                    IntentOutcome::Rejected { reason } => {
                        return Err(format!("canonical settlement rejected: {reason:?}"));
                    }
                    _ => return Err("canonical settlement cannot complete original intent".into()),
                }
            }
            HostedServicePhase::Feedback { pending, .. } => {
                if !matches!(response, AgentServiceIoResponse::Feedback) {
                    return Err("feedback transport response mismatch".into());
                }
                self.mark_hosted_feedback_delivered(pending)?;
            }
            HostedServicePhase::FeedbackAck {
                pending, receipt, ..
            } => {
                let Some(response) = original_response(response)? else {
                    return Ok(AgentServiceProgress::Advanced);
                };
                let ack = pending
                    .feedback_ack
                    .as_ref()
                    .ok_or("ACK checkpoint missing")?;
                response
                    .validate(&ack.correlation)
                    .map_err(|e| e.to_string())?;
                match response.outcome {
                    IntentOutcome::Committed {
                        commit,
                        receipt: result,
                    } => {
                        let crate::world_service::WorldServicePayloadV1::FeedbackAck(signed) =
                            &ack.payload
                        else {
                            return Err("ACK codec mismatch".into());
                        };
                        if result["delivery_state"] != "acked"
                            || result["acknowledged"]
                                != serde_json::to_value(&signed.request)
                                    .map_err(|e| e.to_string())?
                        {
                            return Err("ACK receipt mismatch".into());
                        }
                        self.llm_sidecar.hosted_service_phase =
                            Some(HostedServicePhase::FeedbackAckView {
                                pending,
                                receipt,
                                commit,
                            });
                    }
                    IntentOutcome::Received { .. } | IntentOutcome::Pending => {}
                    IntentOutcome::Unknown => {
                        // Explicit absence permits replaying the exact signed
                        // key/payload after a pre-Submit crash; no new nonce.
                        self.llm_sidecar.hosted_service_phase =
                            Some(HostedServicePhase::FeedbackAck {
                                pending,
                                receipt,
                                submit: true,
                            });
                    }
                    other => return Err(format!("ACK original not completed: {other:?}")),
                }
            }
            HostedServicePhase::FeedbackAckView {
                pending,
                receipt,
                commit,
            } => {
                let AgentServiceIoResponse::View(view) = response else {
                    return Err("ACK minimum View missing".into());
                };
                if !view
                    .version()
                    .commit
                    .satisfies_minimum(&commit)
                    .map_err(|e| e.to_string())?
                {
                    return Err("ACK View too old".into());
                }
                let ack = pending
                    .feedback_ack
                    .as_ref()
                    .ok_or("ACK checkpoint missing")?;
                let crate::world_service::WorldServicePayloadV1::FeedbackAck(signed) = &ack.payload
                else {
                    return Err("ACK codec mismatch".into());
                };
                let agent = &pending.cognition.request.request_context.agent_subject;
                if signed.request.agent_id != *agent
                    || view.version().visibility_scope != format!("agent:{agent}")
                {
                    return Err("ACK View Agent scope mismatch".into());
                }
                let history = view
                    .projection()
                    .feedback_history
                    .as_ref()
                    .ok_or("ACK scoped history missing")?;
                if history.agent_id != *agent {
                    return Err("ACK history Agent mismatch".into());
                }
                history.metadata(agent)?;
                let record = history
                    .records
                    .iter()
                    .find(|record| record.feedback.feedback_id == signed.request.feedback_id)
                    .ok_or("ACK feedback missing")?;
                let feedback = &record.feedback;
                if record.delivery_state != "acked"
                    || record.original_envelope_digest != signed.request.original_envelope_digest
                    || feedback.agent_subject != signed.request.agent_id
                    || feedback.agent_session_id != signed.request.agent_session_id
                    || feedback.agent_turn_id != signed.request.agent_turn_id
                    || feedback.decision_request_id != signed.request.decision_request_id
                    || feedback.request_digest.to_string() != signed.request.request_digest
                    || feedback.feedback_seq != signed.request.feedback_seq
                {
                    return Err("ACK readback mismatch".into());
                }
                if let Some(current) = self.verified_world_view.as_ref()
                    && !view
                        .version()
                        .commit
                        .satisfies_minimum(&current.version().commit)
                        .map_err(|error| error.to_string())?
                {
                    return Err("ACK private View precedes current Viewer View".into());
                }
                if self
                    .config
                    .world_service
                    .as_ref()
                    .is_some_and(|config| config.scope_id == view.version().visibility_scope)
                {
                    self.apply_hosted_verified_view(*view)?;
                } else {
                    // The Agent history is private provider input, not a public
                    // Viewer cursor or a replacement for its verified snapshot.
                    self.llm_sidecar.provider_service_projection = Some(view.projection().clone());
                    self.llm_sidecar
                        .sync_shadow_kernel(&self.world, &self.snapshot_config)?;
                }
                self.llm_sidecar.hosted_service_phase = Some(HostedServicePhase::Finalize {
                    pending,
                    receipt,
                    feedback_acked: true,
                });
            }
            HostedServicePhase::Finalize { .. } => {
                return Err("hosted finalization has no transport result".into());
            }
        }
        Ok(AgentServiceProgress::Advanced)
    }
    pub(super) fn apply_hosted_verified_view(
        &mut self,
        view: VerifiedWorldView,
    ) -> Result<(), String> {
        if let Some(current) = self.verified_world_view.as_ref()
            && !view
                .version()
                .commit
                .satisfies_minimum(&current.version().commit)
                .map_err(|error| error.to_string())?
        {
            return Err("hosted service old view cannot replace newer view".into());
        }
        self.llm_sidecar.provider_service_projection = Some(view.projection().clone());
        self.verified_world_view = Some(view);
        self.llm_sidecar
            .sync_shadow_kernel(&self.world, &self.snapshot_config)
    }
    fn mark_hosted_feedback_delivered(
        &mut self,
        pending: PendingProviderServiceIntent,
    ) -> Result<(), String> {
        let agent = &pending.cognition.request.request_context.agent_subject;
        let previous = self.llm_sidecar.provider_service_pending.clone();
        self.llm_sidecar
            .provider_service_pending
            .get_mut(agent)
            .ok_or("feedback pending missing")?
            .feedback_ack
            .as_mut()
            .ok_or("consumption checkpoint missing")?
            .delivered = true;
        if let Err(error) = self.llm_sidecar.persist_provider_lineage() {
            self.llm_sidecar.provider_service_pending = previous;
            return Err(error);
        }
        let updated = self
            .llm_sidecar
            .provider_service_pending
            .get(agent)
            .cloned()
            .unwrap();
        let Some(HostedServicePhase::Feedback { receipt, .. }) =
            self.llm_sidecar.hosted_service_phase.clone()
        else {
            return Err("feedback phase changed".into());
        };
        let submit = !updated
            .feedback_ack
            .as_ref()
            .ok_or("ACK checkpoint missing")?
            .issued;
        self.llm_sidecar.hosted_service_phase = Some(HostedServicePhase::FeedbackAck {
            pending: updated,
            receipt,
            submit,
        });
        Ok(())
    }
    fn finalize_hosted_service_act(
        &mut self,
        pending: PendingProviderServiceIntent,
        receipt: ProviderServiceCognitionReceipt,
    ) -> Result<(), String> {
        let request = &pending.cognition.request.request_context;
        if self
            .llm_sidecar
            .provider_service_projection
            .as_ref()
            .is_some_and(|projection| {
                projection
                    .scheduler_wakes
                    .iter()
                    .any(|wake| wake.agent_id == request.agent_subject)
            })
        {
            return Err(
                "hosted wake terminal handoff remains pending; original Act retained".into(),
            );
        }
        let action_id = receipt
            .commit_record
            .action_id
            .strip_prefix("action:")
            .and_then(|id| id.parse::<u64>().ok())
            .ok_or("canonical action ID is not numeric")?;
        self.llm_sidecar.track_action(
            action_id,
            request.agent_subject.clone(),
            pending.action.clone(),
            Some(pending.cognition.clone()),
        );
        self.llm_sidecar.clear_settled_service_lease(
            request,
            pending
                .cognition
                .cognition_lease
                .as_ref()
                .ok_or("hosted Act lease missing")?,
        )?;
        self.llm_sidecar
            .finalize_provider_action_with_feedback_checked(action_id, receipt.feedback)?;
        self.llm_sidecar
            .provider_service_pending
            .remove(&request.agent_subject);
        if let Err(error) = self.llm_sidecar.persist_provider_lineage() {
            self.llm_sidecar
                .provider_service_pending
                .insert(request.agent_subject.clone(), pending);
            return Err(error);
        }
        self.llm_sidecar
            .provider_restored_service_checkpoints
            .remove(&request.agent_subject);
        self.defer_next_auto_play_step_after_completion(self.config.play_step_interval);
        Ok(())
    }
}
pub(super) fn original_intent_io(
    correlation: &RequestCorrelation,
    payload: &crate::world_service::WorldServicePayloadV1,
    submit: bool,
) -> AgentServiceIoOperation {
    if submit {
        AgentServiceIoOperation::Submit(SubmitIntentRequest {
            contract_version: WORLD_SERVICE_CONTRACT_VERSION,
            correlation: correlation.clone(),
            deadline_unix_ms: None,
            signed_payload: payload.clone(),
        })
    } else {
        AgentServiceIoOperation::Lookup {
            request: LookupIntentRequest {
                contract_version: WORLD_SERVICE_CONTRACT_VERSION,
                key: correlation.key.clone(),
            },
            original: payload.clone(),
        }
    }
}
pub(super) fn original_response(
    response: AgentServiceIoResponse,
) -> Result<Option<IntentResponse<serde_json::Value>>, String> {
    match response {
        AgentServiceIoResponse::Intent(response)
        | AgentServiceIoResponse::Submit(SubmitObservation::Response(response)) => {
            Ok(Some(response))
        }
        AgentServiceIoResponse::Submit(SubmitObservation::OutcomeUnknown(_)) => Ok(None),
        _ => Err("hosted service response operation mismatch".into()),
    }
}
