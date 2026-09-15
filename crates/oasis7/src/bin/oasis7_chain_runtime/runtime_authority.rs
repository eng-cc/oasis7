use oasis7::network_tier_manifest::LoadedNetworkTierManifest;
use oasis7::runtime::GovernanceFinalitySignerRegistry;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

/// Immutable deployment authority facts captured from the inputs used by this
/// runtime instance.  This is deliberately separate from NodeSnapshot: the
/// deployment files are startup authority, not mutable consensus state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RuntimeAuthorityBinding {
    pub(super) registry_ref: String,
    pub(super) registry_sha256: String,
    pub(super) registry_semantic_sha256: String,
    pub(super) inventory_ref: String,
    pub(super) inventory_sha256: String,
    pub(super) validator_signer_public_keys: BTreeMap<String, String>,
    pub(super) validator_stakes: BTreeMap<String, u64>,
    pub(super) validator_47_provider_peer_id: Option<String>,
}

/// Load and validate the deployment authority consumed by a public-testnet
/// runtime.  A partial binding is rejected, as is a deployment inventory that
/// is not bound by the loaded network-tier manifest.  No repository-local
/// path, digest, or signer is synthesized here.
pub(super) fn load_runtime_authority_binding(
    execution_world_dir: &Path,
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
            let inventory_bytes = fs::read(inventory_path).map_err(|err| {
                format!(
                    "read deployment inventory {} failed: {err}",
                    inventory_path.display()
                )
            })?;
            let inventory: serde_json::Value = serde_json::from_slice(inventory_bytes.as_slice())
                .map_err(|err| {
                format!(
                    "parse deployment inventory {} failed: {err}",
                    inventory_path.display()
                )
            })?;
            let inventory_authority = validate_inventory_against_manifest(
                &inventory,
                loaded,
                inventory_ref.as_str(),
                registry_path,
            )?;
            let registry_sha256 = sha256_regular_file(registry_path, "genesis validator registry")?;
            if registry_sha256 != inventory_authority.generated_registry_sha256 {
                return Err(format!(
                    "generated validator registry digest mismatch: inventory={} actual={} path={}",
                    inventory_authority.generated_registry_sha256,
                    registry_sha256,
                    registry_path.display()
                ));
            }
            let explicit_registry =
                super::governance_registry::load_genesis_finality_registry(registry_path)?;
            validate_registry_against_inventory(&explicit_registry, &inventory_authority)?;
            let world = super::execution_bridge::load_execution_world(execution_world_dir)?;
            let effective_registry = world
                .resolve_governance_effective_finality_signer_registry()
                .map_err(|err| {
                    format!(
                        "failed to resolve effective governance registry for authority preflight: {err:?}"
                    )
                })?;
            let registry_for_status = effective_registry.as_ref().unwrap_or(&explicit_registry);
            if effective_registry
                .as_ref()
                .is_some_and(|effective| effective != &explicit_registry)
            {
                return Err(
                        "explicit genesis validator registry does not match persisted effective governance registry"
                        .to_string(),
                    );
            }
            let registry_ref = startup_ref(registry_path, "genesis validator registry")?;
            let registry_semantic_sha256 = semantic_registry_sha256(registry_for_status)?;
            if registry_semantic_sha256 != inventory_authority.generated_registry_semantic_sha256 {
                return Err(format!(
                    "generated validator registry semantic digest mismatch: inventory={} actual={}",
                    inventory_authority.generated_registry_semantic_sha256,
                    registry_semantic_sha256
                ));
            }
            Ok(Some(RuntimeAuthorityBinding {
                registry_ref,
                registry_sha256,
                registry_semantic_sha256,
                inventory_ref,
                inventory_sha256,
                validator_signer_public_keys: inventory_authority.validator_signer_public_keys,
                validator_stakes: inventory_authority.validator_stakes,
                validator_47_provider_peer_id: inventory_authority.validator_47_provider_peer_id,
            }))
        }
    }
}

#[derive(Debug)]
struct InventoryAuthority {
    validator_signer_public_keys: BTreeMap<String, String>,
    validator_stakes: BTreeMap<String, u64>,
    validator_47_provider_peer_id: Option<String>,
    generated_registry_sha256: String,
    generated_registry_semantic_sha256: String,
}

fn validate_inventory_against_manifest(
    inventory: &serde_json::Value,
    loaded: &LoadedNetworkTierManifest,
    expected_inventory_ref: &str,
    registry_path: &Path,
) -> Result<InventoryAuthority, String> {
    let object = inventory
        .as_object()
        .ok_or_else(|| "deployment inventory must be a JSON object".to_string())?;
    if object
        .get("schema_version")
        .and_then(serde_json::Value::as_str)
        != Some("oasis7.public_testnet_validator_triad_inventory.v1")
    {
        return Err("deployment inventory schema is unsupported".to_string());
    }
    if object.get("repository").and_then(serde_json::Value::as_str) != Some("eng-cc/oasis7") {
        return Err("deployment inventory repository is not eng-cc/oasis7".to_string());
    }
    if object
        .get("network_tier")
        .and_then(serde_json::Value::as_str)
        != Some("public_testnet")
        || object.get("topology").and_then(serde_json::Value::as_str)
            != Some("three_equal_validator")
    {
        return Err("deployment inventory network tier/topology is unsupported".to_string());
    }
    let authority = object
        .get("authority")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| "deployment inventory authority is missing".to_string())?;
    let authority_string = |key: &str| {
        authority
            .get(key)
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| format!("deployment inventory authority.{key} is missing"))
    };
    let expected_world_id = authority_string("world_id")?;
    let expected_chain_id = authority_string("chain_id")?;
    let expected_network_tier = authority_string("network_tier")?;
    let expected_registry_ref = authority_string("registry_ref")?;
    let expected_generated_registry_sha256 = required_sha256(
        authority,
        "generated_registry_sha256",
        "deployment inventory authority",
    )?;
    let expected_generated_registry_semantic_sha256 = required_sha256(
        authority,
        "generated_registry_semantic_sha256",
        "deployment inventory authority",
    )?;
    if expected_inventory_ref != "scripts/public-testnet-validator-triad-inventory.v1.json" {
        return Err(format!(
            "network-tier deployment_inventory.ref is not the governed triad inventory: {expected_inventory_ref}"
        ));
    }
    let expected_registry_name = Path::new(expected_registry_ref)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "deployment inventory authority.registry_ref is invalid".to_string())?;
    if registry_path.file_name().and_then(|value| value.to_str()) != Some(expected_registry_name) {
        return Err(format!(
            "genesis validator registry path does not match inventory authority.registry_ref: expected={expected_registry_name} actual={}",
            registry_path.display()
        ));
    }
    if expected_network_tier != loaded.manifest.tier
        || expected_world_id != loaded.manifest.network_id
        || expected_chain_id != loaded.manifest.chain_id
    {
        return Err(
            "deployment inventory identity does not match the loaded network-tier manifest"
                .to_string(),
        );
    }
    let manifest_bytes = fs::read(Path::new(loaded.source_path.as_str())).map_err(|err| {
        format!(
            "read network-tier manifest {} for registry authority binding failed: {err}",
            loaded.source_path
        )
    })?;
    let manifest_value: serde_json::Value = serde_json::from_slice(manifest_bytes.as_slice())
        .map_err(|err| {
            format!(
                "parse network-tier manifest {} for registry authority binding failed: {err}",
                loaded.source_path
            )
        })?;
    let manifest_registry_binding = manifest_value
        .get("deployment_validator_registry")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| {
            "network-tier manifest is missing deployment_validator_registry authority binding"
                .to_string()
        })?;
    let manifest_registry_ref = required_string(
        manifest_registry_binding,
        "ref",
        "deployment_validator_registry",
    )?;
    if manifest_registry_ref != expected_registry_ref {
        return Err(format!(
            "network-tier manifest deployment validator registry ref does not match inventory authority: expected={expected_registry_ref} actual={manifest_registry_ref}"
        ));
    }
    let manifest_registry_sha256 = required_sha256(
        manifest_registry_binding,
        "sha256",
        "deployment_validator_registry",
    )?;
    if manifest_registry_sha256 != expected_generated_registry_sha256 {
        return Err(format!(
            "network-tier manifest deployment validator registry digest does not match inventory authority: expected={expected_generated_registry_sha256} actual={manifest_registry_sha256}"
        ));
    }
    let manifest_registry_semantic_sha256 = required_sha256(
        manifest_registry_binding,
        "semantic_sha256",
        "deployment_validator_registry",
    )?;
    if manifest_registry_semantic_sha256 != expected_generated_registry_semantic_sha256 {
        return Err(format!(
            "network-tier manifest deployment validator registry semantic digest does not match inventory authority: expected={expected_generated_registry_semantic_sha256} actual={manifest_registry_semantic_sha256}"
        ));
    }
    let validator_set = object
        .get("validator_set")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| "deployment inventory validator_set is missing".to_string())?;
    let count = validator_set
        .get("count")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "deployment inventory validator_set.count is missing".to_string())?;
    if count != 3 || loaded.manifest.validator_policy.target_validator_count != count {
        return Err("deployment inventory validator count does not match manifest".to_string());
    }
    if validator_set.get("stakes") != Some(&serde_json::json!([100, 100, 100]))
        || validator_set.get("total_stake") != Some(&serde_json::json!(300))
        || validator_set.get("required_stake") != Some(&serde_json::json!(200))
        || validator_set.get("quorum")
            != Some(&serde_json::json!({"numerator": 2, "denominator": 3}))
    {
        return Err(
            "deployment inventory validator quorum is not the governed 3x100 policy".to_string(),
        );
    }
    if object.get("governance")
        != Some(&serde_json::json!({"signer_count": 3, "threshold": 2, "threshold_bps": 6667}))
    {
        return Err(
            "deployment inventory governance policy is not the governed 2-of-3 policy".to_string(),
        );
    }
    let nodes = object
        .get("nodes")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| "deployment inventory nodes are missing".to_string())?;
    if nodes.len() != 3 {
        return Err("deployment inventory must contain exactly three validator nodes".to_string());
    }
    let mut validator_signer_public_keys = BTreeMap::new();
    let mut validator_stakes = BTreeMap::new();
    let mut validator_47_provider_peer_id = None;
    for (name, node) in nodes {
        let node = node
            .as_object()
            .ok_or_else(|| format!("deployment inventory node {name} is not an object"))?;
        let node_id = required_value_string(node, "node_id", name)?;
        let signer = required_value_string(node, "finality_signer_public_key", name)?;
        let stake = node
            .get("stake")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| format!("deployment inventory node {name}.stake is missing"))?;
        if stake != 100
            || signer.len() != 64
            || !signer.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(format!(
                "deployment inventory node {name} signer/stake is invalid"
            ));
        }
        validator_signer_public_keys.insert(node_id.clone(), signer.to_ascii_lowercase());
        validator_stakes.insert(node_id.clone(), stake);
        if name == "validator-47" {
            validator_47_provider_peer_id =
                Some(required_value_string(node, "libp2p_peer_id", name)?);
        }
    }
    if !nodes.contains_key("sequencer-204")
        || !nodes.contains_key("storage-205")
        || !nodes.contains_key("validator-47")
        || validator_signer_public_keys.len() != 3
    {
        return Err("deployment inventory validator node set is not canonical".to_string());
    }
    Ok(InventoryAuthority {
        validator_signer_public_keys,
        validator_stakes,
        validator_47_provider_peer_id,
        generated_registry_sha256: expected_generated_registry_sha256,
        generated_registry_semantic_sha256: expected_generated_registry_semantic_sha256,
    })
}

fn validate_registry_against_inventory(
    registry: &GovernanceFinalitySignerRegistry,
    inventory: &InventoryAuthority,
) -> Result<(), String> {
    let expected_signer_bindings = inventory
        .validator_signer_public_keys
        .iter()
        .map(|(node_id, public_key)| {
            (
                format!("governance.finality.v1.{node_id}"),
                public_key.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let expected_validator_stakes = inventory
        .validator_stakes
        .iter()
        .map(|(node_id, stake)| (format!("governance.finality.v1.{node_id}"), *stake))
        .collect::<BTreeMap<_, _>>();
    if registry.slot_id != "governance.finality.v1"
        || registry.threshold != 2
        || registry.threshold_bps != 0
        || registry.signer_bindings != expected_signer_bindings
        || registry.validator_stakes != expected_validator_stakes
    {
        return Err(
            "genesis validator registry does not match the governed inventory validator authority"
                .to_string(),
        );
    }
    Ok(())
}

fn required_value_string(
    object: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    scope: &str,
) -> Result<String, String> {
    object
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("deployment inventory node {scope}.{key} is missing"))
}

fn semantic_registry_sha256(registry: &GovernanceFinalitySignerRegistry) -> Result<String, String> {
    let mut canonical = BTreeMap::new();
    canonical.insert(
        "signer_bindings",
        serde_json::to_value(&registry.signer_bindings).map_err(|err| err.to_string())?,
    );
    canonical.insert(
        "slot_id",
        serde_json::Value::String(registry.slot_id.clone()),
    );
    canonical.insert("threshold", serde_json::json!(registry.threshold));
    canonical.insert("threshold_bps", serde_json::json!(registry.threshold_bps));
    canonical.insert(
        "validator_stakes",
        serde_json::to_value(&registry.validator_stakes).map_err(|err| err.to_string())?,
    );
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|err| format!("encode registry semantic identity failed: {err}"))?;
    Ok(hex::encode(Sha256::digest(bytes.as_slice())))
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
    use oasis7::runtime::{GovernanceFinalitySignerRegistry, World as RuntimeWorld};
    use std::collections::BTreeMap;
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
                    promote_from: vec!["local_devnet".to_string()],
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
        let validator_nodes = serde_json::json!({
            "sequencer-204": {
                "node_id": "triad-testnet-sequencer",
                "roles": ["validator", "sequencer"],
                "stake": 100,
                "finality_signer_public_key": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            },
            "storage-205": {
                "node_id": "triad-testnet-storage",
                "roles": ["validator", "storage"],
                "stake": 100,
                "finality_signer_public_key": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            },
            "validator-47": {
                "node_id": "triad-testnet-validator-47",
                "roles": ["validator", "checkpoint_provider", "full_storage_provider"],
                "stake": 100,
                "finality_signer_public_key": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                "libp2p_peer_id": "12D3KooWFixture"
            }
        });
        let mut inventory_value = serde_json::json!({
            "schema_version": "oasis7.public_testnet_validator_triad_inventory.v1",
            "repository": "eng-cc/oasis7",
            "network_tier": "public_testnet",
            "topology": "three_equal_validator",
            "validator_set": {
                "count": 3,
                "stakes": [100, 100, 100],
                "total_stake": 300,
                "required_stake": 200,
                "quorum": {"numerator": 2, "denominator": 3}
            },
            "authority": {
                "world_id": "oasis7-public-testnet-governed-20260606",
                "chain_id": "oasis7-public-testnet-governed-20260606",
                "network_tier": "public_testnet",
                "registry_ref": "registry.json",
                "generated_registry_sha256": "",
                "generated_registry_semantic_sha256": ""
            },
            "governance": {"signer_count": 3, "threshold": 2, "threshold_bps": 6667},
            "nodes": validator_nodes
        });
        let registry_bytes = serde_json::to_vec(&serde_json::json!({
                "slot_id": "governance.finality.v1",
                "threshold": 2,
                "threshold_bps": 0,
                "validators": [
                    {"node_id": "triad-testnet-sequencer", "finality_signer_public_key": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", "stake": 100},
                    {"node_id": "triad-testnet-storage", "finality_signer_public_key": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", "stake": 100},
                    {"node_id": "triad-testnet-validator-47", "finality_signer_public_key": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc", "stake": 100}
                ]
            }))
            .expect("encode registry");
        fs::write(registry.as_path(), registry_bytes).expect("write registry");
        let registry_sha256 = hex::encode(Sha256::digest(
            fs::read(registry.as_path())
                .expect("read registry")
                .as_slice(),
        ));
        inventory_value["authority"]["generated_registry_sha256"] =
            serde_json::Value::String(registry_sha256.clone());
        let fixture_registry =
            super::super::governance_registry::load_genesis_finality_registry(registry.as_path())
                .expect("load fixture registry for semantic digest");
        let registry_semantic_sha256 =
            semantic_registry_sha256(&fixture_registry).expect("semantic registry digest");
        inventory_value["authority"]["generated_registry_semantic_sha256"] =
            serde_json::Value::String(registry_semantic_sha256.clone());
        fs::write(
            inventory.as_path(),
            serde_json::to_vec(&inventory_value).expect("encode inventory"),
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
                "{{\"deployment_inventory\":{{\"ref\":\"scripts/public-testnet-validator-triad-inventory.v1.json\",\"sha256\":\"{inventory_sha256}\"}},\"deployment_validator_registry\":{{\"ref\":\"registry.json\",\"sha256\":\"{registry_sha256}\",\"semantic_sha256\":\"{registry_semantic_sha256}\"}}}}\n"
            ),
        )
        .expect("write manifest");
        (registry, inventory, manifest_path)
    }

    #[test]
    fn captures_actual_refs_and_file_digests() {
        let (registry, inventory, manifest_path) = write_authority_fixture();
        let binding = load_runtime_authority_binding(
            manifest_path.parent().expect("fixture root"),
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
        assert_eq!(binding.registry_semantic_sha256.len(), 64);
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
        let error = load_runtime_authority_binding(
            Path::new("world"),
            Some(Path::new("registry.json")),
            None,
            None,
        )
        .expect_err("partial binding must fail closed");
        assert!(error.contains("--deployment-inventory"));
    }

    #[test]
    fn rejects_public_testnet_without_authority_inputs() {
        let error = load_runtime_authority_binding(
            Path::new("world"),
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
            manifest_path.parent().expect("fixture root"),
            Some(registry.as_path()),
            Some(inventory.as_path()),
            Some(&loaded_manifest(manifest_path.as_path())),
        )
        .expect_err("digest drift must fail closed");
        assert!(error.contains("digest mismatch"));
    }

    #[test]
    fn rejects_generated_registry_digest_drift() {
        let (registry, inventory, manifest_path) = write_authority_fixture();
        let mut registry_bytes = fs::read(registry.as_path()).expect("read fixture registry");
        registry_bytes.push(b' ');
        fs::write(registry.as_path(), registry_bytes).expect("rewrite fixture registry");
        let error = load_runtime_authority_binding(
            manifest_path.parent().expect("fixture root"),
            Some(registry.as_path()),
            Some(inventory.as_path()),
            Some(&loaded_manifest(manifest_path.as_path())),
        )
        .expect_err("changed generated registry must fail closed");
        assert!(error.contains("generated validator registry digest mismatch"));
    }

    #[test]
    fn rejects_inventory_digest_drift_without_mutating_existing_world() {
        let (registry, inventory, manifest_path) = write_authority_fixture();
        let world_dir = manifest_path.parent().expect("fixture root").join("world");
        let explicit_registry =
            super::super::governance_registry::load_genesis_finality_registry(registry.as_path())
                .expect("load fixture registry");
        let mut world = RuntimeWorld::new_production_hardened();
        world
            .set_governance_finality_signer_registry(explicit_registry)
            .expect("set persisted effective registry");
        world.save_to_dir(world_dir.as_path()).expect("save world");
        let snapshot_before = fs::read(world_dir.join("snapshot.json")).expect("read snapshot");
        let journal_before = fs::read(world_dir.join("journal.json")).expect("read journal");

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
            world_dir.as_path(),
            Some(registry.as_path()),
            Some(inventory.as_path()),
            Some(&loaded_manifest(manifest_path.as_path())),
        )
        .expect_err("digest drift must fail before world bootstrap mutation");
        assert!(error.contains("digest mismatch"));
        assert_eq!(
            snapshot_before,
            fs::read(world_dir.join("snapshot.json")).expect("read snapshot after rejection")
        );
        assert_eq!(
            journal_before,
            fs::read(world_dir.join("journal.json")).expect("read journal after rejection")
        );
    }

    #[test]
    fn rejects_explicit_registry_that_does_not_match_persisted_effective_registry() {
        let (registry, inventory, manifest_path) = write_authority_fixture();
        let world_dir = manifest_path.parent().expect("fixture root").join("world");
        let mut world = RuntimeWorld::new_production_hardened();
        world
            .set_governance_finality_signer_registry(GovernanceFinalitySignerRegistry {
                slot_id: "governance.finality.v1".to_string(),
                threshold: 2,
                threshold_bps: 0,
                signer_bindings: BTreeMap::from([
                    (
                        "governance.finality.v1.triad-testnet-sequencer".to_string(),
                        "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
                            .to_string(),
                    ),
                    (
                        "governance.finality.v1.triad-testnet-storage".to_string(),
                        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                            .to_string(),
                    ),
                    (
                        "governance.finality.v1.triad-testnet-validator-47".to_string(),
                        "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
                            .to_string(),
                    ),
                ]),
                validator_stakes: BTreeMap::from([
                    (
                        "governance.finality.v1.triad-testnet-sequencer".to_string(),
                        100,
                    ),
                    (
                        "governance.finality.v1.triad-testnet-storage".to_string(),
                        100,
                    ),
                    (
                        "governance.finality.v1.triad-testnet-validator-47".to_string(),
                        100,
                    ),
                ]),
            })
            .expect("set persisted effective registry");
        world.save_to_dir(world_dir.as_path()).expect("save world");
        let error = load_runtime_authority_binding(
            world_dir.as_path(),
            Some(registry.as_path()),
            Some(inventory.as_path()),
            Some(&loaded_manifest(manifest_path.as_path())),
        )
        .expect_err("stale explicit registry must fail closed");
        assert!(error.contains("does not match persisted effective governance registry"));
    }
}
