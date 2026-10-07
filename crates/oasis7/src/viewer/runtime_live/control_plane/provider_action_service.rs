use super::*;

impl ViewerRuntimeLiveServer {
    #[cfg(any(test, feature = "test_tier_required"))]
    pub fn test_prepare_canonical_provider_response_with_memory(
        &mut self,
        agent_id: &str,
        action: crate::simulator::Action,
        intents: Vec<crate::simulator::MemoryWriteIntent>,
    ) -> Result<serde_json::Value, String> {
        let value = self.test_prepare_canonical_provider_response(agent_id, action)?;
        let mut context: super::llm_sidecar::RuntimeProviderActionContext =
            serde_json::from_value(value).map_err(|error| error.to_string())?;
        context.response.base_decision_response.memory_write_intents = intents.clone();
        context.response.response_digest =
            crate::simulator::cognition_response_digest(&context.response.base_decision_response);
        context.memory_write_intents = intents;
        serde_json::to_value(context).map_err(|error| error.to_string())
    }
    #[cfg(any(test, feature = "test_tier_required"))]
    pub fn test_canonical_provider_summary(&self) -> serde_json::Value {
        self.llm_sidecar.service_test_summary()
    }
    #[cfg(any(test, feature = "test_tier_required"))]
    pub fn test_prepare_canonical_provider_response(
        &mut self,
        agent_id: &str,
        action: crate::simulator::Action,
    ) -> Result<serde_json::Value, String> {
        use crate::world_service::client::WorldServicePort;
        use oasis7_client_api::world_service::*;
        let client = self
            .world_service_client()
            .map_err(|error| format!("{error:?}"))?
            .ok_or("test requires actual service client")?;
        self.verified_world_view = Some(
            client
                .read_view(ReadWorldViewRequest {
                    contract_version: WORLD_SERVICE_CONTRACT_VERSION,
                    world: client.config().expected_world.clone(),
                    scope_id: client.config().scope_id.clone(),
                    min_commit: None,
                    fixed_commit: None,
                    deadline_unix_ms: None,
                })
                .map_err(|error| error.to_string())?,
        );
        self.configure_service_provider();
        self.llm_sidecar
            .sync_shadow_kernel(&self.world, &self.snapshot_config)?;
        self.llm_sidecar.make_service_test_context(agent_id, action)
    }
    #[cfg(any(test, feature = "test_tier_required"))]
    pub fn test_queue_canonical_provider_response(
        &mut self,
        cognition: serde_json::Value,
        action: crate::simulator::Action,
    ) -> Result<(), String> {
        use crate::world_service::client::WorldServicePort;
        use oasis7_client_api::world_service::*;
        if !self.chain_link_enabled() {
            return Err("test requires chain-linked configuration".into());
        }
        let client = self
            .world_service_client()
            .map_err(|error| format!("{error:?}"))?
            .ok_or("test requires actual service client")?;
        let view = client
            .read_view(ReadWorldViewRequest {
                contract_version: WORLD_SERVICE_CONTRACT_VERSION,
                world: client.config().expected_world.clone(),
                scope_id: client.config().scope_id.clone(),
                min_commit: None,
                fixed_commit: None,
                deadline_unix_ms: None,
            })
            .map_err(|error| error.to_string())?;
        self.verified_world_view = Some(view);
        self.configure_service_provider();
        let mut cognition: super::llm_sidecar::RuntimeProviderActionContext =
            serde_json::from_value(cognition).map_err(|error| error.to_string())?;
        let lease = self
            .llm_sidecar
            .reserve_provider_lease_at_authority(&mut self.world, &cognition.request)?;
        self.llm_sidecar
            .provider_prefix_at_authority(&mut self.world, &cognition.request)?;
        cognition.cognition_lease = Some(lease);
        self.llm_sidecar
            .queue_service_test_response(cognition, action)?;
        Ok(())
    }

    #[cfg(any(test, feature = "test_tier_required"))]
    pub fn test_poll_canonical_provider_response(&mut self) -> Result<(), String> {
        self.llm_sidecar.request_decision();
        self.enqueue_llm_action_from_sidecar()
            .map(|_| ())
            .map_err(|trace| {
                trace
                    .llm_error
                    .unwrap_or_else(|| "provider trace failed".into())
            })
    }
    pub(super) fn configure_service_provider(&mut self) {
        self.llm_sidecar.provider_service_required = self.chain_link_enabled();
        self.llm_sidecar.provider_service_config = self.config.world_service.clone();
        self.llm_sidecar.provider_service_query_state = self.world_service_query_state.clone();
        self.llm_sidecar.provider_service_signer = self.config.world_service_agent_signer.clone();
        self.llm_sidecar.provider_service_projection = self
            .verified_world_view
            .as_ref()
            .map(|view| view.projection().clone());
    }
    pub(super) fn enqueue_service_provider_action(
        &mut self,
    ) -> Result<Option<AgentDecisionTrace>, AgentDecisionTrace> {
        let tick = self
            .llm_sidecar
            .provider_service_projection
            .as_ref()
            .map(|view| view.state.time)
            .unwrap_or(0);
        if self.llm_sidecar.provider_service_projection.is_none() {
            return Err(wake_handoff_error_trace(
                "world_service",
                tick,
                "verified canonical Agent view is unavailable; turn remains pending".into(),
            ));
        }
        if !self.llm_sidecar.provider_service_pending.is_empty() {
            self.retry_committed_provider_action()
                .map_err(|error| wake_handoff_error_trace("world_service", tick, error))?;
            return Ok(None);
        }
        if let Some((agent_id, trace, cognition)) = self.llm_sidecar.pending_admitted_service_wait()
        {
            return self.close_admitted_service_wait(agent_id, trace, cognition, tick);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if self
            .llm_sidecar
            .retry_service_wait_admission(&mut self.world, &self.snapshot_config)
            .map_err(|error| wake_handoff_error_trace("world_service", tick, error))?
        {
            return Ok(None);
        }
        let Some(decision) = self.llm_sidecar.next_llm_decision(
            &mut self.world,
            &self.snapshot_config,
            self.config.world_id.as_str(),
        ) else {
            return Ok(None);
        };
        let trace = decision.decision_trace;
        if trace.as_ref().is_some_and(|trace| {
            trace.parse_error.is_some()
                || (trace.llm_error.is_some()
                    && !super::super::decision_trace::is_trace_only_overflow(trace))
        }) {
            return Err(trace.expect("checked provider error trace"));
        }
        match decision.decision {
            AgentDecision::Act(action) => {
                let cognition = decision.cognition.ok_or_else(|| {
                    wake_handoff_error_trace(
                        &decision.agent_id,
                        tick,
                        "canonical Agent action lacks signed cognition context".into(),
                    )
                })?;
                let runtime_action =
                    simulator_action_to_runtime(&action, &self.world).ok_or_else(|| {
                        wake_handoff_error_trace(
                            &decision.agent_id,
                            tick,
                            "canonical Agent action is unsupported by the registered codec".into(),
                        )
                    })?;
                self.commit_provider_runtime_action(&runtime_action, &cognition, action)
                    .map_err(|error| {
                        wake_handoff_error_trace(&decision.agent_id, tick, error.reason())
                    })?;
                Ok(trace)
            }
            AgentDecision::Wait | AgentDecision::WaitTicks(_) if decision.continuation_admitted => {
                self.close_admitted_service_wait(decision.agent_id, trace, decision.cognition, tick)
            }
            _ => Err(wake_handoff_error_trace(
                &decision.agent_id,
                tick,
                "canonical provider decision has no committed registered intent".into(),
            )),
        }
    }

    fn close_admitted_service_wait(
        &mut self,
        agent_id: String,
        trace: Option<AgentDecisionTrace>,
        cognition: Option<super::llm_sidecar::RuntimeProviderActionContext>,
        tick: u64,
    ) -> Result<Option<AgentDecisionTrace>, AgentDecisionTrace> {
        self.llm_sidecar
            .checkpoint_admitted_service_wait()
            .map_err(|error| wake_handoff_error_trace(&agent_id, tick, error))?;
        let cognition = cognition.as_ref().ok_or_else(|| {
            wake_handoff_error_trace(
                &agent_id,
                tick,
                "canonical admitted Wait lacks original cognition context".into(),
            )
        })?;
        self.settle_provider_cognition_lease_for_request(
            &agent_id,
            cognition.cognition_lease.clone(),
            Some(&cognition.request.request_context),
        )
        .map_err(|error| wake_handoff_error_trace(&agent_id, tick, error))?;
        self.llm_sidecar
            .finish_admitted_service_wait(&agent_id)
            .map_err(|error| wake_handoff_error_trace(&agent_id, tick, error))?;
        let feedback = self.llm_sidecar.provider_feedback(
            cognition,
            None,
            "pending",
            None,
            None,
            Some("continuation_admitted".into()),
        );
        self.deliver_provider_feedback_best_effort(feedback);
        Ok(trace)
    }
}
