//! Runtime-owned wake conditions and continuation lifecycle projections.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::PreconditionSubjectV1;
const WAKE_SCHEMA: &str = "wake-condition.v1";
const CONTINUATION_SCHEMA: &str = "agent-continuation.v1";
const WAKE_CONDITIONS_DIGEST_DOMAIN: &str = "oasis7.cognition.wake-conditions.v1";
const CONTINUATION_STATUS_DIGEST_DOMAIN: &str = "oasis7.cognition.continuation-status.v1";
const MAX_ID_BYTES: usize = 128;
const MAX_PATH_BYTES: usize = 128;
const MAX_ITEM_BYTES: usize = 768;
const MAX_LIST_BYTES: usize = 4096;
const MAX_CONDITIONS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WakeConditionError {
    code: &'static str,
}

impl WakeConditionError {
    fn new(code: &'static str) -> Self {
        Self { code }
    }

    pub fn code(&self) -> &str {
        self.code
    }
}

impl fmt::Display for WakeConditionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code)
    }
}

impl std::error::Error for WakeConditionError {}

/// A deliberately flat one-of shape.  Keeping optional members explicit
/// makes forbidden-field checks fail closed instead of silently ignoring a
/// future condition kind's payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WakeConditionV1 {
    pub schema_version: String,
    pub kind: String,
    #[serde(default)]
    pub logical_tick: Option<u64>,
    #[serde(default)]
    pub event_digest: Option<String>,
    #[serde(default)]
    pub receipt_id: Option<String>,
    #[serde(default)]
    pub subject: Option<PreconditionSubjectV1>,
    #[serde(default)]
    pub path_or_rule: Option<String>,
    #[serde(default)]
    pub operator: Option<String>,
    #[serde(default)]
    pub expected_value_bytes: Option<Vec<u8>>,
}

impl WakeConditionV1 {
    fn canonical_bytes_unchecked(&self) -> Vec<u8> {
        serde_cbor::to_vec(self).unwrap_or_default()
    }
}

pub struct WakeConditionValidator;

impl WakeConditionValidator {
    pub fn validate(conditions: &[WakeConditionV1]) -> Result<(), WakeConditionError> {
        if conditions.is_empty() {
            return Err(WakeConditionError::new("wake_conditions_empty"));
        }
        if conditions.len() > MAX_CONDITIONS {
            return Err(WakeConditionError::new("wake_condition_invalid"));
        }
        let mut seen = BTreeSet::new();
        let mut total = 0usize;
        for condition in conditions {
            Self::validate_one(condition)?;
            let bytes = Self::canonical_bytes(condition);
            if bytes.len() > MAX_ITEM_BYTES || !seen.insert(bytes.clone()) {
                return Err(WakeConditionError::new("wake_condition_invalid"));
            }
            total = total.saturating_add(bytes.len());
            if total > MAX_LIST_BYTES {
                return Err(WakeConditionError::new("wake_condition_invalid"));
            }
        }
        Ok(())
    }

    pub fn canonicalize(
        mut conditions: Vec<WakeConditionV1>,
    ) -> Result<Vec<WakeConditionV1>, WakeConditionError> {
        Self::validate(&conditions)?;
        conditions.sort_by_key(Self::canonical_bytes);
        Ok(conditions)
    }

    pub fn canonical_bytes(condition: &WakeConditionV1) -> Vec<u8> {
        oasis7_wasm_abi::encode_canonical_cbor(condition)
            .unwrap_or_else(|_| condition.canonical_bytes_unchecked())
    }

    pub fn conditions_digest(conditions: &[WakeConditionV1]) -> String {
        // The list is explicitly set-like in the v1 wake contract.  Always
        // sort here as well as in `canonicalize`, so callers cannot
        // accidentally create a different identity by hashing a non-canonical
        // permutation.
        let mut canonical = conditions.to_vec();
        canonical.sort_by_key(Self::canonical_bytes);
        let bytes = oasis7_wasm_abi::encode_canonical_cbor(&canonical).unwrap_or_default();
        h_v1(WAKE_CONDITIONS_DIGEST_DOMAIN, &bytes)
    }

    pub fn evaluate(
        conditions: &[WakeConditionV1],
        context: &WakeEvaluationContext,
    ) -> Result<WakeEvaluation, WakeConditionError> {
        let canonical = Self::canonicalize(conditions.to_vec())?;
        let mut expired = false;
        let mut all_met = true;
        for condition in &canonical {
            let (met, reference_expired) = context.evaluate_one(condition);
            expired |= reference_expired;
            all_met &= met;
        }
        if expired {
            Ok(WakeEvaluation {
                status: "expired".to_string(),
                reason: "wake_condition_expired".to_string(),
                evaluation_tick: context.logical_tick,
            })
        } else if all_met {
            Ok(WakeEvaluation {
                status: "ready".to_string(),
                reason: "condition_met".to_string(),
                evaluation_tick: context.logical_tick,
            })
        } else {
            Ok(WakeEvaluation {
                status: "pending".to_string(),
                reason: "condition_not_met".to_string(),
                evaluation_tick: context.logical_tick,
            })
        }
    }

    pub fn next_wake_tick(
        conditions: &[WakeConditionV1],
    ) -> Result<Option<u64>, WakeConditionError> {
        Self::validate(conditions)?;
        Ok(conditions
            .iter()
            .filter_map(|condition| condition.logical_tick)
            .max())
    }

    fn validate_one(condition: &WakeConditionV1) -> Result<(), WakeConditionError> {
        if condition.schema_version != WAKE_SCHEMA {
            return Err(WakeConditionError::new("wake_condition_invalid"));
        }
        let present = [
            condition.logical_tick.is_some(),
            condition.event_digest.is_some(),
            condition.receipt_id.is_some(),
            condition.subject.is_some(),
            condition.path_or_rule.is_some(),
            condition.operator.is_some(),
            condition.expected_value_bytes.is_some(),
        ];
        let exact = |expected: [bool; 7]| present == expected;
        match condition.kind.as_str() {
            "at_or_after_tick" if exact([true, false, false, false, false, false, false]) => Ok(()),
            "world_event_committed" if exact([false, true, false, false, false, false, false]) => {
                Self::bounded_text(condition.event_digest.as_deref())
            }
            "receipt_linked" if exact([false, false, true, false, false, false, false]) => {
                Self::bounded_text(condition.receipt_id.as_deref())
            }
            "state_predicate" if exact([false, false, false, true, true, true, true]) => {
                let subject = condition.subject.as_ref().expect("presence checked");
                if subject.kind != "world"
                    || subject.id.is_empty()
                    || subject.id.len() > MAX_ID_BYTES
                {
                    return Err(WakeConditionError::new("wake_condition_invalid"));
                }
                let path = condition.path_or_rule.as_deref().expect("presence checked");
                if path.len() > MAX_PATH_BYTES || path != "world.logical_tick" {
                    return Err(WakeConditionError::new("wake_condition_invalid"));
                }
                if !matches!(
                    condition.operator.as_deref(),
                    Some("eq" | "neq" | "lt" | "lte" | "gt" | "gte")
                ) {
                    return Err(WakeConditionError::new("wake_condition_invalid"));
                }
                let expected = condition
                    .expected_value_bytes
                    .as_ref()
                    .expect("presence checked");
                if expected.is_empty() || expected.len() > 512 {
                    return Err(WakeConditionError::new("wake_condition_invalid"));
                }
                Ok(())
            }
            _ => Err(WakeConditionError::new("wake_condition_invalid")),
        }
    }

    fn bounded_text(value: Option<&str>) -> Result<(), WakeConditionError> {
        let Some(value) = value else {
            return Err(WakeConditionError::new("wake_condition_invalid"));
        };
        if value.is_empty() || value.len() > MAX_ID_BYTES {
            Err(WakeConditionError::new("wake_condition_invalid"))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct WakeEvaluation {
    pub status: String,
    pub reason: String,
    pub evaluation_tick: u64,
}

#[derive(Debug, Clone, Default)]
pub struct WakeEvaluationContext {
    logical_tick: u64,
    event_digests: BTreeSet<String>,
    receipt_ids: BTreeSet<String>,
    gc_references: BTreeSet<String>,
    predicate_values: BTreeMap<String, Vec<u8>>,
}

impl WakeEvaluationContext {
    pub fn at(logical_tick: u64) -> Self {
        Self {
            logical_tick,
            ..Self::default()
        }
    }

    pub fn with_event(mut self, digest: &str) -> Self {
        self.event_digests.insert(digest.to_string());
        self
    }

    pub fn with_receipt(mut self, receipt_id: &str) -> Self {
        self.receipt_ids.insert(receipt_id.to_string());
        self
    }

    pub fn with_predicate_value(mut self, path: &str, value: &[u8]) -> Self {
        self.predicate_values
            .insert(path.to_string(), value.to_vec());
        self
    }

    pub fn with_gc_references(mut self, references: &[&str]) -> Self {
        self.gc_references
            .extend(references.iter().map(|reference| (*reference).to_string()));
        self
    }

    fn evaluate_one(&self, condition: &WakeConditionV1) -> (bool, bool) {
        match condition.kind.as_str() {
            "at_or_after_tick" => (
                self.logical_tick >= condition.logical_tick.unwrap_or(u64::MAX),
                false,
            ),
            "world_event_committed" => {
                let digest = condition.event_digest.as_deref().unwrap_or_default();
                (
                    self.event_digests.contains(digest),
                    self.gc_references.contains(digest),
                )
            }
            "receipt_linked" => {
                let receipt = condition.receipt_id.as_deref().unwrap_or_default();
                (
                    self.receipt_ids.contains(receipt),
                    self.gc_references.contains(receipt),
                )
            }
            "state_predicate" => {
                let path = condition.path_or_rule.as_deref().unwrap_or_default();
                let Some(actual) = self.predicate_values.get(path) else {
                    return (false, false);
                };
                let expected = condition
                    .expected_value_bytes
                    .as_deref()
                    .unwrap_or_default();
                let ordering = actual.as_slice().cmp(expected);
                let met = match condition.operator.as_deref() {
                    Some("eq") => ordering == std::cmp::Ordering::Equal,
                    Some("neq") => ordering != std::cmp::Ordering::Equal,
                    Some("lt") => ordering == std::cmp::Ordering::Less,
                    Some("lte") => ordering != std::cmp::Ordering::Greater,
                    Some("gt") => ordering == std::cmp::Ordering::Greater,
                    Some("gte") => ordering != std::cmp::Ordering::Less,
                    _ => false,
                };
                (met, false)
            }
            _ => (false, false),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContinuationBudgetV1 {
    pub unit: String,
    pub value: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuationStatusV1 {
    Scheduled,
    Pending,
    Waking,
    Consumed,
    Completed,
    Cancelled,
    Invalidated,
    Expired,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentContinuation {
    pub schema_version: String,
    pub continuation_id: String,
    pub wake_id: String,
    pub world_id: String,
    pub branch_id: String,
    pub finality_epoch: u64,
    #[serde(default)]
    pub finality_block_hash: Option<String>,
    pub finality_status: String,
    pub reorg_epoch: u64,
    pub runtime_manifest_hash: String,
    pub agent_id: String,
    pub agent_session_id: String,
    pub agent_turn_id: String,
    pub decision_request_id: String,
    pub origin_turn_id: String,
    pub origin_request_digest: String,
    pub continuation_proposal_id: String,
    pub proposal_digest: String,
    #[serde(default)]
    pub action_or_envelope_digest: Option<String>,
    pub wake_conditions: Vec<WakeConditionV1>,
    #[serde(default)]
    pub next_wake_tick: Option<u64>,
    pub remaining_budget: ContinuationBudgetV1,
    #[serde(default)]
    pub valid_until_tick: Option<u64>,
    pub precondition_digest: String,
    pub wake_seq: u64,
    /// The committed logical tick at which this status projection was
    /// produced.  It is Runtime-owned and participates in the status digest.
    #[serde(default)]
    pub logical_tick: u64,
    pub status: ContinuationStatusV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation_status_digest: Option<String>,
    #[serde(default)]
    pub terminal_disposition: Option<String>,
}

impl AgentContinuation {
    /// Validate a complete Runtime-owned continuation projection.  The
    /// legacy persisted shape may omit `continuation_status_digest`, but a
    /// projection crossing into the simulator must carry the Runtime-issued
    /// digest and match the canonical status fields exactly.
    pub fn validate_authoritative(&self) -> Result<(), WakeConditionError> {
        if self.schema_version != CONTINUATION_SCHEMA
            || self.continuation_id.trim().is_empty()
            || self.wake_id.trim().is_empty()
            || self.world_id.trim().is_empty()
            || self.branch_id.trim().is_empty()
            || self.finality_status.trim().is_empty()
            || self.runtime_manifest_hash.trim().is_empty()
            || self.agent_id.trim().is_empty()
            || self.agent_session_id.trim().is_empty()
            || self.agent_turn_id.trim().is_empty()
            || self.decision_request_id.trim().is_empty()
            || self.origin_turn_id.trim().is_empty()
            || self.origin_request_digest.trim().is_empty()
            || self.continuation_proposal_id.trim().is_empty()
            || self.proposal_digest.trim().is_empty()
            || self.precondition_digest.trim().is_empty()
            || self.remaining_budget.value == 0
            || !matches!(self.remaining_budget.unit.as_str(), "steps" | "ticks")
        {
            return Err(WakeConditionError::new("recovery_pending"));
        }
        WakeConditionValidator::validate(self.wake_conditions.as_slice())?;
        let terminal = matches!(
            self.status,
            ContinuationStatusV1::Completed
                | ContinuationStatusV1::Cancelled
                | ContinuationStatusV1::Invalidated
                | ContinuationStatusV1::Expired
                | ContinuationStatusV1::Rejected
        );
        if (terminal && self.terminal_disposition.is_none())
            || (!terminal && self.terminal_disposition.is_some())
            || self.continuation_status_digest.as_deref() != Some(self.status_digest().as_str())
        {
            return Err(WakeConditionError::new("recovery_pending"));
        }
        Ok(())
    }

    /// Derive the Runtime-owned continuation context identity.  Keeping this
    /// method on the durable projection prevents adapters from introducing a
    /// competing local digest algorithm.
    pub fn continuation_digest(&self) -> String {
        h_v1(
            "oasis7.cognition.continuation-context.v1",
            &json!({
                "continuation_proposal_id": self.continuation_proposal_id,
                "proposal_digest": self.proposal_digest,
                "continuation_id": self.continuation_id,
                "wake_id": self.wake_id,
                "wake_seq": self.wake_seq,
                "world_id": self.world_id,
                "branch_id": self.branch_id,
                "finality_epoch": self.finality_epoch,
                "finality_block_hash": self.finality_block_hash,
                "finality_status": self.finality_status,
                "reorg_epoch": self.reorg_epoch,
                "runtime_manifest_hash": self.runtime_manifest_hash,
                "remaining_budget": self.remaining_budget,
                "valid_until_tick": self.valid_until_tick,
                "status": self.status,
                "continuation_status_digest": self.continuation_status_digest,
                "terminal_disposition": self.terminal_disposition,
            }),
        )
    }

    pub fn status_digest(&self) -> String {
        let payload = ContinuationStatusDigestInput {
            continuation_id: &self.continuation_id,
            wake_id: &self.wake_id,
            wake_seq: self.wake_seq,
            from_status: None,
            to_status: self.status,
            logical_tick: self.logical_tick,
            world_id: &self.world_id,
            branch_id: &self.branch_id,
            finality_epoch: self.finality_epoch,
            finality_block_hash: self.finality_block_hash.as_deref(),
            finality_status: &self.finality_status,
            reorg_epoch: self.reorg_epoch,
            proposal_digest: &self.proposal_digest,
            terminal_disposition: self.terminal_disposition.as_deref(),
        };
        h_v1(CONTINUATION_STATUS_DIGEST_DOMAIN, &payload)
    }
}

#[derive(Debug, Serialize)]
struct ContinuationStatusDigestInput<'a> {
    continuation_id: &'a str,
    wake_id: &'a str,
    wake_seq: u64,
    from_status: Option<ContinuationStatusV1>,
    to_status: ContinuationStatusV1,
    logical_tick: u64,
    world_id: &'a str,
    branch_id: &'a str,
    finality_epoch: u64,
    finality_block_hash: Option<&'a str>,
    finality_status: &'a str,
    reorg_epoch: u64,
    proposal_digest: &'a str,
    terminal_disposition: Option<&'a str>,
}

fn h_v1<T: Serialize>(domain: &str, payload: &T) -> String {
    let bytes = oasis7_wasm_abi::encode_canonical_cbor(&(domain, payload))
        .expect("cognition identity payload must be canonicalizable");
    format!("blake3:{}", blake3::hash(&bytes))
}

pub struct ContinuationTransition;

impl ContinuationTransition {
    pub fn validate(
        from: ContinuationStatusV1,
        to: ContinuationStatusV1,
    ) -> Result<(), WakeConditionError> {
        let allowed = match from {
            ContinuationStatusV1::Scheduled => matches!(
                to,
                ContinuationStatusV1::Pending
                    | ContinuationStatusV1::Waking
                    | ContinuationStatusV1::Cancelled
                    | ContinuationStatusV1::Invalidated
                    | ContinuationStatusV1::Expired
                    | ContinuationStatusV1::Rejected
            ),
            ContinuationStatusV1::Pending => matches!(
                to,
                ContinuationStatusV1::Waking
                    | ContinuationStatusV1::Cancelled
                    | ContinuationStatusV1::Invalidated
                    | ContinuationStatusV1::Expired
                    | ContinuationStatusV1::Rejected
            ),
            ContinuationStatusV1::Waking => matches!(
                to,
                ContinuationStatusV1::Consumed
                    | ContinuationStatusV1::Cancelled
                    | ContinuationStatusV1::Invalidated
                    | ContinuationStatusV1::Expired
                    | ContinuationStatusV1::Rejected
            ),
            ContinuationStatusV1::Consumed => matches!(
                to,
                ContinuationStatusV1::Scheduled
                    | ContinuationStatusV1::Completed
                    | ContinuationStatusV1::Cancelled
                    | ContinuationStatusV1::Invalidated
                    | ContinuationStatusV1::Expired
                    | ContinuationStatusV1::Rejected
            ),
            ContinuationStatusV1::Completed
            | ContinuationStatusV1::Cancelled
            | ContinuationStatusV1::Invalidated
            | ContinuationStatusV1::Expired
            | ContinuationStatusV1::Rejected => false,
        };
        if allowed {
            Ok(())
        } else {
            Err(WakeConditionError::new("recovery_pending"))
        }
    }

    pub fn apply(
        continuation: &mut AgentContinuation,
        to: ContinuationStatusV1,
    ) -> Result<(), WakeConditionError> {
        Self::validate(continuation.status, to)?;
        continuation.status = to;
        if !matches!(
            to,
            ContinuationStatusV1::Scheduled
                | ContinuationStatusV1::Pending
                | ContinuationStatusV1::Waking
                | ContinuationStatusV1::Consumed
        ) {
            continuation.terminal_disposition = Some(
                match to {
                    ContinuationStatusV1::Cancelled => "cancelled",
                    ContinuationStatusV1::Invalidated => "reorg_invalidated",
                    ContinuationStatusV1::Expired => "expired",
                    ContinuationStatusV1::Rejected => "rejected",
                    ContinuationStatusV1::Completed => "completed",
                    _ => "",
                }
                .to_string(),
            );
        }
        Ok(())
    }

    pub fn invalidate_for_reorg(
        continuation: &mut AgentContinuation,
        reorg_epoch: u64,
    ) -> Result<ContinuationReorgReport, WakeConditionError> {
        Self::validate(continuation.status, ContinuationStatusV1::Invalidated)?;
        continuation.reorg_epoch = reorg_epoch;
        continuation.status = ContinuationStatusV1::Invalidated;
        continuation.terminal_disposition = Some("reorg_invalidated".to_string());
        Ok(ContinuationReorgReport {
            terminal_disposition: "reorg_invalidated".to_string(),
            provider_invocation_count: 0,
            effect_count: 0,
            receipt_count: 0,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContinuationReorgReport {
    pub terminal_disposition: String,
    pub provider_invocation_count: u64,
    pub effect_count: u64,
    pub receipt_count: u64,
}
