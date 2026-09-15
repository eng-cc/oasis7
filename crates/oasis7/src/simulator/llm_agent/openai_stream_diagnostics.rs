use async_openai::error::OpenAIError;
use async_openai::types::responses::{CreateResponse, ResponseStreamEvent};
use eventsource_stream::EventStream;
use futures_util::StreamExt;
use std::error::Error;
use std::fmt;

use super::decision_flow::summarize_trace_text;
use super::openai_payload::{
    completion_result_from_sdk_stream_events, text_output_from_sdk_stream_events,
};
use super::{LlmClientError, LlmCompletionResult, OpenAiChatCompletionClient};

impl fmt::Debug for OpenAiChatCompletionClient {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OpenAiChatCompletionClient")
            .field("api_base", &self.api_base)
            .field("request_timeout_ms", &self.request_timeout_ms)
            .field(
                "stream_diagnostics_enabled",
                &self.stream_diagnostics_enabled,
            )
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamTransportMetadata {
    pub(super) http_status: Option<u16>,
    pub(super) content_type: Option<String>,
    pub(super) content_encoding: Option<String>,
    pub(super) frame_count: usize,
    pub(super) decoded_frame_count: usize,
    pub(super) elapsed_ms: u64,
    pub(super) nested_transport_cause: String,
}

fn redact_transport_cause(raw: &str) -> String {
    let mut output = Vec::new();
    let mut redact_next = false;
    for token in raw.split_whitespace() {
        if redact_next {
            redact_next = false;
            continue;
        }
        if token.eq_ignore_ascii_case("bearer") {
            output.push("Bearer <redacted>");
            redact_next = true;
        } else if token.to_ascii_lowercase().contains("://") {
            output.push("<url>");
        } else {
            output.push(token);
        }
    }
    summarize_trace_text(output.join(" ").as_str(), 240)
}

pub(super) fn format_stream_transport_diagnostics(metadata: &StreamTransportMetadata) -> String {
    format!(
        "responses stream transport diagnostics: http_status={} content_type={} content_encoding={} frames={} decoded_frames={} elapsed_ms={} cause={}",
        metadata
            .http_status
            .map_or_else(|| "absent".to_string(), |status| status.to_string()),
        metadata.content_type.as_deref().unwrap_or("absent"),
        metadata.content_encoding.as_deref().unwrap_or("absent"),
        metadata.frame_count,
        metadata.decoded_frame_count,
        metadata.elapsed_ms,
        redact_transport_cause(metadata.nested_transport_cause.as_str()),
    )
}

fn stream_transport_error(metadata: StreamTransportMetadata) -> OpenAiRequestError {
    OpenAiRequestError::StreamTransport(metadata)
}

impl OpenAiChatCompletionClient {
    pub(super) fn send_responses_request(
        &self,
        client: &async_openai::Client<async_openai::config::OpenAIConfig>,
        payload: CreateResponse,
    ) -> Result<LlmCompletionResult, OpenAiRequestError> {
        if self.stream_diagnostics_enabled {
            // async-openai owns the response object and exposes only its body stream. The
            // opt-in direct request is therefore a comparative diagnostic path for headers
            // and body transport metadata; the default provider path stays untouched.
            return self.send_instrumented_responses_request(payload);
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| OpenAiRequestError::Other(err.to_string()))?;

        runtime.block_on(async {
            let mut stream = client
                .responses()
                .create_stream(payload)
                .await
                .map_err(OpenAiRequestError::from)?;
            let mut events = Vec::<ResponseStreamEvent>::new();
            while let Some(event) = stream.next().await {
                events.push(event.map_err(OpenAiRequestError::from)?);
            }
            completion_result_from_sdk_stream_events(events).map_err(OpenAiRequestError::Completion)
        })
    }

    pub(super) fn send_responses_request_for_text(
        &self,
        client: &async_openai::Client<async_openai::config::OpenAIConfig>,
        payload: CreateResponse,
    ) -> Result<String, OpenAiRequestError> {
        if self.stream_diagnostics_enabled {
            // See the completion path above: this branch is opt-in comparative diagnostics,
            // not a replacement for the normal async-openai transport.
            return self.send_instrumented_responses_request_for_text(payload);
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| OpenAiRequestError::Other(err.to_string()))?;

        runtime.block_on(async {
            let mut stream = client
                .responses()
                .create_stream(payload)
                .await
                .map_err(OpenAiRequestError::from)?;
            let mut events = Vec::<ResponseStreamEvent>::new();
            while let Some(event) = stream.next().await {
                events.push(event.map_err(OpenAiRequestError::from)?);
            }
            text_output_from_sdk_stream_events(events).map_err(OpenAiRequestError::Completion)
        })
    }

    fn send_instrumented_responses_request(
        &self,
        mut payload: CreateResponse,
    ) -> Result<LlmCompletionResult, OpenAiRequestError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| OpenAiRequestError::Other(err.to_string()))?;

        runtime.block_on(async {
            let events = self
                .collect_instrumented_stream_events(&mut payload)
                .await?;
            completion_result_from_sdk_stream_events(events).map_err(OpenAiRequestError::Completion)
        })
    }

    fn send_instrumented_responses_request_for_text(
        &self,
        mut payload: CreateResponse,
    ) -> Result<String, OpenAiRequestError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| OpenAiRequestError::Other(err.to_string()))?;

        runtime.block_on(async {
            let events = self
                .collect_instrumented_stream_events(&mut payload)
                .await?;
            text_output_from_sdk_stream_events(events).map_err(OpenAiRequestError::Completion)
        })
    }

    async fn collect_instrumented_stream_events(
        &self,
        payload: &mut CreateResponse,
    ) -> Result<Vec<ResponseStreamEvent>, OpenAiRequestError> {
        payload.stream = Some(true);
        let started_at = std::time::Instant::now();
        let response = self
            .diagnostic_http_client
            .post(format!("{}/responses", self.api_base.trim_end_matches('/')))
            .bearer_auth(self.api_key.as_str())
            .json(payload)
            .send()
            .await
            .map_err(|error| {
                stream_transport_error(StreamTransportMetadata {
                    http_status: None,
                    content_type: None,
                    content_encoding: None,
                    frame_count: 0,
                    decoded_frame_count: 0,
                    elapsed_ms: started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                    nested_transport_cause: error.to_string(),
                })
            })?;

        let mut metadata = StreamTransportMetadata {
            http_status: Some(response.status().as_u16()),
            content_type: response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string),
            content_encoding: response
                .headers()
                .get(reqwest::header::CONTENT_ENCODING)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string),
            frame_count: 0,
            decoded_frame_count: 0,
            elapsed_ms: 0,
            nested_transport_cause: format!("HTTP status {}", response.status().as_u16()),
        };
        if !response.status().is_success() {
            metadata.elapsed_ms = started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
            return Err(stream_transport_error(metadata));
        }

        let mut stream = EventStream::new(
            response
                .bytes_stream()
                .map(|result| result.map_err(std::io::Error::other)),
        );
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            metadata.frame_count += 1;
            let event = event.map_err(|error| {
                metadata.elapsed_ms =
                    started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;
                metadata.nested_transport_cause = error.to_string();
                stream_transport_error(metadata.clone())
            })?;
            if event.data == "[DONE]" {
                continue;
            }
            if event.event == "keepalive" {
                continue;
            }
            let parsed =
                serde_json::from_str::<ResponseStreamEvent>(&event.data).map_err(|error| {
                    OpenAiRequestError::Other(format!(
                        "responses stream event JSON decode failed: {}",
                        summarize_trace_text(error.to_string().as_str(), 240)
                    ))
                })?;
            metadata.decoded_frame_count += 1;
            events.push(parsed);
        }
        Ok(events)
    }
}

#[derive(Debug)]
pub(super) enum OpenAiRequestError {
    Timeout(String),
    ParseBody(String),
    Completion(LlmClientError),
    StreamTransport(StreamTransportMetadata),
    Other(String),
}

impl From<OpenAIError> for OpenAiRequestError {
    fn from(value: OpenAIError) -> Self {
        fn error_chain_contains_timeout(err: &dyn Error) -> bool {
            let mut current = Some(err);
            while let Some(err) = current {
                let message = err.to_string().to_ascii_lowercase();
                if message.contains("timed out")
                    || message.contains("timeout")
                    || message.contains("deadline has elapsed")
                {
                    return true;
                }
                current = err.source();
            }
            false
        }

        match value {
            OpenAIError::Reqwest(err) if err.is_timeout() || error_chain_contains_timeout(&err) => {
                Self::Timeout(err.to_string())
            }
            OpenAIError::JSONDeserialize(_, raw_body) => Self::ParseBody(raw_body),
            OpenAIError::StreamError(err) if error_chain_contains_timeout(err.as_ref()) => {
                Self::Timeout(err.to_string())
            }
            other => Self::Other(other.to_string()),
        }
    }
}
