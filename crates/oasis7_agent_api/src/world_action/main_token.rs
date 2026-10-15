const DEFAULT_MAIN_TOKEN_SYMBOL: &str = "OC";
const DEFAULT_MAIN_TOKEN_DECIMALS: u8 = 9;
const DEFAULT_MAIN_TOKEN_BASE_RATE_BPS: u32 = 400;
const DEFAULT_MAIN_TOKEN_MIN_RATE_BPS: u32 = 200;
const DEFAULT_MAIN_TOKEN_MAX_RATE_BPS: u32 = 800;
const DEFAULT_MAIN_TOKEN_TARGET_STAKE_RATIO_BPS: u32 = 6_000;
const DEFAULT_MAIN_TOKEN_STAKE_FEEDBACK_GAIN_BPS: u32 = 1_000;
const DEFAULT_MAIN_TOKEN_EPOCHS_PER_YEAR: u32 = 365;
const DEFAULT_MAIN_TOKEN_STAKING_REWARD_BPS: u32 = 6_000;
const DEFAULT_MAIN_TOKEN_NODE_SERVICE_REWARD_BPS: u32 = 2_000;
const DEFAULT_MAIN_TOKEN_ECOSYSTEM_POOL_BPS: u32 = 1_500;
const DEFAULT_MAIN_TOKEN_SECURITY_RESERVE_BPS: u32 = 500;
const DEFAULT_MAIN_TOKEN_GAS_BASE_FEE_BURN_BPS: u32 = 3_000;
const DEFAULT_MAIN_TOKEN_SLASH_BURN_BPS: u32 = 5_000;
const DEFAULT_MAIN_TOKEN_MODULE_FEE_BURN_BPS: u32 = 2_000;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainTokenConfig {
    pub symbol: String,
    pub decimals: u8,
    pub initial_supply: u64,
    pub max_supply: Option<u64>,
    pub inflation_policy: MainTokenInflationPolicy,
    pub issuance_split: MainTokenIssuanceSplitPolicy,
    pub burn_policy: MainTokenBurnPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainTokenInflationPolicy {
    pub base_rate_bps: u32,
    pub min_rate_bps: u32,
    pub max_rate_bps: u32,
    pub target_stake_ratio_bps: u32,
    pub stake_feedback_gain_bps: u32,
    pub epochs_per_year: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainTokenIssuanceSplitPolicy {
    pub staking_reward_bps: u32,
    pub node_service_reward_bps: u32,
    pub ecosystem_pool_bps: u32,
    pub security_reserve_bps: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MainTokenBurnPolicy {
    pub gas_base_fee_burn_bps: u32,
    pub slash_burn_bps: u32,
    pub module_fee_burn_bps: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MainTokenGenesisAllocationPlan {
    pub bucket_id: String,
    pub ratio_bps: u32,
    pub recipient: String,
    pub cliff_epochs: u64,
    pub linear_unlock_epochs: u64,
    pub start_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MainTokenTreasuryDistribution {
    pub account_id: String,
    pub amount: u64,
}

impl Default for MainTokenConfig {
    fn default() -> Self {
        Self {
            symbol: DEFAULT_MAIN_TOKEN_SYMBOL.to_string(),
            decimals: DEFAULT_MAIN_TOKEN_DECIMALS,
            initial_supply: 0,
            max_supply: None,
            inflation_policy: MainTokenInflationPolicy::default(),
            issuance_split: MainTokenIssuanceSplitPolicy::default(),
            burn_policy: MainTokenBurnPolicy::default(),
        }
    }
}

impl Default for MainTokenInflationPolicy {
    fn default() -> Self {
        Self {
            base_rate_bps: DEFAULT_MAIN_TOKEN_BASE_RATE_BPS,
            min_rate_bps: DEFAULT_MAIN_TOKEN_MIN_RATE_BPS,
            max_rate_bps: DEFAULT_MAIN_TOKEN_MAX_RATE_BPS,
            target_stake_ratio_bps: DEFAULT_MAIN_TOKEN_TARGET_STAKE_RATIO_BPS,
            stake_feedback_gain_bps: DEFAULT_MAIN_TOKEN_STAKE_FEEDBACK_GAIN_BPS,
            epochs_per_year: DEFAULT_MAIN_TOKEN_EPOCHS_PER_YEAR,
        }
    }
}

impl Default for MainTokenIssuanceSplitPolicy {
    fn default() -> Self {
        Self {
            staking_reward_bps: DEFAULT_MAIN_TOKEN_STAKING_REWARD_BPS,
            node_service_reward_bps: DEFAULT_MAIN_TOKEN_NODE_SERVICE_REWARD_BPS,
            ecosystem_pool_bps: DEFAULT_MAIN_TOKEN_ECOSYSTEM_POOL_BPS,
            security_reserve_bps: DEFAULT_MAIN_TOKEN_SECURITY_RESERVE_BPS,
        }
    }
}

impl Default for MainTokenBurnPolicy {
    fn default() -> Self {
        Self {
            gas_base_fee_burn_bps: DEFAULT_MAIN_TOKEN_GAS_BASE_FEE_BURN_BPS,
            slash_burn_bps: DEFAULT_MAIN_TOKEN_SLASH_BURN_BPS,
            module_fee_burn_bps: DEFAULT_MAIN_TOKEN_MODULE_FEE_BURN_BPS,
        }
    }
}
