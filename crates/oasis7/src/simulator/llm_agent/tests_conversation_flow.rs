#[test]
fn restored_llm_agent_repair_round_can_recover_invalid_output() {
    let client = SequenceMockClient::new(vec![
        "not-json".to_string(),
        r#"{"decision":"wait_ticks","ticks":2}"#.to_string(),
    ]);
    let mut config = base_config();
    config.max_repair_rounds = 1;
    config.max_decision_steps = 4;
    let mut behavior = LlmAgentBehavior::new("agent-1", config, client);

    let decision = behavior.decide(&make_observation());
    assert_eq!(decision, AgentDecision::WaitTicks(2));

    let trace = behavior.take_decision_trace().expect("trace exists");
    let diagnostics = trace.llm_diagnostics.expect("diagnostics");
    assert_eq!(diagnostics.retry_count, 1);
    assert!(trace
        .llm_step_trace
        .iter()
        .any(|step| step.step_type == "repair"));
}

#[test]
fn restored_llm_agent_limits_module_call_rounds() {
    let client = SequenceMockClient::new(vec![
        r#"{"type":"module_call","module":"agent.modules.list","args":{}}"#.to_string(),
        r#"{"type":"module_call","module":"agent.modules.list","args":{}}"#.to_string(),
    ]);

    let mut config = base_config();
    config.max_module_calls = 1;
    let mut behavior = LlmAgentBehavior::new("agent-1", config, client);

    let decision = behavior.decide(&make_observation());
    assert_eq!(decision, AgentDecision::Wait);

    let trace = behavior.take_decision_trace().expect("trace exists");
    assert_eq!(trace.llm_effect_intents.len(), 1);
    assert_eq!(trace.llm_effect_receipts.len(), 1);
}

#[test]
fn restored_llm_agent_collapses_mixed_multi_turn_output_to_last_terminal_decision() {
    let calls = Arc::new(AtomicUsize::new(0));
    let client = CountingSequenceMockClient::new(
        vec![
            r#"{"type":"module_call","module":"agent.modules.list","args":{}}

---

{"type":"module_call","module":"environment.current_observation","args":{}}

---

{"decision":"wait"}"#
                .to_string(),
        ],
        Arc::clone(&calls),
    );

    let mut config = base_config();
    config.max_module_calls = 2;
    let mut behavior = LlmAgentBehavior::new("agent-1", config, client);

    let decision = behavior.decide(&make_observation());
    assert_eq!(decision, AgentDecision::Wait);
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    let trace = behavior.take_decision_trace().expect("trace exists");
    assert!(trace.parse_error.is_none());
    assert_eq!(
        trace.llm_diagnostics.expect("diagnostics").retry_count,
        0,
        "collapsed output should not consume repair rounds"
    );
    assert!(trace.llm_chat_messages.iter().any(|msg| msg
        .content
        .contains("multi-turn output collapsed by guardrail")));
}
