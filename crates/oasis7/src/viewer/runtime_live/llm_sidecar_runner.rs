use super::*;

impl RuntimeLlmSidecar {
    pub(in crate::viewer::runtime_live) fn apply_prompt_profile_to_driver(
        &mut self,
        profile: &AgentPromptProfile,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(RuntimeDecisionRunner::Builtin(runner)) = self.runner.as_mut() {
            let _ = runner.set_prompt_overrides(
                profile.agent_id.as_str(),
                profile.system_prompt_override.clone(),
                profile.short_term_goal_override.clone(),
                profile.long_term_goal_override.clone(),
            );
            return;
        }
        #[cfg(target_arch = "wasm32")]
        let Some(RuntimeDecisionRunner::Builtin(runner)) = self.runner.as_mut() else {
            return;
        };
        #[cfg(target_arch = "wasm32")]
        let Some(agent) = runner.get_mut(profile.agent_id.as_str()) else {
            return;
        };
        #[cfg(target_arch = "wasm32")]
        agent.behavior.apply_prompt_overrides(
            profile.system_prompt_override.clone(),
            profile.short_term_goal_override.clone(),
            profile.long_term_goal_override.clone(),
        );
    }

    pub(super) fn ensure_runner_initialized(&mut self) -> Result<(), String> {
        let kernel = self
            .shadow_kernel
            .as_ref()
            .ok_or_else(|| "shadow kernel not initialized".to_string())?;
        let provider_settings = provider_settings_from_env()?;
        if self.runner.is_none() {
            self.runner = Some(match provider_settings.as_ref() {
                #[cfg(not(target_arch = "wasm32"))]
                Some(_) => {
                    RuntimeDecisionRunner::ProviderBacked(AsyncAgentRunner::with_default_capacity())
                }
                #[cfg(not(target_arch = "wasm32"))]
                None => RuntimeDecisionRunner::Builtin(AsyncAgentRunner::with_default_capacity()),
                #[cfg(target_arch = "wasm32")]
                Some(_) => RuntimeDecisionRunner::ProviderBacked(AgentRunner::new()),
                #[cfg(target_arch = "wasm32")]
                None => RuntimeDecisionRunner::Builtin(AgentRunner::new()),
            });
        }
        let runner = self
            .runner
            .as_mut()
            .ok_or_else(|| "llm runner not initialized".to_string())?;
        let mut agent_ids: Vec<String> = kernel.model().agents.keys().cloned().collect();
        agent_ids.sort();
        for agent_id in agent_ids {
            match runner {
                #[cfg(not(target_arch = "wasm32"))]
                RuntimeDecisionRunner::Builtin(runner) => {
                    if self.provider_agent_ids.contains(agent_id.as_str()) {
                        continue;
                    }
                    let mut config = LlmAgentConfig::from_default_sources_for_agent(
                        agent_id.as_str(),
                    )
                    .map_err(|err| format!("llm init failed for {}: {:?}", agent_id, err))?;
                    config.timeout_ms = resolve_runtime_live_llm_timeout_ms(config.timeout_ms);
                    let client = OpenAiChatCompletionClient::from_config(&config)
                        .map_err(|err| format!("llm init failed for {}: {:?}", agent_id, err))?;
                    let mut behavior =
                        LlmAgentBehavior::new(agent_id.clone(), config.clone(), client);
                    let profile = self.prompt_profiles.get(agent_id.as_str());
                    behavior.apply_prompt_overrides(
                        profile.and_then(|profile| profile.system_prompt_override.clone()),
                        profile
                            .and_then(|profile| profile.short_term_goal_override.clone())
                            .or_else(|| Some(runtime_live_phase1_short_term_goal())),
                        profile.and_then(|profile| profile.long_term_goal_override.clone()),
                    );
                    restore_behavior_long_term_memory_from_model(
                        &mut behavior,
                        kernel,
                        agent_id.as_str(),
                    );
                    runner
                        .register(behavior)
                        .map_err(|error| format!("builtin agent init failed: {error}"))?;
                    // The async Builtin runner deliberately enters the same
                    // cognition-context preparation/feedback lifecycle as
                    // ProviderBacked. The set is retained under the historic
                    // provider-prefixed field until that storage is renamed.
                    self.provider_agent_ids.insert(agent_id);
                }
                #[cfg(target_arch = "wasm32")]
                RuntimeDecisionRunner::Builtin(runner) => {
                    if runner.get(agent_id.as_str()).is_some() {
                        continue;
                    }
                    let mut config = LlmAgentConfig::from_default_sources_for_agent(
                        agent_id.as_str(),
                    )
                    .map_err(|err| format!("llm init failed for {}: {:?}", agent_id, err))?;
                    config.timeout_ms = resolve_runtime_live_llm_timeout_ms(config.timeout_ms);
                    let client = OpenAiChatCompletionClient::from_config(&config)
                        .map_err(|err| format!("llm init failed for {}: {:?}", agent_id, err))?;
                    let mut behavior = LlmAgentBehavior::new(agent_id.clone(), config, client);
                    let short_term_goal_override = self
                        .prompt_profiles
                        .get(agent_id.as_str())
                        .and_then(|profile| profile.short_term_goal_override.clone())
                        .or_else(|| Some(runtime_live_phase1_short_term_goal()));
                    let system_prompt_override = self
                        .prompt_profiles
                        .get(agent_id.as_str())
                        .and_then(|profile| profile.system_prompt_override.clone());
                    let long_term_goal_override = self
                        .prompt_profiles
                        .get(agent_id.as_str())
                        .and_then(|profile| profile.long_term_goal_override.clone());
                    behavior.apply_prompt_overrides(
                        system_prompt_override,
                        short_term_goal_override,
                        long_term_goal_override,
                    );
                    restore_behavior_long_term_memory_from_model(
                        &mut behavior,
                        kernel,
                        agent_id.as_str(),
                    );
                    runner.register(behavior);
                }
                RuntimeDecisionRunner::ProviderBacked(runner) => {
                    if self.provider_agent_ids.contains(agent_id.as_str()) {
                        continue;
                    }
                    let settings = provider_settings.as_ref().ok_or_else(|| {
                        "provider runner selected without resolved settings".to_string()
                    })?;
                    let adapter = ProviderLoopbackAdapter::new_with_transport(
                        settings.base_url.as_str(),
                        settings.auth_token.as_deref(),
                        settings.connect_timeout_ms,
                        settings.provider_transport.as_str(),
                    )
                    .map_err(|err| format!("provider init failed for {}: {}", agent_id, err))?;
                    let behavior = ProviderBackedAgentBehavior::new(
                        agent_id.clone(),
                        adapter,
                        provider_phase1_action_catalog(),
                    )
                    .require_continuous_request_context()
                    .with_provider_config_ref(format!(
                        "provider://{}/runtime-live/{}",
                        settings.provider_transport, agent_id
                    ))
                    .with_agent_profile(settings.agent_profile.clone())
                    .with_execution_mode(settings.execution_mode)
                    .with_timeout_budget_ms(settings.decision_timeout_ms)
                    .with_environment_class("runtime_live")
                    .with_memory_summary(provider_phase1_memory_summary());
                    let behavior =
                        if let Some(fallback_reason) = settings.fallback_reason.as_deref() {
                            behavior.with_fallback_reason(fallback_reason)
                        } else {
                            behavior
                        };
                    runner
                        .register(behavior)
                        .map_err(|error| format!("provider agent init failed: {error}"))?;
                    self.provider_agent_ids.insert(agent_id);
                }
            }
        }
        Ok(())
    }
}
