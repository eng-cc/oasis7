use std::error::Error;
use std::fmt;
use std::time::Duration;

use reqwest::blocking::{Client, RequestBuilder};
use reqwest::{Method, StatusCode, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use super::{DecisionRequest, DecisionResponse, FeedbackEnvelope};

const DEFAULT_PROVIDER_LOOPBACK_HTTP_PROVIDER_ID: &str = "provider_loopback_http";
pub const PROVIDER_PHASE1_ACTION_SET_ALIAS: &str = "phase1_low_frequency";
const PROVIDER_PHASE1_REQUIRED_CAPABILITIES: &[&str] = &["decision", "feedback"];
const PROVIDER_PHASE1_REQUIRED_ACTIONS: &[&str] = &[
    "wait",
    "wait_ticks",
    "move_agent",
    "speak_to_nearby",
    "inspect_target",
    "simple_interact",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProviderInfo {
    pub provider_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub supported_action_sets: Vec<String>,
}

impl ProviderInfo {
    pub fn resolved_provider_id(&self) -> &str {
        if self.provider_id.trim().is_empty() {
            DEFAULT_PROVIDER_LOOPBACK_HTTP_PROVIDER_ID
        } else {
            self.provider_id.as_str()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCompatibilityStatus {
    #[default]
    Ready,
    Degraded,
    Incompatible,
}

impl ProviderCompatibilityStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Degraded => "degraded",
            Self::Incompatible => "incompatible",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProviderCompatibilityReport {
    pub status: ProviderCompatibilityStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_supported_actions: Vec<String>,
}

pub fn provider_phase1_required_capabilities() -> &'static [&'static str] {
    PROVIDER_PHASE1_REQUIRED_CAPABILITIES
}

pub fn provider_phase1_required_actions() -> &'static [&'static str] {
    PROVIDER_PHASE1_REQUIRED_ACTIONS
}

pub fn evaluate_provider_compatibility(
    info: &ProviderInfo,
    health: Option<&ProviderHealth>,
) -> ProviderCompatibilityReport {
    let missing_capabilities = PROVIDER_PHASE1_REQUIRED_CAPABILITIES
        .iter()
        .filter(|required| !contains_trimmed_value(info.capabilities.as_slice(), required))
        .map(|required| (*required).to_string())
        .collect::<Vec<_>>();
    if !missing_capabilities.is_empty() {
        return ProviderCompatibilityReport {
            status: ProviderCompatibilityStatus::Incompatible,
            fallback_reason: Some(format!(
                "missing_provider_capabilities:{}",
                missing_capabilities.join(",")
            )),
            missing_capabilities,
            missing_supported_actions: Vec::new(),
        };
    }

    let missing_supported_actions = if contains_trimmed_value(
        info.supported_action_sets.as_slice(),
        PROVIDER_PHASE1_ACTION_SET_ALIAS,
    ) {
        Vec::new()
    } else {
        PROVIDER_PHASE1_REQUIRED_ACTIONS
            .iter()
            .filter(|required| {
                !contains_trimmed_value(info.supported_action_sets.as_slice(), required)
            })
            .map(|required| (*required).to_string())
            .collect::<Vec<_>>()
    };
    if !missing_supported_actions.is_empty() {
        return ProviderCompatibilityReport {
            status: ProviderCompatibilityStatus::Incompatible,
            fallback_reason: Some(format!(
                "missing_supported_actions:{}",
                missing_supported_actions.join(",")
            )),
            missing_capabilities: Vec::new(),
            missing_supported_actions,
        };
    }

    let Some(health) = health else {
        return ProviderCompatibilityReport::default();
    };

    let raw_status = health
        .status
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let status = raw_status.unwrap_or("ok");
    let has_last_error = health
        .last_error
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty());
    let lowered_status = status.to_ascii_lowercase();
    let healthy_status = matches!(lowered_status.as_str(), "ok" | "ready");
    if health.ok && healthy_status && !has_last_error {
        return ProviderCompatibilityReport::default();
    }

    let fallback_reason = health
        .last_error
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            Some(if health.ok {
                format!("provider_health_status:{lowered_status}")
            } else {
                format!(
                    "provider_health_unhealthy:{}",
                    raw_status.unwrap_or("not_ok").to_ascii_lowercase()
                )
            })
        });
    ProviderCompatibilityReport {
        status: ProviderCompatibilityStatus::Degraded,
        fallback_reason,
        missing_capabilities: Vec::new(),
        missing_supported_actions: Vec::new(),
    }
}

fn contains_trimmed_value(values: &[String], expected: &str) -> bool {
    values
        .iter()
        .any(|value| value.trim().eq_ignore_ascii_case(expected))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProviderHealth {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uptime_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queue_depth: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProviderFeedbackAck {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug)]
pub enum ProviderLoopbackHttpError {
    InvalidBaseUrl(String),
    RequestFailed {
        path: String,
        detail: String,
    },
    Unauthorized {
        path: String,
        detail: String,
    },
    UnexpectedStatus {
        path: String,
        status_code: u16,
        body: String,
    },
    DecodeFailed {
        path: String,
        detail: String,
    },
}

impl fmt::Display for ProviderLoopbackHttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBaseUrl(detail) => {
                write!(f, "invalid provider loopback base url: {detail}")
            }
            Self::RequestFailed { path, detail } => {
                write!(f, "provider request {path} failed: {detail}")
            }
            Self::Unauthorized { path, detail } => {
                write!(f, "provider request {path} unauthorized: {detail}")
            }
            Self::UnexpectedStatus {
                path,
                status_code,
                body,
            } => write!(
                f,
                "provider request {path} returned HTTP {status_code}: {body}"
            ),
            Self::DecodeFailed { path, detail } => {
                write!(f, "decode provider response {path} failed: {detail}")
            }
        }
    }
}

impl Error for ProviderLoopbackHttpError {}

#[derive(Debug)]
pub struct ProviderLoopbackHttpClient {
    base_url: Url,
    auth_token: Option<String>,
    http: Client,
}

impl ProviderLoopbackHttpClient {
    pub fn new(
        base_url: &str,
        auth_token: Option<&str>,
        timeout_ms: u64,
    ) -> Result<Self, ProviderLoopbackHttpError> {
        let base_url = validate_provider_loopback_http_base_url(base_url)?;
        let http = Client::builder()
            .timeout(Duration::from_millis(timeout_ms.max(1)))
            .build()
            .map_err(|err| ProviderLoopbackHttpError::RequestFailed {
                path: "<client>".to_string(),
                detail: err.to_string(),
            })?;
        Ok(Self {
            base_url,
            auth_token: auth_token
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned),
            http,
        })
    }

    pub fn provider_info(&self) -> Result<ProviderInfo, ProviderLoopbackHttpError> {
        self.get_json("/v1/provider/info")
    }

    pub fn provider_health(&self) -> Result<ProviderHealth, ProviderLoopbackHttpError> {
        self.get_json("/v1/provider/health")
    }

    pub fn request_decision(
        &self,
        request: &DecisionRequest,
    ) -> Result<DecisionResponse, ProviderLoopbackHttpError> {
        self.post_json("/v1/world-simulator/decision", request)
    }

    pub fn submit_feedback(
        &self,
        feedback: &FeedbackEnvelope,
    ) -> Result<ProviderFeedbackAck, ProviderLoopbackHttpError> {
        self.post_json("/v1/world-simulator/feedback", feedback)
    }

    fn get_json<Response>(&self, path: &str) -> Result<Response, ProviderLoopbackHttpError>
    where
        Response: DeserializeOwned,
    {
        let request = self.build_request(Method::GET, path)?;
        self.send_json(request, path)
    }

    fn post_json<Request, Response>(
        &self,
        path: &str,
        payload: &Request,
    ) -> Result<Response, ProviderLoopbackHttpError>
    where
        Request: Serialize + ?Sized,
        Response: DeserializeOwned,
    {
        let request = self.build_request(Method::POST, path)?.json(payload);
        self.send_json(request, path)
    }

    fn build_request(
        &self,
        method: Method,
        path: &str,
    ) -> Result<RequestBuilder, ProviderLoopbackHttpError> {
        let url = self
            .base_url
            .join(path.trim_start_matches('/'))
            .map_err(|err| ProviderLoopbackHttpError::InvalidBaseUrl(err.to_string()))?;
        let mut request = self.http.request(method, url);
        if let Some(token) = &self.auth_token {
            request = request.bearer_auth(token);
        }
        Ok(request)
    }

    fn send_json<Response>(
        &self,
        request: RequestBuilder,
        path: &str,
    ) -> Result<Response, ProviderLoopbackHttpError>
    where
        Response: DeserializeOwned,
    {
        let response = request
            .send()
            .map_err(|err| ProviderLoopbackHttpError::RequestFailed {
                path: path.to_string(),
                detail: err.to_string(),
            })?;
        let status = response.status();
        let body = response
            .bytes()
            .map_err(|err| ProviderLoopbackHttpError::RequestFailed {
                path: path.to_string(),
                detail: err.to_string(),
            })?;
        if status == StatusCode::UNAUTHORIZED {
            let detail = String::from_utf8_lossy(body.as_ref()).trim().to_string();
            return Err(ProviderLoopbackHttpError::Unauthorized {
                path: path.to_string(),
                detail: if detail.is_empty() {
                    "HTTP 401".to_string()
                } else {
                    detail
                },
            });
        }
        if !status.is_success() {
            return Err(ProviderLoopbackHttpError::UnexpectedStatus {
                path: path.to_string(),
                status_code: status.as_u16(),
                body: String::from_utf8_lossy(body.as_ref()).trim().to_string(),
            });
        }
        serde_json::from_slice(body.as_ref()).map_err(|err| {
            ProviderLoopbackHttpError::DecodeFailed {
                path: path.to_string(),
                detail: err.to_string(),
            }
        })
    }
}

pub fn validate_provider_loopback_http_base_url(
    base_url: &str,
) -> Result<Url, ProviderLoopbackHttpError> {
    let trimmed = base_url.trim();
    if trimmed.is_empty() {
        return Err(ProviderLoopbackHttpError::InvalidBaseUrl(
            "base url cannot be empty".to_string(),
        ));
    }
    let url = Url::parse(trimmed)
        .map_err(|err| ProviderLoopbackHttpError::InvalidBaseUrl(err.to_string()))?;
    if url.scheme() != "http" {
        return Err(ProviderLoopbackHttpError::InvalidBaseUrl(
            "scheme must be http for localhost provider".to_string(),
        ));
    }
    let Some(host) = url.host_str() else {
        return Err(ProviderLoopbackHttpError::InvalidBaseUrl(
            "host is required".to_string(),
        ));
    };
    if !matches!(host, "127.0.0.1" | "localhost" | "::1") {
        return Err(ProviderLoopbackHttpError::InvalidBaseUrl(
            "host must be loopback (127.0.0.1 / localhost / ::1)".to_string(),
        ));
    }
    Ok(url)
}
