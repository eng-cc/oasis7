use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use oasis7_distfs::LocalCasStore;
use serde::{Deserialize, Serialize};

use super::super::error::WorldError;

use super::super::membership_reconciliation::{
    MembershipRevocationAlertSeverity, MembershipRevocationAlertSink,
};
use super::replay_archive::{
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionPolicy,
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRetentionStore,
    MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillSchedulePolicy,
    MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillScheduleStateStore,
};
use super::replay_archive_tiered::{
    MembershipRevocationDeadLetterReplayRollbackGovernanceArchiveTieredOffloadDrillAlertRunReport,
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditTieredOffloadPolicy,
    MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertPolicy,
    MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertRunReport,
    MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertStateStore,
};
use super::replay_audit::{
    MembershipRevocationDeadLetterReplayRollbackAlertStateStore,
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord,
    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditStore,
    MembershipRevocationDeadLetterReplayRollbackGovernanceLevel,
    MembershipRevocationDeadLetterReplayRollbackGovernanceStateStore,
};
use super::{MembershipSyncClient, normalized_schedule_key};
use crate::tiered_file_log;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipRevocationDeadLetterReplayRollbackGovernanceAuditArchiveTier {
    Hot,
    Cold,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceAuditAggregateQueryPolicy {
    pub include_hot: bool,
    pub include_cold: bool,
    pub max_records: usize,
    pub min_audited_at_ms: Option<i64>,
    pub levels: Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceLevel>,
}

impl Default for MembershipRevocationDeadLetterReplayRollbackGovernanceAuditAggregateQueryPolicy {
    fn default() -> Self {
        Self {
            include_hot: true,
            include_cold: true,
            max_records: 200,
            min_audited_at_ms: None,
            levels: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceAuditAggregateQueryRecord {
    pub world_id: String,
    pub node_id: String,
    pub tier: MembershipRevocationDeadLetterReplayRollbackGovernanceAuditArchiveTier,
    pub audit: MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceAuditAggregateQueryReport {
    pub world_id: String,
    pub queried_node_count: usize,
    pub scanned_hot: usize,
    pub scanned_cold: usize,
    pub returned: usize,
    pub records:
        Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceAuditAggregateQueryRecord>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome {
    Emitted,
    SuppressedCooldown,
    SuppressedNoAnomaly,
    SkippedNoDrill,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent {
    pub world_id: String,
    pub node_id: String,
    pub event_at_ms: i64,
    pub outcome:
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<MembershipRevocationAlertSeverity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorState
{
    pub world_id: String,
    pub consumer_id: String,
    pub since_event_at_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since_node_id: Option<String>,
    #[serde(default)]
    pub since_node_event_offset: usize,
}

pub trait MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorStateStore
{
    fn load(
        &self,
        world_id: &str,
        consumer_id: &str,
    ) -> Result<
        Option<
            MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorState,
        >,
        WorldError,
    >;

    fn save(
        &self,
        state: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorState,
    ) -> Result<(), WorldError>;
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorStateStore
{
    states: Arc<
        Mutex<
            BTreeMap<
                (String, String),
                MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorState,
            >,
        >,
    >,
}

impl InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorStateStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorStateStore
    for InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorStateStore
{
    fn load(
        &self,
        world_id: &str,
        consumer_id: &str,
    ) -> Result<
        Option<
            MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorState,
        >,
        WorldError,
    > {
        let key = normalized_schedule_key(world_id, consumer_id)?;
        let guard = self.states.lock().map_err(|_| {
            WorldError::Io(
                "membership revocation dead-letter replay rollback governance recovery drill alert composite sequence cursor state store lock poisoned"
                    .into(),
            )
        })?;
        Ok(guard.get(&key).cloned())
    }

    fn save(
        &self,
        state: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorState,
    ) -> Result<(), WorldError> {
        let key = normalized_schedule_key(&state.world_id, &state.consumer_id)?;
        let mut normalized_state = state.clone();
        normalized_state.world_id = key.0.clone();
        normalized_state.consumer_id = key.1.clone();
        normalized_state.since_node_id = match state.since_node_id.as_deref() {
            Some(node_id) => {
                let (_, normalized_node_id) =
                    normalized_schedule_key(&normalized_state.world_id, node_id)?;
                Some(normalized_node_id)
            }
            None => None,
        };
        let mut guard = self.states.lock().map_err(|_| {
            WorldError::Io(
                "membership revocation dead-letter replay rollback governance recovery drill alert composite sequence cursor state store lock poisoned"
                    .into(),
            )
        })?;
        ensure_composite_sequence_cursor_state_not_rollback(
            guard.get(&key),
            &normalized_state,
        )?;
        guard.insert(key, normalized_state);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FileMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorStateStore
{
    root_dir: PathBuf,
}

impl FileMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorStateStore {
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, WorldError> {
        let root_dir = root_dir.into();
        fs::create_dir_all(&root_dir)?;
        Ok(Self { root_dir })
    }

    fn state_path(&self, world_id: &str, consumer_id: &str) -> Result<PathBuf, WorldError> {
        let (world_id, consumer_id) = normalized_schedule_key(world_id, consumer_id)?;
        Ok(self.root_dir.join(format!(
            "{world_id}.{consumer_id}.revocation-dead-letter-replay-rollback-governance-recovery-drill-alert-composite-sequence-cursor-state.json"
        )))
    }
}

impl MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorStateStore
    for FileMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorStateStore
{
    fn load(
        &self,
        world_id: &str,
        consumer_id: &str,
    ) -> Result<
        Option<
            MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorState,
        >,
        WorldError,
    > {
        let path = self.state_path(world_id, consumer_id)?;
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(path)?;
        if content.trim().is_empty() {
            return Ok(None);
        }
        let state: MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorState =
            serde_json::from_str(&content)?;
        let (normalized_world_id, normalized_consumer_id) =
            normalized_schedule_key(world_id, consumer_id)?;
        if state.world_id != normalized_world_id || state.consumer_id != normalized_consumer_id {
            return Err(WorldError::DistributedValidationFailed {
                reason: "membership revocation dead-letter replay rollback governance recovery drill alert composite sequence cursor state file contains mismatched world_id or consumer_id".to_string(),
            });
        }
        Ok(Some(state))
    }

    fn save(
        &self,
        state: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorState,
    ) -> Result<(), WorldError> {
        let (normalized_world_id, normalized_consumer_id) =
            normalized_schedule_key(&state.world_id, &state.consumer_id)?;
        let path = self.state_path(&normalized_world_id, &normalized_consumer_id)?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        let normalized_since_node_id = match state.since_node_id.as_deref() {
            Some(node_id) => {
                let (_, normalized_node_id) =
                    normalized_schedule_key(&normalized_world_id, node_id)?;
                Some(normalized_node_id)
            }
            None => None,
        };
        let normalized_state =
            MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorState {
                world_id: normalized_world_id,
                consumer_id: normalized_consumer_id,
                since_event_at_ms: state.since_event_at_ms,
                since_node_id: normalized_since_node_id,
                since_node_event_offset: state.since_node_event_offset,
            };
        let existing_state = self.load(&normalized_state.world_id, &normalized_state.consumer_id)?;
        ensure_composite_sequence_cursor_state_not_rollback(
            existing_state.as_ref(),
            &normalized_state,
        )?;
        fs::write(path, serde_json::to_vec(&normalized_state)?)?;
        Ok(())
    }
}

pub trait MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus {
    fn publish(
        &self,
        world_id: &str,
        node_id: &str,
        event: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent,
    ) -> Result<(), WorldError>;

    fn list(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<
        Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent>,
        WorldError,
    >;
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus
{
    events: Arc<
        Mutex<
            BTreeMap<
                (String, String),
                Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent>,
            >,
        >,
    >,
}

impl InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus
    for InMemoryMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus
{
    fn publish(
        &self,
        world_id: &str,
        node_id: &str,
        event: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent,
    ) -> Result<(), WorldError> {
        let key = normalized_schedule_key(world_id, node_id)?;
        let mut guard = self.events.lock().map_err(|_| {
            WorldError::Io(
                "membership revocation dead-letter replay rollback governance recovery drill alert event bus lock poisoned"
                    .into(),
            )
        })?;
        guard.entry(key).or_default().push(event.clone());
        Ok(())
    }

    fn list(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<
        Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent>,
        WorldError,
    > {
        let key = normalized_schedule_key(world_id, node_id)?;
        let guard = self.events.lock().map_err(|_| {
            WorldError::Io(
                "membership revocation dead-letter replay rollback governance recovery drill alert event bus lock poisoned"
                    .into(),
            )
        })?;
        Ok(guard.get(&key).cloned().unwrap_or_default())
    }
}

#[derive(Debug, Clone)]
pub struct FileMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus {
    root_dir: PathBuf,
    cas_store: LocalCasStore,
}

impl FileMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus {
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, WorldError> {
        let root_dir = root_dir.into();
        fs::create_dir_all(&root_dir)?;
        Ok(Self {
            cas_store: LocalCasStore::new(root_dir.join("cas")),
            root_dir,
        })
    }

    fn event_path(&self, world_id: &str, node_id: &str) -> Result<PathBuf, WorldError> {
        let (world_id, node_id) = normalized_schedule_key(world_id, node_id)?;
        Ok(self.root_dir.join(format!(
            "{world_id}.{node_id}.revocation-dead-letter-replay-rollback-governance-recovery-drill-alert-event.jsonl"
        )))
    }

    fn event_cold_refs_path(&self, world_id: &str, node_id: &str) -> Result<PathBuf, WorldError> {
        let (world_id, node_id) = normalized_schedule_key(world_id, node_id)?;
        Ok(self.root_dir.join(format!(
            "{world_id}.{node_id}.revocation-dead-letter-replay-rollback-governance-recovery-drill-alert-event.cold.refs.jsonl"
        )))
    }
}

const RECOVERY_DRILL_ALERT_EVENT_HOT_MAX_RECORDS: usize = 4096;
const RECOVERY_DRILL_ALERT_EVENT_COLD_SEGMENT_MAX_LINES: usize = 256;

impl MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus
    for FileMembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus
{
    fn publish(
        &self,
        world_id: &str,
        node_id: &str,
        event: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent,
    ) -> Result<(), WorldError> {
        let path = self.event_path(world_id, node_id)?;
        let cold_refs_path = self.event_cold_refs_path(world_id, node_id)?;
        let line = serde_json::to_string(event)?;
        tiered_file_log::append_jsonl_line_with_cas_offload(
            path.as_path(),
            cold_refs_path.as_path(),
            &self.cas_store,
            RECOVERY_DRILL_ALERT_EVENT_HOT_MAX_RECORDS,
            RECOVERY_DRILL_ALERT_EVENT_COLD_SEGMENT_MAX_LINES,
            line.as_str(),
        )
    }

    fn list(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<
        Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent>,
        WorldError,
    > {
        let path = self.event_path(world_id, node_id)?;
        let cold_refs_path = self.event_cold_refs_path(world_id, node_id)?;
        let lines = tiered_file_log::collect_jsonl_lines_with_cas_refs(
            path.as_path(),
            cold_refs_path.as_path(),
            &self.cas_store,
        )?;
        let mut events = Vec::new();
        for line in lines {
            events.push(serde_json::from_str(line.as_str())?);
        }
        Ok(events)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationDeadLetterReplayRollbackGovernanceArchiveTieredOffloadDrillAlertEventBusRunReport
{
    pub run_report:
        MembershipRevocationDeadLetterReplayRollbackGovernanceArchiveTieredOffloadDrillAlertRunReport,
    pub alert_event: MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent,
}

impl MembershipSyncClient {
    pub fn query_revocation_dead_letter_replay_rollback_governance_audit_archive_aggregated(
        &self,
        world_id: &str,
        node_ids: &[String],
        policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceAuditAggregateQueryPolicy,
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
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditAggregateQueryReport,
        WorldError,
    > {
        validate_governance_audit_aggregate_query_policy(policy)?;
        if node_ids.is_empty() {
            return Err(WorldError::DistributedValidationFailed {
                reason: "membership revocation dead-letter rollback governance audit aggregate query requires at least one node_id".to_string(),
            });
        }
        let first_node_id = node_ids.first().ok_or_else(|| WorldError::DistributedValidationFailed {
            reason: "membership revocation dead-letter rollback governance audit aggregate query requires at least one node_id".to_string(),
        })?;
        let (normalized_world_id, _) = normalized_schedule_key(world_id, first_node_id)?;
        let mut queried_nodes = BTreeSet::new();
        for node_id in node_ids {
            let (_, node_id) = normalized_schedule_key(&normalized_world_id, node_id)?;
            queried_nodes.insert(node_id);
        }
        let queried_node_count = queried_nodes.len();

        let mut scanned_hot = 0usize;
        let mut scanned_cold = 0usize;
        let mut records = Vec::new();
        for node_id in queried_nodes {
            if policy.include_hot {
                let hot = hot_archive_store.list(&normalized_world_id, &node_id)?;
                scanned_hot = checked_usize_add(
                    scanned_hot,
                    hot.len(),
                    "membership revocation governance aggregate scanned_hot",
                )?;
                append_aggregate_records(
                    &mut records,
                    &normalized_world_id,
                    &node_id,
                    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditArchiveTier::Hot,
                    hot,
                    policy,
                );
            }
            if policy.include_cold {
                let cold = cold_archive_store.list(&normalized_world_id, &node_id)?;
                scanned_cold = checked_usize_add(
                    scanned_cold,
                    cold.len(),
                    "membership revocation governance aggregate scanned_cold",
                )?;
                append_aggregate_records(
                    &mut records,
                    &normalized_world_id,
                    &node_id,
                    MembershipRevocationDeadLetterReplayRollbackGovernanceAuditArchiveTier::Cold,
                    cold,
                    policy,
                );
            }
        }

        records.sort_by(|left, right| {
            right
                .audit
                .audited_at_ms
                .cmp(&left.audit.audited_at_ms)
                .then_with(|| left.node_id.cmp(&right.node_id))
                .then_with(|| left.tier.cmp(&right.tier))
        });
        if records.len() > policy.max_records {
            records.truncate(policy.max_records);
        }

        Ok(
            MembershipRevocationDeadLetterReplayRollbackGovernanceAuditAggregateQueryReport {
                world_id: normalized_world_id,
                queried_node_count,
                scanned_hot,
                scanned_cold,
                returned: records.len(),
                records,
            },
        )
    }

    pub fn query_revocation_dead_letter_replay_rollback_governance_recovery_drill_alert_events_aggregated(
        &self,
        world_id: &str,
        node_ids: &[String],
        min_event_at_ms: Option<i64>,
        outcomes: &[MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome],
        offset: usize,
        max_records: usize,
        event_bus: &(
             dyn MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus
                 + Send
                 + Sync
         ),
    ) -> Result<
        Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent>,
        WorldError,
    > {
        validate_governance_recovery_drill_alert_event_aggregate_query_args(max_records)?;
        let mut events = collect_governance_recovery_drill_alert_events_aggregated(
            world_id,
            node_ids,
            min_event_at_ms,
            outcomes,
            event_bus,
        )?;
        events.sort_by(|left, right| {
            right
                .event_at_ms
                .cmp(&left.event_at_ms)
                .then_with(|| left.node_id.cmp(&right.node_id))
        });
        if offset >= events.len() {
            return Ok(Vec::new());
        }
        Ok(events.into_iter().skip(offset).take(max_records).collect())
    }

    pub fn query_revocation_dead_letter_replay_rollback_governance_recovery_drill_alert_events_incremental_since(
        &self,
        world_id: &str,
        node_ids: &[String],
        since_event_at_ms: i64,
        outcomes: &[MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome],
        max_records: usize,
        event_bus: &(
             dyn MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus
                 + Send
                 + Sync
         ),
    ) -> Result<
        Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent>,
        WorldError,
    > {
        validate_governance_recovery_drill_alert_event_aggregate_query_args(max_records)?;
        let mut events = collect_governance_recovery_drill_alert_events_aggregated(
            world_id, node_ids, None, outcomes, event_bus,
        )?;
        events.retain(|event| event.event_at_ms > since_event_at_ms);
        events.sort_by(|left, right| {
            left.event_at_ms
                .cmp(&right.event_at_ms)
                .then_with(|| left.node_id.cmp(&right.node_id))
        });
        if events.len() > max_records {
            events.truncate(max_records);
        }
        Ok(events)
    }

    pub fn query_revocation_dead_letter_replay_rollback_governance_recovery_drill_alert_events_incremental_since_with_next_watermark(
        &self,
        world_id: &str,
        node_ids: &[String],
        since_event_at_ms: i64,
        outcomes: &[MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome],
        max_records: usize,
        event_bus: &(
             dyn MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus
                 + Send
                 + Sync
         ),
    ) -> Result<
        (
            Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent>,
            i64,
        ),
        WorldError,
    > {
        let events = self
            .query_revocation_dead_letter_replay_rollback_governance_recovery_drill_alert_events_incremental_since(
                world_id,
                node_ids,
                since_event_at_ms,
                outcomes,
                max_records,
                event_bus,
            )?;
        let next_since_event_at_ms = events
            .last()
            .map(|event| event.event_at_ms)
            .unwrap_or(since_event_at_ms)
            .max(since_event_at_ms);
        Ok((events, next_since_event_at_ms))
    }

    pub fn query_revocation_dead_letter_replay_rollback_governance_recovery_drill_alert_events_incremental_since_cursor(
        &self,
        world_id: &str,
        node_ids: &[String],
        since_event_at_ms: i64,
        since_node_id: Option<&str>,
        outcomes: &[MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome],
        max_records: usize,
        event_bus: &(
             dyn MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus
                 + Send
                 + Sync
         ),
    ) -> Result<
        (
            Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent>,
            i64,
            Option<String>,
        ),
        WorldError,
    > {
        validate_governance_recovery_drill_alert_event_aggregate_query_args(max_records)?;
        let normalized_since_node_id = if let Some(node_id) = since_node_id {
            let (_, normalized_node_id) = normalized_schedule_key(world_id, node_id)?;
            Some(normalized_node_id)
        } else {
            None
        };
        let mut events = collect_governance_recovery_drill_alert_events_aggregated(
            world_id, node_ids, None, outcomes, event_bus,
        )?;
        events.retain(|event| {
            if event.event_at_ms > since_event_at_ms {
                return true;
            }
            if event.event_at_ms < since_event_at_ms {
                return false;
            }
            match normalized_since_node_id.as_deref() {
                Some(since_node_id) => event.node_id.as_str() > since_node_id,
                None => true,
            }
        });
        events.sort_by(|left, right| {
            left.event_at_ms
                .cmp(&right.event_at_ms)
                .then_with(|| left.node_id.cmp(&right.node_id))
        });
        if events.len() > max_records {
            events.truncate(max_records);
        }
        let (next_event_at_ms, next_node_id) = match events.last() {
            Some(last) => (last.event_at_ms, Some(last.node_id.clone())),
            None => (since_event_at_ms, normalized_since_node_id),
        };
        Ok((events, next_event_at_ms, next_node_id))
    }

    pub fn query_revocation_dead_letter_replay_rollback_governance_recovery_drill_alert_events_incremental_since_composite_sequence_cursor(
        &self,
        world_id: &str,
        node_ids: &[String],
        since_event_at_ms: i64,
        since_node_id: Option<&str>,
        since_node_event_offset: usize,
        outcomes: &[MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome],
        max_records: usize,
        event_bus: &(
             dyn MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus
                 + Send
                 + Sync
         ),
    ) -> Result<
        (
            Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent>,
            i64,
            Option<String>,
            usize,
        ),
        WorldError,
    > {
        validate_governance_recovery_drill_alert_event_aggregate_query_args(max_records)?;
        let normalized_since_node_id = if let Some(node_id) = since_node_id {
            let (_, normalized_node_id) = normalized_schedule_key(world_id, node_id)?;
            Some(normalized_node_id)
        } else {
            None
        };
        let events = collect_governance_recovery_drill_alert_events_aggregated(
            world_id, node_ids, None, outcomes, event_bus,
        )?;
        let mut node_offsets = BTreeMap::new();
        let mut cursor_rows = Vec::with_capacity(events.len());
        for event in events {
            let next_offset_entry = node_offsets.entry(event.node_id.clone()).or_insert(0usize);
            let node_event_offset = *next_offset_entry;
            *next_offset_entry = checked_usize_increment(
                *next_offset_entry,
                "membership revocation governance composite sequence cursor node_event_offset",
            )?;
            cursor_rows.push((event, node_event_offset));
        }
        cursor_rows.retain(|(event, node_event_offset)| {
            if event.event_at_ms > since_event_at_ms {
                return true;
            }
            if event.event_at_ms < since_event_at_ms {
                return false;
            }
            match normalized_since_node_id.as_deref() {
                Some(since_node_id) => {
                    if event.node_id.as_str() > since_node_id {
                        true
                    } else if event.node_id.as_str() < since_node_id {
                        false
                    } else {
                        *node_event_offset > since_node_event_offset
                    }
                }
                None => true,
            }
        });
        cursor_rows.sort_by(|left, right| {
            left.0
                .event_at_ms
                .cmp(&right.0.event_at_ms)
                .then_with(|| left.0.node_id.cmp(&right.0.node_id))
                .then_with(|| left.1.cmp(&right.1))
        });
        if cursor_rows.len() > max_records {
            cursor_rows.truncate(max_records);
        }
        let (next_event_at_ms, next_node_id, next_node_event_offset) = match cursor_rows.last() {
            Some((event, node_event_offset)) => (
                event.event_at_ms,
                Some(event.node_id.clone()),
                *node_event_offset,
            ),
            None => (
                since_event_at_ms,
                normalized_since_node_id,
                since_node_event_offset,
            ),
        };
        let events = cursor_rows
            .into_iter()
            .map(|(event, _)| event)
            .collect::<Vec<_>>();
        Ok((
            events,
            next_event_at_ms,
            next_node_id,
            next_node_event_offset,
        ))
    }

    pub fn query_revocation_dead_letter_replay_rollback_governance_recovery_drill_alert_events_incremental_with_composite_sequence_cursor_state(
        &self,
        world_id: &str,
        consumer_id: &str,
        node_ids: &[String],
        initial_since_event_at_ms: i64,
        initial_since_node_id: Option<&str>,
        initial_since_node_event_offset: usize,
        outcomes: &[MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome],
        max_records: usize,
        event_bus: &(dyn MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus
              + Send
              + Sync),
        state_store: &(dyn MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorStateStore
              + Send
              + Sync),
    ) -> Result<
        (
            Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent>,
            MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorState,
        ),
        WorldError,
    >{
        let (normalized_world_id, normalized_consumer_id) =
            normalized_schedule_key(world_id, consumer_id)?;
        let loaded_state = state_store.load(&normalized_world_id, &normalized_consumer_id)?;
        let (since_event_at_ms, since_node_id, since_node_event_offset) = match loaded_state {
            Some(state) => (
                state.since_event_at_ms,
                state.since_node_id,
                state.since_node_event_offset,
            ),
            None => (
                initial_since_event_at_ms,
                initial_since_node_id.map(|value| value.to_string()),
                initial_since_node_event_offset,
            ),
        };
        let (events, next_event_at_ms, next_node_id, next_node_event_offset) = self
            .query_revocation_dead_letter_replay_rollback_governance_recovery_drill_alert_events_incremental_since_composite_sequence_cursor(
                &normalized_world_id,
                node_ids,
                since_event_at_ms,
                since_node_id.as_deref(),
                since_node_event_offset,
                outcomes,
                max_records,
                event_bus,
            )?;
        let next_state =
            MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorState {
                world_id: normalized_world_id,
                consumer_id: normalized_consumer_id,
                since_event_at_ms: next_event_at_ms,
                since_node_id: next_node_id,
                since_node_event_offset: next_node_event_offset,
            };
        state_store.save(&next_state)?;
        Ok((events, next_state))
    }

    pub fn summarize_revocation_dead_letter_replay_rollback_governance_recovery_drill_alert_events_aggregated_by_outcome(
        &self,
        world_id: &str,
        node_ids: &[String],
        min_event_at_ms: Option<i64>,
        event_bus: &(
             dyn MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus
                 + Send
                 + Sync
         ),
    ) -> Result<BTreeMap<String, usize>, WorldError> {
        let events = collect_governance_recovery_drill_alert_events_aggregated(
            world_id,
            node_ids,
            min_event_at_ms,
            &[],
            event_bus,
        )?;
        let mut summary = BTreeMap::new();
        for event in events {
            *summary
                .entry(
                    governance_recovery_drill_alert_event_outcome_label(event.outcome).to_string(),
                )
                .or_insert(0) += 1;
        }
        Ok(summary)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_revocation_dead_letter_replay_rollback_governance_archive_tiered_offload_with_drill_schedule_alert_and_event_bus(
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
        event_bus: &(dyn MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus
              + Send
              + Sync),
    ) -> Result<
        MembershipRevocationDeadLetterReplayRollbackGovernanceArchiveTieredOffloadDrillAlertEventBusRunReport,
        WorldError,
    >{
        let run_report = self
            .run_revocation_dead_letter_replay_rollback_governance_archive_tiered_offload_with_drill_schedule_and_alert(
                world_id,
                node_id,
                scheduled_at_ms,
                retention_policy,
                offload_policy,
                drill_schedule_policy,
                drill_alert_policy,
                hot_archive_store,
                cold_archive_store,
                drill_schedule_state_store,
                drill_alert_state_store,
                rollback_alert_state_store,
                rollback_governance_state_store,
                rollback_governance_audit_store,
                alert_sink,
            )?;
        let alert_event =
            alert_event_from_run_report(scheduled_at_ms, &run_report.drill_alert_report);
        event_bus.publish(world_id, node_id, &alert_event)?;
        Ok(
            MembershipRevocationDeadLetterReplayRollbackGovernanceArchiveTieredOffloadDrillAlertEventBusRunReport {
                run_report,
                alert_event,
            },
        )
    }
}

fn validate_governance_audit_aggregate_query_policy(
    policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceAuditAggregateQueryPolicy,
) -> Result<(), WorldError> {
    if !policy.include_hot && !policy.include_cold {
        return Err(WorldError::DistributedValidationFailed {
            reason: "membership revocation dead-letter rollback governance audit aggregate query requires include_hot or include_cold".to_string(),
        });
    }
    if policy.max_records == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "membership revocation dead-letter rollback governance audit aggregate query max_records must be positive".to_string(),
        });
    }
    Ok(())
}

fn validate_governance_recovery_drill_alert_event_aggregate_query_args(
    max_records: usize,
) -> Result<(), WorldError> {
    if max_records == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "membership revocation dead-letter rollback governance recovery drill alert event aggregate query max_records must be positive".to_string(),
        });
    }
    Ok(())
}

fn checked_usize_add(lhs: usize, rhs: usize, context: &str) -> Result<usize, WorldError> {
    lhs.checked_add(rhs)
        .ok_or_else(|| WorldError::DistributedValidationFailed {
            reason: format!("{context} overflow: lhs={lhs}, rhs={rhs}"),
        })
}

fn checked_usize_increment(value: usize, context: &str) -> Result<usize, WorldError> {
    checked_usize_add(value, 1, context)
}

fn compare_composite_sequence_cursor(
    left_since_event_at_ms: i64,
    left_since_node_id: Option<&str>,
    left_since_node_event_offset: usize,
    right_since_event_at_ms: i64,
    right_since_node_id: Option<&str>,
    right_since_node_event_offset: usize,
) -> std::cmp::Ordering {
    left_since_event_at_ms
        .cmp(&right_since_event_at_ms)
        .then_with(|| left_since_node_id.cmp(&right_since_node_id))
        .then_with(|| left_since_node_event_offset.cmp(&right_since_node_event_offset))
}

fn ensure_composite_sequence_cursor_state_not_rollback(
    previous: Option<
        &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorState,
    >,
    next: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventCompositeSequenceCursorState,
) -> Result<(), WorldError> {
    if let Some(previous) = previous {
        let ordering = compare_composite_sequence_cursor(
            previous.since_event_at_ms,
            previous.since_node_id.as_deref(),
            previous.since_node_event_offset,
            next.since_event_at_ms,
            next.since_node_id.as_deref(),
            next.since_node_event_offset,
        );
        if ordering == std::cmp::Ordering::Greater {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "membership revocation dead-letter replay rollback governance recovery drill alert composite sequence cursor state cannot rollback for world {} consumer {}",
                    next.world_id, next.consumer_id
                ),
            });
        }
    }
    Ok(())
}

fn collect_governance_recovery_drill_alert_events_aggregated(
    world_id: &str,
    node_ids: &[String],
    min_event_at_ms: Option<i64>,
    outcomes: &[MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome],
    event_bus: &(
         dyn MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventBus
             + Send
             + Sync
     ),
) -> Result<
    Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent>,
    WorldError,
> {
    if node_ids.is_empty() {
        return Err(WorldError::DistributedValidationFailed {
            reason: "membership revocation dead-letter rollback governance recovery drill alert event aggregate query requires at least one node_id".to_string(),
        });
    }
    let first_node_id = node_ids.first().ok_or_else(|| WorldError::DistributedValidationFailed {
        reason: "membership revocation dead-letter rollback governance recovery drill alert event aggregate query requires at least one node_id".to_string(),
    })?;
    let (normalized_world_id, _) = normalized_schedule_key(world_id, first_node_id)?;
    let mut queried_nodes = BTreeSet::new();
    for node_id in node_ids {
        let (_, node_id) = normalized_schedule_key(&normalized_world_id, node_id)?;
        queried_nodes.insert(node_id);
    }
    let mut events = Vec::new();
    for node_id in queried_nodes {
        let node_events = event_bus.list(&normalized_world_id, &node_id)?;
        for event in node_events {
            if event.world_id != normalized_world_id || event.node_id != node_id {
                continue;
            }
            if let Some(min_event_at_ms) = min_event_at_ms {
                if event.event_at_ms < min_event_at_ms {
                    continue;
                }
            }
            if !outcomes.is_empty() && !outcomes.contains(&event.outcome) {
                continue;
            }
            events.push(event);
        }
    }
    Ok(events)
}

fn governance_recovery_drill_alert_event_outcome_label(
    outcome: MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome,
) -> &'static str {
    match outcome {
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome::Emitted => "emitted",
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome::SuppressedCooldown => "suppressed_cooldown",
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome::SuppressedNoAnomaly => "suppressed_no_anomaly",
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome::SkippedNoDrill => "skipped_no_drill",
    }
}

fn append_aggregate_records(
    records: &mut Vec<
        MembershipRevocationDeadLetterReplayRollbackGovernanceAuditAggregateQueryRecord,
    >,
    world_id: &str,
    node_id: &str,
    tier: MembershipRevocationDeadLetterReplayRollbackGovernanceAuditArchiveTier,
    source: Vec<MembershipRevocationDeadLetterReplayRollbackGovernanceAuditRecord>,
    policy: &MembershipRevocationDeadLetterReplayRollbackGovernanceAuditAggregateQueryPolicy,
) {
    for audit in source {
        if let Some(min_audited_at_ms) = policy.min_audited_at_ms {
            if audit.audited_at_ms < min_audited_at_ms {
                continue;
            }
        }
        if !policy.levels.is_empty() && !policy.levels.contains(&audit.governance_level) {
            continue;
        }
        records.push(
            MembershipRevocationDeadLetterReplayRollbackGovernanceAuditAggregateQueryRecord {
                world_id: world_id.to_string(),
                node_id: node_id.to_string(),
                tier,
                audit,
            },
        );
    }
}

fn alert_event_from_run_report(
    event_at_ms: i64,
    run_report: &MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertRunReport,
) -> MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent {
    let outcome = if !run_report.drill_executed {
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome::SkippedNoDrill
    } else if !run_report.anomaly_detected {
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome::SuppressedNoAnomaly
    } else if run_report.alert_emitted {
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome::Emitted
    } else if run_report.cooldown_blocked {
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome::SuppressedCooldown
    } else {
        MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEventOutcome::SuppressedNoAnomaly
    };
    let severity = if !run_report.anomaly_detected {
        None
    } else if run_report
        .reasons
        .iter()
        .any(|reason| reason == "emergency_history_detected")
    {
        Some(MembershipRevocationAlertSeverity::Critical)
    } else {
        Some(MembershipRevocationAlertSeverity::Warn)
    };
    MembershipRevocationDeadLetterReplayRollbackGovernanceRecoveryDrillAlertEvent {
        world_id: run_report.world_id.clone(),
        node_id: run_report.node_id.clone(),
        event_at_ms,
        outcome,
        reasons: run_report.reasons.clone(),
        severity,
    }
}

#[cfg(test)]
mod tests;
