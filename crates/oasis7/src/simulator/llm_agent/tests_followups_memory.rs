#[test]
fn restored_llm_agent_long_term_memory_can_export_and_restore() {
    let mut behavior = LlmAgentBehavior::new("agent-1", base_config(), MockClient::default());
    behavior
        .memory
        .long_term
        .store_with_tags("mined node alpha", 10, vec!["mining".to_string()]);
    behavior.memory.long_term.store_with_tags(
        "factory beta stalled",
        12,
        vec!["factory".to_string()],
    );

    let exported = behavior.export_long_term_memory_entries();
    assert_eq!(exported.len(), 2);

    let mut restored = LlmAgentBehavior::new("agent-1", base_config(), MockClient::default());
    restored.restore_long_term_memory_entries(&exported);
    let restored_exported = restored.export_long_term_memory_entries();
    assert_eq!(restored_exported.len(), 2);
    assert!(restored_exported
        .iter()
        .any(|entry| entry.content.contains("mined node alpha")));
    assert!(restored_exported
        .iter()
        .any(|entry| entry.content.contains("factory beta stalled")));
}

#[test]
fn restored_player_message_enters_conversation_prompt() {
    let mut behavior = LlmAgentBehavior::new("agent-1", base_config(), MockClient::default());
    assert!(behavior.push_player_message(11, "please explain the next factory step"));

    let conversation_json = behavior.conversation_history_json_for_prompt();
    assert!(conversation_json.contains("please explain the next factory step"));
}
