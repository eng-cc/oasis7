//! Persistence utilities: WorldSnapshot, WorldJournal, and error types.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::Path;

use super::kernel::ChunkRuntimeConfig;
use super::kernel::WorldEvent;
use super::types::{
    ActionEnvelope, ActionId, WorldEventId, WorldTime, CHUNK_GENERATION_SCHEMA_VERSION,
    JOURNAL_VERSION, SNAPSHOT_VERSION,
};
use super::world_model::{WorldConfig, WorldModel};
#[cfg(not(target_arch = "wasm32"))]
use crate::runtime::Snapshot as RuntimeSnapshot;
#[cfg(target_arch = "wasm32")]
use serde_json::Value as RuntimeSnapshot;

// ============================================================================
// Snapshot
// ============================================================================

fn default_snapshot_version() -> u32 {
    SNAPSHOT_VERSION
}

fn default_journal_version() -> u32 {
    JOURNAL_VERSION
}

fn default_chunk_generation_schema_version() -> u32 {
    CHUNK_GENERATION_SCHEMA_VERSION
}

fn is_supported_snapshot_version(version: u32) -> bool {
    version == SNAPSHOT_VERSION || version == SNAPSHOT_VERSION.saturating_sub(1)
}

fn is_supported_journal_version(version: u32) -> bool {
    version == JOURNAL_VERSION || version == JOURNAL_VERSION.saturating_sub(1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerGameplayStageId {
    FirstSessionLoop,
    PostOnboarding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerGameplayStageStatus {
    Active,
    Blocked,
    BranchReady,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerGameplayGoalKind {
    CreateFirstWorldFeedback,
    EstablishFirstCapability,
    TurnMaterialFlowIntoOutput,
    StartFactoryRun,
    StabilizeFirstLine,
    RecoverCapability,
    ChooseMidLoopPath,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerGameplayAction {
    pub action_id: String,
    pub label: String,
    pub protocol_action: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerGameplayRecentFeedback {
    pub action: String,
    pub stage: String,
    pub effect: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    #[serde(default)]
    pub delta_logical_time: WorldTime,
    #[serde(default)]
    pub delta_event_seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerAgentClaimQuoteSnapshot {
    pub slot_index: u8,
    pub reputation_tier: u8,
    pub claim_cap: u8,
    pub owned_claim_count: u8,
    pub activation_fee_amount: u64,
    pub claim_bond_amount: u64,
    pub upkeep_per_epoch: u64,
    pub total_upfront_amount: u64,
    #[serde(default)]
    pub transferable_liquid_balance: u64,
    #[serde(default)]
    pub restricted_starter_claim_balance: u64,
    #[serde(default)]
    pub eligible_claim_balance: u64,
    pub release_cooldown_epochs: u64,
    pub grace_epochs: u64,
    pub idle_warning_epochs: u64,
    pub forced_idle_reclaim_epochs: u64,
    pub forced_reclaim_penalty_bps: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerAgentClaimOwnedSnapshot {
    pub target_agent_id: String,
    pub status: String,
    pub upkeep_paid_through_epoch: u64,
    #[serde(default)]
    pub upfront_restricted_spent_amount: u64,
    #[serde(default)]
    pub upfront_liquid_spent_amount: u64,
    #[serde(default)]
    pub claim_bond_locked_restricted_amount: u64,
    #[serde(default)]
    pub claim_bond_locked_liquid_amount: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_ready_at_epoch: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_ready_in_epochs: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grace_deadline_epoch: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grace_remaining_epochs: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idle_warning_in_epochs: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forced_reclaim_in_epochs: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerAgentClaimSnapshot {
    pub claimer_agent_id: String,
    pub current_epoch: u64,
    pub reputation_tier: u8,
    pub claim_cap: u8,
    pub owned_claim_count: u8,
    pub liquid_main_token_balance: u64,
    #[serde(default)]
    pub restricted_starter_claim_balance: u64,
    #[serde(default)]
    pub slot_1_eligible_claim_balance: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_claim_quote: Option<PlayerAgentClaimQuoteSnapshot>,
    #[serde(default)]
    pub owned_claims: Vec<PlayerAgentClaimOwnedSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerGameplaySnapshot {
    pub stage_id: PlayerGameplayStageId,
    pub stage_status: PlayerGameplayStageStatus,
    pub goal_id: String,
    pub goal_kind: PlayerGameplayGoalKind,
    pub goal_title: String,
    pub objective: String,
    pub progress_detail: String,
    #[serde(default)]
    pub progress_percent: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocker_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocker_detail: Option<String>,
    pub next_step_hint: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_hint: Option<String>,
    #[serde(default)]
    pub available_actions: Vec<PlayerGameplayAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recent_feedback: Option<PlayerGameplayRecentFeedback>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_claim: Option<PlayerAgentClaimSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldSnapshot {
    #[serde(default = "default_snapshot_version")]
    pub version: u32,
    #[serde(default = "default_chunk_generation_schema_version")]
    pub chunk_generation_schema_version: u32,
    pub time: WorldTime,
    pub config: WorldConfig,
    pub model: WorldModel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_snapshot: Option<RuntimeSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_gameplay: Option<PlayerGameplaySnapshot>,
    #[serde(default)]
    pub chunk_runtime: ChunkRuntimeConfig,
    pub next_event_id: WorldEventId,
    pub next_action_id: ActionId,
    pub pending_actions: Vec<ActionEnvelope>,
    pub journal_len: usize,
}

impl WorldSnapshot {
    pub fn to_json(&self) -> Result<String, PersistError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(input: &str) -> Result<Self, PersistError> {
        let snapshot: Self = serde_json::from_str(input)?;
        snapshot.validate_version()?;
        Ok(snapshot)
    }

    pub fn save_json(&self, path: impl AsRef<Path>) -> Result<(), PersistError> {
        write_json_to_path(self, path.as_ref())
    }

    pub fn load_json(path: impl AsRef<Path>) -> Result<Self, PersistError> {
        let snapshot: Self = read_json_from_path(path.as_ref())?;
        snapshot.validate_version()?;
        Ok(snapshot)
    }

    pub(crate) fn validate_version(&self) -> Result<(), PersistError> {
        if is_supported_snapshot_version(self.version) {
            Ok(())
        } else {
            Err(PersistError::UnsupportedVersion {
                kind: "snapshot".to_string(),
                version: self.version,
                expected: SNAPSHOT_VERSION,
            })
        }
    }
}

// ============================================================================
// Journal
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldJournal {
    #[serde(default = "default_journal_version")]
    pub version: u32,
    pub events: Vec<WorldEvent>,
}

impl WorldJournal {
    pub fn new() -> Self {
        Self {
            version: JOURNAL_VERSION,
            events: Vec::new(),
        }
    }

    pub fn to_json(&self) -> Result<String, PersistError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(input: &str) -> Result<Self, PersistError> {
        let journal: Self = serde_json::from_str(input)?;
        journal.validate_version()?;
        Ok(journal)
    }

    pub fn save_json(&self, path: impl AsRef<Path>) -> Result<(), PersistError> {
        write_json_to_path(self, path.as_ref())
    }

    pub fn load_json(path: impl AsRef<Path>) -> Result<Self, PersistError> {
        let journal: Self = read_json_from_path(path.as_ref())?;
        journal.validate_version()?;
        Ok(journal)
    }

    pub(crate) fn validate_version(&self) -> Result<(), PersistError> {
        if is_supported_journal_version(self.version) {
            Ok(())
        } else {
            Err(PersistError::UnsupportedVersion {
                kind: "journal".to_string(),
                version: self.version,
                expected: JOURNAL_VERSION,
            })
        }
    }
}

impl Default for WorldJournal {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistError {
    Io(String),
    Serde(String),
    SnapshotMismatch {
        expected: usize,
        actual: usize,
    },
    ReplayConflict {
        message: String,
    },
    UnsupportedVersion {
        kind: String,
        version: u32,
        expected: u32,
    },
}

impl From<io::Error> for PersistError {
    fn from(err: io::Error) -> Self {
        PersistError::Io(err.to_string())
    }
}

impl From<serde_json::Error> for PersistError {
    fn from(err: serde_json::Error) -> Self {
        PersistError::Serde(err.to_string())
    }
}

// ============================================================================
// Helper functions
// ============================================================================

pub(crate) fn write_json_to_path<T: Serialize>(value: &T, path: &Path) -> Result<(), PersistError> {
    let data = serde_json::to_vec_pretty(value)?;
    fs::write(path, data)?;
    Ok(())
}

pub(crate) fn read_json_from_path<T: DeserializeOwned>(path: &Path) -> Result<T, PersistError> {
    let data = fs::read(path)?;
    Ok(serde_json::from_slice(&data)?)
}
