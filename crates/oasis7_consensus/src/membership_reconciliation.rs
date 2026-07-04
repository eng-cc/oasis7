use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use oasis7_distfs::LocalCasStore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::distributed::topic_membership_reconcile;
use super::error::WorldError;
use super::membership::{
    MembershipDirectorySignerKeyring, MembershipSyncClient, MembershipSyncSubscription,
    to_canonical_cbor,
};
use super::membership_logic;
use super::tiered_file_log;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipRevocationCheckpointAnnounce {
    pub world_id: String,
    pub node_id: String,
    pub announced_at_ms: i64,
    pub revoked_key_ids: Vec<String>,
    pub revoked_set_hash: String,
}

impl MembershipRevocationCheckpointAnnounce {
    pub fn from_revoked_keys(
        world_id: &str,
        node_id: &str,
        announced_at_ms: i64,
        revoked_key_ids: Vec<String>,
    ) -> Result<Self, WorldError> {
        let world_id = membership_logic::normalized_world_id(world_id)?;
        let node_id = normalized_node_id(node_id)?;
        let revoked_key_ids =
            normalize_revoked_key_ids(revoked_key_ids.iter().map(String::as_str))?;
        let revoked_set_hash = revoked_keys_hash(&revoked_key_ids)?;
        Ok(Self {
            world_id,
            node_id,
            announced_at_ms,
            revoked_key_ids,
            revoked_set_hash,
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MembershipRevocationReconcilePolicy {
    pub trusted_nodes: Vec<String>,
    pub auto_revoke_missing_keys: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationReconcileReport {
    pub drained: usize,
    pub in_sync: usize,
    pub diverged: usize,
    pub merged: usize,
    pub rejected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationAlertPolicy {
    pub warn_diverged_threshold: usize,
    pub critical_rejected_threshold: usize,
}

impl Default for MembershipRevocationAlertPolicy {
    fn default() -> Self {
        Self {
            warn_diverged_threshold: 1,
            critical_rejected_threshold: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipRevocationAlertSeverity {
    Warn,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipRevocationAnomalyAlert {
    pub world_id: String,
    pub node_id: String,
    pub detected_at_ms: i64,
    pub severity: MembershipRevocationAlertSeverity,
    pub code: String,
    pub message: String,
    pub drained: usize,
    pub diverged: usize,
    pub rejected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationAlertDedupPolicy {
    pub suppress_window_ms: i64,
}

impl Default for MembershipRevocationAlertDedupPolicy {
    fn default() -> Self {
        Self {
            suppress_window_ms: 60_000,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipRevocationAlertDedupState {
    pub last_emitted_at_by_key: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationReconcileSchedulePolicy {
    pub checkpoint_interval_ms: i64,
    pub reconcile_interval_ms: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipRevocationReconcileScheduleState {
    pub last_checkpoint_at_ms: Option<i64>,
    pub last_reconcile_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationScheduledRunReport {
    pub checkpoint_published: bool,
    pub reconcile_executed: bool,
    pub reconcile_report: Option<MembershipRevocationReconcileReport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationCoordinatedRunReport {
    pub acquired: bool,
    pub emitted_alerts: usize,
    pub run_report: Option<MembershipRevocationScheduledRunReport>,
}

pub trait MembershipRevocationAlertSink {
    fn emit(&self, alert: &MembershipRevocationAnomalyAlert) -> Result<(), WorldError>;
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryMembershipRevocationAlertSink {
    alerts: Arc<Mutex<Vec<MembershipRevocationAnomalyAlert>>>,
}

impl InMemoryMembershipRevocationAlertSink {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn list(&self) -> Result<Vec<MembershipRevocationAnomalyAlert>, WorldError> {
        let guard = self
            .alerts
            .lock()
            .map_err(|_| WorldError::Io("membership revocation alert sink lock poisoned".into()))?;
        Ok(guard.clone())
    }
}

impl MembershipRevocationAlertSink for InMemoryMembershipRevocationAlertSink {
    fn emit(&self, alert: &MembershipRevocationAnomalyAlert) -> Result<(), WorldError> {
        let mut guard = self
            .alerts
            .lock()
            .map_err(|_| WorldError::Io("membership revocation alert sink lock poisoned".into()))?;
        guard.push(alert.clone());
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FileMembershipRevocationAlertSink {
    root_dir: PathBuf,
    cas_store: LocalCasStore,
}

const REVOCATION_ALERT_HOT_MAX_RECORDS: usize = 4096;
const REVOCATION_ALERT_COLD_SEGMENT_MAX_LINES: usize = 256;

impl FileMembershipRevocationAlertSink {
    pub fn new(root_dir: impl Into<PathBuf>) -> Result<Self, WorldError> {
        let root_dir = root_dir.into();
        fs::create_dir_all(&root_dir)?;
        Ok(Self {
            cas_store: LocalCasStore::new(root_dir.join("cas")),
            root_dir,
        })
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }

    fn world_log_path(&self, world_id: &str) -> Result<PathBuf, WorldError> {
        let world_id = membership_logic::normalized_world_id(world_id)?;
        Ok(self
            .root_dir
            .join(format!("{world_id}.revocation-alerts.jsonl")))
    }

    fn world_cold_refs_path(&self, world_id: &str) -> Result<PathBuf, WorldError> {
        let world_id = membership_logic::normalized_world_id(world_id)?;
        Ok(self
            .root_dir
            .join(format!("{world_id}.revocation-alerts.cold.refs.jsonl")))
    }

    pub fn list(
        &self,
        world_id: &str,
    ) -> Result<Vec<MembershipRevocationAnomalyAlert>, WorldError> {
        let path = self.world_log_path(world_id)?;
        let cold_refs_path = self.world_cold_refs_path(world_id)?;
        let lines = tiered_file_log::collect_jsonl_lines_with_cas_refs(
            path.as_path(),
            cold_refs_path.as_path(),
            &self.cas_store,
        )?;
        let mut alerts = Vec::new();
        for line in lines {
            alerts.push(serde_json::from_str(line.as_str())?);
        }
        Ok(alerts)
    }
}

impl MembershipRevocationAlertSink for FileMembershipRevocationAlertSink {
    fn emit(&self, alert: &MembershipRevocationAnomalyAlert) -> Result<(), WorldError> {
        let path = self.world_log_path(&alert.world_id)?;
        let cold_refs_path = self.world_cold_refs_path(&alert.world_id)?;
        let line = serde_json::to_string(alert)?;
        tiered_file_log::append_jsonl_line_with_cas_offload(
            path.as_path(),
            cold_refs_path.as_path(),
            &self.cas_store,
            REVOCATION_ALERT_HOT_MAX_RECORDS,
            REVOCATION_ALERT_COLD_SEGMENT_MAX_LINES,
            line.as_str(),
        )
    }
}

pub trait MembershipRevocationScheduleStateStore {
    fn load(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<MembershipRevocationReconcileScheduleState, WorldError>;

    fn save(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationReconcileScheduleState,
    ) -> Result<(), WorldError>;
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryMembershipRevocationScheduleStateStore {
    states: Arc<Mutex<BTreeMap<(String, String), MembershipRevocationReconcileScheduleState>>>,
}

impl InMemoryMembershipRevocationScheduleStateStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl MembershipRevocationScheduleStateStore for InMemoryMembershipRevocationScheduleStateStore {
    fn load(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<MembershipRevocationReconcileScheduleState, WorldError> {
        let key = normalized_schedule_key(world_id, node_id)?;
        let guard = self.states.lock().map_err(|_| {
            WorldError::Io("membership revocation schedule store lock poisoned".into())
        })?;
        Ok(guard.get(&key).cloned().unwrap_or_default())
    }

    fn save(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationReconcileScheduleState,
    ) -> Result<(), WorldError> {
        let key = normalized_schedule_key(world_id, node_id)?;
        let mut guard = self.states.lock().map_err(|_| {
            WorldError::Io("membership revocation schedule store lock poisoned".into())
        })?;
        guard.insert(key, state.clone());
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FileMembershipRevocationScheduleStateStore {
    root_dir: PathBuf,
}

impl FileMembershipRevocationScheduleStateStore {
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
            "{world_id}.{node_id}.revocation-schedule-state.json"
        )))
    }
}

impl MembershipRevocationScheduleStateStore for FileMembershipRevocationScheduleStateStore {
    fn load(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<MembershipRevocationReconcileScheduleState, WorldError> {
        let path = self.state_path(world_id, node_id)?;
        if !path.exists() {
            return Ok(MembershipRevocationReconcileScheduleState::default());
        }

        let bytes = fs::read(path)?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    fn save(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationReconcileScheduleState,
    ) -> Result<(), WorldError> {
        let path = self.state_path(world_id, node_id)?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }

        let bytes = serde_json::to_vec(state)?;
        fs::write(path, bytes)?;
        Ok(())
    }
}

pub trait MembershipRevocationScheduleCoordinator {
    fn acquire(
        &self,
        world_id: &str,
        node_id: &str,
        now_ms: i64,
        lease_ttl_ms: i64,
    ) -> Result<bool, WorldError>;

    fn release(&self, world_id: &str, node_id: &str) -> Result<(), WorldError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct InMemoryCoordinatorLease {
    holder_node_id: String,
    expires_at_ms: i64,
}

#[derive(Debug, Clone, Default)]
pub struct InMemoryMembershipRevocationScheduleCoordinator {
    leases: Arc<Mutex<BTreeMap<String, InMemoryCoordinatorLease>>>,
}

impl InMemoryMembershipRevocationScheduleCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn holder_node(&self, world_id: &str) -> Result<Option<String>, WorldError> {
        let world_id = membership_logic::normalized_world_id(world_id)?;
        let guard = self.leases.lock().map_err(|_| {
            WorldError::Io("membership revocation coordinator lock poisoned".into())
        })?;
        Ok(guard
            .get(&world_id)
            .map(|lease| lease.holder_node_id.clone()))
    }
}

impl MembershipRevocationScheduleCoordinator for InMemoryMembershipRevocationScheduleCoordinator {
    fn acquire(
        &self,
        world_id: &str,
        node_id: &str,
        now_ms: i64,
        lease_ttl_ms: i64,
    ) -> Result<bool, WorldError> {
        validate_coordinator_lease_ttl_ms(lease_ttl_ms)?;
        let world_id = membership_logic::normalized_world_id(world_id)?;
        let node_id = normalized_node_id(node_id)?;

        let mut guard = self.leases.lock().map_err(|_| {
            WorldError::Io("membership revocation coordinator lock poisoned".into())
        })?;

        if let Some(existing) = guard.get(&world_id) {
            let lease_active = now_ms < existing.expires_at_ms;
            if lease_active && existing.holder_node_id != node_id {
                return Ok(false);
            }
        }

        let expires_at_ms = checked_coordinator_lease_expiry(now_ms, lease_ttl_ms)?;

        guard.insert(
            world_id,
            InMemoryCoordinatorLease {
                holder_node_id: node_id,
                expires_at_ms,
            },
        );
        Ok(true)
    }

    fn release(&self, world_id: &str, node_id: &str) -> Result<(), WorldError> {
        let world_id = membership_logic::normalized_world_id(world_id)?;
        let node_id = normalized_node_id(node_id)?;

        let mut guard = self.leases.lock().map_err(|_| {
            WorldError::Io("membership revocation coordinator lock poisoned".into())
        })?;

        let should_remove = guard
            .get(&world_id)
            .map(|lease| lease.holder_node_id == node_id)
            .unwrap_or(false);
        if should_remove {
            guard.remove(&world_id);
        }
        Ok(())
    }
}

impl MembershipSyncClient {
    pub fn publish_revocation_checkpoint(
        &self,
        world_id: &str,
        node_id: &str,
        announced_at_ms: i64,
        keyring: &MembershipDirectorySignerKeyring,
    ) -> Result<MembershipRevocationCheckpointAnnounce, WorldError> {
        let checkpoint = MembershipRevocationCheckpointAnnounce::from_revoked_keys(
            world_id,
            node_id,
            announced_at_ms,
            keyring.revoked_keys(),
        )?;
        let payload = to_canonical_cbor(&checkpoint)?;
        self.network
            .publish(&topic_membership_reconcile(world_id), &payload)?;
        Ok(checkpoint)
    }

    pub fn drain_revocation_checkpoints(
        &self,
        subscription: &MembershipSyncSubscription,
    ) -> Result<Vec<MembershipRevocationCheckpointAnnounce>, WorldError> {
        let raw = subscription.reconcile_sub.drain();
        let mut checkpoints = Vec::with_capacity(raw.len());
        for bytes in raw {
            checkpoints.push(serde_cbor::from_slice(&bytes)?);
        }
        Ok(checkpoints)
    }

    pub fn reconcile_revocations_with_policy(
        &self,
        world_id: &str,
        subscription: &MembershipSyncSubscription,
        keyring: &mut MembershipDirectorySignerKeyring,
        policy: &MembershipRevocationReconcilePolicy,
    ) -> Result<MembershipRevocationReconcileReport, WorldError> {
        let checkpoints = self.drain_revocation_checkpoints(subscription)?;
        let mut report = MembershipRevocationReconcileReport {
            drained: checkpoints.len(),
            in_sync: 0,
            diverged: 0,
            merged: 0,
            rejected: 0,
        };

        for checkpoint in checkpoints {
            let remote = match validate_revocation_checkpoint(world_id, &checkpoint, policy) {
                Ok(remote) => remote,
                Err(_) => {
                    report.rejected =
                        checked_usize_increment(report.rejected, "reconcile report rejected")?;
                    continue;
                }
            };
            let local: BTreeSet<String> = keyring.revoked_keys().into_iter().collect();

            if local == remote {
                report.in_sync =
                    checked_usize_increment(report.in_sync, "reconcile report in_sync")?;
                continue;
            }

            report.diverged =
                checked_usize_increment(report.diverged, "reconcile report diverged")?;
            if policy.auto_revoke_missing_keys {
                for key_id in remote.difference(&local) {
                    if keyring.revoke_key(key_id)? {
                        report.merged =
                            checked_usize_increment(report.merged, "reconcile report merged")?;
                    }
                }
            }
        }

        Ok(report)
    }

    pub fn evaluate_revocation_reconcile_alerts(
        &self,
        world_id: &str,
        node_id: &str,
        detected_at_ms: i64,
        report: &MembershipRevocationReconcileReport,
        policy: &MembershipRevocationAlertPolicy,
    ) -> Result<Vec<MembershipRevocationAnomalyAlert>, WorldError> {
        let world_id = membership_logic::normalized_world_id(world_id)?;
        let node_id = normalized_node_id(node_id)?;
        let mut alerts = Vec::new();

        if policy.critical_rejected_threshold > 0
            && report.rejected >= policy.critical_rejected_threshold
        {
            alerts.push(MembershipRevocationAnomalyAlert {
                world_id: world_id.clone(),
                node_id: node_id.clone(),
                detected_at_ms,
                severity: MembershipRevocationAlertSeverity::Critical,
                code: "reconcile_rejected".to_string(),
                message: format!(
                    "membership revocation reconcile rejected {} checkpoint(s)",
                    report.rejected
                ),
                drained: report.drained,
                diverged: report.diverged,
                rejected: report.rejected,
            });
        }

        if policy.warn_diverged_threshold > 0 && report.diverged >= policy.warn_diverged_threshold {
            alerts.push(MembershipRevocationAnomalyAlert {
                world_id,
                node_id,
                detected_at_ms,
                severity: MembershipRevocationAlertSeverity::Warn,
                code: "reconcile_diverged".to_string(),
                message: format!(
                    "membership revocation reconcile diverged on {} checkpoint(s)",
                    report.diverged
                ),
                drained: report.drained,
                diverged: report.diverged,
                rejected: report.rejected,
            });
        }

        Ok(alerts)
    }

    pub fn deduplicate_revocation_alerts(
        &self,
        alerts: Vec<MembershipRevocationAnomalyAlert>,
        now_ms: i64,
        policy: &MembershipRevocationAlertDedupPolicy,
        state: &mut MembershipRevocationAlertDedupState,
    ) -> Result<Vec<MembershipRevocationAnomalyAlert>, WorldError> {
        if policy.suppress_window_ms < 0 {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "membership revocation suppress_window_ms must be non-negative, got {}",
                    policy.suppress_window_ms
                ),
            });
        }
        if policy.suppress_window_ms == 0 {
            return Ok(alerts);
        }

        let mut filtered = Vec::new();
        for alert in alerts {
            let key = alert_dedup_key(&alert);
            let suppressed = match state.last_emitted_at_by_key.get(&key) {
                Some(last) => {
                    checked_elapsed_ms(
                        now_ms,
                        *last,
                        "membership revocation dedup suppress window",
                    )? < policy.suppress_window_ms
                }
                None => false,
            };
            if suppressed {
                continue;
            }

            state.last_emitted_at_by_key.insert(key, now_ms);
            filtered.push(alert);
        }

        Ok(filtered)
    }

    pub fn emit_revocation_reconcile_alerts(
        &self,
        sink: &(dyn MembershipRevocationAlertSink + Send + Sync),
        alerts: &[MembershipRevocationAnomalyAlert],
    ) -> Result<usize, WorldError> {
        for alert in alerts {
            sink.emit(alert)?;
        }
        Ok(alerts.len())
    }

    pub fn run_revocation_reconcile_schedule(
        &self,
        world_id: &str,
        node_id: &str,
        now_ms: i64,
        subscription: &MembershipSyncSubscription,
        keyring: &mut MembershipDirectorySignerKeyring,
        reconcile_policy: &MembershipRevocationReconcilePolicy,
        schedule_policy: &MembershipRevocationReconcileSchedulePolicy,
        schedule_state: &mut MembershipRevocationReconcileScheduleState,
    ) -> Result<MembershipRevocationScheduledRunReport, WorldError> {
        validate_schedule_policy(schedule_policy)?;

        let mut report = MembershipRevocationScheduledRunReport {
            checkpoint_published: false,
            reconcile_executed: false,
            reconcile_report: None,
        };

        if schedule_due(
            schedule_state.last_checkpoint_at_ms,
            now_ms,
            schedule_policy.checkpoint_interval_ms,
        )? {
            self.publish_revocation_checkpoint(world_id, node_id, now_ms, keyring)?;
            schedule_state.last_checkpoint_at_ms = Some(now_ms);
            report.checkpoint_published = true;
        }

        if schedule_due(
            schedule_state.last_reconcile_at_ms,
            now_ms,
            schedule_policy.reconcile_interval_ms,
        )? {
            let reconcile_report = self.reconcile_revocations_with_policy(
                world_id,
                subscription,
                keyring,
                reconcile_policy,
            )?;
            schedule_state.last_reconcile_at_ms = Some(now_ms);
            report.reconcile_executed = true;
            report.reconcile_report = Some(reconcile_report);
        }

        Ok(report)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_revocation_reconcile_schedule_with_store_and_alerts(
        &self,
        world_id: &str,
        node_id: &str,
        now_ms: i64,
        subscription: &MembershipSyncSubscription,
        keyring: &mut MembershipDirectorySignerKeyring,
        reconcile_policy: &MembershipRevocationReconcilePolicy,
        schedule_policy: &MembershipRevocationReconcileSchedulePolicy,
        alert_policy: &MembershipRevocationAlertPolicy,
        schedule_store: &(dyn MembershipRevocationScheduleStateStore + Send + Sync),
        alert_sink: &(dyn MembershipRevocationAlertSink + Send + Sync),
    ) -> Result<MembershipRevocationScheduledRunReport, WorldError> {
        let mut schedule_state = schedule_store.load(world_id, node_id)?;
        let report = self.run_revocation_reconcile_schedule(
            world_id,
            node_id,
            now_ms,
            subscription,
            keyring,
            reconcile_policy,
            schedule_policy,
            &mut schedule_state,
        )?;
        schedule_store.save(world_id, node_id, &schedule_state)?;

        if let Some(reconcile_report) = report.reconcile_report.as_ref() {
            let alerts = self.evaluate_revocation_reconcile_alerts(
                world_id,
                node_id,
                now_ms,
                reconcile_report,
                alert_policy,
            )?;
            self.emit_revocation_reconcile_alerts(alert_sink, &alerts)?;
        }

        Ok(report)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_revocation_reconcile_coordinated(
        &self,
        world_id: &str,
        node_id: &str,
        now_ms: i64,
        subscription: &MembershipSyncSubscription,
        keyring: &mut MembershipDirectorySignerKeyring,
        reconcile_policy: &MembershipRevocationReconcilePolicy,
        schedule_policy: &MembershipRevocationReconcileSchedulePolicy,
        alert_policy: &MembershipRevocationAlertPolicy,
        dedup_policy: Option<&MembershipRevocationAlertDedupPolicy>,
        mut dedup_state: Option<&mut MembershipRevocationAlertDedupState>,
        schedule_store: &(dyn MembershipRevocationScheduleStateStore + Send + Sync),
        alert_sink: &(dyn MembershipRevocationAlertSink + Send + Sync),
        coordinator: &(dyn MembershipRevocationScheduleCoordinator + Send + Sync),
        coordinator_lease_ttl_ms: i64,
    ) -> Result<MembershipRevocationCoordinatedRunReport, WorldError> {
        if !coordinator.acquire(world_id, node_id, now_ms, coordinator_lease_ttl_ms)? {
            return Ok(MembershipRevocationCoordinatedRunReport {
                acquired: false,
                emitted_alerts: 0,
                run_report: None,
            });
        }

        let run_outcome = (|| {
            let mut schedule_state = schedule_store.load(world_id, node_id)?;
            let run_report = self.run_revocation_reconcile_schedule(
                world_id,
                node_id,
                now_ms,
                subscription,
                keyring,
                reconcile_policy,
                schedule_policy,
                &mut schedule_state,
            )?;
            schedule_store.save(world_id, node_id, &schedule_state)?;

            let mut emitted_alerts = 0;
            if let Some(reconcile_report) = run_report.reconcile_report.as_ref() {
                let alerts = self.evaluate_revocation_reconcile_alerts(
                    world_id,
                    node_id,
                    now_ms,
                    reconcile_report,
                    alert_policy,
                )?;
                let alerts = if let Some(dedup_policy) = dedup_policy {
                    let state = dedup_state.as_deref_mut().ok_or_else(|| {
                        WorldError::DistributedValidationFailed {
                            reason: "membership revocation dedup_state is required when dedup_policy is configured"
                                .to_string(),
                        }
                    })?;
                    self.deduplicate_revocation_alerts(alerts, now_ms, dedup_policy, state)?
                } else {
                    alerts
                };
                emitted_alerts = self.emit_revocation_reconcile_alerts(alert_sink, &alerts)?;
            }

            Ok(MembershipRevocationCoordinatedRunReport {
                acquired: true,
                emitted_alerts,
                run_report: Some(run_report),
            })
        })();

        let release_outcome = coordinator.release(world_id, node_id);
        match (run_outcome, release_outcome) {
            (Ok(report), Ok(())) => Ok(report),
            (Err(err), Ok(())) => Err(err),
            (Ok(_), Err(release_err)) => Err(release_err),
            (Err(err), Err(_)) => Err(err),
        }
    }
}

fn validate_revocation_checkpoint(
    world_id: &str,
    checkpoint: &MembershipRevocationCheckpointAnnounce,
    policy: &MembershipRevocationReconcilePolicy,
) -> Result<BTreeSet<String>, WorldError> {
    if checkpoint.world_id != world_id {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "membership revocation reconcile world mismatch: expected={world_id}, got={}",
                checkpoint.world_id
            ),
        });
    }

    let node_id = normalized_node_id(&checkpoint.node_id)?;
    if !policy.trusted_nodes.is_empty()
        && !policy
            .trusted_nodes
            .iter()
            .any(|trusted| trusted == &node_id)
    {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "membership revocation checkpoint node {} is not trusted",
                node_id
            ),
        });
    }

    let normalized_keys =
        normalize_revoked_key_ids(checkpoint.revoked_key_ids.iter().map(String::as_str))?;
    let expected_hash = revoked_keys_hash(&normalized_keys)?;
    if expected_hash != checkpoint.revoked_set_hash {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "membership revocation checkpoint hash mismatch for node {}",
                node_id
            ),
        });
    }

    Ok(normalized_keys.into_iter().collect())
}

fn normalize_revoked_key_ids<'a>(
    raw: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<String>, WorldError> {
    let mut normalized = BTreeSet::new();
    for key_id in raw {
        normalized.insert(membership_logic::normalized_key_id(key_id)?);
    }
    Ok(normalized.into_iter().collect())
}

fn revoked_keys_hash(revoked_key_ids: &[String]) -> Result<String, WorldError> {
    let bytes = to_canonical_cbor(&revoked_key_ids)?;
    Ok(sha256_hex(&bytes))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn normalized_node_id(raw: &str) -> Result<String, WorldError> {
    let normalized = raw.trim();
    if normalized.is_empty() {
        return Err(WorldError::DistributedValidationFailed {
            reason: "membership revocation checkpoint node_id cannot be empty".to_string(),
        });
    }
    if normalized.contains('/') || normalized.contains('\\') || normalized.contains("..") {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!("membership revocation checkpoint node_id is invalid: {normalized}"),
        });
    }
    Ok(normalized.to_string())
}

fn validate_schedule_policy(
    schedule_policy: &MembershipRevocationReconcileSchedulePolicy,
) -> Result<(), WorldError> {
    if schedule_policy.checkpoint_interval_ms <= 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "membership revocation checkpoint_interval_ms must be positive, got {}",
                schedule_policy.checkpoint_interval_ms
            ),
        });
    }
    if schedule_policy.reconcile_interval_ms <= 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "membership revocation reconcile_interval_ms must be positive, got {}",
                schedule_policy.reconcile_interval_ms
            ),
        });
    }
    Ok(())
}

fn validate_coordinator_lease_ttl_ms(lease_ttl_ms: i64) -> Result<(), WorldError> {
    if lease_ttl_ms <= 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "membership revocation coordinator lease_ttl_ms must be positive, got {}",
                lease_ttl_ms
            ),
        });
    }
    Ok(())
}

fn checked_coordinator_lease_expiry(now_ms: i64, lease_ttl_ms: i64) -> Result<i64, WorldError> {
    now_ms.checked_add(lease_ttl_ms).ok_or_else(|| {
        WorldError::DistributedValidationFailed {
            reason: format!(
                "membership revocation coordinator lease expiry overflow: now_ms={now_ms}, lease_ttl_ms={lease_ttl_ms}"
            ),
        }
    })
}

fn checked_elapsed_ms(now_ms: i64, last_run_ms: i64, context: &str) -> Result<i64, WorldError> {
    now_ms
        .checked_sub(last_run_ms)
        .ok_or_else(|| WorldError::DistributedValidationFailed {
            reason: format!(
                "{context} elapsed overflow: now_ms={now_ms}, last_run_ms={last_run_ms}"
            ),
        })
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

fn schedule_due(
    last_run_ms: Option<i64>,
    now_ms: i64,
    interval_ms: i64,
) -> Result<bool, WorldError> {
    match last_run_ms {
        None => Ok(true),
        Some(last_run_ms) => Ok(checked_elapsed_ms(
            now_ms,
            last_run_ms,
            "membership revocation reconcile schedule due",
        )? >= interval_ms),
    }
}

fn normalized_schedule_key(world_id: &str, node_id: &str) -> Result<(String, String), WorldError> {
    Ok((
        membership_logic::normalized_world_id(world_id)?,
        normalized_node_id(node_id)?,
    ))
}

fn alert_dedup_key(alert: &MembershipRevocationAnomalyAlert) -> String {
    format!("{}:{}:{}", alert.world_id, alert.node_id, alert.code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_usize_add_rejects_overflow() {
        let err = checked_usize_add(usize::MAX, 1, "membership reconcile checked add")
            .expect_err("overflow should fail");
        match err {
            WorldError::DistributedValidationFailed { reason } => {
                assert!(
                    reason.contains("membership reconcile checked add overflow"),
                    "{reason}"
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn validate_revocation_checkpoint_normalizes_borrowed_keys() {
        let checkpoint = MembershipRevocationCheckpointAnnounce::from_revoked_keys(
            "world-1",
            "node-1",
            42,
            vec![
                " key-b ".to_string(),
                "key-a".to_string(),
                "key-b".to_string(),
            ],
        )
        .expect("checkpoint");
        let policy = MembershipRevocationReconcilePolicy::default();

        let validated = validate_revocation_checkpoint("world-1", &checkpoint, &policy)
            .expect("valid checkpoint");

        assert_eq!(
            checkpoint.revoked_key_ids,
            vec!["key-a".to_string(), "key-b".to_string()]
        );
        assert_eq!(
            validated,
            BTreeSet::from(["key-a".to_string(), "key-b".to_string()])
        );
    }
}
