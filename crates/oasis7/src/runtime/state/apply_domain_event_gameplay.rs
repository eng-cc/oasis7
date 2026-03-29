use super::super::agent_claims::{
    agent_claim_quote, split_agent_claim_bond_refund, split_agent_claim_spend,
    split_agent_claim_upfront_funding,
};
use super::super::main_token::{
    MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL, MAIN_TOKEN_TREASURY_BUCKET_SLASH,
};
use super::*;

const ECONOMIC_CONTRACT_REPUTATION_WINDOW_TICKS: u64 = 20;

impl WorldState {
    pub(super) fn apply_domain_event_gameplay(
        &mut self,
        event: &DomainEvent,
        now: WorldTime,
    ) -> Result<(), WorldError> {
        match event {
            DomainEvent::GameplayPolicyUpdated {
                operator_agent_id,
                electricity_tax_bps,
                data_tax_bps,
                power_trade_fee_bps,
                max_open_contracts_per_agent,
                blocked_agents,
                forbidden_location_ids,
            } => {
                if !self.agents.contains_key(operator_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: operator_agent_id.clone(),
                    });
                }
                let mut normalized_blocked_agents = blocked_agents
                    .iter()
                    .filter_map(|value| {
                        let normalized = value.trim();
                        if normalized.is_empty() {
                            None
                        } else {
                            Some(normalized.to_string())
                        }
                    })
                    .collect::<Vec<_>>();
                normalized_blocked_agents.sort();
                normalized_blocked_agents.dedup();
                let mut normalized_forbidden_location_ids = forbidden_location_ids
                    .iter()
                    .filter_map(|value| {
                        let normalized = value.trim();
                        if normalized.is_empty() {
                            None
                        } else {
                            Some(normalized.to_string())
                        }
                    })
                    .collect::<Vec<_>>();
                normalized_forbidden_location_ids.sort();
                normalized_forbidden_location_ids.dedup();
                self.gameplay_policy = GameplayPolicyState {
                    electricity_tax_bps: *electricity_tax_bps,
                    data_tax_bps: *data_tax_bps,
                    power_trade_fee_bps: *power_trade_fee_bps,
                    max_open_contracts_per_agent: *max_open_contracts_per_agent,
                    blocked_agents: normalized_blocked_agents,
                    forbidden_location_ids: normalized_forbidden_location_ids,
                    updated_at: now,
                };
                self.refresh_industry_progress_stage(now);
                if let Some(cell) = self.agents.get_mut(operator_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::EconomicContractOpened {
                creator_agent_id,
                contract_id,
                counterparty_agent_id,
                settlement_kind,
                settlement_amount,
                reputation_stake,
                expires_at,
                description,
            } => {
                if !self.agents.contains_key(creator_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: creator_agent_id.clone(),
                    });
                }
                if !self.agents.contains_key(counterparty_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: counterparty_agent_id.clone(),
                    });
                }
                self.economic_contracts.insert(
                    contract_id.clone(),
                    EconomicContractState {
                        contract_id: contract_id.clone(),
                        creator_agent_id: creator_agent_id.clone(),
                        counterparty_agent_id: counterparty_agent_id.clone(),
                        settlement_kind: *settlement_kind,
                        settlement_amount: *settlement_amount,
                        reputation_stake: *reputation_stake,
                        expires_at: *expires_at,
                        description: description.clone(),
                        status: EconomicContractStatus::Open,
                        accepted_at: None,
                        settled_at: None,
                        settlement_success: None,
                        transfer_amount: 0,
                        tax_amount: 0,
                        settlement_notes: None,
                    },
                );
                if let Some(cell) = self.agents.get_mut(creator_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::EconomicContractAccepted {
                accepter_agent_id,
                contract_id,
            } => {
                let contract = self
                    .economic_contracts
                    .get_mut(contract_id)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!("economic contract not found: {contract_id}"),
                    })?;
                if contract.status != EconomicContractStatus::Open {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "economic contract status invalid for acceptance: {:?}",
                            contract.status
                        ),
                    });
                }
                if contract.counterparty_agent_id != *accepter_agent_id {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "economic contract accepter mismatch expected={} actual={}",
                            contract.counterparty_agent_id, accepter_agent_id
                        ),
                    });
                }
                contract.status = EconomicContractStatus::Accepted;
                contract.accepted_at = Some(now);
                if let Some(cell) = self.agents.get_mut(accepter_agent_id) {
                    cell.last_active = now;
                } else {
                    return Err(WorldError::AgentNotFound {
                        agent_id: accepter_agent_id.clone(),
                    });
                }
            }
            DomainEvent::EconomicContractSettled {
                operator_agent_id,
                contract_id,
                success,
                transfer_amount,
                tax_amount,
                notes,
                creator_reputation_delta,
                counterparty_reputation_delta,
            } => {
                let (creator_agent_id, counterparty_agent_id, settlement_kind, status) = {
                    let contract = self.economic_contracts.get(contract_id).ok_or_else(|| {
                        WorldError::ResourceBalanceInvalid {
                            reason: format!("economic contract not found: {contract_id}"),
                        }
                    })?;
                    (
                        contract.creator_agent_id.clone(),
                        contract.counterparty_agent_id.clone(),
                        contract.settlement_kind,
                        contract.status,
                    )
                };
                if status != EconomicContractStatus::Accepted {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "economic contract status invalid for settlement: {:?}",
                            status
                        ),
                    });
                }
                if !self.agents.contains_key(operator_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: operator_agent_id.clone(),
                    });
                }

                // Precompute all mutable outcomes first so settlement writes are atomic.
                let mut settlement_apply: Option<(i64, i64, i64)> = None;
                if *success {
                    if *transfer_amount <= 0 {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "economic contract settlement transfer must be > 0, got {}",
                                transfer_amount
                            ),
                        });
                    }
                    if *tax_amount < 0 {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "economic contract settlement tax must be >= 0, got {}",
                                tax_amount
                            ),
                        });
                    }
                    let debit_total =
                        transfer_amount.checked_add(*tax_amount).ok_or_else(|| {
                            WorldError::ResourceBalanceInvalid {
                                reason: format!(
                                "economic contract settlement debit overflow transfer={} tax={}",
                                transfer_amount, tax_amount
                            ),
                            }
                        })?;

                    let creator_current = self
                        .agents
                        .get(&creator_agent_id)
                        .ok_or_else(|| WorldError::AgentNotFound {
                            agent_id: creator_agent_id.clone(),
                        })?
                        .state
                        .resources
                        .get(settlement_kind);
                    if creator_current < debit_total {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "economic contract settlement debit failed agent={} kind={:?} amount={} available={}",
                                creator_agent_id, settlement_kind, debit_total, creator_current
                            ),
                        });
                    }
                    let creator_after_debit = creator_current - debit_total;
                    let counterparty_next = if creator_agent_id == counterparty_agent_id {
                        creator_after_debit.checked_add(*transfer_amount).ok_or_else(|| {
                            WorldError::ResourceBalanceInvalid {
                                reason: format!(
                                    "economic contract settlement credit failed agent={} kind={:?} amount={} overflow",
                                    counterparty_agent_id, settlement_kind, transfer_amount
                                ),
                            }
                        })?
                    } else {
                        let counterparty_current = self
                            .agents
                            .get(&counterparty_agent_id)
                            .ok_or_else(|| WorldError::AgentNotFound {
                                agent_id: counterparty_agent_id.clone(),
                            })?
                            .state
                            .resources
                            .get(settlement_kind);
                        counterparty_current
                            .checked_add(*transfer_amount)
                            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                                reason: format!(
                                    "economic contract settlement credit failed agent={} kind={:?} amount={} overflow",
                                    counterparty_agent_id, settlement_kind, transfer_amount
                                ),
                            })?
                    };
                    let treasury_current =
                        self.resources.get(&settlement_kind).copied().unwrap_or(0);
                    let treasury_next =
                        treasury_current
                            .checked_add(*tax_amount)
                            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                                reason: format!(
                                    "economic contract settlement treasury overflow kind={:?} current={} delta={}",
                                    settlement_kind, treasury_current, tax_amount
                                ),
                            })?;
                    settlement_apply =
                        Some((creator_after_debit, counterparty_next, treasury_next));
                }

                let creator_score_next = if *creator_reputation_delta != 0 {
                    let current = self
                        .reputation_scores
                        .get(&creator_agent_id)
                        .copied()
                        .unwrap_or(0);
                    Some(
                        current
                            .checked_add(*creator_reputation_delta)
                            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                                reason: format!(
                                    "creator reputation overflow agent={} current={} delta={}",
                                    creator_agent_id, current, creator_reputation_delta
                                ),
                            })?,
                    )
                } else {
                    None
                };
                let counterparty_score_next = if *counterparty_reputation_delta != 0 {
                    let current = self
                        .reputation_scores
                        .get(&counterparty_agent_id)
                        .copied()
                        .unwrap_or(0);
                    Some(
                        current
                            .checked_add(*counterparty_reputation_delta)
                            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                                reason: format!(
                                    "counterparty reputation overflow agent={} current={} delta={}",
                                    counterparty_agent_id, current, counterparty_reputation_delta
                                ),
                            })?,
                    )
                } else {
                    None
                };

                if let Some((creator_after_debit, counterparty_next, treasury_next)) =
                    settlement_apply
                {
                    if creator_agent_id == counterparty_agent_id {
                        let creator_cell =
                            self.agents.get_mut(&creator_agent_id).ok_or_else(|| {
                                WorldError::AgentNotFound {
                                    agent_id: creator_agent_id.clone(),
                                }
                            })?;
                        if counterparty_next == 0 {
                            creator_cell
                                .state
                                .resources
                                .amounts
                                .remove(&settlement_kind);
                        } else {
                            creator_cell
                                .state
                                .resources
                                .amounts
                                .insert(settlement_kind, counterparty_next);
                        }
                    } else {
                        let creator_cell =
                            self.agents.get_mut(&creator_agent_id).ok_or_else(|| {
                                WorldError::AgentNotFound {
                                    agent_id: creator_agent_id.clone(),
                                }
                            })?;
                        if creator_after_debit == 0 {
                            creator_cell
                                .state
                                .resources
                                .amounts
                                .remove(&settlement_kind);
                        } else {
                            creator_cell
                                .state
                                .resources
                                .amounts
                                .insert(settlement_kind, creator_after_debit);
                        }

                        let counterparty_cell = self
                            .agents
                            .get_mut(&counterparty_agent_id)
                            .ok_or_else(|| WorldError::AgentNotFound {
                                agent_id: counterparty_agent_id.clone(),
                            })?;
                        if counterparty_next == 0 {
                            counterparty_cell
                                .state
                                .resources
                                .amounts
                                .remove(&settlement_kind);
                        } else {
                            counterparty_cell
                                .state
                                .resources
                                .amounts
                                .insert(settlement_kind, counterparty_next);
                        }
                    }

                    if treasury_next == 0 {
                        self.resources.remove(&settlement_kind);
                    } else {
                        self.resources.insert(settlement_kind, treasury_next);
                    }
                }

                if let Some(next) = creator_score_next {
                    self.reputation_scores
                        .insert(creator_agent_id.clone(), next);
                }
                if let Some(next) = counterparty_score_next {
                    self.reputation_scores
                        .insert(counterparty_agent_id.clone(), next);
                }
                if *success {
                    self.record_successful_contract_pair_settlement(
                        creator_agent_id.as_str(),
                        counterparty_agent_id.as_str(),
                        now,
                    );
                    self.record_reputation_reward_window_gain(
                        creator_agent_id.as_str(),
                        *creator_reputation_delta,
                        now,
                        ECONOMIC_CONTRACT_REPUTATION_WINDOW_TICKS,
                    );
                    self.record_reputation_reward_window_gain(
                        counterparty_agent_id.as_str(),
                        *counterparty_reputation_delta,
                        now,
                        ECONOMIC_CONTRACT_REPUTATION_WINDOW_TICKS,
                    );
                }

                let contract = self
                    .economic_contracts
                    .get_mut(contract_id)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!("economic contract not found: {contract_id}"),
                    })?;
                contract.status = EconomicContractStatus::Settled;
                contract.settled_at = Some(now);
                contract.settlement_success = Some(*success);
                contract.transfer_amount = *transfer_amount;
                contract.tax_amount = *tax_amount;
                contract.settlement_notes = Some(notes.clone());

                self.agents
                    .get_mut(operator_agent_id)
                    .expect("operator existence prechecked")
                    .last_active = now;
            }
            DomainEvent::EconomicContractExpired {
                contract_id,
                creator_agent_id,
                counterparty_agent_id,
                creator_reputation_delta,
                counterparty_reputation_delta,
            } => {
                let contract = self
                    .economic_contracts
                    .get_mut(contract_id)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!("economic contract not found: {contract_id}"),
                    })?;
                match contract.status {
                    EconomicContractStatus::Open | EconomicContractStatus::Accepted => {
                        contract.status = EconomicContractStatus::Expired;
                        contract.settled_at = Some(now);
                        contract.settlement_success = Some(false);
                        contract.transfer_amount = 0;
                        contract.tax_amount = 0;
                        contract.settlement_notes =
                            Some("auto expired by gameplay lifecycle".to_string());
                    }
                    EconomicContractStatus::Settled | EconomicContractStatus::Expired => {
                        return Err(WorldError::ResourceBalanceInvalid {
                            reason: format!(
                                "economic contract already finalized before expiry: {}",
                                contract_id
                            ),
                        });
                    }
                }
                if *creator_reputation_delta != 0 {
                    let score = self
                        .reputation_scores
                        .entry(creator_agent_id.clone())
                        .or_insert(0);
                    *score = score.saturating_add(*creator_reputation_delta);
                }
                if *counterparty_reputation_delta != 0 {
                    let score = self
                        .reputation_scores
                        .entry(counterparty_agent_id.clone())
                        .or_insert(0);
                    *score = score.saturating_add(*counterparty_reputation_delta);
                }
                if let Some(cell) = self.agents.get_mut(creator_agent_id) {
                    cell.last_active = now;
                }
                if let Some(cell) = self.agents.get_mut(counterparty_agent_id) {
                    cell.last_active = now;
                }
            }
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
                credit_main_token_restricted_starter_claim_balance(
                    self,
                    claimer_agent_id,
                    *refunded_bond_restricted_amount,
                )?;
                credit_main_token_liquid_balance(
                    self,
                    claimer_agent_id,
                    *refunded_bond_liquid_amount,
                )?;
                increase_main_token_circulating_supply(self, *refunded_bond_amount)?;
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
                    credit_main_token_restricted_starter_claim_balance(
                        self,
                        claimer_agent_id,
                        *refunded_bond_restricted_amount,
                    )?;
                    credit_main_token_liquid_balance(
                        self,
                        claimer_agent_id,
                        *refunded_bond_liquid_amount,
                    )?;
                    increase_main_token_circulating_supply(self, *refunded_bond_amount)?;
                }
                self.agent_claim_last_processed_epoch = self
                    .agent_claim_last_processed_epoch
                    .max(*reclaimed_at_epoch);
                self.agents
                    .get_mut(claimer_agent_id)
                    .expect("claimer existence prechecked")
                    .last_active = now;
            }
            DomainEvent::AllianceFormed {
                proposer_agent_id,
                alliance_id,
                members,
                charter,
            } => {
                for member in members {
                    if !self.agents.contains_key(member) {
                        return Err(WorldError::AgentNotFound {
                            agent_id: member.clone(),
                        });
                    }
                }
                self.alliances.insert(
                    alliance_id.clone(),
                    AllianceState {
                        alliance_id: alliance_id.clone(),
                        members: members.clone(),
                        charter: charter.clone(),
                        formed_by_agent_id: proposer_agent_id.clone(),
                        formed_at: now,
                    },
                );
                if let Some(cell) = self.agents.get_mut(proposer_agent_id) {
                    cell.last_active = now;
                } else {
                    return Err(WorldError::AgentNotFound {
                        agent_id: proposer_agent_id.clone(),
                    });
                }
                for member in members {
                    if let Some(cell) = self.agents.get_mut(member) {
                        cell.last_active = now;
                    }
                }
            }
            DomainEvent::AllianceJoined {
                operator_agent_id,
                alliance_id,
                member_agent_id,
            } => {
                if !self.agents.contains_key(operator_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: operator_agent_id.clone(),
                    });
                }
                if !self.agents.contains_key(member_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: member_agent_id.clone(),
                    });
                }
                if self.alliances.iter().any(|(id, alliance)| {
                    id != alliance_id
                        && alliance
                            .members
                            .iter()
                            .any(|member| member == member_agent_id)
                }) {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "member {} already belongs to another alliance",
                            member_agent_id
                        ),
                    });
                }
                let alliance = self.alliances.get_mut(alliance_id).ok_or_else(|| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!("alliance not found: {alliance_id}"),
                    }
                })?;
                if !alliance
                    .members
                    .iter()
                    .any(|member| member == operator_agent_id)
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "operator {} is not a member of alliance {}",
                            operator_agent_id, alliance_id
                        ),
                    });
                }
                if alliance
                    .members
                    .iter()
                    .any(|member| member == member_agent_id)
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "member {} already exists in alliance {}",
                            member_agent_id, alliance_id
                        ),
                    });
                }
                alliance.members.push(member_agent_id.clone());
                alliance.members.sort();
                alliance.members.dedup();

                if let Some(cell) = self.agents.get_mut(operator_agent_id) {
                    cell.last_active = now;
                }
                if let Some(cell) = self.agents.get_mut(member_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::AllianceLeft {
                operator_agent_id,
                alliance_id,
                member_agent_id,
            } => {
                if !self.agents.contains_key(operator_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: operator_agent_id.clone(),
                    });
                }
                if !self.agents.contains_key(member_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: member_agent_id.clone(),
                    });
                }
                let alliance = self.alliances.get_mut(alliance_id).ok_or_else(|| {
                    WorldError::ResourceBalanceInvalid {
                        reason: format!("alliance not found: {alliance_id}"),
                    }
                })?;
                if !alliance
                    .members
                    .iter()
                    .any(|member| member == operator_agent_id)
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "operator {} is not a member of alliance {}",
                            operator_agent_id, alliance_id
                        ),
                    });
                }
                let before_len = alliance.members.len();
                alliance.members.retain(|member| member != member_agent_id);
                if alliance.members.len() == before_len {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "member {} not found in alliance {}",
                            member_agent_id, alliance_id
                        ),
                    });
                }
                if alliance.members.len() < ALLIANCE_MIN_MEMBER_COUNT {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "alliance {} member count below minimum {}",
                            alliance_id, ALLIANCE_MIN_MEMBER_COUNT
                        ),
                    });
                }

                if let Some(cell) = self.agents.get_mut(operator_agent_id) {
                    cell.last_active = now;
                }
                if let Some(cell) = self.agents.get_mut(member_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::AllianceDissolved {
                operator_agent_id,
                alliance_id,
                reason: _,
                former_members,
            } => {
                let has_active_war = self.wars.values().any(|war| {
                    war.active
                        && (war.aggressor_alliance_id == *alliance_id
                            || war.defender_alliance_id == *alliance_id)
                });
                if has_active_war {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "cannot dissolve alliance {} while active war exists",
                            alliance_id
                        ),
                    });
                }
                let Some(alliance) = self.alliances.remove(alliance_id) else {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!("alliance not found: {alliance_id}"),
                    });
                };
                if !alliance
                    .members
                    .iter()
                    .any(|member| member == operator_agent_id)
                {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "operator {} is not a member of alliance {}",
                            operator_agent_id, alliance_id
                        ),
                    });
                }
                if let Some(cell) = self.agents.get_mut(operator_agent_id) {
                    cell.last_active = now;
                } else {
                    return Err(WorldError::AgentNotFound {
                        agent_id: operator_agent_id.clone(),
                    });
                }
                let members_for_touch = if former_members.is_empty() {
                    alliance.members
                } else {
                    former_members.clone()
                };
                for member in members_for_touch {
                    if let Some(cell) = self.agents.get_mut(member.as_str()) {
                        cell.last_active = now;
                    }
                }
            }
            DomainEvent::WarDeclared {
                initiator_agent_id,
                war_id,
                aggressor_alliance_id,
                defender_alliance_id,
                objective,
                intensity,
                mobilization_electricity_cost,
                mobilization_data_cost,
            } => {
                if !self.alliances.contains_key(aggressor_alliance_id) {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "war declare aggressor alliance missing: {}",
                            aggressor_alliance_id
                        ),
                    });
                }
                if !self.alliances.contains_key(defender_alliance_id) {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "war declare defender alliance missing: {}",
                            defender_alliance_id
                        ),
                    });
                }
                let Some(initiator) = self.agents.get_mut(initiator_agent_id) else {
                    return Err(WorldError::AgentNotFound {
                        agent_id: initiator_agent_id.clone(),
                    });
                };
                initiator
                    .state
                    .resources
                    .remove(ResourceKind::Electricity, *mobilization_electricity_cost)
                    .map_err(|err| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "war mobilization electricity debit failed for {}: {:?}",
                            initiator_agent_id, err
                        ),
                    })?;
                initiator
                    .state
                    .resources
                    .remove(ResourceKind::Data, *mobilization_data_cost)
                    .map_err(|err| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "war mobilization data debit failed for {}: {:?}",
                            initiator_agent_id, err
                        ),
                    })?;
                self.wars.insert(
                    war_id.clone(),
                    WarState {
                        war_id: war_id.clone(),
                        initiator_agent_id: initiator_agent_id.clone(),
                        aggressor_alliance_id: aggressor_alliance_id.clone(),
                        defender_alliance_id: defender_alliance_id.clone(),
                        objective: objective.clone(),
                        intensity: *intensity,
                        active: true,
                        declared_mobilization_electricity_cost: *mobilization_electricity_cost,
                        declared_mobilization_data_cost: *mobilization_data_cost,
                        max_duration_ticks: 6_u64.saturating_add(u64::from(*intensity) * 2),
                        aggressor_score: 0,
                        defender_score: 0,
                        concluded_at: None,
                        winner_alliance_id: None,
                        loser_alliance_id: None,
                        settlement_summary: None,
                        participant_outcomes: Vec::new(),
                        declared_at: now,
                    },
                );
                if let Some(cell) = self.agents.get_mut(initiator_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::WarConcluded {
                war_id,
                winner_alliance_id,
                loser_alliance_id,
                aggressor_score,
                defender_score,
                summary,
                participant_outcomes,
            } => {
                let Some(state) = self.wars.get_mut(war_id) else {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!("war not found for conclusion: {war_id}"),
                    });
                };
                state.active = false;
                state.aggressor_score = *aggressor_score;
                state.defender_score = *defender_score;
                state.concluded_at = Some(now);
                state.winner_alliance_id = Some(winner_alliance_id.clone());
                let resolved_loser_alliance_id = if loser_alliance_id.is_empty() {
                    if state.aggressor_alliance_id == *winner_alliance_id {
                        state.defender_alliance_id.clone()
                    } else {
                        state.aggressor_alliance_id.clone()
                    }
                } else {
                    loser_alliance_id.clone()
                };
                state.loser_alliance_id = Some(resolved_loser_alliance_id);
                state.settlement_summary = Some(summary.clone());
                state.participant_outcomes = participant_outcomes.clone();

                apply_war_participant_outcomes(
                    &mut self.agents,
                    &mut self.reputation_scores,
                    participant_outcomes,
                    now,
                )?;
            }
            _ => unreachable!("apply_domain_event_gameplay received unsupported event variant"),
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
