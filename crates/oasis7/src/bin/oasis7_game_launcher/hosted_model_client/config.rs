//! Deployment-owned, non-secret opt-in configuration.
use oasis7_model_provider::{connection::Limits, preset::ProviderPreset};
use serde::Deserialize;
use std::{fs, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Config {
    pub enabled: bool,
    pub public_origin: String,
    pub display_name: String,
    pub location: String,
    /// Operator confirms request-body/APM/session replay/core dumps are disabled
    /// for this credential receiver; this is not a public support attestation.
    pub memory_only_controls_confirmed: bool,
    pub limits: Limits,
    pub presets: Vec<ProviderPreset>,
}
impl Config {
    pub fn from_env() -> Result<Option<Self>, String> {
        let Some(path) = std::env::var_os("OASIS7_AGENT_CLIENT_CONFIG") else {
            return Ok(None);
        };
        let path = Path::new(&path);
        if !path.is_absolute() {
            return Err("agent client configuration requires an absolute path".into());
        }
        let content =
            fs::read_to_string(path).map_err(|_| "agent client configuration unavailable")?;
        if content.len() > 64 * 1024 {
            return Err("agent client configuration exceeds limit".into());
        }
        let config: Self =
            toml::from_str(&content).map_err(|_| "invalid agent client configuration")?;
        if !config.enabled {
            return Ok(None);
        }
        config.validate()?;
        Ok(Some(config))
    }
    pub fn validate(&self) -> Result<(), String> {
        let origin =
            url::Url::parse(&self.public_origin).map_err(|_| "invalid agent client origin")?;
        if !self.memory_only_controls_confirmed
            || origin.scheme() != "https"
            || origin.host_str().is_none()
            || !origin.username().is_empty()
            || origin.password().is_some()
            || origin.origin().ascii_serialization() != self.public_origin
            || self.display_name.trim().is_empty()
            || self.display_name.len() > 128
            || self.location.trim().is_empty()
            || self.location.len() > 256
            || self.presets.is_empty()
            || self.presets.len() > 16
            || self.presets.iter().any(|preset| preset.validate().is_err())
        {
            return Err("invalid trusted agent client deployment".into());
        }
        Ok(())
    }
}
