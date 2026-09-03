//! Follow-up recovery, replay, and production lease regressions.

use super::super::*;
use super::agent_cognition_runtime_hardening::{
    AGENT_ID, WORLD_ID, bind_test_turn, continuous_commit_request, continuous_response_artifact,
    digest, envelope, policy, proposal, read_snapshot, response_artifact_for_envelope, temp_dir,
    test_continuation, test_continuation_wake, write_snapshot,
};
use serde_json::json;
use std::fs;

#[test]
fn recovery_repairs_a_missing_committed_turn_completion() {
    let mut world = World::new();
    let envelope = envelope(&world);
    let prepared = world
        .prepare_cognition_envelope(
            envelope.clone(),
            Some(response_artifact_for_envelope(&envelope)),
        )
        .expect("prepare");
    world
        .finalize_cognition_commit(&prepared.commit_id)
        .expect("finalize");
    let dir = temp_dir("missing-committed-completion");
    world.save_to_dir(&dir).expect("save");
    let mut snapshot = read_snapshot(&dir);
    snapshot["cognition"]["cognition_journal"]["events"]
        .as_array_mut()
        .expect("events")
        .pop();
    write_snapshot(&dir, &snapshot);
    fs::remove_dir_all(dir.join(".distfs-state")).expect("force JSON restore");
    let restored = World::load_from_dir(&dir).expect("repair missing completion");
    let events = restored.cognition()["cognition_journal"]["events"]
        .as_array()
        .expect("events after repair");
    assert_eq!(
        events.last().expect("completion")["kind"],
        "CognitionTurnCompleted"
    );
    assert_eq!(events.last().expect("completion")["status"], "committed");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn runtime_binding_rejects_noncanonical_verified_finality_hash() {
    let mut world = World::new();
    let error = world
        .bind_cognition_runtime(
            WORLD_ID,
            "main",
            1,
            Some("hash:not-a-digest".to_string()),
            "verified",
            0,
        )
        .expect_err("verified finality must carry a canonical digest");
    assert!(matches!(
        error,
        WorldError::DistributedValidationFailed { ref reason }
            if reason.contains("runtime_binding")
                || reason.contains("runtime_cognition_base_binding_invalid")
    ));
}

#[test]
fn recovery_quarantines_visible_root_mismatch_but_preserves_trusted_head() {
    let mut world = World::new();
    let envelope = envelope(&world);
    let prepared = world
        .prepare_cognition_envelope(
            envelope.clone(),
            Some(response_artifact_for_envelope(&envelope)),
        )
        .expect("prepare");
    let committed = world
        .finalize_cognition_commit(&prepared.commit_id)
        .expect("finalize");
    let trusted_state_root = world.current_state_root_hash().expect("trusted root");
    let dir = temp_dir("visible-root-mismatch");
    world.save_to_dir(&dir).expect("save");
    let mut snapshot = read_snapshot(&dir);
    snapshot["cognition"]["recovery"]["world_root"] = json!({
        "schema_version": "world-root-view.v1",
        "world_id": WORLD_ID,
        "branch_id": "main",
        "logical_tick": world.state().time,
        "state_root": digest(88),
        "head_status": "canonical",
        "commit_id": committed.commit_id,
        "quarantine_id": null
    });
    write_snapshot(&dir, &snapshot);
    fs::remove_dir_all(dir.join(".distfs-state")).expect("force JSON restore");
    let restored = World::load_from_dir(&dir).expect("restore mismatch");
    let recovery = &restored.cognition()["recovery"];
    assert_eq!(recovery["disposition"], "recovery_pending");
    assert_eq!(recovery["reject_reason"], "world_root_mismatch");
    assert_eq!(recovery["world_root"]["state_root"], trusted_state_root);
    assert_eq!(recovery["world_root"]["head_status"], "canonical");
    assert_eq!(recovery["candidate_root"], committed.staged_state_root);
    assert_eq!(
        recovery["candidate_receipt"]["receipt_id"],
        committed.receipt_id
    );
    assert!(recovery["quarantine_id"].as_str().is_some());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn recovery_repairs_a_committed_marker_with_missing_projection() {
    let mut world = World::new();
    let envelope = envelope(&world);
    let prepared = world
        .prepare_cognition_envelope(
            envelope.clone(),
            Some(response_artifact_for_envelope(&envelope)),
        )
        .expect("prepare");
    let committed = world
        .finalize_cognition_commit(&prepared.commit_id)
        .expect("finalize");
    let dir = temp_dir("missing-committed-projection");
    world.save_to_dir(&dir).expect("save");
    let mut snapshot = read_snapshot(&dir);
    snapshot["cognition"]["receipt_registry"] = json!([]);
    snapshot["cognition"]["idempotency_index"] = json!({});
    write_snapshot(&dir, &snapshot);
    fs::remove_dir_all(dir.join(".distfs-state")).expect("force JSON restore");
    let restored = World::load_from_dir(&dir).expect("restore missing projection");
    assert_eq!(
        restored.cognition()["receipt_registry"][0]["receipt_id"],
        committed.receipt_id
    );
    assert_eq!(
        restored.cognition()["idempotency_index"][&committed.envelope_idempotency_key]["receipt_id"],
        committed.receipt_id
    );
    let repaired = restored.cognition()["recovery"]["repaired_projection_ids"]
        .as_array()
        .expect("repair markers");
    assert!(repaired.iter().any(|id| id == "receipt_registry"));
    assert!(repaired.iter().any(|id| id == "idempotency_index"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn typed_context_digest_handoff_rejects_alternate_goal_or_policy() {
    let mut world = World::new()
        .try_with_cognition_scheduler(policy(), 1)
        .expect("scheduler");
    bind_test_turn(&mut world);
    let admitted_proposal = proposal(&world);
    let admitted = world
        .admit_cognition_continuation(admitted_proposal.clone())
        .expect("admit continuation");
    let context = world
        .validate_cognition_context_digests(&admitted.continuation_id, &admitted_proposal)
        .expect("admitted context remains valid");
    assert_eq!(context.goal_digest, admitted_proposal.goal_digest);

    let mut alternate = admitted_proposal;
    alternate.goal_digest = digest(90);
    alternate.proposal_digest = alternate.proposal_digest();
    let error = world
        .validate_cognition_context_digests(&admitted.continuation_id, &alternate)
        .expect_err("a changed goal must not cross the Runtime wake boundary");
    assert!(matches!(
        error,
        WorldError::DistributedValidationFailed { ref reason }
            if reason.contains("cognition_context_mismatch")
    ));
}

#[test]
fn typed_budget_consumption_is_monotonic_and_completes_on_exhaustion() {
    let mut world = World::new()
        .try_with_cognition_scheduler(policy(), 1)
        .expect("scheduler");
    bind_test_turn(&mut world);
    let admitted = world
        .admit_cognition_continuation(proposal(&world))
        .expect("admit continuation");
    world.step().expect("advance to wake tick");
    let _ = world
        .select_ready_cognition_wakes(world.state().time)
        .expect("select wake");
    let consumed = world
        .consume_cognition_continuation_budget(&admitted.continuation_id, 2)
        .expect("consume exact remaining budget");
    assert_eq!(consumed.remaining_budget.value, 0);
    assert_eq!(consumed.status, ContinuationStatusV1::Completed);
    assert_eq!(world.cognition_scheduler_snapshot()["in_flight"], json!({}));
    assert_eq!(
        world.cognition_continuations()[0]["terminal_disposition"],
        "budget_exhausted"
    );
    let error = world
        .consume_cognition_continuation_budget(&admitted.continuation_id, 1)
        .expect_err("completed continuations cannot spend more budget");
    assert!(matches!(
        error,
        WorldError::DistributedValidationFailed { ref reason }
            if reason.contains("wake_not_in_flight")
    ));
}

#[test]
fn typed_wake_handoff_requires_terminal_or_validated_replan() {
    let mut world = World::new()
        .try_with_cognition_scheduler(policy(), 1)
        .expect("scheduler");
    bind_test_turn(&mut world);
    let admitted = world
        .admit_cognition_continuation(proposal(&world))
        .expect("admit continuation");
    world.step().expect("advance to wake tick");
    let selected = world
        .select_ready_cognition_wakes(world.state().time)
        .expect("select wake");
    let wake_id = selected
        .first()
        .map(|wake| wake.wake_id.clone())
        .or_else(|| {
            world
                .cognition_in_flight_wakes()
                .expect("in-flight")
                .first()
                .map(|wake| wake.wake_id.clone())
        })
        .expect("wake lease");
    let result = world
        .handoff_cognition_wake(
            &wake_id,
            CognitionWakeDispositionV1::Terminal {
                status: ContinuationStatusV1::Completed,
                reason: "provider_terminal".to_string(),
            },
        )
        .expect("terminal disposition handoff");
    assert_eq!(
        result.continuation.continuation_id,
        admitted.continuation_id
    );
    assert_eq!(result.continuation.status, ContinuationStatusV1::Completed);
    assert!(
        world
            .cognition_in_flight_wakes()
            .expect("leases")
            .is_empty()
    );
}

#[test]
fn recovery_rejects_a_tampered_trailing_cognition_event() {
    let mut world = World::new();
    let envelope = envelope(&world);
    let prepared = world
        .prepare_cognition_envelope(
            envelope.clone(),
            Some(response_artifact_for_envelope(&envelope)),
        )
        .expect("prepare");
    world
        .finalize_cognition_commit(&prepared.commit_id)
        .expect("finalize");
    let dir = temp_dir("tampered-trailing-cognition-event");
    world.save_to_dir(&dir).expect("save");
    let mut snapshot = read_snapshot(&dir);
    snapshot["cognition"]["cognition_journal"]["events"]
        .as_array_mut()
        .expect("events")
        .push(json!({"journal_seq": 99, "kind": "ForgedTrailingEvent"}));
    write_snapshot(&dir, &snapshot);
    fs::remove_dir_all(dir.join(".distfs-state")).expect("force JSON restore");
    assert!(World::load_from_dir(&dir).is_err());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn recovery_rejects_a_missing_middle_cognition_event() {
    let mut world = World::new();
    let envelope = envelope(&world);
    let prepared = world
        .prepare_cognition_envelope(
            envelope.clone(),
            Some(response_artifact_for_envelope(&envelope)),
        )
        .expect("prepare");
    world
        .finalize_cognition_commit(&prepared.commit_id)
        .expect("finalize");
    let dir = temp_dir("missing-middle-cognition-event");
    world.save_to_dir(&dir).expect("save");
    let mut snapshot = read_snapshot(&dir);
    snapshot["cognition"]["cognition_journal"]["events"]
        .as_array_mut()
        .expect("events")
        .remove(1);
    write_snapshot(&dir, &snapshot);
    fs::remove_dir_all(dir.join(".distfs-state")).expect("force JSON restore");
    assert!(World::load_from_dir(&dir).is_err());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn unknown_continuation_wake_is_rejected() {
    let mut world = World::new()
        .try_with_cognition_scheduler(policy(), 1)
        .expect("scheduler");
    world
        .bind_cognition_runtime(WORLD_ID, "main", 0, None, "pending", 0)
        .expect("bind World authority");
    let binding = world.current_cognition_runtime_binding().expect("binding");
    let wake: SchedulerWakeV1 = serde_json::from_value(json!({
        "schema_version": "scheduler-wake.v1",
        "wake_id": "wake-unknown-continuation",
        "continuation_id": "continuation-does-not-exist",
        "world_id": WORLD_ID,
        "branch_id": "main",
        "finality_epoch": 0,
        "finality_block_hash": null,
        "finality_status": "pending",
        "reorg_epoch": 0,
        "runtime_manifest_hash": binding.runtime_manifest_hash.to_string(),
        "agent_id": AGENT_ID,
        "agent_session_id": "session.unknown",
        "agent_turn_id": "turn.unknown",
        "decision_request_id": "request.unknown",
        "next_wake_tick": 0,
        "eligible_since_tick": 0,
        "starvation_deadline_tick": 4,
        "initial_priority": 0,
        "wake_seq": 1,
        "retry_seq": 0,
        "status": "pending",
        "pending_reason": "capacity_available"
    }))
    .expect("wake");
    assert!(world.enqueue_cognition_wake(wake).is_err());
}

#[test]
fn scheduler_rejects_duplicate_wake_while_original_lease_is_in_flight() {
    let mut world = World::new()
        .try_with_cognition_scheduler(policy(), 1)
        .expect("scheduler");
    bind_test_turn(&mut world);
    let continuation = world
        .admit_cognition_continuation(proposal(&world))
        .expect("continuation");
    world.step().expect("service scheduler");
    let wake: SchedulerWakeV1 = serde_json::from_value(
        world.cognition_scheduler_snapshot()["in_flight"][&continuation.wake_id].clone(),
    )
    .expect("in-flight wake");
    assert!(world.enqueue_cognition_wake(wake).is_err());
}

#[test]
fn bound_world_rejects_continuation_without_a_registered_turn() {
    let mut world = World::new()
        .try_with_cognition_scheduler(policy(), 1)
        .expect("scheduler");
    world
        .bind_cognition_runtime(WORLD_ID, "main", 0, None, "pending", 0)
        .expect("bind World authority");
    assert!(
        world
            .admit_cognition_continuation(proposal(&world))
            .is_err()
    );
}

#[test]
fn failed_first_proposal_does_not_install_runtime_binding() {
    let mut world = World::new()
        .try_with_cognition_scheduler(policy(), 1)
        .expect("scheduler");
    let before = world.cognition().clone();
    let mut candidate = proposal(&world);
    candidate.proposal_digest = digest(99);
    assert!(world.admit_cognition_continuation(candidate).is_err());
    assert_eq!(world.cognition(), &before);
}

#[test]
fn committed_replay_rejects_an_alternate_action_for_the_same_request() {
    let mut world = World::new();
    world
        .bind_cognition_runtime(WORLD_ID, "main", 0, None, "pending", 0)
        .expect("bind World authority");
    let envelope = envelope(&world);
    let first_action: Action = serde_json::from_value(envelope.action.clone()).expect("action");
    world
        .commit_cognition_action(
            continuous_commit_request(&world),
            first_action,
            continuous_response_artifact(&continuous_commit_request(&world)),
        )
        .expect("first commit");
    let mut alternate = envelope.action.clone();
    alternate["data"]["pos"]["x_cm"] = json!(1);
    let alternate_action: Action = serde_json::from_value(alternate).expect("alternate action");
    let request = continuous_commit_request(&world);
    let error = world
        .commit_cognition_action(
            request.clone(),
            alternate_action,
            continuous_response_artifact(&request),
        )
        .expect_err("alternate action must not replay the original receipt");
    assert!(matches!(
        error,
        WorldError::DistributedValidationFailed { ref reason }
            if reason.contains("envelope_idempotency") || reason.contains("replay")
    ));
}

#[test]
fn direct_response_replay_rejects_a_tampered_trailing_event() {
    let mut world = World::new();
    let envelope = envelope(&world);
    let prepared = world
        .prepare_cognition_envelope(
            envelope.clone(),
            Some(response_artifact_for_envelope(&envelope)),
        )
        .expect("prepare");
    let committed = world
        .finalize_cognition_commit(&prepared.commit_id)
        .expect("finalize");
    let mut snapshot = world.snapshot();
    snapshot.cognition["cognition_journal"]["events"]
        .as_array_mut()
        .expect("events")
        .push(json!({"journal_seq": 99, "kind": "forged"}));
    let tampered = World::from_snapshot(snapshot, world.journal().clone()).expect("snapshot");
    assert!(
        tampered
            .replay_cognition_response(&committed.envelope_digest)
            .is_err(),
        "replay must verify the current journal before accepting a historical head"
    );
}

#[test]
fn saved_world_persists_cognition_commit_before_the_api_returns() {
    let mut world = World::new();
    let envelope = envelope(&world);
    let dir = temp_dir("durable-cognition-commit");
    world.save_to_dir(&dir).expect("save baseline");
    let prepared = world
        .prepare_cognition_envelope(
            envelope.clone(),
            Some(response_artifact_for_envelope(&envelope)),
        )
        .expect("prepare");
    let committed = world
        .finalize_cognition_commit(&prepared.commit_id)
        .expect("finalize");
    let restored = World::load_from_dir(&dir).expect("restore durable commit");
    assert_eq!(
        restored.cognition()["commit_records"][0]["receipt_id"],
        committed.receipt_id
    );
    assert_eq!(restored.state().time, world.state().time);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn unbound_world_rejects_unknown_continuation_wakes() {
    let mut world = World::new()
        .try_with_cognition_scheduler(policy(), 1)
        .expect("scheduler");
    let wake: SchedulerWakeV1 = serde_json::from_value(json!({
        "schema_version": "scheduler-wake.v1",
        "wake_id": "wake-unbound-foreign",
        "continuation_id": "continuation-unbound-foreign",
        "world_id": WORLD_ID,
        "branch_id": "main",
        "finality_epoch": 0,
        "finality_block_hash": null,
        "finality_status": "pending",
        "reorg_epoch": 0,
        "runtime_manifest_hash": world.current_manifest_hash().expect("manifest"),
        "agent_id": AGENT_ID,
        "agent_session_id": "session.foreign",
        "agent_turn_id": "turn.foreign",
        "decision_request_id": "request.foreign",
        "next_wake_tick": 0,
        "eligible_since_tick": 0,
        "starvation_deadline_tick": 4,
        "initial_priority": 0,
        "wake_seq": 1,
        "retry_seq": 0,
        "status": "pending",
        "pending_reason": "capacity_available"
    }))
    .expect("wake");
    assert!(world.enqueue_cognition_wake(wake).is_err());
}

#[test]
fn unbound_continuation_admission_requires_a_registered_turn() {
    let mut world = World::new()
        .try_with_cognition_scheduler(policy(), 1)
        .expect("scheduler");
    assert!(
        world
            .admit_cognition_continuation(proposal(&world))
            .is_err()
    );
}

#[test]
fn production_constructor_installs_a_world_owned_cognition_binding() {
    let world = World::new_production_hardened_with_cognition_binding(
        WORLD_ID, "main", 0, None, "pending", 0,
    )
    .expect("production runtime binding");
    let binding = world
        .current_cognition_runtime_binding()
        .expect("current production binding");
    assert_eq!(binding.world_id, WORLD_ID);
    assert!(binding.base_world_hash.is_canonical_blake3());
}

#[test]
fn production_lease_consumer_releases_only_the_dispatched_wake() {
    let mut world = World::new()
        .try_with_cognition_scheduler(policy(), 1)
        .expect("scheduler");
    let wake = test_continuation_wake(&world, "wake-consumer", "continuation-consumer");
    world
        .install_cognition_continuation_for_test(test_continuation(&wake))
        .expect("install continuation");
    world.enqueue_cognition_wake(wake).expect("enqueue");
    world.step().expect("select lease");
    let consumed = world
        .consume_cognition_wake("wake-consumer", |lease| {
            assert_eq!(lease.wake_id, "wake-consumer");
            Ok(CognitionWakeDispositionV1::Terminal {
                status: ContinuationStatusV1::Completed,
                reason: "provider_terminal".to_string(),
            })
        })
        .expect("consume lease");
    assert_eq!(consumed.wake_id, "wake-consumer");
    assert!(
        world
            .cognition_in_flight_wakes()
            .expect("leases")
            .is_empty()
    );
}
