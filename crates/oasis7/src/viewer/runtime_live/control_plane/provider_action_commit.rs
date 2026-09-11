use super::llm_sidecar::RuntimeProviderActionContext;
use super::*;
use crate::runtime::{
    CognitionCommitRejectReasonV1, RuntimeCognitionBaseBindingV1, RuntimeCognitionCommitRequestV1,
    RuntimeCognitionResponseArtifactV1, RuntimeFeedbackProjectionV1, World as RuntimeWorld,
    classify_cognition_commit_error,
};
use crate::simulator::{Action as SimulatorAction, h_v1};

impl ViewerRuntimeLiveServer {
    /// Retry sidecar finalization from the committed Runtime receipt.
    pub(super) fn retry_committed_provider_action(&mut self) -> Result<(), String> {
        let Some((action_id, _agent_id, cognition)) =
            self.llm_sidecar.pending_provider_action_for_recovery()
        else {
            return Ok(());
        };
        let request = &cognition.request.request_context;
        let marker = self
            .world
            .cognition()
            .get("commit_records")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|value| {
                serde_json::from_value::<crate::runtime::WorldCommitRecordV1>(value.clone()).ok()
            })
            .find(|marker| {
                marker.status == "committed"
                    && marker.agent_id == request.agent_subject
                    && marker.agent_session_id == request.agent_session_id
                    && marker.agent_turn_id == request.agent_turn_id
                    && marker.decision_request_id == request.decision_request_id
                    && marker.request_digest == request.request_digest.to_string()
            });
        let Some(marker) = marker else {
            return Ok(());
        };
        let lineage = self
            .world
            .read_runtime_receipt_lineage(marker.receipt_id.as_str())
            .map_err(|error| {
                format!("Runtime cognition receipt recovery readback failed: {error:?}")
            })?;
        self.world
            .verify_runtime_receipt_lineage(&lineage)
            .map_err(|error| {
                format!("Runtime cognition receipt recovery verification failed: {error:?}")
            })?;
        self.settle_provider_cognition_lease_for_request(
            request.agent_subject.as_str(),
            cognition.cognition_lease.clone(),
            Some(request),
        )?;

        let existing_feedback = self
            .world
            .runtime_feedback_outbox()
            .map_err(|error| format!("Runtime feedback recovery outbox read failed: {error:?}"))?
            .into_iter()
            .find(|record| record.feedback_id == lineage.feedback_id);
        let feedback = if let Some(record) = existing_feedback {
            let payload = record
                .transport_payload()
                .map_err(|error| format!("Runtime feedback recovery payload invalid: {error}"))?;
            let feedback = serde_json::from_value::<crate::simulator::FeedbackEnvelopeV1>(payload)
                .map_err(|error| {
                    format!("Runtime feedback recovery payload decode failed: {error}")
                })?;
            if feedback.status != "committed"
                || feedback.runtime_receipt_id.as_deref() != Some(lineage.receipt_id.as_str())
                || feedback.agent_subject != lineage.agent_id
                || feedback.agent_session_id != lineage.agent_session_id
                || feedback.agent_turn_id != lineage.agent_turn_id
                || feedback.decision_request_id != lineage.decision_request_id
                || feedback.request_digest.to_string() != lineage.request_digest
            {
                return Err("Runtime feedback recovery identity mismatch".to_string());
            }
            feedback
        } else {
            let feedback = self.llm_sidecar.provider_feedback(
                &cognition,
                Some(action_id),
                "committed",
                Some(lineage.receipt_id.clone()),
                Some(lineage.feedback_id.clone()),
                None,
            );
            let projection = RuntimeFeedbackProjectionV1 {
                envelope_digest: Some(lineage.envelope_digest.clone()),
                emitted_events: Vec::new(),
                committed_event_summary: Some(format!(
                    "runtime_receipt_id={} action_id={}",
                    lineage.receipt_id, lineage.action_id
                )),
                world_delta_summary: None,
            };
            self.allocate_runtime_feedback(feedback, projection)
                .map_err(|error| format!("Runtime feedback recovery allocation failed: {error}"))?
                .transport_payload()
                .map_err(|error| format!("Runtime feedback recovery payload invalid: {error}"))
                .and_then(|payload| {
                    serde_json::from_value(payload).map_err(|error| {
                        format!("Runtime feedback recovery payload decode failed: {error}")
                    })
                })?
        };
        self.llm_sidecar
            .consume_provider_memory_after_receipt(
                request.agent_subject.as_str(),
                feedback.clone(),
                &lineage,
                cognition.memory_write_intents.as_slice(),
            )
            .map_err(|error| {
                format!("Runtime receipt recovery memory projection failed: {error}")
            })?;
        self.llm_sidecar
            .finalize_provider_action_with_feedback_checked(action_id, feedback)
            .map_err(|error| format!("Runtime receipt recovery finalization failed: {error}"))?;
        self.drain_provider_feedback_outbox();
        if let Err(error) = self.handoff_runtime_wake_for_agent(
            request.agent_subject.as_str(),
            crate::runtime::ContinuationStatusV1::Completed,
            "provider_action_committed",
        ) {
            return Err(format!(
                "Runtime receipt recovery wake handoff failed: {error}"
            ));
        }
        Ok(())
    }

    pub(super) fn commit_provider_runtime_action(
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
        let action_id = committed
            .action_id
            .strip_prefix("action:")
            .ok_or_else(|| {
                ProviderRuntimeActionCommitError::PostCommit(
                    "Runtime cognition commit returned an invalid action id".to_string(),
                )
            })?
            .parse::<u64>()
            .map_err(|error| {
                ProviderRuntimeActionCommitError::PostCommit(format!(
                    "Runtime cognition action id is not numeric: {error}"
                ))
            })?;
        self.llm_sidecar.track_action(
            action_id,
            cognition.request.request_context.agent_subject.clone(),
            simulator_action,
            Some(cognition.clone()),
        );
        // The Runtime commit is authoritative. Persist the recovery record
        // before settling so an economy persistence fault can retry the same
        // idempotent operation after restart.
        self.settle_provider_cognition_lease_for_request(
            cognition.request.request_context.agent_subject.as_str(),
            cognition.cognition_lease.clone(),
            Some(&cognition.request.request_context),
        )
        .map_err(ProviderRuntimeActionCommitError::PostCommit)?;
        let lineage = self
            .world
            .read_runtime_receipt_lineage(returned_lineage.receipt_id.as_str())
            .map_err(|error| {
                ProviderRuntimeActionCommitError::PostCommit(format!(
                    "Runtime cognition receipt readback failed: {error:?}"
                ))
            })?;
        self.world
            .verify_runtime_receipt_lineage(&lineage)
            .map_err(|error| {
                ProviderRuntimeActionCommitError::PostCommit(format!(
                    "Runtime cognition receipt verification failed: {error:?}"
                ))
            })?;
        if lineage != returned_lineage || lineage.receipt_id != committed.receipt_id {
            return Err(ProviderRuntimeActionCommitError::PostCommit(
                "Runtime cognition receipt readback identity mismatch".to_string(),
            ));
        }
        self.llm_sidecar
            .clear_provider_stale_replans(cognition.request.request_context.agent_subject.as_str());
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
                ProviderRuntimeActionCommitError::PostCommit(format!(
                    "Runtime receipt committed but feedback outbox allocation failed: {error}"
                ))
            })?;
        let feedback = queued_feedback
            .transport_payload()
            .ok()
            .and_then(|payload| serde_json::from_value(payload).ok())
            .unwrap_or(feedback);
        self.llm_sidecar
            .consume_provider_memory_after_receipt(
                cognition.request.request_context.agent_subject.as_str(),
                feedback.clone(),
                &lineage,
                cognition.memory_write_intents.as_slice(),
            )
            .map_err(|error| {
                ProviderRuntimeActionCommitError::PostCommit(format!(
                    "Runtime receipt committed but provider memory projection failed: {error}"
                ))
            })?;

        #[cfg(test)]
        if std::env::var("OASIS7_TEST_PROVIDER_ACTION_FAULT").as_deref() == Ok("persistence") {
            let fault_result = self
                .llm_sidecar
                .install_test_provider_lineage_checkpoint_blocker();
            fault_result.map_err(|error| {
                ProviderRuntimeActionCommitError::PostCommit(format!(
                    "provider action finalization checkpoint fault setup failed: {error}"
                ))
            })?;
        }
        self.llm_sidecar
            .finalize_provider_action_with_feedback_checked(action_id, feedback)
            .map_err(|error| {
                ProviderRuntimeActionCommitError::PostCommit(format!(
                    "provider cognition receipt finalization remains pending: {error}"
                ))
            })?;
        self.drain_provider_feedback_outbox();
        if let Err(error) = self.handoff_runtime_wake_for_agent(
            cognition.request.request_context.agent_subject.as_str(),
            crate::runtime::ContinuationStatusV1::Completed,
            "provider_action_committed",
        ) {
            self.llm_sidecar.retain_provider_wake_recovery_pending(
                cognition.request.request_context.agent_subject.as_str(),
                &cognition.request,
                crate::runtime::ContinuationStatusV1::Completed,
                "provider_action_committed",
            );
            return Err(ProviderRuntimeActionCommitError::WakeHandoff(format!(
                "Runtime wake handoff failed: {error}"
            )));
        }
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

pub(super) enum ProviderRuntimeActionCommitError {
    StaleBase,
    WakeHandoff(String),
    PostCommit(String),
    Message(String),
}

impl ProviderRuntimeActionCommitError {
    pub(super) fn is_stale_base(&self) -> bool {
        matches!(self, Self::StaleBase)
    }

    pub(super) fn is_wake_handoff(&self) -> bool {
        matches!(self, Self::WakeHandoff(_))
    }

    pub(super) fn is_post_commit(&self) -> bool {
        matches!(self, Self::PostCommit(_))
    }

    pub(super) fn reason(&self) -> String {
        match self {
            Self::StaleBase => CognitionCommitRejectReasonV1::StaleBase.code().to_string(),
            Self::WakeHandoff(reason) => reason.clone(),
            Self::PostCommit(reason) => reason.clone(),
            Self::Message(reason) => reason.clone(),
        }
    }
}
