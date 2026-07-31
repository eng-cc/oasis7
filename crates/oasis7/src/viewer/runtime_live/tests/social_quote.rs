use super::*;
use crate::simulator::{ResourceOwner, SocialFactLifecycleState, SocialFactState};

fn seed_active_social_backing_fact(server: &mut ViewerRuntimeLiveServer, agent_id: &str) -> u64 {
    let fact_id = 77;
    server
        .seed_model
        .get_or_insert_default()
        .social_facts
        .insert(
            fact_id,
            SocialFactState {
                fact_id,
                actor: ResourceOwner::Agent {
                    agent_id: agent_id.to_string(),
                },
                schema_id: "social.relation.v1".to_string(),
                subject: ResourceOwner::Agent {
                    agent_id: agent_id.to_string(),
                },
                object: Some(ResourceOwner::Agent {
                    agent_id: agent_id.to_string(),
                }),
                claim: "agent has cooperative history".to_string(),
                confidence_ppm: 750_000,
                evidence_event_ids: Vec::new(),
                ttl_ticks: None,
                expires_at_tick: None,
                stake: None,
                challenge: None,
                lifecycle: SocialFactLifecycleState::Active,
                created_at_tick: 0,
                updated_at_tick: 0,
            },
        );
    fact_id
}

fn signed_declare_social_edge_quote_request(
    agent_id: &str,
    backing_fact_id: u64,
    player_id: &str,
    nonce: u64,
    public_key_hex: &str,
    private_key_hex: &str,
) -> crate::viewer::DeclareSocialEdgeQuoteRequest {
    let mut request = crate::viewer::DeclareSocialEdgeQuoteRequest {
        schema_id: "social.relation.v1".to_string(),
        relation_kind: "trust".to_string(),
        from_agent_id: agent_id.to_string(),
        to_agent_id: agent_id.to_string(),
        weight_bps: 2_000,
        backing_fact_ids: vec![backing_fact_id],
        ttl_ticks: Some(12),
        player_id: player_id.to_string(),
        public_key: Some(public_key_hex.to_string()),
        auth: None,
    };
    request.auth = Some(
        crate::viewer::sign_declare_social_edge_quote_auth_proof(
            &request,
            nonce,
            public_key_hex,
            private_key_hex,
        )
        .expect("sign declare social edge quote auth"),
    );
    request
}

#[test]
fn runtime_declare_social_edge_quote_requires_auth_proof() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let err = server
        .handle_declare_social_edge_quote(crate::viewer::DeclareSocialEdgeQuoteRequest {
            schema_id: "social.relation.v1".to_string(),
            relation_kind: "trust".to_string(),
            from_agent_id: "agent-a".to_string(),
            to_agent_id: "agent-b".to_string(),
            weight_bps: 2_000,
            backing_fact_ids: vec![77],
            ttl_ticks: Some(12),
            player_id: "player-social-quote".to_string(),
            public_key: None,
            auth: None,
        })
        .expect_err("unsigned declare social edge quote must fail");

    assert_eq!(err.code, "auth_proof_required");
    assert_eq!(err.action_id.as_deref(), Some("quote_declare_social_edge"));
}

#[test]
fn runtime_declare_social_edge_quote_is_signed_deterministic_non_mutating_and_player_readable() {
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
    let backing_fact_id = seed_active_social_backing_fact(&mut server, agent_id.as_str());
    let (public_key, private_key) = test_signer(95);
    register_runtime_session(
        &mut server,
        "player-social-quote",
        Some(agent_id.as_str()),
        2795,
        public_key.as_str(),
        private_key.as_str(),
    );
    let state_before = server.world.state().clone();
    let request = signed_declare_social_edge_quote_request(
        agent_id.as_str(),
        backing_fact_id,
        "player-social-quote",
        2796,
        public_key.as_str(),
        private_key.as_str(),
    );

    let quote = server
        .handle_declare_social_edge_quote(request.clone())
        .expect("signed declare social edge quote");
    assert_eq!(quote.actor_id, agent_id);
    assert_eq!(quote.action_kind, "declare_social_edge");
    assert_eq!(quote.schema_id, request.schema_id);
    assert_eq!(quote.subject_id.as_deref(), Some(agent_id.as_str()));
    assert_eq!(quote.object_id.as_deref(), Some(agent_id.as_str()));
    assert_eq!(quote.claim_summary, "trust");
    assert_eq!(quote.confidence_ppm, None);
    assert_eq!(quote.stake_at_risk, 0);
    assert_eq!(quote.ttl_ticks, Some(12));
    assert!(
        quote
            .affected_relationships
            .contains(&"schema:social.relation.v1".to_string())
    );
    assert!(
        quote
            .affected_social_surfaces
            .contains(&"reputation".to_string())
    );
    assert_eq!(quote.recommended_social_action, "declare_edge");
    assert!(quote.why_this_action_matters.contains("trust"));
    assert_eq!(
        server
            .handle_declare_social_edge_quote(request)
            .expect("repeat signed declare social edge quote"),
        quote
    );
    assert_eq!(server.world.state(), &state_before);
}
