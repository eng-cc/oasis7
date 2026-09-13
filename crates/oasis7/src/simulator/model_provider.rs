//! Provider-neutral inference boundary for the native cognition lane.
//!
//! `ModelProvider` owns one physical inference request and its provider
//! diagnostics. It deliberately does not know about world actions,
//! capabilities, receipts, or memory commits; those remain TurnEngine and
//! Runtime responsibilities respectively.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::error::Error;
use std::fmt;

use super::{BudgetContractV1, Digest32};

/// A schema supplied by the trusted host for one provider tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelProviderTool {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: Value,
}

/// The inference-only request. Identity is carried for correlation, but the
/// provider cannot choose or reinterpret the Runtime authority represented by
/// that identity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelProviderRequest {
    pub request_digest: Digest32,
    pub model: String,
    pub system_prompt: String,
    pub user_prompt: String,
    #[serde(default)]
    pub tools: Vec<ModelProviderTool>,
    pub budget: BudgetContractV1,
}

impl ModelProviderRequest {
    pub fn validate(&self) -> Result<(), ModelProviderError> {
        if !self.request_digest.is_canonical_blake3() {
            return Err(ModelProviderError::new(
                "invalid_request_digest",
                "model provider request digest must be a canonical BLAKE3-256 value",
                false,
            ));
        }
        if self.model.trim().is_empty() || self.model.len() > 256 {
            return Err(ModelProviderError::new(
                "invalid_model",
                "model identifier must be non-empty and bounded",
                false,
            ));
        }
        for tool in &self.tools {
            if tool.name.trim().is_empty() || tool.name.len() > 128 {
                return Err(ModelProviderError::new(
                    "invalid_tool_schema",
                    "tool name must be non-empty and bounded",
                    false,
                ));
            }
        }
        Ok(())
    }
}

/// Typed output from one model invocation. The host remains responsible for
/// mapping a decision/tool result into a candidate and Runtime envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelProviderTurn {
    Decision { payload: Value },
    ToolCall { name: String, arguments: Value },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ModelProviderUsage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ModelProviderTrace {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    #[serde(default)]
    pub transcript: Vec<ModelProviderTraceEntry>,
    #[serde(default)]
    pub tool_trace: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_trace: Option<Value>,
    #[serde(default)]
    pub repair_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelProviderTraceEntry {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelProviderResponse {
    pub request_digest: Digest32,
    pub turns: Vec<ModelProviderTurn>,
    #[serde(default)]
    pub output: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default)]
    pub usage: ModelProviderUsage,
    #[serde(default)]
    pub trace: ModelProviderTrace,
}

impl ModelProviderResponse {
    pub fn validate_for(&self, request: &ModelProviderRequest) -> Result<(), ModelProviderError> {
        if self.request_digest != request.request_digest {
            return Err(ModelProviderError::new(
                "response_identity_mismatch",
                "model provider response digest does not match the request",
                false,
            ));
        }
        if self.turns.is_empty() {
            return Err(ModelProviderError::new(
                "empty_model_response",
                "model provider response contains no typed turns",
                false,
            ));
        }
        Ok(())
    }
}

/// Stable provider-independent error envelope. Provider-specific details are
/// retained only as bounded diagnostics by the caller.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelProviderError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_trace: Option<Value>,
}

impl ModelProviderError {
    pub fn new(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable,
            upstream_trace: None,
        }
    }

    pub fn with_upstream_trace(
        code: impl Into<String>,
        message: impl Into<String>,
        retryable: bool,
        upstream_trace: Option<Value>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable,
            upstream_trace,
        }
    }

    pub fn code(&self) -> &str {
        self.code.as_str()
    }
}

impl fmt::Display for ModelProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl Error for ModelProviderError {}

/// Physical inference adapter. It may report usage and provider diagnostics,
/// but cannot commit a candidate or mutate Runtime state.
pub trait ModelProvider: Send {
    fn provider_id(&self) -> &str;

    fn complete(
        &mut self,
        request: &ModelProviderRequest,
    ) -> Result<ModelProviderResponse, ModelProviderError>;

    /// Host-side checked entry point. Invalid identity or malformed provider
    /// output is rejected before it can reach TurnEngine candidate handling.
    fn complete_checked(
        &mut self,
        request: &ModelProviderRequest,
    ) -> Result<ModelProviderResponse, ModelProviderError> {
        request.validate()?;
        let response = self.complete(request)?;
        response.validate_for(request)?;
        Ok(response)
    }
}

/// Additive bridge for the existing native completion client. This keeps
/// `LlmCompletionClient` source-compatible while giving the next TurnEngine
/// implementation one provider-neutral inference seam.
#[cfg(not(target_arch = "wasm32"))]
pub struct LlmCompletionModelProvider<C> {
    provider_id: String,
    client: C,
}

#[cfg(not(target_arch = "wasm32"))]
impl<C> LlmCompletionModelProvider<C> {
    pub fn new(provider_id: impl Into<String>, client: C) -> Self {
        Self {
            provider_id: provider_id.into(),
            client,
        }
    }

    pub fn into_inner(self) -> C {
        self.client
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<C: super::llm_agent::LlmCompletionClient + Send> ModelProvider
    for LlmCompletionModelProvider<C>
{
    fn provider_id(&self) -> &str {
        self.provider_id.as_str()
    }

    fn complete(
        &mut self,
        request: &ModelProviderRequest,
    ) -> Result<ModelProviderResponse, ModelProviderError> {
        request.validate()?;
        let completion_request = super::llm_agent::LlmCompletionRequest {
            model: request.model.clone(),
            system_prompt: request.system_prompt.clone(),
            user_prompt: request.user_prompt.clone(),
            debug_mode: false,
            max_model_calls: Some(request.budget.max_model_calls),
        };
        let completion = self.client.complete(&completion_request).map_err(|error| {
            let (code, retryable) = match &error {
                super::llm_agent::LlmClientError::Http { .. }
                | super::llm_agent::LlmClientError::HttpStatus { .. } => {
                    ("provider_unavailable", true)
                }
                super::llm_agent::LlmClientError::BuildClient { .. } => {
                    ("provider_unavailable", false)
                }
                super::llm_agent::LlmClientError::DecodeResponse { .. }
                | super::llm_agent::LlmClientError::EmptyChoice => {
                    ("invalid_model_response", false)
                }
            };
            ModelProviderError::new(code, error.to_string(), retryable)
        })?;
        let turns = completion
            .turns
            .into_iter()
            .map(|turn| match turn {
                super::llm_agent::LlmCompletionTurn::Decision { payload } => {
                    ModelProviderTurn::Decision { payload }
                }
                super::llm_agent::LlmCompletionTurn::ModuleCall { module, args } => {
                    ModelProviderTurn::ToolCall {
                        name: module,
                        arguments: args,
                    }
                }
            })
            .collect();
        Ok(ModelProviderResponse {
            request_digest: request.request_digest.clone(),
            turns,
            output: completion.output,
            model: completion.model,
            usage: ModelProviderUsage {
                prompt_tokens: completion.prompt_tokens,
                completion_tokens: completion.completion_tokens,
                total_tokens: completion.total_tokens,
            },
            trace: ModelProviderTrace {
                provider_id: Some(self.provider_id.clone()),
                ..ModelProviderTrace::default()
            },
        })
    }
}
