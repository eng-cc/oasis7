use std::collections::{BTreeMap, BTreeSet};

use crate::NodeError;

const DEFAULT_MAIN_TOKEN_GENESIS_CONTROLLER_ACCOUNT_ID: &str = "msig.genesis.v1";
const DEFAULT_MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD: &str = "staking_reward_pool";
const DEFAULT_MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL: &str = "ecosystem_pool";
const DEFAULT_MAIN_TOKEN_TREASURY_BUCKET_SECURITY_RESERVE: &str = "security_reserve";
const DEFAULT_MAIN_TOKEN_TREASURY_CONTROLLER_STAKING_GOVERNANCE: &str =
    "msig.staking_governance.v1";
const DEFAULT_MAIN_TOKEN_TREASURY_CONTROLLER_ECOSYSTEM_GOVERNANCE: &str =
    "msig.ecosystem_governance.v1";
const DEFAULT_MAIN_TOKEN_TREASURY_CONTROLLER_SECURITY_COUNCIL: &str = "msig.security_council.v1";
const DEFAULT_MAIN_TOKEN_CONTROLLER_SIGNER_THRESHOLD: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeMainTokenControllerBindingConfig {
    pub genesis_controller_account_id: String,
    pub treasury_bucket_controller_slots: BTreeMap<String, String>,
    pub controller_signer_policies: BTreeMap<String, NodeMainTokenControllerSignerPolicy>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeMainTokenControllerSignerPolicy {
    pub threshold: u16,
    pub allowed_public_keys: BTreeSet<String>,
}

impl Default for NodeMainTokenControllerSignerPolicy {
    fn default() -> Self {
        Self {
            threshold: DEFAULT_MAIN_TOKEN_CONTROLLER_SIGNER_THRESHOLD,
            allowed_public_keys: BTreeSet::new(),
        }
    }
}

impl Default for NodeMainTokenControllerBindingConfig {
    fn default() -> Self {
        let mut treasury_bucket_controller_slots = BTreeMap::new();
        let mut controller_signer_policies = BTreeMap::new();
        treasury_bucket_controller_slots.insert(
            DEFAULT_MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD.to_string(),
            DEFAULT_MAIN_TOKEN_TREASURY_CONTROLLER_STAKING_GOVERNANCE.to_string(),
        );
        controller_signer_policies.insert(
            DEFAULT_MAIN_TOKEN_TREASURY_CONTROLLER_STAKING_GOVERNANCE.to_string(),
            NodeMainTokenControllerSignerPolicy::default(),
        );
        treasury_bucket_controller_slots.insert(
            DEFAULT_MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL.to_string(),
            DEFAULT_MAIN_TOKEN_TREASURY_CONTROLLER_ECOSYSTEM_GOVERNANCE.to_string(),
        );
        controller_signer_policies.insert(
            DEFAULT_MAIN_TOKEN_TREASURY_CONTROLLER_ECOSYSTEM_GOVERNANCE.to_string(),
            NodeMainTokenControllerSignerPolicy::default(),
        );
        treasury_bucket_controller_slots.insert(
            DEFAULT_MAIN_TOKEN_TREASURY_BUCKET_SECURITY_RESERVE.to_string(),
            DEFAULT_MAIN_TOKEN_TREASURY_CONTROLLER_SECURITY_COUNCIL.to_string(),
        );
        controller_signer_policies.insert(
            DEFAULT_MAIN_TOKEN_TREASURY_CONTROLLER_SECURITY_COUNCIL.to_string(),
            NodeMainTokenControllerSignerPolicy::default(),
        );
        controller_signer_policies.insert(
            DEFAULT_MAIN_TOKEN_GENESIS_CONTROLLER_ACCOUNT_ID.to_string(),
            NodeMainTokenControllerSignerPolicy::default(),
        );
        Self {
            genesis_controller_account_id: DEFAULT_MAIN_TOKEN_GENESIS_CONTROLLER_ACCOUNT_ID
                .to_string(),
            treasury_bucket_controller_slots,
            controller_signer_policies,
        }
    }
}

impl NodeMainTokenControllerBindingConfig {
    pub fn with_genesis_controller_account_id(
        mut self,
        account_id: impl Into<String>,
    ) -> Result<Self, NodeError> {
        self.genesis_controller_account_id = normalize_controller_slot_id(
            account_id.into().as_str(),
            "main_token_controller_binding.genesis_controller_account_id",
        )?;
        Ok(self)
    }

    pub fn with_treasury_bucket_controller_slot(
        mut self,
        bucket_id: impl Into<String>,
        controller_account_id: impl Into<String>,
    ) -> Result<Self, NodeError> {
        let bucket_id = normalize_controller_slot_id(
            bucket_id.into().as_str(),
            "main_token_controller_binding.treasury bucket_id",
        )?;
        let controller_account_id = normalize_controller_slot_id(
            controller_account_id.into().as_str(),
            "main_token_controller_binding.treasury controller_account_id",
        )?;
        self.treasury_bucket_controller_slots
            .insert(bucket_id, controller_account_id);
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), NodeError> {
        normalize_controller_slot_id(
            self.genesis_controller_account_id.as_str(),
            "main_token_controller_binding.genesis_controller_account_id",
        )?;
        for (bucket_id, controller_account_id) in &self.treasury_bucket_controller_slots {
            normalize_controller_slot_id(
                bucket_id.as_str(),
                "main_token_controller_binding.treasury bucket_id",
            )?;
            normalize_controller_slot_id(
                controller_account_id.as_str(),
                "main_token_controller_binding.treasury controller_account_id",
            )?;
        }
        for (controller_account_id, policy) in &self.controller_signer_policies {
            normalize_controller_slot_id(
                controller_account_id.as_str(),
                "main_token_controller_binding.controller_signer_policies account_id",
            )?;
            validate_controller_signer_policy(policy, controller_account_id.as_str())?;
        }
        Ok(())
    }

    pub fn with_controller_signer_policy(
        mut self,
        controller_account_id: impl Into<String>,
        threshold: u16,
        allowed_public_keys: Vec<String>,
    ) -> Result<Self, NodeError> {
        let controller_account_id = normalize_controller_slot_id(
            controller_account_id.into().as_str(),
            "main_token_controller_binding.controller_signer_policies account_id",
        )?;
        let policy = NodeMainTokenControllerSignerPolicy::new(threshold, allowed_public_keys)?;
        self.controller_signer_policies
            .insert(controller_account_id, policy);
        Ok(self)
    }
}

impl NodeMainTokenControllerSignerPolicy {
    pub fn new(threshold: u16, allowed_public_keys: Vec<String>) -> Result<Self, NodeError> {
        if threshold == 0 {
            return Err(NodeError::InvalidConfig {
                reason: "main_token_controller_binding signer threshold must be > 0".to_string(),
            });
        }
        let mut normalized = BTreeSet::new();
        for public_key in allowed_public_keys {
            let public_key = normalize_ed25519_public_key_hex(
                public_key.as_str(),
                "main_token_controller_binding signer public key",
            )?;
            normalized.insert(public_key);
        }
        Ok(Self {
            threshold,
            allowed_public_keys: normalized,
        })
    }
}

fn normalize_controller_slot_id(raw: &str, label: &str) -> Result<String, NodeError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(NodeError::InvalidConfig {
            reason: format!("{label} cannot be empty"),
        });
    }
    Ok(value.to_string())
}

fn validate_controller_signer_policy(
    policy: &NodeMainTokenControllerSignerPolicy,
    controller_account_id: &str,
) -> Result<(), NodeError> {
    if policy.threshold == 0 {
        return Err(NodeError::InvalidConfig {
            reason: format!(
                "main_token controller signer policy threshold must be > 0: controller_account_id={controller_account_id}"
            ),
        });
    }
    for public_key in &policy.allowed_public_keys {
        normalize_ed25519_public_key_hex(
            public_key.as_str(),
            "main_token_controller_binding signer public key",
        )?;
    }
    Ok(())
}

fn normalize_ed25519_public_key_hex(raw: &str, label: &str) -> Result<String, NodeError> {
    let normalized = normalize_controller_slot_id(raw, label)?;
    let bytes = hex::decode(normalized.as_str()).map_err(|err| NodeError::InvalidConfig {
        reason: format!("decode {label} failed: {err}"),
    })?;
    if bytes.len() != 32 {
        return Err(NodeError::InvalidConfig {
            reason: format!(
                "{label} length mismatch: expected 32 bytes, got {}",
                bytes.len()
            ),
        });
    }
    Ok(hex::encode(bytes))
}
