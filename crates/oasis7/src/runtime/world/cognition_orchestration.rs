use super::World;
use super::cognition_persistence_validation::append_cognition_event;
use crate::runtime::cognition::{
    PreconditionSubjectV1, finality_binding_is_legal, world_state_binding_digest_v1,
};
use crate::runtime::cognition_recovery::{RuntimeCognitionBaseBindingV1, cognition_digest_v1};
use crate::runtime::cognition_retention::{
    CognitionRetentionStore, RetentionExecutionProbe, RetentionRecordV1,
};
use crate::runtime::cognition_scheduler::{
    CognitionScheduler, SchedulerEnqueueOutcome, SchedulerPolicyV1, SchedulerWakeV1,
};
use crate::runtime::cognition_wake::{
    AgentContinuation, CognitionContinuationProposalV1, CognitionWakeDispositionV1,
    ContinuationReorgReport, ContinuationStatusV1, ContinuationTransition, WakeConditionV1,
    WakeConditionValidator, WakeEvaluation, WakeEvaluationContext,
};
use crate::runtime::error::WorldError;
use crate::simulator::ResourceKind;
use serde_json::{Value as JsonValue, json};

const CONTINUATION_SCHEMA: &str = "agent-continuation.v1";

#[path = "cognition_orchestration_support.rs"]
mod cognition_orchestration_support;

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

    pub fn with_cognition_scheduler(mut self, policy: SchedulerPolicyV1, capacity: usize) -> Self {
        let scheduler = CognitionScheduler::try_new(policy, capacity)
            .expect("invalid cognition scheduler configuration");
        self.cognition_commit_scheduler_transaction(&scheduler, "SchedulerConfigured", None)
            .expect("scheduler configuration must be durable");
        self
    }

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

    pub fn bind_cognition_runtime(
        &mut self,
        world_id: impl Into<String>,
        branch_id: impl Into<String>,
        finality_epoch: u64,
        finality_block_hash: Option<String>,
        finality_status: impl Into<String>,
        reorg_epoch: u64,
    ) -> Result<(), WorldError> {
        let mut transaction = self.clone();
        transaction.bind_cognition_runtime_inner(
            world_id,
            branch_id,
            finality_epoch,
            finality_block_hash,
            finality_status,
            reorg_epoch,
        )?;
        transaction.persist_runtime_transaction_if_configured()?;
        *self = transaction;
        Ok(())
    }

    fn bind_cognition_runtime_inner(
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
        let world_id = binding["world_id"]
            .as_str()
            .ok_or_else(|| cognition_validation_error("runtime_binding_invalid"))?;
        let branch_id = binding["branch_id"]
            .as_str()
            .ok_or_else(|| cognition_validation_error("runtime_binding_invalid"))?;
        let finality_status = binding["finality_status"]
            .as_str()
            .ok_or_else(|| cognition_validation_error("runtime_binding_invalid"))?;
        if !finality_binding_is_legal(finality_status, binding["finality_block_hash"].as_str()) {
            return Err(cognition_validation_error("runtime_binding_invalid"));
        }
        let base_world_hash = self.current_state_root_hash()?;
        let runtime_manifest_hash = self.current_manifest_hash()?;
        RuntimeCognitionBaseBindingV1 {
            world_id: world_id.to_string(),
            branch_id: branch_id.to_string(),
            finality_epoch,
            finality_block_hash: binding["finality_block_hash"].as_str().map(str::to_string),
            finality_status: finality_status.to_string(),
            base_tick: self.state.time,
            base_world_hash: world_state_binding_digest_v1(
                world_id,
                branch_id,
                finality_epoch,
                binding["finality_block_hash"].as_str(),
                finality_status,
                self.state.time,
                &base_world_hash,
                reorg_epoch,
                &runtime_manifest_hash,
            ),
            reorg_epoch,
            runtime_manifest_hash: cognition_digest_v1(
                "oasis7.runtime.manifest.v1",
                &runtime_manifest_hash,
            ),
        }
        .validate()
        .map_err(|error| cognition_validation_error(error.code()))?;
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

    pub fn start_cognition_turn(
        &mut self,
        agent_id: &str,
        agent_session_id: &str,
        agent_turn_id: &str,
        decision_request_id: &str,
        request_digest: &str,
    ) -> Result<(), WorldError> {
        let bounded = |value: &str| !value.trim().is_empty() && value.len() <= 256;
        if self.cognition_runtime_is_unbound()
            || !bounded(agent_id)
            || !bounded(agent_session_id)
            || !bounded(agent_turn_id)
            || !bounded(decision_request_id)
            || !bounded(request_digest)
        {
            return Err(cognition_validation_error("cognition_turn_invalid"));
        }
        if !self.state.agents.is_empty() && !self.state.agents.contains_key(agent_id) {
            return Err(cognition_validation_error("cognition_agent_missing"));
        }
        let mut transaction = self.clone();
        append_cognition_event(
            &mut transaction.cognition,
            "TurnStarted",
            json!({
                "agent_id": agent_id,
                "agent_session_id": agent_session_id,
                "agent_turn_id": agent_turn_id,
                "decision_request_id": decision_request_id,
                "request_digest": request_digest,
            }),
        )?;
        transaction.persist_runtime_transaction_if_configured()?;
        *self = transaction;
        Ok(())
    }

    pub fn enqueue_cognition_wake(
        &mut self,
        wake: SchedulerWakeV1,
    ) -> Result<SchedulerEnqueueOutcome, WorldError> {
        let mut transaction = self.clone();
        let outcome = transaction.enqueue_cognition_wake_inner(wake)?;
        transaction.persist_runtime_transaction_if_configured()?;
        *self = transaction;
        Ok(outcome)
    }

    fn enqueue_cognition_wake_inner(
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

    #[cfg(test)]
    pub(crate) fn enqueue_cognition_wake_for_test(
        &mut self,
        wake: SchedulerWakeV1,
    ) -> Result<SchedulerEnqueueOutcome, WorldError> {
        let mut transaction = self.clone();
        let mut scheduler = transaction.cognition_scheduler()?;
        scheduler.advance_logical_tick(transaction.state.time);
        let outcome = scheduler
            .try_enqueue(wake.clone())
            .map_err(|error| scheduler_error(error.code()))?;
        transaction.cognition_commit_scheduler_transaction(
            &scheduler,
            "SchedulerWakeEnqueued",
            Some(&wake),
        )?;
        *self = transaction;
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
            self.cognition_wake_conditions_ready(wake, &continuations, tick)
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
        transaction.persist_runtime_transaction_if_configured()?;
        *self = transaction;
        Ok(selected)
    }

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

    pub fn release_cognition_wake(&mut self, wake_id: &str) -> Result<SchedulerWakeV1, WorldError> {
        if wake_id.trim().is_empty() {
            return Err(scheduler_error("in_flight_wake_missing"));
        }
        if let Some(contexts) = self
            .cognition
            .get("continuation_contexts")
            .and_then(JsonValue::as_object)
        {
            let scheduler = self.cognition_scheduler()?;
            if let Some(wake) = scheduler
                .in_flight_wakes()
                .into_iter()
                .find(|wake| wake.wake_id == wake_id)
                && contexts.contains_key(&wake.continuation_id)
            {
                return Err(scheduler_error("wake_disposition_required"));
            }
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
        let recovered = scheduler.recover_capacity_preserving_cursor(self.state.time);
        for recovered_wake in &recovered {
            transaction.cognition_commit_scheduler_transaction(
                &scheduler,
                "SchedulerWakeRecovered",
                Some(recovered_wake),
            )?;
        }
        transaction.persist_runtime_transaction_if_configured()?;
        *self = transaction;
        Ok(wake)
    }

    pub fn recover_cognition_scheduler(
        &mut self,
        tick: u64,
    ) -> Result<Vec<SchedulerWakeV1>, WorldError> {
        let mut transaction = self.clone();
        let mut scheduler = transaction.cognition_scheduler()?;
        let mut recovered = scheduler.recover_in_flight_preserving_cursor(tick);
        recovered.extend(scheduler.recover_capacity_preserving_cursor(tick));
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
        transaction.persist_runtime_transaction_if_configured()?;
        *self = transaction;
        Ok(recovered)
    }

    pub fn cognition_scheduler_snapshot(&self) -> JsonValue {
        self.cognition
            .get("scheduler_state")
            .cloned()
            .unwrap_or(JsonValue::Null)
    }

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

    pub fn consume_cognition_wake<F>(
        &mut self,
        wake_id: &str,
        dispatch: F,
    ) -> Result<SchedulerWakeV1, WorldError>
    where
        F: FnOnce(&SchedulerWakeV1) -> Result<CognitionWakeDispositionV1, WorldError>,
    {
        let wake = self
            .cognition_in_flight_wakes()?
            .into_iter()
            .find(|wake| wake.wake_id == wake_id)
            .ok_or_else(|| scheduler_error("in_flight_wake_missing"))?;
        let disposition = dispatch(&wake)?;
        Ok(self.handoff_cognition_wake(wake_id, disposition)?.wake)
    }

    #[cfg(test)]
    pub(crate) fn install_cognition_continuation_for_test(
        &mut self,
        continuation: AgentContinuation,
    ) -> Result<(), WorldError> {
        let mut continuations = self.cognition_continuations_typed()?;
        continuations.push(continuation);
        self.cognition_set_continuations(&continuations)
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
        self.evaluate_cognition_wake_at_tick(conditions, self.state.time)
    }

    fn evaluate_cognition_wake_at_tick(
        &self,
        conditions: &[WakeConditionV1],
        tick: u64,
    ) -> Result<WakeEvaluation, WorldError> {
        let head_digest = self.current_state_root_hash()?;
        let (event_digests, receipt_ids) = self.cognition_committed_evidence()?;
        let mut context = WakeEvaluationContext::at(tick)
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

    pub fn admit_cognition_continuation(
        &mut self,
        proposal: CognitionContinuationProposalV1,
    ) -> Result<AgentContinuation, WorldError> {
        let mut transaction = self.clone();
        let admitted = transaction.admit_cognition_continuation_inner(proposal, false)?;
        transaction.persist_runtime_transaction_if_configured()?;
        *self = transaction;
        Ok(admitted)
    }

    pub(in crate::runtime::world) fn admit_cognition_continuation_inner(
        &mut self,
        mut proposal: CognitionContinuationProposalV1,
        allow_existing_budget_chain: bool,
    ) -> Result<AgentContinuation, WorldError> {
        if self.cognition_runtime_is_unbound() {
            return Err(cognition_validation_error("runtime_binding_required"));
        }
        self.bind_cognition_proposal_fields(&mut proposal)?;
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
        if !self.cognition_turn_is_registered(
            &proposal.agent_id,
            &proposal.agent_session_id,
            &proposal.agent_turn_id,
            &proposal.decision_request_id,
            &proposal.origin_request_digest,
        ) {
            return Err(cognition_validation_error("continuation_turn_unregistered"));
        }
        if continuations
            .iter()
            .any(|existing| existing.continuation_proposal_id == proposal.continuation_proposal_id)
        {
            return Err(cognition_validation_error("continuation_proposal_conflict"));
        }
        if !allow_existing_budget_chain
            && continuations.iter().any(|existing| {
                existing.agent_id == proposal.agent_id
                    && existing.agent_session_id == proposal.agent_session_id
                    && existing.agent_turn_id == proposal.agent_turn_id
                    && existing.decision_request_id == proposal.decision_request_id
            })
        {
            return Err(cognition_validation_error(
                "continuation_budget_chain_conflict",
            ));
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
        let proposal_context = json!({
            "baseline_observation_digest": proposal.baseline_observation_digest.clone(),
            "goal_digest": proposal.goal_digest.clone(),
            "policy_digest": proposal.policy_digest.clone(),
            "policy_revision": proposal.policy_revision,
            "precondition_summary": proposal.precondition_summary.clone(),
            "precondition_digest": proposal.precondition_digest.clone(),
        });
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
        let mut context_registry = self
            .cognition
            .get("continuation_contexts")
            .and_then(JsonValue::as_object)
            .cloned()
            .unwrap_or_default();
        context_registry.insert(continuation.continuation_id.clone(), proposal_context);
        self.cognition["continuation_contexts"] = JsonValue::Object(context_registry);
        self.cognition_commit_continuation_transaction(&continuations, &scheduler, &wake)?;
        Ok(continuation)
    }

    pub fn schedule_cognition_continuation(
        &mut self,
        _continuation: AgentContinuation,
    ) -> Result<(), WorldError> {
        Err(cognition_validation_error("legacy_continuation_projection"))
    }

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
        transaction.persist_runtime_transaction_if_configured()?;
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
        transaction.persist_runtime_transaction_if_configured()?;
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
            let world_subject = PreconditionSubjectV1 {
                kind: "world".to_string(),
                id: world_id.clone().unwrap_or_default(),
            };
            context = context
                .with_subject_predicate_value(
                    &world_subject,
                    "world.logical_tick",
                    &serde_cbor::to_vec(&self.state.time).map_err(WorldError::from)?,
                )
                .with_subject_predicate_value(
                    &world_subject,
                    "world.reorg_epoch",
                    &serde_cbor::to_vec(&self.cognition_reorg_epoch()).map_err(WorldError::from)?,
                );
            let state_root = self.current_state_root_hash()?;
            context = context.with_subject_predicate_value(
                &world_subject,
                "world.state_root",
                &serde_cbor::to_vec(&state_root).map_err(WorldError::from)?,
            );
            let manifest_hash = self.current_manifest_hash()?;
            context = context.with_subject_predicate_value(
                &world_subject,
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
                    context = context.with_subject_predicate_value(subject, path, &value);
                }
                ("intent", "intent.status") => {
                    let Some(intent) = self.state.agent_intent_ledger.get(&subject.id) else {
                        continue;
                    };
                    let value = serde_cbor::to_vec(&intent.status).map_err(WorldError::from)?;
                    context = context.with_subject_predicate_value(subject, path, &value);
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

    pub(in crate::runtime::world) fn cognition_scheduler(
        &self,
    ) -> Result<CognitionScheduler, WorldError> {
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
            return false;
        };
        if let Some(contexts) = self
            .cognition
            .get("continuation_contexts")
            .and_then(JsonValue::as_object)
            && !contexts.contains_key(&continuation.continuation_id)
        {
            return false;
        }
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
        tick: u64,
    ) -> bool {
        let Some(continuation) = continuations
            .iter()
            .find(|continuation| continuation.continuation_id == wake.continuation_id)
        else {
            return false;
        };
        if !self.cognition_wake_has_active_continuation(wake, continuations) {
            return false;
        }
        self.evaluate_cognition_wake_at_tick(&continuation.wake_conditions, tick)
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
            return Err(cognition_validation_error("foreign_scheduler_wake"));
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

    pub(super) fn cognition_runtime_is_unbound(&self) -> bool {
        self.cognition
            .get("runtime_binding")
            .is_none_or(JsonValue::is_null)
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
