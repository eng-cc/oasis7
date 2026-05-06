use super::*;

#[test]
fn runtime_gameplay_action_claim_uses_bound_player_agent_as_actor() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let claimer_agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    server
        .world
        .set_main_token_supply(crate::runtime::MainTokenSupplyState {
            total_supply: 1_000,
            circulating_supply: 1_000,
            ..crate::runtime::MainTokenSupplyState::default()
        });
    server
        .world
        .set_agent_reputation_score(claimer_agent_id.as_str(), 0)
        .expect("set reputation");
    server
        .world
        .set_main_token_account_balance_with_restricted(claimer_agent_id.as_str(), 0, 0, 650)
        .expect("seed slot-1 claim balance");
    server
        .world
        .submit_action(crate::runtime::Action::RegisterAgent {
            agent_id: "agent-claim-target".to_string(),
            pos: crate::geometry::GeoPos::new(0, 0, 0),
        });
    server.world.step().expect("register claim target");

    let (public_key, private_key) = test_signer(88);
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(claimer_agent_id.as_str()),
        87,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );
    let request = signed_gameplay_action_request(
        crate::viewer::GameplayActionRequest {
            action_id: crate::viewer::ACTION_CLAIM_AGENT.to_string(),
            target_agent_id: "agent-claim-target".to_string(),
            actor_agent_id: Some(claimer_agent_id.clone()),
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
        },
        88,
        public_key.as_str(),
        private_key.as_str(),
    );
    let ack = server
        .handle_gameplay_action(request)
        .expect("claim action accepted");
    assert_eq!(ack.action_id, crate::viewer::ACTION_CLAIM_AGENT);

    server.world.step().expect("apply claim");
    let claim = server
        .world
        .state()
        .agent_claims
        .get("agent-claim-target")
        .expect("claim stored");
    assert_eq!(claim.claim_owner_id, claimer_agent_id);
    assert_eq!(
        server.llm_sidecar.bound_agent_for_player("player-a"),
        Some(claimer_agent_id.as_str())
    );
}

#[test]
fn runtime_gameplay_action_claim_rejects_actor_agent_mismatch() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let claimer_agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    server
        .world
        .submit_action(crate::runtime::Action::RegisterAgent {
            agent_id: "agent-claim-target".to_string(),
            pos: crate::geometry::GeoPos::new(0, 0, 0),
        });
    server.world.step().expect("register claim target");

    let (public_key, private_key) = test_signer(89);
    let _ = register_runtime_session(
        &mut server,
        "player-a",
        Some(claimer_agent_id.as_str()),
        88,
        public_key.as_str(),
        private_key.as_str(),
    );
    let request = signed_gameplay_action_request(
        crate::viewer::GameplayActionRequest {
            action_id: crate::viewer::ACTION_CLAIM_AGENT.to_string(),
            target_agent_id: "agent-claim-target".to_string(),
            actor_agent_id: Some("other-agent".to_string()),
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
        },
        89,
        public_key.as_str(),
        private_key.as_str(),
    );
    let err = server
        .handle_gameplay_action(request)
        .expect_err("mismatched actor must fail");
    assert_eq!(err.code, "actor_agent_mismatch");
}

#[test]
fn runtime_gameplay_action_claim_rejects_target_agent_bound_to_other_player() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let claimer_agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    server
        .world
        .submit_action(crate::runtime::Action::RegisterAgent {
            agent_id: "agent-claim-target".to_string(),
            pos: crate::geometry::GeoPos::new(0, 0, 0),
        });
    server.world.step().expect("register claim target");

    let (claimer_public_key, claimer_private_key) = test_signer(90);
    let _ = register_runtime_session(
        &mut server,
        "player-a",
        Some(claimer_agent_id.as_str()),
        89,
        claimer_public_key.as_str(),
        claimer_private_key.as_str(),
    );
    let (target_public_key, target_private_key) = test_signer(91);
    let target_register_ack = register_runtime_session(
        &mut server,
        "player-b",
        Some("agent-claim-target"),
        90,
        target_public_key.as_str(),
        target_private_key.as_str(),
    );
    assert_eq!(
        target_register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );

    let request = signed_gameplay_action_request(
        crate::viewer::GameplayActionRequest {
            action_id: crate::viewer::ACTION_CLAIM_AGENT.to_string(),
            target_agent_id: "agent-claim-target".to_string(),
            actor_agent_id: Some(claimer_agent_id.clone()),
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
        },
        91,
        claimer_public_key.as_str(),
        claimer_private_key.as_str(),
    );
    let err = server
        .handle_gameplay_action(request)
        .expect_err("target agent bound to another player must fail");
    assert_eq!(err.code, "agent_control_forbidden");
    assert_eq!(err.target_agent_id.as_deref(), Some("agent-claim-target"));
}
