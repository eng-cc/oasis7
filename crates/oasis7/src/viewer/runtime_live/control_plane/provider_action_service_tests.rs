use super::*;

#[test]
fn canonical_chat_refusal_precedes_auth_binding_and_world_mutation() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .unwrap();
    let request = AgentChatRequest {
        agent_id: "agent-a".into(),
        player_id: Some("player-a".into()),
        public_key: None,
        auth: None,
        message: "hello".into(),
        intent_tick: Some(1),
        intent_seq: Some(1),
        world_id: None,
        reorg_epoch: None,
        authority_scope: None,
        replaces_intent_id: None,
    };
    server.config.chain_status_bind = Some("127.0.0.1:1".into());
    let before = serde_json::to_value(server.world.snapshot()).unwrap();
    let before_sidecar = server.test_canonical_provider_summary();
    assert_eq!(
        server.handle_agent_chat(request.clone()).unwrap_err().code,
        "canonical_chat_unsupported"
    );
    server.complete_agent_chat_if_receipt_bound("agent-a", "intent", "digest");
    assert_eq!(
        server.enqueue_pending_provider_agent_chat_replies()[0].code,
        "canonical_chat_unsupported"
    );
    assert_eq!(
        serde_json::to_value(server.world.snapshot()).unwrap(),
        before
    );
    assert_eq!(server.test_canonical_provider_summary(), before_sidecar);
    server.config.chain_status_bind = None;
    server.config.world_service = None;
    assert_eq!(
        server.handle_agent_chat(request).unwrap_err().code,
        "llm_mode_required"
    );
}

#[test]
fn offline_provider_idle_pass_preserves_world_without_service() {
    let mut server =
        ViewerRuntimeLiveServer::new(ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal))
            .expect("offline server");
    server.config.world_service = None;
    server.config.chain_status_bind = None;
    assert!(!server.chain_link_enabled());
    let before = serde_json::to_value(server.world.snapshot()).unwrap();
    assert!(server.enqueue_llm_action_from_sidecar().unwrap().is_none());
    server.retry_committed_provider_action().unwrap();
    assert_eq!(
        serde_json::to_value(server.world.snapshot()).unwrap(),
        before
    );
}

#[test]
fn chain_linked_provider_pass_cannot_mutate_projection_when_service_is_unavailable() {
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_chain_status_bind("127.0.0.1:1"),
    )
    .expect("create chain-linked provider server");
    let before = serde_json::to_value(server.world.snapshot()).expect("snapshot before");
    let error = server
        .enqueue_llm_action_from_sidecar()
        .expect_err("canonical service owns mutations");
    assert!(
        error
            .llm_error
            .as_deref()
            .is_some_and(|message| message.contains("provider turn remains pending"))
    );
    assert_eq!(
        serde_json::to_value(server.world.snapshot()).expect("snapshot after"),
        before
    );
    assert!(server.retry_committed_provider_action().is_err());
    assert_eq!(
        serde_json::to_value(server.world.snapshot()).expect("snapshot after recovery"),
        before
    );
}
