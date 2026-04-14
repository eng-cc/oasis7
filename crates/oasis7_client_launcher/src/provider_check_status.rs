use eframe::egui;
#[cfg(not(target_arch = "wasm32"))]
use oasis7::simulator::ProviderCompatibilityStatus as SimulatorProviderCompatibilityStatus;
use serde::{Deserialize, Serialize};

use crate::UiLanguage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProviderCompatibilityStatus {
    #[default]
    Ready,
    Degraded,
    Incompatible,
}

impl ProviderCompatibilityStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Degraded => "degraded",
            Self::Incompatible => "incompatible",
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<SimulatorProviderCompatibilityStatus> for ProviderCompatibilityStatus {
    fn from(value: SimulatorProviderCompatibilityStatus) -> Self {
        match value {
            SimulatorProviderCompatibilityStatus::Ready => Self::Ready,
            SimulatorProviderCompatibilityStatus::Degraded => Self::Degraded,
            SimulatorProviderCompatibilityStatus::Incompatible => Self::Incompatible,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderSnapshot {
    pub(crate) provider_id: String,
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) protocol_version: String,
    pub(crate) capabilities: Vec<String>,
    pub(crate) supported_action_sets: Vec<String>,
    pub(crate) compatibility_status: ProviderCompatibilityStatus,
    pub(crate) status: String,
    pub(crate) queue_depth: Option<u64>,
    pub(crate) last_error: Option<String>,
    pub(crate) fallback_reason: Option<String>,
    pub(crate) info_latency_ms: u64,
    pub(crate) health_latency_ms: u64,
    pub(crate) total_latency_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum ProviderCheckStatus {
    Disabled,
    Idle,
    Checking,
    Ready(ProviderSnapshot),
    Degraded(ProviderSnapshot),
    Incompatible(ProviderSnapshot),
    Unsupported(String),
    InvalidConfig(String),
    Unreachable(String),
    Unauthorized(String),
}

impl ProviderCheckStatus {
    pub(crate) fn text(&self, language: UiLanguage) -> String {
        match (self, language) {
            (Self::Disabled, UiLanguage::ZhCn) => "未启用".to_string(),
            (Self::Disabled, UiLanguage::EnUs) => "Disabled".to_string(),
            (Self::Idle, UiLanguage::ZhCn) => "待检查".to_string(),
            (Self::Idle, UiLanguage::EnUs) => "Idle".to_string(),
            (Self::Checking, UiLanguage::ZhCn) => "检查中".to_string(),
            (Self::Checking, UiLanguage::EnUs) => "Checking".to_string(),
            (Self::Ready(_), UiLanguage::ZhCn) => "已就绪".to_string(),
            (Self::Ready(_), UiLanguage::EnUs) => "Ready".to_string(),
            (Self::Degraded(_), UiLanguage::ZhCn) => "已降级".to_string(),
            (Self::Degraded(_), UiLanguage::EnUs) => "Degraded".to_string(),
            (Self::Incompatible(_), UiLanguage::ZhCn) => "不兼容".to_string(),
            (Self::Incompatible(_), UiLanguage::EnUs) => "Incompatible".to_string(),
            (Self::Unsupported(_), UiLanguage::ZhCn) => "当前端不支持".to_string(),
            (Self::Unsupported(_), UiLanguage::EnUs) => "Unsupported".to_string(),
            (Self::InvalidConfig(_), UiLanguage::ZhCn) => "配置错误".to_string(),
            (Self::InvalidConfig(_), UiLanguage::EnUs) => "Invalid Config".to_string(),
            (Self::Unreachable(_), UiLanguage::ZhCn) => "不可达".to_string(),
            (Self::Unreachable(_), UiLanguage::EnUs) => "Unreachable".to_string(),
            (Self::Unauthorized(_), UiLanguage::ZhCn) => "认证失败".to_string(),
            (Self::Unauthorized(_), UiLanguage::EnUs) => "Unauthorized".to_string(),
        }
    }

    pub(crate) fn color(&self) -> egui::Color32 {
        match self {
            Self::Disabled | Self::Idle => egui::Color32::from_rgb(130, 130, 130),
            Self::Checking => egui::Color32::from_rgb(201, 146, 44),
            Self::Ready(_) => egui::Color32::from_rgb(62, 152, 92),
            Self::Degraded(_) => egui::Color32::from_rgb(201, 146, 44),
            Self::Incompatible(_) => egui::Color32::from_rgb(196, 84, 84),
            Self::Unsupported(_) => egui::Color32::from_rgb(130, 130, 130),
            Self::InvalidConfig(_) | Self::Unreachable(_) | Self::Unauthorized(_) => {
                egui::Color32::from_rgb(196, 84, 84)
            }
        }
    }

    pub(crate) fn detail(&self) -> Option<String> {
        match self {
            Self::Ready(snapshot) | Self::Degraded(snapshot) | Self::Incompatible(snapshot) => Some(format!(
                "provider_id={} name={} version={} protocol={} compatibility_status={} status={} queue_depth={} capabilities={} supported_action_sets={} check_latency_ms={{info:{}, health:{}, total:{}}} last_error={} fallback_reason={}",
                snapshot.provider_id,
                snapshot.name,
                snapshot.version,
                snapshot.protocol_version,
                snapshot.compatibility_status.as_str(),
                snapshot.status,
                snapshot.queue_depth.map(|value| value.to_string()).unwrap_or_else(|| "n/a".to_string()),
                if snapshot.capabilities.is_empty() { "none".to_string() } else { snapshot.capabilities.join(",") },
                if snapshot.supported_action_sets.is_empty() { "none".to_string() } else { snapshot.supported_action_sets.join(",") },
                snapshot.info_latency_ms,
                snapshot.health_latency_ms,
                snapshot.total_latency_ms,
                snapshot.last_error.as_deref().unwrap_or("none"),
                snapshot.fallback_reason.as_deref().unwrap_or("none")
            )),
            Self::Unsupported(detail)
            | Self::InvalidConfig(detail)
            | Self::Unreachable(detail)
            | Self::Unauthorized(detail) => Some(detail.clone()),
            Self::Disabled | Self::Idle | Self::Checking => None,
        }
    }
}
