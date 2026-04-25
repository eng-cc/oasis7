use super::*;
use crate::runtime::{Action, IndustryStage};
use crate::simulator::{PlayerGameplayGoalKind, PlayerGameplayStageStatus};

fn setup_runtime_industrial_gameplay_session(
    signer_seed: u8,
) -> (ViewerRuntimeLiveServer, String, String, String) {
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
    let (public_key, private_key) = test_signer(signer_seed);
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        u64::from(signer_seed.saturating_sub(1)),
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );
    (server, agent_id, public_key, private_key)
}

fn build_first_smelter_via_gameplay_action(
    server: &mut ViewerRuntimeLiveServer,
    agent_id: &str,
    public_key: &str,
    private_key: &str,
    nonce: u64,
) {
    let build_request = signed_gameplay_action_request(
        crate::viewer::GameplayActionRequest {
            action_id: "build_factory_smelter_mk1".to_string(),
            target_agent_id: agent_id.to_string(),
            actor_agent_id: None,
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
        },
        nonce,
        public_key,
        private_key,
    );
    let build_ack = server
        .handle_gameplay_action(build_request)
        .expect("queue smelter build");
    assert_eq!(build_ack.action_id, "build_factory_smelter_mk1");
    for _ in 0..2 {
        server.world.step().expect("settle smelter build");
    }
    assert!(server.world.has_factory("factory.smelter.mk1"));
    let factory = server
        .world
        .state()
        .factories
        .get("factory.smelter.mk1")
        .expect("smelter factory state");
    let site_ledger = crate::runtime::MaterialLedgerId::site(factory.site_id.as_str());
    server
        .world
        .set_ledger_material_balance(site_ledger.clone(), "iron_ore", 400)
        .expect("seed iron ore for repeated recipes");
    server
        .world
        .set_ledger_material_balance(site_ledger, "carbon_fuel", 120)
        .expect("seed carbon fuel for repeated recipes");
    server
        .world
        .set_material_balance("hardware_part", 200)
        .expect("seed maintenance parts for repeated recipes");
    server
        .world
        .set_resource_balance(crate::simulator::ResourceKind::Electricity, 2_000);
}

fn build_first_assembler_via_gameplay_action(
    server: &mut ViewerRuntimeLiveServer,
    agent_id: &str,
    public_key: &str,
    private_key: &str,
    nonce: u64,
) {
    let agent_ledger = crate::runtime::MaterialLedgerId::agent(agent_id);
    server
        .world
        .set_ledger_material_balance(agent_ledger.clone(), "structural_frame", 8)
        .expect("seed agent structural frame for assembler");
    server
        .world
        .set_ledger_material_balance(agent_ledger.clone(), "iron_ingot", 10)
        .expect("seed agent iron ingot for assembler");
    server
        .world
        .set_ledger_material_balance(agent_ledger, "copper_wire", 8)
        .expect("seed agent copper wire for assembler");

    let build_request = signed_gameplay_action_request(
        crate::viewer::GameplayActionRequest {
            action_id: "build_factory_assembler_mk1".to_string(),
            target_agent_id: agent_id.to_string(),
            actor_agent_id: None,
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
        },
        nonce,
        public_key,
        private_key,
    );
    let build_ack = server
        .handle_gameplay_action(build_request)
        .expect("queue assembler build");
    assert_eq!(build_ack.action_id, "build_factory_assembler_mk1");
    for _ in 0..2 {
        server.world.step().expect("settle assembler build");
    }
    assert!(server.world.has_factory("factory.assembler.mk1"));
}

fn block_secondary_assembler_via_missing_inputs(
    server: &mut ViewerRuntimeLiveServer,
    agent_id: &str,
    public_key: &str,
    private_key: &str,
    nonce: u64,
) {
    let recipe_request = signed_gameplay_action_request(
        crate::viewer::GameplayActionRequest {
            action_id: "schedule_recipe_assembler_control_chip".to_string(),
            target_agent_id: agent_id.to_string(),
            actor_agent_id: None,
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
        },
        nonce,
        public_key,
        private_key,
    );
    let recipe_ack = server
        .handle_gameplay_action(recipe_request)
        .expect("queue blocked assembler recipe");
    assert_eq!(
        recipe_ack.action_id,
        "schedule_recipe_assembler_control_chip"
    );
    for _ in 0..4 {
        server
            .world
            .step()
            .expect("settle blocked assembler recipe");
        if server
            .world
            .state()
            .factories
            .get("factory.assembler.mk1")
            .and_then(|factory| factory.production.current_blocker_kind.as_ref())
            .is_some()
        {
            break;
        }
    }
    assert!(
        server
            .world
            .state()
            .factories
            .get("factory.assembler.mk1")
            .and_then(|factory| factory.production.current_blocker_kind.as_ref())
            .is_some(),
        "expected assembler blocker after missing-input recipe attempt"
    );
}

fn complete_smelter_iron_ingot_jobs(
    server: &mut ViewerRuntimeLiveServer,
    agent_id: &str,
    public_key: &str,
    private_key: &str,
    start_nonce: u64,
    jobs: u64,
) {
    for offset in 0..jobs {
        let recipe_request = signed_gameplay_action_request(
            crate::viewer::GameplayActionRequest {
                action_id: "schedule_recipe_smelter_iron_ingot".to_string(),
                target_agent_id: agent_id.to_string(),
                actor_agent_id: None,
                player_id: "player-a".to_string(),
                public_key: None,
                auth: None,
            },
            start_nonce + offset,
            public_key,
            private_key,
        );
        let recipe_ack = server
            .handle_gameplay_action(recipe_request)
            .expect("queue iron ingot recipe");
        assert_eq!(recipe_ack.action_id, "schedule_recipe_smelter_iron_ingot");

        let completed_before = server.world.state().industry_progress.completed_recipe_jobs;
        for _ in 0..12 {
            server.world.step().expect("settle recipe");
            if server.world.state().industry_progress.completed_recipe_jobs > completed_before {
                break;
            }
        }
        assert!(
            server.world.state().industry_progress.completed_recipe_jobs > completed_before,
            "expected one more completed recipe job"
        );
    }
}

fn setup_industrial_gameplay_with_completed_jobs(
    signer_seed: u8,
    jobs: u64,
) -> ViewerRuntimeLiveServer {
    let (mut server, agent_id, public_key, private_key) =
        setup_runtime_industrial_gameplay_session(signer_seed);
    let build_nonce = u64::from(signer_seed);
    build_first_smelter_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        build_nonce,
    );
    complete_smelter_iron_ingot_jobs(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        build_nonce + 1,
        jobs,
    );
    server
}

fn expect_player_gameplay(
    server: &mut ViewerRuntimeLiveServer,
    context: &'static str,
) -> crate::simulator::PlayerGameplaySnapshot {
    server.compat_snapshot().player_gameplay.expect(context)
}

#[test]
fn runtime_gameplay_action_promotes_first_output_into_resilient_production_goal() {
    let _guard = lock_test_llm_env();
    let mut server = setup_industrial_gameplay_with_completed_jobs(31, 1);
    let gameplay = expect_player_gameplay(&mut server, "player gameplay after industrial progress");
    assert_eq!(
        gameplay.goal_id,
        "post_onboarding.stabilize_first_line_after_output"
    );
    assert_eq!(
        gameplay.goal_title,
        "Harden your first output into resilient production"
    );
    assert_eq!(gameplay.progress_percent, 80);
    assert_eq!(gameplay.stage_status, PlayerGameplayStageStatus::Active);
}

#[test]
fn runtime_gameplay_actions_allow_assembler_build_from_agent_ledger_fallback() {
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
    assert!(disabled_reason.contains("requires one ledger with"));
    assert!(disabled_reason.contains("structural_frame>=8"));
}

#[test]
fn runtime_gameplay_action_unlocks_first_expansion_tradeoff_after_scale_out() {
    let _guard = lock_test_llm_env();
    let mut server = setup_industrial_gameplay_with_completed_jobs(41, 3);
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
    assert!(gameplay
        .branch_hint
        .as_deref()
        .is_some_and(|hint| hint.contains("throughput expansion")));
    assert!(gameplay
        .available_actions
        .iter()
        .any(
            |action| action.action_id == "schedule_recipe_smelter_alloy_plate"
                && action.disabled_reason.is_none()
        ));
    assert!(gameplay
        .available_actions
        .iter()
        .any(|action| action.action_id == "build_factory_assembler_mk1"));
}

#[test]
fn runtime_gameplay_action_promotes_to_generic_midloop_after_governance_ready() {
    let _guard = lock_test_llm_env();
    let mut server = setup_industrial_gameplay_with_completed_jobs(51, 6);
    let gameplay =
        expect_player_gameplay(&mut server, "player gameplay after governance-ready output");
    assert_eq!(gameplay.goal_id, "post_onboarding.choose_midloop_path");
    assert_eq!(
        gameplay.goal_kind,
        PlayerGameplayGoalKind::ChooseMidLoopPath
    );
    assert_eq!(gameplay.progress_percent, 100);
    assert!(gameplay
        .available_actions
        .iter()
        .any(
            |action| action.action_id == "schedule_recipe_smelter_alloy_plate"
                && action.disabled_reason.is_none()
        ));
}

#[test]
fn runtime_gameplay_actions_expose_scale_out_and_governance_recipes_once_assembler_exists() {
    let _guard = lock_test_llm_env();
    let (mut server, agent_id, public_key, private_key) =
        setup_runtime_industrial_gameplay_session(52);
    let build_nonce = 52_u64;
    build_first_smelter_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        build_nonce,
    );
    complete_smelter_iron_ingot_jobs(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        build_nonce + 1,
        6,
    );
    build_first_assembler_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        build_nonce + 10,
    );

    let gameplay = expect_player_gameplay(
        &mut server,
        "player gameplay after assembler build in governance stage",
    );
    assert!(gameplay
        .available_actions
        .iter()
        .any(
            |action| action.action_id == "schedule_recipe_assembler_sensor_pack"
                && action.disabled_reason.is_none()
        ));
    assert!(gameplay
        .available_actions
        .iter()
        .any(
            |action| action.action_id == "schedule_recipe_assembler_module_rack"
                && action.disabled_reason.is_none()
        ));
    assert!(gameplay
        .available_actions
        .iter()
        .any(
            |action| action.action_id == "schedule_recipe_assembler_factory_core"
                && action.disabled_reason.is_none()
        ));
}

#[test]
fn runtime_gameplay_action_drops_midloop_ready_after_last_capability_is_recycled() {
    let _guard = lock_test_llm_env();
    let mut server = setup_industrial_gameplay_with_completed_jobs(61, 6);
    let agent_id = server
        .world
        .state()
        .agents
        .keys()
        .next()
        .cloned()
        .expect("seed agent");
    let gameplay = expect_player_gameplay(
        &mut server,
        "player gameplay before recycling the only output capability",
    );
    assert_eq!(gameplay.goal_id, "post_onboarding.choose_midloop_path");
    assert_eq!(
        server.world.state().industry_progress.stage,
        IndustryStage::Governance
    );

    server.world.submit_action(Action::RecycleFactory {
        operator_agent_id: agent_id,
        factory_id: "factory.smelter.mk1".to_string(),
    });
    server.world.step().expect("recycle smelter capability");

    assert!(!server.world.has_factory("factory.smelter.mk1"));
    assert_eq!(
        server.world.state().industry_progress.stage,
        IndustryStage::Bootstrap
    );

    let gameplay = expect_player_gameplay(
        &mut server,
        "player gameplay after recycling the only output capability",
    );
    assert_ne!(gameplay.goal_id, "post_onboarding.choose_midloop_path");
    assert!(gameplay.progress_percent < 100);
    assert_ne!(
        gameplay.stage_status,
        PlayerGameplayStageStatus::BranchReady
    );
}

#[test]
fn runtime_gameplay_action_keeps_primary_goal_when_secondary_factory_blocks() {
    let _guard = lock_test_llm_env();
    let (mut server, agent_id, public_key, private_key) =
        setup_runtime_industrial_gameplay_session(71);
    let build_nonce = 71_u64;
    build_first_smelter_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        build_nonce,
    );
    complete_smelter_iron_ingot_jobs(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        build_nonce + 1,
        3,
    );
    build_first_assembler_via_gameplay_action(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        build_nonce + 10,
    );
    block_secondary_assembler_via_missing_inputs(
        &mut server,
        agent_id.as_str(),
        public_key.as_str(),
        private_key.as_str(),
        build_nonce + 11,
    );

    let smelter = server
        .world
        .state()
        .factories
        .get("factory.smelter.mk1")
        .expect("smelter state");
    assert!(smelter.production.current_blocker_kind.is_none());
    let assembler = server
        .world
        .state()
        .factories
        .get("factory.assembler.mk1")
        .expect("assembler state");
    assert!(assembler.production.current_blocker_kind.is_some());

    let gameplay = expect_player_gameplay(
        &mut server,
        "player gameplay with healthy smelter and blocked secondary assembler",
    );
    assert_eq!(
        gameplay.goal_id,
        "post_onboarding.choose_first_expansion_tradeoff"
    );
    assert_eq!(
        gameplay.stage_status,
        PlayerGameplayStageStatus::BranchReady
    );
    assert_eq!(gameplay.blocker_kind, None);
}

#[test]
fn chain_linked_gameplay_action_submits_to_chain_and_applies_on_committed_sync() {
    let _guard = lock_test_llm_env();
    let execution_world_dir = runtime_live_temp_dir("chain_gameplay_submit");
    crate::runtime::World::new_production_hardened()
        .save_to_dir(execution_world_dir.as_path())
        .expect("persist initial execution world");

    let chain_status = TestChainStatusServer::start(execution_world_dir.clone());
    let mut server = ViewerRuntimeLiveServer::new(
        ViewerRuntimeLiveServerConfig::new(WorldScenario::Minimal)
            .with_decision_mode(ViewerLiveDecisionMode::Llm)
            .with_chain_status_bind(chain_status.addr.clone())
            .with_chain_poll_interval(Duration::from_millis(50)),
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
    let (public_key, private_key) = test_signer(41);
    let register_ack = register_runtime_session(
        &mut server,
        "player-a",
        Some(agent_id.as_str()),
        40,
        public_key.as_str(),
        private_key.as_str(),
    );
    assert_eq!(
        register_ack.status,
        AuthoritativeRecoveryStatus::SessionRegistered
    );
    server
        .world
        .save_to_dir(execution_world_dir.as_path())
        .expect("persist viewer baseline execution world");

    let submit_request = signed_gameplay_action_request(
        crate::viewer::GameplayActionRequest {
            action_id: "build_factory_smelter_mk1".to_string(),
            target_agent_id: agent_id.clone(),
            actor_agent_id: None,
            player_id: "player-a".to_string(),
            public_key: None,
            auth: None,
        },
        41,
        public_key.as_str(),
        private_key.as_str(),
    );
    let submit_ack = server
        .handle_gameplay_action(submit_request)
        .expect("submit gameplay action to chain runtime");
    assert_eq!(submit_ack.action_id, "build_factory_smelter_mk1");
    assert_eq!(submit_ack.runtime_action_id, 1);
    assert!(
        !server.world.has_factory("factory.smelter.mk1"),
        "chain-linked submit must not mutate local world before committed sync"
    );

    let submitted = chain_status.submitted_gameplay_requests();
    assert_eq!(submitted.len(), 1);
    assert_eq!(submitted[0].action_id, "build_factory_smelter_mk1");
    assert_eq!(submitted[0].target_agent_id, agent_id);

    let mut execution_world = server.world.clone();
    let runtime_action = crate::viewer::build_runtime_action_from_gameplay_request(&submitted[0])
        .expect("rebuild runtime action from submitted request");
    execution_world.submit_action(runtime_action);
    for _ in 0..2 {
        execution_world.step().expect("advance execution world");
    }
    execution_world
        .save_to_dir(execution_world_dir.as_path())
        .expect("persist committed execution world");
    chain_status.committed_height.store(1, Ordering::SeqCst);

    let mut session = RuntimeLiveSession::new();
    session.playing = false;
    session.subscribed.insert(ViewerStream::Events);
    session.subscribed.insert(ViewerStream::Snapshot);
    let (mut writer, peer) = test_writer_pair();
    let progressed = server
        .sync_chain_linked_runtime(&mut session, &mut writer)
        .expect("chain sync should succeed");

    assert!(
        progressed,
        "committed chain world should advance viewer state"
    );
    assert!(server.world.has_factory("factory.smelter.mk1"));
    assert_eq!(server.last_chain_committed_height, 1);
    assert!(read_response_line(&peer, Duration::from_millis(200)).is_some());
}
