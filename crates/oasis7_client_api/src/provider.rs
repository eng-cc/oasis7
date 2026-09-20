use serde::{Deserialize, Serialize};

pub const PROVIDER_PHASE1_ACTION_SET_ALIAS: &str = "phase1_low_frequency";
pub const PROVIDER_WORLD_RESOURCE_MANIFEST_SCHEMA_V1: &str = "oasis7.world_resource_manifest.v1";
pub const PROVIDER_WORLD_RESOURCE_DELTA_SCHEMA_V1: &str = "oasis7.world_resource_delta.v1";
pub const PROVIDER_TRANSPORT_LOOPBACK_HTTP: &str = "loopback_http";
pub const PROVIDER_TRANSPORT_REMOTE_HTTPS: &str = "remote_https";

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain_resource_manifest_schema_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain_resource_delta_schema_version: Option<String>,
}

impl ProviderInfo {
    pub fn resolved_provider_id(&self) -> &str {
        if self.provider_id.trim().is_empty() {
            "provider_loopback_http"
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resource_schema_errors: Vec<String>,
}

pub fn provider_phase1_required_capabilities() -> &'static [&'static str] {
    PROVIDER_PHASE1_REQUIRED_CAPABILITIES
}

pub fn provider_phase1_required_actions() -> &'static [&'static str] {
    PROVIDER_PHASE1_REQUIRED_ACTIONS
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

/// Transport-independent probing boundary. Implementations belong to the
/// caller/runtime and may use HTTP, an in-process bridge, or another adapter.
pub trait ProviderProbe {
    type Error;

    fn provider_info(&self) -> Result<ProviderInfo, Self::Error>;
    fn provider_health(&self) -> Result<ProviderHealth, Self::Error>;
}

/// A stable error envelope for adapters that need a small public error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    InvalidConfig(String),
    Unauthorized(String),
    Unreachable(String),
    DecodeFailed(String),
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig(detail)
            | Self::Unauthorized(detail)
            | Self::Unreachable(detail)
            | Self::DecodeFailed(detail) => f.write_str(detail),
        }
    }
}

impl std::error::Error for ProviderError {}

pub fn evaluate_provider_compatibility(
    info: &ProviderInfo,
    health: Option<&ProviderHealth>,
) -> ProviderCompatibilityReport {
    let resource_schema_errors = provider_resource_schema_errors(info);
    if !resource_schema_errors.is_empty() {
        return ProviderCompatibilityReport {
            status: ProviderCompatibilityStatus::Incompatible,
            fallback_reason: Some(format!(
                "unsupported_world_resource_schema:{}",
                resource_schema_errors.join(",")
            )),
            missing_capabilities: Vec::new(),
            missing_supported_actions: Vec::new(),
            resource_schema_errors,
        };
    }

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
            resource_schema_errors: Vec::new(),
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
            resource_schema_errors: Vec::new(),
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
    if health.ok && matches!(lowered_status.as_str(), "ok" | "ready") && !has_last_error {
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
        resource_schema_errors: Vec::new(),
    }
}

fn provider_resource_schema_errors(info: &ProviderInfo) -> Vec<String> {
    let mut errors = Vec::new();
    match info.chain_resource_manifest_schema_version.as_deref() {
        Some(PROVIDER_WORLD_RESOURCE_MANIFEST_SCHEMA_V1) => {}
        Some(value) if !value.trim().is_empty() => {
            errors.push(format!("world_resource_manifest_schema={value}"))
        }
        _ => errors.push("missing_world_resource_manifest_schema".to_string()),
    }
    match info.chain_resource_delta_schema_version.as_deref() {
        Some(PROVIDER_WORLD_RESOURCE_DELTA_SCHEMA_V1) => {}
        Some(value) if !value.trim().is_empty() => {
            errors.push(format!("world_resource_delta_schema={value}"))
        }
        _ => errors.push("missing_world_resource_delta_schema".to_string()),
    }
    errors
}

fn contains_trimmed_value(values: &[String], expected: &str) -> bool {
    values
        .iter()
        .any(|value| value.trim().eq_ignore_ascii_case(expected))
}
