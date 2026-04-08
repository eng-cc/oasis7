//! World model entities: Agent, Location, Asset, WorldConfig, WorldModel.

use crate::geometry::{
    GeoPos, DEFAULT_CLOUD_DEPTH_CM, DEFAULT_CLOUD_HEIGHT_CM, DEFAULT_CLOUD_WIDTH_CM,
};
use crate::models::RobotBodySpec;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

mod module_market;
mod world_model_physics_specs;

use super::chunking::{chunk_coord_of, ChunkCoord};
use super::fragment_physics::FragmentPhysicalProfile;
use super::memory::LongTermMemoryEntry;
use super::module_visual::ModuleVisualEntity;
use super::power::{AgentPowerStatus, PowerConfig, PowerPlant};
use super::social::{
    default_next_social_edge_id, default_next_social_fact_id, SocialEdgeState, SocialFactState,
};
use super::types::{
    AgentId, AssetId, ChunkResourceBudget, ElementBudgetError, FacilityId, FragmentElementKind,
    FragmentResourceBudget, LocationId, LocationProfile, MaterialKind, PowerOrderSide,
    ResourceKind, ResourceStock, WorldTime, CM_PER_KM, DEFAULT_MOVE_COST_PER_KM_ELECTRICITY,
    DEFAULT_VISIBILITY_RANGE_CM, PPM_BASE,
};
use super::ResourceOwner;
use module_market::{default_next_module_market_order_id, default_next_module_market_sale_id};
pub use module_market::{
    InstalledModuleState, ModuleArtifactBidState, ModuleArtifactListingState, ModuleArtifactState,
};
pub use world_model_physics_specs::{physics_parameter_specs, PhysicsParameterSpec};

const MOVE_COST_REFERENCE_TIME_STEP_S: i64 = 10;
const MOVE_COST_REFERENCE_POWER_UNIT_J: i64 = 1_000;
const DEFAULT_AGENT_SPEED_CM_PER_TICK: i64 =
    DEFAULT_CLOUD_WIDTH_CM + DEFAULT_CLOUD_DEPTH_CM + DEFAULT_CLOUD_HEIGHT_CM;

// ============================================================================
// World Entities
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentKinematics {
    /// Planned movement speed used by time-based movement semantics.
    pub speed_cm_per_tick: i64,
    /// Optional movement target location id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub move_target_location_id: Option<LocationId>,
    /// Optional movement target in absolute world position.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub move_target: Option<GeoPos>,
    /// Tick index when the current move started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub move_started_at_tick: Option<WorldTime>,
    /// Tick index when the current move is expected to arrive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub move_eta_tick: Option<WorldTime>,
    /// Remaining distance in centimeters for the current move.
    pub move_remaining_cm: i64,
}

impl Default for AgentKinematics {
    fn default() -> Self {
        Self {
            speed_cm_per_tick: DEFAULT_AGENT_SPEED_CM_PER_TICK,
            move_target_location_id: None,
            move_target: None,
            move_started_at_tick: None,
            move_eta_tick: None,
            move_remaining_cm: 0,
        }
    }
}

impl AgentKinematics {
    pub fn clear_motion_state(&mut self) {
        self.move_target_location_id = None;
        self.move_target = None;
        self.move_started_at_tick = None;
        self.move_eta_tick = None;
        self.move_remaining_cm = 0;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AgentExecutionDebugContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_check_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_check_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation_schema_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_schema_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment_class: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supported_action_sets: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provider_reported_capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provider_reported_supported_action_sets: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_check_fallback_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_check_error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_config_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_profile: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub pos: GeoPos,
    pub body: RobotBodySpec,
    pub location_id: LocationId,
    pub resources: ResourceStock,
    /// Power status for M4 power system.
    pub power: AgentPowerStatus,
    /// Movement state for upcoming time-based move execution.
    #[serde(default)]
    pub kinematics: AgentKinematics,
    /// Thermal status (heat accumulation).
    #[serde(default)]
    pub thermal: ThermalStatus,
}

impl Agent {
    pub fn new(id: impl Into<String>, location_id: impl Into<String>, pos: GeoPos) -> Self {
        Self {
            id: id.into(),
            pos,
            body: RobotBodySpec::default(),
            location_id: location_id.into(),
            resources: ResourceStock::default(),
            power: AgentPowerStatus::default(),
            kinematics: AgentKinematics::default(),
            thermal: ThermalStatus::default(),
        }
    }

    /// Create a new agent with custom power configuration.
    pub fn new_with_power(
        id: impl Into<String>,
        location_id: impl Into<String>,
        pos: GeoPos,
        power_config: &PowerConfig,
    ) -> Self {
        Self {
            id: id.into(),
            pos,
            body: RobotBodySpec::default(),
            location_id: location_id.into(),
            resources: ResourceStock::default(),
            power: AgentPowerStatus::from_config(power_config),
            kinematics: AgentKinematics::default(),
            thermal: ThermalStatus::default(),
        }
    }

    /// Check if the agent is shut down due to power depletion.
    pub fn is_shutdown(&self) -> bool {
        self.power.is_shutdown()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Location {
    pub id: LocationId,
    pub name: String,
    pub pos: GeoPos,
    pub profile: LocationProfile,
    pub resources: ResourceStock,
    #[serde(default)]
    pub mined_compound_g: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fragment_profile: Option<FragmentPhysicalProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fragment_budget: Option<FragmentResourceBudget>,
}

impl Location {
    pub fn new(id: impl Into<String>, name: impl Into<String>, pos: GeoPos) -> Self {
        Self::new_with_profile(id, name, pos, LocationProfile::default())
    }

    pub fn new_with_profile(
        id: impl Into<String>,
        name: impl Into<String>,
        pos: GeoPos,
        profile: LocationProfile,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            pos,
            profile,
            resources: ResourceStock::default(),
            mined_compound_g: 0,
            fragment_profile: None,
            fragment_budget: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Asset {
    pub id: AssetId,
    pub owner: ResourceOwner,
    pub kind: AssetKind,
    pub quantity: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum AssetKind {
    Resource { kind: ResourceKind },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Factory {
    pub id: FacilityId,
    pub owner: ResourceOwner,
    pub location_id: LocationId,
    pub kind: String,
}

fn default_next_power_order_id() -> u64 {
    1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerOrderState {
    pub order_id: u64,
    pub owner: ResourceOwner,
    pub side: PowerOrderSide,
    pub remaining_amount: i64,
    pub limit_price_per_pu: i64,
    pub created_at: WorldTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PowerOrderBookState {
    #[serde(default = "default_next_power_order_id")]
    pub next_order_id: u64,
    #[serde(default)]
    pub open_orders: Vec<PowerOrderState>,
}

impl Default for PowerOrderBookState {
    fn default() -> Self {
        Self {
            next_order_id: default_next_power_order_id(),
            open_orders: Vec::new(),
        }
    }
}

// ============================================================================
// World Model (aggregate)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WorldModel {
    pub agents: BTreeMap<AgentId, Agent>,
    #[serde(default)]
    pub agent_prompt_profiles: BTreeMap<AgentId, AgentPromptProfile>,
    #[serde(default)]
    pub agent_player_bindings: BTreeMap<AgentId, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub agent_execution_debug_contexts: BTreeMap<AgentId, AgentExecutionDebugContext>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub agent_player_public_key_bindings: BTreeMap<AgentId, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub player_auth_last_nonce: BTreeMap<String, u64>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub agent_long_term_memories: BTreeMap<AgentId, Vec<LongTermMemoryEntry>>,
    pub locations: BTreeMap<LocationId, Location>,
    pub assets: BTreeMap<AssetId, Asset>,
    #[serde(default)]
    pub module_visual_entities: BTreeMap<String, ModuleVisualEntity>,
    #[serde(default)]
    pub module_artifacts: BTreeMap<String, ModuleArtifactState>,
    #[serde(default)]
    pub installed_modules: BTreeMap<String, InstalledModuleState>,
    #[serde(default)]
    pub module_artifact_listings: BTreeMap<String, ModuleArtifactListingState>,
    #[serde(default)]
    pub module_artifact_bids: BTreeMap<String, Vec<ModuleArtifactBidState>>,
    #[serde(default = "default_next_module_market_order_id")]
    pub next_module_market_order_id: u64,
    #[serde(default = "default_next_module_market_sale_id")]
    pub next_module_market_sale_id: u64,
    #[serde(default)]
    pub power_plants: BTreeMap<FacilityId, PowerPlant>,
    #[serde(default)]
    pub power_order_book: PowerOrderBookState,
    #[serde(default = "default_next_social_fact_id")]
    pub next_social_fact_id: u64,
    #[serde(default)]
    pub social_facts: BTreeMap<u64, SocialFactState>,
    #[serde(default = "default_next_social_edge_id")]
    pub next_social_edge_id: u64,
    #[serde(default)]
    pub social_edges: BTreeMap<u64, SocialEdgeState>,
    #[serde(default)]
    pub social_stake_pool: ResourceStock,
    #[serde(default)]
    pub factories: BTreeMap<FacilityId, Factory>,
    #[serde(
        default,
        serialize_with = "serialize_chunk_states",
        deserialize_with = "deserialize_chunk_states"
    )]
    pub chunks: BTreeMap<ChunkCoord, ChunkState>,
    #[serde(
        default,
        serialize_with = "serialize_chunk_resource_budgets",
        deserialize_with = "deserialize_chunk_resource_budgets"
    )]
    pub chunk_resource_budgets: BTreeMap<ChunkCoord, ChunkResourceBudget>,
    #[serde(
        default,
        serialize_with = "serialize_chunk_boundary_reservations",
        deserialize_with = "deserialize_chunk_boundary_reservations"
    )]
    pub chunk_boundary_reservations: BTreeMap<ChunkCoord, Vec<BoundaryReservation>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AgentPromptProfile {
    pub agent_id: AgentId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt_override: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_term_goal_override: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub long_term_goal_override: Option<String>,
    #[serde(default)]
    pub version: u64,
    #[serde(default)]
    pub updated_at_tick: WorldTime,
    #[serde(default)]
    pub updated_by: String,
}

impl AgentPromptProfile {
    pub fn for_agent(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChunkState {
    #[default]
    Unexplored,
    Generated,
    Exhausted,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundaryReservation {
    pub source_chunk: ChunkCoord,
    pub source_fragment_id: LocationId,
    pub source_pos: GeoPos,
    pub source_radius_cm: i64,
    pub min_spacing_cm: i64,
}

fn serialize_chunk_states<S>(
    chunks: &BTreeMap<ChunkCoord, ChunkState>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let encoded: BTreeMap<String, ChunkState> = chunks
        .iter()
        .map(|(coord, state)| (encode_chunk_coord(*coord), *state))
        .collect();
    encoded.serialize(serializer)
}

fn deserialize_chunk_states<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<ChunkCoord, ChunkState>, D::Error>
where
    D: Deserializer<'de>,
{
    let encoded = BTreeMap::<String, ChunkState>::deserialize(deserializer)?;
    let mut decoded = BTreeMap::new();
    for (key, state) in encoded {
        let coord = decode_chunk_coord(&key).map_err(serde::de::Error::custom)?;
        decoded.insert(coord, state);
    }
    Ok(decoded)
}

fn serialize_chunk_resource_budgets<S>(
    budgets: &BTreeMap<ChunkCoord, ChunkResourceBudget>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let encoded: BTreeMap<String, ChunkResourceBudget> = budgets
        .iter()
        .map(|(coord, budget)| (encode_chunk_coord(*coord), budget.clone()))
        .collect();
    encoded.serialize(serializer)
}

fn deserialize_chunk_resource_budgets<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<ChunkCoord, ChunkResourceBudget>, D::Error>
where
    D: Deserializer<'de>,
{
    let encoded = BTreeMap::<String, ChunkResourceBudget>::deserialize(deserializer)?;
    let mut decoded = BTreeMap::new();
    for (key, budget) in encoded {
        let coord = decode_chunk_coord(&key).map_err(serde::de::Error::custom)?;
        decoded.insert(coord, budget);
    }
    Ok(decoded)
}

fn serialize_chunk_boundary_reservations<S>(
    reservations: &BTreeMap<ChunkCoord, Vec<BoundaryReservation>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let encoded: BTreeMap<String, Vec<BoundaryReservation>> = reservations
        .iter()
        .map(|(coord, entries)| (encode_chunk_coord(*coord), entries.clone()))
        .collect();
    encoded.serialize(serializer)
}

fn deserialize_chunk_boundary_reservations<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<ChunkCoord, Vec<BoundaryReservation>>, D::Error>
where
    D: Deserializer<'de>,
{
    let encoded = BTreeMap::<String, Vec<BoundaryReservation>>::deserialize(deserializer)?;
    let mut decoded = BTreeMap::new();
    for (key, entries) in encoded {
        let coord = decode_chunk_coord(&key).map_err(serde::de::Error::custom)?;
        decoded.insert(coord, entries);
    }
    Ok(decoded)
}

fn encode_chunk_coord(coord: ChunkCoord) -> String {
    format!("{}:{}:{}", coord.x, coord.y, coord.z)
}

fn decode_chunk_coord(encoded: &str) -> Result<ChunkCoord, String> {
    let mut parts = encoded.split(':');
    let x = parts
        .next()
        .ok_or_else(|| format!("invalid chunk coord key: {encoded}"))?
        .parse::<i32>()
        .map_err(|_| format!("invalid chunk coord x: {encoded}"))?;
    let y = parts
        .next()
        .ok_or_else(|| format!("invalid chunk coord key: {encoded}"))?
        .parse::<i32>()
        .map_err(|_| format!("invalid chunk coord y: {encoded}"))?;
    let z = parts
        .next()
        .ok_or_else(|| format!("invalid chunk coord key: {encoded}"))?
        .parse::<i32>()
        .map_err(|_| format!("invalid chunk coord z: {encoded}"))?;
    if parts.next().is_some() {
        return Err(format!("invalid chunk coord key: {encoded}"));
    }
    Ok(ChunkCoord { x, y, z })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FragmentResourceError {
    LocationNotFound { location_id: LocationId },
    FragmentBudgetMissing { location_id: LocationId },
    ChunkCoordUnavailable { location_id: LocationId },
    ChunkBudgetMissing { coord: ChunkCoord },
    Budget(ElementBudgetError),
}

impl WorldModel {
    pub fn consume_fragment_resource(
        &mut self,
        location_id: &str,
        space: &SpaceConfig,
        kind: FragmentElementKind,
        amount_g: i64,
    ) -> Result<i64, FragmentResourceError> {
        if amount_g <= 0 {
            return Err(FragmentResourceError::Budget(
                ElementBudgetError::InvalidAmount { amount_g },
            ));
        }

        let location_id_owned = location_id.to_string();
        let (location_pos, location_remaining) = {
            let location = self.locations.get(location_id).ok_or_else(|| {
                FragmentResourceError::LocationNotFound {
                    location_id: location_id_owned.clone(),
                }
            })?;
            let budget = location.fragment_budget.as_ref().ok_or_else(|| {
                FragmentResourceError::FragmentBudgetMissing {
                    location_id: location_id_owned.clone(),
                }
            })?;
            (location.pos, budget.get_remaining(kind))
        };

        if location_remaining < amount_g {
            return Err(FragmentResourceError::Budget(
                ElementBudgetError::Insufficient {
                    kind,
                    requested_g: amount_g,
                    remaining_g: location_remaining,
                },
            ));
        }

        let coord = chunk_coord_of(location_pos, space).ok_or_else(|| {
            FragmentResourceError::ChunkCoordUnavailable {
                location_id: location_id_owned.clone(),
            }
        })?;

        let chunk_remaining = self
            .chunk_resource_budgets
            .get(&coord)
            .ok_or(FragmentResourceError::ChunkBudgetMissing { coord })?
            .get_remaining(kind);
        if chunk_remaining < amount_g {
            return Err(FragmentResourceError::Budget(
                ElementBudgetError::Insufficient {
                    kind,
                    requested_g: amount_g,
                    remaining_g: chunk_remaining,
                },
            ));
        }

        {
            let location = self.locations.get_mut(location_id).ok_or_else(|| {
                FragmentResourceError::LocationNotFound {
                    location_id: location_id_owned.clone(),
                }
            })?;
            let fragment_budget = location.fragment_budget.as_mut().ok_or_else(|| {
                FragmentResourceError::FragmentBudgetMissing {
                    location_id: location_id_owned.clone(),
                }
            })?;
            fragment_budget
                .consume(kind, amount_g)
                .map_err(FragmentResourceError::Budget)?;
        }

        let chunk_budget = self
            .chunk_resource_budgets
            .get_mut(&coord)
            .ok_or(FragmentResourceError::ChunkBudgetMissing { coord })?;
        chunk_budget
            .consume(kind, amount_g)
            .map_err(FragmentResourceError::Budget)?;

        if chunk_budget.is_exhausted() {
            self.chunks.insert(coord, ChunkState::Exhausted);
        }

        Ok(amount_g)
    }
}

// ============================================================================
// World Configuration
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WorldConfig {
    pub visibility_range_cm: i64,
    pub move_cost_per_km_electricity: i64,
    pub space: SpaceConfig,
    /// Power system configuration.
    pub power: PowerConfig,
    /// Physics configuration (radiation/thermal/erosion).
    pub physics: PhysicsConfig,
    /// Economy configuration (refine/manufacture minimal loop).
    pub economy: EconomyConfig,
    /// Asteroid fragment belt generation configuration.
    pub asteroid_fragment: AsteroidFragmentConfig,
}

impl Default for WorldConfig {
    fn default() -> Self {
        Self {
            visibility_range_cm: DEFAULT_VISIBILITY_RANGE_CM,
            move_cost_per_km_electricity: DEFAULT_MOVE_COST_PER_KM_ELECTRICITY,
            space: SpaceConfig::default(),
            power: PowerConfig::default(),
            physics: PhysicsConfig::default(),
            economy: EconomyConfig::default(),
            asteroid_fragment: AsteroidFragmentConfig::default(),
        }
    }
}

impl WorldConfig {
    pub fn sanitized(mut self) -> Self {
        if self.visibility_range_cm < 0 {
            self.visibility_range_cm = 0;
        }
        if self.move_cost_per_km_electricity < 0 {
            self.move_cost_per_km_electricity = 0;
        }
        self.space = self.space.sanitized();
        if self.power.transfer_loss_per_km_bps < 0 {
            self.power.transfer_loss_per_km_bps = 0;
        }
        if self.power.transfer_max_distance_km < 0 {
            self.power.transfer_max_distance_km = 0;
        }
        if self.power.market_base_price_per_pu < 0 {
            self.power.market_base_price_per_pu = 0;
        }
        if self.power.market_price_min_per_pu < 0 {
            self.power.market_price_min_per_pu = 0;
        }
        if self.power.market_price_max_per_pu < self.power.market_price_min_per_pu {
            self.power.market_price_max_per_pu = self.power.market_price_min_per_pu;
        }
        if self.power.market_scarcity_price_max_bps < 0 {
            self.power.market_scarcity_price_max_bps = 0;
        }
        if self.power.market_distance_price_per_km_bps < 0 {
            self.power.market_distance_price_per_km_bps = 0;
        }
        if self.power.market_price_band_bps < 0 {
            self.power.market_price_band_bps = 0;
        }
        self.physics = self.physics.sanitized();
        self.economy = self.economy.sanitized();
        self.asteroid_fragment = self.asteroid_fragment.sanitized();
        self
    }

    pub fn movement_cost(&self, distance_cm: i64) -> i64 {
        let per_km_cost = calibrated_move_cost_per_km(
            self.move_cost_per_km_electricity,
            self.physics.time_step_s,
            self.physics.power_unit_j,
        );
        movement_cost(distance_cm, per_km_cost)
    }
}

// ============================================================================
// Space Configuration
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpaceConfig {
    pub width_cm: i64,
    pub depth_cm: i64,
    pub height_cm: i64,
}

// ============================================================================
// Physics Configuration
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PhysicsConfig {
    pub time_step_s: i64,
    pub power_unit_j: i64,
    pub max_move_distance_cm_per_tick: i64,
    pub max_move_speed_cm_per_s: i64,
    pub radiation_floor: i64,
    pub radiation_floor_cap_per_tick: i64,
    pub radiation_decay_k: f64,
    pub max_harvest_per_tick: i64,
    pub thermal_capacity: i64,
    pub thermal_dissipation: i64,
    pub thermal_dissipation_gradient_bps: i64,
    pub heat_factor: i64,
    pub erosion_rate: f64,
}

impl Default for PhysicsConfig {
    fn default() -> Self {
        Self {
            time_step_s: MOVE_COST_REFERENCE_TIME_STEP_S,
            power_unit_j: MOVE_COST_REFERENCE_POWER_UNIT_J,
            max_move_distance_cm_per_tick: 1_000_000,
            max_move_speed_cm_per_s: 100_000,
            radiation_floor: 1,
            radiation_floor_cap_per_tick: 5,
            radiation_decay_k: 1e-6,
            max_harvest_per_tick: 50,
            thermal_capacity: 100,
            thermal_dissipation: 5,
            thermal_dissipation_gradient_bps: 10_000,
            heat_factor: 1,
            erosion_rate: 1e-6,
        }
    }
}

impl PhysicsConfig {
    pub fn sanitized(mut self) -> Self {
        if self.time_step_s < 0 {
            self.time_step_s = 0;
        }
        if self.power_unit_j < 0 {
            self.power_unit_j = 0;
        }
        if self.max_move_distance_cm_per_tick < 0 {
            self.max_move_distance_cm_per_tick = 0;
        }
        if self.max_move_speed_cm_per_s < 0 {
            self.max_move_speed_cm_per_s = 0;
        }
        if self.radiation_floor < 0 {
            self.radiation_floor = 0;
        }
        if self.radiation_floor_cap_per_tick < 0 {
            self.radiation_floor_cap_per_tick = 0;
        }
        if self.radiation_decay_k < 0.0 {
            self.radiation_decay_k = 0.0;
        }
        if self.max_harvest_per_tick < 0 {
            self.max_harvest_per_tick = 0;
        }
        if self.thermal_capacity < 0 {
            self.thermal_capacity = 0;
        }
        if self.thermal_dissipation < 0 {
            self.thermal_dissipation = 0;
        }
        if self.thermal_dissipation_gradient_bps < 0 {
            self.thermal_dissipation_gradient_bps = 0;
        }
        if self.heat_factor < 0 {
            self.heat_factor = 0;
        }
        if self.erosion_rate < 0.0 {
            self.erosion_rate = 0.0;
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EconomyConfig {
    pub mine_electricity_cost_per_kg: i64,
    pub mine_compound_max_per_action_g: i64,
    pub mine_compound_max_per_location_g: i64,
    pub refine_electricity_cost_per_kg: i64,
    pub refine_hardware_yield_ppm: i64,
    pub factory_build_electricity_cost: i64,
    pub factory_build_hardware_cost: i64,
    pub radiation_power_plant_output_per_tick: i64,
    pub recipe_electricity_cost_per_batch: i64,
    pub recipe_hardware_cost_per_batch: i64,
    pub recipe_data_output_per_batch: i64,
}

impl Default for EconomyConfig {
    fn default() -> Self {
        Self {
            mine_electricity_cost_per_kg: 1,
            mine_compound_max_per_action_g: 5_000,
            mine_compound_max_per_location_g: 8_000,
            refine_electricity_cost_per_kg: 2,
            refine_hardware_yield_ppm: 1_000,
            factory_build_electricity_cost: 10,
            factory_build_hardware_cost: 5,
            radiation_power_plant_output_per_tick: 8,
            recipe_electricity_cost_per_batch: 6,
            recipe_hardware_cost_per_batch: 2,
            recipe_data_output_per_batch: 1,
        }
    }
}

impl EconomyConfig {
    pub fn sanitized(mut self) -> Self {
        if self.mine_electricity_cost_per_kg < 0 {
            self.mine_electricity_cost_per_kg = 0;
        }
        if self.mine_compound_max_per_action_g < 0 {
            self.mine_compound_max_per_action_g = 0;
        }
        if self.mine_compound_max_per_location_g < 0 {
            self.mine_compound_max_per_location_g = 0;
        }
        if self.mine_compound_max_per_location_g > 0
            && self.mine_compound_max_per_action_g > self.mine_compound_max_per_location_g
        {
            self.mine_compound_max_per_action_g = self.mine_compound_max_per_location_g;
        }
        if self.refine_electricity_cost_per_kg < 0 {
            self.refine_electricity_cost_per_kg = 0;
        }
        self.refine_hardware_yield_ppm = self.refine_hardware_yield_ppm.clamp(0, PPM_BASE);
        if self.factory_build_electricity_cost < 0 {
            self.factory_build_electricity_cost = 0;
        }
        if self.factory_build_hardware_cost < 0 {
            self.factory_build_hardware_cost = 0;
        }
        if self.radiation_power_plant_output_per_tick < 0 {
            self.radiation_power_plant_output_per_tick = 0;
        }
        if self.recipe_electricity_cost_per_batch < 0 {
            self.recipe_electricity_cost_per_batch = 0;
        }
        if self.recipe_hardware_cost_per_batch < 0 {
            self.recipe_hardware_cost_per_batch = 0;
        }
        if self.recipe_data_output_per_batch < 0 {
            self.recipe_data_output_per_batch = 0;
        }
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ThermalStatus {
    pub heat: i64,
}

// ============================================================================
// Asteroid Fragment Generator Configuration
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AsteroidFragmentConfig {
    pub base_density_per_km3: f64,
    pub voxel_size_km: i64,
    pub cluster_noise: f64,
    pub layer_scale_height_km: f64,
    pub size_powerlaw_q: f64,
    pub radiation_emission_scale: f64,
    pub radiation_radius_exponent: f64,
    pub radius_min_cm: i64,
    pub radius_max_cm: i64,
    pub min_fragment_spacing_cm: i64,
    pub min_fragments_per_chunk: i64,
    pub starter_core_radius_ratio: f64,
    pub starter_core_density_multiplier: f64,
    pub replenish_interval_ticks: i64,
    pub replenish_percent_ppm: i64,
    pub material_distribution_strategy: MaterialDistributionStrategy,
    pub max_fragments_per_chunk: i64,
    pub max_blocks_per_fragment: i64,
    pub max_blocks_per_chunk: i64,
    pub material_weights: MaterialWeights,
    pub material_radiation_factors: MaterialRadiationFactors,
}

impl Default for AsteroidFragmentConfig {
    fn default() -> Self {
        Self {
            base_density_per_km3: 0.001,
            voxel_size_km: 10,
            cluster_noise: 0.5,
            layer_scale_height_km: 2.0,
            size_powerlaw_q: 3.0,
            radiation_emission_scale: 1e-12,
            radiation_radius_exponent: 3.0,
            radius_min_cm: 25_000,
            radius_max_cm: 500_000,
            min_fragment_spacing_cm: 50_000,
            min_fragments_per_chunk: 6,
            starter_core_radius_ratio: 0.35,
            starter_core_density_multiplier: 1.6,
            replenish_interval_ticks: 100,
            replenish_percent_ppm: 10_000,
            material_distribution_strategy: MaterialDistributionStrategy::Uniform,
            max_fragments_per_chunk: 4_000,
            max_blocks_per_fragment: 64,
            max_blocks_per_chunk: 120_000,
            material_weights: MaterialWeights::default(),
            material_radiation_factors: MaterialRadiationFactors::default(),
        }
    }
}

impl AsteroidFragmentConfig {
    pub fn sanitized(mut self) -> Self {
        if self.base_density_per_km3 < 0.0 {
            self.base_density_per_km3 = 0.0;
        }
        if self.voxel_size_km <= 0 {
            self.voxel_size_km = 1;
        }
        if self.cluster_noise < 0.0 {
            self.cluster_noise = 0.0;
        }
        if self.layer_scale_height_km < 0.0 {
            self.layer_scale_height_km = 0.0;
        }
        if self.size_powerlaw_q <= 0.0 {
            self.size_powerlaw_q = 1.0;
        }
        if !self.radiation_emission_scale.is_finite() || self.radiation_emission_scale < 0.0 {
            self.radiation_emission_scale = 0.0;
        }
        if !self.radiation_radius_exponent.is_finite() || self.radiation_radius_exponent < 0.0 {
            self.radiation_radius_exponent = 0.0;
        }
        if self.radius_min_cm < 0 {
            self.radius_min_cm = 0;
        }
        if self.radius_max_cm < self.radius_min_cm {
            self.radius_max_cm = self.radius_min_cm;
        }
        if self.min_fragment_spacing_cm < 0 {
            self.min_fragment_spacing_cm = 0;
        }
        if self.min_fragments_per_chunk < 0 {
            self.min_fragments_per_chunk = 0;
        }
        if !self.starter_core_radius_ratio.is_finite() {
            self.starter_core_radius_ratio = 0.0;
        }
        self.starter_core_radius_ratio = self.starter_core_radius_ratio.clamp(0.0, 1.0);
        if !self.starter_core_density_multiplier.is_finite()
            || self.starter_core_density_multiplier < 1.0
        {
            self.starter_core_density_multiplier = 1.0;
        }
        if self.replenish_interval_ticks < 0 {
            self.replenish_interval_ticks = 0;
        }
        self.replenish_percent_ppm = self.replenish_percent_ppm.clamp(0, PPM_BASE);
        if self.max_fragments_per_chunk < 0 {
            self.max_fragments_per_chunk = 0;
        }
        if self.min_fragments_per_chunk > self.max_fragments_per_chunk {
            self.min_fragments_per_chunk = self.max_fragments_per_chunk;
        }
        if self.max_blocks_per_fragment < 0 {
            self.max_blocks_per_fragment = 0;
        }
        if self.max_blocks_per_chunk < 0 {
            self.max_blocks_per_chunk = 0;
        }
        self.material_weights = self.material_weights.sanitized();
        self.material_radiation_factors = self.material_radiation_factors.sanitized();
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialDistributionStrategy {
    Uniform,
    CoreMetalRimVolatile,
}

impl Default for MaterialDistributionStrategy {
    fn default() -> Self {
        Self::Uniform
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MaterialWeights {
    pub silicate: u32,
    pub metal: u32,
    pub ice: u32,
    pub carbon: u32,
    pub composite: u32,
}

impl Default for MaterialWeights {
    fn default() -> Self {
        Self {
            silicate: 52,
            metal: 8,
            ice: 18,
            carbon: 18,
            composite: 4,
        }
    }
}

impl MaterialWeights {
    pub fn sanitized(mut self) -> Self {
        if self.silicate == 0
            && self.metal == 0
            && self.ice == 0
            && self.carbon == 0
            && self.composite == 0
        {
            self.silicate = 1;
        }
        self
    }

    pub fn total(&self) -> u32 {
        self.silicate + self.metal + self.ice + self.carbon + self.composite
    }

    pub fn pick(&self, roll: u32) -> MaterialKind {
        let mut acc = self.silicate;
        if roll < acc {
            return MaterialKind::Silicate;
        }
        acc += self.metal;
        if roll < acc {
            return MaterialKind::Metal;
        }
        acc += self.ice;
        if roll < acc {
            return MaterialKind::Ice;
        }
        acc += self.carbon;
        if roll < acc {
            return MaterialKind::Carbon;
        }
        MaterialKind::Composite
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MaterialRadiationFactors {
    pub silicate_bps: u32,
    pub metal_bps: u32,
    pub ice_bps: u32,
    pub carbon_bps: u32,
    pub composite_bps: u32,
}

impl Default for MaterialRadiationFactors {
    fn default() -> Self {
        Self {
            silicate_bps: 7_500,
            metal_bps: 13_000,
            ice_bps: 4_500,
            carbon_bps: 6_000,
            composite_bps: 11_000,
        }
    }
}

impl MaterialRadiationFactors {
    pub fn sanitized(mut self) -> Self {
        self.silicate_bps = self.silicate_bps.min(50_000);
        self.metal_bps = self.metal_bps.min(50_000);
        self.ice_bps = self.ice_bps.min(50_000);
        self.carbon_bps = self.carbon_bps.min(50_000);
        self.composite_bps = self.composite_bps.min(50_000);

        if self.silicate_bps == 0
            && self.metal_bps == 0
            && self.ice_bps == 0
            && self.carbon_bps == 0
            && self.composite_bps == 0
        {
            self.silicate_bps = 1;
        }
        self
    }

    pub fn factor_for(self, material: MaterialKind) -> f64 {
        let bps = match material {
            MaterialKind::Silicate => self.silicate_bps,
            MaterialKind::Metal => self.metal_bps,
            MaterialKind::Ice => self.ice_bps,
            MaterialKind::Carbon => self.carbon_bps,
            MaterialKind::Composite => self.composite_bps,
        };
        bps as f64 / 10_000.0
    }
}

impl Default for SpaceConfig {
    fn default() -> Self {
        Self {
            width_cm: DEFAULT_CLOUD_WIDTH_CM,
            depth_cm: DEFAULT_CLOUD_DEPTH_CM,
            height_cm: DEFAULT_CLOUD_HEIGHT_CM,
        }
    }
}

impl SpaceConfig {
    pub fn sanitized(mut self) -> Self {
        if self.width_cm < 0 {
            self.width_cm = 0;
        }
        if self.depth_cm < 0 {
            self.depth_cm = 0;
        }
        if self.height_cm < 0 {
            self.height_cm = 0;
        }
        self
    }

    pub fn contains(&self, pos: GeoPos) -> bool {
        let max_x = self.width_cm as f64;
        let max_y = self.depth_cm as f64;
        let max_z = self.height_cm as f64;
        pos.x_cm >= 0.0
            && pos.x_cm <= max_x
            && pos.y_cm >= 0.0
            && pos.y_cm <= max_y
            && pos.z_cm >= 0.0
            && pos.z_cm <= max_z
    }
}

fn calibrated_move_cost_per_km(base_per_km_cost: i64, time_step_s: i64, power_unit_j: i64) -> i64 {
    if base_per_km_cost <= 0 {
        return 0;
    }

    let time_step_s = time_step_s.max(1) as i128;
    let power_unit_j = power_unit_j.max(1) as i128;
    let scaled = (base_per_km_cost as i128)
        .saturating_mul(time_step_s)
        .saturating_mul(MOVE_COST_REFERENCE_POWER_UNIT_J as i128);
    let denom = (MOVE_COST_REFERENCE_TIME_STEP_S as i128).saturating_mul(power_unit_j);
    let adjusted = scaled
        .saturating_add(denom.saturating_sub(1))
        .saturating_div(denom);
    adjusted.clamp(0, i64::MAX as i128) as i64
}

/// Calculate movement cost based on distance and per-km cost.
pub fn movement_cost(distance_cm: i64, per_km_cost: i64) -> i64 {
    if distance_cm <= 0 || per_km_cost <= 0 {
        return 0;
    }
    let km = (distance_cm + CM_PER_KM - 1) / CM_PER_KM;
    km.saturating_mul(per_km_cost)
}
