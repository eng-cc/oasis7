//! Provider-backed LLM agent assembly for the world simulator.
//!
//! The game no longer owns a direct OpenAI-compatible client. This module keeps
//! the agent-facing assembly point alive while routing model calls through a
//! [`DecisionProvider`] implementation such as the NewAPI bridge provider.

use std::error::Error;
use std::fmt;

use super::{
    ActionCatalogEntry, DecisionProvider, ProviderBackedAgentBehavior, ProviderExecutionMode,
    DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION, DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION,
};

#[cfg(not(target_arch = "wasm32"))]
use super::{ProviderLoopbackAdapter, ProviderLoopbackHttpError};

pub const DEFAULT_LLM_AGENT_PROFILE: &str = "oasis7_p0_low_freq_npc";
pub const DEFAULT_LLM_AGENT_TIMEOUT_BUDGET_MS: u64 = 3_000;
pub const DEFAULT_LLM_AGENT_MEMORY_SUMMARY: &str = concat!(
    "goal=post_onboarding.establish_first_capability; ",
    "开局默认种子资源通常已足够首个 smelter（例如 electricity>=10 且 data>=5）；",
    "若当前没有 factory.smelter.mk1，不要先 harvest_radiation，优先 build_factory(factory.smelter.mk1)。",
    "只有在 electricity<10 时才先 harvest_radiation；只有在 data<5 时才先 mine_compound/refine_compound。 ",
    "build_factory 成功后立刻 schedule_recipe(",
    "recipe.smelter.iron_ingot|recipe.smelter.copper_wire|recipe.smelter.polymer_resin|recipe.smelter.alloy_plate",
    ")；不要长期停留在 wait/move/speak/inspect。"
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmAgentProviderConfig {
    pub provider_config_ref: Option<String>,
    pub agent_profile: Option<String>,
    pub execution_mode: ProviderExecutionMode,
    pub observation_schema_version: String,
    pub action_schema_version: String,
    pub environment_class: Option<String>,
    pub fallback_reason: Option<String>,
    pub fixture_id: Option<String>,
    pub replay_id: Option<String>,
    pub timeout_budget_ms: u64,
    pub memory_summary: Option<String>,
}

impl Default for LlmAgentProviderConfig {
    fn default() -> Self {
        Self {
            provider_config_ref: None,
            agent_profile: Some(DEFAULT_LLM_AGENT_PROFILE.to_string()),
            execution_mode: ProviderExecutionMode::HeadlessAgent,
            observation_schema_version: DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION.to_string(),
            action_schema_version: DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION.to_string(),
            environment_class: None,
            fallback_reason: None,
            fixture_id: None,
            replay_id: None,
            timeout_budget_ms: DEFAULT_LLM_AGENT_TIMEOUT_BUDGET_MS,
            memory_summary: Some(DEFAULT_LLM_AGENT_MEMORY_SUMMARY.to_string()),
        }
    }
}

impl LlmAgentProviderConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_provider_config_ref(mut self, provider_config_ref: impl Into<String>) -> Self {
        self.provider_config_ref = Some(provider_config_ref.into());
        self
    }

    pub fn with_agent_profile(mut self, agent_profile: impl Into<String>) -> Self {
        self.agent_profile = Some(agent_profile.into());
        self
    }

    pub fn with_execution_mode(mut self, execution_mode: ProviderExecutionMode) -> Self {
        self.execution_mode = execution_mode;
        self
    }

    pub fn with_environment_class(mut self, environment_class: impl Into<String>) -> Self {
        self.environment_class = Some(environment_class.into());
        self
    }

    pub fn with_fallback_reason(mut self, fallback_reason: impl Into<String>) -> Self {
        self.fallback_reason = Some(fallback_reason.into());
        self
    }

    pub fn with_fixture_id(mut self, fixture_id: impl Into<String>) -> Self {
        self.fixture_id = Some(fixture_id.into());
        self
    }

    pub fn with_replay_id(mut self, replay_id: impl Into<String>) -> Self {
        self.replay_id = Some(replay_id.into());
        self
    }

    pub fn with_timeout_budget_ms(mut self, timeout_budget_ms: u64) -> Self {
        self.timeout_budget_ms = timeout_budget_ms.max(1);
        self
    }

    pub fn with_memory_summary(mut self, memory_summary: impl Into<String>) -> Self {
        self.memory_summary = Some(memory_summary.into());
        self
    }

    pub fn without_memory_summary(mut self) -> Self {
        self.memory_summary = None;
        self
    }
}

pub fn provider_phase1_action_catalog() -> Vec<ActionCatalogEntry> {
    vec![
        ActionCatalogEntry::new("wait", "yield current turn without acting"),
        ActionCatalogEntry::new("wait_ticks", "sleep for a bounded number of ticks"),
        ActionCatalogEntry::new("move_agent", "move to a neighboring location"),
        ActionCatalogEntry::new(
            "harvest_radiation",
            "recover electricity before industrial expansion or recipe execution",
        ),
        ActionCatalogEntry::new(
            "mine_compound",
            "extract raw compound mass when recovery needs material inputs",
        ),
        ActionCatalogEntry::new(
            "refine_compound",
            "convert compound mass into hardware output for recovery",
        ),
        ActionCatalogEntry::new(
            "build_factory",
            "start the first compatible factory line for industrial progression",
        ),
        ActionCatalogEntry::new(
            "schedule_recipe",
            "run the next compatible recipe on an existing factory line",
        ),
        ActionCatalogEntry::new("speak_to_nearby", "emit a lightweight nearby speech event"),
        ActionCatalogEntry::new(
            "inspect_target",
            "emit a lightweight target inspection event",
        ),
        ActionCatalogEntry::new(
            "simple_interact",
            "emit a lightweight simple interaction event",
        ),
    ]
}

pub fn build_provider_backed_llm_agent_behavior<P>(
    agent_id: impl Into<String>,
    provider: P,
    action_catalog: Vec<ActionCatalogEntry>,
    config: LlmAgentProviderConfig,
) -> ProviderBackedAgentBehavior<P>
where
    P: DecisionProvider,
{
    let mut behavior = ProviderBackedAgentBehavior::new(agent_id, provider, action_catalog)
        .with_execution_mode(config.execution_mode)
        .with_observation_schema_version(config.observation_schema_version)
        .with_action_schema_version(config.action_schema_version)
        .with_timeout_budget_ms(config.timeout_budget_ms);

    if let Some(provider_config_ref) = config.provider_config_ref {
        behavior = behavior.with_provider_config_ref(provider_config_ref);
    }
    if let Some(agent_profile) = config.agent_profile {
        behavior = behavior.with_agent_profile(agent_profile);
    }
    if let Some(environment_class) = config.environment_class {
        behavior = behavior.with_environment_class(environment_class);
    }
    if let Some(fallback_reason) = config.fallback_reason {
        behavior = behavior.with_fallback_reason(fallback_reason);
    }
    if let Some(fixture_id) = config.fixture_id {
        behavior = behavior.with_fixture_id(fixture_id);
    }
    if let Some(replay_id) = config.replay_id {
        behavior = behavior.with_replay_id(replay_id);
    }
    if let Some(memory_summary) = config.memory_summary {
        behavior = behavior.with_memory_summary(memory_summary);
    }

    behavior
}

#[cfg(not(target_arch = "wasm32"))]
pub fn build_remote_provider_llm_agent_behavior(
    agent_id: impl Into<String>,
    base_url: &str,
    auth_token: Option<&str>,
    timeout_ms: u64,
    transport: &str,
    config: LlmAgentProviderConfig,
) -> Result<ProviderBackedAgentBehavior<ProviderLoopbackAdapter>, LlmAgentProviderBuildError> {
    let adapter = ProviderLoopbackAdapter::new_with_transport(
        base_url,
        auth_token,
        timeout_ms.max(1),
        transport,
    )
    .map_err(LlmAgentProviderBuildError::Provider)?;

    Ok(build_provider_backed_llm_agent_behavior(
        agent_id,
        adapter,
        provider_phase1_action_catalog(),
        config,
    ))
}

#[derive(Debug)]
pub enum LlmAgentProviderBuildError {
    #[cfg(not(target_arch = "wasm32"))]
    Provider(ProviderLoopbackHttpError),
}

impl fmt::Display for LlmAgentProviderBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Provider(err) => write!(f, "provider-backed LLM agent setup failed: {err}"),
        }
    }
}

impl Error for LlmAgentProviderBuildError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulator::{
        AgentBehavior, DecisionProviderError, DecisionRequest, DecisionResponse,
    };

    #[derive(Debug)]
    struct WaitProvider;

    impl DecisionProvider for WaitProvider {
        fn provider_id(&self) -> &str {
            "wait-provider"
        }

        fn decide(
            &mut self,
            _request: &DecisionRequest,
        ) -> Result<DecisionResponse, DecisionProviderError> {
            Ok(DecisionResponse::wait("wait-provider"))
        }
    }

    #[test]
    fn llm_agent_builder_keeps_agent_layer_provider_backed() {
        let behavior = build_provider_backed_llm_agent_behavior(
            "agent-llm",
            WaitProvider,
            provider_phase1_action_catalog(),
            LlmAgentProviderConfig::new().with_execution_mode(ProviderExecutionMode::PlayerParity),
        );

        assert_eq!(behavior.agent_id(), "agent-llm");
    }

    #[test]
    fn provider_phase1_catalog_keeps_llm_agent_actions() {
        let action_refs = provider_phase1_action_catalog()
            .into_iter()
            .map(|entry| entry.action_ref)
            .collect::<Vec<_>>();

        assert!(action_refs.contains(&"move_agent".to_string()));
        assert!(action_refs.contains(&"build_factory".to_string()));
        assert!(action_refs.contains(&"schedule_recipe".to_string()));
    }
}
