use super::*;

const SNAPSHOT_PLAYER_ID: &str = "player-snapshot";

#[test]
fn compat_snapshot_quotes_when_a_queued_intent_can_be_safely_waited_on() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    server.latest_player_gameplay_feedback = Some(crate::simulator::PlayerGameplayRecentFeedback {
        action: "gameplay_action:build_factory_smelter_mk1".to_string(),
        stage: "queued".to_string(),
        effect: "queued gameplay action build_factory_smelter_mk1 for agent-0 as runtime action 7"
            .to_string(),
        intent_summary: Some(
            "queue gameplay action build_factory_smelter_mk1 for agent-0".to_string(),
        ),
        target_agent_id: Some("agent-0".to_string()),
        reason: None,
        hint: Some("advance 1-2 steps to apply the queued gameplay action".to_string()),
        delta_logical_time: 0,
        delta_event_seq: 0,
    });

    let snapshot = server.compat_snapshot(Some(SNAPSHOT_PLAYER_ID));
    let serialized = serde_json::to_value(&snapshot).expect("compat snapshot serializes");
    let gameplay = serialized
        .get("player_gameplay")
        .and_then(serde_json::Value::as_object)
        .expect("queued gameplay snapshot");
    let quote = gameplay
        .get("wait_resolution_quote")
        .and_then(serde_json::Value::as_object)
        .expect("queued committed intent publishes its authoritative wait-resolution quote");

    for field in [
        "resolution_trigger",
        "recheck_tick_or_event",
        "expected_change",
        "unresolved_risk",
        "alternative_unlock_condition",
    ] {
        assert!(
            quote
                .get(field)
                .and_then(serde_json::Value::as_str)
                .is_some(),
            "wait-resolution quote publishes {field}"
        );
    }
    assert_eq!(
        quote
            .get("safe_to_wait")
            .and_then(serde_json::Value::as_bool),
        Some(false),
        "an available advance_step is not, by itself, a safe-wait authorization"
    );
    assert!(
        gameplay["available_actions"]
            .as_array()
            .is_some_and(|actions| actions
                .iter()
                .any(|action| action["action_id"] == "advance_step")),
        "the contract distinguishes the offered advance action from a safe wait"
    );

    let mut legacy_gameplay = serde_json::Value::Object(gameplay.clone());
    legacy_gameplay
        .as_object_mut()
        .expect("legacy gameplay object")
        .remove("wait_resolution_quote");
    serde_json::from_value::<crate::simulator::PlayerGameplaySnapshot>(legacy_gameplay)
        .expect("snapshots written before wait-resolution quotes remain readable");
}

#[test]
fn compat_snapshot_omits_wait_resolution_quote_for_blocked_feedback() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    server.latest_player_gameplay_feedback = Some(crate::simulator::PlayerGameplayRecentFeedback {
        action: "gameplay_action:build_factory_smelter_mk1".to_string(),
        stage: "blocked".to_string(),
        effect: "factory construction remains blocked".to_string(),
        intent_summary: Some(
            "queue gameplay action build_factory_smelter_mk1 for agent-0".to_string(),
        ),
        target_agent_id: Some("agent-0".to_string()),
        reason: Some("required material is unavailable".to_string()),
        hint: Some("repair the material supply before retrying".to_string()),
        delta_logical_time: 0,
        delta_event_seq: 0,
    });

    let serialized = serde_json::to_value(server.compat_snapshot(Some(SNAPSHOT_PLAYER_ID)))
        .expect("compat snapshot serializes");
    let gameplay = serialized
        .get("player_gameplay")
        .and_then(serde_json::Value::as_object)
        .expect("blocked gameplay snapshot");

    assert!(
        gameplay
            .get("wait_resolution_quote")
            .is_none_or(serde_json::Value::is_null),
        "blocked feedback is not presented as a safe-wait quote"
    );
}
