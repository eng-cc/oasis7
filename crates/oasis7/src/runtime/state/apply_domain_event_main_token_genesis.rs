use super::*;

impl WorldState {
    pub(super) fn apply_main_token_genesis_initialized(
        &mut self,
        total_supply: u64,
        allocations: &[MainTokenGenesisAllocationBucketState],
    ) -> Result<(), WorldError> {
        if total_supply == 0 {
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
                reason: "main token ledger is not empty during genesis initialization".to_string(),
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
        if allocated_sum != total_supply {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token allocation sum mismatch: allocated={} total_supply={}",
                    allocated_sum, total_supply
                ),
            });
        }

        self.main_token_supply = MainTokenSupplyState {
            total_supply,
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
        Ok(())
    }

    pub(super) fn apply_main_token_vesting_claimed(
        &mut self,
        bucket_id: &str,
        beneficiary: &str,
        amount: u64,
        nonce: u64,
        now: WorldTime,
    ) -> Result<(), WorldError> {
        if amount == 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "main token vesting claim amount must be > 0".to_string(),
            });
        }
        if nonce == 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "main token vesting claim nonce must be > 0".to_string(),
            });
        }
        if let Some(last_nonce) = self.main_token_claim_nonces.get(beneficiary) {
            if nonce <= *last_nonce {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "main token vesting claim nonce replay: beneficiary={} nonce={} last_nonce={}",
                        beneficiary, nonce, last_nonce
                    ),
                });
            }
        }
        let bucket =
            self.main_token_genesis_buckets
                .get(bucket_id)
                .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                    reason: format!("main token genesis bucket not found: {bucket_id}"),
                })?;
        if bucket.recipient != beneficiary {
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
        if amount != releasable {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token vesting claim amount mismatch: expected={} actual={}",
                    releasable, amount
                ),
            });
        }

        let account =
            self.main_token_balances
                .get_mut(beneficiary)
                .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                    reason: format!("main token beneficiary account not found: {}", beneficiary),
                })?;
        if account.vested_balance < amount {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token vested balance insufficient: beneficiary={} vested={} claim={}",
                    beneficiary, account.vested_balance, amount
                ),
            });
        }
        account.vested_balance -= amount;
        account.liquid_balance =
            account
                .liquid_balance
                .checked_add(amount)
                .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "main token liquid balance overflow: beneficiary={} current={} claim={}",
                        beneficiary, account.liquid_balance, amount
                    ),
                })?;

        let bucket =
            self.main_token_genesis_buckets
                .get_mut(bucket_id)
                .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                    reason: format!("main token genesis bucket not found: {bucket_id}"),
                })?;
        bucket.claimed_amount =
            bucket
                .claimed_amount
                .checked_add(amount)
                .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "main token claimed amount overflow: bucket={} current={} claim={}",
                        bucket_id, bucket.claimed_amount, amount
                    ),
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
            .checked_add(amount)
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
                    self.main_token_supply.circulating_supply, self.main_token_supply.total_supply
                ),
            });
        }
        self.main_token_claim_nonces
            .insert(beneficiary.to_string(), nonce);
        Ok(())
    }

    pub(super) fn apply_main_token_transfer(
        &mut self,
        from_account_id: &str,
        to_account_id: &str,
        amount: u64,
        nonce: u64,
    ) -> Result<(), WorldError> {
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
        if amount == 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "main token transfer amount must be > 0".to_string(),
            });
        }
        if nonce == 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "main token transfer nonce must be > 0".to_string(),
            });
        }
        if let Some(last_nonce) = self.main_token_transfer_nonces.get(from_account_id) {
            if nonce <= *last_nonce {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "main token transfer nonce replay: from_account_id={} nonce={} last_nonce={}",
                        from_account_id, nonce, last_nonce
                    ),
                });
            }
        }

        let from_before =
            self.main_token_balances
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
        if from_before.liquid_balance < amount {
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

        let next_from_liquid = from_before.liquid_balance - amount;
        let next_to_liquid = to_before
            .as_ref()
            .map(|account| account.liquid_balance)
            .unwrap_or(0)
            .checked_add(amount)
            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token transfer target balance overflow: to_account_id={} amount={}",
                    to_account_id, amount
                ),
            })?;

        let from_account =
            self.main_token_balances
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
            .insert(from_account_id.to_string(), nonce);
        Ok(())
    }
}
