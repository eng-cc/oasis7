use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::super::error::WorldError;
use super::replay_audit::{
    MembershipRevocationDeadLetterReplayRollbackAlertStateStore,
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord,
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore,
    MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillReport,
    MembershipRevocationDeadLetterReplayRollbackGovernanceStateStore,
};
use super::{MembershipSyncClient, normalized_schedule_key};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionPolicy {
    pub max_records: usize,
    pub max_age_ms: i64,
}

impl Default for MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionPolicy {
    fn default() -> Self {
        Self {
            max_records: 500,
            max_age_ms: 86_400_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceAuditPruneReport {
    pub world_id: String,
    pub node_id: String,
    pub pruned_at_ms: i64,
    pub before: usize,
    pub after: usize,
    pub pruned_by_age: usize,
    pub pruned_by_capacity: usize,
}

pub trait MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore {
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

    fn replace(
        &self,
        world_id: &str,
        node_id: &str,
        records: &[MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord],
    ) -> Result<(), WorldError>;
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore {
    records: Arc<
        Mutex<
            BTreeMap<
                (String, String),
                Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord>,
            >,
        >,
    >,
}

impl InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore
    for InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore
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
                "membership revocation dead-letter replay rollback governance audit retention store lock poisoned"
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
                "membership revocation dead-letter replay rollback governance audit retention store lock poisoned"
                    .into(),
            )
        })?;
        Ok(guard.get(&key).cloned().unwrap_or_default())
    }

    fn replace(
        &self,
        world_id: &str,
        node_id: &str,
        records: &[MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord],
    ) -> Result<(), WorldError> {
        let key = normalized_schedule_key(world_id, node_id)?;
        let mut guard = self.records.lock().map_err(|_| {
            WorldError::Io(
                "membership revocation dead-letter replay rollback governance audit retention store lock poisoned"
                    .into(),
            )
        })?;
        guard.insert(key, records.to_vec());
        Ok(())
    }
}

impl MembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore
    for InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore
{
    fn append(
        &self,
        world_id: &str,
        node_id: &str,
        record: &MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord,
    ) -> Result<(), WorldError> {
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
            self, world_id, node_id, record,
        )
    }

    fn list(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord>, WorldError>
    {
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::list(
            self, world_id, node_id,
        )
    }
}

#[derive(Debug, Clone)]
pub struct FileMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore {
    root_dir: PathBuf,
}

impl FileMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore {
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

    fn write_records(
        &self,
        world_id: &str,
        node_id: &str,
        records: &[MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord],
    ) -> Result<(), WorldError> {
        let path = self.audit_path(world_id, node_id)?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        if records.is_empty() {
            if path.exists() {
                fs::remove_file(path)?;
            }
            return Ok(());
        }

        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;
        for record in records {
            let line = serde_json::to_string(record)?;
            file.write_all(line.as_bytes())?;
            file.write_all(b"\n")?;
        }
        Ok(())
    }
}

impl MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore
    for FileMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore
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

    fn replace(
        &self,
        world_id: &str,
        node_id: &str,
        records: &[MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord],
    ) -> Result<(), WorldError> {
        self.write_records(world_id, node_id, records)
    }
}

impl MembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore
    for FileMembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore
{
    fn append(
        &self,
        world_id: &str,
        node_id: &str,
        record: &MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord,
    ) -> Result<(), WorldError> {
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::append(
            self, world_id, node_id, record,
        )
    }

    fn list(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord>, WorldError>
    {
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore::list(
            self, world_id, node_id,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillSchedulePolicy {
    pub drill_interval_ms: i64,
    pub recent_audit_limit: usize,
}

impl Default for MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillSchedulePolicy {
    fn default() -> Self {
        Self {
            drill_interval_ms: 300_000,
            recent_audit_limit: 20,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_drill_at_ms: Option<i64>,
}

pub trait MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore {
    fn load_state(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleState,
        WorldError,
    >;

    fn save_state(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleState,
    ) -> Result<(), WorldError>;
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore
{
    states: Arc<
        Mutex<
            BTreeMap<
                (String, String),
                MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleState,
            >,
        >,
    >,
}

impl InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore
    for InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore
{
    fn load_state(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleState, WorldError>
    {
        let key = normalized_schedule_key(world_id, node_id)?;
        let guard = self.states.lock().map_err(|_| {
            WorldError::Io(
                "membership revocation dead-letter replay rollback governance recovery drill schedule state store lock poisoned"
                    .into(),
            )
        })?;
        Ok(guard.get(&key).cloned().unwrap_or_default())
    }

    fn save_state(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleState,
    ) -> Result<(), WorldError> {
        let key = normalized_schedule_key(world_id, node_id)?;
        let mut guard = self.states.lock().map_err(|_| {
            WorldError::Io(
                "membership revocation dead-letter replay rollback governance recovery drill schedule state store lock poisoned"
                    .into(),
            )
        })?;
        guard.insert(key, state.clone());
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FileMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore
{
    root_dir: PathBuf,
}

impl FileMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore {
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, WorldError> {
        let root_dir = root_dir.into();
        fs::create_dir_all(&root_dir)?;
        Ok(Self { root_dir })
    }

    fn state_path(&self, world_id: &str, node_id: &str) -> Result<PathBuf, WorldError> {
        let (world_id, node_id) = normalized_schedule_key(world_id, node_id)?;
        Ok(self.root_dir.join(format!(
            "{world_id}.{node_id}.revocation-dead-letter-replay-rollback-governance-recovery-drill-schedule-state.json"
        )))
    }
}

impl MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore
    for FileMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore
{
    fn load_state(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleState,
        WorldError,
    > {
        let path = self.state_path(world_id, node_id)?;
        if !path.exists() {
            return Ok(
                MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleState::default(),
            );
        }
        let bytes = fs::read(path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn save_state(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleState,
    ) -> Result<(), WorldError> {
        let path = self.state_path(world_id, node_id)?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let bytes = serde_json::to_vec_pretty(state)?;
        fs::write(path, bytes)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduledRunReport {
    pub world_id: String,
    pub node_id: String,
    pub scheduled_at_ms: i64,
    pub drill_due: bool,
    pub drill_executed: bool,
    pub next_due_at_ms: Option<i64>,
    pub drill_report:
        Option<MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillReport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceArchiveDrillScheduledRunReport {
    pub prune_report: MembershipRevocationDeadLetterReplayRollbackGovernanceAuditPruneReport,
    pub drill_run_report:
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduledRunReport,
}

impl MembershipSyncClient {
    pub fn prune_revocation_dead_letter_replay_rollback_governance_audit_archive(
        &self,
        world_id: &str,
        node_id: &str,
        pruned_at_ms: i64,
        retention_policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionPolicy,
        rollback_governance_audit_store: &(
             dyn MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore
                 + Send
                 + Sync
         ),
    ) -> Result<MembershipRevocationDeadLetterReplayRollbackGovernanceAuditPruneReport, WorldError>
    {
        validate_governance_audit_retention_policy(retention_policy)?;
        let (world_id, node_id) = normalized_schedule_key(world_id, node_id)?;
        let records = rollback_governance_audit_store.list(&world_id, &node_id)?;
        let before = records.len();
        let (retained, pruned_by_age, pruned_by_capacity) =
            apply_governance_audit_retention(records, pruned_at_ms, retention_policy)?;
        let after = retained.len();
        rollback_governance_audit_store.replace(&world_id, &node_id, &retained)?;
        Ok(
            MembershipRevocationDeadLetterReplayRollbackGovernanceAuditPruneReport {
                world_id,
                node_id,
                pruned_at_ms,
                before,
                after,
                pruned_by_age,
                pruned_by_capacity,
            },
        )
    }

    pub fn run_revocation_dead_letter_replay_rollback_governance_recovery_drill_schedule(
        &self,
        world_id: &str,
        node_id: &str,
        scheduled_at_ms: i64,
        policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillSchedulePolicy,
        schedule_state_store: &(dyn MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore
              + Send
              + Sync),
        rollback_alert_state_store: &(
             dyn MembershipRevocationDeadLetterReplayRollbackAlertStateStore + Send + Sync
         ),
        rollback_governance_state_store: &(
             dyn MembershipRevocationDeadLetterReplayRollbackGovernanceStateStore + Send + Sync
         ),
        rollback_governance_audit_store: &(
             dyn MembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore + Send + Sync
         ),
    ) -> Result<
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduledRunReport,
        WorldError,
    > {
        validate_recovery_drill_schedule_policy(policy)?;
        let (world_id, node_id) = normalized_schedule_key(world_id, node_id)?;
        let mut schedule_state = schedule_state_store.load_state(&world_id, &node_id)?;
        let (drill_due, next_due_at_ms) = match schedule_state.last_drill_at_ms {
            Some(last_drill_at_ms) => {
                let elapsed_since_last_drill = scheduled_at_ms
                    .checked_sub(last_drill_at_ms)
                    .ok_or_else(|| WorldError::DistributedValidationFailed {
                        reason: format!(
                            "membership revocation dead-letter rollback governance recovery drill schedule elapsed overflow: scheduled_at_ms={scheduled_at_ms}, last_drill_at_ms={last_drill_at_ms}"
                        ),
                    })?;
                let next_due_at_ms = last_drill_at_ms
                    .checked_add(policy.drill_interval_ms)
                    .ok_or_else(|| WorldError::DistributedValidationFailed {
                        reason: format!(
                            "membership revocation dead-letter rollback governance recovery drill schedule next_due_at_ms overflow: last_drill_at_ms={last_drill_at_ms}, drill_interval_ms={}",
                            policy.drill_interval_ms
                        ),
                    })?;
                (
                    elapsed_since_last_drill >= policy.drill_interval_ms,
                    Some(next_due_at_ms),
                )
            }
            None => (true, None),
        };
        if !drill_due {
            return Ok(
                MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduledRunReport {
                    world_id,
                    node_id,
                    scheduled_at_ms,
                    drill_due,
                    drill_executed: false,
                    next_due_at_ms,
                    drill_report: None,
                },
            );
        }

        let drill_report = self
            .run_revocation_dead_letter_replay_rollback_governance_recovery_drill(
                &world_id,
                &node_id,
                scheduled_at_ms,
                policy.recent_audit_limit,
                rollback_alert_state_store,
                rollback_governance_state_store,
                rollback_governance_audit_store,
            )?;
        schedule_state.last_drill_at_ms = Some(scheduled_at_ms);
        schedule_state_store.save_state(&world_id, &node_id, &schedule_state)?;
        Ok(
            MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduledRunReport {
                world_id,
                node_id,
                scheduled_at_ms,
                drill_due,
                drill_executed: true,
                next_due_at_ms: Some(
                    scheduled_at_ms.checked_add(policy.drill_interval_ms).ok_or_else(|| {
                        WorldError::DistributedValidationFailed {
                            reason: format!(
                                "membership revocation dead-letter rollback governance recovery drill schedule next_due_at_ms overflow: scheduled_at_ms={scheduled_at_ms}, drill_interval_ms={}",
                                policy.drill_interval_ms
                            ),
                        }
                    })?,
                ),
                drill_report: Some(drill_report),
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_revocation_dead_letter_replay_rollback_governance_archive_retention_and_recovery_drill_schedule(
        &self,
        world_id: &str,
        node_id: &str,
        scheduled_at_ms: i64,
        retention_policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionPolicy,
        drill_schedule_policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillSchedulePolicy,
        rollback_governance_audit_retention_store: &(
             dyn MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore
                 + Send
                 + Sync
         ),
        schedule_state_store: &(dyn MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore
              + Send
              + Sync),
        rollback_alert_state_store: &(
             dyn MembershipRevocationDeadLetterReplayRollbackAlertStateStore + Send + Sync
         ),
        rollback_governance_state_store: &(
             dyn MembershipRevocationDeadLetterReplayRollbackGovernanceStateStore + Send + Sync
         ),
        rollback_governance_audit_store: &(
             dyn MembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore + Send + Sync
         ),
    ) -> Result<
        MembershipRevocationDeadLetterReplayRollbackGovernanceArchiveDrillScheduledRunReport,
        WorldError,
    > {
        let prune_report = self
            .prune_revocation_dead_letter_replay_rollback_governance_audit_archive(
                world_id,
                node_id,
                scheduled_at_ms,
                retention_policy,
                rollback_governance_audit_retention_store,
            )?;
        let drill_run_report = self
            .run_revocation_dead_letter_replay_rollback_governance_recovery_drill_schedule(
                world_id,
                node_id,
                scheduled_at_ms,
                drill_schedule_policy,
                schedule_state_store,
                rollback_alert_state_store,
                rollback_governance_state_store,
                rollback_governance_audit_store,
            )?;
        Ok(
            MembershipRevocationDeadLetterReplayRollbackGovernanceArchiveDrillScheduledRunReport {
                prune_report,
                drill_run_report,
            },
        )
    }
}

fn validate_governance_audit_retention_policy(
    policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionPolicy,
) -> Result<(), WorldError> {
    if policy.max_records == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "membership revocation dead-letter rollback governance audit retention max_records must be positive".to_string(),
        });
    }
    if policy.max_age_ms <= 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "membership revocation dead-letter rollback governance audit retention max_age_ms must be positive, got {}",
                policy.max_age_ms
            ),
        });
    }
    Ok(())
}

fn validate_recovery_drill_schedule_policy(
    policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillSchedulePolicy,
) -> Result<(), WorldError> {
    if policy.drill_interval_ms <= 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "membership revocation dead-letter rollback governance recovery drill schedule drill_interval_ms must be positive, got {}",
                policy.drill_interval_ms
            ),
        });
    }
    if policy.recent_audit_limit == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason:
                "membership revocation dead-letter rollback governance recovery drill schedule recent_audit_limit must be positive"
                    .to_string(),
        });
    }
    Ok(())
}

fn apply_governance_audit_retention(
    records: Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord>,
    now_ms: i64,
    policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionPolicy,
) -> Result<
    (
        Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord>,
        usize,
        usize,
    ),
    WorldError,
> {
    let mut retained = Vec::with_capacity(records.len());
    let mut pruned_by_age = 0usize;
    for record in records {
        let record_age_ms = now_ms.checked_sub(record.audited_at_ms).ok_or_else(|| {
            WorldError::DistributedValidationFailed {
                reason: format!(
                    "membership revocation dead-letter rollback governance audit age overflow: now_ms={now_ms}, audited_at_ms={}",
                    record.audited_at_ms
                ),
            }
        })?;
        if record_age_ms > policy.max_age_ms {
            pruned_by_age = pruned_by_age.saturating_add(1);
        } else {
            retained.push(record);
        }
    }
    if retained.len() <= policy.max_records {
        return Ok((retained, pruned_by_age, 0));
    }
    let pruned_by_capacity = retained.len().saturating_sub(policy.max_records);
    let retained = retained.split_off(pruned_by_capacity);
    Ok((retained, pruned_by_age, pruned_by_capacity))
}
