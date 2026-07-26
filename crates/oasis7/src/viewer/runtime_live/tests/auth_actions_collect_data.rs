use super::*;

fn signed_collect_data_command(
    mode: crate::viewer::CollectDataCommand,
    nonce: u64,
    public_key_hex: &str,
    private_key_hex: &str,
) -> crate::viewer::CollectDataCommand {
    let mut command = mode;
    match &mut command {
        crate::viewer::CollectDataCommand::Preflight { request }
        | crate::viewer::CollectDataCommand::Submit { request } => {
            request.public_key = Some(public_key_hex.to_string());
        }
    }
    let proof = crate::viewer::sign_collect_data_auth_proof(
        &command,
        nonce,
        public_key_hex,
        private_key_hex,
    )
    .expect("sign collect_data auth");
    match &mut command {
        crate::viewer::CollectDataCommand::Preflight { request }
        | crate::viewer::CollectDataCommand::Submit { request } => request.auth = Some(proof),
    }
    command
}

fn signed_refine_quote_request(
    compound_mass_g: i64,
    player_id: &str,
    nonce: u64,
    public_key_hex: &str,
    private_key_hex: &str,
) -> crate::viewer::RefineQuoteRequest {
    let mut request = crate::viewer::RefineQuoteRequest {
        compound_mass_g,
        player_id: player_id.to_string(),
        public_key: Some(public_key_hex.to_string()),
        auth: None,
    };
    request.auth = Some(
        crate::viewer::sign_refine_quote_auth_proof(
            &request,
            nonce,
            public_key_hex,
            private_key_hex,
        )
        .expect("sign refine quote auth"),
    );
    request
}

fn signed_product_validation_quote_request(
    product_id: &str,
    amount: i64,
    player_id: &str,
    nonce: u64,
    public_key_hex: &str,
    private_key_hex: &str,
) -> crate::viewer::ProductValidationQuoteRequest {
    let mut request = crate::viewer::ProductValidationQuoteRequest {
        product_id: product_id.to_string(),
        amount,
        player_id: player_id.to_string(),
        public_key: Some(public_key_hex.to_string()),
        auth: None,
    };
    request.auth = Some(
        crate::viewer::sign_product_validation_quote_auth_proof(
            &request,
            nonce,
            public_key_hex,
            private_key_hex,
        )
        .expect("sign product validation quote auth"),
    );
    request
}

#[test]
fn runtime_product_validation_quote_is_signed_read_only_and_advisory_before_submission() {
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
    let (public_key, private_key) = test_signer(251);
    register_runtime_session(
        &mut server,
        "player-product-quote",
        Some(agent_id.as_str()),
        251,
        public_key.as_str(),
        private_key.as_str(),
    );
    let state_before = server.world.state().clone();
    let quote = server
        .handle_product_validation_quote(signed_product_validation_quote_request(
            "logistics_drone",
            1,
            "player-product-quote",
            252,
            public_key.as_str(),
            private_key.as_str(),
        ))
        .expect("signed product validation quote");

    assert_eq!(quote.product_id, "logistics_drone");
    assert_eq!(quote.product_role, "explore");
    assert_eq!(quote.stage_before, "bootstrap");
    assert_eq!(quote.stage_after, "bootstrap");
    assert_eq!(quote.unlock_or_value_class, "scale_out");
    assert!(
        quote.submission_allowed,
        "stage guidance must remain advisory"
    );
    assert_eq!(quote.missing_prerequisite, "industry_stage=scale_out");
    assert_eq!(
        quote.reachable_advance_or_recovery,
        "complete_reachable_industry_progress"
    );
    assert_eq!(server.world.state(), &state_before);
}

#[test]
fn runtime_product_validation_quote_requires_auth_proof() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let err = server
        .handle_product_validation_quote(crate::viewer::ProductValidationQuoteRequest {
            product_id: "logistics_drone".to_string(),
            amount: 1,
            player_id: "player-product-quote".to_string(),
            public_key: None,
            auth: None,
        })
        .expect_err("unsigned product validation quote must fail");

    assert_eq!(err.code, "auth_proof_required");
    assert_eq!(err.action_id.as_deref(), Some("quote_validate_product"));
}

#[test]
fn runtime_product_validation_quote_rejects_tampered_auth_proof() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm),
    )
    .expect("runtime server");
    let (public_key, private_key) = test_signer(253);
    let mut request = signed_product_validation_quote_request(
        "logistics_drone",
        1,
        "player-product-quote",
        254,
        public_key.as_str(),
        private_key.as_str(),
    );
    request.auth.as_mut().expect("signed auth").signature = "00".repeat(64);

    let err = server
        .handle_product_validation_quote(request)
        .expect_err("tampered product validation quote auth must fail");

    assert_eq!(err.code, "auth_signature_invalid");
    assert_eq!(err.action_id.as_deref(), Some("quote_validate_product"));
}

#[test]
fn runtime_refine_quote_is_signed_deterministic_and_non_mutating() {
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
    let (public_key, private_key) = test_signer(201);
    register_runtime_session(
        &mut server,
        "player-refine",
        Some(agent_id.as_str()),
        201,
        public_key.as_str(),
        private_key.as_str(),
    );
    server.snapshot_config.economy.factory_build_hardware_cost = 5;
    server
        .snapshot_config
        .economy
        .refine_electricity_cost_per_kg = 1;
    server.snapshot_config.economy.refine_hardware_yield_ppm = 1_000;
    server
        .world
        .set_agent_resource_balance(
            agent_id.as_str(),
            crate::simulator::ResourceKind::Electricity,
            50,
        )
        .expect("seed electricity");
    server
        .world
        .set_agent_resource_balance(
            agent_id.as_str(),
            crate::simulator::ResourceKind::Data,
            2_500,
        )
        .expect("seed compound");
    let state_before = server.world.state().clone();

    let request = signed_refine_quote_request(
        2_500,
        "player-refine",
        202,
        public_key.as_str(),
        private_key.as_str(),
    );
    let quote = server
        .handle_refine_quote(request.clone())
        .expect("quote exact advance");
    assert_eq!(quote.owner_agent_id, agent_id);
    assert_eq!(quote.electricity_cost, 3);
    assert_eq!(quote.electricity_after, 47);
    assert_eq!(quote.hardware_output, 2);
    assert_eq!(quote.target_id, "factory_build_hardware");
    assert_eq!(quote.target_gap_before, 5);
    assert_eq!(quote.target_gap_after, 3);
    assert_eq!(
        quote.target_linkage,
        "reduces_factory_build_hardware_shortfall"
    );
    assert_eq!(quote.value_classification, "partial_progress");
    assert_eq!(
        server.handle_refine_quote(request).expect("repeat quote"),
        quote
    );
    assert_eq!(server.world.state(), &state_before);

    server
        .world
        .set_agent_resource_balance(
            agent_id.as_str(),
            crate::simulator::ResourceKind::Data,
            6_002,
        )
        .expect("seed existing progress");
    let enough = server
        .handle_refine_quote(signed_refine_quote_request(
            6_000,
            "player-refine",
            203,
            public_key.as_str(),
            private_key.as_str(),
        ))
        .expect("quote advance");
    assert_eq!(enough.value_classification, "enough_to_advance");
    assert_eq!(enough.target_gap_after, 0);

    server
        .world
        .set_agent_resource_balance(
            agent_id.as_str(),
            crate::simulator::ResourceKind::Data,
            6_005,
        )
        .expect("seed satisfied goal");
    let poor = server
        .handle_refine_quote(signed_refine_quote_request(
            1_000,
            "player-refine",
            204,
            public_key.as_str(),
            private_key.as_str(),
        ))
        .expect("quote poor tradeoff");
    assert_eq!(poor.value_classification, "poor_power_tradeoff");

    server
        .world
        .set_agent_resource_balance(
            agent_id.as_str(),
            crate::simulator::ResourceKind::Electricity,
            0,
        )
        .expect("drain electricity");
    let insufficient = server
        .handle_refine_quote(signed_refine_quote_request(
            1_000,
            "player-refine",
            205,
            public_key.as_str(),
            private_key.as_str(),
        ))
        .expect_err("insufficient electricity rejected");
    assert_eq!(insufficient.code, "refine_quote_rejected");
}

#[test]
fn runtime_refine_quote_does_not_require_or_mutate_gameplay_readiness() {
    let _guard = lock_test_llm_env();
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Script),
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
    let (public_key, private_key) = test_signer(206);
    register_runtime_session(
        &mut server,
        "player-refine-script",
        Some(agent_id.as_str()),
        206,
        public_key.as_str(),
        private_key.as_str(),
    );
    server
        .world
        .set_agent_resource_balance(
            agent_id.as_str(),
            crate::simulator::ResourceKind::Data,
            2_500,
        )
        .expect("seed compound");
    let state_before = server.world.state().clone();
    let feedback_before = server.latest_player_gameplay_feedback.clone();

    let quote = server
        .handle_refine_quote(signed_refine_quote_request(
            2_500,
            "player-refine-script",
            207,
            public_key.as_str(),
            private_key.as_str(),
        ))
        .expect("script-mode quote remains read-only");

    assert_eq!(quote.owner_agent_id, agent_id);
    assert_eq!(server.world.state(), &state_before);
    assert_eq!(server.latest_player_gameplay_feedback, feedback_before);
}

#[test]
fn runtime_collect_data_quotes_and_submits_the_exact_authenticated_request() {
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
    let (public_key, private_key) = test_signer(181);
    register_runtime_session(
        &mut server,
        "player-collect",
        Some(agent_id.as_str()),
        181,
        public_key.as_str(),
        private_key.as_str(),
    );
    server
        .world
        .set_agent_resource_balance(
            agent_id.as_str(),
            crate::simulator::ResourceKind::Electricity,
            20,
        )
        .expect("seed electricity");
    let data_before = server
        .world
        .agent_resource_balance(agent_id.as_str(), crate::simulator::ResourceKind::Data)
        .expect("initial data balance");

    let preflight = signed_collect_data_command(
        crate::viewer::CollectDataCommand::Preflight {
            request: crate::viewer::CollectDataRequest {
                electricity_cost: 7,
                data_amount: 11,
                player_id: "player-collect".to_string(),
                public_key: None,
                auth: None,
            },
        },
        182,
        public_key.as_str(),
        private_key.as_str(),
    );
    let crate::viewer::runtime_live::player_gameplay::CollectDataResult::Preflight(quote) = server
        .handle_collect_data(preflight)
        .expect("authoritative preflight")
    else {
        panic!("expected preflight quote");
    };
    assert_eq!(quote.collector_agent_id, agent_id);
    assert_eq!(quote.data_owner_agent_id, quote.collector_agent_id);
    assert_eq!(quote.data_recipient_agent_id, quote.collector_agent_id);
    assert_eq!(quote.electricity_after, 13);
    assert_eq!(quote.permission_status, "self_owned_no_grant_required");

    let submit = signed_collect_data_command(
        crate::viewer::CollectDataCommand::Submit {
            request: crate::viewer::CollectDataRequest {
                electricity_cost: 7,
                data_amount: 11,
                player_id: "player-collect".to_string(),
                public_key: None,
                auth: None,
            },
        },
        183,
        public_key.as_str(),
        private_key.as_str(),
    );
    let crate::viewer::runtime_live::player_gameplay::CollectDataResult::Submit(ack) = server
        .handle_collect_data(submit)
        .expect("collect data queued")
    else {
        panic!("expected submit acknowledgement");
    };
    assert_eq!(ack.action_id, "collect_data");
    assert_eq!(ack.target_agent_id, agent_id);
    server.world.step().expect("apply collect data");
    assert_eq!(
        server
            .world
            .agent_resource_balance(agent_id.as_str(), crate::simulator::ResourceKind::Data)
            .expect("data balance"),
        data_before + 11
    );
}
