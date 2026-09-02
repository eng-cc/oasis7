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
    AgentContinuation, CognitionContinuationProposalV1, ContinuationReorgReport,
    ContinuationStatusV1, ContinuationTransition, WakeConditionV1, WakeConditionValidator,
    WakeEvaluation, WakeEvaluationContext,
};
use crate::runtime::error::WorldError;
use crate::simulator::ResourceKind;
use serde_json::{Value as JsonValue, json};

const CONTINUATION_SCHEMA: &str = "agent-continuation.v1";

impl World {
    fn is_terminal_continuation_status(status: ContinuationStatusV1) -> bool {
        matches!(
            status,
            ContinuationStatusV1::Completed
                | ContinuationStatusV1::Cancelled
                | ContinuationStatusV1::Invalidated
                | ContinuationStatusV1::Expired
                | ContinuationStatusV1::Rejected
        )
    }

    fn continuation_lifecycle_event_kind(status: ContinuationStatusV1) -> &'static str {
        match status {
            ContinuationStatusV1::Completed => "ContinuationCompleted",
            ContinuationStatusV1::Cancelled => "ContinuationCancelled",
            ContinuationStatusV1::Invalidated => "ContinuationInvalidated",
            ContinuationStatusV1::Expired => "ContinuationExpired",
            ContinuationStatusV1::Rejected => "ContinuationRejected",
            _ => "ContinuationTransitioned",
        }
    }

    /// Configure the durable World scheduler.  Configuration is encoded in
    /// the cognition projection immediately so a save taken before the first
    /// wake still restores the chosen policy and capacity.
    pub fn with_cognition_scheduler(mut self, policy: SchedulerPolicyV1, capacity: usize) -> Self {
        let scheduler = CognitionScheduler::try_new(policy, capacity)
            .expect("invalid cognition scheduler configuration");
        self.cognition_commit_scheduler_transaction(&scheduler, "SchedulerConfigured", None)
            .expect("scheduler configuration must be durable");
        self
    }

    /// Fallible constructor used by restore/admission paths so invalid
    /// scheduler policy or capacity can never enter the World projection.
    pub fn try_with_cognition_scheduler(
        mut self,
        policy: SchedulerPolicyV1,
        capacity: usize,
    ) -> Result<Self, WorldError> {
        let scheduler = CognitionScheduler::try_new(policy, capacity)
            .map_err(|error| scheduler_error(error.code()))?;
        self.cognition_commit_scheduler_transaction(&scheduler, "SchedulerConfigured", None)?;
        Ok(self)
    }

    /// Bind the scheduler and cognition adapters to the World-owned identity.
    /// An existing contradictory binding is never overwritten.
    pub fn bind_cognition_runtime(
        &mut self,
        world_id: impl Into<String>,
        branch_id: impl Into<String>,
        finality_epoch: u64,
        finality_block_hash: Option<String>,
        finality_status: impl Into<String>,
        reorg_epoch: u64,
    ) -> Result<(), WorldError> {
        let binding = json!({
            "world_id": world_id.into(),
            "branch_id": branch_id.into(),
            "finality_epoch": finality_epoch,
            "finality_block_hash": finality_block_hash,
            "finality_status": finality_status.into(),
            "reorg_epoch": reorg_epoch,
            "runtime_manifest_hash": self.current_manifest_hash()?,
        });
        if binding["world_id"]
            .as_str()
            .unwrap_or_default()
            .trim()
            .is_empty()
            || binding["branch_id"]
                .as_str()
                .unwrap_or_default()
                .trim()
                .is_empty()
            || binding["finality_status"]
                .as_str()
                .unwrap_or_default()
                .trim()
                .is_empty()
        {
            return Err(cognition_validation_error("runtime_binding_invalid"));
        }
        if let Some(existing) = self.cognition.get("runtime_binding")
            && existing != &JsonValue::Null
            && existing != &binding
        {
            return Err(cognition_validation_error("runtime_binding_conflict"));
        }
        let mut projection = self.cognition.as_object().cloned().unwrap_or_default();
        projection.insert("runtime_binding".to_string(), binding);
        self.cognition = JsonValue::Object(projection);
        Ok(())
    }

    pub fn enqueue_cognition_wake(
        &mut self,
        wake: SchedulerWakeV1,
    ) -> Result<SchedulerEnqueueOutcome, WorldError> {
        self.validate_cognition_wake_binding(&wake)?;
        let mut scheduler = self.cognition_scheduler()?;
        scheduler.advance_logical_tick(self.state.time);
        let outcome = scheduler
            .try_enqueue(wake.clone())
            .map_err(|error| scheduler_error(error.code()))?;
        self.cognition_commit_scheduler_transaction(
            &scheduler,
            "SchedulerWakeEnqueued",
            Some(&wake),
        )?;
        Ok(outcome)
    }

    pub fn select_ready_cognition_wakes(
        &mut self,
        tick: u64,
    ) -> Result<Vec<SchedulerWakeV1>, WorldError> {
        if let Some(state) = self
            .cognition
            .get("scheduler_state")
            .filter(|state| !state.is_null())
        {
            self.validate_persisted_cognition_wakes(state)?;
        }
        let mut scheduler = self.cognition_scheduler()?;
        let continuations = self.cognition_continuations_typed()?;
        let stale = scheduler.prune_wakes_if(|wake| {
            self.cognition_wake_has_active_continuation(wake, &continuations)
        });
        let selected = scheduler.select_ready_if(tick, |wake| {
            self.cognition_wake_conditions_ready(wake, &continuations)
        });
        let mut transaction = self.clone();
        for wake in &stale {
            transaction.cognition_commit_scheduler_transaction(
                &scheduler,
                "SchedulerWakeDeactivated",
                Some(wake),
            )?;
        }
        if selected.is_empty() {
            transaction.cognition_commit_scheduler_transaction(
                &scheduler,
                "SchedulerWakeSelected",
                None,
            )?;
        } else {
            for wake in &selected {
                transaction.cognition_commit_scheduler_transaction(
                    &scheduler,
                    "SchedulerWakeSelected",
                    Some(wake),
                )?;
                if !wake.continuation_id.trim().is_empty() {
                    transaction.cognition_commit_scheduler_transaction(
                        &scheduler,
                        "ContinuationWoken",
                        Some(wake),
                    )?;
                }
            }
        }
        *self = transaction;
        Ok(selected)
    }

    /// Run one scheduler service pass from the authoritative World path.
    ///
    /// The returned wakes are leases: capacity remains occupied until the
    /// caller reports completion through `release_cognition_wake`, or until a
    /// terminal continuation transition deactivates the same wake identity.
    /// A World without a configured scheduler is intentionally a no-op so
    /// legacy worlds do not acquire an implicit policy.
    pub fn service_cognition_scheduler_tick(
        &mut self,
        tick: u64,
    ) -> Result<Vec<SchedulerWakeV1>, WorldError> {
        let Some(state) = self
            .cognition
            .get("scheduler_state")
            .filter(|state| !state.is_null())
        else {
            return Ok(Vec::new());
        };
        // An empty ready/backpressure set has no service work. In particular,
        // do not append a journal event on every tick while a lease is
        // waiting for its exact completion release.
        let has_ready_or_pending = state
            .get("active")
            .and_then(JsonValue::as_array)
            .is_some_and(|wakes| !wakes.is_empty())
            || state
                .get("backpressure")
                .and_then(JsonValue::as_object)
                .is_some_and(|wakes| !wakes.is_empty());
        if !has_ready_or_pending {
            return Ok(Vec::new());
        }
        self.select_ready_cognition_wakes(tick)
    }

    /// Release exactly one durable scheduler lease.  Unknown identities are
    /// rejected transactionally, leaving the scheduler projection unchanged.
    pub fn release_cognition_wake(&mut self, wake_id: &str) -> Result<SchedulerWakeV1, WorldError> {
        if wake_id.trim().is_empty() {
            return Err(scheduler_error("in_flight_wake_missing"));
        }
        let mut transaction = self.clone();
        let mut scheduler = transaction.cognition_scheduler()?;
        let wake = scheduler
            .release_in_flight(wake_id)
            .map_err(|error| scheduler_error(error.code()))?;
        transaction.cognition_commit_scheduler_transaction(
            &scheduler,
            "SchedulerWakeReleased",
            Some(&wake),
        )?;
        // A release makes one bounded slot available immediately. Promote
        // due backpressure entries in the same World transaction so a
        // pending wake cannot remain stranded until an out-of-band recovery
        // call.
        let recovered = scheduler.recover_capacity_preserving_cursor(self.state.time);
        for recovered_wake in &recovered {
            transaction.cognition_commit_scheduler_transaction(
                &scheduler,
                "SchedulerWakeRecovered",
                Some(recovered_wake),
            )?;
        }
        *self = transaction;
        Ok(wake)
    }

    /// Release the in-flight slot abandoned by the process that was restored,
    /// then recover pending wakes without advancing the committed cursor.
    pub fn recover_cognition_scheduler(
        &mut self,
        tick: u64,
    ) -> Result<Vec<SchedulerWakeV1>, WorldError> {
        let mut scheduler = self.cognition_scheduler()?;
        let mut recovered = scheduler.recover_in_flight_preserving_cursor(tick);
        recovered.extend(scheduler.recover_capacity_preserving_cursor(tick));
        let mut transaction = self.clone();
        if recovered.is_empty() {
            transaction.cognition_commit_scheduler_transaction(
                &scheduler,
                "SchedulerWakeRecovered",
                None,
            )?;
        } else {
            for wake in &recovered {
                transaction.cognition_commit_scheduler_transaction(
                    &scheduler,
                    "SchedulerWakeRecovered",
                    Some(wake),
                )?;
            }
        }
        *self = transaction;
        Ok(recovered)
    }

    pub fn cognition_scheduler_snapshot(&self) -> JsonValue {
        self.cognition
            .get("scheduler_state")
            .cloned()
            .unwrap_or(JsonValue::Null)
    }

    /// Read the durable scheduler leases for the production dispatcher.
    /// Unconfigured legacy worlds have no cognition leases.
    pub fn cognition_in_flight_wakes(&self) -> Result<Vec<SchedulerWakeV1>, WorldError> {
        if self
            .cognition
            .get("scheduler_state")
            .is_none_or(JsonValue::is_null)
        {
            return Ok(Vec::new());
        }
        Ok(self.cognition_scheduler()?.in_flight_wakes())
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
        let head_digest = self.current_state_root_hash()?;
        let (event_digests, receipt_ids) = self.cognition_committed_evidence()?;
        let mut context = WakeEvaluationContext::at(self.state.time)
            .with_evaluation_head(&head_digest)
            .with_reorg_epoch(self.cognition_reorg_epoch());
        for digest in &event_digests {
            context = context.with_event(digest);
        }
        for receipt_id in &receipt_ids {
            context = context.with_receipt(receipt_id);
        }
        self.evaluate_cognition_wake_with_context(conditions, context)
    }

    pub fn evaluate_cognition_wake_from_committed_projection(
        &self,
        conditions: &[WakeConditionV1],
        event_digest: &str,
        receipt_id: &str,
    ) -> Result<WakeEvaluation, WorldError> {
        let head_digest = self.current_state_root_hash()?;
        let (event_digests, receipt_ids) = self.cognition_committed_evidence()?;
        let mut context = WakeEvaluationContext::at(self.state.time)
            .with_reorg_epoch(self.cognition_reorg_epoch())
            .with_evaluation_head(&head_digest);
        if event_digests.contains(event_digest) {
            context = context.with_event(event_digest);
        }
        if receipt_ids.contains(receipt_id) {
            context = context.with_receipt(receipt_id);
        }
        self.evaluate_cognition_wake_with_context(conditions, context)
    }

    /// Admit an agent-owned continuation proposal at the Runtime boundary.
    /// Identity, sequence, status and digest fields are allocated here; none
    /// are accepted from a caller-owned wire projection.
    pub fn admit_cognition_continuation(
        &mut self,
        proposal: CognitionContinuationProposalV1,
    ) -> Result<AgentContinuation, WorldError> {
        // Admission allocates a process-global proposal sequence as well as
        // binding the cognition projection. Keep both mutations in the same
        // World transaction so a rejected proposal cannot poison the next
        // request with a forged runtime binding or consume an ID.
        let mut transaction = self.clone();
        let admitted = transaction.admit_cognition_continuation_inner(proposal)?;
        *self = transaction;
        Ok(admitted)
    }

    fn admit_cognition_continuation_inner(
        &mut self,
        mut proposal: CognitionContinuationProposalV1,
    ) -> Result<AgentContinuation, WorldError> {
        let runtime_was_bound = !self.cognition_runtime_is_unbound();
        self.bind_cognition_proposal_fields(&mut proposal)?;
        if !runtime_was_bound {
            self.cognition
                .as_object_mut()
                .ok_or_else(|| cognition_validation_error("cognition_projection_not_object"))?
                .insert(
                    "runtime_binding_source".to_string(),
                    JsonValue::String("continuation_admission_compatibility".to_string()),
                );
        }
        proposal
            .validate()
            .map_err(|error| cognition_validation_error(error.code()))?;
        self.validate_cognition_proposal_binding(&proposal)?;
        let wake_conditions = WakeConditionValidator::canonicalize(proposal.wake_conditions)
            .map_err(|error| cognition_validation_error(error.code()))?;
        let derived_next_wake_tick =
            WakeConditionValidator::next_wake_tick_at(&wake_conditions, self.state.time)
                .map_err(|error| cognition_validation_error(error.code()))?;
        if proposal.next_wake_tick.is_none() {
            proposal.next_wake_tick = derived_next_wake_tick;
        }
        if proposal.world_id.trim().is_empty()
            || proposal.branch_id.trim().is_empty()
            || proposal.finality_status.trim().is_empty()
            || proposal.runtime_manifest_hash.trim().is_empty()
            || proposal.agent_id.trim().is_empty()
            || proposal.agent_session_id.trim().is_empty()
            || proposal.agent_turn_id.trim().is_empty()
            || proposal.decision_request_id.trim().is_empty()
            || proposal.origin_turn_id.trim().is_empty()
            || proposal.origin_request_digest.trim().is_empty()
            || proposal.continuation_proposal_id.trim().is_empty()
            || proposal.proposal_digest.trim().is_empty()
            || proposal.precondition_digest.trim().is_empty()
            || proposal.remaining_budget.value == 0
            || !matches!(proposal.remaining_budget.unit.as_str(), "steps" | "ticks")
            || proposal.next_wake_tick != derived_next_wake_tick
            || proposal
                .valid_until_tick
                .is_some_and(|valid_until| valid_until < self.state.time)
        {
            return Err(cognition_validation_error("continuation_proposal_invalid"));
        }
        let mut continuations = self.cognition_continuations_typed()?;
        if runtime_was_bound
            && self.cognition.get("runtime_binding_source")
                != Some(&JsonValue::String(
                    "continuation_admission_compatibility".to_string(),
                ))
            && !self.cognition_turn_is_registered(
                &proposal.agent_id,
                &proposal.agent_session_id,
                &proposal.agent_turn_id,
                &proposal.decision_request_id,
                &proposal.origin_request_digest,
            )
        {
            return Err(cognition_validation_error("continuation_turn_unregistered"));
        }
        if continuations
            .iter()
            .any(|existing| existing.continuation_proposal_id == proposal.continuation_proposal_id)
        {
            return Err(cognition_validation_error("continuation_proposal_conflict"));
        }
        let allocation = self.allocate_next_proposal_id();
        let continuation_id = format!("continuation:{allocation}");
        let wake_id = format!("wake:{allocation}");
        let wake_seq = continuations
            .iter()
            .filter(|existing| existing.agent_id == proposal.agent_id)
            .map(|existing| existing.wake_seq)
            .max()
            .unwrap_or(0)
            .saturating_add(1);
        let mut continuation = AgentContinuation {
            schema_version: CONTINUATION_SCHEMA.to_string(),
            continuation_id,
            wake_id,
            world_id: proposal.world_id,
            branch_id: proposal.branch_id,
            finality_epoch: proposal.finality_epoch,
            finality_block_hash: proposal.finality_block_hash,
            finality_status: proposal.finality_status,
            reorg_epoch: proposal.reorg_epoch,
            runtime_manifest_hash: proposal.runtime_manifest_hash,
            agent_id: proposal.agent_id,
            agent_session_id: proposal.agent_session_id,
            agent_turn_id: proposal.agent_turn_id,
            decision_request_id: proposal.decision_request_id,
            origin_turn_id: proposal.origin_turn_id,
            origin_request_digest: proposal.origin_request_digest,
            continuation_proposal_id: proposal.continuation_proposal_id,
            proposal_digest: proposal.proposal_digest,
            action_or_envelope_digest: proposal.action_or_envelope_digest,
            wake_conditions,
            next_wake_tick: proposal.next_wake_tick,
            remaining_budget: proposal.remaining_budget,
            valid_until_tick: proposal.valid_until_tick,
            precondition_digest: proposal.precondition_digest,
            wake_seq,
            logical_tick: self.state.time,
            status: ContinuationStatusV1::Scheduled,
            continuation_status_digest: None,
            terminal_disposition: None,
        };
        continuation.refresh_status_digest();
        continuation
            .validate_authoritative()
            .map_err(|error| cognition_validation_error(error.code()))?;
        // Untimed event/receipt/state predicates are awakened by committed
        // evidence. The scheduler uses MAX as a non-timed sentinel and World
        // selection re-evaluates the typed conditions before delivery.
        let next_wake_tick = continuation.next_wake_tick.unwrap_or(u64::MAX);
        let mut scheduler = self.cognition_scheduler()?;
        scheduler.advance_logical_tick(self.state.time);
        let wake = SchedulerWakeV1 {
            schema_version: SchedulerWakeV1::SCHEMA_VERSION.to_string(),
            wake_id: continuation.wake_id.clone(),
            continuation_id: continuation.continuation_id.clone(),
            world_id: continuation.world_id.clone(),
            branch_id: continuation.branch_id.clone(),
            finality_epoch: continuation.finality_epoch,
            finality_block_hash: continuation
                .finality_block_hash
                .clone()
                .unwrap_or_else(|| "genesis".to_string()),
            finality_status: continuation.finality_status.clone(),
            reorg_epoch: continuation.reorg_epoch,
            runtime_manifest_hash: continuation.runtime_manifest_hash.clone(),
            agent_id: continuation.agent_id.clone(),
            agent_session_id: continuation.agent_session_id.clone(),
            agent_turn_id: continuation.agent_turn_id.clone(),
            decision_request_id: continuation.decision_request_id.clone(),
            next_wake_tick,
            eligible_since_tick: self.state.time,
            starvation_deadline_tick: self
                .state
                .time
                .saturating_add(scheduler.policy().max_starvation_ticks),
            initial_priority: scheduler.policy().initial_priority,
            wake_seq: continuation.wake_seq,
            retry_seq: 0,
            status: "pending".to_string(),
            pending_reason: "capacity_available".to_string(),
        };
        scheduler
            .try_enqueue(wake.clone())
            .map_err(|error| scheduler_error(error.code()))?;
        continuations.push(continuation.clone());
        self.cognition_commit_continuation_transaction(&continuations, &scheduler, &wake)?;
        Ok(continuation)
    }

    pub fn schedule_cognition_continuation(
        &mut self,
        _continuation: AgentContinuation,
    ) -> Result<(), WorldError> {
        Err(cognition_validation_error("legacy_continuation_projection"))
    }

    /// Apply one Runtime-owned continuation transition and persist its
    /// refreshed status digest in the same World projection update.
    pub fn transition_cognition_continuation(
        &mut self,
        continuation_id: &str,
        to: ContinuationStatusV1,
        logical_tick: u64,
    ) -> Result<AgentContinuation, WorldError> {
        let mut transaction = self.clone();
        let mut continuations = transaction.cognition_continuations_typed()?;
        let continuation = continuations
            .iter_mut()
            .find(|continuation| continuation.continuation_id == continuation_id)
            .ok_or_else(|| cognition_validation_error("continuation_missing"))?;
        ContinuationTransition::apply_at_tick(continuation, to, logical_tick)
            .map_err(|error| cognition_validation_error(error.code()))?;
        continuation
            .validate_authoritative()
            .map_err(|error| cognition_validation_error(error.code()))?;
        let transitioned = continuation.clone();
        let mut scheduler = transaction.cognition_scheduler()?;
        let deactivated = if Self::is_terminal_continuation_status(transitioned.status) {
            scheduler
                .deactivate_wake(transitioned.wake_id.as_str())
                .map_err(|error| scheduler_error(error.code()))?
        } else {
            None
        };
        let event_kind = Self::continuation_lifecycle_event_kind(transitioned.status);
        transaction.cognition_commit_continuation_lifecycle_transaction(
            &continuations,
            &scheduler,
            event_kind,
            &transitioned,
            deactivated.as_ref(),
        )?;
        *self = transaction;
        Ok(transitioned)
    }

    pub fn invalidate_cognition_for_reorg(
        &mut self,
        reorg_epoch: u64,
    ) -> Result<JsonValue, WorldError> {
        let mut transaction = self.clone();
        let mut continuations = transaction.cognition_continuations_typed()?;
        let mut scheduler = transaction.cognition_scheduler()?;
        let mut transitioned = Vec::new();
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
            ContinuationTransition::invalidate_for_reorg_at_tick(
                continuation,
                reorg_epoch,
                self.state.time,
            )
            .map_err(|error| cognition_validation_error(error.code()))?;
            invalidated = invalidated.saturating_add(1);
            let deactivated = scheduler
                .deactivate_wake(continuation.wake_id.as_str())
                .map_err(|error| scheduler_error(error.code()))?;
            transitioned.push((continuation.clone(), deactivated));
        }
        for (continuation, deactivated) in &transitioned {
            transaction.cognition_commit_continuation_lifecycle_transaction(
                &continuations,
                &scheduler,
                "ContinuationInvalidated",
                continuation,
                deactivated.as_ref(),
            )?;
        }
        if transitioned.is_empty() {
            transaction.cognition_commit_scheduler_transaction(
                &scheduler,
                "ContinuationReorgChecked",
                None,
            )?;
        }
        *self = transaction;
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
        let context = self.enrich_cognition_wake_context(conditions, context)?;
        WakeConditionValidator::evaluate(conditions, &context)
            .map_err(|error| cognition_validation_error(error.code()))
    }

    fn enrich_cognition_wake_context(
        &self,
        conditions: &[WakeConditionV1],
        mut context: WakeEvaluationContext,
    ) -> Result<WakeEvaluationContext, WorldError> {
        let world_id = self.cognition_world_id();
        let world_predicate_requested = conditions.iter().any(|condition| {
            condition.kind == "state_predicate"
                && condition.subject.as_ref().is_some_and(|subject| {
                    subject.kind == "world" && world_id.as_deref() == Some(subject.id.as_str())
                })
        });
        if world_predicate_requested {
            context = context
                .with_predicate_u64("world.logical_tick", self.state.time)
                .with_predicate_u64("world.reorg_epoch", self.cognition_reorg_epoch());
            let state_root = self.current_state_root_hash()?;
            context = context.with_predicate_value(
                "world.state_root",
                &serde_cbor::to_vec(&state_root).map_err(WorldError::from)?,
            );
            let manifest_hash = self.current_manifest_hash()?;
            context = context.with_predicate_value(
                "world.runtime_manifest_hash",
                &serde_cbor::to_vec(&manifest_hash).map_err(WorldError::from)?,
            );
        }
        for condition in conditions {
            if condition.kind != "state_predicate" {
                continue;
            }
            let Some(subject) = condition.subject.as_ref() else {
                continue;
            };
            let Some(path) = condition.path_or_rule.as_deref() else {
                continue;
            };
            match (subject.kind.as_str(), path) {
                ("agent", "agent.status")
                | ("agent", "agent.position")
                | ("agent", "agent.inventory_digest")
                | ("agent", "agent.capability_snapshot_hash")
                | ("agent", "agent.resource.electricity")
                | ("agent", "agent.resource.data") => {
                    let Some(cell) = self.state.agents.get(&subject.id) else {
                        continue;
                    };
                    let value = match path {
                        "agent.status" => serde_cbor::to_vec(
                            &cell
                                .activity
                                .as_ref()
                                .map(|activity| activity.status)
                                .unwrap_or(crate::runtime::AgentActivityStatus::Idle),
                        ),
                        "agent.position" => {
                            serde_cbor::to_vec(&(cell.state.pos.x_cm, cell.state.pos.y_cm))
                        }
                        "agent.inventory_digest" => serde_cbor::to_vec(
                            &oasis7_wasm_abi::canonical_hash(&cell.state.body_state.cargo_entries)
                                .map_err(|_| {
                                    cognition_validation_error("predicate_value_invalid")
                                })?,
                        ),
                        "agent.capability_snapshot_hash" => {
                            serde_cbor::to_vec(&self.capability_authorization_root())
                        }
                        "agent.resource.electricity" => {
                            serde_cbor::to_vec(&cell.state.resources.get(ResourceKind::Electricity))
                        }
                        "agent.resource.data" => {
                            serde_cbor::to_vec(&cell.state.resources.get(ResourceKind::Data))
                        }
                        _ => unreachable!(),
                    }
                    .map_err(WorldError::from)?;
                    context = context.with_predicate_value(path, &value);
                }
                ("intent", "intent.status") => {
                    let Some(intent) = self.state.agent_intent_ledger.get(&subject.id) else {
                        continue;
                    };
                    let value = serde_cbor::to_vec(&intent.status).map_err(WorldError::from)?;
                    context = context.with_predicate_value(path, &value);
                }
                _ => {}
            }
        }
        Ok(context)
    }

    fn cognition_world_id(&self) -> Option<String> {
        self.cognition
            .get("runtime_binding")
            .and_then(|binding| binding.get("world_id"))
            .and_then(JsonValue::as_str)
            .filter(|world_id| !world_id.trim().is_empty())
            .map(str::to_string)
            .or_else(|| {
                let world_id = self.chain_resource_manifest().world_id.clone();
                (world_id != "unbound").then_some(world_id)
            })
    }

    fn cognition_reorg_epoch(&self) -> u64 {
        self.state
            .agent_intent_ledger
            .values()
            .filter_map(|intent| intent.reorg_epoch)
            .max()
            .unwrap_or(0)
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

    fn cognition_wake_has_active_continuation(
        &self,
        wake: &SchedulerWakeV1,
        continuations: &[AgentContinuation],
    ) -> bool {
        let Some(continuation) = continuations
            .iter()
            .find(|continuation| continuation.continuation_id == wake.continuation_id)
        else {
            // Legacy scheduler-only projections remain readable while an
            // unbound World is migrated. Once Runtime authority is bound,
            // every wake must resolve to its durable continuation.
            return self.cognition_runtime_is_unbound();
        };
        !matches!(
            continuation.status,
            ContinuationStatusV1::Completed
                | ContinuationStatusV1::Cancelled
                | ContinuationStatusV1::Invalidated
                | ContinuationStatusV1::Expired
                | ContinuationStatusV1::Rejected
        )
    }

    fn cognition_wake_conditions_ready(
        &self,
        wake: &SchedulerWakeV1,
        continuations: &[AgentContinuation],
    ) -> bool {
        let Some(continuation) = continuations
            .iter()
            .find(|continuation| continuation.continuation_id == wake.continuation_id)
        else {
            return self.cognition_runtime_is_unbound();
        };
        if !self.cognition_wake_has_active_continuation(wake, continuations) {
            return false;
        }
        self.evaluate_cognition_wake(&continuation.wake_conditions)
            .map(|evaluation| evaluation.status == "ready")
            .unwrap_or(false)
    }

    pub(super) fn validate_cognition_wake_binding(
        &self,
        wake: &SchedulerWakeV1,
    ) -> Result<(), WorldError> {
        wake.validate()
            .map_err(|error| scheduler_error(error.code()))?;
        if let Some(binding) = self.cognition.get("runtime_binding") {
            let block_hash = binding["finality_block_hash"].as_str().unwrap_or("genesis");
            if binding["world_id"].as_str() != Some(wake.world_id.as_str())
                || binding["branch_id"].as_str() != Some(wake.branch_id.as_str())
                || binding["finality_epoch"].as_u64() != Some(wake.finality_epoch)
                || block_hash != wake.finality_block_hash
                || binding["finality_status"].as_str() != Some(wake.finality_status.as_str())
                || binding["reorg_epoch"].as_u64() != Some(wake.reorg_epoch)
                || binding["runtime_manifest_hash"].as_str()
                    != Some(wake.runtime_manifest_hash.as_str())
            {
                return Err(cognition_validation_error("foreign_scheduler_wake"));
            }
        }
        let continuations = self.cognition_continuations_typed()?;
        let Some(continuation) = continuations
            .iter()
            .find(|continuation| continuation.continuation_id == wake.continuation_id)
        else {
            return if self.cognition_runtime_is_unbound() {
                Ok(())
            } else {
                Err(cognition_validation_error("foreign_scheduler_wake"))
            };
        };
        if continuation.wake_id != wake.wake_id
            || continuation.world_id != wake.world_id
            || continuation.branch_id != wake.branch_id
            || continuation.finality_epoch != wake.finality_epoch
            || continuation
                .finality_block_hash
                .as_deref()
                .unwrap_or("genesis")
                != wake.finality_block_hash
            || continuation.finality_status != wake.finality_status
            || continuation.reorg_epoch != wake.reorg_epoch
            || continuation.runtime_manifest_hash != wake.runtime_manifest_hash
            || continuation.agent_id != wake.agent_id
            || continuation.agent_session_id != wake.agent_session_id
            || continuation.agent_turn_id != wake.agent_turn_id
            || continuation.decision_request_id != wake.decision_request_id
        {
            return Err(cognition_validation_error("foreign_scheduler_wake"));
        }
        Ok(())
    }

    fn cognition_runtime_is_unbound(&self) -> bool {
        self.cognition
            .get("runtime_binding")
            .is_none_or(JsonValue::is_null)
    }

    fn bind_cognition_proposal_fields(
        &mut self,
        proposal: &mut CognitionContinuationProposalV1,
    ) -> Result<(), WorldError> {
        let manifest_hash = self.current_manifest_hash()?;
        if proposal.runtime_manifest_hash.is_empty() {
            proposal.runtime_manifest_hash = manifest_hash.clone();
        } else if proposal.runtime_manifest_hash != manifest_hash {
            return Err(cognition_validation_error("runtime_manifest_mismatch"));
        }
        if let Some(binding) = self.cognition.get("runtime_binding") {
            if proposal.branch_id.is_empty() {
                proposal.branch_id = binding["branch_id"].as_str().unwrap_or("main").to_string();
            }
            if proposal.finality_status.is_empty() {
                proposal.finality_status = binding["finality_status"]
                    .as_str()
                    .unwrap_or("pending")
                    .to_string();
            }
            if proposal.finality_epoch == 0 {
                proposal.finality_epoch = binding["finality_epoch"].as_u64().unwrap_or_default();
            }
            if proposal.reorg_epoch == 0 {
                proposal.reorg_epoch = binding["reorg_epoch"].as_u64().unwrap_or_default();
            }
            if proposal.finality_block_hash.is_none() {
                proposal.finality_block_hash =
                    binding["finality_block_hash"].as_str().map(str::to_string);
            }
            let block_hash = binding["finality_block_hash"].as_str().map(str::to_string);
            if binding["world_id"].as_str() != Some(proposal.world_id.as_str())
                || binding["branch_id"].as_str() != Some(proposal.branch_id.as_str())
                || binding["finality_epoch"].as_u64() != Some(proposal.finality_epoch)
                || block_hash != proposal.finality_block_hash
                || binding["finality_status"].as_str() != Some(proposal.finality_status.as_str())
                || binding["reorg_epoch"].as_u64() != Some(proposal.reorg_epoch)
            {
                return Err(cognition_validation_error("foreign_continuation_proposal"));
            }
        } else {
            if proposal.branch_id.is_empty() {
                proposal.branch_id = "main".to_string();
            }
            if proposal.finality_status.is_empty() {
                proposal.finality_status = "pending".to_string();
            }
            self.bind_cognition_runtime(
                proposal.world_id.clone(),
                proposal.branch_id.clone(),
                proposal.finality_epoch,
                proposal.finality_block_hash.clone(),
                proposal.finality_status.clone(),
                proposal.reorg_epoch,
            )?;
        }
        if !self.state.agents.is_empty() && !self.state.agents.contains_key(&proposal.agent_id) {
            return Err(cognition_validation_error("continuation_agent_missing"));
        }
        Ok(())
    }

    fn validate_cognition_proposal_binding(
        &self,
        proposal: &CognitionContinuationProposalV1,
    ) -> Result<(), WorldError> {
        if proposal.runtime_manifest_hash != self.current_manifest_hash()? {
            return Err(cognition_validation_error("runtime_manifest_mismatch"));
        }
        if let Some(binding) = self.cognition.get("runtime_binding") {
            let block_hash = binding["finality_block_hash"].as_str().map(str::to_string);
            if binding["world_id"].as_str() != Some(proposal.world_id.as_str())
                || binding["branch_id"].as_str() != Some(proposal.branch_id.as_str())
                || binding["finality_epoch"].as_u64() != Some(proposal.finality_epoch)
                || block_hash != proposal.finality_block_hash
                || binding["finality_status"].as_str() != Some(proposal.finality_status.as_str())
                || binding["reorg_epoch"].as_u64() != Some(proposal.reorg_epoch)
            {
                return Err(cognition_validation_error("foreign_continuation_proposal"));
            }
        }
        if !self.state.agents.is_empty() && !self.state.agents.contains_key(&proposal.agent_id) {
            return Err(cognition_validation_error("continuation_agent_missing"));
        }
        Ok(())
    }

    fn cognition_turn_is_registered(
        &self,
        agent_id: &str,
        agent_session_id: &str,
        agent_turn_id: &str,
        decision_request_id: &str,
        request_digest: &str,
    ) -> bool {
        self.cognition
            .get("cognition_journal")
            .and_then(|journal| journal.get("events"))
            .and_then(JsonValue::as_array)
            .is_some_and(|events| {
                events.iter().any(|event| {
                    matches!(
                        event.get("kind").and_then(JsonValue::as_str),
                        Some("TurnStarted")
                            | Some("ContextCaptured")
                            | Some("RequestDispatched")
                            | Some("DecisionValidated")
                    ) && event.get("agent_id").and_then(JsonValue::as_str) == Some(agent_id)
                        && event.get("agent_session_id").and_then(JsonValue::as_str)
                            == Some(agent_session_id)
                        && event.get("agent_turn_id").and_then(JsonValue::as_str)
                            == Some(agent_turn_id)
                        && event.get("decision_request_id").and_then(JsonValue::as_str)
                            == Some(decision_request_id)
                        && event.get("request_digest").and_then(JsonValue::as_str)
                            == Some(request_digest)
                })
            })
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
