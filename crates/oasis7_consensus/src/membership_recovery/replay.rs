use serde::{Deserialize, Serialize};

use super::super::error::WorldError;

#[path = "replay_state_store.rs"]
mod replay_state_store;
mod sort;

pub use replay_state_store::{
    FileMembershipRevocationDeadLetterReplayPolicyStore,
    FileMembershipRevocationDeadLetterReplayStateStore,
    InMemoryMembershipRevocationDeadLetterReplayPolicyStore,
    InMemoryMembershipRevocationDeadLetterReplayStateStore,
};
use sort::sort_dead_letter_bucket;

use super::{
    normalized_schedule_key, validate_coordinator_lease_ttl_ms,
    MembershipRevocationAlertDeadLetterReason, MembershipRevocationAlertDeadLetterRecord,
    MembershipRevocationAlertDeadLetterStore, MembershipRevocationAlertRecoveryStore,
    MembershipRevocationScheduleCoordinator, MembershipSyncClient,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct MembershipRevocationDeadLetterReplayScheduleState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_replay_at_ms: Option<i64>,
    #[serde(default)]
    pub prefer_capacity_evicted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipRevocationDeadLetterReplayPolicy {
    pub max_replay_per_run: usize,
    pub max_retry_limit_exceeded_streak: usize,
}

impl Default for MembershipRevocationDeadLetterReplayPolicy {
    fn default() -> Self {
        Self {
            max_replay_per_run: 64,
            max_retry_limit_exceeded_streak: 3,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MembershipRevocationDeadLetterReplayPolicyState {
    pub active_policy: MembershipRevocationDeadLetterReplayPolicy,
    pub last_stable_policy: MembershipRevocationDeadLetterReplayPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_policy_update_at_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_stable_at_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_rollback_at_ms: Option<i64>,
}

impl Default for MembershipRevocationDeadLetterReplayPolicyState {
    fn default() -> Self {
        let policy = MembershipRevocationDeadLetterReplayPolicy::default();
        Self {
            active_policy: policy.clone(),
            last_stable_policy: policy,
            last_policy_update_at_ms: None,
            last_stable_at_ms: None,
            last_rollback_at_ms: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipRevocationDeadLetterReplayRollbackGuard {
    pub min_attempted: usize,
    pub failure_ratio_per_mille: usize,
    pub dead_letter_ratio_per_mille: usize,
    pub rollback_cooldown_ms: i64,
}

impl Default for MembershipRevocationDeadLetterReplayRollbackGuard {
    fn default() -> Self {
        Self {
            min_attempted: 8,
            failure_ratio_per_mille: 450,
            dead_letter_ratio_per_mille: 300,
            rollback_cooldown_ms: 30_000,
        }
    }
}

pub trait MembershipRevocationDeadLetterReplayStateStore {
    fn load_state(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<MembershipRevocationDeadLetterReplayScheduleState, WorldError>;

    fn save_state(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationDeadLetterReplayScheduleState,
    ) -> Result<(), WorldError>;
}

pub trait MembershipRevocationDeadLetterReplayPolicyStore {
    fn load_policy_state(
        &self,
        world_id: &str,
        node_id: &str,
    ) -> Result<MembershipRevocationDeadLetterReplayPolicyState, WorldError>;

    fn save_policy_state(
        &self,
        world_id: &str,
        node_id: &str,
        state: &MembershipRevocationDeadLetterReplayPolicyState,
    ) -> Result<(), WorldError>;
}

impl MembershipSyncClient {
    #[allow(clippy::too_many_arguments)]
    pub fn replay_revocation_dead_letters_with_policy(
        &self,
        world_id: &str,
        node_id: &str,
        policy: &MembershipRevocationDeadLetterReplayPolicy,
        state: &mut MembershipRevocationDeadLetterReplayScheduleState,
        recovery_store: &(dyn MembershipRevocationAlertRecoveryStore + Send + Sync),
        dead_letter_store: &(dyn MembershipRevocationAlertDeadLetterStore + Send + Sync),
    ) -> Result<usize, WorldError> {
        validate_dead_letter_replay_policy(policy)?;
        let (world_id, node_id) = normalized_schedule_key(world_id, node_id)?;
        let mut dead_letters = dead_letter_store.list(&world_id, &node_id)?;
        if dead_letters.is_empty() {
            return Ok(0);
        }

        let replay_count = dead_letters.len().min(policy.max_replay_per_run);
        let (replay_indices, next_prefer_capacity_evicted) = fair_dead_letter_indices(
            &dead_letters,
            replay_count,
            policy.max_retry_limit_exceeded_streak,
            state.prefer_capacity_evicted,
        );
        let replaying: Vec<MembershipRevocationAlertDeadLetterRecord> = replay_indices
            .iter()
            .map(|index| dead_letters[*index].clone())
            .collect();
        let mut replay_selected = vec![false; dead_letters.len()];
        for index in replay_indices {
            replay_selected[index] = true;
        }
        let remaining: Vec<MembershipRevocationAlertDeadLetterRecord> = dead_letters
            .drain(..)
            .enumerate()
            .filter_map(|(index, record)| (!replay_selected[index]).then_some(record))
            .collect();

        let mut pending = recovery_store.load_pending(&world_id, &node_id)?;
        for record in replaying {
            pending.push(record.pending_alert);
        }
        recovery_store.save_pending(&world_id, &node_id, &pending)?;
        dead_letter_store.replace(&world_id, &node_id, &remaining)?;
        state.prefer_capacity_evicted = next_prefer_capacity_evicted;
        Ok(replay_count)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_revocation_dead_letter_replay_schedule_with_state_store(
        &self,
        world_id: &str,
        node_id: &str,
        now_ms: i64,
        replay_interval_ms: i64,
        replay_policy: &MembershipRevocationDeadLetterReplayPolicy,
        recovery_store: &(dyn MembershipRevocationAlertRecoveryStore + Send + Sync),
        dead_letter_store: &(dyn MembershipRevocationAlertDeadLetterStore + Send + Sync),
        replay_state_store: &(dyn MembershipRevocationDeadLetterReplayStateStore + Send + Sync),
    ) -> Result<usize, WorldError> {
        validate_replay_interval_ms(replay_interval_ms)?;
        validate_dead_letter_replay_policy(replay_policy)?;

        let mut state = replay_state_store.load_state(world_id, node_id)?;
        let should_run = match state.last_replay_at_ms {
            Some(last_replay_at_ms) => {
                let elapsed_since_last_replay = now_ms.checked_sub(last_replay_at_ms).ok_or_else(|| {
                    WorldError::DistributedValidationFailed {
                        reason: format!(
                            "membership revocation dead-letter replay schedule elapsed overflow: now_ms={now_ms}, last_replay_at_ms={last_replay_at_ms}"
                        ),
                    }
                })?;
                elapsed_since_last_replay >= replay_interval_ms
            }
            None => true,
        };
        if !should_run {
            return Ok(0);
        }

        let replayed = self.replay_revocation_dead_letters_with_policy(
            world_id,
            node_id,
            replay_policy,
            &mut state,
            recovery_store,
            dead_letter_store,
        )?;
        state.last_replay_at_ms = Some(now_ms);
        replay_state_store.save_state(world_id, node_id, &state)?;
        Ok(replayed)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_revocation_dead_letter_replay_schedule_coordinated_with_state_store(
        &self,
        world_id: &str,
        target_node_id: &str,
        coordinator_node_id: &str,
        now_ms: i64,
        replay_interval_ms: i64,
        replay_policy: &MembershipRevocationDeadLetterReplayPolicy,
        recovery_store: &(dyn MembershipRevocationAlertRecoveryStore + Send + Sync),
        dead_letter_store: &(dyn MembershipRevocationAlertDeadLetterStore + Send + Sync),
        replay_state_store: &(dyn MembershipRevocationDeadLetterReplayStateStore + Send + Sync),
        coordinator: &(dyn MembershipRevocationScheduleCoordinator + Send + Sync),
        coordinator_lease_ttl_ms: i64,
    ) -> Result<usize, WorldError> {
        validate_coordinator_lease_ttl_ms(coordinator_lease_ttl_ms)?;
        let coordination_world_id =
            super::normalized_dead_letter_replay_coordination_world_id(world_id, target_node_id)?;
        if !coordinator.acquire(
            &coordination_world_id,
            coordinator_node_id,
            now_ms,
            coordinator_lease_ttl_ms,
        )? {
            return Ok(0);
        }

        let replay_outcome = self.run_revocation_dead_letter_replay_schedule_with_state_store(
            world_id,
            target_node_id,
            now_ms,
            replay_interval_ms,
            replay_policy,
            recovery_store,
            dead_letter_store,
            replay_state_store,
        );
        let release_outcome = coordinator.release(&coordination_world_id, coordinator_node_id);
        match (replay_outcome, release_outcome) {
            (Ok(replayed), Ok(())) => Ok(replayed),
            (Err(err), Ok(())) => Err(err),
            (Ok(_), Err(release_err)) => Err(release_err),
            (Err(err), Err(_)) => Err(err),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn recommend_revocation_dead_letter_replay_policy(
        &self,
        world_id: &str,
        node_id: &str,
        current_policy: &MembershipRevocationDeadLetterReplayPolicy,
        replay_state_store: &(dyn MembershipRevocationDeadLetterReplayStateStore + Send + Sync),
        recovery_store: &(dyn MembershipRevocationAlertRecoveryStore + Send + Sync),
        dead_letter_store: &(dyn MembershipRevocationAlertDeadLetterStore + Send + Sync),
        metrics_lookback: usize,
        min_replay_per_run: usize,
        max_replay_per_run: usize,
        max_retry_limit_exceeded_streak: usize,
    ) -> Result<MembershipRevocationDeadLetterReplayPolicy, WorldError> {
        validate_dead_letter_replay_policy(current_policy)?;
        validate_adaptive_policy_bounds(
            metrics_lookback,
            min_replay_per_run,
            max_replay_per_run,
            max_retry_limit_exceeded_streak,
        )?;
        let (world_id, node_id) = normalized_schedule_key(world_id, node_id)?;

        let state = replay_state_store.load_state(&world_id, &node_id)?;
        let dead_letters = dead_letter_store.list(&world_id, &node_id)?;
        let pending = recovery_store.load_pending(&world_id, &node_id)?;
        let metric_lines = dead_letter_store.list_delivery_metrics(&world_id, &node_id)?;
        let metrics = aggregate_recent_delivery_metrics(&metric_lines, metrics_lookback);

        let mut recommendation = current_policy.clone();
        let backlog_total = dead_letters.len().saturating_add(pending.len());
        let retry_backlog = dead_letters
            .iter()
            .filter(|record| {
                record.reason == MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded
            })
            .count();
        let capacity_backlog = dead_letters
            .iter()
            .filter(|record| {
                record.reason == MembershipRevocationAlertDeadLetterReason::CapacityEvicted
            })
            .count();
        let dead_letter_ratio_per_mille = ratio_per_mille(metrics.dead_lettered, metrics.attempted);
        let failure_ratio_per_mille = ratio_per_mille(metrics.failed, metrics.attempted);

        let high_backlog = backlog_total > min_replay_per_run
            && exceeds_double(backlog_total, current_policy.max_replay_per_run);
        if high_backlog || retry_backlog > current_policy.max_replay_per_run {
            let step = current_policy.max_replay_per_run.max(2) / 2;
            recommendation.max_replay_per_run = recommendation
                .max_replay_per_run
                .saturating_add(step.max(1))
                .min(max_replay_per_run);
        }

        let low_backlog =
            backlog_total <= current_policy.max_replay_per_run.saturating_div(2).max(1);
        if low_backlog
            && pending.is_empty()
            && metrics.attempted >= 4
            && metrics.failed == 0
            && metrics.dead_lettered == 0
        {
            recommendation.max_replay_per_run = recommendation
                .max_replay_per_run
                .saturating_sub(1)
                .max(min_replay_per_run);
        }

        if capacity_backlog > 0 {
            if state.prefer_capacity_evicted || dead_letter_ratio_per_mille >= 250 {
                recommendation.max_retry_limit_exceeded_streak = recommendation
                    .max_retry_limit_exceeded_streak
                    .saturating_sub(1)
                    .max(1);
            } else if exceeds_double(retry_backlog, capacity_backlog)
                && failure_ratio_per_mille <= 350
            {
                recommendation.max_retry_limit_exceeded_streak = recommendation
                    .max_retry_limit_exceeded_streak
                    .saturating_add(1);
            }
        } else if retry_backlog > 0 && failure_ratio_per_mille <= 100 {
            recommendation.max_retry_limit_exceeded_streak = recommendation
                .max_retry_limit_exceeded_streak
                .saturating_add(1);
        }

        recommendation.max_replay_per_run = recommendation
            .max_replay_per_run
            .clamp(min_replay_per_run, max_replay_per_run);
        recommendation.max_retry_limit_exceeded_streak = recommendation
            .max_retry_limit_exceeded_streak
            .clamp(1, max_retry_limit_exceeded_streak);
        Ok(recommendation)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_revocation_dead_letter_replay_schedule_coordinated_with_state_store_and_adaptive_policy(
        &self,
        world_id: &str,
        target_node_id: &str,
        coordinator_node_id: &str,
        now_ms: i64,
        replay_interval_ms: i64,
        current_policy: &MembershipRevocationDeadLetterReplayPolicy,
        replay_state_store: &(dyn MembershipRevocationDeadLetterReplayStateStore + Send + Sync),
        recovery_store: &(dyn MembershipRevocationAlertRecoveryStore + Send + Sync),
        dead_letter_store: &(dyn MembershipRevocationAlertDeadLetterStore + Send + Sync),
        coordinator: &(dyn MembershipRevocationScheduleCoordinator + Send + Sync),
        coordinator_lease_ttl_ms: i64,
        metrics_lookback: usize,
        min_replay_per_run: usize,
        max_replay_per_run: usize,
        max_retry_limit_exceeded_streak: usize,
    ) -> Result<(usize, MembershipRevocationDeadLetterReplayPolicy), WorldError> {
        let recommended = self.recommend_revocation_dead_letter_replay_policy(
            world_id,
            target_node_id,
            current_policy,
            replay_state_store,
            recovery_store,
            dead_letter_store,
            metrics_lookback,
            min_replay_per_run,
            max_replay_per_run,
            max_retry_limit_exceeded_streak,
        )?;
        let replayed = self
            .run_revocation_dead_letter_replay_schedule_coordinated_with_state_store(
                world_id,
                target_node_id,
                coordinator_node_id,
                now_ms,
                replay_interval_ms,
                &recommended,
                recovery_store,
                dead_letter_store,
                replay_state_store,
                coordinator,
                coordinator_lease_ttl_ms,
            )?;
        Ok((replayed, recommended))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn recommend_revocation_dead_letter_replay_policy_with_adaptive_guard(
        &self,
        world_id: &str,
        node_id: &str,
        now_ms: i64,
        current_policy: &MembershipRevocationDeadLetterReplayPolicy,
        replay_state_store: &(dyn MembershipRevocationDeadLetterReplayStateStore + Send + Sync),
        recovery_store: &(dyn MembershipRevocationAlertRecoveryStore + Send + Sync),
        dead_letter_store: &(dyn MembershipRevocationAlertDeadLetterStore + Send + Sync),
        metrics_lookback: usize,
        min_replay_per_run: usize,
        max_replay_per_run: usize,
        max_retry_limit_exceeded_streak: usize,
        policy_cooldown_ms: i64,
        max_replay_step_change: usize,
        max_retry_streak_step_change: usize,
    ) -> Result<MembershipRevocationDeadLetterReplayPolicy, WorldError> {
        validate_adaptive_policy_guard_bounds(
            policy_cooldown_ms,
            max_replay_step_change,
            max_retry_streak_step_change,
        )?;
        let recommended = self.recommend_revocation_dead_letter_replay_policy(
            world_id,
            node_id,
            current_policy,
            replay_state_store,
            recovery_store,
            dead_letter_store,
            metrics_lookback,
            min_replay_per_run,
            max_replay_per_run,
            max_retry_limit_exceeded_streak,
        )?;
        let state = replay_state_store.load_state(world_id, node_id)?;
        let within_cooldown = match state.last_replay_at_ms {
            Some(last_replay_at_ms) => {
                let elapsed_since_last_replay = now_ms.checked_sub(last_replay_at_ms).ok_or_else(|| {
                    WorldError::DistributedValidationFailed {
                        reason: format!(
                            "membership revocation dead-letter policy cooldown elapsed overflow: now_ms={now_ms}, last_replay_at_ms={last_replay_at_ms}"
                        ),
                    }
                })?;
                elapsed_since_last_replay < policy_cooldown_ms
            }
            None => false,
        };
        if within_cooldown {
            return Ok(current_policy.clone());
        }

        Ok(clamp_policy_change_with_step_limit(
            current_policy,
            &recommended,
            max_replay_step_change,
            max_retry_streak_step_change,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_revocation_dead_letter_replay_schedule_coordinated_with_state_store_and_guarded_adaptive_policy(
        &self,
        world_id: &str,
        target_node_id: &str,
        coordinator_node_id: &str,
        now_ms: i64,
        replay_interval_ms: i64,
        current_policy: &MembershipRevocationDeadLetterReplayPolicy,
        replay_state_store: &(dyn MembershipRevocationDeadLetterReplayStateStore + Send + Sync),
        recovery_store: &(dyn MembershipRevocationAlertRecoveryStore + Send + Sync),
        dead_letter_store: &(dyn MembershipRevocationAlertDeadLetterStore + Send + Sync),
        coordinator: &(dyn MembershipRevocationScheduleCoordinator + Send + Sync),
        coordinator_lease_ttl_ms: i64,
        metrics_lookback: usize,
        min_replay_per_run: usize,
        max_replay_per_run: usize,
        max_retry_limit_exceeded_streak: usize,
        policy_cooldown_ms: i64,
        max_replay_step_change: usize,
        max_retry_streak_step_change: usize,
    ) -> Result<(usize, MembershipRevocationDeadLetterReplayPolicy), WorldError> {
        let recommended = self.recommend_revocation_dead_letter_replay_policy_with_adaptive_guard(
            world_id,
            target_node_id,
            now_ms,
            current_policy,
            replay_state_store,
            recovery_store,
            dead_letter_store,
            metrics_lookback,
            min_replay_per_run,
            max_replay_per_run,
            max_retry_limit_exceeded_streak,
            policy_cooldown_ms,
            max_replay_step_change,
            max_retry_streak_step_change,
        )?;
        let replayed = self
            .run_revocation_dead_letter_replay_schedule_coordinated_with_state_store(
                world_id,
                target_node_id,
                coordinator_node_id,
                now_ms,
                replay_interval_ms,
                &recommended,
                recovery_store,
                dead_letter_store,
                replay_state_store,
                coordinator,
                coordinator_lease_ttl_ms,
            )?;
        Ok((replayed, recommended))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn recommend_revocation_dead_letter_replay_policy_with_persistence_and_rollback_guard(
        &self,
        world_id: &str,
        node_id: &str,
        now_ms: i64,
        fallback_policy: &MembershipRevocationDeadLetterReplayPolicy,
        replay_state_store: &(dyn MembershipRevocationDeadLetterReplayStateStore + Send + Sync),
        replay_policy_store: &(dyn MembershipRevocationDeadLetterReplayPolicyStore + Send + Sync),
        recovery_store: &(dyn MembershipRevocationAlertRecoveryStore + Send + Sync),
        dead_letter_store: &(dyn MembershipRevocationAlertDeadLetterStore + Send + Sync),
        metrics_lookback: usize,
        min_replay_per_run: usize,
        max_replay_per_run: usize,
        max_retry_limit_exceeded_streak: usize,
        policy_cooldown_ms: i64,
        max_replay_step_change: usize,
        max_retry_streak_step_change: usize,
        rollback_guard: &MembershipRevocationDeadLetterReplayRollbackGuard,
    ) -> Result<(MembershipRevocationDeadLetterReplayPolicy, bool), WorldError> {
        validate_dead_letter_replay_policy(fallback_policy)?;
        validate_dead_letter_replay_rollback_guard(rollback_guard)?;
        let (world_id, node_id) = normalized_schedule_key(world_id, node_id)?;

        let mut policy_state = replay_policy_store.load_policy_state(&world_id, &node_id)?;
        if policy_state.last_policy_update_at_ms.is_none()
            && policy_state.last_stable_at_ms.is_none()
            && policy_state.last_rollback_at_ms.is_none()
        {
            policy_state.active_policy = fallback_policy.clone();
            policy_state.last_stable_policy = fallback_policy.clone();
        }

        let recommended = self.recommend_revocation_dead_letter_replay_policy_with_adaptive_guard(
            &world_id,
            &node_id,
            now_ms,
            &policy_state.active_policy,
            replay_state_store,
            recovery_store,
            dead_letter_store,
            metrics_lookback,
            min_replay_per_run,
            max_replay_per_run,
            max_retry_limit_exceeded_streak,
            policy_cooldown_ms,
            max_replay_step_change,
            max_retry_streak_step_change,
        )?;

        let dead_letters = dead_letter_store.list(&world_id, &node_id)?;
        let pending = recovery_store.load_pending(&world_id, &node_id)?;
        let metric_lines = dead_letter_store.list_delivery_metrics(&world_id, &node_id)?;
        let metrics = aggregate_recent_delivery_metrics(&metric_lines, metrics_lookback);

        let rolled_back =
            should_rollback_to_stable_policy(&policy_state, now_ms, &metrics, rollback_guard)?;

        let applied_policy = if rolled_back {
            let stable = policy_state.last_stable_policy.clone();
            policy_state.active_policy = stable.clone();
            policy_state.last_policy_update_at_ms = Some(now_ms);
            policy_state.last_rollback_at_ms = Some(now_ms);
            stable
        } else {
            policy_state.active_policy = recommended.clone();
            policy_state.last_policy_update_at_ms = Some(now_ms);
            if is_replay_policy_stable(&recommended, dead_letters.len(), pending.len(), &metrics) {
                policy_state.last_stable_policy = recommended.clone();
                policy_state.last_stable_at_ms = Some(now_ms);
            }
            recommended
        };
        replay_policy_store.save_policy_state(&world_id, &node_id, &policy_state)?;
        Ok((applied_policy, rolled_back))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_revocation_dead_letter_replay_schedule_coordinated_with_state_store_and_persisted_guarded_policy(
        &self,
        world_id: &str,
        target_node_id: &str,
        coordinator_node_id: &str,
        now_ms: i64,
        replay_interval_ms: i64,
        fallback_policy: &MembershipRevocationDeadLetterReplayPolicy,
        replay_state_store: &(dyn MembershipRevocationDeadLetterReplayStateStore + Send + Sync),
        replay_policy_store: &(dyn MembershipRevocationDeadLetterReplayPolicyStore + Send + Sync),
        recovery_store: &(dyn MembershipRevocationAlertRecoveryStore + Send + Sync),
        dead_letter_store: &(dyn MembershipRevocationAlertDeadLetterStore + Send + Sync),
        coordinator: &(dyn MembershipRevocationScheduleCoordinator + Send + Sync),
        coordinator_lease_ttl_ms: i64,
        metrics_lookback: usize,
        min_replay_per_run: usize,
        max_replay_per_run: usize,
        max_retry_limit_exceeded_streak: usize,
        policy_cooldown_ms: i64,
        max_replay_step_change: usize,
        max_retry_streak_step_change: usize,
        rollback_guard: &MembershipRevocationDeadLetterReplayRollbackGuard,
    ) -> Result<(usize, MembershipRevocationDeadLetterReplayPolicy, bool), WorldError> {
        let (recommended, rolled_back) = self
            .recommend_revocation_dead_letter_replay_policy_with_persistence_and_rollback_guard(
                world_id,
                target_node_id,
                now_ms,
                fallback_policy,
                replay_state_store,
                replay_policy_store,
                recovery_store,
                dead_letter_store,
                metrics_lookback,
                min_replay_per_run,
                max_replay_per_run,
                max_retry_limit_exceeded_streak,
                policy_cooldown_ms,
                max_replay_step_change,
                max_retry_streak_step_change,
                rollback_guard,
            )?;
        let replayed = self
            .run_revocation_dead_letter_replay_schedule_coordinated_with_state_store(
                world_id,
                target_node_id,
                coordinator_node_id,
                now_ms,
                replay_interval_ms,
                &recommended,
                recovery_store,
                dead_letter_store,
                replay_state_store,
                coordinator,
                coordinator_lease_ttl_ms,
            )?;
        Ok((replayed, recommended, rolled_back))
    }
}

fn validate_replay_interval_ms(replay_interval_ms: i64) -> Result<(), WorldError> {
    if replay_interval_ms <= 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "membership revocation dead-letter replay_interval_ms must be positive, got {}",
                replay_interval_ms
            ),
        });
    }
    Ok(())
}

fn validate_dead_letter_replay_policy(
    policy: &MembershipRevocationDeadLetterReplayPolicy,
) -> Result<(), WorldError> {
    if policy.max_replay_per_run == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "membership revocation dead-letter max_replay_per_run must be positive"
                .to_string(),
        });
    }
    if policy.max_retry_limit_exceeded_streak == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason:
                "membership revocation dead-letter max_retry_limit_exceeded_streak must be positive"
                    .to_string(),
        });
    }
    Ok(())
}

fn validate_adaptive_policy_bounds(
    metrics_lookback: usize,
    min_replay_per_run: usize,
    max_replay_per_run: usize,
    max_retry_limit_exceeded_streak: usize,
) -> Result<(), WorldError> {
    if metrics_lookback == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "membership revocation dead-letter metrics_lookback must be positive"
                .to_string(),
        });
    }
    if min_replay_per_run == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "membership revocation dead-letter min_replay_per_run must be positive"
                .to_string(),
        });
    }
    if min_replay_per_run > max_replay_per_run {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "membership revocation dead-letter replay bounds are invalid: min={} > max={}",
                min_replay_per_run, max_replay_per_run
            ),
        });
    }
    if max_retry_limit_exceeded_streak == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason:
                "membership revocation dead-letter max_retry_limit_exceeded_streak must be positive"
                    .to_string(),
        });
    }
    Ok(())
}

fn validate_adaptive_policy_guard_bounds(
    policy_cooldown_ms: i64,
    max_replay_step_change: usize,
    max_retry_streak_step_change: usize,
) -> Result<(), WorldError> {
    if policy_cooldown_ms <= 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "membership revocation dead-letter policy_cooldown_ms must be positive, got {}",
                policy_cooldown_ms
            ),
        });
    }
    if max_replay_step_change == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: "membership revocation dead-letter max_replay_step_change must be positive"
                .to_string(),
        });
    }
    if max_retry_streak_step_change == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason:
                "membership revocation dead-letter max_retry_streak_step_change must be positive"
                    .to_string(),
        });
    }
    Ok(())
}

fn validate_dead_letter_replay_rollback_guard(
    guard: &MembershipRevocationDeadLetterReplayRollbackGuard,
) -> Result<(), WorldError> {
    if guard.min_attempted == 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason:
                "membership revocation dead-letter rollback guard min_attempted must be positive"
                    .to_string(),
        });
    }
    if guard.failure_ratio_per_mille > 1_000 {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "membership revocation dead-letter rollback guard failure_ratio_per_mille must be <= 1000, got {}",
                guard.failure_ratio_per_mille
            ),
        });
    }
    if guard.dead_letter_ratio_per_mille > 1_000 {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "membership revocation dead-letter rollback guard dead_letter_ratio_per_mille must be <= 1000, got {}",
                guard.dead_letter_ratio_per_mille
            ),
        });
    }
    if guard.rollback_cooldown_ms <= 0 {
        return Err(WorldError::DistributedValidationFailed {
            reason: format!(
                "membership revocation dead-letter rollback guard rollback_cooldown_ms must be positive, got {}",
                guard.rollback_cooldown_ms
            ),
        });
    }
    Ok(())
}

fn should_rollback_to_stable_policy(
    state: &MembershipRevocationDeadLetterReplayPolicyState,
    now_ms: i64,
    metrics: &super::MembershipRevocationAlertDeliveryMetrics,
    guard: &MembershipRevocationDeadLetterReplayRollbackGuard,
) -> Result<bool, WorldError> {
    if state.active_policy == state.last_stable_policy {
        return Ok(false);
    }
    if metrics.attempted < guard.min_attempted {
        return Ok(false);
    }
    let failure_ratio_per_mille = ratio_per_mille(metrics.failed, metrics.attempted);
    let dead_letter_ratio_per_mille = ratio_per_mille(metrics.dead_lettered, metrics.attempted);
    let unhealthy = failure_ratio_per_mille >= guard.failure_ratio_per_mille
        || dead_letter_ratio_per_mille >= guard.dead_letter_ratio_per_mille;
    if !unhealthy {
        return Ok(false);
    }
    match state.last_rollback_at_ms {
        Some(last_rollback_at_ms) => {
            let elapsed_since_last_rollback =
                now_ms
                    .checked_sub(last_rollback_at_ms)
                    .ok_or_else(|| WorldError::DistributedValidationFailed {
                        reason: format!(
                            "membership revocation dead-letter rollback cooldown elapsed overflow: now_ms={now_ms}, last_rollback_at_ms={last_rollback_at_ms}"
                        ),
                    })?;
            Ok(elapsed_since_last_rollback >= guard.rollback_cooldown_ms)
        }
        None => Ok(true),
    }
}

fn is_replay_policy_stable(
    policy: &MembershipRevocationDeadLetterReplayPolicy,
    dead_letter_len: usize,
    pending_len: usize,
    metrics: &super::MembershipRevocationAlertDeliveryMetrics,
) -> bool {
    let backlog_total = dead_letter_len.saturating_add(pending_len);
    backlog_total <= policy.max_replay_per_run
        && metrics.attempted >= 4
        && metrics.failed == 0
        && metrics.dead_lettered == 0
}

fn clamp_policy_change_with_step_limit(
    current_policy: &MembershipRevocationDeadLetterReplayPolicy,
    recommended_policy: &MembershipRevocationDeadLetterReplayPolicy,
    max_replay_step_change: usize,
    max_retry_streak_step_change: usize,
) -> MembershipRevocationDeadLetterReplayPolicy {
    MembershipRevocationDeadLetterReplayPolicy {
        max_replay_per_run: clamp_usize_delta(
            current_policy.max_replay_per_run,
            recommended_policy.max_replay_per_run,
            max_replay_step_change,
        ),
        max_retry_limit_exceeded_streak: clamp_usize_delta(
            current_policy.max_retry_limit_exceeded_streak,
            recommended_policy.max_retry_limit_exceeded_streak,
            max_retry_streak_step_change,
        )
        .max(1),
    }
}

fn clamp_usize_delta(current: usize, target: usize, max_step_change: usize) -> usize {
    if current == target {
        return target;
    }
    if current < target {
        current.saturating_add(max_step_change).min(target)
    } else {
        current.saturating_sub(max_step_change).max(target)
    }
}

pub(super) fn aggregate_recent_delivery_metrics(
    metric_lines: &[(i64, super::MembershipRevocationAlertDeliveryMetrics)],
    metrics_lookback: usize,
) -> super::MembershipRevocationAlertDeliveryMetrics {
    let start = metric_lines.len().saturating_sub(metrics_lookback);
    metric_lines[start..].iter().fold(
        super::MembershipRevocationAlertDeliveryMetrics::default(),
        |mut total, (_, metrics)| {
            total.attempted = total.attempted.saturating_add(metrics.attempted);
            total.succeeded = total.succeeded.saturating_add(metrics.succeeded);
            total.failed = total.failed.saturating_add(metrics.failed);
            total.deferred = total.deferred.saturating_add(metrics.deferred);
            total.buffered = total.buffered.saturating_add(metrics.buffered);
            total.dropped_capacity = total
                .dropped_capacity
                .saturating_add(metrics.dropped_capacity);
            total.dropped_retry_limit = total
                .dropped_retry_limit
                .saturating_add(metrics.dropped_retry_limit);
            total.dead_lettered = total.dead_lettered.saturating_add(metrics.dead_lettered);
            total
        },
    )
}

fn ratio_per_mille(numerator: usize, denominator: usize) -> usize {
    if denominator == 0 {
        return 0;
    }
    let scaled = (numerator as u128).saturating_mul(1000) / (denominator as u128);
    usize::try_from(scaled).unwrap_or(usize::MAX)
}

fn exceeds_double(lhs: usize, rhs: usize) -> bool {
    (lhs as u128) > (rhs as u128).saturating_mul(2)
}

fn fair_dead_letter_indices(
    dead_letters: &[MembershipRevocationAlertDeadLetterRecord],
    replay_count: usize,
    max_retry_limit_exceeded_streak: usize,
    prefer_capacity_evicted: bool,
) -> (Vec<usize>, bool) {
    let mut retry_limit_exceeded = Vec::new();
    let mut capacity_evicted = Vec::new();
    for (index, record) in dead_letters.iter().enumerate() {
        match record.reason {
            MembershipRevocationAlertDeadLetterReason::RetryLimitExceeded => {
                retry_limit_exceeded.push(index);
            }
            MembershipRevocationAlertDeadLetterReason::CapacityEvicted => {
                capacity_evicted.push(index);
            }
        }
    }
    sort_dead_letter_bucket(dead_letters, &mut retry_limit_exceeded);
    sort_dead_letter_bucket(dead_letters, &mut capacity_evicted);

    let mut selected = Vec::with_capacity(replay_count);
    let mut retry_cursor = 0usize;
    let mut capacity_cursor = 0usize;
    let mut retry_streak = 0usize;
    let mut prefer_capacity_next = prefer_capacity_evicted;

    while selected.len() < replay_count {
        let retry_available = retry_cursor < retry_limit_exceeded.len();
        let capacity_available = capacity_cursor < capacity_evicted.len();
        if !retry_available && !capacity_available {
            break;
        }

        let take_capacity = if prefer_capacity_next && capacity_available {
            true
        } else if retry_available && capacity_available {
            retry_streak >= max_retry_limit_exceeded_streak
        } else {
            !retry_available && capacity_available
        };

        if take_capacity {
            selected.push(capacity_evicted[capacity_cursor]);
            capacity_cursor = capacity_cursor.saturating_add(1);
            retry_streak = 0;
            prefer_capacity_next = false;
            continue;
        }

        if retry_available {
            selected.push(retry_limit_exceeded[retry_cursor]);
            retry_cursor = retry_cursor.saturating_add(1);
            retry_streak = retry_streak.saturating_add(1);
            if capacity_cursor < capacity_evicted.len()
                && retry_streak >= max_retry_limit_exceeded_streak
            {
                prefer_capacity_next = true;
            }
            continue;
        }

        selected.push(capacity_evicted[capacity_cursor]);
        capacity_cursor = capacity_cursor.saturating_add(1);
        retry_streak = 0;
        prefer_capacity_next = false;
    }

    let capacity_selected = selected
        .iter()
        .filter(|index| {
            dead_letters[**index].reason
                == MembershipRevocationAlertDeadLetterReason::CapacityEvicted
        })
        .count();
    let capacity_remaining = capacity_cursor < capacity_evicted.len();
    let next_prefer_capacity_evicted = capacity_remaining && capacity_selected == 0;

    (selected, next_prefer_capacity_evicted)
}

#[cfg(test)]
mod tests;
