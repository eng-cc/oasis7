use super::*;
use crate::runtime::agent_claims::split_agent_claim_upfront_funding;

impl World {
    pub(super) fn action_to_event_gameplay(
        &self,
        action_id: ActionId,
        action: &Action,
    ) -> Result<WorldEventBody, WorldError> {
        match action {
            Action::ClaimAgent {
                claimer_agent_id,
                target_agent_id,
            } => {
                if !self.state.agents.contains_key(claimer_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: claimer_agent_id.clone(),
                        },
                    }));
                }
                if !self.state.agents.contains_key(target_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: target_agent_id.clone(),
                        },
                    }));
                }
                if self.state.agent_claims.contains_key(target_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "agent already claimed: target_agent_id={target_agent_id}"
                            )],
                        },
                    }));
                }

                let quote = match self.agent_claim_quote_for_owner(claimer_agent_id) {
                    Ok(quote) => quote,
                    Err(err) => {
                        return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!("claim agent rejected: {err:?}")],
                            },
                        }));
                    }
                };
                let funding = match split_agent_claim_upfront_funding(
                    quote.slot_index,
                    self.main_token_liquid_balance(claimer_agent_id),
                    self.main_token_restricted_starter_claim_balance(claimer_agent_id),
                    quote.activation_fee_amount,
                    quote.claim_bond_amount,
                    quote.upkeep_per_epoch,
                ) {
                    Ok(funding) => funding,
                    Err(reason) => {
                        return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!(
                                    "claim agent rejected: restricted/liquid funding unavailable: {reason}"
                                )],
                            },
                        }));
                    }
                };
                let current_epoch = self.current_governance_epoch();
                let event = DomainEvent::AgentClaimed {
                    claimer_agent_id: claimer_agent_id.clone(),
                    target_agent_id: target_agent_id.clone(),
                    reputation_tier: quote.reputation_tier,
                    slot_index: quote.slot_index,
                    activation_fee_amount: quote.activation_fee_amount,
                    activation_fee_burn_amount: quote.activation_fee_burn_amount,
                    activation_fee_treasury_amount: quote.activation_fee_treasury_amount,
                    claim_bond_amount: quote.claim_bond_amount,
                    upfront_restricted_spent_amount: funding.upfront.restricted_amount,
                    upfront_liquid_spent_amount: funding.upfront.liquid_amount,
                    claim_bond_locked_restricted_amount: funding.claim_bond.restricted_amount,
                    claim_bond_locked_liquid_amount: funding.claim_bond.liquid_amount,
                    upkeep_per_epoch: quote.upkeep_per_epoch,
                    claimed_at_epoch: current_epoch,
                    upkeep_paid_through_epoch: current_epoch,
                    release_cooldown_epochs: quote.release_cooldown_epochs,
                    grace_epochs: quote.grace_epochs,
                    idle_warning_epochs: quote.idle_warning_epochs,
                    forced_idle_reclaim_epochs: quote.forced_idle_reclaim_epochs,
                    forced_reclaim_penalty_bps: quote.forced_reclaim_penalty_bps,
                };
                let mut preview_state = self.state.clone();
                if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("claim agent rejected: {err:?}")],
                        },
                    }));
                }
                Ok(WorldEventBody::Domain(event))
            }
            Action::ReleaseAgentClaim {
                claimer_agent_id,
                target_agent_id,
            } => {
                if !self.state.agents.contains_key(claimer_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: claimer_agent_id.clone(),
                        },
                    }));
                }
                let Some(claim) = self.state.agent_claims.get(target_agent_id) else {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "agent claim not found: target_agent_id={target_agent_id}"
                            )],
                        },
                    }));
                };
                if claim.claim_owner_id != *claimer_agent_id {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "agent claim owner mismatch: target_agent_id={} owner={} claimer={}",
                                target_agent_id, claim.claim_owner_id, claimer_agent_id
                            )],
                        },
                    }));
                }
                let requested_at_epoch = self.current_governance_epoch();
                let ready_at_epoch =
                    requested_at_epoch.saturating_add(claim.release_cooldown_epochs);
                let event = DomainEvent::AgentClaimReleaseRequested {
                    claimer_agent_id: claimer_agent_id.clone(),
                    target_agent_id: target_agent_id.clone(),
                    requested_at_epoch,
                    ready_at_epoch,
                };
                let mut preview_state = self.state.clone();
                if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("release agent claim rejected: {err:?}")],
                        },
                    }));
                }
                Ok(WorldEventBody::Domain(event))
            }
            Action::FormAlliance {
                proposer_agent_id,
                alliance_id,
                members,
                charter,
            } => {
                if !self.state.agents.contains_key(proposer_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: proposer_agent_id.clone(),
                        },
                    }));
                }
                let alliance_id = alliance_id.trim();
                if alliance_id.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["alliance_id cannot be empty".to_string()],
                        },
                    }));
                }
                if self.state.alliances.contains_key(alliance_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("alliance already exists: {alliance_id}")],
                        },
                    }));
                }
                let charter = charter.trim();
                if charter.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["alliance charter cannot be empty".to_string()],
                        },
                    }));
                }

                let mut member_set = BTreeSet::new();
                member_set.insert(proposer_agent_id.trim().to_string());
                for member in members {
                    let member = member.trim();
                    if member.is_empty() {
                        continue;
                    }
                    member_set.insert(member.to_string());
                }
                if member_set.len() < 2 {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["alliance requires at least 2 unique members".to_string()],
                        },
                    }));
                }
                let normalized_members: Vec<String> = member_set.into_iter().collect();
                if normalized_members.len() > WAR_MAX_ALLIANCE_MEMBERS {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "alliance members exceed max {WAR_MAX_ALLIANCE_MEMBERS}"
                            )],
                        },
                    }));
                }
                for member in &normalized_members {
                    if !self.state.agents.contains_key(member) {
                        return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::AgentNotFound {
                                agent_id: member.clone(),
                            },
                        }));
                    }
                    if let Some(current_alliance_id) = self.agent_alliance_id(member.as_str()) {
                        return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                            action_id,
                            reason: RejectReason::RuleDenied {
                                notes: vec![format!(
                                    "member {} already belongs to alliance {}",
                                    member, current_alliance_id
                                )],
                            },
                        }));
                    }
                }
                Ok(WorldEventBody::Domain(DomainEvent::AllianceFormed {
                    proposer_agent_id: proposer_agent_id.clone(),
                    alliance_id: alliance_id.to_string(),
                    members: normalized_members,
                    charter: charter.to_string(),
                }))
            }
            Action::JoinAlliance {
                operator_agent_id,
                alliance_id,
                member_agent_id,
            } => {
                if !self.state.agents.contains_key(operator_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: operator_agent_id.clone(),
                        },
                    }));
                }
                if !self.state.agents.contains_key(member_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: member_agent_id.clone(),
                        },
                    }));
                }
                let alliance_id = alliance_id.trim();
                if alliance_id.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["alliance_id cannot be empty".to_string()],
                        },
                    }));
                }
                let Some(alliance) = self.state.alliances.get(alliance_id) else {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("alliance not found: {alliance_id}")],
                        },
                    }));
                };
                if !alliance
                    .members
                    .iter()
                    .any(|member| member == operator_agent_id)
                {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "operator {} is not a member of alliance {}",
                                operator_agent_id, alliance_id
                            )],
                        },
                    }));
                }
                if alliance
                    .members
                    .iter()
                    .any(|member| member == member_agent_id)
                {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "member {} already in alliance {}",
                                member_agent_id, alliance_id
                            )],
                        },
                    }));
                }
                if let Some(current_alliance_id) = self.agent_alliance_id(member_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "member {} already belongs to alliance {}",
                                member_agent_id, current_alliance_id
                            )],
                        },
                    }));
                }
                if self.alliance_has_active_war(alliance_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "alliance {} has active war and cannot change members",
                                alliance_id
                            )],
                        },
                    }));
                }
                if alliance.members.len().saturating_add(1) > WAR_MAX_ALLIANCE_MEMBERS {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "alliance members exceed max {WAR_MAX_ALLIANCE_MEMBERS}"
                            )],
                        },
                    }));
                }
                Ok(WorldEventBody::Domain(DomainEvent::AllianceJoined {
                    operator_agent_id: operator_agent_id.clone(),
                    alliance_id: alliance_id.to_string(),
                    member_agent_id: member_agent_id.clone(),
                }))
            }
            Action::LeaveAlliance {
                operator_agent_id,
                alliance_id,
                member_agent_id,
            } => {
                if !self.state.agents.contains_key(operator_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: operator_agent_id.clone(),
                        },
                    }));
                }
                if !self.state.agents.contains_key(member_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: member_agent_id.clone(),
                        },
                    }));
                }
                let alliance_id = alliance_id.trim();
                if alliance_id.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["alliance_id cannot be empty".to_string()],
                        },
                    }));
                }
                let Some(alliance) = self.state.alliances.get(alliance_id) else {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("alliance not found: {alliance_id}")],
                        },
                    }));
                };
                if !alliance
                    .members
                    .iter()
                    .any(|member| member == operator_agent_id)
                {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "operator {} is not a member of alliance {}",
                                operator_agent_id, alliance_id
                            )],
                        },
                    }));
                }
                if !alliance
                    .members
                    .iter()
                    .any(|member| member == member_agent_id)
                {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "member {} is not in alliance {}",
                                member_agent_id, alliance_id
                            )],
                        },
                    }));
                }
                if self.alliance_has_active_war(alliance_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "alliance {} has active war and cannot change members",
                                alliance_id
                            )],
                        },
                    }));
                }
                if alliance.members.len() <= WAR_MIN_ALLIANCE_MEMBERS {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "alliance must keep at least {WAR_MIN_ALLIANCE_MEMBERS} members; use dissolve_alliance instead"
                            )],
                        },
                    }));
                }
                Ok(WorldEventBody::Domain(DomainEvent::AllianceLeft {
                    operator_agent_id: operator_agent_id.clone(),
                    alliance_id: alliance_id.to_string(),
                    member_agent_id: member_agent_id.clone(),
                }))
            }
            Action::DissolveAlliance {
                operator_agent_id,
                alliance_id,
                reason,
            } => {
                if !self.state.agents.contains_key(operator_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: operator_agent_id.clone(),
                        },
                    }));
                }
                let alliance_id = alliance_id.trim();
                if alliance_id.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["alliance_id cannot be empty".to_string()],
                        },
                    }));
                }
                let Some(alliance) = self.state.alliances.get(alliance_id) else {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("alliance not found: {alliance_id}")],
                        },
                    }));
                };
                if !alliance
                    .members
                    .iter()
                    .any(|member| member == operator_agent_id)
                {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "operator {} is not a member of alliance {}",
                                operator_agent_id, alliance_id
                            )],
                        },
                    }));
                }
                if self.alliance_has_active_war(alliance_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "alliance {} has active war and cannot dissolve",
                                alliance_id
                            )],
                        },
                    }));
                }
                let reason = reason.trim();
                if reason.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["dissolve reason cannot be empty".to_string()],
                        },
                    }));
                }
                Ok(WorldEventBody::Domain(DomainEvent::AllianceDissolved {
                    operator_agent_id: operator_agent_id.clone(),
                    alliance_id: alliance_id.to_string(),
                    reason: reason.to_string(),
                    former_members: alliance.members.clone(),
                }))
            }
            Action::DeclareWar {
                initiator_agent_id,
                war_id,
                aggressor_alliance_id,
                defender_alliance_id,
                objective,
                intensity,
            } => {
                if !self.state.agents.contains_key(initiator_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: initiator_agent_id.clone(),
                        },
                    }));
                }
                let war_id = war_id.trim();
                if war_id.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["war_id cannot be empty".to_string()],
                        },
                    }));
                }
                if self.state.wars.contains_key(war_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("war already exists: {war_id}")],
                        },
                    }));
                }
                let aggressor_alliance_id = aggressor_alliance_id.trim();
                let defender_alliance_id = defender_alliance_id.trim();
                if aggressor_alliance_id.is_empty() || defender_alliance_id.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![
                                "aggressor_alliance_id and defender_alliance_id cannot be empty"
                                    .to_string(),
                            ],
                        },
                    }));
                }
                if aggressor_alliance_id == defender_alliance_id {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![
                                "aggressor_alliance_id and defender_alliance_id must differ"
                                    .to_string(),
                            ],
                        },
                    }));
                }
                let Some(aggressor) = self.state.alliances.get(aggressor_alliance_id) else {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "aggressor alliance not found: {}",
                                aggressor_alliance_id
                            )],
                        },
                    }));
                };
                if aggressor.members.len() < WAR_MIN_ALLIANCE_MEMBERS {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "aggressor alliance requires at least {WAR_MIN_ALLIANCE_MEMBERS} members"
                            )],
                        },
                    }));
                }
                if !aggressor
                    .members
                    .iter()
                    .any(|member| member == initiator_agent_id)
                {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "initiator {} is not a member of aggressor alliance {}",
                                initiator_agent_id, aggressor_alliance_id
                            )],
                        },
                    }));
                }
                let Some(defender) = self.state.alliances.get(defender_alliance_id) else {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "defender alliance not found: {}",
                                defender_alliance_id
                            )],
                        },
                    }));
                };
                if defender.members.len() < WAR_MIN_ALLIANCE_MEMBERS {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "defender alliance requires at least {WAR_MIN_ALLIANCE_MEMBERS} members"
                            )],
                        },
                    }));
                }
                let aggressor_member_set: BTreeSet<&str> =
                    aggressor.members.iter().map(String::as_str).collect();
                let has_member_overlap = defender
                    .members
                    .iter()
                    .any(|member| aggressor_member_set.contains(member.as_str()));
                if has_member_overlap {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![
                                "aggressor and defender alliances cannot share members".to_string()
                            ],
                        },
                    }));
                }
                let objective = objective.trim();
                if objective.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["war objective cannot be empty".to_string()],
                        },
                    }));
                }
                if *intensity == 0 {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["war intensity must be > 0".to_string()],
                        },
                    }));
                }
                if *intensity > WAR_MAX_INTENSITY {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("war intensity exceeds max {}", WAR_MAX_INTENSITY)],
                        },
                    }));
                }
                let has_active_conflict = self.state.wars.values().any(|war| {
                    if !war.active {
                        return false;
                    }
                    war.aggressor_alliance_id == aggressor_alliance_id
                        || war.defender_alliance_id == aggressor_alliance_id
                        || war.aggressor_alliance_id == defender_alliance_id
                        || war.defender_alliance_id == defender_alliance_id
                });
                if has_active_conflict {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["aggressor or defender alliance already has an active war"
                                .to_string()],
                        },
                    }));
                }
                let (mobilization_electricity_cost, mobilization_data_cost) =
                    Self::war_mobilization_costs(*intensity);
                let initiator_electricity = self
                    .state
                    .agents
                    .get(initiator_agent_id)
                    .map(|cell| cell.state.resources.get(ResourceKind::Electricity))
                    .unwrap_or(0);
                if initiator_electricity < mobilization_electricity_cost {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::InsufficientResource {
                            agent_id: initiator_agent_id.clone(),
                            kind: ResourceKind::Electricity,
                            requested: mobilization_electricity_cost,
                            available: initiator_electricity,
                        },
                    }));
                }
                let initiator_data = self
                    .state
                    .agents
                    .get(initiator_agent_id)
                    .map(|cell| cell.state.resources.get(ResourceKind::Data))
                    .unwrap_or(0);
                if initiator_data < mobilization_data_cost {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::InsufficientResource {
                            agent_id: initiator_agent_id.clone(),
                            kind: ResourceKind::Data,
                            requested: mobilization_data_cost,
                            available: initiator_data,
                        },
                    }));
                }
                Ok(WorldEventBody::Domain(DomainEvent::WarDeclared {
                    initiator_agent_id: initiator_agent_id.clone(),
                    war_id: war_id.to_string(),
                    aggressor_alliance_id: aggressor_alliance_id.to_string(),
                    defender_alliance_id: defender_alliance_id.to_string(),
                    objective: objective.to_string(),
                    intensity: *intensity,
                    mobilization_electricity_cost,
                    mobilization_data_cost,
                }))
            }
            Action::OpenGovernanceProposal {
                proposer_agent_id,
                proposal_key,
                title,
                description,
                options,
                voting_window_ticks,
                quorum_weight,
                pass_threshold_bps,
            } => {
                if !self.state.agents.contains_key(proposer_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: proposer_agent_id.clone(),
                        },
                    }));
                }
                let proposal_key = proposal_key.trim();
                if proposal_key.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["proposal_key cannot be empty".to_string()],
                        },
                    }));
                }
                if self.state.governance_proposals.contains_key(proposal_key) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("proposal already exists: {proposal_key}")],
                        },
                    }));
                }
                let title = title.trim();
                if title.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["proposal title cannot be empty".to_string()],
                        },
                    }));
                }
                let description = description.trim();
                if description.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["proposal description cannot be empty".to_string()],
                        },
                    }));
                }
                if *voting_window_ticks < GOVERNANCE_MIN_VOTING_WINDOW_TICKS
                    || *voting_window_ticks > GOVERNANCE_MAX_VOTING_WINDOW_TICKS
                {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "voting_window_ticks must be within {}..={}",
                                GOVERNANCE_MIN_VOTING_WINDOW_TICKS,
                                GOVERNANCE_MAX_VOTING_WINDOW_TICKS
                            )],
                        },
                    }));
                }
                if *quorum_weight == 0 {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["quorum_weight must be > 0".to_string()],
                        },
                    }));
                }
                if *pass_threshold_bps < GOVERNANCE_MIN_PASS_THRESHOLD_BPS
                    || *pass_threshold_bps > GOVERNANCE_MAX_PASS_THRESHOLD_BPS
                {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "pass_threshold_bps must be within {}..={}",
                                GOVERNANCE_MIN_PASS_THRESHOLD_BPS,
                                GOVERNANCE_MAX_PASS_THRESHOLD_BPS
                            )],
                        },
                    }));
                }
                let mut unique_options = BTreeSet::new();
                for value in options {
                    let normalized = value.trim();
                    if normalized.is_empty() {
                        continue;
                    }
                    unique_options.insert(normalized.to_string());
                }
                if unique_options.len() < 2 {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["governance proposal requires at least 2 unique options"
                                .to_string()],
                        },
                    }));
                }
                let closes_at = self.state.time.saturating_add(*voting_window_ticks);
                Ok(WorldEventBody::Domain(
                    DomainEvent::GovernanceProposalOpened {
                        proposer_agent_id: proposer_agent_id.clone(),
                        proposal_key: proposal_key.to_string(),
                        title: title.to_string(),
                        description: description.to_string(),
                        options: unique_options.into_iter().collect(),
                        voting_window_ticks: *voting_window_ticks,
                        closes_at,
                        quorum_weight: *quorum_weight,
                        pass_threshold_bps: *pass_threshold_bps,
                    },
                ))
            }
            Action::CastGovernanceVote {
                voter_agent_id,
                proposal_key,
                option,
                weight,
            } => {
                if !self.state.agents.contains_key(voter_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: voter_agent_id.clone(),
                        },
                    }));
                }
                let proposal_key = proposal_key.trim();
                if proposal_key.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["proposal_key cannot be empty".to_string()],
                        },
                    }));
                }
                let Some(proposal) = self.state.governance_proposals.get(proposal_key) else {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("governance proposal not found: {}", proposal_key)],
                        },
                    }));
                };
                if proposal.status != GovernanceProposalStatus::Open {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "governance proposal is not open: {}",
                                proposal_key
                            )],
                        },
                    }));
                }
                if self.state.time > proposal.closes_at {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "governance proposal has closed at {}",
                                proposal.closes_at
                            )],
                        },
                    }));
                }
                let option = option.trim();
                if option.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["vote option cannot be empty".to_string()],
                        },
                    }));
                }
                if !proposal.options.iter().any(|candidate| candidate == option) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "vote option '{}' is not allowed for proposal {}",
                                option, proposal_key
                            )],
                        },
                    }));
                }
                if *weight == 0 {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["vote weight must be > 0".to_string()],
                        },
                    }));
                }
                if *weight > GOVERNANCE_MAX_VOTE_WEIGHT {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "vote weight must be <= {GOVERNANCE_MAX_VOTE_WEIGHT}"
                            )],
                        },
                    }));
                }
                if let Err(WorldError::ResourceBalanceInvalid { reason }) = self
                    .state
                    .governance_effective_vote_weight_for_agent(proposal, voter_agent_id, *weight)
                {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![reason],
                        },
                    }));
                }
                Ok(WorldEventBody::Domain(DomainEvent::GovernanceVoteCast {
                    voter_agent_id: voter_agent_id.clone(),
                    proposal_key: proposal_key.to_string(),
                    option: option.to_string(),
                    weight: *weight,
                }))
            }
            Action::ResolveCrisis {
                resolver_agent_id,
                crisis_id,
                strategy,
                success,
            } => {
                if !self.state.agents.contains_key(resolver_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: resolver_agent_id.clone(),
                        },
                    }));
                }
                let crisis_id = crisis_id.trim();
                if crisis_id.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["crisis_id cannot be empty".to_string()],
                        },
                    }));
                }
                let Some(crisis) = self.state.crises.get(crisis_id) else {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("crisis not found: {crisis_id}")],
                        },
                    }));
                };
                if crisis.status != CrisisStatus::Active {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "crisis is not active and cannot be resolved: {}",
                                crisis_id
                            )],
                        },
                    }));
                }
                if self.state.time > crisis.expires_at {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "crisis expired at {} and cannot be resolved: {}",
                                crisis.expires_at, crisis_id
                            )],
                        },
                    }));
                }
                let strategy = strategy.trim();
                if strategy.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["crisis strategy cannot be empty".to_string()],
                        },
                    }));
                }
                let severity = crisis.severity.max(1);
                let impact = if *success {
                    i64::from(severity).saturating_mul(CRISIS_BASE_IMPACT_PER_SEVERITY)
                } else {
                    -i64::from(severity).saturating_mul(CRISIS_BASE_IMPACT_PER_SEVERITY)
                };
                Ok(WorldEventBody::Domain(DomainEvent::CrisisResolved {
                    resolver_agent_id: resolver_agent_id.clone(),
                    crisis_id: crisis_id.to_string(),
                    strategy: strategy.to_string(),
                    success: *success,
                    impact,
                }))
            }
            Action::GrantMetaProgress {
                operator_agent_id,
                target_agent_id,
                track,
                points,
                achievement_id,
            } => {
                if !self.state.agents.contains_key(operator_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: operator_agent_id.clone(),
                        },
                    }));
                }
                if !self.state.agents.contains_key(target_agent_id) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::AgentNotFound {
                            agent_id: target_agent_id.clone(),
                        },
                    }));
                }
                let track = track.trim();
                if track.is_empty() {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["meta progression track cannot be empty".to_string()],
                        },
                    }));
                }
                if *points == 0 {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::InvalidAmount { amount: *points },
                    }));
                }
                let normalized_achievement = achievement_id.as_ref().map(|value| value.trim());
                if normalized_achievement.is_some_and(|value| value.is_empty()) {
                    return Ok(WorldEventBody::Domain(DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec!["achievement_id cannot be empty".to_string()],
                        },
                    }));
                }
                Ok(WorldEventBody::Domain(DomainEvent::MetaProgressGranted {
                    operator_agent_id: operator_agent_id.clone(),
                    target_agent_id: target_agent_id.clone(),
                    track: track.to_string(),
                    points: *points,
                    achievement_id: normalized_achievement.map(str::to_string),
                }))
            }
            _ => unreachable!("action_to_event_gameplay received unsupported action variant"),
        }
    }
}
