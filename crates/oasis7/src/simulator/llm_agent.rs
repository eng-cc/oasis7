//! Provider-backed LLM agent assembly for the world simulator.
//!
//! The game no longer owns a direct OpenAI-compatible client. This module keeps
//! the agent-facing assembly point alive while routing model calls through a
//! [`DecisionProvider`] implementation such as the NewAPI bridge provider.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use serde::Serialize;

use super::agent::{
    ActionResult, AgentBehavior, AgentDecision, AgentDecisionTrace, LlmChatMessageTrace,
    LlmChatRole, LlmDecisionDiagnostics, LlmEffectIntentTrace, LlmEffectReceiptTrace,
    LlmPromptSectionTrace, LlmStepTrace,
};
use super::kernel::{Observation, RejectReason, WorldEvent, WorldEventKind};
use super::memory::{AgentMemory, LongTermMemoryEntry, MemoryEntry};
use super::types::{
    Action, ModuleInstallTarget, ResourceKind, ResourceOwner, CM_PER_KM,
    DEFAULT_MOVE_COST_PER_KM_ELECTRICITY,
};
use super::{
    ActionCatalogEntry, DecisionProvider, ProviderBackedAgentBehavior, ProviderExecutionMode,
    DEFAULT_PROVIDER_ACTION_SCHEMA_VERSION, DEFAULT_PROVIDER_OBSERVATION_SCHEMA_VERSION,
};

#[cfg(not(target_arch = "wasm32"))]
use super::{ProviderLoopbackAdapter, ProviderLoopbackHttpError};

mod behavior_guardrails;
mod behavior_loop;
mod behavior_prompt;
mod behavior_prompt_modules;
mod behavior_runtime_helpers;
mod decision_flow;
mod execution_controls;
mod memory_selector;
mod prompt_assembly;

pub use memory_selector::{MemorySelector, MemorySelectorConfig};
pub use prompt_assembly::{
    PromptAssembler, PromptAssemblyInput, PromptAssemblyOutput, PromptBudget, PromptStepContext,
};

use decision_flow::{
    parse_limit_arg, parse_llm_turn_payloads_with_debug_mode, prompt_section_kind_name,
    prompt_section_priority_name, summarize_trace_text, DecisionRewriteReceipt,
    ExecuteUntilCondition, ExecuteUntilDirective, LlmModuleCallRequest, ModuleCallExchange,
    ParsedLlmTurn,
};
use execution_controls::{
    default_execute_until_conditions_for_action, ActionReplanGuardState, ActiveExecuteUntil,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmPromptProfile {
    Compact,
    Balanced,
}

impl LlmPromptProfile {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "compact" => Some(Self::Compact),
            "balanced" => Some(Self::Balanced),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Balanced => "balanced",
        }
    }

    pub fn prompt_budget(self) -> PromptBudget {
        match self {
            Self::Compact => PromptBudget {
                context_window_tokens: 4_096,
                reserved_output_tokens: 768,
                safety_margin_tokens: 352,
            },
            Self::Balanced => PromptBudget {
                context_window_tokens: 4_608,
                reserved_output_tokens: 896,
                safety_margin_tokens: 480,
            },
        }
    }

    pub fn memory_selector_config(self) -> MemorySelectorConfig {
        match self {
            Self::Compact => MemorySelectorConfig {
                short_term_candidate_limit: 8,
                long_term_candidate_limit: 12,
                short_term_top_k: 3,
                long_term_top_k: 4,
            },
            Self::Balanced => MemorySelectorConfig {
                short_term_candidate_limit: 8,
                long_term_candidate_limit: 12,
                short_term_top_k: 2,
                long_term_top_k: 3,
            },
        }
    }
}

pub const DEFAULT_LLM_SYSTEM_PROMPT: &str = "你是硅基文明发展 Agent。按“读规则/观察 -> 资源稳态 -> 产业建设 -> 治理协作 -> 危机韧性”推进文明进程，每轮仅提交一个可执行 decision。若规则或动作前置条件不明确，先调用 world.rules.guide 与 environment.current_observation，再做决策。";
pub const DEFAULT_LLM_SHORT_TERM_GOAL: &str = "先识别当前阶段最关键瓶颈，并按前置条件逐步推进：能源与数据稳定后再扩产，扩产后推进治理与风险处理。遇到 action_rejected 时根据 reject_reason 切换到补前置动作，避免原样重复失败参数。";
pub const DEFAULT_LLM_LONG_TERM_GOAL: &str =
    "构建可持续、可治理、具韧性的文明系统，让资源、组织与风险应对形成长期正反馈，并保持阶段推进可解释。";
pub const DEFAULT_LLM_MAX_MODULE_CALLS: usize = 3;
pub const DEFAULT_LLM_MAX_DECISION_STEPS: usize = 4;
pub const DEFAULT_LLM_MAX_REPAIR_ROUNDS: usize = 1;
pub const DEFAULT_LLM_PROMPT_MAX_HISTORY_ITEMS: usize = 4;
pub const DEFAULT_LLM_PROMPT_PROFILE: LlmPromptProfile = LlmPromptProfile::Balanced;
pub const DEFAULT_LLM_FORCE_REPLAN_AFTER_SAME_ACTION: usize = 4;
pub const DEFAULT_LLM_HARVEST_MAX_AMOUNT_CAP: i64 = 100;
pub const DEFAULT_LLM_EXECUTE_UNTIL_AUTO_REENTER_TICKS: usize = 4;
pub const DEFAULT_LLM_DEBUG_MODE: bool = false;
pub const DEFAULT_LLM_HARVEST_EXECUTE_UNTIL_MAX_TICKS: u64 = 3;
const DEFAULT_RECIPE_HARDWARE_COST_PER_BATCH: i64 = 2;
const DEFAULT_RECIPE_ELECTRICITY_COST_PER_BATCH: i64 = 6;
const DEFAULT_REFINE_RECOVERY_MASS_G_PER_HARDWARE: i64 = 1_000;
const DEFAULT_REFINE_ELECTRICITY_COST_PER_KG: i64 = 2;
const DEFAULT_MINE_COMPOUND_MAX_PER_ACTION_G: i64 = 5_000;
const DEFAULT_MINE_ELECTRICITY_COST_PER_KG: i64 = 1;
const DEFAULT_MINE_DEPLETED_LOCATION_COOLDOWN_TICKS: u64 = 6;
const DEFAULT_MINE_FAILURE_STREAK_WINDOW_TICKS: u64 = 24;
const DEFAULT_MAX_MOVE_DISTANCE_CM_PER_TICK: i64 = 1_000_000;
const TRACKED_RECIPE_IDS: [&str; 11] = [
    "recipe.smelter.iron_ingot",
    "recipe.smelter.copper_wire",
    "recipe.smelter.polymer_resin",
    "recipe.smelter.alloy_plate",
    "recipe.assembler.gear",
    "recipe.assembler.control_chip",
    "recipe.assembler.motor_mk1",
    "recipe.assembler.logistics_drone",
    "recipe.assembler.sensor_pack",
    "recipe.assembler.module_rack",
    "recipe.assembler.factory_core",
];

const DEFAULT_SHORT_TERM_MEMORY_CAPACITY: usize = 128;
const DEFAULT_LONG_TERM_MEMORY_CAPACITY: usize = 256;
const LLM_PROMPT_MODULE_CALL_KIND: &str = "llm.prompt.module_call";
const LLM_PROMPT_MODULE_CALL_CAP_REF: &str = "llm.prompt.module_access";
const LLM_PROMPT_MODULE_CALL_ORIGIN: &str = "llm_agent";
const PROMPT_MODULE_RESULT_MAX_CHARS: usize = 520;
const PROMPT_MODULE_ARGS_MAX_CHARS: usize = 192;
const PROMPT_MEMORY_DIGEST_MAX_CHARS: usize = 360;
const PROMPT_CONVERSATION_ITEM_MAX_CHARS: usize = 320;
const PROMPT_CONVERSATION_MAX_ITEMS: usize = 12;
const PROMPT_OBSERVATION_VISIBLE_AGENTS_MAX: usize = 5;
const PROMPT_OBSERVATION_VISIBLE_LOCATIONS_MAX: usize = 5;
const CONVERSATION_HISTORY_MAX_ITEMS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PromptLastActionSummary {
    kind: String,
    success: bool,
    reject_reason: Option<String>,
    decision_rewrite: Option<DecisionRewriteReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct RecipeCoverageProgress {
    completed: BTreeSet<String>,
}

impl RecipeCoverageProgress {
    fn tracked_factory_kind(recipe_id: &str) -> Option<&'static str> {
        match recipe_id.trim() {
            recipe_id if recipe_id.starts_with("recipe.smelter.") => Some("factory.smelter.mk1"),
            recipe_id if recipe_id.starts_with("recipe.assembler.") => {
                Some("factory.assembler.mk1")
            }
            _ => None,
        }
    }

    fn is_tracked(recipe_id: &str) -> bool {
        TRACKED_RECIPE_IDS
            .iter()
            .any(|candidate| candidate == &recipe_id.trim())
    }

    fn mark_completed(&mut self, recipe_id: &str) {
        let normalized = recipe_id.trim();
        if Self::is_tracked(normalized) {
            self.completed.insert(normalized.to_string());
        }
    }

    fn is_completed(&self, recipe_id: &str) -> bool {
        self.completed.contains(recipe_id.trim())
    }

    fn missing_recipe_ids(&self) -> Vec<String> {
        TRACKED_RECIPE_IDS
            .iter()
            .filter(|recipe_id| !self.completed.contains(**recipe_id))
            .map(|recipe_id| (*recipe_id).to_string())
            .collect()
    }

    fn next_uncovered_recipe_for_factory_kind_excluding(
        &self,
        factory_kind: &str,
        current_recipe_id: &str,
    ) -> Option<String> {
        let current_recipe_id = current_recipe_id.trim();
        self.missing_recipe_ids().into_iter().find(|recipe_id| {
            recipe_id.as_str() != current_recipe_id
                && Self::tracked_factory_kind(recipe_id.as_str()) == Some(factory_kind)
        })
    }

    fn summary_json(&self) -> serde_json::Value {
        let completed = TRACKED_RECIPE_IDS
            .iter()
            .filter(|recipe_id| self.completed.contains(**recipe_id))
            .map(|recipe_id| (*recipe_id).to_string())
            .collect::<Vec<_>>();
        let missing = self.missing_recipe_ids();
        serde_json::json!({
            "tracked_total": TRACKED_RECIPE_IDS.len(),
            "completed": completed,
            "missing": missing,
        })
    }

    fn is_fully_covered(&self) -> bool {
        TRACKED_RECIPE_IDS
            .iter()
            .all(|recipe_id| self.completed.contains(*recipe_id))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MineFailureStreak {
    count: u32,
    last_time: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct KnownModuleArtifactRecord {
    pub wasm_hash: String,
    pub publisher_agent_id: String,
    pub bytes_len: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module_id_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct KnownInstalledModuleRecord {
    pub module_id: String,
    pub module_version: String,
    pub wasm_hash: String,
    pub installer_agent_id: String,
    pub install_target: ModuleInstallTarget,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LlmPromptOverrides {
    pub system_prompt: Option<String>,
    pub short_term_goal: Option<String>,
    pub long_term_goal: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmAgentConfig {
    pub model: String,
    pub system_prompt: String,
    pub short_term_goal: String,
    pub long_term_goal: String,
    pub max_module_calls: usize,
    pub max_decision_steps: usize,
    pub max_repair_rounds: usize,
    pub prompt_max_history_items: usize,
    pub prompt_profile: LlmPromptProfile,
    pub force_replan_after_same_action: usize,
    pub harvest_max_amount_cap: i64,
    pub execute_until_auto_reenter_ticks: usize,
    pub llm_debug_mode: bool,
}

impl Default for LlmAgentConfig {
    fn default() -> Self {
        Self {
            model: "provider_backed".to_string(),
            system_prompt: DEFAULT_LLM_SYSTEM_PROMPT.to_string(),
            short_term_goal: DEFAULT_LLM_SHORT_TERM_GOAL.to_string(),
            long_term_goal: DEFAULT_LLM_LONG_TERM_GOAL.to_string(),
            max_module_calls: DEFAULT_LLM_MAX_MODULE_CALLS,
            max_decision_steps: DEFAULT_LLM_MAX_DECISION_STEPS,
            max_repair_rounds: DEFAULT_LLM_MAX_REPAIR_ROUNDS,
            prompt_max_history_items: DEFAULT_LLM_PROMPT_MAX_HISTORY_ITEMS,
            prompt_profile: DEFAULT_LLM_PROMPT_PROFILE,
            force_replan_after_same_action: DEFAULT_LLM_FORCE_REPLAN_AFTER_SAME_ACTION,
            harvest_max_amount_cap: DEFAULT_LLM_HARVEST_MAX_AMOUNT_CAP,
            execute_until_auto_reenter_ticks: DEFAULT_LLM_EXECUTE_UNTIL_AUTO_REENTER_TICKS,
            llm_debug_mode: DEFAULT_LLM_DEBUG_MODE,
        }
    }
}

impl LlmAgentConfig {
    pub fn provider_backed() -> Self {
        Self::default()
    }

    fn prompt_budget(&self) -> PromptBudget {
        self.prompt_profile.prompt_budget()
    }

    fn memory_selector_config(&self) -> MemorySelectorConfig {
        self.prompt_profile.memory_selector_config()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmCompletionRequest {
    pub model: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub debug_mode: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LlmCompletionTurn {
    Decision {
        payload: serde_json::Value,
    },
    ModuleCall {
        module: String,
        args: serde_json::Value,
    },
}

pub trait LlmCompletionClient {
    fn complete(
        &self,
        request: &LlmCompletionRequest,
    ) -> Result<LlmCompletionResult, LlmClientError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct LlmCompletionResult {
    pub turns: Vec<LlmCompletionTurn>,
    pub output: String,
    pub model: Option<String>,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmClientError {
    BuildClient { message: String },
    Http { message: String },
    HttpStatus { code: u16, message: String },
    DecodeResponse { message: String },
    EmptyChoice,
}

impl fmt::Display for LlmClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmClientError::BuildClient { message } => write!(f, "client build failed: {message}"),
            LlmClientError::Http { message } => write!(f, "http request failed: {message}"),
            LlmClientError::HttpStatus { code, message } => {
                write!(f, "http status {code}: {message}")
            }
            LlmClientError::DecodeResponse { message } => {
                write!(f, "decode response failed: {message}")
            }
            LlmClientError::EmptyChoice => write!(f, "empty completion choice"),
        }
    }
}

impl Error for LlmClientError {}

fn sanitize_prompt_override(value: Option<String>) -> Option<String> {
    let Some(value) = value else {
        return None;
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[derive(Debug)]
pub struct LlmAgentBehavior<C: LlmCompletionClient> {
    agent_id: String,
    config: LlmAgentConfig,
    prompt_overrides: LlmPromptOverrides,
    client: C,
    memory: AgentMemory,
    next_effect_intent_id: u64,
    pending_trace: Option<AgentDecisionTrace>,
    replan_guard_state: ActionReplanGuardState,
    active_execute_until: Option<ActiveExecuteUntil>,
    conversation_history: Vec<LlmChatMessageTrace>,
    conversation_trace_cursor: usize,
    last_action_summary: Option<PromptLastActionSummary>,
    pending_decision_rewrite: Option<DecisionRewriteReceipt>,
    known_factory_locations: BTreeMap<String, String>,
    known_factory_kinds_by_id: BTreeMap<String, String>,
    known_factory_kind_aliases: BTreeMap<String, String>,
    known_module_artifacts: BTreeMap<String, KnownModuleArtifactRecord>,
    known_installed_modules: BTreeMap<String, KnownInstalledModuleRecord>,
    move_distance_exceeded_targets: BTreeSet<String>,
    known_compound_availability_by_location: BTreeMap<String, i64>,
    depleted_mine_location_cooldowns: BTreeMap<String, u64>,
    mine_failure_streaks_by_location: BTreeMap<String, MineFailureStreak>,
    recipe_coverage: RecipeCoverageProgress,
}

impl<C: LlmCompletionClient> LlmAgentBehavior<C> {
    pub fn new(agent_id: impl Into<String>, config: LlmAgentConfig, client: C) -> Self {
        Self::new_with_memory(
            agent_id,
            config,
            client,
            AgentMemory::with_capacities(
                DEFAULT_SHORT_TERM_MEMORY_CAPACITY,
                DEFAULT_LONG_TERM_MEMORY_CAPACITY,
            ),
        )
    }

    pub fn new_with_memory(
        agent_id: impl Into<String>,
        config: LlmAgentConfig,
        client: C,
        memory: AgentMemory,
    ) -> Self {
        Self {
            agent_id: agent_id.into(),
            config,
            prompt_overrides: LlmPromptOverrides::default(),
            client,
            memory,
            next_effect_intent_id: 0,
            pending_trace: None,
            replan_guard_state: ActionReplanGuardState::default(),
            active_execute_until: None,
            conversation_history: Vec::new(),
            conversation_trace_cursor: 0,
            last_action_summary: None,
            pending_decision_rewrite: None,
            known_factory_locations: BTreeMap::new(),
            known_factory_kinds_by_id: BTreeMap::new(),
            known_factory_kind_aliases: BTreeMap::new(),
            known_module_artifacts: BTreeMap::new(),
            known_installed_modules: BTreeMap::new(),
            move_distance_exceeded_targets: BTreeSet::new(),
            known_compound_availability_by_location: BTreeMap::new(),
            depleted_mine_location_cooldowns: BTreeMap::new(),
            mine_failure_streaks_by_location: BTreeMap::new(),
            recipe_coverage: RecipeCoverageProgress::default(),
        }
    }

    pub fn apply_prompt_overrides(
        &mut self,
        system_prompt: Option<String>,
        short_term_goal: Option<String>,
        long_term_goal: Option<String>,
    ) {
        self.prompt_overrides.system_prompt = sanitize_prompt_override(system_prompt);
        self.prompt_overrides.short_term_goal = sanitize_prompt_override(short_term_goal);
        self.prompt_overrides.long_term_goal = sanitize_prompt_override(long_term_goal);
    }

    pub fn prompt_overrides(&self) -> LlmPromptOverrides {
        self.prompt_overrides.clone()
    }

    pub fn export_long_term_memory_entries(&self) -> Vec<LongTermMemoryEntry> {
        self.memory.export_long_term_entries()
    }

    pub fn restore_long_term_memory_entries(&mut self, entries: &[LongTermMemoryEntry]) {
        self.memory.restore_long_term_entries(entries.to_vec());
    }

    pub fn push_player_message(&mut self, time: u64, message: impl AsRef<str>) -> bool {
        self.append_conversation_message(time, LlmChatRole::Player, message.as_ref())
            .is_some()
    }
}

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
    use crate::geometry::GeoPos;
    use crate::simulator::{
        AgentBehavior, DecisionProviderError, DecisionRequest, DecisionResponse, LocationProfile,
        ObservedLocation, ObservedModuleLifecycleState, ObservedModuleMarketState,
        ObservedPowerMarketState, ObservedSocialState, ResourceKind, ResourceStock,
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

    #[derive(Debug)]
    struct StaticCompletionClient {
        result: LlmCompletionResult,
    }

    impl LlmCompletionClient for StaticCompletionClient {
        fn complete(
            &self,
            _request: &LlmCompletionRequest,
        ) -> Result<LlmCompletionResult, LlmClientError> {
            Ok(self.result.clone())
        }
    }

    fn make_observation() -> Observation {
        let mut self_resources = ResourceStock::new();
        let _ = self_resources.add(ResourceKind::Electricity, 20);
        let _ = self_resources.add(ResourceKind::Data, 10);

        Observation {
            time: 1,
            agent_id: "agent-llm".to_string(),
            pos: GeoPos::new(0, 0, 0),
            self_resources,
            visibility_range_cm: 10_000,
            visible_agents: Vec::new(),
            visible_locations: vec![ObservedLocation {
                location_id: "loc-home".to_string(),
                name: "Home".to_string(),
                pos: GeoPos::new(0, 0, 0),
                profile: LocationProfile::default(),
                distance_cm: 0,
            }],
            module_lifecycle: ObservedModuleLifecycleState::default(),
            module_market: ObservedModuleMarketState::default(),
            power_market: ObservedPowerMarketState::default(),
            social_state: ObservedSocialState::default(),
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

    #[test]
    fn restored_decision_flow_parses_provider_neutral_actions() {
        let turns = vec![LlmCompletionTurn::Decision {
            payload: serde_json::json!({
                "decision": "schedule_recipe",
                "factory_id": "factory-1",
                "recipe_id": "recipe.smelter.iron_ingot",
                "batches": 1
            }),
        }];

        let parsed = decision_flow::parse_llm_turn_payloads(turns.as_slice(), "agent-llm");
        match parsed.first().expect("parsed turn") {
            ParsedLlmTurn::Decision {
                decision: AgentDecision::Act(Action::ScheduleRecipe { recipe_id, .. }),
                parse_error: None,
                ..
            } => assert_eq!(recipe_id, "recipe.smelter.iron_ingot"),
            other => panic!("unexpected parsed turn: {other:?}"),
        }
    }

    #[test]
    fn restored_prompt_layer_builds_agent_prompt_without_direct_client() {
        let behavior = LlmAgentBehavior::new(
            "agent-llm",
            LlmAgentConfig::provider_backed(),
            StaticCompletionClient {
                result: LlmCompletionResult {
                    turns: Vec::new(),
                    output: String::new(),
                    model: Some("provider-backed-test".to_string()),
                    prompt_tokens: None,
                    completion_tokens: None,
                    total_tokens: None,
                },
            },
        );

        let system_prompt = behavior.system_prompt();
        assert!(system_prompt.contains("agent_submit_decision"));
        let prompt = behavior.user_prompt(&make_observation(), &[], 0, 4);
        assert!(prompt.contains("schedule_recipe"));
        assert!(prompt.contains("execute_until"));
        assert!(prompt.contains("observation"));
    }
}
