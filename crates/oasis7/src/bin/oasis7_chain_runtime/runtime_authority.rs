use oasis7::network_tier_manifest::LoadedNetworkTierManifest;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

/// Immutable deployment authority facts captured from the inputs used by this
/// runtime instance.  This is deliberately separate from NodeSnapshot: the
/// deployment files are startup authority, not mutable consensus state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeAuthorityBinding {
    pub(super) registry_ref: String,
    pub(super) registry_sha256: String,
    pub(super) inventory_ref: String,
    pub(super) inventory_sha256: String,
}

/// Load and validate the deployment authority consumed by a public-testnet
/// runtime.  A partial binding is rejected, as is a deployment inventory that
/// is not bound by the loaded network-tier manifest.  No repository-local
/// path, digest, or signer is synthesized here.
pub(super) fn load_runtime_authority_binding(
    registry_path: Option<&Path>,
    inventory_path: Option<&Path>,
    loaded_network_tier_manifest: Option<&LoadedNetworkTierManifest>,
) -> Result<Option<RuntimeAuthorityBinding>, String> {
    match (registry_path, inventory_path) {
        (None, None) => {
            if loaded_network_tier_manifest
                .is_some_and(|loaded| loaded.manifest.tier == "public_testnet")
            {
                return Err(
                    "public-testnet deployment authority requires --genesis-validator-registry and --deployment-inventory"
                        .to_string(),
                );
            }
            return Ok(None);
        }
        (None, Some(_)) => {
            return Err(
                "deployment inventory authority requires --genesis-validator-registry".to_string(),
            );
        }
        (Some(_), None) => {
            return Err(
                "genesis validator registry authority requires --deployment-inventory".to_string(),
            );
        }
        (Some(registry_path), Some(inventory_path)) => {
            let loaded = loaded_network_tier_manifest.ok_or_else(|| {
                "deployment authority binding requires --network-tier-manifest".to_string()
            })?;
            let manifest_path = Path::new(loaded.source_path.as_str());
            let manifest_bytes = fs::read(manifest_path).map_err(|err| {
                format!(
                    "read network-tier manifest {} for deployment authority binding failed: {err}",
                    manifest_path.display()
                )
            })?;
            let manifest: serde_json::Value = serde_json::from_slice(manifest_bytes.as_slice())
                .map_err(|err| {
                    format!(
                        "parse network-tier manifest {} for deployment authority binding failed: {err}",
                        manifest_path.display()
                    )
                })?;
            let inventory_binding = manifest
                .get("deployment_inventory")
                .and_then(serde_json::Value::as_object)
                .ok_or_else(|| {
                    "network-tier manifest is missing deployment_inventory authority binding"
                        .to_string()
                })?;
            let inventory_ref = required_string(inventory_binding, "ref", "deployment_inventory")?;
            let expected_inventory_sha256 =
                required_sha256(inventory_binding, "sha256", "deployment_inventory")?;
            let inventory_sha256 = sha256_regular_file(inventory_path, "deployment inventory")?;
            if inventory_sha256 != expected_inventory_sha256 {
                return Err(format!(
                    "deployment inventory digest mismatch: manifest={} actual={} path={}",
                    expected_inventory_sha256,
                    inventory_sha256,
                    inventory_path.display()
                ));
            }
            let registry_sha256 = sha256_regular_file(registry_path, "genesis validator registry")?;
            let registry_ref = startup_ref(registry_path, "genesis validator registry")?;
            Ok(Some(RuntimeAuthorityBinding {
                registry_ref,
                registry_sha256,
                inventory_ref,
                inventory_sha256,
            }))
        }
    }
}

fn startup_ref(path: &Path, label: &str) -> Result<String, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|err| format!("read {label} metadata {} failed: {err}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(format!(
            "{label} must be a regular non-symlink file: {}",
            path.display()
        ));
    }
    let value = path.to_string_lossy().trim().to_string();
    if value.is_empty() {
        return Err(format!("{label} reference must be non-empty"));
    }
    Ok(value)
}

fn sha256_regular_file(path: &Path, label: &str) -> Result<String, String> {
    let _ = startup_ref(path, label)?;
    let bytes = fs::read(path)
        .map_err(|err| format!("read {label} {} for sha256 failed: {err}", path.display()))?;
    Ok(hex::encode(Sha256::digest(bytes.as_slice())))
}

fn required_string(
    object: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    scope: &str,
) -> Result<String, String> {
    let value = object
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{scope}.{key} must be a non-empty string"))?;
    Ok(value.to_string())
}

fn required_sha256(
    object: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    scope: &str,
) -> Result<String, String> {
    let value = required_string(object, key, scope)?.to_ascii_lowercase();
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!(
            "{scope}.{key} must be a 64-character sha256 hex digest"
        ));
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oasis7::network_tier_manifest::{
        LoadedNetworkTierManifest, NETWORK_TIER_MANIFEST_SCHEMA_V1, NetworkTierClaimsPolicy,
        NetworkTierEndpointPolicy, NetworkTierManifest, NetworkTierPromotionPolicy,
        NetworkTierRuntimeRefs, NetworkTierTokenPolicy, NetworkTierValidatorPolicy,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_dir() -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("oasis7-runtime-authority-{suffix}"));
        fs::create_dir_all(path.as_path()).expect("create temp dir");
        path
    }

    fn loaded_manifest(path: &Path) -> LoadedNetworkTierManifest {
        LoadedNetworkTierManifest {
            source_path: path.to_string_lossy().to_string(),
            manifest: NetworkTierManifest {
                schema_version: NETWORK_TIER_MANIFEST_SCHEMA_V1.to_string(),
                tier: "public_testnet".to_string(),
                status: "rehearsal".to_string(),
                network_id: "oasis7-public-testnet-governed-20260606".to_string(),
                chain_id: "oasis7-public-testnet-governed-20260606".to_string(),
                runtime_refs: NetworkTierRuntimeRefs {
                    release_candidate_bundle_ref: "bundle.json".to_string(),
                    genesis_ref: "genesis.json".to_string(),
                    bootstrap_peer_ref: "peers.txt".to_string(),
                },
                endpoint_policy: NetworkTierEndpointPolicy {
                    rpc_ref: "http://127.0.0.1:6631".to_string(),
                    explorer_ref: "http://127.0.0.1:6632".to_string(),
                    faucet_ref: None,
                },
                validator_policy: NetworkTierValidatorPolicy {
                    governance_mode: "governance_registry".to_string(),
                    validator_admission: "allowlist_or_governed_candidate".to_string(),
                    target_validator_count: 3,
                    allow_observer_nodes: true,
                },
                token_policy: NetworkTierTokenPolicy {
                    symbol: "OC".to_string(),
                    faucet_mode: "guarded_testnet_faucet".to_string(),
                    reset_policy: "resettable".to_string(),
                    value_semantics: "testnet".to_string(),
                },
                claims_policy: NetworkTierClaimsPolicy {
                    allowed_claims: vec!["public_testnet".to_string()],
                    denied_claims: vec!["mainnet_live".to_string()],
                },
                promotion_policy: NetworkTierPromotionPolicy {
                    promote_from: vec!["shared_devnet".to_string()],
                    required_gates: vec!["runtime_bootstrap".to_string()],
                },
                evidence_refs: Vec::new(),
            },
            bootstrap_peers: Vec::new(),
        }
    }

    fn write_authority_fixture() -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
        let root = unique_temp_dir();
        let registry = root.join("registry.json");
        let inventory = root.join("inventory.json");
        fs::write(
            registry.as_path(),
            b"{\"validators\":[{\"node_id\":\"validator-47\"}]}\n",
        )
        .expect("write registry");
        fs::write(
            inventory.as_path(),
            b"{\"schema_version\":\"oasis7.public_testnet_validator_triad_inventory.v1\"}\n",
        )
        .expect("write inventory");
        let inventory_sha256 = hex::encode(Sha256::digest(
            fs::read(inventory.as_path())
                .expect("read inventory")
                .as_slice(),
        ));
        let manifest_path = root.join("manifest.json");
        fs::write(
            manifest_path.as_path(),
            format!(
                "{{\"deployment_inventory\":{{\"ref\":\"scripts/public-testnet-validator-triad-inventory.v1.json\",\"sha256\":\"{inventory_sha256}\"}}}}\n"
            ),
        )
        .expect("write manifest");
        (registry, inventory, manifest_path)
    }

    #[test]
    fn captures_actual_refs_and_file_digests() {
        let (registry, inventory, manifest_path) = write_authority_fixture();
        let binding = load_runtime_authority_binding(
            Some(registry.as_path()),
            Some(inventory.as_path()),
            Some(&loaded_manifest(manifest_path.as_path())),
        )
        .expect("authority binding")
        .expect("binding present");
        assert_eq!(binding.registry_ref, registry.to_string_lossy());
        assert_eq!(
            binding.inventory_ref,
            "scripts/public-testnet-validator-triad-inventory.v1.json"
        );
        assert_eq!(
            binding.registry_sha256,
            hex::encode(Sha256::digest(
                fs::read(registry.as_path())
                    .expect("read registry")
                    .as_slice(),
            ))
        );
        assert_eq!(
            binding.inventory_sha256,
            hex::encode(Sha256::digest(
                fs::read(inventory.as_path())
                    .expect("read inventory")
                    .as_slice(),
            ))
        );
    }

    #[test]
    fn rejects_partial_authority_inputs() {
        let error = load_runtime_authority_binding(Some(Path::new("registry.json")), None, None)
            .expect_err("partial binding must fail closed");
        assert!(error.contains("--deployment-inventory"));
    }

    #[test]
    fn rejects_public_testnet_without_authority_inputs() {
        let error = load_runtime_authority_binding(
            None,
            None,
            Some(&loaded_manifest(Path::new("missing-manifest.json"))),
        )
        .expect_err("public testnet authority must fail closed");
        assert!(error.contains("public-testnet deployment authority"));
    }

    #[test]
    fn rejects_manifest_inventory_digest_drift() {
        let (registry, inventory, manifest_path) = write_authority_fixture();
        let manifest = fs::read_to_string(manifest_path.as_path()).expect("read manifest");
        fs::write(
            manifest_path.as_path(),
            manifest.replace(
                &hex::encode(Sha256::digest(
                    fs::read(inventory.as_path())
                        .expect("read inventory")
                        .as_slice(),
                )),
                &"0".repeat(64),
            ),
        )
        .expect("rewrite manifest");
        let error = load_runtime_authority_binding(
            Some(registry.as_path()),
            Some(inventory.as_path()),
            Some(&loaded_manifest(manifest_path.as_path())),
        )
        .expect_err("digest drift must fail closed");
        assert!(error.contains("digest mismatch"));
    }
}
