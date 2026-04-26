use super::helpers::{
    add_main_token_treasury_balance, debit_main_token_restricted_starter_claim_balance,
    debit_main_token_treasury_balance, restricted_starter_claim_grant_can_be_inserted,
};
use super::*;

impl WorldState {
    pub(super) fn apply_restricted_starter_claim_liveops_pool_top_up(
        &mut self,
        controller_account_id: &str,
        top_up_id: &str,
        source_treasury_bucket_id: &str,
        target_treasury_bucket_id: &str,
        amount: u64,
        topped_up_at_epoch: u64,
    ) -> Result<(), WorldError> {
        let controller_account_id = controller_account_id.trim();
        if controller_account_id.is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason:
                    "restricted claim liveops pool top-up controller_account_id cannot be empty"
                        .to_string(),
            });
        }
        let top_up_id = top_up_id.trim();
        if top_up_id.is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "restricted claim liveops pool top_up_id cannot be empty".to_string(),
            });
        }
        if self
            .restricted_starter_claim_liveops_pool_top_up_records
            .contains_key(top_up_id)
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "restricted claim liveops pool top_up_id already exists: {top_up_id}"
                ),
            });
        }
        let source_treasury_bucket_id = source_treasury_bucket_id.trim();
        if source_treasury_bucket_id != MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "restricted claim liveops pool top-up source bucket must be ecosystem_pool: {}",
                    source_treasury_bucket_id
                ),
            });
        }
        let target_treasury_bucket_id = target_treasury_bucket_id.trim();
        if target_treasury_bucket_id
            != MAIN_TOKEN_TREASURY_BUCKET_RESTRICTED_STARTER_CLAIM_LIVEOPS_POOL
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "restricted claim liveops pool top-up target bucket must be restricted starter claim liveops pool: {}",
                    target_treasury_bucket_id
                ),
            });
        }
        if amount == 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "restricted claim liveops pool top-up amount must be > 0".to_string(),
            });
        }

        debit_main_token_treasury_balance(
            &mut self.main_token_treasury_balances,
            source_treasury_bucket_id,
            amount,
        )?;
        add_main_token_treasury_balance(
            &mut self.main_token_treasury_balances,
            target_treasury_bucket_id,
            amount,
        )?;
        self.restricted_starter_claim_liveops_pool_top_up_records
            .insert(
                top_up_id.to_string(),
                RestrictedStarterClaimLiveopsPoolTopUpRecord {
                    controller_account_id: controller_account_id.to_string(),
                    top_up_id: top_up_id.to_string(),
                    source_treasury_bucket_id: source_treasury_bucket_id.to_string(),
                    target_treasury_bucket_id: target_treasury_bucket_id.to_string(),
                    amount,
                    topped_up_at_epoch,
                },
            );
        Ok(())
    }

    pub(super) fn apply_restricted_starter_claim_grant_issued(
        &mut self,
        issuer_id: &str,
        beneficiary_account_id: &str,
        source_treasury_bucket_id: &str,
        amount: u64,
        issuance_reason: &str,
        spend_scope: &str,
        issued_at_epoch: u64,
        expires_at_epoch: u64,
    ) -> Result<(), WorldError> {
        let issuer_id = issuer_id.trim();
        if issuer_id.is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "restricted grant issuer_id cannot be empty".to_string(),
            });
        }
        let beneficiary_account_id = beneficiary_account_id.trim();
        if beneficiary_account_id.is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "restricted grant beneficiary_account_id cannot be empty".to_string(),
            });
        }
        let source_treasury_bucket_id = source_treasury_bucket_id.trim();
        if source_treasury_bucket_id.is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "restricted grant source_treasury_bucket_id cannot be empty".to_string(),
            });
        }
        let issuance_reason = issuance_reason.trim();
        if issuance_reason.is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "restricted grant issuance_reason cannot be empty".to_string(),
            });
        }
        let spend_scope = spend_scope.trim();
        if spend_scope.is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "restricted grant spend_scope cannot be empty".to_string(),
            });
        }
        if amount == 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "restricted grant amount must be > 0".to_string(),
            });
        }
        if expires_at_epoch <= issued_at_epoch {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "restricted grant expires_at_epoch must be > issued_at_epoch: expires={} issued={}",
                    expires_at_epoch, issued_at_epoch
                ),
            });
        }
        if !restricted_starter_claim_grant_can_be_inserted(self, beneficiary_account_id) {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "restricted grant already active or pending settlement: beneficiary={beneficiary_account_id}"
                ),
            });
        }
        if self
            .main_token_balances
            .get(beneficiary_account_id)
            .map(|balance| balance.restricted_starter_claim_balance)
            .unwrap_or(0)
            > 0
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "restricted grant beneficiary already has restricted balance: beneficiary={beneficiary_account_id}"
                ),
            });
        }

        debit_main_token_treasury_balance(
            &mut self.main_token_treasury_balances,
            source_treasury_bucket_id,
            amount,
        )?;
        let account = self
            .main_token_balances
            .entry(beneficiary_account_id.to_string())
            .or_insert_with(|| MainTokenAccountBalance {
                account_id: beneficiary_account_id.to_string(),
                ..MainTokenAccountBalance::default()
            });
        account.restricted_starter_claim_balance = account
            .restricted_starter_claim_balance
            .checked_add(amount)
            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "restricted grant credit overflow: beneficiary={} current={} amount={}",
                    beneficiary_account_id, account.restricted_starter_claim_balance, amount
                ),
            })?;
        self.main_token_supply.circulating_supply = self
            .main_token_supply
            .circulating_supply
            .checked_add(amount)
            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "restricted grant circulating overflow: current={} amount={}",
                    self.main_token_supply.circulating_supply, amount
                ),
            })?;
        if self.main_token_supply.circulating_supply > self.main_token_supply.total_supply {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "restricted grant circulating exceeds total: circulating={} total={}",
                    self.main_token_supply.circulating_supply, self.main_token_supply.total_supply
                ),
            });
        }
        self.restricted_starter_claim_grants.insert(
            beneficiary_account_id.to_string(),
            RestrictedStarterClaimGrantState {
                beneficiary_account_id: beneficiary_account_id.to_string(),
                issuer_id: issuer_id.to_string(),
                issuance_reason: issuance_reason.to_string(),
                spend_scope: spend_scope.to_string(),
                source_treasury_bucket_id: source_treasury_bucket_id.to_string(),
                issued_amount: amount,
                issued_at_epoch,
                expires_at_epoch,
                status: RestrictedStarterClaimGrantStatus::Issued,
                status_updated_at_epoch: None,
                status_reason: None,
            },
        );
        Ok(())
    }

    pub(super) fn apply_restricted_starter_claim_grant_expired(
        &mut self,
        beneficiary_account_id: &str,
        issuer_id: &str,
        issuance_reason: &str,
        spend_scope: &str,
        source_treasury_bucket_id: &str,
        issued_amount: u64,
        expired_amount: u64,
        issued_at_epoch: u64,
        expired_at_epoch: u64,
        configured_expires_at_epoch: u64,
    ) -> Result<(), WorldError> {
        let grant =
            self.restricted_starter_claim_grants
                .get_mut(beneficiary_account_id)
                .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "restricted grant not found for expiration: beneficiary={beneficiary_account_id}"
                    ),
                })?;
        if grant.status != RestrictedStarterClaimGrantStatus::Issued {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "restricted grant already terminal before expiration: beneficiary={} status={:?}",
                    beneficiary_account_id, grant.status
                ),
            });
        }
        if grant.issuer_id != issuer_id
            || grant.issuance_reason != issuance_reason
            || grant.spend_scope != spend_scope
            || grant.source_treasury_bucket_id != source_treasury_bucket_id
            || grant.issued_amount != issued_amount
            || grant.issued_at_epoch != issued_at_epoch
            || grant.expires_at_epoch != configured_expires_at_epoch
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "restricted grant expiration metadata mismatch: beneficiary={beneficiary_account_id}"
                ),
            });
        }
        if expired_at_epoch < configured_expires_at_epoch {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "restricted grant expired before configured epoch: beneficiary={} configured={} actual={}",
                    beneficiary_account_id, configured_expires_at_epoch, expired_at_epoch
                ),
            });
        }
        debit_main_token_restricted_starter_claim_balance(
            &mut self.main_token_balances,
            beneficiary_account_id,
            expired_amount,
        )?;
        add_main_token_treasury_balance(
            &mut self.main_token_treasury_balances,
            source_treasury_bucket_id,
            expired_amount,
        )?;
        if self.main_token_supply.circulating_supply < expired_amount {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "restricted grant expiration circulating insufficient: circulating={} amount={}",
                    self.main_token_supply.circulating_supply, expired_amount
                ),
            });
        }
        self.main_token_supply.circulating_supply -= expired_amount;
        grant.status = RestrictedStarterClaimGrantStatus::Expired;
        grant.status_updated_at_epoch = Some(expired_at_epoch);
        grant.status_reason = Some("expired".to_string());
        Ok(())
    }

    pub(super) fn apply_restricted_starter_claim_grant_revoked(
        &mut self,
        beneficiary_account_id: &str,
        issuer_id: &str,
        issuance_reason: &str,
        spend_scope: &str,
        source_treasury_bucket_id: &str,
        issued_amount: u64,
        revoked_amount: u64,
        issued_at_epoch: u64,
        revoked_at_epoch: u64,
        configured_expires_at_epoch: u64,
        revoke_reason: &str,
    ) -> Result<(), WorldError> {
        let grant = self
            .restricted_starter_claim_grants
            .get_mut(beneficiary_account_id)
            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "restricted grant not found for revoke: beneficiary={beneficiary_account_id}"
                ),
            })?;
        if grant.status != RestrictedStarterClaimGrantStatus::Issued {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "restricted grant already terminal before revoke: beneficiary={} status={:?}",
                    beneficiary_account_id, grant.status
                ),
            });
        }
        if grant.issuer_id != issuer_id
            || grant.issuance_reason != issuance_reason
            || grant.spend_scope != spend_scope
            || grant.source_treasury_bucket_id != source_treasury_bucket_id
            || grant.issued_amount != issued_amount
            || grant.issued_at_epoch != issued_at_epoch
            || grant.expires_at_epoch != configured_expires_at_epoch
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "restricted grant revoke metadata mismatch: beneficiary={beneficiary_account_id}"
                ),
            });
        }
        if revoke_reason.trim().is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "restricted grant revoke_reason cannot be empty".to_string(),
            });
        }
        debit_main_token_restricted_starter_claim_balance(
            &mut self.main_token_balances,
            beneficiary_account_id,
            revoked_amount,
        )?;
        add_main_token_treasury_balance(
            &mut self.main_token_treasury_balances,
            source_treasury_bucket_id,
            revoked_amount,
        )?;
        if self.main_token_supply.circulating_supply < revoked_amount {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "restricted grant revoke circulating insufficient: circulating={} amount={}",
                    self.main_token_supply.circulating_supply, revoked_amount
                ),
            });
        }
        self.main_token_supply.circulating_supply -= revoked_amount;
        grant.status = RestrictedStarterClaimGrantStatus::Revoked;
        grant.status_updated_at_epoch = Some(revoked_at_epoch);
        grant.status_reason = Some(revoke_reason.to_string());
        Ok(())
    }

    pub(super) fn apply_first_agent_claim_approval_requested(
        &mut self,
        request_id: u64,
        claimer_agent_id: &str,
        requested_slot_index: u8,
        requested_reputation_tier: u8,
        requested_total_upfront_amount: u64,
        requested_at_epoch: u64,
    ) -> Result<(), WorldError> {
        if request_id == 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "first agent claim approval request_id must be > 0".to_string(),
            });
        }
        let claimer_agent_id = claimer_agent_id.trim();
        if claimer_agent_id.is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "first agent claim approval claimer_agent_id cannot be empty".to_string(),
            });
        }
        if requested_slot_index != 1 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "first agent claim approval requested_slot_index must be 1: {}",
                    requested_slot_index
                ),
            });
        }
        if requested_total_upfront_amount == 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "first agent claim approval requested_total_upfront_amount must be > 0"
                    .to_string(),
            });
        }
        if self
            .first_agent_claim_approval_requests
            .contains_key(&request_id)
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "first agent claim approval request already exists: request_id={request_id}"
                ),
            });
        }
        self.first_agent_claim_approval_requests.insert(
            request_id,
            FirstAgentClaimApprovalRequestState {
                request_id,
                claimer_agent_id: claimer_agent_id.to_string(),
                requested_slot_index,
                requested_reputation_tier,
                requested_total_upfront_amount,
                requested_at_epoch,
                status: FirstAgentClaimApprovalRequestStatus::Pending,
                updated_at_epoch: requested_at_epoch,
                operator_account_id: None,
                approved_amount: None,
                expires_at_epoch: None,
                rejection_reason: None,
            },
        );
        let latest_request_id = self
            .latest_first_agent_claim_approval_request_ids_by_claimer
            .entry(claimer_agent_id.to_string())
            .or_insert(request_id);
        *latest_request_id = (*latest_request_id).max(request_id);
        self.next_first_agent_claim_approval_request_id = self
            .next_first_agent_claim_approval_request_id
            .max(request_id.saturating_add(1))
            .max(1);
        Ok(())
    }

    pub(super) fn apply_first_agent_claim_approval_approved(
        &mut self,
        request_id: u64,
        operator_account_id: &str,
        claimer_agent_id: &str,
        approved_amount: u64,
        issuance_reason: &str,
        spend_scope: &str,
        source_treasury_bucket_id: &str,
        approved_at_epoch: u64,
        expires_at_epoch: u64,
    ) -> Result<(), WorldError> {
        let request = self
            .first_agent_claim_approval_requests
            .get(&request_id)
            .cloned()
            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "first agent claim approval request missing for approve: request_id={request_id}"
                ),
            })?;
        if request.status != FirstAgentClaimApprovalRequestStatus::Pending {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "first agent claim approval request not pending for approve: request_id={} status={:?}",
                    request_id, request.status
                ),
            });
        }
        if request.claimer_agent_id != claimer_agent_id {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "first agent claim approval claimer mismatch: request_id={} expected={} actual={}",
                    request_id, request.claimer_agent_id, claimer_agent_id
                ),
            });
        }
        if approved_amount == 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "first agent claim approval approved_amount must be > 0".to_string(),
            });
        }
        self.apply_restricted_starter_claim_grant_issued(
            operator_account_id,
            claimer_agent_id,
            source_treasury_bucket_id,
            approved_amount,
            issuance_reason,
            spend_scope,
            approved_at_epoch,
            expires_at_epoch,
        )?;
        let request = self
            .first_agent_claim_approval_requests
            .get_mut(&request_id)
            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "first agent claim approval request missing after grant issue: request_id={request_id}"
                ),
            })?;
        request.status = FirstAgentClaimApprovalRequestStatus::Approved;
        request.updated_at_epoch = approved_at_epoch;
        request.operator_account_id = Some(operator_account_id.to_string());
        request.approved_amount = Some(approved_amount);
        request.expires_at_epoch = Some(expires_at_epoch);
        request.rejection_reason = None;
        Ok(())
    }

    pub(super) fn apply_first_agent_claim_approval_rejected(
        &mut self,
        request_id: u64,
        operator_account_id: &str,
        claimer_agent_id: &str,
        rejected_at_epoch: u64,
        reason: &str,
    ) -> Result<(), WorldError> {
        let request = self
            .first_agent_claim_approval_requests
            .get_mut(&request_id)
            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "first agent claim approval request missing for reject: request_id={request_id}"
                ),
            })?;
        if request.status != FirstAgentClaimApprovalRequestStatus::Pending {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "first agent claim approval request not pending for reject: request_id={} status={:?}",
                    request_id, request.status
                ),
            });
        }
        if request.claimer_agent_id != claimer_agent_id {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "first agent claim approval claimer mismatch for reject: request_id={} expected={} actual={}",
                    request_id, request.claimer_agent_id, claimer_agent_id
                ),
            });
        }
        let reason = reason.trim();
        if reason.is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "first agent claim approval reject reason cannot be empty".to_string(),
            });
        }
        request.status = FirstAgentClaimApprovalRequestStatus::Rejected;
        request.updated_at_epoch = rejected_at_epoch;
        request.operator_account_id = Some(operator_account_id.to_string());
        request.approved_amount = None;
        request.expires_at_epoch = None;
        request.rejection_reason = Some(reason.to_string());
        Ok(())
    }
}
