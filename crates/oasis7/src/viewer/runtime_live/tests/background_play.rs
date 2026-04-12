use super::*;

#[test]
fn runtime_background_play_retries_transient_llm_access_failure_after_prior_progress() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    std::env::remove_var(crate::simulator::ENV_LLM_MODEL);
    std::env::remove_var(crate::simulator::ENV_LLM_BASE_URL);
    std::env::remove_var(crate::simulator::ENV_LLM_API_KEY);

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    server
        .world
        .step()
        .expect("advance world once before failure");

    let baseline_time = server.world.state().time;
    let (mut writer, _client) = test_writer_pair();
    let mut session = RuntimeLiveSession::new();
    session.playing = true;

    server
        .advance_runtime(&mut session, &mut writer, "play", 1, None, false)
        .expect("play loop handled");

    assert!(
        session.playing,
        "background play should keep retrying after the first transient LLM failure once progress exists"
    );
    assert_eq!(
        session.transient_play_failures, 1,
        "first transient failure should increment retry budget"
    );
    assert_eq!(
        server.world.state().time,
        baseline_time,
        "failed retry should not advance world time"
    );
    let feedback = server
        .latest_player_gameplay_feedback
        .as_ref()
        .expect("blocked feedback recorded");
    assert_eq!(feedback.action, "play");
    assert_eq!(feedback.stage, "blocked");
    assert!(feedback.effect.contains("will retry on the next play tick"));
}

#[test]
fn runtime_background_play_stops_after_retry_budget_exhausted() {
    let _guard = runtime_provider_env_lock().lock().expect("env lock");
    clear_runtime_provider_env();
    std::env::remove_var(crate::simulator::ENV_LLM_MODEL);
    std::env::remove_var(crate::simulator::ENV_LLM_BASE_URL);
    std::env::remove_var(crate::simulator::ENV_LLM_API_KEY);

    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    server
        .world
        .step()
        .expect("advance world once before failure");

    let (mut writer, _client) = test_writer_pair();
    let mut session = RuntimeLiveSession::new();
    session.playing = true;

    for expected_failures in 1..BACKGROUND_PLAY_TRANSIENT_FAILURE_BUDGET {
        server
            .advance_runtime(&mut session, &mut writer, "play", 1, None, false)
            .expect("play loop handled");
        assert!(
            session.playing,
            "background play should still retry before budget is exhausted"
        );
        assert_eq!(session.transient_play_failures, expected_failures);
    }

    server
        .advance_runtime(&mut session, &mut writer, "play", 1, None, false)
        .expect("play loop handled");
    assert!(
        !session.playing,
        "background play should stop once transient failure budget is exhausted"
    );
    assert_eq!(
        session.transient_play_failures,
        BACKGROUND_PLAY_TRANSIENT_FAILURE_BUDGET,
    );
}
