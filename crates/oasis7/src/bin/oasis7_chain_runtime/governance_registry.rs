use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use oasis7::network_tier_manifest::LoadedNetworkTierManifest;
use oasis7::runtime::{
    GovernanceFinalitySignerRegistry, GovernanceMainTokenControllerRegistry,
    GovernanceThresholdSignerPolicy,
};
use oasis7_node::{
    NodeConfig, NodeMainTokenControllerBindingConfig, NodeMainTokenControllerSignerPolicy,
    NodePosConfig, PosValidator,
};
use serde::Deserialize;

const DEFAULT_FINALITY_SLOT_ID: &str = "governance.finality.v1";
const GOVERNANCE_REGISTRY_DEFAULT_VALIDATOR_STAKE: u64 = 100;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum GenesisValidatorRegistryManifest {
    Document(GenesisValidatorRegistryDocument),
    PublicManifestEntries(Vec<GenesisValidatorRegistryEntry>),
}

#[derive(Debug, Deserialize)]
struct GenesisValidatorRegistryDocument {
    #[serde(default = "default_finality_slot_id")]
    slot_id: String,
    threshold: u16,
    #[serde(default)]
    threshold_bps: u16,
    validators: Vec<GenesisValidatorRegistryEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct GenesisValidatorRegistryEntry {
    #[serde(default)]
    slot_id: Option<String>,
    #[serde(default)]
    node_id: Option<String>,
    #[serde(default)]
    signer_id: Option<String>,
    #[serde(default = "default_ed25519_scheme")]
    scheme: String,
    #[serde(default)]
    finality_signer_public_key: Option<String>,
    #[serde(default)]
    public_key_hex: Option<String>,
    #[serde(default = "default_genesis_validator_stake")]
    stake: u64,
    #[serde(default)]
    threshold: Option<u16>,
}

fn default_finality_slot_id() -> String {
    DEFAULT_FINALITY_SLOT_ID.to_string()
}

fn default_ed25519_scheme() -> String {
    "ed25519".to_string()
}

fn default_genesis_validator_stake() -> u64 {
    GOVERNANCE_REGISTRY_DEFAULT_VALIDATOR_STAKE
}

pub(super) fn ensure_world_governance_validator_registry(
    execution_world_dir: &Path,
    genesis_validator_registry_path: Option<&Path>,
    loaded_network_tier_manifest: Option<&LoadedNetworkTierManifest>,
) -> Result<(), String> {
    if world_has_effective_finality_registry(execution_world_dir)? {
        return Ok(());
    }
    if let Some(manifest_path) = genesis_validator_registry_path {
        import_genesis_validator_registry(execution_world_dir, manifest_path)?;
        return Ok(());
    }
    if network_tier_requires_governance_validator_registry(loaded_network_tier_manifest) {
        return Err(public_tier_missing_registry_error(
            execution_world_dir,
            loaded_network_tier_manifest,
        ));
    }
    Ok(())
}

pub(super) fn apply_world_governance_registry_overrides(
    mut config: NodeConfig,
    execution_world_dir: &Path,
) -> Result<NodeConfig, String> {
    let world = super::execution_bridge::load_execution_world(execution_world_dir)?;
    if let Some(registry) = world
        .resolve_governance_effective_finality_signer_registry()
        .map_err(|err| {
            format!("failed to resolve world governance effective finality registry: {err:?}")
        })?
    {
        let pos_config =
            node_pos_config_from_world_finality_registry(&registry, &config.pos_config);
        config = config.with_pos_config(pos_config).map_err(|err| {
            format!("failed to apply world governance finality registry: {err:?}")
        })?;
    }
    if let Some(registry) = world.governance_main_token_controller_registry() {
        let binding = node_main_token_controller_binding_from_registry(
            registry,
            config.main_token_controller_binding.clone(),
        );
        config = config
            .with_main_token_controller_binding(binding)
            .map_err(|err| {
                format!("failed to apply world governance controller registry: {err:?}")
            })?;
    }
    Ok(config)
}

fn world_has_effective_finality_registry(execution_world_dir: &Path) -> Result<bool, String> {
    let world = super::execution_bridge::load_execution_world(execution_world_dir)?;
    world
        .resolve_governance_effective_finality_signer_registry()
        .map(|registry| registry.is_some())
        .map_err(|err| {
            format!("failed to resolve world governance effective finality registry: {err:?}")
        })
}

fn import_genesis_validator_registry(
    execution_world_dir: &Path,
    manifest_path: &Path,
) -> Result<(), String> {
    let manifest_bytes = std::fs::read(manifest_path).map_err(|err| {
        format!(
            "read genesis validator registry {} failed: {err}",
            manifest_path.display()
        )
    })?;
    let manifest: GenesisValidatorRegistryManifest =
        serde_json::from_slice(manifest_bytes.as_slice()).map_err(|err| {
            format!(
                "decode genesis validator registry {} failed: {err}",
                manifest_path.display()
            )
        })?;
    let registry = build_genesis_finality_registry(manifest)?;
    if execution_world_has_persisted_state(execution_world_dir) {
        return Err(format!(
            "refusing to import genesis validator registry into existing execution world {} without reconciliation; use the dedicated governance registry import/migration path for non-empty worlds",
            execution_world_dir.display()
        ));
    }
    let mut world = super::execution_bridge::load_execution_world(execution_world_dir)?;
    if world
        .resolve_governance_effective_finality_signer_registry()
        .map_err(|err| {
            format!("failed to resolve world governance effective finality registry: {err:?}")
        })?
        .is_some()
    {
        return Ok(());
    }
    world
        .set_governance_finality_signer_registry(registry)
        .map_err(|err| format!("write genesis finality registry failed: {err:?}"))?;
    world.save_to_dir(execution_world_dir).map_err(|err| {
        format!(
            "save execution world {} after genesis validator registry import failed: {err:?}",
            execution_world_dir.display()
        )
    })
}

fn build_genesis_finality_registry(
    manifest: GenesisValidatorRegistryManifest,
) -> Result<GovernanceFinalitySignerRegistry, String> {
    match manifest {
        GenesisValidatorRegistryManifest::Document(document) => {
            build_genesis_finality_registry_from_entries(
                document.slot_id.trim(),
                document.threshold,
                document.threshold_bps,
                document.validators.as_slice(),
            )
        }
        GenesisValidatorRegistryManifest::PublicManifestEntries(entries) => {
            let finality_slot_id = DEFAULT_FINALITY_SLOT_ID;
            let finality_entries = entries
                .iter()
                .filter(|entry| entry.slot_id.as_deref() == Some(finality_slot_id))
                .cloned()
                .collect::<Vec<_>>();
            let threshold =
                explicit_public_manifest_threshold(finality_slot_id, finality_entries.as_slice())?;
            build_genesis_finality_registry_from_entries(
                finality_slot_id,
                threshold,
                0,
                finality_entries.as_slice(),
            )
        }
    }
}

fn explicit_public_manifest_threshold(
    finality_slot_id: &str,
    entries: &[GenesisValidatorRegistryEntry],
) -> Result<u16, String> {
    let mut thresholds = entries
        .iter()
        .filter_map(|entry| entry.threshold)
        .collect::<BTreeSet<_>>();
    match thresholds.len() {
        1 => Ok(thresholds.pop_first().expect("one threshold")),
        0 => Err(format!(
            "public-manifest genesis validator registry for slot {finality_slot_id} requires explicit threshold; use document form or set threshold on each finality entry"
        )),
        _ => Err(format!(
            "public-manifest genesis validator registry for slot {finality_slot_id} has inconsistent entry thresholds: {thresholds:?}"
        )),
    }
}

fn execution_world_has_persisted_state(execution_world_dir: &Path) -> bool {
    [
        "snapshot.json",
        "journal.json",
        "snapshot.manifest.json",
        "journal.segments.json",
    ]
    .iter()
    .any(|name| execution_world_dir.join(name).exists())
        || execution_world_dir.join(".distfs-state").exists()
}

fn build_genesis_finality_registry_from_entries(
    slot_id: &str,
    threshold: u16,
    threshold_bps: u16,
    entries: &[GenesisValidatorRegistryEntry],
) -> Result<GovernanceFinalitySignerRegistry, String> {
    let slot_id = slot_id.trim();
    if slot_id.is_empty() {
        return Err("genesis validator registry slot_id cannot be empty".to_string());
    }
    if threshold == 0 {
        return Err("genesis validator registry threshold must be > 0".to_string());
    }
    let mut signer_bindings = BTreeMap::new();
    let mut validator_stakes = BTreeMap::new();
    for entry in entries {
        validate_genesis_validator_entry(entry)?;
        let node_id = genesis_validator_entry_node_id(entry)?;
        let public_key_hex = genesis_validator_entry_public_key(entry)?;
        if signer_bindings
            .insert(node_id.clone(), public_key_hex.to_string())
            .is_some()
        {
            return Err(format!(
                "genesis validator registry duplicates node_id {node_id}"
            ));
        }
        validator_stakes.insert(node_id, entry.stake);
    }
    if signer_bindings.is_empty() {
        return Err(format!(
            "genesis validator registry contains no validators for slot {slot_id}"
        ));
    }
    if usize::from(threshold) > signer_bindings.len() {
        return Err(format!(
            "genesis validator registry threshold {threshold} exceeds validator count {}",
            signer_bindings.len()
        ));
    }
    Ok(GovernanceFinalitySignerRegistry {
        slot_id: slot_id.to_string(),
        threshold,
        threshold_bps,
        signer_bindings,
        validator_stakes,
    })
}

fn validate_genesis_validator_entry(entry: &GenesisValidatorRegistryEntry) -> Result<(), String> {
    if !entry.scheme.trim().eq_ignore_ascii_case("ed25519") {
        return Err(format!(
            "genesis validator registry only supports ed25519 finality signers: node_id={:?} signer_id={:?} scheme={}",
            entry.node_id, entry.signer_id, entry.scheme
        ));
    }
    if entry.stake == 0 {
        return Err(format!(
            "genesis validator registry stake must be > 0: node_id={:?} signer_id={:?}",
            entry.node_id, entry.signer_id
        ));
    }
    let public_key_hex = genesis_validator_entry_public_key(entry)?;
    if public_key_hex.len() != 64 || !public_key_hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(format!(
            "genesis validator registry finality signer public key must be 32-byte hex: node_id={:?} signer_id={:?}",
            entry.node_id, entry.signer_id
        ));
    }
    Ok(())
}

fn genesis_validator_entry_node_id(
    entry: &GenesisValidatorRegistryEntry,
) -> Result<String, String> {
    entry
        .node_id
        .as_deref()
        .or(entry.signer_id.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| "genesis validator registry entry requires node_id or signer_id".to_string())
}

fn genesis_validator_entry_public_key<'a>(
    entry: &'a GenesisValidatorRegistryEntry,
) -> Result<&'a str, String> {
    entry
        .finality_signer_public_key
        .as_deref()
        .or(entry.public_key_hex.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "genesis validator registry entry requires finality_signer_public_key or public_key_hex"
                .to_string()
        })
}

fn network_tier_requires_governance_validator_registry(
    loaded_network_tier_manifest: Option<&LoadedNetworkTierManifest>,
) -> bool {
    let Some(loaded) = loaded_network_tier_manifest else {
        return false;
    };
    let tier = loaded.manifest.tier.as_str();
    let admission = loaded
        .manifest
        .validator_policy
        .validator_admission
        .as_str();
    matches!(tier, "public_testnet" | "mainnet")
        || matches!(
            admission,
            "allowlist_or_governed_candidate" | "governance_registry_only"
        )
}

fn public_tier_missing_registry_error(
    execution_world_dir: &Path,
    loaded_network_tier_manifest: Option<&LoadedNetworkTierManifest>,
) -> String {
    let tier = loaded_network_tier_manifest
        .map(|loaded| loaded.manifest.tier.as_str())
        .unwrap_or("unknown");
    let admission = loaded_network_tier_manifest
        .map(|loaded| {
            loaded
                .manifest
                .validator_policy
                .validator_admission
                .as_str()
        })
        .unwrap_or("unknown");
    format!(
        "public validator registry required before startup: execution_world_dir={} tier={} validator_admission={} has no effective governance finality registry; provide --genesis-validator-registry <path> for first boot or migrate the world-state registry before restart",
        execution_world_dir.display(),
        tier,
        admission
    )
}

fn node_pos_config_from_world_finality_registry(
    registry: &GovernanceFinalitySignerRegistry,
    fallback: &NodePosConfig,
) -> NodePosConfig {
    let validator_signer_public_keys = registry
        .signer_bindings
        .iter()
        .map(|(binding_key, public_key_hex)| {
            (
                validator_id_from_registry_binding(registry.slot_id.as_str(), binding_key),
                public_key_hex.clone(),
            )
        })
        .collect::<BTreeMap<String, String>>();
    let validators = registry
        .signer_bindings
        .keys()
        .map(|binding_key| {
            let validator_id =
                validator_id_from_registry_binding(registry.slot_id.as_str(), binding_key);
            PosValidator {
                stake: registry
                    .validator_stakes
                    .get(validator_id.as_str())
                    .or_else(|| registry.validator_stakes.get(binding_key.as_str()))
                    .copied()
                    .unwrap_or(GOVERNANCE_REGISTRY_DEFAULT_VALIDATOR_STAKE),
                validator_id,
            }
        })
        .collect::<Vec<PosValidator>>();
    let validator_player_ids = validator_signer_public_keys
        .keys()
        .cloned()
        .map(|validator_id| (validator_id.clone(), validator_id))
        .collect::<BTreeMap<String, String>>();
    NodePosConfig {
        validators,
        validator_player_ids,
        validator_signer_public_keys,
        supermajority_numerator: fallback.supermajority_numerator,
        supermajority_denominator: fallback.supermajority_denominator,
        epoch_length_slots: fallback.epoch_length_slots,
        slot_duration_ms: fallback.slot_duration_ms,
        ticks_per_slot: fallback.ticks_per_slot,
        proposal_tick_phase: fallback.proposal_tick_phase,
        adaptive_tick_scheduler_enabled: fallback.adaptive_tick_scheduler_enabled,
        slot_clock_genesis_unix_ms: fallback.slot_clock_genesis_unix_ms,
        max_past_slot_lag: fallback.max_past_slot_lag,
    }
}

fn validator_id_from_registry_binding(slot_id: &str, binding_key: &str) -> String {
    let prefix = format!("{slot_id}.");
    binding_key
        .strip_prefix(prefix.as_str())
        .filter(|value| !value.is_empty())
        .unwrap_or(binding_key)
        .to_string()
}

fn node_main_token_controller_binding_from_registry(
    registry: &GovernanceMainTokenControllerRegistry,
    mut fallback: NodeMainTokenControllerBindingConfig,
) -> NodeMainTokenControllerBindingConfig {
    fallback.genesis_controller_account_id = registry.genesis_controller_account_id.clone();
    if !registry.treasury_bucket_controller_slots.is_empty() {
        fallback.treasury_bucket_controller_slots =
            registry.treasury_bucket_controller_slots.clone();
    }
    fallback.controller_signer_policies = registry
        .controller_signer_policies
        .iter()
        .map(|(account_id, policy)| {
            (
                account_id.clone(),
                node_main_token_controller_signer_policy_from_registry(policy),
            )
        })
        .collect::<BTreeMap<String, NodeMainTokenControllerSignerPolicy>>();
    fallback
}

fn node_main_token_controller_signer_policy_from_registry(
    policy: &GovernanceThresholdSignerPolicy,
) -> NodeMainTokenControllerSignerPolicy {
    NodeMainTokenControllerSignerPolicy {
        threshold: policy.threshold,
        allowed_public_keys: policy
            .allowed_public_keys
            .iter()
            .cloned()
            .collect::<BTreeSet<String>>(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_world_governance_registry_overrides, ensure_world_governance_validator_registry,
    };
    use oasis7::network_tier_manifest::{
        LoadedNetworkTierManifest, NetworkTierClaimsPolicy, NetworkTierEndpointPolicy,
        NetworkTierManifest, NetworkTierPromotionPolicy, NetworkTierRuntimeRefs,
        NetworkTierTokenPolicy, NetworkTierValidatorPolicy, NETWORK_TIER_MANIFEST_SCHEMA_V1,
    };
    use oasis7::runtime::{
        Action, GovernanceExecutionPolicy, GovernanceFinalitySignerRegistry,
        GovernanceMainTokenControllerRegistry, GovernanceThresholdSignerPolicy, World,
    };
    use oasis7_node::{NodeConfig, NodeRole};
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(prefix: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("duration")
            .as_nanos();
        std::env::temp_dir().join(format!("oasis7-chain-governance-{prefix}-{unique}"))
    }

    fn public_testnet_loaded_manifest() -> LoadedNetworkTierManifest {
        LoadedNetworkTierManifest {
            source_path: "test-network-tier.json".to_string(),
            manifest: NetworkTierManifest {
                schema_version: NETWORK_TIER_MANIFEST_SCHEMA_V1.to_string(),
                tier: "public_testnet".to_string(),
                status: "live".to_string(),
                network_id: "oasis7-public-testnet".to_string(),
                chain_id: "oasis7-public-testnet".to_string(),
                runtime_refs: NetworkTierRuntimeRefs {
                    release_candidate_bundle_ref: "bundle.json".to_string(),
                    genesis_ref: "genesis.json".to_string(),
                    bootstrap_peer_ref: "bootstrap.txt".to_string(),
                },
                endpoint_policy: NetworkTierEndpointPolicy {
                    rpc_ref: "rpc.json".to_string(),
                    explorer_ref: "explorer.json".to_string(),
                    faucet_ref: Some("faucet.json".to_string()),
                },
                validator_policy: NetworkTierValidatorPolicy {
                    governance_mode: "governance_registry".to_string(),
                    validator_admission: "allowlist_or_governed_candidate".to_string(),
                    target_validator_count: 3,
                    allow_observer_nodes: true,
                },
                token_policy: NetworkTierTokenPolicy {
                    symbol: "O7T".to_string(),
                    faucet_mode: "guarded_testnet_faucet".to_string(),
                    reset_policy: "resettable".to_string(),
                    value_semantics: "testnet".to_string(),
                },
                claims_policy: NetworkTierClaimsPolicy {
                    allowed_claims: vec!["public_testnet".to_string()],
                    denied_claims: vec![
                        "mainnet_live".to_string(),
                        "production_oc_settlement".to_string(),
                    ],
                },
                promotion_policy: NetworkTierPromotionPolicy {
                    promote_from: vec!["shared_devnet".to_string()],
                    required_gates: vec!["shared_devnet_pass".to_string()],
                },
                evidence_refs: Vec::new(),
            },
            bootstrap_peers: Vec::new(),
        }
    }

    #[test]
    fn world_registry_overrides_node_controller_binding() {
        let temp_dir = temp_dir("registry-override");
        let mut world = World::new();
        world
            .set_governance_main_token_controller_registry(GovernanceMainTokenControllerRegistry {
                genesis_controller_account_id: "msig.genesis.v1".to_string(),
                treasury_bucket_controller_slots: BTreeMap::from([(
                    "staking_reward_pool".to_string(),
                    "msig.staking_governance.v1".to_string(),
                )]),
                restricted_starter_claim_admin_account_ids: BTreeSet::from([
                    "msig.staking_governance.v1".to_string(),
                ]),
                controller_signer_policies: BTreeMap::from([
                    (
                        "msig.genesis.v1".to_string(),
                        GovernanceThresholdSignerPolicy {
                            threshold: 2,
                            allowed_public_keys: BTreeSet::from([
                                "6249e5a58278dbc4e629a16b5d33f6b84c39e3ceeb10e963bb9ef64ea4daac30"
                                    .to_string(),
                                "7014e88a6336ec91fc7e6ffb044b50232e4411ec403f90123fa8a202a3420a04"
                                    .to_string(),
                            ]),
                        },
                    ),
                    (
                        "msig.staking_governance.v1".to_string(),
                        GovernanceThresholdSignerPolicy {
                            threshold: 2,
                            allowed_public_keys: BTreeSet::from([
                                "13c160fc0f516b9a5663aa00c2a5446be6467f68ce341fdd79cdb64224dffd20"
                                    .to_string(),
                                "10fa4d90abf753ec1aa54aee3ea53bab25f43e7078897e1fb6a3777af2255bcb"
                                    .to_string(),
                            ]),
                        },
                    ),
                ]),
            })
            .expect("set controller registry");
        world.save_to_dir(&temp_dir).expect("save execution world");

        let config =
            NodeConfig::new("node-a", "world-a", NodeRole::Sequencer).expect("node config");
        let config = apply_world_governance_registry_overrides(config, &temp_dir)
            .expect("apply registry overrides");

        assert_eq!(
            config
                .main_token_controller_binding
                .genesis_controller_account_id,
            "msig.genesis.v1"
        );
        assert_eq!(
            config
                .main_token_controller_binding
                .treasury_bucket_controller_slots
                .get("staking_reward_pool")
                .map(String::as_str),
            Some("msig.staking_governance.v1")
        );
        assert_eq!(
            config
                .main_token_controller_binding
                .controller_signer_policies
                .get("msig.genesis.v1")
                .map(|policy| policy.threshold),
            Some(2)
        );
    }

    #[test]
    fn world_finality_registry_overrides_node_pos_config() {
        let temp_dir = temp_dir("finality-override");
        let mut world = World::new();
        world
            .set_governance_finality_signer_registry(GovernanceFinalitySignerRegistry {
                slot_id: "governance.finality.v1".to_string(),
                threshold: 2,
                threshold_bps: 0,
                signer_bindings: BTreeMap::from([
                    (
                        "validator-a".to_string(),
                        "1111111111111111111111111111111111111111111111111111111111111111"
                            .to_string(),
                    ),
                    (
                        "validator-b".to_string(),
                        "2222222222222222222222222222222222222222222222222222222222222222"
                            .to_string(),
                    ),
                    (
                        "validator-c".to_string(),
                        "3333333333333333333333333333333333333333333333333333333333333333"
                            .to_string(),
                    ),
                ]),
                validator_stakes: BTreeMap::from([
                    ("validator-a".to_string(), 70),
                    ("validator-b".to_string(), 20),
                    ("validator-c".to_string(), 10),
                ]),
            })
            .expect("set finality registry");
        world.save_to_dir(&temp_dir).expect("save execution world");

        let mut config =
            NodeConfig::new("node-a", "world-a", NodeRole::Sequencer).expect("node config");
        config.pos_config.slot_duration_ms = 12_000;
        config.pos_config.ticks_per_slot = 10;
        config.pos_config.proposal_tick_phase = 9;
        let config = apply_world_governance_registry_overrides(config, &temp_dir)
            .expect("apply registry overrides");

        let validator_ids = config
            .pos_config
            .validators
            .iter()
            .map(|validator| validator.validator_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            validator_ids,
            vec!["validator-a", "validator-b", "validator-c"]
        );
        let validator_stakes = config
            .pos_config
            .validators
            .iter()
            .map(|validator| (validator.validator_id.as_str(), validator.stake))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(validator_stakes.get("validator-a"), Some(&70));
        assert_eq!(validator_stakes.get("validator-b"), Some(&20));
        assert_eq!(validator_stakes.get("validator-c"), Some(&10));
        assert_eq!(
            config
                .pos_config
                .validator_signer_public_keys
                .get("validator-b")
                .map(String::as_str),
            Some("2222222222222222222222222222222222222222222222222222222222222222")
        );
        assert_eq!(
            config
                .pos_config
                .validator_player_ids
                .get("validator-c")
                .map(String::as_str),
            Some("validator-c")
        );
        assert_eq!(config.pos_config.slot_duration_ms, 12_000);
        assert_eq!(config.pos_config.ticks_per_slot, 10);
        assert_eq!(config.pos_config.proposal_tick_phase, 9);
    }

    #[test]
    fn genesis_validator_registry_initializes_empty_world_and_uses_stake() {
        let temp_dir = temp_dir("genesis-finality-registry");
        std::fs::create_dir_all(&temp_dir).expect("create temp dir");
        let manifest_path = temp_dir.join("genesis-validator-registry.json");
        std::fs::write(
            manifest_path.as_path(),
            serde_json::to_vec_pretty(&serde_json::json!({
                "slot_id": "governance.finality.v1",
                "threshold": 2,
                "validators": [
                    {
                        "node_id": "validator-a",
                        "scheme": "ed25519",
                        "finality_signer_public_key": "1111111111111111111111111111111111111111111111111111111111111111",
                        "stake": 70
                    },
                    {
                        "node_id": "validator-b",
                        "scheme": "ed25519",
                        "finality_signer_public_key": "2222222222222222222222222222222222222222222222222222222222222222",
                        "stake": 20
                    },
                    {
                        "node_id": "validator-c",
                        "scheme": "ed25519",
                        "finality_signer_public_key": "3333333333333333333333333333333333333333333333333333333333333333",
                        "stake": 10
                    }
                ]
            }))
            .expect("encode manifest"),
        )
        .expect("write manifest");

        ensure_world_governance_validator_registry(
            temp_dir.as_path(),
            Some(manifest_path.as_path()),
            Some(&public_testnet_loaded_manifest()),
        )
        .expect("ensure genesis registry");
        let config = apply_world_governance_registry_overrides(
            NodeConfig::new("node-a", "world-a", NodeRole::Sequencer).expect("node config"),
            temp_dir.as_path(),
        )
        .expect("apply registry overrides");

        let validator_stakes = config
            .pos_config
            .validators
            .iter()
            .map(|validator| (validator.validator_id.as_str(), validator.stake))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(validator_stakes.get("validator-a"), Some(&70));
        assert_eq!(validator_stakes.get("validator-b"), Some(&20));
        assert_eq!(validator_stakes.get("validator-c"), Some(&10));
        assert_eq!(
            config
                .pos_config
                .validator_signer_public_keys
                .get("validator-c")
                .map(String::as_str),
            Some("3333333333333333333333333333333333333333333333333333333333333333")
        );
    }

    #[test]
    fn genesis_public_manifest_requires_explicit_threshold_for_four_validators() {
        let temp_dir = temp_dir("genesis-public-manifest-threshold");
        std::fs::create_dir_all(&temp_dir).expect("create temp dir");
        let manifest_path = temp_dir.join("public-manifest-entries.json");
        std::fs::write(
            manifest_path.as_path(),
            serde_json::to_vec_pretty(&serde_json::json!([
                {
                    "slot_id": "governance.finality.v1",
                    "signer_id": "validator-a",
                    "scheme": "ed25519",
                    "public_key_hex": "1111111111111111111111111111111111111111111111111111111111111111"
                },
                {
                    "slot_id": "governance.finality.v1",
                    "signer_id": "validator-b",
                    "scheme": "ed25519",
                    "public_key_hex": "2222222222222222222222222222222222222222222222222222222222222222"
                },
                {
                    "slot_id": "governance.finality.v1",
                    "signer_id": "validator-c",
                    "scheme": "ed25519",
                    "public_key_hex": "3333333333333333333333333333333333333333333333333333333333333333"
                },
                {
                    "slot_id": "governance.finality.v1",
                    "signer_id": "validator-d",
                    "scheme": "ed25519",
                    "public_key_hex": "4444444444444444444444444444444444444444444444444444444444444444"
                }
            ]))
            .expect("encode manifest"),
        )
        .expect("write manifest");

        let err = ensure_world_governance_validator_registry(
            temp_dir.as_path(),
            Some(manifest_path.as_path()),
            Some(&public_testnet_loaded_manifest()),
        )
        .expect_err("public manifest entries require explicit threshold");

        assert!(err.contains("requires explicit threshold"), "{err}");
    }

    #[test]
    fn genesis_public_manifest_preserves_explicit_three_of_four_threshold() {
        let temp_dir = temp_dir("genesis-public-manifest-three-of-four");
        std::fs::create_dir_all(&temp_dir).expect("create temp dir");
        let manifest_path = temp_dir.join("public-manifest-entries.json");
        std::fs::write(
            manifest_path.as_path(),
            serde_json::to_vec_pretty(&serde_json::json!([
                {
                    "slot_id": "governance.finality.v1",
                    "signer_id": "validator-a",
                    "scheme": "ed25519",
                    "threshold": 3,
                    "public_key_hex": "1111111111111111111111111111111111111111111111111111111111111111"
                },
                {
                    "slot_id": "governance.finality.v1",
                    "signer_id": "validator-b",
                    "scheme": "ed25519",
                    "threshold": 3,
                    "public_key_hex": "2222222222222222222222222222222222222222222222222222222222222222"
                },
                {
                    "slot_id": "governance.finality.v1",
                    "signer_id": "validator-c",
                    "scheme": "ed25519",
                    "threshold": 3,
                    "public_key_hex": "3333333333333333333333333333333333333333333333333333333333333333"
                },
                {
                    "slot_id": "governance.finality.v1",
                    "signer_id": "validator-d",
                    "scheme": "ed25519",
                    "threshold": 3,
                    "public_key_hex": "4444444444444444444444444444444444444444444444444444444444444444"
                }
            ]))
            .expect("encode manifest"),
        )
        .expect("write manifest");

        ensure_world_governance_validator_registry(
            temp_dir.as_path(),
            Some(manifest_path.as_path()),
            Some(&public_testnet_loaded_manifest()),
        )
        .expect("ensure genesis registry");
        let world = World::load_from_dir(temp_dir.as_path()).expect("load world");
        let registry = world
            .resolve_governance_effective_finality_signer_registry()
            .expect("resolve registry")
            .expect("registry");

        assert_eq!(registry.threshold, 3);
        assert_eq!(registry.signer_bindings.len(), 4);
    }

    #[test]
    fn genesis_validator_registry_rejects_existing_world_without_registry() {
        let temp_dir = temp_dir("genesis-existing-world");
        let world = World::new();
        world.save_to_dir(&temp_dir).expect("save existing world");
        let manifest_path = temp_dir.join("genesis-validator-registry.json");
        std::fs::write(
            manifest_path.as_path(),
            serde_json::to_vec_pretty(&serde_json::json!({
                "slot_id": "governance.finality.v1",
                "threshold": 1,
                "validators": [
                    {
                        "node_id": "validator-a",
                        "scheme": "ed25519",
                        "finality_signer_public_key": "1111111111111111111111111111111111111111111111111111111111111111",
                        "stake": 100
                    }
                ]
            }))
            .expect("encode manifest"),
        )
        .expect("write manifest");

        let err = ensure_world_governance_validator_registry(
            temp_dir.as_path(),
            Some(manifest_path.as_path()),
            Some(&public_testnet_loaded_manifest()),
        )
        .expect_err("existing world must use migration path");

        assert!(
            err.contains("refusing to import genesis validator registry"),
            "{err}"
        );
        assert!(
            err.contains("dedicated governance registry import"),
            "{err}"
        );
    }

    #[test]
    fn public_tier_without_world_registry_or_genesis_manifest_fails() {
        let temp_dir = temp_dir("public-missing-registry");
        std::fs::create_dir_all(&temp_dir).expect("create temp dir");

        let err = ensure_world_governance_validator_registry(
            temp_dir.as_path(),
            None,
            Some(&public_testnet_loaded_manifest()),
        )
        .expect_err("public tier must require registry truth");

        assert!(err.contains("public validator registry required"), "{err}");
        assert!(err.contains("public_testnet"), "{err}");
        assert!(err.contains("--genesis-validator-registry"), "{err}");
    }

    #[test]
    fn world_finality_registry_strips_slot_prefix_from_validator_ids() {
        let temp_dir = temp_dir("finality-prefix-override");
        let mut world = World::new();
        world
            .set_governance_finality_signer_registry(GovernanceFinalitySignerRegistry {
                slot_id: "governance.finality.v1".to_string(),
                threshold: 2,
                threshold_bps: 0,
                signer_bindings: BTreeMap::from([
                    (
                        "governance.finality.v1.triad-testnet-sequencer".to_string(),
                        "1111111111111111111111111111111111111111111111111111111111111111"
                            .to_string(),
                    ),
                    (
                        "governance.finality.v1.triad-testnet-storage".to_string(),
                        "2222222222222222222222222222222222222222222222222222222222222222"
                            .to_string(),
                    ),
                ]),
                validator_stakes: BTreeMap::from([
                    (
                        "governance.finality.v1.triad-testnet-sequencer".to_string(),
                        60,
                    ),
                    (
                        "governance.finality.v1.triad-testnet-storage".to_string(),
                        40,
                    ),
                ]),
            })
            .expect("set finality registry");
        world.save_to_dir(&temp_dir).expect("save execution world");

        let config = apply_world_governance_registry_overrides(
            NodeConfig::new("triad-testnet-sequencer", "world-a", NodeRole::Sequencer)
                .expect("node config"),
            &temp_dir,
        )
        .expect("apply registry overrides");

        let validator_ids = config
            .pos_config
            .validators
            .iter()
            .map(|validator| validator.validator_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            validator_ids,
            vec!["triad-testnet-sequencer", "triad-testnet-storage"]
        );
        assert_eq!(
            config
                .pos_config
                .validator_signer_public_keys
                .get("triad-testnet-sequencer")
                .map(String::as_str),
            Some("1111111111111111111111111111111111111111111111111111111111111111")
        );
        let validator_stakes = config
            .pos_config
            .validators
            .iter()
            .map(|validator| (validator.validator_id.as_str(), validator.stake))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(validator_stakes.get("triad-testnet-sequencer"), Some(&60));
        assert_eq!(validator_stakes.get("triad-testnet-storage"), Some(&40));
    }

    #[test]
    fn world_effective_finality_registry_overrides_node_pos_config_after_validator_activation() {
        let temp_dir = temp_dir("effective-finality-override");
        let mut world = World::new();
        world
            .set_governance_execution_policy(GovernanceExecutionPolicy {
                epoch_length_ticks: 10,
                ..GovernanceExecutionPolicy::default()
            })
            .expect("set governance policy");
        world
            .set_governance_finality_signer_registry(GovernanceFinalitySignerRegistry {
                slot_id: "governance.finality.v1".to_string(),
                threshold: 2,
                threshold_bps: 0,
                signer_bindings: BTreeMap::from([
                    (
                        "validator-a".to_string(),
                        "1111111111111111111111111111111111111111111111111111111111111111"
                            .to_string(),
                    ),
                    (
                        "validator-b".to_string(),
                        "2222222222222222222222222222222222222222222222222222222222222222"
                            .to_string(),
                    ),
                ]),
                validator_stakes: BTreeMap::from([
                    ("validator-a".to_string(), 100),
                    ("validator-b".to_string(), 100),
                ]),
            })
            .expect("set finality registry");
        world
            .set_governance_main_token_controller_registry(GovernanceMainTokenControllerRegistry {
                genesis_controller_account_id: "msig.genesis.v1".to_string(),
                treasury_bucket_controller_slots: BTreeMap::from([(
                    "ecosystem_pool".to_string(),
                    "liveops".to_string(),
                )]),
                restricted_starter_claim_admin_account_ids: BTreeSet::from(["liveops".to_string()]),
                controller_signer_policies: BTreeMap::from([
                    (
                        "msig.genesis.v1".to_string(),
                        GovernanceThresholdSignerPolicy {
                            threshold: 1,
                            allowed_public_keys: BTreeSet::from([
                                "6249e5a58278dbc4e629a16b5d33f6b84c39e3ceeb10e963bb9ef64ea4daac30"
                                    .to_string(),
                            ]),
                        },
                    ),
                    (
                        "liveops".to_string(),
                        GovernanceThresholdSignerPolicy {
                            threshold: 1,
                            allowed_public_keys: BTreeSet::from([
                                "13c160fc0f516b9a5663aa00c2a5446be6467f68ce341fdd79cdb64224dffd20"
                                    .to_string(),
                            ]),
                        },
                    ),
                ]),
            })
            .expect("set controller registry");
        world.submit_action(Action::SubmitGovernanceValidatorAdmission {
            controller_account_id: "msig.genesis.v1".to_string(),
            candidate_id: "candidate-c".to_string(),
            node_id: "validator-c".to_string(),
            finality_signer_public_key:
                "3333333333333333333333333333333333333333333333333333333333333333".to_string(),
            stake: 35,
            operator_owner: "ops.team".to_string(),
            public_manifest_hash: "manifest-c".to_string(),
        });
        world.step().expect("submit validator admission");
        world.submit_action(Action::ApproveGovernanceValidatorAdmission {
            controller_account_id: "msig.genesis.v1".to_string(),
            candidate_id: "candidate-c".to_string(),
        });
        world.step().expect("approve validator admission");
        world.submit_action(Action::ActivateGovernanceValidatorAdmission {
            controller_account_id: "msig.genesis.v1".to_string(),
            candidate_id: "candidate-c".to_string(),
            activation_epoch: 0,
        });
        world.step().expect("activate validator admission");
        world.save_to_dir(&temp_dir).expect("save execution world");

        let config =
            NodeConfig::new("node-a", "world-a", NodeRole::Sequencer).expect("node config");
        let config = apply_world_governance_registry_overrides(config, &temp_dir)
            .expect("apply registry overrides");

        let validator_ids = config
            .pos_config
            .validators
            .iter()
            .map(|validator| validator.validator_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            validator_ids,
            vec!["validator-a", "validator-b", "validator-c"]
        );
        assert_eq!(
            config
                .pos_config
                .validator_signer_public_keys
                .get("validator-c")
                .map(String::as_str),
            Some("3333333333333333333333333333333333333333333333333333333333333333")
        );
        assert_eq!(
            config
                .pos_config
                .validators
                .iter()
                .find(|validator| validator.validator_id == "validator-c")
                .map(|validator| validator.stake),
            Some(35)
        );
    }
}
