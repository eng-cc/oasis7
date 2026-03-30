use super::*;

impl WorldState {
    pub(super) fn apply_domain_event_gameplay_claims(
        &mut self,
        event: &DomainEvent,
        now: WorldTime,
    ) -> Result<(), WorldError> {
        match event {
            DomainEvent::AgentClaimed {
                claimer_agent_id,
                target_agent_id,
                reputation_tier,
                slot_index,
                activation_fee_amount,
                activation_fee_burn_amount,
                activation_fee_treasury_amount,
                claim_bond_amount,
                upfront_restricted_spent_amount,
                upfront_liquid_spent_amount,
                claim_bond_locked_restricted_amount,
                claim_bond_locked_liquid_amount,
                upkeep_per_epoch,
                claimed_at_epoch,
                upkeep_paid_through_epoch,
                release_cooldown_epochs,
                grace_epochs,
                idle_warning_epochs,
                forced_idle_reclaim_epochs,
                forced_reclaim_penalty_bps,
            } => {
                ensure_agent_claim_actor_exists(self, claimer_agent_id)?;
                ensure_agent_claim_target_exists(self, target_agent_id)?;
                if self.agent_claims.contains_key(target_agent_id) {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!("agent claim already exists: target={target_agent_id}"),
                    });
                }
                let owned_claim_count = owner_claim_count(self, claimer_agent_id);
                let reputation_score = self
                    .reputation_scores
                    .get(claimer_agent_id)
                    .copied()
                    .unwrap_or(0);
                let quote =
                    agent_claim_quote(reputation_score, owned_claim_count).map_err(|reason| {
                        WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "agent claim quote mismatch: owner={} reason={reason}",
                                claimer_agent_id
                            ),
                        }
                    })?;
                if quote.reputation_tier != *reputation_tier
                    || quote.slot_index != *slot_index
                    || quote.activation_fee_amount != *activation_fee_amount
                    || quote.activation_fee_burn_amount != *activation_fee_burn_amount
                    || quote.activation_fee_treasury_amount != *activation_fee_treasury_amount
                    || quote.claim_bond_amount != *claim_bond_amount
                    || quote.upkeep_per_epoch != *upkeep_per_epoch
                    || quote.release_cooldown_epochs != *release_cooldown_epochs
                    || quote.grace_epochs != *grace_epochs
                    || quote.idle_warning_epochs != *idle_warning_epochs
                    || quote.forced_idle_reclaim_epochs != *forced_idle_reclaim_epochs
                    || quote.forced_reclaim_penalty_bps != *forced_reclaim_penalty_bps
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim quote fields diverged: owner={} target={}",
                            claimer_agent_id, target_agent_id
                        ),
                    });
                }
                if *activation_fee_amount == 0 || *claim_bond_amount == 0 || *upkeep_per_epoch == 0
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim costs must be positive: target={target_agent_id}"
                        ),
                    });
                }
                if *upkeep_paid_through_epoch < *claimed_at_epoch {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim upkeep epoch mismatch: target={} claimed={} paid_through={}",
                            target_agent_id, claimed_at_epoch, upkeep_paid_through_epoch
                        ),
                    });
                }

                let upfront_amount = activation_fee_amount
                    .checked_add(*claim_bond_amount)
                    .and_then(|value| value.checked_add(*upkeep_per_epoch))
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim upfront overflow: target={} activation={} bond={} upkeep={}",
                            target_agent_id, activation_fee_amount, claim_bond_amount, upkeep_per_epoch
                        ),
                    })?;
                let liquid_balance = self
                    .main_token_balances
                    .get(claimer_agent_id)
                    .map(|balance| balance.liquid_balance)
                    .unwrap_or(0);
                let restricted_balance = self
                    .main_token_balances
                    .get(claimer_agent_id)
                    .map(|balance| balance.restricted_starter_claim_balance)
                    .unwrap_or(0);
                let expected_funding = split_agent_claim_upfront_funding(
                    *slot_index,
                    liquid_balance,
                    restricted_balance,
                    *activation_fee_amount,
                    *claim_bond_amount,
                    *upkeep_per_epoch,
                )
                .map_err(|reason| WorldError::ResourceBalanceInvalid { reason })?;
                if expected_funding.upfront.restricted_amount != *upfront_restricted_spent_amount
                    || expected_funding.upfront.liquid_amount != *upfront_liquid_spent_amount
                    || expected_funding.claim_bond.restricted_amount
                        != *claim_bond_locked_restricted_amount
                    || expected_funding.claim_bond.liquid_amount != *claim_bond_locked_liquid_amount
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim funding split mismatch: target={} slot={} upfront_restricted={} upfront_liquid={} bond_restricted={} bond_liquid={}",
                            target_agent_id,
                            slot_index,
                            upfront_restricted_spent_amount,
                            upfront_liquid_spent_amount,
                            claim_bond_locked_restricted_amount,
                            claim_bond_locked_liquid_amount
                        ),
                    });
                }
                debit_main_token_restricted_starter_claim_balance(
                    self,
                    claimer_agent_id,
                    *upfront_restricted_spent_amount,
                )?;
                debit_main_token_liquid_balance(
                    self,
                    claimer_agent_id,
                    *upfront_liquid_spent_amount,
                )?;
                decrease_main_token_circulating_supply(self, upfront_amount)?;
                burn_main_token_supply(self, *activation_fee_burn_amount)?;
                add_main_token_treasury_balance(
                    self,
                    MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL,
                    activation_fee_treasury_amount
                        .checked_add(*upkeep_per_epoch)
                        .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "agent claim treasury overflow: target={} activation_treasury={} upkeep={}",
                                target_agent_id,
                                activation_fee_treasury_amount,
                                upkeep_per_epoch
                            ),
                        })?,
                )?;

                self.agent_claims.insert(
                    target_agent_id.clone(),
                    AgentClaimState {
                        target_agent_id: target_agent_id.clone(),
                        claim_owner_id: claimer_agent_id.clone(),
                        reputation_tier: *reputation_tier,
                        slot_index: *slot_index,
                        activation_fee_amount: *activation_fee_amount,
                        activation_fee_burn_amount: *activation_fee_burn_amount,
                        activation_fee_treasury_amount: *activation_fee_treasury_amount,
                        claim_bond_amount: *claim_bond_amount,
                        locked_bond_amount: *claim_bond_amount,
                        upfront_restricted_spent_amount: *upfront_restricted_spent_amount,
                        upfront_liquid_spent_amount: *upfront_liquid_spent_amount,
                        claim_bond_locked_restricted_amount: *claim_bond_locked_restricted_amount,
                        claim_bond_locked_liquid_amount: *claim_bond_locked_liquid_amount,
                        upkeep_per_epoch: *upkeep_per_epoch,
                        release_cooldown_epochs: *release_cooldown_epochs,
                        grace_epochs: *grace_epochs,
                        idle_warning_epochs: *idle_warning_epochs,
                        forced_idle_reclaim_epochs: *forced_idle_reclaim_epochs,
                        forced_reclaim_penalty_bps: *forced_reclaim_penalty_bps,
                        claimed_at_epoch: *claimed_at_epoch,
                        upkeep_paid_through_epoch: *upkeep_paid_through_epoch,
                        delinquent_since_epoch: None,
                        grace_deadline_epoch: None,
                        release_requested_at_epoch: None,
                        release_ready_at_epoch: None,
                        idle_warning_emitted_at_epoch: None,
                    },
                );
                self.agent_claim_last_processed_epoch =
                    self.agent_claim_last_processed_epoch.max(*claimed_at_epoch);
                self.agents
                    .get_mut(claimer_agent_id)
                    .expect("claimer existence prechecked")
                    .last_active = now;
            }
            DomainEvent::AgentClaimReleaseRequested {
                claimer_agent_id,
                target_agent_id,
                requested_at_epoch,
                ready_at_epoch,
            } => {
                let claim = self.agent_claims.get_mut(target_agent_id).ok_or_else(|| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!("agent claim not found: target={target_agent_id}"),
                    }
                })?;
                if claim.claim_owner_id != *claimer_agent_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim release owner mismatch: target={} owner={} claimer={}",
                            target_agent_id, claim.claim_owner_id, claimer_agent_id
                        ),
                    });
                }
                if claim.release_requested_at_epoch.is_some() {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim release already requested: target={target_agent_id}"
                        ),
                    });
                }
                if *ready_at_epoch < *requested_at_epoch {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim release epoch mismatch: target={} requested={} ready={}",
                            target_agent_id, requested_at_epoch, ready_at_epoch
                        ),
                    });
                }
                claim.release_requested_at_epoch = Some(*requested_at_epoch);
                claim.release_ready_at_epoch = Some(*ready_at_epoch);
                self.agents
                    .get_mut(claimer_agent_id)
                    .expect("claimer existence prechecked")
                    .last_active = now;
            }
            DomainEvent::AgentClaimUpkeepSettled {
                claimer_agent_id,
                target_agent_id,
                settled_at_epoch,
                charged_epochs,
                amount,
                restricted_spent_amount,
                liquid_spent_amount,
                upkeep_paid_through_epoch,
            } => {
                let claim = self.agent_claims.get(target_agent_id).ok_or_else(|| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!("agent claim not found: target={target_agent_id}"),
                    }
                })?;
                if claim.claim_owner_id != *claimer_agent_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim upkeep owner mismatch: target={} owner={} claimer={}",
                            target_agent_id, claim.claim_owner_id, claimer_agent_id
                        ),
                    });
                }
                if *charged_epochs == 0 || *amount == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim upkeep settlement must be positive: target={target_agent_id}"
                        ),
                    });
                }
                let expected_amount = claim
                    .upkeep_per_epoch
                    .checked_mul(*charged_epochs)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim upkeep overflow: target={} upkeep={} epochs={}",
                            target_agent_id, claim.upkeep_per_epoch, charged_epochs
                        ),
                    })?;
                if expected_amount != *amount {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim upkeep mismatch: target={} expected={} actual={}",
                            target_agent_id, expected_amount, amount
                        ),
                    });
                }
                let liquid_balance = self
                    .main_token_balances
                    .get(claimer_agent_id)
                    .map(|balance| balance.liquid_balance)
                    .unwrap_or(0);
                let restricted_balance = self
                    .main_token_balances
                    .get(claimer_agent_id)
                    .map(|balance| balance.restricted_starter_claim_balance)
                    .unwrap_or(0);
                let expected_funding = split_agent_claim_spend(
                    claim.slot_index,
                    liquid_balance,
                    restricted_balance,
                    *amount,
                )
                .map_err(|reason| WorldError::ResourceBalanceInvalid { reason })?;
                if expected_funding.restricted_amount != *restricted_spent_amount
                    || expected_funding.liquid_amount != *liquid_spent_amount
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim upkeep funding split mismatch: target={} slot={} restricted={} liquid={}",
                            target_agent_id,
                            claim.slot_index,
                            restricted_spent_amount,
                            liquid_spent_amount
                        ),
                    });
                }
                debit_main_token_restricted_starter_claim_balance(
                    self,
                    claimer_agent_id,
                    *restricted_spent_amount,
                )?;
                debit_main_token_liquid_balance(self, claimer_agent_id, *liquid_spent_amount)?;
                decrease_main_token_circulating_supply(self, *amount)?;
                add_main_token_treasury_balance(
                    self,
                    MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL,
                    *amount,
                )?;
                let claim = self.agent_claims.get_mut(target_agent_id).ok_or_else(|| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim not found after debit: target={target_agent_id}"
                        ),
                    }
                })?;
                claim.upkeep_paid_through_epoch = *upkeep_paid_through_epoch;
                claim.delinquent_since_epoch = None;
                claim.grace_deadline_epoch = None;
                self.agent_claim_last_processed_epoch =
                    self.agent_claim_last_processed_epoch.max(*settled_at_epoch);
                self.agents
                    .get_mut(claimer_agent_id)
                    .expect("claimer existence prechecked")
                    .last_active = now;
            }
            DomainEvent::AgentClaimEnteredGrace {
                claimer_agent_id,
                target_agent_id,
                delinquent_since_epoch,
                grace_deadline_epoch,
                upkeep_arrears_amount,
            } => {
                let claim = self.agent_claims.get_mut(target_agent_id).ok_or_else(|| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!("agent claim not found: target={target_agent_id}"),
                    }
                })?;
                if claim.claim_owner_id != *claimer_agent_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim grace owner mismatch: target={} owner={} claimer={}",
                            target_agent_id, claim.claim_owner_id, claimer_agent_id
                        ),
                    });
                }
                if *upkeep_arrears_amount == 0 {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim grace arrears must be positive: target={target_agent_id}"
                        ),
                    });
                }
                if *grace_deadline_epoch < *delinquent_since_epoch {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim grace epoch mismatch: target={} delinquent={} deadline={}",
                            target_agent_id, delinquent_since_epoch, grace_deadline_epoch
                        ),
                    });
                }
                claim.delinquent_since_epoch = Some(*delinquent_since_epoch);
                claim.grace_deadline_epoch = Some(*grace_deadline_epoch);
            }
            DomainEvent::AgentClaimIdleWarning {
                claimer_agent_id,
                target_agent_id,
                warning_emitted_at_epoch,
                forced_reclaim_at_epoch,
            } => {
                let claim = self.agent_claims.get_mut(target_agent_id).ok_or_else(|| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!("agent claim not found: target={target_agent_id}"),
                    }
                })?;
                if claim.claim_owner_id != *claimer_agent_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim idle warning owner mismatch: target={} owner={} claimer={}",
                            target_agent_id, claim.claim_owner_id, claimer_agent_id
                        ),
                    });
                }
                if *forced_reclaim_at_epoch < *warning_emitted_at_epoch {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim idle warning epoch mismatch: target={} warning={} forced={}",
                            target_agent_id, warning_emitted_at_epoch, forced_reclaim_at_epoch
                        ),
                    });
                }
                claim.idle_warning_emitted_at_epoch = Some(*warning_emitted_at_epoch);
            }
            DomainEvent::AgentClaimReleased {
                claimer_agent_id,
                target_agent_id,
                released_at_epoch,
                refunded_bond_amount,
                refunded_bond_restricted_amount,
                refunded_bond_liquid_amount,
                refunded_bond_restricted_sink,
                refunded_bond_restricted_sink_bucket_id,
            } => {
                let claim = self.agent_claims.remove(target_agent_id).ok_or_else(|| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!("agent claim not found: target={target_agent_id}"),
                    }
                })?;
                if claim.claim_owner_id != *claimer_agent_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim release owner mismatch: target={} owner={} claimer={}",
                            target_agent_id, claim.claim_owner_id, claimer_agent_id
                        ),
                    });
                }
                if *refunded_bond_amount != claim.locked_bond_amount {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim release refund mismatch: target={} expected={} actual={}",
                            target_agent_id, claim.locked_bond_amount, refunded_bond_amount
                        ),
                    });
                }
                let expected_refund = split_agent_claim_bond_refund(
                    claim.claim_bond_locked_restricted_amount,
                    claim.claim_bond_locked_liquid_amount,
                    0,
                )
                .map_err(|reason| WorldError::ResourceBalanceInvalid { reason })?;
                if expected_refund.restricted_amount != *refunded_bond_restricted_amount
                    || expected_refund.liquid_amount != *refunded_bond_liquid_amount
                    || refunded_bond_restricted_amount.saturating_add(*refunded_bond_liquid_amount)
                        != *refunded_bond_amount
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim release refund provenance mismatch: target={} restricted={} liquid={} total={}",
                            target_agent_id,
                            refunded_bond_restricted_amount,
                            refunded_bond_liquid_amount,
                            refunded_bond_amount
                        ),
                    });
                }
                let (expected_sink, expected_sink_bucket_id) =
                    restricted_refund_sink_for_account(self, claimer_agent_id);
                if *refunded_bond_restricted_sink != expected_sink
                    || *refunded_bond_restricted_sink_bucket_id != expected_sink_bucket_id
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim release restricted refund sink mismatch: target={} expected={:?}/{} actual={:?}/{}",
                            target_agent_id,
                            expected_sink,
                            expected_sink_bucket_id,
                            refunded_bond_restricted_sink,
                            refunded_bond_restricted_sink_bucket_id
                        ),
                    });
                }
                let mut circulating_refund_amount = *refunded_bond_liquid_amount;
                match refunded_bond_restricted_sink {
                    RestrictedStarterClaimRefundSink::BeneficiaryRestrictedBalance => {
                        credit_main_token_restricted_starter_claim_balance(
                            self,
                            claimer_agent_id,
                            *refunded_bond_restricted_amount,
                        )?;
                        circulating_refund_amount = circulating_refund_amount
                            .checked_add(*refunded_bond_restricted_amount)
                            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                                reason: format!(
                                    "agent claim release circulating refund overflow: target={} liquid={} restricted={}",
                                    target_agent_id,
                                    refunded_bond_liquid_amount,
                                    refunded_bond_restricted_amount
                                ),
                            })?;
                    }
                    RestrictedStarterClaimRefundSink::SourceTreasuryBucket => {
                        add_main_token_treasury_balance(
                            self,
                            refunded_bond_restricted_sink_bucket_id,
                            *refunded_bond_restricted_amount,
                        )?;
                    }
                }
                credit_main_token_liquid_balance(
                    self,
                    claimer_agent_id,
                    *refunded_bond_liquid_amount,
                )?;
                increase_main_token_circulating_supply(self, circulating_refund_amount)?;
                self.agent_claim_last_processed_epoch = self
                    .agent_claim_last_processed_epoch
                    .max(*released_at_epoch);
                self.agents
                    .get_mut(claimer_agent_id)
                    .expect("claimer existence prechecked")
                    .last_active = now;
            }
            DomainEvent::AgentClaimReclaimed {
                claimer_agent_id,
                target_agent_id,
                reclaimed_at_epoch,
                reason: _,
                upkeep_arrears_amount,
                collected_upkeep_amount,
                penalty_amount,
                refunded_bond_amount,
                refunded_bond_restricted_amount,
                refunded_bond_liquid_amount,
                refunded_bond_restricted_sink,
                refunded_bond_restricted_sink_bucket_id,
            } => {
                let claim = self.agent_claims.remove(target_agent_id).ok_or_else(|| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!("agent claim not found: target={target_agent_id}"),
                    }
                })?;
                if claim.claim_owner_id != *claimer_agent_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim reclaim owner mismatch: target={} owner={} claimer={}",
                            target_agent_id, claim.claim_owner_id, claimer_agent_id
                        ),
                    });
                }
                let expected_collected = claim.locked_bond_amount.min(*upkeep_arrears_amount);
                let remaining_after_upkeep =
                    claim.locked_bond_amount.saturating_sub(expected_collected);
                let expected_penalty = remaining_after_upkeep
                    .saturating_mul(u64::from(claim.forced_reclaim_penalty_bps))
                    / 10_000;
                let expected_refund = remaining_after_upkeep.saturating_sub(expected_penalty);
                if expected_collected != *collected_upkeep_amount
                    || expected_penalty != *penalty_amount
                    || expected_refund != *refunded_bond_amount
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim reclaim settlement mismatch: target={} collected={} penalty={} refund={}",
                            target_agent_id, collected_upkeep_amount, penalty_amount, refunded_bond_amount
                        ),
                    });
                }
                let expected_refund_split = split_agent_claim_bond_refund(
                    claim.claim_bond_locked_restricted_amount,
                    claim.claim_bond_locked_liquid_amount,
                    collected_upkeep_amount.saturating_add(*penalty_amount),
                )
                .map_err(|reason| WorldError::ResourceBalanceInvalid { reason })?;
                if expected_refund_split.restricted_amount != *refunded_bond_restricted_amount
                    || expected_refund_split.liquid_amount != *refunded_bond_liquid_amount
                    || refunded_bond_restricted_amount.saturating_add(*refunded_bond_liquid_amount)
                        != *refunded_bond_amount
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim reclaim refund provenance mismatch: target={} restricted={} liquid={} total={}",
                            target_agent_id,
                            refunded_bond_restricted_amount,
                            refunded_bond_liquid_amount,
                            refunded_bond_amount
                        ),
                    });
                }
                let (expected_sink, expected_sink_bucket_id) =
                    restricted_refund_sink_for_account(self, claimer_agent_id);
                if *refunded_bond_restricted_sink != expected_sink
                    || *refunded_bond_restricted_sink_bucket_id != expected_sink_bucket_id
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "agent claim reclaim restricted refund sink mismatch: target={} expected={:?}/{} actual={:?}/{}",
                            target_agent_id,
                            expected_sink,
                            expected_sink_bucket_id,
                            refunded_bond_restricted_sink,
                            refunded_bond_restricted_sink_bucket_id
                        ),
                    });
                }
                if *collected_upkeep_amount > 0 {
                    add_main_token_treasury_balance(
                        self,
                        MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL,
                        *collected_upkeep_amount,
                    )?;
                }
                if *penalty_amount > 0 {
                    add_main_token_treasury_balance(
                        self,
                        MAIN_TOKEN_TREASURY_BUCKET_SLASH,
                        *penalty_amount,
                    )?;
                }
                if *refunded_bond_amount > 0 {
                    let mut circulating_refund_amount = *refunded_bond_liquid_amount;
                    match refunded_bond_restricted_sink {
                        RestrictedStarterClaimRefundSink::BeneficiaryRestrictedBalance => {
                            credit_main_token_restricted_starter_claim_balance(
                                self,
                                claimer_agent_id,
                                *refunded_bond_restricted_amount,
                            )?;
                            circulating_refund_amount = circulating_refund_amount
                                .checked_add(*refunded_bond_restricted_amount)
                                .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                                    reason: format!(
                                        "agent claim reclaim circulating refund overflow: target={} liquid={} restricted={}",
                                        target_agent_id,
                                        refunded_bond_liquid_amount,
                                        refunded_bond_restricted_amount
                                    ),
                                })?;
                        }
                        RestrictedStarterClaimRefundSink::SourceTreasuryBucket => {
                            add_main_token_treasury_balance(
                                self,
                                refunded_bond_restricted_sink_bucket_id,
                                *refunded_bond_restricted_amount,
                            )?;
                        }
                    }
                    credit_main_token_liquid_balance(
                        self,
                        claimer_agent_id,
                        *refunded_bond_liquid_amount,
                    )?;
                    increase_main_token_circulating_supply(self, circulating_refund_amount)?;
                }
                self.agent_claim_last_processed_epoch = self
                    .agent_claim_last_processed_epoch
                    .max(*reclaimed_at_epoch);
                self.agents
                    .get_mut(claimer_agent_id)
                    .expect("claimer existence prechecked")
                    .last_active = now;
            }
            _ => unreachable!("apply_domain_event_gameplay_claims received unsupported event"),
        }
        Ok(())
    }
}

fn ensure_agent_claim_actor_exists(
    state: &WorldState,
    claimer_agent_id: &str,
) -> Result<(), WorldError> {
    if state.agents.contains_key(claimer_agent_id) {
        Ok(())
    } else {
        Err(WorldError::AgentNotFound {
            agent_id: claimer_agent_id.to_string(),
        })
    }
}

fn ensure_agent_claim_target_exists(
    state: &WorldState,
    target_agent_id: &str,
) -> Result<(), WorldError> {
    if state.agents.contains_key(target_agent_id) {
        Ok(())
    } else {
        Err(WorldError::AgentNotFound {
            agent_id: target_agent_id.to_string(),
        })
    }
}

fn owner_claim_count(state: &WorldState, claimer_agent_id: &str) -> usize {
    state
        .agent_claims
        .values()
        .filter(|claim| claim.claim_owner_id == claimer_agent_id)
        .count()
}

fn debit_main_token_liquid_balance(
    state: &mut WorldState,
    account_id: &str,
    amount: u64,
) -> Result<(), WorldError> {
    let account = state
        .main_token_balances
        .get_mut(account_id)
        .ok_or_else(|| WorldError::ResourceBalanceInvalid {
            reason: format!("main token account not found: {account_id}"),
        })?;
    if account.liquid_balance < amount {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!(
                "main token liquid balance insufficient: account={} balance={} amount={}",
                account_id, account.liquid_balance, amount
            ),
        });
    }
    account.liquid_balance -= amount;
    Ok(())
}

fn debit_main_token_restricted_starter_claim_balance(
    state: &mut WorldState,
    account_id: &str,
    amount: u64,
) -> Result<(), WorldError> {
    let account = state
        .main_token_balances
        .get_mut(account_id)
        .ok_or_else(|| WorldError::ResourceBalanceInvalid {
            reason: format!("main token account not found: {account_id}"),
        })?;
    if account.restricted_starter_claim_balance < amount {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!(
                "restricted starter claim balance insufficient: account={} balance={} amount={}",
                account_id, account.restricted_starter_claim_balance, amount
            ),
        });
    }
    account.restricted_starter_claim_balance -= amount;
    Ok(())
}

fn credit_main_token_liquid_balance(
    state: &mut WorldState,
    account_id: &str,
    amount: u64,
) -> Result<(), WorldError> {
    let account = state
        .main_token_balances
        .entry(account_id.to_string())
        .or_insert_with(|| MainTokenAccountBalance {
            account_id: account_id.to_string(),
            ..MainTokenAccountBalance::default()
        });
    account.liquid_balance = account.liquid_balance.checked_add(amount).ok_or_else(|| {
        WorldError::ResourceBalanceInvalid {
            reason: format!(
                "main token liquid credit overflow: account={} current={} amount={}",
                account_id, account.liquid_balance, amount
            ),
        }
    })?;
    Ok(())
}

fn credit_main_token_restricted_starter_claim_balance(
    state: &mut WorldState,
    account_id: &str,
    amount: u64,
) -> Result<(), WorldError> {
    let account = state
        .main_token_balances
        .entry(account_id.to_string())
        .or_insert_with(|| MainTokenAccountBalance {
            account_id: account_id.to_string(),
            ..MainTokenAccountBalance::default()
        });
    account.restricted_starter_claim_balance = account
        .restricted_starter_claim_balance
        .checked_add(amount)
        .ok_or_else(|| WorldError::ResourceBalanceInvalid {
            reason: format!(
                "restricted starter claim credit overflow: account={} current={} amount={}",
                account_id, account.restricted_starter_claim_balance, amount
            ),
        })?;
    Ok(())
}

fn add_main_token_treasury_balance(
    state: &mut WorldState,
    bucket_id: &str,
    amount: u64,
) -> Result<(), WorldError> {
    let next = state
        .main_token_treasury_balances
        .get(bucket_id)
        .copied()
        .unwrap_or(0)
        .checked_add(amount)
        .ok_or_else(|| WorldError::ResourceBalanceInvalid {
            reason: format!(
                "main token treasury overflow: bucket={} amount={}",
                bucket_id, amount
            ),
        })?;
    state
        .main_token_treasury_balances
        .insert(bucket_id.to_string(), next);
    Ok(())
}

fn burn_main_token_supply(state: &mut WorldState, burn_amount: u64) -> Result<(), WorldError> {
    if state.main_token_supply.total_supply < burn_amount {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!(
                "main token total supply insufficient for burn: total={} burn={}",
                state.main_token_supply.total_supply, burn_amount
            ),
        });
    }
    state.main_token_supply.total_supply -= burn_amount;
    state.main_token_supply.total_burned = state
        .main_token_supply
        .total_burned
        .checked_add(burn_amount)
        .ok_or_else(|| WorldError::ResourceBalanceInvalid {
            reason: format!(
                "main token total_burned overflow: current={} burn={}",
                state.main_token_supply.total_burned, burn_amount
            ),
        })?;
    Ok(())
}

fn decrease_main_token_circulating_supply(
    state: &mut WorldState,
    amount: u64,
) -> Result<(), WorldError> {
    if state.main_token_supply.circulating_supply < amount {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!(
                "main token circulating supply insufficient: circulating={} amount={}",
                state.main_token_supply.circulating_supply, amount
            ),
        });
    }
    state.main_token_supply.circulating_supply -= amount;
    Ok(())
}

fn increase_main_token_circulating_supply(
    state: &mut WorldState,
    amount: u64,
) -> Result<(), WorldError> {
    state.main_token_supply.circulating_supply = state
        .main_token_supply
        .circulating_supply
        .checked_add(amount)
        .ok_or_else(|| WorldError::ResourceBalanceInvalid {
            reason: format!(
                "main token circulating supply overflow: current={} amount={}",
                state.main_token_supply.circulating_supply, amount
            ),
        })?;
    if state.main_token_supply.circulating_supply > state.main_token_supply.total_supply {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!(
                "main token circulating exceeds total supply: circulating={} total={}",
                state.main_token_supply.circulating_supply, state.main_token_supply.total_supply
            ),
        });
    }
    Ok(())
}

fn restricted_refund_sink_for_account(
    state: &WorldState,
    account_id: &str,
) -> (RestrictedStarterClaimRefundSink, String) {
    let Some(grant) = state.restricted_starter_claim_grants.get(account_id) else {
        return (
            RestrictedStarterClaimRefundSink::BeneficiaryRestrictedBalance,
            String::new(),
        );
    };
    match grant.status {
        RestrictedStarterClaimGrantStatus::Issued => (
            RestrictedStarterClaimRefundSink::BeneficiaryRestrictedBalance,
            String::new(),
        ),
        RestrictedStarterClaimGrantStatus::Expired | RestrictedStarterClaimGrantStatus::Revoked => {
            (
                RestrictedStarterClaimRefundSink::SourceTreasuryBucket,
                grant.source_treasury_bucket_id.clone(),
            )
        }
    }
}
