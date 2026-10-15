//! Production inference transport shared by native agents and legacy adapters.
use async_openai::types::responses::{
    CreateResponse, CreateResponseArgs, FunctionTool, OutputItem, Response, Tool,
    ToolChoiceOptions, ToolChoiceParam,
};
use async_openai::{Client as AsyncOpenAiClient, config::OpenAIConfig};
use serde::Serialize;
use std::{error::Error, fmt};
mod config;
pub mod config_constants;
pub mod openai_payload;
pub mod openai_retry;
pub mod openai_stream_diagnostics;
use config_constants::ENV_LLM_STREAM_DIAGNOSTICS;
mod harness_provider;
pub use config::*;
pub use harness_provider::CompletionToolProfile;
use openai_payload::{
    build_responses_request_payload, build_text_probe_request_payload,
    build_tool_probe_request_payload, normalize_openai_api_base_url,
};
use openai_retry::{RATE_LIMIT_RETRY_DELAY_MS, is_concurrency_limit_error, retry_attempts};
use openai_stream_diagnostics::{OpenAiRequestError, format_stream_transport_diagnostics};
pub trait CompletionClientConfig {
    fn base_url(&self) -> &str;
    fn api_key(&self) -> &str;
    fn timeout_ms(&self) -> u64;
}
pub fn parse_debug_mode_flag(value: &str) -> Option<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}
fn summarize_trace_text(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    text.chars().take(max_chars).collect::<String>() + "...(truncated)"
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlmCompletionRequest {
    pub model: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub debug_mode: bool,
    /// Native request budgets count physical provider invocations.  When set,
    /// the completion client must not hide another provider call inside its
    /// transport retry loop.  `None` preserves the legacy unbudgeted client
    /// behavior, including the bounded OpenAI concurrency retry.
    pub max_model_calls: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LlmCompletionTurn {
    Decision {
        payload: serde_json::Value,
    },
    ModuleCall {
        module: String,
        args: serde_json::Value,
    },
}

pub trait LlmCompletionClient {
    fn complete(
        &self,
        request: &LlmCompletionRequest,
    ) -> Result<LlmCompletionResult, LlmClientError>;
}

#[derive(Debug, Clone, PartialEq)]
pub struct LlmCompletionResult {
    pub turns: Vec<LlmCompletionTurn>,
    pub output: String,
    pub model: Option<String>,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
}

#[derive(Clone)]
pub struct OpenAiChatCompletionClient {
    client: AsyncOpenAiClient<OpenAIConfig>,
    diagnostic_http_client: reqwest::Client,
    api_base: String,
    api_key: String,
    request_timeout_ms: u64,
    stream_diagnostics_enabled: bool,
    legacy_tool_profile: bool,
}

impl OpenAiChatCompletionClient {
    pub fn request_timeout_ms(&self) -> u64 {
        self.request_timeout_ms
    }

    pub fn from_config(config: &impl CompletionClientConfig) -> Result<Self, LlmClientError> {
        let request_timeout_ms = config.timeout_ms().max(1);
        let api_base = normalize_openai_api_base_url(config.base_url());
        let http_client = Self::build_http_client(request_timeout_ms)?;
        let client = Self::build_client_with_http_client(
            api_base.as_str(),
            config.api_key(),
            http_client.clone(),
        )?;

        Ok(Self {
            legacy_tool_profile: true,
            client,
            diagnostic_http_client: http_client,
            api_base,
            api_key: config.api_key().to_owned(),
            request_timeout_ms,
            stream_diagnostics_enabled: std::env::var(ENV_LLM_STREAM_DIAGNOSTICS)
                .ok()
                .and_then(|value| parse_debug_mode_flag(value.as_str()))
                .unwrap_or(false),
        })
    }

    pub fn probe_hello_response(&self, model: &str) -> Result<String, LlmClientError> {
        let payload = build_text_probe_request_payload(model, "hello")?;
        match self.send_responses_request_for_text(&self.client, payload) {
            Ok(text) => Ok(text),
            Err(OpenAiRequestError::ParseBody(raw_body)) => Err(LlmClientError::DecodeResponse {
                message: format!(
                    "responses sdk decode failed (text probe): {}",
                    summarize_trace_text(raw_body.as_str(), 320)
                ),
            }),
            Err(OpenAiRequestError::Timeout(err)) => Err(LlmClientError::Http {
                message: format!(
                    "request timed out after {}ms: {}",
                    self.request_timeout_ms, err
                ),
            }),
            Err(OpenAiRequestError::Completion(err)) => Err(err),
            Err(OpenAiRequestError::StreamTransport(metadata)) => Err(LlmClientError::Http {
                message: format_stream_transport_diagnostics(&metadata),
            }),
            Err(OpenAiRequestError::Other(err)) => Err(LlmClientError::Http { message: err }),
        }
    }

    pub fn probe_required_tool_response(
        &self,
        model: &str,
    ) -> Result<LlmCompletionResult, LlmClientError> {
        let payload = build_tool_probe_request_payload(model)?;
        match self.send_responses_request(&self.client, payload) {
            Ok(result) => Ok(result),
            Err(OpenAiRequestError::ParseBody(raw_body)) => Err(LlmClientError::DecodeResponse {
                message: format!(
                    "responses sdk decode failed (tool probe): {}",
                    summarize_trace_text(raw_body.as_str(), 320)
                ),
            }),
            Err(OpenAiRequestError::Timeout(err)) => Err(LlmClientError::Http {
                message: format!(
                    "request timed out after {}ms: {}",
                    self.request_timeout_ms, err
                ),
            }),
            Err(OpenAiRequestError::Completion(err)) => Err(err),
            Err(OpenAiRequestError::StreamTransport(metadata)) => Err(LlmClientError::Http {
                message: format_stream_transport_diagnostics(&metadata),
            }),
            Err(OpenAiRequestError::Other(err)) => Err(LlmClientError::Http { message: err }),
        }
    }

    fn build_http_client(timeout_ms: u64) -> Result<reqwest::Client, LlmClientError> {
        #[cfg(target_arch = "wasm32")]
        let builder = {
            let _ = timeout_ms;
            reqwest::Client::builder()
        };

        #[cfg(not(target_arch = "wasm32"))]
        let builder =
            reqwest::Client::builder().timeout(std::time::Duration::from_millis(timeout_ms.max(1)));

        builder.build().map_err(|err| LlmClientError::BuildClient {
            message: err.to_string(),
        })
    }

    /// One durable native call reservation permits one physical HTTP attempt.
    /// async-openai's default executor adds three retries even when our outer
    /// retry loop is disabled, so install its plain public ReqwestService.
    fn single_attempt_client(&self) -> Result<Self, LlmClientError> {
        #[cfg(not(target_arch = "wasm32"))]
        let builder = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(
                self.request_timeout_ms.max(1),
            ))
            .retry(reqwest::retry::never())
            .redirect(reqwest::redirect::Policy::none());
        #[cfg(target_arch = "wasm32")]
        let builder = reqwest::Client::builder();
        let http = builder.build().map_err(|err| LlmClientError::BuildClient {
            message: err.to_string(),
        })?;
        let config = OpenAIConfig::new()
            .with_api_base(self.api_base.clone())
            .with_api_key(self.api_key.clone());
        let mut client = self.clone();
        client.client = AsyncOpenAiClient::with_config(config)
            .with_http_client(http.clone())
            .with_http_service(async_openai::middleware::ReqwestService::new(http.clone()));
        client.diagnostic_http_client = http;
        Ok(client)
    }

    fn build_client_with_http_client(
        api_base: &str,
        api_key: &str,
        http_client: reqwest::Client,
    ) -> Result<AsyncOpenAiClient<OpenAIConfig>, LlmClientError> {
        let config = OpenAIConfig::new()
            .with_api_base(api_base.to_string())
            .with_api_key(api_key.to_string());

        Ok(AsyncOpenAiClient::with_config(config).with_http_client(http_client))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmClientError {
    BuildClient { message: String },
    Http { message: String },
    HttpStatus { code: u16, message: String },
    DecodeResponse { message: String },
    EmptyChoice,
}

impl fmt::Display for LlmClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LlmClientError::BuildClient { message } => write!(f, "client build failed: {message}"),
            LlmClientError::Http { message } => write!(f, "http request failed: {message}"),
            LlmClientError::HttpStatus { code, message } => {
                write!(f, "http status {code}: {message}")
            }
            LlmClientError::DecodeResponse { message } => {
                write!(f, "decode response failed: {message}")
            }
            LlmClientError::EmptyChoice => write!(f, "empty completion choice"),
        }
    }
}

impl Error for LlmClientError {}

pub mod legacy_tool_names {
    pub const OPENAI_TOOL_AGENT_MODULES_LIST: &str = "agent_modules_list";
    pub const OPENAI_TOOL_ENVIRONMENT_CURRENT_OBSERVATION: &str = "environment_current_observation";
    pub const OPENAI_TOOL_MEMORY_SHORT_TERM_RECENT: &str = "memory_short_term_recent";
    pub const OPENAI_TOOL_MEMORY_LONG_TERM_SEARCH: &str = "memory_long_term_search";
    pub const OPENAI_TOOL_WORLD_RULES_GUIDE: &str = "world_rules_guide";
    pub const OPENAI_TOOL_MODULE_LIFECYCLE_STATUS: &str = "module_lifecycle_status";
    pub const OPENAI_TOOL_POWER_ORDER_BOOK_STATUS: &str = "power_order_book_status";
    pub const OPENAI_TOOL_MODULE_MARKET_STATUS: &str = "module_market_status";
    pub const OPENAI_TOOL_SOCIAL_STATE_STATUS: &str = "social_state_status";
    pub const OPENAI_TOOL_AGENT_SUBMIT_DECISION: &str = "agent_submit_decision";
    pub const OPENAI_TOOL_AGENT_DEBUG_GRANT_RESOURCE: &str = "agent_debug_grant_resource";
}
use legacy_tool_names::*;

impl LlmCompletionClient for OpenAiChatCompletionClient {
    fn complete(
        &self,
        request: &LlmCompletionRequest,
    ) -> Result<LlmCompletionResult, LlmClientError> {
        let payload = build_responses_request_payload(request)?;
        self.complete_payload(request, payload)
    }
}

impl OpenAiChatCompletionClient {
    fn complete_payload(
        &self,
        request: &LlmCompletionRequest,
        payload: CreateResponse,
    ) -> Result<LlmCompletionResult, LlmClientError> {
        if request.max_model_calls == Some(0) {
            return Err(LlmClientError::DecodeResponse {
                message: "model call budget exhausted".into(),
            });
        }
        let single_attempt = request
            .max_model_calls
            .map(|_| self.single_attempt_client())
            .transpose()?;
        let transport = single_attempt.as_ref().unwrap_or(self);
        let retry_attempts = retry_attempts(request.max_model_calls);
        for attempt in 0..=retry_attempts {
            match transport.send_responses_request(&transport.client, payload.clone()) {
                Ok(result) => return Ok(result),
                Err(OpenAiRequestError::ParseBody(raw_body))
                    if attempt < retry_attempts
                        && is_concurrency_limit_error(raw_body.as_str()) =>
                {
                    #[cfg(not(target_arch = "wasm32"))]
                    std::thread::sleep(std::time::Duration::from_millis(RATE_LIMIT_RETRY_DELAY_MS));
                    continue;
                }
                Err(OpenAiRequestError::ParseBody(raw_body)) => {
                    return Err(LlmClientError::DecodeResponse {
                        message: format!(
                            "responses sdk decode failed (primary request): {}",
                            summarize_trace_text(raw_body.as_str(), 320)
                        ),
                    });
                }
                Err(OpenAiRequestError::Timeout(err)) => {
                    return Err(LlmClientError::Http {
                        message: format!(
                            "request timed out after {}ms: {}",
                            self.request_timeout_ms, err
                        ),
                    });
                }
                Err(OpenAiRequestError::Completion(err)) => {
                    return Err(err);
                }
                Err(OpenAiRequestError::StreamTransport(metadata)) => {
                    return Err(LlmClientError::Http {
                        message: format_stream_transport_diagnostics(&metadata),
                    });
                }
                Err(OpenAiRequestError::Other(err)) => {
                    return Err(LlmClientError::Http { message: err });
                }
            }
        }

        Err(LlmClientError::Http {
            message: "responses request exhausted retry budget".to_string(),
        })
    }
}

#[cfg(test)]
#[path = "diagnostic_compat_tests.rs"]
mod diagnostic_compat_tests;
