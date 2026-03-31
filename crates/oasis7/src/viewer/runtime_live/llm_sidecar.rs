use std::collections::BTreeMap;
use std::env;

use crate::geometry::GeoPos;
use crate::runtime::{
    Action as RuntimeAction, CausedBy as RuntimeCausedBy, DomainEvent as RuntimeDomainEvent,
    ModuleSourcePackage, World as RuntimeWorld, WorldEvent as RuntimeWorldEvent,
    WorldEventBody as RuntimeWorldEventBody,
};
use crate::simulator::{
    Action as SimulatorAction, ActionCatalogEntry, ActionResult, AgentDecision, AgentDecisionTrace,
    AgentPromptProfile, AgentRunner, ChunkRuntimeConfig, LlmAgentBehavior,
    OpenAiChatCompletionClient, OpenClawAdapter, ProviderBackedAgentBehavior,
    ProviderExecutionMode, ResourceOwner, WorldConfig, WorldEvent, WorldEventKind, WorldJournal,
    WorldKernel, WorldSnapshot, CHUNK_GENERATION_SCHEMA_VERSION, SNAPSHOT_VERSION,
};
use crate::viewer::live::ViewerLiveDecisionMode;
use crate::viewer::protocol::{AgentChatAck, AgentChatError};
use sha2::{Digest, Sha256};

use super::super::{location_id_for_pos, mapping::runtime_state_to_simulator_model};

#[derive(Clone, Debug)]
pub(super) struct RuntimePendingAction {
    pub(super) agent_id: String,
    pub(super) action: SimulatorAction,
}

#[derive(Debug, Clone)]
pub(super) struct RuntimeLlmDecision {
    pub(super) agent_id: String,
    pub(super) decision: AgentDecision,
    pub(super) decision_trace: Option<AgentDecisionTrace>,
}

const BUILTIN_LLM_PROVIDER_MODE: &str = "builtin_llm";
const OPENCLAW_LOCAL_HTTP_PROVIDER_MODE: &str = "openclaw_local_http";
const DEFAULT_OPENCLAW_CONNECT_TIMEOUT_MS: u64 = 3_000;
const DEFAULT_OPENCLAW_AGENT_PROFILE: &str = "oasis7_p0_low_freq_npc";
const VIEWER_AGENT_PROVIDER_MODE_ENV: &str = "OASIS7_AGENT_PROVIDER_MODE";
const VIEWER_OPENCLAW_BASE_URL_ENV: &str = "OASIS7_OPENCLAW_BASE_URL";
const VIEWER_OPENCLAW_AUTH_TOKEN_ENV: &str = "OASIS7_OPENCLAW_AUTH_TOKEN";
const VIEWER_OPENCLAW_CONNECT_TIMEOUT_MS_ENV: &str = "OASIS7_OPENCLAW_CONNECT_TIMEOUT_MS";
const VIEWER_OPENCLAW_AGENT_PROFILE_ENV: &str = "OASIS7_OPENCLAW_AGENT_PROFILE";
const VIEWER_OPENCLAW_EXECUTION_MODE_ENV: &str = "OASIS7_OPENCLAW_EXECUTION_MODE";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::viewer::runtime_live) struct OpenClawDecisionSettings {
    pub(in crate::viewer::runtime_live) base_url: String,
    pub(in crate::viewer::runtime_live) auth_token: Option<String>,
    pub(in crate::viewer::runtime_live) connect_timeout_ms: u64,
    pub(in crate::viewer::runtime_live) agent_profile: String,
    pub(in crate::viewer::runtime_live) execution_mode: ProviderExecutionMode,
}

enum RuntimeDecisionRunner {
    Builtin(AgentRunner<LlmAgentBehavior<OpenAiChatCompletionClient>>),
    OpenClaw(AgentRunner<ProviderBackedAgentBehavior<OpenClawAdapter>>),
}

fn env_requests_openclaw_provider() -> bool {
    named_env_var(VIEWER_AGENT_PROVIDER_MODE_ENV)
        .map(|value| value.trim().to_string())
        .is_some_and(|value| value == OPENCLAW_LOCAL_HTTP_PROVIDER_MODE)
}

pub(in crate::viewer::runtime_live) fn openclaw_settings_from_env(
) -> Result<Option<OpenClawDecisionSettings>, String> {
    let provider_mode = named_env_var(VIEWER_AGENT_PROVIDER_MODE_ENV).unwrap_or_default();
    let provider_mode = provider_mode.trim();
    if provider_mode.is_empty() || provider_mode == BUILTIN_LLM_PROVIDER_MODE {
        return Ok(None);
    }
    if provider_mode != OPENCLAW_LOCAL_HTTP_PROVIDER_MODE {
        return Err(format!(
            "unsupported agent provider mode `{provider_mode}`; expected builtin_llm or openclaw_local_http"
        ));
    }

    let base_url = named_env_var(VIEWER_OPENCLAW_BASE_URL_ENV).unwrap_or_default();
    let base_url = base_url.trim();
    if base_url.is_empty() {
        return Err(format!(
            "{VIEWER_OPENCLAW_BASE_URL_ENV} is required for openclaw_local_http"
        ));
    }

    let connect_timeout_ms = named_env_var(VIEWER_OPENCLAW_CONNECT_TIMEOUT_MS_ENV)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(|value| {
            value.parse::<u64>().map_err(|err| {
                format!("invalid {VIEWER_OPENCLAW_CONNECT_TIMEOUT_MS_ENV} value `{value}`: {err}")
            })
        })
        .transpose()?
        .unwrap_or(DEFAULT_OPENCLAW_CONNECT_TIMEOUT_MS);
    if connect_timeout_ms == 0 {
        return Err(format!(
            "{VIEWER_OPENCLAW_CONNECT_TIMEOUT_MS_ENV} must be greater than zero"
        ));
    }

    let agent_profile = named_env_var(VIEWER_OPENCLAW_AGENT_PROFILE_ENV)
        .unwrap_or_else(|| DEFAULT_OPENCLAW_AGENT_PROFILE.to_string());
    let agent_profile = agent_profile.trim();
    if agent_profile.is_empty() {
        return Err(format!(
            "{VIEWER_OPENCLAW_AGENT_PROFILE_ENV} cannot be empty for openclaw_local_http"
        ));
    }

    let auth_token = named_env_var(VIEWER_OPENCLAW_AUTH_TOKEN_ENV)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let execution_mode = named_env_var(VIEWER_OPENCLAW_EXECUTION_MODE_ENV)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(|value| {
            ProviderExecutionMode::parse(value.as_str()).ok_or_else(|| {
                format!(
                    "invalid {VIEWER_OPENCLAW_EXECUTION_MODE_ENV} value `{value}`: expected player_parity or headless_agent"
                )
            })
        })
        .transpose()?
        .unwrap_or(ProviderExecutionMode::HeadlessAgent);

    Ok(Some(OpenClawDecisionSettings {
        base_url: base_url.to_string(),
        auth_token,
        connect_timeout_ms,
        agent_profile: agent_profile.to_string(),
        execution_mode,
    }))
}

fn named_env_var(env_name: &str) -> Option<String> {
    env::var(env_name).ok()
}

fn openclaw_phase1_action_catalog() -> Vec<ActionCatalogEntry> {
    vec![
        ActionCatalogEntry::new("wait", "yield current turn without acting"),
        ActionCatalogEntry::new("wait_ticks", "sleep for a bounded number of ticks"),
        ActionCatalogEntry::new("move_agent", "move to a neighboring location"),
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

#[derive(Clone, Debug)]
struct RuntimeChatIntentAckRecord {
    ack: AgentChatAck,
    message_hash: String,
    public_key: Option<String>,
    intent_tick: Option<u64>,
}

impl RuntimeLlmDecision {
    fn from_error(world: &RuntimeWorld, message: String) -> Self {
        let agent_id = world
            .state()
            .agents
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| "runtime-agent-0".to_string());
        let trace = AgentDecisionTrace {
            agent_id: agent_id.clone(),
            time: world.state().time,
            decision: AgentDecision::Wait,
            llm_input: None,
            llm_output: None,
            llm_error: Some(message),
            parse_error: None,
            llm_diagnostics: None,
            llm_effect_intents: Vec::new(),
            llm_effect_receipts: Vec::new(),
            llm_step_trace: Vec::new(),
            llm_prompt_section_trace: Vec::new(),
            llm_chat_messages: Vec::new(),
        };
        Self {
            agent_id,
            decision: AgentDecision::Wait,
            decision_trace: Some(trace),
        }
    }
}

pub(in crate::viewer::runtime_live) struct RuntimeLlmSidecar {
    pub(in crate::viewer::runtime_live) decision_mode: ViewerLiveDecisionMode,
    pub(in crate::viewer::runtime_live) prompt_profiles: BTreeMap<String, AgentPromptProfile>,
    pub(in crate::viewer::runtime_live) prompt_profile_history:
        BTreeMap<String, BTreeMap<u64, AgentPromptProfile>>,
    pub(in crate::viewer::runtime_live) agent_player_bindings: BTreeMap<String, String>,
    pub(in crate::viewer::runtime_live) player_agent_bindings: BTreeMap<String, String>,
    pub(in crate::viewer::runtime_live) agent_public_key_bindings: BTreeMap<String, String>,
    pub(in crate::viewer::runtime_live) player_auth_last_nonce: BTreeMap<String, u64>,
    player_chat_intent_acks: BTreeMap<(String, String, u64), RuntimeChatIntentAckRecord>,
    llm_decision_mailbox: u64,
    runner: Option<RuntimeDecisionRunner>,
    shadow_kernel: Option<WorldKernel>,
    pending_actions: BTreeMap<u64, RuntimePendingAction>,
}

impl RuntimeLlmSidecar {
    pub(in crate::viewer::runtime_live) fn new(decision_mode: ViewerLiveDecisionMode) -> Self {
        Self {
            decision_mode,
            prompt_profiles: BTreeMap::new(),
            prompt_profile_history: BTreeMap::new(),
            agent_player_bindings: BTreeMap::new(),
            player_agent_bindings: BTreeMap::new(),
            agent_public_key_bindings: BTreeMap::new(),
            player_auth_last_nonce: BTreeMap::new(),
            player_chat_intent_acks: BTreeMap::new(),
            llm_decision_mailbox: 0,
            runner: None,
            shadow_kernel: None,
            pending_actions: BTreeMap::new(),
        }
    }

    pub(in crate::viewer::runtime_live) fn is_llm_mode(&self) -> bool {
        matches!(self.decision_mode, ViewerLiveDecisionMode::Llm)
    }

    pub(in crate::viewer::runtime_live) fn supports_prompt_control(&self) -> bool {
        !env_requests_openclaw_provider()
    }

    pub(in crate::viewer::runtime_live) fn supports_agent_chat(&self) -> bool {
        !env_requests_openclaw_provider()
    }

    pub(in crate::viewer::runtime_live) fn ensure_gameplay_ready(
        &mut self,
        world: &RuntimeWorld,
        config: &WorldConfig,
    ) -> Result<(), String> {
        if !self.is_llm_mode() {
            return Err("gameplay requires runtime live server running with --llm".to_string());
        }
        self.sync_shadow_kernel(world, config)?;
        self.ensure_runner_initialized().map_err(|message| {
            format!("gameplay requires a configured and reachable LLM provider: {message}")
        })?;
        Ok(())
    }

    pub(in crate::viewer::runtime_live) fn consume_player_auth_nonce(
        &mut self,
        player_id: &str,
        nonce: u64,
    ) -> Result<(), String> {
        let player_id = player_id.trim();
        if player_id.is_empty() {
            return Err("player_id cannot be empty".to_string());
        }
        if nonce == 0 {
            return Err("auth nonce must be greater than zero".to_string());
        }
        if let Some(last_nonce) = self.player_auth_last_nonce.get(player_id) {
            if nonce <= *last_nonce {
                return Err(format!(
                    "auth nonce replay for {}: expected nonce > {}, received {}",
                    player_id, last_nonce, nonce
                ));
            }
        }
        self.player_auth_last_nonce
            .insert(player_id.to_string(), nonce);
        Ok(())
    }

    pub(super) fn find_chat_intent_replay(
        &self,
        player_id: &str,
        agent_id: &str,
        intent_seq: u64,
        intent_tick: Option<u64>,
        message: &str,
        public_key: Option<&str>,
    ) -> Result<Option<AgentChatAck>, String> {
        let key = (
            player_id.trim().to_string(),
            agent_id.trim().to_string(),
            intent_seq,
        );
        let Some(record) = self.player_chat_intent_acks.get(&key) else {
            return Ok(None);
        };
        let normalized_public_key = normalize_optional_public_key(public_key);
        let message_hash = hash_chat_message(message);
        if record.message_hash != message_hash
            || record.intent_tick != intent_tick
            || record.public_key != normalized_public_key
        {
            return Err(format!(
                "agent_chat intent_seq conflict for {} on {} seq {}",
                key.0, key.1, intent_seq
            ));
        }
        let mut ack = record.ack.clone();
        ack.idempotent_replay = true;
        Ok(Some(ack))
    }

    pub(super) fn record_chat_intent_ack(
        &mut self,
        player_id: &str,
        agent_id: &str,
        intent_seq: u64,
        intent_tick: Option<u64>,
        message: &str,
        public_key: Option<&str>,
        ack: &AgentChatAck,
    ) {
        let key = (
            player_id.trim().to_string(),
            agent_id.trim().to_string(),
            intent_seq,
        );
        let record = RuntimeChatIntentAckRecord {
            ack: ack.clone(),
            message_hash: hash_chat_message(message),
            public_key: normalize_optional_public_key(public_key),
            intent_tick,
        };
        self.player_chat_intent_acks.insert(key, record);
    }

    pub(in crate::viewer::runtime_live) fn clear_chat_intent_acks_for_player(
        &mut self,
        player_id: &str,
    ) {
        let player_id = player_id.trim();
        self.player_chat_intent_acks
            .retain(|(record_player_id, _, _), _| record_player_id != player_id);
    }

    pub(in crate::viewer::runtime_live) fn bound_agent_for_player(
        &self,
        player_id: &str,
    ) -> Option<&str> {
        self.player_agent_bindings
            .get(player_id.trim())
            .map(String::as_str)
    }

    pub(in crate::viewer::runtime_live) fn clear_player_binding(
        &mut self,
        player_id: &str,
    ) -> Option<WorldEventKind> {
        let player_id = player_id.trim();
        let agent_id = self.player_agent_bindings.remove(player_id)?;
        self.agent_player_bindings.remove(agent_id.as_str());
        let public_key = self.agent_public_key_bindings.remove(agent_id.as_str());
        Some(WorldEventKind::AgentPlayerUnbound {
            agent_id,
            player_id: player_id.to_string(),
            public_key,
        })
    }

    pub(in crate::viewer::runtime_live) fn bind_agent_player(
        &mut self,
        agent_id: &str,
        player_id: &str,
        public_key: Option<&str>,
        allow_player_rebind: bool,
    ) -> Result<Vec<WorldEventKind>, String> {
        let agent_id = agent_id.trim();
        let player_id = player_id.trim();
        if agent_id.is_empty() {
            return Err("agent_id cannot be empty".to_string());
        }
        if player_id.is_empty() {
            return Err("player_id cannot be empty".to_string());
        }
        let requested_public_key = public_key
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let current_player = self.agent_player_bindings.get(agent_id).cloned();
        let current_public_key = self.agent_public_key_bindings.get(agent_id).cloned();
        let previous_agent_id = self.player_agent_bindings.get(player_id).cloned();
        if previous_agent_id
            .as_deref()
            .is_some_and(|bound_agent_id| bound_agent_id != agent_id)
            && !allow_player_rebind
        {
            return Err(format!(
                "player {} is already bound to agent {}, explicit rebind required",
                player_id,
                previous_agent_id.unwrap_or_default()
            ));
        }
        let target_public_key = if current_player.as_deref() == Some(player_id) {
            requested_public_key
                .clone()
                .or_else(|| current_public_key.clone())
        } else {
            requested_public_key.clone()
        };
        if current_player.as_deref() == Some(player_id) && current_public_key == target_public_key {
            return Ok(Vec::new());
        }
        let mut events = Vec::new();
        if let Some(previous_agent_id) = previous_agent_id
            .as_deref()
            .filter(|bound_agent_id| *bound_agent_id != agent_id)
        {
            self.player_agent_bindings.remove(player_id);
            self.agent_player_bindings.remove(previous_agent_id);
            let previous_public_key = self.agent_public_key_bindings.remove(previous_agent_id);
            events.push(WorldEventKind::AgentPlayerUnbound {
                agent_id: previous_agent_id.to_string(),
                player_id: player_id.to_string(),
                public_key: previous_public_key,
            });
        }
        if let Some(previous_player_id) = current_player
            .as_deref()
            .filter(|bound_player_id| *bound_player_id != player_id)
        {
            self.player_agent_bindings.remove(previous_player_id);
            events.push(WorldEventKind::AgentPlayerUnbound {
                agent_id: agent_id.to_string(),
                player_id: previous_player_id.to_string(),
                public_key: current_public_key.clone(),
            });
        }

        self.agent_player_bindings
            .insert(agent_id.to_string(), player_id.to_string());
        self.player_agent_bindings
            .insert(player_id.to_string(), agent_id.to_string());
        match target_public_key.clone() {
            Some(value) => {
                self.agent_public_key_bindings
                    .insert(agent_id.to_string(), value);
            }
            None => {
                self.agent_public_key_bindings.remove(agent_id);
            }
        }
        events.push(WorldEventKind::AgentPlayerBound {
            agent_id: agent_id.to_string(),
            player_id: player_id.to_string(),
            public_key: target_public_key,
        });
        Ok(events)
    }

    pub(super) fn upsert_prompt_profile(&mut self, profile: AgentPromptProfile) {
        self.prompt_profile_history
            .entry(profile.agent_id.clone())
            .or_default()
            .insert(profile.version, profile.clone());
        self.prompt_profiles
            .insert(profile.agent_id.clone(), profile);
    }

    pub(super) fn request_decision(&mut self) {
        if self.is_llm_mode() {
            self.llm_decision_mailbox = self.llm_decision_mailbox.saturating_add(1);
        }
    }

    pub(super) fn apply_prompt_profile_to_driver(&mut self, profile: &AgentPromptProfile) {
        let Some(runner) = self.runner.as_mut() else {
            return;
        };
        let RuntimeDecisionRunner::Builtin(runner) = runner else {
            return;
        };
        let Some(agent) = runner.get_mut(profile.agent_id.as_str()) else {
            return;
        };
        agent.behavior.apply_prompt_overrides(
            profile.system_prompt_override.clone(),
            profile.short_term_goal_override.clone(),
            profile.long_term_goal_override.clone(),
        );
    }

    pub(super) fn push_chat_message(
        &mut self,
        world: &RuntimeWorld,
        config: &WorldConfig,
        agent_id: &str,
        message: &str,
    ) -> Result<(), AgentChatError> {
        if !self.is_llm_mode() {
            return Err(AgentChatError {
                code: "llm_mode_required".to_string(),
                message: "agent chat requires runtime live server running with --llm".to_string(),
                agent_id: Some(agent_id.to_string()),
            });
        }
        if let Err(message) = self.sync_shadow_kernel(world, config) {
            return Err(AgentChatError {
                code: "llm_init_failed".to_string(),
                message,
                agent_id: Some(agent_id.to_string()),
            });
        }
        if let Err(message) = self.ensure_runner_initialized() {
            return Err(AgentChatError {
                code: "llm_init_failed".to_string(),
                message,
                agent_id: Some(agent_id.to_string()),
            });
        }
        let runner = match self.runner.as_mut() {
            Some(runner) => runner,
            None => {
                return Err(AgentChatError {
                    code: "llm_init_failed".to_string(),
                    message: "llm runner not initialized".to_string(),
                    agent_id: Some(agent_id.to_string()),
                });
            }
        };
        let RuntimeDecisionRunner::Builtin(runner) = runner else {
            return Err(AgentChatError {
                code: "agent_provider_chat_unsupported".to_string(),
                message:
                    "agent chat is not yet supported when runtime live uses OpenClaw(Local HTTP)"
                        .to_string(),
                agent_id: Some(agent_id.to_string()),
            });
        };
        let Some(agent) = runner.get_mut(agent_id) else {
            return Err(AgentChatError {
                code: "agent_not_registered".to_string(),
                message: format!("agent {} is not registered in llm runner", agent_id),
                agent_id: Some(agent_id.to_string()),
            });
        };
        if !agent
            .behavior
            .push_player_message(world.state().time, message)
        {
            return Err(AgentChatError {
                code: "empty_message".to_string(),
                message: "chat message cannot be empty".to_string(),
                agent_id: Some(agent_id.to_string()),
            });
        }
        Ok(())
    }

    pub(super) fn next_llm_decision(
        &mut self,
        world: &RuntimeWorld,
        config: &WorldConfig,
    ) -> Option<RuntimeLlmDecision> {
        if !self.is_llm_mode() || self.llm_decision_mailbox == 0 {
            return None;
        }
        self.llm_decision_mailbox = self.llm_decision_mailbox.saturating_sub(1);

        if let Err(message) = self.sync_shadow_kernel(world, config) {
            return Some(RuntimeLlmDecision::from_error(world, message));
        }
        if let Err(message) = self.ensure_runner_initialized() {
            return Some(RuntimeLlmDecision::from_error(world, message));
        }
        let kernel = match self.shadow_kernel.as_mut() {
            Some(kernel) => kernel,
            None => {
                return Some(RuntimeLlmDecision::from_error(
                    world,
                    "shadow kernel not initialized".to_string(),
                ));
            }
        };
        let runner = match self.runner.as_mut() {
            Some(runner) => runner,
            None => {
                return Some(RuntimeLlmDecision::from_error(
                    world,
                    "llm runner not initialized".to_string(),
                ));
            }
        };
        let result = match runner {
            RuntimeDecisionRunner::Builtin(runner) => {
                let result = runner.tick_decide_only(kernel);
                sync_llm_runner_long_term_memory(kernel, runner);
                result
            }
            RuntimeDecisionRunner::OpenClaw(runner) => runner.tick_decide_only(kernel),
        };
        result.map(|tick| RuntimeLlmDecision {
            agent_id: tick.agent_id,
            decision: tick.decision,
            decision_trace: tick.decision_trace,
        })
    }

    pub(super) fn track_action(
        &mut self,
        action_id: u64,
        agent_id: String,
        action: SimulatorAction,
    ) {
        self.pending_actions
            .insert(action_id, RuntimePendingAction { agent_id, action });
    }

    pub(super) fn notify_action_result(
        &mut self,
        action_id: u64,
        event: WorldEvent,
        rejected: bool,
    ) {
        let Some(pending) = self.pending_actions.remove(&action_id) else {
            return;
        };
        let success = !rejected;
        let action_result = ActionResult {
            action: pending.action,
            action_id,
            success,
            event,
        };
        if let Some(runner) = self.runner.as_mut() {
            match runner {
                RuntimeDecisionRunner::Builtin(runner) => {
                    let _ = runner.notify_action_result(pending.agent_id.as_str(), &action_result);
                }
                RuntimeDecisionRunner::OpenClaw(runner) => {
                    let _ = runner.notify_action_result(pending.agent_id.as_str(), &action_result);
                }
            }
        }
    }

    pub(in crate::viewer::runtime_live) fn notify_action_result_if_needed(
        &mut self,
        runtime_event: &RuntimeWorldEvent,
        mapped_event: WorldEvent,
    ) {
        let Some(caused_by) = runtime_event.caused_by.as_ref() else {
            return;
        };
        let RuntimeCausedBy::Action(action_id) = caused_by else {
            return;
        };
        let rejected = matches!(
            runtime_event.body,
            RuntimeWorldEventBody::Domain(RuntimeDomainEvent::ActionRejected { .. })
        );
        self.notify_action_result(*action_id, mapped_event, rejected);
    }

    fn sync_shadow_kernel(
        &mut self,
        world: &RuntimeWorld,
        config: &WorldConfig,
    ) -> Result<(), String> {
        let runtime_snapshot = world.snapshot();
        let next_event_id = runtime_snapshot.last_event_id.saturating_add(1).max(1);
        let next_action_id = runtime_snapshot.next_action_id.max(1);
        let model = runtime_state_to_simulator_model(world.state(), self);
        let snapshot = WorldSnapshot {
            version: SNAPSHOT_VERSION,
            chunk_generation_schema_version: CHUNK_GENERATION_SCHEMA_VERSION,
            time: world.state().time,
            config: config.clone(),
            model,
            runtime_snapshot: Some(runtime_snapshot),
            player_gameplay: None,
            chunk_runtime: ChunkRuntimeConfig::default(),
            next_event_id,
            next_action_id,
            pending_actions: Vec::new(),
            journal_len: 0,
        };
        let kernel = WorldKernel::from_snapshot(snapshot, WorldJournal::new())
            .map_err(|err| format!("runtime live shadow kernel rebuild failed: {err:?}"))?;
        self.shadow_kernel = Some(kernel);
        Ok(())
    }

    fn ensure_runner_initialized(&mut self) -> Result<(), String> {
        let kernel = self
            .shadow_kernel
            .as_ref()
            .ok_or_else(|| "shadow kernel not initialized".to_string())?;
        let openclaw_settings = openclaw_settings_from_env()?;
        if self.runner.is_none() {
            self.runner = Some(match openclaw_settings.as_ref() {
                Some(_) => RuntimeDecisionRunner::OpenClaw(AgentRunner::new()),
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
                RuntimeDecisionRunner::Builtin(runner) => {
                    if runner.get(agent_id.as_str()).is_some() {
                        continue;
                    }
                    let mut behavior = LlmAgentBehavior::from_env(agent_id.clone())
                        .map_err(|err| format!("llm init failed for {}: {err}", agent_id))?;
                    if let Some(profile) = self.prompt_profiles.get(agent_id.as_str()) {
                        behavior.apply_prompt_overrides(
                            profile.system_prompt_override.clone(),
                            profile.short_term_goal_override.clone(),
                            profile.long_term_goal_override.clone(),
                        );
                    }
                    restore_behavior_long_term_memory_from_model(
                        &mut behavior,
                        kernel,
                        agent_id.as_str(),
                    );
                    runner.register(behavior);
                }
                RuntimeDecisionRunner::OpenClaw(runner) => {
                    if runner.get(agent_id.as_str()).is_some() {
                        continue;
                    }
                    let settings = openclaw_settings.as_ref().ok_or_else(|| {
                        "openclaw runner selected without resolved settings".to_string()
                    })?;
                    let adapter = OpenClawAdapter::new(
                        settings.base_url.as_str(),
                        settings.auth_token.as_deref(),
                        settings.connect_timeout_ms,
                    )
                    .map_err(|err| format!("openclaw init failed for {}: {}", agent_id, err))?;
                    let behavior = ProviderBackedAgentBehavior::new(
                        agent_id.clone(),
                        adapter,
                        openclaw_phase1_action_catalog(),
                    )
                    .with_provider_config_ref(format!(
                        "openclaw://local-http/runtime-live/pid-{}/{}",
                        std::process::id(),
                        agent_id
                    ))
                    .with_agent_profile(settings.agent_profile.clone())
                    .with_execution_mode(settings.execution_mode)
                    .with_environment_class("runtime_live");
                    runner.register(behavior);
                }
            }
        }
        Ok(())
    }
}

fn normalize_optional_public_key(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn hash_chat_message(message: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(message.as_bytes());
    hex::encode(hasher.finalize())
}

pub(in crate::viewer::runtime_live) fn simulator_action_label(action: &SimulatorAction) -> String {
    format!("{action:?}")
}

pub(in crate::viewer::runtime_live) fn simulator_action_to_runtime(
    action: &SimulatorAction,
    world: &RuntimeWorld,
) -> Option<RuntimeAction> {
    match action {
        SimulatorAction::RegisterAgent {
            agent_id,
            location_id,
        } => Some(RuntimeAction::RegisterAgent {
            agent_id: agent_id.clone(),
            pos: resolve_runtime_location(world, location_id)?,
        }),
        SimulatorAction::MoveAgent { agent_id, to } => Some(RuntimeAction::MoveAgent {
            agent_id: agent_id.clone(),
            to: resolve_runtime_location(world, to)?,
        }),
        SimulatorAction::TransferResource {
            from,
            to,
            kind,
            amount,
        } => match (from, to) {
            (
                ResourceOwner::Agent {
                    agent_id: from_agent_id,
                },
                ResourceOwner::Agent {
                    agent_id: to_agent_id,
                },
            ) => Some(RuntimeAction::TransferResource {
                from_agent_id: from_agent_id.clone(),
                to_agent_id: to_agent_id.clone(),
                kind: *kind,
                amount: *amount,
            }),
            _ => None,
        },
        SimulatorAction::FormAlliance {
            proposer_agent_id,
            alliance_id,
            members,
            charter,
        } => Some(RuntimeAction::FormAlliance {
            proposer_agent_id: proposer_agent_id.clone(),
            alliance_id: alliance_id.clone(),
            members: members.clone(),
            charter: charter.clone(),
        }),
        SimulatorAction::JoinAlliance {
            operator_agent_id,
            alliance_id,
            member_agent_id,
        } => Some(RuntimeAction::JoinAlliance {
            operator_agent_id: operator_agent_id.clone(),
            alliance_id: alliance_id.clone(),
            member_agent_id: member_agent_id.clone(),
        }),
        SimulatorAction::LeaveAlliance {
            operator_agent_id,
            alliance_id,
            member_agent_id,
        } => Some(RuntimeAction::LeaveAlliance {
            operator_agent_id: operator_agent_id.clone(),
            alliance_id: alliance_id.clone(),
            member_agent_id: member_agent_id.clone(),
        }),
        SimulatorAction::DissolveAlliance {
            operator_agent_id,
            alliance_id,
            reason,
        } => Some(RuntimeAction::DissolveAlliance {
            operator_agent_id: operator_agent_id.clone(),
            alliance_id: alliance_id.clone(),
            reason: reason.clone(),
        }),
        SimulatorAction::DeclareWar {
            initiator_agent_id,
            war_id,
            aggressor_alliance_id,
            defender_alliance_id,
            objective,
            intensity,
        } => Some(RuntimeAction::DeclareWar {
            initiator_agent_id: initiator_agent_id.clone(),
            war_id: war_id.clone(),
            aggressor_alliance_id: aggressor_alliance_id.clone(),
            defender_alliance_id: defender_alliance_id.clone(),
            objective: objective.clone(),
            intensity: *intensity,
        }),
        SimulatorAction::OpenGovernanceProposal {
            proposer_agent_id,
            proposal_key,
            title,
            description,
            options,
            voting_window_ticks,
            quorum_weight,
            pass_threshold_bps,
        } => Some(RuntimeAction::OpenGovernanceProposal {
            proposer_agent_id: proposer_agent_id.clone(),
            proposal_key: proposal_key.clone(),
            title: title.clone(),
            description: description.clone(),
            options: options.clone(),
            voting_window_ticks: *voting_window_ticks,
            quorum_weight: *quorum_weight,
            pass_threshold_bps: *pass_threshold_bps,
        }),
        SimulatorAction::CastGovernanceVote {
            voter_agent_id,
            proposal_key,
            option,
            weight,
        } => Some(RuntimeAction::CastGovernanceVote {
            voter_agent_id: voter_agent_id.clone(),
            proposal_key: proposal_key.clone(),
            option: option.clone(),
            weight: *weight,
        }),
        SimulatorAction::ResolveCrisis {
            resolver_agent_id,
            crisis_id,
            strategy,
            success,
        } => Some(RuntimeAction::ResolveCrisis {
            resolver_agent_id: resolver_agent_id.clone(),
            crisis_id: crisis_id.clone(),
            strategy: strategy.clone(),
            success: *success,
        }),
        SimulatorAction::GrantMetaProgress {
            operator_agent_id,
            target_agent_id,
            track,
            points,
            achievement_id,
        } => Some(RuntimeAction::GrantMetaProgress {
            operator_agent_id: operator_agent_id.clone(),
            target_agent_id: target_agent_id.clone(),
            track: track.clone(),
            points: *points,
            achievement_id: achievement_id.clone(),
        }),
        SimulatorAction::OpenEconomicContract {
            creator_agent_id,
            contract_id,
            counterparty_agent_id,
            settlement_kind,
            settlement_amount,
            reputation_stake,
            expires_at,
            description,
        } => Some(RuntimeAction::OpenEconomicContract {
            creator_agent_id: creator_agent_id.clone(),
            contract_id: contract_id.clone(),
            counterparty_agent_id: counterparty_agent_id.clone(),
            settlement_kind: *settlement_kind,
            settlement_amount: *settlement_amount,
            reputation_stake: *reputation_stake,
            expires_at: *expires_at,
            description: description.clone(),
        }),
        SimulatorAction::AcceptEconomicContract {
            accepter_agent_id,
            contract_id,
        } => Some(RuntimeAction::AcceptEconomicContract {
            accepter_agent_id: accepter_agent_id.clone(),
            contract_id: contract_id.clone(),
        }),
        SimulatorAction::SettleEconomicContract {
            operator_agent_id,
            contract_id,
            success,
            notes,
        } => Some(RuntimeAction::SettleEconomicContract {
            operator_agent_id: operator_agent_id.clone(),
            contract_id: contract_id.clone(),
            success: *success,
            notes: notes.clone(),
        }),
        SimulatorAction::CompileModuleArtifactFromSource {
            publisher_agent_id,
            module_id,
            manifest_path,
            source_files,
        } => Some(RuntimeAction::CompileModuleArtifactFromSource {
            publisher_agent_id: publisher_agent_id.clone(),
            module_id: module_id.clone(),
            source_package: ModuleSourcePackage {
                manifest_path: manifest_path.clone(),
                files: source_files.clone(),
            },
        }),
        SimulatorAction::DeployModuleArtifact {
            publisher_agent_id,
            wasm_hash,
            wasm_bytes,
            ..
        } => Some(RuntimeAction::DeployModuleArtifact {
            publisher_agent_id: publisher_agent_id.clone(),
            wasm_hash: wasm_hash.clone(),
            wasm_bytes: wasm_bytes.clone(),
        }),
        SimulatorAction::ListModuleArtifactForSale {
            seller_agent_id,
            wasm_hash,
            price_kind,
            price_amount,
        } => Some(RuntimeAction::ListModuleArtifactForSale {
            seller_agent_id: seller_agent_id.clone(),
            wasm_hash: wasm_hash.clone(),
            price_kind: *price_kind,
            price_amount: *price_amount,
        }),
        SimulatorAction::BuyModuleArtifact {
            buyer_agent_id,
            wasm_hash,
        } => Some(RuntimeAction::BuyModuleArtifact {
            buyer_agent_id: buyer_agent_id.clone(),
            wasm_hash: wasm_hash.clone(),
        }),
        SimulatorAction::DelistModuleArtifact {
            seller_agent_id,
            wasm_hash,
        } => Some(RuntimeAction::DelistModuleArtifact {
            seller_agent_id: seller_agent_id.clone(),
            wasm_hash: wasm_hash.clone(),
        }),
        SimulatorAction::DestroyModuleArtifact {
            owner_agent_id,
            wasm_hash,
            reason,
        } => Some(RuntimeAction::DestroyModuleArtifact {
            owner_agent_id: owner_agent_id.clone(),
            wasm_hash: wasm_hash.clone(),
            reason: reason.clone(),
        }),
        SimulatorAction::PlaceModuleArtifactBid {
            bidder_agent_id,
            wasm_hash,
            price_kind,
            price_amount,
        } => Some(RuntimeAction::PlaceModuleArtifactBid {
            bidder_agent_id: bidder_agent_id.clone(),
            wasm_hash: wasm_hash.clone(),
            price_kind: *price_kind,
            price_amount: *price_amount,
        }),
        SimulatorAction::CancelModuleArtifactBid {
            bidder_agent_id,
            wasm_hash,
            bid_order_id,
        } => Some(RuntimeAction::CancelModuleArtifactBid {
            bidder_agent_id: bidder_agent_id.clone(),
            wasm_hash: wasm_hash.clone(),
            bid_order_id: *bid_order_id,
        }),
        _ => None,
    }
}

fn resolve_runtime_location(world: &RuntimeWorld, location_id: &str) -> Option<GeoPos> {
    if let Some(pos) = parse_runtime_location_id(location_id) {
        return Some(pos);
    }
    world
        .state()
        .agents
        .values()
        .map(|cell| cell.state.pos)
        .find(|pos| location_id_for_pos(*pos) == location_id)
}

fn parse_runtime_location_id(location_id: &str) -> Option<GeoPos> {
    let raw = location_id.strip_prefix("runtime:")?;
    let mut parts = raw.split(':');
    let x = parts.next()?.parse::<i64>().ok()?;
    let y = parts.next()?.parse::<i64>().ok()?;
    let z = parts.next()?.parse::<i64>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(GeoPos::new(x as f64, y as f64, z as f64))
}

fn restore_behavior_long_term_memory_from_model(
    behavior: &mut LlmAgentBehavior<OpenAiChatCompletionClient>,
    kernel: &WorldKernel,
    agent_id: &str,
) {
    if let Some(entries) = kernel.long_term_memory_for_agent(agent_id) {
        behavior.restore_long_term_memory_entries(entries);
    } else {
        behavior.restore_long_term_memory_entries(&[]);
    }
}

fn sync_llm_runner_long_term_memory(
    kernel: &mut WorldKernel,
    runner: &AgentRunner<LlmAgentBehavior<OpenAiChatCompletionClient>>,
) {
    for agent_id in runner.agent_ids() {
        let Some(agent) = runner.get(agent_id.as_str()) else {
            continue;
        };
        let entries = agent.behavior.export_long_term_memory_entries();
        if let Err(message) = kernel.set_agent_long_term_memory(agent_id.as_str(), entries) {
            eprintln!(
                "viewer runtime live: skip long-term memory sync for {}: {}",
                agent_id, message
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bind_agent_player_emits_unbind_before_rebind_for_same_agent() {
        let mut sidecar = RuntimeLlmSidecar::new(ViewerLiveDecisionMode::Llm);
        sidecar
            .agent_player_bindings
            .insert("agent-1".to_string(), "player-a".to_string());
        sidecar
            .player_agent_bindings
            .insert("player-a".to_string(), "agent-1".to_string());
        sidecar
            .agent_public_key_bindings
            .insert("agent-1".to_string(), "pubkey-a".to_string());

        let events = sidecar
            .bind_agent_player("agent-1", "player-b", Some("pubkey-b"), false)
            .expect("rebind should succeed");
        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            WorldEventKind::AgentPlayerUnbound {
                agent_id,
                player_id,
                public_key
            } if agent_id == "agent-1"
                && player_id == "player-a"
                && public_key.as_deref() == Some("pubkey-a")
        ));
        assert!(matches!(
            &events[1],
            WorldEventKind::AgentPlayerBound {
                agent_id,
                player_id,
                public_key
            } if agent_id == "agent-1"
                && player_id == "player-b"
                && public_key.as_deref() == Some("pubkey-b")
        ));
        assert_eq!(
            sidecar
                .agent_player_bindings
                .get("agent-1")
                .map(String::as_str),
            Some("player-b")
        );
        assert_eq!(
            sidecar
                .player_agent_bindings
                .get("player-b")
                .map(String::as_str),
            Some("agent-1")
        );
        assert!(!sidecar.player_agent_bindings.contains_key("player-a"));
        assert_eq!(
            sidecar
                .agent_public_key_bindings
                .get("agent-1")
                .map(String::as_str),
            Some("pubkey-b")
        );
    }
}
