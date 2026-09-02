use super::*;

fn setup_before_smelter_settlement(seed: u8) -> (ViewerRuntimeLiveServer, String, String, String) {
    let (mut server, agent_id, public_key, private_key) =
        setup_runtime_industrial_gameplay_session(seed);
    build_first_smelter_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        u64::from(seed),
    );
    let builder_ledger = crate::runtime::MaterialLedgerId::agent(agent_id.as_str());
    for (kind, amount) in [
        ("structural_frame", 8),
        ("iron_ingot", 10),
        ("copper_wire", 8),
    ] {
        server
            .world
            .set_ledger_material_balance(builder_ledger.clone(), kind, amount)
            .expect("seed assembler build material");
    }
    (server, agent_id, public_key, private_key)
}

#[test]
fn viewer_disables_assembler_before_starter_smelter_settlement() {
    let _guard = lock_test_llm_env();
    let (mut server, agent_id, _, _) = setup_before_smelter_settlement(79);
    let gameplay = expect_player_gameplay(&mut server, "assembler before smelter settlement");
    let action = gameplay
        .available_actions
        .iter()
        .find(|action| action.action_id == crate::viewer::ACTION_BUILD_ASSEMBLER_MK1)
        .expect("assembler build action");
    let reason = action
        .disabled_reason
        .as_deref()
        .expect("assembler must remain disabled");
    assert!(reason.contains("starter Smelter production"));
    assert!(reason.contains("recipe.smelter.iron_ingot"));
    assert_eq!(action.target_agent_id.as_deref(), Some(agent_id.as_str()));
}

#[test]
fn runtime_rejects_assembler_atomically_before_starter_smelter_settlement() {
    let _guard = lock_test_llm_env();
    let (mut server, agent_id, public_key, private_key) = setup_before_smelter_settlement(80);
    let builder_ledger = crate::runtime::MaterialLedgerId::agent(agent_id.as_str());
    let power_before = server
        .world
        .agent_resource_balance(agent_id.as_str(), ResourceKind::Electricity)
        .expect("builder power before rejection");

    server
        .handle_gameplay_action(signed_gameplay_action_request(
            crate::viewer::GameplayActionRequest {
                action_id: crate::viewer::ACTION_BUILD_ASSEMBLER_MK1.to_string(),
                target_agent_id: agent_id.clone(),
                actor_agent_id: None,
                player_id: "player-a".to_string(),
                public_key: None,
                auth: None,
            },
            81,
            public_key.as_str(),
            private_key.as_str(),
        ))
        .expect("queue assembler action");
    server
        .world
        .step()
        .expect("settle rejected assembler action");

    assert!(
        !server
            .world
            .has_factory(crate::viewer::FACTORY_ASSEMBLER_MK1)
    );
    assert_eq!(server.world.pending_factory_builds_len(), 0);
    assert_eq!(
        server
            .world
            .agent_resource_balance(agent_id.as_str(), ResourceKind::Electricity)
            .expect("builder power after rejection"),
        power_before
    );
    for (kind, amount) in [
        ("structural_frame", 8),
        ("iron_ingot", 10),
        ("copper_wire", 8),
    ] {
        assert_eq!(
            server.world.ledger_material_balance(&builder_ledger, kind),
            amount,
            "rejected build consumed {kind}"
        );
    }
}
