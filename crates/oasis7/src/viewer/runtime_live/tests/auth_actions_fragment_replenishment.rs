use super::*;

fn signed_fragment_refill_preview_request(
    chunk: crate::simulator::ChunkCoord,
    player_id: &str,
    nonce: u64,
    public_key_hex: &str,
    private_key_hex: &str,
) -> crate::viewer::FragmentRefillPreviewRequest {
    let mut request = crate::viewer::FragmentRefillPreviewRequest {
        chunk,
        player_id: player_id.to_string(),
        public_key: Some(public_key_hex.to_string()),
        auth: None,
    };
    request.auth = Some(
        crate::viewer::sign_fragment_refill_preview_auth_proof(
            &request,
            nonce,
            public_key_hex,
            private_key_hex,
        )
        .expect("sign fragment refill preview auth"),
    );
    request
}

#[test]
fn runtime_fragment_refill_preview_is_authenticated_deterministic_non_mutating_and_player_readable()
{
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
    let (public_key, private_key) = test_signer(254);
    register_runtime_session(
        &mut server,
        "player-fragment-refill-preview",
        Some(agent_id.as_str()),
        281,
        public_key.as_str(),
        private_key.as_str(),
    );
    let chunk = crate::simulator::ChunkCoord { x: 0, y: 0, z: 0 };
    let mut seed_model = server.seed_model.clone().unwrap_or_default();
    seed_model
        .chunks
        .insert(chunk, crate::simulator::ChunkState::Generated);
    server.seed_model = Some(seed_model);
    let state_before = server.world.state().clone();
    let request = signed_fragment_refill_preview_request(
        chunk,
        "player-fragment-refill-preview",
        282,
        public_key.as_str(),
        private_key.as_str(),
    );

    let quote = server
        .handle_fragment_refill_preview(request.clone())
        .expect("signed fragment refill preview");

    assert!(quote.next_replenish_tick.is_some() || !quote.replenishment_enabled);
    assert!(quote.estimated_replenished_frag_count >= 0);
    assert!(!quote.next_industrial_goal_relevance.is_empty());
    assert!(!quote.recommended_resource_action.is_empty());
    assert_eq!(
        server
            .handle_fragment_refill_preview(request)
            .expect("repeat preview"),
        quote
    );
    assert_eq!(server.world.state(), &state_before);
}

#[test]
fn runtime_fragment_refill_preview_requires_an_auth_proof() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");

    let error = server
        .handle_fragment_refill_preview(crate::viewer::FragmentRefillPreviewRequest {
            chunk: crate::simulator::ChunkCoord { x: 0, y: 0, z: 0 },
            player_id: "player-fragment-refill-preview".to_string(),
            public_key: None,
            auth: None,
        })
        .expect_err("unsigned preview must fail");

    assert_eq!(error.code, "auth_proof_required");
    assert_eq!(
        error.action_id.as_deref(),
        Some("preview_fragment_replenishment")
    );
}
