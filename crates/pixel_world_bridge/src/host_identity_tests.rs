use super::*;

fn surface(gameplay: Value, agents: &[Value], selected: Value) -> Value {
    build_commercial_surface("en", &gameplay, agents, &[], &[], &[], &selected)
}

#[test]
fn observation_recommendation_and_in_flight_receipt_keep_distinct_subjects() {
    let mut agents = vec![json!({"id":"A"}), json!({"id":"B"})];
    for _ in 0..2 {
        let state = surface(
            json!({
                "acceptedIntentId":"original-A", "acceptedIntentTarget":"A",
                "executionState":"executing",
                "recommendedAction":{"targetAgentId":"B"}
            }),
            &agents,
            json!({"kind":"agent","id":"B"}),
        );
        assert_eq!(state["active_agent_id"], "B");
        assert_eq!(state["next_action"]["target_agent_id"], "B");
        assert_eq!(state["action_receipt"]["target_agent_id"], "A");
        agents.reverse();
    }
}

#[test]
fn missing_subjects_do_not_borrow_selection_or_first_agent() {
    let agents = vec![json!({"id":"A"}), json!({"id":"B"})];
    let state = surface(
        json!({
            "executionState":"sent", "recentFeedback":{"action":"submit"},
            "availableActions":[{"actionId":"claim_first_agent","targetAgentId":"A"}]
        }),
        &agents,
        json!({"kind":"agent","id":"B"}),
    );
    assert_eq!(state["active_agent_id"], "B");
    assert!(state["next_action"]["target_agent_id"].is_null());
    assert!(state["action_receipt"]["target_agent_id"].is_null());
    let unselected = surface(
        json!({"recommendedAction":{"targetAgentId":"A"}}),
        &agents,
        Value::Null,
    );
    assert!(unselected["active_agent_id"].is_null());
}

#[test]
fn explicit_player_level_receipt_stays_null_despite_stale_intent() {
    for binding in [
        json!({"receiptTargetAgentId":null}),
        json!({"recentFeedback":{"agentId":null,"action":"submit"}}),
    ] {
        let mut gameplay =
            json!({"acceptedIntentId":"old-A","acceptedIntentTarget":"A","executionState":"sent"});
        gameplay
            .as_object_mut()
            .unwrap()
            .extend(binding.as_object().unwrap().clone());
        let state = surface(
            gameplay,
            &[json!({"id":"B"})],
            json!({"kind":"agent","id":"B"}),
        );
        assert!(state["action_receipt"]["target_agent_id"].is_null());
    }
}

#[test]
fn original_local_receipt_binding_wins_over_runtime_intent_and_recommendation() {
    let state = surface(
        json!({
            "acceptedIntentId":"old-A", "acceptedIntentTarget":"A", "executionState":"sent",
            "receiptTargetAgentId":"C", "recentFeedback":{"agentId":"C","action":"submit"},
            "recommendedAction":{"targetAgentId":"B"}
        }),
        &[json!({"id":"B"})],
        json!({"kind":"agent","id":"B"}),
    );
    assert_eq!(state["action_receipt"]["target_agent_id"], "C");
}

#[test]
fn render_projection_keeps_receipt_a_after_selection_moves_to_b() {
    let state = build_render_state(&json!({
        "lists":{"agents":[{"id":"B"},{"id":"A"}]},
        "selectedKind":"agent", "selectedId":"B",
        "gameplay":{
            "executionState":"sent", "receiptTargetAgentId":"A",
            "recentFeedback":{"action":"build","stage":"sent"},
            "recommendedAction":{"targetAgentId":"B"}
        }
    }));
    assert_eq!(state["selection"]["id"], "B");
    assert_eq!(state["commercial_surface"]["active_agent_id"], "B");
    assert_eq!(
        state["commercial_surface"]["action_receipt"]["target_agent_id"],
        "A"
    );
    assert_eq!(state["receipt_target"]["agent_id"], "A");
}
