use super::*;

impl WorldState {
    pub(super) fn apply_domain_event_governance_meta(
        &mut self,
        event: &DomainEvent,
        now: WorldTime,
    ) -> Result<(), WorldError> {
        match event {
            DomainEvent::GovernanceProposalOpened {
                proposer_agent_id,
                proposal_key,
                title,
                description,
                options,
                voting_window_ticks,
                closes_at,
                quorum_weight,
                pass_threshold_bps,
            } => {
                if !self.agents.contains_key(proposer_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: proposer_agent_id.clone(),
                    });
                }
                let mut vote_weight_snapshot = BTreeMap::new();
                for agent_id in self.agents.keys() {
                    vote_weight_snapshot.insert(
                        agent_id.clone(),
                        self.governance_identity_snapshot_for_agent(agent_id.as_str(), now),
                    );
                }
                self.governance_proposals.insert(
                    proposal_key.clone(),
                    GovernanceProposalState {
                        proposal_key: proposal_key.clone(),
                        proposer_agent_id: proposer_agent_id.clone(),
                        title: title.clone(),
                        description: description.clone(),
                        options: options.clone(),
                        voting_window_ticks: *voting_window_ticks,
                        quorum_weight: *quorum_weight,
                        pass_threshold_bps: *pass_threshold_bps,
                        opened_at: now,
                        closes_at: *closes_at,
                        status: GovernanceProposalStatus::Open,
                        finalized_at: None,
                        winning_option: None,
                        winning_weight: 0,
                        total_weight_at_finalize: 0,
                        snapshot_at_tick: now,
                        vote_weight_snapshot,
                    },
                );
                self.governance_votes
                    .entry(proposal_key.clone())
                    .or_insert_with(|| GovernanceVoteState {
                        proposal_key: proposal_key.clone(),
                        votes_by_agent: BTreeMap::new(),
                        tallies: BTreeMap::new(),
                        total_weight: 0,
                        last_updated_at: now,
                    });
                if let Some(cell) = self.agents.get_mut(proposer_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::GovernanceVoteCast {
                voter_agent_id,
                proposal_key,
                option,
                weight,
            } => {
                if !self.agents.contains_key(voter_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: voter_agent_id.clone(),
                    });
                }
                let Some(proposal) = self.governance_proposals.get(proposal_key).cloned() else {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "governance vote references unknown proposal: {proposal_key}"
                        ),
                    });
                };
                if proposal.status != GovernanceProposalStatus::Open {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!("governance proposal is not open: {proposal_key}"),
                    });
                }
                if now > proposal.closes_at {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "governance proposal already closed at {}: {}",
                            proposal.closes_at, proposal_key
                        ),
                    });
                }
                let effective_weight = self.governance_effective_vote_weight_for_agent(
                    &proposal,
                    voter_agent_id,
                    *weight,
                )?;

                let state = self
                    .governance_votes
                    .entry(proposal_key.clone())
                    .or_insert_with(|| GovernanceVoteState {
                        proposal_key: proposal_key.clone(),
                        votes_by_agent: BTreeMap::new(),
                        tallies: BTreeMap::new(),
                        total_weight: 0,
                        last_updated_at: now,
                    });

                if let Some(previous_ballot) = state.votes_by_agent.get(voter_agent_id).cloned() {
                    let previous_weight = u64::from(previous_ballot.weight);
                    state.total_weight = state.total_weight.saturating_sub(previous_weight);
                    if let Some(entry) = state.tallies.get_mut(&previous_ballot.option) {
                        *entry = entry.saturating_sub(previous_weight);
                        if *entry == 0 {
                            state.tallies.remove(&previous_ballot.option);
                        }
                    }
                }

                state.votes_by_agent.insert(
                    voter_agent_id.clone(),
                    GovernanceVoteBallotState {
                        option: option.clone(),
                        weight: effective_weight,
                        voted_at: now,
                    },
                );
                let vote_weight = u64::from(effective_weight);
                let current_tally = state.tallies.get(option).copied().unwrap_or(0);
                *state.tallies.entry(option.clone()).or_insert(0) =
                    current_tally.saturating_add(vote_weight);
                state.total_weight = state.total_weight.saturating_add(vote_weight);
                state.last_updated_at = now;

                if let Some(cell) = self.agents.get_mut(voter_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::GovernanceProposalFinalized {
                proposal_key,
                winning_option,
                winning_weight,
                total_weight,
                passed,
            } => {
                let Some(state) = self.governance_proposals.get_mut(proposal_key) else {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!("governance proposal missing: {proposal_key}"),
                    });
                };
                state.status = if *passed {
                    GovernanceProposalStatus::Passed
                } else {
                    GovernanceProposalStatus::Rejected
                };
                state.finalized_at = Some(now);
                state.winning_option = winning_option.clone();
                state.winning_weight = *winning_weight;
                state.total_weight_at_finalize = *total_weight;
                if let Some(vote_state) = self.governance_votes.get_mut(proposal_key) {
                    vote_state.last_updated_at = now;
                }
            }
            DomainEvent::CrisisSpawned {
                crisis_id,
                kind,
                severity,
                expires_at,
            } => {
                self.crises.insert(
                    crisis_id.clone(),
                    CrisisState {
                        crisis_id: crisis_id.clone(),
                        kind: kind.clone(),
                        severity: *severity,
                        status: CrisisStatus::Active,
                        opened_at: now,
                        expires_at: *expires_at,
                        resolver_agent_id: None,
                        strategy: None,
                        success: None,
                        impact: 0,
                        resolved_at: None,
                    },
                );
            }
            DomainEvent::CrisisResolved {
                resolver_agent_id,
                crisis_id,
                strategy,
                success,
                impact,
            } => {
                let entry = self
                    .crises
                    .entry(crisis_id.clone())
                    .or_insert_with(|| CrisisState {
                        crisis_id: crisis_id.clone(),
                        kind: "legacy".to_string(),
                        severity: 1,
                        status: CrisisStatus::Resolved,
                        opened_at: now,
                        expires_at: now,
                        resolver_agent_id: None,
                        strategy: None,
                        success: None,
                        impact: 0,
                        resolved_at: None,
                    });
                entry.status = CrisisStatus::Resolved;
                entry.resolver_agent_id = Some(resolver_agent_id.clone());
                entry.strategy = Some(strategy.clone());
                entry.success = Some(*success);
                entry.impact = *impact;
                entry.resolved_at = Some(now);
                if let Some(cell) = self.agents.get_mut(resolver_agent_id) {
                    cell.last_active = now;
                } else {
                    return Err(WorldError::AgentNotFound {
                        agent_id: resolver_agent_id.clone(),
                    });
                }
            }
            DomainEvent::CrisisTimedOut {
                crisis_id,
                penalty_impact,
            } => {
                let Some(entry) = self.crises.get_mut(crisis_id) else {
                    return Err(WorldError::ResourceBalanceInvalid {
                        reason: format!("crisis not found for timeout: {crisis_id}"),
                    });
                };
                entry.status = CrisisStatus::TimedOut;
                entry.success = Some(false);
                entry.impact = *penalty_impact;
                entry.resolved_at = Some(now);
            }
            DomainEvent::MetaProgressGranted {
                operator_agent_id,
                target_agent_id,
                track,
                points,
                achievement_id,
            } => {
                if !self.agents.contains_key(operator_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: operator_agent_id.clone(),
                    });
                }
                if !self.agents.contains_key(target_agent_id) {
                    return Err(WorldError::AgentNotFound {
                        agent_id: target_agent_id.clone(),
                    });
                }

                let progress = self
                    .meta_progress
                    .entry(target_agent_id.clone())
                    .or_insert_with(|| MetaProgressState {
                        agent_id: target_agent_id.clone(),
                        track_points: BTreeMap::new(),
                        total_points: 0,
                        achievements: Vec::new(),
                        unlocked_tiers: BTreeMap::new(),
                        last_granted_at: now,
                    });
                let next_track_points = progress
                    .track_points
                    .get(track)
                    .copied()
                    .unwrap_or(0)
                    .saturating_add(*points);
                progress
                    .track_points
                    .insert(track.clone(), next_track_points);
                progress.total_points = progress.total_points.saturating_add(*points);
                progress.last_granted_at = now;
                if let Some(achievement_id) = achievement_id {
                    if !progress
                        .achievements
                        .iter()
                        .any(|item| item == achievement_id)
                    {
                        progress.achievements.push(achievement_id.clone());
                        progress.achievements.sort();
                    }
                }
                unlock_meta_track_tiers(track, next_track_points, progress);

                if let Some(cell) = self.agents.get_mut(operator_agent_id) {
                    cell.last_active = now;
                }
                if let Some(cell) = self.agents.get_mut(target_agent_id) {
                    cell.last_active = now;
                }
            }
            DomainEvent::ProductValidated {
                requester_agent_id,
                stack,
                tradable,
                ..
            } => {
                self.latest_product_validation = Some(LastProductValidationState {
                    product_id: stack.kind.clone(),
                    tradable: *tradable,
                });
                if let Some(cell) = self.agents.get_mut(requester_agent_id) {
                    cell.last_active = now;
                }
            }
            _ => unreachable!(
                "apply_domain_event_governance_meta received unsupported event variant"
            ),
        }
        Ok(())
    }
}
