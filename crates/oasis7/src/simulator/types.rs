//! Core type definitions: IDs, constants, and resource types.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ============================================================================
// Type Aliases
// ============================================================================

pub type AgentId = String;
pub type LocationId = String;
pub type AssetId = String;
pub type FacilityId = String;
pub type WorldTime = u64;
pub type WorldEventId = u64;
pub type ActionId = u64;

// ============================================================================
// Constants
// ============================================================================

pub const CM_PER_KM: i64 = 100_000;
pub const DEFAULT_VISIBILITY_RANGE_CM: i64 = 10_000_000;
pub const DEFAULT_MOVE_COST_PER_KM_ELECTRICITY: i64 = 1;
pub const SNAPSHOT_VERSION: u32 = 3;
pub const JOURNAL_VERSION: u32 = 3;
pub const CHUNK_GENERATION_SCHEMA_VERSION: u32 = 1;
pub const PPM_BASE: i64 = 1_000_000;
pub const DEFAULT_ELEMENT_RECOVERABILITY_PPM: i64 = 850_000;

// ============================================================================
// Resource Types
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Electricity,
    Data,
}

// ============================================================================
// Location Physical Profile
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialKind {
    Silicate,
    Metal,
    Ice,
    Carbon,
    Composite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FragmentElementKind {
    Oxygen,
    Silicon,
    Magnesium,
    Aluminum,
    Calcium,
    Iron,
    Nickel,
    Cobalt,
    Titanium,
    Chromium,
    Hydrogen,
    Carbon,
    Nitrogen,
    Sulfur,
    Copper,
    Zinc,
    Lithium,
    Neodymium,
    Uranium,
    Thorium,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ElementComposition {
    pub ppm: BTreeMap<FragmentElementKind, u32>,
}

impl ElementComposition {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, kind: FragmentElementKind) -> u32 {
        *self.ppm.get(&kind).unwrap_or(&0)
    }

    pub fn set(&mut self, kind: FragmentElementKind, value: u32) {
        if value == 0 {
            self.ppm.remove(&kind);
        } else {
            self.ppm.insert(kind, value);
        }
    }

    pub fn total_ppm(&self) -> u64 {
        self.ppm.values().map(|value| *value as u64).sum()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct FragmentResourceBudget {
    pub total_by_element_g: BTreeMap<FragmentElementKind, i64>,
    pub remaining_by_element_g: BTreeMap<FragmentElementKind, i64>,
}

impl FragmentResourceBudget {
    pub fn get_total(&self, kind: FragmentElementKind) -> i64 {
        *self.total_by_element_g.get(&kind).unwrap_or(&0)
    }

    pub fn get_remaining(&self, kind: FragmentElementKind) -> i64 {
        *self.remaining_by_element_g.get(&kind).unwrap_or(&0)
    }

    pub fn from_mass_and_elements(
        mass_g: i64,
        elements: &ElementComposition,
        recoverability_ppm: i64,
    ) -> Self {
        if mass_g <= 0 {
            return Self::default();
        }
        let recoverability_ppm = recoverability_ppm.clamp(0, PPM_BASE);
        let mut out = Self::default();

        for (element, ppm) in &elements.ppm {
            if *ppm == 0 {
                continue;
            }
            let total = mass_g
                .saturating_mul(*ppm as i64)
                .saturating_mul(recoverability_ppm)
                .saturating_div(PPM_BASE)
                .saturating_div(PPM_BASE);
            if total > 0 {
                out.total_by_element_g.insert(*element, total);
                out.remaining_by_element_g.insert(*element, total);
            }
        }

        out
    }

    pub fn consume(
        &mut self,
        kind: FragmentElementKind,
        amount_g: i64,
    ) -> Result<i64, ElementBudgetError> {
        if amount_g <= 0 {
            return Err(ElementBudgetError::InvalidAmount { amount_g });
        }

        let available = self.get_remaining(kind);
        if available < amount_g {
            return Err(ElementBudgetError::Insufficient {
                kind,
                requested_g: amount_g,
                remaining_g: available,
            });
        }

        let next = available - amount_g;
        if next == 0 {
            self.remaining_by_element_g.remove(&kind);
        } else {
            self.remaining_by_element_g.insert(kind, next);
        }
        Ok(amount_g)
    }

    pub fn is_exhausted(&self) -> bool {
        self.remaining_by_element_g
            .values()
            .all(|value| *value <= 0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ChunkResourceBudget {
    pub total_by_element_g: BTreeMap<FragmentElementKind, i64>,
    pub remaining_by_element_g: BTreeMap<FragmentElementKind, i64>,
}

impl ChunkResourceBudget {
    pub fn get_total(&self, kind: FragmentElementKind) -> i64 {
        *self.total_by_element_g.get(&kind).unwrap_or(&0)
    }

    pub fn get_remaining(&self, kind: FragmentElementKind) -> i64 {
        *self.remaining_by_element_g.get(&kind).unwrap_or(&0)
    }

    pub fn accumulate_fragment(&mut self, fragment: &FragmentResourceBudget) {
        for (element, total) in &fragment.total_by_element_g {
            if *total <= 0 {
                continue;
            }
            let entry = self.total_by_element_g.entry(*element).or_insert(0);
            *entry = entry.saturating_add(*total);
        }
        for (element, remaining) in &fragment.remaining_by_element_g {
            if *remaining <= 0 {
                continue;
            }
            let entry = self.remaining_by_element_g.entry(*element).or_insert(0);
            *entry = entry.saturating_add(*remaining);
        }
    }

    pub fn consume(
        &mut self,
        kind: FragmentElementKind,
        amount_g: i64,
    ) -> Result<i64, ElementBudgetError> {
        if amount_g <= 0 {
            return Err(ElementBudgetError::InvalidAmount { amount_g });
        }

        let available = self.get_remaining(kind);
        if available < amount_g {
            return Err(ElementBudgetError::Insufficient {
                kind,
                requested_g: amount_g,
                remaining_g: available,
            });
        }

        let next = available - amount_g;
        if next == 0 {
            self.remaining_by_element_g.remove(&kind);
        } else {
            self.remaining_by_element_g.insert(kind, next);
        }
        Ok(amount_g)
    }

    pub fn is_exhausted(&self) -> bool {
        self.remaining_by_element_g
            .values()
            .all(|value| *value <= 0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementBudgetError {
    InvalidAmount {
        amount_g: i64,
    },
    Insufficient {
        kind: FragmentElementKind,
        requested_g: i64,
        remaining_g: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocationProfile {
    pub material: MaterialKind,
    pub radius_cm: i64,
    pub radiation_emission_per_tick: i64,
}

impl Default for LocationProfile {
    fn default() -> Self {
        Self {
            material: MaterialKind::Silicate,
            radius_cm: 100,
            radiation_emission_per_tick: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ResourceStock {
    pub amounts: BTreeMap<ResourceKind, i64>,
}

impl ResourceStock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, kind: ResourceKind) -> i64 {
        *self.amounts.get(&kind).unwrap_or(&0)
    }

    pub fn set(&mut self, kind: ResourceKind, amount: i64) -> Result<(), StockError> {
        if amount < 0 {
            return Err(StockError::NegativeAmount { amount });
        }
        if amount == 0 {
            self.amounts.remove(&kind);
        } else {
            self.amounts.insert(kind, amount);
        }
        Ok(())
    }

    pub fn add(&mut self, kind: ResourceKind, amount: i64) -> Result<(), StockError> {
        if amount < 0 {
            return Err(StockError::NegativeAmount { amount });
        }
        let current = self.get(kind);
        let next = current.checked_add(amount).ok_or(StockError::Overflow {
            kind,
            current,
            delta: amount,
        })?;
        self.set(kind, next)
    }

    pub fn remove(&mut self, kind: ResourceKind, amount: i64) -> Result<(), StockError> {
        if amount < 0 {
            return Err(StockError::NegativeAmount { amount });
        }
        let current = self.get(kind);
        if current < amount {
            return Err(StockError::Insufficient {
                kind,
                requested: amount,
                available: current,
            });
        }
        self.set(kind, current - amount)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StockError {
    NegativeAmount {
        amount: i64,
    },
    Overflow {
        kind: ResourceKind,
        current: i64,
        delta: i64,
    },
    Insufficient {
        kind: ResourceKind,
        requested: i64,
        available: i64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ResourceOwner {
    Agent { agent_id: AgentId },
    Location { location_id: LocationId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum ModuleInstallTarget {
    #[default]
    SelfAgent,
    LocationInfrastructure {
        location_id: LocationId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerOrderSide {
    Buy,
    Sell,
}

// ============================================================================
// Action Types
// ============================================================================

use super::module_visual::ModuleVisualEntity;
use super::social::{SocialAdjudicationDecision, SocialStake};
use crate::geometry::GeoPos;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActionEnvelope {
    pub id: ActionId,
    pub action: Action,
    #[serde(default)]
    pub submitter: ActionSubmitter,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionSubmitter {
    System,
    Agent { agent_id: AgentId },
    Player { player_id: String },
}

impl Default for ActionSubmitter {
    fn default() -> Self {
        ActionSubmitter::System
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum Action {
    RegisterLocation {
        location_id: LocationId,
        name: String,
        pos: GeoPos,
        profile: LocationProfile,
    },
    RegisterAgent {
        agent_id: AgentId,
        location_id: LocationId,
    },
    RegisterPowerPlant {
        facility_id: FacilityId,
        location_id: LocationId,
        owner: ResourceOwner,
        capacity_per_tick: i64,
        fuel_cost_per_pu: i64,
        maintenance_cost: i64,
        efficiency: f64,
        degradation: f64,
    },
    UpsertModuleVisualEntity {
        entity: ModuleVisualEntity,
    },
    RemoveModuleVisualEntity {
        entity_id: String,
    },
    MoveAgent {
        agent_id: AgentId,
        to: LocationId,
    },
    SpeakToNearby {
        agent_id: AgentId,
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_agent_id: Option<AgentId>,
    },
    InspectTarget {
        agent_id: AgentId,
        target_kind: String,
        target_id: String,
    },
    SimpleInteract {
        agent_id: AgentId,
        target_kind: String,
        target_id: String,
        interaction: String,
    },
    HarvestRadiation {
        agent_id: AgentId,
        max_amount: i64,
    },
    BuyPower {
        buyer: ResourceOwner,
        seller: ResourceOwner,
        amount: i64,
        price_per_pu: i64,
    },
    SellPower {
        seller: ResourceOwner,
        buyer: ResourceOwner,
        amount: i64,
        price_per_pu: i64,
    },
    PlacePowerOrder {
        owner: ResourceOwner,
        side: PowerOrderSide,
        amount: i64,
        limit_price_per_pu: i64,
    },
    CancelPowerOrder {
        owner: ResourceOwner,
        order_id: u64,
    },
    TransferResource {
        from: ResourceOwner,
        to: ResourceOwner,
        kind: ResourceKind,
        amount: i64,
    },
    DebugGrantResource {
        owner: ResourceOwner,
        kind: ResourceKind,
        amount: i64,
    },
    MineCompound {
        owner: ResourceOwner,
        location_id: LocationId,
        compound_mass_g: i64,
    },
    RefineCompound {
        owner: ResourceOwner,
        compound_mass_g: i64,
    },
    BuildFactory {
        owner: ResourceOwner,
        location_id: LocationId,
        factory_id: FacilityId,
        factory_kind: String,
    },
    ScheduleRecipe {
        owner: ResourceOwner,
        factory_id: FacilityId,
        recipe_id: String,
        batches: i64,
    },
    CompileModuleArtifactFromSource {
        publisher_agent_id: AgentId,
        module_id: String,
        manifest_path: String,
        source_files: BTreeMap<String, Vec<u8>>,
    },
    DeployModuleArtifact {
        publisher_agent_id: AgentId,
        wasm_hash: String,
        wasm_bytes: Vec<u8>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        module_id_hint: Option<String>,
    },
    InstallModuleFromArtifact {
        installer_agent_id: AgentId,
        module_id: String,
        module_version: String,
        wasm_hash: String,
        activate: bool,
    },
    InstallModuleToTargetFromArtifact {
        installer_agent_id: AgentId,
        module_id: String,
        module_version: String,
        wasm_hash: String,
        activate: bool,
        install_target: ModuleInstallTarget,
    },
    InstallMicroDepot {
        installer_agent_id: AgentId,
        facility_id: FacilityId,
        location_id: LocationId,
        owner_claim_id: String,
        module_id: String,
        module_version: String,
        wasm_hash: String,
        entrypoint: String,
        service_radius_cm: i64,
        #[serde(default)]
        supported_resource_kinds: Vec<String>,
    },
    ServiceMicroDepotRepair {
        agent_id: AgentId,
        facility_id: FacilityId,
        target_id: String,
        base_cost_class: crate::simulator::MicroDepotPressureClass,
        base_risk_class: crate::simulator::MicroDepotPressureClass,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        blocker_type: Option<String>,
    },
    ServiceMicroDepotLogistics {
        agent_id: AgentId,
        facility_id: FacilityId,
        target_id: String,
        base_cost_class: crate::simulator::MicroDepotPressureClass,
        base_risk_class: crate::simulator::MicroDepotPressureClass,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        blocker_type: Option<String>,
    },
    PayMicroDepotUpkeep {
        agent_id: AgentId,
        facility_id: FacilityId,
    },
    SuspendMicroDepot {
        agent_id: AgentId,
        facility_id: FacilityId,
    },
    ReclaimMicroDepot {
        agent_id: AgentId,
        facility_id: FacilityId,
    },
    ListModuleArtifactForSale {
        seller_agent_id: AgentId,
        wasm_hash: String,
        price_kind: ResourceKind,
        price_amount: i64,
    },
    BuyModuleArtifact {
        buyer_agent_id: AgentId,
        wasm_hash: String,
    },
    DelistModuleArtifact {
        seller_agent_id: AgentId,
        wasm_hash: String,
    },
    DestroyModuleArtifact {
        owner_agent_id: AgentId,
        wasm_hash: String,
        reason: String,
    },
    PlaceModuleArtifactBid {
        bidder_agent_id: AgentId,
        wasm_hash: String,
        price_kind: ResourceKind,
        price_amount: i64,
    },
    CancelModuleArtifactBid {
        bidder_agent_id: AgentId,
        wasm_hash: String,
        bid_order_id: u64,
    },
    PublishSocialFact {
        actor: ResourceOwner,
        schema_id: String,
        subject: ResourceOwner,
        object: Option<ResourceOwner>,
        claim: String,
        confidence_ppm: i64,
        evidence_event_ids: Vec<WorldEventId>,
        ttl_ticks: Option<u64>,
        stake: Option<SocialStake>,
    },
    ChallengeSocialFact {
        challenger: ResourceOwner,
        fact_id: u64,
        reason: String,
        stake: Option<SocialStake>,
    },
    AdjudicateSocialFact {
        adjudicator: ResourceOwner,
        fact_id: u64,
        decision: SocialAdjudicationDecision,
        notes: String,
    },
    RevokeSocialFact {
        actor: ResourceOwner,
        fact_id: u64,
        reason: String,
    },
    DeclareSocialEdge {
        declarer: ResourceOwner,
        schema_id: String,
        relation_kind: String,
        from: ResourceOwner,
        to: ResourceOwner,
        weight_bps: i64,
        backing_fact_ids: Vec<u64>,
        ttl_ticks: Option<u64>,
    },
    FormAlliance {
        proposer_agent_id: AgentId,
        alliance_id: String,
        members: Vec<AgentId>,
        charter: String,
    },
    JoinAlliance {
        operator_agent_id: AgentId,
        alliance_id: String,
        member_agent_id: AgentId,
    },
    LeaveAlliance {
        operator_agent_id: AgentId,
        alliance_id: String,
        member_agent_id: AgentId,
    },
    DissolveAlliance {
        operator_agent_id: AgentId,
        alliance_id: String,
        reason: String,
    },
    DeclareWar {
        initiator_agent_id: AgentId,
        war_id: String,
        aggressor_alliance_id: String,
        defender_alliance_id: String,
        objective: String,
        intensity: u32,
    },
    OpenGovernanceProposal {
        proposer_agent_id: AgentId,
        proposal_key: String,
        title: String,
        description: String,
        options: Vec<String>,
        voting_window_ticks: u64,
        quorum_weight: u64,
        pass_threshold_bps: u16,
    },
    CastGovernanceVote {
        voter_agent_id: AgentId,
        proposal_key: String,
        option: String,
        weight: u32,
    },
    ResolveCrisis {
        resolver_agent_id: AgentId,
        crisis_id: String,
        strategy: String,
        success: bool,
    },
    GrantMetaProgress {
        operator_agent_id: AgentId,
        target_agent_id: AgentId,
        track: String,
        points: i64,
        achievement_id: Option<String>,
    },
    OpenEconomicContract {
        creator_agent_id: AgentId,
        contract_id: String,
        counterparty_agent_id: AgentId,
        settlement_kind: ResourceKind,
        settlement_amount: i64,
        reputation_stake: i64,
        expires_at: u64,
        description: String,
    },
    AcceptEconomicContract {
        accepter_agent_id: AgentId,
        contract_id: String,
    },
    SettleEconomicContract {
        operator_agent_id: AgentId,
        contract_id: String,
        success: bool,
        notes: String,
    },
}
