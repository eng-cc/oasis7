use super::*;

#[test]
fn llm_agent_preserves_recent_replays_across_action_id_rollover() {
    const REPLAY_WINDOW: u64 = 64;
    let recipe_id = "recipe.assembler.control_chip";
    let mut behavior = LlmAgentBehavior::new("agent-1", base_config(), MockClient::default());
    let high_start = u64::MAX - (REPLAY_WINDOW - 1);

    for offset in 0..REPLAY_WINDOW {
        behavior.on_event(&runtime_recipe_completed_event_with_identity(
            "agent-1", recipe_id, 20_000 + offset, high_start + offset,
        ));
    }
    assert_eq!(
        behavior.recipe_coverage.completion_receipt_ids.len(),
        REPLAY_WINDOW as usize
    );

    behavior.on_event(&runtime_recipe_completed_event_with_identity(
        "agent-1", recipe_id, 30_000, 1,
    ));
    assert!(behavior.recipe_coverage.completion_receipt_ids.contains(&1));
    let memory_after_rollover = behavior.memory.short_term.len();
    behavior.on_event(&runtime_recipe_completed_event_with_identity(
        "agent-1", recipe_id, 30_001, 1,
    ));
    assert_eq!(behavior.memory.short_term.len(), memory_after_rollover);

    for low_job_id in 2..=REPLAY_WINDOW {
        behavior.on_event(&runtime_recipe_completed_event_with_identity(
            "agent-1", recipe_id, 30_001 + low_job_id, low_job_id,
        ));
    }
    assert!(!behavior
        .recipe_coverage
        .completion_receipt_ids
        .contains(&high_start));
    behavior.on_event(&runtime_recipe_completed_event_with_identity(
        "agent-1", recipe_id, 31_000, REPLAY_WINDOW + 1,
    ));
    assert_eq!(
        behavior.recipe_coverage.completion_receipt_ids.len(),
        REPLAY_WINDOW as usize
    );
    assert!(!behavior.recipe_coverage.completion_receipt_ids.contains(&1));
    behavior.on_event(&runtime_recipe_completed_event_with_identity(
        "agent-1", recipe_id, 31_001, 1,
    ));
    assert!(behavior.recipe_coverage.completion_receipt_ids.contains(&1));
}
