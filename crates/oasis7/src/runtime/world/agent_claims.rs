use super::super::agent_claims::{
    agent_claim_quote, split_agent_claim_bond_refund, split_agent_claim_spend, AgentClaimCostQuote,
};
use super::super::{AgentClaimState, DomainEvent, WorldError, WorldEvent, WorldEventBody};
use super::World;

impl World {
    pub fn agent_claim(&self, target_agent_id: &str) -> Option<&AgentClaimState> {
        self.state.agent_claims.get(target_agent_id)
    }

    pub fn claimed_agent_count(&self, claimer_agent_id: &str) -> usize {
        self.state
            .agent_claims
            .values()
            .filter(|claim| claim.claim_owner_id == claimer_agent_id)
            .count()
    }

    pub(super) fn agent_claim_quote_for_owner(
        &self,
        claimer_agent_id: &str,
    ) -> Result<AgentClaimCostQuote, WorldError> {
        let owned_claim_count = self.claimed_agent_count(claimer_agent_id);
        let reputation_score = self
            .state
            .reputation_scores
            .get(claimer_agent_id)
            .copied()
            .unwrap_or(0);
        agent_claim_quote(reputation_score, owned_claim_count).map_err(|reason| {
            WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "agent claim quote unavailable: claimer={} reason={reason}",
                    claimer_agent_id
                ),
            }
        })
    }

    pub(super) fn process_agent_claim_epochs(&mut self) -> Result<Vec<WorldEvent>, WorldError> {
        let current_epoch = self.current_agent_claim_epoch();
        if current_epoch <= self.state.agent_claim_last_processed_epoch {
            return Ok(Vec::new());
        }

        let mut emitted = Vec::new();
        let mut target_agent_ids = self.state.agent_claims.keys().cloned().collect::<Vec<_>>();
        target_agent_ids.sort();

        for target_agent_id in target_agent_ids {
            let Some(claim) = self.state.agent_claims.get(&target_agent_id).cloned() else {
                continue;
            };
            self.process_agent_claim_epoch(current_epoch, &claim, &mut emitted)?;
        }

        self.state.agent_claim_last_processed_epoch = current_epoch;
        Ok(emitted)
    }

    fn process_agent_claim_epoch(
        &mut self,
        current_epoch: u64,
        claim: &AgentClaimState,
        emitted: &mut Vec<WorldEvent>,
    ) -> Result<(), WorldError> {
        let Some(latest_claim) = self.state.agent_claims.get(&claim.target_agent_id).cloned()
        else {
            return Ok(());
        };

        let Some((charged_epochs, amount_due)) = claim_amount_due(&latest_claim, current_epoch)?
        else {
            return self.process_agent_claim_release_or_idle(current_epoch, &latest_claim, emitted);
        };

        let funding = split_agent_claim_spend(
            latest_claim.slot_index,
            self.main_token_liquid_balance(latest_claim.claim_owner_id.as_str()),
            self.main_token_restricted_starter_claim_balance(latest_claim.claim_owner_id.as_str()),
            amount_due,
        );
        if let Ok(funding) = funding {
            self.append_agent_claim_event(
                DomainEvent::AgentClaimUpkeepSettled {
                    claimer_agent_id: latest_claim.claim_owner_id.clone(),
                    target_agent_id: latest_claim.target_agent_id.clone(),
                    settled_at_epoch: current_epoch,
                    charged_epochs,
                    amount: amount_due,
                    restricted_spent_amount: funding.restricted_amount,
                    liquid_spent_amount: funding.liquid_amount,
                    upkeep_paid_through_epoch: current_epoch,
                },
                emitted,
            )?;
            let Some(refreshed_claim) = self
                .state
                .agent_claims
                .get(&latest_claim.target_agent_id)
                .cloned()
            else {
                return Ok(());
            };
            return self.process_agent_claim_release_or_idle(
                current_epoch,
                &refreshed_claim,
                emitted,
            );
        }

        if let Some(grace_deadline_epoch) = latest_claim.grace_deadline_epoch {
            if current_epoch > grace_deadline_epoch {
                let (collected_upkeep_amount, penalty_amount, refunded_bond_amount) =
                    reclaim_bond_settlement(
                        latest_claim.locked_bond_amount,
                        amount_due,
                        latest_claim.forced_reclaim_penalty_bps,
                    );
                let refund_sink =
                    self.restricted_starter_claim_refund_sink(latest_claim.claim_owner_id.as_str());
                let refunded_bond_split = split_agent_claim_bond_refund(
                    latest_claim.claim_bond_locked_restricted_amount,
                    latest_claim.claim_bond_locked_liquid_amount,
                    collected_upkeep_amount.saturating_add(penalty_amount),
                )
                .map_err(|reason| WorldError::ResourceBalanceInvalid { reason })?;
                self.append_agent_claim_event(
                    DomainEvent::AgentClaimReclaimed {
                        claimer_agent_id: latest_claim.claim_owner_id.clone(),
                        target_agent_id: latest_claim.target_agent_id.clone(),
                        reclaimed_at_epoch: current_epoch,
                        reason: "upkeep_delinquent".to_string(),
                        upkeep_arrears_amount: amount_due,
                        collected_upkeep_amount,
                        penalty_amount,
                        refunded_bond_amount,
                        refunded_bond_restricted_amount: refunded_bond_split.restricted_amount,
                        refunded_bond_liquid_amount: refunded_bond_split.liquid_amount,
                        refunded_bond_restricted_sink: refund_sink.sink,
                        refunded_bond_restricted_sink_bucket_id: refund_sink
                            .treasury_bucket_id
                            .unwrap_or_default(),
                    },
                    emitted,
                )?;
            }
            return Ok(());
        }

        let grace_deadline_epoch =
            current_epoch.saturating_add(latest_claim.grace_epochs.saturating_sub(1));
        self.append_agent_claim_event(
            DomainEvent::AgentClaimEnteredGrace {
                claimer_agent_id: latest_claim.claim_owner_id.clone(),
                target_agent_id: latest_claim.target_agent_id.clone(),
                delinquent_since_epoch: latest_claim.upkeep_paid_through_epoch.saturating_add(1),
                grace_deadline_epoch,
                upkeep_arrears_amount: amount_due,
            },
            emitted,
        )?;
        Ok(())
    }

    fn process_agent_claim_release_or_idle(
        &mut self,
        current_epoch: u64,
        claim: &AgentClaimState,
        emitted: &mut Vec<WorldEvent>,
    ) -> Result<(), WorldError> {
        if let Some(ready_at_epoch) = claim.release_ready_at_epoch {
            if current_epoch >= ready_at_epoch {
                let refund_sink =
                    self.restricted_starter_claim_refund_sink(claim.claim_owner_id.as_str());
                let refunded_bond_split = split_agent_claim_bond_refund(
                    claim.claim_bond_locked_restricted_amount,
                    claim.claim_bond_locked_liquid_amount,
                    0,
                )
                .map_err(|reason| WorldError::ResourceBalanceInvalid { reason })?;
                self.append_agent_claim_event(
                    DomainEvent::AgentClaimReleased {
                        claimer_agent_id: claim.claim_owner_id.clone(),
                        target_agent_id: claim.target_agent_id.clone(),
                        released_at_epoch: current_epoch,
                        refunded_bond_amount: claim.locked_bond_amount,
                        refunded_bond_restricted_amount: refunded_bond_split.restricted_amount,
                        refunded_bond_liquid_amount: refunded_bond_split.liquid_amount,
                        refunded_bond_restricted_sink: refund_sink.sink,
                        refunded_bond_restricted_sink_bucket_id: refund_sink
                            .treasury_bucket_id
                            .unwrap_or_default(),
                    },
                    emitted,
                )?;
                return Ok(());
            }
        }

        let last_control_epoch = self.agent_last_control_epoch(claim.target_agent_id.as_str());
        let idle_epochs = current_epoch.saturating_sub(last_control_epoch);
        if idle_epochs >= claim.forced_idle_reclaim_epochs {
            let (_, penalty_amount, refunded_bond_amount) = reclaim_bond_settlement(
                claim.locked_bond_amount,
                0,
                claim.forced_reclaim_penalty_bps,
            );
            let refund_sink =
                self.restricted_starter_claim_refund_sink(claim.claim_owner_id.as_str());
            let refunded_bond_split = split_agent_claim_bond_refund(
                claim.claim_bond_locked_restricted_amount,
                claim.claim_bond_locked_liquid_amount,
                penalty_amount,
            )
            .map_err(|reason| WorldError::ResourceBalanceInvalid { reason })?;
            self.append_agent_claim_event(
                DomainEvent::AgentClaimReclaimed {
                    claimer_agent_id: claim.claim_owner_id.clone(),
                    target_agent_id: claim.target_agent_id.clone(),
                    reclaimed_at_epoch: current_epoch,
                    reason: "idle_timeout".to_string(),
                    upkeep_arrears_amount: 0,
                    collected_upkeep_amount: 0,
                    penalty_amount,
                    refunded_bond_amount,
                    refunded_bond_restricted_amount: refunded_bond_split.restricted_amount,
                    refunded_bond_liquid_amount: refunded_bond_split.liquid_amount,
                    refunded_bond_restricted_sink: refund_sink.sink,
                    refunded_bond_restricted_sink_bucket_id: refund_sink
                        .treasury_bucket_id
                        .unwrap_or_default(),
                },
                emitted,
            )?;
            return Ok(());
        }

        if idle_epochs >= claim.idle_warning_epochs && claim.idle_warning_emitted_at_epoch.is_none()
        {
            self.append_agent_claim_event(
                DomainEvent::AgentClaimIdleWarning {
                    claimer_agent_id: claim.claim_owner_id.clone(),
                    target_agent_id: claim.target_agent_id.clone(),
                    warning_emitted_at_epoch: current_epoch,
                    forced_reclaim_at_epoch: last_control_epoch
                        .saturating_add(claim.forced_idle_reclaim_epochs),
                },
                emitted,
            )?;
        }
        Ok(())
    }

    fn current_agent_claim_epoch(&self) -> u64 {
        self.agent_claim_epoch_for_tick(self.state.time)
    }

    fn agent_last_control_epoch(&self, target_agent_id: &str) -> u64 {
        self.state
            .agents
            .get(target_agent_id)
            .map(|cell| self.agent_claim_epoch_for_tick(cell.last_active))
            .unwrap_or_else(|| self.current_agent_claim_epoch())
    }

    fn agent_claim_epoch_for_tick(&self, tick: u64) -> u64 {
        let epoch_length = self.governance_execution_policy().epoch_length_ticks.max(1);
        tick / epoch_length
    }

    fn append_agent_claim_event(
        &mut self,
        event: DomainEvent,
        emitted: &mut Vec<WorldEvent>,
    ) -> Result<(), WorldError> {
        self.append_event(WorldEventBody::Domain(event), None)?;
        if let Some(event) = self.journal.events.last() {
            emitted.push(event.clone());
        }
        Ok(())
    }
}

fn claim_amount_due(
    claim: &AgentClaimState,
    current_epoch: u64,
) -> Result<Option<(u64, u64)>, WorldError> {
    let charged_epochs = current_epoch.saturating_sub(claim.upkeep_paid_through_epoch);
    if charged_epochs == 0 {
        return Ok(None);
    }
    let amount = claim
        .upkeep_per_epoch
        .checked_mul(charged_epochs)
        .ok_or_else(|| WorldError::ResourceBalanceInvalid {
            reason: format!(
                "agent claim upkeep overflow: target={} upkeep_per_epoch={} charged_epochs={}",
                claim.target_agent_id, claim.upkeep_per_epoch, charged_epochs
            ),
        })?;
    Ok(Some((charged_epochs, amount)))
}

fn reclaim_bond_settlement(
    locked_bond_amount: u64,
    upkeep_arrears_amount: u64,
    penalty_bps: u16,
) -> (u64, u64, u64) {
    let collected_upkeep_amount = locked_bond_amount.min(upkeep_arrears_amount);
    let remaining_after_upkeep = locked_bond_amount.saturating_sub(collected_upkeep_amount);
    let penalty_amount = remaining_after_upkeep.saturating_mul(u64::from(penalty_bps)) / 10_000;
    let refunded_bond_amount = remaining_after_upkeep.saturating_sub(penalty_amount);
    (
        collected_upkeep_amount,
        penalty_amount,
        refunded_bond_amount,
    )
}
