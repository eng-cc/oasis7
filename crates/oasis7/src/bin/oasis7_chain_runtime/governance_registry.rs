use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use oasis7::runtime::{GovernanceMainTokenControllerRegistry, GovernanceThresholdSignerPolicy};
use oasis7_node::{
    NodeConfig, NodeMainTokenControllerBindingConfig, NodeMainTokenControllerSignerPolicy,
};

pub(super) fn apply_world_governance_registry_overrides(
    config: NodeConfig,
    execution_world_dir: &Path,
) -> Result<NodeConfig, String> {
    let world = super::execution_bridge::load_execution_world(execution_world_dir)?;
    let Some(registry) = world.governance_main_token_controller_registry() else {
        return Ok(config);
    };
    let binding = node_main_token_controller_binding_from_registry(
        registry,
        config.main_token_controller_binding.clone(),
    );
    config
        .with_main_token_controller_binding(binding)
        .map_err(|err| format!("failed to apply world governance controller registry: {err:?}"))
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
}
