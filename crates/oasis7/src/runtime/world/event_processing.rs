use super::super::{
    main_token_bucket_unlocked_amount, util::hash_json, Action, ActionEnvelope, ActionId, CausedBy,
    CrisisStatus, DomainEvent, EconomicContractStatus, EpochSettlementReport, GovernanceEvent,
    GovernanceProposalStatus, MainTokenConfig, MainTokenFeeKind,
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
mod main_token;

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
            | Action::TopUpRestrictedStarterClaimLiveopsPool { .. }
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
            | Action::SubmitGovernanceValidatorAdmission { .. }
            | Action::ApproveGovernanceValidatorAdmission { .. }
            | Action::ActivateGovernanceValidatorAdmission { .. }
            | Action::RevokeGovernanceValidatorAdmission { .. }
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
