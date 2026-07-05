use std::collections::BTreeSet;
use std::time::Duration;

use super::ExecutionBridgeRecord;

const EXECUTION_BRIDGE_STAGE_WARN_THRESHOLD: Duration = Duration::from_millis(500);
const EXECUTION_BRIDGE_TOTAL_WARN_THRESHOLD: Duration = Duration::from_millis(1_000);

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct SimulatorMirrorCommitObservation {
    pub(super) action_count: usize,
    pub(super) rejected_action_count: usize,
    pub(super) snapshot_bytes: usize,
    pub(super) journal_bytes: usize,
}

pub(super) struct CommitObservation<'a> {
    pub(super) world_id: &'a str,
    pub(super) height: u64,
    pub(super) action_count: usize,
    pub(super) runtime_action_count: usize,
    pub(super) simulator: SimulatorMirrorCommitObservation,
    pub(super) snapshot_bytes: usize,
    pub(super) journal_bytes: usize,
    pub(super) decode_ms: Duration,
    pub(super) runtime_step_ms: Duration,
    pub(super) simulator_step_ms: Duration,
    pub(super) serialize_ms: Duration,
    pub(super) cas_put_ms: Duration,
    pub(super) simulator_persist_ms: Duration,
    pub(super) world_head_proof_ms: Duration,
    pub(super) record_persist_ms: Duration,
    pub(super) persist_world_ms: Duration,
    pub(super) checkpoint_ms: Duration,
    pub(super) retention_ms: Duration,
    pub(super) total_ms: Duration,
}

pub(super) struct RestoreObservation<'a> {
    pub(super) world_id: &'a str,
    pub(super) height: u64,
    pub(super) pinned_ref_count: usize,
    pub(super) blob_count: usize,
    pub(super) bundle_bytes: usize,
    pub(super) snapshot_bytes: usize,
    pub(super) journal_bytes: usize,
    pub(super) simulator_mirror_present: bool,
    pub(super) simulator_snapshot_bytes: usize,
    pub(super) simulator_journal_bytes: usize,
    pub(super) blob_store_ms: Duration,
    pub(super) decode_ms: Duration,
    pub(super) rebuild_ms: Duration,
    pub(super) simulator_restore_ms: Duration,
    pub(super) persist_ms: Duration,
    pub(super) total_ms: Duration,
}

pub(super) struct CheckpointInstallObservation<'a> {
    pub(super) world_id: &'a str,
    pub(super) height: u64,
    pub(super) checkpoint_id: &'a str,
    pub(super) blob_count: usize,
    pub(super) bundle_bytes: usize,
    pub(super) pinned_ref_count: usize,
    pub(super) snapshot_bytes: usize,
    pub(super) journal_bytes: usize,
    pub(super) blob_store_ms: Duration,
    pub(super) pin_check_ms: Duration,
    pub(super) decode_ms: Duration,
    pub(super) rebuild_ms: Duration,
    pub(super) persist_ms: Duration,
    pub(super) retention_ms: Duration,
    pub(super) total_ms: Duration,
}

pub(super) fn execution_record_recovery_ref_count(record: &ExecutionBridgeRecord) -> usize {
    let mut refs = BTreeSet::new();
    refs.extend(record.latest_state_ref.iter().cloned());
    refs.extend(record.snapshot_ref.iter().cloned());
    refs.extend(record.journal_ref.iter().cloned());
    if let Some(simulator_mirror) = record.simulator_mirror.as_ref() {
        refs.insert(simulator_mirror.snapshot_ref.clone());
        refs.insert(simulator_mirror.journal_ref.clone());
    }
    refs.len()
}

pub(super) fn emit_stale_height_restore_start(
    world_id: &str,
    height: u64,
    pinned_ref_count: usize,
    simulator_mirror_present: bool,
) {
    emit_execution_bridge_observation(
        tracing::Level::INFO,
        format!(
            "execution bridge stale-height restore start world_id={world_id} height={height} pinned_ref_count={pinned_ref_count} simulator_mirror_present={simulator_mirror_present}"
        ),
        "execution bridge stale-height restore start",
    );
}

pub(super) fn emit_stale_height_restore_complete(observation: RestoreObservation<'_>) {
    let level = level_for_execution_bridge_sample(
        observation.total_ms,
        &[
            observation.blob_store_ms,
            observation.decode_ms,
            observation.rebuild_ms,
            observation.simulator_restore_ms,
            observation.persist_ms,
        ],
    );
    emit_execution_bridge_observation(
        info_or_warn(level),
        format!(
            "execution bridge stale-height restore complete world_id={} height={} pinned_ref_count={} blob_count={} bundle_bytes={} snapshot_bytes={} journal_bytes={} simulator_mirror_present={} simulator_snapshot_bytes={} simulator_journal_bytes={} blob_store_ms={} decode_ms={} rebuild_ms={} simulator_restore_ms={} persist_ms={} total_ms={}",
            observation.world_id,
            observation.height,
            observation.pinned_ref_count,
            observation.blob_count,
            observation.bundle_bytes,
            observation.snapshot_bytes,
            observation.journal_bytes,
            observation.simulator_mirror_present,
            observation.simulator_snapshot_bytes,
            observation.simulator_journal_bytes,
            duration_ms(observation.blob_store_ms),
            duration_ms(observation.decode_ms),
            duration_ms(observation.rebuild_ms),
            duration_ms(observation.simulator_restore_ms),
            duration_ms(observation.persist_ms),
            duration_ms(observation.total_ms)
        ),
        "execution bridge stale-height restore complete",
    );
}

pub(super) fn emit_commit_observation(observation: CommitObservation<'_>) {
    let level = level_for_execution_bridge_sample(
        observation.total_ms,
        &[
            observation.decode_ms,
            observation.runtime_step_ms,
            observation.simulator_step_ms,
            observation.serialize_ms,
            observation.cas_put_ms,
            observation.simulator_persist_ms,
            observation.world_head_proof_ms,
            observation.record_persist_ms,
            observation.persist_world_ms,
            observation.checkpoint_ms,
            observation.retention_ms,
        ],
    );
    let message = format!(
        "execution bridge commit observation world_id={} height={} action_count={} runtime_action_count={} simulator_action_count={} simulator_rejected_action_count={} snapshot_bytes={} journal_bytes={} simulator_snapshot_bytes={} simulator_journal_bytes={} decode_ms={} runtime_step_ms={} simulator_step_ms={} serialize_ms={} cas_put_ms={} simulator_persist_ms={} world_head_proof_ms={} record_persist_ms={} persist_world_ms={} checkpoint_ms={} retention_ms={} total_ms={}",
        observation.world_id,
        observation.height,
        observation.action_count,
        observation.runtime_action_count,
        observation.simulator.action_count,
        observation.simulator.rejected_action_count,
        observation.snapshot_bytes,
        observation.journal_bytes,
        observation.simulator.snapshot_bytes,
        observation.simulator.journal_bytes,
        duration_ms(observation.decode_ms),
        duration_ms(observation.runtime_step_ms),
        duration_ms(observation.simulator_step_ms),
        duration_ms(observation.serialize_ms),
        duration_ms(observation.cas_put_ms),
        duration_ms(observation.simulator_persist_ms),
        duration_ms(observation.world_head_proof_ms),
        duration_ms(observation.record_persist_ms),
        duration_ms(observation.persist_world_ms),
        duration_ms(observation.checkpoint_ms),
        duration_ms(observation.retention_ms),
        duration_ms(observation.total_ms)
    );
    if level == tracing::Level::WARN {
        emit_execution_bridge_observation(
            tracing::Level::WARN,
            message,
            "execution bridge commit observation",
        );
    } else {
        tracing::debug!(message = %message, "execution bridge commit observation");
    }
}

pub(super) fn emit_checkpoint_bundle_install_start(
    world_id: &str,
    height: u64,
    checkpoint_id: &str,
    blob_count: usize,
    bundle_bytes: usize,
    pinned_ref_count: usize,
) {
    emit_execution_bridge_observation(
        tracing::Level::INFO,
        format!(
            "execution checkpoint bundle install start world_id={world_id} height={height} checkpoint_id={checkpoint_id} blob_count={blob_count} bundle_bytes={bundle_bytes} pinned_ref_count={pinned_ref_count} simulator_mirror_present=false"
        ),
        "execution checkpoint bundle install start",
    );
}

pub(super) fn emit_checkpoint_bundle_install_complete(
    observation: CheckpointInstallObservation<'_>,
) {
    let level = level_for_execution_bridge_sample(
        observation.total_ms,
        &[
            observation.blob_store_ms,
            observation.pin_check_ms,
            observation.decode_ms,
            observation.rebuild_ms,
            observation.persist_ms,
            observation.retention_ms,
        ],
    );
    emit_execution_bridge_observation(
        info_or_warn(level),
        format!(
            "execution checkpoint bundle install complete world_id={} height={} checkpoint_id={} blob_count={} bundle_bytes={} pinned_ref_count={} snapshot_bytes={} journal_bytes={} simulator_mirror_present=false blob_store_ms={} pin_check_ms={} decode_ms={} rebuild_ms={} persist_ms={} retention_ms={} total_ms={}",
            observation.world_id,
            observation.height,
            observation.checkpoint_id,
            observation.blob_count,
            observation.bundle_bytes,
            observation.pinned_ref_count,
            observation.snapshot_bytes,
            observation.journal_bytes,
            duration_ms(observation.blob_store_ms),
            duration_ms(observation.pin_check_ms),
            duration_ms(observation.decode_ms),
            duration_ms(observation.rebuild_ms),
            duration_ms(observation.persist_ms),
            duration_ms(observation.retention_ms),
            duration_ms(observation.total_ms)
        ),
        "execution checkpoint bundle install complete",
    );
}

fn duration_ms(duration: Duration) -> u128 {
    duration.as_millis()
}

fn max_duration(durations: &[Duration]) -> Duration {
    durations.iter().copied().max().unwrap_or_default()
}

fn level_for_execution_bridge_sample(total: Duration, stages: &[Duration]) -> tracing::Level {
    if total >= EXECUTION_BRIDGE_TOTAL_WARN_THRESHOLD
        || max_duration(stages) >= EXECUTION_BRIDGE_STAGE_WARN_THRESHOLD
    {
        tracing::Level::WARN
    } else {
        tracing::Level::DEBUG
    }
}

fn info_or_warn(level: tracing::Level) -> tracing::Level {
    if level == tracing::Level::WARN {
        tracing::Level::WARN
    } else {
        tracing::Level::INFO
    }
}

fn emit_execution_bridge_observation(
    level: tracing::Level,
    stderr_message: String,
    event_message: &'static str,
) {
    oasis7::observability::emit_stderr_or_event(level, stderr_message.as_str(), event_message);
}
