//! Chain-side resource manifest and delta schema.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
#[cfg(target_arch = "wasm32")]
use std::fmt;

#[cfg(not(target_arch = "wasm32"))]
use crate::geometry::DEFAULT_CLOUD_WIDTH_CM;
#[cfg(not(target_arch = "wasm32"))]
use crate::runtime::WorldState;
#[cfg(not(target_arch = "wasm32"))]
use crate::runtime::{ActionId, MaterialLedgerId, WorldEventId, WorldTime};
#[cfg(not(target_arch = "wasm32"))]
use crate::simulator::SpaceConfig;
use crate::simulator::{
    ChunkCoord, ChunkGenerationCause, ChunkRuntimeConfig, ChunkState, FragmentElementKind,
    ResourceKind, WorldConfig, WorldEvent, WorldEventKind, WorldModel, chunk_coord_of, chunk_seed,
};

#[cfg(target_arch = "wasm32")]
pub type WorldTime = u64;
#[cfg(target_arch = "wasm32")]
pub type WorldEventId = u64;
#[cfg(target_arch = "wasm32")]
pub type ActionId = u64;

#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(try_from = "String", into = "String")]
pub enum MaterialLedgerId {
    World,
    Agent(String),
    Site(String),
    Factory(String),
}

#[cfg(target_arch = "wasm32")]
impl Default for MaterialLedgerId {
    fn default() -> Self {
        Self::World
    }
}

#[cfg(target_arch = "wasm32")]
impl fmt::Display for MaterialLedgerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MaterialLedgerId::World => write!(f, "world"),
            MaterialLedgerId::Agent(agent_id) => write!(f, "agent:{agent_id}"),
            MaterialLedgerId::Site(site_id) => write!(f, "site:{site_id}"),
            MaterialLedgerId::Factory(factory_id) => write!(f, "factory:{factory_id}"),
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl From<MaterialLedgerId> for String {
    fn from(value: MaterialLedgerId) -> Self {
        value.to_string()
    }
}

#[cfg(target_arch = "wasm32")]
impl TryFrom<String> for MaterialLedgerId {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value == "world" {
            return Ok(MaterialLedgerId::World);
        }
        if let Some(rest) = value.strip_prefix("agent:") {
            if rest.trim().is_empty() {
                return Err("agent ledger id cannot be empty".to_string());
            }
            return Ok(MaterialLedgerId::Agent(rest.to_string()));
        }
        if let Some(rest) = value.strip_prefix("site:") {
            if rest.trim().is_empty() {
                return Err("site ledger id cannot be empty".to_string());
            }
            return Ok(MaterialLedgerId::Site(rest.to_string()));
        }
        if let Some(rest) = value.strip_prefix("factory:") {
            if rest.trim().is_empty() {
                return Err("factory ledger id cannot be empty".to_string());
            }
            return Ok(MaterialLedgerId::Factory(rest.to_string()));
        }
        Err(format!("invalid material ledger id: {value}"))
    }
}

pub const CHAIN_RESOURCE_MANIFEST_SCHEMA_V1: &str = "oasis7.world_resource_manifest.v1";
pub const CHAIN_RESOURCE_DELTA_SCHEMA_V1: &str = "oasis7.world_resource_delta.v1";
pub const CHUNK_GENERATION_SCHEMA_V1: &str = "oasis7.chunk_generation.v1";

fn hash_json<T: Serialize>(value: &T) -> Result<String, serde_json::Error> {
    let bytes = serde_json::to_vec(value)?;
    Ok(sha256_hex(&bytes))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainResourceManifest {
    pub schema_version: String,
    pub world_id: String,
    pub chain_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub genesis_ref: Option<String>,
    pub world_seed: u64,
    pub chunk_generation_schema_version: String,
    #[serde(default)]
    pub world_config_hash: String,
    #[serde(default)]
    pub generation_algorithm_id: String,
    #[serde(default)]
    pub generation_algorithm_hash: String,
    pub created_at_height: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at_block_hash: Option<String>,
    pub manifest_height: u64,
    pub manifest_hash: String,
    pub resource_balances: BTreeMap<ResourceKind, i64>,
    pub material_ledgers: BTreeMap<MaterialLedgerId, BTreeMap<String, i64>>,
    #[serde(default)]
    pub generated_chunks: BTreeMap<String, ChainChunkResourceManifestEntry>,
}

impl ChainResourceManifest {
    pub fn empty_at_height(world_id: impl Into<String>, world_seed: u64, height: u64) -> Self {
        let mut manifest = Self {
            schema_version: CHAIN_RESOURCE_MANIFEST_SCHEMA_V1.to_string(),
            world_id: world_id.into(),
            chain_id: "unbound".to_string(),
            genesis_ref: None,
            world_seed,
            chunk_generation_schema_version: CHUNK_GENERATION_SCHEMA_V1.to_string(),
            world_config_hash: String::new(),
            generation_algorithm_id: "runtime_ledger_v1".to_string(),
            generation_algorithm_hash: String::new(),
            created_at_height: height,
            created_at_block_hash: None,
            manifest_height: height,
            manifest_hash: String::new(),
            resource_balances: BTreeMap::new(),
            material_ledgers: BTreeMap::new(),
            generated_chunks: BTreeMap::new(),
        };
        manifest.manifest_hash = manifest.canonical_hash();
        manifest
    }

    pub fn canonical_hash(&self) -> String {
        let mut clone = self.clone();
        clone.manifest_hash.clear();
        hash_json(&clone).unwrap_or_else(|_| String::new())
    }

    pub fn refresh_hashes(&mut self) {
        for entry in self.generated_chunks.values_mut() {
            entry.manifest_hash = entry.canonical_hash();
        }
        self.manifest_hash = self.canonical_hash();
    }

    pub fn is_schema_current(&self) -> bool {
        self.schema_version == CHAIN_RESOURCE_MANIFEST_SCHEMA_V1
            && self.manifest_hash == self.canonical_hash()
    }

    pub fn from_simulator_state(
        context: ChainResourceDerivationContext<'_>,
        model: &WorldModel,
        config: &WorldConfig,
        chunk_runtime: &ChunkRuntimeConfig,
        journal: &[WorldEvent],
    ) -> Self {
        let world_seed = chunk_runtime.world_seed;
        let world_config_hash = hash_json(config).unwrap_or_default();
        let generation_algorithm_hash = hash_json(&(config, chunk_runtime)).unwrap_or_default();
        let mut generated_chunks = BTreeMap::new();
        for (coord, state) in &model.chunks {
            if !matches!(state, ChunkState::Generated | ChunkState::Exhausted) {
                continue;
            }
            let default_chunk_budget = Default::default();
            let chunk_budget = model
                .chunk_resource_budgets
                .get(coord)
                .unwrap_or(&default_chunk_budget);
            let mut fragment_refs = Vec::new();
            let mut block_count = 0_u32;
            for location in model.locations.values() {
                if chunk_coord_of(location.pos, &config.space) != Some(*coord) {
                    continue;
                }
                let Some(fragment_profile) = location.fragment_profile.as_ref() else {
                    continue;
                };
                block_count =
                    block_count.saturating_add(fragment_profile.blocks.blocks.len() as u32);
                fragment_refs.push(ChainFragmentResourceRef {
                    fragment_id: location.id.clone(),
                    location_id: location.id.clone(),
                    profile_hash: hash_json(fragment_profile).unwrap_or_default(),
                    budget_total_hash: location
                        .fragment_budget
                        .as_ref()
                        .map(|budget| hash_json(&budget.total_by_element_g).unwrap_or_default())
                        .unwrap_or_default(),
                    budget_remaining_hash: location
                        .fragment_budget
                        .as_ref()
                        .map(|budget| hash_json(&budget.remaining_by_element_g).unwrap_or_default())
                        .unwrap_or_default(),
                });
            }
            let commit_ref = latest_chunk_commit_ref(
                journal,
                *coord,
                context.manifest_height,
                context.commit_block_hash,
            );
            let chunk_seed_base = if chunk_runtime.asteroid_fragment_enabled {
                chunk_runtime.asteroid_fragment_seed()
            } else {
                world_seed
            };
            let mut entry = ChainChunkResourceManifestEntry {
                schema_version: CHAIN_RESOURCE_MANIFEST_SCHEMA_V1.to_string(),
                world_id: context.world_id.to_string(),
                chain_id: context.chain_id.to_string(),
                world_seed,
                chunk_generation_schema_version: CHUNK_GENERATION_SCHEMA_V1.to_string(),
                coord: *coord,
                seed: chunk_seed(chunk_seed_base, *coord),
                chunk_status: if matches!(state, ChunkState::Exhausted) {
                    ChainChunkResourceStatus::Exhausted
                } else {
                    ChainChunkResourceStatus::Committed
                },
                fragment_count: fragment_refs.len() as u32,
                block_count,
                fragment_refs,
                chunk_budget_total_hash: hash_json(&chunk_budget.total_by_element_g)
                    .unwrap_or_default(),
                chunk_budget_remaining_hash: hash_json(&chunk_budget.remaining_by_element_g)
                    .unwrap_or_default(),
                total_by_element_g: chunk_budget.total_by_element_g.clone(),
                remaining_by_element_g: chunk_budget.remaining_by_element_g.clone(),
                commit_ref,
                manifest_hash: String::new(),
            };
            entry.manifest_hash = entry.canonical_hash();
            generated_chunks.insert(chunk_key(*coord), entry);
        }
        let mut manifest = Self {
            schema_version: CHAIN_RESOURCE_MANIFEST_SCHEMA_V1.to_string(),
            world_id: context.world_id.to_string(),
            chain_id: context.chain_id.to_string(),
            genesis_ref: context.genesis_ref.map(ToOwned::to_owned),
            world_seed,
            chunk_generation_schema_version: CHUNK_GENERATION_SCHEMA_V1.to_string(),
            world_config_hash,
            generation_algorithm_id: "simulator_chunk_fragment_v1".to_string(),
            generation_algorithm_hash,
            created_at_height: context.created_at_height,
            created_at_block_hash: context.commit_block_hash.map(ToOwned::to_owned),
            manifest_height: context.manifest_height,
            manifest_hash: String::new(),
            resource_balances: aggregate_agent_resource_balances(model),
            material_ledgers: BTreeMap::new(),
            generated_chunks,
        };
        manifest.manifest_hash = manifest.canonical_hash();
        manifest
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_runtime_state(
        context: ChainResourceDerivationContext<'_>,
        world_config_hash: impl Into<String>,
        generation_algorithm_hash: impl Into<String>,
        state: &WorldState,
    ) -> Self {
        let world_config_hash = world_config_hash.into();
        let generation_algorithm_hash = generation_algorithm_hash.into();
        let world_seed = runtime_world_seed(
            context.world_id,
            context.chain_id,
            world_config_hash.as_str(),
        );
        let mut generated_chunks = BTreeMap::new();
        for (chunk_key, agents) in runtime_agents_by_chunk(state) {
            let coord = agents
                .first()
                .map(|(_, coord)| *coord)
                .unwrap_or(ChunkCoord { x: 0, y: 0, z: 0 });
            let mut fragment_refs = Vec::new();
            for (agent_id, _) in agents {
                fragment_refs.push(ChainFragmentResourceRef {
                    fragment_id: format!("runtime-agent:{agent_id}"),
                    location_id: format!("runtime-agent:{agent_id}"),
                    profile_hash: hash_json(agent_id).unwrap_or_default(),
                    budget_total_hash: hash_json(&state.resources).unwrap_or_default(),
                    budget_remaining_hash: hash_json(&state.resources).unwrap_or_default(),
                });
            }
            let mut entry = ChainChunkResourceManifestEntry {
                schema_version: CHAIN_RESOURCE_MANIFEST_SCHEMA_V1.to_string(),
                world_id: context.world_id.to_string(),
                chain_id: context.chain_id.to_string(),
                world_seed,
                chunk_generation_schema_version: CHUNK_GENERATION_SCHEMA_V1.to_string(),
                coord,
                seed: chunk_seed(world_seed, coord),
                chunk_status: ChainChunkResourceStatus::Committed,
                fragment_count: fragment_refs.len() as u32,
                block_count: fragment_refs.len() as u32,
                fragment_refs,
                chunk_budget_total_hash: hash_json(&state.resources).unwrap_or_default(),
                chunk_budget_remaining_hash: hash_json(&state.resources).unwrap_or_default(),
                total_by_element_g: BTreeMap::new(),
                remaining_by_element_g: BTreeMap::new(),
                commit_ref: ChainResourceCommitRef {
                    height: context.manifest_height,
                    block_hash: context.commit_block_hash.map(ToOwned::to_owned),
                    event_id: None,
                    action_id: None,
                },
                manifest_hash: String::new(),
            };
            entry.manifest_hash = entry.canonical_hash();
            generated_chunks.insert(chunk_key, entry);
        }
        let mut manifest = Self {
            schema_version: CHAIN_RESOURCE_MANIFEST_SCHEMA_V1.to_string(),
            world_id: context.world_id.to_string(),
            chain_id: context.chain_id.to_string(),
            genesis_ref: context.genesis_ref.map(ToOwned::to_owned),
            world_seed,
            chunk_generation_schema_version: CHUNK_GENERATION_SCHEMA_V1.to_string(),
            world_config_hash,
            generation_algorithm_id: "runtime_agent_chunk_v1".to_string(),
            generation_algorithm_hash,
            created_at_height: context.created_at_height,
            created_at_block_hash: context.commit_block_hash.map(ToOwned::to_owned),
            manifest_height: context.manifest_height,
            manifest_hash: String::new(),
            resource_balances: aggregate_runtime_resource_balances(state),
            material_ledgers: state.material_ledgers.clone(),
            generated_chunks,
        };
        manifest.manifest_hash = manifest.canonical_hash();
        manifest
    }
}

impl Default for ChainResourceManifest {
    fn default() -> Self {
        Self::empty_at_height("unbound", 0, 0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainChunkResourceManifestEntry {
    pub schema_version: String,
    pub world_id: String,
    pub chain_id: String,
    pub world_seed: u64,
    pub chunk_generation_schema_version: String,
    pub coord: ChunkCoord,
    pub seed: u64,
    pub chunk_status: ChainChunkResourceStatus,
    pub fragment_count: u32,
    pub block_count: u32,
    #[serde(default)]
    pub fragment_refs: Vec<ChainFragmentResourceRef>,
    #[serde(default)]
    pub chunk_budget_total_hash: String,
    #[serde(default)]
    pub chunk_budget_remaining_hash: String,
    pub total_by_element_g: BTreeMap<FragmentElementKind, i64>,
    pub remaining_by_element_g: BTreeMap<FragmentElementKind, i64>,
    #[serde(default)]
    pub commit_ref: ChainResourceCommitRef,
    #[serde(default)]
    pub manifest_hash: String,
}

impl ChainChunkResourceManifestEntry {
    pub fn canonical_hash(&self) -> String {
        let mut clone = self.clone();
        clone.manifest_hash.clear();
        hash_json(&clone).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChainChunkResourceStatus {
    #[default]
    Committed,
    ChainPending,
    Provisional,
    Exhausted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ChainFragmentResourceRef {
    pub fragment_id: String,
    pub location_id: String,
    pub profile_hash: String,
    pub budget_total_hash: String,
    pub budget_remaining_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ChainResourceCommitRef {
    pub height: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<WorldEventId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_id: Option<ActionId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainResourceDelta {
    pub schema_version: String,
    pub world_id: String,
    pub chain_id: String,
    pub delta_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_id: Option<ActionId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_id: Option<WorldEventId>,
    pub ordering_key: ChainResourceOrderingKey,
    pub base_manifest_hash: String,
    pub resulting_manifest_hash: String,
    pub block_height: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit_block_hash: Option<String>,
    pub tick: WorldTime,
    pub source: ChainResourceDeltaSource,
    pub replay_status: ChainResourceReplayStatus,
    pub entries: Vec<ChainResourceDeltaEntry>,
}

impl ChainResourceDelta {
    pub fn empty_for_manifest(
        world_id: impl Into<String>,
        manifest_hash: impl Into<String>,
        block_height: u64,
        tick: WorldTime,
    ) -> Self {
        let manifest_hash = manifest_hash.into();
        Self {
            schema_version: CHAIN_RESOURCE_DELTA_SCHEMA_V1.to_string(),
            world_id: world_id.into(),
            chain_id: "unbound".to_string(),
            delta_id: format!("resource-delta-{block_height}-{tick}"),
            action_id: None,
            event_id: None,
            ordering_key: ChainResourceOrderingKey {
                height: block_height,
                event_sequence: 0,
                action_sequence: 0,
            },
            base_manifest_hash: manifest_hash.clone(),
            resulting_manifest_hash: manifest_hash,
            block_height,
            commit_block_hash: None,
            tick,
            source: ChainResourceDeltaSource::Genesis,
            replay_status: ChainResourceReplayStatus::Committed,
            entries: Vec::new(),
        }
    }

    pub fn is_schema_current(&self) -> bool {
        self.schema_version == CHAIN_RESOURCE_DELTA_SCHEMA_V1
            && !self.base_manifest_hash.trim().is_empty()
            && !self.resulting_manifest_hash.trim().is_empty()
    }

    pub fn latest_from_simulator_journal(
        context: ChainResourceDerivationContext<'_>,
        manifest: &ChainResourceManifest,
        journal: &[WorldEvent],
    ) -> Self {
        for event in journal.iter().rev() {
            if let Some(delta) = delta_from_simulator_event(context, manifest, event) {
                return delta;
            }
        }
        let mut delta = Self::empty_for_manifest(
            context.world_id,
            manifest.manifest_hash.clone(),
            context.manifest_height,
            context.tick,
        );
        delta.chain_id = context.chain_id.to_string();
        delta.commit_block_hash = context.commit_block_hash.map(ToOwned::to_owned);
        delta
    }

    pub fn latest_from_runtime_manifest(
        context: ChainResourceDerivationContext<'_>,
        manifest: &ChainResourceManifest,
    ) -> Self {
        let mut entries = Vec::new();
        for chunk in manifest.generated_chunks.values() {
            entries.push(ChainResourceDeltaEntry::ChunkResource {
                coord: chunk.coord,
                element: FragmentElementKind::Iron,
                total_delta_g: 0,
                remaining_delta_g: 0,
                resulting_remaining_g: 0,
                remaining_after_hash: chunk.chunk_budget_remaining_hash.clone(),
                chunk_remaining_after_hash: chunk.chunk_budget_remaining_hash.clone(),
            });
        }
        Self {
            schema_version: CHAIN_RESOURCE_DELTA_SCHEMA_V1.to_string(),
            world_id: context.world_id.to_string(),
            chain_id: context.chain_id.to_string(),
            delta_id: format!(
                "resource-delta-{}-{}",
                context.manifest_height, context.tick
            ),
            action_id: None,
            event_id: None,
            ordering_key: ChainResourceOrderingKey {
                height: context.manifest_height,
                event_sequence: 0,
                action_sequence: 0,
            },
            base_manifest_hash: if entries.is_empty() {
                manifest.manifest_hash.clone()
            } else {
                ChainResourceManifest::empty_at_height(
                    manifest.world_id.clone(),
                    manifest.world_seed,
                    manifest.created_at_height,
                )
                .canonical_hash()
            },
            resulting_manifest_hash: manifest.manifest_hash.clone(),
            block_height: context.manifest_height,
            commit_block_hash: context.commit_block_hash.map(ToOwned::to_owned),
            tick: context.tick,
            source: ChainResourceDeltaSource::Genesis,
            replay_status: ChainResourceReplayStatus::Committed,
            entries,
        }
    }
}

impl Default for ChainResourceDelta {
    fn default() -> Self {
        Self::empty_for_manifest(
            "unbound",
            ChainResourceManifest::default().manifest_hash,
            0,
            0,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ChainResourceDerivationContext<'a> {
    pub world_id: &'a str,
    pub chain_id: &'a str,
    pub genesis_ref: Option<&'a str>,
    pub created_at_height: u64,
    pub manifest_height: u64,
    pub commit_block_hash: Option<&'a str>,
    pub tick: WorldTime,
}

impl<'a> ChainResourceDerivationContext<'a> {
    pub fn simulator_default(tick: WorldTime) -> Self {
        Self {
            world_id: "simulator-world",
            chain_id: "simulator-chain",
            genesis_ref: None,
            created_at_height: 0,
            manifest_height: tick,
            commit_block_hash: None,
            tick,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainResourceOrderingKey {
    pub height: u64,
    pub event_sequence: u64,
    pub action_sequence: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChainResourceDeltaSource {
    ChunkGenerated,
    MineCompound,
    Replenish,
    #[default]
    Genesis,
    Migration,
}

fn latest_chunk_commit_ref(
    journal: &[WorldEvent],
    coord: ChunkCoord,
    height: u64,
    block_hash: Option<&str>,
) -> ChainResourceCommitRef {
    let event_id = journal.iter().rev().find_map(|event| match &event.kind {
        WorldEventKind::ChunkGenerated {
            coord: event_coord, ..
        } if *event_coord == coord => Some(event.id),
        _ => None,
    });
    ChainResourceCommitRef {
        height,
        block_hash: block_hash.map(ToOwned::to_owned),
        event_id,
        action_id: None,
    }
}

fn chunk_key(coord: ChunkCoord) -> String {
    format!("chunk:{}:{}:{}", coord.x, coord.y, coord.z)
}

fn aggregate_agent_resource_balances(model: &WorldModel) -> BTreeMap<ResourceKind, i64> {
    let mut balances: BTreeMap<ResourceKind, i64> = BTreeMap::new();
    for agent in model.agents.values() {
        for (kind, amount) in &agent.resources.amounts {
            let entry = balances.entry(*kind).or_insert(0);
            *entry = (*entry).saturating_add(*amount);
        }
    }
    balances
}

#[cfg(not(target_arch = "wasm32"))]
fn aggregate_runtime_resource_balances(state: &WorldState) -> BTreeMap<ResourceKind, i64> {
    let mut balances = state.resources.clone();
    for agent in state.agents.values() {
        for (kind, amount) in &agent.state.resources.amounts {
            let entry = balances.entry(*kind).or_insert(0);
            *entry = (*entry).saturating_add(*amount);
        }
    }
    balances
}

#[cfg(not(target_arch = "wasm32"))]
fn runtime_agents_by_chunk(state: &WorldState) -> BTreeMap<String, Vec<(&String, ChunkCoord)>> {
    let mut out: BTreeMap<String, Vec<(&String, ChunkCoord)>> = BTreeMap::new();
    let space = runtime_chunk_space_for_state(state);
    for (agent_id, agent) in &state.agents {
        let coord =
            chunk_coord_of(agent.state.pos, &space).unwrap_or(ChunkCoord { x: 0, y: 0, z: 0 });
        out.entry(chunk_key(coord))
            .or_default()
            .push((agent_id, coord));
    }
    out
}

#[cfg(not(target_arch = "wasm32"))]
fn runtime_chunk_space_for_state(state: &WorldState) -> SpaceConfig {
    let mut space = SpaceConfig::default();
    for agent in state.agents.values() {
        space.width_cm = space
            .width_cm
            .max(agent.state.pos.x_cm.saturating_add(DEFAULT_CLOUD_WIDTH_CM));
        space.depth_cm = space
            .depth_cm
            .max(agent.state.pos.y_cm.saturating_add(DEFAULT_CLOUD_WIDTH_CM));
        space.height_cm = space
            .height_cm
            .max(agent.state.pos.z_cm.saturating_add(DEFAULT_CLOUD_WIDTH_CM));
    }
    space
}

#[cfg(not(target_arch = "wasm32"))]
fn runtime_world_seed(world_id: &str, chain_id: &str, world_config_hash: &str) -> u64 {
    let hash = sha256_hex(format!("{world_id}:{chain_id}:{world_config_hash}").as_bytes());
    u64::from_str_radix(hash.get(0..16).unwrap_or("1"), 16)
        .unwrap_or(1)
        .max(1)
}

fn delta_from_simulator_event(
    context: ChainResourceDerivationContext<'_>,
    manifest: &ChainResourceManifest,
    event: &WorldEvent,
) -> Option<ChainResourceDelta> {
    let mut entries = Vec::new();
    let source = match &event.kind {
        WorldEventKind::ChunkGenerated {
            coord,
            chunk_budget,
            cause,
            ..
        } => {
            for (element, total) in &chunk_budget.total_by_element_g {
                let remaining = chunk_budget.get_remaining(*element);
                entries.push(ChainResourceDeltaEntry::ChunkResource {
                    coord: *coord,
                    element: *element,
                    total_delta_g: *total,
                    remaining_delta_g: remaining,
                    resulting_remaining_g: remaining,
                    remaining_after_hash: hash_json(&chunk_budget.remaining_by_element_g)
                        .unwrap_or_default(),
                    chunk_remaining_after_hash: hash_json(&chunk_budget.remaining_by_element_g)
                        .unwrap_or_default(),
                });
            }
            match cause {
                ChunkGenerationCause::Init => ChainResourceDeltaSource::Genesis,
                ChunkGenerationCause::Observe | ChunkGenerationCause::Action => {
                    ChainResourceDeltaSource::ChunkGenerated
                }
            }
        }
        WorldEventKind::FragmentsReplenished {
            entries: replenished,
        } => {
            for replenished_entry in replenished {
                let Some(fragment_budget) = replenished_entry.location.fragment_budget.as_ref()
                else {
                    continue;
                };
                for (element, total) in &fragment_budget.total_by_element_g {
                    let remaining = fragment_budget.get_remaining(*element);
                    entries.push(ChainResourceDeltaEntry::ChunkResource {
                        coord: replenished_entry.coord,
                        element: *element,
                        total_delta_g: *total,
                        remaining_delta_g: remaining,
                        resulting_remaining_g: remaining,
                        remaining_after_hash: hash_json(&fragment_budget.remaining_by_element_g)
                            .unwrap_or_default(),
                        chunk_remaining_after_hash: manifest
                            .generated_chunks
                            .get(&chunk_key(replenished_entry.coord))
                            .map(|entry| entry.chunk_budget_remaining_hash.clone())
                            .unwrap_or_default(),
                    });
                }
            }
            ChainResourceDeltaSource::Replenish
        }
        WorldEventKind::CompoundMined {
            location_id,
            extracted_elements,
            ..
        } => {
            let chunk = manifest.generated_chunks.values().find(|entry| {
                entry
                    .fragment_refs
                    .iter()
                    .any(|fragment_ref| fragment_ref.location_id == *location_id)
            })?;
            for (element, amount) in extracted_elements {
                let resulting_remaining = chunk
                    .remaining_by_element_g
                    .get(element)
                    .copied()
                    .unwrap_or(0);
                entries.push(ChainResourceDeltaEntry::ChunkResource {
                    coord: chunk.coord,
                    element: *element,
                    total_delta_g: 0,
                    remaining_delta_g: amount.saturating_neg(),
                    resulting_remaining_g: resulting_remaining,
                    remaining_after_hash: chunk.chunk_budget_remaining_hash.clone(),
                    chunk_remaining_after_hash: chunk.chunk_budget_remaining_hash.clone(),
                });
            }
            ChainResourceDeltaSource::MineCompound
        }
        _ => return None,
    };
    if entries.is_empty() {
        return None;
    }
    Some(ChainResourceDelta {
        schema_version: CHAIN_RESOURCE_DELTA_SCHEMA_V1.to_string(),
        world_id: context.world_id.to_string(),
        chain_id: context.chain_id.to_string(),
        delta_id: format!(
            "resource-delta-{}-{}-{}",
            context.manifest_height, event.id, event.time
        ),
        action_id: None,
        event_id: Some(event.id),
        ordering_key: ChainResourceOrderingKey {
            height: context.manifest_height,
            event_sequence: event.id,
            action_sequence: 0,
        },
        base_manifest_hash: event_base_manifest_hash(manifest, event),
        resulting_manifest_hash: manifest.manifest_hash.clone(),
        block_height: context.manifest_height,
        commit_block_hash: context.commit_block_hash.map(ToOwned::to_owned),
        tick: event.time,
        source,
        replay_status: ChainResourceReplayStatus::Committed,
        entries,
    })
}

fn event_base_manifest_hash(manifest: &ChainResourceManifest, event: &WorldEvent) -> String {
    if matches!(
        event.kind,
        WorldEventKind::ChunkGenerated {
            cause: ChunkGenerationCause::Init,
            ..
        }
    ) {
        return ChainResourceManifest::empty_at_height(
            manifest.world_id.clone(),
            manifest.world_seed,
            manifest.created_at_height,
        )
        .canonical_hash();
    }
    let mut base = manifest.clone();
    base.manifest_height = manifest.manifest_height.saturating_sub(1);
    base.manifest_hash = base.canonical_hash();
    base.manifest_hash
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChainResourceReplayStatus {
    #[default]
    Committed,
    Rejected,
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "scope", rename_all = "snake_case")]
pub enum ChainResourceDeltaEntry {
    RuntimeResource {
        kind: ResourceKind,
        delta: i64,
        resulting_balance: i64,
    },
    MaterialLedger {
        ledger_id: MaterialLedgerId,
        material_kind: String,
        delta: i64,
        resulting_balance: i64,
    },
    ChunkResource {
        coord: ChunkCoord,
        element: FragmentElementKind,
        total_delta_g: i64,
        remaining_delta_g: i64,
        resulting_remaining_g: i64,
        remaining_after_hash: String,
        chunk_remaining_after_hash: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_resource_manifest_hash_is_stable_across_roundtrip() {
        let mut manifest = ChainResourceManifest::empty_at_height("world-a", 42, 7);
        manifest.chain_id = "chain-a".to_string();
        manifest.world_config_hash = "world-config-hash".to_string();
        manifest.generation_algorithm_hash = "algorithm-hash".to_string();
        manifest.manifest_hash = manifest.canonical_hash();

        let encoded = serde_json::to_string(&manifest).expect("encode manifest");
        let decoded: ChainResourceManifest =
            serde_json::from_str(encoded.as_str()).expect("decode manifest");

        assert_eq!(decoded.schema_version, CHAIN_RESOURCE_MANIFEST_SCHEMA_V1);
        assert_eq!(
            decoded.chunk_generation_schema_version,
            CHUNK_GENERATION_SCHEMA_V1
        );
        assert_eq!(decoded.manifest_hash, manifest.manifest_hash);
        assert!(decoded.is_schema_current());
    }

    #[test]
    fn chain_resource_delta_requires_world_resource_schema_and_commit_hashes() {
        let delta = ChainResourceDelta::empty_for_manifest("world-a", "manifest-hash", 9, 11);

        assert_eq!(delta.schema_version, CHAIN_RESOURCE_DELTA_SCHEMA_V1);
        assert_eq!(delta.chain_id, "unbound");
        assert_eq!(delta.ordering_key.height, 9);
        assert!(delta.is_schema_current());
    }
}
