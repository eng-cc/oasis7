use super::*;
use crate::runtime::main_token::{
    is_main_token_treasury_distribution_bucket, FirstAgentClaimApprovalRequestStatus,
    MainTokenTreasuryDistribution, RestrictedStarterClaimGrantStatus,
    FIRST_AGENT_CLAIM_APPROVAL_ISSUANCE_REASON, MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL,
    MAIN_TOKEN_TREASURY_BUCKET_RESTRICTED_STARTER_CLAIM_LIVEOPS_POOL,
    RESTRICTED_STARTER_CLAIM_GRANT_SPEND_SCOPE_SLOT_1_ONLY,
};
use std::collections::BTreeSet;

impl World {
    pub(super) fn evaluate_transfer_main_token_action(
        &self,
        action_id: ActionId,
        from_account_id: &str,
        to_account_id: &str,
        amount: u64,
        nonce: u64,
    ) -> DomainEvent {
        let from_account_id = from_account_id.trim();
        let to_account_id = to_account_id.trim();
        if from_account_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["from_account_id cannot be empty".to_string()],
                },
            };
        }
        if to_account_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["to_account_id cannot be empty".to_string()],
                },
            };
        }
        if from_account_id == to_account_id {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["from_account_id and to_account_id cannot be the same".to_string()],
                },
            };
        }
        if amount == 0 {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["amount must be > 0".to_string()],
                },
            };
        }
        if nonce == 0 {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["nonce must be > 0".to_string()],
                },
            };
        }

        let event = DomainEvent::MainTokenTransferred {
            from_account_id: from_account_id.to_string(),
            to_account_id: to_account_id.to_string(),
            amount,
            nonce,
        };
        let mut preview_state = self.state.clone();
        if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!("main token transfer rejected: {err:?}")],
                },
            };
        }
        event
    }

    pub(super) fn evaluate_distribute_main_token_treasury_action(
        &self,
        action_id: ActionId,
        proposal_id: ProposalId,
        distribution_id: &str,
        bucket_id: &str,
        distributions: &[MainTokenTreasuryDistribution],
    ) -> DomainEvent {
        if self.state.main_token_genesis_buckets.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["main token genesis is not initialized".to_string()],
                },
            };
        }
        if proposal_id == 0 {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["proposal_id must be > 0".to_string()],
                },
            };
        }
        let Some(proposal) = self.proposals.get(&proposal_id) else {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "distribute main token treasury rejected: governance proposal not found ({proposal_id})"
                    )],
                },
            };
        };
        match proposal.status {
            ProposalStatus::Approved { .. } | ProposalStatus::Applied { .. } => {}
            _ => {
                return DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "distribute main token treasury rejected: governance proposal must be approved or applied ({proposal_id})"
                        )],
                    },
                };
            }
        }

        let distribution_id = distribution_id.trim();
        if distribution_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["distribution_id cannot be empty".to_string()],
                },
            };
        }
        if self
            .state
            .main_token_treasury_distribution_records
            .contains_key(distribution_id)
        {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "distribute main token treasury rejected: distribution_id already exists ({distribution_id})"
                    )],
                },
            };
        }

        let bucket_id = bucket_id.trim();
        if !is_main_token_treasury_distribution_bucket(bucket_id) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "distribute main token treasury rejected: unsupported bucket ({bucket_id})"
                    )],
                },
            };
        }
        if distributions.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["distributions cannot be empty".to_string()],
                },
            };
        }

        let mut normalized_distributions = Vec::with_capacity(distributions.len());
        let mut seen_accounts = BTreeSet::new();
        let mut total_amount = 0_u64;
        for item in distributions {
            let account_id = item.account_id.trim();
            if account_id.is_empty() {
                return DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "distribute main token treasury rejected: account_id cannot be empty (distribution_id={distribution_id})"
                        )],
                    },
                };
            }
            if item.amount == 0 {
                return DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "distribute main token treasury rejected: amount must be > 0 (distribution_id={distribution_id} account_id={account_id})"
                        )],
                    },
                };
            }
            if !seen_accounts.insert(account_id.to_string()) {
                return DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "distribute main token treasury rejected: duplicate account_id ({account_id})"
                        )],
                    },
                };
            }
            total_amount = match total_amount.checked_add(item.amount) {
                Some(value) => value,
                None => {
                    return DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!(
                                "distribute main token treasury rejected: total_amount overflow (distribution_id={distribution_id})"
                            )],
                        },
                    };
                }
            };
            normalized_distributions.push(MainTokenTreasuryDistribution {
                account_id: account_id.to_string(),
                amount: item.amount,
            });
        }

        let bucket_balance = self
            .state
            .main_token_treasury_balances
            .get(bucket_id)
            .copied()
            .unwrap_or(0);
        if bucket_balance < total_amount {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "distribute main token treasury rejected: treasury bucket insufficient (bucket={bucket_id} balance={bucket_balance} total={total_amount})"
                    )],
                },
            };
        }

        let event = DomainEvent::MainTokenTreasuryDistributed {
            proposal_id,
            distribution_id: distribution_id.to_string(),
            bucket_id: bucket_id.to_string(),
            total_amount,
            distributions: normalized_distributions,
        };
        let mut preview_state = self.state.clone();
        if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!("distribute main token treasury rejected: {err:?}")],
                },
            };
        }
        event
    }

    pub(super) fn evaluate_top_up_restricted_starter_claim_liveops_pool_action(
        &self,
        action_id: ActionId,
        controller_account_id: &str,
        top_up_id: &str,
        amount: u64,
    ) -> DomainEvent {
        let Some(registry) = self.governance_main_token_controller_registry() else {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![
                        "restricted claim liveops pool top-up rejected: main token controller registry is not configured"
                            .to_string(),
                    ],
                },
            };
        };
        let controller_account_id = controller_account_id.trim();
        if controller_account_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![
                        "restricted claim liveops pool top-up controller_account_id cannot be empty"
                            .to_string(),
                    ],
                },
            };
        }
        let expected_controller_account_id = match Self::ecosystem_treasury_controller_account_id(
            registry,
            "restricted claim liveops pool top-up",
        ) {
            Ok(account_id) => account_id,
            Err(err) => {
                return DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "restricted claim liveops pool top-up rejected: {err:?}"
                        )],
                    },
                };
            }
        };
        if controller_account_id != expected_controller_account_id {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted claim liveops pool top-up rejected: controller_account_id does not match ecosystem treasury controller slot expected={} actual={}",
                        expected_controller_account_id, controller_account_id
                    )],
                },
            };
        }
        let top_up_id = top_up_id.trim();
        if top_up_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![
                        "restricted claim liveops pool top_up_id cannot be empty".to_string()
                    ],
                },
            };
        }
        if self
            .state
            .restricted_starter_claim_liveops_pool_top_up_records
            .contains_key(top_up_id)
        {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted claim liveops pool top-up rejected: top_up_id already exists ({top_up_id})"
                    )],
                },
            };
        }
        if amount == 0 {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![
                        "restricted claim liveops pool top-up amount must be > 0".to_string()
                    ],
                },
            };
        }
        let source_bucket_balance =
            self.main_token_treasury_balance(MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL);
        if source_bucket_balance < amount {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted claim liveops pool top-up treasury insufficient: bucket={} balance={} amount={amount}",
                        MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL, source_bucket_balance
                    )],
                },
            };
        }

        let event = DomainEvent::RestrictedStarterClaimLiveopsPoolToppedUp {
            controller_account_id: controller_account_id.to_string(),
            top_up_id: top_up_id.to_string(),
            source_treasury_bucket_id: MAIN_TOKEN_TREASURY_BUCKET_ECOSYSTEM_POOL.to_string(),
            target_treasury_bucket_id:
                MAIN_TOKEN_TREASURY_BUCKET_RESTRICTED_STARTER_CLAIM_LIVEOPS_POOL.to_string(),
            amount,
            topped_up_at_epoch: self.current_governance_epoch(),
        };
        let mut preview_state = self.state.clone();
        if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted claim liveops pool top-up rejected: {err:?}"
                    )],
                },
            };
        }
        event
    }

    pub(super) fn evaluate_issue_restricted_starter_claim_grant_action(
        &self,
        action_id: ActionId,
        issuer_account_id: &str,
        beneficiary_account_id: &str,
        amount: u64,
        issuance_reason: &str,
        expires_at_epoch: u64,
    ) -> DomainEvent {
        let issuer_account_id = issuer_account_id.trim();
        if issuer_account_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["issuer_account_id cannot be empty".to_string()],
                },
            };
        }
        if let Err(reason) = self.ensure_restricted_starter_claim_admin(issuer_account_id) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![reason],
                },
            };
        }
        let beneficiary_account_id = beneficiary_account_id.trim();
        if beneficiary_account_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["beneficiary_account_id cannot be empty".to_string()],
                },
            };
        }
        if amount == 0 {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["restricted grant amount must be > 0".to_string()],
                },
            };
        }
        let issuance_reason = issuance_reason.trim();
        if issuance_reason.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["issuance_reason cannot be empty".to_string()],
                },
            };
        }
        let current_epoch = self.current_governance_epoch();
        if expires_at_epoch <= current_epoch {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted grant expires_at_epoch must be > current_epoch ({expires_at_epoch} <= {current_epoch})"
                    )],
                },
            };
        }
        let source_bucket_id = MAIN_TOKEN_TREASURY_BUCKET_RESTRICTED_STARTER_CLAIM_LIVEOPS_POOL;
        let source_bucket_balance = self.main_token_treasury_balance(source_bucket_id);
        if source_bucket_balance < amount {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted grant treasury insufficient: bucket={source_bucket_id} balance={source_bucket_balance} amount={amount}"
                    )],
                },
            };
        }
        if !self.restricted_starter_claim_grant_can_be_reissued(beneficiary_account_id) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted grant already active or pending settlement: beneficiary={beneficiary_account_id}"
                    )],
                },
            };
        }
        if self.main_token_restricted_starter_claim_balance(beneficiary_account_id) > 0 {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted grant beneficiary already has seeded restricted balance without reusable grant slot: beneficiary={beneficiary_account_id}"
                    )],
                },
            };
        }

        let event = DomainEvent::RestrictedStarterClaimGrantIssued {
            issuer_id: issuer_account_id.to_string(),
            beneficiary_account_id: beneficiary_account_id.to_string(),
            source_treasury_bucket_id: source_bucket_id.to_string(),
            amount,
            issuance_reason: issuance_reason.to_string(),
            spend_scope: RESTRICTED_STARTER_CLAIM_GRANT_SPEND_SCOPE_SLOT_1_ONLY.to_string(),
            issued_at_epoch: current_epoch,
            expires_at_epoch,
        };
        let mut preview_state = self.state.clone();
        if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!("issue restricted grant rejected: {err:?}")],
                },
            };
        }
        event
    }

    pub(super) fn evaluate_revoke_restricted_starter_claim_grant_action(
        &self,
        action_id: ActionId,
        issuer_account_id: &str,
        beneficiary_account_id: &str,
        revoke_reason: &str,
    ) -> DomainEvent {
        let issuer_account_id = issuer_account_id.trim();
        if issuer_account_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["issuer_account_id cannot be empty".to_string()],
                },
            };
        }
        if let Err(reason) = self.ensure_restricted_starter_claim_admin(issuer_account_id) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![reason],
                },
            };
        }
        let beneficiary_account_id = beneficiary_account_id.trim();
        if beneficiary_account_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["beneficiary_account_id cannot be empty".to_string()],
                },
            };
        }
        let revoke_reason = revoke_reason.trim();
        if revoke_reason.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["revoke_reason cannot be empty".to_string()],
                },
            };
        }

        let Some(grant) = self
            .state
            .restricted_starter_claim_grants
            .get(beneficiary_account_id)
        else {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted grant not found: beneficiary={beneficiary_account_id}"
                    )],
                },
            };
        };
        if grant.status != RestrictedStarterClaimGrantStatus::Issued {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted grant is no longer active: beneficiary={} status={:?}",
                        beneficiary_account_id, grant.status
                    )],
                },
            };
        }
        if grant.issuer_id != issuer_account_id {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "restricted grant issuer mismatch: beneficiary={} expected={} actual={}",
                        beneficiary_account_id, grant.issuer_id, issuer_account_id
                    )],
                },
            };
        }

        let event = DomainEvent::RestrictedStarterClaimGrantRevoked {
            beneficiary_account_id: beneficiary_account_id.to_string(),
            issuer_id: issuer_account_id.to_string(),
            issuance_reason: grant.issuance_reason.clone(),
            spend_scope: grant.spend_scope.clone(),
            source_treasury_bucket_id: grant.source_treasury_bucket_id.clone(),
            issued_amount: grant.issued_amount,
            revoked_amount: self
                .main_token_restricted_starter_claim_balance(beneficiary_account_id),
            issued_at_epoch: grant.issued_at_epoch,
            revoked_at_epoch: self.current_governance_epoch(),
            configured_expires_at_epoch: grant.expires_at_epoch,
            revoke_reason: revoke_reason.to_string(),
        };
        let mut preview_state = self.state.clone();
        if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!("revoke restricted grant rejected: {err:?}")],
                },
            };
        }
        event
    }

    pub(super) fn evaluate_submit_first_agent_claim_approval_request_action(
        &self,
        action_id: ActionId,
        claimer_agent_id: &str,
    ) -> DomainEvent {
        let claimer_agent_id = claimer_agent_id.trim();
        if claimer_agent_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["claimer_agent_id cannot be empty".to_string()],
                },
            };
        }
        let quote = match self.first_agent_claim_approval_quote(claimer_agent_id) {
            Ok(quote) => quote,
            Err(note) => {
                return DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied { notes: vec![note] },
                };
            }
        };
        if self
            .state
            .first_agent_claim_approval_requests
            .values()
            .any(|request| {
                request.claimer_agent_id == claimer_agent_id
                    && request.status == FirstAgentClaimApprovalRequestStatus::Pending
            })
        {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "first agent claim approval request already pending: claimer_agent_id={claimer_agent_id}"
                    )],
                },
            };
        }
        let request_id = self.state.next_first_agent_claim_approval_request_id.max(1);
        let event = DomainEvent::FirstAgentClaimApprovalRequested {
            request_id,
            claimer_agent_id: claimer_agent_id.to_string(),
            requested_slot_index: quote.slot_index,
            requested_reputation_tier: quote.reputation_tier,
            requested_total_upfront_amount: quote
                .activation_fee_amount
                .saturating_add(quote.claim_bond_amount)
                .saturating_add(quote.upkeep_per_epoch),
            requested_at_epoch: self.current_governance_epoch(),
        };
        let mut preview_state = self.state.clone();
        if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "submit first agent claim approval request rejected: {err:?}"
                    )],
                },
            };
        }
        event
    }

    pub(super) fn evaluate_approve_first_agent_claim_approval_request_action(
        &self,
        action_id: ActionId,
        operator_account_id: &str,
        request_id: u64,
        expires_at_epoch: u64,
    ) -> DomainEvent {
        let operator_account_id = operator_account_id.trim();
        if operator_account_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["operator_account_id cannot be empty".to_string()],
                },
            };
        }
        if let Err(reason) = self.ensure_restricted_starter_claim_admin(operator_account_id) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![reason],
                },
            };
        }
        if request_id == 0 {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["request_id must be > 0".to_string()],
                },
            };
        }
        let Some(request) = self
            .state
            .first_agent_claim_approval_requests
            .get(&request_id)
        else {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "first agent claim approval request not found: request_id={request_id}"
                    )],
                },
            };
        };
        if request.status != FirstAgentClaimApprovalRequestStatus::Pending {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "first agent claim approval request is not pending: request_id={} status={:?}",
                        request_id, request.status
                    )],
                },
            };
        }
        let current_epoch = self.current_governance_epoch();
        if expires_at_epoch <= current_epoch {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "first agent claim approval expires_at_epoch must be > current_epoch ({expires_at_epoch} <= {current_epoch})"
                    )],
                },
            };
        }
        let quote = match self.first_agent_claim_approval_quote(request.claimer_agent_id.as_str()) {
            Ok(quote) => quote,
            Err(note) => {
                return DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied { notes: vec![note] },
                };
            }
        };
        let approved_amount = quote
            .activation_fee_amount
            .saturating_add(quote.claim_bond_amount)
            .saturating_add(quote.upkeep_per_epoch);
        let source_bucket_id = MAIN_TOKEN_TREASURY_BUCKET_RESTRICTED_STARTER_CLAIM_LIVEOPS_POOL;
        let source_bucket_balance = self.main_token_treasury_balance(source_bucket_id);
        if source_bucket_balance < approved_amount {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "first agent claim approval treasury insufficient: bucket={source_bucket_id} balance={source_bucket_balance} amount={approved_amount}"
                    )],
                },
            };
        }

        let event = DomainEvent::FirstAgentClaimApprovalApproved {
            request_id,
            operator_account_id: operator_account_id.to_string(),
            claimer_agent_id: request.claimer_agent_id.clone(),
            approved_amount,
            issuance_reason: FIRST_AGENT_CLAIM_APPROVAL_ISSUANCE_REASON.to_string(),
            spend_scope: RESTRICTED_STARTER_CLAIM_GRANT_SPEND_SCOPE_SLOT_1_ONLY.to_string(),
            source_treasury_bucket_id: source_bucket_id.to_string(),
            approved_at_epoch: current_epoch,
            expires_at_epoch,
        };
        let mut preview_state = self.state.clone();
        if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "approve first agent claim approval request rejected: {err:?}"
                    )],
                },
            };
        }
        event
    }

    pub(super) fn evaluate_reject_first_agent_claim_approval_request_action(
        &self,
        action_id: ActionId,
        operator_account_id: &str,
        request_id: u64,
        reason: &str,
    ) -> DomainEvent {
        let operator_account_id = operator_account_id.trim();
        if operator_account_id.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["operator_account_id cannot be empty".to_string()],
                },
            };
        }
        if let Err(note) = self.ensure_restricted_starter_claim_admin(operator_account_id) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied { notes: vec![note] },
            };
        }
        if request_id == 0 {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["request_id must be > 0".to_string()],
                },
            };
        }
        let Some(request) = self
            .state
            .first_agent_claim_approval_requests
            .get(&request_id)
        else {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "first agent claim approval request not found: request_id={request_id}"
                    )],
                },
            };
        };
        if request.status != FirstAgentClaimApprovalRequestStatus::Pending {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "first agent claim approval request is not pending: request_id={} status={:?}",
                        request_id, request.status
                    )],
                },
            };
        }
        let reason = reason.trim();
        if reason.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["reason cannot be empty".to_string()],
                },
            };
        }

        let event = DomainEvent::FirstAgentClaimApprovalRejected {
            request_id,
            operator_account_id: operator_account_id.to_string(),
            claimer_agent_id: request.claimer_agent_id.clone(),
            rejected_at_epoch: self.current_governance_epoch(),
            reason: reason.to_string(),
        };
        let mut preview_state = self.state.clone();
        if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "reject first agent claim approval request rejected: {err:?}"
                    )],
                },
            };
        }
        event
    }

    fn first_agent_claim_approval_quote(
        &self,
        claimer_agent_id: &str,
    ) -> Result<crate::runtime::AgentClaimCostQuote, String> {
        if !self.state.agents.contains_key(claimer_agent_id) {
            return Err(format!(
                "first agent claim approval rejected: claimer_agent_id not found ({claimer_agent_id})"
            ));
        }
        let owned_claim_count = self
            .state
            .agent_claims
            .values()
            .filter(|claim| claim.claim_owner_id == claimer_agent_id)
            .count();
        if owned_claim_count > 0 {
            return Err(format!(
                "first agent claim approval only supports slot-1 onboarding: claimer_agent_id={} owned_claim_count={owned_claim_count}",
                claimer_agent_id
            ));
        }
        if !self.restricted_starter_claim_grant_can_be_reissued(claimer_agent_id) {
            return Err(format!(
                "first agent claim approval rejected: restricted starter claim grant already active or pending settlement: claimer_agent_id={claimer_agent_id}"
            ));
        }
        if self.main_token_restricted_starter_claim_balance(claimer_agent_id) > 0 {
            return Err(format!(
                "first agent claim approval rejected: claimer already has restricted starter claim balance: claimer_agent_id={} balance={}",
                claimer_agent_id,
                self.main_token_restricted_starter_claim_balance(claimer_agent_id)
            ));
        }
        let reputation_score = self
            .state
            .reputation_scores
            .get(claimer_agent_id)
            .copied()
            .unwrap_or(0);
        let quote = crate::runtime::agent_claim_quote(reputation_score, owned_claim_count)
            .map_err(|reason| {
                format!(
                    "first agent claim approval rejected: cannot build slot-1 quote for claimer_agent_id={} ({reason})",
                    claimer_agent_id
                )
            })?;
        if quote.slot_index != 1 {
            return Err(format!(
                "first agent claim approval only supports slot-1 requests: claimer_agent_id={} slot_index={}",
                claimer_agent_id, quote.slot_index
            ));
        }
        Ok(quote)
    }
}
