//! Snapshot and journal types for world state persistence.

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
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
use super::world::{
    ModuleTickRoutingDeterministicSnapshot, WorldRuntimeBackpressureStats, WorldRuntimeMemoryLimits,
};
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
    #[serde(default)]
    pub module_tick_routing_metrics: ModuleTickRoutingDeterministicSnapshot,
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
    #[serde(default)]
    pub rollback_authority_registry: RollbackAuthorityRegistry,
    #[serde(default)]
    pub consumed_rollback_nonces: BTreeSet<String>,
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
    #[serde(default)]
    pub rollback_ticket: String,
    #[serde(default)]
    pub on_call_approver: String,
    #[serde(default)]
    pub governance_approver: String,
    #[serde(default)]
    pub on_call_authority_id: String,
    #[serde(default)]
    pub governance_authority_id: String,
    #[serde(default)]
    pub authorization_nonce: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RollbackAuthorityRole {
    OnCall,
    Governance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackAuthorityRecord {
    pub authority_id: String,
    pub role: RollbackAuthorityRole,
    pub public_key_hex: String,
    pub active: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RollbackAuthorityRegistry {
    records: BTreeMap<String, RollbackAuthorityRecord>,
}

impl<'de> Deserialize<'de> for RollbackAuthorityRegistry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct SerializedRegistry {
            records: BTreeMap<String, RollbackAuthorityRecord>,
        }

        let serialized = SerializedRegistry::deserialize(deserializer)?;
        if serialized.records.is_empty() {
            return Ok(Self::default());
        }
        for (key, record) in &serialized.records {
            if key != record.authority_id.trim() {
                return Err(D::Error::custom(format!(
                    "rollback authority registry key {key:?} does not match authority_id {:?}",
                    record.authority_id
                )));
            }
        }
        Self::new(serialized.records.into_values()).map_err(|error| {
            D::Error::custom(format!("invalid rollback authority registry: {error:?}"))
        })
    }
}

impl RollbackAuthorityRegistry {
    pub fn new(
        records: impl IntoIterator<Item = RollbackAuthorityRecord>,
    ) -> Result<Self, WorldError> {
        let mut registry = Self::default();
        for mut record in records {
            let authority_id = record.authority_id.trim().to_string();
            if authority_id.is_empty() || record.public_key_hex.trim().is_empty() {
                return Err(WorldError::DistributedValidationFailed {
                    reason: "rollback authority requires nonblank id and public key".to_string(),
                });
            }
            if hex::decode(record.public_key_hex.trim()).map(|bytes| bytes.len()) != Ok(32) {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "rollback authority {authority_id} has invalid Ed25519 public key"
                    ),
                });
            }
            record.authority_id = authority_id.clone();
            record.public_key_hex = record.public_key_hex.trim().to_ascii_lowercase();
            if registry
                .records
                .insert(authority_id.clone(), record)
                .is_some()
            {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!("duplicate rollback authority id {authority_id}"),
                });
            }
        }
        for role in [
            RollbackAuthorityRole::OnCall,
            RollbackAuthorityRole::Governance,
        ] {
            let count = registry
                .records
                .values()
                .filter(|record| record.role == role)
                .count();
            if count != 1 {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "rollback authority registry requires exactly one active {role:?} authority"
                    ),
                });
            }
        }
        if registry.records.values().any(|record| !record.active) {
            return Err(WorldError::DistributedValidationFailed {
                reason: "rollback authority registry records must be active".to_string(),
            });
        }
        let unique_keys = registry
            .records
            .values()
            .map(|record| record.public_key_hex.trim().to_ascii_lowercase())
            .collect::<BTreeSet<_>>();
        if unique_keys.len() != registry.records.len() {
            return Err(WorldError::DistributedValidationFailed {
                reason: "rollback authority roles must use distinct Ed25519 public keys"
                    .to_string(),
            });
        }
        Ok(registry)
    }

    pub fn validate(&self) -> Result<(), WorldError> {
        if self.records.is_empty() {
            return Ok(());
        }
        for (key, record) in &self.records {
            if key != record.authority_id.trim() {
                return Err(WorldError::DistributedValidationFailed {
                    reason: format!(
                        "rollback authority registry key {key:?} does not match authority_id {:?}",
                        record.authority_id
                    ),
                });
            }
        }
        let validated = Self::new(self.records.values().cloned())?;
        if validated != *self {
            return Err(WorldError::DistributedValidationFailed {
                reason: "rollback authority registry is not normalized".to_string(),
            });
        }
        Ok(())
    }

    pub fn get(&self, authority_id: &str) -> Option<&RollbackAuthorityRecord> {
        self.records.get(authority_id)
    }

    pub fn records(&self) -> impl Iterator<Item = &RollbackAuthorityRecord> {
        self.records.values()
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackIntent {
    pub schema_version: u32,
    pub rollback_ticket: String,
    pub snapshot_hash: String,
    pub snapshot_journal_len: usize,
    pub target_journal_len: usize,
    pub expected_target_state_root: String,
    pub target_batch_id: Option<String>,
    pub reason: String,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
    pub nonce: String,
}

impl RollbackIntent {
    pub fn canonical_signing_payload(&self) -> Result<Vec<u8>, WorldError> {
        if self.schema_version != 1 {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "unsupported rollback intent schema version {}",
                    self.schema_version
                ),
            });
        }
        let mut payload = b"oasis7:world-rollback-authorization:v1\0".to_vec();
        payload.extend(serde_json::to_vec(self)?);
        Ok(payload)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackApprovalSignature {
    pub authority_id: String,
    pub role: RollbackAuthorityRole,
    pub signature_scheme: String,
    pub signature_hex: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackAuthorizationEnvelope {
    pub intent: RollbackIntent,
    pub signatures: Vec<RollbackApprovalSignature>,
}
