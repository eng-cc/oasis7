use super::super::{
    main_token_bucket_unlocked_amount, util::hash_json, Action, ActionEnvelope, ActionId, CausedBy,
    CrisisStatus, DomainEvent, EconomicContractStatus, EpochSettlementReport,
    GovernanceEvent, GovernanceProposalStatus, MainTokenConfig, MainTokenFeeKind,
    MainTokenGenesisAllocationBucketState, MainTokenGenesisAllocationPlan,
    MainTokenNodePointsBridgeDistribution, MaterialLedgerId, MaterialStack,
    MaterialTransitPriority, NodeRewardMintRecord, NodeSettlement, ProposalId, ProposalStatus,
    RejectReason, WorldError, WorldEvent, WorldEventBody, WorldEventId, WorldTime,
};
use super::body::{evaluate_expand_body_interface, validate_body_kernel_view};
use super::logistics::{
    MATERIAL_TRANSFER_LOSS_PER_KM_BPS, MATERIAL_TRANSFER_MAX_DISTANCE_KM,
    MATERIAL_TRANSFER_MAX_INFLIGHT, MATERIAL_TRANSFER_SPEED_KM_PER_TICK,
};
use super::World;
use crate::geometry::space_distance_cm;
use crate::runtime::main_token::{
    main_token_account_id_from_node_public_key, validate_main_token_config_bounds,
    MAIN_TOKEN_BPS_DENOMINATOR, MAIN_TOKEN_TREASURY_BUCKET_NODE_SERVICE_REWARD,
};
use crate::simulator::ResourceKind;
use std::collections::BTreeSet;

const GOVERNANCE_MIN_VOTING_WINDOW_TICKS: u64 = 1;
const GOVERNANCE_MAX_VOTING_WINDOW_TICKS: u64 = 1_440;
const GOVERNANCE_MIN_PASS_THRESHOLD_BPS: u16 = 5_000;
const GOVERNANCE_MAX_PASS_THRESHOLD_BPS: u16 = 10_000;
const GOVERNANCE_MAX_VOTE_WEIGHT: u32 = 100;
const WAR_MAX_INTENSITY: u32 = 10;
const WAR_MIN_ALLIANCE_MEMBERS: usize = 2;
const WAR_MAX_ALLIANCE_MEMBERS: usize = 16;
const WAR_DECLARE_BASE_ELECTRICITY_COST: i64 = 12;
const WAR_DECLARE_ELECTRICITY_COST_PER_INTENSITY: i64 = 4;
const WAR_DECLARE_BASE_DATA_COST: i64 = 8;
const WAR_DECLARE_DATA_COST_PER_INTENSITY: i64 = 3;
const CRISIS_BASE_IMPACT_PER_SEVERITY: i64 = 10;
const GAMEPLAY_POLICY_MAX_TAX_BPS: u16 = 10_000;
const GAMEPLAY_POLICY_MIN_CONTRACT_QUOTA: u16 = 1;
const GAMEPLAY_POLICY_MAX_CONTRACT_QUOTA: u16 = 64;
const GAMEPLAY_POLICY_UPDATE_MIN_GOVERNANCE_TOTAL_WEIGHT: u64 = 3;
const ECONOMIC_CONTRACT_MAX_REPUTATION_STAKE: i64 = 10_000;
const ECONOMIC_CONTRACT_SUCCESS_REPUTATION_AMOUNT_DIVISOR: i64 = 10;
const ECONOMIC_CONTRACT_SUCCESS_REPUTATION_REWARD_CAP: i64 = 12;
const ECONOMIC_CONTRACT_PAIR_COOLDOWN_TICKS: u64 = 5;
const ECONOMIC_CONTRACT_REPUTATION_WINDOW_TICKS: u64 = 20;
const ECONOMIC_CONTRACT_REPUTATION_WINDOW_CAP: i64 = 24;
const MAIN_TOKEN_POLICY_UPDATE_DELAY_EPOCHS: u64 = 2;

mod action_to_event_core;
pub(super) mod action_to_event_economy;
mod action_to_event_gameplay;
mod action_to_event_policy_contract;

impl World {
    // ---------------------------------------------------------------------
    // Internal helpers
    // ---------------------------------------------------------------------

    fn agent_alliance_id(&self, agent_id: &str) -> Option<&str> {
        self.state
            .alliances
            .iter()
            .find(|(_, alliance)| alliance.members.iter().any(|member| member == agent_id))
            .map(|(alliance_id, _)| alliance_id.as_str())
    }

    fn alliance_has_active_war(&self, alliance_id: &str) -> bool {
        self.state.wars.values().any(|war| {
            war.active
                && (war.aggressor_alliance_id == alliance_id
                    || war.defender_alliance_id == alliance_id)
        })
    }

    fn has_policy_update_governance_authorization(&self, operator_agent_id: &str) -> bool {
        self.state.governance_proposals.values().any(|proposal| {
            proposal.proposer_agent_id == operator_agent_id
                && proposal.status == GovernanceProposalStatus::Passed
                && proposal.total_weight_at_finalize
                    >= GAMEPLAY_POLICY_UPDATE_MIN_GOVERNANCE_TOTAL_WEIGHT
        })
    }

    fn war_mobilization_costs(intensity: u32) -> (i64, i64) {
        let intensity = i64::from(intensity.max(1));
        let electricity = WAR_DECLARE_BASE_ELECTRICITY_COST
            .saturating_add(intensity.saturating_mul(WAR_DECLARE_ELECTRICITY_COST_PER_INTENSITY));
        let data = WAR_DECLARE_BASE_DATA_COST
            .saturating_add(intensity.saturating_mul(WAR_DECLARE_DATA_COST_PER_INTENSITY));
        (electricity, data)
    }

    fn economic_contract_success_reputation_reward(
        settlement_amount: i64,
        reputation_stake: i64,
    ) -> i64 {
        let amount_based_reward = settlement_amount
            .saturating_div(ECONOMIC_CONTRACT_SUCCESS_REPUTATION_AMOUNT_DIVISOR)
            .max(1);
        amount_based_reward
            .min(reputation_stake)
            .min(ECONOMIC_CONTRACT_SUCCESS_REPUTATION_REWARD_CAP)
    }

    pub(super) fn replay_from(&mut self, start_index: usize) -> Result<(), WorldError> {
        let start_index = start_index.min(self.journal.events.len());
        let events: Vec<WorldEvent> = self.journal.events[start_index..].to_vec();
        let mut replaying_tick: Option<WorldTime> = None;
        for event in events {
            if let Some(tick) = replaying_tick {
                if event.time != tick {
                    self.record_tick_consensus_for_tick(tick)?;
                }
            }
            self.apply_event_body(&event.body, event.time)?;
            self.state.time = event.time;
            self.next_event_id = self.next_event_id.max(event.id.saturating_add(1));
            replaying_tick = Some(event.time);
        }
        if let Some(tick) = replaying_tick {
            self.record_tick_consensus_for_tick(tick)?;
        }
        Ok(())
    }

    pub(super) fn action_to_event(
        &self,
        envelope: &ActionEnvelope,
    ) -> Result<WorldEventBody, WorldError> {
        let action_id = envelope.id;
        match &envelope.action {
            Action::RegisterAgent { .. }
            | Action::MoveAgent { .. }
            | Action::QueryObservation { .. }
            | Action::EmitObservation { .. }
            | Action::BodyAction { .. }
            | Action::EmitBodyAttributes { .. }
            | Action::ExpandBodyInterface { .. }
            | Action::DeployModuleArtifact { .. }
            | Action::CompileModuleArtifactFromSource { .. }
            | Action::InstallModuleFromArtifact { .. }
            | Action::InstallModuleFromArtifactWithFinality { .. }
            | Action::InstallModuleToTargetFromArtifact { .. }
            | Action::InstallModuleToTargetFromArtifactWithFinality { .. }
            | Action::UpgradeModuleFromArtifact { .. }
            | Action::UpgradeModuleFromArtifactWithFinality { .. }
            | Action::RollbackModuleInstance { .. }
            | Action::RollbackModuleInstanceWithFinality { .. }
            | Action::ModuleReleaseSubmit { .. }
            | Action::ModuleReleaseShadow { .. }
            | Action::ModuleReleaseApproveRole { .. }
            | Action::ModuleReleaseBindRoles { .. }
            | Action::ModuleReleaseSubmitAttestation { .. }
            | Action::ModuleReleaseReject { .. }
            | Action::ModuleReleaseApply { .. }
            | Action::ModuleReleaseApplyWithFinality { .. }
            | Action::ListModuleArtifactForSale { .. }
            | Action::BuyModuleArtifact { .. }
            | Action::DelistModuleArtifact { .. }
            | Action::DestroyModuleArtifact { .. }
            | Action::PlaceModuleArtifactBid { .. }
            | Action::CancelModuleArtifactBid { .. }
            | Action::TransferResource { .. }
            | Action::RedeemPower { .. }
            | Action::RedeemPowerSigned { .. }
            | Action::ApplyNodePointsSettlementSigned { .. }
            | Action::InitializeMainTokenGenesis { .. }
            | Action::ClaimMainTokenVesting { .. }
            | Action::TransferMainToken { .. }
            | Action::ApplyMainTokenEpochIssuance { .. }
            | Action::SettleMainTokenFee { .. }
            | Action::UpdateMainTokenPolicy { .. }
            | Action::DistributeMainTokenTreasury { .. }
            | Action::IssueRestrictedStarterClaimGrant { .. }
            | Action::RevokeRestrictedStarterClaimGrant { .. }
            | Action::TransferMaterial { .. } => {
                self.action_to_event_core(action_id, &envelope.action)
            }
            Action::FormAlliance { .. }
            | Action::JoinAlliance { .. }
            | Action::LeaveAlliance { .. }
            | Action::DissolveAlliance { .. }
            | Action::DeclareWar { .. }
            | Action::OpenGovernanceProposal { .. }
            | Action::CastGovernanceVote { .. }
            | Action::ResolveCrisis { .. }
            | Action::GrantMetaProgress { .. } => {
                self.action_to_event_gameplay(action_id, &envelope.action)
            }
            Action::ClaimAgent { .. } | Action::ReleaseAgentClaim { .. } => {
                self.action_to_event_gameplay(action_id, &envelope.action)
            }
            Action::UpdateGameplayPolicy { .. }
            | Action::UpdateRestrictedStarterClaimAdminRegistry { .. }
            | Action::OpenEconomicContract { .. }
            | Action::AcceptEconomicContract { .. }
            | Action::SettleEconomicContract { .. } => {
                self.action_to_event_policy_contract(action_id, &envelope.action)
            }
            Action::EmitResourceTransfer { .. }
            | Action::CollectData { .. }
            | Action::GrantDataAccess { .. }
            | Action::RevokeDataAccess { .. }
            | Action::BuildFactory { .. }
            | Action::BuildFactoryWithModule { .. }
            | Action::MaintainFactory { .. }
            | Action::RecycleFactory { .. }
            | Action::ScheduleRecipe { .. }
            | Action::ScheduleRecipeWithModule { .. }
            | Action::ValidateProduct { .. }
            | Action::ValidateProductWithModule { .. }
            | Action::GovernMaterialProfile { .. }
            | Action::GovernProductProfile { .. }
            | Action::GovernRecipeProfile { .. }
            | Action::GovernFactoryProfile { .. } => {
                self.action_to_event_economy(action_id, &envelope.action)
            }
        }
    }

    fn select_material_consume_ledger_with_world_fallback(
        &self,
        preferred_ledger: MaterialLedgerId,
        consume: &[MaterialStack],
    ) -> MaterialLedgerId {
        if self.has_materials_in_ledger(&preferred_ledger, consume) {
            preferred_ledger
        } else {
            MaterialLedgerId::world()
        }
    }

    fn evaluate_apply_node_points_settlement_action(
        &self,
        action_id: ActionId,
        report: &EpochSettlementReport,
        signer_node_id: &str,
        mint_records: &[NodeRewardMintRecord],
    ) -> DomainEvent {
        let settlement_hash = match hash_json(report) {
            Ok(hash) => hash,
            Err(err) => {
                return DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!("settlement hash compute failed: {err:?}")],
                    },
                };
            }
        };
        let (main_token_bridge_total_amount, main_token_bridge_distributions) =
            match self.build_main_token_bridge_distributions_for_settlement(report) {
                Ok(values) => values,
                Err(reason) => {
                    return DomainEvent::ActionRejected {
                        action_id,
                        reason: RejectReason::RuleDenied {
                            notes: vec![format!("apply node points settlement rejected: {reason}")],
                        },
                    };
                }
            };

        let event = DomainEvent::NodePointsSettlementApplied {
            report: report.clone(),
            signer_node_id: signer_node_id.to_string(),
            settlement_hash,
            minted_records: mint_records.to_vec(),
            main_token_bridge_total_amount,
            main_token_bridge_distributions,
        };
        let mut preview_state = self.state.clone();
        if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!("apply node points settlement rejected: {err:?}")],
                },
            };
        }
        event
    }

    fn build_main_token_bridge_distributions_for_settlement(
        &self,
        report: &EpochSettlementReport,
    ) -> Result<(u64, Vec<MainTokenNodePointsBridgeDistribution>), String> {
        if self
            .state
            .main_token_node_points_bridge_records
            .contains_key(&report.epoch_index)
        {
            return Err(format!(
                "main token bridge already processed for epoch={}",
                report.epoch_index
            ));
        }
        let Some(issuance) = self
            .state
            .main_token_epoch_issuance_records
            .get(&report.epoch_index)
        else {
            return Ok((0, Vec::new()));
        };
        let bridge_budget = issuance.node_service_reward_amount;
        if bridge_budget == 0 {
            return Ok((0, Vec::new()));
        }
        let treasury_balance = self
            .state
            .main_token_treasury_balances
            .get(MAIN_TOKEN_TREASURY_BUCKET_NODE_SERVICE_REWARD)
            .copied()
            .unwrap_or(0);
        if treasury_balance < bridge_budget {
            return Err(format!(
                "main token bridge treasury insufficient for epoch={} balance={} budget={}",
                report.epoch_index, treasury_balance, bridge_budget
            ));
        }

        let eligible = report
            .settlements
            .iter()
            .filter(|settlement| settlement.awarded_points > 0)
            .cloned()
            .collect::<Vec<_>>();
        if eligible.is_empty() {
            return Ok((0, Vec::new()));
        }

        let (total_amount, raw_distributions) =
            distribute_main_token_bridge_budget(bridge_budget, eligible.as_slice());
        let mut distributions = Vec::with_capacity(raw_distributions.len());
        for item in raw_distributions {
            let account_id = self.resolve_main_token_bridge_account_id_for_node(&item.node_id)?;
            distributions.push(MainTokenNodePointsBridgeDistribution {
                node_id: item.node_id,
                account_id,
                amount: item.amount,
            });
        }

        Ok((total_amount, distributions))
    }

    fn resolve_main_token_bridge_account_id_for_node(
        &self,
        node_id: &str,
    ) -> Result<String, String> {
        if let Some(account_id) = self.state.node_main_token_account_bindings.get(node_id) {
            let account_id = account_id.trim();
            if account_id.is_empty() {
                return Err(format!(
                    "main token account binding cannot be empty: node={}",
                    node_id
                ));
            }
            return Ok(account_id.to_string());
        }

        let public_key = self
            .state
            .node_identity_bindings
            .get(node_id)
            .ok_or_else(|| format!("main token account binding missing for node={node_id}"))?;
        Ok(main_token_account_id_from_node_public_key(public_key))
    }

    fn build_main_token_genesis_allocations(
        &self,
        plans: &[MainTokenGenesisAllocationPlan],
    ) -> Result<Vec<MainTokenGenesisAllocationBucketState>, String> {
        if plans.is_empty() {
            return Err("allocations cannot be empty".to_string());
        }
        let mut seen_bucket_ids = BTreeSet::new();
        let mut ratio_sum = 0_u64;
        for plan in plans {
            if plan.bucket_id.trim().is_empty() {
                return Err("allocation bucket_id cannot be empty".to_string());
            }
            if !seen_bucket_ids.insert(plan.bucket_id.as_str()) {
                return Err(format!(
                    "duplicate allocation bucket_id: {}",
                    plan.bucket_id
                ));
            }
            if plan.recipient.trim().is_empty() {
                return Err(format!(
                    "allocation recipient cannot be empty: bucket={}",
                    plan.bucket_id
                ));
            }
            if plan.ratio_bps == 0 {
                return Err(format!(
                    "allocation ratio must be > 0: bucket={}",
                    plan.bucket_id
                ));
            }
            ratio_sum = ratio_sum.saturating_add(u64::from(plan.ratio_bps));
        }
        if ratio_sum != 10_000 {
            return Err(format!(
                "allocation ratio sum must be 10000 bps, got {}",
                ratio_sum
            ));
        }

        let initial_supply = self.state.main_token_config.initial_supply;
        if initial_supply == 0 {
            return Err("main token initial_supply must be > 0".to_string());
        }

        let mut allocations = Vec::with_capacity(plans.len());
        let mut distributed = 0_u64;
        for plan in plans {
            let allocated_u128 =
                (u128::from(initial_supply) * u128::from(plan.ratio_bps)) / u128::from(10_000_u32);
            let allocated_amount = u64::try_from(allocated_u128).map_err(|_| {
                format!(
                    "allocated amount overflow: bucket={} amount={allocated_u128}",
                    plan.bucket_id
                )
            })?;
            distributed = distributed
                .checked_add(allocated_amount)
                .ok_or_else(|| "distributed allocation overflow".to_string())?;
            allocations.push(MainTokenGenesisAllocationBucketState {
                bucket_id: plan.bucket_id.clone(),
                ratio_bps: plan.ratio_bps,
                recipient: plan.recipient.clone(),
                cliff_epochs: plan.cliff_epochs,
                linear_unlock_epochs: plan.linear_unlock_epochs,
                start_epoch: plan.start_epoch,
                allocated_amount,
                claimed_amount: 0,
            });
        }

        let mut remainder = initial_supply.saturating_sub(distributed);
        allocations.sort_by(|a, b| {
            b.ratio_bps
                .cmp(&a.ratio_bps)
                .then_with(|| a.bucket_id.cmp(&b.bucket_id))
        });
        let mut index = 0_usize;
        while remainder > 0 && !allocations.is_empty() {
            let target = index % allocations.len();
            allocations[target].allocated_amount =
                allocations[target].allocated_amount.saturating_add(1);
            remainder -= 1;
            index = index.saturating_add(1);
        }
        allocations.sort_by(|a, b| a.bucket_id.cmp(&b.bucket_id));
        Ok(allocations)
    }

    fn evaluate_initialize_main_token_genesis_action(
        &self,
        action_id: ActionId,
        allocations: &[MainTokenGenesisAllocationPlan],
    ) -> DomainEvent {
        if !self.state.main_token_genesis_buckets.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["main token genesis is already initialized".to_string()],
                },
            };
        }
        if self.state.main_token_supply.total_supply > 0
            || self.state.main_token_supply.total_issued > 0
            || self.state.main_token_supply.total_burned > 0
        {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["main token supply is already initialized".to_string()],
                },
            };
        }

        let resolved_allocations = match self.build_main_token_genesis_allocations(allocations) {
            Ok(values) => values,
            Err(reason) => {
                return DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!("initialize main token genesis rejected: {reason}")],
                    },
                };
            }
        };

        let event = DomainEvent::MainTokenGenesisInitialized {
            total_supply: self.state.main_token_config.initial_supply,
            allocations: resolved_allocations,
        };
        let mut preview_state = self.state.clone();
        if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!("initialize main token genesis rejected: {err:?}")],
                },
            };
        }
        event
    }

    fn evaluate_claim_main_token_vesting_action(
        &self,
        action_id: ActionId,
        bucket_id: &str,
        beneficiary: &str,
        nonce: u64,
    ) -> DomainEvent {
        if bucket_id.trim().is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["bucket_id cannot be empty".to_string()],
                },
            };
        }
        if beneficiary.trim().is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["beneficiary cannot be empty".to_string()],
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
        let Some(bucket) = self.state.main_token_genesis_buckets.get(bucket_id) else {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!("genesis bucket not found: {bucket_id}")],
                },
            };
        };
        if bucket.recipient != beneficiary {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "beneficiary mismatch: bucket recipient={} claim beneficiary={}",
                        bucket.recipient, beneficiary
                    )],
                },
            };
        }
        let unlocked = main_token_bucket_unlocked_amount(bucket, self.state.time);
        let releasable = unlocked.saturating_sub(bucket.claimed_amount);
        if releasable == 0 {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "no releasable vesting balance for bucket={} at epoch={}",
                        bucket_id, self.state.time
                    )],
                },
            };
        }

        let event = DomainEvent::MainTokenVestingClaimed {
            bucket_id: bucket_id.to_string(),
            beneficiary: beneficiary.to_string(),
            amount: releasable,
            nonce,
        };
        let mut preview_state = self.state.clone();
        if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!("claim main token vesting rejected: {err:?}")],
                },
            };
        }
        event
    }

    fn resolve_main_token_effective_config_for_epoch(&self, epoch_index: u64) -> &MainTokenConfig {
        self.state
            .main_token_scheduled_policy_updates
            .range(..=epoch_index)
            .next_back()
            .map(|(_, item)| &item.next_config)
            .unwrap_or(&self.state.main_token_config)
    }

    fn resolve_main_token_effective_rate_bps(
        &self,
        config: &MainTokenConfig,
        actual_stake_ratio_bps: u32,
    ) -> Result<u32, String> {
        if actual_stake_ratio_bps > MAIN_TOKEN_BPS_DENOMINATOR {
            return Err(format!(
                "actual_stake_ratio_bps must be <= 10000, got {}",
                actual_stake_ratio_bps
            ));
        }
        let policy = &config.inflation_policy;
        if policy.epochs_per_year == 0 {
            return Err("inflation_policy.epochs_per_year must be > 0".to_string());
        }
        if policy.min_rate_bps > policy.max_rate_bps {
            return Err(format!(
                "inflation_policy min_rate_bps > max_rate_bps: {} > {}",
                policy.min_rate_bps, policy.max_rate_bps
            ));
        }
        let target = i128::from(policy.target_stake_ratio_bps);
        let actual = i128::from(actual_stake_ratio_bps);
        let gain = i128::from(policy.stake_feedback_gain_bps);
        let base = i128::from(policy.base_rate_bps);
        let feedback = target
            .saturating_sub(actual)
            .saturating_mul(gain)
            .saturating_div(i128::from(MAIN_TOKEN_BPS_DENOMINATOR));
        let rate = base.saturating_add(feedback);
        let clamped = rate.clamp(
            i128::from(policy.min_rate_bps),
            i128::from(policy.max_rate_bps),
        );
        u32::try_from(clamped)
            .map_err(|_| format!("effective inflation rate out of range: {clamped}"))
    }

    fn resolve_main_token_epoch_issued_amount(
        &self,
        config: &MainTokenConfig,
        inflation_rate_bps: u32,
    ) -> Result<u64, String> {
        let supply = &self.state.main_token_supply;
        let policy = &config.inflation_policy;
        if policy.epochs_per_year == 0 {
            return Err("inflation_policy.epochs_per_year must be > 0".to_string());
        }
        let numerator = u128::from(supply.circulating_supply)
            .checked_mul(u128::from(inflation_rate_bps))
            .ok_or_else(|| {
                format!(
                    "main token issuance overflow: circulating={} rate_bps={}",
                    supply.circulating_supply, inflation_rate_bps
                )
            })?;
        let denominator = u128::from(policy.epochs_per_year)
            .saturating_mul(u128::from(MAIN_TOKEN_BPS_DENOMINATOR));
        if denominator == 0 {
            return Err("main token issuance denominator cannot be zero".to_string());
        }
        let mut issued = u64::try_from(numerator / denominator).map_err(|_| {
            "main token issuance amount conversion overflow while converting to u64".to_string()
        })?;

        if let Some(max_supply) = config.max_supply {
            if supply.total_supply > max_supply {
                return Err(format!(
                    "main token total_supply already exceeds max_supply: total={} max={}",
                    supply.total_supply, max_supply
                ));
            }
            let remaining = max_supply.saturating_sub(supply.total_supply);
            issued = issued.min(remaining);
        }
        Ok(issued)
    }

    fn resolve_main_token_epoch_split_amounts(
        &self,
        config: &MainTokenConfig,
        issued_amount: u64,
    ) -> Result<(u64, u64, u64, u64), String> {
        let split = &config.issuance_split;
        let split_sum = u64::from(split.staking_reward_bps)
            .saturating_add(u64::from(split.node_service_reward_bps))
            .saturating_add(u64::from(split.ecosystem_pool_bps))
            .saturating_add(u64::from(split.security_reserve_bps));
        if split_sum != u64::from(MAIN_TOKEN_BPS_DENOMINATOR) {
            return Err(format!(
                "main token issuance split sum must be 10000 bps, got {}",
                split_sum
            ));
        }

        let staking_reward_amount = issued_amount
            .saturating_mul(u64::from(split.staking_reward_bps))
            / u64::from(MAIN_TOKEN_BPS_DENOMINATOR);
        let node_service_reward_amount = issued_amount
            .saturating_mul(u64::from(split.node_service_reward_bps))
            / u64::from(MAIN_TOKEN_BPS_DENOMINATOR);
        let ecosystem_pool_amount = issued_amount
            .saturating_mul(u64::from(split.ecosystem_pool_bps))
            / u64::from(MAIN_TOKEN_BPS_DENOMINATOR);
        let distributed = staking_reward_amount
            .checked_add(node_service_reward_amount)
            .and_then(|value| value.checked_add(ecosystem_pool_amount))
            .ok_or_else(|| {
                format!(
                    "main token issuance split overflow: issued={} staking={} node_service={} ecosystem={}",
                    issued_amount,
                    staking_reward_amount,
                    node_service_reward_amount,
                    ecosystem_pool_amount
                )
            })?;
        let security_reserve_amount = issued_amount.saturating_sub(distributed);
        Ok((
            staking_reward_amount,
            node_service_reward_amount,
            ecosystem_pool_amount,
            security_reserve_amount,
        ))
    }

    fn evaluate_apply_main_token_epoch_issuance_action(
        &self,
        action_id: ActionId,
        epoch_index: u64,
        actual_stake_ratio_bps: u32,
    ) -> DomainEvent {
        if self.state.main_token_genesis_buckets.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["main token genesis is not initialized".to_string()],
                },
            };
        }
        if self
            .state
            .main_token_epoch_issuance_records
            .contains_key(&epoch_index)
        {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "main token epoch issuance already exists: epoch={epoch_index}"
                    )],
                },
            };
        }

        let effective_config = self.resolve_main_token_effective_config_for_epoch(epoch_index);
        let inflation_rate_bps = match self
            .resolve_main_token_effective_rate_bps(effective_config, actual_stake_ratio_bps)
        {
            Ok(value) => value,
            Err(reason) => {
                return DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!("apply epoch issuance rejected: {reason}")],
                    },
                };
            }
        };
        let issued_amount = match self
            .resolve_main_token_epoch_issued_amount(effective_config, inflation_rate_bps)
        {
            Ok(value) => value,
            Err(reason) => {
                return DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!("apply epoch issuance rejected: {reason}")],
                    },
                };
            }
        };
        let (
            staking_reward_amount,
            node_service_reward_amount,
            ecosystem_pool_amount,
            security_reserve_amount,
        ) = match self.resolve_main_token_epoch_split_amounts(effective_config, issued_amount) {
            Ok(values) => values,
            Err(reason) => {
                return DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!("apply epoch issuance rejected: {reason}")],
                    },
                };
            }
        };

        let event = DomainEvent::MainTokenEpochIssued {
            epoch_index,
            inflation_rate_bps,
            issued_amount,
            staking_reward_amount,
            node_service_reward_amount,
            ecosystem_pool_amount,
            security_reserve_amount,
        };
        let mut preview_state = self.state.clone();
        if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!("apply epoch issuance rejected: {err:?}")],
                },
            };
        }
        event
    }

    fn resolve_main_token_fee_burn_bps(
        &self,
        config: &MainTokenConfig,
        fee_kind: MainTokenFeeKind,
    ) -> u32 {
        let policy = &config.burn_policy;
        match fee_kind {
            MainTokenFeeKind::GasBaseFee => policy.gas_base_fee_burn_bps,
            MainTokenFeeKind::SlashPenalty => policy.slash_burn_bps,
            MainTokenFeeKind::ModuleFee => policy.module_fee_burn_bps,
        }
    }

    fn evaluate_settle_main_token_fee_action(
        &self,
        action_id: ActionId,
        fee_kind: MainTokenFeeKind,
        amount: u64,
    ) -> DomainEvent {
        if self.state.main_token_genesis_buckets.is_empty() {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["main token genesis is not initialized".to_string()],
                },
            };
        }
        if amount == 0 {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec!["main token fee amount must be > 0".to_string()],
                },
            };
        }
        let effective_config = self.resolve_main_token_effective_config_for_epoch(self.state.time);
        let burn_bps = self.resolve_main_token_fee_burn_bps(effective_config, fee_kind);
        if burn_bps > MAIN_TOKEN_BPS_DENOMINATOR {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "main token burn bps must be <= 10000, got {}",
                        burn_bps
                    )],
                },
            };
        }
        let burn_amount =
            amount.saturating_mul(u64::from(burn_bps)) / u64::from(MAIN_TOKEN_BPS_DENOMINATOR);
        let treasury_amount = amount.saturating_sub(burn_amount);

        let event = DomainEvent::MainTokenFeeSettled {
            fee_kind,
            amount,
            burn_amount,
            treasury_amount,
        };
        let mut preview_state = self.state.clone();
        if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!("settle main token fee rejected: {err:?}")],
                },
            };
        }
        event
    }

    fn evaluate_update_main_token_policy_action(
        &self,
        action_id: ActionId,
        proposal_id: ProposalId,
        next: &MainTokenConfig,
    ) -> DomainEvent {
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
                        "update main token policy rejected: governance proposal not found ({proposal_id})"
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
                            "update main token policy rejected: governance proposal must be approved or applied ({proposal_id})"
                        )],
                    },
                };
            }
        }
        if let Err(reason) = validate_main_token_config_bounds(next) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!("update main token policy rejected: {reason}")],
                },
            };
        }
        if next.initial_supply != self.state.main_token_config.initial_supply {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "update main token policy rejected: initial_supply cannot change (current={} next={})",
                        self.state.main_token_config.initial_supply, next.initial_supply
                    )],
                },
            };
        }
        if let Some(max_supply) = next.max_supply {
            if max_supply < self.state.main_token_supply.total_supply {
                return DomainEvent::ActionRejected {
                    action_id,
                    reason: RejectReason::RuleDenied {
                        notes: vec![format!(
                            "update main token policy rejected: max_supply cannot be below total_supply (max={} total={})",
                            max_supply, self.state.main_token_supply.total_supply
                        )],
                    },
                };
            }
        }

        let effective_epoch = self
            .state
            .time
            .saturating_add(MAIN_TOKEN_POLICY_UPDATE_DELAY_EPOCHS);
        if self
            .state
            .main_token_scheduled_policy_updates
            .contains_key(&effective_epoch)
        {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "update main token policy rejected: effective_epoch already scheduled ({effective_epoch})"
                    )],
                },
            };
        }
        if self
            .state
            .main_token_scheduled_policy_updates
            .values()
            .any(|item| item.proposal_id == proposal_id)
        {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!(
                        "update main token policy rejected: proposal already scheduled ({proposal_id})"
                    )],
                },
            };
        }

        let event = DomainEvent::MainTokenPolicyUpdateScheduled {
            proposal_id,
            effective_epoch,
            next: next.clone(),
        };
        let mut preview_state = self.state.clone();
        if let Err(err) = preview_state.apply_domain_event(&event, self.state.time) {
            return DomainEvent::ActionRejected {
                action_id,
                reason: RejectReason::RuleDenied {
                    notes: vec![format!("update main token policy rejected: {err:?}")],
                },
            };
        }
        event
    }

    fn evaluate_redeem_power_action(
        &self,
        node_id: &str,
        target_agent_id: &str,
        redeem_credits: u64,
        nonce: u64,
        signed: Option<(&str, &str)>,
    ) -> DomainEvent {
        if self
            .state
            .reward_signature_governance_policy
            .require_redeem_signature
            && signed.is_none()
        {
            return self.power_redeem_rejected(
                node_id,
                target_agent_id,
                redeem_credits,
                nonce,
                "redeem signature is required by governance policy".to_string(),
            );
        }
        if let Some((signer_node_id, signature)) = signed {
            if signer_node_id.trim().is_empty() {
                return self.power_redeem_rejected(
                    node_id,
                    target_agent_id,
                    redeem_credits,
                    nonce,
                    "signer_node_id cannot be empty".to_string(),
                );
            }
            if signature.trim().is_empty() {
                return self.power_redeem_rejected(
                    node_id,
                    target_agent_id,
                    redeem_credits,
                    nonce,
                    "redeem signature cannot be empty".to_string(),
                );
            }
            if self
                .state
                .reward_signature_governance_policy
                .require_redeem_signer_match_node_id
                && signer_node_id != node_id
            {
                return self.power_redeem_rejected(
                    node_id,
                    target_agent_id,
                    redeem_credits,
                    nonce,
                    format!(
                        "redeem signer_node_id must match node_id by governance policy: signer={} node={}",
                        signer_node_id, node_id
                    ),
                );
            }
            if let Err(reason) = self.verify_redeem_power_signature(
                node_id,
                target_agent_id,
                redeem_credits,
                nonce,
                signer_node_id,
                signature,
            ) {
                return self.power_redeem_rejected(
                    node_id,
                    target_agent_id,
                    redeem_credits,
                    nonce,
                    format!("redeem signature verification failed: {reason}"),
                );
            }
        }

        if node_id.trim().is_empty() {
            return self.power_redeem_rejected(
                node_id,
                target_agent_id,
                redeem_credits,
                nonce,
                "node_id cannot be empty".to_string(),
            );
        }
        if !self.state.node_identity_bindings.contains_key(node_id) {
            return self.power_redeem_rejected(
                node_id,
                target_agent_id,
                redeem_credits,
                nonce,
                format!("node identity not bound: {node_id}"),
            );
        }
        if !self.state.agents.contains_key(target_agent_id) {
            return self.power_redeem_rejected(
                node_id,
                target_agent_id,
                redeem_credits,
                nonce,
                format!("target agent not found: {target_agent_id}"),
            );
        }
        if redeem_credits == 0 {
            return self.power_redeem_rejected(
                node_id,
                target_agent_id,
                redeem_credits,
                nonce,
                "redeem_credits must be > 0".to_string(),
            );
        }
        if nonce == 0 {
            return self.power_redeem_rejected(
                node_id,
                target_agent_id,
                redeem_credits,
                nonce,
                "nonce must be > 0".to_string(),
            );
        }
        if let Some(last_nonce) = self.state.node_redeem_nonces.get(node_id) {
            if nonce <= *last_nonce {
                return self.power_redeem_rejected(
                    node_id,
                    target_agent_id,
                    redeem_credits,
                    nonce,
                    format!(
                        "nonce replay detected: nonce={} last_nonce={}",
                        nonce, last_nonce
                    ),
                );
            }
        }
        let credits_per_power_unit = self.state.reward_asset_config.credits_per_power_unit;
        if credits_per_power_unit == 0 {
            return self.power_redeem_rejected(
                node_id,
                target_agent_id,
                redeem_credits,
                nonce,
                "credits_per_power_unit must be positive".to_string(),
            );
        }
        let granted_power_units_u64 = redeem_credits / credits_per_power_unit;
        if granted_power_units_u64 == 0 {
            return self.power_redeem_rejected(
                node_id,
                target_agent_id,
                redeem_credits,
                nonce,
                format!(
                    "redeem credits below minimum conversion: credits={} per_unit={}",
                    redeem_credits, credits_per_power_unit
                ),
            );
        }
        if granted_power_units_u64 > i64::MAX as u64 {
            return self.power_redeem_rejected(
                node_id,
                target_agent_id,
                redeem_credits,
                nonce,
                "granted power units overflow".to_string(),
            );
        }
        let granted_power_units = granted_power_units_u64 as i64;
        let min_redeem_power_unit = self.state.reward_asset_config.min_redeem_power_unit;
        if min_redeem_power_unit <= 0 {
            return self.power_redeem_rejected(
                node_id,
                target_agent_id,
                redeem_credits,
                nonce,
                "min_redeem_power_unit must be positive".to_string(),
            );
        }
        if granted_power_units < min_redeem_power_unit {
            return self.power_redeem_rejected(
                node_id,
                target_agent_id,
                redeem_credits,
                nonce,
                format!(
                    "granted power below minimum unit: granted={} min={}",
                    granted_power_units, min_redeem_power_unit
                ),
            );
        }
        let max_redeem_power_per_epoch = self.state.reward_asset_config.max_redeem_power_per_epoch;
        if max_redeem_power_per_epoch <= 0 {
            return self.power_redeem_rejected(
                node_id,
                target_agent_id,
                redeem_credits,
                nonce,
                "max_redeem_power_per_epoch must be positive".to_string(),
            );
        }
        let next_redeemed = match self
            .state
            .protocol_power_reserve
            .redeemed_power_units
            .checked_add(granted_power_units)
        {
            Some(value) => value,
            None => {
                return self.power_redeem_rejected(
                    node_id,
                    target_agent_id,
                    redeem_credits,
                    nonce,
                    "redeemed_power_units overflow".to_string(),
                );
            }
        };
        if next_redeemed > max_redeem_power_per_epoch {
            return self.power_redeem_rejected(
                node_id,
                target_agent_id,
                redeem_credits,
                nonce,
                format!(
                    "epoch redeem cap exceeded: next={} cap={}",
                    next_redeemed, max_redeem_power_per_epoch
                ),
            );
        }
        let available_credits = self.node_power_credit_balance(node_id);
        if available_credits < redeem_credits {
            return self.power_redeem_rejected(
                node_id,
                target_agent_id,
                redeem_credits,
                nonce,
                format!(
                    "insufficient power credits: balance={} requested={}",
                    available_credits, redeem_credits
                ),
            );
        }
        if self.state.protocol_power_reserve.available_power_units < granted_power_units {
            return self.power_redeem_rejected(
                node_id,
                target_agent_id,
                redeem_credits,
                nonce,
                format!(
                    "insufficient protocol power reserve: available={} requested={}",
                    self.state.protocol_power_reserve.available_power_units, granted_power_units
                ),
            );
        }

        DomainEvent::PowerRedeemed {
            node_id: node_id.to_string(),
            target_agent_id: target_agent_id.to_string(),
            burned_credits: redeem_credits,
            granted_power_units,
            reserve_remaining: self.state.protocol_power_reserve.available_power_units
                - granted_power_units,
            nonce,
        }
    }

    fn power_redeem_rejected(
        &self,
        node_id: &str,
        target_agent_id: &str,
        redeem_credits: u64,
        nonce: u64,
        reason: String,
    ) -> DomainEvent {
        DomainEvent::PowerRedeemRejected {
            node_id: node_id.to_string(),
            target_agent_id: target_agent_id.to_string(),
            redeem_credits,
            nonce,
            reason,
        }
    }

    pub(super) fn append_event(
        &mut self,
        body: WorldEventBody,
        caused_by: Option<CausedBy>,
    ) -> Result<WorldEventId, WorldError> {
        self.apply_event_body(&body, self.state.time)?;
        let event_id = self.allocate_next_event_id();
        self.journal.append(WorldEvent {
            id: event_id,
            time: self.state.time,
            caused_by,
            body,
        });
        self.enforce_journal_event_limit();
        self.record_tick_consensus_for_tick(self.state.time)?;
        Ok(event_id)
    }

    fn apply_event_body(
        &mut self,
        body: &WorldEventBody,
        time: WorldTime,
    ) -> Result<(), WorldError> {
        match body {
            WorldEventBody::Domain(event) => {
                self.state.apply_domain_event(event, time)?;
                self.state.route_domain_event(event);
                if let super::super::DomainEvent::ModuleInstalled {
                    instance_id,
                    module_id,
                    module_version,
                    active,
                    ..
                } = event
                {
                    let schedule_key = if instance_id.trim().is_empty() {
                        module_id.as_str()
                    } else {
                        instance_id.as_str()
                    };
                    if *active {
                        self.sync_tick_schedule_for_instance(
                            schedule_key,
                            module_id.as_str(),
                            module_version.as_str(),
                            time,
                        )?;
                    } else {
                        self.remove_tick_schedule(schedule_key);
                    }
                }
                if let super::super::DomainEvent::ModuleUpgraded {
                    instance_id,
                    module_id,
                    to_module_version,
                    active,
                    ..
                } = event
                {
                    if *active {
                        self.sync_tick_schedule_for_instance(
                            instance_id.as_str(),
                            module_id.as_str(),
                            to_module_version.as_str(),
                            time,
                        )?;
                    } else {
                        self.remove_tick_schedule(instance_id.as_str());
                    }
                }
            }
            WorldEventBody::EffectQueued(intent) => {
                self.push_pending_effect_bounded(intent.clone());
            }
            WorldEventBody::ReceiptAppended(receipt) => {
                let mut removed = false;
                if self.inflight_effects.remove(&receipt.intent_id).is_some() {
                    removed = true;
                }
                let before = self.pending_effects.len();
                self.pending_effects
                    .retain(|intent| intent.intent_id != receipt.intent_id);
                if before != self.pending_effects.len() {
                    removed = true;
                }
                if !removed {
                    return Err(WorldError::ReceiptUnknownIntent {
                        intent_id: receipt.intent_id.clone(),
                    });
                }
            }
            WorldEventBody::PolicyDecisionRecorded(_) => {}
            WorldEventBody::RuleDecisionRecorded(_) => {}
            WorldEventBody::ActionOverridden(_) => {}
            WorldEventBody::Governance(event) => {
                self.apply_governance_event(event)?;
            }
            WorldEventBody::ModuleEvent(event) => {
                self.apply_module_event(event, time)?;
            }
            WorldEventBody::ModuleCallFailed(_) => {}
            WorldEventBody::ModuleEmitted(_) => {}
            WorldEventBody::ModuleStateUpdated(update) => {
                self.state
                    .module_states
                    .insert(update.module_id.clone(), update.state.clone());
            }
            WorldEventBody::ModuleRuntimeCharged(charge) => {
                self.apply_module_runtime_charge_event(charge, time)?;
            }
            WorldEventBody::SnapshotCreated(_) => {}
            WorldEventBody::ManifestUpdated(update) => {
                self.manifest = update.manifest.clone();
            }
            WorldEventBody::RollbackApplied(_) => {}
        }
        self.state.time = time;
        Ok(())
    }
}

fn distribute_main_token_bridge_budget(
    total_budget: u64,
    settlements: &[NodeSettlement],
) -> (u64, Vec<MainTokenNodePointsBridgeDistribution>) {
    if total_budget == 0 || settlements.is_empty() {
        return (0, Vec::new());
    }
    let total_points = settlements
        .iter()
        .map(|settlement| settlement.awarded_points)
        .sum::<u64>();
    if total_points == 0 {
        return (0, Vec::new());
    }

    let mut distributions = Vec::with_capacity(settlements.len());
    let mut distributed = 0_u64;
    for settlement in settlements {
        let amount_u128 = u128::from(total_budget)
            .saturating_mul(u128::from(settlement.awarded_points))
            / u128::from(total_points);
        let amount = u64::try_from(amount_u128).unwrap_or(u64::MAX);
        distributed = distributed.saturating_add(amount);
        distributions.push(MainTokenNodePointsBridgeDistribution {
            node_id: settlement.node_id.clone(),
            account_id: settlement.node_id.clone(),
            amount,
        });
    }

    let mut remainder = total_budget.saturating_sub(distributed);
    distributions.sort_by(|left, right| {
        let left_points = settlements
            .iter()
            .find(|settlement| settlement.node_id == left.node_id)
            .map(|settlement| settlement.awarded_points)
            .unwrap_or(0);
        let right_points = settlements
            .iter()
            .find(|settlement| settlement.node_id == right.node_id)
            .map(|settlement| settlement.awarded_points)
            .unwrap_or(0);
        right_points
            .cmp(&left_points)
            .then_with(|| left.node_id.cmp(&right.node_id))
    });
    let mut index = 0_usize;
    while remainder > 0 && !distributions.is_empty() {
        let target = index % distributions.len();
        distributions[target].amount = distributions[target].amount.saturating_add(1);
        remainder -= 1;
        index = index.saturating_add(1);
    }

    distributions.retain(|item| item.amount > 0);
    distributions.sort_by(|left, right| left.node_id.cmp(&right.node_id));
    (total_budget, distributions)
}
