use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use super::super::error::WorldError;

use super::super::membership_reconciliation::{
    MembershipRevocationAlertSeverity, MembershipRevocationAlertSink,
    MembershipRevocationAnomalyAlert,
};
use super::replay_archive::{
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditPruneReport,
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionPolicy,
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore,
    MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillSchedulePolicy,
    MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore,
    MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduledRunReport,
};
use super::replay_audit::{
    MembershipRevocationDeadLetterReplayRollbackAlertStateStore,
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord,
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore,
    MembershipRevocationDeadLetterReplayRollbackGovernanceLevel,
    MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillReport,
    MembershipRevocationDeadLetterReplayRollbackGovernanceStateStore,
};
use super::{MembershipSyncClient, normalized_schedule_key};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadPolicy {
    pub hot_max_records: usize,
    pub offload_min_age_ms: i64,
    pub max_offload_records: usize,
}

impl Default for MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadPolicy {
    fn default() -> Self {
        Self {
            hot_max_records: 200,
            offload_min_age_ms: 3_600_000,
            max_offload_records: 200,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadReport {
    pub world_id: String,
    pub node_id: String,
    pub offloaded_at_ms: i64,
    pub hot_before: usize,
    pub hot_after: usize,
    pub cold_before: usize,
    pub cold_after: usize,
    pub offloaded: usize,
    pub offloaded_by_age: usize,
    pub offloaded_by_capacity: usize,
    pub kept_due_to_rate_limit: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertPolicy {
    pub max_alert_silence_ms: i64,
    pub rollback_streak_threshold: usize,
    pub alert_cooldown_ms: i64,
}

impl Default for MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertPolicy {
    fn default() -> Self {
        Self {
            max_alert_silence_ms: 900_000,
            rollback_streak_threshold: 3,
            alert_cooldown_ms: 300_000,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_alert_at_ms: Option<i64>,
}

pub trait MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertStateStore {
    fn load_state(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertState,
        WorldError,
    >;

    fn save_state(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertState,
    ) -> Result<(), WorldError>;
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertStateStore
{
    states: Arc<
        Mutex<
            BTreeMap<
                (String, String),
                MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertState,
            >,
        >,
    >,
}

impl InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertStateStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertStateStore
    for InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertStateStore
{
    fn load_state(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertState,
        WorldError,
    > {
        let key = normalized_schedule_key(world_id, node_id)?;
        let guard = self.states.lock().map_err(|_| {
            WorldError::Io(
                "membership revocation dead-letter replay rollback governance recovery drill alert state store lock poisoned"
                    .into(),
            )
        })?;
        Ok(guard.get(&key).cloned().unwrap_or_default())
    }

    fn save_state(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertState,
    ) -> Result<(), WorldError> {
        let key = normalized_schedule_key(world_id, node_id)?;
        let mut guard = self.states.lock().map_err(|_| {
            WorldError::Io(
                "membership revocation dead-letter replay rollback governance recovery drill alert state store lock poisoned"
                    .into(),
            )
        })?;
        guard.insert(key, state.clone());
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FileMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertStateStore {
    root_dir: PathBuf,
}

impl FileMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertStateStore {
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, WorldError> {
        let root_dir = root_dir.into();
        fs::create_dir_all(&root_dir)?;
        Ok(Self { root_dir })
    }

    fn state_path(&self, world_id: &str, node_id: &str) -> Result<PathBuf, WorldError> {
        let (world_id, node_id) = normalized_schedule_key(world_id, node_id)?;
        Ok(self.root_dir.join(format!(
            "{world_id}.{node_id}.revocation-dead-letter-replay-rollback-governance-recovery-drill-alert-state.json"
        )))
    }
}

impl MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertStateStore
    for FileMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertStateStore
{
    fn load_state(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertState,
        WorldError,
    > {
        let path = self.state_path(world_id, node_id)?;
        if !path.exists() {
            return Ok(
                MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertState::default(),
            );
        }
        let bytes = fs::read(path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn save_state(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertState,
    ) -> Result<(), WorldError> {
        let path = self.state_path(world_id, node_id)?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, serde_json::to_vec_pretty(state)?)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertRunReport {
    pub world_id: String,
    pub node_id: String,
    pub evaluated_at_ms: i64,
    pub drill_executed: bool,
    pub anomaly_detected: bool,
    pub alert_emitted: bool,
    pub cooldown_blocked: bool,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceArchiveTieredOffloadDrillAlertRunReport
{
    pub prune_report: MembershipRevocationDeadLetterReplayRollbackGovernanceAuditPruneReport,
    pub offload_report:
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadReport,
    pub drill_run_report:
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduledRunReport,
    pub drill_alert_report:
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertRunReport,
}

#[derive(Debug, Clone)]
struct MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadPlan {
    hot_records_after: Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord>,
    offloaded_records: Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord>,
    offloaded_by_age: usize,
    offloaded_by_capacity: usize,
    kept_due_to_rate_limit: usize,
}

impl MembershipSyncClient {
    pub fn offload_revocation_dead_letter_replay_rollback_governance_audit_archive_tiered(
        &self,
        world_id: &str,
        node_id: &str,
        offloaded_at_ms: i64,
        offload_policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadPolicy,
        hot_archive_store: &(
             dyn MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore
                 + Send
                 + Sync
         ),
        cold_archive_store: &(
             dyn MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore
                 + Send
                 + Sync
         ),
    ) -> Result<
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadReport,
        WorldError,
    > {
        validate_governance_audit_tiered_offload_policy(offload_policy)?;
        let (world_id, node_id) = normalized_schedule_key(world_id, node_id)?;
        let hot_records_before = hot_archive_store.list(&world_id, &node_id)?;
        let cold_records_before = cold_archive_store.list(&world_id, &node_id)?;
        let plan = plan_governance_audit_tiered_offload(
            hot_records_before.clone(),
            offloaded_at_ms,
            offload_policy,
        )?;
        if plan.offloaded_records.is_empty() {
            return Ok(
                MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadReport {
                    world_id,
                    node_id,
                    offloaded_at_ms,
                    hot_before: hot_records_before.len(),
                    hot_after: hot_records_before.len(),
                    cold_before: cold_records_before.len(),
                    cold_after: cold_records_before.len(),
                    offloaded: 0,
                    offloaded_by_age: 0,
                    offloaded_by_capacity: 0,
                    kept_due_to_rate_limit: plan.kept_due_to_rate_limit,
                },
            );
        }

        let mut cold_records_after = cold_records_before.clone();
        cold_records_after.extend(plan.offloaded_records.clone());
        cold_archive_store.replace(&world_id, &node_id, &cold_records_after)?;
        if let Err(hot_replace_error) =
            hot_archive_store.replace(&world_id, &node_id, &plan.hot_records_after)
        {
            let compensation =
                cold_archive_store.replace(&world_id, &node_id, &cold_records_before);
            return match compensation {
                Ok(_) => Err(WorldError::Io(format!(
                    "membership revocation dead-letter rollback governance tiered offload hot replace failed and cold layer rolled back: {hot_replace_error:?}"
                ))),
                Err(cold_rollback_error) => Err(WorldError::Io(format!(
                    "membership revocation dead-letter rollback governance tiered offload hot replace failed and cold rollback failed: hot={hot_replace_error:?}, cold={cold_rollback_error:?}"
                ))),
            };
        }

        Ok(
            MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadReport {
                world_id,
                node_id,
                offloaded_at_ms,
                hot_before: hot_records_before.len(),
                hot_after: plan.hot_records_after.len(),
                cold_before: cold_records_before.len(),
                cold_after: cold_records_after.len(),
                offloaded: plan.offloaded_records.len(),
                offloaded_by_age: plan.offloaded_by_age,
                offloaded_by_capacity: plan.offloaded_by_capacity,
                kept_due_to_rate_limit: plan.kept_due_to_rate_limit,
            },
        )
    }

    pub fn emit_revocation_dead_letter_replay_rollback_governance_recovery_drill_alert_if_needed(
        &self,
        world_id: &str,
        node_id: &str,
        evaluated_at_ms: i64,
        drill_run_report: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduledRunReport,
        alert_policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertPolicy,
        alert_state_store: &(
             dyn MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertStateStore
                 + Send
                 + Sync
         ),
        alert_sink: &(dyn MembershipRevocationAlertSink + Send + Sync),
    ) -> Result<
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertRunReport,
        WorldError,
    > {
        validate_recovery_drill_alert_policy(alert_policy)?;
        let (world_id, node_id) = normalized_schedule_key(world_id, node_id)?;
        if !drill_run_report.drill_executed {
            return Ok(
                MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertRunReport {
                    world_id,
                    node_id,
                    evaluated_at_ms,
                    drill_executed: false,
                    anomaly_detected: false,
                    alert_emitted: false,
                    cooldown_blocked: false,
                    reasons: Vec::new(),
                },
            );
        }

        let drill_report = drill_run_report
            .drill_report
            .as_ref()
            .ok_or_else(|| WorldError::DistributedValidationFailed {
                reason: "membership revocation dead-letter rollback governance recovery drill scheduled report missing drill_report while drill_executed is true".to_string(),
            })?;
        let reasons =
            evaluate_recovery_drill_alert_reasons(drill_report, evaluated_at_ms, alert_policy)?;
        if reasons.is_empty() {
            return Ok(
                MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertRunReport {
                    world_id,
                    node_id,
                    evaluated_at_ms,
                    drill_executed: true,
                    anomaly_detected: false,
                    alert_emitted: false,
                    cooldown_blocked: false,
                    reasons,
                },
            );
        }

        let mut alert_state = alert_state_store.load_state(&world_id, &node_id)?;
        let cooldown_blocked = match alert_state.last_alert_at_ms {
            Some(last_alert_at_ms) => {
                let cooldown_elapsed = evaluated_at_ms.checked_sub(last_alert_at_ms).ok_or_else(|| {
                    WorldError::DistributedValidationFailed {
                        reason: format!(
                            "membership revocation dead-letter rollback governance recovery drill alert cooldown age overflow: evaluated_at_ms={evaluated_at_ms}, last_alert_at_ms={last_alert_at_ms}"
                        ),
                    }
                })?;
                cooldown_elapsed < alert_policy.alert_cooldown_ms
            }
            None => false,
        };
        if cooldown_blocked {
            return Ok(
                MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertRunReport {
                    world_id,
                    node_id,
                    evaluated_at_ms,
                    drill_executed: true,
                    anomaly_detected: true,
                    alert_emitted: false,
                    cooldown_blocked: true,
                    reasons,
                },
            );
        }

        let severity = if reasons
            .iter()
            .any(|reason| reason == "emergency_history_detected")
        {
            MembershipRevocationAlertSeverity::Critical
        } else {
            MembershipRevocationAlertSeverity::Warn
        };
        let alert = MembershipRevocationAnomalyAlert {
            world_id: world_id.clone(),
            node_id: node_id.clone(),
            detected_at_ms: evaluated_at_ms,
            severity,
            code: "rollback_governance_recovery_drill_anomaly".to_string(),
            message: format!(
                "membership revocation rollback governance recovery drill detected anomalies: {}",
                reasons.join(",")
            ),
            drained: drill_report.recent_audits.len(),
            diverged: drill_report.governance_state.rollback_streak,
            rejected: usize::from(drill_report.has_emergency_history),
        };
        alert_sink.emit(&alert)?;
        alert_state.last_alert_at_ms = Some(evaluated_at_ms);
        alert_state_store.save_state(&world_id, &node_id, &alert_state)?;
        Ok(
            MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertRunReport {
                world_id,
                node_id,
                evaluated_at_ms,
                drill_executed: true,
                anomaly_detected: true,
                alert_emitted: true,
                cooldown_blocked: false,
                reasons,
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_revocation_dead_letter_replay_rollback_governance_archive_tiered_offload_with_drill_schedule_and_alert(
        &self,
        world_id: &str,
        node_id: &str,
        scheduled_at_ms: i64,
        retention_policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionPolicy,
        offload_policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadPolicy,
        drill_schedule_policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillSchedulePolicy,
        drill_alert_policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertPolicy,
        hot_archive_store: &(dyn MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore
              + Send
              + Sync),
        cold_archive_store: &(dyn MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore
              + Send
              + Sync),
        drill_schedule_state_store: &(dyn MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore
              + Send
              + Sync),
        drill_alert_state_store: &(dyn MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertStateStore
              + Send
              + Sync),
        rollback_alert_state_store: &(dyn MembershipRevocationDeadLetterReplayRollbackAlertStateStore
              + Send
              + Sync),
        rollback_governance_state_store: &(dyn MembershipRevocationDeadLetterReplayRollbackGovernanceStateStore
              + Send
              + Sync),
        rollback_governance_audit_store: &(dyn MembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore
              + Send
              + Sync),
        alert_sink: &(dyn MembershipRevocationAlertSink + Send + Sync),
    ) -> Result<
        MembershipRevocationDeadLetterReplayRollbackGovernanceArchiveTieredOffloadDrillAlertRunReport,
        WorldError,
    >{
        let prune_report = self
            .prune_revocation_dead_letter_replay_rollback_governance_audit_archive(
                world_id,
                node_id,
                scheduled_at_ms,
                retention_policy,
                hot_archive_store,
            )?;
        let offload_report = self
            .offload_revocation_dead_letter_replay_rollback_governance_audit_archive_tiered(
                world_id,
                node_id,
                scheduled_at_ms,
                offload_policy,
                hot_archive_store,
                cold_archive_store,
            )?;
        let drill_run_report = self
            .run_revocation_dead_letter_replay_rollback_governance_recovery_drill_schedule(
                world_id,
                node_id,
                scheduled_at_ms,
                drill_schedule_policy,
                drill_schedule_state_store,
                rollback_alert_state_store,
                rollback_governance_state_store,
                rollback_governance_audit_store,
            )?;
        let drill_alert_report = self
            .emit_revocation_dead_letter_replay_rollback_governance_recovery_drill_alert_if_needed(
                world_id,
                node_id,
                scheduled_at_ms,
                &drill_run_report,
                drill_alert_policy,
                drill_alert_state_store,
                alert_sink,
            )?;
        Ok(
            MembershipRevocationDeadLetterReplayRollbackGovernanceArchiveTieredOffloadDrillAlertRunReport {
                prune_report,
                offload_report,
                drill_run_report,
                drill_alert_report,
            },
        )
    }
}

fn validate_governance_audit_tiered_offload_policy(
    policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadPolicy,
) -> Result<(), WorldError> {
    if policy.hot_max_records == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "membership revocation dead-letter rollback governance tiered offload hot_max_records must be positive".to_string(),
        });
    }
    if policy.offload_min_age_ms <= 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "membership revocation dead-letter rollback governance tiered offload offload_min_age_ms must be positive, got {}",
                policy.offload_min_age_ms
            ),
        });
    }
    if policy.max_offload_records == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "membership revocation dead-letter rollback governance tiered offload max_offload_records must be positive".to_string(),
        });
    }
    Ok(())
}

fn validate_recovery_drill_alert_policy(
    policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertPolicy,
) -> Result<(), WorldError> {
    if policy.max_alert_silence_ms <= 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "membership revocation dead-letter rollback governance recovery drill alert policy max_alert_silence_ms must be positive, got {}",
                policy.max_alert_silence_ms
            ),
        });
    }
    if policy.rollback_streak_threshold == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "membership revocation dead-letter rollback governance recovery drill alert policy rollback_streak_threshold must be positive".to_string(),
        });
    }
    if policy.alert_cooldown_ms <= 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "membership revocation dead-letter rollback governance recovery drill alert policy alert_cooldown_ms must be positive, got {}",
                policy.alert_cooldown_ms
            ),
        });
    }
    Ok(())
}

fn plan_governance_audit_tiered_offload(
    records: Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord>,
    now_ms: i64,
    policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadPolicy,
) -> Result<MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadPlan, WorldError>
{
    let mut selected = vec![false; records.len()];
    let mut selected_by_age = vec![false; records.len()];
    let mut selected_by_capacity = vec![false; records.len()];
    for (index, record) in records.iter().enumerate() {
        let record_age_ms = now_ms.checked_sub(record.audited_at_ms).ok_or_else(|| {
            WorldError::DistributedValidationFailed {
                reason: format!(
                    "membership revocation dead-letter rollback governance tiered offload audit age overflow: now_ms={now_ms}, audited_at_ms={}",
                    record.audited_at_ms
                ),
            }
        })?;
        if record_age_ms >= policy.offload_min_age_ms {
            selected[index] = true;
            selected_by_age[index] = true;
        }
    }

    let unselected_count = selected.iter().filter(|marked| !**marked).count();
    if unselected_count > policy.hot_max_records {
        let mut need_move = unselected_count - policy.hot_max_records;
        for (index, marked) in selected.iter_mut().enumerate() {
            if need_move == 0 {
                break;
            }
            if !*marked {
                *marked = true;
                selected_by_capacity[index] = true;
                need_move -= 1;
            }
        }
    }

    let mut hot_records_after = Vec::new();
    let mut offloaded_records = Vec::new();
    let mut kept_due_to_rate_limit = 0usize;
    let mut offloaded_by_age = 0usize;
    let mut offloaded_by_capacity = 0usize;
    for (index, record) in records.into_iter().enumerate() {
        if selected[index] && offloaded_records.len() < policy.max_offload_records {
            if selected_by_age[index] {
                offloaded_by_age += 1;
            } else if selected_by_capacity[index] {
                offloaded_by_capacity += 1;
            }
            offloaded_records.push(record);
        } else {
            if selected[index] {
                kept_due_to_rate_limit += 1;
            }
            hot_records_after.push(record);
        }
    }

    Ok(
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadPlan {
            hot_records_after,
            offloaded_records,
            offloaded_by_age,
            offloaded_by_capacity,
            kept_due_to_rate_limit,
        },
    )
}

fn evaluate_recovery_drill_alert_reasons(
    drill_report: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillReport,
    now_ms: i64,
    policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertPolicy,
) -> Result<Vec<String>, WorldError> {
    let mut reasons = Vec::new();
    let silence_exceeded = match drill_report.alert_state.last_alert_at_ms {
        Some(last_alert_at_ms) => {
            let silence_elapsed = now_ms.checked_sub(last_alert_at_ms).ok_or_else(|| {
                WorldError::DistributedValidationFailed {
                    reason: format!(
                        "membership revocation dead-letter rollback governance recovery drill alert silence age overflow: now_ms={now_ms}, last_alert_at_ms={last_alert_at_ms}"
                    ),
                }
            })?;
            silence_elapsed > policy.max_alert_silence_ms
        }
        None => true,
    };
    if silence_exceeded {
        reasons.push("alert_state_silence_exceeded".to_string());
    }
    if drill_report.governance_state.rollback_streak >= policy.rollback_streak_threshold {
        reasons.push("rollback_streak_threshold_exceeded".to_string());
    }
    if drill_report.has_emergency_history
        || drill_report.governance_state.last_level
            == MembershipRevocationDeadLetterReplayRollbackGovernanceLevel::Emergency
    {
        reasons.push("emergency_history_detected".to_string());
    }
    Ok(reasons)
}
