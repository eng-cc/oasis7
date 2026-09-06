//! S14 regression coverage for canonical cognition retention and persistence.

use super::super::*;
use super::agent_cognition_runtime_hardening::{
    bind_test_turn, digest, envelope, response_artifact_for_envelope, temp_dir,
};
use crate::runtime::RetentionRecordV1;
use serde_json::{Value, json};
use std::fs;

fn terminal_record() -> RetentionRecordV1 {
    serde_json::from_value(json!({
        "schema_version": "cognition-retention-record.v1",
        "world_id": "world-s14-retention",
        "envelope_idempotency_key": "key:s14-retention",
        "envelope_digest": digest(1),
        "agent_session_id": "session.s14-retention",
        "agent_turn_id": "turn.s14-retention",
        "decision_request_id": "request.s14-retention",
        "status": "committed",
        "base_tick": 10,
        "issued_at_tick": 10,
        "terminal_disposition": null,
        "receipt_id": "receipt:s14-retention",
        "receipt_digest": digest(2),
        "response_artifact_id": "artifact:s14-retention",
        "continuation_id": "continuation:s14-retention",
        "commit_record_id": "commit:s14-retention"
    }))
    .expect("decode S14 terminal record")
}

#[test]
fn gc_compacts_canonical_commit_receipt_response_and_journal_projections() {
    let mut world = World::new();
    bind_test_turn(&mut world);
    let decision = envelope(&world);
    let prepared = world
        .prepare_cognition_envelope(
            decision.clone(),
            Some(response_artifact_for_envelope(&decision)),
        )
        .expect("prepare canonical cognition commit");
    world
        .finalize_cognition_commit(&prepared.commit_id)
        .expect("finalize canonical cognition commit");

    let report = world
        .gc_cognition(1_000, 1_000)
        .expect("GC should compact expired canonical cognition");
    assert_eq!(report.deleted_count, 1);
    assert_eq!(
        report.deleted_keys,
        vec![prepared.envelope_idempotency_key.clone()]
    );
    assert!(
        world.cognition()["commit_records"]
            .as_array()
            .expect("commit records")
            .is_empty()
    );
    assert!(
        world.cognition()["responses"]
            .as_array()
            .expect("responses")
            .is_empty()
    );
    assert!(
        world.cognition()["receipt_registry"]
            .as_array()
            .expect("receipt registry")
            .is_empty()
    );
    assert!(
        world.cognition()["receipt_lineage_registry"]
            .as_array()
            .expect("receipt lineage registry")
            .is_empty()
    );
    assert!(
        world.cognition()["idempotency_index"]
            .as_object()
            .expect("idempotency index")
            .is_empty()
    );
    assert!(
        world.cognition()["cognition_journal"]["events"]
            .as_array()
            .expect("cognition journal")
            .is_empty()
    );
}

#[test]
fn attached_world_persists_record_pin_and_gc_mutations_before_publish() {
    let mut world = World::new();
    let dir = temp_dir("s14-retention-transaction");
    world
        .save_to_dir(&dir)
        .expect("attach persistence directory");

    world
        .record_cognition_terminal(terminal_record())
        .expect("persist terminal retention record");
    world
        .pin_cognition_reference("key:s14-retention", "replay-manifest")
        .expect("persist retention pin");
    world
        .gc_cognition(100, 11)
        .expect("persist retention GC floor");

    let restored = World::load_from_dir(&dir).expect("restore attached world");
    let retention = &restored.cognition()["retention_state"];
    assert_eq!(retention["gc_floor_tick"], 11);
    assert!(
        retention["records"]
            .as_object()
            .expect("retention records")
            .contains_key("key:s14-retention")
    );
    assert_eq!(
        retention["pins"]["key:s14-retention"][0],
        Value::String("replay-manifest".to_string())
    );
    let _ = fs::remove_dir_all(dir);
}
