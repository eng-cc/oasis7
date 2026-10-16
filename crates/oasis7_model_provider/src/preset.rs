//! Deployment-owned provider combinations. No caller-supplied origin or headers.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderId {
    #[serde(rename = "deepseek")]
    DeepSeek,
    #[serde(rename = "openrouter")]
    OpenRouter,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    ChatCompletions,
    Responses,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestLimits {
    pub max_input_bytes: usize,
    pub max_response_bytes: usize,
    pub max_output_tokens: u32,
    pub timeout_ms: u64,
}
impl Default for RequestLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 256 * 1024,
            max_response_bytes: 2 * 1024 * 1024,
            max_output_tokens: 4096,
            timeout_ms: 30_000,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderPreset {
    pub id: String,
    pub provider: ProviderId,
    pub protocol: Protocol,
    pub model: String,
    pub routes: Vec<String>,
    pub revision: u64,
    pub limits: RequestLimits,
}
impl ProviderPreset {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.id.is_empty()
            || self.id.len() > 128
            || self.id.chars().any(char::is_control)
            || self.revision == 0
            || self.model.is_empty()
            || self.model.len() > 256
            || self.model.chars().any(char::is_control)
            || self.routes.len() > 16
            || self
                .routes
                .iter()
                .any(|v| v.is_empty() || v.len() > 128 || v.chars().any(char::is_control))
            || (self.provider == ProviderId::OpenRouter && self.routes.is_empty())
            || (self.provider == ProviderId::DeepSeek && !self.routes.is_empty())
            || !(1..=1_048_576).contains(&self.limits.max_input_bytes)
            || !(1..=4_194_304).contains(&self.limits.max_response_bytes)
            || !(1..=32768).contains(&self.limits.max_output_tokens)
            || !(1..=120_000).contains(&self.limits.timeout_ms)
        {
            return Err("invalid_provider_preset");
        }
        // Responses route controls are not yet proven equivalent on OpenRouter.
        if self.provider == ProviderId::OpenRouter && self.protocol == Protocol::Responses {
            return Err("capability_unsupported");
        }
        Ok(())
    }
    pub fn host(&self) -> &'static str {
        match self.provider {
            ProviderId::DeepSeek => "api.deepseek.com",
            ProviderId::OpenRouter => "openrouter.ai",
        }
    }
    pub fn path(&self) -> &'static str {
        match (self.provider, self.protocol) {
            (ProviderId::DeepSeek, Protocol::ChatCompletions) => "/chat/completions",
            (ProviderId::DeepSeek, Protocol::Responses) => "/responses",
            (ProviderId::OpenRouter, Protocol::ChatCompletions) => "/api/v1/chat/completions",
            (ProviderId::OpenRouter, Protocol::Responses) => "/api/v1/responses",
        }
    }
}
