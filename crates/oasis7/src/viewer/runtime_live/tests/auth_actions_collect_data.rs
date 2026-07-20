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
