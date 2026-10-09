//! Simulator-side policy objects for the continuous-agent Harness.
//!
//! These types are deliberately limited to bounded Agent-private cognition.
//! Runtime remains authoritative for receipts, durable continuation status and
//! world effects; this module only validates and projects those boundaries.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use unicode_normalization::UnicodeNormalization;

use super::continuous_agent_harness::{CognitionError, Digest32, MemoryWriteIntentV1, h_v1};

#[path = "cognition_continuation.rs"]
mod cognition_continuation;
pub use cognition_continuation::*;

const MEMORY_SNAPSHOT_DOMAIN: &str = "oasis7.cognition.memory-context.v1";
const MEMORY_INTENT_DOMAIN: &str = "oasis7.cognition.memory-write-intent.v1";
const GOAL_SNAPSHOT_DOMAIN: &str = "oasis7.cognition.goal-snapshot.v1";
const MAX_MEMORY_INTENTS: usize = 8;
const MAX_MEMORY_SUMMARY_BYTES: usize = 512;
const MAX_MEMORY_TAGS: usize = 8;
const MAX_MEMORY_TAG_BYTES: usize = 64;
const MAX_MEMORY_PAYLOAD_BYTES: usize = 4096;
const MAX_GOAL_SUMMARY_BYTES: usize = 512;
const MAX_BLOCKED_REASON_BYTES: usize = 256;

fn error(code: &'static str, message: impl Into<String>) -> CognitionError {
    CognitionError::new(code, message)
}

fn normalized_text(
    value: &str,
    max_bytes: usize,
    too_large: &'static str,
    invalid: &'static str,
) -> Result<String, CognitionError> {
    let normalized: String = value.nfc().collect::<String>().trim().to_string();
    if normalized.len() > max_bytes {
        return Err(error(
            too_large,
            "bounded cognition text exceeds its byte limit",
        ));
    }
    if normalized.chars().any(char::is_control) {
        return Err(error(
            invalid,
            "cognition text contains a control character",
        ));
    }
    Ok(normalized)
}

fn digest_for_value(domain: &str, value: &Value) -> String {
    h_v1(domain, value).0
}

// ---------------------------------------------------------------------------
// Memory retrieval and write policy
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryContextEntryV1 {
    pub id: String,
    pub summary: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryContextSnapshotV1 {
    pub revision: u64,
    pub entries: Vec<MemoryContextEntryV1>,
    pub scope: String,
    pub digest: String,
}

impl MemoryContextSnapshotV1 {
    pub fn empty(scope: impl Into<String>) -> Self {
        let mut snapshot = Self {
            revision: 0,
            entries: Vec::new(),
            scope: scope.into(),
            digest: String::new(),
        };
        snapshot.digest = snapshot.computed_digest();
        snapshot
    }

    pub fn from_value(value: Value) -> Result<Self, CognitionError> {
        let snapshot: Self = serde_json::from_value(value)
            .map_err(|e| error("memory_snapshot_invalid", e.to_string()))?;
        if snapshot.scope.trim().is_empty() {
            return Err(error("memory_snapshot_invalid", "memory scope is required"));
        }
        if snapshot.digest != snapshot.computed_digest() {
            return Err(error(
                "memory_snapshot_digest_mismatch",
                "memory snapshot digest does not match canonical entries",
            ));
        }
        Ok(snapshot)
    }

    pub fn computed_digest(&self) -> String {
        let mut value = serde_json::to_value(self).expect("memory snapshot is serializable");
        value
            .as_object_mut()
            .expect("memory snapshot is an object")
            .remove("digest");
        digest_for_value(MEMORY_SNAPSHOT_DOMAIN, &value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryWritePolicyContextV1 {
    pub agent_id: String,
    pub agent_session_id: String,
    pub agent_turn_id: String,
    pub request_digest: String,
    pub source: String,
    pub provenance: String,
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
        Self {
            max_intents_per_turn: MAX_MEMORY_INTENTS,
            max_summary_bytes: MAX_MEMORY_SUMMARY_BYTES,
            max_tags_per_intent: MAX_MEMORY_TAGS,
            max_tag_bytes: MAX_MEMORY_TAG_BYTES,
            max_payload_bytes: MAX_MEMORY_PAYLOAD_BYTES,
        }
    }
}

impl MemoryWriteIntentPolicyV1 {
    fn check_identity(context: &MemoryWritePolicyContextV1) -> Result<(), CognitionError> {
        if context.agent_id.trim().is_empty()
            || context.agent_session_id.trim().is_empty()
            || context.agent_turn_id.trim().is_empty()
            || context.request_digest.trim().is_empty()
        {
            return Err(error(
                "memory_source_mismatch",
                "memory intent identity is incomplete",
            ));
        }
        Ok(())
    }

    fn check_context(context: &MemoryWritePolicyContextV1) -> Result<(), CognitionError> {
        Self::check_identity(context)?;
        if !matches!(context.source.as_str(), "provider" | "harness" | "builtin") {
            return Err(error(
                "memory_source_mismatch",
                "memory intent source is not a recognized Harness source",
            ));
        }
        if context.source == "provider" && context.provenance != "provider_unverified" {
            return Err(error(
                "memory_source_mismatch",
                "provider cannot self-assign authoritative memory provenance",
            ));
        }
        Ok(())
    }

    fn normalize_inner(
        &self,
        intent: MemoryWriteIntentV1,
        context: &MemoryWritePolicyContextV1,
    ) -> Result<NormalizedMemoryWriteIntentV1, CognitionError> {
        Self::check_context(context)?;
        if intent.schema_version != 1 {
            return Err(error(
                "memory_source_mismatch",
                "unsupported memory intent schema version",
            ));
        }
        if !matches!(intent.scope.as_str(), "turn_private" | "session_private") {
            return Err(error(
                "memory_scope_denied",
                "target lane permits only turn_private and session_private memory",
            ));
        }
        let summary = intent
            .summary
            .map(|value| {
                let value = normalized_text(
                    &value,
                    self.max_summary_bytes,
                    "memory_summary_too_large",
                    "memory_summary_invalid",
                )?;
                if value.is_empty() {
                    return Err(error("memory_summary_invalid", "summary must not be empty"));
                }
                Ok(value)
            })
            .transpose()?;
        if intent.tags.len() > self.max_tags_per_intent {
            return Err(error(
                "memory_tag_count_exceeded",
                "memory intent has too many tags",
            ));
        }
        let mut tags = Vec::with_capacity(intent.tags.len());
        for raw in intent.tags {
            let tag = normalized_text(
                &raw,
                self.max_tag_bytes,
                "memory_tag_invalid",
                "memory_tag_invalid",
            )?;
            if tag.is_empty() {
                return Err(error(
                    "memory_tag_invalid",
                    "tags must not contain empty values",
                ));
            }
            tags.push(tag);
        }
        tags.sort_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
        tags.dedup();
        Ok(NormalizedMemoryWriteIntentV1 {
            schema_version: 1,
            scope: intent.scope,
            summary,
            tags,
            compatibility_reason: intent.compatibility_reason,
        })
    }

    pub fn normalize(
        &self,
        intent: MemoryWriteIntentV1,
        context: &MemoryWritePolicyContextV1,
    ) -> Result<NormalizedMemoryWriteIntentV1, CognitionError> {
        let normalized = self.normalize_inner(intent, context)?;
        if self.canonical_intent_bytes(&normalized).len() > self.max_payload_bytes {
            return Err(error(
                "memory_payload_too_large",
                "memory intent exceeds the canonical payload bound",
            ));
        }
        Ok(normalized)
    }

    pub fn normalize_legacy(
        &self,
        mut intent: MemoryWriteIntentV1,
        context: &MemoryWritePolicyContextV1,
    ) -> Result<NormalizedMemoryWriteIntentV1, CognitionError> {
        if intent.scope != "short_term" {
            return self.normalize(intent, context);
        }
        intent.scope = "turn_private".to_string();
        let mut normalized = self.normalize_inner(intent, context)?;
        normalized.compatibility_reason = Some("memory_scope_alias_used".to_string());
        if self.canonical_intent_bytes(&normalized).len() > self.max_payload_bytes {
            return Err(error(
                "memory_payload_too_large",
                "memory intent exceeds the canonical payload bound",
            ));
        }
        Ok(normalized)
    }

    pub fn normalize_batch(
        &self,
        intents: Vec<MemoryWriteIntentV1>,
        context: &MemoryWritePolicyContextV1,
    ) -> Result<Vec<NormalizedMemoryWriteIntentV1>, CognitionError> {
        if intents.len() > self.max_intents_per_turn {
            return Err(error(
                "memory_intent_count_exceeded",
                "turn contains too many memory intents",
            ));
        }
        let normalized = intents
            .into_iter()
            .map(|intent| self.normalize(intent, context))
            .collect::<Result<Vec<_>, _>>()?;
        let total = normalized
            .iter()
            .map(|intent| self.canonical_intent_bytes(intent).len())
            .sum::<usize>();
        if total > self.max_payload_bytes {
            return Err(error(
                "memory_payload_too_large",
                "turn memory intents exceed the canonical payload bound",
            ));
        }
        Ok(normalized)
    }

    fn canonical_intent_value(&self, intent: &NormalizedMemoryWriteIntentV1) -> Value {
        json!({
            "schema_version": intent.schema_version,
            "scope": intent.scope,
            "summary_present": intent.summary.is_some(),
            "summary": intent.summary,
            "tags": intent.tags,
            "compatibility_reason": intent.compatibility_reason,
        })
    }

    fn canonical_intent_bytes(&self, intent: &NormalizedMemoryWriteIntentV1) -> Vec<u8> {
        oasis7_wasm_abi::encode_canonical_cbor(&self.canonical_intent_value(intent))
            .expect("memory intent is canonicalizable")
    }

    pub fn intent_digest(
        &self,
        intent: &NormalizedMemoryWriteIntentV1,
        context: &MemoryWritePolicyContextV1,
    ) -> Result<Digest32, CognitionError> {
        // Digest construction binds provenance as data.  Validation of a
        // provider's claim belongs to the policy/commit gate; retaining the
        // value here also makes forged provenance produce a distinct digest.
        Self::check_identity(context)?;
        if !matches!(context.source.as_str(), "provider" | "harness" | "builtin") {
            return Err(error(
                "memory_source_mismatch",
                "memory intent source is not a recognized Harness source",
            ));
        }
        Ok(h_v1(
            MEMORY_INTENT_DOMAIN,
            &json!({
                "agent_id": context.agent_id,
                "agent_session_id": context.agent_session_id,
                "agent_turn_id": context.agent_turn_id,
                "request_digest": context.request_digest,
                "source": context.source,
                "provenance": context.provenance,
                "intent": self.canonical_intent_value(intent),
            }),
        ))
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
        for correction in &mut self.corrections {
            if correction.status == "accepted"
                && correction.agent_id == agent_id
                && correction
                    .active_decision_request_id
                    .as_ref()
                    .or(correction.earliest_decision_request_id.as_ref())
                    .map(String::as_str)
                    == Some(decision_request_id)
                && correction
                    .active_request_digest
                    .as_ref()
                    .or(correction.earliest_request_digest.as_ref())
                    .map(String::as_str)
                    == Some(request_digest)
            {
                correction.status = "ignored".into();
                correction.reason = reason.to_string();
            }
        }
    }

    /// The host must authorize the subject/session before calling this method.
    /// Exact revision and target checks prevent an old client overwriting a
    /// newer memory or a different subject's private context.
    pub fn correct_memory(
        &mut self,
        mut correction: MemoryCorrectionV1,
    ) -> Result<MemoryCorrectionV1, CognitionError> {
        if correction.correction_id.trim().is_empty() || correction.correction_id.len() > 256 {
            return Err(error(
                "memory_correction_identity_invalid",
                "invalid correction identity",
            ));
        }
        correction.replacement_summary = normalized_text(
            &correction.replacement_summary,
            MAX_MEMORY_SUMMARY_BYTES,
            "memory_summary_too_large",
            "memory_summary_invalid",
        )?;
        if correction.replacement_summary.is_empty() {
            return Err(error("memory_summary_invalid", "empty correction summary"));
        }
        if let Some(previous) = self
            .corrections
            .iter()
            .find(|item| item.correction_id == correction.correction_id)
        {
            if previous.agent_id == correction.agent_id
                && previous.agent_session_id == correction.agent_session_id
                && previous.scope == correction.scope
                && previous.target_memory_id == correction.target_memory_id
                && previous.expected_revision == correction.expected_revision
                && previous.replacement_summary == correction.replacement_summary
            {
                return Ok(previous.clone());
            }
            return Err(error(
                "memory_correction_identity_reused",
                "correction identity has different payload",
            ));
        }
        correction.earliest_decision_request_id = None;
        correction.earliest_request_digest = None;
        correction.active_decision_request_id = None;
        correction.active_request_digest = None;
        correction.committed_decision_request_id = None;
        correction.committed_request_digest = None;
        correction.runtime_receipt_id = None;
        correction.action_id = None;
        let target = self.entries.iter_mut().find(|entry| {
            entry.get("intent_digest").and_then(Value::as_str)
                == Some(correction.target_memory_id.as_str())
                && entry.get("agent_id").and_then(Value::as_str)
                    == Some(correction.agent_id.as_str())
                && entry.get("agent_session_id").and_then(Value::as_str)
                    == Some(correction.agent_session_id.as_str())
                && entry.get("scope").and_then(Value::as_str) == Some(correction.scope.as_str())
        });
        if correction.expected_revision != self.revision {
            correction.status = "stale".into();
            correction.reason = "memory_revision_changed".into();
        } else if let Some(target) = target {
            target["summary"] = json!(correction.replacement_summary);
            self.revision = self.revision.saturating_add(1);
            correction.status = "accepted".into();
            correction.reason = "next_retrieval_pending".into();
        } else {
            correction.status = "ignored".into();
            correction.reason = "memory_target_or_scope_unavailable".into();
        }
        correction.memory_revision = self.revision;
        self.corrections.push(correction.clone());
        Ok(correction)
    }

    pub fn bind_corrections_to_decision(
        &mut self,
        turn: &super::continuous_agent_harness::ContinuousAgentTurnContextV1,
    ) -> Result<(), CognitionError> {
        turn.validate_for_agent(&turn.agent_id)?;
        let sources = turn.memory_snapshot.entries.iter().map(|selected| {
            let source = self.entries.iter().find(|entry| entry.get("intent_digest").and_then(Value::as_str) == Some(selected.id.as_str()));
            json!({ "memory_id": selected.id,
                "origin_receipt_id": source.and_then(|entry| entry.get("receipt_id")).cloned(),
                "correction_refs": self.corrections.iter().filter(|correction| correction.target_memory_id == selected.id && correction.agent_id == turn.agent_id && correction.agent_session_id == turn.agent_session_id && matches!(correction.status.as_str(), "accepted" | "applied") && correction.replacement_summary == selected.summary).map(|correction| correction.correction_id.clone()).collect::<Vec<_>>()
            })
        }).collect::<Vec<_>>();
        self.referenced_context_by_agent.insert(
            turn.agent_id.clone(),
            json!({
                "agent_session_id": turn.agent_session_id,
                "decision_request_id": turn.decision_request_id,
                "request_digest": turn.request_digest.to_string(),
                "scope": turn.memory_snapshot.scope,
                "revision": turn.memory_snapshot.revision,
            "entries": turn.memory_snapshot.entries,
            "sources": sources,
            "source": "private_memory_retrieval_context",
                "used_for_decision": false,
                "current_use": "included_in_prepared_decision_request",
                "correction_hint": "correct_by_memory_id_and_revision",
            }),
        );
        for correction in &mut self.corrections {
            if correction.status == "accepted"
                && correction.earliest_decision_request_id.is_none()
                && correction.agent_id == turn.agent_id
                && correction.agent_session_id == turn.agent_session_id
                && correction.scope == turn.memory_snapshot.scope
                && turn.memory_snapshot.revision >= correction.memory_revision
                && turn.memory_snapshot.entries.iter().any(|entry| {
                    entry.id == correction.target_memory_id
                        && entry.summary == correction.replacement_summary
                })
            {
                correction.earliest_decision_request_id = Some(turn.decision_request_id.clone());
                correction.earliest_request_digest = Some(turn.request_digest.to_string());
                correction.active_decision_request_id = Some(turn.decision_request_id.clone());
                correction.active_request_digest = Some(turn.request_digest.to_string());
                correction.reason = "runtime_result_pending".into();
            }
        }
        Ok(())
    }

    /// Host-only continuation of an authoritative stale parent. Earliest
    /// preparation provenance remains immutable; only finalization moves.
    pub(crate) fn rebind_corrections_after_stale_parent(
        &mut self,
        parent_request_id: &str,
        parent_digest: &str,
        turn: &super::continuous_agent_harness::ContinuousAgentTurnContextV1,
    ) -> Result<(), CognitionError> {
        turn.validate_for_agent(&turn.agent_id)?;
        for correction in &mut self.corrections {
            if correction.status == "accepted"
                && correction.agent_id == turn.agent_id
                && correction.agent_session_id == turn.agent_session_id
                && correction.scope == turn.memory_snapshot.scope
                && turn.memory_snapshot.revision >= correction.memory_revision
                && correction
                    .active_decision_request_id
                    .as_ref()
                    .or(correction.earliest_decision_request_id.as_ref())
                    .map(String::as_str)
                    == Some(parent_request_id)
                && correction
                    .active_request_digest
                    .as_ref()
                    .or(correction.earliest_request_digest.as_ref())
                    .map(String::as_str)
                    == Some(parent_digest)
                && turn.memory_snapshot.entries.iter().any(|entry| {
                    entry.id == correction.target_memory_id
                        && entry.summary == correction.replacement_summary
                })
            {
                correction.active_decision_request_id = Some(turn.decision_request_id.clone());
                correction.active_request_digest = Some(turn.request_digest.to_string());
            }
        }
        Ok(())
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
        let limit = max_entries.min(MAX_MEMORY_INTENTS);
        let mut selected = self
            .entries
            .iter()
            .filter(|entry| {
                entry.get("agent_id").and_then(Value::as_str) == Some(agent_id)
                    && entry.get("agent_session_id").and_then(Value::as_str)
                        == Some(agent_session_id)
                    && entry.get("scope").and_then(Value::as_str) == Some(scope)
            })
            .filter_map(|entry| {
                let id = entry.get("intent_digest")?.as_str()?.to_string();
                let summary = entry.get("summary")?.as_str()?.to_string();
                let tags = entry
                    .get("tags")
                    .and_then(Value::as_array)
                    .map(|tags| {
                        tags.iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                Some(MemoryContextEntryV1 { id, summary, tags })
            })
            .collect::<Vec<_>>();
        if selected.len() > limit {
            let keep_from = selected.len() - limit;
            selected.drain(..keep_from);
        }
        let mut snapshot = MemoryContextSnapshotV1 {
            revision: self.revision,
            entries: selected,
            scope: scope.to_string(),
            digest: String::new(),
        };
        snapshot.digest = snapshot.computed_digest();
        snapshot
    }

    pub fn apply(
        &mut self,
        intent: NormalizedMemoryWriteIntentV1,
        digest: Digest32,
        outcome: MemoryWritePolicyOutcome,
    ) -> Result<(), CognitionError> {
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
            .map_err(|e| error("memory_payload_invalid", e.to_string()))?;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalSnapshotInputV1 {
    pub revision: u64,
    pub short_term_summary: String,
    pub long_term_summary: String,
    #[serde(default)]
    pub blocked_reason: Option<String>,
    pub provenance: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalSnapshotV1 {
    pub revision: u64,
    pub short_term_summary: String,
    pub long_term_summary: String,
    #[serde(default)]
    pub blocked_reason: Option<String>,
    pub provenance: String,
    pub digest: String,
}

impl GoalSnapshotV1 {
    pub fn empty() -> Self {
        let mut snapshot = Self {
            revision: 0,
            short_term_summary: String::new(),
            long_term_summary: String::new(),
            blocked_reason: None,
            provenance: "harness_projection".to_string(),
            digest: String::new(),
        };
        snapshot.digest = snapshot.computed_digest();
        snapshot
    }

    pub fn from_value(value: Value) -> Result<Self, CognitionError> {
        let snapshot: Self = serde_json::from_value(value)
            .map_err(|e| error("goal_snapshot_invalid", e.to_string()))?;
        if snapshot.digest != snapshot.computed_digest() {
            return Err(error(
                "goal_snapshot_digest_mismatch",
                "goal snapshot digest does not match canonical projection",
            ));
        }
        Ok(snapshot)
    }

    pub fn computed_digest(&self) -> String {
        let mut value = serde_json::to_value(self).expect("goal snapshot is serializable");
        value
            .as_object_mut()
            .expect("goal snapshot is an object")
            .remove("digest");
        digest_for_value(GOAL_SNAPSHOT_DOMAIN, &value)
    }
}

pub struct GoalSnapshotProjector;

impl GoalSnapshotProjector {
    pub fn project(
        host: Option<GoalSnapshotInputV1>,
        legacy: Option<GoalSnapshotInputV1>,
    ) -> Result<GoalSnapshotV1, CognitionError> {
        let input = if let Some(input) = host {
            if input.provenance != "harness_projection" {
                return Err(error(
                    "goal_snapshot_invalid",
                    "host goal projection has an invalid provenance",
                ));
            }
            input
        } else if let Some(input) = legacy {
            if input.provenance != "legacy_provider" {
                return Err(error(
                    "goal_snapshot_invalid",
                    "legacy goal projection has an invalid provenance",
                ));
            }
            input
        } else {
            return Ok(GoalSnapshotV1::empty());
        };
        let short_term_summary = normalized_text(
            &input.short_term_summary,
            MAX_GOAL_SUMMARY_BYTES,
            "goal_snapshot_too_large",
            "goal_snapshot_invalid",
        )?;
        let long_term_summary = normalized_text(
            &input.long_term_summary,
            MAX_GOAL_SUMMARY_BYTES,
            "goal_snapshot_too_large",
            "goal_snapshot_invalid",
        )?;
        let blocked_reason = input
            .blocked_reason
            .map(|value| {
                let normalized = normalized_text(
                    &value,
                    MAX_BLOCKED_REASON_BYTES,
                    "goal_snapshot_too_large",
                    "goal_snapshot_invalid",
                )?;
                Ok((!normalized.is_empty()).then_some(normalized))
            })
            .transpose()?
            .flatten();
        let mut snapshot = GoalSnapshotV1 {
            revision: input.revision,
            short_term_summary,
            long_term_summary,
            blocked_reason,
            provenance: input.provenance,
            digest: String::new(),
        };
        snapshot.digest = snapshot.computed_digest();
        Ok(snapshot)
    }
}
