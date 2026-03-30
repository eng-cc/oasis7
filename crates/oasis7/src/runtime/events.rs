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

mod domain_event;

pub use self::domain_event::{CausedBy, DomainEvent, RejectReason};

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
