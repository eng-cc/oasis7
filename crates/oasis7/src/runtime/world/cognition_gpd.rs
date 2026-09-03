//! Typed Runtime handoff seams for gameplay-reviewed continuation invariants.
//!
//! The agent/provider owns the proposed observation, goal and policy values;
//! World owns their admitted digest, budget chain, wake lease and terminal
//! disposition. These methods keep that boundary explicit and transactional.

use super::World;
use crate::runtime::cognition_wake::{
    AgentContinuation, CognitionBudgetConsumptionV1, CognitionContextDigestsV1,
    CognitionContinuationProposalV1, CognitionWakeDispositionV1, CognitionWakeHandoffResultV1,
    ContinuationStatusV1, ContinuationTransition, WakeConditionValidator,
};
use crate::runtime::error::WorldError;
use serde_json::Value as JsonValue;

fn handoff_error(code: &str) -> WorldError {
    WorldError::DistributedValidationFailed {
        reason: format!("cognition handoff failed: {code}"),
    }
}

fn terminal_status(status: ContinuationStatusV1) -> bool {
    matches!(
        status,
        ContinuationStatusV1::Completed
            | ContinuationStatusV1::Cancelled
            | ContinuationStatusV1::Invalidated
            | ContinuationStatusV1::Expired
            | ContinuationStatusV1::Rejected
    )
}

fn transition_terminal(
    continuation: &mut AgentContinuation,
    status: ContinuationStatusV1,
    logical_tick: u64,
) -> Result<(), WorldError> {
    if status == ContinuationStatusV1::Completed {
        if !matches!(continuation.status, ContinuationStatusV1::Consumed) {
            if !matches!(continuation.status, ContinuationStatusV1::Waking) {
                ContinuationTransition::apply_at_tick(
                    continuation,
                    ContinuationStatusV1::Waking,
                    logical_tick,
                )
                .map_err(|error| handoff_error(error.code()))?;
            }
            ContinuationTransition::apply_at_tick(
                continuation,
                ContinuationStatusV1::Consumed,
                logical_tick,
            )
            .map_err(|error| handoff_error(error.code()))?;
        }
    }
    ContinuationTransition::apply_at_tick(continuation, status, logical_tick)
        .map_err(|error| handoff_error(error.code()))
}

fn set_continuations(
    world: &mut World,
    continuations: &[AgentContinuation],
) -> Result<(), WorldError> {
    let mut projection = world.cognition.as_object().cloned().unwrap_or_default();
    projection.insert(
        "continuations".to_string(),
        serde_json::to_value(continuations).map_err(WorldError::from)?,
    );
    world.cognition = JsonValue::Object(projection);
    Ok(())
}

fn context_registry_entry(world: &World, continuation_id: &str) -> Option<JsonValue> {
    world
        .cognition
        .get("continuation_contexts")
        .and_then(JsonValue::as_object)
        .and_then(|contexts| contexts.get(continuation_id))
        .cloned()
}

fn proposal_context_matches(entry: &JsonValue, proposal: &CognitionContinuationProposalV1) -> bool {
    entry
        .get("baseline_observation_digest")
        .and_then(JsonValue::as_str)
        == Some(proposal.baseline_observation_digest.as_str())
        && entry.get("goal_digest").and_then(JsonValue::as_str)
            == Some(proposal.goal_digest.as_str())
        && entry.get("policy_digest").and_then(JsonValue::as_str)
            == Some(proposal.policy_digest.as_str())
        && entry.get("policy_revision").and_then(JsonValue::as_u64)
            == Some(proposal.policy_revision)
        && entry
            .get("precondition_summary")
            .and_then(JsonValue::as_str)
            == Some(proposal.precondition_summary.as_str())
        && entry.get("precondition_digest").and_then(JsonValue::as_str)
            == Some(proposal.precondition_digest.as_str())
}

impl World {
    /// Verify all proposal context digests and its full canonical proposal
    /// identity against the durable continuation. This is the typed seam for
    /// admission/wake revalidation; Runtime intentionally has no product-level
    /// source from which to invent a replacement goal or policy.
    pub fn validate_cognition_context_digests(
        &self,
        continuation_id: &str,
        proposal: &CognitionContinuationProposalV1,
    ) -> Result<CognitionContextDigestsV1, WorldError> {
        let context = CognitionContextDigestsV1::from_proposal(proposal);
        context
            .validate()
            .map_err(|error| handoff_error(error.code()))?;
        proposal
            .validate()
            .map_err(|error| handoff_error(error.code()))?;
        let continuation = self
            .cognition_continuations_typed()?
            .into_iter()
            .find(|value| value.continuation_id == continuation_id)
            .ok_or_else(|| handoff_error("continuation_missing"))?;
        let context_entry = context_registry_entry(self, continuation_id)
            .ok_or_else(|| handoff_error("cognition_context_missing"))?;
        if !proposal_context_matches(&context_entry, proposal) {
            return Err(handoff_error("cognition_context_mismatch"));
        }
        let binding = self.current_cognition_runtime_binding()?;
        let binding_block_hash = binding
            .finality_block_hash
            .as_ref()
            .map(ToString::to_string);
        let proposal_matches_current = proposal.world_id == binding.world_id
            && proposal.branch_id == binding.branch_id
            && proposal.finality_epoch == binding.finality_epoch
            && proposal.finality_block_hash == binding_block_hash
            && proposal.finality_status == binding.finality_status
            && proposal.reorg_epoch == binding.reorg_epoch
            && proposal.runtime_manifest_hash == self.current_manifest_hash()?;
        let wake_conditions =
            WakeConditionValidator::canonicalize(proposal.wake_conditions.clone())
                .map_err(|error| handoff_error(error.code()))?;
        let continuation_matches = continuation.proposal_digest == proposal.proposal_digest
            && continuation.world_id == proposal.world_id
            && continuation.branch_id == proposal.branch_id
            && continuation.finality_epoch == proposal.finality_epoch
            && continuation.finality_block_hash == proposal.finality_block_hash
            && continuation.finality_status == proposal.finality_status
            && continuation.reorg_epoch == proposal.reorg_epoch
            && continuation.runtime_manifest_hash == proposal.runtime_manifest_hash
            && continuation.agent_id == proposal.agent_id
            && continuation.agent_session_id == proposal.agent_session_id
            && continuation.agent_turn_id == proposal.agent_turn_id
            && continuation.decision_request_id == proposal.decision_request_id
            && continuation.origin_turn_id == proposal.origin_turn_id
            && continuation.origin_request_digest == proposal.origin_request_digest
            && continuation.continuation_proposal_id == proposal.continuation_proposal_id
            && continuation.action_or_envelope_digest == proposal.action_or_envelope_digest
            && continuation.wake_conditions == wake_conditions
            && continuation.next_wake_tick == proposal.next_wake_tick
            && continuation.remaining_budget == proposal.remaining_budget
            && continuation.valid_until_tick == proposal.valid_until_tick
            && continuation.precondition_digest == proposal.precondition_digest;
        if !proposal_matches_current || !continuation_matches {
            return Err(handoff_error("cognition_context_mismatch"));
        }
        Ok(context)
    }

    /// Consume a positive amount from a leased wake's remaining budget. The
    /// lease is deactivated in the same World transaction, and exhaustion is
    /// represented by a terminal Completed continuation with zero budget.
    pub fn consume_cognition_continuation_budget(
        &mut self,
        continuation_id: &str,
        amount: u64,
    ) -> Result<CognitionBudgetConsumptionV1, WorldError> {
        if amount == 0 {
            return Err(handoff_error("continuation_budget_amount_invalid"));
        }
        let mut transaction = self.clone();
        let mut scheduler = transaction.cognition_scheduler()?;
        let wake = scheduler
            .in_flight_wakes()
            .into_iter()
            .find(|wake| wake.continuation_id == continuation_id)
            .ok_or_else(|| handoff_error("wake_not_in_flight"))?;
        transaction.validate_cognition_wake_binding(&wake)?;
        let mut continuations = transaction.cognition_continuations_typed()?;
        let continuation = continuations
            .iter_mut()
            .find(|value| value.continuation_id == continuation_id)
            .ok_or_else(|| handoff_error("continuation_missing"))?;
        if amount > continuation.remaining_budget.value {
            return Err(handoff_error("continuation_budget_exhausted"));
        }
        if !matches!(
            continuation.status,
            ContinuationStatusV1::Scheduled
                | ContinuationStatusV1::Pending
                | ContinuationStatusV1::Waking
        ) {
            return Err(handoff_error("continuation_terminal"));
        }
        if !matches!(continuation.status, ContinuationStatusV1::Waking) {
            ContinuationTransition::apply_at_tick(
                continuation,
                ContinuationStatusV1::Waking,
                transaction.state.time,
            )
            .map_err(|error| handoff_error(error.code()))?;
        }
        continuation.remaining_budget.value -= amount;
        ContinuationTransition::apply_at_tick(
            continuation,
            ContinuationStatusV1::Consumed,
            transaction.state.time,
        )
        .map_err(|error| handoff_error(error.code()))?;
        if continuation.remaining_budget.value == 0 {
            transition_terminal(
                continuation,
                ContinuationStatusV1::Completed,
                transaction.state.time,
            )?;
        }
        if continuation.remaining_budget.value == 0 {
            continuation.terminal_disposition = Some("budget_exhausted".to_string());
            continuation.refresh_status_digest();
        }
        let consumed = continuation.clone();
        let deactivated = scheduler
            .deactivate_wake(&wake.wake_id)
            .map_err(|error| handoff_error(error.code()))?;
        transaction.cognition_commit_continuation_lifecycle_transaction(
            &continuations,
            &scheduler,
            if consumed.status == ContinuationStatusV1::Completed {
                "ContinuationCompleted"
            } else {
                "ContinuationBudgetConsumed"
            },
            &consumed,
            deactivated.as_ref(),
        )?;
        transaction.persist_runtime_transaction_if_configured()?;
        *self = transaction;
        Ok(CognitionBudgetConsumptionV1 {
            continuation_id: consumed.continuation_id,
            wake_id: consumed.wake_id,
            consumed: amount,
            remaining_budget: consumed.remaining_budget,
            status: consumed.status,
            continuation_status_digest: consumed
                .continuation_status_digest
                .ok_or_else(|| handoff_error("continuation_status_digest_missing"))?,
        })
    }

    /// Atomically hand a selected wake to either a terminal disposition or a
    /// fully validated next proposal. Replan budgets must be monotonic and
    /// the next proposal is admitted through the same Runtime gate.
    pub fn handoff_cognition_wake(
        &mut self,
        wake_id: &str,
        disposition: CognitionWakeDispositionV1,
    ) -> Result<CognitionWakeHandoffResultV1, WorldError> {
        let mut transaction = self.clone();
        let mut scheduler = transaction.cognition_scheduler()?;
        let wake = scheduler
            .in_flight_wakes()
            .into_iter()
            .find(|wake| wake.wake_id == wake_id)
            .ok_or_else(|| handoff_error("wake_not_in_flight"))?;
        transaction.validate_cognition_wake_binding(&wake)?;
        let mut continuations = transaction.cognition_continuations_typed()?;
        let continuation = continuations
            .iter_mut()
            .find(|value| value.continuation_id == wake.continuation_id)
            .ok_or_else(|| handoff_error("continuation_missing"))?;
        if !matches!(
            continuation.status,
            ContinuationStatusV1::Scheduled
                | ContinuationStatusV1::Pending
                | ContinuationStatusV1::Waking
        ) {
            return Err(handoff_error("continuation_terminal"));
        }
        let (event_kind, replanned_proposal) = match disposition {
            CognitionWakeDispositionV1::Terminal { status, reason } => {
                if !terminal_status(status) || reason.trim().is_empty() || reason.len() > 128 {
                    return Err(handoff_error("wake_terminal_disposition_invalid"));
                }
                transition_terminal(continuation, status, transaction.state.time)?;
                continuation.terminal_disposition = Some(reason);
                continuation.refresh_status_digest();
                ("ContinuationHandoffTerminal", None)
            }
            CognitionWakeDispositionV1::Replan {
                proposal,
                budget_spent,
            } => {
                if budget_spent == 0 || budget_spent > continuation.remaining_budget.value {
                    return Err(handoff_error("continuation_budget_exhausted"));
                }
                let remaining = continuation.remaining_budget.value - budget_spent;
                if proposal.remaining_budget.unit != continuation.remaining_budget.unit
                    || proposal.remaining_budget.value > remaining
                {
                    return Err(handoff_error("continuation_budget_non_monotonic"));
                }
                let Some(context_entry) =
                    context_registry_entry(&transaction, &wake.continuation_id)
                else {
                    return Err(handoff_error("cognition_context_missing"));
                };
                if !proposal_context_matches(&context_entry, &proposal) {
                    return Err(handoff_error("cognition_context_mismatch"));
                }
                if !matches!(continuation.status, ContinuationStatusV1::Waking) {
                    ContinuationTransition::apply_at_tick(
                        continuation,
                        ContinuationStatusV1::Waking,
                        transaction.state.time,
                    )
                    .map_err(|error| handoff_error(error.code()))?;
                }
                continuation.remaining_budget.value = remaining;
                ContinuationTransition::apply_at_tick(
                    continuation,
                    ContinuationStatusV1::Consumed,
                    transaction.state.time,
                )
                .map_err(|error| handoff_error(error.code()))?;
                ("ContinuationReplanned", Some(proposal))
            }
        };
        let transitioned = continuation.clone();
        let deactivated = scheduler
            .deactivate_wake(&wake.wake_id)
            .map_err(|error| handoff_error(error.code()))?;
        set_continuations(&mut transaction, &continuations)?;
        let replanned_continuation = if let Some(proposal) = replanned_proposal {
            transaction.cognition["scheduler_state"] = scheduler.snapshot_json();
            Some(transaction.admit_cognition_continuation_inner(proposal, true)?)
        } else {
            transaction.cognition["scheduler_state"] = scheduler.snapshot_json();
            None
        };
        let final_scheduler = transaction.cognition_scheduler()?;
        let final_continuations = transaction.cognition_continuations_typed()?;
        transaction.cognition_commit_continuation_lifecycle_transaction(
            &final_continuations,
            &final_scheduler,
            event_kind,
            &transitioned,
            deactivated.as_ref(),
        )?;
        transaction.persist_runtime_transaction_if_configured()?;
        *self = transaction;
        Ok(CognitionWakeHandoffResultV1 {
            wake,
            continuation: transitioned,
            replanned_continuation,
        })
    }
}
