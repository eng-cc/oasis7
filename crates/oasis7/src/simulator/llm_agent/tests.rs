use super::*;
use crate::geometry::GeoPos;
use crate::simulator::{
    Action, LocationProfile, Observation, ObservedAgent, ObservedLocation,
    ObservedModuleLifecycleState, ObservedModuleMarketState, ObservedPowerMarketState,
    ObservedSocialState, ResourceKind, ResourceStock,
};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Debug, Default, Clone)]
struct MockClient {
    output: Option<String>,
    err: Option<LlmClientError>,
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

fn completion_turns_from_output(output: &str) -> Vec<LlmCompletionTurn> {
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
            model: "provider-neutral-test".to_string(),
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
            model: Some("provider-neutral-count".to_string()),
            prompt_tokens: Some(12),
            completion_tokens: Some(4),
            total_tokens: Some(16),
        })
    }
}

fn make_observation() -> Observation {
    let mut self_resources = ResourceStock::new();
    let _ = self_resources.add(ResourceKind::Electricity, 30);
    let _ = self_resources.add(ResourceKind::Data, 12);

    Observation {
        time: 7,
        agent_id: "agent-1".to_string(),
        pos: GeoPos::new(0, 0, 0),
        self_resources,
        visibility_range_cm: 100,
        visible_agents: vec![ObservedAgent {
            agent_id: "agent-2".to_string(),
            location_id: "loc-2".to_string(),
            pos: GeoPos::new(1, 0, 0),
            distance_cm: 1,
        }],
        visible_locations: vec![ObservedLocation {
            location_id: "loc-2".to_string(),
            name: "outpost".to_string(),
            pos: GeoPos::new(1, 0, 0),
            profile: LocationProfile::default(),
            distance_cm: 1,
        }],
        module_lifecycle: ObservedModuleLifecycleState::default(),
        module_market: ObservedModuleMarketState::default(),
        power_market: ObservedPowerMarketState::default(),
        social_state: ObservedSocialState::default(),
    }
}

fn base_config() -> LlmAgentConfig {
    LlmAgentConfig {
        model: "provider-neutral-test".to_string(),
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

include!("tests_conversation_flow.rs");
include!("tests_tool_execution.rs");
include!("tests_edge_cases.rs");
include!("tests_followups.rs");
include!("tests_part3_module_lifecycle.rs");
