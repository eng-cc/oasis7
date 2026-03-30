use super::*;

impl World {
    pub(super) fn action_to_event_policy_contract(
        &self,
        action_id: ActionId,
        action: &Action,
    ) -> Result<WorldEventBody, WorldError> {
        match action {
            Action::UpdateGameplayPolicy {
                operator_agent_id,
                electricity_tax_bps,
                data_tax_bps,
                power_trade_fee_bps,
                max_open_contracts_per_agent,
                blocked_agents,
                forbidden_location_ids,
            } => {
                if !self.state.agents.contains_key(operator_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: operator_agent_id.clone(),
                        },
                    }));
                }
                if !self.has_policy_update_governance_authorization(operator_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "update gameplay policy requires passed governance proposal total_weight >= {}",
                                GAMEPLAY_POLICY_UPDATE_MIN_GOVERNANCE_TOTAL_WEIGHT
                            )],
                        },
                    }));
                }
                if *electricity_tax_bps > GAMEPLAY_POLICY_MAX_TAX_BPS {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "electricity_tax_bps must be <= {}",
                                GAMEPLAY_POLICY_MAX_TAX_BPS
                            )],
                        },
                    }));
                }
                if *data_tax_bps > GAMEPLAY_POLICY_MAX_TAX_BPS {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "data_tax_bps must be <= {}",
                                GAMEPLAY_POLICY_MAX_TAX_BPS
                            )],
                        },
                    }));
                }
                if *power_trade_fee_bps > GAMEPLAY_POLICY_MAX_TAX_BPS {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "power_trade_fee_bps must be <= {}",
                                GAMEPLAY_POLICY_MAX_TAX_BPS
                            )],
                        },
                    }));
                }
                if *max_open_contracts_per_agent < GAMEPLAY_POLICY_MIN_CONTRACT_QUOTA
                    || *max_open_contracts_per_agent > GAMEPLAY_POLICY_MAX_CONTRACT_QUOTA
                {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "max_open_contracts_per_agent must be within {}..={}",
                                GAMEPLAY_POLICY_MIN_CONTRACT_QUOTA,
                                GAMEPLAY_POLICY_MAX_CONTRACT_QUOTA
                            )],
                        },
                    }));
                }
                let mut normalized_blocked_agents = BTreeSet::new();
                for value in blocked_agents {
                    let candidate = value.trim();
                    if candidate.is_empty() {
                        continue;
                    }
                    if !self.state.agents.contains_key(candidate) {
                        return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::AgentNotFound {
                                agent_id: candidate.to_string(),
                            },
                        }));
                    }
                    normalized_blocked_agents.insert(candidate.to_string());
                }
                let mut normalized_forbidden_location_ids = BTreeSet::new();
                for value in forbidden_location_ids {
                    let candidate = value.trim();
                    if candidate.is_empty() {
                        continue;
                    }
                    normalized_forbidden_location_ids.insert(candidate.to_string());
                }
                Ok(WorldEventBody::Domain(DomainEvent::GameplayPolicyUpdated {
                    operator_agent_id: operator_agent_id.clone(),
                    electricity_tax_bps: *electricity_tax_bps,
                    data_tax_bps: *data_tax_bps,
                    power_trade_fee_bps: *power_trade_fee_bps,
                    max_open_contracts_per_agent: *max_open_contracts_per_agent,
                    blocked_agents: normalized_blocked_agents.into_iter().collect(),
                    forbidden_location_ids: normalized_forbidden_location_ids.into_iter().collect(),
                }))
            }
            Action::UpdateRestrictedStarterClaimAdminRegistry {
                controller_account_id,
                next_admin_account_ids,
            } => {
                let Some(current_registry) = self.governance_main_token_controller_registry() else {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![
                                "update restricted claim admin registry rejected: main token controller registry is not configured"
                                    .to_string(),
                            ],
                        },
                    }));
                };
                let controller_account_id = controller_account_id.trim();
                let expected_controller_account_id =
                    match Self::restricted_starter_claim_admin_registry_controller_account_id(
                        current_registry,
                    ) {
                        Ok(account_id) => account_id,
                        Err(err) => {
                            return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                                action_id,
                                reason: RejectReason::RuleDenied {
                                    notes: vec![format!(
                                        "update restricted claim admin registry rejected: {err:?}"
                                    )],
                                },
                            }))
                        }
                    };
                if controller_account_id != expected_controller_account_id {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "update restricted claim admin registry rejected: controller_account_id does not match ecosystem treasury controller slot expected={} actual={}",
                                expected_controller_account_id, controller_account_id
                            )],
                        },
                    }));
                }
                let next_admin_account_ids = next_admin_account_ids
                    .iter()
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string)
                    .collect::<BTreeSet<String>>();
                let mut next_registry = current_registry.clone();
                next_registry.restricted_starter_claim_admin_account_ids =
                    next_admin_account_ids.clone();
                if let Err(err) =
                    Self::validate_governance_main_token_controller_registry(next_registry)
                {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "update restricted claim admin registry rejected: {err:?}"
                            )],
                        },
                    }));
                }
                Ok(WorldEventBody::Governance(
                    GovernanceEvent::RestrictedStarterClaimAdminRegistryUpdated {
                        controller_account_id: controller_account_id.to_string(),
                        previous_admin_account_ids: current_registry
                            .restricted_starter_claim_admin_account_ids
                            .iter()
                            .cloned()
                            .collect(),
                        next_admin_account_ids: next_admin_account_ids.into_iter().collect(),
                    },
                ))
            }
            Action::OpenEconomicContract {
                creator_agent_id,
                contract_id,
                counterparty_agent_id,
                settlement_kind,
                settlement_amount,
                reputation_stake,
                expires_at,
                description,
            } => {
                if !self.state.agents.contains_key(creator_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: creator_agent_id.clone(),
                        },
                    }));
                }
                if !self.state.agents.contains_key(counterparty_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: counterparty_agent_id.clone(),
                        },
                    }));
                }
                if creator_agent_id == counterparty_agent_id {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["economic contract requires distinct parties".to_string()],
                        },
                    }));
                }
                let contract_id = contract_id.trim();
                if contract_id.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["contract_id cannot be empty".to_string()],
                        },
                    }));
                }
                if self.state.economic_contracts.contains_key(contract_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("economic contract already exists: {contract_id}")],
                        },
                    }));
                }
                if *settlement_amount <= 0 {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::InvalidAmount {
                            amount: *settlement_amount,
                        },
                    }));
                }
                if *reputation_stake <= 0
                    || *reputation_stake > ECONOMIC_CONTRACT_MAX_REPUTATION_STAKE
                {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "reputation_stake must be within 1..={}",
                                ECONOMIC_CONTRACT_MAX_REPUTATION_STAKE
                            )],
                        },
                    }));
                }
                if *expires_at <= self.state.time {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![
                                "expires_at must be greater than current world time".to_string()
                            ],
                        },
                    }));
                }
                let description = description.trim();
                if description.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["economic contract description cannot be empty".to_string()],
                        },
                    }));
                }
                if self
                    .state
                    .gameplay_policy
                    .blocked_agents
                    .iter()
                    .any(|value| value == creator_agent_id || value == counterparty_agent_id)
                {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["economic contract blocked by gameplay policy".to_string()],
                        },
                    }));
                }
                let active_contract_count = self
                    .state
                    .economic_contracts
                    .values()
                    .filter(|contract| {
                        contract.creator_agent_id == *creator_agent_id
                            && matches!(
                                contract.status,
                                EconomicContractStatus::Open | EconomicContractStatus::Accepted
                            )
                    })
                    .count();
                if active_contract_count
                    >= usize::from(self.state.gameplay_policy.max_open_contracts_per_agent)
                {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "economic contract quota exceeded for creator {}",
                                creator_agent_id
                            )],
                        },
                    }));
                }
                Ok(WorldEventBody::Domain(
                    DomainEvent::EconomicContractOpened {
                        creator_agent_id: creator_agent_id.clone(),
                        contract_id: contract_id.to_string(),
                        counterparty_agent_id: counterparty_agent_id.clone(),
                        settlement_kind: *settlement_kind,
                        settlement_amount: *settlement_amount,
                        reputation_stake: *reputation_stake,
                        expires_at: *expires_at,
                        description: description.to_string(),
                    },
                ))
            }
            Action::AcceptEconomicContract {
                accepter_agent_id,
                contract_id,
            } => {
                if !self.state.agents.contains_key(accepter_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: accepter_agent_id.clone(),
                        },
                    }));
                }
                let contract_id = contract_id.trim();
                if contract_id.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["contract_id cannot be empty".to_string()],
                        },
                    }));
                }
                let Some(contract) = self.state.economic_contracts.get(contract_id) else {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("economic contract not found: {contract_id}")],
                        },
                    }));
                };
                if contract.status != EconomicContractStatus::Open {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("economic contract is not open: {}", contract_id)],
                        },
                    }));
                }
                if contract.counterparty_agent_id != *accepter_agent_id {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "contract accepter mismatch expected {}",
                                contract.counterparty_agent_id
                            )],
                        },
                    }));
                }
                if self.state.time > contract.expires_at {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "economic contract has expired at {}",
                                contract.expires_at
                            )],
                        },
                    }));
                }
                Ok(WorldEventBody::Domain(
                    DomainEvent::EconomicContractAccepted {
                        accepter_agent_id: accepter_agent_id.clone(),
                        contract_id: contract_id.to_string(),
                    },
                ))
            }
            Action::SettleEconomicContract {
                operator_agent_id,
                contract_id,
                success,
                notes,
            } => {
                if !self.state.agents.contains_key(operator_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: operator_agent_id.clone(),
                        },
                    }));
                }
                let contract_id = contract_id.trim();
                if contract_id.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["contract_id cannot be empty".to_string()],
                        },
                    }));
                }
                let Some(contract) = self.state.economic_contracts.get(contract_id) else {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("economic contract not found: {contract_id}")],
                        },
                    }));
                };
                if contract.status != EconomicContractStatus::Accepted {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "economic contract status is not accepted: {}",
                                contract_id
                            )],
                        },
                    }));
                }
                if contract.creator_agent_id != *operator_agent_id
                    && contract.counterparty_agent_id != *operator_agent_id
                {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![
                                "settlement operator must belong to contract parties".to_string()
                            ],
                        },
                    }));
                }
                let notes = notes.trim();
                if notes.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![
                                "economic contract settlement notes cannot be empty".to_string()
                            ],
                        },
                    }));
                }

                let (
                    transfer_amount,
                    tax_amount,
                    creator_reputation_delta,
                    counterparty_reputation_delta,
                ) = if *success {
                    if let Some(ready_at) = self.state.economic_contract_pair_cooldown_ready_at(
                        contract.creator_agent_id.as_str(),
                        contract.counterparty_agent_id.as_str(),
                        ECONOMIC_CONTRACT_PAIR_COOLDOWN_TICKS,
                    ) {
                        if self.state.time < ready_at {
                            return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                                action_id,
                                reason: RejectReason::RuleDenied {
                                    notes: vec![format!(
                                        "economic contract settlement denied: pair cooldown active until tick {}",
                                        ready_at
                                    )],
                                },
                            }));
                        }
                    }
                    if contract.settlement_kind == ResourceKind::Data
                        && !self.state.has_data_access_permission(
                            contract.creator_agent_id.as_str(),
                            contract.counterparty_agent_id.as_str(),
                        )
                    {
                        return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!(
                                    "economic contract data settlement denied: missing access grant owner={} grantee={}",
                                    contract.creator_agent_id, contract.counterparty_agent_id
                                )],
                            },
                        }));
                    }
                    let tax_bps = match contract.settlement_kind {
                        ResourceKind::Electricity => self
                            .state
                            .gameplay_policy
                            .electricity_tax_bps
                            .saturating_add(self.state.gameplay_policy.power_trade_fee_bps)
                            .min(GAMEPLAY_POLICY_MAX_TAX_BPS),
                        ResourceKind::Data => self.state.gameplay_policy.data_tax_bps,
                    };
                    let tax_amount = contract
                        .settlement_amount
                        .saturating_mul(i64::from(tax_bps))
                        .saturating_div(10_000);
                    let total_required = contract.settlement_amount.saturating_add(tax_amount);
                    let available = self
                        .state
                        .agents
                        .get(&contract.creator_agent_id)
                        .map(|cell| cell.state.resources.get(contract.settlement_kind))
                        .unwrap_or(0);
                    if available < total_required {
                        return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::InsufficientResource {
                                agent_id: contract.creator_agent_id.clone(),
                                kind: contract.settlement_kind,
                                requested: total_required,
                                available,
                            },
                        }));
                    }
                    let success_reputation_reward =
                        Self::economic_contract_success_reputation_reward(
                            contract.settlement_amount,
                            contract.reputation_stake,
                        );
                    let creator_reward_budget = self.state.available_reputation_reward_budget(
                        contract.creator_agent_id.as_str(),
                        self.state.time,
                        ECONOMIC_CONTRACT_REPUTATION_WINDOW_TICKS,
                        ECONOMIC_CONTRACT_REPUTATION_WINDOW_CAP,
                    );
                    let counterparty_reward_budget = self.state.available_reputation_reward_budget(
                        contract.counterparty_agent_id.as_str(),
                        self.state.time,
                        ECONOMIC_CONTRACT_REPUTATION_WINDOW_TICKS,
                        ECONOMIC_CONTRACT_REPUTATION_WINDOW_CAP,
                    );
                    let creator_reward = success_reputation_reward.min(creator_reward_budget);
                    let counterparty_reward =
                        success_reputation_reward.min(counterparty_reward_budget);
                    (
                        contract.settlement_amount,
                        tax_amount,
                        creator_reward,
                        counterparty_reward,
                    )
                } else {
                    (0, 0, -contract.reputation_stake, 0)
                };

                Ok(WorldEventBody::Domain(
                    DomainEvent::EconomicContractSettled {
                        operator_agent_id: operator_agent_id.clone(),
                        contract_id: contract_id.to_string(),
                        success: *success,
                        transfer_amount,
                        tax_amount,
                        notes: notes.to_string(),
                        creator_reputation_delta,
                        counterparty_reputation_delta,
                    },
                ))
            }
            _ => {
                unreachable!("action_to_event_policy_contract received unsupported action variant")
            }
        }
    }
}
