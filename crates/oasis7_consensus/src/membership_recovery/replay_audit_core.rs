use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::super::error::WorldError;

use super::super::membership_reconciliation::{
    MembershipRevocationAlertSeverity, MembershipRevocationAlertSink,
    MembershipRevocationAnomalyAlert,
};
use super::{
    normalized_schedule_key, MembershipRevocationAlertDeadLetterStore,
    MembershipRevocationAlertDeliveryMetrics, MembershipRevocationAlertRecoveryStore,
    MembershipRevocationDeadLetterReplayPolicy, MembershipRevocationDeadLetterReplayPolicyStore,
    MembershipRevocationDeadLetterReplayRollbackGuard,
    MembershipRevocationDeadLetterReplayStateStore, MembershipRevocationScheduleCoordinator,
    MembershipSyncClient,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipRevocationDeadLetterReplayPolicyAdoptionAuditDecision {
    Adopted,
    RolledBackToStable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipRevocationDeadLetterReplayPolicyAdoptionAuditRecord {
    pub world_id: String,
    pub node_id: String,
    pub audited_at_ms: i64,
    pub decision: MembershipRevocationDeadLetterReplayPolicyAdoptionAuditDecision,
    pub recommended_policy: MembershipRevocationDeadLetterReplayPolicy,
    pub applied_policy: MembershipRevocationDeadLetterReplayPolicy,
    pub stable_policy: MembershipRevocationDeadLetterReplayPolicy,
    pub backlog_dead_letters: usize,
    pub backlog_pending: usize,
    pub metrics: MembershipRevocationAlertDeliveryMetrics,
    pub rollback_triggered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationDeadLetterReplayRollbackAlertPolicy {
    pub rollback_window_ms: i64,
    pub max_rollbacks_per_window: usize,
    pub min_attempted: usize,
    pub alert_cooldown_ms: i64,
}

impl Default for MembershipRevocationDeadLetterReplayRollbackAlertPolicy {
    fn default() -> Self {
        Self {
            rollback_window_ms: 120_000,
            max_rollbacks_per_window: 3,
            min_attempted: 8,
            alert_cooldown_ms: 60_000,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipRevocationDeadLetterReplayRollbackAlertState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_alert_at_ms: Option<i64>,
}

pub trait MembershipRevocationDeadLetterReplayRollbackAlertStateStore {
    fn load_alert_state(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<MembershipRevocationDeadLetterReplayRollbackAlertState, WorldError>;

    fn save_alert_state(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationDeadLetterReplayRollbackAlertState,
    ) -> Result<(), WorldError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MembershipRevocationDeadLetterReplayRollbackGovernanceLevel {
    #[default]
    Normal,
    Stable,
    Emergency,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernancePolicy {
    pub level_one_rollback_streak: usize,
    pub level_two_rollback_streak: usize,
    pub level_two_emergency_policy: MembershipRevocationDeadLetterReplayPolicy,
}

impl Default for MembershipRevocationDeadLetterReplayRollbackGovernancePolicy {
    fn default() -> Self {
        Self {
            level_one_rollback_streak: 2,
            level_two_rollback_streak: 4,
            level_two_emergency_policy: MembershipRevocationDeadLetterReplayPolicy {
                max_replay_per_run: 2,
                max_retry_limit_exceeded_streak: 1,
            },
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceState {
    #[serde(default)]
    pub rollback_streak: usize,
    #[serde(default)]
    pub last_level: MembershipRevocationDeadLetterReplayRollbackGovernanceLevel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_level_updated_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord {
    pub world_id: String,
    pub node_id: String,
    pub audited_at_ms: i64,
    pub governance_level: MembershipRevocationDeadLetterReplayRollbackGovernanceLevel,
    pub rollback_streak: usize,
    pub rolled_back: bool,
    pub applied_policy: MembershipRevocationDeadLetterReplayPolicy,
    pub alert_emitted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillReport {
    pub world_id: String,
    pub node_id: String,
    pub drill_at_ms: i64,
    pub alert_state: MembershipRevocationDeadLetterReplayRollbackAlertState,
    pub governance_state: MembershipRevocationDeadLetterReplayRollbackGovernanceState,
    pub recent_audits: Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord>,
    pub has_emergency_history: bool,
}

type MembershipRevocationDeadLetterReplayRollbackGovernanceRunResult = (
    usize,
    MembershipRevocationDeadLetterReplayPolicy,
    bool,
    bool,
    MembershipRevocationDeadLetterReplayRollbackGovernanceLevel,
);

pub trait MembershipRevocationDeadLetterReplayRollbackGovernanceStateStore {
    fn load_governance_state(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<MembershipRevocationDeadLetterReplayRollbackGovernanceState, WorldError>;

    fn save_governance_state(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationDeadLetterReplayRollbackGovernanceState,
    ) -> Result<(), WorldError>;
}

pub trait MembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore {
    fn append(
        &self,
        world_id: &str,
        node_id: &str,
        record: &MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord,
    ) -> Result<(), WorldError>;

    fn list(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord>, WorldError>;
}

pub trait MembershipRevocationDeadLetterReplayPolicyAuditStore {
    fn append(
        &self,
        world_id: &str,
        node_id: &str,
        record: &MembershipRevocationDeadLetterReplayPolicyAdoptionAuditRecord,
    ) -> Result<(), WorldError>;

    fn list(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<MembershipRevocationDeadLetterReplayPolicyAdoptionAuditRecord>, WorldError>;
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryMembershipRevocationDeadLetterReplayPolicyAuditStore {
    records: Arc<
        Mutex<
            BTreeMap<
                (String, String),
                Vec<MembershipRevocationDeadLetterReplayPolicyAdoptionAuditRecord>,
            >,
        >,
    >,
}

impl InMemoryMembershipRevocationDeadLetterReplayPolicyAuditStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MembershipRevocationDeadLetterReplayPolicyAuditStore
    for InMemoryMembershipRevocationDeadLetterReplayPolicyAuditStore
{
    fn append(
        &self,
        world_id: &str,
        node_id: &str,
        record: &MembershipRevocationDeadLetterReplayPolicyAdoptionAuditRecord,
    ) -> Result<(), WorldError> {
        let key = normalized_schedule_key(world_id, node_id)?;
        let mut guard = self.records.lock().map_err(|_| {
            WorldError::Io(
                "membership revocation dead-letter replay policy audit store lock poisoned".into(),
            )
        })?;
        guard.entry(key).or_default().push(record.clone());
        Ok(())
    }

    fn list(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<MembershipRevocationDeadLetterReplayPolicyAdoptionAuditRecord>, WorldError>
    {
        let key = normalized_schedule_key(world_id, node_id)?;
        let guard = self.records.lock().map_err(|_| {
            WorldError::Io(
                "membership revocation dead-letter replay policy audit store lock poisoned".into(),
            )
        })?;
        Ok(guard.get(&key).cloned().unwrap_or_default())
    }
}

#[derive(Debug, Clone)]
pub struct FileMembershipRevocationDeadLetterReplayPolicyAuditStore {
    root_dir: PathBuf,
}

impl FileMembershipRevocationDeadLetterReplayPolicyAuditStore {
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, WorldError> {
        let root_dir = root_dir.into();
        fs::create_dir_all(&root_dir)?;
        Ok(Self { root_dir })
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    fn audit_path(&self, world_id: &str, node_id: &str) -> Result<PathBuf, WorldError> {
        let (world_id, node_id) = normalized_schedule_key(world_id, node_id)?;
        Ok(self.root_dir.join(format!(
            "{world_id}.{node_id}.revocation-dead-letter-replay-policy-audit.jsonl"
        )))
    }
}

impl MembershipRevocationDeadLetterReplayPolicyAuditStore
    for FileMembershipRevocationDeadLetterReplayPolicyAuditStore
{
    fn append(
        &self,
        world_id: &str,
        node_id: &str,
        record: &MembershipRevocationDeadLetterReplayPolicyAdoptionAuditRecord,
    ) -> Result<(), WorldError> {
        let path = self.audit_path(world_id, node_id)?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let line = serde_json::to_string(record)?;
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        Ok(())
    }

    fn list(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<MembershipRevocationDeadLetterReplayPolicyAdoptionAuditRecord>, WorldError>
    {
        let path = self.audit_path(world_id, node_id)?;
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = OpenOptions::new().read(true).open(path)?;
        let reader = BufReader::new(file);
        let mut records = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            records.push(serde_json::from_str(&line)?);
        }
        Ok(records)
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryMembershipRevocationDeadLetterReplayRollbackAlertStateStore {
    states: Arc<
        Mutex<BTreeMap<(String, String), MembershipRevocationDeadLetterReplayRollbackAlertState>>,
    >,
}

impl InMemoryMembershipRevocationDeadLetterReplayRollbackAlertStateStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MembershipRevocationDeadLetterReplayRollbackAlertStateStore
    for InMemoryMembershipRevocationDeadLetterReplayRollbackAlertStateStore
{
    fn load_alert_state(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<MembershipRevocationDeadLetterReplayRollbackAlertState, WorldError> {
        let key = normalized_schedule_key(world_id, node_id)?;
        let guard = self.states.lock().map_err(|_| {
            WorldError::Io(
                "membership revocation dead-letter replay rollback alert state lock poisoned"
                    .into(),
            )
        })?;
        Ok(guard.get(&key).cloned().unwrap_or_default())
    }

    fn save_alert_state(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationDeadLetterReplayRollbackAlertState,
    ) -> Result<(), WorldError> {
        let key = normalized_schedule_key(world_id, node_id)?;
        let mut guard = self.states.lock().map_err(|_| {
            WorldError::Io(
                "membership revocation dead-letter replay rollback alert state lock poisoned"
                    .into(),
            )
        })?;
        guard.insert(key, state.clone());
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FileMembershipRevocationDeadLetterReplayRollbackAlertStateStore {
    root_dir: PathBuf,
}

impl FileMembershipRevocationDeadLetterReplayRollbackAlertStateStore {
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, WorldError> {
        let root_dir = root_dir.into();
        fs::create_dir_all(&root_dir)?;
        Ok(Self { root_dir })
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    fn state_path(&self, world_id: &str, node_id: &str) -> Result<PathBuf, WorldError> {
        let (world_id, node_id) = normalized_schedule_key(world_id, node_id)?;
        Ok(self.root_dir.join(format!(
            "{world_id}.{node_id}.revocation-dead-letter-replay-rollback-alert-state.json"
        )))
    }
}

impl MembershipRevocationDeadLetterReplayRollbackAlertStateStore
    for FileMembershipRevocationDeadLetterReplayRollbackAlertStateStore
{
    fn load_alert_state(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<MembershipRevocationDeadLetterReplayRollbackAlertState, WorldError> {
        let path = self.state_path(world_id, node_id)?;
        if !path.exists() {
            return Ok(MembershipRevocationDeadLetterReplayRollbackAlertState::default());
        }
        let bytes = fs::read(path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn save_alert_state(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationDeadLetterReplayRollbackAlertState,
    ) -> Result<(), WorldError> {
        let path = self.state_path(world_id, node_id)?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, serde_json::to_vec(state)?)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore {
    states: Arc<
        Mutex<
            BTreeMap<(String, String), MembershipRevocationDeadLetterReplayRollbackGovernanceState>,
        >,
    >,
}

impl InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MembershipRevocationDeadLetterReplayRollbackGovernanceStateStore
    for InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore
{
    fn load_governance_state(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<MembershipRevocationDeadLetterReplayRollbackGovernanceState, WorldError> {
        let key = normalized_schedule_key(world_id, node_id)?;
        let guard = self.states.lock().map_err(|_| {
            WorldError::Io(
                "membership revocation dead-letter replay rollback governance state lock poisoned"
                    .into(),
            )
        })?;
        Ok(guard.get(&key).cloned().unwrap_or_default())
    }

    fn save_governance_state(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationDeadLetterReplayRollbackGovernanceState,
    ) -> Result<(), WorldError> {
        let key = normalized_schedule_key(world_id, node_id)?;
        let mut guard = self.states.lock().map_err(|_| {
            WorldError::Io(
                "membership revocation dead-letter replay rollback governance state lock poisoned"
                    .into(),
            )
        })?;
        guard.insert(key, state.clone());
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FileMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore {
    root_dir: PathBuf,
}

impl FileMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore {
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, WorldError> {
        let root_dir = root_dir.into();
        fs::create_dir_all(&root_dir)?;
        Ok(Self { root_dir })
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    fn state_path(&self, world_id: &str, node_id: &str) -> Result<PathBuf, WorldError> {
        let (world_id, node_id) = normalized_schedule_key(world_id, node_id)?;
        Ok(self.root_dir.join(format!(
            "{world_id}.{node_id}.revocation-dead-letter-replay-rollback-governance-state.json"
        )))
    }
}

impl MembershipRevocationDeadLetterReplayRollbackGovernanceStateStore
    for FileMembershipRevocationDeadLetterReplayRollbackGovernanceStateStore
{
    fn load_governance_state(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<MembershipRevocationDeadLetterReplayRollbackGovernanceState, WorldError> {
        let path = self.state_path(world_id, node_id)?;
        if !path.exists() {
            return Ok(MembershipRevocationDeadLetterReplayRollbackGovernanceState::default());
        }
        let bytes = fs::read(path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn save_governance_state(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationDeadLetterReplayRollbackGovernanceState,
    ) -> Result<(), WorldError> {
        let path = self.state_path(world_id, node_id)?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, serde_json::to_vec(state)?)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore {
    records: Arc<
        Mutex<
            BTreeMap<
                (String, String),
                Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord>,
            >,
        >,
    >,
}

impl InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore
    for InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore
{
    fn append(
        &self,
        world_id: &str,
        node_id: &str,
        record: &MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord,
    ) -> Result<(), WorldError> {
        let key = normalized_schedule_key(world_id, node_id)?;
        let mut guard = self.records.lock().map_err(|_| {
            WorldError::Io(
                "membership revocation dead-letter replay rollback governance audit store lock poisoned"
                    .into(),
            )
        })?;
        guard.entry(key).or_default().push(record.clone());
        Ok(())
    }

    fn list(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord>, WorldError>
    {
        let key = normalized_schedule_key(world_id, node_id)?;
        let guard = self.records.lock().map_err(|_| {
            WorldError::Io(
                "membership revocation dead-letter replay rollback governance audit store lock poisoned"
                    .into(),
            )
        })?;
        Ok(guard.get(&key).cloned().unwrap_or_default())
    }
}

#[derive(Debug, Clone)]
pub struct FileMembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore {
    root_dir: PathBuf,
}

impl FileMembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore {
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, WorldError> {
        let root_dir = root_dir.into();
        fs::create_dir_all(&root_dir)?;
        Ok(Self { root_dir })
    }

    fn audit_path(&self, world_id: &str, node_id: &str) -> Result<PathBuf, WorldError> {
        let (world_id, node_id) = normalized_schedule_key(world_id, node_id)?;
        Ok(self.root_dir.join(format!(
            "{world_id}.{node_id}.revocation-dead-letter-replay-rollback-governance-audit.jsonl"
        )))
    }
}

impl MembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore
    for FileMembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore
{
    fn append(
        &self,
        world_id: &str,
        node_id: &str,
        record: &MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord,
    ) -> Result<(), WorldError> {
        let path = self.audit_path(world_id, node_id)?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let line = serde_json::to_string(record)?;
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        Ok(())
    }

    fn list(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord>, WorldError>
    {
        let path = self.audit_path(world_id, node_id)?;
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = OpenOptions::new().read(true).open(path)?;
        let reader = BufReader::new(file);
        let mut records = Vec::new();
        for line in reader.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            records.push(serde_json::from_str(&line)?);
        }
        Ok(records)
    }
}
