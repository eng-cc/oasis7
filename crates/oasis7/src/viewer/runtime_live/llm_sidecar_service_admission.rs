//! Durable fresh admission. No transport or actor callback is performed by apply.
use super::*;
use crate::viewer::runtime_live::agent_service_io::*;
use crate::world_service::client::WorldServicePort;
use oasis7_client_api::world_service::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(in crate::viewer::runtime_live) struct HostedAdmission {
    context: ProviderContextState,
    observation: Observation,
    metadata_identity: String,
    stage: String,
    commit: Option<CommitRef>,
    lease: Option<crate::runtime::CognitionLeaseV1>,
}
impl RuntimeLlmSidecar {
    pub(in crate::viewer::runtime_live) fn install_resumed_hosted_admission(
        &mut self,
        context: ProviderContextState,
        observation: Observation,
        metadata_identity: String,
    ) -> Result<(), String> {
        if self.hosted_admission.is_some() {
            return Err("resumed admission conflicts with existing original".into());
        }
        self.hosted_admission = Some(HostedAdmission {
            context,
            observation,
            metadata_identity,
            stage: "reserve".into(),
            commit: None,
            lease: None,
        });
        Ok(())
    }
    pub(in crate::viewer::runtime_live) fn restore_hosted_admission_original(
        &mut self,
    ) -> Result<(), String> {
        let Some(mut admission) = self.hosted_admission.clone() else {
            return Ok(());
        };
        let request = &admission.context.request_context;
        request
            .validate_production_lane()
            .map_err(|error| error.to_string())?;
        admission
            .context
            .turn_context
            .validate_for_agent(&request.agent_subject)
            .map_err(|error| error.to_string())?;
        if admission.context.turn_context.request_digest != request.request_digest
            || admission.observation.agent_id != request.agent_subject
            || admission.observation.time != request.runtime_binding.base_tick
        {
            return Err("restored fresh admission context identity mismatch".into());
        }
        if !matches!(
            admission.stage.as_str(),
            "reserve" | "reserve_view" | "prefix" | "prefix_view" | "dispatch"
        ) {
            return Err("restored fresh admission stage invalid".into());
        }
        // A private phase label is not canonical completion proof. Reconcile
        // the original signed Reserve and Prefix through Lookup after restart.
        admission.stage = "reserve".into();
        admission.commit = None;
        self.hosted_admission = Some(admission);
        Ok(())
    }
    pub(in crate::viewer::runtime_live) fn capture_hosted_admission(
        &mut self,
    ) -> Result<(), String> {
        self.ensure_canonical_agent_durable_admission()?;
        let metadata_identity = self
            .fresh_provider_metadata_identity()?
            .ok_or("fresh provider metadata is not ready")?;
        let settings = provider_settings_from_env()?
            .ok_or("fresh service admission requires configured provider")?;
        let view = self
            .provider_service_projection
            .as_ref()
            .ok_or("fresh signed view missing")?;
        let authority = view
            .agent_context
            .as_ref()
            .ok_or("fresh Agent authority missing")?;
        let agent = authority.agent_id.clone();
        if self.provider_active_turns.contains_key(&agent) {
            return Ok(());
        }
        let binding = view
            .runtime_binding
            .clone()
            .ok_or("fresh canonical binding missing")?;
        let session = authority
            .capability_invocation_context
            .presenter
            .session_id
            .clone()
            .ok_or("fresh canonical session missing")?;
        let capability = ProviderCapabilityContext {
            catalog: authority.capability_catalog.clone(),
            invocation: authority.capability_invocation_context.clone(),
            session_id: session.clone(),
        };
        let observation = self
            .shadow_kernel
            .as_mut()
            .ok_or("fresh observation kernel missing")?
            .observe(&agent)
            .map_err(|error| format!("{error:?}"))?;
        let sequence = self
            .provider_context_seq
            .get(&agent)
            .copied()
            .unwrap_or(1)
            .max(1);
        let (turn_context, request_context) = build_provider_context(ProviderContextInput {
            session_id: &session,
            sequence,
            agent_id: &agent,
            observation: observation.clone(),
            settings: &settings,
            runtime_binding: binding,
            recent_event_summary: &[],
            capability_context: capability,
            replan_cause: None,
            continuation: None,
            memory_store: &self.provider_memory_store,
            goal_snapshot: trusted_provider_goal_snapshot(self.prompt_profiles.get(&agent))?,
        })?;
        self.hosted_admission = Some(HostedAdmission {
            context: ProviderContextState {
                turn_context,
                request_context,
            },
            observation,
            metadata_identity,
            stage: "reserve".into(),
            commit: None,
            lease: None,
        });
        self.provider_context_seq
            .insert(agent, sequence.saturating_add(1));
        self.persist_provider_lineage()?;
        Ok(())
    }
    pub(in crate::viewer::runtime_live) fn hosted_admission_job(
        &mut self,
        world: &RuntimeWorld,
    ) -> Result<Option<AgentServiceIoOperation>, String> {
        let Some(admission) = self.hosted_admission.clone() else {
            return Ok(None);
        };
        self.persist_provider_lineage()?;
        let request = &admission.context.request_context;
        request
            .validate_production_lane()
            .map_err(|error| error.to_string())?;
        admission
            .context
            .turn_context
            .validate_for_agent(&request.agent_subject)
            .map_err(|error| error.to_string())?;
        if admission.stage.ends_with("_view") {
            let config = self
                .provider_service_config
                .as_ref()
                .ok_or("canonical configuration missing")?;
            return Ok(Some(AgentServiceIoOperation::View(ReadWorldViewRequest {
                contract_version: WORLD_SERVICE_CONTRACT_VERSION,
                world: config.expected_world.clone(),
                scope_id: config.scope_id.clone(),
                min_commit: admission.commit,
                fixed_commit: None,
                deadline_unix_ms: None,
            })));
        }
        if admission.stage == "dispatch" {
            return Ok(None);
        };
        let (phase, operation) = if admission.stage == "reserve" {
            (
                "reserve".into(),
                crate::world_service::wire::SchedulerOperationV1::ReserveLease(
                    async_support::provider_cognition_lease_request(
                        world,
                        &admission.context,
                        true,
                    )?,
                ),
            )
        } else if admission.stage == "prefix" {
            (
                format!("prefix:{}", request.transport_attempt),
                crate::world_service::wire::SchedulerOperationV1::ProviderPrefix {
                    request: request.clone(),
                    context_digest: async_support::runtime_provider_context_digest(request),
                },
            )
        } else {
            return Err("unknown durable admission stage".into());
        };
        let (pending, existed) =
            self.prepare_service_scheduler_checkpoint(request, &phase, operation, None)?;
        Ok(Some(if existed {
            AgentServiceIoOperation::Lookup {
                request: LookupIntentRequest {
                    contract_version: WORLD_SERVICE_CONTRACT_VERSION,
                    key: pending.correlation.key,
                },
                original: pending.payload,
            }
        } else {
            AgentServiceIoOperation::Submit(SubmitIntentRequest {
                contract_version: WORLD_SERVICE_CONTRACT_VERSION,
                correlation: pending.correlation,
                deadline_unix_ms: None,
                signed_payload: pending.payload,
            })
        }))
    }
    pub(in crate::viewer::runtime_live) fn dispatch_hosted_admission(
        &mut self,
    ) -> Result<bool, String> {
        let Some(admission) = self
            .hosted_admission
            .clone()
            .filter(|admission| admission.stage == "dispatch")
        else {
            return Ok(false);
        };
        let Some(identity) = self.fresh_provider_metadata_identity()? else {
            return Ok(false);
        };
        if identity != admission.metadata_identity {
            return Err("fresh provider configuration changed; original admission fenced".into());
        };
        let context = admission.context;
        let agent = context.request_context.agent_subject.clone();
        let lease = admission.lease.ok_or("fresh reserved lease missing")?;
        lineage_generation_recovery::validate_provider_lease_identity(
            &agent,
            &context.request_context,
            &lease,
        )?;
        if self.provider_active_turns.contains_key(&agent) {
            return Err("fresh active marker requires recovery; redispatch fenced".into());
        };
        self.provider_contexts
            .insert(agent.clone(), context.clone());
        self.provider_active_turns
            .insert(agent.clone(), context.clone());
        self.bind_provider_cognition_lease(agent.clone(), lease.clone());
        self.persist_provider_lineage()?;
        let observation = admission.observation;
        let runner = self
            .runner
            .as_mut()
            .and_then(RuntimeDecisionRunner::async_runner_mut)
            .ok_or("fresh native runner missing")?;
        runner.sync_logical_tick(context.request_context.runtime_binding.base_tick);
        runner
            .start_turn_with_request_context_and_observation_and_lease(
                &agent,
                observation,
                context.turn_context,
                context.request_context,
                lease,
            )
            .map_err(|error| format!("{error:?}"))?;
        self.hosted_admission = None;
        self.persist_provider_lineage()?;
        Ok(true)
    }
}
impl crate::viewer::ViewerRuntimeLiveServer {
    pub(in crate::viewer::runtime_live) fn prepare_fresh_service_io(
        &mut self,
        eligible: bool,
    ) -> Result<AgentServiceProgress, String> {
        if self.config.world_service.is_none() || self.llm_sidecar.hosted_service_inflight.is_some()
        {
            return Ok(AgentServiceProgress::Idle);
        }
        if self.llm_sidecar.hosted_admission.is_none() {
            if !eligible
                || self
                    .llm_sidecar
                    .fresh_provider_metadata_identity()?
                    .is_none()
            {
                return Ok(AgentServiceProgress::Idle);
            }
            if !self.llm_sidecar.provider_active_turns.is_empty()
                || !self.llm_sidecar.pending_runtime_wakes.is_empty()
                || !self.llm_sidecar.provider_continuation_proposals.is_empty()
                || !self.llm_sidecar.provider_recovery_pending.is_empty()
                || !self
                    .llm_sidecar
                    .provider_continuation_recovery_pending
                    .is_empty()
                || !self.llm_sidecar.provider_retry_contexts.is_empty()
                || self
                    .llm_sidecar
                    .provider_service_projection
                    .as_ref()
                    .is_some_and(|view| {
                        self.llm_sidecar
                            .provider_wait_until
                            .values()
                            .any(|until| *until > view.state.time)
                    })
            {
                return Ok(AgentServiceProgress::Idle);
            }
            if !self.llm_sidecar.hosted_fresh_view_ready && !self.should_advance_auto_play_step() {
                return Ok(AgentServiceProgress::Idle);
            }
            self.config.ensure_service_agent_lineage_store()?;
            if self.llm_sidecar.hosted_fresh_view_ready {
                self.llm_sidecar.capture_hosted_admission()?;
                self.llm_sidecar.hosted_fresh_view_ready = false;
            }
        } else if self
            .llm_sidecar
            .hosted_admission
            .as_ref()
            .is_some_and(|admission| admission.stage == "dispatch")
        {
            if !eligible {
                return Ok(AgentServiceProgress::Idle);
            }
            return Ok(if self.llm_sidecar.dispatch_hosted_admission()? {
                AgentServiceProgress::Advanced
            } else {
                AgentServiceProgress::Idle
            });
        }
        if let Some(admission) = self.llm_sidecar.hosted_admission.as_ref() {
            if matches!(admission.stage.as_str(), "reserve" | "prefix") {
                let request = &admission.context.request_context;
                let phase = if admission.stage == "reserve" {
                    "reserve".into()
                } else {
                    format!("prefix:{}", request.transport_attempt)
                };
                let issued = self
                    .llm_sidecar
                    .provider_scheduler_pending
                    .contains_key(&format!("{}:{phase}", request.provider_invocation_key()));
                if !issued {
                    if !eligible {
                        return Ok(AgentServiceProgress::Idle);
                    }
                    let Some(identity) = self.llm_sidecar.fresh_provider_metadata_identity()?
                    else {
                        return Ok(AgentServiceProgress::Idle);
                    };
                    if identity != admission.metadata_identity {
                        return Err(
                            "fresh provider configuration changed; original admission fenced"
                                .into(),
                        );
                    }
                }
            }
        }
        let config_digest = self.hosted_service_config_digest()?;
        if self
            .llm_sidecar
            .hosted_service_config_binding
            .as_ref()
            .is_some_and(|original| original != &config_digest)
        {
            return Err("fresh service original configuration changed; fenced".into());
        }
        let client = self
            .world_service_client()
            .map_err(|error| format!("{error:?}"))?
            .ok_or("canonical service missing")?;
        let operation = if self.llm_sidecar.hosted_admission.is_some() {
            self.llm_sidecar
                .hosted_admission_job(&self.world)?
                .ok_or("fresh admission stage has no operation")?
        } else {
            AgentServiceIoOperation::View(ReadWorldViewRequest {
                contract_version: WORLD_SERVICE_CONTRACT_VERSION,
                world: client.config().expected_world.clone(),
                scope_id: client.config().scope_id.clone(),
                min_commit: None,
                fixed_commit: None,
                deadline_unix_ms: None,
            })
        };
        let phase_digest = format!(
            "admission:{}",
            self.llm_sidecar
                .hosted_admission
                .as_ref()
                .map(|admission| admission.stage.as_str())
                .unwrap_or("view")
        );
        self.llm_sidecar.hosted_service_generation =
            self.llm_sidecar.hosted_service_generation.saturating_add(1);
        let token = AgentServiceIoToken {
            generation: self.llm_sidecar.hosted_service_generation,
            config_digest: config_digest.clone(),
            phase_digest,
        };
        self.llm_sidecar.hosted_service_inflight = Some(token.clone());
        self.llm_sidecar
            .hosted_service_config_binding
            .get_or_insert(config_digest);
        Ok(AgentServiceProgress::NeedsIo(AgentServiceIoJob {
            token,
            client: Some(client),
            operation,
        }))
    }
    pub(in crate::viewer::runtime_live) fn apply_fresh_service_io(
        &mut self,
        result: AgentServiceIoResult,
    ) -> Result<AgentServiceProgress, String> {
        if self.llm_sidecar.hosted_service_inflight.as_ref() != Some(&result.token) {
            return Err("fresh admission completion token mismatch".into());
        }
        self.llm_sidecar.hosted_service_inflight = None;
        if result.token.config_digest != self.hosted_service_config_digest()? {
            return Err("fresh service configuration changed; fenced".into());
        }
        let stage = self
            .llm_sidecar
            .hosted_admission
            .as_ref()
            .map(|admission| admission.stage.clone())
            .unwrap_or("view".into());
        if result.token.phase_digest != format!("admission:{stage}") {
            return Err("fresh admission phase mismatch".into());
        }
        let response = result.response?;
        if stage == "view" || stage.ends_with("_view") {
            let AgentServiceIoResponse::View(view) = response else {
                return Err("fresh admission expected verified view".into());
            };
            if let Some(commit) = self
                .llm_sidecar
                .hosted_admission
                .as_ref()
                .and_then(|admission| admission.commit.as_ref())
            {
                if !view
                    .version()
                    .commit
                    .satisfies_minimum(commit)
                    .map_err(|error| error.to_string())?
                {
                    return Err("fresh admission view precedes original commit".into());
                }
            }
            self.apply_hosted_verified_view(view)?;
            if stage == "view" {
                // Context observation belongs to a later eligible prepare, never an apply callback.
                self.llm_sidecar.hosted_fresh_view_ready = true;
            } else {
                let admission = self
                    .llm_sidecar
                    .hosted_admission
                    .as_mut()
                    .ok_or("fresh admission missing")?;
                admission.stage = if stage == "reserve_view" {
                    "prefix"
                } else {
                    "dispatch"
                }
                .into();
                admission.commit = None;
                self.llm_sidecar.persist_provider_lineage()?;
            }
            return Ok(AgentServiceProgress::Advanced);
        }
        let Some(response) =
            crate::viewer::runtime_live::agent_service_phase::original_response(response)?
        else {
            return Ok(AgentServiceProgress::Advanced);
        };
        let admission = self
            .llm_sidecar
            .hosted_admission
            .clone()
            .ok_or("original admission missing")?;
        let request = &admission.context.request_context;
        let phase = if stage == "reserve" {
            "reserve".into()
        } else {
            format!("prefix:{}", request.transport_attempt)
        };
        let checkpoint = self
            .llm_sidecar
            .provider_scheduler_pending
            .get(&format!("{}:{phase}", request.provider_invocation_key()))
            .ok_or("original admission checkpoint missing")?;
        response
            .validate(&checkpoint.correlation)
            .map_err(|error| error.to_string())?;
        match response.outcome {
            IntentOutcome::Committed { commit, receipt } => {
                let lease = if stage == "reserve" {
                    let lease: crate::runtime::CognitionLeaseV1 =
                        serde_json::from_value(receipt).map_err(|error| error.to_string())?;
                    lineage_generation_recovery::validate_provider_lease_identity(
                        &request.agent_subject,
                        request,
                        &lease,
                    )?;
                    Some(lease)
                } else {
                    if receipt.get("dispatched").and_then(Value::as_bool) != Some(true)
                        || receipt.get("request_digest").and_then(Value::as_str)
                            != Some(request.request_digest.as_str())
                    {
                        return Err("canonical prefix receipt identity mismatch".into());
                    }
                    admission.lease.clone()
                };
                let mut next = admission;
                next.stage = format!("{stage}_view");
                next.commit = Some(commit);
                next.lease = lease;
                self.llm_sidecar.hosted_admission = Some(next);
                self.llm_sidecar.persist_provider_lineage()?;
            }
            IntentOutcome::Received { .. } | IntentOutcome::Pending | IntentOutcome::Unknown => {}
            IntentOutcome::Rejected { reason } => {
                return Err(format!("canonical admission rejected: {reason:?}"));
            }
            _ => return Err("canonical admission cannot complete original checkpoint".into()),
        }
        Ok(AgentServiceProgress::Advanced)
    }
}
