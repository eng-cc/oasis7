//! Structured stateless protocol history; this module grants no game authority.
use crate::preset::{Protocol, ProviderId, ProviderPreset};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Message {
    User {
        content: String,
    },
    Assistant {
        content: Option<String>,
        tool_calls: Vec<ToolCall>,
        reasoning_content: Option<String>,
        reasoning_details: Option<Value>,
    },
    ToolResult {
        call_id: String,
        content: String,
    },
    ResponsesOutput {
        items: Vec<Value>,
    },
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Conversation {
    pub system_prompt: String,
    pub messages: Vec<Message>,
    pub tools: Vec<Tool>,
    pub max_output_tokens: u32,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}
#[derive(Clone)]
pub struct ProviderResponse {
    pub text: String,
    pub tool_calls: Vec<ToolCall>,
    pub replay: Vec<Message>,
    pub usage: Usage,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProviderFailure {
    AuthenticationFailed,
    BudgetExhausted,
    QuotaUnavailable,
    RateLimited,
    ModelUnavailable,
    CapabilityUnsupported,
    TransportFailed,
    InvalidResponse,
    InputLimit,
    ResponseLimit,
    SessionExpired,
}
impl std::fmt::Display for ProviderFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::AuthenticationFailed => "authentication_failed",
            Self::BudgetExhausted => "budget_exhausted",
            Self::QuotaUnavailable => "quota_unavailable",
            Self::RateLimited => "rate_limited",
            Self::ModelUnavailable => "model_unavailable",
            Self::CapabilityUnsupported => "capability_unsupported",
            Self::TransportFailed => "transport_failed",
            Self::InvalidResponse => "invalid_response",
            Self::InputLimit => "input_limit",
            Self::ResponseLimit => "response_limit",
            Self::SessionExpired => "session_expired",
        })
    }
}
impl std::error::Error for ProviderFailure {}
fn bad() -> ProviderFailure {
    ProviderFailure::InvalidResponse
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, ProviderFailure> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(bad)
}
impl Conversation {
    fn validate(&self, preset: &ProviderPreset) -> Result<(), ProviderFailure> {
        preset
            .validate()
            .map_err(|_| ProviderFailure::CapabilityUnsupported)?;
        if self.max_output_tokens == 0
            || self.max_output_tokens > preset.limits.max_output_tokens
            || self.messages.len() > 256
            || self.tools.len() > 128
        {
            return Err(ProviderFailure::InputLimit);
        }
        let names = self
            .tools
            .iter()
            .map(|t| t.name.as_str())
            .collect::<BTreeSet<_>>();
        if names.len() != self.tools.len()
            || self
                .tools
                .iter()
                .any(|t| t.name.is_empty() || t.name.len() > 128 || !t.parameters.is_object())
        {
            return Err(ProviderFailure::CapabilityUnsupported);
        }
        let mut pending = BTreeSet::new();
        let mut seen = BTreeSet::new();
        for message in &self.messages {
            match message {
                Message::Assistant { tool_calls, .. } => {
                    if !pending.is_empty() {
                        return Err(bad());
                    }
                    for c in tool_calls {
                        if !names.contains(c.name.as_str())
                            || c.id.is_empty()
                            || !seen.insert(c.id.clone())
                            || !c.arguments.is_object()
                        {
                            return Err(bad());
                        }
                        pending.insert(c.id.clone());
                    }
                }
                Message::ToolResult { call_id, .. } => {
                    if !pending.remove(call_id) {
                        return Err(bad());
                    }
                }
                Message::ResponsesOutput { items } => {
                    if preset.protocol != Protocol::Responses || !pending.is_empty() {
                        return Err(bad());
                    }
                    for item in items {
                        validate_response_item(item)?;
                        if item["type"] == "function_call" {
                            let c = parse_response_call(item)?;
                            if !names.contains(c.name.as_str()) || !seen.insert(c.id.clone()) {
                                return Err(bad());
                            }
                            pending.insert(c.id);
                        }
                    }
                }
                Message::User { .. } => {
                    if !pending.is_empty() {
                        return Err(bad());
                    }
                }
            }
        }
        if !pending.is_empty() {
            return Err(bad());
        }
        Ok(())
    }
}
pub fn encode_request(
    preset: &ProviderPreset,
    conversation: &Conversation,
) -> Result<Vec<u8>, ProviderFailure> {
    conversation.validate(preset)?;
    let mut body = match preset.protocol {
        Protocol::ChatCompletions => {
            let mut messages = vec![json!({"role":"system","content":conversation.system_prompt})];
            for message in &conversation.messages {
                messages.push(match message {
                Message::User{content}=>json!({"role":"user","content":content}),
                Message::ToolResult{call_id,content}=>json!({"role":"tool","tool_call_id":call_id,"content":content}),
                Message::Assistant{content,tool_calls,reasoning_content,reasoning_details}=>{
                    let mut m=json!({"role":"assistant","content":content});
                    if !tool_calls.is_empty(){m["tool_calls"]=Value::Array(tool_calls.iter().map(|c|json!({"id":c.id,"type":"function","function":{"name":c.name,"arguments":c.arguments.to_string()}})).collect());}
                    if let Some(r)=reasoning_content {m["reasoning_content"]=json!(r);}
                    if let Some(r)=reasoning_details {m["reasoning_details"]=r.clone();}m
                },Message::ResponsesOutput{..}=>return Err(ProviderFailure::CapabilityUnsupported),
            });
            }
            json!({"model":preset.model,"messages":messages,"tools":conversation.tools.iter().map(|t|json!({"type":"function","function":{"name":t.name,"description":t.description,"parameters":t.parameters}})).collect::<Vec<_>>(),"max_tokens":conversation.max_output_tokens,"stream":false})
        }
        Protocol::Responses => {
            let mut input = Vec::new();
            for message in &conversation.messages {
                match message {
                    Message::User { content } => {
                        input.push(json!({"role":"user","content":content}))
                    }
                    Message::ToolResult { call_id, content } => input.push(
                        json!({"type":"function_call_output","call_id":call_id,"output":content}),
                    ),
                    Message::ResponsesOutput { items } => input.extend(items.clone()),
                    Message::Assistant { .. } => {
                        return Err(ProviderFailure::CapabilityUnsupported);
                    }
                }
            }
            json!({"model":preset.model,"instructions":conversation.system_prompt,"input":input,"tools":conversation.tools.iter().map(|t|json!({"type":"function","name":t.name,"description":t.description,"parameters":t.parameters})).collect::<Vec<_>>(),"max_output_tokens":conversation.max_output_tokens,"stream":false,"store":false})
        }
    };
    if preset.provider == ProviderId::OpenRouter {
        body["provider"] =
            json!({"only":preset.routes,"allow_fallbacks":false,"require_parameters":true});
    }
    let bytes = serde_json::to_vec(&body).map_err(|_| ProviderFailure::InputLimit)?;
    if bytes.len() > preset.limits.max_input_bytes {
        return Err(ProviderFailure::InputLimit);
    }
    Ok(bytes)
}
fn parse_arguments(value: &Value) -> Result<Value, ProviderFailure> {
    let args: Value = serde_json::from_str(value.as_str().ok_or_else(bad)?).map_err(|_| bad())?;
    if !args.is_object() {
        return Err(bad());
    }
    Ok(args)
}
fn parse_response_call(v: &Value) -> Result<ToolCall, ProviderFailure> {
    Ok(ToolCall {
        id: string(v, "call_id")?.into(),
        name: string(v, "name")?.into(),
        arguments: parse_arguments(&v["arguments"])?,
    })
}
fn validate_response_item(v: &Value) -> Result<(), ProviderFailure> {
    match string(v, "type")? {
        "function_call" => {
            parse_response_call(v)?;
        }
        "message" => {
            if v["role"] != "assistant" || !v["content"].is_array() {
                return Err(bad());
            }
        }
        "reasoning" => {}
        _ => return Err(ProviderFailure::CapabilityUnsupported),
    }
    Ok(())
}
fn usage(v: &Value, input: &str, output: &str) -> Usage {
    let input_tokens = v[input].as_u64();
    let output_tokens = v[output].as_u64();
    let reported = v["total_tokens"].as_u64();
    // A partial, malformed or internally inconsistent bill is unknown. Never
    // refund a reservation on a plausible-looking total alone.
    let total_tokens = input_tokens
        .zip(output_tokens)
        .and_then(|(i, o)| i.checked_add(o))
        .zip(reported)
        .filter(|(sum, total)| sum == total)
        .map(|(_, total)| total);
    Usage {
        input_tokens,
        output_tokens,
        total_tokens,
    }
}

pub fn decode_response(
    preset: &ProviderPreset,
    bytes: &[u8],
    content_type: &str,
    tools: &[Tool],
) -> Result<ProviderResponse, ProviderFailure> {
    if bytes.len() > preset.limits.max_response_bytes {
        return Err(ProviderFailure::ResponseLimit);
    }
    // Streaming is deliberately not enabled in this first adapter. An unexpected
    // event stream is not downgraded into text or a partially accepted tool call.
    if content_type
        .split(';')
        .next()
        .is_some_and(|s| s.trim() == "text/event-stream")
    {
        return Err(ProviderFailure::CapabilityUnsupported);
    }
    let value: Value = serde_json::from_slice(bytes).map_err(|_| bad())?;
    if value.get("error").is_some() {
        return Err(bad());
    }
    if value
        .get("model")
        .and_then(Value::as_str)
        .is_some_and(|m| m != preset.model)
    {
        return Err(ProviderFailure::CapabilityUnsupported);
    }
    let (text, calls, replay, usage) = match preset.protocol {
        Protocol::ChatCompletions => {
            let choices = value["choices"]
                .as_array()
                .filter(|v| v.len() == 1)
                .ok_or_else(bad)?;
            let choice = &choices[0];
            let reason = string(choice, "finish_reason")?;
            if !["stop", "tool_calls"].contains(&reason) {
                return Err(ProviderFailure::CapabilityUnsupported);
            }
            let message = &choice["message"];
            if message["role"] != "assistant" {
                return Err(bad());
            }
            let content = message["content"].as_str().map(str::to_owned);
            let mut calls = Vec::new();
            if let Some(items) = message.get("tool_calls") {
                for item in items.as_array().ok_or_else(bad)? {
                    if item["type"] != "function" {
                        return Err(bad());
                    }
                    calls.push(ToolCall {
                        id: string(item, "id")?.into(),
                        name: string(&item["function"], "name")?.into(),
                        arguments: parse_arguments(&item["function"]["arguments"])?,
                    });
                }
            }
            if (reason == "tool_calls") == calls.is_empty()
                || (content.as_deref().unwrap_or("").is_empty() && calls.is_empty())
            {
                return Err(bad());
            }
            let replay = vec![Message::Assistant {
                content: content.clone(),
                tool_calls: calls.clone(),
                reasoning_content: message["reasoning_content"].as_str().map(str::to_owned),
                reasoning_details: message.get("reasoning_details").cloned(),
            }];
            (
                content.unwrap_or_default(),
                calls,
                replay,
                usage(&value["usage"], "prompt_tokens", "completion_tokens"),
            )
        }
        Protocol::Responses => {
            if value["status"] != "completed" {
                return Err(bad());
            }
            let items = value["output"].as_array().ok_or_else(bad)?;
            let mut text = String::new();
            let mut calls = Vec::new();
            for item in items {
                validate_response_item(item)?;
                if item["type"] == "function_call" {
                    calls.push(parse_response_call(item)?);
                } else if item["type"] == "message" {
                    for c in item["content"].as_array().ok_or_else(bad)? {
                        if c["type"] != "output_text" {
                            return Err(ProviderFailure::CapabilityUnsupported);
                        }
                        text.push_str(c["text"].as_str().ok_or_else(bad)?);
                    }
                }
            }
            if text.is_empty() && calls.is_empty() {
                return Err(bad());
            }
            (
                text,
                calls,
                vec![Message::ResponsesOutput {
                    items: items.clone(),
                }],
                usage(&value["usage"], "input_tokens", "output_tokens"),
            )
        }
    };
    let allowed = tools
        .iter()
        .map(|t| t.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut ids = BTreeMap::new();
    for c in &calls {
        if !allowed.contains(c.name.as_str()) || c.id.len() > 256 || ids.insert(&c.id, ()).is_some()
        {
            return Err(ProviderFailure::CapabilityUnsupported);
        }
    }
    Ok(ProviderResponse {
        text,
        tool_calls: calls,
        replay,
        usage,
    })
}
pub fn probe_conversation() -> Conversation {
    Conversation{system_prompt:"Protocol capability check. Call the protocol_echo tool with value ready. Do not perform any other task.".into(),messages:vec![Message::User{content:"Check only.".into()}],tools:vec![Tool{name:"protocol_echo".into(),description:"Non-mutating protocol capability check".into(),parameters:json!({"type":"object","properties":{"value":{"type":"string","enum":["ready"]}},"required":["value"],"additionalProperties":false})}],max_output_tokens:128}
}
#[cfg(test)]
#[path = "protocol_tests.rs"]
mod tests;
