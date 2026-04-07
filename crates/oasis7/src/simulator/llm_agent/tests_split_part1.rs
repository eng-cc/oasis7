use super::*;
use crate::geometry::GeoPos;
use crate::simulator::{
    Action, LlmChatRole, Observation, ObservedAgent, ObservedLocation, PowerOrderSide,
    RejectReason, ResourceKind, ResourceOwner, ResourceStock, SocialAdjudicationDecision,
    SocialStake, WorldEvent, WorldEventKind,
};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::io::Read;
use std::io::Write;
use std::net::TcpListener;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Default, Clone)]
struct MockClient {
    output: Option<String>,
    err: Option<LlmClientError>,
}

struct EnvVarGuard {
    key: String,
    previous: Option<String>,
}

impl EnvVarGuard {
    fn capture(key: impl Into<String>) -> Self {
        let key = key.into();
        Self {
            previous: std::env::var(key.as_str()).ok(),
            key,
        }
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var(self.key.as_str(), value),
            None => std::env::remove_var(self.key.as_str()),
        }
    }
}

fn llm_env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn completion_turn_from_value(value: serde_json::Value) -> LlmCompletionTurn {
    if value
        .get("type")
        .and_then(|value| value.as_str())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("module_call"))
    {
        return LlmCompletionTurn::ModuleCall {
            module: value
                .get("module")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
            args: value
                .get("args")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
        };
    }

    LlmCompletionTurn::Decision { payload: value }
}

fn next_json_start(raw: &str, from: usize) -> Option<usize> {
    raw.get(from..)?
        .char_indices()
        .find_map(|(offset, ch)| match ch {
            '{' | '[' => Some(from + offset),
            _ => None,
        })
}

fn extract_json_block_from(raw: &str, start: usize) -> Option<(usize, usize)> {
    let open_char = raw.get(start..)?.chars().next()?;
    if open_char != '{' && open_char != '[' {
        return None;
    }
    let close_char = if open_char == '{' { '}' } else { ']' };

    let mut depth: u32 = 0;
    let mut in_string = false;
    let mut escaped = false;

    for (offset, ch) in raw[start..].char_indices() {
        let index = start + offset;
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            c if c == open_char => depth = depth.saturating_add(1),
            c if c == close_char => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some((start, index));
                }
            }
            _ => {}
        }
    }

    None
}

fn extract_json_blocks(raw: &str) -> Vec<&str> {
    let mut blocks = Vec::new();
    let mut cursor = 0_usize;

    while let Some(start) = next_json_start(raw, cursor) {
        let Some((_, end)) = extract_json_block_from(raw, start) else {
            break;
        };
        if let Some(block) = raw.get(start..=end) {
            blocks.push(block);
        }
        cursor = end.saturating_add(1);
    }

    blocks
}

pub(super) fn completion_turns_from_output(output: &str) -> Vec<LlmCompletionTurn> {
    let blocks = extract_json_blocks(output);
    if blocks.is_empty() {
        return Vec::new();
    }
    blocks
        .into_iter()
        .filter_map(|block| serde_json::from_str::<serde_json::Value>(block).ok())
        .map(completion_turn_from_value)
        .collect()
}

impl LlmCompletionClient for MockClient {
    fn complete(
        &self,
        request: &LlmCompletionRequest,
    ) -> Result<LlmCompletionResult, LlmClientError> {
        if let Some(err) = &self.err {
            return Err(err.clone());
        }
        let output = self
            .output
            .clone()
            .unwrap_or_else(|| "{\"decision\":\"wait\"}".to_string());
        Ok(LlmCompletionResult {
            turns: completion_turns_from_output(output.as_str()),
            output,
            model: Some(request.model.clone()),
            prompt_tokens: Some(12),
            completion_tokens: Some(4),
            total_tokens: Some(16),
        })
    }
}

#[derive(Debug, Clone)]
struct SequenceMockClient {
    outputs: RefCell<VecDeque<String>>,
    model: String,
}

impl SequenceMockClient {
    fn new(outputs: Vec<String>) -> Self {
        Self {
            outputs: RefCell::new(outputs.into()),
            model: "gpt-test".to_string(),
        }
    }
}

impl LlmCompletionClient for SequenceMockClient {
    fn complete(
        &self,
        _request: &LlmCompletionRequest,
    ) -> Result<LlmCompletionResult, LlmClientError> {
        let output = self
            .outputs
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| "{\"decision\":\"wait\"}".to_string());
        Ok(LlmCompletionResult {
            turns: completion_turns_from_output(output.as_str()),
            output,
            model: Some(self.model.clone()),
            prompt_tokens: Some(12),
            completion_tokens: Some(4),
            total_tokens: Some(16),
        })
    }
}

#[derive(Debug, Clone)]
struct StressMockClient {
    calls: Arc<AtomicUsize>,
}

impl StressMockClient {
    fn new(calls: Arc<AtomicUsize>) -> Self {
        Self { calls }
    }
}

impl LlmCompletionClient for StressMockClient {
    fn complete(
        &self,
        _request: &LlmCompletionRequest,
    ) -> Result<LlmCompletionResult, LlmClientError> {
        let call_index = self.calls.fetch_add(1, Ordering::SeqCst);
        let output = match call_index % 5 {
            0 => r#"{"type":"plan","missing":["memory"],"next":"module_call"}"#,
            1 => r#"{"type":"module_call","module":"memory.short_term.recent","args":{"limit":4}}"#,
            2 => r#"{"type":"decision_draft","decision":{"decision":"move_agent","to":"loc-2"},"confidence":0.64,"need_verify":true}"#,
            3 => "not-json",
            _ => r#"{"decision":"move_agent","to":"loc-2"}"#,
        }
        .to_string();

        Ok(LlmCompletionResult {
            turns: completion_turns_from_output(output.as_str()),
            output,
            model: Some("gpt-stress".to_string()),
            prompt_tokens: Some(16),
            completion_tokens: Some(6),
            total_tokens: Some(22),
        })
    }
}

#[derive(Debug, Clone)]
struct CountingSequenceMockClient {
    outputs: RefCell<VecDeque<String>>,
    calls: Arc<AtomicUsize>,
}

impl CountingSequenceMockClient {
    fn new(outputs: Vec<String>, calls: Arc<AtomicUsize>) -> Self {
        Self {
            outputs: RefCell::new(outputs.into()),
            calls,
        }
    }
}

impl LlmCompletionClient for CountingSequenceMockClient {
    fn complete(
        &self,
        _request: &LlmCompletionRequest,
    ) -> Result<LlmCompletionResult, LlmClientError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let output = self
            .outputs
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| "{\"decision\":\"wait\"}".to_string());
        Ok(LlmCompletionResult {
            turns: completion_turns_from_output(output.as_str()),
            output,
            model: Some("gpt-count".to_string()),
            prompt_tokens: Some(12),
            completion_tokens: Some(4),
            total_tokens: Some(16),
        })
    }
}

fn make_observation() -> Observation {
    let mut self_resources = ResourceStock::default();
    self_resources
        .add(ResourceKind::Electricity, 30)
        .expect("seed electricity");
    self_resources
        .add(ResourceKind::Data, 0)
        .expect("seed data");
    self_resources
        .add(ResourceKind::Data, 12)
        .expect("seed data");

    Observation {
        time: 7,
        agent_id: "agent-1".to_string(),
        pos: GeoPos {
            x_cm: 0.0,
            y_cm: 0.0,
            z_cm: 0.0,
        },
        self_resources,
        visibility_range_cm: 100,
        visible_agents: vec![ObservedAgent {
            agent_id: "agent-2".to_string(),
            location_id: "loc-2".to_string(),
            pos: GeoPos {
                x_cm: 1.0,
                y_cm: 0.0,
                z_cm: 0.0,
            },
            distance_cm: 1,
        }],
        visible_locations: vec![ObservedLocation {
            location_id: "loc-2".to_string(),
            name: "outpost".to_string(),
            pos: GeoPos {
                x_cm: 1.0,
                y_cm: 0.0,
                z_cm: 0.0,
            },
            profile: Default::default(),
            distance_cm: 1,
        }],
        module_lifecycle: Default::default(),
        module_market: Default::default(),
        power_market: Default::default(),
        social_state: Default::default(),
    }
}

fn make_dense_observation(time: u64, extra: usize) -> Observation {
    let mut observation = make_observation();
    observation.time = time;

    for index in 0..extra {
        observation.visible_agents.push(ObservedAgent {
            agent_id: format!("agent-extra-{index}"),
            location_id: format!("loc-extra-{index}"),
            pos: GeoPos {
                x_cm: 100.0 + index as f64,
                y_cm: (index as f64) * 0.5,
                z_cm: 0.0,
            },
            distance_cm: 100 + index as i64,
        });
        observation.visible_locations.push(ObservedLocation {
            location_id: format!("loc-extra-{index}"),
            name: format!("outpost-extra-{index}"),
            pos: GeoPos {
                x_cm: 100.0 + index as f64,
                y_cm: (index as f64) * 0.5,
                z_cm: 0.0,
            },
            profile: Default::default(),
            distance_cm: 100 + index as i64,
        });
    }

    observation
}

fn base_config() -> LlmAgentConfig {
    LlmAgentConfig {
        model: "gpt-test".to_string(),
        base_url: "https://example.invalid/v1".to_string(),
        api_key: "test-key".to_string(),
        timeout_ms: 1000,
        system_prompt: "prompt".to_string(),
        short_term_goal: "short-goal".to_string(),
        long_term_goal: "long-goal".to_string(),
        max_module_calls: 3,
        max_decision_steps: 4,
        max_repair_rounds: 1,
        prompt_max_history_items: 4,
        prompt_profile: LlmPromptProfile::Balanced,
        force_replan_after_same_action: DEFAULT_LLM_FORCE_REPLAN_AFTER_SAME_ACTION,
        harvest_max_amount_cap: DEFAULT_LLM_HARVEST_MAX_AMOUNT_CAP,
        execute_until_auto_reenter_ticks: DEFAULT_LLM_EXECUTE_UNTIL_AUTO_REENTER_TICKS,
        llm_debug_mode: DEFAULT_LLM_DEBUG_MODE,
    }
}

#[test]
fn llm_prompt_profile_uses_relaxed_token_budget_for_stability() {
    let compact = LlmPromptProfile::Compact.prompt_budget();
    assert_eq!(compact.context_window_tokens, 4_096);
    assert_eq!(compact.reserved_output_tokens, 768);
    assert_eq!(compact.safety_margin_tokens, 352);
    assert_eq!(compact.effective_input_budget_tokens(), 2_976);

    let balanced = LlmPromptProfile::Balanced.prompt_budget();
    assert_eq!(balanced.context_window_tokens, 4_608);
    assert_eq!(balanced.reserved_output_tokens, 896);
    assert_eq!(balanced.safety_margin_tokens, 480);
    assert_eq!(balanced.effective_input_budget_tokens(), 3_232);
}

#[test]
fn llm_config_uses_default_system_prompt() {
    let mut vars = BTreeMap::new();
    vars.insert(ENV_LLM_MODEL.to_string(), "gpt-4o-mini".to_string());
    vars.insert(
        ENV_LLM_BASE_URL.to_string(),
        "https://api.example.com/v1".to_string(),
    );
    vars.insert(ENV_LLM_API_KEY.to_string(), "secret".to_string());

    let config = LlmAgentConfig::from_env_with(|key| vars.get(key).cloned(), "").unwrap();
    assert_eq!(config.system_prompt, DEFAULT_LLM_SYSTEM_PROMPT);
    assert_eq!(config.timeout_ms, DEFAULT_LLM_TIMEOUT_MS);
    assert_eq!(config.short_term_goal, DEFAULT_LLM_SHORT_TERM_GOAL);
    assert_eq!(config.long_term_goal, DEFAULT_LLM_LONG_TERM_GOAL);
    assert_eq!(config.max_module_calls, DEFAULT_LLM_MAX_MODULE_CALLS);
    assert_eq!(config.max_decision_steps, DEFAULT_LLM_MAX_DECISION_STEPS);
    assert_eq!(config.max_repair_rounds, DEFAULT_LLM_MAX_REPAIR_ROUNDS);
    assert_eq!(
        config.prompt_max_history_items,
        DEFAULT_LLM_PROMPT_MAX_HISTORY_ITEMS
    );
    assert_eq!(config.prompt_profile, DEFAULT_LLM_PROMPT_PROFILE);
    assert_eq!(
        config.force_replan_after_same_action,
        DEFAULT_LLM_FORCE_REPLAN_AFTER_SAME_ACTION
    );
    assert_eq!(
        config.harvest_max_amount_cap,
        DEFAULT_LLM_HARVEST_MAX_AMOUNT_CAP
    );
    assert_eq!(
        config.execute_until_auto_reenter_ticks,
        DEFAULT_LLM_EXECUTE_UNTIL_AUTO_REENTER_TICKS
    );
    assert_eq!(config.llm_debug_mode, DEFAULT_LLM_DEBUG_MODE);
}

#[test]
fn llm_config_reads_system_prompt_from_env() {
    let mut vars = BTreeMap::new();
    vars.insert(ENV_LLM_MODEL.to_string(), "gpt-4o-mini".to_string());
    vars.insert(
        ENV_LLM_BASE_URL.to_string(),
        "https://api.example.com/v1".to_string(),
    );
    vars.insert(ENV_LLM_API_KEY.to_string(), "secret".to_string());
    vars.insert(
        ENV_LLM_SYSTEM_PROMPT.to_string(),
        "自定义 system prompt".to_string(),
    );
    vars.insert(ENV_LLM_TIMEOUT_MS.to_string(), "2000".to_string());

    let config = LlmAgentConfig::from_env_with(|key| vars.get(key).cloned(), "").unwrap();
    assert_eq!(config.system_prompt, "自定义 system prompt");
    assert_eq!(config.timeout_ms, 2000);
}

#[test]
fn llm_config_reads_goal_and_module_limits_from_env() {
    let mut vars = BTreeMap::new();
    vars.insert(ENV_LLM_MODEL.to_string(), "gpt-4o-mini".to_string());
    vars.insert(
        ENV_LLM_BASE_URL.to_string(),
        "https://api.example.com/v1".to_string(),
    );
    vars.insert(ENV_LLM_API_KEY.to_string(), "secret".to_string());
    vars.insert(
        ENV_LLM_SHORT_TERM_GOAL.to_string(),
        "保持本轮高效".to_string(),
    );
    vars.insert(
        ENV_LLM_LONG_TERM_GOAL.to_string(),
        "建立长期资源优势".to_string(),
    );
    vars.insert(ENV_LLM_MAX_MODULE_CALLS.to_string(), "5".to_string());

    let config = LlmAgentConfig::from_env_with(|key| vars.get(key).cloned(), "").unwrap();
    assert_eq!(config.short_term_goal, "保持本轮高效");
    assert_eq!(config.long_term_goal, "建立长期资源优势");
    assert_eq!(config.max_module_calls, 5);
}

#[test]
fn llm_config_reads_multistep_and_prompt_fields_from_env() {
    let mut vars = BTreeMap::new();
    vars.insert(ENV_LLM_MODEL.to_string(), "gpt-4o-mini".to_string());
    vars.insert(
        ENV_LLM_BASE_URL.to_string(),
        "https://api.example.com/v1".to_string(),
    );
    vars.insert(ENV_LLM_API_KEY.to_string(), "secret".to_string());
    vars.insert(ENV_LLM_MAX_DECISION_STEPS.to_string(), "6".to_string());
    vars.insert(ENV_LLM_MAX_REPAIR_ROUNDS.to_string(), "2".to_string());
    vars.insert(
        ENV_LLM_PROMPT_MAX_HISTORY_ITEMS.to_string(),
        "7".to_string(),
    );
    vars.insert(ENV_LLM_PROMPT_PROFILE.to_string(), "compact".to_string());
    vars.insert(
        ENV_LLM_FORCE_REPLAN_AFTER_SAME_ACTION.to_string(),
        "9".to_string(),
    );
    vars.insert(ENV_LLM_HARVEST_MAX_AMOUNT_CAP.to_string(), "88".to_string());
    vars.insert(
        ENV_LLM_EXECUTE_UNTIL_AUTO_REENTER_TICKS.to_string(),
        "5".to_string(),
    );

    let config = LlmAgentConfig::from_env_with(|key| vars.get(key).cloned(), "").unwrap();
    assert_eq!(config.max_decision_steps, 6);
    assert_eq!(config.max_repair_rounds, 2);
    assert_eq!(config.prompt_max_history_items, 7);
    assert_eq!(config.prompt_profile, LlmPromptProfile::Compact);
    assert_eq!(config.force_replan_after_same_action, 9);
    assert_eq!(config.harvest_max_amount_cap, 88);
    assert_eq!(config.execute_until_auto_reenter_ticks, 5);
}

#[test]
fn llm_config_reads_debug_mode_from_env() {
    let mut vars = BTreeMap::new();
    vars.insert(ENV_LLM_MODEL.to_string(), "gpt-4o-mini".to_string());
    vars.insert(
        ENV_LLM_BASE_URL.to_string(),
        "https://api.example.com/v1".to_string(),
    );
    vars.insert(ENV_LLM_API_KEY.to_string(), "secret".to_string());
    vars.insert(ENV_LLM_DEBUG_MODE.to_string(), "true".to_string());

    let config = LlmAgentConfig::from_env_with(|key| vars.get(key).cloned(), "").unwrap();
    assert!(config.llm_debug_mode);
}

#[test]
fn llm_config_rejects_invalid_debug_mode() {
    let mut vars = BTreeMap::new();
    vars.insert(ENV_LLM_MODEL.to_string(), "gpt-4o-mini".to_string());
    vars.insert(
        ENV_LLM_BASE_URL.to_string(),
        "https://api.example.com/v1".to_string(),
    );
    vars.insert(ENV_LLM_API_KEY.to_string(), "secret".to_string());
    vars.insert(ENV_LLM_DEBUG_MODE.to_string(), "maybe".to_string());

    let err = LlmAgentConfig::from_env_with(|key| vars.get(key).cloned(), "").unwrap_err();
    assert!(matches!(err, LlmConfigError::InvalidDebugMode { .. }));
}

#[test]
fn llm_config_rejects_invalid_harvest_max_amount_cap() {
    let mut vars = BTreeMap::new();
    vars.insert(ENV_LLM_MODEL.to_string(), "gpt-4o-mini".to_string());
    vars.insert(
        ENV_LLM_BASE_URL.to_string(),
        "https://api.example.com/v1".to_string(),
    );
    vars.insert(ENV_LLM_API_KEY.to_string(), "secret".to_string());
    vars.insert(ENV_LLM_HARVEST_MAX_AMOUNT_CAP.to_string(), "0".to_string());

    let err = LlmAgentConfig::from_env_with(|key| vars.get(key).cloned(), "").unwrap_err();
    assert!(matches!(
        err,
        LlmConfigError::InvalidHarvestMaxAmountCap { .. }
    ));
}

#[test]
fn llm_config_rejects_invalid_execute_until_auto_reenter_ticks() {
    let mut vars = BTreeMap::new();
    vars.insert(ENV_LLM_MODEL.to_string(), "gpt-4o-mini".to_string());
    vars.insert(
        ENV_LLM_BASE_URL.to_string(),
        "https://api.example.com/v1".to_string(),
    );
    vars.insert(ENV_LLM_API_KEY.to_string(), "secret".to_string());
    vars.insert(
        ENV_LLM_EXECUTE_UNTIL_AUTO_REENTER_TICKS.to_string(),
        "-1".to_string(),
    );

    let err = LlmAgentConfig::from_env_with(|key| vars.get(key).cloned(), "").unwrap_err();
    assert!(matches!(
        err,
        LlmConfigError::InvalidExecuteUntilAutoReenterTicks { .. }
    ));
}

#[test]
fn llm_config_rejects_invalid_prompt_profile() {
    let mut vars = BTreeMap::new();
    vars.insert(ENV_LLM_MODEL.to_string(), "gpt-4o-mini".to_string());
    vars.insert(
        ENV_LLM_BASE_URL.to_string(),
        "https://api.example.com/v1".to_string(),
    );
    vars.insert(ENV_LLM_API_KEY.to_string(), "secret".to_string());
    vars.insert(
        ENV_LLM_PROMPT_PROFILE.to_string(),
        "invalid-profile".to_string(),
    );

    let err = LlmAgentConfig::from_env_with(|key| vars.get(key).cloned(), "").unwrap_err();
    assert!(matches!(err, LlmConfigError::InvalidPromptProfile { .. }));
}

#[test]
fn llm_config_agent_scoped_goal_overrides_global_value() {
    let mut vars = BTreeMap::new();
    vars.insert(ENV_LLM_MODEL.to_string(), "gpt-4o-mini".to_string());
    vars.insert(
        ENV_LLM_BASE_URL.to_string(),
        "https://api.example.com/v1".to_string(),
    );
    vars.insert(ENV_LLM_API_KEY.to_string(), "secret".to_string());
    vars.insert(
        ENV_LLM_SHORT_TERM_GOAL.to_string(),
        "global-short".to_string(),
    );
    vars.insert(
        "OASIS7_LLM_SHORT_TERM_GOAL_AGENT_1".to_string(),
        "agent-short".to_string(),
    );

    let config = LlmAgentConfig::from_env_with(|key| vars.get(key).cloned(), "agent-1").unwrap();
    assert_eq!(config.short_term_goal, "agent-short");
    assert_eq!(config.long_term_goal, DEFAULT_LLM_LONG_TERM_GOAL);
}

#[test]
fn llm_env_var_reads_oasis7_prefix() {
    let _env_lock = llm_env_lock().lock().expect("env lock");
    let _primary_guard = EnvVarGuard::capture(ENV_LLM_MODEL);
    std::env::set_var(ENV_LLM_MODEL, "oasis7-model");

    assert_eq!(llm_env_var(ENV_LLM_MODEL).as_deref(), Some("oasis7-model"));
}

#[test]
fn llm_env_var_rejects_removed_old_brand_prefix() {
    let _env_lock = llm_env_lock().lock().expect("env lock");
    let _primary_guard = EnvVarGuard::capture(ENV_LLM_MODEL);
    let removed_old_brand_model = removed_old_brand_llm_env("MODEL");
    let _removed_old_brand_guard = EnvVarGuard::capture(removed_old_brand_model.as_str());
    std::env::remove_var(ENV_LLM_MODEL);
    std::env::set_var(removed_old_brand_model.as_str(), "removed-old-brand-model");

    assert!(llm_env_var(ENV_LLM_MODEL).is_none());
}

#[test]
fn llm_config_from_env_for_agent_rejects_removed_old_brand_prefix() {
    let _env_lock = llm_env_lock().lock().expect("env lock");
    let _model_guard = EnvVarGuard::capture(ENV_LLM_MODEL);
    let _base_url_guard = EnvVarGuard::capture(ENV_LLM_BASE_URL);
    let _api_key_guard = EnvVarGuard::capture(ENV_LLM_API_KEY);
    let removed_old_brand_goal = removed_old_brand_llm_env("SHORT_TERM_GOAL_AGENT_1");
    let removed_old_brand_model = removed_old_brand_llm_env("MODEL");
    let removed_old_brand_base_url = removed_old_brand_llm_env("BASE_URL");
    let removed_old_brand_api_key = removed_old_brand_llm_env("API_KEY");
    let _removed_old_brand_goal_guard = EnvVarGuard::capture(removed_old_brand_goal.as_str());
    let _removed_old_brand_model_guard = EnvVarGuard::capture(removed_old_brand_model.as_str());
    let _removed_old_brand_base_url_guard =
        EnvVarGuard::capture(removed_old_brand_base_url.as_str());
    let _removed_old_brand_api_key_guard =
        EnvVarGuard::capture(removed_old_brand_api_key.as_str());
    std::env::remove_var(ENV_LLM_MODEL);
    std::env::remove_var(ENV_LLM_BASE_URL);
    std::env::remove_var(ENV_LLM_API_KEY);
    std::env::set_var(removed_old_brand_model.as_str(), "removed-old-brand-model");
    std::env::set_var(
        removed_old_brand_base_url.as_str(),
        "https://removed-old-brand.example.com/v1",
    );
    std::env::set_var(removed_old_brand_api_key.as_str(), "removed-old-brand-secret");
    std::env::set_var(removed_old_brand_goal.as_str(), "removed-old-brand-agent-short");

    let error = LlmAgentConfig::from_env_for_agent("agent-1").expect_err("missing oasis7 env");
    assert!(matches!(
        error,
        LlmConfigError::MissingEnv { key } if key == ENV_LLM_MODEL
    ));
}

fn removed_old_brand_llm_env(suffix: &str) -> String {
    ["AGENT", "WORLD", "LLM", suffix].join("_")
}

#[test]
fn llm_config_reads_from_config_file() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path_buf = std::env::temp_dir().join(format!("oasis7-llm-config-{unique}.toml"));
    let path = Path::new(&path_buf);
    let content = r#"
model = "fallback-model"
model_provider = "ark"
profile = "default"

[model_providers.ark]
base_url = "https://api.example.com/v1"
auth_token = "secret"

[profiles.default]
model = "gpt-4o-mini"
model_provider = "ark"

[llm]
timeout_ms = 4567
"#;
    std::fs::write(path, content).unwrap();

    let config = LlmAgentConfig::from_config_file(path).unwrap();

    std::fs::remove_file(path).ok();

    assert_eq!(config.model, "gpt-4o-mini");
    assert_eq!(config.base_url, "https://api.example.com/v1");
    assert_eq!(config.api_key, "secret");
    assert_eq!(config.timeout_ms, 4567);
    assert_eq!(config.system_prompt, DEFAULT_LLM_SYSTEM_PROMPT);
    assert_eq!(config.max_decision_steps, DEFAULT_LLM_MAX_DECISION_STEPS);
    assert_eq!(config.max_repair_rounds, DEFAULT_LLM_MAX_REPAIR_ROUNDS);
    assert_eq!(
        config.prompt_max_history_items,
        DEFAULT_LLM_PROMPT_MAX_HISTORY_ITEMS
    );
    assert_eq!(config.prompt_profile, DEFAULT_LLM_PROMPT_PROFILE);
    assert_eq!(
        config.force_replan_after_same_action,
        DEFAULT_LLM_FORCE_REPLAN_AFTER_SAME_ACTION
    );
    assert_eq!(
        config.harvest_max_amount_cap,
        DEFAULT_LLM_HARVEST_MAX_AMOUNT_CAP
    );
    assert_eq!(
        config.execute_until_auto_reenter_ticks,
        DEFAULT_LLM_EXECUTE_UNTIL_AUTO_REENTER_TICKS
    );
    assert_eq!(config.llm_debug_mode, DEFAULT_LLM_DEBUG_MODE);
}

#[test]
fn llm_config_prefers_llm_table_core_fields_over_profile_defaults() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path_buf =
        std::env::temp_dir().join(format!("oasis7-llm-config-llm-core-priority-{unique}.toml"));
    let path = Path::new(&path_buf);
    let content = r#"
model_provider = "ark"
profile = "default"

[model_providers.ark]
base_url = "https://provider.example.com/v1"
auth_token = "provider-secret"

[profiles.default]
model = "profile-model"
model_provider = "ark"

[llm]
model = "llm-model"
base_url = "https://llm.example.com/v1"
api_key = "llm-secret"
"#;
    std::fs::write(path, content).unwrap();

    let config = LlmAgentConfig::from_config_file(path).unwrap();

    std::fs::remove_file(path).ok();

    assert_eq!(config.model, "llm-model");
    assert_eq!(config.base_url, "https://llm.example.com/v1");
    assert_eq!(config.api_key, "llm-secret");
}

#[test]
fn llm_config_reads_agent_scoped_goal_overrides_from_llm_agent_overrides_table() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path_buf = std::env::temp_dir().join(format!(
        "oasis7-llm-config-agent-goal-override-{unique}.toml"
    ));
    let path = Path::new(&path_buf);
    let content = r#"
model_provider = "ark"
profile = "default"

[model_providers.ark]
base_url = "https://api.example.com/v1"
auth_token = "secret"

[profiles.default]
model = "gpt-4o-mini"
model_provider = "ark"

[llm]
short_term_goal = "global-short"

[llm.agent_overrides.AGENT_1]
short_term_goal = "agent-short"
"#;
    std::fs::write(path, content).unwrap();

    let config = LlmAgentConfig::from_config_file_for_agent(path, "agent-1").unwrap();

    std::fs::remove_file(path).ok();

    assert_eq!(config.short_term_goal, "agent-short");
    assert_eq!(config.long_term_goal, DEFAULT_LLM_LONG_TERM_GOAL);
}

#[test]
fn normalize_openai_api_base_url_handles_suffix_variants() {
    assert_eq!(
        normalize_openai_api_base_url("https://api.example.com/v1"),
        "https://api.example.com/v1"
    );
    assert_eq!(
        normalize_openai_api_base_url("https://api.example.com/v1/"),
        "https://api.example.com/v1"
    );
    assert_eq!(
        normalize_openai_api_base_url("https://api.example.com/v1/chat/completions"),
        "https://api.example.com/v1"
    );
    assert_eq!(
        normalize_openai_api_base_url("https://api.example.com/v1/responses"),
        "https://api.example.com/v1"
    );
}

#[test]
fn openai_client_respects_configured_timeout_without_hidden_retry() {
    let mut config = base_config();
    config.timeout_ms = 200;
    config.base_url = spawn_slow_openai_like_server(Duration::from_millis(600));
    let client = OpenAiChatCompletionClient::from_config(&config).expect("client");
    let request = LlmCompletionRequest {
        model: config.model.clone(),
        system_prompt: config.system_prompt.clone(),
        user_prompt: "return wait".to_string(),
        debug_mode: false,
    };
    let started_at = Instant::now();
    let error = client.complete(&request).expect_err("request should time out");

    assert_eq!(client.request_timeout_ms, 200);
    assert!(
        started_at.elapsed() < Duration::from_secs(2),
        "client should fail fast near the configured timeout instead of retrying with a long fallback"
    );
    match error {
        LlmClientError::Http { message } => {
            assert!(
                message.contains("request timed out after 200ms"),
                "unexpected timeout message: {message}"
            );
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

fn spawn_slow_openai_like_server(response_delay: Duration) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind slow openai server");
    let bind = listener.local_addr().expect("listener addr");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
        let mut buffer = [0_u8; 4096];
        let _ = stream.read(&mut buffer);
        thread::sleep(response_delay);
        let body = serde_json::json!({
            "id": "resp_test",
            "object": "response",
            "model": "gpt-test",
            "output": [],
            "usage": {
                "input_tokens": 1,
                "output_tokens": 1,
                "total_tokens": 2
            }
        })
        .to_string();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
    });
    format!("http://127.0.0.1:{}/v1", bind.port())
}

#[test]
fn responses_tools_register_expected_function_names() {
    let tools = responses_tools();
    assert_eq!(tools.len(), 10);

    let names = tools
        .into_iter()
        .filter_map(|tool| match tool {
            Tool::Function(function_tool) => Some(function_tool.name),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        names,
        vec![
            OPENAI_TOOL_AGENT_MODULES_LIST.to_string(),
            OPENAI_TOOL_ENVIRONMENT_CURRENT_OBSERVATION.to_string(),
            OPENAI_TOOL_MEMORY_SHORT_TERM_RECENT.to_string(),
            OPENAI_TOOL_MEMORY_LONG_TERM_SEARCH.to_string(),
            OPENAI_TOOL_WORLD_RULES_GUIDE.to_string(),
            OPENAI_TOOL_MODULE_LIFECYCLE_STATUS.to_string(),
            OPENAI_TOOL_POWER_ORDER_BOOK_STATUS.to_string(),
            OPENAI_TOOL_MODULE_MARKET_STATUS.to_string(),
            OPENAI_TOOL_SOCIAL_STATE_STATUS.to_string(),
            OPENAI_TOOL_AGENT_SUBMIT_DECISION.to_string(),
        ]
    );
}

#[test]
fn responses_tools_register_debug_grant_tool_in_debug_mode_only() {
    let normal = responses_tools_with_debug_mode(false);
    assert_eq!(normal.len(), 10);
    let normal_names = normal
        .into_iter()
        .filter_map(|tool| match tool {
            Tool::Function(function_tool) => Some(function_tool.name),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(!normal_names.contains(&OPENAI_TOOL_AGENT_DEBUG_GRANT_RESOURCE.to_string()));

    let debug = responses_tools_with_debug_mode(true);
    assert_eq!(debug.len(), 11);
    let debug_names = debug
        .into_iter()
        .filter_map(|tool| match tool {
            Tool::Function(function_tool) => Some(function_tool.name),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(debug_names.contains(&OPENAI_TOOL_AGENT_DEBUG_GRANT_RESOURCE.to_string()));
}

#[test]
fn response_function_call_maps_to_typed_module_call_turn() {
    let output_item = OutputItem::FunctionCall(async_openai::types::responses::FunctionToolCall {
        arguments: "{\"limit\":5}".to_string(),
        call_id: "call_1".to_string(),
        name: OPENAI_TOOL_MEMORY_SHORT_TERM_RECENT.to_string(),
        id: None,
        status: None,
    });

    let turn = output_item_to_completion_turn(&output_item).expect("module_call turn");
    match turn {
        LlmCompletionTurn::ModuleCall { module, args } => {
            assert_eq!(module, "memory.short_term.recent");
            assert_eq!(args.get("limit").and_then(|v| v.as_i64()), Some(5));
        }
        other => panic!("expected module_call turn, got {other:?}"),
    }
}

#[test]
fn response_function_call_maps_module_lifecycle_status_tool_name() {
    let output_item = OutputItem::FunctionCall(async_openai::types::responses::FunctionToolCall {
        arguments: "{}".to_string(),
        call_id: "call_lifecycle".to_string(),
        name: OPENAI_TOOL_MODULE_LIFECYCLE_STATUS.to_string(),
        id: None,
        status: None,
    });

    let turn = output_item_to_completion_turn(&output_item).expect("module_call turn");
    match turn {
        LlmCompletionTurn::ModuleCall { module, args } => {
            assert_eq!(module, "module.lifecycle.status");
            assert_eq!(args, serde_json::json!({}));
        }
        other => panic!("expected module_call turn, got {other:?}"),
    }
}

#[test]
fn response_function_call_maps_world_rules_guide_tool_name() {
    let output_item = OutputItem::FunctionCall(async_openai::types::responses::FunctionToolCall {
        arguments: "{\"topic\":\"industry\"}".to_string(),
        call_id: "call_world_rules".to_string(),
        name: OPENAI_TOOL_WORLD_RULES_GUIDE.to_string(),
        id: None,
        status: None,
    });

    let turn = output_item_to_completion_turn(&output_item).expect("module_call turn");
    match turn {
        LlmCompletionTurn::ModuleCall { module, args } => {
            assert_eq!(module, "world.rules.guide");
            assert_eq!(
                args.get("topic").and_then(|value| value.as_str()),
                Some("industry")
            );
        }
        other => panic!("expected module_call turn, got {other:?}"),
    }
}

#[test]
fn response_function_call_maps_power_order_book_status_tool_name() {
    let output_item = OutputItem::FunctionCall(async_openai::types::responses::FunctionToolCall {
        arguments: "{\"limit_orders\":12}".to_string(),
        call_id: "call_order_book".to_string(),
        name: OPENAI_TOOL_POWER_ORDER_BOOK_STATUS.to_string(),
        id: None,
        status: None,
    });

    let turn = output_item_to_completion_turn(&output_item).expect("module_call turn");
    match turn {
        LlmCompletionTurn::ModuleCall { module, args } => {
            assert_eq!(module, "power.order_book.status");
            assert_eq!(
                args.get("limit_orders").and_then(|value| value.as_i64()),
                Some(12)
            );
        }
        other => panic!("expected module_call turn, got {other:?}"),
    }
}

#[test]
fn response_function_call_maps_module_market_status_tool_name() {
    let output_item = OutputItem::FunctionCall(async_openai::types::responses::FunctionToolCall {
        arguments: "{\"wasm_hash\":\"hash-1\",\"limit_listings\":6}".to_string(),
        call_id: "call_module_market".to_string(),
        name: OPENAI_TOOL_MODULE_MARKET_STATUS.to_string(),
        id: None,
        status: None,
    });

    let turn = output_item_to_completion_turn(&output_item).expect("module_call turn");
    match turn {
        LlmCompletionTurn::ModuleCall { module, args } => {
            assert_eq!(module, "module.market.status");
            assert_eq!(
                args.get("wasm_hash").and_then(|value| value.as_str()),
                Some("hash-1")
            );
            assert_eq!(
                args.get("limit_listings").and_then(|value| value.as_i64()),
                Some(6)
            );
        }
        other => panic!("expected module_call turn, got {other:?}"),
    }
}

#[test]
fn response_function_call_maps_social_state_status_tool_name() {
    let output_item = OutputItem::FunctionCall(async_openai::types::responses::FunctionToolCall {
        arguments: "{\"include_inactive\":false,\"limit_facts\":5}".to_string(),
        call_id: "call_social_state".to_string(),
        name: OPENAI_TOOL_SOCIAL_STATE_STATUS.to_string(),
        id: None,
        status: None,
    });

    let turn = output_item_to_completion_turn(&output_item).expect("module_call turn");
    match turn {
        LlmCompletionTurn::ModuleCall { module, args } => {
            assert_eq!(module, "social.state.status");
            assert_eq!(
                args.get("include_inactive")
                    .and_then(|value| value.as_bool()),
                Some(false)
            );
            assert_eq!(
                args.get("limit_facts").and_then(|value| value.as_i64()),
                Some(5)
            );
        }
        other => panic!("expected module_call turn, got {other:?}"),
    }
}

#[test]
fn responses_tools_module_lifecycle_schema_declares_filter_and_limits() {
    let lifecycle = responses_tools()
        .into_iter()
        .find_map(|tool| match tool {
            Tool::Function(function_tool)
                if function_tool.name == OPENAI_TOOL_MODULE_LIFECYCLE_STATUS =>
            {
                Some(function_tool)
            }
            _ => None,
        })
        .expect("module lifecycle tool exists");
    let parameters = lifecycle.parameters.expect("module lifecycle parameters");
    let properties = parameters
        .get("properties")
        .and_then(|value| value.as_object())
        .expect("module lifecycle properties");
    assert!(properties.contains_key("module_id"));
    assert!(properties.contains_key("limit_artifacts"));
    assert!(properties.contains_key("limit_installed"));
}

#[test]
fn response_function_call_invalid_json_arguments_are_preserved_as_raw() {
    let output_item = OutputItem::FunctionCall(async_openai::types::responses::FunctionToolCall {
        arguments: "not-json".to_string(),
        call_id: "call_2".to_string(),
        name: OPENAI_TOOL_AGENT_MODULES_LIST.to_string(),
        id: None,
        status: None,
    });

    let turn = output_item_to_completion_turn(&output_item).expect("module_call turn");
    match turn {
        LlmCompletionTurn::ModuleCall { args, .. } => {
            assert_eq!(
                args.get("_raw").and_then(|value| value.as_str()),
                Some("not-json")
            );
        }
        other => panic!("expected module_call turn, got {other:?}"),
    }
}

#[test]
fn response_function_call_maps_decision_tool_to_typed_decision_turn() {
    let output_item = OutputItem::FunctionCall(async_openai::types::responses::FunctionToolCall {
        arguments: "{\"decision\":\"wait_ticks\",\"ticks\":2}".to_string(),
        call_id: "call_decision".to_string(),
        name: OPENAI_TOOL_AGENT_SUBMIT_DECISION.to_string(),
        id: None,
        status: None,
    });

    let turn = output_item_to_completion_turn(&output_item).expect("decision turn");
    match turn {
        LlmCompletionTurn::Decision { payload } => {
            assert_eq!(
                payload.get("decision").and_then(|value| value.as_str()),
                Some("wait_ticks")
            );
            assert_eq!(
                payload.get("ticks").and_then(|value| value.as_i64()),
                Some(2)
            );
        }
        other => panic!("expected decision turn, got {other:?}"),
    }
}

#[test]
fn response_function_call_maps_debug_grant_tool_to_typed_decision_turn() {
    let output_item = OutputItem::FunctionCall(async_openai::types::responses::FunctionToolCall {
        arguments: "{\"owner\":\"self\",\"kind\":\"data\",\"amount\":3}".to_string(),
        call_id: "call_debug".to_string(),
        name: OPENAI_TOOL_AGENT_DEBUG_GRANT_RESOURCE.to_string(),
        id: None,
        status: None,
    });

    let turn = output_item_to_completion_turn(&output_item).expect("decision turn");
    match turn {
        LlmCompletionTurn::Decision { payload } => {
            assert_eq!(
                payload.get("decision").and_then(|value| value.as_str()),
                Some("debug_grant_resource")
            );
            assert_eq!(
                payload.get("kind").and_then(|value| value.as_str()),
                Some("data")
            );
            assert_eq!(
                payload.get("amount").and_then(|value| value.as_i64()),
                Some(3)
            );
        }
        other => panic!("expected decision turn, got {other:?}"),
    }
}
