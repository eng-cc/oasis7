use std::collections::{BTreeMap, VecDeque};
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Command;

use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use oasis7::simulator::{
    Action, DecisionRequest, DecisionResponse, FeedbackEnvelope, ProviderHealth,
    ProviderInfo, ProviderDecision, ProviderDiagnostics, ProviderErrorEnvelope,
    ProviderTokenUsage, ProviderTraceEnvelope, ProviderTranscriptEntry,
};
use reqwest::blocking::Client;
use serde::Deserialize;
use serde_json::{json, Value};

const DEFAULT_BIND_ADDR: &str = "127.0.0.1:5841";
const DEFAULT_OPENCLAW_AGENT_ID: &str = "main";
const DEFAULT_OPENCLAW_THINKING: &str = "off";
const DEFAULT_PROVIDER_ID: &str = "openclaw_local_bridge";
const DEFAULT_PROTOCOL_VERSION: &str = "world-simulator-openclaw-local-http-v1";
const MAX_RECENT_FEEDBACK: usize = 8;
const DEFAULT_PROVIDER_AGENT_PROFILE: &str = "oasis7_p0_low_freq_npc";

#[path = "oasis7_provider_local_bridge/support.rs"]
mod support;
#[cfg(test)]
#[path = "oasis7_provider_local_bridge/tests.rs"]
mod tests;

use self::support::{
    agent_output_from_json, estimated_current_location_id, local_session_id_from_session_key,
    nearest_reachable_non_current_location_id, should_fallback_to_local_agent,
};

#[derive(Debug, Clone)]
struct CliOptions {
    bind_addr: String,
    openclaw_bin: String,
    openclaw_agent_id: String,
    openclaw_thinking: String,
    gateway_health_url: String,
    auth_token: Option<String>,
}

impl Default for CliOptions {
    fn default() -> Self {
        Self {
            bind_addr: DEFAULT_BIND_ADDR.to_string(),
            openclaw_bin: "openclaw".to_string(),
            openclaw_agent_id: DEFAULT_OPENCLAW_AGENT_ID.to_string(),
            openclaw_thinking: DEFAULT_OPENCLAW_THINKING.to_string(),
            gateway_health_url: default_gateway_health_url(),
            auth_token: None,
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
        })
    }

    fn provider_info(&self) -> ProviderInfo {
        ProviderInfo {
            provider_id: DEFAULT_PROVIDER_ID.to_string(),
            name: Some("OpenClaw Local Bridge".to_string()),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
            protocol_version: Some(DEFAULT_PROTOCOL_VERSION.to_string()),
            capabilities: vec![
                "decision".to_string(),
                "feedback".to_string(),
                "loopback_only".to_string(),
                format!("agent:{}", self.options.openclaw_agent_id),
            ],
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

    fn provider_health(&self) -> ProviderHealth {
        let active_requests = self.active_requests.load(Ordering::Relaxed);
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
                let detail = format!("openclaw_gateway_unreachable: {err}");
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

    fn handle_decision(
        &self,
        request: DecisionRequest,
        invoker: &dyn AgentInvoker,
    ) -> DecisionResponse {
        if let Err(err) = request.validate_contract() {
            self.set_last_error(Some(err.to_string()));
            return provider_error_response(err.code, err.message, false, None, None);
        }
        if let Some(err) = validate_profile(request.agent_profile.as_deref()) {
            self.set_last_error(Some(err.clone()));
            return provider_error_response("unsupported_agent_profile", err, false, None, None);
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
        let session_key = build_session_key(&request, self.options.openclaw_agent_id.as_str());
        let timeout_seconds = timeout_seconds_from_budget(request.timeout_budget_ms);
        let invoke_result = invoker.invoke(AgentInvocation {
            openclaw_bin: self.options.openclaw_bin.clone(),
            agent_id: self.options.openclaw_agent_id.clone(),
            thinking: self.options.openclaw_thinking.clone(),
            session_key: session_key.clone(),
            timeout_seconds,
            prompt,
            idempotency_key: format!("{session_key}-{timeout_seconds}"),
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
                            provider_id: Some(DEFAULT_PROVIDER_ID.to_string()),
                            provider_version: output.provider_version.clone(),
                            latency_ms: Some(latency_ms.max(output.duration_ms.unwrap_or(0))),
                            retry_count: 0,
                        },
                        trace_payload: ProviderTraceEnvelope {
                            provider_id: Some(DEFAULT_PROVIDER_ID.to_string()),
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
                            provider_id: Some(DEFAULT_PROVIDER_ID.to_string()),
                            provider_version: output.provider_version.clone(),
                            latency_ms: Some(latency_ms.max(output.duration_ms.unwrap_or(0))),
                            retry_count: 0,
                        },
                        trace_payload: ProviderTraceEnvelope {
                            provider_id: Some(DEFAULT_PROVIDER_ID.to_string()),
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
                let detail = format!("openclaw_gateway_unreachable: {err}");
                self.set_last_error(Some(detail.clone()));
                provider_error_response(
                    "openclaw_gateway_unreachable",
                    detail,
                    true,
                    Some(latency_ms),
                    Some("decision invocation failed".to_string()),
                )
            }
        }
    }

    fn set_last_error(&self, detail: Option<String>) {
        *self.last_error.lock().expect("last_error lock") = detail;
    }
}

#[derive(Debug, Clone)]
struct AgentInvocation {
    openclaw_bin: String,
    agent_id: String,
    thinking: String,
    session_key: String,
    timeout_seconds: u64,
    prompt: String,
    idempotency_key: String,
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
struct OpenClawCliInvoker;

impl AgentInvoker for OpenClawCliInvoker {
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

fn invoke_gateway_agent(invocation: AgentInvocation) -> Result<AgentInvocationOutput, String> {
    let params = build_gateway_agent_params(&invocation)
        .map_err(|err| format!("serialize gateway call params failed: {err}"))?;
    let rpc_timeout_ms = invocation
        .timeout_seconds
        .saturating_mul(1000)
        .saturating_add(2000);
    let output = Command::new(invocation.openclaw_bin.as_str())
        .arg("gateway")
        .arg("call")
        .arg("agent")
        .arg("--expect-final")
        .arg("--json")
        .arg("--timeout")
        .arg(rpc_timeout_ms.to_string())
        .arg("--params")
        .arg(params)
        .output()
        .map_err(|err| format!("spawn openclaw gateway call agent failed: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(output.stderr.as_slice())
            .trim()
            .to_string();
        let stdout = String::from_utf8_lossy(output.stdout.as_slice())
            .trim()
            .to_string();
        return Err(format!(
            "openclaw gateway call agent exited with status {}: stderr={} stdout={}",
            output.status, stderr, stdout,
        ));
    }
    let payload = String::from_utf8(output.stdout)
        .map_err(|err| format!("openclaw gateway call agent stdout was not utf8: {err}"))?;
    agent_output_from_json(invocation.prompt, payload.as_str(), None)
}

fn invoke_local_agent(
    invocation: AgentInvocation,
    gateway_error: &str,
) -> Result<AgentInvocationOutput, String> {
    let session_id = local_session_id_from_session_key(invocation.session_key.as_str());
    let output = Command::new(invocation.openclaw_bin.as_str())
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
        .arg(invocation.timeout_seconds.to_string())
        .arg("--json")
        .output()
        .map_err(|err| format!("spawn openclaw local agent failed: {err}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(output.stderr.as_slice())
            .trim()
            .to_string();
        let stdout = String::from_utf8_lossy(output.stdout.as_slice())
            .trim()
            .to_string();
        return Err(format!(
            "openclaw gateway fallback failed after `{}`; local agent exited with status {}: stderr={} stdout={}",
            gateway_error, output.status, stderr, stdout,
        ));
    }
    let payload = String::from_utf8(output.stdout)
        .map_err(|err| format!("openclaw local agent stdout was not utf8: {err}"))?;
    agent_output_from_json(
        invocation.prompt,
        payload.as_str(),
        Some(format!(
            "invocation_fallback=local_embedded; reason={}",
            summarize_text(gateway_error, 240)
        )),
    )
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let options = match parse_options(args.iter().skip(1).map(String::as_str)) {
        Ok(options) => options,
        Err(err) => {
            eprintln!("{err}");
            print_help();
            std::process::exit(1);
        }
    };
    let state = match ProviderState::new(options.clone()) {
        Ok(state) => state,
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(1);
        }
    };
    let listener = TcpListener::bind(options.bind_addr.as_str())
        .unwrap_or_else(|err| panic!("bind {} failed: {err}", options.bind_addr));
    println!(
        "oasis7_provider_local_bridge listening on http://{} (agent={}, gateway_health={})",
        options.bind_addr, options.openclaw_agent_id, options.gateway_health_url
    );
    let invoker: Arc<dyn AgentInvoker> = Arc::new(OpenClawCliInvoker);
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else { continue };
        let state = state.clone();
        let invoker = invoker.clone();
        std::thread::spawn(move || {
            if let Err(err) = handle_connection(&mut stream, &state, invoker.as_ref()) {
                eprintln!("oasis7_provider_local_bridge connection error: {err}");
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
            "--openclaw-bin" => {
                options.openclaw_bin = required_value(&mut iter, "--openclaw-bin")?.to_string();
            }
            "--openclaw-agent" => {
                options.openclaw_agent_id =
                    required_value(&mut iter, "--openclaw-agent")?.to_string();
            }
            "--openclaw-thinking" => {
                options.openclaw_thinking =
                    required_value(&mut iter, "--openclaw-thinking")?.to_string();
            }
            "--gateway-health-url" => {
                options.gateway_health_url =
                    required_value(&mut iter, "--gateway-health-url")?.to_string();
            }
            "--auth-token" => {
                options.auth_token = Some(required_value(&mut iter, "--auth-token")?.to_string());
            }
            "-h" | "--help" => return Err("help requested".to_string()),
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    if options.bind_addr.trim().is_empty() {
        return Err("--bind requires a non-empty value".to_string());
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
    eprintln!(
        "Usage: oasis7_provider_local_bridge [options]\n\n  --bind <host:port>            Loopback bind address (default: 127.0.0.1:5841)\n  --openclaw-bin <path>         OpenClaw CLI path (default: openclaw)\n  --openclaw-agent <id>         OpenClaw agent id (default: main)\n  --openclaw-thinking <level>   OpenClaw thinking level (default: off)\n  --gateway-health-url <url>    OpenClaw Gateway health URL\n  --auth-token <token>          Optional bearer token for bridge endpoints\n"
    );
}

fn handle_connection(
    stream: &mut TcpStream,
    state: &ProviderState,
    invoker: &dyn AgentInvoker,
) -> Result<(), String> {
    let request = read_http_request(stream)?;
    if !authorize_request(state, &request) {
        return write_json_response(stream, 401, &json!({"error":"Unauthorized"}));
    }
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/v1/provider/info") => write_json_response(stream, 200, &state.provider_info()),
        ("GET", "/v1/provider/health") | ("GET", "/health") => {
            write_json_response(stream, 200, &state.provider_health())
        }
        ("POST", "/v1/world-simulator/decision") => {
            let decoded: DecisionRequest = serde_json::from_slice(request.body.as_slice())
                .map_err(|err| format!("decode decision request failed: {err}"))?;
            let response = state.handle_decision(decoded, invoker);
            write_json_response(stream, 200, &response)
        }
        ("POST", "/v1/world-simulator/feedback") => {
            let decoded: FeedbackEnvelope = serde_json::from_slice(request.body.as_slice())
                .map_err(|err| format!("decode feedback request failed: {err}"))?;
            state.record_feedback(decoded);
            write_json_response(stream, 200, &json!({"ok": true}))
        }
        _ => write_json_response(stream, 404, &json!({"error":"Not Found"})),
    }
}

fn authorize_request(state: &ProviderState, request: &RecordedHttpRequest) -> bool {
    let Some(expected) = state.options.auth_token.as_deref() else {
        return true;
    };
    request
        .headers
        .get("authorization")
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(|value| value == expected)
        .unwrap_or(false)
}

#[derive(Debug)]
struct RecordedHttpRequest {
    method: String,
    path: String,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

fn read_http_request(stream: &mut TcpStream) -> Result<RecordedHttpRequest, String> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 2048];
    let mut header_end = None;
    let mut content_length = 0_usize;

    loop {
        let bytes = stream
            .read(&mut chunk)
            .map_err(|err| format!("read request failed: {err}"))?;
        if bytes == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..bytes]);
        if header_end.is_none() {
            header_end = find_header_terminator(buffer.as_slice());
            if let Some(boundary) = header_end {
                let header = std::str::from_utf8(&buffer[..boundary])
                    .map_err(|err| format!("request header was not utf8: {err}"))?;
                content_length = header
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        if name.eq_ignore_ascii_case("content-length") {
                            value.trim().parse::<usize>().ok()
                        } else {
                            None
                        }
                    })
                    .unwrap_or(0);
            }
        }
        if let Some(boundary) = header_end {
            if buffer.len() >= boundary + 4 + content_length {
                break;
            }
        }
    }

    let boundary = header_end.ok_or_else(|| "request missing header boundary".to_string())?;
    let header = std::str::from_utf8(&buffer[..boundary])
        .map_err(|err| format!("request header was not utf8: {err}"))?;
    let mut lines = header.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| "request missing request line".to_string())?;
    let mut request_line_parts = request_line.split_whitespace();
    let method = request_line_parts
        .next()
        .ok_or_else(|| "request line missing method".to_string())?
        .to_string();
    let path = request_line_parts
        .next()
        .ok_or_else(|| "request line missing path".to_string())?
        .to_string();
    let mut headers = BTreeMap::new();
    for line in lines {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    let body = buffer[(boundary + 4)..(boundary + 4 + content_length)].to_vec();
    Ok(RecordedHttpRequest {
        method,
        path,
        headers,
        body,
    })
}

fn find_header_terminator(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

fn write_json_response(
    stream: &mut TcpStream,
    status_code: u16,
    body: &impl serde::Serialize,
) -> Result<(), String> {
    let payload =
        serde_json::to_string(body).map_err(|err| format!("serialize response failed: {err}"))?;
    let status_text = match status_code {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        _ => "Error",
    };
    let response = format!(
        "HTTP/1.1 {status_code} {status_text}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        payload.len(),
        payload
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|err| format!("write response failed: {err}"))
}

fn provider_error_response(
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
            provider_id: Some(DEFAULT_PROVIDER_ID.to_string()),
            provider_version: Some(env!("CARGO_PKG_VERSION").to_string()),
            latency_ms,
            retry_count: 0,
        },
        trace_payload: ProviderTraceEnvelope {
            provider_id: Some(DEFAULT_PROVIDER_ID.to_string()),
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

fn build_decision_prompt(request: &DecisionRequest, recent_feedback: &[String]) -> String {
    let observation = &request.observation;
    let nearby_agents = observation
        .observation
        .nearby_entities
        .iter()
        .filter(|entity| entity.kind == "agent")
        .map(|entity| {
            json!({
                "entity_ref": entity.entity_ref,
                "relation": entity.relation,
                "relative_hint": entity.relative_hint,
                "interaction_hint": entity.interaction_hint,
            })
        })
        .collect::<Vec<_>>();
    let nearby_locations = observation
        .observation
        .nearby_entities
        .iter()
        .filter(|entity| entity.kind == "location")
        .map(|entity| {
            json!({
                "entity_ref": entity.entity_ref,
                "relation": entity.relation,
                "relative_hint": entity.relative_hint,
                "interaction_hint": entity.interaction_hint,
            })
        })
        .collect::<Vec<_>>();
    let resources = observation.observation.self_state.resource_summary.clone();
    let action_catalog = observation
        .action_catalog
        .iter()
        .map(|entry| json!({"action_ref": entry.action_ref, "summary": entry.summary}))
        .collect::<Vec<_>>();
    let current_location_id =
        estimated_current_location_id(&observation.observation).map(str::to_string);
    let nearest_non_current_location_id =
        nearest_reachable_non_current_location_id(&observation.observation);
    let can_legally_move_to_visible_non_current_location = observation
        .action_catalog
        .iter()
        .any(|entry| entry.action_ref == "move_agent")
        && nearest_non_current_location_id.is_some();
    let profile_guidance = profile_guidance(
        request,
        current_location_id.as_deref(),
        nearest_non_current_location_id.as_deref(),
    );
    let context = json!({
        "agent_profile": request.agent_profile,
        "mode": observation.mode.as_str(),
        "agent_id": observation.agent_id,
        "world_time": observation.world_time,
        "timeout_budget_ms": request.timeout_budget_ms,
        "self_state": observation.observation.self_state,
        "mission_context": observation.observation.mission_context,
        "resources": resources,
        "nearby_agents": nearby_agents,
        "nearby_locations": nearby_locations,
        "recent_events": observation.observation.recent_events,
        "local_navigation_graph": observation.observation.local_navigation_graph,
        "interaction_targets": observation.observation.interaction_targets,
        "current_location_id": current_location_id,
        "nearest_non_current_location_id": nearest_non_current_location_id,
        "can_legally_move_to_visible_non_current_location": can_legally_move_to_visible_non_current_location,
        "recent_event_summary": observation.recent_event_summary,
        "memory_summary": observation.memory_summary,
        "recent_feedback": recent_feedback,
        "action_catalog": action_catalog,
    });
    format!(
        concat!(
            "You are controlling a low-frequency NPC in oasis7. ",
            "Return ONLY one minified JSON object with no markdown, no prose, and no code fences. ",
            "Respect the profile and only use actions from action_catalog. ",
            "For profile oasis7_p0_low_freq_npc, prefer legal forward progress over idle waiting. ",
            "If move_agent is legal and a visible non-current location exists, prefer move_agent to the nearest such location. ",
            "Never choose move_agent.to equal to the current location. ",
            "Choose wait or wait_ticks only when no legal progress action exists or a recoverable error just happened. ",
            "Output schema:\n",
            "{{\"decision\":\"wait\"}}\n",
            "{{\"decision\":\"wait_ticks\",\"ticks\":<u64>}}\n",
            "{{\"decision\":\"act\",\"action_ref\":\"move_agent\",\"args\":{{\"to\":\"<location_id>\"}}}}\n",
            "{{\"decision\":\"act\",\"action_ref\":\"speak_to_nearby\",\"args\":{{\"message\":\"<short text>\",\"target_agent_id\":\"<optional agent id>\"}}}}\n",
            "{{\"decision\":\"act\",\"action_ref\":\"inspect_target\",\"args\":{{\"target_kind\":\"agent|location|object\",\"target_id\":\"<id>\"}}}}\n",
            "{{\"decision\":\"act\",\"action_ref\":\"simple_interact\",\"args\":{{\"target_kind\":\"agent|location|object\",\"target_id\":\"<id>\",\"interaction\":\"<verb>\"}}}}\n",
            "Profile guidance:\n{}\n",
            "Do not invent ids. Keep messages short. Context JSON follows:\n{}"
        ),
        profile_guidance,
        context
    )
}

fn profile_guidance(
    request: &DecisionRequest,
    current_location_id: Option<&str>,
    nearest_non_current_location_id: Option<&str>,
) -> String {
    match request.agent_profile.as_deref() {
        Some(DEFAULT_PROVIDER_AGENT_PROFILE) => {
            let current_location = current_location_id.unwrap_or("unknown");
            let next_location = nearest_non_current_location_id.unwrap_or("none");
            format!(
                concat!(
                    "- Goal priority: keep making low-risk forward progress and avoid repeated idle wait.
",
                    "- If a visible non-current location exists, moving there counts as progress for patrol parity.
",
                    "- Current visible location: {current_location}.
",
                    "- Preferred next visible non-current location: {next_location}.
",
                    "- Do not output wait if move_agent to that preferred location is legal and no recoverable failure blocks it.
",
                    "- Use inspect_target before wait when the target is ambiguous.
"
                ),
                current_location = current_location,
                next_location = next_location,
            )
        }
        Some(profile) => format!("- Active profile: {profile}"),
        None => "- No explicit profile provided; stay conservative and legal.".to_string(),
    }
}

fn timeout_seconds_from_budget(timeout_budget_ms: u64) -> u64 {
    ((timeout_budget_ms.max(1000) + 999) / 1000).max(1)
}

fn build_gateway_agent_params(invocation: &AgentInvocation) -> Result<String, serde_json::Error> {
    serde_json::to_string(&json!({
        "message": invocation.prompt,
        "agentId": invocation.agent_id,
        "sessionKey": invocation.session_key,
        "deliver": false,
        "channel": "webchat",
        "lane": "nested",
        "thinking": invocation.thinking,
        "timeout": invocation.timeout_seconds,
        "idempotencyKey": invocation.idempotency_key,
    }))
}

fn build_session_key(request: &DecisionRequest, openclaw_agent_id: &str) -> String {
    let raw = format!(
        "agent:{openclaw_agent_id}:subagent:world-simulator:{}:{}:{}",
        request
            .provider_config_ref
            .as_deref()
            .unwrap_or("default-config"),
        request
            .agent_profile
            .as_deref()
            .unwrap_or("default-profile"),
        request.observation.agent_id,
    );
    raw.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == ':' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct ModelDecisionEnvelope {
    decision: String,
    #[serde(default)]
    ticks: Option<u64>,
    #[serde(default)]
    action_ref: Option<String>,
    #[serde(default)]
    args: Option<Value>,
}

fn parse_model_decision(
    agent_id: &str,
    request: &DecisionRequest,
    raw_text: &str,
) -> Result<(ProviderDecision, u32), String> {
    let mut schema_repair_count = 0;
    let mut candidate = raw_text.trim().to_string();
    if candidate.starts_with("```") {
        candidate = strip_code_fence(candidate.as_str());
        schema_repair_count += 1;
    }
    let envelope: ModelDecisionEnvelope = match serde_json::from_str(candidate.as_str()) {
        Ok(value) => value,
        Err(_) => {
            let repaired = extract_json_object(candidate.as_str()).ok_or_else(|| {
                format!(
                    "no JSON object found in model output: {}",
                    summarize_text(raw_text, 240)
                )
            })?;
            schema_repair_count += 1;
            serde_json::from_str(repaired.as_str()).map_err(|err| {
                format!(
                    "parse repaired model output failed: {err}; raw={}",
                    summarize_text(raw_text, 240)
                )
            })?
        }
    };
    let decision = match envelope.decision.as_str() {
        "wait" => ProviderDecision::Wait,
        "wait_ticks" => ProviderDecision::WaitTicks {
            ticks: envelope
                .ticks
                .filter(|ticks| *ticks > 0)
                .ok_or_else(|| "wait_ticks requires ticks > 0".to_string())?,
        },
        "act" => {
            let action_ref = envelope
                .action_ref
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "act requires non-empty action_ref".to_string())?;
            if !request
                .observation
                .action_catalog
                .iter()
                .any(|entry| entry.action_ref == action_ref)
            {
                return Err(format!(
                    "action_ref `{action_ref}` not present in action_catalog"
                ));
            }
            let args = envelope.args.unwrap_or(Value::Null);
            ProviderDecision::Act {
                action_ref: action_ref.to_string(),
                action: build_action_from_args(agent_id, action_ref, &args)?,
            }
        }
        other => return Err(format!("unsupported decision `{other}`")),
    };
    Ok((decision, schema_repair_count))
}

fn build_action_from_args(
    agent_id: &str,
    action_ref: &str,
    args: &Value,
) -> Result<Action, String> {
    match action_ref {
        "move_agent" => Ok(Action::MoveAgent {
            agent_id: agent_id.to_string(),
            to: required_string(args, "to")?,
        }),
        "speak_to_nearby" => Ok(Action::SpeakToNearby {
            agent_id: agent_id.to_string(),
            message: required_string(args, "message")?,
            target_agent_id: optional_string(args, "target_agent_id"),
        }),
        "inspect_target" => Ok(Action::InspectTarget {
            agent_id: agent_id.to_string(),
            target_kind: required_string(args, "target_kind")?,
            target_id: required_string(args, "target_id")?,
        }),
        "simple_interact" => Ok(Action::SimpleInteract {
            agent_id: agent_id.to_string(),
            target_kind: required_string(args, "target_kind")?,
            target_id: required_string(args, "target_id")?,
            interaction: required_string(args, "interaction")?,
        }),
        other => Err(format!("unsupported action_ref `{other}`")),
    }
}

fn apply_profile_guardrails(
    request: &DecisionRequest,
    decision: ProviderDecision,
) -> (ProviderDecision, Option<String>) {
    if !request
        .observation
        .memory_summary
        .as_deref()
        .unwrap_or_default()
        .contains("goal=巡游移动")
    {
        return (decision, None);
    }
    let current_location_id = estimated_current_location_id(&request.observation.observation);
    let preferred_location =
        nearest_reachable_non_current_location_id(&request.observation.observation);
    let Some(preferred_location) = preferred_location else {
        return (decision, None);
    };
    let move_available = request
        .observation
        .action_catalog
        .iter()
        .any(|entry| entry.action_ref == "move_agent");
    if !move_available {
        return (decision, None);
    }
    match decision {
        ProviderDecision::Wait | ProviderDecision::WaitTicks { .. } => (
            ProviderDecision::Act {
                action_ref: "move_agent".to_string(),
                action: Action::MoveAgent {
                    agent_id: request.observation.agent_id.clone(),
                    to: preferred_location.clone(),
                },
            },
            Some(format!(
                "profile_guardrail_reroute: patrol goal converted passive decision into move_agent(to={})",
                preferred_location
            )),
        ),
        ProviderDecision::Act { action_ref, action } => match action {
            Action::MoveAgent { agent_id, to }
                if to == current_location_id.unwrap_or_default()
                    || !request
                        .observation
                        .observation
                        .nearby_entities
                        .iter()
                        .any(|entity| entity.kind == "location" && entity.entity_ref == to) =>
            {
                (
                    ProviderDecision::Act {
                        action_ref: "move_agent".to_string(),
                        action: Action::MoveAgent {
                            agent_id,
                            to: preferred_location.clone(),
                        },
                    },
                    Some(format!(
                        "profile_guardrail_reroute: invalid move target replaced with move_agent(to={})",
                        preferred_location
                    )),
                )
            }
            _ => (
                ProviderDecision::Act { action_ref, action },
                None,
            ),
        },
    }
}

fn required_string(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("missing non-empty string field `{key}`"))
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn strip_code_fence(text: &str) -> String {
    let stripped = text.trim().trim_start_matches("```");
    let stripped = stripped.strip_prefix("json").unwrap_or(stripped);
    stripped.trim().trim_end_matches("```").trim().to_string()
}

fn extract_json_object(text: &str) -> Option<String> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(text[start..=end].trim().to_string())
}

fn summarize_text(text: &str, max_chars: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        trimmed.to_string()
    } else {
        let prefix = trimmed.chars().take(max_chars).collect::<String>();
        format!("{}…", prefix)
    }
}

fn default_gateway_health_url() -> String {
    let config_path = env::var("HOME")
        .ok()
        .map(PathBuf::from)
        .map(|home| home.join(".openclaw/openclaw.json"));
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
