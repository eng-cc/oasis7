use serde_json::{Value, json};

use super::super::{
    LlmChatMessageTrace, LlmChatRole, LlmDecisionDiagnostics, LlmPromptSectionTrace, LlmStepTrace,
};
use super::{
    AgentDecision, AgentDecisionTrace, ContinuousAgentResponseContextV1, DecisionProviderError,
    DecisionRequest, DecisionResponse, Observation, ProviderTraceEnvelope,
};

const MAX_PROVIDER_TRANSCRIPT_ENTRIES: usize = 64;
const MAX_PROVIDER_TOOL_TRACE_ENTRIES: usize = 64;
const MAX_PROVIDER_TRANSCRIPT_ENTRY_BYTES: usize = 2 * 1024;
const MAX_PROVIDER_TOOL_TRACE_ENTRY_BYTES: usize = 1024;
const MAX_PROVIDER_SUMMARY_BYTES: usize = 1024;
const MAX_PROVIDER_TRACE_BYTES: usize = 32 * 1024;
const TRACE_REDACTED_VALUE: &str = "<redacted>";
const TRACE_OVERFLOW_DIAGNOSTIC: &str = "trace_payload_too_large";

fn trace_key_is_sensitive(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect::<String>();
    [
        "credential",
        "token",
        "authorization",
        "privatekey",
        "password",
        "secret",
        "cookie",
        "apikey",
        "accesskey",
        "refreshkey",
        "sessionkey",
        "path",
    ]
    .iter()
    .any(|sensitive_key| normalized.contains(sensitive_key))
}

fn trace_text_is_sensitive(text: &str) -> bool {
    let normalized = text.to_ascii_lowercase();
    [
        "credential",
        "authorization",
        "private_key",
        "privatekey",
        "password",
        "secret",
        "api_key",
        "apikey",
        "/private/",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
        || normalized.contains("token-secret")
        || normalized.contains("token=")
        || normalized.contains("token:")
}

fn redact_trace_json(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| {
                    (
                        key.clone(),
                        if trace_key_is_sensitive(key) {
                            Value::String(TRACE_REDACTED_VALUE.to_string())
                        } else {
                            redact_trace_json(value)
                        },
                    )
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(redact_trace_json).collect()),
        Value::String(text) if trace_text_is_sensitive(text) => {
            Value::String(TRACE_REDACTED_VALUE.to_string())
        }
        _ => value.clone(),
    }
}

fn redact_trace_text(text: &str) -> String {
    if let Ok(value) = serde_json::from_str::<Value>(text) {
        return serde_json::to_string(&redact_trace_json(&value))
            .unwrap_or_else(|_| TRACE_REDACTED_VALUE.to_string());
    }
    if trace_text_is_sensitive(text) {
        TRACE_REDACTED_VALUE.to_string()
    } else {
        text.to_string()
    }
}

fn truncate_trace_text(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

fn normalize_trace_text(text: &str, max_bytes: usize, overflow: &mut bool) -> String {
    if text.len() > max_bytes {
        *overflow = true;
    }
    let redacted = redact_trace_text(text);
    truncate_trace_text(&redacted, max_bytes)
}

fn normalize_trace_fallback_text(text: &str, max_bytes: usize) -> String {
    let mut ignored_overflow = false;
    normalize_trace_text(text, max_bytes, &mut ignored_overflow)
}

fn normalize_trace_json(value: &Value, max_bytes: usize, overflow: &mut bool) -> String {
    let serialized =
        serde_json::to_string(value).unwrap_or_else(|_| TRACE_REDACTED_VALUE.to_string());
    normalize_trace_text(&serialized, max_bytes, overflow)
}

fn provider_trace_payload_exceeds_bounds(payload: &ProviderTraceEnvelope) -> bool {
    let summary_exceeds = payload
        .input_summary
        .as_ref()
        .is_some_and(|summary| summary.len() > MAX_PROVIDER_SUMMARY_BYTES)
        || payload
            .output_summary
            .as_ref()
            .is_some_and(|summary| summary.len() > MAX_PROVIDER_SUMMARY_BYTES)
        || payload
            .provider_id
            .as_ref()
            .is_some_and(|provider_id| provider_id.len() > MAX_PROVIDER_SUMMARY_BYTES);
    let transcript_exceeds = payload.transcript.len() > MAX_PROVIDER_TRANSCRIPT_ENTRIES
        || payload
            .transcript
            .iter()
            .any(|entry| entry.content.len() > MAX_PROVIDER_TRANSCRIPT_ENTRY_BYTES);
    let tool_trace_exceeds = payload.tool_trace.len() > MAX_PROVIDER_TOOL_TRACE_ENTRIES
        || payload
            .tool_trace
            .iter()
            .any(|entry| entry.len() > MAX_PROVIDER_TOOL_TRACE_ENTRY_BYTES);
    let upstream_exceeds = payload
        .upstream_trace
        .as_ref()
        .and_then(|trace| serde_json::to_vec(trace).ok())
        .is_some_and(|trace| trace.len() > MAX_PROVIDER_TRACE_BYTES);
    summary_exceeds || transcript_exceeds || tool_trace_exceeds || upstream_exceeds
}

fn serialized_trace_size(trace: &AgentDecisionTrace) -> usize {
    serde_json::to_vec(trace).map_or(usize::MAX, |serialized| serialized.len())
}

fn append_trace_overflow_diagnostic(trace: &mut AgentDecisionTrace) {
    let message = match trace.llm_error.take() {
        Some(existing) if existing.contains(TRACE_OVERFLOW_DIAGNOSTIC) => existing,
        Some(existing) if existing.is_empty() => TRACE_OVERFLOW_DIAGNOSTIC.to_string(),
        Some(existing) => format!("{TRACE_OVERFLOW_DIAGNOSTIC}: {existing}"),
        None => TRACE_OVERFLOW_DIAGNOSTIC.to_string(),
    };
    trace.llm_error = Some(truncate_trace_text(&message, MAX_PROVIDER_SUMMARY_BYTES));
}

fn normalize_trace_aggregate(trace: &mut AgentDecisionTrace, mut overflow: bool) {
    while serialized_trace_size(trace) > MAX_PROVIDER_TRACE_BYTES {
        overflow = true;
        if trace.llm_chat_messages.pop().is_some() {
            continue;
        }
        if trace.llm_step_trace.pop().is_some() {
            continue;
        }
        if trace.llm_output.take().is_some() {
            continue;
        }
        if trace.llm_input.take().is_some() {
            continue;
        }
        if trace.parse_error.take().is_some() {
            continue;
        }
        if let Some(diagnostics) = trace.llm_diagnostics.as_mut() {
            if diagnostics.model.take().is_some() {
                continue;
            }
        }
        break;
    }

    if overflow {
        append_trace_overflow_diagnostic(trace);
    }

    // Provider trace content is never authoritative.  If even its required
    // envelope was inflated by an untrusted provider value, keep only a
    // compact diagnostic projection; the candidate decision itself is held
    // separately by the caller and is not changed by this fallback.
    if serialized_trace_size(trace) > MAX_PROVIDER_TRACE_BYTES {
        trace.agent_id = truncate_trace_text(&trace.agent_id, 256);
        trace.decision = AgentDecision::Wait;
        trace.llm_input = None;
        trace.llm_output = None;
        trace.parse_error = None;
        trace.llm_diagnostics = None;
        trace.llm_effect_intents.clear();
        trace.llm_effect_receipts.clear();
        trace.llm_step_trace.clear();
        trace.llm_prompt_section_trace.clear();
        trace.llm_chat_messages.clear();
        trace.llm_error = Some(TRACE_OVERFLOW_DIAGNOSTIC.to_string());
    }
}

pub(super) fn provider_error_to_trace(error: &DecisionProviderError) -> AgentDecisionTrace {
    let mut overflow = false;
    let mut llm_output = json!({
        "provider_error": {
            "code": error.code,
            "message": normalize_trace_text(
                &error.message,
                MAX_PROVIDER_SUMMARY_BYTES,
                &mut overflow,
            ),
            "retryable": error.retryable,
        },
    });
    if let Some(upstream_trace) = error.upstream_trace.as_ref() {
        llm_output["upstream_trace"] = redact_trace_json(upstream_trace);
    }
    let output = normalize_trace_json(&llm_output, MAX_PROVIDER_SUMMARY_BYTES, &mut overflow);
    let mut trace = AgentDecisionTrace {
        agent_id: String::new(),
        time: 0,
        decision: AgentDecision::Wait,
        llm_input: None,
        llm_output: Some(output),
        llm_error: Some(normalize_trace_text(
            &error.as_trace_message(),
            MAX_PROVIDER_SUMMARY_BYTES,
            &mut overflow,
        )),
        parse_error: None,
        llm_diagnostics: Some(LlmDecisionDiagnostics {
            model: None,
            latency_ms: None,
            prompt_tokens: None,
            completion_tokens: None,
            total_tokens: None,
            retry_count: 0,
        }),
        llm_effect_intents: vec![],
        llm_effect_receipts: vec![],
        llm_step_trace: vec![],
        llm_prompt_section_trace: vec![],
        llm_chat_messages: vec![],
    };
    normalize_trace_aggregate(&mut trace, overflow);
    trace
}

pub(super) fn response_to_trace(
    agent_id: &str,
    observation: &Observation,
    request: &DecisionRequest,
    response: &DecisionResponse,
    outer_response: Option<&ContinuousAgentResponseContextV1>,
    decision: &AgentDecision,
    parse_error: Option<String>,
    provider_error: Option<String>,
) -> AgentDecisionTrace {
    let mut overflow = provider_trace_payload_exceeds_bounds(&response.trace_payload);
    let input_summary = response
        .trace_payload
        .input_summary
        .as_deref()
        .map(|summary| normalize_trace_text(summary, MAX_PROVIDER_SUMMARY_BYTES, &mut overflow))
        .or_else(|| {
            serde_json::to_string(request)
                .ok()
                .map(|summary| normalize_trace_fallback_text(&summary, MAX_PROVIDER_SUMMARY_BYTES))
        });
    let output_summary = outer_response
        .and_then(|outer| serde_json::to_string(outer).ok())
        .map(|summary| normalize_trace_fallback_text(&summary, MAX_PROVIDER_SUMMARY_BYTES))
        .or_else(|| {
            response
                .trace_payload
                .output_summary
                .as_deref()
                .map(|summary| {
                    normalize_trace_text(summary, MAX_PROVIDER_SUMMARY_BYTES, &mut overflow)
                })
        })
        .or_else(|| {
            serde_json::to_string(response)
                .ok()
                .map(|summary| normalize_trace_fallback_text(&summary, MAX_PROVIDER_SUMMARY_BYTES))
        });
    let transcript = response
        .trace_payload
        .transcript
        .iter()
        .take(MAX_PROVIDER_TRANSCRIPT_ENTRIES)
        .map(|entry| LlmChatMessageTrace {
            time: observation.time,
            agent_id: agent_id.to_string(),
            role: match entry.role.as_str() {
                "player" => LlmChatRole::Player,
                "tool" => LlmChatRole::Tool,
                "system" => LlmChatRole::System,
                _ => LlmChatRole::Agent,
            },
            content: normalize_trace_text(
                &entry.content,
                MAX_PROVIDER_TRANSCRIPT_ENTRY_BYTES,
                &mut overflow,
            ),
        })
        .collect();
    let step_trace = response
        .trace_payload
        .tool_trace
        .iter()
        .take(MAX_PROVIDER_TOOL_TRACE_ENTRIES)
        .enumerate()
        .map(|(index, summary)| LlmStepTrace {
            step_index: index,
            step_type: "provider_tool_trace".to_string(),
            input_summary: normalize_trace_text(
                summary,
                MAX_PROVIDER_TOOL_TRACE_ENTRY_BYTES,
                &mut overflow,
            ),
            output_summary: normalize_trace_text(
                summary,
                MAX_PROVIDER_TOOL_TRACE_ENTRY_BYTES,
                &mut overflow,
            ),
            status: "ok".to_string(),
        })
        .collect();
    let model = response
        .diagnostics
        .provider_id
        .clone()
        .or_else(|| response.trace_payload.provider_id.clone())
        .map(|model| normalize_trace_text(&model, MAX_PROVIDER_SUMMARY_BYTES, &mut overflow));
    let llm_error = provider_error
        .map(|error| normalize_trace_text(&error, MAX_PROVIDER_SUMMARY_BYTES, &mut overflow));
    let parse_error = parse_error
        .map(|error| normalize_trace_text(&error, MAX_PROVIDER_SUMMARY_BYTES, &mut overflow));
    let mut trace = AgentDecisionTrace {
        agent_id: agent_id.to_string(),
        time: observation.time,
        decision: decision.clone(),
        llm_input: input_summary,
        llm_output: output_summary,
        llm_error,
        parse_error,
        llm_diagnostics: Some(LlmDecisionDiagnostics {
            model,
            latency_ms: response
                .diagnostics
                .latency_ms
                .or(response.trace_payload.latency_ms),
            prompt_tokens: response
                .trace_payload
                .token_usage
                .as_ref()
                .and_then(|usage| usage.prompt_tokens),
            completion_tokens: response
                .trace_payload
                .token_usage
                .as_ref()
                .and_then(|usage| usage.completion_tokens),
            total_tokens: response
                .trace_payload
                .token_usage
                .as_ref()
                .and_then(|usage| usage.total_tokens),
            retry_count: response.diagnostics.retry_count,
        }),
        llm_effect_intents: vec![],
        llm_effect_receipts: vec![],
        llm_step_trace: step_trace,
        llm_prompt_section_trace: Vec::<LlmPromptSectionTrace>::new(),
        llm_chat_messages: transcript,
    };
    normalize_trace_aggregate(&mut trace, overflow);
    trace
}
