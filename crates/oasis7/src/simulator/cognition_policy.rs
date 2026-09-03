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
    if normalized.as_bytes().len() > max_bytes {
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
    entries: Vec<Value>,
    committed_by_digest: BTreeMap<String, String>,
}

impl MemoryWriteStore {
    pub fn entries(&self) -> &[Value] {
        &self.entries
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
        Ok(())
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
        receipt.validate().map_err(|runtime_error| {
            error("memory_runtime_receipt_invalid", runtime_error.to_string())
        })?;
        self.apply(
            intent,
            digest,
            MemoryWritePolicyOutcome::Committed {
                receipt_id: receipt.receipt_id.clone(),
                provenance: "runtime_authoritative".to_string(),
            },
        )
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
