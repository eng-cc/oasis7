//! LLM-powered agent behavior and OpenAI-compatible completion client.

use async_openai::config::OpenAIConfig;
use async_openai::error::OpenAIError;
use async_openai::types::responses::{
    CreateResponse, CreateResponseArgs, FunctionTool, OutputItem, Response, ResponseStreamEvent,
    Tool, ToolChoiceOptions, ToolChoiceParam,
};
use async_openai::Client as AsyncOpenAiClient;
use futures_util::StreamExt;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;

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

mod behavior_guardrails;
mod behavior_loop;
mod behavior_prompt;
mod behavior_runtime_helpers;
mod config_helpers;
mod decision_flow;
mod execution_controls;
mod memory_selector;
mod openai_payload;
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

use config_helpers::{
    goal_value, parse_non_negative_usize, parse_positive_i64, parse_positive_usize, required_env,
    toml_value_to_string,
};
use openai_payload::{
    build_responses_request_payload, completion_result_from_sdk_stream_events,
    normalize_openai_api_base_url,
};
#[cfg(test)]
use openai_payload::{
    output_item_to_completion_turn, responses_tools, responses_tools_with_debug_mode,
};

pub const ENV_LLM_MODEL: &str = "OASIS7_LLM_MODEL";
pub const ENV_LLM_BASE_URL: &str = "OASIS7_LLM_BASE_URL";
pub const ENV_LLM_API_KEY: &str = "OASIS7_LLM_API_KEY";
pub const ENV_LLM_TIMEOUT_MS: &str = "OASIS7_LLM_TIMEOUT_MS";
pub const ENV_LLM_SYSTEM_PROMPT: &str = "OASIS7_LLM_SYSTEM_PROMPT";
pub const ENV_LLM_SHORT_TERM_GOAL: &str = "OASIS7_LLM_SHORT_TERM_GOAL";
pub const ENV_LLM_LONG_TERM_GOAL: &str = "OASIS7_LLM_LONG_TERM_GOAL";
pub const ENV_LLM_MAX_MODULE_CALLS: &str = "OASIS7_LLM_MAX_MODULE_CALLS";
pub const ENV_LLM_MAX_DECISION_STEPS: &str = "OASIS7_LLM_MAX_DECISION_STEPS";
pub const ENV_LLM_MAX_REPAIR_ROUNDS: &str = "OASIS7_LLM_MAX_REPAIR_ROUNDS";
pub const ENV_LLM_PROMPT_MAX_HISTORY_ITEMS: &str = "OASIS7_LLM_PROMPT_MAX_HISTORY_ITEMS";
pub const ENV_LLM_PROMPT_PROFILE: &str = "OASIS7_LLM_PROMPT_PROFILE";
pub const ENV_LLM_FORCE_REPLAN_AFTER_SAME_ACTION: &str =
    "OASIS7_LLM_FORCE_REPLAN_AFTER_SAME_ACTION";
pub const ENV_LLM_HARVEST_MAX_AMOUNT_CAP: &str = "OASIS7_LLM_HARVEST_MAX_AMOUNT_CAP";
pub const ENV_LLM_EXECUTE_UNTIL_AUTO_REENTER_TICKS: &str =
    "OASIS7_LLM_EXECUTE_UNTIL_AUTO_REENTER_TICKS";
pub const ENV_LLM_DEBUG_MODE: &str = "OASIS7_LLM_DEBUG_MODE";
const TOML_LLM_TABLE: &str = "llm";
const TOML_LLM_MODEL: &str = "model";
const TOML_LLM_BASE_URL: &str = "base_url";
const TOML_LLM_API_KEY: &str = "api_key";
const TOML_LLM_TIMEOUT_MS: &str = "timeout_ms";
const TOML_LLM_SYSTEM_PROMPT: &str = "system_prompt";
const TOML_LLM_SHORT_TERM_GOAL: &str = "short_term_goal";
const TOML_LLM_LONG_TERM_GOAL: &str = "long_term_goal";
const TOML_LLM_MAX_MODULE_CALLS: &str = "max_module_calls";
const TOML_LLM_MAX_DECISION_STEPS: &str = "max_decision_steps";
const TOML_LLM_MAX_REPAIR_ROUNDS: &str = "max_repair_rounds";
const TOML_LLM_PROMPT_MAX_HISTORY_ITEMS: &str = "prompt_max_history_items";
const TOML_LLM_PROMPT_PROFILE: &str = "prompt_profile";
const TOML_LLM_FORCE_REPLAN_AFTER_SAME_ACTION: &str = "force_replan_after_same_action";
const TOML_LLM_HARVEST_MAX_AMOUNT_CAP: &str = "harvest_max_amount_cap";
const TOML_LLM_EXECUTE_UNTIL_AUTO_REENTER_TICKS: &str = "execute_until_auto_reenter_ticks";
const TOML_LLM_DEBUG_MODE: &str = "debug_mode";
const TOML_LLM_PROFILE: &str = "profile";
const TOML_LLM_MODEL_PROVIDER: &str = "model_provider";
const TOML_MODEL_PROVIDERS_TABLE: &str = "model_providers";
const TOML_PROFILES_TABLE: &str = "profiles";
const TOML_MODEL_PROVIDER_AUTH_TOKEN: &str = "auth_token";
const TOML_LLM_AGENT_OVERRIDES_TABLE: &str = "agent_overrides";
const ENV_LLM_SHORT_TERM_GOAL_AGENT_PREFIX: &str = "OASIS7_LLM_SHORT_TERM_GOAL_";
const ENV_LLM_LONG_TERM_GOAL_AGENT_PREFIX: &str = "OASIS7_LLM_LONG_TERM_GOAL_";

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

pub const DEFAULT_CONFIG_FILE_NAME: &str = "config.toml";
pub const DEFAULT_LLM_TIMEOUT_MS: u64 = 180_000;
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
const TRACKED_RECIPE_IDS: [&str; 6] = [
    "recipe.smelter.iron_ingot",
    "recipe.smelter.copper_wire",
    "recipe.smelter.polymer_resin",
    "recipe.assembler.control_chip",
    "recipe.assembler.motor_mk1",
    "recipe.assembler.logistics_drone",
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
    pub base_url: String,
    pub api_key: String,
    pub timeout_ms: u64,
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

fn parse_debug_mode_flag(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

impl LlmAgentConfig {
    pub fn from_default_sources() -> Result<Self, LlmConfigError> {
        Self::from_default_sources_for_agent("")
    }

    pub fn from_default_sources_for_agent(agent_id: &str) -> Result<Self, LlmConfigError> {
        let config_path = Path::new(DEFAULT_CONFIG_FILE_NAME);
        if config_path.exists() {
            return Self::from_config_file_for_agent(config_path, agent_id);
        }
        Self::from_env_for_agent(agent_id)
    }

    pub fn from_config_file(path: &Path) -> Result<Self, LlmConfigError> {
        Self::from_config_file_for_agent(path, "")
    }

    pub fn from_config_file_for_agent(path: &Path, agent_id: &str) -> Result<Self, LlmConfigError> {
        let content = fs::read_to_string(path).map_err(|err| LlmConfigError::ReadConfigFile {
            path: path.display().to_string(),
            message: err.to_string(),
        })?;
        let value: toml::Value =
            toml::from_str(&content).map_err(|err| LlmConfigError::ParseConfigFile {
                path: path.display().to_string(),
                message: err.to_string(),
            })?;
        let table = value
            .as_table()
            .ok_or_else(|| LlmConfigError::ParseConfigFile {
                path: path.display().to_string(),
                message: "root is not a TOML table".to_string(),
            })?;

        Self::from_env_with(
            |key| config_value_for_env_key(table, key).or_else(|| llm_env_var(key)),
            agent_id,
        )
    }

    pub fn from_env() -> Result<Self, LlmConfigError> {
        Self::from_env_for_agent("")
    }

    pub fn from_env_for_agent(agent_id: &str) -> Result<Self, LlmConfigError> {
        Self::from_env_with(llm_env_var, agent_id)
    }

    fn from_env_with<F>(mut getter: F, agent_id: &str) -> Result<Self, LlmConfigError>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let model = required_env(&mut getter, ENV_LLM_MODEL)?;
        let base_url = required_env(&mut getter, ENV_LLM_BASE_URL)?;
        let api_key = required_env(&mut getter, ENV_LLM_API_KEY)?;
        let timeout_ms = match getter(ENV_LLM_TIMEOUT_MS) {
            Some(value) => value
                .parse::<u64>()
                .map_err(|_| LlmConfigError::InvalidTimeout { value })?,
            None => DEFAULT_LLM_TIMEOUT_MS,
        };
        let system_prompt = getter(ENV_LLM_SYSTEM_PROMPT)
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_LLM_SYSTEM_PROMPT.to_string());
        let short_term_goal = goal_value(&mut getter, ENV_LLM_SHORT_TERM_GOAL, agent_id)
            .unwrap_or_else(|| DEFAULT_LLM_SHORT_TERM_GOAL.to_string());
        let long_term_goal = goal_value(&mut getter, ENV_LLM_LONG_TERM_GOAL, agent_id)
            .unwrap_or_else(|| DEFAULT_LLM_LONG_TERM_GOAL.to_string());
        let max_module_calls = parse_positive_usize(
            &mut getter,
            ENV_LLM_MAX_MODULE_CALLS,
            DEFAULT_LLM_MAX_MODULE_CALLS,
            |value| LlmConfigError::InvalidMaxModuleCalls { value },
        )?;
        let max_decision_steps = parse_positive_usize(
            &mut getter,
            ENV_LLM_MAX_DECISION_STEPS,
            DEFAULT_LLM_MAX_DECISION_STEPS,
            |value| LlmConfigError::InvalidMaxDecisionSteps { value },
        )?;
        let max_repair_rounds = parse_positive_usize(
            &mut getter,
            ENV_LLM_MAX_REPAIR_ROUNDS,
            DEFAULT_LLM_MAX_REPAIR_ROUNDS,
            |value| LlmConfigError::InvalidMaxRepairRounds { value },
        )?;
        let prompt_max_history_items = parse_positive_usize(
            &mut getter,
            ENV_LLM_PROMPT_MAX_HISTORY_ITEMS,
            DEFAULT_LLM_PROMPT_MAX_HISTORY_ITEMS,
            |value| LlmConfigError::InvalidPromptMaxHistoryItems { value },
        )?;
        let prompt_profile = match getter(ENV_LLM_PROMPT_PROFILE) {
            Some(value) => LlmPromptProfile::parse(value.as_str())
                .ok_or(LlmConfigError::InvalidPromptProfile { value })?,
            None => DEFAULT_LLM_PROMPT_PROFILE,
        };
        let force_replan_after_same_action = parse_non_negative_usize(
            &mut getter,
            ENV_LLM_FORCE_REPLAN_AFTER_SAME_ACTION,
            DEFAULT_LLM_FORCE_REPLAN_AFTER_SAME_ACTION,
            |value| LlmConfigError::InvalidForceReplanAfterSameAction { value },
        )?;
        let harvest_max_amount_cap = parse_positive_i64(
            &mut getter,
            ENV_LLM_HARVEST_MAX_AMOUNT_CAP,
            DEFAULT_LLM_HARVEST_MAX_AMOUNT_CAP,
            |value| LlmConfigError::InvalidHarvestMaxAmountCap { value },
        )?;
        let execute_until_auto_reenter_ticks = parse_non_negative_usize(
            &mut getter,
            ENV_LLM_EXECUTE_UNTIL_AUTO_REENTER_TICKS,
            DEFAULT_LLM_EXECUTE_UNTIL_AUTO_REENTER_TICKS,
            |value| LlmConfigError::InvalidExecuteUntilAutoReenterTicks { value },
        )?;
        let llm_debug_mode = match getter(ENV_LLM_DEBUG_MODE) {
            Some(value) => parse_debug_mode_flag(value.as_str())
                .ok_or(LlmConfigError::InvalidDebugMode { value })?,
            None => DEFAULT_LLM_DEBUG_MODE,
        };

        Ok(Self {
            model,
            base_url,
            api_key,
            timeout_ms,
            system_prompt,
            short_term_goal,
            long_term_goal,
            max_module_calls,
            max_decision_steps,
            max_repair_rounds,
            prompt_max_history_items,
            prompt_profile,
            force_replan_after_same_action,
            harvest_max_amount_cap,
            execute_until_auto_reenter_ticks,
            llm_debug_mode,
        })
    }

    fn prompt_budget(&self) -> PromptBudget {
        self.prompt_profile.prompt_budget()
    }

    fn memory_selector_config(&self) -> MemorySelectorConfig {
        self.prompt_profile.memory_selector_config()
    }
}

fn llm_env_var(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

fn llm_table<'a>(table: &'a toml::value::Table) -> Option<&'a toml::value::Table> {
    table.get(TOML_LLM_TABLE).and_then(toml::Value::as_table)
}

fn non_empty_toml_string(table: &toml::value::Table, key: &str) -> Option<String> {
    table
        .get(key)
        .and_then(toml_value_to_string)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn selected_profile_table<'a>(table: &'a toml::value::Table) -> Option<&'a toml::value::Table> {
    let profile_name = llm_table(table)
        .and_then(|llm| non_empty_toml_string(llm, TOML_LLM_PROFILE))
        .or_else(|| non_empty_toml_string(table, TOML_LLM_PROFILE))?;
    table
        .get(TOML_PROFILES_TABLE)
        .and_then(toml::Value::as_table)
        .and_then(|profiles| profiles.get(profile_name.as_str()))
        .and_then(toml::Value::as_table)
}

fn selected_model_provider_name(table: &toml::value::Table) -> Option<String> {
    if let Some(provider) =
        llm_table(table).and_then(|llm| non_empty_toml_string(llm, TOML_LLM_MODEL_PROVIDER))
    {
        return Some(provider);
    }
    if let Some(provider) = selected_profile_table(table)
        .and_then(|profile| non_empty_toml_string(profile, TOML_LLM_MODEL_PROVIDER))
    {
        return Some(provider);
    }
    if let Some(provider) = non_empty_toml_string(table, TOML_LLM_MODEL_PROVIDER) {
        return Some(provider);
    }
    let providers = table
        .get(TOML_MODEL_PROVIDERS_TABLE)
        .and_then(toml::Value::as_table)?;
    if providers.len() == 1 {
        return providers.keys().next().cloned();
    }
    None
}

fn selected_model_provider_table<'a>(
    table: &'a toml::value::Table,
) -> Option<&'a toml::value::Table> {
    let provider_name = selected_model_provider_name(table)?;
    table
        .get(TOML_MODEL_PROVIDERS_TABLE)
        .and_then(toml::Value::as_table)
        .and_then(|providers| providers.get(provider_name.as_str()))
        .and_then(toml::Value::as_table)
}

fn resolve_model_from_config(table: &toml::value::Table) -> Option<String> {
    llm_table(table)
        .and_then(|llm| non_empty_toml_string(llm, TOML_LLM_MODEL))
        .or_else(|| {
            selected_profile_table(table)
                .and_then(|profile| non_empty_toml_string(profile, TOML_LLM_MODEL))
        })
        .or_else(|| non_empty_toml_string(table, TOML_LLM_MODEL))
}

fn resolve_base_url_from_config(table: &toml::value::Table) -> Option<String> {
    llm_table(table)
        .and_then(|llm| non_empty_toml_string(llm, TOML_LLM_BASE_URL))
        .or_else(|| {
            selected_model_provider_table(table)
                .and_then(|provider| non_empty_toml_string(provider, TOML_LLM_BASE_URL))
        })
}

fn resolve_api_key_from_config(table: &toml::value::Table) -> Option<String> {
    llm_table(table)
        .and_then(|llm| non_empty_toml_string(llm, TOML_LLM_API_KEY))
        .or_else(|| {
            selected_model_provider_table(table).and_then(|provider| {
                non_empty_toml_string(provider, TOML_MODEL_PROVIDER_AUTH_TOKEN)
            })
        })
        .or_else(|| {
            selected_model_provider_table(table)
                .and_then(|provider| non_empty_toml_string(provider, TOML_LLM_API_KEY))
        })
}

fn resolve_llm_key_from_config(table: &toml::value::Table, key: &str) -> Option<String> {
    llm_table(table).and_then(|llm| non_empty_toml_string(llm, key))
}

fn resolve_agent_goal_override_from_config(
    table: &toml::value::Table,
    key: &str,
) -> Option<String> {
    let (normalized_agent, field) =
        if let Some(suffix) = key.strip_prefix(ENV_LLM_SHORT_TERM_GOAL_AGENT_PREFIX) {
            (suffix, TOML_LLM_SHORT_TERM_GOAL)
        } else if let Some(suffix) = key.strip_prefix(ENV_LLM_LONG_TERM_GOAL_AGENT_PREFIX) {
            (suffix, TOML_LLM_LONG_TERM_GOAL)
        } else {
            return None;
        };
    llm_table(table)
        .and_then(|llm| llm.get(TOML_LLM_AGENT_OVERRIDES_TABLE))
        .and_then(toml::Value::as_table)
        .and_then(|overrides| overrides.get(normalized_agent))
        .and_then(toml::Value::as_table)
        .and_then(|override_entry| non_empty_toml_string(override_entry, field))
}

fn config_value_for_env_key(table: &toml::value::Table, key: &str) -> Option<String> {
    match key {
        ENV_LLM_MODEL => resolve_model_from_config(table),
        ENV_LLM_BASE_URL => resolve_base_url_from_config(table),
        ENV_LLM_API_KEY => resolve_api_key_from_config(table),
        ENV_LLM_TIMEOUT_MS => resolve_llm_key_from_config(table, TOML_LLM_TIMEOUT_MS),
        ENV_LLM_SYSTEM_PROMPT => resolve_llm_key_from_config(table, TOML_LLM_SYSTEM_PROMPT),
        ENV_LLM_SHORT_TERM_GOAL => resolve_llm_key_from_config(table, TOML_LLM_SHORT_TERM_GOAL),
        ENV_LLM_LONG_TERM_GOAL => resolve_llm_key_from_config(table, TOML_LLM_LONG_TERM_GOAL),
        ENV_LLM_MAX_MODULE_CALLS => resolve_llm_key_from_config(table, TOML_LLM_MAX_MODULE_CALLS),
        ENV_LLM_MAX_DECISION_STEPS => {
            resolve_llm_key_from_config(table, TOML_LLM_MAX_DECISION_STEPS)
        }
        ENV_LLM_MAX_REPAIR_ROUNDS => resolve_llm_key_from_config(table, TOML_LLM_MAX_REPAIR_ROUNDS),
        ENV_LLM_PROMPT_MAX_HISTORY_ITEMS => {
            resolve_llm_key_from_config(table, TOML_LLM_PROMPT_MAX_HISTORY_ITEMS)
        }
        ENV_LLM_PROMPT_PROFILE => resolve_llm_key_from_config(table, TOML_LLM_PROMPT_PROFILE),
        ENV_LLM_FORCE_REPLAN_AFTER_SAME_ACTION => {
            resolve_llm_key_from_config(table, TOML_LLM_FORCE_REPLAN_AFTER_SAME_ACTION)
        }
        ENV_LLM_HARVEST_MAX_AMOUNT_CAP => {
            resolve_llm_key_from_config(table, TOML_LLM_HARVEST_MAX_AMOUNT_CAP)
        }
        ENV_LLM_EXECUTE_UNTIL_AUTO_REENTER_TICKS => {
            resolve_llm_key_from_config(table, TOML_LLM_EXECUTE_UNTIL_AUTO_REENTER_TICKS)
        }
        ENV_LLM_DEBUG_MODE => resolve_llm_key_from_config(table, TOML_LLM_DEBUG_MODE),
        _ => resolve_agent_goal_override_from_config(table, key),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmConfigError {
    MissingEnv { key: &'static str },
    EmptyEnv { key: &'static str },
    InvalidTimeout { value: String },
    InvalidMaxModuleCalls { value: String },
    InvalidMaxDecisionSteps { value: String },
    InvalidMaxRepairRounds { value: String },
    InvalidPromptMaxHistoryItems { value: String },
    InvalidPromptProfile { value: String },
    InvalidForceReplanAfterSameAction { value: String },
    InvalidHarvestMaxAmountCap { value: String },
    InvalidExecuteUntilAutoReenterTicks { value: String },
    InvalidDebugMode { value: String },
    ReadConfigFile { path: String, message: String },
    ParseConfigFile { path: String, message: String },
}

impl fmt::Display for LlmConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmConfigError::MissingEnv { key } => write!(f, "missing env variable: {key}"),
            LlmConfigError::EmptyEnv { key } => write!(f, "empty env variable: {key}"),
            LlmConfigError::InvalidTimeout { value } => {
                write!(f, "invalid timeout value: {value}")
            }
            LlmConfigError::InvalidMaxModuleCalls { value } => {
                write!(f, "invalid max module calls value: {value}")
            }
            LlmConfigError::InvalidMaxDecisionSteps { value } => {
                write!(f, "invalid max decision steps value: {value}")
            }
            LlmConfigError::InvalidMaxRepairRounds { value } => {
                write!(f, "invalid max repair rounds value: {value}")
            }
            LlmConfigError::InvalidPromptMaxHistoryItems { value } => {
                write!(f, "invalid prompt max history items value: {value}")
            }
            LlmConfigError::InvalidPromptProfile { value } => {
                write!(f, "invalid prompt profile value: {value}")
            }
            LlmConfigError::InvalidForceReplanAfterSameAction { value } => {
                write!(f, "invalid force replan after same action value: {value}")
            }
            LlmConfigError::InvalidHarvestMaxAmountCap { value } => {
                write!(f, "invalid harvest max amount cap value: {value}")
            }
            LlmConfigError::InvalidExecuteUntilAutoReenterTicks { value } => {
                write!(f, "invalid execute_until auto reenter ticks value: {value}")
            }
            LlmConfigError::InvalidDebugMode { value } => {
                write!(f, "invalid debug mode value: {value}")
            }
            LlmConfigError::ReadConfigFile { path, message } => {
                write!(f, "read config file failed ({path}): {message}")
            }
            LlmConfigError::ParseConfigFile { path, message } => {
                write!(f, "parse config file failed ({path}): {message}")
            }
        }
    }
}

impl Error for LlmConfigError {}

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

#[derive(Debug, Clone)]
pub struct OpenAiChatCompletionClient {
    client: AsyncOpenAiClient<OpenAIConfig>,
    request_timeout_ms: u64,
}

impl OpenAiChatCompletionClient {
    pub fn from_config(config: &LlmAgentConfig) -> Result<Self, LlmClientError> {
        let request_timeout_ms = config.timeout_ms.max(1);
        let api_base = normalize_openai_api_base_url(config.base_url.as_str());
        let client = Self::build_client(
            api_base.as_str(),
            config.api_key.as_str(),
            request_timeout_ms,
        )?;

        Ok(Self {
            client,
            request_timeout_ms,
        })
    }

    fn build_http_client(timeout_ms: u64) -> Result<reqwest::Client, LlmClientError> {
        #[cfg(target_arch = "wasm32")]
        let builder = {
            let _ = timeout_ms;
            reqwest::Client::builder()
        };

        #[cfg(not(target_arch = "wasm32"))]
        let builder =
            reqwest::Client::builder().timeout(std::time::Duration::from_millis(timeout_ms.max(1)));

        builder.build().map_err(|err| LlmClientError::BuildClient {
            message: err.to_string(),
        })
    }

    fn build_client(
        api_base: &str,
        api_key: &str,
        timeout_ms: u64,
    ) -> Result<AsyncOpenAiClient<OpenAIConfig>, LlmClientError> {
        let config = OpenAIConfig::new()
            .with_api_base(api_base.to_string())
            .with_api_key(api_key.to_string());

        let http_client = Self::build_http_client(timeout_ms)?;
        Ok(AsyncOpenAiClient::with_config(config).with_http_client(http_client))
    }

    fn send_responses_request(
        &self,
        client: &AsyncOpenAiClient<OpenAIConfig>,
        payload: CreateResponse,
    ) -> Result<LlmCompletionResult, OpenAiRequestError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| OpenAiRequestError::Other(err.to_string()))?;

        runtime.block_on(async {
            let mut stream = client
                .responses()
                .create_stream(payload)
                .await
                .map_err(OpenAiRequestError::from)?;
            let mut events = Vec::<ResponseStreamEvent>::new();
            while let Some(event) = stream.next().await {
                events.push(event.map_err(OpenAiRequestError::from)?);
            }
            completion_result_from_sdk_stream_events(events).map_err(OpenAiRequestError::Completion)
        })
    }
}

#[derive(Debug)]
enum OpenAiRequestError {
    Timeout(String),
    ParseBody(String),
    Completion(LlmClientError),
    Other(String),
}

impl From<OpenAIError> for OpenAiRequestError {
    fn from(value: OpenAIError) -> Self {
        fn error_chain_contains_timeout(err: &dyn Error) -> bool {
            let mut current = Some(err);
            while let Some(err) = current {
                let message = err.to_string().to_ascii_lowercase();
                if message.contains("timed out")
                    || message.contains("timeout")
                    || message.contains("deadline has elapsed")
                {
                    return true;
                }
                current = err.source();
            }
            false
        }

        match value {
            OpenAIError::Reqwest(err) if err.is_timeout() || error_chain_contains_timeout(&err) => {
                Self::Timeout(err.to_string())
            }
            OpenAIError::JSONDeserialize(_, raw_body) => Self::ParseBody(raw_body),
            OpenAIError::StreamError(err) if error_chain_contains_timeout(err.as_ref()) => {
                Self::Timeout(err.to_string())
            }
            other => Self::Other(other.to_string()),
        }
    }
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

const OPENAI_TOOL_AGENT_MODULES_LIST: &str = "agent_modules_list";
const OPENAI_TOOL_ENVIRONMENT_CURRENT_OBSERVATION: &str = "environment_current_observation";
const OPENAI_TOOL_MEMORY_SHORT_TERM_RECENT: &str = "memory_short_term_recent";
const OPENAI_TOOL_MEMORY_LONG_TERM_SEARCH: &str = "memory_long_term_search";
const OPENAI_TOOL_WORLD_RULES_GUIDE: &str = "world_rules_guide";
const OPENAI_TOOL_MODULE_LIFECYCLE_STATUS: &str = "module_lifecycle_status";
const OPENAI_TOOL_POWER_ORDER_BOOK_STATUS: &str = "power_order_book_status";
const OPENAI_TOOL_MODULE_MARKET_STATUS: &str = "module_market_status";
const OPENAI_TOOL_SOCIAL_STATE_STATUS: &str = "social_state_status";
const OPENAI_TOOL_AGENT_SUBMIT_DECISION: &str = "agent_submit_decision";
const OPENAI_TOOL_AGENT_DEBUG_GRANT_RESOURCE: &str = "agent_debug_grant_resource";

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

impl LlmCompletionClient for OpenAiChatCompletionClient {
    fn complete(
        &self,
        request: &LlmCompletionRequest,
    ) -> Result<LlmCompletionResult, LlmClientError> {
        let payload = build_responses_request_payload(request)?;

        match self.send_responses_request(&self.client, payload.clone()) {
            Ok(result) => return Ok(result),
            Err(OpenAiRequestError::ParseBody(raw_body)) => {
                return Err(LlmClientError::DecodeResponse {
                    message: format!(
                        "responses sdk decode failed (primary request): {}",
                        summarize_trace_text(raw_body.as_str(), 320)
                    ),
                });
            }
            Err(OpenAiRequestError::Timeout(err)) => {
                return Err(LlmClientError::Http {
                    message: format!(
                        "request timed out after {}ms: {}",
                        self.request_timeout_ms, err
                    ),
                });
            }
            Err(OpenAiRequestError::Completion(err)) => {
                return Err(err);
            }
            Err(OpenAiRequestError::Other(err)) => {
                return Err(LlmClientError::Http { message: err });
            }
        }
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

impl LlmAgentBehavior<OpenAiChatCompletionClient> {
    pub fn from_env(agent_id: impl Into<String>) -> Result<Self, LlmAgentBuildError> {
        let agent_id = agent_id.into();
        let config = LlmAgentConfig::from_default_sources_for_agent(agent_id.as_str())
            .map_err(LlmAgentBuildError::Config)?;
        let client =
            OpenAiChatCompletionClient::from_config(&config).map_err(LlmAgentBuildError::Client)?;
        Ok(Self::new(agent_id, config, client))
    }
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
pub enum LlmAgentBuildError {
    Config(LlmConfigError),
    Client(LlmClientError),
}

impl fmt::Display for LlmAgentBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmAgentBuildError::Config(err) => write!(f, "llm config error: {err}"),
            LlmAgentBuildError::Client(err) => write!(f, "llm client error: {err}"),
        }
    }
}

impl Error for LlmAgentBuildError {}

#[cfg(test)]
mod tests;
