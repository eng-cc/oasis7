#[test]
fn restored_provider_config_keeps_remote_bridge_metadata() {
    let config = LlmAgentProviderConfig::new()
        .with_provider_config_ref("remote_https:newapi")
        .with_agent_profile("oasis7-test-agent")
        .with_environment_class("world_simulator_mvp")
        .with_fallback_reason("bridge-first")
        .with_fixture_id("fixture-1")
        .with_replay_id("replay-1")
        .with_timeout_budget_ms(0)
        .with_memory_summary("memory-summary");

    assert_eq!(config.provider_config_ref.as_deref(), Some("remote_https:newapi"));
    assert_eq!(config.agent_profile.as_deref(), Some("oasis7-test-agent"));
    assert_eq!(config.environment_class.as_deref(), Some("world_simulator_mvp"));
    assert_eq!(config.fallback_reason.as_deref(), Some("bridge-first"));
    assert_eq!(config.fixture_id.as_deref(), Some("fixture-1"));
    assert_eq!(config.replay_id.as_deref(), Some("replay-1"));
    assert_eq!(config.timeout_budget_ms, 1);
    assert_eq!(config.memory_summary.as_deref(), Some("memory-summary"));
}

#[test]
fn restored_provider_phase1_catalog_has_required_mvp_actions() {
    let action_refs = provider_phase1_action_catalog()
        .into_iter()
        .map(|entry| entry.action_ref)
        .collect::<Vec<_>>();

    for required in [
        "wait",
        "move_agent",
        "harvest_radiation",
        "mine_compound",
        "refine_compound",
        "build_factory",
        "schedule_recipe",
    ] {
        assert!(action_refs.contains(&required.to_string()), "missing {required}");
    }
}
