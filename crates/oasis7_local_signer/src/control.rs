use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::SignerError;
use crate::identity::{canonical_json, parse_sha256_hex, sha256};
use crate::types::{BatchGrant, InstallationConfig, Policy};

pub(crate) const MAINTENANCE_FILE: &str = "control/maintenance.json";
pub(crate) const MAINTENANCE_SCHEMA: &str = "oasis7.local_signer_maintenance.v1";

/// Durable admin transaction state shared with the signing worker. `prepared`
/// blocks signing; `committed` is written only after the target is durable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MaintenanceRecord {
    pub schema_version: String,
    pub installation_id: String,
    pub transaction_id: String,
    pub phase: String,
    pub operation_key: String,
    pub target_path: String,
    pub target_sha256: String,
    pub base_sha256: Option<String>,
    pub timestamp_ms: Option<u64>,
}

pub fn parse_policy_candidate(
    bytes: &[u8],
    expected_sha256: &str,
    installation: &InstallationConfig,
) -> Result<Policy, SignerError> {
    let policy: Policy = parse_candidate(bytes, expected_sha256)?;
    policy
        .validate(installation)
        .map_err(SignerError::InvalidInput)?;
    Ok(policy)
}

pub fn parse_grant_candidate(
    bytes: &[u8],
    expected_sha256: &str,
    installation: &InstallationConfig,
    policy: &Policy,
) -> Result<BatchGrant, SignerError> {
    let grant: BatchGrant = parse_candidate(bytes, expected_sha256)?;
    grant
        .validate(installation, policy)
        .map_err(SignerError::InvalidInput)?;
    Ok(grant)
}

pub fn parse_candidate<T: DeserializeOwned + Serialize>(
    bytes: &[u8],
    expected_sha256: &str,
) -> Result<T, SignerError> {
    let expected = parse_sha256_hex(expected_sha256)?;
    if sha256(bytes) != expected {
        return Err(SignerError::AuthorizationDenied);
    }
    let value: T = serde_json::from_slice(bytes)
        .map_err(|_| SignerError::InvalidInput("candidate JSON schema is invalid".to_owned()))?;
    let canonical = canonical_json(&value)?;
    if canonical != bytes {
        return Err(SignerError::InvalidInput(
            "candidate must be compact canonical UTF-8 JSON".to_owned(),
        ));
    }
    Ok(value)
}
