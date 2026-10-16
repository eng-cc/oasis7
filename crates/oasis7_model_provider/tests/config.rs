use oasis7_model_provider::{ModelProviderConfig, config_constants::*, config_value_for_env_key};

#[test]
fn shared_config_resolver_preserves_selected_provider_and_agent_goal_precedence() {
    let text = r#"
model = "base"
profile = "native"
[profiles.native]
model = "profile-model"
model_provider = "local"
[model_providers.local]
base_url = "http://127.0.0.1:1234/v1"
auth_token = "local-secret"
[llm]
short_term_goal = "full short goal"
long_term_goal = "full long goal"
[llm.agent_overrides.AGENT_1]
short_term_goal = "agent-specific goal"
"#;
    let value: toml::Value = toml::from_str(text).unwrap();
    let table = value.as_table().unwrap();
    let config = ModelProviderConfig::from_values_for_agent(
        |key| config_value_for_env_key(table, key),
        "agent--1",
    )
    .unwrap();
    let file = std::env::temp_dir().join(format!(
        "oasis7-model-config-{}-{}.toml",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::write(&file, text).unwrap();
    let loaded = ModelProviderConfig::from_config_file_for_agent(&file, "agent--1");
    std::fs::remove_file(&file).unwrap();
    let loaded = loaded.unwrap();
    assert_eq!(loaded.model, config.model);
    assert_eq!(loaded.short_term_goal, config.short_term_goal);
    assert_eq!(config.model, "profile-model");
    assert_eq!(config.base_url, "http://127.0.0.1:1234/v1");
    assert_eq!(config.api_key, "local-secret");
    assert_eq!(
        config.short_term_goal.as_deref(),
        Some("agent-specific goal")
    );
    assert_eq!(config.long_term_goal.as_deref(), Some("full long goal"));
    assert!(!format!("{config:?}").contains("local-secret"));
    assert_eq!(
        config_value_for_env_key(table, ENV_LLM_MODEL).as_deref(),
        Some("profile-model")
    );
}

#[test]
fn invalid_budget_and_timeout_fail_before_provider_construction() {
    for (key, value) in [
        (ENV_LLM_TIMEOUT_MS, "not-a-number"),
        (ENV_LLM_MAX_MODULE_CALLS, "0"),
    ] {
        assert!(
            ModelProviderConfig::from_values_for_agent(
                |k| match k {
                    ENV_LLM_BASE_URL => Some("http://127.0.0.1:1/v1".into()),
                    ENV_LLM_API_KEY => Some("secret".into()),
                    k if k == key => Some(value.into()),
                    _ => None,
                },
                "agent"
            )
            .is_err()
        );
    }
}
