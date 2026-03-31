use super::helpers::{
    add_main_token_treasury_balance, main_token_fee_treasury_bucket,
    resolve_main_token_effective_config_for_epoch,
};
use super::*;
use crate::runtime::MainTokenTreasuryDistribution;
use crate::runtime::main_token::{
    is_main_token_treasury_distribution_bucket, validate_main_token_config_bounds,
};
use std::collections::BTreeSet;

impl WorldState {
    pub(super) fn apply_main_token_epoch_issued(
        &mut self,
        epoch_index: u64,
        inflation_rate_bps: u32,
        issued_amount: u64,
        staking_reward_amount: u64,
        node_service_reward_amount: u64,
        ecosystem_pool_amount: u64,
        security_reserve_amount: u64,
    ) -> Result<(), WorldError> {
        if self.main_token_epoch_issuance_records.contains_key(&epoch_index) {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!("main token epoch issuance already exists: epoch={epoch_index}"),
            });
        }
        let split_sum = staking_reward_amount
            .checked_add(node_service_reward_amount)
            .and_then(|value| value.checked_add(ecosystem_pool_amount))
            .and_then(|value| value.checked_add(security_reserve_amount))
            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token epoch split overflow: epoch={} staking={} node_service={} ecosystem={} security={}",
                    epoch_index,
                    staking_reward_amount,
                    node_service_reward_amount,
                    ecosystem_pool_amount,
                    security_reserve_amount
                ),
            })?;
        if split_sum != issued_amount {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token epoch split mismatch: epoch={} issued={} split_sum={}",
                    epoch_index, issued_amount, split_sum
                ),
            });
        }

        let next_total_issued = self
            .main_token_supply
            .total_issued
            .checked_add(issued_amount)
            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token total_issued overflow: current={} issued={}",
                    self.main_token_supply.total_issued, issued_amount
                ),
            })?;
        let next_total_supply = self
            .main_token_supply
            .total_supply
            .checked_add(issued_amount)
            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token total_supply overflow: current={} issued={}",
                    self.main_token_supply.total_supply, issued_amount
                ),
            })?;
        let effective_config = resolve_main_token_effective_config_for_epoch(self, epoch_index);
        if let Some(max_supply) = effective_config.max_supply {
            if next_total_supply > max_supply {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "main token total_supply exceeds max_supply: next={} max={}",
                        next_total_supply, max_supply
                    ),
                });
            }
        }

        add_main_token_treasury_balance(
            &mut self.main_token_treasury_balances,
            MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD,
            staking_reward_amount,
        )?;
        add_main_token_treasury_balance(
            &mut self.main_token_treasury_balances,
            MAIN_TOKEN_TREASURY_BUCKET_NODE_SERVICE_REWARD,
            node_service_reward_amount,
        )?;
        add_main_token_treasury_balance(
            &mut self.main_token_treasury_balances,
            MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL,
            ecosystem_pool_amount,
        )?;
        add_main_token_treasury_balance(
            &mut self.main_token_treasury_balances,
            MAIN_TOKEN_TREASURY_BUCKET_SECURITY_RESERVE,
            security_reserve_amount,
        )?;

        self.main_token_supply.total_issued = next_total_issued;
        self.main_token_supply.total_supply = next_total_supply;
        self.main_token_epoch_issuance_records.insert(
            epoch_index,
            MainTokenEpochIssuanceRecord {
                epoch_index,
                inflation_rate_bps,
                issued_amount,
                staking_reward_amount,
                node_service_reward_amount,
                ecosystem_pool_amount,
                security_reserve_amount,
            },
        );
        Ok(())
    }

    pub(super) fn apply_main_token_fee_settled(
        &mut self,
        fee_kind: MainTokenFeeKind,
        amount: u64,
        burn_amount: u64,
        treasury_amount: u64,
    ) -> Result<(), WorldError> {
        if amount == 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "main token fee amount must be > 0".to_string(),
            });
        }
        let settled_sum =
            burn_amount
                .checked_add(treasury_amount)
                .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "main token fee settled overflow: amount={} burn={} treasury={}",
                        amount, burn_amount, treasury_amount
                    ),
                })?;
        if settled_sum != amount {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token fee settled mismatch: amount={} burn={} treasury={}",
                    amount, burn_amount, treasury_amount
                ),
            });
        }
        if self.main_token_supply.circulating_supply < amount {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token circulating supply insufficient for fee settlement: circulating={} amount={}",
                    self.main_token_supply.circulating_supply, amount
                ),
            });
        }
        if self.main_token_supply.total_supply < burn_amount {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token total_supply insufficient for burn: total={} burn={}",
                    self.main_token_supply.total_supply, burn_amount
                ),
            });
        }

        self.main_token_supply.circulating_supply -= amount;
        self.main_token_supply.total_supply -= burn_amount;
        self.main_token_supply.total_burned = self
            .main_token_supply
            .total_burned
            .checked_add(burn_amount)
            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token total_burned overflow: current={} burn={}",
                    self.main_token_supply.total_burned, burn_amount
                ),
            })?;
        add_main_token_treasury_balance(
            &mut self.main_token_treasury_balances,
            main_token_fee_treasury_bucket(fee_kind),
            treasury_amount,
        )?;

        if self.main_token_supply.circulating_supply > self.main_token_supply.total_supply {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token circulating exceeds total supply after fee settlement: circulating={} total={}",
                    self.main_token_supply.circulating_supply, self.main_token_supply.total_supply
                ),
            });
        }
        Ok(())
    }

    pub(super) fn apply_main_token_policy_update_scheduled(
        &mut self,
        proposal_id: ProposalId,
        effective_epoch: u64,
        next: &MainTokenConfig,
        now: WorldTime,
    ) -> Result<(), WorldError> {
        if proposal_id == 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "main token policy proposal_id must be > 0".to_string(),
            });
        }
        if effective_epoch <= now {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token policy effective_epoch must be > now: effective={} now={}",
                    effective_epoch, now
                ),
            });
        }
        if let Err(reason) = validate_main_token_config_bounds(next) {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!("main token policy config out of bounds: {reason}"),
            });
        }
        if next.initial_supply != self.main_token_config.initial_supply {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token policy cannot change initial_supply: current={} next={}",
                    self.main_token_config.initial_supply, next.initial_supply
                ),
            });
        }
        if let Some(max_supply) = next.max_supply {
            if max_supply < self.main_token_supply.total_supply {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "main token policy max_supply cannot be below current total_supply: max={} total={}",
                        max_supply, self.main_token_supply.total_supply
                    ),
                });
            }
        }
        if self
            .main_token_scheduled_policy_updates
            .contains_key(&effective_epoch)
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token policy effective_epoch already scheduled: {}",
                    effective_epoch
                ),
            });
        }
        if self
            .main_token_scheduled_policy_updates
            .values()
            .any(|item| item.proposal_id == proposal_id)
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!("main token policy proposal already scheduled: {}", proposal_id),
            });
        }

        self.main_token_scheduled_policy_updates.insert(
            effective_epoch,
            MainTokenScheduledPolicyUpdate {
                proposal_id,
                effective_epoch,
                next_config: next.clone(),
            },
        );
        Ok(())
    }

    pub(super) fn apply_main_token_treasury_distributed(
        &mut self,
        proposal_id: ProposalId,
        distribution_id: &str,
        bucket_id: &str,
        total_amount: u64,
        distributions: &[MainTokenTreasuryDistribution],
        now: WorldTime,
    ) -> Result<(), WorldError> {
        if proposal_id == 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "main token treasury distribution proposal_id must be > 0".to_string(),
            });
        }
        let distribution_id = distribution_id.trim();
        if distribution_id.is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "main token treasury distribution_id cannot be empty".to_string(),
            });
        }
        let bucket_id = bucket_id.trim();
        if !is_main_token_treasury_distribution_bucket(bucket_id) {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token treasury distribution bucket is not allowed: {}",
                    bucket_id
                ),
            });
        }
        if total_amount == 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "main token treasury total_amount must be > 0".to_string(),
            });
        }
        if distributions.is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "main token treasury distribution list cannot be empty".to_string(),
            });
        }
        if self
            .main_token_treasury_distribution_records
            .contains_key(distribution_id)
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token treasury distribution_id already exists: {}",
                    distribution_id
                ),
            });
        }

        let mut seen_accounts = BTreeSet::new();
        let mut distributions_sum = 0_u64;
        for item in distributions {
            let account_id = item.account_id.trim();
            if account_id.is_empty() {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "main token treasury distribution account_id cannot be empty: distribution_id={}",
                        distribution_id
                    ),
                });
            }
            if item.amount == 0 {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "main token treasury distribution amount must be > 0: distribution_id={} account_id={}",
                        distribution_id, account_id
                    ),
                });
            }
            if !seen_accounts.insert(account_id.to_string()) {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "duplicate main token treasury distribution account_id: distribution_id={} account_id={}",
                        distribution_id, account_id
                    ),
                });
            }
            distributions_sum =
                distributions_sum
                    .checked_add(item.amount)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token treasury distribution sum overflow: distribution_id={}",
                            distribution_id
                        ),
                    })?;
        }
        if distributions_sum != total_amount {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token treasury distribution sum mismatch: distribution_id={} total={} sum={}",
                    distribution_id, total_amount, distributions_sum
                ),
            });
        }

        let bucket_balance = self
            .main_token_treasury_balances
            .get(bucket_id)
            .copied()
            .unwrap_or(0);
        if bucket_balance < total_amount {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token treasury bucket insufficient: bucket={} balance={} total={}",
                    bucket_id, bucket_balance, total_amount
                ),
            });
        }
        self.main_token_treasury_balances
            .insert(bucket_id.to_string(), bucket_balance - total_amount);

        for item in distributions {
            let account_id = item.account_id.trim();
            let account = self
                .main_token_balances
                .entry(account_id.to_string())
                .or_insert_with(|| MainTokenAccountBalance {
                    account_id: account_id.to_string(),
                    ..MainTokenAccountBalance::default()
                });
            if account.account_id != account_id {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "main token treasury account key mismatch: key={} value={}",
                        account_id, account.account_id
                    ),
                });
            }
            account.liquid_balance = account
                .liquid_balance
                .checked_add(item.amount)
                .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "main token treasury account overflow: account={} current={} amount={}",
                        account_id, account.liquid_balance, item.amount
                    ),
                })?;
        }

        self.main_token_supply.circulating_supply = self
            .main_token_supply
            .circulating_supply
            .checked_add(total_amount)
            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token circulating overflow: current={} amount={}",
                    self.main_token_supply.circulating_supply, total_amount
                ),
            })?;
        if self.main_token_supply.circulating_supply > self.main_token_supply.total_supply {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token circulating exceeds total: circulating={} total={}",
                    self.main_token_supply.circulating_supply, self.main_token_supply.total_supply
                ),
            });
        }

        self.main_token_treasury_distribution_records.insert(
            distribution_id.to_string(),
            MainTokenTreasuryDistributionRecord {
                proposal_id,
                distribution_id: distribution_id.to_string(),
                bucket_id: bucket_id.to_string(),
                total_amount,
                distributions: distributions.to_vec(),
                distributed_epoch: now,
            },
        );
        Ok(())
    }
}
