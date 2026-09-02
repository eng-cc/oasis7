//! World-owned durable feedback delivery state.
//!
//! Provider transports are allowed to fail or be restarted. The Runtime keeps
//! the exact feedback envelope and its delivery lease in the cognition
//! projection, so retrying delivery never requires re-running cognition.

use super::World;
use crate::runtime::cognition_recovery::{
    RuntimeFeedbackOutboxRecordV1, default_cognition_persistence_projection,
};
use crate::runtime::error::WorldError;
use crate::runtime::world::cognition_persistence_validation::append_cognition_event;
use crate::simulator::FeedbackEnvelopeV1;
use serde_json::{Value as JsonValue, json};
use std::collections::BTreeMap;

const OUTBOX_FIELD: &str = "feedback_outbox";

fn feedback_error(code: impl Into<String>) -> WorldError {
    WorldError::CapabilityAuthorizationDenied {
        reason: code.into(),
    }
}

fn parse_outbox(
    projection: &JsonValue,
) -> Result<BTreeMap<String, RuntimeFeedbackOutboxRecordV1>, WorldError> {
    let Some(value) = projection.get(OUTBOX_FIELD) else {
        return Ok(BTreeMap::new());
    };
    let records: BTreeMap<String, RuntimeFeedbackOutboxRecordV1> =
        serde_json::from_value(value.clone())
            .map_err(|_| feedback_error("feedback_outbox_invalid"))?;
    for (key, record) in &records {
        if key != &record.feedback_id {
            return Err(feedback_error("feedback_outbox_key_mismatch"));
        }
        record
            .validate()
            .map_err(|error| feedback_error(error.code()))?;
    }
    Ok(records)
}

fn current_projection(world: &World) -> JsonValue {
    if world.cognition().is_null() {
        default_cognition_persistence_projection()
    } else {
        world.cognition().clone()
    }
}

fn persist_outbox(
    world: &mut World,
    mut projection: JsonValue,
    records: BTreeMap<String, RuntimeFeedbackOutboxRecordV1>,
    kind: &str,
    record: &RuntimeFeedbackOutboxRecordV1,
) -> Result<(), WorldError> {
    let object = projection
        .as_object_mut()
        .ok_or_else(|| feedback_error("cognition_projection_not_object"))?;
    object.insert(OUTBOX_FIELD.to_string(), serde_json::to_value(records)?);
    append_cognition_event(
        &mut projection,
        kind,
        json!({
            "feedback_id": record.feedback_id,
            "feedback_seq": record.feedback_seq,
            "agent_id": record.agent_subject,
            "agent_session_id": record.agent_session_id,
            "agent_turn_id": record.agent_turn_id,
            "decision_request_id": record.decision_request_id,
            "request_digest": record.request_digest,
            "feedback_digest": record.envelope_digest,
            "feedback_state": record.state,
            "attempt": record.attempt,
        }),
    )?;
    world.cognition = projection;
    Ok(())
}

impl World {
    /// Queue one Runtime-authoritative feedback envelope. Re-enqueueing the
    /// exact canonical envelope is idempotent; reusing its identity with any
    /// changed field is rejected before the projection is touched.
    pub fn enqueue_runtime_feedback(
        &mut self,
        feedback: FeedbackEnvelopeV1,
    ) -> Result<RuntimeFeedbackOutboxRecordV1, WorldError> {
        let record = RuntimeFeedbackOutboxRecordV1::from_feedback(&feedback)
            .map_err(|error| feedback_error(error.code()))?;
        let projection = current_projection(self);
        let mut records = parse_outbox(&projection)?;
        if let Some(existing) = records.get(&record.feedback_id) {
            if existing.envelope_digest != record.envelope_digest {
                return Err(feedback_error("feedback_identity_collision"));
            }
            return Ok(existing.clone());
        }
        if records.values().any(|existing| {
            existing.agent_subject == record.agent_subject
                && existing.agent_session_id == record.agent_session_id
                && existing.feedback_seq == record.feedback_seq
        }) {
            return Err(feedback_error("feedback_sequence_collision"));
        }
        records.insert(record.feedback_id.clone(), record.clone());
        persist_outbox(self, projection, records, "ProviderFeedbackQueued", &record)?;
        Ok(record)
    }

    /// Return every durable feedback record, including acknowledged records
    /// retained for idempotent replay/collision detection.
    pub fn runtime_feedback_outbox(
        &self,
    ) -> Result<Vec<RuntimeFeedbackOutboxRecordV1>, WorldError> {
        Ok(parse_outbox(&current_projection(self))?
            .into_values()
            .collect())
    }

    /// Return pending records in canonical feedback-id order for a transport
    /// worker. Runtime never invokes a provider while claiming a lease.
    pub fn pending_runtime_feedback(
        &self,
    ) -> Result<Vec<RuntimeFeedbackOutboxRecordV1>, WorldError> {
        let mut records = self
            .runtime_feedback_outbox()?
            .into_iter()
            .filter(|record| record.state == "pending")
            .collect::<Vec<_>>();
        records.sort_by(|left, right| {
            (
                &left.agent_subject,
                &left.agent_session_id,
                left.feedback_seq,
                &left.feedback_id,
            )
                .cmp(&(
                    &right.agent_subject,
                    &right.agent_session_id,
                    right.feedback_seq,
                    &right.feedback_id,
                ))
        });
        Ok(records)
    }

    /// Claim one pending record for delivery. A repeated claim of an existing
    /// lease is idempotent and does not increment the attempt counter.
    pub fn claim_runtime_feedback(
        &mut self,
        feedback_id: &str,
    ) -> Result<RuntimeFeedbackOutboxRecordV1, WorldError> {
        let projection = current_projection(self);
        let mut records = parse_outbox(&projection)?;
        let record = records
            .get_mut(feedback_id)
            .ok_or_else(|| feedback_error("feedback_outbox_missing"))?;
        if record.state == "acked" {
            return Err(feedback_error("feedback_already_acked"));
        }
        if record.state == "in_flight" {
            return Ok(record.clone());
        }
        if record.state == "pending" {
            record.state = "in_flight".to_string();
            record.attempt = record.attempt.saturating_add(1);
            record.last_error = None;
        }
        let result = record.clone();
        persist_outbox(
            self,
            projection,
            records,
            "ProviderFeedbackClaimed",
            &result,
        )?;
        Ok(result)
    }

    /// Return a failed delivery lease to pending without losing its attempt
    /// count. The transport can retry later, including after a restart.
    pub fn retry_runtime_feedback(
        &mut self,
        feedback_id: &str,
        reason: impl Into<String>,
    ) -> Result<RuntimeFeedbackOutboxRecordV1, WorldError> {
        let projection = current_projection(self);
        let mut records = parse_outbox(&projection)?;
        let record = records
            .get_mut(feedback_id)
            .ok_or_else(|| feedback_error("feedback_outbox_missing"))?;
        if record.state == "acked" {
            return Err(feedback_error("feedback_already_acked"));
        }
        record.state = "pending".to_string();
        let mut reason = reason.into();
        reason.truncate(256);
        record.last_error = (!reason.is_empty()).then_some(reason);
        let result = record.clone();
        persist_outbox(
            self,
            projection,
            records,
            "ProviderFeedbackRetryable",
            &result,
        )?;
        Ok(result)
    }

    /// Mark a claimed envelope delivered. Keeping the terminal record makes
    /// transport retries idempotent and prevents identity reuse.
    pub fn ack_runtime_feedback(&mut self, feedback_id: &str) -> Result<(), WorldError> {
        let projection = current_projection(self);
        let mut records = parse_outbox(&projection)?;
        let record = records
            .get_mut(feedback_id)
            .ok_or_else(|| feedback_error("feedback_outbox_missing"))?;
        if record.state == "acked" {
            return Ok(());
        }
        if record.state != "in_flight" {
            return Err(feedback_error("feedback_not_in_flight"));
        }
        record.state = "acked".to_string();
        record.last_error = None;
        let result = record.clone();
        persist_outbox(
            self,
            projection,
            records,
            "ProviderFeedbackAcknowledged",
            &result,
        )?;
        Ok(())
    }

    /// Recover a process-crash lease. No provider is called and no envelope is
    /// discarded; every in-flight item becomes pending exactly once.
    pub(super) fn recover_feedback_outbox_projection(
        projection: &mut JsonValue,
    ) -> Result<bool, WorldError> {
        let mut records = parse_outbox(projection)?;
        let mut recovered = Vec::new();
        for record in records.values_mut() {
            if record.state == "in_flight" {
                record.state = "pending".to_string();
                record.last_error = Some("recovered_after_restart".to_string());
                recovered.push(record.clone());
            }
        }
        if recovered.is_empty() {
            return Ok(false);
        }
        projection[OUTBOX_FIELD] = serde_json::to_value(records)?;
        for record in recovered {
            append_cognition_event(
                projection,
                "ProviderFeedbackRecovered",
                json!({
                    "feedback_id": record.feedback_id,
                    "feedback_seq": record.feedback_seq,
                    "feedback_digest": record.envelope_digest,
                    "feedback_state": record.state,
                    "attempt": record.attempt,
                }),
            )?;
        }
        Ok(true)
    }
}
