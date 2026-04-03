use super::super::*;
use super::pos;
use crate::models::AgentState;
use crate::simulator::ResourceKind;

fn test_agent_cell(agent_id: &str) -> AgentCell {
    AgentCell::new(AgentState::new(agent_id, pos(0.0, 0.0)), 0)
}

fn sample_contract(creator_agent_id: &str, counterparty_agent_id: &str) -> EconomicContractState {
    EconomicContractState {
        contract_id: "contract.guard".to_string(),
        creator_agent_id: creator_agent_id.to_string(),
        counterparty_agent_id: counterparty_agent_id.to_string(),
        settlement_kind: ResourceKind::Data,
        settlement_amount: 10,
        reputation_stake: 1,
        expires_at: 50,
        description: "guard contract".to_string(),
        status: EconomicContractStatus::Accepted,
        accepted_at: Some(5),
        settled_at: None,
        settlement_success: None,
        transfer_amount: 0,
        tax_amount: 0,
        settlement_notes: None,
    }
}

fn sample_claim(target_agent_id: &str, claimer_agent_id: &str) -> AgentClaimState {
    AgentClaimState {
        target_agent_id: target_agent_id.to_string(),
        claim_owner_id: claimer_agent_id.to_string(),
        reputation_tier: 0,
        slot_index: 1,
        activation_fee_amount: 10,
        activation_fee_burn_amount: 1,
        activation_fee_treasury_amount: 4,
        claim_bond_amount: 6,
        locked_bond_amount: 6,
        upfront_restricted_spent_amount: 0,
        upfront_liquid_spent_amount: 16,
        claim_bond_locked_restricted_amount: 0,
        claim_bond_locked_liquid_amount: 6,
        upkeep_per_epoch: 2,
        release_cooldown_epochs: 1,
        grace_epochs: 1,
        idle_warning_epochs: 1,
        forced_idle_reclaim_epochs: 2,
        forced_reclaim_penalty_bps: 500,
        claimed_at_epoch: 3,
        upkeep_paid_through_epoch: 3,
        delinquent_since_epoch: None,
        grace_deadline_epoch: None,
        release_requested_at_epoch: None,
        release_ready_at_epoch: None,
        idle_warning_emitted_at_epoch: None,
    }
}

#[test]
fn economic_contract_settlement_missing_operator_returns_agent_not_found() {
    let mut state = WorldState::default();
    state
        .agents
        .insert("creator".to_string(), test_agent_cell("creator"));
    state
        .agents
        .insert("counterparty".to_string(), test_agent_cell("counterparty"));
    state.economic_contracts.insert(
        "contract.guard".to_string(),
        sample_contract("creator", "counterparty"),
    );

    let err = state
        .apply_domain_event(
            &DomainEvent::EconomicContractSettled {
                operator_agent_id: "missing".to_string(),
                contract_id: "contract.guard".to_string(),
                success: false,
                transfer_amount: 0,
                tax_amount: 0,
                notes: "missing operator".to_string(),
                creator_reputation_delta: 0,
                counterparty_reputation_delta: 0,
            },
            8,
        )
        .expect_err("missing operator must be rejected");

    assert_eq!(
        err,
        WorldError::AgentNotFound {
            agent_id: "missing".to_string(),
        }
    );
    assert_eq!(
        state
            .economic_contracts
            .get("contract.guard")
            .expect("contract kept")
            .status,
        EconomicContractStatus::Accepted
    );
}

#[test]
fn claim_release_request_missing_claimer_returns_error_without_mutating_claim() {
    let mut state = WorldState::default();
    state
        .agents
        .insert("target".to_string(), test_agent_cell("target"));
    state
        .agent_claims
        .insert("target".to_string(), sample_claim("target", "claimer"));

    let err = state
        .apply_domain_event(
            &DomainEvent::AgentClaimReleaseRequested {
                claimer_agent_id: "claimer".to_string(),
                target_agent_id: "target".to_string(),
                requested_at_epoch: 4,
                ready_at_epoch: 5,
            },
            9,
        )
        .expect_err("missing claimer must be rejected");

    assert_eq!(
        err,
        WorldError::AgentNotFound {
            agent_id: "claimer".to_string(),
        }
    );
    let claim = state.agent_claims.get("target").expect("claim retained");
    assert_eq!(claim.release_requested_at_epoch, None);
    assert_eq!(claim.release_ready_at_epoch, None);
}

#[test]
fn claim_reclaim_missing_claimer_returns_error_without_removing_claim() {
    let mut state = WorldState::default();
    state
        .agents
        .insert("target".to_string(), test_agent_cell("target"));
    state
        .agent_claims
        .insert("target".to_string(), sample_claim("target", "claimer"));

    let err = state
        .apply_domain_event(
            &DomainEvent::AgentClaimReclaimed {
                claimer_agent_id: "claimer".to_string(),
                target_agent_id: "target".to_string(),
                reclaimed_at_epoch: 8,
                reason: "missing claimer".to_string(),
                upkeep_arrears_amount: 0,
                collected_upkeep_amount: 0,
                penalty_amount: 0,
                refunded_bond_amount: 6,
                refunded_bond_restricted_amount: 0,
                refunded_bond_liquid_amount: 6,
                refunded_bond_restricted_sink:
                    RestrictedStarterClaimRefundSink::BeneficiaryRestrictedBalance,
                refunded_bond_restricted_sink_bucket_id: String::new(),
            },
            10,
        )
        .expect_err("missing claimer must be rejected");

    assert_eq!(
        err,
        WorldError::AgentNotFound {
            agent_id: "claimer".to_string(),
        }
    );
    assert!(
        state.agent_claims.contains_key("target"),
        "claim should remain present after early error"
    );
}
