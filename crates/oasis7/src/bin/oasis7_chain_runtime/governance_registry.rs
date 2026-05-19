use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use oasis7::runtime::{
    GovernanceFinalitySignerRegistry, GovernanceMainTokenControllerRegistry,
    GovernanceThresholdSignerPolicy,
};
use oasis7_node::{
    NodeConfig, NodeMainTokenControllerBindingConfig, NodeMainTokenControllerSignerPolicy,
    NodePosConfig, PosValidator,
};

const WORLD_REGISTRY_EQUAL_VALIDATOR_STAKE: u64 = 100;

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
    let validators = validator_signer_public_keys
        .keys()
        .cloned()
        .map(|validator_id| PosValidator {
            validator_id,
            stake: WORLD_REGISTRY_EQUAL_VALIDATOR_STAKE,
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
    use super::apply_world_governance_registry_overrides;
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
        assert!(config
            .pos_config
            .validators
            .iter()
            .all(|validator| validator.stake == 100));
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
    }
}
