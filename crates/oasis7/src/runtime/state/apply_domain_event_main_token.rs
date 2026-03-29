use super::super::events::MainTokenFeeKind;
use super::super::main_token::{
    is_main_token_treasury_distribution_bucket, validate_main_token_config_bounds,
    MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL, MAIN_TOKEN_TREASURY_BUCKET_GAS_FEE,
    MAIN_TOKEN_TREASURY_BUCKET_MODULE_FEE, MAIN_TOKEN_TREASURY_BUCKET_NODE_SERVICE_REWARD,
    MAIN_TOKEN_TREASURY_BUCKET_SECURITY_RESERVE, MAIN_TOKEN_TREASURY_BUCKET_SLASH,
    MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD,
};
use super::*;
use std::collections::BTreeSet;

impl WorldState {
    pub(super) fn apply_domain_event_main_token(
        &mut self,
        event: &DomainEvent,
        now: WorldTime,
    ) -> Result<(), WorldError> {
        match event {
            DomainEvent::MainTokenGenesisInitialized {
                total_supply,
                allocations,
            } => {
                if *total_supply == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "main token genesis total_supply must be > 0".to_string(),
                    });
                }
                if allocations.is_empty() {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "main token genesis allocations cannot be empty".to_string(),
                    });
                }
                if !self.main_token_genesis_buckets.is_empty() {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "main token genesis already initialized".to_string(),
                    });
                }
                if !self.main_token_balances.is_empty()
                    || !self.main_token_treasury_balances.is_empty()
                    || !self.main_token_claim_nonces.is_empty()
                    || !self.main_token_transfer_nonces.is_empty()
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "main token ledger is not empty during genesis initialization"
                            .to_string(),
                    });
                }
                if self.main_token_supply.total_supply > 0
                    || self.main_token_supply.total_issued > 0
                    || self.main_token_supply.total_burned > 0
                    || self.main_token_supply.circulating_supply > 0
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "main token supply already initialized".to_string(),
                    });
                }

                let mut ratio_sum = 0_u64;
                let mut allocated_sum = 0_u64;
                let mut buckets = BTreeMap::new();
                let mut recipient_vested = BTreeMap::<String, u64>::new();
                for allocation in allocations {
                    if allocation.bucket_id.trim().is_empty() {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: "main token allocation bucket_id cannot be empty".to_string(),
                        });
                    }
                    if allocation.recipient.trim().is_empty() {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "main token allocation recipient cannot be empty: bucket={}",
                                allocation.bucket_id
                            ),
                        });
                    }
                    if allocation.ratio_bps == 0 {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "main token allocation ratio must be > 0: bucket={}",
                                allocation.bucket_id
                            ),
                        });
                    }
                    if allocation.claimed_amount != 0 {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "main token allocation claimed_amount must be 0 at genesis: bucket={}",
                                allocation.bucket_id
                            ),
                        });
                    }
                    ratio_sum = ratio_sum.saturating_add(u64::from(allocation.ratio_bps));
                    allocated_sum = allocated_sum.saturating_add(allocation.allocated_amount);
                    if buckets
                        .insert(allocation.bucket_id.clone(), allocation.clone())
                        .is_some()
                    {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "duplicate main token allocation bucket_id: {}",
                                allocation.bucket_id
                            ),
                        });
                    }
                    recipient_vested
                        .entry(allocation.recipient.clone())
                        .and_modify(|value| {
                            *value = value.saturating_add(allocation.allocated_amount);
                        })
                        .or_insert(allocation.allocated_amount);
                }
                if ratio_sum != 10_000 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token allocation ratio sum must be 10000 bps, got {}",
                            ratio_sum
                        ),
                    });
                }
                if allocated_sum != *total_supply {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token allocation sum mismatch: allocated={} total_supply={}",
                            allocated_sum, total_supply
                        ),
                    });
                }

                self.main_token_supply = MainTokenSupplyState {
                    total_supply: *total_supply,
                    circulating_supply: 0,
                    total_issued: 0,
                    total_burned: 0,
                };
                self.main_token_genesis_buckets = buckets;
                for (recipient, vested_amount) in recipient_vested {
                    self.main_token_balances.insert(
                        recipient.clone(),
                        MainTokenAccountBalance {
                            account_id: recipient,
                            liquid_balance: 0,
                            vested_balance: vested_amount,
                            restricted_starter_claim_balance: 0,
                        },
                    );
                }
            }
            DomainEvent::MainTokenVestingClaimed {
                bucket_id,
                beneficiary,
                amount,
                nonce,
            } => {
                if *amount == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "main token vesting claim amount must be > 0".to_string(),
                    });
                }
                if *nonce == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "main token vesting claim nonce must be > 0".to_string(),
                    });
                }
                if let Some(last_nonce) = self.main_token_claim_nonces.get(beneficiary) {
                    if *nonce <= *last_nonce {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "main token vesting claim nonce replay: beneficiary={} nonce={} last_nonce={}",
                                beneficiary, nonce, last_nonce
                            ),
                        });
                    }
                }
                let bucket = self
                    .main_token_genesis_buckets
                    .get(bucket_id)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!("main token genesis bucket not found: {bucket_id}"),
                    })?;
                if bucket.recipient != *beneficiary {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token vesting beneficiary mismatch: bucket recipient={} beneficiary={}",
                            bucket.recipient, beneficiary
                        ),
                    });
                }
                let unlocked_amount = main_token_bucket_unlocked_amount(bucket, now);
                let releasable = unlocked_amount.saturating_sub(bucket.claimed_amount);
                if releasable == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token vesting has no releasable amount: bucket={} epoch={}",
                            bucket_id, now
                        ),
                    });
                }
                if *amount != releasable {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token vesting claim amount mismatch: expected={} actual={}",
                            releasable, amount
                        ),
                    });
                }

                let account = self
                    .main_token_balances
                    .get_mut(beneficiary)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token beneficiary account not found: {}",
                            beneficiary
                        ),
                    })?;
                if account.vested_balance < *amount {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token vested balance insufficient: beneficiary={} vested={} claim={}",
                            beneficiary, account.vested_balance, amount
                        ),
                    });
                }
                account.vested_balance -= *amount;
                account.liquid_balance =
                    account
                        .liquid_balance
                        .checked_add(*amount)
                        .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "main token liquid balance overflow: beneficiary={} current={} claim={}",
                                beneficiary, account.liquid_balance, amount
                            ),
                        })?;

                let bucket = self
                    .main_token_genesis_buckets
                    .get_mut(bucket_id)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!("main token genesis bucket not found: {bucket_id}"),
                    })?;
                bucket.claimed_amount =
                    bucket.claimed_amount.checked_add(*amount).ok_or_else(|| {
                        WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "main token claimed amount overflow: bucket={} current={} claim={}",
                                bucket_id, bucket.claimed_amount, amount
                            ),
                        }
                    })?;
                if bucket.claimed_amount > bucket.allocated_amount {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token claimed exceeds allocation: bucket={} claimed={} allocated={}",
                            bucket_id, bucket.claimed_amount, bucket.allocated_amount
                        ),
                    });
                }

                self.main_token_supply.circulating_supply = self
                    .main_token_supply
                    .circulating_supply
                    .checked_add(*amount)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token circulating supply overflow: current={} delta={}",
                            self.main_token_supply.circulating_supply, amount
                        ),
                    })?;
                if self.main_token_supply.circulating_supply > self.main_token_supply.total_supply {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token circulating exceeds total supply: circulating={} total={}",
                            self.main_token_supply.circulating_supply,
                            self.main_token_supply.total_supply
                        ),
                    });
                }
                self.main_token_claim_nonces
                    .insert(beneficiary.clone(), *nonce);
            }
            DomainEvent::MainTokenTransferred {
                from_account_id,
                to_account_id,
                amount,
                nonce,
            } => {
                let from_account_id = from_account_id.trim();
                let to_account_id = to_account_id.trim();
                if from_account_id.is_empty() {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "main token transfer from_account_id cannot be empty".to_string(),
                    });
                }
                if to_account_id.is_empty() {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "main token transfer to_account_id cannot be empty".to_string(),
                    });
                }
                if from_account_id == to_account_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token transfer from_account_id and to_account_id cannot be the same: {}",
                            from_account_id
                        ),
                    });
                }
                if *amount == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "main token transfer amount must be > 0".to_string(),
                    });
                }
                if *nonce == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "main token transfer nonce must be > 0".to_string(),
                    });
                }
                if let Some(last_nonce) = self.main_token_transfer_nonces.get(from_account_id) {
                    if *nonce <= *last_nonce {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "main token transfer nonce replay: from_account_id={} nonce={} last_nonce={}",
                                from_account_id, nonce, last_nonce
                            ),
                        });
                    }
                }

                let from_before = self
                    .main_token_balances
                    .get(from_account_id)
                    .cloned()
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token transfer source account not found: {}",
                            from_account_id
                        ),
                    })?;
                if from_before.account_id != from_account_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token transfer source account key mismatch: key={} value={}",
                            from_account_id, from_before.account_id
                        ),
                    });
                }
                if from_before.liquid_balance < *amount {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token transfer source balance insufficient: from_account_id={} balance={} amount={}",
                            from_account_id, from_before.liquid_balance, amount
                        ),
                    });
                }

                let to_before = self.main_token_balances.get(to_account_id).cloned();
                if let Some(account) = to_before.as_ref() {
                    if account.account_id != to_account_id {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "main token transfer target account key mismatch: key={} value={}",
                                to_account_id, account.account_id
                            ),
                        });
                    }
                }

                let next_from_liquid = from_before.liquid_balance - *amount;
                let next_to_liquid = to_before
                    .as_ref()
                    .map(|account| account.liquid_balance)
                    .unwrap_or(0)
                    .checked_add(*amount)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token transfer target balance overflow: to_account_id={} amount={}",
                            to_account_id, amount
                        ),
                    })?;

                let from_account = self
                    .main_token_balances
                    .get_mut(from_account_id)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token transfer source account not found: {}",
                            from_account_id
                        ),
                    })?;
                from_account.liquid_balance = next_from_liquid;

                let to_account = self
                    .main_token_balances
                    .entry(to_account_id.to_string())
                    .or_insert_with(|| MainTokenAccountBalance {
                        account_id: to_account_id.to_string(),
                        ..MainTokenAccountBalance::default()
                    });
                to_account.liquid_balance = next_to_liquid;
                self.main_token_transfer_nonces
                    .insert(from_account_id.to_string(), *nonce);
            }
            DomainEvent::MainTokenEpochIssued {
                epoch_index,
                inflation_rate_bps,
                issued_amount,
                staking_reward_amount,
                node_service_reward_amount,
                ecosystem_pool_amount,
                security_reserve_amount,
            } => {
                if self
                    .main_token_epoch_issuance_records
                    .contains_key(epoch_index)
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token epoch issuance already exists: epoch={epoch_index}"
                        ),
                    });
                }
                let split_sum = staking_reward_amount
                    .checked_add(*node_service_reward_amount)
                    .and_then(|value| value.checked_add(*ecosystem_pool_amount))
                    .and_then(|value| value.checked_add(*security_reserve_amount))
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
                if split_sum != *issued_amount {
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
                    .checked_add(*issued_amount)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token total_issued overflow: current={} issued={}",
                            self.main_token_supply.total_issued, issued_amount
                        ),
                    })?;
                let next_total_supply = self
                    .main_token_supply
                    .total_supply
                    .checked_add(*issued_amount)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token total_supply overflow: current={} issued={}",
                            self.main_token_supply.total_supply, issued_amount
                        ),
                    })?;
                let effective_config =
                    resolve_main_token_effective_config_for_epoch(self, *epoch_index);
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
                    *staking_reward_amount,
                )?;
                add_main_token_treasury_balance(
                    &mut self.main_token_treasury_balances,
                    MAIN_TOKEN_TREASURY_BUCKET_NODE_SERVICE_REWARD,
                    *node_service_reward_amount,
                )?;
                add_main_token_treasury_balance(
                    &mut self.main_token_treasury_balances,
                    MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL,
                    *ecosystem_pool_amount,
                )?;
                add_main_token_treasury_balance(
                    &mut self.main_token_treasury_balances,
                    MAIN_TOKEN_TREASURY_BUCKET_SECURITY_RESERVE,
                    *security_reserve_amount,
                )?;

                self.main_token_supply.total_issued = next_total_issued;
                self.main_token_supply.total_supply = next_total_supply;
                self.main_token_epoch_issuance_records.insert(
                    *epoch_index,
                    MainTokenEpochIssuanceRecord {
                        epoch_index: *epoch_index,
                        inflation_rate_bps: *inflation_rate_bps,
                        issued_amount: *issued_amount,
                        staking_reward_amount: *staking_reward_amount,
                        node_service_reward_amount: *node_service_reward_amount,
                        ecosystem_pool_amount: *ecosystem_pool_amount,
                        security_reserve_amount: *security_reserve_amount,
                    },
                );
            }
            DomainEvent::MainTokenFeeSettled {
                fee_kind,
                amount,
                burn_amount,
                treasury_amount,
            } => {
                if *amount == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "main token fee amount must be > 0".to_string(),
                    });
                }
                let settled_sum = burn_amount.checked_add(*treasury_amount).ok_or_else(|| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token fee settled overflow: amount={} burn={} treasury={}",
                            amount, burn_amount, treasury_amount
                        ),
                    }
                })?;
                if settled_sum != *amount {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token fee settled mismatch: amount={} burn={} treasury={}",
                            amount, burn_amount, treasury_amount
                        ),
                    });
                }
                if self.main_token_supply.circulating_supply < *amount {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token circulating supply insufficient for fee settlement: circulating={} amount={}",
                            self.main_token_supply.circulating_supply, amount
                        ),
                    });
                }
                if self.main_token_supply.total_supply < *burn_amount {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token total_supply insufficient for burn: total={} burn={}",
                            self.main_token_supply.total_supply, burn_amount
                        ),
                    });
                }

                self.main_token_supply.circulating_supply -= *amount;
                self.main_token_supply.total_supply -= *burn_amount;
                self.main_token_supply.total_burned = self
                    .main_token_supply
                    .total_burned
                    .checked_add(*burn_amount)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token total_burned overflow: current={} burn={}",
                            self.main_token_supply.total_burned, burn_amount
                        ),
                    })?;
                add_main_token_treasury_balance(
                    &mut self.main_token_treasury_balances,
                    main_token_fee_treasury_bucket(*fee_kind),
                    *treasury_amount,
                )?;

                if self.main_token_supply.circulating_supply > self.main_token_supply.total_supply {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token circulating exceeds total supply after fee settlement: circulating={} total={}",
                            self.main_token_supply.circulating_supply,
                            self.main_token_supply.total_supply
                        ),
                    });
                }
            }
            DomainEvent::MainTokenPolicyUpdateScheduled {
                proposal_id,
                effective_epoch,
                next,
            } => {
                if *proposal_id == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "main token policy proposal_id must be > 0".to_string(),
                    });
                }
                if *effective_epoch <= now {
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
                    .contains_key(effective_epoch)
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
                    .any(|item| item.proposal_id == *proposal_id)
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token policy proposal already scheduled: {}",
                            proposal_id
                        ),
                    });
                }

                self.main_token_scheduled_policy_updates.insert(
                    *effective_epoch,
                    MainTokenScheduledPolicyUpdate {
                        proposal_id: *proposal_id,
                        effective_epoch: *effective_epoch,
                        next_config: next.clone(),
                    },
                );
            }
            DomainEvent::MainTokenTreasuryDistributed {
                proposal_id,
                distribution_id,
                bucket_id,
                total_amount,
                distributions,
            } => {
                if *proposal_id == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: "main token treasury distribution proposal_id must be > 0"
                            .to_string(),
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
                if *total_amount == 0 {
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
                        distributions_sum.checked_add(item.amount).ok_or_else(|| {
                            WorldError::ResourceBalanceInvalid {
                                reason: format!(
                                "main token treasury distribution sum overflow: distribution_id={}",
                                distribution_id
                            ),
                            }
                        })?;
                }
                if distributions_sum != *total_amount {
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
                if bucket_balance < *total_amount {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token treasury bucket insufficient: bucket={} balance={} total={}",
                            bucket_id, bucket_balance, total_amount
                        ),
                    });
                }
                self.main_token_treasury_balances
                    .insert(bucket_id.to_string(), bucket_balance - *total_amount);

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
                    account.liquid_balance =
                        account
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
                    .checked_add(*total_amount)
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
                            self.main_token_supply.circulating_supply,
                            self.main_token_supply.total_supply
                        ),
                    });
                }

                self.main_token_treasury_distribution_records.insert(
                    distribution_id.to_string(),
                    MainTokenTreasuryDistributionRecord {
                        proposal_id: *proposal_id,
                        distribution_id: distribution_id.to_string(),
                        bucket_id: bucket_id.to_string(),
                        total_amount: *total_amount,
                        distributions: distributions.clone(),
                        distributed_epoch: now,
                    },
                );
            }
            _ => unreachable!("apply_domain_event_main_token received unsupported event variant"),
        }
        Ok(())
    }
}

fn add_main_token_treasury_balance(
    balances: &mut BTreeMap<String, u64>,
    bucket_id: &str,
    amount: u64,
) -> Result<(), WorldError> {
    let next = balances
        .get(bucket_id)
        .copied()
        .unwrap_or(0)
        .checked_add(amount)
        .ok_or_else(|| WorldError::ResourceBalanceInvalid {
            reason: format!(
                "main token treasury balance overflow: bucket={} amount={}",
                bucket_id, amount
            ),
        })?;
    balances.insert(bucket_id.to_string(), next);
    Ok(())
}

fn main_token_fee_treasury_bucket(fee_kind: MainTokenFeeKind) -> &'static str {
    match fee_kind {
        MainTokenFeeKind::GasBaseFee => MAIN_TOKEN_TREASURY_BUCKET_GAS_FEE,
        MainTokenFeeKind::SlashPenalty => MAIN_TOKEN_TREASURY_BUCKET_SLASH,
        MainTokenFeeKind::ModuleFee => MAIN_TOKEN_TREASURY_BUCKET_MODULE_FEE,
    }
}

fn resolve_main_token_effective_config_for_epoch(
    state: &WorldState,
    epoch_index: u64,
) -> &MainTokenConfig {
    state
        .main_token_scheduled_policy_updates
        .range(..=epoch_index)
        .next_back()
        .map(|(_, item)| &item.next_config)
        .unwrap_or(&state.main_token_config)
}
