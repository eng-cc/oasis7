use std::collections::BTreeMap;
use std::collections::VecDeque;
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;

use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use oasis7::observability::{emit_stderr_or_event, init_tracing, resolve_trace_session_id};
use oasis7::simulator::{
    DecisionRequest, DecisionResponse, FeedbackEnvelope, ProviderDecision, ProviderDiagnostics,
    ProviderErrorEnvelope, ProviderHealth, ProviderInfo, ProviderTokenUsage, ProviderTraceEnvelope,
    ProviderTranscriptEntry,
};
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::json;
use serde_json::Value;
use tracing::{error, info, Level};

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
const DEFAULT_LETAI_BASE_URL: &str = "https://api.letai.run/v1";
const DEFAULT_LETAI_MAX_OUTPUT_TOKENS: u64 = 256;
const DEFAULT_LETAI_TEMPERATURE: f64 = 0.0;
const DEFAULT_LETAI_RETRY_COUNT: u64 = 2;
const DEFAULT_LETAI_RETRY_DELAY_MS: u64 = 1000;
const DEFAULT_LETAI_USER_AGENT: &str = "oasis7-letai-provider-rust/1.0";
const DEFAULT_LETAI_PLATFORM_BASE_URL: &str = "https://api.letai.run";
const DEFAULT_LETAI_AUTO_TOPUP_RETRY_COUNT: u64 = 3;
const DEFAULT_LETAI_AUTO_TOPUP_RETRY_DELAY_MS: u64 = 1000;
const LETAI_QUOTA_UNITS_PER_USD: f64 = 500_000.0;

fn default_provider_cli_bin() -> String {
    env::var("OASIS7_PROVIDER_CLI_BIN")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| ["open", "claw"].concat())
}

#[path = "oasis7_provider_local_bridge/auth_support.rs"]
mod auth_support;
#[allow(dead_code)]
#[path = "oasis7_newapi_bridge_service/credit_adapter.rs"]
mod credit_adapter;
#[path = "oasis7_provider_local_bridge/http_bridge_support.rs"]
mod http_bridge_support;
#[path = "oasis7_provider_local_bridge/support.rs"]
mod support;
#[cfg(test)]
mod api {
    use std::io::Write;

    pub(super) fn write_http_response<W: Write>(
        stream: &mut W,
        status_code: u16,
        content_type: &str,
        body: &[u8],
        head_only: bool,
    ) -> Result<(), String> {
        let status_text = match status_code {
            200 => "OK",
            201 => "Created",
            400 => "Bad Request",
            404 => "Not Found",
            405 => "Method Not Allowed",
            409 => "Conflict",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            _ => "Error",
        };
        let headers = format!(
            "HTTP/1.1 {status_code} {status_text}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream
            .write_all(headers.as_bytes())
            .map_err(|err| format!("write response header failed: {err}"))?;
        if !head_only {
            stream
                .write_all(body)
                .map_err(|err| format!("write response body failed: {err}"))?;
        }
        stream
            .flush()
            .map_err(|err| format!("flush response failed: {err}"))
    }
}
#[cfg(test)]
#[path = "oasis7_provider_local_bridge/tests.rs"]
mod tests;

use self::auth_support::{
    load_cached_newapi_bridge_state, parse_newapi_bridge_bearer_selector, NewapiBridgeStateCache,
};
#[path = "oasis7_provider_local_bridge/agent_decision.rs"]
mod agent_decision;
use self::agent_decision::{
    apply_profile_guardrails, build_decision_prompt, build_gateway_agent_params, build_session_key,
    deterministic_mock_decision, parse_model_decision, provider_decision_label, summarize_text,
    timeout_seconds_from_budget,
};
use self::credit_adapter::{LetaiOpenApiAdapter, LetaiUserTopupRequest};
use self::http_bridge_support::handle_connection;
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

    fn load_newapi_bridge_state(&self) -> Option<Value> {
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
            return self.provider_error_response(err.code, err.message, false, None, None);
        }
        if let Some(err) = validate_profile(request.agent_profile.as_deref()) {
            self.set_last_error(Some(err.clone()));
            return self.provider_error_response(
                "unsupported_agent_profile",
                err,
                false,
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
    ) -> DecisionResponse {
        DecisionResponse {
            decision: ProviderDecision::Wait,
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
                schema_repair_count: 0,
            },
            memory_write_intents: Vec::new(),
        }
    }

    fn set_last_error(&self, detail: Option<String>) {
        *self.last_error.lock().expect("last_error lock") = detail;
    }
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

#[derive(Debug, Clone)]
struct LetaiChatConfig {
    base_url: String,
    api_key: String,
    model: String,
    system_prompt: Option<String>,
    max_output_tokens: u64,
    temperature: f64,
    stream: bool,
    response_format_json_object: bool,
    retry_count: u64,
    retry_delay_ms: u64,
    auto_topup_usd: Option<String>,
    platform_base_url: String,
    platform_key: Option<String>,
    platform_user_id: Option<String>,
    auto_topup_retry_count: u64,
    auto_topup_retry_delay_ms: u64,
}

#[derive(Debug, Clone)]
struct LetaiChatResult {
    content: String,
    model: String,
    duration_ms: u64,
    prompt_tokens: Option<u64>,
    completion_tokens: Option<u64>,
    total_tokens: Option<u64>,
}

fn invoke_rust_direct_letai(invocation: AgentInvocation) -> Result<AgentInvocationOutput, String> {
    let config = load_letai_chat_config(invocation.route_label.as_deref())?;
    let started = Instant::now();
    let result = send_letai_chat_completion_with_retries(&config, &invocation)?;
    Ok(AgentInvocationOutput {
        prompt: invocation.prompt,
        text: result.content,
        provider_version: Some(format!("letai/{}", result.model)),
        duration_ms: Some(result.duration_ms.max(started.elapsed().as_millis() as u64)),
        prompt_tokens: result.prompt_tokens,
        completion_tokens: result.completion_tokens,
        total_tokens: result.total_tokens,
        route_note: Some("invocation_backend=rust_direct_letai".to_string()),
    })
}

fn load_letai_chat_config(route_label: Option<&str>) -> Result<LetaiChatConfig, String> {
    let route = load_letai_routes_path_config(route_label)?;
    let base_url = normalize_base_url(
        route_or_env_optional(
            &route,
            "base_url",
            &["OASIS7_REMOTE_LLM_BASE_URL", "LETAI_BASE_URL"],
        )
        .unwrap_or_else(|| DEFAULT_LETAI_BASE_URL.to_string())
        .as_str(),
    );
    let api_key = route_string(&route, "api_key")
        .or_else(|| {
            route_label
                .and_then(load_newapi_bridge_state_route_token)
                .or_else(|| env_optional(&["OASIS7_REMOTE_LLM_API_KEY", "LETAI_API_KEY"]))
        })
        .ok_or_else(|| "missing required remote LLM api key".to_string())?;
    let model = route_or_env_optional(&route, "model", &["OASIS7_REMOTE_LLM_MODEL", "LETAI_MODEL"])
        .ok_or_else(|| "missing required remote LLM model".to_string())?;
    Ok(LetaiChatConfig {
        base_url,
        api_key,
        model,
        system_prompt: route_or_env_optional(
            &route,
            "system_prompt",
            &["OASIS7_REMOTE_LLM_SYSTEM_PROMPT", "LETAI_SYSTEM_PROMPT"],
        ),
        max_output_tokens: route_or_env_u64(
            &route,
            "max_output_tokens",
            DEFAULT_LETAI_MAX_OUTPUT_TOKENS,
            &[
                "OASIS7_REMOTE_LLM_MAX_OUTPUT_TOKENS",
                "LETAI_MAX_OUTPUT_TOKENS",
            ],
        )?,
        temperature: route_or_env_f64(
            &route,
            "temperature",
            DEFAULT_LETAI_TEMPERATURE,
            &["OASIS7_REMOTE_LLM_TEMPERATURE", "LETAI_TEMPERATURE"],
        )?,
        stream: route_or_env_bool(
            &route,
            "stream",
            false,
            &["OASIS7_REMOTE_LLM_STREAM", "LETAI_STREAM"],
        ),
        response_format_json_object: route_or_env_bool(
            &route,
            "response_format_json_object",
            false,
            &[
                "OASIS7_REMOTE_LLM_RESPONSE_FORMAT_JSON_OBJECT",
                "LETAI_RESPONSE_FORMAT_JSON_OBJECT",
            ],
        ),
        retry_count: route_or_env_u64(
            &route,
            "retry_count",
            DEFAULT_LETAI_RETRY_COUNT,
            &["OASIS7_REMOTE_LLM_RETRY_COUNT", "LETAI_RETRY_COUNT"],
        )?
        .max(1),
        retry_delay_ms: route_or_env_u64(
            &route,
            "retry_delay_ms",
            DEFAULT_LETAI_RETRY_DELAY_MS,
            &["OASIS7_REMOTE_LLM_RETRY_DELAY_MS", "LETAI_RETRY_DELAY_MS"],
        )?,
        auto_topup_usd: env_optional(&["OASIS7_REMOTE_LLM_AUTO_TOPUP_USD", "LETAI_AUTO_TOPUP_USD"]),
        platform_base_url: normalize_base_url(
            env_optional(&[
                "OASIS7_REMOTE_LLM_PLATFORM_BASE_URL",
                "OASIS7_NEWAPI_BRIDGE_LETAI_BASE_URL",
                "LETAI_PLATFORM_BASE_URL",
            ])
            .unwrap_or_else(|| DEFAULT_LETAI_PLATFORM_BASE_URL.to_string())
            .as_str(),
        ),
        platform_key: env_optional(&[
            "OASIS7_REMOTE_LLM_PLATFORM_KEY",
            "OASIS7_NEWAPI_BRIDGE_LETAI_PLATFORM_KEY",
            "LETAI_PLATFORM_KEY",
        ]),
        platform_user_id: env_optional(&[
            "OASIS7_REMOTE_LLM_PLATFORM_USER_ID",
            "LETAI_PLATFORM_USER_ID",
        ]),
        auto_topup_retry_count: env_u64(
            DEFAULT_LETAI_AUTO_TOPUP_RETRY_COUNT,
            &[
                "OASIS7_REMOTE_LLM_AUTO_TOPUP_RETRY_COUNT",
                "LETAI_AUTO_TOPUP_RETRY_COUNT",
            ],
        )?
        .max(1),
        auto_topup_retry_delay_ms: env_u64(
            DEFAULT_LETAI_AUTO_TOPUP_RETRY_DELAY_MS,
            &[
                "OASIS7_REMOTE_LLM_AUTO_TOPUP_RETRY_DELAY_MS",
                "LETAI_AUTO_TOPUP_RETRY_DELAY_MS",
            ],
        )?,
    })
}

fn send_letai_chat_completion_with_retries(
    config: &LetaiChatConfig,
    invocation: &AgentInvocation,
) -> Result<LetaiChatResult, String> {
    let mut last_error = String::new();
    for attempt in 1..=config.retry_count {
        match send_letai_chat_completion(config, invocation) {
            Ok(result) => return Ok(result),
            Err(err) => {
                last_error = err;
                if maybe_auto_topup_letai_user(config, last_error.as_str())? {
                    return send_letai_chat_completion_after_topup(config, invocation);
                }
                if attempt >= config.retry_count || !is_retryable_letai_error(last_error.as_str()) {
                    break;
                }
                let diagnostics = json!({
                    "event": "rust_direct_letai_chat_retry",
                    "attempt": attempt + 1,
                    "retry_count": config.retry_count,
                    "reason": summarize_text(last_error.as_str(), 300),
                    "diagnostics": error_diagnostics_json(last_error.as_str()),
                });
                eprintln!("{}", diagnostics);
                if config.retry_delay_ms > 0 {
                    std::thread::sleep(Duration::from_millis(config.retry_delay_ms));
                }
            }
        }
    }
    Err(last_error)
}

fn send_letai_chat_completion_after_topup(
    config: &LetaiChatConfig,
    invocation: &AgentInvocation,
) -> Result<LetaiChatResult, String> {
    let mut last_error = String::new();
    for attempt in 1..=config.auto_topup_retry_count {
        if attempt > 1 && config.auto_topup_retry_delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(config.auto_topup_retry_delay_ms));
        }
        match send_letai_chat_completion(config, invocation) {
            Ok(result) => return Ok(result),
            Err(err) => {
                last_error = err;
                if !should_auto_topup_letai_error(last_error.as_str())
                    || attempt >= config.auto_topup_retry_count
                {
                    break;
                }
                eprintln!(
                    "{}",
                    json!({
                        "event": "rust_direct_letai_auto_topup_retry_wait",
                        "attempt": attempt + 1,
                        "retry_count": config.auto_topup_retry_count,
                        "delay_ms": config.auto_topup_retry_delay_ms,
                    })
                );
            }
        }
    }
    Err(format!(
        "upstream chat completion still low quota after auto topup: {}",
        summarize_text(last_error.as_str(), 500)
    ))
}

fn send_letai_chat_completion(
    config: &LetaiChatConfig,
    invocation: &AgentInvocation,
) -> Result<LetaiChatResult, String> {
    let timeout = Duration::from_secs(invocation.timeout_seconds.max(1));
    let client = Client::builder()
        .timeout(timeout)
        .build()
        .map_err(|err| format!("build LetAI HTTP client failed: {err}"))?;
    let mut messages = Vec::new();
    if let Some(system_prompt) = config.system_prompt.as_deref() {
        messages.push(json!({"role": "system", "content": system_prompt}));
    }
    messages.push(json!({"role": "user", "content": invocation.prompt}));
    let mut body = json!({
        "model": config.model,
        "messages": messages,
        "temperature": config.temperature,
        "stream": config.stream,
        "max_tokens": config.max_output_tokens,
        "user": format!("oasis7-provider:{}", invocation.agent_id),
    });
    if config.response_format_json_object {
        body["response_format"] = json!({"type": "json_object"});
    }
    let started = Instant::now();
    let url = format!("{}/chat/completions", config.base_url);
    eprintln!(
        "{}",
        json!({
            "event": "rust_direct_letai_chat_request",
            "target": safe_url_summary(url.as_str()),
            "model": config.model,
            "stream": config.stream,
            "timeout_ms": timeout.as_millis(),
            "max_output_tokens": config.max_output_tokens,
            "temperature": config.temperature,
            "response_format_json_object": config.response_format_json_object,
            "system_prompt_present": config.system_prompt.is_some(),
            "prompt_len": invocation.prompt.len(),
            "agent_id": invocation.agent_id,
        })
    );
    let mut response = client
        .post(url.as_str())
        .bearer_auth(config.api_key.as_str())
        .header("Content-Type", "application/json")
        .header("User-Agent", DEFAULT_LETAI_USER_AGENT)
        .json(&body)
        .send()
        .map_err(|err| format!("upstream chat completion request failed: {err}"))?;
    let status = response.status();
    let headers = response.headers().clone();
    if !status.is_success() {
        let detail = response
            .text()
            .unwrap_or_else(|err| format!("read error body failed: {err}"));
        return Err(format!(
            "upstream chat completion returned HTTP {}: {}",
            status.as_u16(),
            summarize_text(detail.as_str(), 500)
        ));
    }
    let mut result = if config.stream {
        decode_letai_sse_completion(&mut response, Some(status.as_u16()), &headers)?
    } else {
        let payload = response
            .text()
            .map_err(|err| format!("read upstream response body failed: {err}"))?;
        decode_letai_completion_payload(payload.as_str(), Some(status.as_u16()), &headers)?
    };
    result.duration_ms = started.elapsed().as_millis() as u64;
    Ok(result)
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

fn decode_letai_completion_payload(
    payload: &str,
    status_code: Option<u16>,
    headers: &reqwest::header::HeaderMap,
) -> Result<LetaiChatResult, String> {
    let trimmed = payload.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "upstream response body was empty; diagnostics={}",
            json!({
                "status_code": status_code,
                "headers": response_header_summary(headers),
                "body_len": payload.len(),
            })
        ));
    }
    if trimmed.lines().any(|line| line.trim().starts_with("data:")) {
        let cursor = std::io::Cursor::new(trimmed.as_bytes());
        return decode_letai_sse_reader(cursor, status_code, headers);
    }
    let decoded: Value = serde_json::from_str(trimmed)
        .map_err(|err| format!("decode upstream response JSON failed: {err}"))?;
    let choices = decoded.get("choices").and_then(Value::as_array);
    let Some(first_choice) = choices.and_then(|choices| choices.first()) else {
        return Err(format!(
            "upstream response missing choices[0]; diagnostics={}",
            json!({
                "status_code": status_code,
                "headers": response_header_summary(headers),
                "top_level_keys": value_keys(&decoded),
                "choices_len": choices.map(Vec::len),
            })
        ));
    };
    let content = content_from_choice(first_choice);
    if content.trim().is_empty() {
        return Err(format!(
            "upstream response missing choices[0].message.content; diagnostics={}",
            json!({
                "status_code": status_code,
                "headers": response_header_summary(headers),
                "top_level_keys": value_keys(&decoded),
                "choices_len": choices.map(Vec::len),
                "choice_sample": summarize_choice(first_choice),
            })
        ));
    }
    let usage = decoded.get("usage").unwrap_or(&Value::Null);
    Ok(LetaiChatResult {
        content: content.trim().to_string(),
        model: decoded
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        duration_ms: 0,
        prompt_tokens: usage.get("prompt_tokens").and_then(Value::as_u64),
        completion_tokens: usage.get("completion_tokens").and_then(Value::as_u64),
        total_tokens: usage.get("total_tokens").and_then(Value::as_u64),
    })
}

fn decode_letai_sse_completion(
    response: &mut reqwest::blocking::Response,
    status_code: Option<u16>,
    headers: &reqwest::header::HeaderMap,
) -> Result<LetaiChatResult, String> {
    decode_letai_sse_reader(response, status_code, headers)
}

fn decode_letai_sse_reader<R: Read>(
    reader: R,
    status_code: Option<u16>,
    headers: &reqwest::header::HeaderMap,
) -> Result<LetaiChatResult, String> {
    let mut text_parts = Vec::new();
    let mut usage = Value::Null;
    let mut last_chunk = Value::Null;
    let mut model = "unknown".to_string();
    let mut line_count = 0_u64;
    let mut data_event_count = 0_u64;
    let mut done_count = 0_u64;
    let mut parse_errors = Vec::new();
    let mut chunk_samples = Vec::new();
    for line_result in BufReader::new(reader).lines() {
        line_count += 1;
        let line = line_result.map_err(|err| format!("read upstream SSE failed: {err}"))?;
        let line = line.trim();
        if line.is_empty() || !line.starts_with("data:") {
            continue;
        }
        let data = line[5..].trim();
        if data.is_empty() {
            continue;
        }
        data_event_count += 1;
        if data == "[DONE]" {
            done_count += 1;
            continue;
        }
        let chunk: Value = match serde_json::from_str(data) {
            Ok(chunk) => chunk,
            Err(err) => {
                if parse_errors.len() < 5 {
                    parse_errors.push(json!({
                        "error": err.to_string(),
                        "data_len": data.len(),
                        "data_sha256_16": short_sha256(data),
                    }));
                }
                continue;
            }
        };
        if let Some(chunk_model) = chunk.get("model").and_then(Value::as_str) {
            model = chunk_model.to_string();
        }
        if chunk_samples.len() < 5 {
            chunk_samples.push(summarize_sse_chunk(&chunk));
        }
        if let Some(chunk_usage) = chunk.get("usage").filter(|value| value.is_object()) {
            usage = chunk_usage.clone();
        }
        if let Some(choices) = chunk.get("choices").and_then(Value::as_array) {
            for choice in choices {
                if let Some(content) = choice
                    .get("delta")
                    .and_then(|delta| delta.get("content"))
                    .and_then(Value::as_str)
                {
                    text_parts.push(content.to_string());
                }
                if text_parts.is_empty() {
                    let fallback = content_from_choice(choice);
                    if !fallback.is_empty() {
                        text_parts.push(fallback);
                    }
                }
            }
        }
        last_chunk = chunk;
    }
    let content = text_parts.join("").trim().to_string();
    if content.is_empty() {
        return Err(format!(
            "upstream SSE response did not contain assistant content; diagnostics={}",
            json!({
                "status_code": status_code,
                "headers": response_header_summary(headers),
                "line_count": line_count,
                "data_event_count": data_event_count,
                "done_count": done_count,
                "parse_error_count": parse_errors.len(),
                "parse_error_samples": parse_errors,
                "chunk_samples": chunk_samples,
                "usage_present": usage.is_object(),
                "last_chunk_keys": value_keys(&last_chunk),
            })
        ));
    }
    Ok(LetaiChatResult {
        content,
        model,
        duration_ms: 0,
        prompt_tokens: usage.get("prompt_tokens").and_then(Value::as_u64),
        completion_tokens: usage.get("completion_tokens").and_then(Value::as_u64),
        total_tokens: usage.get("total_tokens").and_then(Value::as_u64),
    })
}

fn content_from_choice(choice: &Value) -> String {
    let Some(message) = choice.get("message") else {
        return String::new();
    };
    let Some(content) = message.get("content") else {
        return String::new();
    };
    if let Some(text) = content.as_str() {
        return text.trim().to_string();
    }
    if let Some(items) = content.as_array() {
        return items
            .iter()
            .filter_map(|item| {
                item.get("text")
                    .and_then(Value::as_str)
                    .or_else(|| item.get("content").and_then(Value::as_str))
                    .or_else(|| item.get("value").and_then(Value::as_str))
            })
            .collect::<String>()
            .trim()
            .to_string();
    }
    String::new()
}

fn summarize_sse_chunk(chunk: &Value) -> Value {
    let mut sample = json!({
        "top_level_keys": value_keys(chunk),
        "model": chunk.get("model").and_then(Value::as_str),
        "object": chunk.get("object").and_then(Value::as_str),
        "usage_present": chunk.get("usage").is_some_and(Value::is_object),
    });
    if let Some(choices) = chunk.get("choices").and_then(Value::as_array) {
        sample["choices_len"] = json!(choices.len());
        if let Some(choice) = choices.first() {
            sample["choice0"] = summarize_choice(choice);
        }
    } else if let Some(choices) = chunk.get("choices") {
        sample["choices_type"] = json!(value_type_name(choices));
    }
    if let Some(error) = chunk.get("error").filter(|value| value.is_object()) {
        sample["error"] = json!({
            "keys": value_keys(error),
            "code": error.get("code").and_then(Value::as_str),
            "type": error.get("type").and_then(Value::as_str),
            "message": error.get("message").and_then(Value::as_str).map(|value| summarize_text(value, 80)),
        });
    }
    sample
}

fn summarize_choice(choice: &Value) -> Value {
    let mut summary = json!({
        "keys": value_keys(choice),
        "finish_reason": choice.get("finish_reason"),
    });
    if let Some(delta) = choice.get("delta").filter(|value| value.is_object()) {
        summary["delta"] = json!({
            "keys": value_keys(delta),
            "content_present": delta.get("content").and_then(Value::as_str).is_some(),
            "content_len": delta.get("content").and_then(Value::as_str).map(str::len),
            "role": delta.get("role").and_then(Value::as_str),
        });
    }
    if let Some(message) = choice.get("message").filter(|value| value.is_object()) {
        summary["message"] = json!({
            "keys": value_keys(message),
            "content_present": message.get("content").and_then(Value::as_str).is_some(),
            "content_len": message.get("content").and_then(Value::as_str).map(str::len),
        });
    }
    summary
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

fn normalize_base_url(raw: &str) -> String {
    let mut base = raw.trim().trim_end_matches('/').to_string();
    for suffix in ["/chat/completions", "/responses"] {
        if base.ends_with(suffix) {
            let keep = base.len() - suffix.len();
            base.truncate(keep);
        }
    }
    base
}

fn load_letai_routes_path_config(route_label: Option<&str>) -> Result<Value, String> {
    let Some(routes_path) = env_optional(&["OASIS7_REMOTE_LLM_ROUTES_PATH"]) else {
        if route_label.is_some_and(|label| !label.trim().is_empty())
            && env_optional(&["OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH"]).is_none()
            && env_optional(&["OASIS7_REMOTE_LLM_API_KEY", "LETAI_API_KEY"]).is_none()
        {
            return Err(
                "OASIS7_REMOTE_LLM_ROUTE_LABEL requires either OASIS7_REMOTE_LLM_ROUTES_PATH or OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH"
                    .to_string(),
            );
        }
        return Ok(Value::Null);
    };
    let raw = fs::read_to_string(routes_path.as_str())
        .map_err(|err| format!("failed to read OASIS7_REMOTE_LLM_ROUTES_PATH: {err}"))?;
    let payload = serde_json::from_str::<Value>(raw.as_str())
        .map_err(|err| format!("OASIS7_REMOTE_LLM_ROUTES_PATH must contain valid JSON: {err}"))?;
    let Some(routes) = payload.as_object() else {
        return Err("OASIS7_REMOTE_LLM_ROUTES_PATH root must be a JSON object".to_string());
    };
    let lookup_label = route_label
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("default");
    let Some(route) = routes.get(lookup_label) else {
        return Err(format!(
            "route config `{lookup_label}` was not found in OASIS7_REMOTE_LLM_ROUTES_PATH"
        ));
    };
    if !route.is_object() {
        return Err(format!(
            "route config `{lookup_label}` must be a JSON object"
        ));
    }
    Ok(route.clone())
}

fn route_string(route: &Value, key: &str) -> Option<String> {
    route
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn route_or_env_optional(route: &Value, route_key: &str, env_names: &[&str]) -> Option<String> {
    route_string(route, route_key).or_else(|| env_optional(env_names))
}

fn route_or_env_u64(
    route: &Value,
    route_key: &str,
    default: u64,
    env_names: &[&str],
) -> Result<u64, String> {
    if let Some(value) = route.get(route_key).filter(|value| !value.is_null()) {
        if let Some(number) = value.as_u64() {
            return Ok(number);
        }
        if let Some(raw) = value.as_str() {
            return raw.parse::<u64>().map_err(|err| {
                format!("invalid integer for route field {route_key}: {raw}: {err}")
            });
        }
        return Err(format!(
            "invalid integer for route field {route_key}: {}",
            value_type_name(value)
        ));
    }
    env_u64(default, env_names)
}

fn route_or_env_f64(
    route: &Value,
    route_key: &str,
    default: f64,
    env_names: &[&str],
) -> Result<f64, String> {
    if let Some(value) = route.get(route_key).filter(|value| !value.is_null()) {
        if let Some(number) = value.as_f64() {
            return Ok(number);
        }
        if let Some(raw) = value.as_str() {
            return raw
                .parse::<f64>()
                .map_err(|err| format!("invalid float for route field {route_key}: {raw}: {err}"));
        }
        return Err(format!(
            "invalid float for route field {route_key}: {}",
            value_type_name(value)
        ));
    }
    env_f64(default, env_names)
}

fn route_or_env_bool(route: &Value, route_key: &str, default: bool, env_names: &[&str]) -> bool {
    if let Some(value) = route.get(route_key).filter(|value| !value.is_null()) {
        if let Some(flag) = value.as_bool() {
            return flag;
        }
        if let Some(raw) = value.as_str() {
            return matches!(
                raw.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            );
        }
        return false;
    }
    env_bool(default, env_names)
}

fn env_optional(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        env::var(name)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn env_u64(default: u64, names: &[&str]) -> Result<u64, String> {
    let Some(raw) = env_optional(names) else {
        return Ok(default);
    };
    raw.parse::<u64>()
        .map_err(|err| format!("invalid integer for {}: {raw}: {err}", names.join(" or ")))
}

fn env_f64(default: f64, names: &[&str]) -> Result<f64, String> {
    let Some(raw) = env_optional(names) else {
        return Ok(default);
    };
    raw.parse::<f64>()
        .map_err(|err| format!("invalid float for {}: {raw}: {err}", names.join(" or ")))
}

fn env_bool(default: bool, names: &[&str]) -> bool {
    let Some(raw) = env_optional(names) else {
        return default;
    };
    matches!(
        raw.to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn load_newapi_bridge_state_route_token(route_label: &str) -> Option<String> {
    let state_path = env_optional(&["OASIS7_REMOTE_LLM_NEWAPI_BRIDGE_STATE_PATH"])?;
    let raw = fs::read_to_string(state_path).ok()?;
    let payload = serde_json::from_str::<Value>(raw.as_str()).ok()?;
    let bindings = payload.get("bindings").and_then(Value::as_array)?;
    let project_bindings = payload.get("project_bindings").and_then(Value::as_array)?;
    let (by_ref, by_bridge_user_id) = parse_newapi_bridge_bearer_selector(route_label)?;
    let binding = bindings.iter().find(|entry| {
        entry
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|status| status == "active")
            && ((by_ref.is_some()
                && entry.get("newapi_user_ref").and_then(Value::as_str) == by_ref)
                || (by_bridge_user_id.is_some()
                    && entry.get("bridge_user_id").and_then(Value::as_str) == by_bridge_user_id))
    })?;
    let bridge_user_id = binding.get("bridge_user_id").and_then(Value::as_str)?;
    project_bindings
        .iter()
        .filter(|entry| entry.get("bridge_user_id").and_then(Value::as_str) == Some(bridge_user_id))
        .filter_map(|entry| entry.get("token_key").and_then(Value::as_str))
        .filter(|token| !token.trim().is_empty())
        .last()
        .map(str::to_string)
}

fn response_header_summary(headers: &reqwest::header::HeaderMap) -> Value {
    let mut map = serde_json::Map::new();
    for key in [
        "content-type",
        "x-request-id",
        "x-trace-id",
        "cf-ray",
        "server",
    ] {
        if let Some(value) = headers.get(key).and_then(|value| value.to_str().ok()) {
            map.insert(key.to_string(), json!(summarize_text(value, 120)));
        }
    }
    Value::Object(map)
}

fn safe_url_summary(url: &str) -> Value {
    match reqwest::Url::parse(url) {
        Ok(parsed) => json!({
            "scheme": parsed.scheme(),
            "host": parsed.host_str().unwrap_or_default(),
            "port": parsed.port(),
            "path": parsed.path(),
        }),
        Err(_) => json!({
            "scheme": "",
            "host": "",
            "port": null,
            "path": "",
        }),
    }
}

fn value_keys(value: &Value) -> Vec<String> {
    value
        .as_object()
        .map(|object| object.keys().cloned().collect())
        .unwrap_or_default()
}

fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn is_retryable_letai_error(error: &str) -> bool {
    let lowered = error.to_ascii_lowercase();
    lowered.contains("did not contain assistant content")
        || lowered.contains("read operation timed out")
        || lowered.contains("operation timed out")
        || lowered.contains("remote end closed connection without response")
        || lowered.contains("connection reset")
        || lowered.contains("timed out")
}

fn error_diagnostics_json(error: &str) -> Value {
    let Some((_, diagnostics)) = error.split_once("diagnostics=") else {
        return Value::Null;
    };
    serde_json::from_str::<Value>(diagnostics.trim()).unwrap_or(Value::Null)
}

fn should_auto_topup_letai_error(error: &str) -> bool {
    error
        .to_ascii_lowercase()
        .contains("insufficient_user_quota")
        || error.contains("余额")
}

fn maybe_auto_topup_letai_user(config: &LetaiChatConfig, error: &str) -> Result<bool, String> {
    if !should_auto_topup_letai_error(error) {
        return Ok(false);
    }
    let Some(topup_usd) = config.auto_topup_usd.as_deref() else {
        return Ok(false);
    };
    let Some(quota) = quota_from_usd(topup_usd)? else {
        return Ok(false);
    };
    let Some(platform_key) = config.platform_key.as_deref() else {
        return Ok(false);
    };
    let Some(platform_user_id) = config.platform_user_id.as_deref() else {
        return Ok(false);
    };
    let adapter = LetaiOpenApiAdapter::new(
        config.platform_base_url.as_str(),
        platform_key,
        None,
        30_000,
    )
    .map_err(|err| format!("build LetAI auto topup adapter failed: {err}"))?;
    let external_order_id = format!(
        "oasis7-local-auto-topup-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| format!("system clock before unix epoch: {err}"))?
            .as_secs()
    );
    adapter
        .topup_user(
            platform_user_id,
            &LetaiUserTopupRequest {
                external_order_id: external_order_id.clone(),
                quota,
                amount: Some(topup_usd.trim().to_string()),
                currency: Some("USD".to_string()),
            },
        )
        .map_err(|err| format!("auto topup failed: {}: {}", err.code, err.message))?;
    eprintln!(
        "{}",
        json!({
            "event": "rust_direct_letai_auto_topup",
            "quota": quota,
            "amount_usd": topup_usd.trim(),
            "external_order_id": external_order_id,
        })
    );
    Ok(true)
}

fn quota_from_usd(raw: &str) -> Result<Option<u64>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let amount = trimmed
        .parse::<f64>()
        .map_err(|err| format!("invalid auto topup USD amount: {trimmed}: {err}"))?;
    if amount <= 0.0 {
        return Ok(None);
    }
    let quota = (amount * LETAI_QUOTA_UNITS_PER_USD).floor();
    if !quota.is_finite() || quota <= 0.0 || quota > u64::MAX as f64 {
        return Err(format!("invalid auto topup USD amount: {trimmed}"));
    }
    Ok(Some(quota as u64))
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
