//! Action and domain event types.

use crate::geometry::GeoPos;
use crate::models::{BodyKernelView, BodySlotType};
use crate::simulator::{ModuleInstallTarget, ResourceKind};
use oasis7_wasm_abi::{
    FactoryModuleSpec, FactoryProfileV1, MaterialProfileV1, MaterialStack, ModuleManifest,
    ProductProfileV1, ProductValidationDecision, RecipeExecutionPlan, RecipeProfileV1,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::BTreeMap;

use super::gameplay_state::WarParticipantOutcome;
use super::governance::GovernanceFinalityCertificate;
use super::main_token::{
    MainTokenConfig, MainTokenGenesisAllocationBucketState, MainTokenGenesisAllocationPlan,
    MainTokenNodePointsBridgeDistribution, MainTokenTreasuryDistribution,
    RestrictedStarterClaimRefundSink,
};
use super::node_points::EpochSettlementReport;
use super::reward_asset::NodeRewardMintRecord;
use super::types::{ActionId, MaterialLedgerId, ProposalId, WorldTime};

fn default_world_material_ledger() -> MaterialLedgerId {
    MaterialLedgerId::world()
}

fn default_module_action_fee_kind() -> ResourceKind {
    ResourceKind::Electricity
}

fn default_material_transit_priority() -> MaterialTransitPriority {
    MaterialTransitPriority::Standard
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IndustryStage {
    Bootstrap,
    ScaleOut,
    Governance,
}

impl Default for IndustryStage {
    fn default() -> Self {
        Self::Bootstrap
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialMarketQuote {
    pub kind: String,
    pub requested_amount: i64,
    pub local_available_amount: i64,
    pub world_available_amount: i64,
    pub local_deficit_amount: i64,
    pub transit_loss_bps: i64,
    pub governance_tax_bps: u16,
    pub effective_cost_index_ppm: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MainTokenFeeKind {
    GasBaseFee,
    SlashPenalty,
    ModuleFee,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialTransitPriority {
    Urgent,
    Standard,
}

impl Default for MaterialTransitPriority {
    fn default() -> Self {
        Self::Standard
    }
}

/// An envelope wrapping an action with its ID.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionEnvelope {
    pub id: ActionId,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Observation {
    pub time: WorldTime,
    pub agent_id: String,
    pub pos: GeoPos,
    pub visibility_range_cm: i64,
    pub visible_agents: Vec<ObservedAgent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservedAgent {
    pub agent_id: String,
    pub pos: GeoPos,
    pub distance_cm: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleSourcePackage {
    pub manifest_path: String,
    #[serde(default)]
    pub files: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ModuleProfileChanges {
    #[serde(default)]
    pub product_profiles: Vec<ProductProfileV1>,
    #[serde(default)]
    pub recipe_profiles: Vec<RecipeProfileV1>,
    #[serde(default)]
    pub factory_profiles: Vec<FactoryProfileV1>,
}

impl ModuleProfileChanges {
    pub fn is_empty(&self) -> bool {
        self.product_profiles.is_empty()
            && self.recipe_profiles.is_empty()
            && self.factory_profiles.is_empty()
    }
}

/// Actions that can be submitted to the world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Action {
    RegisterAgent {
        agent_id: String,
        pos: GeoPos,
    },
    MoveAgent {
        agent_id: String,
        to: GeoPos,
    },
    QueryObservation {
        agent_id: String,
    },
    EmitObservation {
        observation: Observation,
    },
    BodyAction {
        agent_id: String,
        kind: String,
        payload: JsonValue,
    },
    EmitBodyAttributes {
        agent_id: String,
        view: BodyKernelView,
        reason: String,
    },
    ExpandBodyInterface {
        agent_id: String,
        interface_module_item_id: String,
    },
    DeployModuleArtifact {
        publisher_agent_id: String,
        wasm_hash: String,
        wasm_bytes: Vec<u8>,
    },
    CompileModuleArtifactFromSource {
        publisher_agent_id: String,
        module_id: String,
        source_package: ModuleSourcePackage,
    },
    InstallModuleFromArtifact {
        installer_agent_id: String,
        manifest: ModuleManifest,
        activate: bool,
    },
    InstallModuleFromArtifactWithFinality {
        installer_agent_id: String,
        manifest: ModuleManifest,
        activate: bool,
        finality_certificate: GovernanceFinalityCertificate,
    },
    InstallModuleToTargetFromArtifact {
        installer_agent_id: String,
        manifest: ModuleManifest,
        activate: bool,
        #[serde(default)]
        install_target: ModuleInstallTarget,
    },
    InstallModuleToTargetFromArtifactWithFinality {
        installer_agent_id: String,
        manifest: ModuleManifest,
        activate: bool,
        #[serde(default)]
        install_target: ModuleInstallTarget,
        finality_certificate: GovernanceFinalityCertificate,
    },
    UpgradeModuleFromArtifact {
        upgrader_agent_id: String,
        instance_id: String,
        from_module_version: String,
        manifest: ModuleManifest,
        activate: bool,
    },
    UpgradeModuleFromArtifactWithFinality {
        upgrader_agent_id: String,
        instance_id: String,
        from_module_version: String,
        manifest: ModuleManifest,
        activate: bool,
        finality_certificate: GovernanceFinalityCertificate,
    },
    ModuleReleaseSubmit {
        requester_agent_id: String,
        manifest: ModuleManifest,
        activate: bool,
        #[serde(default)]
        install_target: ModuleInstallTarget,
        #[serde(default)]
        required_roles: Vec<String>,
        #[serde(default)]
        profile_changes: ModuleProfileChanges,
    },
    ModuleReleaseShadow {
        operator_agent_id: String,
        request_id: u64,
    },
    ModuleReleaseSubmitAttestation {
        operator_agent_id: String,
        request_id: u64,
        signer_node_id: String,
        platform: String,
        build_manifest_hash: String,
        source_hash: String,
        wasm_hash: String,
        proof_cid: String,
        #[serde(default)]
        builder_image_digest: String,
        #[serde(default)]
        container_platform: String,
        #[serde(default)]
        canonicalizer_version: String,
    },
    ModuleReleaseApproveRole {
        approver_agent_id: String,
        request_id: u64,
        role: String,
    },
    ModuleReleaseBindRoles {
        operator_agent_id: String,
        target_agent_id: String,
        #[serde(default)]
        roles: Vec<String>,
    },
    ModuleReleaseReject {
        rejector_agent_id: String,
        request_id: u64,
        reason: String,
    },
    ModuleReleaseApply {
        operator_agent_id: String,
        request_id: u64,
    },
    ModuleReleaseApplyWithFinality {
        operator_agent_id: String,
        request_id: u64,
        finality_certificate: GovernanceFinalityCertificate,
    },
    RollbackModuleInstance {
        operator_agent_id: String,
        instance_id: String,
        target_module_version: String,
    },
    RollbackModuleInstanceWithFinality {
        operator_agent_id: String,
        instance_id: String,
        target_module_version: String,
        finality_certificate: GovernanceFinalityCertificate,
    },
    ListModuleArtifactForSale {
        seller_agent_id: String,
        wasm_hash: String,
        price_kind: ResourceKind,
        price_amount: i64,
    },
    BuyModuleArtifact {
        buyer_agent_id: String,
        wasm_hash: String,
    },
    DelistModuleArtifact {
        seller_agent_id: String,
        wasm_hash: String,
    },
    DestroyModuleArtifact {
        owner_agent_id: String,
        wasm_hash: String,
        reason: String,
    },
    PlaceModuleArtifactBid {
        bidder_agent_id: String,
        wasm_hash: String,
        price_kind: ResourceKind,
        price_amount: i64,
    },
    CancelModuleArtifactBid {
        bidder_agent_id: String,
        wasm_hash: String,
        bid_order_id: u64,
    },
    TransferResource {
        from_agent_id: String,
        to_agent_id: String,
        kind: ResourceKind,
        amount: i64,
    },
    CollectData {
        collector_agent_id: String,
        electricity_cost: i64,
        data_amount: i64,
    },
    GrantDataAccess {
        owner_agent_id: String,
        grantee_agent_id: String,
    },
    RevokeDataAccess {
        owner_agent_id: String,
        grantee_agent_id: String,
    },
    RedeemPower {
        node_id: String,
        target_agent_id: String,
        redeem_credits: u64,
        nonce: u64,
    },
    RedeemPowerSigned {
        node_id: String,
        target_agent_id: String,
        redeem_credits: u64,
        nonce: u64,
        signer_node_id: String,
        signature: String,
    },
    ApplyNodePointsSettlementSigned {
        report: EpochSettlementReport,
        signer_node_id: String,
        mint_records: Vec<NodeRewardMintRecord>,
    },
    InitializeMainTokenGenesis {
        allocations: Vec<MainTokenGenesisAllocationPlan>,
    },
    ClaimMainTokenVesting {
        bucket_id: String,
        beneficiary: String,
        nonce: u64,
    },
    TransferMainToken {
        from_account_id: String,
        to_account_id: String,
        amount: u64,
        nonce: u64,
    },
    ApplyMainTokenEpochIssuance {
        epoch_index: u64,
        actual_stake_ratio_bps: u32,
    },
    SettleMainTokenFee {
        fee_kind: MainTokenFeeKind,
        amount: u64,
    },
    UpdateMainTokenPolicy {
        proposal_id: ProposalId,
        next: MainTokenConfig,
    },
    DistributeMainTokenTreasury {
        proposal_id: ProposalId,
        distribution_id: String,
        bucket_id: String,
        distributions: Vec<MainTokenTreasuryDistribution>,
    },
    IssueRestrictedStarterClaimGrant {
        issuer_account_id: String,
        beneficiary_account_id: String,
        amount: u64,
        issuance_reason: String,
        expires_at_epoch: u64,
    },
    RevokeRestrictedStarterClaimGrant {
        issuer_account_id: String,
        beneficiary_account_id: String,
        revoke_reason: String,
    },
    ClaimAgent {
        claimer_agent_id: String,
        target_agent_id: String,
    },
    ReleaseAgentClaim {
        claimer_agent_id: String,
        target_agent_id: String,
    },
    TransferMaterial {
        requester_agent_id: String,
        from_ledger: MaterialLedgerId,
        to_ledger: MaterialLedgerId,
        kind: String,
        amount: i64,
        distance_km: i64,
        #[serde(default)]
        priority: Option<MaterialTransitPriority>,
    },
    FormAlliance {
        proposer_agent_id: String,
        alliance_id: String,
        #[serde(default)]
        members: Vec<String>,
        charter: String,
    },
    JoinAlliance {
        operator_agent_id: String,
        alliance_id: String,
        member_agent_id: String,
    },
    LeaveAlliance {
        operator_agent_id: String,
        alliance_id: String,
        member_agent_id: String,
    },
    DissolveAlliance {
        operator_agent_id: String,
        alliance_id: String,
        reason: String,
    },
    DeclareWar {
        initiator_agent_id: String,
        war_id: String,
        aggressor_alliance_id: String,
        defender_alliance_id: String,
        objective: String,
        intensity: u32,
    },
    OpenGovernanceProposal {
        proposer_agent_id: String,
        proposal_key: String,
        title: String,
        description: String,
        #[serde(default)]
        options: Vec<String>,
        voting_window_ticks: u64,
        quorum_weight: u64,
        pass_threshold_bps: u16,
    },
    CastGovernanceVote {
        voter_agent_id: String,
        proposal_key: String,
        option: String,
        weight: u32,
    },
    ResolveCrisis {
        resolver_agent_id: String,
        crisis_id: String,
        strategy: String,
        success: bool,
    },
    GrantMetaProgress {
        operator_agent_id: String,
        target_agent_id: String,
        track: String,
        points: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        achievement_id: Option<String>,
    },
    UpdateGameplayPolicy {
        operator_agent_id: String,
        electricity_tax_bps: u16,
        data_tax_bps: u16,
        power_trade_fee_bps: u16,
        max_open_contracts_per_agent: u16,
        #[serde(default)]
        blocked_agents: Vec<String>,
        #[serde(default)]
        forbidden_location_ids: Vec<String>,
    },
    UpdateRestrictedStarterClaimAdminRegistry {
        controller_account_id: String,
        #[serde(default)]
        next_admin_account_ids: Vec<String>,
    },
    OpenEconomicContract {
        creator_agent_id: String,
        contract_id: String,
        counterparty_agent_id: String,
        settlement_kind: ResourceKind,
        settlement_amount: i64,
        reputation_stake: i64,
        expires_at: WorldTime,
        description: String,
    },
    AcceptEconomicContract {
        accepter_agent_id: String,
        contract_id: String,
    },
    SettleEconomicContract {
        operator_agent_id: String,
        contract_id: String,
        success: bool,
        notes: String,
    },
    EmitResourceTransfer {
        from_agent_id: String,
        to_agent_id: String,
        kind: ResourceKind,
        amount: i64,
    },
    BuildFactory {
        builder_agent_id: String,
        site_id: String,
        spec: FactoryModuleSpec,
    },
    BuildFactoryWithModule {
        builder_agent_id: String,
        site_id: String,
        module_id: String,
        spec: FactoryModuleSpec,
    },
    MaintainFactory {
        operator_agent_id: String,
        factory_id: String,
        parts: i64,
    },
    RecycleFactory {
        operator_agent_id: String,
        factory_id: String,
    },
    ScheduleRecipe {
        requester_agent_id: String,
        factory_id: String,
        recipe_id: String,
        plan: RecipeExecutionPlan,
    },
    ScheduleRecipeWithModule {
        requester_agent_id: String,
        factory_id: String,
        recipe_id: String,
        module_id: String,
        desired_batches: u32,
        deterministic_seed: u64,
    },
    ValidateProduct {
        requester_agent_id: String,
        module_id: String,
        stack: MaterialStack,
        decision: ProductValidationDecision,
    },
    ValidateProductWithModule {
        requester_agent_id: String,
        module_id: String,
        stack: MaterialStack,
        deterministic_seed: u64,
    },
    GovernMaterialProfile {
        operator_agent_id: String,
        proposal_id: ProposalId,
        profile: MaterialProfileV1,
    },
    GovernProductProfile {
        operator_agent_id: String,
        proposal_id: ProposalId,
        profile: ProductProfileV1,
    },
    GovernRecipeProfile {
        operator_agent_id: String,
        proposal_id: ProposalId,
        profile: RecipeProfileV1,
    },
    GovernFactoryProfile {
        operator_agent_id: String,
        proposal_id: ProposalId,
        profile: FactoryProfileV1,
    },
}

impl Action {
    pub fn actor_id(&self) -> Option<&str> {
        match self {
            Action::RegisterAgent { agent_id, .. }
            | Action::MoveAgent { agent_id, .. }
            | Action::QueryObservation { agent_id, .. }
            | Action::BodyAction { agent_id, .. }
            | Action::EmitBodyAttributes { agent_id, .. }
            | Action::ExpandBodyInterface { agent_id, .. } => Some(agent_id.as_str()),
            Action::EmitObservation { observation } => Some(observation.agent_id.as_str()),
            Action::DeployModuleArtifact {
                publisher_agent_id, ..
            }
            | Action::CompileModuleArtifactFromSource {
                publisher_agent_id, ..
            } => Some(publisher_agent_id.as_str()),
            Action::InstallModuleFromArtifact {
                installer_agent_id, ..
            }
            | Action::InstallModuleFromArtifactWithFinality {
                installer_agent_id, ..
            }
            | Action::InstallModuleToTargetFromArtifact {
                installer_agent_id, ..
            } => Some(installer_agent_id.as_str()),
            Action::InstallModuleToTargetFromArtifactWithFinality {
                installer_agent_id, ..
            } => Some(installer_agent_id.as_str()),
            Action::UpgradeModuleFromArtifact {
                upgrader_agent_id, ..
            } => Some(upgrader_agent_id.as_str()),
            Action::UpgradeModuleFromArtifactWithFinality {
                upgrader_agent_id, ..
            } => Some(upgrader_agent_id.as_str()),
            Action::ModuleReleaseSubmit {
                requester_agent_id, ..
            } => Some(requester_agent_id.as_str()),
            Action::ModuleReleaseShadow {
                operator_agent_id, ..
            }
            | Action::ModuleReleaseSubmitAttestation {
                operator_agent_id, ..
            }
            | Action::ModuleReleaseBindRoles {
                operator_agent_id, ..
            }
            | Action::ModuleReleaseApply {
                operator_agent_id, ..
            }
            | Action::ModuleReleaseApplyWithFinality {
                operator_agent_id, ..
            }
            | Action::RollbackModuleInstance {
                operator_agent_id, ..
            }
            | Action::RollbackModuleInstanceWithFinality {
                operator_agent_id, ..
            } => Some(operator_agent_id.as_str()),
            Action::ModuleReleaseApproveRole {
                approver_agent_id, ..
            } => Some(approver_agent_id.as_str()),
            Action::ModuleReleaseReject {
                rejector_agent_id, ..
            } => Some(rejector_agent_id.as_str()),
            Action::ListModuleArtifactForSale {
                seller_agent_id, ..
            }
            | Action::DelistModuleArtifact {
                seller_agent_id, ..
            } => Some(seller_agent_id.as_str()),
            Action::BuyModuleArtifact { buyer_agent_id, .. } => Some(buyer_agent_id.as_str()),
            Action::DestroyModuleArtifact { owner_agent_id, .. } => Some(owner_agent_id.as_str()),
            Action::PlaceModuleArtifactBid {
                bidder_agent_id, ..
            }
            | Action::CancelModuleArtifactBid {
                bidder_agent_id, ..
            } => Some(bidder_agent_id.as_str()),
            Action::TransferResource { from_agent_id, .. }
            | Action::EmitResourceTransfer { from_agent_id, .. } => Some(from_agent_id.as_str()),
            Action::CollectData {
                collector_agent_id, ..
            } => Some(collector_agent_id.as_str()),
            Action::GrantDataAccess { owner_agent_id, .. }
            | Action::RevokeDataAccess { owner_agent_id, .. } => Some(owner_agent_id.as_str()),
            Action::RedeemPower { node_id, .. } | Action::RedeemPowerSigned { node_id, .. } => {
                Some(node_id.as_str())
            }
            Action::ApplyNodePointsSettlementSigned { signer_node_id, .. } => {
                Some(signer_node_id.as_str())
            }
            Action::InitializeMainTokenGenesis { .. } => None,
            Action::ClaimMainTokenVesting { beneficiary, .. } => Some(beneficiary.as_str()),
            Action::TransferMainToken {
                from_account_id, ..
            } => Some(from_account_id.as_str()),
            Action::ApplyMainTokenEpochIssuance { .. }
            | Action::SettleMainTokenFee { .. }
            | Action::UpdateMainTokenPolicy { .. }
            | Action::DistributeMainTokenTreasury { .. } => None,
            Action::IssueRestrictedStarterClaimGrant {
                issuer_account_id, ..
            }
            | Action::RevokeRestrictedStarterClaimGrant {
                issuer_account_id, ..
            } => Some(issuer_account_id.as_str()),
            Action::ClaimAgent {
                claimer_agent_id, ..
            }
            | Action::ReleaseAgentClaim {
                claimer_agent_id, ..
            } => Some(claimer_agent_id.as_str()),
            Action::TransferMaterial {
                requester_agent_id, ..
            }
            | Action::ScheduleRecipe {
                requester_agent_id, ..
            }
            | Action::ScheduleRecipeWithModule {
                requester_agent_id, ..
            }
            | Action::ValidateProduct {
                requester_agent_id, ..
            }
            | Action::ValidateProductWithModule {
                requester_agent_id, ..
            } => Some(requester_agent_id.as_str()),
            Action::FormAlliance {
                proposer_agent_id, ..
            }
            | Action::OpenGovernanceProposal {
                proposer_agent_id, ..
            } => Some(proposer_agent_id.as_str()),
            Action::JoinAlliance {
                operator_agent_id, ..
            }
            | Action::LeaveAlliance {
                operator_agent_id, ..
            }
            | Action::DissolveAlliance {
                operator_agent_id, ..
            }
            | Action::GrantMetaProgress {
                operator_agent_id, ..
            }
            | Action::UpdateGameplayPolicy {
                operator_agent_id, ..
            }
            | Action::SettleEconomicContract {
                operator_agent_id, ..
            }
            | Action::MaintainFactory {
                operator_agent_id, ..
            }
            | Action::RecycleFactory {
                operator_agent_id, ..
            }
            | Action::GovernMaterialProfile {
                operator_agent_id, ..
            }
            | Action::GovernProductProfile {
                operator_agent_id, ..
            }
            | Action::GovernRecipeProfile {
                operator_agent_id, ..
            }
            | Action::GovernFactoryProfile {
                operator_agent_id, ..
            } => Some(operator_agent_id.as_str()),
            Action::UpdateRestrictedStarterClaimAdminRegistry {
                controller_account_id, ..
            } => Some(controller_account_id.as_str()),
            Action::DeclareWar {
                initiator_agent_id, ..
            } => Some(initiator_agent_id.as_str()),
            Action::CastGovernanceVote { voter_agent_id, .. } => Some(voter_agent_id.as_str()),
            Action::ResolveCrisis {
                resolver_agent_id, ..
            } => Some(resolver_agent_id.as_str()),
            Action::OpenEconomicContract {
                creator_agent_id, ..
            } => Some(creator_agent_id.as_str()),
            Action::AcceptEconomicContract {
                accepter_agent_id, ..
            } => Some(accepter_agent_id.as_str()),
            Action::BuildFactory {
                builder_agent_id, ..
            }
            | Action::BuildFactoryWithModule {
                builder_agent_id, ..
            } => Some(builder_agent_id.as_str()),
        }
    }
}

/// Domain events that describe state changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum DomainEvent {
    AgentRegistered {
        agent_id: String,
        pos: GeoPos,
    },
    AgentMoved {
        agent_id: String,
        from: GeoPos,
        to: GeoPos,
    },
    ActionAccepted {
        action_id: ActionId,
        action_kind: String,
        actor_id: String,
        #[serde(default)]
        eta_ticks: u64,
        #[serde(default)]
        notes: Vec<String>,
    },
    ActionRejected {
        action_id: ActionId,
        reason: RejectReason,
    },
    Observation {
        observation: Observation,
    },
    BodyAttributesUpdated {
        agent_id: String,
        view: BodyKernelView,
        reason: String,
    },
    BodyAttributesRejected {
        agent_id: String,
        reason: String,
    },
    BodyInterfaceExpanded {
        agent_id: String,
        slot_capacity: u16,
        expansion_level: u16,
        consumed_item_id: String,
        new_slot_id: String,
        slot_type: BodySlotType,
    },
    BodyInterfaceExpandRejected {
        agent_id: String,
        consumed_item_id: String,
        reason: String,
    },
    ModuleArtifactDeployed {
        publisher_agent_id: String,
        wasm_hash: String,
        bytes_len: u64,
        #[serde(default = "default_module_action_fee_kind")]
        fee_kind: ResourceKind,
        #[serde(default)]
        fee_amount: i64,
    },
    ModuleInstalled {
        installer_agent_id: String,
        #[serde(default)]
        instance_id: String,
        module_id: String,
        module_version: String,
        #[serde(default)]
        wasm_hash: String,
        #[serde(default)]
        install_target: ModuleInstallTarget,
        active: bool,
        proposal_id: ProposalId,
        manifest_hash: String,
        #[serde(default = "default_module_action_fee_kind")]
        fee_kind: ResourceKind,
        #[serde(default)]
        fee_amount: i64,
    },
    ModuleUpgraded {
        upgrader_agent_id: String,
        instance_id: String,
        module_id: String,
        from_module_version: String,
        to_module_version: String,
        wasm_hash: String,
        #[serde(default)]
        install_target: ModuleInstallTarget,
        active: bool,
        proposal_id: ProposalId,
        manifest_hash: String,
        #[serde(default = "default_module_action_fee_kind")]
        fee_kind: ResourceKind,
        #[serde(default)]
        fee_amount: i64,
    },
    ModuleReleaseRequested {
        request_id: u64,
        requester_agent_id: String,
        manifest: ModuleManifest,
        activate: bool,
        #[serde(default)]
        install_target: ModuleInstallTarget,
        #[serde(default)]
        required_roles: Vec<String>,
        #[serde(default)]
        profile_changes: ModuleProfileChanges,
    },
    ModuleReleaseShadowed {
        request_id: u64,
        operator_agent_id: String,
        manifest_hash: String,
    },
    ModuleReleaseAttested {
        request_id: u64,
        operator_agent_id: String,
        signer_node_id: String,
        platform: String,
        build_manifest_hash: String,
        source_hash: String,
        wasm_hash: String,
        proof_cid: String,
        #[serde(default)]
        builder_image_digest: String,
        #[serde(default)]
        container_platform: String,
        #[serde(default)]
        canonicalizer_version: String,
    },
    ModuleReleaseRoleApproved {
        request_id: u64,
        approver_agent_id: String,
        role: String,
    },
    ModuleReleaseRolesBound {
        operator_agent_id: String,
        target_agent_id: String,
        #[serde(default)]
        roles: Vec<String>,
    },
    ModuleReleaseRejected {
        request_id: u64,
        rejector_agent_id: String,
        reason: String,
    },
    ModuleReleaseApplied {
        request_id: u64,
        operator_agent_id: String,
        installer_agent_id: String,
        instance_id: String,
        module_id: String,
        module_version: String,
        proposal_id: ProposalId,
        manifest_hash: String,
    },
    ModuleRollbackApplied {
        operator_agent_id: String,
        instance_id: String,
        module_id: String,
        from_module_version: String,
        to_module_version: String,
        wasm_hash: String,
        #[serde(default)]
        install_target: ModuleInstallTarget,
        active: bool,
        proposal_id: ProposalId,
        manifest_hash: String,
        #[serde(default = "default_module_action_fee_kind")]
        fee_kind: ResourceKind,
        #[serde(default)]
        fee_amount: i64,
    },
    ModuleArtifactListed {
        seller_agent_id: String,
        wasm_hash: String,
        price_kind: ResourceKind,
        price_amount: i64,
        #[serde(default)]
        order_id: u64,
        #[serde(default = "default_module_action_fee_kind")]
        fee_kind: ResourceKind,
        #[serde(default)]
        fee_amount: i64,
    },
    ModuleArtifactDelisted {
        seller_agent_id: String,
        wasm_hash: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        order_id: Option<u64>,
        #[serde(default = "default_module_action_fee_kind")]
        fee_kind: ResourceKind,
        #[serde(default)]
        fee_amount: i64,
    },
    ModuleArtifactDestroyed {
        owner_agent_id: String,
        wasm_hash: String,
        reason: String,
        #[serde(default = "default_module_action_fee_kind")]
        fee_kind: ResourceKind,
        #[serde(default)]
        fee_amount: i64,
    },
    ModuleArtifactBidPlaced {
        bidder_agent_id: String,
        wasm_hash: String,
        order_id: u64,
        price_kind: ResourceKind,
        price_amount: i64,
    },
    ModuleArtifactBidCancelled {
        bidder_agent_id: String,
        wasm_hash: String,
        order_id: u64,
        reason: String,
    },
    ModuleArtifactSaleCompleted {
        buyer_agent_id: String,
        seller_agent_id: String,
        wasm_hash: String,
        price_kind: ResourceKind,
        price_amount: i64,
        #[serde(default)]
        sale_id: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        listing_order_id: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        bid_order_id: Option<u64>,
    },
    ResourceTransferred {
        from_agent_id: String,
        to_agent_id: String,
        kind: ResourceKind,
        amount: i64,
    },
    DataCollected {
        collector_agent_id: String,
        electricity_cost: i64,
        data_amount: i64,
    },
    DataAccessGranted {
        owner_agent_id: String,
        grantee_agent_id: String,
    },
    DataAccessRevoked {
        owner_agent_id: String,
        grantee_agent_id: String,
    },
    PowerRedeemed {
        node_id: String,
        target_agent_id: String,
        burned_credits: u64,
        granted_power_units: i64,
        reserve_remaining: i64,
        nonce: u64,
    },
    PowerRedeemRejected {
        node_id: String,
        target_agent_id: String,
        redeem_credits: u64,
        nonce: u64,
        reason: String,
    },
    NodePointsSettlementApplied {
        report: EpochSettlementReport,
        signer_node_id: String,
        settlement_hash: String,
        minted_records: Vec<NodeRewardMintRecord>,
        #[serde(default)]
        main_token_bridge_total_amount: u64,
        #[serde(default)]
        main_token_bridge_distributions: Vec<MainTokenNodePointsBridgeDistribution>,
    },
    MainTokenGenesisInitialized {
        total_supply: u64,
        allocations: Vec<MainTokenGenesisAllocationBucketState>,
    },
    MainTokenVestingClaimed {
        bucket_id: String,
        beneficiary: String,
        amount: u64,
        nonce: u64,
    },
    MainTokenTransferred {
        from_account_id: String,
        to_account_id: String,
        amount: u64,
        nonce: u64,
    },
    MainTokenEpochIssued {
        epoch_index: u64,
        inflation_rate_bps: u32,
        issued_amount: u64,
        staking_reward_amount: u64,
        node_service_reward_amount: u64,
        ecosystem_pool_amount: u64,
        security_reserve_amount: u64,
    },
    MainTokenFeeSettled {
        fee_kind: MainTokenFeeKind,
        amount: u64,
        burn_amount: u64,
        treasury_amount: u64,
    },
    MainTokenPolicyUpdateScheduled {
        proposal_id: ProposalId,
        effective_epoch: u64,
        next: MainTokenConfig,
    },
    MainTokenTreasuryDistributed {
        proposal_id: ProposalId,
        distribution_id: String,
        bucket_id: String,
        total_amount: u64,
        distributions: Vec<MainTokenTreasuryDistribution>,
    },
    RestrictedStarterClaimGrantIssued {
        issuer_id: String,
        beneficiary_account_id: String,
        source_treasury_bucket_id: String,
        amount: u64,
        issuance_reason: String,
        spend_scope: String,
        issued_at_epoch: u64,
        expires_at_epoch: u64,
    },
    RestrictedStarterClaimGrantExpired {
        beneficiary_account_id: String,
        issuer_id: String,
        issuance_reason: String,
        spend_scope: String,
        source_treasury_bucket_id: String,
        issued_amount: u64,
        expired_amount: u64,
        issued_at_epoch: u64,
        expired_at_epoch: u64,
        configured_expires_at_epoch: u64,
    },
    RestrictedStarterClaimGrantRevoked {
        beneficiary_account_id: String,
        issuer_id: String,
        issuance_reason: String,
        spend_scope: String,
        source_treasury_bucket_id: String,
        issued_amount: u64,
        revoked_amount: u64,
        issued_at_epoch: u64,
        revoked_at_epoch: u64,
        configured_expires_at_epoch: u64,
        revoke_reason: String,
    },
    AgentClaimed {
        claimer_agent_id: String,
        target_agent_id: String,
        reputation_tier: u8,
        slot_index: u8,
        activation_fee_amount: u64,
        activation_fee_burn_amount: u64,
        activation_fee_treasury_amount: u64,
        claim_bond_amount: u64,
        #[serde(default)]
        upfront_restricted_spent_amount: u64,
        #[serde(default)]
        upfront_liquid_spent_amount: u64,
        #[serde(default)]
        claim_bond_locked_restricted_amount: u64,
        #[serde(default)]
        claim_bond_locked_liquid_amount: u64,
        upkeep_per_epoch: u64,
        claimed_at_epoch: u64,
        upkeep_paid_through_epoch: u64,
        release_cooldown_epochs: u64,
        grace_epochs: u64,
        idle_warning_epochs: u64,
        forced_idle_reclaim_epochs: u64,
        forced_reclaim_penalty_bps: u16,
    },
    AgentClaimReleaseRequested {
        claimer_agent_id: String,
        target_agent_id: String,
        requested_at_epoch: u64,
        ready_at_epoch: u64,
    },
    AgentClaimUpkeepSettled {
        claimer_agent_id: String,
        target_agent_id: String,
        settled_at_epoch: u64,
        charged_epochs: u64,
        amount: u64,
        #[serde(default)]
        restricted_spent_amount: u64,
        #[serde(default)]
        liquid_spent_amount: u64,
        upkeep_paid_through_epoch: u64,
    },
    AgentClaimEnteredGrace {
        claimer_agent_id: String,
        target_agent_id: String,
        delinquent_since_epoch: u64,
        grace_deadline_epoch: u64,
        upkeep_arrears_amount: u64,
    },
    AgentClaimIdleWarning {
        claimer_agent_id: String,
        target_agent_id: String,
        warning_emitted_at_epoch: u64,
        forced_reclaim_at_epoch: u64,
    },
    AgentClaimReleased {
        claimer_agent_id: String,
        target_agent_id: String,
        released_at_epoch: u64,
        refunded_bond_amount: u64,
        #[serde(default)]
        refunded_bond_restricted_amount: u64,
        #[serde(default)]
        refunded_bond_liquid_amount: u64,
        #[serde(default)]
        refunded_bond_restricted_sink: RestrictedStarterClaimRefundSink,
        #[serde(default)]
        refunded_bond_restricted_sink_bucket_id: String,
    },
    AgentClaimReclaimed {
        claimer_agent_id: String,
        target_agent_id: String,
        reclaimed_at_epoch: u64,
        reason: String,
        upkeep_arrears_amount: u64,
        collected_upkeep_amount: u64,
        penalty_amount: u64,
        refunded_bond_amount: u64,
        #[serde(default)]
        refunded_bond_restricted_amount: u64,
        #[serde(default)]
        refunded_bond_liquid_amount: u64,
        #[serde(default)]
        refunded_bond_restricted_sink: RestrictedStarterClaimRefundSink,
        #[serde(default)]
        refunded_bond_restricted_sink_bucket_id: String,
    },
    MaterialTransferred {
        requester_agent_id: String,
        from_ledger: MaterialLedgerId,
        to_ledger: MaterialLedgerId,
        kind: String,
        amount: i64,
        distance_km: i64,
        #[serde(default = "default_material_transit_priority")]
        priority: MaterialTransitPriority,
    },
    MaterialTransitStarted {
        job_id: ActionId,
        requester_agent_id: String,
        from_ledger: MaterialLedgerId,
        to_ledger: MaterialLedgerId,
        kind: String,
        amount: i64,
        distance_km: i64,
        loss_bps: i64,
        ready_at: WorldTime,
        #[serde(default = "default_material_transit_priority")]
        priority: MaterialTransitPriority,
    },
    MaterialTransitCompleted {
        job_id: ActionId,
        requester_agent_id: String,
        from_ledger: MaterialLedgerId,
        to_ledger: MaterialLedgerId,
        kind: String,
        sent_amount: i64,
        received_amount: i64,
        loss_amount: i64,
        distance_km: i64,
        #[serde(default = "default_material_transit_priority")]
        priority: MaterialTransitPriority,
    },
    FactoryBuildStarted {
        job_id: ActionId,
        builder_agent_id: String,
        site_id: String,
        spec: FactoryModuleSpec,
        #[serde(default = "default_world_material_ledger")]
        consume_ledger: MaterialLedgerId,
        ready_at: WorldTime,
    },
    FactoryBuilt {
        job_id: ActionId,
        builder_agent_id: String,
        site_id: String,
        spec: FactoryModuleSpec,
    },
    FactoryDurabilityChanged {
        factory_id: String,
        previous_durability_ppm: i64,
        durability_ppm: i64,
        reason: String,
    },
    FactoryMaintained {
        operator_agent_id: String,
        factory_id: String,
        #[serde(default = "default_world_material_ledger")]
        consume_ledger: MaterialLedgerId,
        consumed_parts: i64,
        durability_ppm: i64,
    },
    FactoryRecycled {
        operator_agent_id: String,
        factory_id: String,
        #[serde(default = "default_world_material_ledger")]
        recycle_ledger: MaterialLedgerId,
        recovered: Vec<MaterialStack>,
        durability_ppm: i64,
    },
    RecipeStarted {
        job_id: ActionId,
        requester_agent_id: String,
        factory_id: String,
        recipe_id: String,
        accepted_batches: u32,
        consume: Vec<MaterialStack>,
        produce: Vec<MaterialStack>,
        byproducts: Vec<MaterialStack>,
        power_required: i64,
        duration_ticks: u32,
        #[serde(default = "default_world_material_ledger")]
        consume_ledger: MaterialLedgerId,
        #[serde(default = "default_world_material_ledger")]
        output_ledger: MaterialLedgerId,
        #[serde(default)]
        bottleneck_tags: Vec<String>,
        #[serde(default)]
        market_quotes: Vec<MaterialMarketQuote>,
        ready_at: WorldTime,
    },
    RecipeCompleted {
        job_id: ActionId,
        requester_agent_id: String,
        factory_id: String,
        recipe_id: String,
        accepted_batches: u32,
        produce: Vec<MaterialStack>,
        byproducts: Vec<MaterialStack>,
        #[serde(default = "default_world_material_ledger")]
        output_ledger: MaterialLedgerId,
        #[serde(default)]
        bottleneck_tags: Vec<String>,
    },
    FactoryProductionBlocked {
        action_id: ActionId,
        requester_agent_id: String,
        factory_id: String,
        recipe_id: String,
        blocker_kind: String,
        blocker_detail: String,
    },
    FactoryProductionResumed {
        job_id: ActionId,
        requester_agent_id: String,
        factory_id: String,
        recipe_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        previous_blocked_at: Option<WorldTime>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        previous_blocker_kind: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        previous_blocker_detail: Option<String>,
    },
    GameplayPolicyUpdated {
        operator_agent_id: String,
        electricity_tax_bps: u16,
        data_tax_bps: u16,
        power_trade_fee_bps: u16,
        max_open_contracts_per_agent: u16,
        blocked_agents: Vec<String>,
        forbidden_location_ids: Vec<String>,
    },
    EconomicContractOpened {
        creator_agent_id: String,
        contract_id: String,
        counterparty_agent_id: String,
        settlement_kind: ResourceKind,
        settlement_amount: i64,
        reputation_stake: i64,
        expires_at: WorldTime,
        description: String,
    },
    EconomicContractAccepted {
        accepter_agent_id: String,
        contract_id: String,
    },
    EconomicContractSettled {
        operator_agent_id: String,
        contract_id: String,
        success: bool,
        transfer_amount: i64,
        tax_amount: i64,
        notes: String,
        creator_reputation_delta: i64,
        counterparty_reputation_delta: i64,
    },
    EconomicContractExpired {
        contract_id: String,
        creator_agent_id: String,
        counterparty_agent_id: String,
        creator_reputation_delta: i64,
        counterparty_reputation_delta: i64,
    },
    AllianceFormed {
        proposer_agent_id: String,
        alliance_id: String,
        members: Vec<String>,
        charter: String,
    },
    AllianceJoined {
        operator_agent_id: String,
        alliance_id: String,
        member_agent_id: String,
    },
    AllianceLeft {
        operator_agent_id: String,
        alliance_id: String,
        member_agent_id: String,
    },
    AllianceDissolved {
        operator_agent_id: String,
        alliance_id: String,
        reason: String,
        #[serde(default)]
        former_members: Vec<String>,
    },
    WarDeclared {
        initiator_agent_id: String,
        war_id: String,
        aggressor_alliance_id: String,
        defender_alliance_id: String,
        objective: String,
        intensity: u32,
        #[serde(default)]
        mobilization_electricity_cost: i64,
        #[serde(default)]
        mobilization_data_cost: i64,
    },
    WarConcluded {
        war_id: String,
        winner_alliance_id: String,
        #[serde(default)]
        loser_alliance_id: String,
        aggressor_score: i64,
        defender_score: i64,
        summary: String,
        #[serde(default)]
        participant_outcomes: Vec<WarParticipantOutcome>,
    },
    GovernanceProposalOpened {
        proposer_agent_id: String,
        proposal_key: String,
        title: String,
        description: String,
        options: Vec<String>,
        voting_window_ticks: u64,
        closes_at: WorldTime,
        quorum_weight: u64,
        pass_threshold_bps: u16,
    },
    GovernanceVoteCast {
        voter_agent_id: String,
        proposal_key: String,
        option: String,
        weight: u32,
    },
    GovernanceProposalFinalized {
        proposal_key: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        winning_option: Option<String>,
        winning_weight: u64,
        total_weight: u64,
        passed: bool,
    },
    CrisisSpawned {
        crisis_id: String,
        kind: String,
        severity: u32,
        expires_at: WorldTime,
    },
    CrisisResolved {
        resolver_agent_id: String,
        crisis_id: String,
        strategy: String,
        success: bool,
        impact: i64,
    },
    CrisisTimedOut {
        crisis_id: String,
        penalty_impact: i64,
    },
    MetaProgressGranted {
        operator_agent_id: String,
        target_agent_id: String,
        track: String,
        points: i64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        achievement_id: Option<String>,
    },
    ProductValidated {
        requester_agent_id: String,
        module_id: String,
        stack: MaterialStack,
        stack_limit: u32,
        tradable: bool,
        quality_levels: Vec<String>,
        notes: Vec<String>,
    },
    MaterialProfileGoverned {
        operator_agent_id: String,
        proposal_id: ProposalId,
        profile: MaterialProfileV1,
    },
    ProductProfileGoverned {
        operator_agent_id: String,
        proposal_id: ProposalId,
        profile: ProductProfileV1,
    },
    RecipeProfileGoverned {
        operator_agent_id: String,
        proposal_id: ProposalId,
        profile: RecipeProfileV1,
    },
    FactoryProfileGoverned {
        operator_agent_id: String,
        proposal_id: ProposalId,
        profile: FactoryProfileV1,
    },
}

impl DomainEvent {
    pub fn agent_id(&self) -> Option<&str> {
        match self {
            DomainEvent::AgentRegistered { agent_id, .. } => Some(agent_id.as_str()),
            DomainEvent::AgentMoved { agent_id, .. } => Some(agent_id.as_str()),
            DomainEvent::ActionAccepted { actor_id, .. } => Some(actor_id.as_str()),
            DomainEvent::Observation { observation } => Some(observation.agent_id.as_str()),
            DomainEvent::BodyAttributesUpdated { agent_id, .. } => Some(agent_id.as_str()),
            DomainEvent::BodyAttributesRejected { agent_id, .. } => Some(agent_id.as_str()),
            DomainEvent::BodyInterfaceExpanded { agent_id, .. } => Some(agent_id.as_str()),
            DomainEvent::BodyInterfaceExpandRejected { agent_id, .. } => Some(agent_id.as_str()),
            DomainEvent::ModuleArtifactDeployed {
                publisher_agent_id, ..
            } => Some(publisher_agent_id.as_str()),
            DomainEvent::ModuleInstalled {
                installer_agent_id, ..
            } => Some(installer_agent_id.as_str()),
            DomainEvent::ModuleUpgraded {
                upgrader_agent_id, ..
            } => Some(upgrader_agent_id.as_str()),
            DomainEvent::ModuleReleaseRequested {
                requester_agent_id, ..
            } => Some(requester_agent_id.as_str()),
            DomainEvent::ModuleReleaseShadowed {
                operator_agent_id, ..
            } => Some(operator_agent_id.as_str()),
            DomainEvent::ModuleReleaseAttested {
                operator_agent_id, ..
            } => Some(operator_agent_id.as_str()),
            DomainEvent::ModuleReleaseRoleApproved {
                approver_agent_id, ..
            } => Some(approver_agent_id.as_str()),
            DomainEvent::ModuleReleaseRolesBound {
                operator_agent_id, ..
            } => Some(operator_agent_id.as_str()),
            DomainEvent::ModuleReleaseRejected {
                rejector_agent_id, ..
            } => Some(rejector_agent_id.as_str()),
            DomainEvent::ModuleReleaseApplied {
                operator_agent_id, ..
            } => Some(operator_agent_id.as_str()),
            DomainEvent::ModuleRollbackApplied {
                operator_agent_id, ..
            } => Some(operator_agent_id.as_str()),
            DomainEvent::ModuleArtifactListed {
                seller_agent_id, ..
            } => Some(seller_agent_id.as_str()),
            DomainEvent::ModuleArtifactDelisted {
                seller_agent_id, ..
            } => Some(seller_agent_id.as_str()),
            DomainEvent::ModuleArtifactDestroyed { owner_agent_id, .. } => {
                Some(owner_agent_id.as_str())
            }
            DomainEvent::ModuleArtifactBidPlaced {
                bidder_agent_id, ..
            } => Some(bidder_agent_id.as_str()),
            DomainEvent::ModuleArtifactBidCancelled {
                bidder_agent_id, ..
            } => Some(bidder_agent_id.as_str()),
            DomainEvent::ModuleArtifactSaleCompleted { buyer_agent_id, .. } => {
                Some(buyer_agent_id.as_str())
            }
            DomainEvent::ActionRejected { .. } => None,
            DomainEvent::ResourceTransferred { from_agent_id, .. } => Some(from_agent_id.as_str()),
            DomainEvent::DataCollected {
                collector_agent_id, ..
            } => Some(collector_agent_id.as_str()),
            DomainEvent::DataAccessGranted { owner_agent_id, .. } => Some(owner_agent_id.as_str()),
            DomainEvent::DataAccessRevoked { owner_agent_id, .. } => Some(owner_agent_id.as_str()),
            DomainEvent::PowerRedeemed {
                target_agent_id, ..
            } => Some(target_agent_id.as_str()),
            DomainEvent::PowerRedeemRejected {
                target_agent_id, ..
            } => Some(target_agent_id.as_str()),
            DomainEvent::NodePointsSettlementApplied { .. } => None,
            DomainEvent::MainTokenGenesisInitialized { .. } => None,
            DomainEvent::MainTokenVestingClaimed { beneficiary, .. } => Some(beneficiary.as_str()),
            DomainEvent::MainTokenTransferred {
                from_account_id, ..
            } => Some(from_account_id.as_str()),
            DomainEvent::MainTokenEpochIssued { .. } => None,
            DomainEvent::MainTokenFeeSettled { .. } => None,
            DomainEvent::MainTokenPolicyUpdateScheduled { .. } => None,
            DomainEvent::MainTokenTreasuryDistributed { .. } => None,
            DomainEvent::RestrictedStarterClaimGrantIssued { issuer_id, .. }
            | DomainEvent::RestrictedStarterClaimGrantRevoked { issuer_id, .. } => {
                Some(issuer_id.as_str())
            }
            DomainEvent::RestrictedStarterClaimGrantExpired { .. } => None,
            DomainEvent::AgentClaimed {
                claimer_agent_id, ..
            }
            | DomainEvent::AgentClaimReleaseRequested {
                claimer_agent_id, ..
            }
            | DomainEvent::AgentClaimUpkeepSettled {
                claimer_agent_id, ..
            }
            | DomainEvent::AgentClaimEnteredGrace {
                claimer_agent_id, ..
            }
            | DomainEvent::AgentClaimIdleWarning {
                claimer_agent_id, ..
            }
            | DomainEvent::AgentClaimReleased {
                claimer_agent_id, ..
            }
            | DomainEvent::AgentClaimReclaimed {
                claimer_agent_id, ..
            } => Some(claimer_agent_id.as_str()),
            DomainEvent::MaterialTransferred {
                requester_agent_id, ..
            } => Some(requester_agent_id.as_str()),
            DomainEvent::MaterialTransitStarted {
                requester_agent_id, ..
            } => Some(requester_agent_id.as_str()),
            DomainEvent::MaterialTransitCompleted {
                requester_agent_id, ..
            } => Some(requester_agent_id.as_str()),
            DomainEvent::FactoryBuildStarted {
                builder_agent_id, ..
            } => Some(builder_agent_id.as_str()),
            DomainEvent::FactoryBuilt {
                builder_agent_id, ..
            } => Some(builder_agent_id.as_str()),
            DomainEvent::FactoryDurabilityChanged { .. } => None,
            DomainEvent::FactoryMaintained {
                operator_agent_id, ..
            } => Some(operator_agent_id.as_str()),
            DomainEvent::FactoryRecycled {
                operator_agent_id, ..
            } => Some(operator_agent_id.as_str()),
            DomainEvent::RecipeStarted {
                requester_agent_id, ..
            } => Some(requester_agent_id.as_str()),
            DomainEvent::RecipeCompleted {
                requester_agent_id, ..
            } => Some(requester_agent_id.as_str()),
            DomainEvent::FactoryProductionBlocked {
                requester_agent_id, ..
            } => Some(requester_agent_id.as_str()),
            DomainEvent::FactoryProductionResumed {
                requester_agent_id, ..
            } => Some(requester_agent_id.as_str()),
            DomainEvent::GameplayPolicyUpdated {
                operator_agent_id, ..
            } => Some(operator_agent_id.as_str()),
            DomainEvent::EconomicContractOpened {
                creator_agent_id, ..
            } => Some(creator_agent_id.as_str()),
            DomainEvent::EconomicContractAccepted {
                accepter_agent_id, ..
            } => Some(accepter_agent_id.as_str()),
            DomainEvent::EconomicContractSettled {
                operator_agent_id, ..
            } => Some(operator_agent_id.as_str()),
            DomainEvent::EconomicContractExpired {
                creator_agent_id, ..
            } => Some(creator_agent_id.as_str()),
            DomainEvent::AllianceFormed {
                proposer_agent_id, ..
            } => Some(proposer_agent_id.as_str()),
            DomainEvent::AllianceJoined {
                operator_agent_id, ..
            } => Some(operator_agent_id.as_str()),
            DomainEvent::AllianceLeft {
                operator_agent_id, ..
            } => Some(operator_agent_id.as_str()),
            DomainEvent::AllianceDissolved {
                operator_agent_id, ..
            } => Some(operator_agent_id.as_str()),
            DomainEvent::WarDeclared {
                initiator_agent_id, ..
            } => Some(initiator_agent_id.as_str()),
            DomainEvent::WarConcluded { .. } => None,
            DomainEvent::GovernanceProposalOpened {
                proposer_agent_id, ..
            } => Some(proposer_agent_id.as_str()),
            DomainEvent::GovernanceVoteCast { voter_agent_id, .. } => Some(voter_agent_id.as_str()),
            DomainEvent::GovernanceProposalFinalized { .. } => None,
            DomainEvent::CrisisSpawned { .. } => None,
            DomainEvent::CrisisResolved {
                resolver_agent_id, ..
            } => Some(resolver_agent_id.as_str()),
            DomainEvent::CrisisTimedOut { .. } => None,
            DomainEvent::MetaProgressGranted {
                target_agent_id, ..
            } => Some(target_agent_id.as_str()),
            DomainEvent::ProductValidated {
                requester_agent_id, ..
            } => Some(requester_agent_id.as_str()),
            DomainEvent::MaterialProfileGoverned {
                operator_agent_id, ..
            } => Some(operator_agent_id.as_str()),
            DomainEvent::ProductProfileGoverned {
                operator_agent_id, ..
            } => Some(operator_agent_id.as_str()),
            DomainEvent::RecipeProfileGoverned {
                operator_agent_id, ..
            } => Some(operator_agent_id.as_str()),
            DomainEvent::FactoryProfileGoverned {
                operator_agent_id, ..
            } => Some(operator_agent_id.as_str()),
        }
    }
}

/// Reasons why an action was rejected.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum RejectReason {
    AgentAlreadyExists {
        agent_id: String,
    },
    AgentNotFound {
        agent_id: String,
    },
    AgentsNotCoLocated {
        agent_id: String,
        other_agent_id: String,
    },
    InvalidAmount {
        amount: i64,
    },
    InsufficientResource {
        agent_id: String,
        kind: ResourceKind,
        requested: i64,
        available: i64,
    },
    InsufficientResources {
        deficits: BTreeMap<ResourceKind, i64>,
    },
    InsufficientMaterial {
        material_kind: String,
        requested: i64,
        available: i64,
    },
    MaterialTransferDistanceExceeded {
        distance_km: i64,
        max_distance_km: i64,
    },
    MaterialTransitCapacityExceeded {
        in_flight: usize,
        max_in_flight: usize,
    },
    FactoryNotFound {
        factory_id: String,
    },
    FactoryBusy {
        factory_id: String,
        active_jobs: usize,
        recipe_slots: u16,
    },
    RuleDenied {
        notes: Vec<String>,
    },
}

/// The cause of an event, for audit purposes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum CausedBy {
    Action(ActionId),
    Effect { intent_id: String },
}
