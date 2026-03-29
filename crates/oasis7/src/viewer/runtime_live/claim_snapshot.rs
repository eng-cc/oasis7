use crate::runtime::{
    agent_claim_cap_for_tier, agent_claim_quote, agent_claim_reputation_tier, AgentClaimState,
    WorldState,
};
use crate::simulator::persist::{
    PlayerAgentClaimOwnedSnapshot, PlayerAgentClaimQuoteSnapshot, PlayerAgentClaimSnapshot,
};

pub(super) fn build_player_agent_claim_snapshot(
    state: &WorldState,
    primary_agent_id: &str,
    epoch_length_ticks: u64,
) -> Option<PlayerAgentClaimSnapshot> {
    if !state.agents.contains_key(primary_agent_id) {
        return None;
    }

    let current_epoch = agent_claim_epoch(state.time, epoch_length_ticks);
    let owned_claims_count = state
        .agent_claims
        .values()
        .filter(|claim| claim.claim_owner_id == primary_agent_id)
        .count();
    let reputation_score = state
        .reputation_scores
        .get(primary_agent_id)
        .copied()
        .unwrap_or(0);
    let liquid_main_token_balance = state
        .main_token_balances
        .get(primary_agent_id)
        .map(|balance| balance.liquid_balance)
        .unwrap_or(0);
    let restricted_starter_claim_balance = state
        .main_token_balances
        .get(primary_agent_id)
        .map(|balance| balance.restricted_starter_claim_balance)
        .unwrap_or(0);

    let next_claim_quote = match agent_claim_quote(reputation_score, owned_claims_count) {
        Ok(quote) => {
            let total_upfront_amount = quote
                .activation_fee_amount
                .saturating_add(quote.claim_bond_amount)
                .saturating_add(quote.upkeep_per_epoch);
            let eligible_claim_balance = if quote.slot_index == 1 {
                liquid_main_token_balance.saturating_add(restricted_starter_claim_balance)
            } else {
                liquid_main_token_balance
            };
            Some(PlayerAgentClaimQuoteSnapshot {
                slot_index: quote.slot_index,
                reputation_tier: quote.reputation_tier,
                claim_cap: quote.claim_cap,
                owned_claim_count: u8::try_from(owned_claims_count).unwrap_or(u8::MAX),
                activation_fee_amount: quote.activation_fee_amount,
                claim_bond_amount: quote.claim_bond_amount,
                upkeep_per_epoch: quote.upkeep_per_epoch,
                total_upfront_amount,
                transferable_liquid_balance: liquid_main_token_balance,
                restricted_starter_claim_balance,
                eligible_claim_balance,
                release_cooldown_epochs: quote.release_cooldown_epochs,
                grace_epochs: quote.grace_epochs,
                idle_warning_epochs: quote.idle_warning_epochs,
                forced_idle_reclaim_epochs: quote.forced_idle_reclaim_epochs,
                forced_reclaim_penalty_bps: quote.forced_reclaim_penalty_bps,
                blocked_reason: if quote.slot_index > 1
                    && liquid_main_token_balance < total_upfront_amount
                    && restricted_starter_claim_balance > 0
                {
                    Some(format!(
                        "restricted_balance_not_eligible_for_slot slot={} liquid={} restricted={} required={}",
                        quote.slot_index,
                        liquid_main_token_balance,
                        restricted_starter_claim_balance,
                        total_upfront_amount
                    ))
                } else if eligible_claim_balance < total_upfront_amount {
                    Some(format!(
                        "insufficient_claim_eligible_main_token eligible={} liquid={} restricted={} required={}",
                        eligible_claim_balance,
                        liquid_main_token_balance,
                        restricted_starter_claim_balance,
                        total_upfront_amount
                    ))
                } else {
                    None
                },
            })
        }
        Err(reason) => Some(PlayerAgentClaimQuoteSnapshot {
            slot_index: 0,
            reputation_tier: agent_claim_reputation_tier(reputation_score),
            claim_cap: agent_claim_cap_for_tier(agent_claim_reputation_tier(reputation_score)),
            owned_claim_count: u8::try_from(owned_claims_count).unwrap_or(u8::MAX),
            activation_fee_amount: 0,
            claim_bond_amount: 0,
            upkeep_per_epoch: 0,
            total_upfront_amount: 0,
            transferable_liquid_balance: liquid_main_token_balance,
            restricted_starter_claim_balance,
            eligible_claim_balance: liquid_main_token_balance
                .saturating_add(restricted_starter_claim_balance),
            release_cooldown_epochs: 0,
            grace_epochs: 0,
            idle_warning_epochs: 0,
            forced_idle_reclaim_epochs: 0,
            forced_reclaim_penalty_bps: 0,
            blocked_reason: Some(reason),
        }),
    };

    let mut owned_claims = state
        .agent_claims
        .values()
        .filter(|claim| claim.claim_owner_id == primary_agent_id)
        .cloned()
        .collect::<Vec<_>>();
    owned_claims.sort_by(|left, right| left.target_agent_id.cmp(&right.target_agent_id));

    Some(PlayerAgentClaimSnapshot {
        claimer_agent_id: primary_agent_id.to_string(),
        current_epoch,
        reputation_tier: agent_claim_reputation_tier(reputation_score),
        claim_cap: agent_claim_cap_for_tier(agent_claim_reputation_tier(reputation_score)),
        owned_claim_count: u8::try_from(owned_claims_count).unwrap_or(u8::MAX),
        liquid_main_token_balance,
        restricted_starter_claim_balance,
        slot_1_eligible_claim_balance: liquid_main_token_balance
            .saturating_add(restricted_starter_claim_balance),
        next_claim_quote,
        owned_claims: owned_claims
            .iter()
            .map(|claim| owned_claim_to_snapshot(state, claim, current_epoch, epoch_length_ticks))
            .collect(),
    })
}

fn owned_claim_to_snapshot(
    state: &WorldState,
    claim: &AgentClaimState,
    current_epoch: u64,
    epoch_length_ticks: u64,
) -> PlayerAgentClaimOwnedSnapshot {
    let status = claim_status(claim, current_epoch);
    let last_control_epoch = state
        .agents
        .get(&claim.target_agent_id)
        .map(|cell| agent_claim_epoch(cell.last_active, epoch_length_ticks))
        .unwrap_or(current_epoch);
    let release_ready_in_epochs = claim
        .release_ready_at_epoch
        .map(|epoch| epoch.saturating_sub(current_epoch));
    let grace_remaining_epochs = claim
        .grace_deadline_epoch
        .map(|epoch| epoch.saturating_sub(current_epoch));
    let idle_warning_in_epochs = (claim.idle_warning_emitted_at_epoch.is_none()).then(|| {
        last_control_epoch
            .saturating_add(claim.idle_warning_epochs)
            .saturating_sub(current_epoch)
    });
    let forced_reclaim_in_epochs = Some(
        last_control_epoch
            .saturating_add(claim.forced_idle_reclaim_epochs)
            .saturating_sub(current_epoch),
    );

    PlayerAgentClaimOwnedSnapshot {
        target_agent_id: claim.target_agent_id.clone(),
        status: status.to_string(),
        upkeep_paid_through_epoch: claim.upkeep_paid_through_epoch,
        upfront_restricted_spent_amount: claim.upfront_restricted_spent_amount,
        upfront_liquid_spent_amount: claim.upfront_liquid_spent_amount,
        claim_bond_locked_restricted_amount: claim.claim_bond_locked_restricted_amount,
        claim_bond_locked_liquid_amount: claim.claim_bond_locked_liquid_amount,
        release_ready_at_epoch: claim.release_ready_at_epoch,
        release_ready_in_epochs,
        grace_deadline_epoch: claim.grace_deadline_epoch,
        grace_remaining_epochs,
        idle_warning_in_epochs,
        forced_reclaim_in_epochs,
    }
}

fn claim_status(claim: &AgentClaimState, current_epoch: u64) -> &'static str {
    if claim.grace_deadline_epoch.is_some() {
        "upkeep_grace"
    } else if let Some(ready_epoch) = claim.release_ready_at_epoch {
        if current_epoch >= ready_epoch {
            "release_ready"
        } else {
            "release_cooldown"
        }
    } else if claim.idle_warning_emitted_at_epoch.is_some() {
        "idle_reclaim_candidate"
    } else {
        "claimed_active"
    }
}

fn agent_claim_epoch(time: u64, epoch_length_ticks: u64) -> u64 {
    time / epoch_length_ticks.max(1)
}
