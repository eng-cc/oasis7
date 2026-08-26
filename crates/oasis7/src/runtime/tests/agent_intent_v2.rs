use super::super::*;
use super::pos;
use crate::models::AgentState;

const AGENT_ID: &str = "agent-intent-v2";

fn legacy_agent_cell() -> AgentCell {
    AgentCell::new(AgentState::new(AGENT_ID, pos(0, 0)), 7)
}

fn canonical_intent(
    intent_id: &str,
    status: &str,
    summary: &str,
    replaced_by: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": 2,
        "agent_id": AGENT_ID,
        "intent_id": intent_id,
        "kind": "start_recipe",
        "summary": summary,
        "target_id": "factory-intent-v2",
        "status": status,
        "source": "player",
        "logical_time": 7,
        "event_seq": 11,
        "updated_at": 7,
        "receipt_ref": null,
        "reason_code": null,
        "reason_summary": null,
        "replaced_by": replaced_by,
    })
}

fn cell_with_intent(intent: serde_json::Value) -> AgentCell {
    let mut value = serde_json::to_value(legacy_agent_cell()).expect("encode legacy cell");
    value
        .as_object_mut()
        .expect("agent cell object")
        .insert("intent".to_string(), intent);
    serde_json::from_value(value).expect("decode AgentCell with AgentIntentV2")
}

fn intent_event(event_type: &str, intent: serde_json::Value) -> DomainEvent {
    serde_json::from_value(serde_json::json!({
        "type": event_type,
        "data": { "intent": intent },
    }))
    .expect("decode canonical AgentIntentV2 runtime event")
}

#[test]
fn legacy_agent_cell_without_intent_remains_explicitly_missing() {
    let cell = legacy_agent_cell();
    let encoded = serde_json::to_value(&cell).expect("encode legacy cell");
    assert!(
        !encoded
            .as_object()
            .expect("agent cell object")
            .contains_key("intent")
    );

    let restored: AgentCell = serde_json::from_value(encoded).expect("decode legacy cell");
    let restored_json = serde_json::to_value(restored).expect("re-encode legacy cell");
    assert!(
        !restored_json
            .as_object()
            .expect("restored agent cell object")
            .contains_key("intent")
    );
}

#[test]
fn accepted_intent_v2_roundtrips_authority_position_and_freshness() {
    let expected = canonical_intent(
        "intent-accepted-1",
        "accepted",
        "Start the iron recipe",
        None,
    );
    let cell = cell_with_intent(expected.clone());
    let encoded = serde_json::to_value(cell).expect("encode accepted AgentIntentV2");

    assert_eq!(encoded.get("intent"), Some(&expected));
    assert_eq!(encoded["intent"]["logical_time"], 7);
    assert_eq!(encoded["intent"]["event_seq"], 11);
    let intent = encoded["intent"]
        .as_object()
        .expect("canonical intent object");
    for projection_only_field in [
        "world_id",
        "reorg_epoch",
        "source_class",
        "freshness",
        "control_state",
    ] {
        assert!(
            !intent.contains_key(projection_only_field),
            "AgentCell intent must not persist projection-only field {projection_only_field}"
        );
    }
}

#[test]
fn replaced_and_terminal_intents_keep_explicit_lifecycle_dispositions() {
    for (status, replaced_by) in [
        ("blocked", None),
        ("completed", None),
        ("rejected", None),
        ("expired", None),
        ("cancelled", None),
        ("superseded", Some("intent-replacement-2")),
    ] {
        let expected = canonical_intent(
            "intent-lifecycle-1",
            status,
            "Keep this intent auditable",
            replaced_by,
        );
        let encoded = serde_json::to_value(cell_with_intent(expected.clone()))
            .expect("encode lifecycle AgentIntentV2");
        assert_eq!(encoded.get("intent"), Some(&expected));
        assert_eq!(encoded["intent"]["status"], status);
        assert_eq!(
            encoded["intent"]["replaced_by"],
            serde_json::json!(replaced_by)
        );
    }
}

#[test]
fn duplicate_intent_identity_is_idempotent_and_conflict_is_not_silent() {
    let accepted = canonical_intent("intent-idempotency-1", "accepted", "Start recipe", None);
    let conflict = canonical_intent(
        "intent-idempotency-1",
        "accepted",
        "Silently replace the original intent",
        None,
    );

    let mut state = WorldState::default();
    state
        .agents
        .insert(AGENT_ID.to_string(), legacy_agent_cell());
    let accepted_event = intent_event("AgentIntentAccepted", accepted.clone());
    state
        .apply_domain_event(&accepted_event, 7)
        .expect("accept intent");
    let after_accept = serde_json::to_vec(&state).expect("encode accepted state");

    state
        .apply_domain_event(&accepted_event, 8)
        .expect("replay identical intent");
    assert_eq!(
        serde_json::to_vec(&state).expect("encode replayed state"),
        after_accept,
        "replaying one intent_id must not create a second mutation"
    );

    let conflict_event = intent_event("AgentIntentAccepted", conflict);
    assert!(
        state.apply_domain_event(&conflict_event, 9).is_err(),
        "same intent_id with different content must be an explicit conflict"
    );
    assert_eq!(
        serde_json::to_vec(&state).expect("encode conflict state"),
        after_accept,
        "an intent identity conflict must not silently overwrite authority"
    );
}

#[test]
fn replaced_intent_event_is_replayable_without_promoting_activity_or_receipt() {
    let accepted = canonical_intent("intent-replay-1", "accepted", "Start recipe", None);
    let superseded = canonical_intent(
        "intent-replay-1",
        "superseded",
        "Start recipe",
        Some("intent-replacement-2"),
    );
    let mut state = WorldState::default();
    state
        .agents
        .insert(AGENT_ID.to_string(), legacy_agent_cell());
    state
        .apply_domain_event(&intent_event("AgentIntentAccepted", accepted), 7)
        .expect("accept original intent");
    state
        .apply_domain_event(&intent_event("AgentIntentReplaced", superseded), 8)
        .expect("replace original intent");

    let encoded = serde_json::to_value(&state).expect("encode replaced intent state");
    let decoded: WorldState =
        serde_json::from_value(encoded.clone()).expect("decode replaced state");
    assert_eq!(
        serde_json::to_value(decoded).expect("re-encode replaced state"),
        encoded,
        "replay/snapshot roundtrip must preserve the replacement identity"
    );
    let agent = &encoded["agents"][AGENT_ID];
    assert!(
        agent.get("activity").is_none(),
        "intent must not become activity"
    );
    assert!(
        agent.get("receipt").is_none(),
        "intent must not become a world receipt"
    );
}

#[test]
fn blocked_intent_can_resume_but_completion_requires_a_committed_receipt() {
    let accepted = canonical_intent("intent-lifecycle-2", "accepted", "Start recipe", None);
    let mut blocked = accepted.clone();
    blocked["status"] = serde_json::json!("blocked");
    blocked["reason_code"] = serde_json::json!("insufficient_power");
    blocked["reason_summary"] = serde_json::json!("Restore power to continue");
    blocked["event_seq"] = serde_json::json!(12);
    let mut resumed = accepted.clone();
    resumed["event_seq"] = serde_json::json!(13);
    resumed["updated_at"] = serde_json::json!(8);
    let mut completed_without_receipt = resumed.clone();
    completed_without_receipt["status"] = serde_json::json!("completed");
    completed_without_receipt["event_seq"] = serde_json::json!(14);
    let mut completed = completed_without_receipt.clone();
    completed["receipt_ref"] = serde_json::json!("world-event:14");

    let mut state = WorldState::default();
    state
        .agents
        .insert(AGENT_ID.to_string(), legacy_agent_cell());
    state
        .apply_domain_event(&intent_event("AgentIntentAccepted", accepted), 7)
        .unwrap();
    state
        .apply_domain_event(&intent_event("AgentIntentTransitioned", blocked), 7)
        .unwrap();
    assert_eq!(
        state.agents[AGENT_ID].intent.as_ref().unwrap().status,
        "blocked"
    );
    state
        .apply_domain_event(&intent_event("AgentIntentTransitioned", resumed), 8)
        .unwrap();
    assert_eq!(
        state.agents[AGENT_ID].intent.as_ref().unwrap().status,
        "accepted"
    );
    assert!(
        state
            .apply_domain_event(
                &intent_event("AgentIntentTransitioned", completed_without_receipt),
                9
            )
            .is_err()
    );
    state
        .apply_domain_event(
            &intent_event("AgentIntentTransitioned", completed.clone()),
            9,
        )
        .unwrap();
    assert_eq!(
        state.agents[AGENT_ID].intent.as_ref().unwrap().receipt_ref,
        Some("world-event:14".to_string())
    );
}

#[test]
fn terminal_intent_is_immutable_and_transition_replay_is_idempotent() {
    let accepted = canonical_intent("intent-terminal-1", "accepted", "Start recipe", None);
    let mut rejected = accepted.clone();
    rejected["status"] = serde_json::json!("rejected");
    rejected["reason_code"] = serde_json::json!("policy_denied");
    rejected["reason_summary"] = serde_json::json!("This instruction is not permitted");
    rejected["event_seq"] = serde_json::json!(12);
    let mut cancelled = rejected.clone();
    cancelled["status"] = serde_json::json!("cancelled");
    cancelled["event_seq"] = serde_json::json!(13);

    let mut state = WorldState::default();
    state
        .agents
        .insert(AGENT_ID.to_string(), legacy_agent_cell());
    state
        .apply_domain_event(&intent_event("AgentIntentAccepted", accepted), 7)
        .unwrap();
    let event = intent_event("AgentIntentTransitioned", rejected);
    state.apply_domain_event(&event, 8).unwrap();
    let terminal = serde_json::to_vec(&state).unwrap();
    state.apply_domain_event(&event, 9).unwrap();
    assert_eq!(serde_json::to_vec(&state).unwrap(), terminal);
    assert!(
        state
            .apply_domain_event(&intent_event("AgentIntentTransitioned", cancelled), 10)
            .is_err()
    );
    assert_eq!(serde_json::to_vec(&state).unwrap(), terminal);
}
