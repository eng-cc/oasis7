//! World-owned cognition scheduler, wake, continuation and retention bridge.
//!
//! The focused cognition modules are deliberately provider-free state
//! machines.  This bridge gives `World` one durable owner for them: every
//! mutation is encoded back into the additive cognition projection so the
//! existing JSON/distfs snapshot pipeline restores the exact same lifecycle.

use super::World;
use crate::runtime::cognition_retention::{
    CognitionRetentionStore, RetentionExecutionProbe, RetentionRecordV1,
};
use crate::runtime::cognition_scheduler::{
    CognitionScheduler, SchedulerEnqueueOutcome, SchedulerPolicyV1, SchedulerWakeV1,
};
use crate::runtime::cognition_wake::{
    AgentContinuation, ContinuationReorgReport, ContinuationStatusV1, ContinuationTransition,
    WakeConditionV1, WakeConditionValidator, WakeEvaluation, WakeEvaluationContext,
};
use crate::runtime::error::WorldError;
use serde_json::{Value as JsonValue, json};

const CONTINUATION_SCHEMA: &str = "agent-continuation.v1";

impl World {
    /// Configure the durable World scheduler.  Configuration is encoded in
    /// the cognition projection immediately so a save taken before the first
    /// wake still restores the chosen policy and capacity.
    pub fn with_cognition_scheduler(mut self, policy: SchedulerPolicyV1, capacity: usize) -> Self {
        let scheduler = CognitionScheduler::new(policy, capacity);
        self.cognition_set_scheduler(&scheduler);
        self
    }

    pub fn enqueue_cognition_wake(
        &mut self,
        wake: SchedulerWakeV1,
    ) -> Result<SchedulerEnqueueOutcome, WorldError> {
        let mut scheduler = self.cognition_scheduler()?;
        let outcome = scheduler
            .try_enqueue(wake)
            .map_err(|error| scheduler_error(error.code()))?;
        self.cognition_set_scheduler(&scheduler);
        Ok(outcome)
    }

    pub fn select_ready_cognition_wakes(
        &mut self,
        tick: u64,
    ) -> Result<Vec<SchedulerWakeV1>, WorldError> {
        let mut scheduler = self.cognition_scheduler()?;
        let selected = scheduler.select_ready(tick);
        self.cognition_set_scheduler(&scheduler);
        Ok(selected)
    }

    /// Release the in-flight slot abandoned by the process that was restored,
    /// then recover pending wakes without advancing the committed cursor.
    pub fn recover_cognition_scheduler(
        &mut self,
        tick: u64,
    ) -> Result<Vec<SchedulerWakeV1>, WorldError> {
        let mut scheduler = self.cognition_scheduler()?;
        scheduler.release_capacity();
        let recovered = scheduler.recover_capacity_preserving_cursor(tick);
        self.cognition_set_scheduler(&scheduler);
        Ok(recovered)
    }

    pub fn cognition_scheduler_snapshot(&self) -> JsonValue {
        self.cognition
            .get("scheduler_state")
            .cloned()
            .unwrap_or(JsonValue::Null)
    }

    pub fn cognition_execution_metrics(&self) -> JsonValue {
        self.cognition
            .get("scheduler_state")
            .and_then(|state| state.get("metrics"))
            .cloned()
            .or_else(|| self.cognition.get("metrics").cloned())
            .unwrap_or_else(|| {
                json!({
                    "recovery_wake_count": 0,
                    "provider_invocation_count": 0,
                    "effect_count": 0,
                    "debit_count": 0
                })
            })
    }

    pub fn evaluate_cognition_wake(
        &self,
        conditions: &[WakeConditionV1],
    ) -> Result<WakeEvaluation, WorldError> {
        self.evaluate_cognition_wake_with_context(
            conditions,
            WakeEvaluationContext::at(self.state.time),
        )
    }

    pub fn evaluate_cognition_wake_from_committed_projection(
        &self,
        conditions: &[WakeConditionV1],
        event_digest: &str,
        receipt_id: &str,
    ) -> Result<WakeEvaluation, WorldError> {
        let context = WakeEvaluationContext::at(self.state.time)
            .with_event(event_digest)
            .with_receipt(receipt_id)
            .with_predicate_value("world.logical_tick", &[self.state.time.min(255) as u8]);
        self.evaluate_cognition_wake_with_context(conditions, context)
    }

    pub fn schedule_cognition_continuation(
        &mut self,
        continuation: AgentContinuation,
    ) -> Result<(), WorldError> {
        if continuation.schema_version != CONTINUATION_SCHEMA {
            return Err(cognition_validation_error("continuation_schema_mismatch"));
        }
        WakeConditionValidator::validate(continuation.wake_conditions.as_slice())
            .map_err(|error| cognition_validation_error(error.code()))?;
        let mut continuations = self.cognition_continuations_typed()?;
        if continuations
            .iter()
            .any(|existing| existing.continuation_id == continuation.continuation_id)
        {
            return Err(cognition_validation_error("continuation_id_conflict"));
        }
        continuations.push(continuation);
        self.cognition_set_continuations(&continuations)?;
        Ok(())
    }

    pub fn invalidate_cognition_for_reorg(
        &mut self,
        reorg_epoch: u64,
    ) -> Result<JsonValue, WorldError> {
        let mut continuations = self.cognition_continuations_typed()?;
        let mut invalidated = 0u64;
        for continuation in &mut continuations {
            if matches!(
                continuation.status,
                ContinuationStatusV1::Completed
                    | ContinuationStatusV1::Cancelled
                    | ContinuationStatusV1::Invalidated
                    | ContinuationStatusV1::Expired
                    | ContinuationStatusV1::Rejected
            ) {
                continue;
            }
            ContinuationTransition::invalidate_for_reorg(continuation, reorg_epoch)
                .map_err(|error| cognition_validation_error(error.code()))?;
            invalidated = invalidated.saturating_add(1);
        }
        self.cognition_set_continuations(&continuations)?;
        let report = ContinuationReorgReport {
            terminal_disposition: if invalidated > 0 {
                "reorg_invalidated".to_string()
            } else {
                "already_terminal".to_string()
            },
            provider_invocation_count: 0,
            effect_count: 0,
            receipt_count: 0,
        };
        serde_json::to_value(report).map_err(WorldError::from)
    }

    pub fn cognition_continuations(&self) -> JsonValue {
        self.cognition
            .get("continuations")
            .cloned()
            .unwrap_or_else(|| JsonValue::Array(Vec::new()))
    }

    pub fn record_cognition_terminal(
        &mut self,
        record: RetentionRecordV1,
    ) -> Result<(), WorldError> {
        let mut store = self.cognition_retention_store()?;
        store.insert(record);
        self.cognition_set_retention_store(&store)
    }

    pub fn pin_cognition_reference(&mut self, key: &str, reference: &str) {
        let mut store = self
            .cognition_retention_store()
            .unwrap_or_else(|_| CognitionRetentionStore::with_horizon(0));
        store.pin_reference(key, reference);
        let _ = self.cognition_set_retention_store(&store);
    }

    pub fn gc_cognition(
        &mut self,
        now_tick: u64,
        gc_floor_tick: u64,
    ) -> Result<crate::runtime::RetentionGcReport, WorldError> {
        let mut store = self.cognition_retention_store()?;
        let report = store
            .gc(now_tick, gc_floor_tick)
            .map_err(|error| cognition_validation_error(error.code()))?;
        self.cognition_set_retention_store(&store)?;
        Ok(report)
    }

    pub fn replay_cognition_terminal(
        &self,
        key: &str,
        digest: &str,
    ) -> Result<JsonValue, WorldError> {
        let store = self.cognition_retention_store()?;
        let mut probe = RetentionExecutionProbe::default();
        let result = store
            .replay(key, digest, &mut probe)
            .map_err(|error| cognition_validation_error(error.code()))?;
        serde_json::to_value(result).map_err(WorldError::from)
    }

    fn evaluate_cognition_wake_with_context(
        &self,
        conditions: &[WakeConditionV1],
        context: WakeEvaluationContext,
    ) -> Result<WakeEvaluation, WorldError> {
        WakeConditionValidator::evaluate(conditions, &context)
            .map_err(|error| cognition_validation_error(error.code()))
    }

    fn cognition_scheduler(&self) -> Result<CognitionScheduler, WorldError> {
        let state = self
            .cognition
            .get("scheduler_state")
            .filter(|state| !state.is_null())
            .cloned()
            .ok_or_else(|| cognition_validation_error("scheduler_unconfigured"))?;
        CognitionScheduler::from_snapshot_json(state).map_err(|error| scheduler_error(error.code()))
    }

    fn cognition_set_scheduler(&mut self, scheduler: &CognitionScheduler) {
        let mut projection = self.cognition.as_object().cloned().unwrap_or_default();
        projection.insert("scheduler_state".to_string(), scheduler.snapshot_json());
        self.cognition = JsonValue::Object(projection);
    }

    fn cognition_continuations_typed(&self) -> Result<Vec<AgentContinuation>, WorldError> {
        let value = self
            .cognition
            .get("continuations")
            .cloned()
            .unwrap_or_else(|| JsonValue::Array(Vec::new()));
        serde_json::from_value(value).map_err(WorldError::from)
    }

    fn cognition_set_continuations(
        &mut self,
        continuations: &[AgentContinuation],
    ) -> Result<(), WorldError> {
        let mut projection = self.cognition.as_object().cloned().unwrap_or_default();
        projection.insert(
            "continuations".to_string(),
            serde_json::to_value(continuations).map_err(WorldError::from)?,
        );
        self.cognition = JsonValue::Object(projection);
        Ok(())
    }

    fn cognition_retention_store(&self) -> Result<CognitionRetentionStore, WorldError> {
        let Some(value) = self.cognition.get("retention_state") else {
            return Ok(CognitionRetentionStore::default());
        };
        serde_json::from_value(value.clone()).map_err(WorldError::from)
    }

    fn cognition_set_retention_store(
        &mut self,
        store: &CognitionRetentionStore,
    ) -> Result<(), WorldError> {
        let mut projection = self.cognition.as_object().cloned().unwrap_or_default();
        projection.insert(
            "retention_state".to_string(),
            serde_json::to_value(store).map_err(WorldError::from)?,
        );
        self.cognition = JsonValue::Object(projection);
        Ok(())
    }
}

fn cognition_validation_error(code: &str) -> WorldError {
    WorldError::DistributedValidationFailed {
        reason: format!("cognition validation failed: {code}"),
    }
}

fn scheduler_error(code: &str) -> WorldError {
    WorldError::DistributedValidationFailed {
        reason: format!("cognition scheduler failed: {code}"),
    }
}
