//! World state management.

use crate::models::AgentState;
use crate::simulator::{ModuleInstallTarget, ResourceKind};
use oasis7_wasm_abi::{
    FactoryModuleSpec, FactoryProfileV1, MaterialProfileV1, MaterialStack, ModuleManifest,
    ProductProfileV1, RecipeProfileV1,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use super::agent_cell::AgentCell;
use super::error::WorldError;
use super::events::ModuleProfileChanges;
use super::events::{DomainEvent, IndustryStage, MaterialMarketQuote, MaterialTransitPriority};
use super::gameplay_state::{
    AgentClaimState, AllianceState, CrisisState, CrisisStatus, EconomicContractState,
    EconomicContractStatus, GameplayPolicyState, GovernanceIdentityProfileState,
    GovernanceIdentityStatus, GovernanceProposalState, GovernanceProposalStatus,
    GovernanceVoteBallotState, GovernanceVoteState, GovernanceVoteWeightSnapshotState,
    MetaProgressState, WarParticipantOutcome, WarState,
    GOVERNANCE_IDENTITY_DEFAULT_MAX_VOTE_WEIGHT,
};
use super::governance::{GovernanceFinalitySignerRegistry, GovernanceMainTokenControllerRegistry};
use super::main_token::{
    main_token_bucket_unlocked_amount, MainTokenAccountBalance, MainTokenConfig,
    MainTokenEpochIssuanceRecord, MainTokenGenesisAllocationBucketState,
    MainTokenNodePointsBridgeDistribution, MainTokenNodePointsBridgeEpochRecord,
    MainTokenScheduledPolicyUpdate, MainTokenSupplyState, MainTokenTreasuryDistributionRecord,
    RestrictedStarterClaimGrantState, MAIN_TOKEN_TREASURY_BUCKET_NODE_SERVICE_REWARD,
};
use super::node_points::EpochSettlementReport;
use super::reward_asset::{
    reward_mint_signature_v1, verify_reward_mint_signature_v2, NodeAssetBalance,
    NodeRewardMintRecord, ProtocolPowerReserve, RewardAssetConfig, RewardSignatureGovernancePolicy,
    SystemOrderPoolBudget, REWARD_MINT_SIGNATURE_V1_PREFIX, REWARD_MINT_SIGNATURE_V2_PREFIX,
};
use super::types::{ActionId, MaterialLedgerId, ProposalId, WorldTime};
use super::util::{deserialize_btreemap_u64_keys, hash_json};

mod apply_domain_event_core;
mod apply_domain_event_gameplay;
mod apply_domain_event_governance_meta;
mod apply_domain_event_industry;
mod apply_domain_event_main_token;

fn default_world_material_ledger() -> MaterialLedgerId {
    MaterialLedgerId::world()
}

fn default_material_ledgers() -> BTreeMap<MaterialLedgerId, BTreeMap<String, i64>> {
    let mut ledgers = BTreeMap::new();
    ledgers.insert(MaterialLedgerId::world(), BTreeMap::new());
    ledgers
}

fn default_module_market_order_id() -> u64 {
    1
}

fn default_module_market_sale_id() -> u64 {
    1
}

fn default_next_module_instance_id() -> u64 {
    1
}

fn default_next_module_release_request_id() -> u64 {
    1
}

fn default_factory_durability_ppm() -> i64 {
    1_000_000
}

fn default_factory_production_state() -> FactoryProductionState {
    FactoryProductionState::default()
}

fn default_module_release_required_roles() -> Vec<String> {
    vec![
        "security".to_string(),
        "economy".to_string(),
        "runtime".to_string(),
    ]
}

const ALLIANCE_MIN_MEMBER_COUNT: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactoryProductionStatus {
    Idle,
    Running,
    Blocked,
}

impl Default for FactoryProductionStatus {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactoryProductionState {
    #[serde(default)]
    pub status: FactoryProductionStatus,
    #[serde(default)]
    pub active_jobs: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_job_id: Option<ActionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_recipe_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_started_at: Option<WorldTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_completed_at: Option<WorldTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_blocked_at: Option<WorldTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_resumed_at: Option<WorldTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_blocker_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_blocker_detail: Option<String>,
    #[serde(default)]
    pub completed_jobs: u64,
}

impl Default for FactoryProductionState {
    fn default() -> Self {
        Self {
            status: FactoryProductionStatus::Idle,
            active_jobs: 0,
            current_job_id: None,
            current_recipe_id: None,
            last_started_at: None,
            last_completed_at: None,
            last_blocked_at: None,
            last_resumed_at: None,
            current_blocker_kind: None,
            current_blocker_detail: None,
            completed_jobs: 0,
        }
    }
}

/// Persisted factory instance state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactoryState {
    pub factory_id: String,
    pub site_id: String,
    pub builder_agent_id: String,
    pub spec: FactoryModuleSpec,
    #[serde(default = "default_world_material_ledger")]
    pub input_ledger: MaterialLedgerId,
    #[serde(default = "default_world_material_ledger")]
    pub output_ledger: MaterialLedgerId,
    #[serde(default = "default_factory_durability_ppm")]
    pub durability_ppm: i64,
    #[serde(default = "default_factory_production_state")]
    pub production: FactoryProductionState,
    pub built_at: WorldTime,
}

/// In-flight factory construction tracked by job id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FactoryBuildJobState {
    pub job_id: ActionId,
    pub builder_agent_id: String,
    pub site_id: String,
    pub spec: FactoryModuleSpec,
    #[serde(default = "default_world_material_ledger")]
    pub consume_ledger: MaterialLedgerId,
    pub ready_at: WorldTime,
}

/// In-flight recipe execution tracked by job id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeJobState {
    pub job_id: ActionId,
    pub requester_agent_id: String,
    pub factory_id: String,
    pub recipe_id: String,
    pub accepted_batches: u32,
    pub consume: Vec<MaterialStack>,
    pub produce: Vec<MaterialStack>,
    pub byproducts: Vec<MaterialStack>,
    pub power_required: i64,
    pub duration_ticks: u32,
    #[serde(default = "default_world_material_ledger")]
    pub consume_ledger: MaterialLedgerId,
    #[serde(default = "default_world_material_ledger")]
    pub output_ledger: MaterialLedgerId,
    #[serde(default)]
    pub bottleneck_tags: Vec<String>,
    pub ready_at: WorldTime,
}

/// In-flight material transit tracked by job id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialTransitJobState {
    pub job_id: ActionId,
    pub requester_agent_id: String,
    pub from_ledger: MaterialLedgerId,
    pub to_ledger: MaterialLedgerId,
    pub kind: String,
    pub amount: i64,
    pub distance_km: i64,
    pub loss_bps: i64,
    #[serde(default)]
    pub priority: MaterialTransitPriority,
    pub ready_at: WorldTime,
}

/// Lightweight observability state for industry progression and market snapshots.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct IndustryProgressState {
    #[serde(default)]
    pub stage: IndustryStage,
    #[serde(default)]
    pub stage_updated_at: WorldTime,
    #[serde(default)]
    pub completed_recipe_jobs: u64,
    #[serde(default)]
    pub completed_material_transits: u64,
    #[serde(default)]
    pub latest_market_quotes: BTreeMap<String, MaterialMarketQuote>,
}

/// Active market listing for one module artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleArtifactListingState {
    #[serde(default)]
    pub order_id: u64,
    pub seller_agent_id: String,
    pub price_kind: ResourceKind,
    pub price_amount: i64,
    pub listed_at: WorldTime,
}

/// Active bid order for one module artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleArtifactBidState {
    pub order_id: u64,
    pub bidder_agent_id: String,
    pub price_kind: ResourceKind,
    pub price_amount: i64,
    pub bid_at: WorldTime,
}

/// Installed module instance tracked independently from global module_id activation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleInstanceState {
    pub instance_id: String,
    pub module_id: String,
    pub module_version: String,
    #[serde(default)]
    pub wasm_hash: String,
    pub owner_agent_id: String,
    #[serde(default)]
    pub install_target: ModuleInstallTarget,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub installed_at: WorldTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleReleaseRequestStatus {
    Requested,
    Shadowed,
    PartiallyApproved,
    Approved,
    Rejected,
    Applied,
}

impl Default for ModuleReleaseRequestStatus {
    fn default() -> Self {
        Self::Requested
    }
}

/// One rebuild attestation submitted for a module release request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleReleaseAttestationState {
    pub request_id: u64,
    pub signer_node_id: String,
    pub platform: String,
    pub submitted_by_agent_id: String,
    pub build_manifest_hash: String,
    pub source_hash: String,
    pub wasm_hash: String,
    pub proof_cid: String,
    #[serde(default)]
    pub builder_image_digest: String,
    #[serde(default)]
    pub container_platform: String,
    #[serde(default)]
    pub canonicalizer_version: String,
    #[serde(default)]
    pub submitted_at: WorldTime,
}

/// Module release request tracked through governance closure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleReleaseRequestState {
    pub request_id: u64,
    pub requester_agent_id: String,
    pub manifest: ModuleManifest,
    pub activate: bool,
    #[serde(default)]
    pub install_target: ModuleInstallTarget,
    #[serde(default)]
    pub profile_changes: ModuleProfileChanges,
    #[serde(default = "default_module_release_required_roles")]
    pub required_roles: Vec<String>,
    #[serde(default)]
    pub role_approvals: BTreeMap<String, String>,
    #[serde(default)]
    pub attestations: BTreeMap<String, ModuleReleaseAttestationState>,
    #[serde(default)]
    pub status: ModuleReleaseRequestStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_manifest_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_manifest_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_proposal_id: Option<ProposalId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejected_reason: Option<String>,
    #[serde(default)]
    pub created_at: WorldTime,
    #[serde(default)]
    pub updated_at: WorldTime,
}

/// Persistent mapping from module release request lifecycle to release manifest lifecycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleReleaseManifestMappingState {
    pub request_id: u64,
    pub release_id: String,
    pub module_id: String,
    #[serde(default)]
    pub attestation_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_wasm_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_source_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_build_manifest_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_builder_image_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_container_platform: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_canonicalizer_version: Option<String>,
    #[serde(default)]
    pub attestation_platforms: Vec<String>,
    #[serde(default)]
    pub attestation_proof_cids: Vec<String>,
    #[serde(default)]
    pub receipt_evidence_conflict: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_manifest_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_manifest_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applied_proposal_id: Option<ProposalId>,
    #[serde(default)]
    pub status: ModuleReleaseRequestStatus,
    #[serde(default)]
    pub created_at: WorldTime,
    #[serde(default)]
    pub updated_at: WorldTime,
}

/// The mutable state of the world.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldState {
    pub time: WorldTime,
    pub agents: BTreeMap<String, AgentCell>,
    #[serde(default)]
    pub resources: BTreeMap<ResourceKind, i64>,
    #[serde(default)]
    pub materials: BTreeMap<String, i64>,
    #[serde(default = "default_material_ledgers")]
    pub material_ledgers: BTreeMap<MaterialLedgerId, BTreeMap<String, i64>>,
    #[serde(default)]
    pub material_profiles: BTreeMap<String, MaterialProfileV1>,
    #[serde(default)]
    pub product_profiles: BTreeMap<String, ProductProfileV1>,
    #[serde(default)]
    pub recipe_profiles: BTreeMap<String, RecipeProfileV1>,
    #[serde(default)]
    pub factory_profiles: BTreeMap<String, FactoryProfileV1>,
    #[serde(default)]
    pub factories: BTreeMap<String, FactoryState>,
    #[serde(default, deserialize_with = "deserialize_btreemap_u64_keys")]
    pub pending_factory_builds: BTreeMap<ActionId, FactoryBuildJobState>,
    #[serde(default, deserialize_with = "deserialize_btreemap_u64_keys")]
    pub pending_recipe_jobs: BTreeMap<ActionId, RecipeJobState>,
    #[serde(default, deserialize_with = "deserialize_btreemap_u64_keys")]
    pub pending_material_transits: BTreeMap<ActionId, MaterialTransitJobState>,
    #[serde(default)]
    pub industry_progress: IndustryProgressState,
    #[serde(default)]
    pub alliances: BTreeMap<String, AllianceState>,
    #[serde(default)]
    pub gameplay_policy: GameplayPolicyState,
    #[serde(default)]
    pub data_access_permissions: BTreeMap<String, BTreeSet<String>>,
    #[serde(default)]
    pub economic_contracts: BTreeMap<String, EconomicContractState>,
    #[serde(default)]
    pub agent_claims: BTreeMap<String, AgentClaimState>,
    #[serde(default)]
    pub agent_claim_last_processed_epoch: u64,
    #[serde(default)]
    pub contract_pair_last_success_settled_at: BTreeMap<String, WorldTime>,
    #[serde(default)]
    pub reputation_reward_window_started_at: BTreeMap<String, WorldTime>,
    #[serde(default)]
    pub reputation_reward_window_accumulated: BTreeMap<String, i64>,
    #[serde(default)]
    pub reputation_scores: BTreeMap<String, i64>,
    #[serde(default)]
    pub wars: BTreeMap<String, WarState>,
    #[serde(default)]
    pub governance_votes: BTreeMap<String, GovernanceVoteState>,
    #[serde(default)]
    pub governance_proposals: BTreeMap<String, GovernanceProposalState>,
    #[serde(default)]
    pub governance_identity_profiles: BTreeMap<String, GovernanceIdentityProfileState>,
    #[serde(default)]
    pub crises: BTreeMap<String, CrisisState>,
    #[serde(default)]
    pub meta_progress: BTreeMap<String, MetaProgressState>,
    #[serde(default)]
    pub module_states: BTreeMap<String, Vec<u8>>,
    #[serde(default)]
    pub module_artifact_owners: BTreeMap<String, String>,
    #[serde(default)]
    pub module_artifact_listings: BTreeMap<String, ModuleArtifactListingState>,
    #[serde(default)]
    pub module_artifact_bids: BTreeMap<String, Vec<ModuleArtifactBidState>>,
    #[serde(default)]
    pub module_instances: BTreeMap<String, ModuleInstanceState>,
    #[serde(default, deserialize_with = "deserialize_btreemap_u64_keys")]
    pub module_release_requests: BTreeMap<u64, ModuleReleaseRequestState>,
    #[serde(default, deserialize_with = "deserialize_btreemap_u64_keys")]
    pub module_release_manifest_mappings: BTreeMap<u64, ModuleReleaseManifestMappingState>,
    #[serde(default = "default_next_module_release_request_id")]
    pub next_module_release_request_id: u64,
    #[serde(default)]
    pub module_release_role_bindings: BTreeMap<String, BTreeSet<String>>,
    #[serde(default)]
    pub installed_module_targets: BTreeMap<String, ModuleInstallTarget>,
    #[serde(default = "default_next_module_instance_id")]
    pub next_module_instance_id: u64,
    #[serde(default = "default_module_market_order_id")]
    pub next_module_market_order_id: u64,
    #[serde(default = "default_module_market_sale_id")]
    pub next_module_market_sale_id: u64,
    #[serde(default)]
    pub main_token_config: MainTokenConfig,
    #[serde(default)]
    pub main_token_supply: MainTokenSupplyState,
    #[serde(default)]
    pub main_token_balances: BTreeMap<String, MainTokenAccountBalance>,
    #[serde(default)]
    pub restricted_starter_claim_grants: BTreeMap<String, RestrictedStarterClaimGrantState>,
    #[serde(default)]
    pub main_token_genesis_buckets: BTreeMap<String, MainTokenGenesisAllocationBucketState>,
    #[serde(default, deserialize_with = "deserialize_btreemap_u64_keys")]
    pub main_token_epoch_issuance_records: BTreeMap<u64, MainTokenEpochIssuanceRecord>,
    #[serde(default)]
    pub main_token_treasury_balances: BTreeMap<String, u64>,
    #[serde(default)]
    pub main_token_claim_nonces: BTreeMap<String, u64>,
    #[serde(default)]
    pub main_token_transfer_nonces: BTreeMap<String, u64>,
    #[serde(default, deserialize_with = "deserialize_btreemap_u64_keys")]
    pub main_token_scheduled_policy_updates: BTreeMap<u64, MainTokenScheduledPolicyUpdate>,
    #[serde(default, deserialize_with = "deserialize_btreemap_u64_keys")]
    pub main_token_node_points_bridge_records: BTreeMap<u64, MainTokenNodePointsBridgeEpochRecord>,
    #[serde(default)]
    pub main_token_treasury_distribution_records:
        BTreeMap<String, MainTokenTreasuryDistributionRecord>,
    #[serde(default)]
    pub reward_asset_config: RewardAssetConfig,
    #[serde(default)]
    pub node_asset_balances: BTreeMap<String, NodeAssetBalance>,
    #[serde(default)]
    pub protocol_power_reserve: ProtocolPowerReserve,
    #[serde(default)]
    pub reward_mint_records: Vec<NodeRewardMintRecord>,
    #[serde(default)]
    pub node_redeem_nonces: BTreeMap<String, u64>,
    #[serde(default, deserialize_with = "deserialize_btreemap_u64_keys")]
    pub system_order_pool_budgets: BTreeMap<u64, SystemOrderPoolBudget>,
    #[serde(default)]
    pub node_identity_bindings: BTreeMap<String, String>,
    #[serde(default)]
    pub node_main_token_account_bindings: BTreeMap<String, String>,
    #[serde(default)]
    pub governance_finality_signer_registry: Option<GovernanceFinalitySignerRegistry>,
    #[serde(default)]
    pub governance_main_token_controller_registry: Option<GovernanceMainTokenControllerRegistry>,
    #[serde(default)]
    pub reward_signature_governance_policy: RewardSignatureGovernancePolicy,
}

impl Default for WorldState {
    fn default() -> Self {
        Self {
            time: 0,
            agents: BTreeMap::new(),
            resources: BTreeMap::new(),
            materials: BTreeMap::new(),
            material_ledgers: default_material_ledgers(),
            material_profiles: BTreeMap::new(),
            product_profiles: BTreeMap::new(),
            recipe_profiles: BTreeMap::new(),
            factory_profiles: BTreeMap::new(),
            factories: BTreeMap::new(),
            pending_factory_builds: BTreeMap::new(),
            pending_recipe_jobs: BTreeMap::new(),
            pending_material_transits: BTreeMap::new(),
            industry_progress: IndustryProgressState::default(),
            alliances: BTreeMap::new(),
            gameplay_policy: GameplayPolicyState::default(),
            data_access_permissions: BTreeMap::new(),
            economic_contracts: BTreeMap::new(),
            agent_claims: BTreeMap::new(),
            agent_claim_last_processed_epoch: 0,
            contract_pair_last_success_settled_at: BTreeMap::new(),
            reputation_reward_window_started_at: BTreeMap::new(),
            reputation_reward_window_accumulated: BTreeMap::new(),
            reputation_scores: BTreeMap::new(),
            wars: BTreeMap::new(),
            governance_votes: BTreeMap::new(),
            governance_proposals: BTreeMap::new(),
            governance_identity_profiles: BTreeMap::new(),
            crises: BTreeMap::new(),
            meta_progress: BTreeMap::new(),
            module_states: BTreeMap::new(),
            module_artifact_owners: BTreeMap::new(),
            module_artifact_listings: BTreeMap::new(),
            module_artifact_bids: BTreeMap::new(),
            module_instances: BTreeMap::new(),
            module_release_requests: BTreeMap::new(),
            module_release_manifest_mappings: BTreeMap::new(),
            next_module_release_request_id: default_next_module_release_request_id(),
            module_release_role_bindings: BTreeMap::new(),
            installed_module_targets: BTreeMap::new(),
            next_module_instance_id: default_next_module_instance_id(),
            next_module_market_order_id: default_module_market_order_id(),
            next_module_market_sale_id: default_module_market_sale_id(),
            main_token_config: MainTokenConfig::default(),
            main_token_supply: MainTokenSupplyState::default(),
            main_token_balances: BTreeMap::new(),
            restricted_starter_claim_grants: BTreeMap::new(),
            main_token_genesis_buckets: BTreeMap::new(),
            main_token_epoch_issuance_records: BTreeMap::new(),
            main_token_treasury_balances: BTreeMap::new(),
            main_token_claim_nonces: BTreeMap::new(),
            main_token_transfer_nonces: BTreeMap::new(),
            main_token_scheduled_policy_updates: BTreeMap::new(),
            main_token_node_points_bridge_records: BTreeMap::new(),
            main_token_treasury_distribution_records: BTreeMap::new(),
            reward_asset_config: RewardAssetConfig::default(),
            node_asset_balances: BTreeMap::new(),
            protocol_power_reserve: ProtocolPowerReserve::default(),
            reward_mint_records: Vec::new(),
            node_redeem_nonces: BTreeMap::new(),
            system_order_pool_budgets: BTreeMap::new(),
            node_identity_bindings: BTreeMap::new(),
            node_main_token_account_bindings: BTreeMap::new(),
            governance_finality_signer_registry: None,
            governance_main_token_controller_registry: None,
            reward_signature_governance_policy: RewardSignatureGovernancePolicy::default(),
        }
    }
}

impl WorldState {
    pub fn migrate_compat_material_ledgers(&mut self) {
        self.material_ledgers
            .entry(MaterialLedgerId::world())
            .or_default();

        let world_ledger = self
            .material_ledgers
            .get(&MaterialLedgerId::world())
            .cloned()
            .unwrap_or_default();
        if world_ledger.is_empty() && !self.materials.is_empty() {
            self.material_ledgers
                .insert(MaterialLedgerId::world(), self.materials.clone());
        }

        sync_compat_world_materials(&self.material_ledgers, &mut self.materials);
    }

    pub fn has_data_access_permission(
        &self,
        owner_agent_id: &str,
        accessor_agent_id: &str,
    ) -> bool {
        if owner_agent_id == accessor_agent_id {
            return true;
        }
        self.data_access_permissions
            .get(owner_agent_id)
            .is_some_and(|grantees| grantees.contains(accessor_agent_id))
    }

    pub fn economic_contract_pair_key(left_agent_id: &str, right_agent_id: &str) -> String {
        if left_agent_id <= right_agent_id {
            format!("{left_agent_id}|{right_agent_id}")
        } else {
            format!("{right_agent_id}|{left_agent_id}")
        }
    }

    pub fn economic_contract_pair_cooldown_ready_at(
        &self,
        left_agent_id: &str,
        right_agent_id: &str,
        cooldown_ticks: u64,
    ) -> Option<WorldTime> {
        let pair_key = Self::economic_contract_pair_key(left_agent_id, right_agent_id);
        self.contract_pair_last_success_settled_at
            .get(&pair_key)
            .map(|last| last.saturating_add(cooldown_ticks))
    }

    pub fn available_reputation_reward_budget(
        &self,
        agent_id: &str,
        now: WorldTime,
        window_ticks: u64,
        window_cap: i64,
    ) -> i64 {
        if window_cap <= 0 {
            return 0;
        }
        let in_window = self
            .reputation_reward_window_started_at
            .get(agent_id)
            .is_some_and(|window_started_at| now.saturating_sub(*window_started_at) < window_ticks);
        if !in_window {
            return window_cap;
        }
        let accumulated = self
            .reputation_reward_window_accumulated
            .get(agent_id)
            .copied()
            .unwrap_or(0)
            .max(0);
        window_cap.saturating_sub(accumulated).max(0)
    }

    pub fn record_successful_contract_pair_settlement(
        &mut self,
        left_agent_id: &str,
        right_agent_id: &str,
        now: WorldTime,
    ) {
        let pair_key = Self::economic_contract_pair_key(left_agent_id, right_agent_id);
        self.contract_pair_last_success_settled_at
            .insert(pair_key, now);
    }

    pub fn record_reputation_reward_window_gain(
        &mut self,
        agent_id: &str,
        reward_delta: i64,
        now: WorldTime,
        window_ticks: u64,
    ) {
        if reward_delta <= 0 {
            return;
        }
        let in_window = self
            .reputation_reward_window_started_at
            .get(agent_id)
            .is_some_and(|window_started_at| now.saturating_sub(*window_started_at) < window_ticks);
        if in_window {
            let current = self
                .reputation_reward_window_accumulated
                .get(agent_id)
                .copied()
                .unwrap_or(0);
            self.reputation_reward_window_accumulated
                .insert(agent_id.to_string(), current.saturating_add(reward_delta));
        } else {
            self.reputation_reward_window_started_at
                .insert(agent_id.to_string(), now);
            self.reputation_reward_window_accumulated
                .insert(agent_id.to_string(), reward_delta);
        }
    }

    pub fn set_reputation_score(
        &mut self,
        agent_id: &str,
        reputation_score: i64,
    ) -> Result<(), WorldError> {
        if !self.agents.contains_key(agent_id) {
            return Err(WorldError::AgentNotFound {
                agent_id: agent_id.to_string(),
            });
        }
        self.reputation_scores
            .insert(agent_id.to_string(), reputation_score);
        Ok(())
    }

    pub fn set_governance_identity_profile(
        &mut self,
        profile: GovernanceIdentityProfileState,
    ) -> Result<(), WorldError> {
        if !self.agents.contains_key(profile.agent_id.as_str()) {
            return Err(WorldError::AgentNotFound {
                agent_id: profile.agent_id.clone(),
            });
        }
        self.governance_identity_profiles
            .insert(profile.agent_id.clone(), profile);
        Ok(())
    }

    pub fn governance_identity_snapshot_for_agent(
        &self,
        agent_id: &str,
        snapshot_tick: WorldTime,
    ) -> GovernanceVoteWeightSnapshotState {
        let reputation_score = self.reputation_scores.get(agent_id).copied().unwrap_or(0);
        if let Some(profile) = self.governance_identity_profiles.get(agent_id) {
            let vote_weight_cap = self.governance_vote_weight_cap_from_profile(
                reputation_score,
                profile,
                snapshot_tick,
            );
            return GovernanceVoteWeightSnapshotState {
                agent_id: agent_id.to_string(),
                reputation_score,
                stake_locked: profile.stake_locked,
                status: profile.status,
                vote_weight_cap,
            };
        }
        GovernanceVoteWeightSnapshotState {
            agent_id: agent_id.to_string(),
            reputation_score,
            stake_locked: 0,
            status: GovernanceIdentityStatus::Active,
            vote_weight_cap: GOVERNANCE_IDENTITY_DEFAULT_MAX_VOTE_WEIGHT,
        }
    }

    pub fn governance_effective_vote_weight_for_agent(
        &self,
        proposal: &GovernanceProposalState,
        voter_agent_id: &str,
        requested_weight: u32,
    ) -> Result<u32, WorldError> {
        if proposal.vote_weight_snapshot.is_empty() {
            return Ok(requested_weight);
        }
        let Some(snapshot) = proposal.vote_weight_snapshot.get(voter_agent_id) else {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "voter is not in governance snapshot: proposal={} voter={}",
                    proposal.proposal_key, voter_agent_id
                ),
            });
        };
        if snapshot.status != GovernanceIdentityStatus::Active {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "governance identity is not active: proposal={} voter={} status={:?}",
                    proposal.proposal_key, voter_agent_id, snapshot.status
                ),
            });
        }
        if snapshot.vote_weight_cap == 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "governance vote weight cap is zero: proposal={} voter={}",
                    proposal.proposal_key, voter_agent_id
                ),
            });
        }
        if requested_weight > snapshot.vote_weight_cap {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "vote weight {} exceeds snapshot cap {} for voter {} in proposal {}",
                    requested_weight,
                    snapshot.vote_weight_cap,
                    voter_agent_id,
                    proposal.proposal_key
                ),
            });
        }
        Ok(requested_weight)
    }

    fn governance_vote_weight_cap_from_profile(
        &self,
        reputation_score: i64,
        profile: &GovernanceIdentityProfileState,
        snapshot_tick: WorldTime,
    ) -> u32 {
        if profile.status != GovernanceIdentityStatus::Active {
            return 0;
        }
        if snapshot_tick < profile.warmup_until_tick {
            return 0;
        }
        let stake_component = integer_sqrt_u64(profile.stake_locked);
        let reputation_component = reputation_score.max(0).saturating_div(10) as u64;
        let raw_weight = stake_component
            .saturating_add(reputation_component)
            .max(1)
            .min(u64::from(GOVERNANCE_IDENTITY_DEFAULT_MAX_VOTE_WEIGHT));
        raw_weight as u32
    }

    fn settle_module_action_fee(
        &mut self,
        agent_id: &str,
        fee_kind: ResourceKind,
        fee_amount: i64,
        now: WorldTime,
    ) -> Result<(), WorldError> {
        if fee_amount < 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!("module action fee must be >= 0, got {}", fee_amount),
            });
        }

        let cell = self
            .agents
            .get_mut(agent_id)
            .ok_or_else(|| WorldError::AgentNotFound {
                agent_id: agent_id.to_string(),
            })?;
        if fee_amount > 0 {
            cell.state
                .resources
                .remove(fee_kind, fee_amount)
                .map_err(|err| WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "module action fee debit failed: agent={} kind={:?} amount={} err={:?}",
                        agent_id, fee_kind, fee_amount, err
                    ),
                })?;
            let treasury = self.resources.entry(fee_kind).or_insert(0);
            *treasury = treasury.saturating_add(fee_amount);
        }
        cell.last_active = now;
        Ok(())
    }

    pub fn apply_domain_event(
        &mut self,
        event: &DomainEvent,
        now: WorldTime,
    ) -> Result<(), WorldError> {
        self.migrate_compat_material_ledgers();
        match event {
            DomainEvent::AgentRegistered { .. }
            | DomainEvent::AgentMoved { .. }
            | DomainEvent::ActionAccepted { .. }
            | DomainEvent::ActionRejected { .. }
            | DomainEvent::Observation { .. }
            | DomainEvent::BodyAttributesUpdated { .. }
            | DomainEvent::BodyAttributesRejected { .. }
            | DomainEvent::BodyInterfaceExpanded { .. }
            | DomainEvent::BodyInterfaceExpandRejected { .. }
            | DomainEvent::ModuleArtifactDeployed { .. }
            | DomainEvent::ModuleInstalled { .. }
            | DomainEvent::ModuleUpgraded { .. }
            | DomainEvent::ModuleReleaseRequested { .. }
            | DomainEvent::ModuleReleaseShadowed { .. }
            | DomainEvent::ModuleReleaseAttested { .. }
            | DomainEvent::ModuleReleaseRoleApproved { .. }
            | DomainEvent::ModuleReleaseRolesBound { .. }
            | DomainEvent::ModuleReleaseRejected { .. }
            | DomainEvent::ModuleReleaseApplied { .. }
            | DomainEvent::ModuleRollbackApplied { .. }
            | DomainEvent::ModuleArtifactListed { .. }
            | DomainEvent::ModuleArtifactDelisted { .. }
            | DomainEvent::ModuleArtifactDestroyed { .. }
            | DomainEvent::ModuleArtifactBidPlaced { .. }
            | DomainEvent::ModuleArtifactBidCancelled { .. }
            | DomainEvent::ModuleArtifactSaleCompleted { .. }
            | DomainEvent::ResourceTransferred { .. }
            | DomainEvent::DataCollected { .. }
            | DomainEvent::DataAccessGranted { .. }
            | DomainEvent::DataAccessRevoked { .. }
            | DomainEvent::PowerRedeemed { .. }
            | DomainEvent::PowerRedeemRejected { .. }
            | DomainEvent::NodePointsSettlementApplied { .. }
            | DomainEvent::MainTokenGenesisInitialized { .. }
            | DomainEvent::MainTokenVestingClaimed { .. }
            | DomainEvent::MainTokenTransferred { .. }
            | DomainEvent::MainTokenEpochIssued { .. }
            | DomainEvent::MainTokenFeeSettled { .. }
            | DomainEvent::MainTokenPolicyUpdateScheduled { .. }
            | DomainEvent::MainTokenTreasuryDistributed { .. }
            | DomainEvent::RestrictedStarterClaimGrantIssued { .. }
            | DomainEvent::RestrictedStarterClaimGrantExpired { .. }
            | DomainEvent::RestrictedStarterClaimGrantRevoked { .. }
            | DomainEvent::MaterialTransferred { .. }
            | DomainEvent::MaterialTransitStarted { .. }
            | DomainEvent::MaterialTransitCompleted { .. }
            | DomainEvent::FactoryBuildStarted { .. }
            | DomainEvent::FactoryBuilt { .. }
            | DomainEvent::FactoryDurabilityChanged { .. }
            | DomainEvent::FactoryMaintained { .. }
            | DomainEvent::FactoryRecycled { .. }
            | DomainEvent::RecipeStarted { .. }
            | DomainEvent::RecipeCompleted { .. }
            | DomainEvent::FactoryProductionBlocked { .. }
            | DomainEvent::FactoryProductionResumed { .. }
            | DomainEvent::MaterialProfileGoverned { .. }
            | DomainEvent::ProductProfileGoverned { .. }
            | DomainEvent::RecipeProfileGoverned { .. }
            | DomainEvent::FactoryProfileGoverned { .. } => {
                self.apply_domain_event_core(event, now)?
            }
            DomainEvent::GameplayPolicyUpdated { .. }
            | DomainEvent::EconomicContractOpened { .. }
            | DomainEvent::EconomicContractAccepted { .. }
            | DomainEvent::EconomicContractSettled { .. }
            | DomainEvent::EconomicContractExpired { .. }
            | DomainEvent::AgentClaimed { .. }
            | DomainEvent::AgentClaimReleaseRequested { .. }
            | DomainEvent::AgentClaimUpkeepSettled { .. }
            | DomainEvent::AgentClaimEnteredGrace { .. }
            | DomainEvent::AgentClaimIdleWarning { .. }
            | DomainEvent::AgentClaimReleased { .. }
            | DomainEvent::AgentClaimReclaimed { .. }
            | DomainEvent::AllianceFormed { .. }
            | DomainEvent::AllianceJoined { .. }
            | DomainEvent::AllianceLeft { .. }
            | DomainEvent::AllianceDissolved { .. }
            | DomainEvent::WarDeclared { .. }
            | DomainEvent::WarConcluded { .. } => self.apply_domain_event_gameplay(event, now)?,
            DomainEvent::GovernanceProposalOpened { .. }
            | DomainEvent::GovernanceVoteCast { .. }
            | DomainEvent::GovernanceProposalFinalized { .. }
            | DomainEvent::CrisisSpawned { .. }
            | DomainEvent::CrisisResolved { .. }
            | DomainEvent::CrisisTimedOut { .. }
            | DomainEvent::MetaProgressGranted { .. }
            | DomainEvent::ProductValidated { .. } => {
                self.apply_domain_event_governance_meta(event, now)?
            }
        }
        sync_compat_world_materials(&self.material_ledgers, &mut self.materials);
        Ok(())
    }

    pub fn route_domain_event(&mut self, event: &DomainEvent) {
        match event {
            DomainEvent::ResourceTransferred {
                from_agent_id,
                to_agent_id,
                ..
            } => {
                if let Some(cell) = self.agents.get_mut(from_agent_id) {
                    cell.mailbox.push_back(event.clone());
                }
                if from_agent_id != to_agent_id {
                    if let Some(cell) = self.agents.get_mut(to_agent_id) {
                        cell.mailbox.push_back(event.clone());
                    }
                }
            }
            DomainEvent::DataAccessGranted {
                owner_agent_id,
                grantee_agent_id,
            }
            | DomainEvent::DataAccessRevoked {
                owner_agent_id,
                grantee_agent_id,
            } => {
                if let Some(cell) = self.agents.get_mut(owner_agent_id) {
                    cell.mailbox.push_back(event.clone());
                }
                if owner_agent_id != grantee_agent_id {
                    if let Some(cell) = self.agents.get_mut(grantee_agent_id) {
                        cell.mailbox.push_back(event.clone());
                    }
                }
            }
            _ => {
                let Some(agent_id) = event.agent_id() else {
                    return;
                };
                if let Some(cell) = self.agents.get_mut(agent_id) {
                    cell.mailbox.push_back(event.clone());
                }
            }
        }
    }
}

fn unlock_meta_track_tiers(track: &str, track_points: i64, progress: &mut MetaProgressState) {
    const META_TIER_THRESHOLDS: [(&str, i64); 3] = [("bronze", 20), ("silver", 50), ("gold", 100)];
    let unlocked_tiers = progress
        .unlocked_tiers
        .entry(track.to_string())
        .or_default();
    for (tier, threshold) in META_TIER_THRESHOLDS {
        if track_points < threshold {
            continue;
        }
        if !unlocked_tiers.iter().any(|value| value == tier) {
            unlocked_tiers.push(tier.to_string());
        }
        let achievement_id = format!("tier.{track}.{tier}");
        if !progress
            .achievements
            .iter()
            .any(|value| value == &achievement_id)
        {
            progress.achievements.push(achievement_id);
        }
    }
    unlocked_tiers.sort();
    unlocked_tiers.dedup();
    progress.achievements.sort();
    progress.achievements.dedup();
}

fn apply_node_points_settlement_event(
    state: &mut WorldState,
    report: &EpochSettlementReport,
    signer_node_id: &str,
    settlement_hash: &str,
    minted_records: &[NodeRewardMintRecord],
    main_token_bridge_total_amount: u64,
    main_token_bridge_distributions: &[MainTokenNodePointsBridgeDistribution],
) -> Result<(), WorldError> {
    if signer_node_id.trim().is_empty() {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: "settlement signer_node_id cannot be empty".to_string(),
        });
    }
    let expected_hash = hash_json(report)?;
    if expected_hash != settlement_hash {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!(
                "settlement_hash mismatch: expected={} actual={}",
                expected_hash, settlement_hash
            ),
        });
    }
    let points_per_credit = state.reward_asset_config.points_per_credit;
    if points_per_credit == 0 {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: "points_per_credit must be positive".to_string(),
        });
    }
    if !state.node_identity_bindings.contains_key(signer_node_id) {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!("node identity is not bound: {signer_node_id}"),
        });
    }

    let mut settlement_points = BTreeMap::new();
    for settlement in &report.settlements {
        if settlement.node_id.trim().is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "report settlement contains empty node_id".to_string(),
            });
        }
        if settlement_points
            .insert(settlement.node_id.clone(), settlement.awarded_points)
            .is_some()
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "duplicate settlement node in report: {}",
                    settlement.node_id
                ),
            });
        }
        if !state
            .node_identity_bindings
            .contains_key(settlement.node_id.as_str())
        {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!("node identity is not bound: {}", settlement.node_id),
            });
        }
    }

    let mut budget = state
        .system_order_pool_budgets
        .get(&report.epoch_index)
        .cloned();
    if let Some(item) = budget.as_mut() {
        ensure_system_order_budget_caps_for_epoch(report, item);
    }

    let mut seen_nodes = BTreeMap::<String, ()>::new();
    for record in minted_records {
        if record.epoch_index != report.epoch_index {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "mint record epoch mismatch: report={} record={}",
                    report.epoch_index, record.epoch_index
                ),
            });
        }
        if record.signer_node_id != signer_node_id {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "mint record signer mismatch: event={} record={}",
                    signer_node_id, record.signer_node_id
                ),
            });
        }
        if record.settlement_hash != settlement_hash {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "mint record settlement_hash mismatch for node {}",
                    record.node_id
                ),
            });
        }
        let Some(awarded_points) = settlement_points.get(record.node_id.as_str()) else {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "mint record node is missing in report settlements: {}",
                    record.node_id
                ),
            });
        };
        if record.source_awarded_points != *awarded_points {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "mint record awarded points mismatch for node {}: report={} record={}",
                    record.node_id, awarded_points, record.source_awarded_points
                ),
            });
        }
        if record.minted_power_credits == 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "mint record has zero minted_power_credits for node {}",
                    record.node_id
                ),
            });
        }
        let max_minted = record.source_awarded_points / points_per_credit;
        if record.minted_power_credits > max_minted {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "minted credits exceed settlement cap for node {}: minted={} cap={}",
                    record.node_id, record.minted_power_credits, max_minted
                ),
            });
        }
        if seen_nodes.insert(record.node_id.clone(), ()).is_some() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "duplicate mint record node in one action: {}",
                    record.node_id
                ),
            });
        }
        if state.reward_mint_records.iter().any(|existing| {
            existing.epoch_index == record.epoch_index && existing.node_id == record.node_id
        }) {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "mint record already exists for epoch={} node={}",
                    record.epoch_index, record.node_id
                ),
            });
        }
        verify_reward_mint_record_signature_with_state(state, record).map_err(|reason| {
            WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "mint record signature invalid (epoch={} node={}): {}",
                    record.epoch_index, record.node_id, reason
                ),
            }
        })?;

        if let Some(item) = budget.as_mut() {
            let node_cap = item
                .node_credit_caps
                .get(record.node_id.as_str())
                .copied()
                .unwrap_or(0);
            let node_allocated = item
                .node_credit_allocated
                .get(record.node_id.as_str())
                .copied()
                .unwrap_or(0);
            let node_remaining = node_cap.saturating_sub(node_allocated);
            if record.minted_power_credits > node_remaining {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "minted credits exceed node budget cap for {}: minted={} remaining={}",
                        record.node_id, record.minted_power_credits, node_remaining
                    ),
                });
            }
            if record.minted_power_credits > item.remaining_credit_budget {
                return Err(WorldError::ResourceBalanceInvalid {
                    reason: format!(
                        "minted credits exceed remaining system order budget: minted={} remaining={}",
                        record.minted_power_credits, item.remaining_credit_budget
                    ),
                });
            }
            item.remaining_credit_budget = item
                .remaining_credit_budget
                .saturating_sub(record.minted_power_credits);
            item.node_credit_allocated
                .entry(record.node_id.clone())
                .and_modify(|value| *value = value.saturating_add(record.minted_power_credits))
                .or_insert(record.minted_power_credits);
        }
    }

    for record in minted_records {
        let balance = state
            .node_asset_balances
            .entry(record.node_id.clone())
            .or_insert_with(|| NodeAssetBalance {
                node_id: record.node_id.clone(),
                ..NodeAssetBalance::default()
            });
        balance.power_credit_balance = balance
            .power_credit_balance
            .saturating_add(record.minted_power_credits);
        balance.total_minted_credits = balance
            .total_minted_credits
            .saturating_add(record.minted_power_credits);
        state.reward_mint_records.push(record.clone());
    }
    if let Some(item) = budget {
        state
            .system_order_pool_budgets
            .insert(report.epoch_index, item);
    }
    apply_main_token_bridge_from_settlement_event(
        state,
        report,
        settlement_hash,
        main_token_bridge_total_amount,
        main_token_bridge_distributions,
    )?;
    Ok(())
}

fn apply_main_token_bridge_from_settlement_event(
    state: &mut WorldState,
    report: &EpochSettlementReport,
    settlement_hash: &str,
    total_amount: u64,
    distributions: &[MainTokenNodePointsBridgeDistribution],
) -> Result<(), WorldError> {
    if state
        .main_token_node_points_bridge_records
        .contains_key(&report.epoch_index)
    {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!(
                "main token node points bridge already processed for epoch={}",
                report.epoch_index
            ),
        });
    }
    if settlement_hash.trim().is_empty() {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: "main token bridge settlement_hash cannot be empty".to_string(),
        });
    }

    let expected_budget = state
        .main_token_epoch_issuance_records
        .get(&report.epoch_index)
        .map(|record| record.node_service_reward_amount)
        .unwrap_or(0);
    if total_amount > expected_budget {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!(
                "main token bridge total exceeds epoch node_service budget: epoch={} total={} budget={}",
                report.epoch_index, total_amount, expected_budget
            ),
        });
    }

    let mut distribution_sum = 0_u64;
    let mut seen_nodes = BTreeSet::new();
    for item in distributions {
        if item.node_id.trim().is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: "main token bridge distribution node_id cannot be empty".to_string(),
            });
        }
        if item.account_id.trim().is_empty() {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token bridge distribution account_id cannot be empty: node={}",
                    item.node_id
                ),
            });
        }
        if item.amount == 0 {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token bridge distribution amount must be > 0: node={}",
                    item.node_id
                ),
            });
        }
        if !seen_nodes.insert(item.node_id.clone()) {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "duplicate main token bridge distribution for node={}",
                    item.node_id
                ),
            });
        }
        distribution_sum = distribution_sum.checked_add(item.amount).ok_or_else(|| {
            WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token bridge distribution sum overflow: epoch={}",
                    report.epoch_index
                ),
            }
        })?;
    }
    if distribution_sum != total_amount {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!(
                "main token bridge sum mismatch: epoch={} total={} distributions_sum={}",
                report.epoch_index, total_amount, distribution_sum
            ),
        });
    }

    let treasury_balance = state
        .main_token_treasury_balances
        .get(MAIN_TOKEN_TREASURY_BUCKET_NODE_SERVICE_REWARD)
        .copied()
        .unwrap_or(0);
    if treasury_balance < total_amount {
        return Err(WorldError::ResourceBalanceInvalid {
            reason: format!(
                "main token bridge treasury insufficient: epoch={} balance={} total={}",
                report.epoch_index, treasury_balance, total_amount
            ),
        });
    }

    if total_amount > 0 {
        state.main_token_treasury_balances.insert(
            MAIN_TOKEN_TREASURY_BUCKET_NODE_SERVICE_REWARD.to_string(),
            treasury_balance - total_amount,
        );
        for item in distributions {
            let account = state
                .main_token_balances
                .entry(item.account_id.clone())
                .or_insert_with(|| MainTokenAccountBalance {
                    account_id: item.account_id.clone(),
                    ..MainTokenAccountBalance::default()
                });
            account.liquid_balance =
                account
                    .liquid_balance
                    .checked_add(item.amount)
                    .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                        reason: format!(
                            "main token bridge account overflow: account={} current={} amount={}",
                            item.account_id, account.liquid_balance, item.amount
                        ),
                    })?;
        }
        state.main_token_supply.circulating_supply = state
            .main_token_supply
            .circulating_supply
            .checked_add(total_amount)
            .ok_or_else(|| WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token bridge circulating overflow: current={} amount={}",
                    state.main_token_supply.circulating_supply, total_amount
                ),
            })?;
        if state.main_token_supply.circulating_supply > state.main_token_supply.total_supply {
            return Err(WorldError::ResourceBalanceInvalid {
                reason: format!(
                    "main token bridge circulating exceeds total: circulating={} total={}",
                    state.main_token_supply.circulating_supply,
                    state.main_token_supply.total_supply
                ),
            });
        }
    }

    state.main_token_node_points_bridge_records.insert(
        report.epoch_index,
        MainTokenNodePointsBridgeEpochRecord {
            epoch_index: report.epoch_index,
            settlement_hash: settlement_hash.to_string(),
            total_amount,
            distributions: distributions.to_vec(),
        },
    );
    Ok(())
}

fn integer_sqrt_u64(value: u64) -> u64 {
    if value < 2 {
        return value;
    }
    let mut x0 = value;
    let mut x1 = (x0 + value / x0) / 2;
    while x1 < x0 {
        x0 = x1;
        x1 = (x0 + value / x0) / 2;
    }
    x0
}

fn add_material_balance(
    balances: &mut BTreeMap<String, i64>,
    kind: &str,
    amount: i64,
) -> Result<(), String> {
    if amount < 0 {
        return Err(format!("negative material amount not allowed: {amount}"));
    }
    let entry = balances.entry(kind.to_string()).or_insert(0);
    *entry = entry.saturating_add(amount);
    if *entry == 0 {
        balances.remove(kind);
    }
    Ok(())
}

fn add_material_balance_for_ledger(
    ledgers: &mut BTreeMap<MaterialLedgerId, BTreeMap<String, i64>>,
    ledger: &MaterialLedgerId,
    kind: &str,
    amount: i64,
) -> Result<(), String> {
    let balances = ledgers.entry(ledger.clone()).or_default();
    add_material_balance(balances, kind, amount)
}

fn remove_material_balance(
    balances: &mut BTreeMap<String, i64>,
    kind: &str,
    amount: i64,
) -> Result<(), String> {
    if amount < 0 {
        return Err(format!("negative material amount not allowed: {amount}"));
    }
    let current = balances.get(kind).copied().unwrap_or(0);
    if current < amount {
        return Err(format!(
            "insufficient material {kind}: requested={amount} available={current}"
        ));
    }
    let next = current - amount;
    if next == 0 {
        balances.remove(kind);
    } else {
        balances.insert(kind.to_string(), next);
    }
    Ok(())
}

fn remove_material_balance_for_ledger(
    ledgers: &mut BTreeMap<MaterialLedgerId, BTreeMap<String, i64>>,
    ledger: &MaterialLedgerId,
    kind: &str,
    amount: i64,
) -> Result<(), String> {
    let balances = ledgers.entry(ledger.clone()).or_default();
    remove_material_balance(balances, kind, amount)
}

fn sync_compat_world_materials(
    ledgers: &BTreeMap<MaterialLedgerId, BTreeMap<String, i64>>,
    compat_world_materials_cache: &mut BTreeMap<String, i64>,
) {
    let world_materials = ledgers
        .get(&MaterialLedgerId::world())
        .cloned()
        .unwrap_or_default();
    *compat_world_materials_cache = world_materials;
}

fn apply_war_participant_outcomes(
    agents: &mut BTreeMap<String, AgentCell>,
    reputation_scores: &mut BTreeMap<String, i64>,
    outcomes: &[WarParticipantOutcome],
    now: WorldTime,
) -> Result<(), WorldError> {
    for outcome in outcomes {
        let Some(cell) = agents.get_mut(outcome.agent_id.as_str()) else {
            return Err(WorldError::AgentNotFound {
                agent_id: outcome.agent_id.clone(),
            });
        };

        apply_agent_resource_delta(
            cell,
            ResourceKind::Electricity,
            outcome.electricity_delta,
            outcome.agent_id.as_str(),
            "war electricity outcome",
        )?;
        apply_agent_resource_delta(
            cell,
            ResourceKind::Data,
            outcome.data_delta,
            outcome.agent_id.as_str(),
            "war data outcome",
        )?;
        cell.last_active = now;

        if outcome.reputation_delta != 0 {
            let score = reputation_scores
                .entry(outcome.agent_id.clone())
                .or_insert(0);
            *score = score.saturating_add(outcome.reputation_delta);
        }
    }
    Ok(())
}

fn apply_agent_resource_delta(
    cell: &mut AgentCell,
    kind: ResourceKind,
    delta: i64,
    agent_id: &str,
    context: &str,
) -> Result<(), WorldError> {
    if delta == 0 {
        return Ok(());
    }
    if delta > 0 {
        return cell.state.resources.add(kind, delta).map_err(|err| {
            WorldError::ResourceBalanceInvalid {
                reason: format!("{context} apply failed for {agent_id}: {err:?}"),
            }
        });
    }
    cell.state
        .resources
        .remove(kind, delta.saturating_abs())
        .map_err(|err| WorldError::ResourceBalanceInvalid {
            reason: format!("{context} apply failed for {agent_id}: {err:?}"),
        })
}

fn remove_resource_balance(
    balances: &mut BTreeMap<ResourceKind, i64>,
    kind: ResourceKind,
    amount: i64,
) -> Result<(), String> {
    if amount < 0 {
        return Err(format!("negative resource amount not allowed: {amount}"));
    }
    let current = balances.get(&kind).copied().unwrap_or(0);
    if current < amount {
        return Err(format!(
            "insufficient resource {:?}: requested={amount} available={current}",
            kind
        ));
    }
    let next = current - amount;
    if next == 0 {
        balances.remove(&kind);
    } else {
        balances.insert(kind, next);
    }
    Ok(())
}

fn verify_reward_mint_record_signature_with_state(
    state: &WorldState,
    record: &NodeRewardMintRecord,
) -> Result<(), String> {
    let signer_public_key = state
        .node_identity_bindings
        .get(record.signer_node_id.as_str())
        .map(String::as_str)
        .ok_or_else(|| {
            format!(
                "reward mint signer identity is not bound: {}",
                record.signer_node_id
            )
        })?;
    if record
        .signature
        .starts_with(REWARD_MINT_SIGNATURE_V2_PREFIX)
    {
        return verify_reward_mint_signature_v2(
            record.signature.as_str(),
            record.epoch_index,
            record.node_id.as_str(),
            record.source_awarded_points,
            record.minted_power_credits,
            record.settlement_hash.as_str(),
            record.signer_node_id.as_str(),
            signer_public_key,
        );
    }
    if record
        .signature
        .starts_with(REWARD_MINT_SIGNATURE_V1_PREFIX)
    {
        if !state
            .reward_signature_governance_policy
            .allow_mintsig_v1_fallback
        {
            return Err("mintsig:v1 is disabled by governance policy".to_string());
        }
        let expected_signature = reward_mint_signature_v1(
            record.epoch_index,
            record.node_id.as_str(),
            record.source_awarded_points,
            record.minted_power_credits,
            record.settlement_hash.as_str(),
            record.signer_node_id.as_str(),
            signer_public_key,
        );
        if expected_signature != record.signature {
            return Err(format!(
                "reward mint signature mismatch for node {} at epoch {}",
                record.node_id, record.epoch_index
            ));
        }
        return Ok(());
    }
    Err(format!(
        "unsupported reward mint signature version for node {} at epoch {}",
        record.node_id, record.epoch_index
    ))
}

fn ensure_system_order_budget_caps_for_epoch(
    report: &EpochSettlementReport,
    budget: &mut SystemOrderPoolBudget,
) {
    if !budget.node_credit_caps.is_empty() {
        return;
    }
    if budget.total_credit_budget == 0 || report.settlements.is_empty() {
        return;
    }

    let total_awarded_points = report
        .settlements
        .iter()
        .map(|settlement| settlement.awarded_points)
        .sum::<u64>();
    if total_awarded_points == 0 {
        return;
    }

    let mut distributed = 0_u64;
    for settlement in &report.settlements {
        let cap = budget
            .total_credit_budget
            .saturating_mul(settlement.awarded_points)
            / total_awarded_points;
        distributed = distributed.saturating_add(cap);
        budget
            .node_credit_caps
            .insert(settlement.node_id.clone(), cap);
    }

    let mut remainder = budget.total_credit_budget.saturating_sub(distributed);
    if remainder == 0 {
        return;
    }
    let mut ranked = report
        .settlements
        .iter()
        .map(|settlement| (settlement.node_id.as_str(), settlement.awarded_points))
        .collect::<Vec<_>>();
    ranked.sort_by(|(a_node_id, a_points), (b_node_id, b_points)| {
        b_points
            .cmp(a_points)
            .then_with(|| a_node_id.cmp(b_node_id))
    });
    let mut index = 0_usize;
    while remainder > 0 && !ranked.is_empty() {
        let node_id = ranked[index % ranked.len()].0;
        if let Some(cap) = budget.node_credit_caps.get_mut(node_id) {
            *cap = cap.saturating_add(1);
            remainder -= 1;
        }
        index = index.saturating_add(1);
    }
}
