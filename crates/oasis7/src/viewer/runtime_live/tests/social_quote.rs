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

fn signed_publish_social_fact_quote_request(
    agent_id: &str,
    evidence_event_id: u64,
    player_id: &str,
    nonce: u64,
    public_key_hex: &str,
    private_key_hex: &str,
) -> crate::viewer::PublishSocialFactQuoteRequest {
    let mut request = crate::viewer::PublishSocialFactQuoteRequest {
        schema_id: "social.reputation.v1".to_string(),
        subject_agent_id: agent_id.to_string(),
        object_agent_id: Some(agent_id.to_string()),
        claim: "agent fulfilled the evidence-backed delivery".to_string(),
        confidence_ppm: 875_000,
        evidence_event_ids: vec![evidence_event_id],
        ttl_ticks: Some(12),
        stake: None,
        player_id: player_id.to_string(),
        public_key: Some(public_key_hex.to_string()),
        auth: None,
    };
    request.auth = Some(
        crate::viewer::sign_publish_social_fact_quote_auth_proof(
            &request,
            nonce,
            public_key_hex,
            private_key_hex,
        )
        .expect("sign publish social fact quote auth"),
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

#[test]
fn runtime_publish_social_fact_quote_is_signed_deterministic_non_mutating_and_player_readable() {
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
    let evidence_event_id = server
        .world
        .journal()
        .events
        .first()
        .expect("seed runtime evidence event")
        .id;
    let (public_key, private_key) = test_signer(96);
    register_runtime_session(
        &mut server,
        "player-publish-social-fact-quote",
        Some(agent_id.as_str()),
        2896,
        public_key.as_str(),
        private_key.as_str(),
    );
    let state_before = server.world.state().clone();
    let request = signed_publish_social_fact_quote_request(
        agent_id.as_str(),
        evidence_event_id,
        "player-publish-social-fact-quote",
        2897,
        public_key.as_str(),
        private_key.as_str(),
    );

    let quote = server
        .handle_publish_social_fact_quote(request.clone())
        .expect("signed publish social fact quote");
    assert_eq!(quote.actor_id, agent_id);
    assert_eq!(quote.action_kind, "publish_fact");
    assert_eq!(quote.schema_id, request.schema_id);
    assert_eq!(quote.subject_id.as_deref(), Some(agent_id.as_str()));
    assert_eq!(quote.object_id.as_deref(), Some(agent_id.as_str()));
    assert!(quote.claim_summary.contains("evidence-backed delivery"));
    assert_eq!(quote.confidence_ppm, Some(875_000));
    assert_eq!(quote.stake_at_risk, 0);
    assert_eq!(quote.ttl_ticks, Some(12));
    assert!(
        quote
            .affected_social_surfaces
            .contains(&"reputation".to_string())
    );
    assert_eq!(quote.cooperation_opportunity_delta, "positive");
    assert_eq!(quote.blacklist_or_dispute_risk, "challengeable_claim");
    assert_eq!(quote.governance_or_claim_relevance, "evidence_backed_claim");
    assert_eq!(quote.recommended_social_action, "publish_fact");
    assert!(quote.why_this_action_matters.contains(agent_id.as_str()));
    assert_eq!(
        server
            .handle_publish_social_fact_quote(request)
            .expect("repeat signed publish social fact quote"),
        quote
    );
    assert_eq!(server.world.state(), &state_before);
}

#[test]
fn runtime_publish_social_fact_quote_rejects_missing_runtime_evidence_without_mutation() {
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
    let (public_key, private_key) = test_signer(97);
    register_runtime_session(
        &mut server,
        "player-publish-social-fact-missing-evidence",
        Some(agent_id.as_str()),
        2898,
        public_key.as_str(),
        private_key.as_str(),
    );
    let state_before = server.world.state().clone();
    let err = server
        .handle_publish_social_fact_quote(signed_publish_social_fact_quote_request(
            agent_id.as_str(),
            999_999,
            "player-publish-social-fact-missing-evidence",
            2899,
            public_key.as_str(),
            private_key.as_str(),
        ))
        .expect_err("missing runtime evidence must reject publish social fact quote");

    assert_eq!(err.code, "publish_social_fact_quote_rejected");
    assert_eq!(err.action_id.as_deref(), Some("quote_publish_social_fact"));
    assert!(
        err.message
            .contains("social evidence event missing: 999999")
    );
    assert_eq!(server.world.state(), &state_before);
}

fn signed_social_contact_quote_request(
    contact_purpose: &str,
    first_contact_class: crate::viewer::FirstContactClass,
    player_id: &str,
    nonce: u64,
    public_key_hex: &str,
    private_key_hex: &str,
) -> crate::viewer::SocialContactQuoteRequest {
    let mut request = crate::viewer::SocialContactQuoteRequest {
        contact_purpose: contact_purpose.to_string(),
        first_contact_class,
        player_id: player_id.to_string(),
        public_key: Some(public_key_hex.to_string()),
        auth: None,
    };
    request.auth = Some(
        crate::viewer::sign_social_contact_quote_auth_proof(
            &request,
            nonce,
            public_key_hex,
            private_key_hex,
        )
        .expect("sign social contact quote auth"),
    );
    request
}

#[test]
fn runtime_social_contact_quote_is_authenticated_deterministic_non_mutating_and_has_only_the_six_readable_fields()
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
    let (public_key, private_key) = test_signer(98);
    register_runtime_session(
        &mut server,
        "player-social-contact-quote",
        Some(agent_id.as_str()),
        2998,
        public_key.as_str(),
        private_key.as_str(),
    );
    let state_before = server.world.state().clone();
    let request = signed_social_contact_quote_request(
        "sell a small surplus locally",
        crate::viewer::FirstContactClass::TradeOrService,
        "player-social-contact-quote",
        2999,
        public_key.as_str(),
        private_key.as_str(),
    );

    let quote = server
        .handle_social_contact_quote(request.clone())
        .expect("signed social contact quote");
    assert_eq!(
        quote.first_contact_class,
        crate::viewer::FirstContactClass::TradeOrService
    );
    assert_eq!(quote.contact_purpose, request.contact_purpose);
    assert!(!quote.expected_mutual_value.is_empty());
    assert!(!quote.risk_or_commitment.is_empty());
    assert!(quote.solo_lane_preserved);
    assert!(!quote.recommended_contact_action.is_empty());
    assert!(quote.defer_reason.is_empty());
    let quote_json = serde_json::to_value(&quote).expect("serialize social contact quote");
    assert_eq!(
        quote_json
            .as_object()
            .expect("social contact quote object")
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>(),
        [
            "contact_purpose",
            "expected_mutual_value",
            "first_contact_class",
            "risk_or_commitment",
            "solo_lane_preserved",
            "recommended_contact_action",
            "defer_reason",
        ]
        .into_iter()
        .collect(),
    );
    assert_eq!(
        server
            .handle_social_contact_quote(request)
            .expect("repeat signed social contact quote"),
        quote
    );
    assert_eq!(server.world.state(), &state_before);
}

#[test]
fn runtime_social_contact_quote_uses_closed_classes_and_explicitly_defers_ineligible_contact() {
    assert_eq!(
        serde_json::to_string(&crate::viewer::FirstContactClass::TradeOrService)
            .expect("serialize trade class"),
        "\"trade_or_service\""
    );
    assert_eq!(
        serde_json::to_string(&crate::viewer::FirstContactClass::MutualAid)
            .expect("serialize mutual aid class"),
        "\"mutual_aid\""
    );
    assert_eq!(
        serde_json::to_string(&crate::viewer::FirstContactClass::InformationExchange)
            .expect("serialize information exchange class"),
        "\"information_exchange\""
    );
    assert_eq!(
        serde_json::to_string(&crate::viewer::FirstContactClass::DeferContact)
            .expect("serialize defer class"),
        "\"defer_contact\""
    );
    assert_eq!(
        serde_json::to_string(&crate::viewer::FirstContactClass::OrganizationEscalation)
            .expect("serialize escalation class"),
        "\"organization_escalation\""
    );
    assert!(
        serde_json::from_str::<crate::viewer::FirstContactClass>("\"unbounded_diplomacy\"")
            .is_err()
    );

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
    let (public_key, private_key) = test_signer(99);
    register_runtime_session(
        &mut server,
        "player-social-contact-defer",
        Some(agent_id.as_str()),
        3098,
        public_key.as_str(),
        private_key.as_str(),
    );
    let state_before = server.world.state().clone();
    let quote = server
        .handle_social_contact_quote(signed_social_contact_quote_request(
            "",
            crate::viewer::FirstContactClass::TradeOrService,
            "player-social-contact-defer",
            3099,
            public_key.as_str(),
            private_key.as_str(),
        ))
        .expect("ineligible first contact returns an explicit defer quote");

    assert_eq!(
        quote.first_contact_class,
        crate::viewer::FirstContactClass::DeferContact
    );
    assert!(!quote.defer_reason.is_empty());
    assert!(quote.solo_lane_preserved);
    assert_ne!(
        quote.first_contact_class,
        crate::viewer::FirstContactClass::OrganizationEscalation
    );
    assert_eq!(server.world.state(), &state_before);
}

#[test]
fn runtime_social_contact_quote_preserves_low_commitment_classes_and_defers_escalation() {
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
    let (public_key, private_key) = test_signer(100);
    register_runtime_session(
        &mut server,
        "player-social-contact-classes",
        Some(agent_id.as_str()),
        3198,
        public_key.as_str(),
        private_key.as_str(),
    );
    let state_before = server.world.state().clone();

    for (nonce, class) in [
        (3199, crate::viewer::FirstContactClass::MutualAid),
        (3200, crate::viewer::FirstContactClass::InformationExchange),
    ] {
        let quote = server
            .handle_social_contact_quote(signed_social_contact_quote_request(
                "remove the current local blocker",
                class.clone(),
                "player-social-contact-classes",
                nonce,
                public_key.as_str(),
                private_key.as_str(),
            ))
            .expect("low-commitment first contact quote");
        assert_eq!(quote.first_contact_class, class);
        assert!(quote.defer_reason.is_empty());
        assert!(quote.solo_lane_preserved);
    }

    let escalation = server
        .handle_social_contact_quote(signed_social_contact_quote_request(
            "join a regional organization",
            crate::viewer::FirstContactClass::OrganizationEscalation,
            "player-social-contact-classes",
            3201,
            public_key.as_str(),
            private_key.as_str(),
        ))
        .expect("organization escalation is deferred from first contact");
    assert_eq!(
        escalation.first_contact_class,
        crate::viewer::FirstContactClass::DeferContact
    );
    assert!(!escalation.defer_reason.is_empty());
    assert!(escalation.solo_lane_preserved);
    assert_eq!(server.world.state(), &state_before);
}
