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

#[test]
fn starter_milestone_survives_latest_recipe_and_restart() {
    let _guard = lock_test_llm_env();
    let (mut server, agent_id, public_key, private_key) =
        setup_runtime_industrial_gameplay_session(81);
    build_first_smelter_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        81,
    );

    let completed_before = server.world.state().industry_progress.completed_recipe_jobs;
    server
        .handle_gameplay_action(signed_gameplay_action_request(
            crate::viewer::GameplayActionRequest {
                action_id: crate::viewer::ACTION_SCHEDULE_SMELTER_IRON_INGOT.to_string(),
                target_agent_id: agent_id.clone(),
                actor_agent_id: None,
                player_id: "player-a".to_string(),
                public_key: None,
                auth: None,
            },
            82,
            public_key.as_str(),
            private_key.as_str(),
        ))
        .expect("queue starter iron settlement");
    for _ in 0..12 {
        server.world.step().expect("settle starter iron settlement");
        if server.world.state().industry_progress.completed_recipe_jobs > completed_before {
            break;
        }
    }
    let milestone = server
        .world
        .state()
        .industry_progress
        .starter_industrial_milestone
        .clone()
        .expect("starter iron settlement must persist its milestone");
    assert_eq!(
        milestone.profile_id,
        crate::runtime::STARTER_INDUSTRIAL_PROFILE_ID
    );
    assert_eq!(milestone.profile_revision, 1);

    let smelter_site_ledger = crate::runtime::MaterialLedgerId::site(
        server
            .world
            .state()
            .factories
            .get(crate::viewer::FACTORY_SMELTER_MK1)
            .expect("starter smelter")
            .site_id
            .as_str(),
    );
    server
        .world
        .set_ledger_material_balance(smelter_site_ledger, "copper_ore", 36)
        .expect("seed later recipe");
    server
        .world
        .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Electricity, i64::MAX)
        .expect("fund later recipe");
    let completed_before = server.world.state().industry_progress.completed_recipe_jobs;
    server
        .handle_gameplay_action(signed_gameplay_action_request(
            crate::viewer::GameplayActionRequest {
                action_id: crate::viewer::ACTION_SCHEDULE_SMELTER_COPPER_WIRE.to_string(),
                target_agent_id: agent_id.clone(),
                actor_agent_id: None,
                player_id: "player-a".to_string(),
                public_key: None,
                auth: None,
            },
            1_000,
            public_key.as_str(),
            private_key.as_str(),
        ))
        .expect("queue follow-up recipe");
    for _ in 0..12 {
        server.world.step().expect("settle follow-up recipe");
        if server.world.state().industry_progress.completed_recipe_jobs > completed_before {
            break;
        }
    }
    assert!(
        server.world.state().industry_progress.completed_recipe_jobs > completed_before,
        "follow-up recipe must settle"
    );

    let state = server.world.state();
    assert_eq!(
        state
            .factories
            .get(crate::viewer::FACTORY_SMELTER_MK1)
            .and_then(|factory| factory.production.last_completed_recipe_id.as_deref()),
        Some("recipe.smelter.copper_wire"),
        "a later recipe must overwrite only the volatile latest-completion projection"
    );
    let restored: crate::runtime::WorldState = serde_json::from_slice(
        &serde_json::to_vec(state).expect("serialize state after later recipe"),
    )
    .expect("restore state after later recipe");
    assert_eq!(
        restored.industry_progress.starter_industrial_milestone,
        Some(milestone),
        "restart must retain the profile-scoped milestone"
    );
    assert!(
        restored
            .starter_industrial_feasibility()
            .candidate_available(),
        "later completion and restart must not relock the assembler candidate"
    );

    let mut restored_world = crate::runtime::World::new_with_state(restored);
    let build_assembler =
        crate::viewer::gameplay_actions::build_runtime_action_from_gameplay_request(
            &crate::viewer::GameplayActionRequest {
                action_id: crate::viewer::ACTION_BUILD_ASSEMBLER_MK1.to_string(),
                target_agent_id: agent_id,
                actor_agent_id: None,
                player_id: "player-a".to_string(),
                public_key: None,
                auth: None,
            },
        )
        .expect("build restored assembler action");
    restored_world.submit_action(build_assembler);
    restored_world
        .step()
        .expect("settle restored assembler build");
    restored_world
        .step()
        .expect("complete restored assembler build");
    assert!(restored_world.has_factory(crate::viewer::FACTORY_ASSEMBLER_MK1));
}
