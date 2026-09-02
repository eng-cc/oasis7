use super::industrial_progression::{
    build_first_assembler_via_gameplay_action, build_first_smelter_via_gameplay_action,
    expect_player_gameplay, setup_industrial_gameplay_with_completed_jobs,
    setup_runtime_industrial_gameplay_session, smelter_schedule_action,
};
use super::*;
use crate::runtime::Action;
use crate::simulator::{PlayerGameplayGoalKind, PlayerGameplayStageStatus, ResourceKind};

fn submit_iron_ingot_schedule(
    server: &mut ViewerRuntimeLiveServer,
    agent_id: &str,
    public_key: &str,
    private_key: &str,
    nonce: u64,
) {
    server
        .handle_gameplay_action(signed_gameplay_action_request(
            crate::viewer::GameplayActionRequest {
                action_id: crate::viewer::ACTION_SCHEDULE_SMELTER_IRON_INGOT.to_string(),
                target_agent_id: agent_id.to_string(),
                actor_agent_id: None,
                player_id: "player-a".to_string(),
                public_key: None,
                auth: None,
            },
            nonce,
            public_key,
            private_key,
        ))
        .expect("queue iron ingot schedule");
    server
        .world
        .step()
        .expect("settle rejected iron ingot schedule");
}

fn seed_assembler_site_materials(server: &mut ViewerRuntimeLiveServer, materials: &[(&str, i64)]) {
    let site_ledger = server
        .world
        .state()
        .factories
        .get(crate::viewer::FACTORY_ASSEMBLER_MK1)
        .expect("assembler factory")
        .site_id
        .clone();
    let site_ledger = crate::runtime::MaterialLedgerId::site(site_ledger);
    for (kind, amount) in materials {
        server
            .world
            .set_ledger_material_balance(site_ledger.clone(), *kind, *amount)
            .expect("seed assembler site material");
    }
}

#[test]
fn runtime_gameplay_smelter_readiness_matches_submit_material_and_owner_power_checks() {
    let _guard = lock_test_llm_env();
    let (mut server, agent_id, public_key, private_key) =
        setup_runtime_industrial_gameplay_session(43);
    build_first_smelter_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        43,
    );
    server
        .world
        .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Electricity, i64::MAX)
        .expect("fund agent electricity");
    server
        .world
        .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Data, i64::MAX)
        .expect("fund agent data");

    let input_ledger = server
        .world
        .state()
        .factories
        .get("factory.smelter.mk1")
        .expect("smelter factory")
        .input_ledger
        .clone();
    server
        .world
        .set_ledger_material_balance(input_ledger, "iron_ore", 0)
        .expect("drain smelter iron ore");
    server
        .world
        .set_material_balance("iron_ore", 0)
        .expect("drain world iron ore");

    let gameplay = expect_player_gameplay(&mut server, "material readiness snapshot");
    let material_reason =
        smelter_schedule_action(&gameplay, crate::viewer::ACTION_SCHEDULE_SMELTER_IRON_INGOT)
            .disabled_reason
            .as_deref()
            .expect("missing material should disable iron ingot scheduling");
    assert!(material_reason.contains("insufficient iron_ore"));
    assert!(material_reason.contains("replenish iron_ore"));

    submit_iron_ingot_schedule(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        44,
    );
    assert_eq!(
        server
            .world
            .state()
            .factories
            .get("factory.smelter.mk1")
            .and_then(|factory| factory.production.current_blocker_kind.as_deref()),
        Some("material_shortage"),
        "submit must reject the material shortage surfaced by readiness"
    );

    let input_ledger = server
        .world
        .state()
        .factories
        .get("factory.smelter.mk1")
        .expect("smelter factory")
        .input_ledger
        .clone();
    server
        .world
        .set_ledger_material_balance(input_ledger, "iron_ore", 400)
        .expect("restore smelter iron ore");
    server
        .world
        .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Electricity, 0)
        .expect("drain factory-owner electricity");

    let gameplay = expect_player_gameplay(&mut server, "owner power readiness snapshot");
    let power_reason =
        smelter_schedule_action(&gameplay, crate::viewer::ACTION_SCHEDULE_SMELTER_IRON_INGOT)
            .disabled_reason
            .as_deref()
            .expect("missing owner power should disable iron ingot scheduling");
    assert!(power_reason.contains("insufficient factory-owner electricity"));
    assert!(power_reason.contains("replenish electricity"));

    submit_iron_ingot_schedule(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        45,
    );
    assert_eq!(
        server
            .world
            .state()
            .factories
            .get("factory.smelter.mk1")
            .and_then(|factory| factory.production.current_blocker_kind.as_deref()),
        Some("power_shortage"),
        "submit must reject the owner power shortage surfaced by readiness"
    );
}

#[test]
fn runtime_gameplay_actions_keep_retired_starter_factory_rebuild_disabled() {
    let _guard = lock_test_llm_env();
    let mut server = setup_industrial_gameplay_with_completed_jobs(73, 6);
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");

    server.world.submit_action(Action::RecycleFactory {
        operator_agent_id: agent_id,
        factory_id: "factory.smelter.mk1".to_string(),
    });
    server.world.step().expect("settle starter smelter recycle");
    assert!(
        server
            .world
            .state()
            .retired_factory_ids
            .contains("factory.smelter.mk1")
    );

    let gameplay = expect_player_gameplay(&mut server, "retired factory rebuild readiness");
    let action = gameplay
        .available_actions
        .iter()
        .find(|action| action.action_id == "build_factory_smelter_mk1")
        .expect("retired smelter rebuild action");
    let disabled_reason = action
        .disabled_reason
        .as_deref()
        .expect("retired factory identity must disable rebuild");
    assert!(
        disabled_reason.contains("retired"),
        "reason={disabled_reason}"
    );
    assert!(
        disabled_reason.contains("factory.smelter.mk1"),
        "reason={disabled_reason}"
    );
}

#[test]
fn runtime_gameplay_allows_recipe_when_current_site_authority_revision_changes() {
    let _guard = lock_test_llm_env();
    let (mut server, agent_id, public_key, private_key) =
        setup_runtime_industrial_gameplay_session(74);
    build_first_smelter_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        74,
    );
    server
        .world
        .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Electricity, i64::MAX)
        .expect("fund factory owner electricity");

    let factory = server
        .world
        .state()
        .factories
        .get("factory.smelter.mk1")
        .expect("smelter factory");
    let site_id = factory.site_id.clone();
    let construction_revision = factory
        .site_authority_revision
        .expect("construction site authority revision");
    let mut site_authority = server
        .world
        .state()
        .factory_site_authorities
        .get(site_id.as_str())
        .expect("smelter site authority")
        .clone();
    site_authority.authority_revision = site_authority.authority_revision.saturating_add(1);
    assert_ne!(site_authority.authority_revision, construction_revision);
    server
        .world
        .set_factory_site_authority(site_authority)
        .expect("advance current site authority revision");
    assert_eq!(
        server
            .world
            .state()
            .factories
            .get("factory.smelter.mk1")
            .and_then(|factory| factory.site_authority_revision),
        Some(construction_revision),
        "construction-time revision should remain historical"
    );

    let gameplay = expect_player_gameplay(&mut server, "current site revision recipe readiness");
    let action =
        smelter_schedule_action(&gameplay, crate::viewer::ACTION_SCHEDULE_SMELTER_IRON_INGOT);
    assert_eq!(
        action.disabled_reason, None,
        "current authority should govern recipe readiness even when its revision advanced"
    );
}

#[test]
fn runtime_gameplay_smelter_readiness_does_not_fallback_to_global_material_ledger() {
    let _guard = lock_test_llm_env();
    let (mut server, agent_id, public_key, private_key) =
        setup_runtime_industrial_gameplay_session(46);
    build_first_smelter_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        46,
    );
    server
        .world
        .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Electricity, i64::MAX)
        .expect("fund agent electricity");
    server
        .world
        .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Data, i64::MAX)
        .expect("fund agent data");

    let factory = server
        .world
        .state()
        .factories
        .get("factory.smelter.mk1")
        .expect("smelter factory");
    let input_ledger = factory.input_ledger.clone();
    let site_id = factory.site_id.clone();
    for kind in ["iron_ore", "carbon_fuel"] {
        server
            .world
            .set_ledger_material_balance(input_ledger.clone(), kind, 0)
            .expect("drain factory input ledger");
        server
            .world
            .set_material_balance(kind, 200)
            .expect("seed global fallback material");
    }

    let gameplay = expect_player_gameplay(&mut server, "site material readiness snapshot");
    let action =
        smelter_schedule_action(&gameplay, crate::viewer::ACTION_SCHEDULE_SMELTER_IRON_INGOT);
    let disabled_reason = action
        .disabled_reason
        .as_deref()
        .expect("missing site inputs must disable scheduling even when world has materials");
    assert!(disabled_reason.contains("insufficient iron_ore"));
    assert!(disabled_reason.contains(format!("site:{site_id}").as_str()));
    assert!(disabled_reason.contains("replenish iron_ore"));
}

#[test]
fn runtime_gameplay_action_unlocks_first_expansion_tradeoff_after_scale_out() {
    let _guard = lock_test_llm_env();
    let mut server = setup_industrial_gameplay_with_completed_jobs(41, 4);
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .expect("scale-out agent")
        .clone();
    server
        .world
        .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Electricity, i64::MAX)
        .expect("fund scale-out agent electricity");
    server
        .world
        .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Data, i64::MAX)
        .expect("fund scale-out agent data");
    let smelter_site_ledger = crate::runtime::MaterialLedgerId::site(
        server
            .world
            .state()
            .factories
            .get(crate::viewer::FACTORY_SMELTER_MK1)
            .expect("scale-out smelter")
            .site_id
            .as_str(),
    );
    server
        .world
        .set_ledger_material_balance(smelter_site_ledger.clone(), "iron_ingot", 200)
        .expect("seed scale-out alloy iron ingot");
    server
        .world
        .set_ledger_material_balance(smelter_site_ledger, "copper_wire", 200)
        .expect("seed scale-out alloy copper wire");
    let builder_ledger = crate::runtime::MaterialLedgerId::agent(agent_id.as_str());
    for (kind, amount) in [
        ("structural_frame", 8),
        ("iron_ingot", 10),
        ("copper_wire", 8),
    ] {
        server
            .world
            .set_ledger_material_balance(builder_ledger.clone(), kind, amount)
            .expect("seed scale-out assembler construction material");
    }
    server
        .world
        .set_resource_balance(ResourceKind::Electricity, 2_000);
    let gameplay = expect_player_gameplay(&mut server, "player gameplay after scale-out");
    assert_eq!(
        gameplay.goal_id,
        "post_onboarding.choose_first_expansion_tradeoff"
    );
    assert_eq!(
        gameplay.goal_kind,
        PlayerGameplayGoalKind::ChooseFirstExpansionTradeoff
    );
    assert_eq!(
        gameplay.stage_status,
        PlayerGameplayStageStatus::BranchReady
    );
    assert_eq!(gameplay.progress_percent, 92);
    assert!(
        gameplay
            .branch_hint
            .as_deref()
            .is_some_and(|hint| hint.contains("throughput expansion"))
    );
    assert!(
        gameplay
            .available_actions
            .iter()
            .any(
                |action| action.action_id == "schedule_recipe_smelter_alloy_plate"
                    && action.disabled_reason.is_none()
            )
    );
    assert!(
        gameplay
            .available_actions
            .iter()
            .any(|action| action.action_id == "build_factory_assembler_mk1")
    );
    assert!((1..=3).contains(&gameplay.branch_recommendations.len()));
    assert_eq!(
        gameplay
            .branch_recommendations
            .iter()
            .map(|recommendation| recommendation.action_id.as_str())
            .collect::<Vec<_>>(),
        vec![
            "schedule_recipe_smelter_alloy_plate",
            "build_factory_assembler_mk1",
        ]
    );
    for recommendation in &gameplay.branch_recommendations {
        assert!(!recommendation.route_label.trim().is_empty());
        assert!(!recommendation.immediate_gain.trim().is_empty());
        assert!(!recommendation.future_beat_changed.trim().is_empty());
        assert_eq!(
            recommendation.future_beats.len(),
            2,
            "each actionable branch recommendation must expose exactly two future beats"
        );
        assert!(
            recommendation
                .future_beats
                .iter()
                .all(|beat| !beat.trim().is_empty()),
            "future beats must be player-readable"
        );
        assert_ne!(
            recommendation.future_beats[0].trim(),
            recommendation.future_beats[1].trim(),
            "future beats must describe substantively distinct changes"
        );
        assert!(!recommendation.risk_or_lockin.trim().is_empty());
        assert!(!recommendation.next_session_hook.trim().is_empty());
        assert!(
            gameplay
                .available_actions
                .iter()
                .any(|action| action.action_id == recommendation.action_id
                    && action.disabled_reason.is_none())
        );
    }
    assert_eq!(
        gameplay.small_player_lane_id.as_deref(),
        Some("local_operator")
    );
    assert_eq!(
        gameplay.leverage_class.as_deref(),
        Some("regional_specialization_option")
    );
    assert_eq!(gameplay.same_loop_repeat_count, 4);
    assert!(!gameplay.grind_only_flag);
    assert_eq!(
        gameplay.major_power_dependency_status.as_deref(),
        Some("independent_path_available")
    );
    assert_eq!(
        gameplay.recovery_path_kind.as_deref(),
        Some("repair_rebuild_or_pivot")
    );
    assert!(
        gameplay
            .recovery_path_detail
            .as_deref()
            .is_some_and(|detail| detail.contains("repair"))
    );
}

#[test]
fn runtime_gameplay_actions_allow_assembler_build_from_authoritative_builder_ledger() {
    let _guard = lock_test_llm_env();
    let (mut server, agent_id, public_key, private_key) =
        setup_runtime_industrial_gameplay_session(35);
    build_first_smelter_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        35,
    );
    let agent_ledger = crate::runtime::MaterialLedgerId::agent(agent_id.as_str());
    server
        .world
        .set_ledger_material_balance(agent_ledger.clone(), "iron_ingot", 10)
        .expect("seed agent iron ingot");
    server
        .world
        .set_ledger_material_balance(agent_ledger.clone(), "copper_wire", 8)
        .expect("seed agent copper wire");
    server
        .world
        .set_ledger_material_balance(agent_ledger, "structural_frame", 8)
        .expect("seed agent structural frame");

    let gameplay = expect_player_gameplay(
        &mut server,
        "player gameplay after seeding assembler build materials on agent ledger",
    );
    let assembler_action = gameplay
        .available_actions
        .iter()
        .find(|action| action.action_id == "build_factory_assembler_mk1")
        .expect("assembler build action");
    assert_eq!(assembler_action.disabled_reason, None);
}

#[test]
fn runtime_gameplay_actions_do_not_enable_assembler_build_from_world_ledger() {
    let _guard = lock_test_llm_env();
    let (mut server, agent_id, public_key, private_key) =
        setup_runtime_industrial_gameplay_session(37);
    build_first_smelter_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        37,
    );
    let builder_ledger = crate::runtime::MaterialLedgerId::agent(agent_id.as_str());
    let world_ledger = crate::runtime::MaterialLedgerId::world();
    for (kind, amount) in [
        ("structural_frame", 8),
        ("iron_ingot", 10),
        ("copper_wire", 8),
    ] {
        server
            .world
            .set_ledger_material_balance(builder_ledger.clone(), kind, 0)
            .expect("drain authoritative builder ledger");
        server
            .world
            .set_ledger_material_balance(world_ledger.clone(), kind, amount)
            .expect("seed world compatibility ledger");
    }

    let gameplay = expect_player_gameplay(
        &mut server,
        "player gameplay with construction materials only on world ledger",
    );
    let assembler_action = gameplay
        .available_actions
        .iter()
        .find(|action| action.action_id == "build_factory_assembler_mk1")
        .expect("assembler build action");
    let disabled_reason = assembler_action
        .disabled_reason
        .as_deref()
        .expect("world ledger must not enable builder construction");
    assert!(disabled_reason.contains("insufficient builder material"));
    assert!(disabled_reason.contains(builder_ledger.to_string().as_str()));
    assert!(disabled_reason.contains("replenish"));
}

#[test]
fn runtime_gameplay_actions_disable_factory_build_when_builder_power_is_insufficient() {
    let _guard = lock_test_llm_env();
    let (mut server, agent_id, _, _) = setup_runtime_industrial_gameplay_session(38);
    server
        .world
        .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Electricity, 0)
        .expect("drain builder construction power");

    let gameplay = expect_player_gameplay(&mut server, "builder power readiness snapshot");
    let action = gameplay
        .available_actions
        .iter()
        .find(|action| action.action_id == "build_factory_smelter_mk1")
        .expect("smelter build action");
    let disabled_reason = action
        .disabled_reason
        .as_deref()
        .expect("missing builder power must disable construction");
    assert!(disabled_reason.contains("insufficient builder electricity"));
    assert!(disabled_reason.contains("replenish electricity"));
}

#[test]
fn runtime_gameplay_actions_disable_factory_build_when_site_authority_is_inactive() {
    let _guard = lock_test_llm_env();
    let (mut server, agent_id, _, _) = setup_runtime_industrial_gameplay_session(39);
    let mut site_authority = server
        .world
        .state()
        .factory_site_authorities
        .get("site-smelter")
        .expect("smelter site authority")
        .clone();
    site_authority.authority_revision = site_authority.authority_revision.saturating_add(1);
    site_authority.active = false;
    server
        .world
        .set_factory_site_authority(site_authority)
        .expect("deactivate smelter site authority");

    let gameplay = expect_player_gameplay(&mut server, "inactive site readiness snapshot");
    let action = gameplay
        .available_actions
        .iter()
        .find(|action| action.action_id == "build_factory_smelter_mk1")
        .expect("smelter build action");
    let disabled_reason = action
        .disabled_reason
        .as_deref()
        .expect("inactive site authority must disable construction");
    assert!(disabled_reason.contains("site authority inactive_or_stale"));
    assert!(disabled_reason.contains("chunk_ready=true"));
    assert!(
        disabled_reason.contains(agent_id.as_str()) || disabled_reason.contains("site-smelter")
    );
}

#[test]
fn runtime_gameplay_actions_keep_assembler_build_disabled_when_cost_is_split_across_ledgers() {
    let _guard = lock_test_llm_env();
    let (mut server, agent_id, public_key, private_key) =
        setup_runtime_industrial_gameplay_session(36);
    build_first_smelter_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        36,
    );
    let agent_ledger = crate::runtime::MaterialLedgerId::agent(agent_id.as_str());
    server
        .world
        .set_ledger_material_balance(agent_ledger.clone(), "iron_ingot", 10)
        .expect("seed agent iron ingot");
    server
        .world
        .set_ledger_material_balance(agent_ledger, "copper_wire", 8)
        .expect("seed agent copper wire");

    let gameplay = expect_player_gameplay(
        &mut server,
        "player gameplay with split assembler build materials across ledgers",
    );
    let assembler_action = gameplay
        .available_actions
        .iter()
        .find(|action| action.action_id == "build_factory_assembler_mk1")
        .expect("assembler build action");
    let disabled_reason = assembler_action
        .disabled_reason
        .as_deref()
        .expect("split ledger cost should keep assembler action disabled");
    assert!(disabled_reason.contains("insufficient builder material"));
    assert!(disabled_reason.contains("structural_frame"));
    assert!(disabled_reason.contains("replenish"));
}

#[test]
fn runtime_gameplay_actions_do_not_enable_assembler_schedule_from_world_ledger() {
    let _guard = lock_test_llm_env();
    let (mut server, agent_id, public_key, private_key) =
        setup_runtime_industrial_gameplay_session(40);
    build_first_smelter_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        40,
    );
    build_first_assembler_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        50,
    );
    let assembler_site = server
        .world
        .state()
        .factories
        .get("factory.assembler.mk1")
        .expect("assembler factory state")
        .site_id
        .clone();
    let site_ledger = crate::runtime::MaterialLedgerId::site(assembler_site);
    let world_ledger = crate::runtime::MaterialLedgerId::world();
    server
        .world
        .set_ledger_material_balance(site_ledger.clone(), "iron_ingot", 0)
        .expect("drain authoritative assembler site ledger");
    server
        .world
        .set_ledger_material_balance(world_ledger, "iron_ingot", 8)
        .expect("seed world compatibility ledger");

    let gameplay = expect_player_gameplay(
        &mut server,
        "player gameplay with assembler inputs only on world ledger",
    );
    let gear_action = gameplay
        .available_actions
        .iter()
        .find(|action| action.action_id == "schedule_recipe_assembler_gear")
        .expect("assembler gear schedule action");
    let disabled_reason = gear_action
        .disabled_reason
        .as_deref()
        .expect("world ledger must not enable assembler scheduling");
    assert!(disabled_reason.contains("insufficient iron_ingot"));
    assert!(disabled_reason.contains(site_ledger.to_string().as_str()));
    assert!(disabled_reason.contains("replenish"));
}

#[test]
fn runtime_gameplay_actions_enable_assembler_motor_at_runtime_power_boundary() {
    let _guard = lock_test_llm_env();
    let (mut server, agent_id, public_key, private_key) =
        setup_runtime_industrial_gameplay_session(52);
    build_first_smelter_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        52,
    );
    build_first_assembler_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        62,
    );
    seed_assembler_site_materials(
        &mut server,
        &[("gear", 4), ("copper_wire", 6), ("hardware_part", 1)],
    );

    server
        .world
        .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Electricity, 14)
        .expect("seed exact motor runtime power");
    let gameplay = expect_player_gameplay(&mut server, "motor runtime power boundary");
    let action = smelter_schedule_action(
        &gameplay,
        crate::viewer::ACTION_SCHEDULE_ASSEMBLER_MOTOR_MK1,
    );
    assert_eq!(
        action.disabled_reason, None,
        "runtime-valid motor power must keep the published action enabled"
    );

    server
        .world
        .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Electricity, 13)
        .expect("drain one unit below motor runtime power");
    let gameplay = expect_player_gameplay(&mut server, "motor below power boundary");
    let action = smelter_schedule_action(
        &gameplay,
        crate::viewer::ACTION_SCHEDULE_ASSEMBLER_MOTOR_MK1,
    );
    let disabled_reason = action
        .disabled_reason
        .as_deref()
        .expect("one below runtime motor power must disable scheduling");
    assert!(disabled_reason.contains("need 14"));
    assert!(disabled_reason.contains("replenish electricity"));
}

#[test]
fn runtime_gameplay_actions_enable_assembler_drone_at_runtime_power_boundary() {
    let _guard = lock_test_llm_env();
    let (mut server, agent_id, public_key, private_key) =
        setup_runtime_industrial_gameplay_session(53);
    build_first_smelter_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        53,
    );
    build_first_assembler_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        63,
    );
    seed_assembler_site_materials(
        &mut server,
        &[
            ("motor_mk1", 2),
            ("control_chip", 1),
            ("iron_ingot", 2),
            ("hardware_part", 2),
        ],
    );

    server
        .world
        .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Electricity, 12)
        .expect("seed exact drone runtime power");
    let gameplay = expect_player_gameplay(&mut server, "drone runtime power boundary");
    let action = smelter_schedule_action(
        &gameplay,
        crate::viewer::ACTION_SCHEDULE_ASSEMBLER_LOGISTICS_DRONE,
    );
    assert_eq!(
        action.disabled_reason, None,
        "runtime-valid drone power must keep the published action enabled"
    );

    server
        .world
        .set_agent_resource_balance(agent_id.as_str(), ResourceKind::Electricity, 11)
        .expect("drain one unit below drone runtime power");
    let gameplay = expect_player_gameplay(&mut server, "drone below power boundary");
    let action = smelter_schedule_action(
        &gameplay,
        crate::viewer::ACTION_SCHEDULE_ASSEMBLER_LOGISTICS_DRONE,
    );
    let disabled_reason = action
        .disabled_reason
        .as_deref()
        .expect("one below runtime drone power must disable scheduling");
    assert!(disabled_reason.contains("need 12"));
    assert!(disabled_reason.contains("replenish electricity"));
}
