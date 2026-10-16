//! Compatibility adapter for the shared Harness inference contract.
pub use oasis7_agent_harness::{
    ModelProvider, ModelProviderError, ModelProviderRequest, ModelProviderResponse,
    ModelProviderTool, ModelProviderTrace, ModelProviderTraceEntry, ModelProviderTurn,
    ModelProviderUsage,
};

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
