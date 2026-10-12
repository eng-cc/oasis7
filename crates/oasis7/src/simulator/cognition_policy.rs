//! Simulator-side policy objects for the continuous-agent Harness.
//!
//! These types are deliberately limited to bounded Agent-private cognition.
//! Runtime remains authoritative for receipts, durable continuation status and
//! world effects; this module only validates and projects those boundaries.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::continuous_agent_harness::{CognitionError, Digest32, MemoryWriteIntentV1};

pub use oasis7_agent_api::{GoalSnapshotV1, MemoryContextEntryV1, MemoryContextSnapshotV1};

use oasis7_agent_harness::{
    GoalSnapshotInputV1 as HarnessGoalSnapshotInputV1,
    GoalSnapshotProjector as HarnessGoalSnapshotProjector,
    MemoryWriteIntentPolicyV1 as HarnessMemoryWriteIntentPolicyV1,
    MemoryWritePolicyContextV1 as HarnessMemoryWritePolicyContextV1,
    MemoryWriteStore as HarnessMemoryWriteStore,
    NormalizedMemoryWriteIntentV1 as HarnessNormalizedMemoryWriteIntentV1,
};

#[path = "cognition_continuation.rs"]
mod cognition_continuation;
pub use cognition_continuation::*;

fn error(code: &'static str, message: impl Into<String>) -> CognitionError {
    CognitionError::new(code, message)
}

// ---------------------------------------------------------------------------
// Memory retrieval and write policy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryWritePolicyContextV1 {
    pub agent_id: String,
    pub agent_session_id: String,
    pub agent_turn_id: String,
    pub request_digest: String,
    pub source: String,
    pub provenance: String,
}

impl From<&MemoryWritePolicyContextV1> for HarnessMemoryWritePolicyContextV1 {
    fn from(value: &MemoryWritePolicyContextV1) -> Self {
        Self {
            agent_id: value.agent_id.clone(),
            agent_session_id: value.agent_session_id.clone(),
            agent_turn_id: value.agent_turn_id.clone(),
            request_digest: value.request_digest.clone(),
            source: value.source.clone(),
            provenance: value.provenance.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NormalizedMemoryWriteIntentV1 {
    pub schema_version: u16,
    pub scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility_reason: Option<String>,
}

impl From<NormalizedMemoryWriteIntentV1> for MemoryWriteIntentV1 {
    fn from(value: NormalizedMemoryWriteIntentV1) -> Self {
        Self {
            schema_version: value.schema_version,
            scope: value.scope,
            summary: value.summary,
            tags: value.tags,
            compatibility_reason: value.compatibility_reason,
        }
    }
}

impl From<MemoryWriteIntentV1> for oasis7_agent_api::MemoryWriteIntentV1 {
    fn from(value: MemoryWriteIntentV1) -> Self {
        Self {
            schema_version: value.schema_version,
            scope: value.scope,
            summary: value.summary,
            tags: value.tags,
            compatibility_reason: value.compatibility_reason,
        }
    }
}

impl From<&NormalizedMemoryWriteIntentV1> for HarnessNormalizedMemoryWriteIntentV1 {
    fn from(value: &NormalizedMemoryWriteIntentV1) -> Self {
        Self {
            schema_version: value.schema_version,
            scope: value.scope.clone(),
            summary: value.summary.clone(),
            tags: value.tags.clone(),
            compatibility_reason: value.compatibility_reason.clone(),
        }
    }
}

impl From<HarnessNormalizedMemoryWriteIntentV1> for NormalizedMemoryWriteIntentV1 {
    fn from(value: HarnessNormalizedMemoryWriteIntentV1) -> Self {
        Self {
            schema_version: value.schema_version,
            scope: value.scope,
            summary: value.summary,
            tags: value.tags,
            compatibility_reason: value.compatibility_reason,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryWriteIntentPolicyV1 {
    pub max_intents_per_turn: usize,
    pub max_summary_bytes: usize,
    pub max_tags_per_intent: usize,
    pub max_tag_bytes: usize,
    pub max_payload_bytes: usize,
}

impl Default for MemoryWriteIntentPolicyV1 {
    fn default() -> Self {
        let policy = HarnessMemoryWriteIntentPolicyV1::default();
        Self {
            max_intents_per_turn: policy.max_intents_per_turn,
            max_summary_bytes: policy.max_summary_bytes,
            max_tags_per_intent: policy.max_tags_per_intent,
            max_tag_bytes: policy.max_tag_bytes,
            max_payload_bytes: policy.max_payload_bytes,
        }
    }
}

impl MemoryWriteIntentPolicyV1 {
    fn harness_policy(&self) -> HarnessMemoryWriteIntentPolicyV1 {
        HarnessMemoryWriteIntentPolicyV1 {
            max_intents_per_turn: self.max_intents_per_turn,
            max_summary_bytes: self.max_summary_bytes,
            max_tags_per_intent: self.max_tags_per_intent,
            max_tag_bytes: self.max_tag_bytes,
            max_payload_bytes: self.max_payload_bytes,
        }
    }

    pub fn normalize(
        &self,
        intent: MemoryWriteIntentV1,
        context: &MemoryWritePolicyContextV1,
    ) -> Result<NormalizedMemoryWriteIntentV1, CognitionError> {
        self.harness_policy()
            .normalize(
                intent.into(),
                &HarnessMemoryWritePolicyContextV1::from(context),
            )
            .map(Into::into)
    }

    pub fn normalize_legacy(
        &self,
        intent: MemoryWriteIntentV1,
        context: &MemoryWritePolicyContextV1,
    ) -> Result<NormalizedMemoryWriteIntentV1, CognitionError> {
        self.harness_policy()
            .normalize_legacy(
                intent.into(),
                &HarnessMemoryWritePolicyContextV1::from(context),
            )
            .map(Into::into)
    }

    pub fn normalize_batch(
        &self,
        intents: Vec<MemoryWriteIntentV1>,
        context: &MemoryWritePolicyContextV1,
    ) -> Result<Vec<NormalizedMemoryWriteIntentV1>, CognitionError> {
        self.harness_policy()
            .normalize_batch(
                intents.into_iter().map(Into::into).collect(),
                &HarnessMemoryWritePolicyContextV1::from(context),
            )
            .map(|intents| intents.into_iter().map(Into::into).collect())
    }

    pub fn intent_digest(
        &self,
        intent: &NormalizedMemoryWriteIntentV1,
        context: &MemoryWritePolicyContextV1,
    ) -> Result<Digest32, CognitionError> {
        self.harness_policy().intent_digest(
            &HarnessNormalizedMemoryWriteIntentV1::from(intent),
            &HarnessMemoryWritePolicyContextV1::from(context),
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryWritePolicyOutcome {
    Committed {
        receipt_id: String,
        provenance: String,
    },
    Rejected,
    Failed,
    Pending,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryWriteStore {
    /// Monotonic store revision used to bind a retrieved snapshot to the
    /// committed memory projection that produced it.  Older checkpoints did
    /// not persist a revision, so serde defaults them to the empty revision.
    #[serde(default)]
    revision: u64,
    entries: Vec<Value>,
    committed_by_digest: BTreeMap<String, String>,
    #[serde(default)]
    corrections: Vec<MemoryCorrectionV1>,
    #[serde(default)]
    referenced_context_by_agent: BTreeMap<String, Value>,
}

/// Private-memory correction evidence. `applied` means a corrected retrieval
/// participated in a committed decision, never that the prediction came true.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryCorrectionV1 {
    pub correction_id: String,
    pub agent_id: String,
    pub agent_session_id: String,
    pub scope: String,
    pub target_memory_id: String,
    pub expected_revision: u64,
    pub replacement_summary: String,
    pub status: String,
    pub reason: String,
    pub memory_revision: u64,
    pub earliest_decision_request_id: Option<String>,
    pub earliest_request_digest: Option<String>,
    #[serde(default)]
    pub active_decision_request_id: Option<String>,
    #[serde(default)]
    pub active_request_digest: Option<String>,
    #[serde(default)]
    pub committed_decision_request_id: Option<String>,
    #[serde(default)]
    pub committed_request_digest: Option<String>,
    pub runtime_receipt_id: Option<String>,
    pub action_id: Option<String>,
}

impl MemoryWriteStore {
    fn with_harness_store<T>(
        &mut self,
        operation: impl FnOnce(&mut HarnessMemoryWriteStore) -> Result<T, CognitionError>,
    ) -> Result<T, CognitionError> {
        let mut core: HarnessMemoryWriteStore = serde_json::from_value(
            serde_json::to_value(&*self)
                .map_err(|err| error("memory_store_adapter_invalid", err.to_string()))?,
        )
        .map_err(|err| error("memory_store_adapter_invalid", err.to_string()))?;
        let result = operation(&mut core);
        *self = serde_json::from_value(
            serde_json::to_value(core)
                .map_err(|err| error("memory_store_adapter_invalid", err.to_string()))?,
        )
        .map_err(|err| error("memory_store_adapter_invalid", err.to_string()))?;
        result
    }

    fn harness_store(&self) -> Result<HarnessMemoryWriteStore, CognitionError> {
        serde_json::from_value(
            serde_json::to_value(self)
                .map_err(|err| error("memory_store_adapter_invalid", err.to_string()))?,
        )
        .map_err(|err| error("memory_store_adapter_invalid", err.to_string()))
    }

    pub fn corrections(&self) -> &[MemoryCorrectionV1] {
        &self.corrections
    }

    pub fn referenced_memory_context(&self, agent_id: &str) -> Option<&Value> {
        self.referenced_context_by_agent.get(agent_id)
    }

    pub fn ignore_corrections_for_decision(
        &mut self,
        agent_id: &str,
        decision_request_id: &str,
        request_digest: &str,
        reason: &str,
    ) {
        self.with_harness_store(|store| {
            store.ignore_corrections_for_decision(
                agent_id,
                decision_request_id,
                request_digest,
                reason,
            );
            Ok(())
        })
        .expect("memory checkpoint schema must round-trip through the canonical store");
    }

    /// The host must authorize the subject/session before calling this method.
    /// Exact revision and target checks prevent an old client overwriting a
    /// newer memory or a different subject's private context.
    pub fn correct_memory(
        &mut self,
        correction: MemoryCorrectionV1,
    ) -> Result<MemoryCorrectionV1, CognitionError> {
        let correction = serde_json::from_value(
            serde_json::to_value(correction)
                .map_err(|err| error("memory_correction_invalid", err.to_string()))?,
        )
        .map_err(|err| error("memory_correction_invalid", err.to_string()))?;
        self.with_harness_store(|store| store.correct_memory(correction))
            .and_then(|correction| {
                serde_json::from_value(
                    serde_json::to_value(correction)
                        .map_err(|err| error("memory_correction_invalid", err.to_string()))?,
                )
                .map_err(|err| error("memory_correction_invalid", err.to_string()))
            })
    }

    pub fn bind_corrections_to_decision(
        &mut self,
        turn: &super::continuous_agent_harness::ContinuousAgentTurnContextV1,
    ) -> Result<(), CognitionError> {
        let turn = serde_json::from_value(
            serde_json::to_value(turn)
                .map_err(|err| error("memory_context_invalid", err.to_string()))?,
        )
        .map_err(|err| error("memory_context_invalid", err.to_string()))?;
        self.with_harness_store(|store| store.bind_corrections_to_decision(&turn))
    }

    /// Host-only continuation of an authoritative stale parent. Earliest
    /// preparation provenance remains immutable; only finalization moves.
    pub(crate) fn rebind_corrections_after_stale_parent(
        &mut self,
        parent_request_id: &str,
        parent_digest: &str,
        turn: &super::continuous_agent_harness::ContinuousAgentTurnContextV1,
    ) -> Result<(), CognitionError> {
        let turn = serde_json::from_value(
            serde_json::to_value(turn)
                .map_err(|err| error("memory_context_invalid", err.to_string()))?,
        )
        .map_err(|err| error("memory_context_invalid", err.to_string()))?;
        self.with_harness_store(|store| {
            store.rebind_corrections_after_stale_parent(parent_request_id, parent_digest, &turn)
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn finalize_corrections(
        &mut self,
        receipt: &crate::runtime::RuntimeReceiptLineageV1,
    ) -> Result<(), CognitionError> {
        receipt
            .validate()
            .map_err(|err| error("memory_runtime_receipt_invalid", err.to_string()))?;
        if let Some(context) = self.referenced_context_by_agent.get_mut(&receipt.agent_id)
            && context.get("agent_session_id").and_then(Value::as_str)
                == Some(receipt.agent_session_id.as_str())
            && context.get("decision_request_id").and_then(Value::as_str)
                == Some(receipt.decision_request_id.as_str())
            && context.get("request_digest").and_then(Value::as_str)
                == Some(receipt.request_digest.as_str())
        {
            context["used_for_decision"] = json!(true);
            context["current_use"] = json!("committed_decision_context");
        }
        for correction in &mut self.corrections {
            if correction.status == "accepted"
                && correction.agent_id == receipt.agent_id
                && correction.agent_session_id == receipt.agent_session_id
                && correction
                    .active_decision_request_id
                    .as_ref()
                    .or(correction.earliest_decision_request_id.as_ref())
                    .map(String::as_str)
                    == Some(receipt.decision_request_id.as_str())
                && correction
                    .active_request_digest
                    .as_ref()
                    .or(correction.earliest_request_digest.as_ref())
                    .map(String::as_str)
                    == Some(receipt.request_digest.as_str())
            {
                correction.status = "applied".into();
                correction.reason = "corrected_context_committed_decision".into();
                correction.runtime_receipt_id = Some(receipt.receipt_id.clone());
                correction.action_id = Some(receipt.action_id.clone());
                correction.committed_decision_request_id =
                    Some(receipt.decision_request_id.clone());
                correction.committed_request_digest = Some(receipt.request_digest.clone());
            }
        }
        Ok(())
    }
    pub fn entries(&self) -> &[Value] {
        &self.entries
    }

    /// Build a deterministic, bounded snapshot for the next turn of one
    /// Agent session.  Runtime-authoritative entries carry their originating
    /// Harness identity; entries from another Agent/session (or legacy
    /// checkpoints without that identity) are never projected into this
    /// context.  The newest entries are selected while preserving commit
    /// order in the resulting snapshot.
    pub fn context_snapshot(
        &self,
        agent_id: &str,
        agent_session_id: &str,
        scope: &str,
        max_entries: usize,
    ) -> MemoryContextSnapshotV1 {
        self.harness_store()
            .expect("memory checkpoint schema must round-trip through the canonical store")
            .context_snapshot(agent_id, agent_session_id, scope, max_entries)
    }

    pub fn apply(
        &mut self,
        intent: NormalizedMemoryWriteIntentV1,
        digest: Digest32,
        outcome: MemoryWritePolicyOutcome,
    ) -> Result<(), CognitionError> {
        // Compatibility fixture lane: production commits must enter through
        // apply_runtime_receipt_with_context after the Runtime receipt gate.
        let digest_key = digest.as_str().to_string();
        if let Some(receipt) = self.committed_by_digest.get(&digest_key) {
            if matches!(outcome, MemoryWritePolicyOutcome::Committed { ref receipt_id, .. } if receipt_id == receipt)
            {
                return Ok(());
            }
            return Err(error(
                "memory_digest_mismatch",
                "a committed memory digest was replayed with a different receipt",
            ));
        }
        let MemoryWritePolicyOutcome::Committed {
            receipt_id,
            provenance,
        } = outcome
        else {
            return Err(error(
                "memory_no_committed_outcome",
                "authoritative memory requires a committed Runtime receipt",
            ));
        };
        if receipt_id.trim().is_empty() || provenance != "runtime_authoritative" {
            return Err(error(
                "memory_source_mismatch",
                "memory commit provenance is not Runtime-authoritative",
            ));
        }
        let mut entry = serde_json::to_value(intent)
            .map_err(|err| error("memory_payload_invalid", err.to_string()))?;
        let object = entry
            .as_object_mut()
            .expect("normalized memory intent is an object");
        object.insert("intent_digest".to_string(), json!(digest_key));
        object.insert("receipt_id".to_string(), json!(receipt_id));
        object.insert("provenance".to_string(), json!(provenance));
        self.committed_by_digest.insert(
            digest_key,
            object["receipt_id"]
                .as_str()
                .unwrap_or_default()
                .to_string(),
        );
        self.entries.push(entry);
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    fn annotate_context(&mut self, digest: &Digest32, context: &MemoryWritePolicyContextV1) {
        let digest = digest.as_str();
        if let Some(entry) = self
            .entries
            .iter_mut()
            .rev()
            .find(|entry| entry.get("intent_digest").and_then(Value::as_str) == Some(digest))
            && let Some(object) = entry.as_object_mut()
        {
            object.insert("agent_id".to_string(), json!(context.agent_id));
            object.insert(
                "agent_session_id".to_string(),
                json!(context.agent_session_id),
            );
            object.insert("agent_turn_id".to_string(), json!(context.agent_turn_id));
            object.insert("request_digest".to_string(), json!(context.request_digest));
        }
    }

    /// Commit only from the Runtime receipt projection.  The lower-level
    /// `apply` method remains for the policy unit fixtures; production async
    /// runner paths must use this gate so receipt, action, feedback and turn
    /// lineage are checked before persistence.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn apply_runtime_receipt(
        &mut self,
        intent: NormalizedMemoryWriteIntentV1,
        digest: Digest32,
        receipt: &crate::runtime::RuntimeReceiptLineageV1,
    ) -> Result<(), CognitionError> {
        self.apply_runtime_receipt_with_context(intent, digest, receipt, None)
    }

    /// Runtime receipt gate with optional Harness identity metadata.  The
    /// metadata is deliberately outside the intent digest: it is retrieval
    /// partitioning evidence, while the receipt and policy context remain the
    /// authority for whether the write may be committed.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn apply_runtime_receipt_with_context(
        &mut self,
        intent: NormalizedMemoryWriteIntentV1,
        digest: Digest32,
        receipt: &crate::runtime::RuntimeReceiptLineageV1,
        context: Option<&MemoryWritePolicyContextV1>,
    ) -> Result<(), CognitionError> {
        receipt.validate().map_err(|runtime_error| {
            error("memory_runtime_receipt_invalid", runtime_error.to_string())
        })?;
        if let Some(context) = context
            && (context.agent_id != receipt.agent_id
                || context.agent_session_id != receipt.agent_session_id
                || context.agent_turn_id != receipt.agent_turn_id
                || context.request_digest != receipt.request_digest)
        {
            return Err(error(
                "memory_runtime_context_mismatch",
                "memory context does not match committed receipt",
            ));
        }
        self.apply(
            intent,
            digest.clone(),
            MemoryWritePolicyOutcome::Committed {
                receipt_id: receipt.receipt_id.clone(),
                provenance: "runtime_authoritative".to_string(),
            },
        )?;
        if let Some(context) = context {
            self.annotate_context(&digest, context);
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// GoalSnapshot projection
// ---------------------------------------------------------------------------

/// Oasis-facing DTO retained for source compatibility. Projection policy is
/// canonical in `oasis7_agent_harness` and converted explicitly at this seam.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalSnapshotInputV1 {
    pub revision: u64,
    pub short_term_summary: String,
    pub long_term_summary: String,
    #[serde(default)]
    pub blocked_reason: Option<String>,
    pub provenance: String,
}

pub struct GoalSnapshotProjector;

impl GoalSnapshotProjector {
    pub fn project(
        host: Option<GoalSnapshotInputV1>,
        legacy: Option<GoalSnapshotInputV1>,
    ) -> Result<GoalSnapshotV1, CognitionError> {
        let into_harness = |input: GoalSnapshotInputV1| HarnessGoalSnapshotInputV1 {
            revision: input.revision,
            short_term_summary: input.short_term_summary,
            long_term_summary: input.long_term_summary,
            blocked_reason: input.blocked_reason,
            provenance: input.provenance,
        };
        HarnessGoalSnapshotProjector::project(host.map(&into_harness), legacy.map(&into_harness))
    }
}
