use super::*;

#[test]
fn runtime_continuation_hydration_reads_typed_active_projection() {
    let mut world = RuntimeWorld::new();
    let mut continuation: crate::runtime::AgentContinuation =
        serde_json::from_value(serde_json::json!({
            "schema_version": "agent-continuation.v1",
            "continuation_id": "continuation-typed-readback",
            "wake_id": "wake-typed-readback",
            "world_id": "world-typed-readback",
            "branch_id": "main",
            "finality_epoch": 0,
            "finality_block_hash": null,
            "finality_status": "pending",
            "reorg_epoch": 0,
            "runtime_manifest_hash": "manifest-typed-readback",
            "agent_id": "agent-typed-readback",
            "agent_session_id": "session-typed-readback",
            "agent_turn_id": "turn-typed-readback",
            "decision_request_id": "request-typed-readback",
            "origin_turn_id": "turn-typed-readback",
            "origin_request_digest": "origin-typed-readback",
            "continuation_proposal_id": "proposal-typed-readback",
            "proposal_digest": "proposal-digest-typed-readback",
            "action_or_envelope_digest": null,
            "wake_conditions": [{
                "schema_version": "wake-condition.v1",
                "kind": "at_or_after_tick",
                "logical_tick": 0
            }],
            "next_wake_tick": 0,
            "remaining_budget": {"unit": "steps", "value": 1},
            "valid_until_tick": 10,
            "precondition_digest": "precondition-typed-readback",
            "wake_seq": 1,
            "logical_tick": 0,
            "status": "scheduled",
            "terminal_disposition": null
        }))
        .expect("typed continuation fixture");
    continuation.refresh_status_digest();
    continuation
        .validate_authoritative()
        .expect("typed continuation fixture is authoritative");
    world
        .install_cognition_continuation_for_test(continuation.clone())
        .expect("install typed continuation fixture");
    let wake: crate::runtime::SchedulerWakeV1 = serde_json::from_value(serde_json::json!({
        "schema_version": "scheduler-wake.v1",
        "wake_id": "wake-typed-readback",
        "continuation_id": "continuation-typed-readback",
        "world_id": "world-typed-readback",
        "branch_id": "main",
        "finality_epoch": 0,
        "finality_block_hash": "genesis",
        "finality_status": "pending",
        "reorg_epoch": 0,
        "runtime_manifest_hash": "manifest-typed-readback",
        "agent_id": "agent-typed-readback",
        "agent_session_id": "session-typed-readback",
        "agent_turn_id": "turn-typed-readback",
        "decision_request_id": "request-typed-readback",
        "next_wake_tick": 0,
        "eligible_since_tick": 0,
        "starvation_deadline_tick": 1,
        "initial_priority": 0,
        "wake_seq": 1,
        "retry_seq": 0,
        "status": "pending",
        "pending_reason": "capacity_available"
    }))
    .expect("typed wake fixture");

    assert_eq!(
        active_runtime_continuation_for_wake(&world, &wake)
            .expect("typed active continuation")
            .continuation_id,
        continuation.continuation_id
    );
}
