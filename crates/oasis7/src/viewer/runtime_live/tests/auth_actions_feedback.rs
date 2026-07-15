use super::*;

#[test]
fn runtime_gameplay_action_dispatch_rejection_emits_error_then_rejected_snapshot() {
    const ACTION_ID: &str = "build_factory_smelter_mk1";
    const ERROR_MESSAGE: &str = "gameplay_action requires auth proof";
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let request = crate::viewer::GameplayActionRequest {
        action_id: ACTION_ID.to_string(),
        target_agent_id: agent_id.clone(),
        actor_agent_id: None,
        player_id: "player-a".to_string(),
        public_key: None,
        auth: None,
    };
    let mut session = RuntimeLiveSession::new();
    session.subscribed.insert(ViewerStream::Snapshot);
    let (mut writer, peer) = test_writer_pair();

    server
        .handle_request(
            ViewerRequest::GameplayAction { request },
            &mut session,
            &mut writer,
        )
        .expect("synchronous gameplay rejection should remain a protocol response");
    writer.flush().expect("flush gameplay rejection responses");

    let responses = read_available_runtime_live_responses(&peer, Duration::from_millis(200));
    assert_eq!(
        responses.len(),
        2,
        "snapshot subscribers should receive rejection context immediately after the error"
    );
    match &responses[0] {
        ViewerResponse::GameplayActionError { error } => {
            assert_eq!(error.code, "auth_proof_required");
            assert_eq!(error.message, ERROR_MESSAGE);
            assert_eq!(error.action_id.as_deref(), Some(ACTION_ID));
            assert_eq!(error.target_agent_id.as_deref(), Some(agent_id.as_str()));
        }
        other => panic!("expected gameplay action error first, got {other:?}"),
    }
    match &responses[1] {
        ViewerResponse::Snapshot { snapshot } => {
            let gameplay = snapshot
                .player_gameplay
                .as_ref()
                .expect("player gameplay snapshot");
            assert_eq!(
                gameplay
                    .recent_feedback
                    .as_ref()
                    .expect("rejected feedback")
                    .stage,
                "rejected"
            );
            assert_eq!(
                gameplay.response_window_class.as_deref(),
                Some("request_rejected")
            );
        }
        other => panic!("expected rejection snapshot second, got {other:?}"),
    }
    assert_eq!(
        server
            .latest_player_gameplay_feedback
            .as_ref()
            .map(|feedback| (
                feedback.action.as_str(),
                feedback.stage.as_str(),
                feedback.target_agent_id.as_deref(),
                feedback.reason.as_deref(),
                feedback.hint.as_deref(),
                feedback.delta_logical_time,
                feedback.delta_event_seq,
            )),
        Some((
            "gameplay_action:build_factory_smelter_mk1",
            "rejected",
            Some(agent_id.as_str()),
            Some("auth_proof_required: gameplay_action requires auth proof"),
            Some("correct the rejected request before retrying"),
            0,
            0,
        ))
    );
}

#[test]
fn runtime_gameplay_action_dispatch_rejection_without_snapshot_subscription_emits_only_error() {
    const ACTION_ID: &str = "build_factory_smelter_mk1";
    const ERROR_MESSAGE: &str = "gameplay_action requires auth proof";
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let request = crate::viewer::GameplayActionRequest {
        action_id: ACTION_ID.to_string(),
        target_agent_id: agent_id.clone(),
        actor_agent_id: None,
        player_id: "player-a".to_string(),
        public_key: None,
        auth: None,
    };
    let mut session = RuntimeLiveSession::new();
    let (mut writer, peer) = test_writer_pair();

    server
        .handle_request(
            ViewerRequest::GameplayAction { request },
            &mut session,
            &mut writer,
        )
        .expect("synchronous gameplay rejection should remain a protocol response");
    writer.flush().expect("flush gameplay rejection response");

    let responses = read_available_runtime_live_responses(&peer, Duration::from_millis(200));
    assert_eq!(
        responses.len(),
        1,
        "unsubscribed clients must not receive a snapshot"
    );
    match &responses[0] {
        ViewerResponse::GameplayActionError { error } => {
            assert_eq!(error.code, "auth_proof_required");
            assert_eq!(error.message, ERROR_MESSAGE);
            assert_eq!(error.action_id.as_deref(), Some(ACTION_ID));
            assert_eq!(error.target_agent_id.as_deref(), Some(agent_id.as_str()));
        }
        other => panic!("expected only gameplay action error, got {other:?}"),
    }
    let feedback = server
        .latest_player_gameplay_feedback
        .as_ref()
        .expect("server should retain rejected feedback without snapshot subscription");
    assert_eq!(feedback.action, "gameplay_action:build_factory_smelter_mk1");
    assert_eq!(feedback.stage, "rejected");
    assert_eq!(feedback.target_agent_id.as_deref(), Some(agent_id.as_str()));
}

#[test]
fn runtime_gameplay_action_rejection_feedback_classifies_operational_and_unknown_hints() {
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");

    server.record_gameplay_action_rejection(&crate::viewer::GameplayActionError {
        code: "chain_submit_unavailable".to_string(),
        message: "chain gameplay submit transport failed".to_string(),
        action_id: Some("mine".to_string()),
        target_agent_id: Some("agent-1".to_string()),
    });
    assert_eq!(
        server
            .latest_player_gameplay_feedback
            .as_ref()
            .and_then(|feedback| feedback.hint.as_deref()),
        Some("restore the chain gameplay submission path, then retry")
    );

    server.record_gameplay_action_rejection(&crate::viewer::GameplayActionError {
        code: "llm_mode_required".to_string(),
        message: "gameplay actions require LLM mode".to_string(),
        action_id: Some("mine".to_string()),
        target_agent_id: Some("agent-1".to_string()),
    });
    assert_eq!(
        server
            .latest_player_gameplay_feedback
            .as_ref()
            .and_then(|feedback| feedback.hint.as_deref()),
        Some("restore or wait for runtime and provider readiness, then retry")
    );

    server.record_gameplay_action_rejection(&crate::viewer::GameplayActionError {
        code: "future_gameplay_rejection".to_string(),
        message: "future rejection detail".to_string(),
        action_id: Some("mine".to_string()),
        target_agent_id: Some("agent-1".to_string()),
    });
    assert_eq!(
        server
            .latest_player_gameplay_feedback
            .as_ref()
            .and_then(|feedback| feedback.hint.as_deref()),
        Some("inspect the rejection details before retrying")
    );
}
