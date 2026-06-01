#[test]
fn restored_force_replan_plan_can_finalize_without_module_call_when_missing_is_empty() {
    let client = SequenceMockClient::new(vec![
        r#"{"decision":"harvest_radiation","max_amount":5}"#.to_string(),
        r#"{"decision":"harvest_radiation","max_amount":5}"#.to_string(),
        r#"{"type":"plan","missing":[],"next":"module_call"}"#.to_string(),
        r#"{"decision":"move_agent","to":"loc-2"}"#.to_string(),
    ]);

    let mut config = base_config();
    config.max_decision_steps = 5;
    config.max_repair_rounds = 1;
    config.force_replan_after_same_action = 2;
    config.execute_until_auto_reenter_ticks = 0;

    let mut behavior = LlmAgentBehavior::new("agent-1", config, client);

    let mut observation = make_observation();
    observation.time = 70;
    let first = behavior.decide(&observation);
    assert!(matches!(
        first,
        AgentDecision::Act(Action::HarvestRadiation { .. })
    ));

    observation.time = 71;
    let second = behavior.decide(&observation);
    assert!(matches!(
        second,
        AgentDecision::Act(Action::HarvestRadiation { .. })
    ));

    observation.time = 72;
    let third = behavior.decide(&observation);
    assert!(matches!(
        third,
        AgentDecision::Act(Action::MoveAgent { .. })
    ));

    let trace = behavior.take_decision_trace().expect("trace exists");
    assert!(trace.parse_error.is_none());
    assert!(trace.llm_effect_intents.is_empty());
    assert!(trace
        .llm_step_trace
        .iter()
        .any(|step| step.output_summary.contains("deprecated in dialogue mode")));
}

#[test]
fn restored_dialogue_module_and_draft_flow_still_works() {
    let client = SequenceMockClient::new(vec![
        r#"{"type":"module_call","module":"agent.modules.list","args":{}}"#.to_string(),
        r#"{"type":"decision_draft","decision":{"decision":"move_agent","to":"loc-2"},"confidence":0.78,"need_verify":false}"#.to_string(),
    ]);
    let mut config = base_config();
    config.max_decision_steps = 6;
    let mut behavior = LlmAgentBehavior::new("agent-1", config, client);

    let decision = behavior.decide(&make_observation());
    assert!(matches!(
        decision,
        AgentDecision::Act(Action::MoveAgent { .. })
    ));

    let trace = behavior.take_decision_trace().expect("trace exists");
    assert!(trace.parse_error.is_none());
    assert_eq!(trace.llm_effect_intents.len(), 1);
    assert_eq!(trace.llm_effect_receipts.len(), 1);
    assert!(trace
        .llm_step_trace
        .iter()
        .any(|step| step.output_summary.contains("decision_draft")));
}
