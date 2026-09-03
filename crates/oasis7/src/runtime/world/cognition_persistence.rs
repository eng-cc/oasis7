#[path = "cognition_persistence_authority.rs"]
mod cognition_persistence_authority;
#[path = "cognition_persistence_recovery.rs"]
mod cognition_persistence_recovery;
#[path = "cognition_persistence_reports.rs"]
mod cognition_persistence_reports;
#[path = "cognition_persistence_support.rs"]
mod cognition_persistence_support;
#[path = "cognition_persistence_transactions.rs"]
mod cognition_persistence_transactions;

use super::World;
use super::cognition_persistence_validation::{
    append_cognition_event, cognition_error, cognition_validation, validate_cognition_journal_head,
    validate_cognition_journal_integrity, validate_marker_current_world,
    validate_response_lineage_binding, validate_response_record,
};
use crate::runtime::cognition::{
    AgentDecisionEnvelopeV1, MvccValidator, world_state_binding_digest_v1,
};
use crate::runtime::cognition_recovery::{
    CognitionReceiptViewV1, CognitionResponseRecordV1, RuntimeReceiptLineageV1,
    WorldCommitRecordV1, WorldRootViewV1, cognition_digest_v1,
    default_cognition_persistence_projection, response_artifact_digest,
};
use crate::runtime::error::WorldError;
use cognition_persistence_support::{stale_base_error, validate_lineage_binding, validate_marker};
use serde::{Deserialize, Serialize};
use serde_json::{Value as JsonValue, json};
use std::collections::BTreeMap;

const PROJECTION_SCHEMA: &str = "cognition-persistence.v1";
const JOURNAL_SCHEMA: &str = "cognition-journal.v1";
const WORLD_COMMIT_SCHEMA: &str = "world-commit-record.v1";
const RUNTIME_BINDING_DIGEST_DOMAIN: &str = "oasis7.runtime.manifest.v1";
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
    #[serde(default)]
    staged_actions: BTreeMap<String, JsonValue>,
    #[serde(flatten)]
    extra: BTreeMap<String, JsonValue>,
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
    schema_version: String,
    #[serde(default)]
    journal_seq: u64,
    #[serde(default)]
    parent_event_digest: String,
    #[serde(default)]
    event_kind: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    world_id: String,
    #[serde(default)]
    branch_id: String,
    #[serde(default)]
    finality_epoch: u64,
    #[serde(default)]
    finality_block_hash: Option<String>,
    #[serde(default)]
    finality_status: String,
    #[serde(default)]
    reorg_epoch: u64,
    #[serde(default)]
    logical_tick: u64,
    #[serde(default)]
    envelope_idempotency_key: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    receipt_id: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_id: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_session_id: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_turn_id: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    decision_request_id: Option<String>,
    #[serde(default)]
    request_digest: Option<String>,
    #[serde(default)]
    response_digest: Option<String>,
    #[serde(default)]
    envelope_digest: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    feedback_id: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    continuation_id: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    wake_id: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    scheduler_policy_digest: Option<String>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor_seq: Option<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    event_digest: Option<String>,
    #[serde(default)]
    retry_seq: u64,
    #[serde(default)]
    transport_attempt: u64,
    #[serde(default)]
    status: String,
    #[serde(default)]
    payload_digest: String,
    #[serde(default)]
    causal_refs: Vec<String>,
    #[serde(flatten)]
    extra: BTreeMap<String, JsonValue>,
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
    #[serde(flatten)]
    extra: BTreeMap<String, JsonValue>,
}

pub(super) fn canonical_runtime_binding_digest(raw_hash: &str) -> String {
    cognition_digest_v1(RUNTIME_BINDING_DIGEST_DOMAIN, &raw_hash)
}

pub(super) struct CognitionRuntimeAuthority {
    world_id: String,
    branch_id: String,
    finality_epoch: u64,
    finality_block_hash: Option<String>,
    finality_status: String,
    reorg_epoch: u64,
    runtime_manifest_hash: String,
    base_world_hash: String,
}
impl World {
    pub fn prepare_cognition_envelope(
        &mut self,
        envelope: AgentDecisionEnvelopeV1,
        response_artifact: Option<JsonValue>,
    ) -> Result<WorldCommitRecordV1, WorldError> {
        let mut transaction = self.clone();
        let result = transaction.prepare_cognition_envelope_inner(envelope, response_artifact);
        match result {
            Ok(marker) => {
                transaction.persist_runtime_transaction_if_configured()?;
                *self = transaction;
                Ok(marker)
            }
            Err(error) => {
                if transaction.cognition != self.cognition {
                    transaction.persist_runtime_transaction_if_configured()?;
                    *self = transaction;
                }
                Err(error)
            }
        }
    }

    fn prepare_cognition_envelope_inner(
        &mut self,
        envelope: AgentDecisionEnvelopeV1,
        response_artifact: Option<JsonValue>,
    ) -> Result<WorldCommitRecordV1, WorldError> {
        let response_identity = response_artifact
            .as_ref()
            .map(|artifact| {
                cognition_persistence_support::validate_prepare_response_artifact(
                    &envelope, artifact,
                )
            })
            .transpose()?;
        // Resolve an exact durable marker before checking the current head.
        // A retry of a committed envelope is allowed to read its original
        // receipt even after that commit advanced the World state.
        let existing_projection = if self.cognition.is_null() {
            default_cognition_persistence_projection()
        } else {
            self.cognition.clone()
        };
        let existing_parsed: CognitionProjection = serde_json::from_value(existing_projection)
            .map_err(|error| cognition_error("invalid_cognition_projection", error))?;
        if let Some(existing) = existing_parsed
            .commit_records
            .iter()
            .find(|record| record.envelope_idempotency_key == envelope.envelope_idempotency_key)
        {
            if existing.envelope_digest != envelope.envelope_digest {
                return Err(cognition_validation("envelope_idempotency_conflict"));
            }
            if let Some(artifact) = response_artifact.as_ref() {
                let response_matches = existing_parsed.responses.iter().any(|response| {
                    response.envelope_digest == existing.envelope_digest
                        && response.response_artifact.as_ref() == Some(&artifact)
                });
                if !response_matches {
                    return Err(cognition_validation("response_artifact_lineage_mismatch"));
                }
            }
            return Ok(existing.clone());
        }
        if self.has_recorded_stale_cognition_rejection(&envelope)? {
            return Err(stale_base_error());
        }
        if let Err(error) = MvccValidator::validate(self, &envelope) {
            if error.code() == "stale_base" {
                self.record_stale_cognition_rejection(
                    &envelope.agent_id,
                    &envelope.agent_session_id,
                    &envelope.agent_turn_id,
                    &envelope.decision_request_id,
                    &envelope.request_digest,
                    &envelope.envelope_digest,
                    &envelope.envelope_idempotency_key,
                )?;
                return Err(stale_base_error());
            }
            return Err(cognition_validation(error.code()));
        }
        serde_json::from_value::<crate::runtime::Action>(envelope.action.clone())
            .map_err(|error| cognition_error("cognition_action_invalid", error))?;
        let world_root = self.current_state_root_hash()?;
        let manifest_hash = self.current_manifest_hash()?;
        let binding_was_unbound = self.cognition_runtime_is_unbound();
        let expected_world_hash = if binding_was_unbound {
            world_root.clone()
        } else {
            world_state_binding_digest_v1(
                &envelope.world_id,
                &envelope.branch_id,
                envelope.finality_epoch,
                envelope.finality_block_hash.as_deref(),
                &envelope.finality_status,
                self.state.time,
                &world_root,
                envelope.reorg_epoch,
                &manifest_hash,
            )
        };
        let expected_manifest_hash = if binding_was_unbound {
            manifest_hash.clone()
        } else {
            cognition_digest_v1(RUNTIME_BINDING_DIGEST_DOMAIN, &manifest_hash)
        };
        if envelope.base_tick != self.state.time
            || envelope.base_world_hash != expected_world_hash
            || envelope.runtime_manifest_hash != expected_manifest_hash
        {
            self.record_stale_cognition_rejection(
                &envelope.agent_id,
                &envelope.agent_session_id,
                &envelope.agent_turn_id,
                &envelope.decision_request_id,
                &envelope.request_digest,
                &envelope.envelope_digest,
                &envelope.envelope_idempotency_key,
            )?;
            return Err(stale_base_error());
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
        let finality_block_hash = envelope.finality_block_hash.clone();
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
            response_context_discriminator: response_identity
                .as_ref()
                .map_or_else(String::new, |identity| {
                    identity.context_discriminator.clone()
                }),
            response_context_version: response_identity
                .as_ref()
                .map_or(0, |identity| identity.context_version),
            response_retry_seq: response_identity
                .as_ref()
                .map_or(envelope.retry_seq, |identity| identity.retry_seq),
            transport_attempt: response_identity
                .as_ref()
                .map_or(0, |identity| identity.transport_attempt),
            response_artifact_digest: response_artifact
                .as_ref()
                .map_or_else(String::new, response_artifact_digest),
            abort_reason: None,
        };
        let mut marker = marker;
        marker.receipt_digest = cognition_persistence_support::receipt_digest_for_marker(&marker);
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
            .entry("staged_actions")
            .or_insert_with(|| JsonValue::Object(serde_json::Map::new()))
            .as_object_mut()
            .ok_or_else(|| cognition_validation("staged_actions_not_object"))?
            .insert(marker.commit_id.clone(), envelope.action.clone());
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
        let mut transaction = self.clone();
        let committed = transaction.finalize_cognition_commit_inner(commit_id)?;
        // Receipt durability is committed evidence. Re-evaluate untimed
        // receipt/event/state wakes in the same World transaction so queue
        // service does not depend on a later tick or starvation timeout.
        transaction.reevaluate_cognition_wakes_after_commit()?;
        transaction.persist_runtime_transaction_if_configured()?;
        *self = transaction;
        Ok(committed)
    }

    /// Abort a prepared cognition commit at a durable terminal boundary.
    /// Aborts discard the staged action and record cancellation/failure
    /// evidence without publishing a receipt or advancing World state.
    pub fn abort_cognition_commit(
        &mut self,
        commit_id: &str,
        reason: &str,
    ) -> Result<WorldCommitRecordV1, WorldError> {
        if !matches!(
            reason,
            "stale_base"
                | "cancelled"
                | "late_response_after_cancel"
                | "recovery_operator_abort"
                | "reorg_invalidated"
        ) {
            return Err(cognition_validation("cognition_abort_reason_invalid"));
        }
        let mut transaction = self.clone();
        let projection = transaction.cognition.clone();
        let parsed: CognitionProjection = serde_json::from_value(projection.clone())
            .map_err(|error| cognition_error("invalid_cognition_projection", error))?;
        let marker = parsed
            .commit_records
            .iter()
            .find(|record| record.commit_id == commit_id)
            .cloned()
            .ok_or_else(|| cognition_validation("commit_record_missing"))?;
        if marker.status == "aborted" {
            if marker.abort_reason.as_deref() == Some(reason) {
                return Ok(marker.clone());
            }
            if reason == "late_response_after_cancel"
                && marker.abort_reason.as_deref() == Some("cancelled")
            {
                let aborted_marker = marker.clone();
                let late_response = json!({
                    "status": "rejected",
                    "reject_reason": reason,
                    "envelope_idempotency_key": &marker.envelope_idempotency_key,
                    "envelope_digest": &marker.envelope_digest,
                    "receipt_id": &marker.receipt_id,
                    "agent_id": &marker.agent_id,
                    "agent_session_id": &marker.agent_session_id,
                    "agent_turn_id": &marker.agent_turn_id,
                    "decision_request_id": &marker.decision_request_id,
                    "request_digest": &marker.request_digest,
                    "feedback_id": &marker.feedback_id,
                });
                append_cognition_event(
                    &mut transaction.cognition,
                    "LateResponseRejected",
                    late_response,
                )?;
                transaction.persist_runtime_transaction_if_configured()?;
                *self = transaction;
                return Ok(aborted_marker);
            }
            return Err(cognition_validation("cognition_abort_conflict"));
        }
        if marker.status != "prepared" {
            return Err(cognition_validation("commit_record_not_prepared"));
        }
        let mut aborted = marker.clone();
        aborted.status = "aborted".to_string();
        aborted.abort_reason = Some(reason.to_string());
        cognition_persistence_support::validate_marker(&aborted)?;
        let records = transaction
            .cognition
            .get_mut("commit_records")
            .and_then(JsonValue::as_array_mut)
            .ok_or_else(|| cognition_validation("commit_records_not_array"))?;
        let record = records
            .iter_mut()
            .find(|record| record.get("commit_id").and_then(JsonValue::as_str) == Some(commit_id))
            .ok_or_else(|| cognition_validation("commit_record_missing"))?;
        *record = serde_json::to_value(&aborted).map_err(WorldError::from)?;
        transaction
            .cognition
            .get_mut("staged_actions")
            .and_then(JsonValue::as_object_mut)
            .ok_or_else(|| cognition_validation("staged_actions_not_object"))?
            .remove(commit_id);
        transaction.cognition["idempotency_index"][aborted.envelope_idempotency_key.clone()] = json!({
            "envelope_digest": &aborted.envelope_digest,
            "disposition": "aborted",
            "receipt_id": &aborted.receipt_id,
        });
        let event_details = json!({
            "status": "rejected",
            "reject_reason": reason,
            "envelope_idempotency_key": &aborted.envelope_idempotency_key,
            "envelope_digest": &aborted.envelope_digest,
            "receipt_id": &aborted.receipt_id,
            "agent_id": &aborted.agent_id,
            "agent_session_id": &aborted.agent_session_id,
            "agent_turn_id": &aborted.agent_turn_id,
            "decision_request_id": &aborted.decision_request_id,
            "request_digest": &aborted.request_digest,
            "feedback_id": &aborted.feedback_id,
        });
        if reason == "late_response_after_cancel" {
            // A late provider response is evidence against an already
            // terminal turn, never a new response-bearing turn. Keep the
            // rejection as its own durable event so recovery can distinguish
            // it from the cancellation that closed the original lease.
            append_cognition_event(
                &mut transaction.cognition,
                "LateResponseRejected",
                event_details.clone(),
            )?;
        }
        append_cognition_event(
            &mut transaction.cognition,
            "CognitionTurnCancelled",
            event_details.clone(),
        )?;
        append_cognition_event(
            &mut transaction.cognition,
            "CognitionTurnCompleted",
            event_details,
        )?;
        transaction.persist_runtime_transaction_if_configured()?;
        *self = transaction;
        Ok(aborted)
    }

    pub(super) fn reevaluate_cognition_wakes_after_commit(&mut self) -> Result<(), WorldError> {
        let has_scheduler = self
            .cognition
            .get("scheduler_state")
            .is_some_and(|state| !state.is_null());
        if has_scheduler {
            // A commit changes receipt, journal-event and state evidence. It
            // may immediately lease only wakes whose conditions depend on
            // that evidence; time-only wakes remain under the normal tick
            // service path.
            let _ = self.select_ready_cognition_wakes_inner(self.state.time, true)?;
        }
        Ok(())
    }

    fn finalize_cognition_commit_inner(
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
        if !self.pending_actions.is_empty() {
            return Err(cognition_validation("cognition_pending_actions"));
        }
        if !marker.agent_id.is_empty() {
            let response = parsed
                .responses
                .iter()
                .find(|response| response.envelope_digest == marker.envelope_digest)
                .ok_or_else(|| cognition_validation("response_missing"))?;
            validate_response_lineage_binding(response, marker)?;
        }
        let action_value = parsed
            .staged_actions
            .get(commit_id)
            .cloned()
            .ok_or_else(|| cognition_validation("cognition_staged_action_missing"))?;
        let action_digest = cognition_digest_v1("oasis7.cognition.action.v1", &action_value);
        let action = serde_json::from_value::<crate::runtime::Action>(action_value)
            .map_err(|error| cognition_error("cognition_action_invalid", error))?;

        // The World clone is the transaction workspace.  `step` applies the
        // decoded action, event journal and derived state together; the
        // workspace becomes visible only after all cognition projections have
        // been updated below.
        let mut staged_world = self.clone();
        let action_id = marker
            .action_id
            .strip_prefix("action:")
            .and_then(|id| id.parse::<u64>().ok())
            .ok_or_else(|| cognition_validation("cognition_action_id_invalid"))?;
        staged_world.submit_action_with_id(action_id, action);
        staged_world.step()?;
        let staged_state_root = staged_world.current_state_root_hash()?;
        let mut committed = marker.clone();
        committed.status = "committed".to_string();
        committed.staged_state_root = staged_state_root;
        let mut next = projection;
        next["staged_actions"]
            .as_object_mut()
            .ok_or_else(|| cognition_validation("staged_actions_not_object"))?
            .remove(commit_id);
        if !next.get("action_digests").is_some_and(JsonValue::is_object) {
            next.as_object_mut()
                .ok_or_else(|| cognition_validation("cognition_projection_not_object"))?
                .insert("action_digests".to_string(), json!({}));
        }
        next.get_mut("action_digests")
            .and_then(JsonValue::as_object_mut)
            .ok_or_else(|| cognition_validation("action_digests_not_object"))?
            .insert(committed.commit_id.clone(), json!(action_digest));
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
        append_cognition_event(
            &mut next,
            "CognitionTurnCompleted",
            json!({
                "status": "committed",
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
        staged_world.cognition = next;
        *self = staged_world;
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
        let projection_value = serde_json::to_value(&projection).map_err(WorldError::from)?;
        let journal = projection_value
            .get("cognition_journal")
            .ok_or_else(|| cognition_validation("cognition_journal_missing"))?;
        validate_cognition_journal_integrity(journal)?;
        validate_cognition_journal_head(journal)?;
        let response = projection
            .responses
            .iter()
            .find(|response| response.envelope_digest == envelope_digest)
            .ok_or_else(|| cognition_validation("response_missing"))?;
        validate_response_record(&projection_value, response)?;
        let marker = projection
            .commit_records
            .iter()
            .find(|marker| {
                marker.status == "committed" && marker.envelope_digest == envelope_digest
            })
            .ok_or_else(|| cognition_validation("response_commit_marker_missing"))?;
        validate_marker(marker)?;
        validate_response_lineage_binding(response, marker)?;
        Ok(response.clone())
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
        if RuntimeReceiptLineageV1::from_durable_commit_record(marker).is_none() {
            return Err(cognition_validation(
                "receipt_lineage_dense_marker_required",
            ));
        }
        validate_lineage_binding(&parsed, marker, &lineage)?;
        let derived = RuntimeReceiptLineageV1::from_durable_commit_record(marker)
            .expect("dense marker checked above");
        if derived != lineage {
            return Err(cognition_validation("receipt_lineage_caller_override"));
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
        validate_marker(marker)?;
        if let Some(response) = parsed
            .responses
            .iter()
            .find(|response| response.envelope_digest == marker.envelope_digest)
        {
            validate_response_lineage_binding(response, marker)?;
        }
        validate_lineage_binding(&parsed, marker, lineage)?;
        let derived = RuntimeReceiptLineageV1::from_durable_commit_record(marker)
            .ok_or_else(|| cognition_validation("receipt_lineage_dense_marker_required"))?;
        if derived != *lineage {
            return Err(cognition_validation("receipt_lineage_binding_mismatch"));
        }
        Ok(lineage.clone())
    }

    pub(super) fn cognition_committed_evidence(
        &self,
    ) -> Result<
        (
            std::collections::BTreeSet<String>,
            std::collections::BTreeSet<String>,
        ),
        WorldError,
    > {
        let projection = if self.cognition.is_null() {
            default_cognition_persistence_projection()
        } else {
            self.cognition.clone()
        };
        let parsed: CognitionProjection = serde_json::from_value(projection.clone())
            .map_err(|error| cognition_error("invalid_cognition_projection", error))?;
        let mut committed_receipts = std::collections::BTreeSet::new();
        for marker in parsed
            .commit_records
            .iter()
            .filter(|marker| marker.status == "committed")
        {
            validate_marker(marker)?;
            if parsed.receipt_registry.iter().any(|receipt| {
                receipt.receipt_id == marker.receipt_id
                    && receipt.receipt_digest == marker.receipt_digest
            }) {
                committed_receipts.insert(marker.receipt_id.clone());
            }
        }

        let mut committed_events = std::collections::BTreeSet::new();
        let events = projection
            .get("cognition_journal")
            .and_then(|journal| journal.get("events"))
            .and_then(JsonValue::as_array)
            .ok_or_else(|| cognition_validation("cognition_events_missing"))?;
        if !events.is_empty() {
            let journal = projection
                .get("cognition_journal")
                .ok_or_else(|| cognition_validation("cognition_journal_missing"))?;
            // Wake evidence is authoritative only after the same dense
            // journal and head checks used by recovery/replay. A forged
            // event that happens to carry a self-consistent digest must not
            // satisfy a live wake by bypassing those checks.
            validate_cognition_journal_integrity(journal)?;
            validate_cognition_journal_head(journal)?;
        }
        for event in events {
            let Some(event_digest) = event.get("event_digest").and_then(JsonValue::as_str) else {
                continue;
            };
            let mut unsigned = event.clone();
            let Some(object) = unsigned.as_object_mut() else {
                return Err(cognition_validation("cognition_event_not_object"));
            };
            object.remove("event_digest");
            let expected = cognition_digest_v1("oasis7.cognition.event.v1", &unsigned);
            if expected != event_digest {
                return Err(cognition_validation("cognition_event_digest_mismatch"));
            }
            committed_events.insert(event_digest.to_string());
        }
        Ok((committed_events, committed_receipts))
    }
}
