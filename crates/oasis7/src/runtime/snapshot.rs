//! Snapshot and journal types for world state persistence.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::consensus::{TickConsensusRecord, TickConsensusRejectionAuditEvent};
use super::effect::{CapabilityGrant, EffectIntent};
use super::error::WorldError;
use super::events::ActionEnvelope;
use super::governance::{GovernanceExecutionPolicy, GovernanceIdentityPenaltyRecord, Proposal};
use super::manifest::Manifest;
use super::modules::{ModuleLimits, ModuleRegistry};
use super::policy::PolicySet;
use super::state::WorldState;
use super::types::{ActionId, IntentSeq, ProposalId, WorldEventId, WorldTime};
use super::util::{deserialize_btreemap_u64_keys, read_json_from_path, write_json_to_path};
use super::world::{WorldRuntimeBackpressureStats, WorldRuntimeMemoryLimits};
use super::world_event::WorldEvent;
use crate::chain_resource_schema::{ChainResourceDelta, ChainResourceManifest};

/// Policy for how many snapshots to retain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotRetentionPolicy {
    pub max_snapshots: usize,
}

impl Default for SnapshotRetentionPolicy {
    fn default() -> Self {
        Self { max_snapshots: 10 }
    }
}

/// A record of a saved snapshot for catalog purposes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotRecord {
    pub snapshot_hash: String,
    pub journal_len: usize,
    pub created_at: WorldTime,
    pub manifest_hash: String,
}

/// Catalog of all recorded snapshots with retention policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotCatalog {
    pub records: Vec<SnapshotRecord>,
    pub retention: SnapshotRetentionPolicy,
}

impl Default for SnapshotCatalog {
    fn default() -> Self {
        Self {
            records: Vec::new(),
            retention: SnapshotRetentionPolicy::default(),
        }
    }
}

/// A complete snapshot of the world state at a point in time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub snapshot_catalog: SnapshotCatalog,
    pub manifest: Manifest,
    #[serde(default)]
    pub chain_resource_manifest: ChainResourceManifest,
    #[serde(default)]
    pub latest_chain_resource_delta: Option<ChainResourceDelta>,
    #[serde(default)]
    pub module_registry: ModuleRegistry,
    #[serde(default)]
    pub module_artifacts: BTreeSet<String>,
    #[serde(default = "module_limits_unbounded")]
    pub module_limits_max: ModuleLimits,
    pub state: WorldState,
    pub journal_len: usize,
    pub last_event_id: WorldEventId,
    #[serde(default)]
    pub event_id_era: u64,
    pub next_action_id: ActionId,
    #[serde(default)]
    pub action_id_era: u64,
    pub next_intent_id: IntentSeq,
    #[serde(default)]
    pub intent_id_era: u64,
    pub next_proposal_id: ProposalId,
    #[serde(default)]
    pub proposal_id_era: u64,
    pub pending_actions: Vec<ActionEnvelope>,
    pub pending_effects: Vec<EffectIntent>,
    pub inflight_effects: BTreeMap<String, EffectIntent>,
    #[serde(default)]
    pub module_tick_schedule: BTreeMap<String, WorldTime>,
    pub capabilities: BTreeMap<String, CapabilityGrant>,
    pub policies: PolicySet,
    #[serde(deserialize_with = "deserialize_btreemap_u64_keys")]
    pub proposals: BTreeMap<ProposalId, Proposal>,
    pub scheduler_cursor: Option<String>,
    #[serde(default)]
    pub runtime_memory_limits: WorldRuntimeMemoryLimits,
    #[serde(default)]
    pub runtime_backpressure_stats: WorldRuntimeBackpressureStats,
    #[serde(default)]
    pub tick_consensus_records: Vec<TickConsensusRecord>,
    #[serde(default)]
    pub tick_consensus_total_record_count: usize,
    #[serde(default)]
    pub tick_consensus_archived_record_count: usize,
    #[serde(default)]
    pub tick_consensus_hot_from_tick: Option<WorldTime>,
    #[serde(default)]
    pub tick_consensus_hot_to_tick: Option<WorldTime>,
    #[serde(default = "default_tick_consensus_authority_source")]
    pub tick_consensus_authority_source: String,
    #[serde(default)]
    pub tick_consensus_rejection_audit_events: Vec<TickConsensusRejectionAuditEvent>,
    #[serde(default)]
    pub governance_execution_policy: GovernanceExecutionPolicy,
    #[serde(default)]
    pub governance_emergency_brake_until_tick: Option<WorldTime>,
    #[serde(default, deserialize_with = "deserialize_btreemap_u64_keys")]
    pub governance_identity_penalties: BTreeMap<u64, GovernanceIdentityPenaltyRecord>,
    #[serde(default = "default_next_governance_identity_penalty_id")]
    pub next_governance_identity_penalty_id: u64,
}

fn module_limits_unbounded() -> ModuleLimits {
    ModuleLimits::unbounded()
}

fn default_next_governance_identity_penalty_id() -> u64 {
    1
}

fn default_tick_consensus_authority_source() -> String {
    super::consensus::DEFAULT_TICK_CONSENSUS_AUTHORITY_SOURCE.to_string()
}

impl Snapshot {
    pub fn to_json(&self) -> Result<String, WorldError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(input: &str) -> Result<Self, WorldError> {
        Ok(serde_json::from_str(input)?)
    }

    pub fn save_json(&self, path: impl AsRef<Path>) -> Result<(), WorldError> {
        write_json_to_path(self, path.as_ref())
    }

    pub fn load_json(path: impl AsRef<Path>) -> Result<Self, WorldError> {
        read_json_from_path(path.as_ref())
    }
}

/// Metadata about a snapshot creation event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotMeta {
    pub journal_len: usize,
}

/// The journal containing all world events since the last snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Journal {
    pub events: Vec<WorldEvent>,
}

impl Journal {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub fn append(&mut self, event: WorldEvent) {
        self.events.push(event);
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn to_json(&self) -> Result<String, WorldError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(input: &str) -> Result<Self, WorldError> {
        Ok(serde_json::from_str(input)?)
    }

    pub fn save_json(&self, path: impl AsRef<Path>) -> Result<(), WorldError> {
        write_json_to_path(self, path.as_ref())
    }

    pub fn load_json(path: impl AsRef<Path>) -> Result<Self, WorldError> {
        read_json_from_path(path.as_ref())
    }
}

impl Default for Journal {
    fn default() -> Self {
        Self::new()
    }
}

/// Event recorded when a rollback is applied.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RollbackEvent {
    pub snapshot_hash: String,
    pub snapshot_journal_len: usize,
    pub prior_journal_len: usize,
    pub reason: String,
}
