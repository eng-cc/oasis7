//! Simulator continuation policy and Runtime projection seams.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::super::continuous_agent_harness::{CognitionError, Digest32, h_v1};

const CONTINUATION_PROPOSAL_DOMAIN: &str = "oasis7.cognition.continuation-proposal.v1";
const MAX_WAKE_CONDITIONS: usize = 16;
const MAX_WAKE_ITEM_BYTES: usize = 768;
const MAX_WAKE_LIST_BYTES: usize = 4096;

fn error(code: &'static str, message: impl Into<String>) -> CognitionError {
    CognitionError::new(code, message)
}

// ---------------------------------------------------------------------------
// Harness continuation proposal and Runtime status projection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuationBudgetV1 {
    pub unit: String,
    pub value: u64,
}

/// The current authoritative cognition snapshot used when admitting or
/// waking a continuation.  A proposal carries the snapshot it was derived
/// from; this value is supplied by the Runtime-facing host at the boundary.
/// Keeping the comparison in the Harness makes stale proposals fail before a
/// provider invocation, while Runtime remains the owner of world truth and
/// action effects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuationAuthorityContextV1 {
    pub baseline_observation_digest: String,
    pub goal_digest: String,
    pub policy_digest: String,
    pub precondition_digest: String,
}

impl ContinuationAuthorityContextV1 {
    pub fn validate(&self) -> Result<(), CognitionError> {
        for (name, value) in [
            (
                "baseline_observation_digest",
                &self.baseline_observation_digest,
            ),
            ("goal_digest", &self.goal_digest),
            ("policy_digest", &self.policy_digest),
            ("precondition_digest", &self.precondition_digest),
        ] {
            if value.trim().is_empty() || value.len() > 512 {
                return Err(error(
                    "continuation_context_invalid",
                    format!("{name} is required and bounded"),
                ));
            }
        }
        Ok(())
    }

    pub fn validate_proposal(
        &self,
        proposal: &ContinuationProposalV1,
    ) -> Result<(), CognitionError> {
        self.validate()?;
        if self.baseline_observation_digest != proposal.baseline_observation_digest
            || self.goal_digest != proposal.goal_digest
            || self.policy_digest != proposal.policy_digest
            || self.precondition_digest != proposal.precondition_digest
        {
            return Err(error(
                "continuation_context_stale",
                "continuation lineage no longer matches the authoritative cognition context",
            ));
        }
        Ok(())
    }
}

/// Durable-budget projection returned by the Harness after one wake delivery.
/// The Runtime is still the durable owner; this value is intentionally an
/// audit/coordination projection and never a world-effect receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuationBudgetProgressV1 {
    pub chain_id: String,
    pub wake_id: String,
    pub unit: String,
    pub consumed: u64,
    pub remaining: u64,
    pub exhausted: bool,
    pub duplicate: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_disposition: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WakeConditionSubjectV1 {
    pub kind: String,
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WakeConditionV1 {
    pub schema_version: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logical_tick: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipt_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<WakeConditionSubjectV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_or_rule: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operator: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_value_bytes: Option<Vec<u8>>,
}

impl WakeConditionV1 {
    fn canonical_bytes(&self) -> Vec<u8> {
        oasis7_wasm_abi::encode_canonical_cbor(self).expect("wake condition is canonicalizable")
    }
}

fn validate_wake_conditions(conditions: &[WakeConditionV1]) -> Result<(), CognitionError> {
    if conditions.is_empty() {
        return Err(error(
            "wake_conditions_empty",
            "continuation requires a bounded non-empty wake condition list",
        ));
    }
    if conditions.len() > MAX_WAKE_CONDITIONS {
        return Err(error(
            "continuation_wake_invalid",
            "continuation wake condition list exceeds its bound",
        ));
    }
    let mut seen = BTreeSet::new();
    let mut total = 0usize;
    let mut previous: Option<Vec<u8>> = None;
    for condition in conditions {
        let valid = match condition.kind.as_str() {
            "at_or_after_tick" => {
                condition.logical_tick.is_some()
                    && condition.event_digest.is_none()
                    && condition.receipt_id.is_none()
                    && condition.subject.is_none()
                    && condition.path_or_rule.is_none()
                    && condition.operator.is_none()
                    && condition.expected_value_bytes.is_none()
            }
            "world_event_committed" => {
                condition
                    .event_digest
                    .as_ref()
                    .is_some_and(|v| !v.is_empty())
                    && condition.logical_tick.is_none()
                    && condition.receipt_id.is_none()
                    && condition.subject.is_none()
                    && condition.path_or_rule.is_none()
                    && condition.operator.is_none()
                    && condition.expected_value_bytes.is_none()
            }
            "receipt_linked" => {
                condition.receipt_id.as_ref().is_some_and(|v| !v.is_empty())
                    && condition.logical_tick.is_none()
                    && condition.event_digest.is_none()
                    && condition.subject.is_none()
                    && condition.path_or_rule.is_none()
                    && condition.operator.is_none()
                    && condition.expected_value_bytes.is_none()
            }
            "state_predicate" => {
                condition.logical_tick.is_none()
                    && condition.event_digest.is_none()
                    && condition.receipt_id.is_none()
                    && condition.subject.is_some()
                    && condition
                        .path_or_rule
                        .as_ref()
                        .is_some_and(|v| !v.is_empty())
                    && condition.operator.as_ref().is_some_and(|v| !v.is_empty())
                    && condition
                        .expected_value_bytes
                        .as_ref()
                        .is_some_and(|v| v.len() <= 512)
            }
            _ => false,
        };
        if condition.schema_version != "wake-condition.v1" || !valid {
            return Err(error("continuation_wake_invalid", "invalid wake condition"));
        }
        let bytes = condition.canonical_bytes();
        if bytes.len() > MAX_WAKE_ITEM_BYTES || !seen.insert(bytes.clone()) {
            return Err(error(
                "continuation_wake_invalid",
                "duplicate or oversized wake condition",
            ));
        }
        if previous.as_ref().is_some_and(|prior| prior > &bytes) {
            return Err(error(
                "continuation_wake_invalid",
                "wake conditions must be sorted by canonical bytes",
            ));
        }
        previous = Some(bytes.clone());
        total += bytes.len();
        if total > MAX_WAKE_LIST_BYTES {
            return Err(error(
                "continuation_wake_invalid",
                "wake condition list is oversized",
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuationProposalV1 {
    pub schema_version: u16,
    pub continuation_proposal_id: String,
    pub world_id: String,
    pub agent_id: String,
    pub agent_session_id: String,
    pub agent_turn_id: String,
    pub decision_request_id: String,
    pub origin_turn_id: String,
    pub origin_request_digest: String,
    pub action_or_plan_kind: String,
    #[serde(default)]
    pub action_or_envelope_digest: Option<String>,
    pub remaining_budget: ContinuationBudgetV1,
    pub baseline_observation_digest: String,
    pub goal_digest: String,
    pub policy_digest: String,
    pub policy_revision: u64,
    pub precondition_summary: String,
    pub precondition_digest: String,
    pub wake_conditions: Vec<WakeConditionV1>,
    #[serde(default)]
    pub valid_until_tick: Option<u64>,
    pub source: String,
    pub proposal_digest: String,
}

impl ContinuationProposalV1 {
    /// Return the complete simulator-owned wire payload Runtime must admit.
    /// Keeping this projection here prevents callers from silently dropping
    /// policy, observation, budget, or wake bindings while adapting to a
    /// Runtime persistence type.
    pub fn runtime_admission_payload(&self) -> Result<Value, CognitionError> {
        self.validate()?;
        serde_json::to_value(self)
            .map_err(|e| error("continuation_admission_encoding_failed", e.to_string()))
    }

    /// Canonical bytes for the Runtime admission seam. Runtime may persist
    /// these bytes or derive its own admission digest without trusting a
    /// provider-supplied serialization.
    pub fn runtime_admission_bytes(&self) -> Result<Vec<u8>, CognitionError> {
        let payload = self.runtime_admission_payload()?;
        oasis7_wasm_abi::encode_canonical_cbor(&payload)
            .map_err(|e| error("continuation_admission_encoding_failed", e.to_string()))
    }

    pub fn runtime_admission_digest(&self) -> Result<Digest32, CognitionError> {
        self.proposal_digest()
    }

    pub fn proposal_digest(&self) -> Result<Digest32, CognitionError> {
        let mut value = serde_json::to_value(self)
            .map_err(|e| error("continuation_canonical_encoding_failed", e.to_string()))?;
        value
            .as_object_mut()
            .expect("continuation proposal is an object")
            .remove("proposal_digest");
        Ok(h_v1(CONTINUATION_PROPOSAL_DOMAIN, &value))
    }

    pub fn validate(&self) -> Result<(), CognitionError> {
        if self.schema_version != 1 {
            return Err(error(
                "continuation_schema_invalid",
                "unsupported proposal version",
            ));
        }
        for (name, value) in [
            ("continuation_proposal_id", &self.continuation_proposal_id),
            ("world_id", &self.world_id),
            ("agent_id", &self.agent_id),
            ("agent_session_id", &self.agent_session_id),
            ("agent_turn_id", &self.agent_turn_id),
            ("decision_request_id", &self.decision_request_id),
            ("origin_turn_id", &self.origin_turn_id),
            ("origin_request_digest", &self.origin_request_digest),
            ("action_or_plan_kind", &self.action_or_plan_kind),
            (
                "baseline_observation_digest",
                &self.baseline_observation_digest,
            ),
            ("goal_digest", &self.goal_digest),
            ("policy_digest", &self.policy_digest),
            ("precondition_digest", &self.precondition_digest),
            ("source", &self.source),
            ("proposal_digest", &self.proposal_digest),
        ] {
            if value.trim().is_empty() {
                return Err(error(
                    "continuation_binding_invalid",
                    format!("{name} is required"),
                ));
            }
        }
        if !matches!(self.remaining_budget.unit.as_str(), "steps" | "ticks")
            || self.remaining_budget.value == 0
        {
            return Err(error(
                "continuation_budget_invalid",
                "continuation budget must be a positive steps or ticks value",
            ));
        }
        validate_wake_conditions(&self.wake_conditions)?;
        let expected = self.proposal_digest()?;
        if expected.as_str() != self.proposal_digest {
            return Err(error(
                "continuation_digest_mismatch",
                "continuation proposal digest does not match canonical fields",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeContinuationStatusV1 {
    pub status: String,
    #[serde(default)]
    pub terminal_disposition: Option<String>,
    pub continuation_id: String,
    pub wake_id: String,
    pub wake_seq: u64,
    pub continuation_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuationHandle {
    pub proposal: ContinuationProposalV1,
    #[serde(default)]
    pub chain_id: String,
    pub continuation_id: String,
    pub wake_id: String,
    pub wake_seq: u64,
    pub continuation_digest: String,
    #[serde(default)]
    pub continuation_status_digest: String,
    pub status: String,
    #[serde(default)]
    pub terminal_disposition: Option<String>,
    pub active: bool,
    pub provenance: String,
    pub world_effect: bool,
    pub provider_invocation_count: u64,
    #[serde(default)]
    pub remaining_budget: ContinuationBudgetV1,
    #[serde(default)]
    pub consumed_budget: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinuationInvalidationReason {
    ObservationChanged,
    GoalChanged,
    PolicyChanged,
    PreconditionChanged,
    BudgetExhausted,
    Rejected,
    Stale,
    Timeout,
    Expired,
    Reorg,
    Cancelled,
}

impl ContinuationInvalidationReason {
    fn status(self) -> &'static str {
        match self {
            Self::ObservationChanged
            | Self::GoalChanged
            | Self::PolicyChanged
            | Self::PreconditionChanged
            | Self::Stale
            | Self::Reorg => "invalidated",
            Self::Rejected => "rejected",
            Self::BudgetExhausted => "expired",
            Self::Timeout | Self::Expired => "expired",
            Self::Cancelled => "cancelled",
        }
    }

    fn reason(self) -> &'static str {
        match self {
            Self::ObservationChanged => "observation_changed",
            Self::GoalChanged => "goal_changed",
            Self::PolicyChanged => "policy_changed",
            Self::PreconditionChanged => "precondition_changed",
            Self::BudgetExhausted => "budget_exhausted",
            Self::Rejected => "rejected",
            Self::Stale => "stale",
            Self::Timeout => "timeout",
            Self::Expired => "expired",
            Self::Reorg => "reorg",
            Self::Cancelled => "cancelled",
        }
    }
}

pub type ContinuationProjectionV1 = ContinuationHandle;

#[derive(Debug, Clone, Default)]
pub struct ContinuationHarness {
    active: BTreeMap<String, ContinuationProposalV1>,
    chains: BTreeMap<String, ContinuationChainState>,
    deliveries: BTreeMap<(String, String), ContinuationBudgetProgressV1>,
}

#[derive(Debug, Clone)]
struct ContinuationChainState {
    unit: String,
    remaining: u64,
    consumed: u64,
    terminal_disposition: Option<String>,
}

impl ContinuationHarness {
    fn chain_id(proposal: &ContinuationProposalV1) -> String {
        h_v1(
            "oasis7.cognition.continuation-chain.v1",
            &json!({
                "world_id": proposal.world_id,
                "agent_id": proposal.agent_id,
                "agent_session_id": proposal.agent_session_id,
                "origin_turn_id": proposal.origin_turn_id,
                "origin_request_digest": proposal.origin_request_digest,
            }),
        )
        .to_string()
    }

    fn handle_for(
        &self,
        proposal: ContinuationProposalV1,
        chain_id: String,
        status: &str,
        active: bool,
        provenance: &str,
        continuation_id: String,
        wake_id: String,
        wake_seq: u64,
        continuation_digest: String,
        continuation_status_digest: String,
        terminal_disposition: Option<String>,
    ) -> ContinuationHandle {
        let (remaining, consumed) = self
            .chains
            .get(&chain_id)
            .map(|state| {
                (
                    ContinuationBudgetV1 {
                        unit: state.unit.clone(),
                        value: state.remaining,
                    },
                    state.consumed,
                )
            })
            .unwrap_or_else(|| (proposal.remaining_budget.clone(), 0));
        ContinuationHandle {
            proposal,
            chain_id,
            continuation_id,
            wake_id,
            wake_seq,
            continuation_digest,
            continuation_status_digest,
            status: status.to_string(),
            terminal_disposition,
            active,
            provenance: provenance.to_string(),
            world_effect: false,
            provider_invocation_count: 0,
            remaining_budget: remaining,
            consumed_budget: consumed,
        }
    }

    fn submit_inner(
        &mut self,
        proposal: ContinuationProposalV1,
        context: Option<&ContinuationAuthorityContextV1>,
    ) -> Result<ContinuationHandle, CognitionError> {
        proposal.validate()?;
        if let Some(context) = context {
            context.validate_proposal(&proposal)?;
        }
        let chain_id = Self::chain_id(&proposal);
        if let Some(existing) = self.active.get(&proposal.continuation_proposal_id) {
            if existing.proposal_digest != proposal.proposal_digest {
                return Err(error(
                    "continuation_duplicate",
                    "continuation proposal id was reused with a different digest",
                ));
            }
            return Ok(self.handle_for(
                existing.clone(),
                chain_id,
                "scheduled",
                true,
                "harness_policy",
                String::new(),
                String::new(),
                0,
                String::new(),
                String::new(),
                None,
            ));
        }
        if let Some(state) = self.chains.get(&chain_id) {
            if state.unit != proposal.remaining_budget.unit {
                return Err(error(
                    "continuation_budget_unit_mismatch",
                    "continuation chain cannot change budget units",
                ));
            }
            if state.remaining == 0 {
                return Err(error(
                    "continuation_budget_exhausted",
                    "continuation chain has no remaining budget",
                ));
            }
            if state.terminal_disposition.is_some() {
                return Err(error(
                    "continuation_terminal",
                    "continuation chain has already reached a terminal disposition",
                ));
            }
            if proposal.remaining_budget.value > state.remaining {
                return Err(error(
                    "continuation_budget_increase",
                    "continuation proposal budget exceeds the chain remainder",
                ));
            }
        } else {
            self.chains.insert(
                chain_id.clone(),
                ContinuationChainState {
                    unit: proposal.remaining_budget.unit.clone(),
                    remaining: proposal.remaining_budget.value,
                    consumed: 0,
                    terminal_disposition: None,
                },
            );
        }
        if self.active.values().any(|existing| {
            Self::chain_id(existing) == chain_id
                && existing.continuation_proposal_id != proposal.continuation_proposal_id
        }) {
            return Err(error(
                "continuation_active",
                "a continuation in this cognition chain is already active",
            ));
        }
        self.active
            .insert(proposal.continuation_proposal_id.clone(), proposal.clone());
        Ok(self.handle_for(
            proposal,
            chain_id,
            "scheduled",
            true,
            "harness_policy",
            String::new(),
            String::new(),
            0,
            String::new(),
            String::new(),
            None,
        ))
    }

    pub fn submit(
        &mut self,
        proposal: ContinuationProposalV1,
    ) -> Result<ContinuationHandle, CognitionError> {
        self.submit_inner(proposal, None)
    }

    /// Production continuation admission. The old `submit` method remains a
    /// compatibility fixture lane; production callers must provide the
    /// current Runtime-derived context through this seam.
    pub fn submit_with_context(
        &mut self,
        proposal: ContinuationProposalV1,
        context: &ContinuationAuthorityContextV1,
    ) -> Result<ContinuationHandle, CognitionError> {
        self.submit_inner(proposal, Some(context))
    }

    /// Consume exactly one logical wake delivery (or a bounded number of
    /// units for a tick-based contract). Duplicate delivery of the same wake
    /// is idempotent and cannot debit the chain twice.
    pub fn consume_wake(
        &mut self,
        handle: &mut ContinuationHandle,
        wake_id: &str,
        units: u64,
    ) -> Result<ContinuationBudgetProgressV1, CognitionError> {
        let chain_id = handle.chain_id.clone();
        if chain_id.is_empty() || wake_id.trim().is_empty() {
            return Err(error(
                "continuation_delivery_invalid",
                "a continuation chain and wake identity are required",
            ));
        }
        if let Some(previous) = self
            .deliveries
            .get(&(chain_id.clone(), wake_id.to_string()))
            .cloned()
        {
            handle.remaining_budget = ContinuationBudgetV1 {
                unit: previous.unit.clone(),
                value: previous.remaining,
            };
            handle.consumed_budget = previous.consumed;
            if previous.exhausted {
                handle.active = false;
                handle.status = "expired".to_string();
                handle.terminal_disposition = previous.terminal_disposition.clone();
            }
            let mut duplicate = previous;
            duplicate.duplicate = true;
            return Ok(duplicate);
        }
        if !handle.active
            || !self
                .active
                .contains_key(&handle.proposal.continuation_proposal_id)
        {
            return Err(error(
                "continuation_unknown",
                "continuation handle is not active",
            ));
        }
        if units == 0 {
            return Err(error(
                "continuation_budget_invalid",
                "continuation consumption must be positive",
            ));
        }
        let state = self
            .chains
            .get_mut(&chain_id)
            .ok_or_else(|| error("continuation_unknown", "continuation chain is not active"))?;
        if units > state.remaining {
            return Err(error(
                "continuation_budget_exhausted",
                "continuation delivery exceeds the remaining chain budget",
            ));
        }
        state.remaining -= units;
        state.consumed = state.consumed.saturating_add(units);
        let exhausted = state.remaining == 0;
        if exhausted {
            state.terminal_disposition = Some("budget_exhausted".to_string());
        }
        let progress = ContinuationBudgetProgressV1 {
            chain_id: chain_id.clone(),
            wake_id: wake_id.to_string(),
            unit: state.unit.clone(),
            consumed: state.consumed,
            remaining: state.remaining,
            exhausted,
            duplicate: false,
            terminal_disposition: exhausted.then(|| "budget_exhausted".to_string()),
        };
        self.deliveries
            .insert((chain_id, wake_id.to_string()), progress.clone());
        handle.remaining_budget = ContinuationBudgetV1 {
            unit: progress.unit.clone(),
            value: progress.remaining,
        };
        handle.consumed_budget = progress.consumed;
        if exhausted {
            handle.active = false;
            handle.status = "expired".to_string();
            handle.terminal_disposition = progress.terminal_disposition.clone();
            self.active
                .remove(&handle.proposal.continuation_proposal_id);
        }
        Ok(progress)
    }

    pub fn consume_runtime_status(
        &mut self,
        handle: ContinuationHandle,
        runtime: RuntimeContinuationStatusV1,
    ) -> Result<ContinuationProjectionV1, CognitionError> {
        let _ = (handle, runtime);
        Err(error(
            "continuation_runtime_status_unverified",
            "legacy Runtime status lacks an authoritative continuation projection",
        ))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn reconcile_runtime_budget(
        &mut self,
        proposal: &ContinuationProposalV1,
        runtime: &crate::runtime::AgentContinuation,
    ) -> Result<(String, ContinuationBudgetV1, u64), CognitionError> {
        let chain_id = Self::chain_id(proposal);
        let delivery_key = (chain_id.clone(), runtime.wake_id.clone());
        if let Some(previous) = self.deliveries.get(&delivery_key) {
            if runtime.remaining_budget.unit != previous.unit
                || runtime.remaining_budget.value != previous.remaining
            {
                return Err(error(
                    "continuation_budget_replay_mismatch",
                    "a Runtime wake was replayed with a different remaining budget",
                ));
            }
            return Ok((
                chain_id,
                ContinuationBudgetV1 {
                    unit: previous.unit.clone(),
                    value: previous.remaining,
                },
                previous.consumed,
            ));
        }

        let (progress, remaining, consumed) = {
            let state = self
                .chains
                .get_mut(&chain_id)
                .ok_or_else(|| error("continuation_unknown", "continuation chain is not active"))?;
            if runtime.remaining_budget.unit != state.unit {
                return Err(error(
                    "continuation_budget_unit_mismatch",
                    "Runtime changed the continuation budget unit",
                ));
            }
            if runtime.remaining_budget.value > state.remaining {
                return Err(error(
                    "continuation_budget_increase",
                    "Runtime increased the remaining continuation budget",
                ));
            }
            let delta = state.remaining - runtime.remaining_budget.value;
            state.remaining = runtime.remaining_budget.value;
            state.consumed = state.consumed.saturating_add(delta);
            let exhausted = state.remaining == 0;
            let progress = (delta > 0).then(|| ContinuationBudgetProgressV1 {
                chain_id: chain_id.clone(),
                wake_id: runtime.wake_id.clone(),
                unit: state.unit.clone(),
                consumed: state.consumed,
                remaining: state.remaining,
                exhausted,
                duplicate: false,
                terminal_disposition: if exhausted {
                    runtime
                        .terminal_disposition
                        .clone()
                        .or_else(|| Some("budget_exhausted".to_string()))
                } else {
                    None
                },
            });
            (progress, state.remaining, state.consumed)
        };
        if let Some(progress) = progress {
            self.deliveries.insert(delivery_key, progress);
        }
        Ok((
            chain_id,
            ContinuationBudgetV1 {
                unit: runtime.remaining_budget.unit.clone(),
                value: remaining,
            },
            consumed,
        ))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn consume_runtime_projection(
        &mut self,
        handle: ContinuationHandle,
        runtime: &crate::runtime::AgentContinuation,
    ) -> Result<ContinuationProjectionV1, CognitionError> {
        let proposal = self
            .active
            .get(&handle.proposal.continuation_proposal_id)
            .filter(|candidate| candidate.proposal_digest == handle.proposal.proposal_digest)
            .cloned()
            .ok_or_else(|| error("continuation_unknown", "continuation handle is not active"))?;
        runtime.validate_authoritative().map_err(|runtime_error| {
            error(
                "continuation_runtime_projection_invalid",
                runtime_error.to_string(),
            )
        })?;
        if runtime.continuation_proposal_id != proposal.continuation_proposal_id
            || runtime.proposal_digest != proposal.proposal_digest
            || runtime.world_id != proposal.world_id
            || runtime.agent_id != proposal.agent_id
            || runtime.agent_session_id != proposal.agent_session_id
            || runtime.agent_turn_id != proposal.agent_turn_id
            || runtime.decision_request_id != proposal.decision_request_id
            || runtime.origin_turn_id != proposal.origin_turn_id
            || runtime.origin_request_digest != proposal.origin_request_digest
            || runtime.action_or_envelope_digest != proposal.action_or_envelope_digest
            || runtime.precondition_digest != proposal.precondition_digest
            || runtime.valid_until_tick != proposal.valid_until_tick
        {
            return Err(error(
                "continuation_runtime_correlation_mismatch",
                "Runtime continuation projection does not match the proposal lineage",
            ));
        }
        let (chain_id, remaining_budget, consumed_budget) =
            self.reconcile_runtime_budget(&proposal, runtime)?;
        let (status, active) = match runtime.status {
            crate::runtime::ContinuationStatusV1::Scheduled => ("scheduled", true),
            crate::runtime::ContinuationStatusV1::Pending => ("pending", true),
            crate::runtime::ContinuationStatusV1::Waking => ("waking", true),
            crate::runtime::ContinuationStatusV1::Consumed => ("consumed", true),
            crate::runtime::ContinuationStatusV1::Completed => ("completed", false),
            crate::runtime::ContinuationStatusV1::Cancelled => ("cancelled", false),
            crate::runtime::ContinuationStatusV1::Invalidated => ("invalidated", false),
            crate::runtime::ContinuationStatusV1::Expired => ("expired", false),
            crate::runtime::ContinuationStatusV1::Rejected => ("rejected", false),
        };
        if !active {
            if let Some(state) = self.chains.get_mut(&chain_id) {
                state.terminal_disposition = runtime
                    .terminal_disposition
                    .clone()
                    .or_else(|| Some(status.to_string()));
            }
            self.active.remove(&proposal.continuation_proposal_id);
        }
        Ok(ContinuationHandle {
            proposal,
            chain_id,
            continuation_id: runtime.continuation_id.clone(),
            wake_id: runtime.wake_id.clone(),
            wake_seq: runtime.wake_seq,
            continuation_digest: runtime.continuation_digest(),
            continuation_status_digest: runtime
                .continuation_status_digest
                .clone()
                .expect("validated Runtime continuation has a status digest"),
            status: status.to_string(),
            terminal_disposition: runtime.terminal_disposition.clone(),
            active,
            provenance: "runtime_authoritative".to_string(),
            world_effect: false,
            provider_invocation_count: 0,
            remaining_budget,
            consumed_budget,
        })
    }

    /// Runtime projection consumer for the production wake path. The
    /// compatibility consumer above intentionally accepts only lineage; this
    /// seam also rechecks all four current cognition digests before accepting
    /// a pending/consumed/terminal status update.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn consume_runtime_projection_with_context(
        &mut self,
        handle: ContinuationHandle,
        runtime: &crate::runtime::AgentContinuation,
        context: &ContinuationAuthorityContextV1,
    ) -> Result<ContinuationProjectionV1, CognitionError> {
        let proposal = self
            .active
            .get(&handle.proposal.continuation_proposal_id)
            .filter(|candidate| candidate.proposal_digest == handle.proposal.proposal_digest)
            .cloned()
            .ok_or_else(|| error("continuation_unknown", "continuation handle is not active"))?;
        context.validate_proposal(&proposal)?;
        self.consume_runtime_projection(handle, runtime)
    }

    /// Consume a Runtime-ready wake only after rechecking the current
    /// observation/goal/policy/precondition context and reconciling the
    /// Runtime-owned budget transition. A consumed wake retires the old
    /// proposal; the caller must explicitly submit the next bounded proposal
    /// or terminal disposition, so a lease/event cannot silently become an
    /// action.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn advance_ready_wake(
        &mut self,
        handle: ContinuationHandle,
        runtime: &crate::runtime::AgentContinuation,
        context: &ContinuationAuthorityContextV1,
    ) -> Result<ContinuationHandle, CognitionError> {
        let proposal = self
            .active
            .get(&handle.proposal.continuation_proposal_id)
            .filter(|candidate| candidate.proposal_digest == handle.proposal.proposal_digest)
            .cloned()
            .ok_or_else(|| error("continuation_unknown", "continuation handle is not active"))?;
        context.validate_proposal(&proposal)?;
        runtime.validate_authoritative().map_err(|runtime_error| {
            error(
                "continuation_runtime_projection_invalid",
                runtime_error.to_string(),
            )
        })?;
        if !matches!(
            runtime.status,
            crate::runtime::ContinuationStatusV1::Consumed
                | crate::runtime::ContinuationStatusV1::Completed
                | crate::runtime::ContinuationStatusV1::Cancelled
                | crate::runtime::ContinuationStatusV1::Invalidated
                | crate::runtime::ContinuationStatusV1::Expired
                | crate::runtime::ContinuationStatusV1::Rejected
        ) {
            return Err(error(
                "continuation_wake_not_ready",
                "a pending or waking continuation cannot advance to the next cognition step",
            ));
        }
        if runtime.status == crate::runtime::ContinuationStatusV1::Consumed {
            let chain_id = Self::chain_id(&proposal);
            let delivery_key = (chain_id.clone(), runtime.wake_id.clone());
            let prior = self.deliveries.get(&delivery_key);
            let chain_remaining = self
                .chains
                .get(&chain_id)
                .ok_or_else(|| error("continuation_unknown", "continuation chain is not active"))?
                .remaining;
            if prior.is_none() && runtime.remaining_budget.value == chain_remaining {
                return Err(error(
                    "continuation_budget_not_consumed",
                    "a consumed wake must carry a lower remaining budget",
                ));
            }
        }
        let mut projected = self.consume_runtime_projection(handle, runtime)?;
        if runtime.status == crate::runtime::ContinuationStatusV1::Consumed {
            projected.active = false;
            projected.status = "consumed".to_string();
            self.active
                .remove(&projected.proposal.continuation_proposal_id);
        }
        Ok(projected)
    }

    pub fn invalidate(
        &mut self,
        handle: ContinuationHandle,
        reason: ContinuationInvalidationReason,
    ) -> Result<ContinuationProjectionV1, CognitionError> {
        let proposal = self
            .active
            .get(&handle.proposal.continuation_proposal_id)
            .filter(|candidate| candidate.proposal_digest == handle.proposal.proposal_digest)
            .cloned()
            .ok_or_else(|| error("continuation_unknown", "continuation handle is not active"))?;
        self.active
            .remove(&handle.proposal.continuation_proposal_id);
        let chain_id = Self::chain_id(&proposal);
        if let Some(state) = self.chains.get_mut(&chain_id) {
            state.terminal_disposition = Some(reason.reason().to_string());
        }
        let (remaining_budget, consumed_budget) = self
            .chains
            .get(&chain_id)
            .map(|state| {
                (
                    ContinuationBudgetV1 {
                        unit: state.unit.clone(),
                        value: state.remaining,
                    },
                    state.consumed,
                )
            })
            .unwrap_or_else(|| (proposal.remaining_budget.clone(), 0));
        Ok(ContinuationHandle {
            proposal,
            chain_id,
            continuation_id: String::new(),
            wake_id: String::new(),
            wake_seq: 0,
            continuation_digest: String::new(),
            continuation_status_digest: String::new(),
            status: reason.status().to_string(),
            terminal_disposition: Some(reason.reason().to_string()),
            active: false,
            provenance: "harness_policy".to_string(),
            world_effect: false,
            provider_invocation_count: 0,
            remaining_budget,
            consumed_budget,
        })
    }

    pub fn active_count(&self) -> usize {
        self.active.len()
    }
}
