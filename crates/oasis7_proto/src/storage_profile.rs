use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

const STORAGE_PROFILE_ALLOWED_VALUES: &str = "dev_local, release_default, soak_forensics";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageProfile {
    DevLocal,
    ReleaseDefault,
    SoakForensics,
}

impl StorageProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DevLocal => "dev_local",
            Self::ReleaseDefault => "release_default",
            Self::SoakForensics => "soak_forensics",
        }
    }

    pub fn allowed_values() -> &'static str {
        STORAGE_PROFILE_ALLOWED_VALUES
    }
}

impl fmt::Display for StorageProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for StorageProfile {
    type Err = String;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "dev_local" => Ok(Self::DevLocal),
            "release_default" => Ok(Self::ReleaseDefault),
            "soak_forensics" => Ok(Self::SoakForensics),
            _ => Err(format!(
                "storage profile must be one of: {}",
                Self::allowed_values()
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageProfileConfig {
    pub profile: StorageProfile,
    pub execution_hot_head_heights: u64,
    pub execution_checkpoint_interval: u64,
    pub execution_checkpoint_keep: u64,
    pub execution_sidecar_generations_keep: u32,
    pub tick_consensus_hot_limit: usize,
    pub tick_consensus_archive_segment_len: usize,
    pub replication_max_hot_commit_messages: usize,
    pub metrics_emit_interval_ms: u64,
}

impl StorageProfileConfig {
    pub fn for_profile(profile: StorageProfile) -> Self {
        match profile {
            StorageProfile::DevLocal => Self {
                profile,
                execution_hot_head_heights: 32,
                execution_checkpoint_interval: 32,
                execution_checkpoint_keep: 4,
                execution_sidecar_generations_keep: 2,
                tick_consensus_hot_limit: 128,
                tick_consensus_archive_segment_len: 64,
                replication_max_hot_commit_messages: 4_096,
                metrics_emit_interval_ms: 30_000,
            },
            StorageProfile::ReleaseDefault => Self {
                profile,
                execution_hot_head_heights: 64,
                execution_checkpoint_interval: 64,
                execution_checkpoint_keep: 8,
                execution_sidecar_generations_keep: 4,
                tick_consensus_hot_limit: 256,
                tick_consensus_archive_segment_len: 128,
                replication_max_hot_commit_messages: 8_192,
                metrics_emit_interval_ms: 15_000,
            },
            StorageProfile::SoakForensics => Self {
                profile,
                execution_hot_head_heights: 512,
                execution_checkpoint_interval: 16,
                execution_checkpoint_keep: 32,
                execution_sidecar_generations_keep: 8,
                tick_consensus_hot_limit: 1_024,
                tick_consensus_archive_segment_len: 256,
                replication_max_hot_commit_messages: 16_384,
                metrics_emit_interval_ms: 5_000,
            },
        }
    }
}

impl Default for StorageProfileConfig {
    fn default() -> Self {
        Self::for_profile(StorageProfile::DevLocal)
    }
}

impl From<StorageProfile> for StorageProfileConfig {
    fn from(profile: StorageProfile) -> Self {
        Self::for_profile(profile)
    }
}

#[cfg(test)]
mod tests {
    use super::{StorageProfile, StorageProfileConfig};

    #[test]
    fn storage_profile_round_trips_as_strings() {
        for (raw, expected) in [
            ("dev_local", StorageProfile::DevLocal),
            ("release_default", StorageProfile::ReleaseDefault),
            ("soak_forensics", StorageProfile::SoakForensics),
        ] {
            let parsed = raw
                .parse::<StorageProfile>()
                .expect("parse storage profile");
            assert_eq!(parsed, expected);
            assert_eq!(parsed.as_str(), raw);
        }
    }

    #[test]
    fn storage_profile_rejects_unknown_values() {
        let err = "unknown"
            .parse::<StorageProfile>()
            .expect_err("should fail");
        assert!(err.contains("dev_local"));
        assert!(err.contains("release_default"));
        assert!(err.contains("soak_forensics"));
    }

    #[test]
    fn storage_profile_default_matches_dev_local_defaults() {
        assert_eq!(
            StorageProfileConfig::default(),
            StorageProfileConfig::for_profile(StorageProfile::DevLocal)
        );
    }

    #[test]
    fn storage_profiles_scale_retention_density() {
        let dev_local = StorageProfileConfig::for_profile(StorageProfile::DevLocal);
        let release_default = StorageProfileConfig::for_profile(StorageProfile::ReleaseDefault);
        let soak_forensics = StorageProfileConfig::for_profile(StorageProfile::SoakForensics);

        assert!(release_default.execution_hot_head_heights > dev_local.execution_hot_head_heights);
        assert!(
            soak_forensics.execution_hot_head_heights > release_default.execution_hot_head_heights
        );
        assert!(
            release_default.replication_max_hot_commit_messages
                > dev_local.replication_max_hot_commit_messages
        );
        assert!(
            soak_forensics.replication_max_hot_commit_messages
                > release_default.replication_max_hot_commit_messages
        );
        assert!(
            soak_forensics.execution_checkpoint_interval
                < release_default.execution_checkpoint_interval
        );
        assert_eq!(release_default.execution_hot_head_heights, 64);
        assert_eq!(
            release_default.execution_hot_head_heights,
            release_default.execution_checkpoint_interval
        );
    }
}
