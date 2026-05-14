use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const NETWORK_TIER_MANIFEST_SCHEMA_V1: &str = "oasis7.network_tier_manifest.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkTierManifest {
    pub schema_version: String,
    pub tier: String,
    pub status: String,
    pub network_id: String,
    pub chain_id: String,
    pub runtime_refs: NetworkTierRuntimeRefs,
    pub endpoint_policy: NetworkTierEndpointPolicy,
    pub validator_policy: NetworkTierValidatorPolicy,
    pub token_policy: NetworkTierTokenPolicy,
    pub claims_policy: NetworkTierClaimsPolicy,
    pub promotion_policy: NetworkTierPromotionPolicy,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkTierRuntimeRefs {
    pub release_candidate_bundle_ref: String,
    pub genesis_ref: String,
    pub bootstrap_peer_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkTierEndpointPolicy {
    pub rpc_ref: String,
    pub explorer_ref: String,
    pub faucet_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkTierValidatorPolicy {
    pub governance_mode: String,
    pub validator_admission: String,
    pub target_validator_count: u64,
    pub allow_observer_nodes: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkTierTokenPolicy {
    pub symbol: String,
    pub faucet_mode: String,
    pub reset_policy: String,
    pub value_semantics: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkTierClaimsPolicy {
    #[serde(default)]
    pub allowed_claims: Vec<String>,
    #[serde(default)]
    pub denied_claims: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkTierPromotionPolicy {
    #[serde(default)]
    pub promote_from: Vec<String>,
    #[serde(default)]
    pub required_gates: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LoadedNetworkTierManifest {
    pub source_path: String,
    pub manifest: NetworkTierManifest,
    pub bootstrap_peers: Vec<String>,
}

impl LoadedNetworkTierManifest {
    pub fn load(path: &Path) -> Result<Self, String> {
        let source_path = path
            .canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .to_string();
        let raw = fs::read_to_string(path).map_err(|err| {
            format!(
                "read network tier manifest {} failed: {err}",
                path.display()
            )
        })?;
        let manifest: NetworkTierManifest = serde_json::from_str(raw.as_str()).map_err(|err| {
            format!(
                "parse network tier manifest {} failed: {err}",
                path.display()
            )
        })?;
        validate_manifest(&manifest, path)?;
        let bootstrap_path =
            resolve_manifest_relative_path(path, manifest.runtime_refs.bootstrap_peer_ref.as_str());
        let bootstrap_peers = load_bootstrap_peers(bootstrap_path.as_path())?;
        Ok(Self {
            source_path,
            manifest,
            bootstrap_peers,
        })
    }
}

fn validate_manifest(manifest: &NetworkTierManifest, path: &Path) -> Result<(), String> {
    if manifest.schema_version != NETWORK_TIER_MANIFEST_SCHEMA_V1 {
        return Err(format!(
            "network tier manifest {} has unsupported schema_version `{}`",
            path.display(),
            manifest.schema_version
        ));
    }
    validate_choice(
        manifest.tier.as_str(),
        &["local_devnet", "shared_devnet", "public_testnet", "mainnet"],
        "tier",
        path,
    )?;
    validate_choice(
        manifest.status.as_str(),
        &["planned", "specified_skeleton_only", "rehearsal", "live"],
        "status",
        path,
    )?;
    validate_non_empty(manifest.network_id.as_str(), "network_id", path)?;
    validate_non_empty(manifest.chain_id.as_str(), "chain_id", path)?;
    validate_non_empty(
        manifest.runtime_refs.release_candidate_bundle_ref.as_str(),
        "runtime_refs.release_candidate_bundle_ref",
        path,
    )?;
    validate_non_empty(
        manifest.runtime_refs.genesis_ref.as_str(),
        "runtime_refs.genesis_ref",
        path,
    )?;
    validate_non_empty(
        manifest.runtime_refs.bootstrap_peer_ref.as_str(),
        "runtime_refs.bootstrap_peer_ref",
        path,
    )?;
    validate_non_empty(
        manifest.endpoint_policy.rpc_ref.as_str(),
        "endpoint_policy.rpc_ref",
        path,
    )?;
    validate_non_empty(
        manifest.endpoint_policy.explorer_ref.as_str(),
        "endpoint_policy.explorer_ref",
        path,
    )?;
    if let Some(faucet_ref) = manifest.endpoint_policy.faucet_ref.as_deref() {
        validate_non_empty(faucet_ref, "endpoint_policy.faucet_ref", path)?;
    }
    validate_choice(
        manifest.validator_policy.governance_mode.as_str(),
        &["bootstrap_local", "shared_ops", "governance_registry"],
        "validator_policy.governance_mode",
        path,
    )?;
    validate_choice(
        manifest.validator_policy.validator_admission.as_str(),
        &[
            "local_only",
            "shared_allowlist",
            "allowlist_or_governed_candidate",
            "governance_registry_only",
        ],
        "validator_policy.validator_admission",
        path,
    )?;
    if manifest.validator_policy.target_validator_count == 0 {
        return Err(format!(
            "network tier manifest {} requires validator_policy.target_validator_count > 0",
            path.display()
        ));
    }
    validate_non_empty(
        manifest.token_policy.symbol.as_str(),
        "token_policy.symbol",
        path,
    )?;
    validate_choice(
        manifest.token_policy.faucet_mode.as_str(),
        &["none", "operator_grant", "guarded_testnet_faucet"],
        "token_policy.faucet_mode",
        path,
    )?;
    validate_choice(
        manifest.token_policy.reset_policy.as_str(),
        &["ephemeral", "resettable", "frozen"],
        "token_policy.reset_policy",
        path,
    )?;
    validate_choice(
        manifest.token_policy.value_semantics.as_str(),
        &["preview", "testnet", "production"],
        "token_policy.value_semantics",
        path,
    )?;
    Ok(())
}

fn validate_non_empty(raw: &str, field: &str, path: &Path) -> Result<(), String> {
    if raw.trim().is_empty() {
        Err(format!(
            "network tier manifest {} requires non-empty {}",
            path.display(),
            field
        ))
    } else {
        Ok(())
    }
}

fn validate_choice(raw: &str, choices: &[&str], field: &str, path: &Path) -> Result<(), String> {
    if choices.iter().any(|choice| raw == *choice) {
        Ok(())
    } else {
        Err(format!(
            "network tier manifest {} has invalid {} `{}`; expected one of: {}",
            path.display(),
            field,
            raw,
            choices.join(", ")
        ))
    }
}

fn resolve_manifest_relative_path(manifest_path: &Path, raw: &str) -> PathBuf {
    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() {
        return candidate;
    }
    if let Some(parent) = manifest_path.parent() {
        let manifest_relative = parent.join(&candidate);
        if manifest_relative.exists() {
            return manifest_relative;
        }
    }
    candidate
}

fn load_bootstrap_peers(path: &Path) -> Result<Vec<String>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("read bootstrap peer ref {} failed: {err}", path.display()))?;
    let mut peers = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        peers.push(trimmed.to_string());
    }
    if peers.is_empty() {
        return Err(format!(
            "bootstrap peer ref {} does not contain any peers",
            path.display()
        ));
    }
    Ok(peers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("oasis7-network-tier-{label}-{nonce}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn load_manifest_reads_bootstrap_peer_file() {
        let dir = temp_dir("load");
        let peers_path = dir.join("bootstrap.txt");
        fs::write(
            &peers_path,
            "# comment\n/ip4/127.0.0.1/tcp/4100\n/dns4/bootstrap.example/tcp/4101\n",
        )
        .expect("write peers");
        let manifest_path = dir.join("manifest.json");
        fs::write(
            &manifest_path,
            format!(
                r#"{{
  "schema_version": "{NETWORK_TIER_MANIFEST_SCHEMA_V1}",
  "tier": "public_testnet",
  "status": "rehearsal",
  "network_id": "oasis7-public-testnet",
  "chain_id": "oasis7-public-testnet",
  "runtime_refs": {{
    "release_candidate_bundle_ref": "output/release-candidates/public-testnet.json",
    "genesis_ref": "doc/testing/templates/public-testnet-genesis.example.json",
    "bootstrap_peer_ref": "{}"
  }},
  "endpoint_policy": {{
    "rpc_ref": "https://public-testnet.example.invalid/rpc",
    "explorer_ref": "https://public-testnet.example.invalid/explorer",
    "faucet_ref": "https://public-testnet.example.invalid/faucet"
  }},
  "validator_policy": {{
    "governance_mode": "shared_ops",
    "validator_admission": "allowlist_or_governed_candidate",
    "target_validator_count": 4,
    "allow_observer_nodes": true
  }},
  "token_policy": {{
    "symbol": "OC",
    "faucet_mode": "guarded_testnet_faucet",
    "reset_policy": "resettable",
    "value_semantics": "testnet"
  }},
  "claims_policy": {{
    "allowed_claims": ["public_testnet"],
    "denied_claims": ["mainnet_live"]
  }},
  "promotion_policy": {{
    "promote_from": ["shared_devnet"],
    "required_gates": ["public_rpc_ready"]
  }},
  "evidence_refs": ["doc/testing/evidence/public-testnet.md"]
}}"#,
                peers_path.display()
            ),
        )
        .expect("write manifest");

        let loaded = LoadedNetworkTierManifest::load(manifest_path.as_path()).expect("load");
        assert_eq!(loaded.manifest.tier, "public_testnet");
        assert_eq!(
            loaded.bootstrap_peers,
            vec![
                "/ip4/127.0.0.1/tcp/4100".to_string(),
                "/dns4/bootstrap.example/tcp/4101".to_string()
            ]
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn load_manifest_rejects_unknown_tier() {
        let dir = temp_dir("invalid");
        let peers_path = dir.join("bootstrap.txt");
        fs::write(&peers_path, "/ip4/127.0.0.1/tcp/4100\n").expect("write peers");
        let manifest_path = dir.join("manifest.json");
        fs::write(
            &manifest_path,
            format!(
                r#"{{
  "schema_version": "{NETWORK_TIER_MANIFEST_SCHEMA_V1}",
  "tier": "wrong",
  "status": "planned",
  "network_id": "oasis7-public-testnet",
  "chain_id": "oasis7-public-testnet",
  "runtime_refs": {{
    "release_candidate_bundle_ref": "a",
    "genesis_ref": "b",
    "bootstrap_peer_ref": "{}"
  }},
  "endpoint_policy": {{
    "rpc_ref": "https://public-testnet.example.invalid/rpc",
    "explorer_ref": "https://public-testnet.example.invalid/explorer",
    "faucet_ref": null
  }},
  "validator_policy": {{
    "governance_mode": "shared_ops",
    "validator_admission": "shared_allowlist",
    "target_validator_count": 3,
    "allow_observer_nodes": true
  }},
  "token_policy": {{
    "symbol": "OC",
    "faucet_mode": "operator_grant",
    "reset_policy": "resettable",
    "value_semantics": "preview"
  }},
  "claims_policy": {{
    "allowed_claims": [],
    "denied_claims": []
  }},
  "promotion_policy": {{
    "promote_from": [],
    "required_gates": []
  }},
  "evidence_refs": []
}}"#,
                peers_path.display()
            ),
        )
        .expect("write manifest");

        let err = LoadedNetworkTierManifest::load(manifest_path.as_path()).expect_err("reject");
        assert!(err.contains("invalid tier"));

        let _ = fs::remove_dir_all(dir);
    }
}
