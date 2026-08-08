//! Agent interface: AgentBehavior trait, AgentDecision, ActionResult.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

use super::kernel::{
    MicroDepotActionKind, MicroDepotPressureClass, MicroDepotQuotePreview, Observation,
    RejectReason, WorldEvent, WorldEventKind,
};
use super::types::{Action, ActionId, WorldTime};

// ============================================================================
// Agent Interface (observe → decide → act)
// ============================================================================

/// Core trait for Agent behavior in the world.
///
/// An Agent follows the observe → decide → act loop:
/// 1. `observe`: Receive the current observation of the world
/// 2. `decide`: Based on observation, decide what action to take
/// 3. `act`: The kernel executes the decided action and produces events
///
/// This trait is designed to be implemented by various agent types:
/// - Simple rule-based agents
/// - LLM-powered agents
/// - Scripted agents for testing
pub trait AgentBehavior {
    /// Returns the agent's unique identifier.
    fn agent_id(&self) -> &str;

    /// Called when the agent receives a new observation.
    /// Returns an optional action to take, or None if the agent chooses to wait.
    fn decide(&mut self, observation: &Observation) -> AgentDecision;

    /// Called after an action is executed to notify the agent of the result.
    /// This allows the agent to update internal state based on action outcomes.
    fn on_action_result(&mut self, _result: &ActionResult) {
        // Default: no-op
    }

    /// Called after a read-only world query is evaluated.
    fn on_query_result(&mut self, _result: &AgentQueryResult) {
        // Default: no-op
    }

    /// Called when an event affecting this agent occurs.
    /// This allows the agent to react to external events.
    fn on_event(&mut self, _event: &WorldEvent) {
        // Default: no-op
    }

    /// Takes and clears the latest decision trace if available.
    ///
    /// Useful for live debugging/visualization pipelines that need decision internals
    /// (e.g. LLM prompt/completion I/O).
    fn take_decision_trace(&mut self) -> Option<AgentDecisionTrace> {
        None
    }
}

/// The result of an agent's decision process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentDecision {
    /// The agent decides to perform an action.
    Act(Action),
    /// The agent requests read-only information from the current world snapshot.
    Query(AgentQuery),
    /// The agent decides to wait/skip this turn.
    Wait,
    /// The agent decides to wait for a specific number of ticks.
    WaitTicks(u64),
}

/// A read-only request an Agent can make against the current world snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentQuery {
    EvaluateMicroDepotQuote(MicroDepotQuoteRequest),
}

/// Arguments needed to preview a micro-depot service without creating an intent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MicroDepotQuoteRequest {
    pub agent_id: String,
    pub facility_id: String,
    pub target_id: String,
    pub action_kind: MicroDepotActionKind,
    pub base_cost_class: MicroDepotPressureClass,
    pub base_risk_class: MicroDepotPressureClass,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocker_type: Option<String>,
}

/// The structured outcome of a read-only Agent query.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentQueryResult {
    EvaluateMicroDepotQuote {
        request: MicroDepotQuoteRequest,
        /// Facility stock copied from the same snapshot used for the preview.
        available_units_by_kind: Option<BTreeMap<String, i64>>,
        result: Result<MicroDepotQuotePreview, String>,
    },
}

impl AgentDecision {
    /// Returns true if the agent decided to act.
    pub fn is_act(&self) -> bool {
        matches!(self, AgentDecision::Act(_))
    }

    /// Returns the action if the agent decided to act.
    pub fn action(&self) -> Option<&Action> {
        match self {
            AgentDecision::Act(action) => Some(action),
            _ => None,
        }
    }
}

/// Optional decision trace payload, intended for observability/diagnostics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentDecisionTrace {
    pub agent_id: String,
    pub time: WorldTime,
    pub decision: AgentDecision,
    pub llm_input: Option<String>,
    pub llm_output: Option<String>,
    pub llm_error: Option<String>,
    pub parse_error: Option<String>,
    #[serde(default)]
    pub llm_diagnostics: Option<LlmDecisionDiagnostics>,
    #[serde(default)]
    pub llm_effect_intents: Vec<LlmEffectIntentTrace>,
    #[serde(default)]
    pub llm_effect_receipts: Vec<LlmEffectReceiptTrace>,
    #[serde(default)]
    pub llm_step_trace: Vec<LlmStepTrace>,
    #[serde(default)]
    pub llm_prompt_section_trace: Vec<LlmPromptSectionTrace>,
    #[serde(default)]
    pub llm_chat_messages: Vec<LlmChatMessageTrace>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmEffectIntentTrace {
    pub intent_id: String,
    pub kind: String,
    pub params: JsonValue,
    pub cap_ref: String,
    pub origin: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LlmEffectReceiptTrace {
    pub intent_id: String,
    pub status: String,
    pub payload: JsonValue,
    pub cost_cents: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmStepTrace {
    pub step_index: usize,
    pub step_type: String,
    pub input_summary: String,
    pub output_summary: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmPromptSectionTrace {
    pub step_index: usize,
    pub kind: String,
    pub priority: String,
    pub included: bool,
    pub estimated_tokens: usize,
    pub emitted_tokens: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmChatRole {
    Player,
    Agent,
    Tool,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmChatMessageTrace {
    pub time: WorldTime,
    pub agent_id: String,
    pub role: LlmChatRole,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LlmDecisionDiagnostics {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
    #[serde(default)]
    pub retry_count: u32,
}

/// Result of an action execution, providing feedback to the agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionResult {
    /// The action that was executed.
    pub action: Action,
    /// The action ID assigned by the kernel.
    pub action_id: ActionId,
    /// Whether the action succeeded.
    pub success: bool,
    /// The resulting event (success or rejection).
    pub event: WorldEvent,
}

impl ActionResult {
    /// Returns true if the action was rejected.
    pub fn is_rejected(&self) -> bool {
        matches!(self.event.kind, WorldEventKind::ActionRejected { .. })
    }

    /// Returns the rejection reason if the action was rejected.
    pub fn reject_reason(&self) -> Option<&RejectReason> {
        match &self.event.kind {
            WorldEventKind::ActionRejected { reason } => Some(reason),
            _ => None,
        }
    }
}
