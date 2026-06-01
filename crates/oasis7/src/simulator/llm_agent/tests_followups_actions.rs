#[test]
fn restored_llm_agent_runtime_prompt_overrides_take_effect() {
    let mut behavior = LlmAgentBehavior::new("agent-1", base_config(), MockClient::default());
    behavior.apply_prompt_overrides(
        Some("runtime-system".to_string()),
        Some("runtime-short".to_string()),
        Some("runtime-long".to_string()),
    );

    let system_prompt = behavior.system_prompt();
    assert!(system_prompt.contains("runtime-system"));
    assert!(system_prompt.contains("runtime-short"));
    assert!(system_prompt.contains("runtime-long"));
    assert!(!system_prompt.contains("short-goal"));
    assert!(!system_prompt.contains("long-goal"));
}

#[test]
fn restored_llm_agent_user_prompt_contains_failure_recovery_policy() {
    let behavior = LlmAgentBehavior::new("agent-1", base_config(), MockClient::default());
    let prompt = behavior.user_prompt(&make_observation(), &[], 0, 4);

    assert!(prompt.contains("action_rejected"));
    assert!(prompt.contains("reject_reason"));
    assert!(prompt.contains("build_factory"));
    assert!(prompt.contains("schedule_recipe"));
}

#[test]
fn restored_harvest_amount_is_clamped_by_runtime_policy() {
    let client = MockClient {
        output: Some(r#"{"decision":"harvest_radiation","max_amount":999999}"#.to_string()),
        err: None,
    };
    let mut config = base_config();
    config.harvest_max_amount_cap = 42;
    let mut behavior = LlmAgentBehavior::new("agent-1", config, client);

    let decision = behavior.decide(&make_observation());
    match decision {
        AgentDecision::Act(Action::HarvestRadiation { max_amount, .. }) => {
            assert_eq!(max_amount, 42);
        }
        other => panic!("unexpected decision: {other:?}"),
    }
}
