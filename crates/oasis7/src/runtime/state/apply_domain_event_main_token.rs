use super::super::events::MainTokenFeeKind;
use super::super::main_token::{
    FirstAgentClaimApprovalRequestState, FirstAgentClaimApprovalRequestStatus,
    RestrictedStarterClaimGrantState, RestrictedStarterClaimGrantStatus,
    RestrictedStarterClaimLiveopsPoolTopUpRecord, MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL,
    MAIN_TOKEN_TREASURY_BUCKET_GAS_FEE, MAIN_TOKEN_TREASURY_BUCKET_MODULE_FEE,
    MAIN_TOKEN_TREASURY_BUCKET_NODE_SERVICE_REWARD,
    MAIN_TOKEN_TREASURY_BUCKET_RESTRICTED_STARTER_CLAIM_LIVEOPS_POOL,
    MAIN_TOKEN_TREASURY_BUCKET_SECURITY_RESERVE, MAIN_TOKEN_TREASURY_BUCKET_SLASH,
    MAIN_TOKEN_TREASURY_BUCKET_STAKING_REWARD,
};
use super::*;

#[path = "apply_domain_event_main_token_economy.rs"]
mod economy;
#[path = "apply_domain_event_main_token_genesis.rs"]
mod genesis;
#[path = "apply_domain_event_main_token_helpers.rs"]
mod helpers;
#[path = "apply_domain_event_main_token_restricted_claims.rs"]
mod restricted_claims;

impl WorldState {
    pub(super) fn apply_domain_event_main_token(
        &mut self,
        event: &DomainEvent,
        now: WorldTime,
    ) -> Result<(), WorldError> {
        match event {
            DomainEvent::MainTokenGenesisInitialized {
                total_supply,
                allocations,
            } => self.apply_main_token_genesis_initialized(*total_supply, allocations)?,
            DomainEvent::MainTokenVestingClaimed {
                bucket_id,
                beneficiary,
                amount,
                nonce,
            } => {
                self.apply_main_token_vesting_claimed(bucket_id, beneficiary, *amount, *nonce, now)?
            }
            DomainEvent::MainTokenTransferred {
                from_account_id,
                to_account_id,
                amount,
                nonce,
            } => self.apply_main_token_transfer(from_account_id, to_account_id, *amount, *nonce)?,
            DomainEvent::MainTokenEpochIssued {
                epoch_index,
                inflation_rate_bps,
                issued_amount,
                staking_reward_amount,
                node_service_reward_amount,
                ecosystem_pool_amount,
                security_reserve_amount,
            } => self.apply_main_token_epoch_issued(
                *epoch_index,
                *inflation_rate_bps,
                *issued_amount,
                *staking_reward_amount,
                *node_service_reward_amount,
                *ecosystem_pool_amount,
                *security_reserve_amount,
            )?,
            DomainEvent::MainTokenFeeSettled {
                fee_kind,
                amount,
                burn_amount,
                treasury_amount,
            } => self.apply_main_token_fee_settled(
                *fee_kind,
                *amount,
                *burn_amount,
                *treasury_amount,
            )?,
            DomainEvent::MainTokenPolicyUpdateScheduled {
                proposal_id,
                effective_epoch,
                next,
            } => self.apply_main_token_policy_update_scheduled(
                *proposal_id,
                *effective_epoch,
                next,
                now,
            )?,
            DomainEvent::MainTokenTreasuryDistributed {
                proposal_id,
                distribution_id,
                bucket_id,
                total_amount,
                distributions,
            } => self.apply_main_token_treasury_distributed(
                *proposal_id,
                distribution_id,
                bucket_id,
                *total_amount,
                distributions,
                now,
            )?,
            DomainEvent::RestrictedStarterClaimLiveopsPoolToppedUp {
                controller_account_id,
                top_up_id,
                source_treasury_bucket_id,
                target_treasury_bucket_id,
                amount,
                topped_up_at_epoch,
            } => self.apply_restricted_starter_claim_liveops_pool_top_up(
                controller_account_id,
                top_up_id,
                source_treasury_bucket_id,
                target_treasury_bucket_id,
                *amount,
                *topped_up_at_epoch,
            )?,
            DomainEvent::RestrictedStarterClaimGrantIssued {
                issuer_id,
                beneficiary_account_id,
                source_treasury_bucket_id,
                amount,
                issuance_reason,
                spend_scope,
                issued_at_epoch,
                expires_at_epoch,
            } => self.apply_restricted_starter_claim_grant_issued(
                issuer_id,
                beneficiary_account_id,
                source_treasury_bucket_id,
                *amount,
                issuance_reason,
                spend_scope,
                *issued_at_epoch,
                *expires_at_epoch,
            )?,
            DomainEvent::RestrictedStarterClaimGrantExpired {
                beneficiary_account_id,
                issuer_id,
                issuance_reason,
                spend_scope,
                source_treasury_bucket_id,
                issued_amount,
                expired_amount,
                issued_at_epoch,
                expired_at_epoch,
                configured_expires_at_epoch,
            } => self.apply_restricted_starter_claim_grant_expired(
                beneficiary_account_id,
                issuer_id,
                issuance_reason,
                spend_scope,
                source_treasury_bucket_id,
                *issued_amount,
                *expired_amount,
                *issued_at_epoch,
                *expired_at_epoch,
                *configured_expires_at_epoch,
            )?,
            DomainEvent::RestrictedStarterClaimGrantRevoked {
                beneficiary_account_id,
                issuer_id,
                issuance_reason,
                spend_scope,
                source_treasury_bucket_id,
                issued_amount,
                revoked_amount,
                issued_at_epoch,
                revoked_at_epoch,
                configured_expires_at_epoch,
                revoke_reason,
            } => self.apply_restricted_starter_claim_grant_revoked(
                beneficiary_account_id,
                issuer_id,
                issuance_reason,
                spend_scope,
                source_treasury_bucket_id,
                *issued_amount,
                *revoked_amount,
                *issued_at_epoch,
                *revoked_at_epoch,
                *configured_expires_at_epoch,
                revoke_reason,
            )?,
            DomainEvent::FirstAgentClaimApprovalRequested {
                request_id,
                claimer_agent_id,
                requested_slot_index,
                requested_reputation_tier,
                requested_total_upfront_amount,
                requested_at_epoch,
            } => self.apply_first_agent_claim_approval_requested(
                *request_id,
                claimer_agent_id,
                *requested_slot_index,
                *requested_reputation_tier,
                *requested_total_upfront_amount,
                *requested_at_epoch,
            )?,
            DomainEvent::FirstAgentClaimApprovalApproved {
                request_id,
                operator_account_id,
                claimer_agent_id,
                approved_amount,
                issuance_reason,
                spend_scope,
                source_treasury_bucket_id,
                approved_at_epoch,
                expires_at_epoch,
            } => self.apply_first_agent_claim_approval_approved(
                *request_id,
                operator_account_id,
                claimer_agent_id,
                *approved_amount,
                issuance_reason,
                spend_scope,
                source_treasury_bucket_id,
                *approved_at_epoch,
                *expires_at_epoch,
            )?,
            DomainEvent::FirstAgentClaimApprovalRejected {
                request_id,
                operator_account_id,
                claimer_agent_id,
                rejected_at_epoch,
                reason,
            } => self.apply_first_agent_claim_approval_rejected(
                *request_id,
                operator_account_id,
                claimer_agent_id,
                *rejected_at_epoch,
                reason,
            )?,
            _ => unreachable!("apply_domain_event_main_token received unsupported event variant"),
        }
        Ok(())
    }
}
