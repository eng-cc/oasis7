use super::super::agent_claims::{
    agent_claim_quote, split_agent_claim_bond_refund, split_agent_claim_spend,
    split_agent_claim_upfront_funding,
};
use super::super::main_token::{
    RestrictedStarterClaimGrantStatus, RestrictedStarterClaimRefundSink,
    MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL, MAIN_TOKEN_TREASURY_BUCKET_SLASH,
};
use super::*;

#[path = "apply_domain_event_gameplay_claims.rs"]
mod claims;

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

                touch_agent_last_active_required(self, operator_agent_id, now)?;
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
            event @ DomainEvent::AgentClaimed { .. }
            | event @ DomainEvent::AgentClaimReleaseRequested { .. }
            | event @ DomainEvent::AgentClaimUpkeepSettled { .. }
            | event @ DomainEvent::AgentClaimEnteredGrace { .. }
            | event @ DomainEvent::AgentClaimIdleWarning { .. }
            | event @ DomainEvent::AgentClaimReleased { .. }
            | event @ DomainEvent::AgentClaimReclaimed { .. } => {
                self.apply_domain_event_gameplay_claims(event, now)?;
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
