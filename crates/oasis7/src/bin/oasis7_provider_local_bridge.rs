use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::env;
use std::fs;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;

use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use oasis7::observability::{emit_stderr_or_event, init_tracing, resolve_trace_session_id};
use oasis7::simulator::{
    DecisionRequest, DecisionResponse, FeedbackEnvelope, ProviderAgentChatRequest,
    ProviderAgentChatResponse, ProviderDecision, ProviderDiagnostics, ProviderErrorEnvelope,
    ProviderHealth, ProviderInfo, ProviderTokenUsage, ProviderTraceEnvelope,
    ProviderTranscriptEntry, provider_agent_chat_log_key,
};
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::Value;
use serde_json::json;
use tracing::{Level, error, info, warn};

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:5841";
const DEFAULT_PROVIDER_AGENT_ID: &str = "main";
const DEFAULT_PROVIDER_THINKING: &str = "off";
const DEFAULT_PROVIDER_ID: &str = "provider_local_bridge";
const MOCK_PROVIDER_ID: &str = "provider_local_mock";
const DEFAULT_PROTOCOL_VERSION: &str = "world-simulator-provider-loopback-http-v1";
const MAX_RECENT_FEEDBACK: usize = 8;
const DEFAULT_PROVIDER_AGENT_PROFILE: &str = "oasis7_p0_low_freq_npc";
const TRACE_SESSION_PROCESS_LABEL: &str = "oasis7_provider_local_bridge";
const MIN_BRIDGE_AUTH_TOKEN_LEN: usize = 24;

fn default_provider_cli_bin() -> String {
    env::var("OASIS7_PROVIDER_CLI_BIN")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| ["open", "claw"].concat())
}

#[cfg(test)]
#[path = "oasis7_provider_local_bridge/api_test_support.rs"]
mod api;
#[path = "oasis7_provider_local_bridge/auth_support.rs"]
mod auth_support;
#[allow(dead_code)]
#[path = "oasis7_newapi_bridge_service/credit_adapter.rs"]
mod credit_adapter;
#[path = "oasis7_provider_local_bridge/http_bridge_support.rs"]
mod http_bridge_support;
#[path = "oasis7_provider_local_bridge/letai_direct.rs"]
mod letai_direct;
#[path = "oasis7_provider_local_bridge/support.rs"]
mod support;
#[cfg(test)]
#[path = "oasis7_provider_local_bridge/tests.rs"]
mod tests;

use self::auth_support::{
    NewapiBridgeStateCache, load_cached_newapi_bridge_state, parse_newapi_bridge_bearer_selector,
};
#[path = "oasis7_provider_local_bridge/agent_decision.rs"]
mod agent_decision;
use self::agent_decision::{
    apply_profile_guardrails, build_decision_prompt, build_gateway_agent_params, build_session_key,
    deterministic_mock_decision, parse_model_decision, provider_decision_label, summarize_text,
    timeout_seconds_from_budget,
};
use self::http_bridge_support::handle_connection;
#[cfg(test)]
use self::letai_direct::{
    LetaiChatConfig, decode_letai_completion_payload, decode_letai_sse_reader,
    load_letai_chat_config, maybe_auto_topup_letai_user, quota_from_usd,
    should_auto_topup_letai_error,
};
use self::letai_direct::{error_diagnostics_json, invoke_rust_direct_letai};
use self::support::{
    agent_output_from_json, local_session_id_from_session_key, should_fallback_to_local_agent,
};

#[derive(Debug, Clone)]
struct CliOptions {
    bind_addr: String,
    provider_cli_bin: String,
    provider_agent_id: String,
    provider_thinking: String,
    gateway_health_url: String,
    auth_token: Option<String>,
    auth_route_map: BTreeMap<String, String>,
    auth_route_from_bearer: bool,
    mode: ProviderMode,
    provider_backend: ProviderBackend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderMode {
    Real,
    Mock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderBackend {
    RustDirectLetai,
    LegacyCli,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            bind_addr: DEFAULT_BIND_ADDR.to_string(),
            provider_cli_bin: default_provider_cli_bin(),
            provider_agent_id: DEFAULT_PROVIDER_AGENT_ID.to_string(),
            provider_thinking: DEFAULT_PROVIDER_THINKING.to_string(),
            gateway_health_url: default_gateway_health_url(),
            auth_token: None,
            auth_route_map: BTreeMap::new(),
            auth_route_from_bearer: false,
            mode: ProviderMode::Real,
            provider_backend: ProviderBackend::RustDirectLetai,
        }
    }
}

#[derive(Debug, Clone)]
struct ProviderState {
    started_at: Instant,
    options: CliOptions,
    http: Client,
    active_requests: Arc<AtomicU64>,
    last_error: Arc<Mutex<Option<String>>>,
    recent_feedback: Arc<Mutex<VecDeque<String>>>,
    newapi_bridge_state_cache: Arc<Mutex<NewapiBridgeStateCache>>,
}

impl ProviderState {
    fn new(options: CliOptions) -> Result<Self, String> {
        let http = Client::builder()
            .timeout(Duration::from_millis(1500))
            .build()
            .map_err(|err| format!("build provider http client failed: {err}"))?;
        Ok(Self {
            started_at: Instant::now(),
            options,
            http,
            active_requests: Arc::new(AtomicU64::new(0)),
            last_error: Arc::new(Mutex::new(None)),
            recent_feedback: Arc::new(Mutex::new(VecDeque::new())),
            newapi_bridge_state_cache: Arc::new(Mutex::new(NewapiBridgeStateCache::default())),
        })
    }

    fn provider_info(&self) -> ProviderInfo {
        ProviderInfo {
            provider_id: self.provider_id().to_string(),
            name: Some(self.provider_name().to_string()),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
            protocol_version: Some(DEFAULT_PROTOCOL_VERSION.to_string()),
            capabilities: self.provider_capabilities(),
            supported_action_sets: vec![
                "wait".to_string(),
                "wait_ticks".to_string(),
                "move_agent".to_string(),
                "speak_to_nearby".to_string(),
                "inspect_target".to_string(),
                "simple_interact".to_string(),
            ],
            chain_resource_manifest_schema_version: Some(
                oasis7::runtime::CHAIN_RESOURCE_MANIFEST_SCHEMA_V1.to_string(),
            ),
            chain_resource_delta_schema_version: Some(
                oasis7::runtime::CHAIN_RESOURCE_DELTA_SCHEMA_V1.to_string(),
            ),
        }
    }

    fn provider_id(&self) -> &'static str {
        match self.options.mode {
            ProviderMode::Real => DEFAULT_PROVIDER_ID,
            ProviderMode::Mock => MOCK_PROVIDER_ID,
        }
    }

    fn provider_name(&self) -> &'static str {
        match self.options.mode {
            ProviderMode::Real => "Local Provider Bridge",
            ProviderMode::Mock => "Local Mock Provider Bridge",
        }
    }

    fn provider_capabilities(&self) -> Vec<String> {
        let mut capabilities = vec![
            "decision".to_string(),
            "feedback".to_string(),
            "agent_chat".to_string(),
            "loopback_only".to_string(),
        ];
        match self.options.mode {
            ProviderMode::Real => {
                capabilities.push(format!("agent:{}", self.options.provider_agent_id));
            }
            ProviderMode::Mock => {
                capabilities.push("mock".to_string());
                capabilities.push("deterministic".to_string());
                capabilities.push("no_billing".to_string());
                capabilities.push("no_quota".to_string());
            }
        }
        capabilities
    }

    fn provider_health(&self) -> ProviderHealth {
        let active_requests = self.active_requests.load(Ordering::Relaxed);
        if self.options.mode == ProviderMode::Mock {
            self.set_last_error(None);
            return ProviderHealth {
                ok: true,
                status: Some("ready".to_string()),
                uptime_ms: Some(self.started_at.elapsed().as_millis() as u64),
                last_error: None,
                queue_depth: Some(active_requests),
            };
        }
        match self
            .http
            .get(self.options.gateway_health_url.as_str())
            .send()
        {
            Ok(response) => {
                let ok = response.status().is_success();
                if ok {
                    self.set_last_error(None);
                } else {
                    self.set_last_error(Some(format!(
                        "gateway health returned HTTP {}",
                        response.status().as_u16()
                    )));
                }
                ProviderHealth {
                    ok,
                    status: Some(if ok { "ok" } else { "degraded" }.to_string()),
                    uptime_ms: Some(self.started_at.elapsed().as_millis() as u64),
                    last_error: self.last_error.lock().expect("last_error lock").clone(),
                    queue_depth: Some(active_requests),
                }
            }
            Err(err) => {
                let detail = format!("provider_gateway_unreachable: {err}");
                self.set_last_error(Some(detail.clone()));
                ProviderHealth {
                    ok: false,
                    status: Some("degraded".to_string()),
                    uptime_ms: Some(self.started_at.elapsed().as_millis() as u64),
                    last_error: Some(detail),
                    queue_depth: Some(active_requests),
                }
            }
        }
    }

    fn record_feedback(&self, feedback: FeedbackEnvelope) {
        let mut recent_feedback = self.recent_feedback.lock().expect("recent_feedback lock");
        let summary = feedback.world_delta_summary.unwrap_or_else(|| {
            format!(
                "action_id={}; success={}; reject_reason={}",
                feedback.action_id,
                feedback.success,
                feedback.reject_reason.unwrap_or_else(|| "none".to_string())
            )
        });
        if recent_feedback.len() >= MAX_RECENT_FEEDBACK {
            recent_feedback.pop_front();
        }
        recent_feedback.push_back(summary);
    }

    fn handle_agent_chat(
        &self,
        request: ProviderAgentChatRequest,
        route_label: Option<&str>,
        invoker: &dyn AgentInvoker,
    ) -> Result<ProviderAgentChatResponse, String> {
        let agent_id = request.agent_id.trim();
        if agent_id.is_empty() {
            return Err("agent_chat requires non-empty agent_id".to_string());
        }
        let player_id = request.player_id.trim();
        if player_id.is_empty() {
            return Err("agent_chat requires non-empty player_id".to_string());
        }
        let message = request.message.trim();
        if message.is_empty() {
            return Err("agent_chat requires non-empty message".to_string());
        }
        let chat_request_key = provider_agent_chat_log_key(&request);
        let started = Instant::now();
        info!(
            target: "oasis7_provider_local_bridge",
            chat_request_key = chat_request_key.as_str(),
            agent_id,
            player_id,
            world_time = request.world_time,
            route_label_present = route_label.is_some(),
            backend = ?self.options.provider_backend,
            "provider agent chat request started"
        );
        if self.options.mode == ProviderMode::Mock {
            info!(
                target: "oasis7_provider_local_bridge",
                chat_request_key = chat_request_key.as_str(),
                agent_id,
                player_id,
                elapsed_ms = started.elapsed().as_millis() as u64,
                "provider agent chat request succeeded"
            );
            return Ok(ProviderAgentChatResponse {
                agent_id: agent_id.to_string(),
                message: format!("[local-mock-chat] {agent_id} 收到：{message}"),
                location_id: request.location_id,
            });
        }

        self.active_requests.fetch_add(1, Ordering::SeqCst);
        let prompt = build_agent_chat_prompt(&request);
        let session_key = format!(
            "agent:{}:subagent:world-simulator-chat:{}",
            self.options.provider_agent_id, agent_id
        );
        let invoke_result = invoker.invoke(AgentInvocation {
            provider_cli_bin: self.options.provider_cli_bin.clone(),
            agent_id: self.options.provider_agent_id.clone(),
            thinking: self.options.provider_thinking.clone(),
            session_key: session_key.clone(),
            timeout_seconds: timeout_seconds_from_budget(60_000),
            prompt,
            idempotency_key: agent_chat_idempotency_key(&session_key, &request),
            chat_request_key: Some(chat_request_key.clone()),
            route_label: route_label.map(ToOwned::to_owned),
        });
        self.active_requests.fetch_sub(1, Ordering::SeqCst);

        match invoke_result {
            Ok(output) => {
                self.set_last_error(None);
                info!(
                    target: "oasis7_provider_local_bridge",
                    chat_request_key = chat_request_key.as_str(),
                    agent_id,
                    player_id,
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    provider_version = output.provider_version.as_deref().unwrap_or("unknown"),
                    "provider agent chat request succeeded"
                );
                Ok(ProviderAgentChatResponse {
                    agent_id: agent_id.to_string(),
                    message: summarize_text(output.text.as_str(), 700),
                    location_id: request.location_id,
                })
            }
            Err(err) => {
                let detail = format!("provider_agent_chat_unreachable: {err}");
                self.set_last_error(Some(detail.clone()));
                warn!(
                    target: "oasis7_provider_local_bridge",
                    chat_request_key = chat_request_key.as_str(),
                    agent_id,
                    player_id,
                    elapsed_ms = started.elapsed().as_millis() as u64,
                    error = detail.as_str(),
                    "provider agent chat request failed"
                );
                Err(detail)
            }
        }
    }

    fn resolve_newapi_bridge_route_label(&self, bearer: &str) -> Option<String> {
        let normalized = bearer.trim();
        if normalized.is_empty() {
            return None;
        }
        let payload = self.load_newapi_bridge_state()?;
        let bindings = payload.get("bindings").and_then(Value::as_array)?;
        let project_bindings = payload.get("project_bindings").and_then(Value::as_array)?;
        let (by_ref, by_bridge_user_id) = match parse_newapi_bridge_bearer_selector(normalized) {
            Some(selector) => selector,
            None => return None,
        };
        bindings.iter().find_map(|entry| {
            let object = entry.as_object()?;
            if object.get("status").and_then(Value::as_str) != Some("active") {
                return None;
            }
            let ref_match = by_ref.is_some_and(|value| {
                object.get("newapi_user_ref").and_then(Value::as_str) == Some(value)
            });
            let user_match = by_bridge_user_id.is_some_and(|value| {
                object.get("bridge_user_id").and_then(Value::as_str) == Some(value)
            });
            let bridge_user_id = object.get("bridge_user_id").and_then(Value::as_str)?;
            let has_token = project_bindings.iter().any(|project| {
                let Some(project) = project.as_object() else {
                    return false;
                };
                project.get("bridge_user_id").and_then(Value::as_str) == Some(bridge_user_id)
                    && project
                        .get("token_key")
                        .and_then(Value::as_str)
                        .is_some_and(|value| !value.trim().is_empty())
            });
            if (ref_match || user_match) && has_token {
                Some(normalized.to_string())
            } else {
                None
            }
        })
    }

    fn load_newapi_bridge_state(&self) -> Option<Arc<Value>> {
        load_cached_newapi_bridge_state(&self.newapi_bridge_state_cache)
    }

    fn handle_decision(
        &self,
        request: DecisionRequest,
        route_label: Option<&str>,
        invoker: &dyn AgentInvoker,
    ) -> DecisionResponse {
        if let Err(err) = request.validate_contract() {
            self.set_last_error(Some(err.to_string()));
            return self.provider_error_response(err.code, err.message, false, None, None, None);
        }
        if let Some(err) = validate_profile(request.agent_profile.as_deref()) {
            self.set_last_error(Some(err.clone()));
            return self.provider_error_response(
                "unsupported_agent_profile",
                err,
                false,
                None,
                None,
                None,
            );
        }
        if self.options.mode == ProviderMode::Mock {
            return self.handle_mock_decision(request);
        }

        self.active_requests.fetch_add(1, Ordering::SeqCst);
        let started_at = Instant::now();
        let recent_feedback = self
            .recent_feedback
            .lock()
            .expect("recent_feedback lock")
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        let prompt = build_decision_prompt(&request, recent_feedback.as_slice());
        let session_key = build_session_key(&request, self.options.provider_agent_id.as_str());
        let timeout_seconds = timeout_seconds_from_budget(request.timeout_budget_ms);
        let invoke_result = invoker.invoke(AgentInvocation {
            provider_cli_bin: self.options.provider_cli_bin.clone(),
            agent_id: self.options.provider_agent_id.clone(),
            thinking: self.options.provider_thinking.clone(),
            session_key: session_key.clone(),
            timeout_seconds,
            prompt,
            idempotency_key: format!("{session_key}-{timeout_seconds}"),
            chat_request_key: None,
            route_label: route_label.map(ToOwned::to_owned),
        });
        self.active_requests.fetch_sub(1, Ordering::SeqCst);
        let latency_ms = started_at.elapsed().as_millis() as u64;

        match invoke_result {
            Ok(output) => match parse_model_decision(
                request.observation.agent_id.as_str(),
                &request,
                output.text.as_str(),
            ) {
                Ok((decision, schema_repair_count)) => {
                    let (decision, module_command) = match decision {
                        ProviderDecision::ModuleCommand { module_command } => {
                            (ProviderDecision::Wait, Some(module_command))
                        }
                        decision => (decision, None),
                    };
                    let (decision, guardrail_note) = apply_profile_guardrails(&request, decision);
                    self.set_last_error(None);
                    let mut tool_trace = Vec::new();
                    if let Some(route_note) = output.route_note.clone() {
                        tool_trace.push(route_note);
                    }
                    if let Some(note) = guardrail_note.clone() {
                        tool_trace.push(note);
                    }
                    DecisionResponse {
                        decision,
                        module_command,
                        provider_error: None,
                        diagnostics: ProviderDiagnostics {
                            provider_id: Some(self.provider_id().to_string()),
                            provider_version: output.provider_version.clone(),
                            latency_ms: Some(latency_ms.max(output.duration_ms.unwrap_or(0))),
                            retry_count: 0,
                        },
                        trace_payload: ProviderTraceEnvelope {
                            provider_id: Some(self.provider_id().to_string()),
                            input_summary: Some(summarize_text(output.prompt.as_str(), 512)),
                            output_summary: Some(match (&output.route_note, &guardrail_note) {
                                (Some(route_note), Some(note)) => format!(
                                    "{}; {}; model_output={}",
                                    route_note,
                                    note,
                                    summarize_text(output.text.as_str(), 512)
                                ),
                                (Some(route_note), None) => format!(
                                    "{}; model_output={}",
                                    route_note,
                                    summarize_text(output.text.as_str(), 512)
                                ),
                                (None, Some(note)) => format!(
                                    "{}; model_output={}",
                                    note,
                                    summarize_text(output.text.as_str(), 512)
                                ),
                                (None, None) => summarize_text(output.text.as_str(), 512),
                            }),
                            latency_ms: Some(latency_ms.max(output.duration_ms.unwrap_or(0))),
                            transcript: vec![
                                ProviderTranscriptEntry {
                                    role: "user".to_string(),
                                    content: summarize_text(output.prompt.as_str(), 4000),
                                },
                                ProviderTranscriptEntry {
                                    role: "assistant".to_string(),
                                    content: summarize_text(output.text.as_str(), 4000),
                                },
                            ],
                            tool_trace,
                            token_usage: Some(ProviderTokenUsage {
                                prompt_tokens: output.prompt_tokens,
                                completion_tokens: output.completion_tokens,
                                total_tokens: output.total_tokens,
                            }),
                            cost_cents: None,
                            upstream_trace: output.upstream_trace.clone(),
                            schema_repair_count,
                        },
                        memory_write_intents: Vec::new(),
                    }
                }
                Err(err) => {
                    let detail = format!("bridge_model_output_invalid: {err}");
                    self.set_last_error(Some(detail.clone()));
                    let (decision, guardrail_note) =
                        apply_profile_guardrails(&request, ProviderDecision::Wait);
                    let mut tool_trace = Vec::new();
                    if let Some(route_note) = output.route_note.clone() {
                        tool_trace.push(route_note);
                    }
                    tool_trace.push(detail.clone());
                    if let Some(note) = guardrail_note.clone() {
                        tool_trace.push(note);
                    }
                    DecisionResponse {
                        decision,
                        module_command: None,
                        provider_error: None,
                        diagnostics: ProviderDiagnostics {
                            provider_id: Some(self.provider_id().to_string()),
                            provider_version: output.provider_version.clone(),
                            latency_ms: Some(latency_ms.max(output.duration_ms.unwrap_or(0))),
                            retry_count: 0,
                        },
                        trace_payload: ProviderTraceEnvelope {
                            provider_id: Some(self.provider_id().to_string()),
                            input_summary: Some(summarize_text(output.prompt.as_str(), 512)),
                            output_summary: Some(match guardrail_note {
                                Some(note) => format!(
                                    "invalid_model_output: {}; {}; raw={}",
                                    detail,
                                    note,
                                    summarize_text(output.text.as_str(), 512)
                                ),
                                None => format!(
                                    "invalid_model_output: {}; raw={}",
                                    detail,
                                    summarize_text(output.text.as_str(), 512)
                                ),
                            }),
                            latency_ms: Some(latency_ms.max(output.duration_ms.unwrap_or(0))),
                            transcript: vec![
                                ProviderTranscriptEntry {
                                    role: "user".to_string(),
                                    content: summarize_text(output.prompt.as_str(), 4000),
                                },
                                ProviderTranscriptEntry {
                                    role: "assistant".to_string(),
                                    content: summarize_text(output.text.as_str(), 4000),
                                },
                            ],
                            tool_trace,
                            token_usage: Some(ProviderTokenUsage {
                                prompt_tokens: output.prompt_tokens,
                                completion_tokens: output.completion_tokens,
                                total_tokens: output.total_tokens,
                            }),
                            cost_cents: None,
                            upstream_trace: output.upstream_trace.clone(),
                            schema_repair_count: 1,
                        },
                        memory_write_intents: Vec::new(),
                    }
                }
            },
            Err(err) => {
                let detail = format!("provider_gateway_unreachable: {err}");
                self.set_last_error(Some(detail.clone()));
                self.provider_error_response(
                    "provider_gateway_unreachable",
                    detail,
                    true,
                    Some(latency_ms),
                    Some("decision invocation failed".to_string()),
                    Some(provider_error_upstream_trace(err.as_str())),
                )
            }
        }
    }

    fn handle_mock_decision(&self, request: DecisionRequest) -> DecisionResponse {
        self.set_last_error(None);
        let decision = deterministic_mock_decision(&request);
        let action_label = provider_decision_label(&decision);
        DecisionResponse {
            decision,
            module_command: None,
            provider_error: None,
            diagnostics: ProviderDiagnostics {
                provider_id: Some(self.provider_id().to_string()),
                provider_version: Some(format!("{}-mock", env!("CARGO_PKG_VERSION"))),
                latency_ms: Some(0),
                retry_count: 0,
            },
            trace_payload: ProviderTraceEnvelope {
                provider_id: Some(self.provider_id().to_string()),
                input_summary: Some(format!(
                    "deterministic_mock request agent={} world_time={}",
                    request.observation.agent_id, request.observation.world_time
                )),
                output_summary: Some(format!(
                    "deterministic_mock decision={action_label}; no_provider_cli=true; no_billing=true; no_quota=true"
                )),
                latency_ms: Some(0),
                transcript: Vec::new(),
                tool_trace: vec![
                    "deterministic_mock=no_provider_cli".to_string(),
                    "no_billing=true".to_string(),
                    "no_quota=true".to_string(),
                ],
                token_usage: None,
                cost_cents: None,
                upstream_trace: None,
                schema_repair_count: 0,
            },
            memory_write_intents: Vec::new(),
        }
    }

    fn provider_error_response(
        &self,
        code: impl Into<String>,
        message: impl Into<String>,
        retryable: bool,
        latency_ms: Option<u64>,
        output_summary: Option<String>,
        upstream_trace: Option<Value>,
    ) -> DecisionResponse {
        DecisionResponse {
            decision: ProviderDecision::Wait,
            module_command: None,
            provider_error: Some(ProviderErrorEnvelope {
                code: code.into(),
                message: message.into(),
                retryable,
            }),
            diagnostics: ProviderDiagnostics {
                provider_id: Some(self.provider_id().to_string()),
                provider_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                latency_ms,
                retry_count: 0,
            },
            trace_payload: ProviderTraceEnvelope {
                provider_id: Some(self.provider_id().to_string()),
                input_summary: None,
                output_summary,
                latency_ms,
                transcript: Vec::new(),
                tool_trace: Vec::new(),
                token_usage: None,
                cost_cents: None,
                upstream_trace,
                schema_repair_count: 0,
            },
            memory_write_intents: Vec::new(),
        }
    }

    fn set_last_error(&self, detail: Option<String>) {
        *self.last_error.lock().expect("last_error lock") = detail;
    }
}

fn build_agent_chat_prompt(request: &ProviderAgentChatRequest) -> String {
    let context = json!({
        "agent_id": request.agent_id,
        "player_id": request.player_id,
        "world_time": request.world_time,
        "location_id": request.location_id,
        "resources": request.resources,
        "recent_feedback": request.recent_feedback,
        "player_message": request.message,
    });
    format!(
        concat!(
            "You are the selected oasis7 agent. Reply directly to the player in the same language as their message. ",
            "Do not return JSON. Do not claim unavailable facts. ",
            "If asked where you are, use location_id exactly. If asked about nearby resources, use resources exactly. ",
            "Keep the reply under 80 Chinese characters or 60 English words. Context follows:\n{}"
        ),
        context
    )
}

fn provider_error_upstream_trace(error: &str) -> Value {
    let diagnostics = error_diagnostics_json(error);
    json!({
        "stage": "decision_invocation",
        "error_summary": summarize_text(error, 500),
        "diagnostics": diagnostics,
        "diagnostics_present": !diagnostics.is_null(),
    })
}

#[derive(Debug, Clone)]
struct AgentInvocation {
    provider_cli_bin: String,
    agent_id: String,
    thinking: String,
    session_key: String,
    timeout_seconds: u64,
    prompt: String,
    idempotency_key: String,
    chat_request_key: Option<String>,
    route_label: Option<String>,
}

#[derive(Debug, Clone)]
struct AgentInvocationOutput {
    prompt: String,
    text: String,
    provider_version: Option<String>,
    duration_ms: Option<u64>,
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
    total_tokens: Option<u64>,
    route_note: Option<String>,
    upstream_trace: Option<Value>,
}

trait AgentInvoker: Send + Sync {
    fn invoke(&self, invocation: AgentInvocation) -> Result<AgentInvocationOutput, String>;
}

#[derive(Debug, Clone, Default)]
struct ProviderCliInvoker;

#[derive(Debug, Clone, Default)]
struct RustDirectLetaiInvoker;

impl AgentInvoker for ProviderCliInvoker {
    fn invoke(&self, invocation: AgentInvocation) -> Result<AgentInvocationOutput, String> {
        match invoke_gateway_agent(invocation.clone()) {
            Ok(output) => Ok(output),
            Err(err) if should_fallback_to_local_agent(err.as_str()) => {
                invoke_local_agent(invocation, err.as_str())
            }
            Err(err) => Err(err),
        }
    }
}

impl AgentInvoker for RustDirectLetaiInvoker {
    fn invoke(&self, invocation: AgentInvocation) -> Result<AgentInvocationOutput, String> {
        invoke_rust_direct_letai(invocation)
    }
}

fn invoke_gateway_agent(invocation: AgentInvocation) -> Result<AgentInvocationOutput, String> {
    let params = build_gateway_agent_params(&invocation)
        .map_err(|err| format!("serialize gateway call params failed: {err}"))?;
    let rpc_timeout_ms = invocation
        .timeout_seconds
        .saturating_mul(1000)
        .saturating_add(2000);
    let started = Instant::now();
    let output = Command::new(invocation.provider_cli_bin.as_str())
        .arg("gateway")
        .arg("call")
        .arg("agent")
        .arg("--expect-final")
        .arg("--json")
        .arg("--timeout")
        .arg(rpc_timeout_ms.to_string())
        .arg("--params")
        .arg(params)
        .envs(route_label_env(invocation.route_label.as_deref()))
        .env(
            oasis7::observability::TRACE_SESSION_ID_ENV,
            resolve_trace_session_id(TRACE_SESSION_PROCESS_LABEL),
        )
        .output()
        .map_err(|err| format!("spawn provider gateway call agent failed: {err}"))?;
    if !output.status.success() {
        let summary = provider_cli_failure_summary(
            "gateway_call_agent",
            &output,
            started.elapsed(),
            invocation.route_label.as_deref().is_some(),
            rpc_timeout_ms,
        );
        error!(target: "oasis7_provider_local_bridge", %summary, "provider cli invocation failed");
        return Err(format!("provider gateway call agent failed: {summary}"));
    }
    let payload = String::from_utf8(output.stdout)
        .map_err(|err| format!("provider gateway call agent stdout was not utf8: {err}"))?;
    agent_output_from_json(invocation.prompt, payload.as_str(), None)
}

fn invoke_local_agent(
    invocation: AgentInvocation,
    gateway_error: &str,
) -> Result<AgentInvocationOutput, String> {
    let session_id = local_session_id_from_session_key(invocation.session_key.as_str());
    let timeout_ms = invocation.timeout_seconds.saturating_mul(1000);
    let started = Instant::now();
    let output = Command::new(invocation.provider_cli_bin.as_str())
        .arg("agent")
        .arg("--agent")
        .arg(invocation.agent_id.as_str())
        .arg("--message")
        .arg(invocation.prompt.as_str())
        .arg("--local")
        .arg("--session-id")
        .arg(session_id.as_str())
        .arg("--thinking")
        .arg(invocation.thinking.as_str())
        .arg("--timeout")
        .arg(timeout_ms.to_string())
        .arg("--json")
        .envs(route_label_env(invocation.route_label.as_deref()))
        .env(
            oasis7::observability::TRACE_SESSION_ID_ENV,
            resolve_trace_session_id(TRACE_SESSION_PROCESS_LABEL),
        )
        .output()
        .map_err(|err| format!("spawn provider local agent failed: {err}"))?;
    if !output.status.success() {
        let summary = provider_cli_failure_summary(
            "local_agent_fallback",
            &output,
            started.elapsed(),
            invocation.route_label.as_deref().is_some(),
            timeout_ms,
        );
        error!(target: "oasis7_provider_local_bridge", %summary, "provider cli fallback failed");
        return Err(format!(
            "provider gateway fallback failed after `{}`; local agent failed: {}",
            summarize_text(gateway_error, 240),
            summary,
        ));
    }
    let payload = String::from_utf8(output.stdout)
        .map_err(|err| format!("provider local agent stdout was not utf8: {err}"))?;
    agent_output_from_json(
        invocation.prompt,
        payload.as_str(),
        Some(format!(
            "invocation_fallback=local_embedded; reason={}",
            summarize_text(gateway_error, 240)
        )),
    )
}

fn provider_cli_failure_summary(
    mode: &str,
    output: &std::process::Output,
    elapsed: Duration,
    route_label_present: bool,
    timeout_ms: u64,
) -> String {
    let stderr = String::from_utf8_lossy(output.stderr.as_slice());
    let stdout = String::from_utf8_lossy(output.stdout.as_slice());
    json!({
        "mode": mode,
        "status": output.status.to_string(),
        "elapsed_ms": elapsed.as_millis(),
        "timeout_ms": timeout_ms,
        "route_label_present": route_label_present,
        "stderr": text_diagnostic_summary(stderr.as_ref(), true),
        "stdout": text_diagnostic_summary(stdout.as_ref(), false),
        "stderr_events": stderr_json_events(stderr.as_ref(), 8),
    })
    .to_string()
}

fn text_diagnostic_summary(text: &str, include_tail: bool) -> Value {
    let trimmed = text.trim();
    let mut summary = json!({
        "len": trimmed.len(),
        "sha256_16": short_sha256(trimmed),
        "truncated": trimmed.len() > 320,
    });
    if include_tail {
        summary["tail"] = Value::String(summarize_text(trimmed, 320));
    }
    summary
}

fn stderr_json_events(text: &str, max_events: usize) -> Vec<Value> {
    let mut events = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('{') || !trimmed.ends_with('}') {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
            continue;
        };
        if value.get("event").is_some() {
            events.push(value);
        }
    }
    if events.len() > max_events {
        events.split_off(events.len() - max_events)
    } else {
        events
    }
}

fn short_sha256(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hasher.finalize();
    hex::encode(&digest[..8])
}

fn agent_chat_idempotency_key(session_key: &str, request: &ProviderAgentChatRequest) -> String {
    let stable_chat_key = short_sha256(
        format!(
            "{}\0{}\0{}\0{}",
            request.world_time, request.agent_id, request.player_id, request.message
        )
        .as_str(),
    );
    format!("{session_key}-{}-{stable_chat_key}", request.world_time)
}

fn main() {
    init_tracing(TRACE_SESSION_PROCESS_LABEL);
    let trace_session_id = resolve_trace_session_id(TRACE_SESSION_PROCESS_LABEL);
    let args: Vec<String> = env::args().collect();
    let options = match parse_options(args.iter().skip(1).map(String::as_str)) {
        Ok(options) => options,
        Err(err) => {
            error!(trace_session_id = %trace_session_id, error = %err, "failed to parse provider local bridge options");
            print_help();
            std::process::exit(1);
        }
    };
    let state = match ProviderState::new(options.clone()) {
        Ok(state) => state,
        Err(err) => {
            error!(trace_session_id = %trace_session_id, error = %err, "failed to initialize provider local bridge state");
            std::process::exit(1);
        }
    };
    let listener = TcpListener::bind(options.bind_addr.as_str())
        .unwrap_or_else(|err| panic!("bind {} failed: {err}", options.bind_addr));
    info!(
        trace_session_id = %trace_session_id,
        bind_addr = %options.bind_addr,
        provider_agent_id = %options.provider_agent_id,
        gateway_health_url = %options.gateway_health_url,
        provider_backend = ?options.provider_backend,
        "provider local bridge listening"
    );
    println!(
        "oasis7_provider_local_bridge listening on http://{} (agent={}, gateway_health={})",
        options.bind_addr, options.provider_agent_id, options.gateway_health_url
    );
    let invoker: Arc<dyn AgentInvoker> = match options.provider_backend {
        ProviderBackend::RustDirectLetai => Arc::new(RustDirectLetaiInvoker),
        ProviderBackend::LegacyCli => Arc::new(ProviderCliInvoker),
    };
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let state = state.clone();
        let invoker = invoker.clone();
        std::thread::spawn(move || {
            if let Err(err) = handle_connection(&mut stream, &state, invoker.as_ref()) {
                let stderr_message =
                    format!("oasis7_provider_local_bridge connection error: {err}");
                emit_stderr_or_event(
                    Level::WARN,
                    stderr_message.as_str(),
                    "provider local bridge connection failed",
                );
            }
        });
    }
}

fn parse_options<'a>(args: impl Iterator<Item = &'a str>) -> Result<CliOptions, String> {
    let mut options = CliOptions::default();
    let mut iter = args.peekable();
    while let Some(arg) = iter.next() {
        match arg {
            "--bind" => {
                options.bind_addr = required_value(&mut iter, "--bind")?.to_string();
            }
            "--provider-cli-bin" => {
                options.provider_cli_bin =
                    required_value(&mut iter, "--provider-cli-bin")?.to_string();
                options.provider_backend = ProviderBackend::LegacyCli;
            }
            "--provider-backend" => {
                options.provider_backend =
                    parse_provider_backend(required_value(&mut iter, "--provider-backend")?)?;
            }
            "--provider-agent" => {
                options.provider_agent_id =
                    required_value(&mut iter, "--provider-agent")?.to_string();
            }
            "--provider-thinking" => {
                options.provider_thinking =
                    required_value(&mut iter, "--provider-thinking")?.to_string();
            }
            "--gateway-health-url" => {
                options.gateway_health_url =
                    required_value(&mut iter, "--gateway-health-url")?.to_string();
            }
            "--auth-token" => {
                options.auth_token = Some(required_value(&mut iter, "--auth-token")?.to_string());
            }
            "--auth-route-map" => {
                let path = required_value(&mut iter, "--auth-route-map")?;
                options.auth_route_map = load_auth_route_map(path)?;
            }
            "--auth-route-from-bearer" => {
                options.auth_route_from_bearer = true;
            }
            "--mode" => {
                options.mode = parse_provider_mode(required_value(&mut iter, "--mode")?)?;
            }
            "--mock" => {
                options.mode = ProviderMode::Mock;
            }
            "-h" | "--help" => return Err("help requested".to_string()),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    if options.bind_addr.trim().is_empty() {
        return Err("--bind requires a non-empty value".to_string());
    }
    if let Some(token) = options.auth_token.as_deref() {
        if token.trim().len() < MIN_BRIDGE_AUTH_TOKEN_LEN {
            return Err(format!(
                "--auth-token must be at least {MIN_BRIDGE_AUTH_TOKEN_LEN} characters"
            ));
        }
    }
    Ok(options)
}

fn required_value<'a>(
    iter: &mut std::iter::Peekable<impl Iterator<Item = &'a str>>,
    flag: &str,
) -> Result<&'a str, String> {
    iter.next()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn print_help() {
    eprintln!(concat!(
        "Usage: oasis7_provider_local_bridge [options]\n\n",
        "  --bind <host:port>            Loopback bind address (default: 127.0.0.1:5841)\n",
        "  --provider-backend <rust-direct-letai|legacy-cli>\n",
        "                                provider upstream backend (default: rust-direct-letai)\n",
        "  --provider-cli-bin <path>     legacy provider CLI path; implies --provider-backend legacy-cli\n",
        "  --provider-agent <id>         provider agent id (default: main)\n",
        "  --provider-thinking <level>   provider thinking level (default: off)\n",
        "  --gateway-health-url <url>    provider gateway health URL\n",
        "  --auth-token <token>          Optional single bearer token for bridge endpoints\n",
        "  --auth-route-map <path>       Optional JSON file mapping bearer token to route label\n",
        "  --auth-route-from-bearer      Treat request bearer token itself as route label\n",
        "  --mode <real|mock>            Bridge decision backend (default: real)\n",
        "  --mock                        Alias for --mode mock; deterministic no-billing local testing\n",
    ));
}

fn route_label_env(route_label: Option<&str>) -> Vec<(&'static str, String)> {
    vec![(
        "OASIS7_REMOTE_LLM_ROUTE_LABEL",
        route_label.unwrap_or_default().to_string(),
    )]
}

fn load_auth_route_map(path: &str) -> Result<BTreeMap<String, String>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("read auth route map `{path}` failed: {err}"))?;
    let parsed = serde_json::from_str::<BTreeMap<String, String>>(raw.as_str())
        .map_err(|err| format!("parse auth route map `{path}` failed: {err}"))?;
    let normalized = parsed
        .into_iter()
        .filter_map(|(token, route)| {
            let token = token.trim().to_string();
            let route = route.trim().to_string();
            if token.len() < MIN_BRIDGE_AUTH_TOKEN_LEN || route.is_empty() {
                None
            } else {
                Some((token, route))
            }
        })
        .collect::<BTreeMap<_, _>>();
    if normalized.is_empty() {
        return Err(format!(
            "auth route map `{path}` did not contain any usable entries"
        ));
    }
    Ok(normalized)
}

fn validate_profile(agent_profile: Option<&str>) -> Option<String> {
    let Some(agent_profile) = agent_profile
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return None;
    };
    if matches!(agent_profile, DEFAULT_PROVIDER_AGENT_PROFILE) {
        None
    } else {
        Some(format!(
            "unsupported agent_profile `{agent_profile}`; expected {DEFAULT_PROVIDER_AGENT_PROFILE}"
        ))
    }
}

fn parse_provider_mode(raw: &str) -> Result<ProviderMode, String> {
    match raw.trim() {
        "real" | "provider" | "provider_cli" => Ok(ProviderMode::Real),
        "mock" | "deterministic" | "local_mock" | "local-mock" => Ok(ProviderMode::Mock),
        other => Err(format!(
            "unsupported provider bridge mode `{other}`; expected real or mock"
        )),
    }
}

fn parse_provider_backend(raw: &str) -> Result<ProviderBackend, String> {
    match raw.trim() {
        "rust-direct-letai" | "rust_direct_letai" | "direct-letai" | "direct" | "rust" => {
            Ok(ProviderBackend::RustDirectLetai)
        }
        "legacy-cli" | "legacy_cli" | "cli" | "provider-cli" => Ok(ProviderBackend::LegacyCli),
        other => Err(format!(
            "unsupported provider backend `{other}`; expected rust-direct-letai or legacy-cli"
        )),
    }
}

fn default_gateway_health_url() -> String {
    let config_path = env::var("HOME").ok().map(PathBuf::from).map(|home| {
        let dir = [".open", "claw"].concat();
        let file = ["open", "claw", ".json"].concat();
        home.join(dir).join(file)
    });
    if let Some(config_path) = config_path {
        if let Ok(raw) = fs::read_to_string(config_path) {
            if let Ok(value) = serde_json::from_str::<Value>(raw.as_str()) {
                if let Some(port) = value
                    .get("gateway")
                    .and_then(|gateway| gateway.get("port"))
                    .and_then(Value::as_u64)
                {
                    return format!("http://127.0.0.1:{port}/health");
                }
            }
        }
    }
    "http://127.0.0.1:18789/health".to_string()
}
