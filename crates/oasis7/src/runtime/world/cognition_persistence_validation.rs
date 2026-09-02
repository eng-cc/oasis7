use super::World;
use crate::runtime::cognition_recovery::{
    CognitionRecoveryReport, CognitionResponseRecordV1, WorldCommitRecordV1, WorldRootViewV1,
    cognition_digest_v1, response_artifact_digest,
};
use crate::runtime::error::WorldError;
use serde_json::{Value as JsonValue, json};

pub(super) fn append_cognition_event(
    projection: &mut JsonValue,
    kind: &str,
    details: JsonValue,
) -> Result<u64, WorldError> {
    let journal = projection
        .get_mut("cognition_journal")
        .ok_or_else(|| validation("cognition_journal_missing"))?
        .as_object_mut()
        .ok_or_else(|| validation("cognition_journal_not_object"))?;
    let head_seq = journal
        .get("head_seq")
        .and_then(JsonValue::as_u64)
        .unwrap_or_default();
    let next_seq;
    {
        let events = journal
            .entry("events")
            .or_insert_with(|| JsonValue::Array(Vec::new()))
            .as_array_mut()
            .ok_or_else(|| validation("cognition_events_not_array"))?;
        next_seq = head_seq
            .max(
                events
                    .iter()
                    .filter_map(|event| event.get("journal_seq").and_then(JsonValue::as_u64))
                    .max()
                    .unwrap_or_default(),
            )
            .saturating_add(1);
        let mut event = details
            .as_object()
            .cloned()
            .ok_or_else(|| validation("cognition_event_details_not_object"))?;
        event.insert("journal_seq".to_string(), json!(next_seq));
        event.insert("kind".to_string(), json!(kind));
        events.push(JsonValue::Object(event));
    }
    journal.insert("head_seq".to_string(), json!(next_seq));
    let events = journal
        .get("events")
        .cloned()
        .ok_or_else(|| validation("cognition_events_missing"))?;
    journal.insert(
        "head_digest".to_string(),
        cognition_digest_v1(
            "oasis7.cognition.journal-head.v1",
            &json!({"head_seq": next_seq, "events": events}),
        )
        .into(),
    );
    Ok(next_seq)
}

pub(super) fn validate_marker_current_world(
    world: &World,
    marker: &WorldCommitRecordV1,
) -> Result<(), WorldError> {
    if marker.agent_id.trim().is_empty() {
        return Ok(());
    }
    let root = world.current_state_root_hash()?;
    let manifest = world.current_manifest_hash()?;
    if marker.parent_tick != world.state().time
        || marker.parent_world_hash != root
        || marker.staged_state_root != root
        || marker.runtime_manifest_hash != manifest
    {
        return Err(validation("cognition_world_head_mismatch"));
    }
    if let Some(binding) = world.cognition().get("runtime_binding") {
        let block_hash = binding["finality_block_hash"].as_str().unwrap_or("genesis");
        if binding["world_id"].as_str() != Some(marker.world_id.as_str())
            || binding["branch_id"].as_str() != Some(marker.branch_id.as_str())
            || binding["finality_epoch"].as_u64() != Some(marker.finality_epoch)
            || block_hash != marker.finality_block_hash
            || binding["finality_status"].as_str() != Some(marker.finality_status.as_str())
            || binding["reorg_epoch"].as_u64() != Some(marker.reorg_epoch)
            || binding["runtime_manifest_hash"].as_str()
                != Some(marker.runtime_manifest_hash.as_str())
        {
            return Err(validation("cognition_finality_binding_mismatch"));
        }
    }
    Ok(())
}

pub(super) fn validate_response_record(
    projection: &JsonValue,
    response: &CognitionResponseRecordV1,
) -> Result<(), WorldError> {
    if !is_canonical_digest(&response.response_digest) {
        return Ok(());
    }
    let artifact = response
        .response_artifact
        .as_ref()
        .ok_or_else(|| validation("response_artifact_missing"))?;
    if response_artifact_digest(artifact) != response.response_digest {
        return Err(validation("response_artifact_digest_mismatch"));
    }
    let journal = projection
        .get("cognition_journal")
        .ok_or_else(|| validation("cognition_journal_missing"))?;
    let head_seq = journal
        .get("head_seq")
        .and_then(JsonValue::as_u64)
        .unwrap_or_default();
    let events = journal
        .get("events")
        .cloned()
        .unwrap_or_else(|| JsonValue::Array(Vec::new()));
    let expected = cognition_digest_v1(
        "oasis7.cognition.journal-head.v1",
        &json!({"head_seq": head_seq, "events": events}),
    );
    if response.journal_head.trim().is_empty() || expected != response.journal_head {
        return Err(validation("response_journal_head_mismatch"));
    }
    Ok(())
}

pub(super) fn persist_recovery_report(
    projection: &mut JsonValue,
    report: &CognitionRecoveryReport,
) -> Result<(), WorldError> {
    let object = projection
        .as_object_mut()
        .ok_or_else(|| validation("cognition_projection_not_object"))?;
    let recovery = object
        .entry("recovery")
        .or_insert_with(|| JsonValue::Object(serde_json::Map::new()))
        .as_object_mut()
        .ok_or_else(|| validation("recovery_projection_not_object"))?;
    recovery.insert("disposition".to_string(), json!(&report.disposition));
    recovery.insert("reject_reason".to_string(), json!(&report.reject_reason));
    recovery.insert(
        "idempotency_key".to_string(),
        json!(&report.idempotency_key),
    );
    recovery.insert("quarantine_id".to_string(), json!(&report.quarantine_id));
    recovery.insert("candidate_root".to_string(), json!(&report.candidate_root));
    recovery.insert(
        "candidate_receipt".to_string(),
        json!(&report.candidate_receipt),
    );
    recovery.insert("receipt".to_string(), json!(&report.receipt));
    recovery.insert("world_root".to_string(), json!(&report.world_root));
    recovery.insert("journal_head".to_string(), json!(&report.journal_head));
    recovery.insert("retry_count".to_string(), json!(report.retry_count));
    recovery.insert(
        "revalidation_count".to_string(),
        json!(report.revalidation_count),
    );
    recovery.insert(
        "projection_repairs".to_string(),
        json!(report.projection_repairs),
    );
    recovery.insert(
        "provider_invocation_count".to_string(),
        json!(report.provider_invocation_count),
    );
    recovery.insert(
        "kernel_invocation_count".to_string(),
        json!(report.kernel_invocation_count),
    );
    recovery.insert("effect_count".to_string(), json!(report.effect_count));
    recovery.insert("debit_count".to_string(), json!(report.debit_count));
    recovery.insert(
        "receipt_count".to_string(),
        json!(u64::from(report.receipt.is_some())),
    );
    recovery.insert(
        "world_receipt_linked_count".to_string(),
        json!(report.world_receipt_linked_count),
    );
    recovery.insert(
        "response_replayed".to_string(),
        json!(report.response_replayed),
    );
    Ok(())
}

pub(super) fn cognition_error<T: std::fmt::Display>(code: &'static str, error: T) -> WorldError {
    WorldError::DistributedValidationFailed {
        reason: format!("{code}: {error}"),
    }
}

pub(super) fn cognition_validation(code: impl Into<String>) -> WorldError {
    WorldError::DistributedValidationFailed {
        reason: format!("cognition validation failed: {}", code.into()),
    }
}

pub(super) fn legacy_recovery_report() -> CognitionRecoveryReport {
    CognitionRecoveryReport {
        world_root: None,
        receipt: None,
        disposition: "rejected".to_string(),
        reject_reason: Some("legacy_no_cognition_proof".to_string()),
        auto_submitted: false,
        idempotency_key: None,
        quarantine_id: None,
        candidate_root: None,
        candidate_receipt: None,
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

pub(super) fn parent_root(marker: &WorldCommitRecordV1) -> WorldRootViewV1 {
    WorldRootViewV1 {
        schema_version: "world-root-view.v1".to_string(),
        world_id: marker.world_id.clone(),
        branch_id: marker.branch_id.clone(),
        logical_tick: marker.parent_tick,
        state_root: marker.parent_world_hash.clone(),
        head_status: "recovery_pending".to_string(),
        commit_id: None,
        quarantine_id: None,
    }
}

fn is_canonical_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("blake3:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validation(code: &str) -> WorldError {
    WorldError::DistributedValidationFailed {
        reason: format!("cognition validation failed: {code}"),
    }
}
