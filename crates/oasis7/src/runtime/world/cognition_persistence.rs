//! World-owned durable cognition projection and recovery.
//!
//! The provider and kernel are intentionally absent from this module.  Once a
//! process has restarted, the only inputs accepted by recovery are the
//! persisted commit marker, cognition journal, response artifact, receipt
//! registry, idempotency index and root projection.  Recovery may repair
//! projections, but it never re-executes an effect or creates a debit.

use super::World;
use super::cognition_persistence_validation::{
    append_cognition_event, cognition_error, cognition_validation, legacy_recovery_report,
    parent_root, persist_recovery_report, validate_marker_current_world, validate_response_record,
};
use crate::runtime::cognition::{AgentDecisionEnvelopeV1, MvccValidator};
use crate::runtime::cognition_recovery::{
    CognitionReceiptViewV1, CognitionRecoveryReport, CognitionResponseRecordV1,
    RuntimeReceiptLineageV1, WorldCommitRecordV1, WorldRootViewV1, cognition_digest_v1,
    default_cognition_persistence_projection, response_artifact_digest,
};
use crate::runtime::cognition_scheduler::{CognitionScheduler, SchedulerWakeV1};
use crate::runtime::cognition_wake::AgentContinuation;
use crate::runtime::error::WorldError;
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::collections::BTreeMap;

const PROJECTION_SCHEMA: &str = "cognition-persistence.v1";
const JOURNAL_SCHEMA: &str = "cognition-journal.v1";
const WORLD_COMMIT_SCHEMA: &str = "world-commit-record.v1";

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct CognitionProjection {
    #[serde(default)]
    schema_version: String,
    #[serde(default)]
    cognition_journal: CognitionJournalProjection,
    #[serde(default)]
    responses: Vec<CognitionResponseRecordV1>,
    #[serde(default)]
    commit_records: Vec<WorldCommitRecordV1>,
    #[serde(default)]
    receipt_registry: Vec<CognitionReceiptViewV1>,
    #[serde(default)]
    receipt_lineage_registry: Vec<RuntimeReceiptLineageV1>,
    #[serde(default)]
    idempotency_index: BTreeMap<String, IdempotencyProjection>,
    #[serde(default)]
    recovery: Option<RecoveryProjection>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct CognitionJournalProjection {
    #[serde(default)]
    schema_version: String,
    #[serde(default)]
    head_seq: u64,
    #[serde(default)]
    head_digest: String,
    #[serde(default)]
    events: Vec<CognitionJournalEvent>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct CognitionJournalEvent {
    #[serde(default)]
    journal_seq: u64,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    envelope_idempotency_key: String,
    #[serde(default)]
    receipt_id: Option<String>,
    #[serde(default)]
    agent_id: Option<String>,
    #[serde(default)]
    agent_session_id: Option<String>,
    #[serde(default)]
    agent_turn_id: Option<String>,
    #[serde(default)]
    decision_request_id: Option<String>,
    #[serde(default)]
    request_digest: Option<String>,
    #[serde(default)]
    feedback_id: Option<String>,
    #[serde(default)]
    continuation_id: Option<String>,
    #[serde(default)]
    wake_id: Option<String>,
    #[serde(default)]
    scheduler_policy_digest: Option<String>,
    #[serde(default)]
    cursor_seq: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct IdempotencyProjection {
    #[serde(default)]
    envelope_digest: String,
    #[serde(default)]
    disposition: String,
    #[serde(default)]
    receipt_id: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct RecoveryProjection {
    #[serde(default)]
    crash_prefix: String,
    #[serde(default)]
    world_root: Option<WorldRootViewV1>,
    #[serde(default)]
    repaired_projection_ids: Vec<String>,
}

/// Return the additive projection exposed by a world.  This is useful for
/// checkpoint tooling while keeping the fields themselves runtime-owned.
impl World {
    pub fn cognition(&self) -> &JsonValue {
        &self.cognition
    }

    /// Prepare a cognition envelope against the actual current World head.
    /// The returned marker is provisional; receipt/idempotency authority is
    /// created only by finalize_cognition_commit.
    pub fn prepare_cognition_envelope(
        &mut self,
        envelope: AgentDecisionEnvelopeV1,
        response_artifact: Option<JsonValue>,
    ) -> Result<WorldCommitRecordV1, WorldError> {
        MvccValidator::validate(self, &envelope)
            .map_err(|error| cognition_validation(error.code()))?;
        let world_root = self.current_state_root_hash()?;
        let manifest_hash = self.current_manifest_hash()?;
        if envelope.base_tick != self.state.time
            || envelope.base_world_hash != world_root
            || envelope.runtime_manifest_hash != manifest_hash
        {
            return Err(cognition_validation("cognition_world_head_mismatch"));
        }
        self.bind_cognition_runtime(
            envelope.world_id.clone(),
            envelope.branch_id.clone(),
            envelope.finality_epoch,
            envelope.finality_block_hash.clone(),
            envelope.finality_status.clone(),
            envelope.reorg_epoch,
        )?;

        let projection = if self.cognition.is_null() {
            default_cognition_persistence_projection()
        } else {
            self.cognition.clone()
        };
        let parsed: CognitionProjection = serde_json::from_value(projection.clone())
            .map_err(|error| cognition_error("invalid_cognition_projection", error))?;
        if parsed.schema_version != PROJECTION_SCHEMA {
            return Err(cognition_validation("cognition_projection_schema_mismatch"));
        }
        if let Some(existing) = parsed
            .commit_records
            .iter()
            .find(|record| record.envelope_idempotency_key == envelope.envelope_idempotency_key)
        {
            if existing.envelope_digest == envelope.envelope_digest {
                return Ok(existing.clone());
            }
            return Err(cognition_validation("envelope_idempotency_conflict"));
        }

        let action_id = format!("action:{}", self.allocate_next_action_id());
        let commit_id = cognition_digest_v1(
            "oasis7.cognition.commit-id.v1",
            &json!({
                "envelope_digest": envelope.envelope_digest,
                "envelope_idempotency_key": envelope.envelope_idempotency_key,
            }),
        );
        let receipt_id = cognition_digest_v1(
            "oasis7.cognition.receipt-id.v1",
            &json!({"commit_id": commit_id}),
        );
        let receipt_digest = cognition_digest_v1(
            "oasis7.cognition.receipt.v1",
            &json!({
                "receipt_id": receipt_id,
                "envelope_digest": envelope.envelope_digest,
                "action_id": action_id,
                "world_root": world_root,
            }),
        );
        let staged_event_root = cognition_digest_v1(
            "oasis7.cognition.staged-event-root.v1",
            &json!({
                "world_root": world_root,
                "envelope_digest": envelope.envelope_digest,
            }),
        );
        let finality_block_hash = envelope
            .finality_block_hash
            .clone()
            .unwrap_or_else(|| "genesis".to_string());
        let marker = WorldCommitRecordV1 {
            schema_version: WORLD_COMMIT_SCHEMA.to_string(),
            commit_id,
            envelope_idempotency_key: envelope.envelope_idempotency_key.clone(),
            envelope_digest: envelope.envelope_digest.clone(),
            world_id: envelope.world_id.clone(),
            branch_id: envelope.branch_id.clone(),
            finality_epoch: envelope.finality_epoch,
            finality_block_hash,
            finality_status: envelope.finality_status.clone(),
            finality_binding_digest: envelope.derive_finality_binding_digest(),
            runtime_manifest_hash: manifest_hash,
            action_id,
            parent_tick: envelope.base_tick,
            parent_world_hash: world_root.clone(),
            staged_event_root,
            staged_state_root: world_root,
            receipt_id,
            receipt_digest,
            reorg_epoch: envelope.reorg_epoch,
            cognition_journal_seq: 0,
            status: "prepared".to_string(),
            agent_id: envelope.agent_id,
            agent_session_id: envelope.agent_session_id,
            agent_turn_id: envelope.agent_turn_id,
            decision_request_id: envelope.decision_request_id,
            request_digest: envelope.request_digest,
            feedback_id: cognition_digest_v1(
                "oasis7.cognition.feedback-id.v1",
                &envelope.envelope_digest,
            ),
            abort_reason: None,
        };
        validate_marker(&marker)?;
        let mut next = projection;
        let object = next
            .as_object_mut()
            .ok_or_else(|| cognition_validation("cognition_projection_not_object"))?;
        object
            .entry("commit_records")
            .or_insert_with(|| JsonValue::Array(Vec::new()))
            .as_array_mut()
            .ok_or_else(|| cognition_validation("commit_records_not_array"))?
            .push(serde_json::to_value(&marker).map_err(WorldError::from)?);
        object
            .entry("idempotency_index")
            .or_insert_with(|| JsonValue::Object(serde_json::Map::new()))
            .as_object_mut()
            .ok_or_else(|| cognition_validation("idempotency_index_not_object"))?
            .insert(
                marker.envelope_idempotency_key.clone(),
                json!({
                    "envelope_digest": marker.envelope_digest,
                    "disposition": "prepared",
                    "receipt_id": marker.receipt_id,
                }),
            );
        let seq = append_cognition_event(
            &mut next,
            "DecisionValidated",
            json!({
                "envelope_idempotency_key": marker.envelope_idempotency_key,
                "agent_id": marker.agent_id,
                "agent_session_id": marker.agent_session_id,
                "agent_turn_id": marker.agent_turn_id,
                "decision_request_id": marker.decision_request_id,
                "request_digest": marker.request_digest,
                "feedback_id": marker.feedback_id,
            }),
        )?;
        if let Some(record) = next
            .get_mut("commit_records")
            .and_then(JsonValue::as_array_mut)
            .and_then(|records| records.last_mut())
        {
            record["cognition_journal_seq"] = json!(seq);
        }
        if let Some(artifact) = response_artifact {
            let response = CognitionResponseRecordV1 {
                response_digest: response_artifact_digest(&artifact),
                response_artifact: Some(artifact),
                envelope_digest: marker.envelope_digest.clone(),
                journal_head: next["cognition_journal"]["head_digest"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
            };
            next["responses"]
                .as_array_mut()
                .ok_or_else(|| cognition_validation("responses_not_array"))?
                .push(serde_json::to_value(response).map_err(WorldError::from)?);
        }
        self.cognition = next;
        let mut prepared = marker;
        prepared.cognition_journal_seq = seq;
        Ok(prepared)
    }

    /// Finalize a prepared marker and atomically project its receipt,
    /// idempotency disposition, dense journal link and runtime lineage.
    pub fn finalize_cognition_commit(
        &mut self,
        commit_id: &str,
    ) -> Result<WorldCommitRecordV1, WorldError> {
        let projection = self.cognition.clone();
        let parsed: CognitionProjection = serde_json::from_value(projection.clone())
            .map_err(|error| cognition_error("invalid_cognition_projection", error))?;
        let marker = parsed
            .commit_records
            .iter()
            .find(|record| record.commit_id == commit_id)
            .ok_or_else(|| cognition_validation("commit_record_missing"))?;
        if marker.status == "committed" {
            return Ok(marker.clone());
        }
        if marker.status != "prepared" {
            return Err(cognition_validation("commit_record_not_prepared"));
        }
        validate_marker_current_world(self, marker)?;
        let mut committed = marker.clone();
        committed.status = "committed".to_string();
        let mut next = projection;
        let records = next["commit_records"]
            .as_array_mut()
            .ok_or_else(|| cognition_validation("commit_records_not_array"))?;
        let record = records
            .iter_mut()
            .find(|record| record.get("commit_id").and_then(JsonValue::as_str) == Some(commit_id))
            .ok_or_else(|| cognition_validation("commit_record_missing"))?;
        *record = serde_json::to_value(&committed).map_err(WorldError::from)?;
        next["receipt_registry"]
            .as_array_mut()
            .ok_or_else(|| cognition_validation("receipt_registry_not_array"))?
            .push(json!({
                "receipt_id": committed.receipt_id,
                "receipt_digest": committed.receipt_digest,
            }));
        next["idempotency_index"][committed.envelope_idempotency_key.clone()] = json!({
            "envelope_digest": committed.envelope_digest,
            "disposition": "committed",
            "receipt_id": committed.receipt_id,
        });
        append_cognition_event(
            &mut next,
            "WorldReceiptLinked",
            json!({
                "envelope_idempotency_key": committed.envelope_idempotency_key,
                "receipt_id": committed.receipt_id,
                "agent_id": committed.agent_id,
                "agent_session_id": committed.agent_session_id,
                "agent_turn_id": committed.agent_turn_id,
                "decision_request_id": committed.decision_request_id,
                "request_digest": committed.request_digest,
                "feedback_id": committed.feedback_id,
            }),
        )?;
        if let Some(lineage) = RuntimeReceiptLineageV1::from_durable_commit_record(&committed) {
            next["receipt_lineage_registry"]
                .as_array_mut()
                .ok_or_else(|| cognition_validation("receipt_lineage_registry_not_array"))?
                .push(serde_json::to_value(lineage).map_err(WorldError::from)?);
        }
        let journal_head = next["cognition_journal"]["head_digest"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        if let Some(response) = next["responses"].as_array_mut().and_then(|responses| {
            responses.iter_mut().find(|response| {
                response.get("envelope_digest").and_then(JsonValue::as_str)
                    == Some(committed.envelope_digest.as_str())
            })
        }) {
            response["journal_head"] = json!(journal_head);
        }
        self.cognition = next;
        Ok(committed)
    }

    /// Runtime readback verifier for the opaque receipt projection consumed
    /// by simulator adapters.
    pub fn verify_runtime_receipt_lineage(
        &self,
        lineage: &RuntimeReceiptLineageV1,
    ) -> Result<(), WorldError> {
        let readback = self.read_runtime_receipt_lineage(&lineage.receipt_id)?;
        if &readback == lineage {
            Ok(())
        } else {
            Err(cognition_validation("receipt_lineage_readback_mismatch"))
        }
    }

    /// Read and validate a canonical response artifact without re-invoking a
    /// provider. Missing artifacts, digest mismatches, and stale journal
    /// heads are rejected.
    pub fn replay_cognition_response(
        &self,
        envelope_digest: &str,
    ) -> Result<CognitionResponseRecordV1, WorldError> {
        let projection: CognitionProjection = serde_json::from_value(self.cognition.clone())
            .map_err(|error| cognition_error("invalid_cognition_projection", error))?;
        let response = projection
            .responses
            .iter()
            .find(|response| response.envelope_digest == envelope_digest)
            .ok_or_else(|| cognition_validation("response_missing"))?;
        let projection_value = serde_json::to_value(&projection).map_err(WorldError::from)?;
        validate_response_record(&projection_value, response)?;
        Ok(response.clone())
    }

    pub(super) fn cognition_commit_scheduler_transaction(
        &mut self,
        scheduler: &CognitionScheduler,
        kind: &str,
        wake: Option<&SchedulerWakeV1>,
    ) -> Result<(), WorldError> {
        let mut next = self.cognition.clone();
        next["scheduler_state"] = scheduler.snapshot_json();
        let mut details = json!({
            "scheduler_policy_digest": scheduler.policy_config_digest(),
            "cursor_seq": scheduler.cursor_seq(),
        });
        if let Some(wake) = wake {
            let object = details
                .as_object_mut()
                .ok_or_else(|| cognition_validation("cognition_event_details_not_object"))?;
            object.extend(
                serde_json::to_value(wake)
                    .map_err(WorldError::from)?
                    .as_object()
                    .cloned()
                    .unwrap_or_default(),
            );
        }
        append_cognition_event(&mut next, kind, details)?;
        self.cognition = next;
        Ok(())
    }

    pub(super) fn cognition_commit_continuation_transaction(
        &mut self,
        continuations: &[AgentContinuation],
        scheduler: &CognitionScheduler,
        wake: &SchedulerWakeV1,
    ) -> Result<(), WorldError> {
        let mut next = self.cognition.clone();
        next["continuations"] = serde_json::to_value(continuations).map_err(WorldError::from)?;
        next["scheduler_state"] = scheduler.snapshot_json();
        append_cognition_event(
            &mut next,
            "ContinuationScheduled",
            json!({
                "continuation_id": wake.continuation_id,
                "wake_id": wake.wake_id,
                "world_id": wake.world_id,
                "agent_id": wake.agent_id,
                "agent_session_id": wake.agent_session_id,
                "agent_turn_id": wake.agent_turn_id,
                "decision_request_id": wake.decision_request_id,
                "scheduler_policy_digest": scheduler.policy_config_digest(),
                "cursor_seq": scheduler.cursor_seq(),
            }),
        )?;
        self.cognition = next;
        Ok(())
    }

    pub fn record_cognition_commit(
        &mut self,
        marker: WorldCommitRecordV1,
    ) -> Result<(), WorldError> {
        let _ = marker;
        return Err(cognition_validation("legacy_commit_projection_fenced"));
    }

    pub fn project_runtime_receipt_lineage(
        &mut self,
        lineage: RuntimeReceiptLineageV1,
    ) -> Result<(), WorldError> {
        lineage
            .validate()
            .map_err(|error| cognition_validation(error.code()))?;
        let projection = if self.cognition.is_null() {
            default_cognition_persistence_projection()
        } else {
            self.cognition.clone()
        };
        let mut parsed: CognitionProjection = serde_json::from_value(projection.clone())
            .map_err(|error| cognition_error("invalid_cognition_projection", error))?;
        let marker = parsed
            .commit_records
            .iter()
            .find(|marker| marker.status == "committed" && marker.receipt_id == lineage.receipt_id)
            .ok_or_else(|| cognition_validation("receipt_commit_marker_missing"))?;
        validate_lineage_binding(&parsed, marker, &lineage)?;
        if let Some(derived) = RuntimeReceiptLineageV1::from_durable_commit_record(marker) {
            if derived != lineage {
                return Err(cognition_validation("receipt_lineage_caller_override"));
            }
        }
        if let Some(existing) = parsed
            .receipt_lineage_registry
            .iter()
            .find(|existing| existing.receipt_id == lineage.receipt_id)
        {
            if existing == &lineage {
                return Ok(());
            }
            return Err(cognition_validation("receipt_lineage_conflict"));
        }
        parsed.receipt_lineage_registry.push(lineage);
        let mut projection = projection;
        projection
            .as_object_mut()
            .ok_or_else(|| cognition_validation("cognition_projection_not_object"))?
            .insert(
                "receipt_lineage_registry".to_string(),
                serde_json::to_value(parsed.receipt_lineage_registry).map_err(WorldError::from)?,
            );
        self.cognition = projection;
        Ok(())
    }

    pub fn read_runtime_receipt_lineage(
        &self,
        receipt_id: &str,
    ) -> Result<RuntimeReceiptLineageV1, WorldError> {
        let projection = if self.cognition.is_null() {
            default_cognition_persistence_projection()
        } else {
            self.cognition.clone()
        };
        let parsed: CognitionProjection = serde_json::from_value(projection)
            .map_err(|error| cognition_error("invalid_cognition_projection", error))?;
        let lineage = parsed
            .receipt_lineage_registry
            .iter()
            .find(|lineage| lineage.receipt_id == receipt_id)
            .ok_or_else(|| cognition_validation("receipt_lineage_missing"))?;
        lineage
            .validate()
            .map_err(|error| cognition_validation(error.code()))?;
        let marker = parsed
            .commit_records
            .iter()
            .find(|marker| marker.status == "committed" && marker.receipt_id == receipt_id)
            .ok_or_else(|| cognition_validation("receipt_commit_marker_missing"))?;
        validate_lineage_binding(&parsed, marker, lineage)?;
        if let Some(derived) = RuntimeReceiptLineageV1::from_durable_commit_record(marker) {
            if derived != *lineage {
                return Err(cognition_validation("receipt_lineage_binding_mismatch"));
            }
        }
        Ok(lineage.clone())
    }

    /// Reconcile a persisted cognition commit without invoking provider,
    /// kernel, effect, or debit code.
    pub fn recover_cognition(&mut self) -> Result<CognitionRecoveryReport, WorldError> {
        let mut projection = if self.cognition.is_null() {
            default_cognition_persistence_projection()
        } else {
            self.cognition.clone()
        };
        let mut parsed: CognitionProjection = serde_json::from_value(projection.clone())
            .map_err(|error| cognition_error("invalid_cognition_projection", error))?;

        if parsed.schema_version.is_empty() {
            return Ok(legacy_recovery_report());
        }
        if parsed.schema_version != PROJECTION_SCHEMA {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "cognition projection schema mismatch expected={PROJECTION_SCHEMA} actual={}",
                    parsed.schema_version
                ),
            });
        }
        if !parsed.cognition_journal.schema_version.is_empty()
            && parsed.cognition_journal.schema_version != JOURNAL_SCHEMA
        {
            return Err(WorldError::DistributedValidationFailed {
                reason: format!(
                    "cognition journal schema mismatch expected={JOURNAL_SCHEMA} actual={}",
                    parsed.cognition_journal.schema_version
                ),
            });
        }

        // A continuation that advertises an authoritative status digest must
        // be validated before restore exposes it to scheduling.  Snapshots
        // from before the digest field existed remain read-only legacy data;
        // they are not silently promoted to an authoritative projection.
        if let Some(value) = projection.get("continuations") {
            let continuations: Vec<AgentContinuation> = serde_json::from_value(value.clone())
                .map_err(|error| cognition_error("invalid_continuation_projection", error))?;
            for continuation in continuations {
                if continuation.continuation_status_digest.is_some() {
                    continuation
                        .validate_authoritative()
                        .map_err(|error| cognition_validation(error.code()))?;
                }
            }
        }

        let Some(marker) = select_commit_record(&parsed.commit_records) else {
            return Ok(legacy_recovery_report());
        };
        validate_marker(marker)?;
        let dense_marker = !marker.agent_id.is_empty();
        if dense_marker {
            validate_marker_current_world(self, marker)?;
        }

        let recovery = parsed.recovery.clone().unwrap_or_default();
        let root = if dense_marker {
            WorldRootViewV1 {
                schema_version: "world-root-view.v1".to_string(),
                world_id: marker.world_id.clone(),
                branch_id: marker.branch_id.clone(),
                logical_tick: self.state.time,
                state_root: self.current_state_root_hash()?,
                head_status: "canonical".to_string(),
                commit_id: (marker.status == "committed").then(|| marker.commit_id.clone()),
                quarantine_id: None,
            }
        } else {
            recovery
                .world_root
                .clone()
                .unwrap_or_else(|| parent_root(marker))
        };
        let response = parsed
            .responses
            .iter()
            .find(|response| response.envelope_digest == marker.envelope_digest);
        if dense_marker {
            let response = response.ok_or_else(|| cognition_validation("response_missing"))?;
            validate_response_record(&projection, response)?;
        }

        if recovery.crash_prefix == "conflict"
            || marker_root_conflict(marker, &root)
            || has_receipt_conflict(&parsed.receipt_registry, marker)
            || has_idempotency_conflict(&parsed.idempotency_index, marker)
            || response.is_some_and(|response| response.envelope_digest != marker.envelope_digest)
        {
            let report = conflict_report(marker, root);
            persist_recovery_report(&mut projection, &report)?;
            self.cognition = projection;
            return Ok(report);
        }

        if marker.status == "prepared" {
            let report = pending_report(marker, root);
            persist_recovery_report(&mut projection, &report)?;
            self.cognition = projection;
            return Ok(report);
        }

        // A committed marker is the authority.  If the root still points at
        // the parent (the crash window between commit and root projection),
        // repair that projection in the same in-memory transaction.
        let receipt = CognitionReceiptViewV1 {
            receipt_id: marker.receipt_id.clone(),
            receipt_digest: marker.receipt_digest.clone(),
        };
        let receipt_present = parsed.receipt_registry.iter().any(|candidate| {
            candidate.receipt_id == marker.receipt_id
                && candidate.receipt_digest == marker.receipt_digest
        });
        let idempotency_present = parsed
            .idempotency_index
            .get(&marker.envelope_idempotency_key)
            .is_some_and(|entry| {
                entry.envelope_digest == marker.envelope_digest
                    && entry.disposition == "committed"
                    && entry.receipt_id == marker.receipt_id
            });
        let linked_present = parsed.cognition_journal.events.iter().any(|event| {
            event.kind == "WorldReceiptLinked"
                && event.envelope_idempotency_key == marker.envelope_idempotency_key
                && event
                    .receipt_id
                    .as_deref()
                    .is_none_or(|receipt_id| receipt_id == marker.receipt_id)
        });

        let mut repaired_ids = recovery.repaired_projection_ids;
        let mut projection_repairs = 0u64;
        if !receipt_present {
            parsed.receipt_registry.push(receipt.clone());
            record_repair(&mut repaired_ids, "receipt_registry");
            projection_repairs += 1;
        }
        if !idempotency_present {
            parsed.idempotency_index.insert(
                marker.envelope_idempotency_key.clone(),
                IdempotencyProjection {
                    envelope_digest: marker.envelope_digest.clone(),
                    disposition: "committed".to_string(),
                    receipt_id: marker.receipt_id.clone(),
                },
            );
            record_repair(&mut repaired_ids, "idempotency_index");
            projection_repairs += 1;
        }
        if !linked_present {
            let next_seq = parsed
                .cognition_journal
                .head_seq
                .max(
                    parsed
                        .cognition_journal
                        .events
                        .iter()
                        .map(|event| event.journal_seq)
                        .max()
                        .unwrap_or(0),
                )
                .saturating_add(1);
            parsed.cognition_journal.events.push(CognitionJournalEvent {
                journal_seq: next_seq,
                kind: "WorldReceiptLinked".to_string(),
                envelope_idempotency_key: marker.envelope_idempotency_key.clone(),
                receipt_id: Some(marker.receipt_id.clone()),
                ..CognitionJournalEvent::default()
            });
            parsed.cognition_journal.head_seq = next_seq;
            record_repair(&mut repaired_ids, "world_receipt_link");
            projection_repairs += 1;
        }
        let mut canonical_root = root;
        if !dense_marker
            && (canonical_root.state_root == marker.parent_world_hash
                || canonical_root.commit_id.is_none()
                || canonical_root.head_status != "canonical"
                || canonical_root.quarantine_id.is_some())
        {
            canonical_root.logical_tick = marker.parent_tick.saturating_add(1);
            canonical_root.state_root = marker.staged_state_root.clone();
            canonical_root.head_status = "canonical".to_string();
            canonical_root.commit_id = Some(marker.commit_id.clone());
            canonical_root.quarantine_id = None;
            record_repair(&mut repaired_ids, "world_root");
        }
        let repaired_count = repaired_ids
            .iter()
            .filter(|id| {
                matches!(
                    id.as_str(),
                    "receipt_registry" | "idempotency_index" | "world_receipt_link"
                )
            })
            .count() as u64;
        if projection_repairs == 0 && repaired_count > 0 {
            // The durable repair markers make a second recovery exactly
            // idempotent while still requiring the projections above to be
            // structurally present and marker-bound.
            projection_repairs = repaired_count;
        }

        if !repaired_ids.is_empty() {
            let object = projection.as_object_mut().ok_or_else(|| {
                WorldError::DistributedValidationFailed {
                    reason: "cognition projection must be an object".to_string(),
                }
            })?;
            if !receipt_present {
                let receipts = object
                    .entry("receipt_registry")
                    .or_insert_with(|| JsonValue::Array(Vec::new()));
                receipts
                    .as_array_mut()
                    .ok_or_else(|| WorldError::DistributedValidationFailed {
                        reason: "cognition receipt_registry must be an array".to_string(),
                    })?
                    .push(json!(&receipt));
            }
            if !idempotency_present {
                object
                    .entry("idempotency_index")
                    .or_insert_with(|| JsonValue::Object(serde_json::Map::new()));
                object
                    .get_mut("idempotency_index")
                    .and_then(JsonValue::as_object_mut)
                    .ok_or_else(|| WorldError::DistributedValidationFailed {
                        reason: "cognition idempotency_index must be an object".to_string(),
                    })?
                    .insert(
                        marker.envelope_idempotency_key.clone(),
                        json!({
                            "envelope_digest": marker.envelope_digest.clone(),
                            "disposition": "committed",
                            "receipt_id": marker.receipt_id.clone(),
                        }),
                    );
            }
            if !linked_present {
                let journal = object.entry("cognition_journal").or_insert_with(|| {
                    json!({
                        "schema_version": JOURNAL_SCHEMA,
                        "head_seq": 0,
                        "head_digest": "",
                        "events": []
                    })
                });
                let journal = journal.as_object_mut().ok_or_else(|| {
                    WorldError::DistributedValidationFailed {
                        reason: "cognition_journal must be an object".to_string(),
                    }
                })?;
                journal
                    .entry("events")
                    .or_insert_with(|| JsonValue::Array(Vec::new()));
                journal
                    .get_mut("events")
                    .and_then(JsonValue::as_array_mut)
                    .ok_or_else(|| WorldError::DistributedValidationFailed {
                        reason: "cognition journal events must be an array".to_string(),
                    })?
                    .push(json!({
                        "journal_seq": parsed.cognition_journal.head_seq.max(
                            parsed.cognition_journal.events.iter().map(|event| event.journal_seq).max().unwrap_or(0),
                        ).saturating_add(1),
                        "kind": "WorldReceiptLinked",
                        "envelope_idempotency_key": marker.envelope_idempotency_key.clone(),
                        "receipt_id": marker.receipt_id.clone(),
                    }));
                journal.insert(
                    "head_seq".to_string(),
                    json!(
                        parsed
                            .cognition_journal
                            .head_seq
                            .max(
                                parsed
                                    .cognition_journal
                                    .events
                                    .iter()
                                    .map(|event| event.journal_seq)
                                    .max()
                                    .unwrap_or(0),
                            )
                            .saturating_add(1)
                    ),
                );
            }
            object
                .entry("recovery")
                .or_insert_with(|| JsonValue::Object(serde_json::Map::new()));
            if let Some(recovery) = object
                .get_mut("recovery")
                .and_then(JsonValue::as_object_mut)
            {
                if repaired_ids.iter().any(|id| id == "world_root") {
                    recovery.insert("world_root".to_string(), json!(&canonical_root));
                }
                recovery.insert("repaired_projection_ids".to_string(), json!(repaired_ids));
            }
            self.cognition = projection;
        }

        let journal_head = response
            .map(|response| response.journal_head.clone())
            .filter(|head| !head.is_empty())
            .unwrap_or_else(|| parsed.cognition_journal.head_digest.clone());
        let response_replayed = response.is_some();
        Ok(CognitionRecoveryReport {
            world_root: Some(canonical_root),
            receipt: Some(receipt),
            disposition: "committed".to_string(),
            reject_reason: None,
            auto_submitted: false,
            idempotency_key: Some(marker.envelope_idempotency_key.clone()),
            quarantine_id: None,
            candidate_root: None,
            candidate_receipt: None,
            journal_head,
            retry_count: 0,
            revalidation_count: 0,
            projection_repairs,
            provider_invocation_count: 0,
            kernel_invocation_count: 0,
            effect_count: 0,
            debit_count: 0,
            world_receipt_linked_count: 1,
            event_count: 1,
            response_replayed,
        })
    }
}

fn validate_lineage_binding(
    projection: &CognitionProjection,
    marker: &WorldCommitRecordV1,
    lineage: &RuntimeReceiptLineageV1,
) -> Result<(), WorldError> {
    if marker.receipt_digest != lineage.receipt_digest
        || marker.envelope_digest != lineage.envelope_digest
        || marker.action_id != lineage.action_id
        || !projection.receipt_registry.iter().any(|receipt| {
            receipt.receipt_id == marker.receipt_id
                && receipt.receipt_digest == marker.receipt_digest
        })
        || !projection
            .idempotency_index
            .get(&marker.envelope_idempotency_key)
            .is_some_and(|entry| {
                entry.envelope_digest == marker.envelope_digest
                    && entry.disposition == "committed"
                    && entry.receipt_id == marker.receipt_id
            })
        || !projection.cognition_journal.events.iter().any(|event| {
            event.kind == "WorldReceiptLinked"
                && event.envelope_idempotency_key == marker.envelope_idempotency_key
                && event
                    .receipt_id
                    .as_deref()
                    .is_none_or(|receipt_id| receipt_id == marker.receipt_id)
                && event
                    .agent_id
                    .as_deref()
                    .is_none_or(|agent_id| agent_id == lineage.agent_id)
                && event
                    .agent_session_id
                    .as_deref()
                    .is_none_or(|session_id| session_id == lineage.agent_session_id)
                && event
                    .agent_turn_id
                    .as_deref()
                    .is_none_or(|turn_id| turn_id == lineage.agent_turn_id)
                && event
                    .decision_request_id
                    .as_deref()
                    .is_none_or(|request_id| request_id == lineage.decision_request_id)
                && event
                    .request_digest
                    .as_deref()
                    .is_none_or(|request_digest| request_digest == lineage.request_digest)
                && event
                    .feedback_id
                    .as_deref()
                    .is_none_or(|feedback_id| feedback_id == lineage.feedback_id)
        })
    {
        return Err(cognition_validation("receipt_lineage_binding_mismatch"));
    }
    if !marker.agent_id.is_empty()
        && (marker.agent_id != lineage.agent_id
            || marker.agent_session_id != lineage.agent_session_id
            || marker.agent_turn_id != lineage.agent_turn_id
            || marker.decision_request_id != lineage.decision_request_id
            || marker.request_digest != lineage.request_digest
            || marker.feedback_id != lineage.feedback_id)
    {
        return Err(cognition_validation("receipt_lineage_binding_mismatch"));
    }
    if !marker.agent_id.is_empty()
        && !projection.cognition_journal.events.iter().any(|event| {
            event.kind == "WorldReceiptLinked"
                && event.envelope_idempotency_key == marker.envelope_idempotency_key
                && event.receipt_id.as_deref() == Some(marker.receipt_id.as_str())
                && event.agent_id.as_deref() == Some(marker.agent_id.as_str())
                && event.agent_session_id.as_deref() == Some(marker.agent_session_id.as_str())
                && event.agent_turn_id.as_deref() == Some(marker.agent_turn_id.as_str())
                && event.decision_request_id.as_deref() == Some(marker.decision_request_id.as_str())
                && event.request_digest.as_deref() == Some(marker.request_digest.as_str())
                && event.feedback_id.as_deref() == Some(marker.feedback_id.as_str())
        })
    {
        return Err(cognition_validation("receipt_lineage_binding_mismatch"));
    }
    Ok(())
}

fn select_commit_record(records: &[WorldCommitRecordV1]) -> Option<&WorldCommitRecordV1> {
    records
        .iter()
        .max_by_key(|record| (record.cognition_journal_seq, record.commit_id.as_str()))
}

fn validate_marker(marker: &WorldCommitRecordV1) -> Result<(), WorldError> {
    if marker.schema_version != WORLD_COMMIT_SCHEMA
        || marker.commit_id.trim().is_empty()
        || marker.envelope_idempotency_key.trim().is_empty()
        || marker.envelope_digest.trim().is_empty()
        || marker.world_id.trim().is_empty()
        || marker.branch_id.trim().is_empty()
        || marker.finality_block_hash.trim().is_empty()
        || marker.finality_status.trim().is_empty()
        || marker.finality_binding_digest.trim().is_empty()
        || marker.runtime_manifest_hash.trim().is_empty()
        || marker.action_id.trim().is_empty()
        || marker.parent_world_hash.trim().is_empty()
        || marker.staged_event_root.trim().is_empty()
        || marker.staged_state_root.trim().is_empty()
        || marker.receipt_id.trim().is_empty()
        || marker.receipt_digest.trim().is_empty()
        || marker.abort_reason.is_some()
        || !matches!(marker.status.as_str(), "prepared" | "committed")
    {
        return Err(WorldError::DistributedValidationFailed {
            reason: "invalid_commit_record".to_string(),
        });
    }
    let dense = [
        &marker.agent_id,
        &marker.agent_session_id,
        &marker.agent_turn_id,
        &marker.decision_request_id,
        &marker.request_digest,
        &marker.feedback_id,
    ];
    if marker.agent_id.is_empty() != dense.iter().all(|value| value.is_empty()) {
        return Err(WorldError::DistributedValidationFailed {
            reason: "invalid_commit_record_identity".to_string(),
        });
    }
    if !marker.agent_id.is_empty() && dense.iter().any(|value| value.trim().is_empty()) {
        return Err(WorldError::DistributedValidationFailed {
            reason: "invalid_commit_record_identity".to_string(),
        });
    }
    Ok(())
}

fn marker_root_conflict(marker: &WorldCommitRecordV1, root: &WorldRootViewV1) -> bool {
    if root.world_id != marker.world_id || root.branch_id != marker.branch_id {
        return true;
    }
    let parent = root.state_root == marker.parent_world_hash
        && root.logical_tick == marker.parent_tick
        && root.commit_id.is_none();
    let next = root.state_root == marker.staged_state_root
        && root.logical_tick == marker.parent_tick.saturating_add(1)
        && root.commit_id.as_deref() == Some(marker.commit_id.as_str());
    let stable = marker.status == "committed"
        && root.state_root == marker.staged_state_root
        && root.logical_tick == marker.parent_tick
        && root.commit_id.as_deref() == Some(marker.commit_id.as_str());
    !parent && !next && !stable
}

fn has_receipt_conflict(receipts: &[CognitionReceiptViewV1], marker: &WorldCommitRecordV1) -> bool {
    receipts.iter().any(|receipt| {
        receipt.receipt_id == marker.receipt_id && receipt.receipt_digest != marker.receipt_digest
    })
}

fn has_idempotency_conflict(
    index: &BTreeMap<String, IdempotencyProjection>,
    marker: &WorldCommitRecordV1,
) -> bool {
    index
        .get(&marker.envelope_idempotency_key)
        .is_some_and(|entry| {
            entry.envelope_digest != marker.envelope_digest
                || (entry.disposition == "committed" && entry.receipt_id != marker.receipt_id)
        })
}

fn pending_report(marker: &WorldCommitRecordV1, root: WorldRootViewV1) -> CognitionRecoveryReport {
    let mut root = root;
    root.head_status = "recovery_pending".to_string();
    root.commit_id = None;
    root.quarantine_id = None;
    CognitionRecoveryReport {
        world_root: Some(root),
        receipt: None,
        disposition: "recovery_pending".to_string(),
        reject_reason: None,
        auto_submitted: false,
        idempotency_key: Some(marker.envelope_idempotency_key.clone()),
        quarantine_id: None,
        candidate_root: None,
        candidate_receipt: None,
        journal_head: String::new(),
        retry_count: 0,
        revalidation_count: 1,
        projection_repairs: 0,
        provider_invocation_count: 0,
        kernel_invocation_count: 0,
        effect_count: 0,
        debit_count: 0,
        world_receipt_linked_count: 0,
        event_count: 0,
        response_replayed: false,
    }
}

fn conflict_report(marker: &WorldCommitRecordV1, root: WorldRootViewV1) -> CognitionRecoveryReport {
    let quarantine_id = format!("quarantine:{}", marker.commit_id);
    let mut root = root;
    root.state_root = marker.parent_world_hash.clone();
    root.logical_tick = marker.parent_tick;
    root.head_status = "recovery_pending".to_string();
    root.commit_id = None;
    root.quarantine_id = Some(quarantine_id.clone());
    CognitionRecoveryReport {
        world_root: Some(root),
        receipt: None,
        disposition: "recovery_pending".to_string(),
        reject_reason: Some("commit_conflict".to_string()),
        auto_submitted: false,
        idempotency_key: Some(marker.envelope_idempotency_key.clone()),
        quarantine_id: Some(quarantine_id),
        candidate_root: Some(marker.staged_state_root.clone()),
        candidate_receipt: Some(CognitionReceiptViewV1 {
            receipt_id: marker.receipt_id.clone(),
            receipt_digest: marker.receipt_digest.clone(),
        }),
        journal_head: String::new(),
        retry_count: 0,
        revalidation_count: 0,
        projection_repairs: 0,
        provider_invocation_count: 0,
        kernel_invocation_count: 0,
        effect_count: 0,
        debit_count: 0,
        world_receipt_linked_count: 0,
        event_count: 0,
        response_replayed: false,
    }
}

fn record_repair(repaired_ids: &mut Vec<String>, id: &str) {
    if !repaired_ids.iter().any(|existing| existing == id) {
        repaired_ids.push(id.to_string());
    }
}
