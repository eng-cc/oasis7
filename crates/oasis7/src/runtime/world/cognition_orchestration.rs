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
        let mut scheduler = self.cognition_scheduler()?;
        let selected = scheduler.select_ready(tick);
        self.cognition_commit_scheduler_transaction(&scheduler, "SchedulerWakeSelected", None)?;
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
        self.cognition_commit_scheduler_transaction(&scheduler, "SchedulerWakeRecovered", None)?;
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
        let head_digest = self.current_state_root_hash()?;
        self.evaluate_cognition_wake_with_context(
            conditions,
            WakeEvaluationContext::at(self.state.time)
                .with_evaluation_head(&head_digest)
                .with_reorg_epoch(self.cognition_reorg_epoch()),
        )
    }

    pub fn evaluate_cognition_wake_from_committed_projection(
        &self,
        conditions: &[WakeConditionV1],
        event_digest: &str,
        receipt_id: &str,
    ) -> Result<WakeEvaluation, WorldError> {
        let head_digest = self.current_state_root_hash()?;
        let context = WakeEvaluationContext::at(self.state.time)
            .with_event(event_digest)
            .with_receipt(receipt_id)
            .with_predicate_u64("world.logical_tick", self.state.time)
            .with_reorg_epoch(self.cognition_reorg_epoch())
            .with_evaluation_head(&head_digest);
        self.evaluate_cognition_wake_with_context(conditions, context)
    }

    /// Admit an agent-owned continuation proposal at the Runtime boundary.
    /// Identity, sequence, status and digest fields are allocated here; none
    /// are accepted from a caller-owned wire projection.
    pub fn admit_cognition_continuation(
        &mut self,
        mut proposal: CognitionContinuationProposalV1,
    ) -> Result<AgentContinuation, WorldError> {
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
        let next_wake_tick = continuation
            .next_wake_tick
            .ok_or_else(|| cognition_validation_error("continuation_wake_tick_missing"))?;
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
        mut continuation: AgentContinuation,
    ) -> Result<(), WorldError> {
        if continuation.schema_version != CONTINUATION_SCHEMA {
            return Err(cognition_validation_error("continuation_schema_mismatch"));
        }
        if continuation.continuation_status_digest.is_none() {
            return Err(cognition_validation_error("legacy_continuation_projection"));
        }
        WakeConditionValidator::validate(continuation.wake_conditions.as_slice())
            .map_err(|error| cognition_validation_error(error.code()))?;
        let derived_next_wake_tick = WakeConditionValidator::next_wake_tick_at(
            &continuation.wake_conditions,
            self.state.time,
        )
        .map_err(|error| cognition_validation_error(error.code()))?;
        if continuation.next_wake_tick != derived_next_wake_tick {
            return Err(cognition_validation_error(
                "continuation_wake_tick_mismatch",
            ));
        }
        // Legacy adapters may omit the Runtime digest.  Recompute it from
        // the full projection before persistence so the stored record is
        // authoritative rather than caller-signed.
        continuation.refresh_status_digest();
        continuation
            .validate_authoritative()
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

    /// Apply one Runtime-owned continuation transition and persist its
    /// refreshed status digest in the same World projection update.
    pub fn transition_cognition_continuation(
        &mut self,
        continuation_id: &str,
        to: ContinuationStatusV1,
        logical_tick: u64,
    ) -> Result<AgentContinuation, WorldError> {
        let mut continuations = self.cognition_continuations_typed()?;
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
        self.cognition_set_continuations(&continuations)?;
        Ok(transitioned)
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
            ContinuationTransition::invalidate_for_reorg_at_tick(
                continuation,
                reorg_epoch,
                self.state.time,
            )
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
        let context = self.enrich_cognition_wake_context(conditions, context)?;
        WakeConditionValidator::evaluate(conditions, &context)
            .map_err(|error| cognition_validation_error(error.code()))
    }

    fn enrich_cognition_wake_context(
        &self,
        conditions: &[WakeConditionV1],
        mut context: WakeEvaluationContext,
    ) -> Result<WakeEvaluationContext, WorldError> {
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

    fn validate_cognition_wake_binding(&self, wake: &SchedulerWakeV1) -> Result<(), WorldError> {
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
        if let Some(continuation) = continuations
            .iter()
            .find(|continuation| continuation.continuation_id == wake.continuation_id)
            && (continuation.wake_id != wake.wake_id
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
                || continuation.decision_request_id != wake.decision_request_id)
        {
            return Err(cognition_validation_error("foreign_scheduler_wake"));
        }
        Ok(())
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
