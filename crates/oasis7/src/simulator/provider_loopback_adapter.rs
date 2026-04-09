use super::{
    Action, DecisionProvider, DecisionProviderError, DecisionRequest, DecisionResponse,
    FeedbackEnvelope, ProviderDecision, ProviderFeedbackAck, ProviderLoopbackHttpClient,
    ProviderLoopbackHttpError,
};

const DEFAULT_PROVIDER_LOOPBACK_ADAPTER_PROVIDER_ID: &str = "provider_loopback_http";
const PHASE1_ALLOWED_ACTION_REFS: &[&str] = &[
    "wait",
    "wait_ticks",
    "move_agent",
    "speak_to_nearby",
    "inspect_target",
    "simple_interact",
];

#[derive(Debug)]
pub struct ProviderLoopbackAdapter {
    provider_id: String,
    client: ProviderLoopbackHttpClient,
}

impl ProviderLoopbackAdapter {
    pub fn new(
        base_url: &str,
        auth_token: Option<&str>,
        timeout_ms: u64,
    ) -> Result<Self, ProviderLoopbackHttpError> {
        let client = ProviderLoopbackHttpClient::new(base_url, auth_token, timeout_ms)?;
        Ok(Self::with_client(
            DEFAULT_PROVIDER_LOOPBACK_ADAPTER_PROVIDER_ID,
            client,
        ))
    }

    pub fn with_client(provider_id: impl Into<String>, client: ProviderLoopbackHttpClient) -> Self {
        let provider_id = provider_id.into();
        Self {
            provider_id: if provider_id.trim().is_empty() {
                DEFAULT_PROVIDER_LOOPBACK_ADAPTER_PROVIDER_ID.to_string()
            } else {
                provider_id
            },
            client,
        }
    }

    pub fn phase1_allowed_action_refs() -> &'static [&'static str] {
        PHASE1_ALLOWED_ACTION_REFS
    }

    fn validate_response(
        &self,
        request: &DecisionRequest,
        response: &DecisionResponse,
    ) -> Result<(), DecisionProviderError> {
        if let Some(error) = &response.provider_error {
            return Err(DecisionProviderError::new(
                error.code.clone(),
                error.message.clone(),
                error.retryable,
            ));
        }
        match &response.decision {
            ProviderDecision::Wait => Ok(()),
            ProviderDecision::WaitTicks { .. } => {
                if Self::phase1_allowed_action_refs().contains(&"wait_ticks") {
                    Ok(())
                } else {
                    Err(DecisionProviderError::new(
                        "action_ref_not_allowed",
                        "ProviderLoopbackAdapter phase-1 whitelist does not permit wait_ticks",
                        false,
                    ))
                }
            }
            ProviderDecision::Act { action_ref, action } => {
                if !request
                    .observation
                    .action_catalog
                    .iter()
                    .any(|entry| entry.action_ref == *action_ref)
                {
                    return Err(DecisionProviderError::new(
                        "action_ref_not_in_catalog",
                        format!(
                            "action_ref `{action_ref}` is not present in request action_catalog"
                        ),
                        false,
                    ));
                }
                if !Self::phase1_allowed_action_refs()
                    .iter()
                    .any(|allowed| *allowed == action_ref)
                {
                    return Err(DecisionProviderError::new(
                        "action_ref_not_allowed",
                        format!("action_ref `{action_ref}` is outside ProviderLoopbackAdapter phase-1 whitelist"),
                        false,
                    ));
                }
                match resolved_action_ref(action) {
                    Some(expected_action_ref) if expected_action_ref == action_ref => Ok(()),
                    Some(expected_action_ref) => Err(DecisionProviderError::new(
                        "action_ref_mismatch",
                        format!(
                            "action_ref `{action_ref}` does not match serialized action kind `{expected_action_ref}`"
                        ),
                        false,
                    )),
                    None => Err(DecisionProviderError::new(
                        "action_kind_not_supported",
                        format!(
                            "action_ref `{action_ref}` is phase-1 allowed but its Action variant is not yet supported by ProviderLoopbackAdapter"
                        ),
                        false,
                    )),
                }
            }
        }
    }

    fn map_http_error(error: ProviderLoopbackHttpError) -> DecisionProviderError {
        match error {
            ProviderLoopbackHttpError::InvalidBaseUrl(detail) => {
                DecisionProviderError::new("provider_config_invalid", detail, false)
            }
            ProviderLoopbackHttpError::RequestFailed { detail, .. } => {
                DecisionProviderError::new("provider_unreachable", detail, true)
            }
            ProviderLoopbackHttpError::Unauthorized { detail, .. } => {
                DecisionProviderError::new("provider_unauthorized", detail, false)
            }
            ProviderLoopbackHttpError::UnexpectedStatus {
                status_code, body, ..
            } => DecisionProviderError::new(
                format!("provider_http_{status_code}"),
                if body.is_empty() {
                    format!("provider loopback bridge returned HTTP {status_code}")
                } else {
                    format!("provider loopback bridge returned HTTP {status_code}: {body}")
                },
                status_code >= 500,
            ),
            ProviderLoopbackHttpError::DecodeFailed { detail, .. } => {
                DecisionProviderError::new("provider_payload_invalid", detail, false)
            }
        }
    }

    fn map_feedback_ack_error(ack: ProviderFeedbackAck) -> Result<(), DecisionProviderError> {
        if ack.ok {
            return Ok(());
        }
        Err(DecisionProviderError::new(
            ack.error_code
                .unwrap_or_else(|| "feedback_rejected".to_string()),
            ack.error
                .unwrap_or_else(|| "provider feedback endpoint rejected payload".to_string()),
            false,
        ))
    }
}

impl DecisionProvider for ProviderLoopbackAdapter {
    fn provider_id(&self) -> &str {
        self.provider_id.as_str()
    }

    fn decide(
        &mut self,
        request: &DecisionRequest,
    ) -> Result<DecisionResponse, DecisionProviderError> {
        let response = self
            .client
            .request_decision(request)
            .map_err(Self::map_http_error)?;
        self.validate_response(request, &response)?;
        Ok(response)
    }

    fn push_feedback(&mut self, feedback: &FeedbackEnvelope) -> Result<(), DecisionProviderError> {
        let ack = self
            .client
            .submit_feedback(feedback)
            .map_err(Self::map_http_error)?;
        Self::map_feedback_ack_error(ack)
    }
}

fn resolved_action_ref(action: &Action) -> Option<&'static str> {
    match action {
        Action::MoveAgent { .. } => Some("move_agent"),
        Action::SpeakToNearby { .. } => Some("speak_to_nearby"),
        Action::InspectTarget { .. } => Some("inspect_target"),
        Action::SimpleInteract { .. } => Some("simple_interact"),
        _ => None,
    }
}
