#[test]
fn restored_llm_agent_client_error_falls_back_to_wait_with_trace() {
    let client = MockClient {
        output: None,
        err: Some(LlmClientError::HttpStatus {
            code: 503,
            message: "bridge unavailable".to_string(),
        }),
    };
    let mut behavior = LlmAgentBehavior::new("agent-1", base_config(), client);

    let decision = behavior.decide(&make_observation());
    assert_eq!(decision, AgentDecision::Wait);

    let trace = behavior.take_decision_trace().expect("trace exists");
    let llm_error = trace.llm_error.expect("llm error");
    assert!(llm_error.contains("503"));
    assert!(llm_error.contains("bridge unavailable"));
}

#[test]
fn restored_llm_agent_empty_output_uses_repair_budget_then_waits() {
    let client = SequenceMockClient::new(vec!["".to_string(), "".to_string()]);
    let mut config = base_config();
    config.max_repair_rounds = 1;
    config.max_decision_steps = 2;
    let mut behavior = LlmAgentBehavior::new("agent-1", config, client);

    let decision = behavior.decide(&make_observation());
    assert_eq!(decision, AgentDecision::Wait);

    let trace = behavior.take_decision_trace().expect("trace exists");
    assert!(trace
        .parse_error
        .as_deref()
        .is_some_and(|value| value.contains("no actionable")));
    assert_eq!(trace.llm_diagnostics.expect("diagnostics").retry_count, 1);
}

#[test]
fn restored_prompt_profile_uses_relaxed_token_budget_for_stability() {
    let compact = LlmPromptProfile::Compact.prompt_budget();
    assert_eq!(compact.context_window_tokens, 4_096);
    assert_eq!(compact.reserved_output_tokens, 768);
    assert_eq!(compact.safety_margin_tokens, 352);
    assert_eq!(compact.effective_input_budget_tokens(), 2_976);

    let balanced = LlmPromptProfile::Balanced.prompt_budget();
    assert_eq!(balanced.context_window_tokens, 4_608);
    assert_eq!(balanced.reserved_output_tokens, 896);
    assert_eq!(balanced.safety_margin_tokens, 480);
    assert_eq!(balanced.effective_input_budget_tokens(), 3_232);
}
